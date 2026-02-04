mod parser;
mod scanner;
mod spinner;

use anyhow::{Context, Result};
use chrono::{DateTime, Local, TimeDelta, Utc};
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    text::Line,
    widgets::Paragraph,
    Frame, Terminal,
};
use std::collections::HashMap;
use std::io;
use std::path::Path;

// --- Config --------------------------------------------------------------

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

fn load_config() -> Result<Config> {
    let home = std::env::var("HOME").context("HOME env var not set")?;
    let config_path = Path::new(&home)
        .join(".config")
        .join("claude-tracker")
        .join("config.toml");

    match std::fs::read_to_string(&config_path) {
        Ok(contents) => {
            toml::from_str(&contents).with_context(|| format!("parsing {:?}", config_path))
        }
        Err(_) => Ok(Config::default()),
    }
}

// --- View model ---------------------------------------------------------

struct ProjectSummary {
    project: String,
    total_minutes: i64,
    last_activity: DateTime<Utc>,
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
            });
        entry.total_minutes += session.duration.num_seconds() / 60;
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

// --- Rendering ----------------------------------------------------------

const NOW_MARKER: &str = "  ← now";
const NOW_MARKER_COLS: usize = 7; // "← " is 1 display column despite 3 UTF-8 bytes

fn render(f: &mut Frame, summaries: &[ProjectSummary], spinner: &spinner::Spinner) {
    let width = f.area().width as usize;

    let most_recent_idx = summaries
        .iter()
        .enumerate()
        .max_by_key(|(_, s)| s.last_activity)
        .map(|(i, _)| i);

    let total_minutes: i64 = summaries.iter().map(|s| s.total_minutes).sum();

    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(format!("claude-tracker  {}", spinner.current())));
    lines.push(Line::from(""));

    for (i, summary) in summaries.iter().enumerate() {
        let name = last_segment(&summary.project);
        let time = format!("{}m", summary.total_minutes);
        let (suffix, suffix_cols) = if Some(i) == most_recent_idx {
            (NOW_MARKER, NOW_MARKER_COLS)
        } else {
            ("", 0)
        };

        let inner = width.saturating_sub(2 + suffix_cols);
        let pad = inner.saturating_sub(name.len() + time.len());

        lines.push(Line::from(format!(
            "  {}{}{}{}",
            name,
            " ".repeat(pad),
            time,
            suffix
        )));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(format!(
        "  Today: {}m  ({}h {}m)",
        total_minutes,
        total_minutes / 60,
        total_minutes % 60
    )));

    f.render_widget(Paragraph::new(lines), f.area());
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

fn load_sessions(projects_dir: &Path, idle_threshold: TimeDelta) -> Result<Vec<parser::Session>> {
    let session_files = scanner::find_session_files(projects_dir);
    let today = Local::now().date_naive();
    let mut sessions = Vec::new();

    for file_path in &session_files {
        let contents = std::fs::read_to_string(file_path)
            .with_context(|| format!("reading {:?}", file_path))?;

        let messages: Vec<parser::ParsedMessage> = contents
            .lines()
            .filter_map(parser::parse_message)
            .collect();

        if let Some(session) = parser::assemble_session(&messages, idle_threshold) {
            if parser::is_today(&session, today) {
                sessions.push(session);
            }
        }
    }

    Ok(sessions)
}

// --- Entry point --------------------------------------------------------

use std::time::{Duration, Instant};

const TICK_RATE: Duration = Duration::from_millis(100);
const REFRESH_INTERVAL: Duration = Duration::from_secs(2);

fn main() -> Result<()> {
    let home = std::env::var("HOME").context("HOME env var not set")?;
    let projects_dir = Path::new(&home).join(".claude").join("projects");

    let config = load_config()?;
    let idle_threshold = TimeDelta::minutes(config.idle_timeout_minutes as i64);

    let sessions = load_sessions(&projects_dir, idle_threshold)?;

    if sessions.is_empty() {
        println!("No sessions today.");
        return Ok(());
    }

    let mut summaries = aggregate_sessions(&sessions);
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

    let mut needs_refresh = false;

    loop {
        term.draw(|f| render(f, &summaries, &spinner))?;

        if event::poll(TICK_RATE)? {
            match event::read()? {
                Event::Key(k) if k.code == KeyCode::Char('q') => break,
                Event::Key(k) if k.code == KeyCode::Char('r') => {
                    spinner.reset();
                    needs_refresh = true;
                }
                _ => {}
            }
        }

        spinner.tick();

        if needs_refresh || last_refresh.elapsed() >= REFRESH_INTERVAL {
            // Drain event queue before expensive refresh, checking for quit
            let mut should_quit = false;
            while event::poll(Duration::ZERO)? {
                if let Event::Key(k) = event::read()? {
                    if k.code == KeyCode::Char('q') {
                        should_quit = true;
                        break;
                    }
                }
            }
            if should_quit {
                break;
            }

            let sessions = load_sessions(&projects_dir, idle_threshold)?;
            summaries = aggregate_sessions(&sessions);
            last_refresh = Instant::now();
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
}
