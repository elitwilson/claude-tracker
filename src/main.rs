mod parser;
mod scanner;
mod spinner;
mod store;

use anyhow::{Context, Result};
use chrono::{DateTime, Local, TimeDelta, TimeZone, Utc};
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Layout},
    prelude::Stylize,
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Cell, Paragraph, Row, Table},
    Frame, Terminal,
};
use std::collections::HashMap;
use std::io;
use std::path::Path;
use std::sync::mpsc;

// --- Config --------------------------------------------------------------

const DEFAULT_CONFIG: &str = r#"# Claude Tracker Configuration

# Minutes of inactivity before a gap is considered idle time (excluded from duration)
idle_timeout_minutes = 15
"#;

#[derive(serde::Deserialize)]
struct Config {
    #[serde(default = "default_idle_timeout_minutes")]
    idle_timeout_minutes: u64,
}

fn default_idle_timeout_minutes() -> u64 {
    15
}

impl Default for Config {
    fn default() -> Self {
        Self {
            idle_timeout_minutes: default_idle_timeout_minutes(),
        }
    }
}

fn config_path() -> Result<std::path::PathBuf> {
    let home = std::env::var("HOME").context("HOME env var not set")?;
    Ok(Path::new(&home)
        .join(".config")
        .join("claude-tracker")
        .join("config.toml"))
}

fn ensure_config_exists() -> Result<()> {
    let path = config_path()?;
    if !path.exists() {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, DEFAULT_CONFIG)?;
    }
    Ok(())
}

fn load_config() -> Result<Config> {
    let path = config_path()?;
    match std::fs::read_to_string(&path) {
        Ok(contents) => {
            toml::from_str(&contents).with_context(|| format!("parsing {:?}", path))
        }
        Err(_) => Ok(Config::default()),
    }
}

fn open_config_in_editor() -> Result<()> {
    let path = config_path()?;
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());
    std::process::Command::new(&editor)
        .arg(&path)
        .status()
        .with_context(|| format!("failed to open {} with {}", path.display(), editor))?;
    Ok(())
}

// --- Timeframe -----------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
enum Timeframe {
    Today,
    Last7Days,
    Last30Days,
}

impl Timeframe {
    fn next(self) -> Self {
        match self {
            Timeframe::Today => Timeframe::Last7Days,
            Timeframe::Last7Days => Timeframe::Last30Days,
            Timeframe::Last30Days => Timeframe::Today,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            Timeframe::Today => "Today",
            Timeframe::Last7Days => "Last 7 days",
            Timeframe::Last30Days => "Last 30 days",
        }
    }

    /// Compute [start, end) boundaries in UTC. Start and end are local
    /// midnight, converted to UTC for comparison against stored timestamps.
    fn boundaries(&self) -> (DateTime<Utc>, DateTime<Utc>) {
        let today = Local::now().date_naive();
        let days_back: i64 = match self {
            Timeframe::Today => 0,
            Timeframe::Last7Days => 6,
            Timeframe::Last30Days => 29,
        };
        let start_date = today - TimeDelta::days(days_back);
        let end_date = today + TimeDelta::days(1);

        let start_utc = Local
            .from_local_datetime(&start_date.and_hms_opt(0, 0, 0).unwrap())
            .single()
            .unwrap()
            .with_timezone(&Utc);
        let end_utc = Local
            .from_local_datetime(&end_date.and_hms_opt(0, 0, 0).unwrap())
            .single()
            .unwrap()
            .with_timezone(&Utc);

        (start_utc, end_utc)
    }
}

// --- View model ---------------------------------------------------------

struct ProjectSummary {
    project: String,
    total_minutes: i64,
    last_activity: DateTime<Utc>,
    input_tokens: u64,
    output_tokens: u64,
    cache_creation_input_tokens: u64,
    cache_read_input_tokens: u64,
}

