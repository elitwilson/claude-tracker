mod parser;
mod scanner;

use anyhow::{Context, Result};
use chrono::Local;
use std::fs;
use std::path::Path;

fn main() -> Result<()> {
    let home = std::env::var("HOME").context("HOME env var not set")?;
    let projects_dir = Path::new(&home).join(".claude").join("projects");

    let session_files = scanner::find_session_files(&projects_dir);

    let today = Local::now().date_naive();
    let mut sessions = Vec::new();

    for file_path in &session_files {
        let contents = fs::read_to_string(file_path)
            .with_context(|| format!("reading {:?}", file_path))?;

        let messages: Vec<parser::ParsedMessage> = contents
            .lines()
            .filter_map(parser::parse_message)
            .collect();

        if let Some(session) = parser::assemble_session(&messages) {
            if parser::is_today(&session, today) {
                sessions.push(session);
            }
        }
    }

    sessions.sort_by_key(|s| s.start);

    if sessions.is_empty() {
        println!("No sessions today.");
        return Ok(());
    }

    println!("Sessions for {}:", today);
    let mut total_secs: i64 = 0;
    for session in &sessions {
        let start_local = session.start.with_timezone(&Local);
        let end_local = session.end.with_timezone(&Local);
        let secs = session.duration.num_seconds();
        total_secs += secs;
        println!(
            "  {}  {}m  ({}–{})",
            session.project,
            secs / 60,
            start_local.format("%H:%M"),
            end_local.format("%H:%M"),
        );
    }
    println!("Total: {}m across {} session(s)", total_secs / 60, sessions.len());

    Ok(())
}
