use chrono::{DateTime, Local, NaiveDate, TimeDelta, Utc};

#[derive(Debug)]
pub struct ParsedMessage {
    pub timestamp: DateTime<Utc>,
    pub cwd: Option<String>,
}

#[derive(Debug)]
pub struct Session {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub duration: TimeDelta,
    pub project: String,
}

/// Parse a single JSONL line into a ParsedMessage.
/// Returns None for non-user/assistant messages and unparseable lines.
pub fn parse_message(line: &str) -> Option<ParsedMessage> {
    let value: serde_json::Value = serde_json::from_str(line).ok()?;

    let msg_type = value.get("type")?.as_str()?;

    match msg_type {
        "user" | "assistant" => {
            let timestamp_str = value.get("timestamp")?.as_str()?;
            let timestamp: DateTime<Utc> = timestamp_str.parse().ok()?;
            let cwd = value.get("cwd").and_then(|v| v.as_str()).map(|s| s.to_string());
            Some(ParsedMessage { timestamp, cwd })
        }
        _ => None,
    }
}

/// Assemble a list of parsed messages into a Session.
/// Returns None if the message list is empty.
pub fn assemble_session(messages: &[ParsedMessage]) -> Option<Session> {
    if messages.is_empty() {
        return None;
    }

    let start = messages.first()?.timestamp;
    let end = messages.last()?.timestamp;
    let duration = end - start;
    let project = messages
        .iter()
        .filter_map(|m| m.cwd.as_deref())
        .next()
        .unwrap_or("")
        .to_string();

    Some(Session {
        start,
        end,
        duration,
        project,
    })
}

/// True if the session has any activity on the given local calendar date.
pub fn is_today(session: &Session, today: NaiveDate) -> bool {
    let start_date = session.start.with_timezone(&Local).date_naive();
    let end_date = session.end.with_timezone(&Local).date_naive();
    start_date == today || end_date == today
}

#[cfg(test)]
mod tests;