/// Group sessions by project path, sum durations, track latest end time.
/// Sorted alphabetically by last path segment for stable display order.
fn aggregate_sessions(sessions: &[parser::Session]) -> Vec<ProjectSummary> {
    let mut map: HashMap<String, ProjectSummary> = HashMap::new();

    for session in sessions {
        let entry = map
            .entry(session.project.clone())
            .or_insert_with(|| ProjectSummary {
                project: session.project.clone(),
                total_minutes: 0,
                last_activity: session.end,
                input_tokens: 0,
                output_tokens: 0,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            });
        entry.total_minutes += session.duration.num_seconds() / 60;
        entry.input_tokens += session.input_tokens;
        entry.output_tokens += session.output_tokens;
        entry.cache_creation_input_tokens += session.cache_creation_input_tokens;
        entry.cache_read_input_tokens += session.cache_read_input_tokens;
        if session.end > entry.last_activity {
            entry.last_activity = session.end;
        }
    }

    let mut summaries: Vec<_> = map.into_values().collect();
    summaries.sort_by(|a, b| last_segment(&a.project).cmp(last_segment(&b.project)));
    summaries
}

fn last_segment(path: &str) -> &str {
    Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(path)
}

fn format_tokens(n: u64) -> String {
    if n >= 1_000_000 {
        let val = n as f64 / 1_000_000.0;
        let rounded = (val * 10.0).round() / 10.0;
        if rounded == rounded.floor() {
            format!("{}M", rounded as u64)
        } else {
            format!("{:.1}M", rounded)
        }
    } else if n >= 1_000 {
        let val = n as f64 / 1_000.0;
        let rounded = (val * 10.0).round() / 10.0;
        if rounded == rounded.floor() {
            format!("{}k", rounded as u64)
        } else {
            format!("{:.1}k", rounded)
        }
    } else {
        format!("{}", n)
    }
}

// --- Rendering ----------------------------------------------------------

enum PendingAction {
    Quit,
    Config,
}

enum KeyOutcome {
    Quit,
    OpenConfig,
    Continue,
}

/// Dispatch a keypress against the pending-action state.
/// Confirms or cancels a pending action on Enter / any-other-key,
/// and arms the initial 'q' → Quit when nothing is pending.
fn handle_key(pending: &mut Option<PendingAction>, code: KeyCode) -> KeyOutcome {
    if pending.is_some() {
        match code {
            KeyCode::Enter => match pending.take() {
                Some(PendingAction::Quit) => KeyOutcome::Quit,
                Some(PendingAction::Config) => KeyOutcome::OpenConfig,
                None => unreachable!(),
            },
            _ => {
                *pending = None;
                KeyOutcome::Continue
            }
        }
    } else if code == KeyCode::Char('q') {
        *pending = Some(PendingAction::Quit);
        KeyOutcome::Continue
    } else {
        KeyOutcome::Continue
    }
}

