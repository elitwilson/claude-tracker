use chrono::{DateTime, TimeDelta, Utc};

#[derive(Debug, PartialEq)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_input_tokens: u64,
    pub cache_read_input_tokens: u64,
}

#[derive(Debug)]
pub struct ParsedMessage {
    pub timestamp: DateTime<Utc>,
    pub cwd: Option<String>,
    pub usage: Option<TokenUsage>,
}

#[derive(Debug)]
pub struct Session {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
    pub duration: TimeDelta,
    pub project: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_input_tokens: u64,
    pub cache_read_input_tokens: u64,
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
            let usage = value
                .get("message")
                .and_then(|m| m.get("usage"))
                .and_then(|u| {
                    Some(TokenUsage {
                        input_tokens: u.get("input_tokens")?.as_u64()?,
                        output_tokens: u.get("output_tokens")?.as_u64()?,
                        cache_creation_input_tokens: u
                            .get("cache_creation_input_tokens")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0),
                        cache_read_input_tokens: u
                            .get("cache_read_input_tokens")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0),
                    })
                });
            Some(ParsedMessage { timestamp, cwd, usage })
        }
        _ => None,
    }
}

/// Assemble a list of parsed messages into a Session.
/// Returns None if the message list is empty.
/// Gaps between consecutive messages that meet or exceed `idle_threshold`
/// are excluded from the duration (clock pauses during idle).
pub fn assemble_session(messages: &[ParsedMessage], idle_threshold: TimeDelta) -> Option<Session> {
    if messages.is_empty() {
        return None;
    }

    let start = messages.first()?.timestamp;
    let end = messages.last()?.timestamp;
    let duration = messages
        .windows(2)
        .map(|w| w[1].timestamp - w[0].timestamp)
        .filter(|gap| *gap < idle_threshold)
        .sum();
    let project = messages
        .iter()
        .filter_map(|m| m.cwd.as_deref())
        .next()
        .unwrap_or("")
        .to_string();

    let input_tokens: u64 = messages.iter().filter_map(|m| m.usage.as_ref()).map(|u| u.input_tokens).sum();
    let output_tokens: u64 = messages.iter().filter_map(|m| m.usage.as_ref()).map(|u| u.output_tokens).sum();
    let cache_creation_input_tokens: u64 = messages.iter().filter_map(|m| m.usage.as_ref()).map(|u| u.cache_creation_input_tokens).sum();
    let cache_read_input_tokens: u64 = messages.iter().filter_map(|m| m.usage.as_ref()).map(|u| u.cache_read_input_tokens).sum();

    Some(Session {
        start,
        end,
        duration,
        project,
        input_tokens,
        output_tokens,
        cache_creation_input_tokens,
        cache_read_input_tokens,
    })
}

#[cfg(test)]
mod tests;