fn render(f: &mut Frame, summaries: &[ProjectSummary], spinner: &spinner::Spinner, pending: &Option<PendingAction>, timeframe_label: &str) {
    let most_recent_idx = summaries
        .iter()
        .enumerate()
        .max_by_key(|(_, s)| s.last_activity)
        .map(|(i, _)| i);

    let total_minutes: i64 = summaries.iter().map(|s| s.total_minutes).sum();
    let total_input_tokens: u64 = summaries.iter().map(|s| s.input_tokens + s.cache_creation_input_tokens).sum();
    let total_output_tokens: u64 = summaries.iter().map(|s| s.output_tokens).sum();
    let total_cache_read_tokens: u64 = summaries.iter().map(|s| s.cache_read_input_tokens).sum();

    let chunks = Layout::vertical([
        Constraint::Length(1),                       // header
        Constraint::Length(1),                       // blank
        Constraint::Length(summaries.len() as u16 + 3), // table + header + border
        Constraint::Length(1),                       // blank
        Constraint::Length(1),                       // totals
        Constraint::Length(1),                       // blank
        Constraint::Length(1),                       // footer
        Constraint::Fill(1),                         // remaining
    ])
    .split(f.area());

    // Header
    f.render_widget(
        Paragraph::new(format!("claude-tracker  {}", spinner.current()))
            .style(Style::new().bold()),
        chunks[0],
    );

    // Project table
    let rows: Vec<Row> = summaries
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let name = last_segment(&s.project);
            let name_cell = if Some(i) == most_recent_idx {
                Cell::new(Text::from(Line::from(vec![
                    Span::styled("  ● ", Style::new().fg(Color::Green)),
                    Span::raw(name.to_string()),
                ])))
            } else {
                Cell::new(format!("    {}", name))
            };
            Row::new([
                name_cell,
                Cell::new(Text::from(format!("{}m ({}h {}m)", s.total_minutes, s.total_minutes / 60, s.total_minutes % 60)).alignment(Alignment::Right)),
                Cell::new(Text::from(format_tokens(s.input_tokens + s.cache_creation_input_tokens)).alignment(Alignment::Right)),
                Cell::new(Text::from(format_tokens(s.output_tokens)).alignment(Alignment::Right)),
                Cell::new(Text::from(format_tokens(s.cache_read_input_tokens)).alignment(Alignment::Right)),
            ])
        })
        .collect();

    let block = Block::new()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(Color::DarkGray))
        .title(" projects ");

    let header = Row::new([
        Cell::new(""),
        Cell::new(Text::from("Time").alignment(Alignment::Right)).style(Style::new().italic()),
        Cell::new(Text::from("Input").alignment(Alignment::Right)).style(Style::new().italic()),
        Cell::new(Text::from("Output").alignment(Alignment::Right)).style(Style::new().italic()),
        Cell::new(Text::from("Cache").alignment(Alignment::Right)).style(Style::new().italic()),
    ])
    .style(Style::new().bold());

    let table = Table::new(rows, [
        Constraint::Fill(1),
        Constraint::Min(12), // "time" / "98m (1h 38m)"
        Constraint::Min(5),  // "input" / "30.8M"
        Constraint::Min(6),  // "output" / "2.9k"
        Constraint::Min(5),  // "cache" / "44.2M"
    ])
    .header(header)
    .block(block)
    .column_spacing(2);

    f.render_widget(table, chunks[2]);

    // Totals
    f.render_widget(
        Paragraph::new(format!(
            "  {}: {}m  ({}h {}m)  {} in  {} out  {} cache",
            timeframe_label,
            total_minutes,
            total_minutes / 60,
            total_minutes % 60,
            format_tokens(total_input_tokens),
            format_tokens(total_output_tokens),
            format_tokens(total_cache_read_tokens),
        )),
        chunks[4],
    );

    // Footer
    let (footer_text, footer_style) = match pending {
        Some(PendingAction::Quit) => ("  Quit? Press Enter to confirm", Style::new()),
        Some(PendingAction::Config) => ("  Open config? Press Enter to confirm", Style::new()),
        None => ("  t timeframe · r refresh · c config · q quit", Style::new().dim()),
    };
    f.render_widget(
        Paragraph::new(footer_text).style(footer_style),
        chunks[6],
    );
}

// --- Terminal lifecycle -------------------------------------------------

fn setup() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    Ok(Terminal::new(CrosstermBackend::new(stdout))?)
}

fn teardown(term: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    terminal::disable_raw_mode()?;
    execute!(term.backend_mut(), LeaveAlternateScreen)?;
    term.show_cursor()?;
    Ok(())
}

// --- Data loading -------------------------------------------------------

fn scan_and_parse(
    projects_dir: &Path,
    idle_threshold: TimeDelta,
) -> Result<Vec<(String, parser::Session)>> {
    let session_files = scanner::find_session_files(projects_dir);
    let mut results = Vec::new();

    for file_path in &session_files {
        let contents = std::fs::read_to_string(file_path)
            .with_context(|| format!("reading {:?}", file_path))?;

        let messages: Vec<parser::ParsedMessage> = contents
            .lines()
            .filter_map(parser::parse_message)
            .collect();

        if let Some(session) = parser::assemble_session(&messages, idle_threshold) {
            let source_path = file_path
                .strip_prefix(projects_dir)
                .with_context(|| format!("stripping prefix from {:?}", file_path))?
                .to_str()
                .context("non-UTF8 path")?;
            results.push((source_path.to_string(), session));
        }
    }

    Ok(results)
}

// --- Entry point --------------------------------------------------------

use std::time::{Duration, Instant};

const TICK_RATE: Duration = Duration::from_millis(100);
const REFRESH_INTERVAL: Duration = Duration::from_secs(2);

fn main() -> Result<()> {
    let home = std::env::var("HOME").context("HOME env var not set")?;
    let projects_dir = Path::new(&home).join(".claude").join("projects");

    ensure_config_exists()?;
    let config = load_config()?;
    let idle_threshold = TimeDelta::minutes(config.idle_timeout_minutes as i64);

    let db_path = config_path()?.with_file_name("sessions.db");
    let store = store::Store::new(&db_path)?;

    for (source_path, session) in scan_and_parse(&projects_dir, idle_threshold)? {
        store.upsert(&source_path, &session)?;
    }

    let mut timeframe = Timeframe::Today;
    let mut summaries = {
        let (start, end) = timeframe.boundaries();
        aggregate_sessions(&store.query_range(start, end)?)
    };
    let mut spinner = spinner::Spinner::new();
    let mut last_refresh = Instant::now();

    // Restore terminal on panic so we don't leave alternate screen active
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = terminal::disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        hook(info);
    }));

    let mut term = setup()?;

    let (tx, rx) = mpsc::channel::<Result<Vec<(String, parser::Session)>>>();
    let mut scan_in_progress = false;
    let mut needs_refresh = false;
    let mut pending: Option<PendingAction> = None;

    loop {
        term.draw(|f| render(f, &summaries, &spinner, &pending, timeframe.label()))?;

        // Process completed background scan
        if let Ok(result) = rx.try_recv() {
            scan_in_progress = false;
            if let Ok(sessions) = result {
                for (source_path, session) in sessions {
                    store.upsert(&source_path, &session)?;
                }
                let (start, end) = timeframe.boundaries();
                summaries = aggregate_sessions(&store.query_range(start, end)?);
            }
            last_refresh = Instant::now();
        }

        if event::poll(TICK_RATE)? {
            if let Event::Key(k) = event::read()? {
                let had_pending = pending.is_some();
                match handle_key(&mut pending, k.code) {
                    KeyOutcome::Quit => break,
                    KeyOutcome::OpenConfig => {
                        teardown(&mut term)?;
                        open_config_in_editor()?;
                        return Ok(());
                    }
                    // Only process r/c when the key wasn't consumed by pending logic.
                    KeyOutcome::Continue if !had_pending => match k.code {
                        KeyCode::Char('r') => {
                            if !scan_in_progress {
                                spinner.reset();
                                needs_refresh = true;
                            }
                        }
                        KeyCode::Char('c') => {
                            pending = Some(PendingAction::Config);
                        }
                        KeyCode::Char('t') => {
                            timeframe = timeframe.next();
                            let (start, end) = timeframe.boundaries();
                            summaries = aggregate_sessions(&store.query_range(start, end)?);
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
        }

        spinner.tick();

        // Spawn background scan if not already running and due
        if !scan_in_progress && (needs_refresh || last_refresh.elapsed() >= REFRESH_INTERVAL) {
            let dir = projects_dir.clone();
            let threshold = idle_threshold;
            let sender = tx.clone();
            std::thread::spawn(move || {
                let _ = sender.send(scan_and_parse(&dir, threshold));
            });
            scan_in_progress = true;
            needs_refresh = false;
        }
    }

    teardown(&mut term)
}

// --- Tests --------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeDelta;

    fn session(project: &str, end: &str, secs: i64) -> parser::Session {
        let end: DateTime<Utc> = end.parse().unwrap();
        parser::Session {
            start: end - TimeDelta::seconds(secs),
            end,
            duration: TimeDelta::seconds(secs),
            project: project.to_string(),
            input_tokens: 0,
            output_tokens: 0,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
        }
    }

    fn session_with_tokens(
        project: &str,
        end: &str,
        secs: i64,
        input: u64,
        output: u64,
        cache_create: u64,
        cache_read: u64,
    ) -> parser::Session {
        let end: DateTime<Utc> = end.parse().unwrap();
        parser::Session {
            start: end - TimeDelta::seconds(secs),
            end,
            duration: TimeDelta::seconds(secs),
            project: project.to_string(),
            input_tokens: input,
            output_tokens: output,
            cache_creation_input_tokens: cache_create,
            cache_read_input_tokens: cache_read,
        }
    }

    #[test]
    fn groups_and_sums_duration() {
        let input = vec![
            session("/work/api", "2026-02-03T10:00:00Z", 600),  // 10m
            session("/work/api", "2026-02-03T11:00:00Z", 1200), // 20m
        ];

        let out = aggregate_sessions(&input);

        assert_eq!(out.len(), 1);
        assert_eq!(out[0].total_minutes, 30);
    }

    #[test]
    fn tracks_latest_end_as_last_activity() {
        let late: DateTime<Utc> = "2026-02-03T11:00:00Z".parse().unwrap();

        let input = vec![
            session("/work/api", "2026-02-03T10:00:00Z", 600),
            session("/work/api", "2026-02-03T11:00:00Z", 600),
        ];

        let out = aggregate_sessions(&input);

        assert_eq!(out[0].last_activity, late);
    }

    #[test]
    fn separate_projects_stay_separate() {
        let input = vec![
            session("/work/api", "2026-02-03T10:00:00Z", 600),  // 10m
            session("/work/cli", "2026-02-03T10:00:00Z", 1800), // 30m
        ];

        let out = aggregate_sessions(&input);

        assert_eq!(out.len(), 2);
        let api = out.iter().find(|s| s.project == "/work/api").unwrap();
        let cli = out.iter().find(|s| s.project == "/work/cli").unwrap();
        assert_eq!(api.total_minutes, 10);
        assert_eq!(cli.total_minutes, 30);
    }

    // --- Token aggregation ---------------------------------------------------

    #[test]
    fn aggregates_token_totals_across_sessions() {
        let input = vec![
            session_with_tokens("/work/api", "2026-02-03T10:00:00Z", 600, 100, 50, 200, 300),
            session_with_tokens("/work/api", "2026-02-03T11:00:00Z", 600, 50, 25, 100, 150),
        ];

        let out = aggregate_sessions(&input);

        assert_eq!(out.len(), 1);
        assert_eq!(out[0].input_tokens, 150);
        assert_eq!(out[0].output_tokens, 75);
        assert_eq!(out[0].cache_creation_input_tokens, 300);
        assert_eq!(out[0].cache_read_input_tokens, 450);
    }

    #[test]
    fn token_totals_stay_separate_per_project() {
        let input = vec![
            session_with_tokens("/work/api", "2026-02-03T10:00:00Z", 600, 100, 50, 0, 0),
            session_with_tokens("/work/cli", "2026-02-03T10:00:00Z", 600, 200, 75, 0, 0),
        ];

        let out = aggregate_sessions(&input);

        let api = out.iter().find(|s| s.project == "/work/api").unwrap();
        let cli = out.iter().find(|s| s.project == "/work/cli").unwrap();
        assert_eq!(api.input_tokens, 100);
        assert_eq!(api.output_tokens, 50);
        assert_eq!(cli.input_tokens, 200);
        assert_eq!(cli.output_tokens, 75);
    }

    // --- Token formatting ----------------------------------------------------

    #[test]
    fn formats_tokens_below_thousand() {
        assert_eq!(format_tokens(150), "150");
    }

    #[test]
    fn formats_tokens_in_thousands() {
        assert_eq!(format_tokens(3200), "3.2k");
    }

    #[test]
    fn formats_tokens_in_millions() {
        assert_eq!(format_tokens(1_200_000), "1.2M");
    }

    #[test]
    fn formats_exact_thousand() {
        assert_eq!(format_tokens(1000), "1k");
    }

    // --- Timeframe cycling ---------------------------------------------------

    #[test]
    fn timeframe_cycles_today_7d_30d_and_wraps() {
        let mut tf = Timeframe::Today;

        tf = tf.next();
        assert_eq!(tf, Timeframe::Last7Days);

        tf = tf.next();
        assert_eq!(tf, Timeframe::Last30Days);

        tf = tf.next();
        assert_eq!(tf, Timeframe::Today);
    }
}
