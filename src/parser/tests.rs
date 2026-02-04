use super::*;
use chrono::{DateTime, TimeDelta, Utc};

// Realistic fixtures drawn from actual Claude Code transcript format.
// Include enough fields to verify the parser ignores irrelevant keys.

const USER_MESSAGE: &str = r#"{
    "type": "user",
    "timestamp": "2026-02-03T17:36:56.625Z",
    "cwd": "/Users/etwilson/workdev/tools/make-it-so-cli",
    "sessionId": "8e17c8fc-560f-43be-9e19-c99b6a6da169",
    "message": {"role": "user", "content": [{"type": "text", "text": "hello"}]},
    "uuid": "602ff260-e1a6-489f-b3cc-9ec2dac08e6a",
    "parentUuid": null,
    "isSidechain": false
}"#;

const ASSISTANT_MESSAGE: &str = r#"{
    "type": "assistant",
    "timestamp": "2026-02-03T17:37:02.289Z",
    "cwd": "/Users/etwilson/workdev/tools/make-it-so-cli",
    "sessionId": "8e17c8fc-560f-43be-9e19-c99b6a6da169",
    "message": {
        "role": "assistant",
        "content": [{"type": "text", "text": "Hi there"}],
        "usage": {"input_tokens": 100, "output_tokens": 50}
    },
    "uuid": "d2c39245-755d-4684-bbf7-05756ea0b3ac",
    "parentUuid": "602ff260-e1a6-489f-b3cc-9ec2dac08e6a"
}"#;

// Assistant message with all four token usage fields (including cache).
const ASSISTANT_MESSAGE_WITH_CACHE: &str = r#"{
    "type": "assistant",
    "timestamp": "2026-02-03T17:37:10.000Z",
    "cwd": "/Users/etwilson/workdev/tools/make-it-so-cli",
    "sessionId": "8e17c8fc-560f-43be-9e19-c99b6a6da169",
    "message": {
        "role": "assistant",
        "content": [{"type": "text", "text": "Sure thing"}],
        "usage": {
            "input_tokens": 100,
            "output_tokens": 50,
            "cache_creation_input_tokens": 200,
            "cache_read_input_tokens": 300
        }
    },
    "uuid": "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
    "parentUuid": "602ff260-e1a6-489f-b3cc-9ec2dac08e6a"
}"#;

// Assistant message with no usage field at all — edge case.
const ASSISTANT_MESSAGE_NO_USAGE: &str = r#"{
    "type": "assistant",
    "timestamp": "2026-02-03T17:37:15.000Z",
    "cwd": "/Users/etwilson/workdev/tools/make-it-so-cli",
    "sessionId": "8e17c8fc-560f-43be-9e19-c99b6a6da169",
    "message": {
        "role": "assistant",
        "content": [{"type": "text", "text": "Done"}]
    },
    "uuid": "bbbbbbbb-cccc-dddd-eeee-ffffffffffff",
    "parentUuid": "602ff260-e1a6-489f-b3cc-9ec2dac08e6a"
}"#;

const QUEUE_OPERATION: &str = r#"{
    "type": "queue-operation",
    "operation": "dequeue",
    "timestamp": "2026-02-03T17:36:56.582Z",
    "sessionId": "8e17c8fc-560f-43be-9e19-c99b6a6da169"
}"#;

// Note: file-history-snapshot does NOT have a top-level timestamp.
// The timestamp is nested inside "snapshot". This is real format behavior.
const FILE_HISTORY_SNAPSHOT: &str = r#"{
    "type": "file-history-snapshot",
    "messageId": "602ff260-e1a6-489f-b3cc-9ec2dac08e6a",
    "snapshot": {
        "messageId": "602ff260-e1a6-489f-b3cc-9ec2dac08e6a",
        "trackedFileBackups": {},
        "timestamp": "2026-02-03T17:36:56.628Z"
    },
    "isSnapshotUpdate": false
}"#;

#[test]
fn parses_user_message() {
    let msg = parse_message(USER_MESSAGE).unwrap();

    let expected_ts: DateTime<Utc> = "2026-02-03T17:36:56.625Z".parse().unwrap();
    assert_eq!(msg.timestamp, expected_ts);
    assert_eq!(msg.cwd.as_deref(), Some("/Users/etwilson/workdev/tools/make-it-so-cli"));
}

#[test]
fn parses_assistant_message() {
    let msg = parse_message(ASSISTANT_MESSAGE).unwrap();

    let expected_ts: DateTime<Utc> = "2026-02-03T17:37:02.289Z".parse().unwrap();
    assert_eq!(msg.timestamp, expected_ts);
    assert_eq!(msg.cwd.as_deref(), Some("/Users/etwilson/workdev/tools/make-it-so-cli"));
}

#[test]
fn skips_queue_operation() {
    assert!(parse_message(QUEUE_OPERATION).is_none());
}

#[test]
fn skips_file_history_snapshot() {
    assert!(parse_message(FILE_HISTORY_SNAPSHOT).is_none());
}

#[test]
fn handles_malformed_line() {
    assert!(parse_message("this is not valid json").is_none());
}

#[test]
fn calculates_session_duration() {
    let messages = vec![
        ParsedMessage {
            timestamp: "2026-02-03T10:00:00Z".parse().unwrap(),
            cwd: Some("/Users/etwilson/workdev/project".to_string()),
            usage: None,
        },
        ParsedMessage {
            timestamp: "2026-02-03T10:05:30Z".parse().unwrap(),
            cwd: Some("/Users/etwilson/workdev/project".to_string()),
            usage: None,
        },
    ];

    let session = assemble_session(&messages, TimeDelta::minutes(15)).unwrap();

    assert_eq!(session.start, "2026-02-03T10:00:00Z".parse::<DateTime<Utc>>().unwrap());
    assert_eq!(session.end, "2026-02-03T10:05:30Z".parse::<DateTime<Utc>>().unwrap());
    assert_eq!(session.duration.num_seconds(), 330); // 5 min 30 sec, single gap below threshold
    assert_eq!(session.project, "/Users/etwilson/workdev/project");
}

#[test]
fn filters_empty_sessions() {
    let messages: Vec<ParsedMessage> = vec![];

    assert!(assemble_session(&messages, TimeDelta::minutes(15)).is_none());
}

// --- Idle timeout gap tests ---------------------------------------------

/// Helper: a message at the given timestamp with a fixed cwd.
fn msg(timestamp: &str) -> ParsedMessage {
    ParsedMessage {
        timestamp: timestamp.parse().unwrap(),
        cwd: Some("/work/project".to_string()),
        usage: None,
    }
}

#[test]
fn gaps_below_threshold_count_fully() {
    // Gaps: 5m, 3m — both under 15m threshold → full 8m counted
    let messages = vec![
        msg("2026-02-03T10:00:00Z"),
        msg("2026-02-03T10:05:00Z"),
        msg("2026-02-03T10:08:00Z"),
    ];

    let session = assemble_session(&messages, TimeDelta::minutes(15)).unwrap();

    assert_eq!(session.duration.num_seconds(), 480); // 8 min
}

#[test]
fn gap_above_threshold_is_excluded() {
    // Gaps: 5m (below), 30m (above) → only the 5m counts
    let messages = vec![
        msg("2026-02-03T10:00:00Z"),
        msg("2026-02-03T10:05:00Z"),
        msg("2026-02-03T10:35:00Z"),
    ];

    let session = assemble_session(&messages, TimeDelta::minutes(15)).unwrap();

    assert_eq!(session.duration.num_seconds(), 300); // 5 min
}

#[test]
fn mixed_gaps_only_large_ones_dropped() {
    // Gaps: 3m (below), 20m (above), 4m (below) → 3 + 4 = 7m
    let messages = vec![
        msg("2026-02-03T10:00:00Z"),
        msg("2026-02-03T10:03:00Z"),
        msg("2026-02-03T10:23:00Z"),
        msg("2026-02-03T10:27:00Z"),
    ];

    let session = assemble_session(&messages, TimeDelta::minutes(15)).unwrap();

    assert_eq!(session.duration.num_seconds(), 420); // 7 min
}

// --- Token usage tests --------------------------------------------------

#[test]
fn extracts_token_usage_from_assistant_message() {
    let msg = parse_message(ASSISTANT_MESSAGE_WITH_CACHE).unwrap();

    assert_eq!(
        msg.usage,
        Some(TokenUsage {
            input_tokens: 100,
            output_tokens: 50,
            cache_creation_input_tokens: 200,
            cache_read_input_tokens: 300,
        })
    );
}

#[test]
fn user_message_has_no_token_usage() {
    let msg = parse_message(USER_MESSAGE).unwrap();

    assert!(msg.usage.is_none());
}

#[test]
fn assistant_message_without_usage_field_returns_none() {
    let msg = parse_message(ASSISTANT_MESSAGE_NO_USAGE).unwrap();

    assert!(msg.usage.is_none());
}

#[test]
fn assemble_session_sums_token_usage() {
    let messages = vec![
        ParsedMessage {
            timestamp: "2026-02-03T10:00:00Z".parse().unwrap(),
            cwd: Some("/work/project".to_string()),
            usage: Some(TokenUsage {
                input_tokens: 100,
                output_tokens: 50,
                cache_creation_input_tokens: 200,
                cache_read_input_tokens: 300,
            }),
        },
        ParsedMessage {
            timestamp: "2026-02-03T10:05:00Z".parse().unwrap(),
            cwd: Some("/work/project".to_string()),
            usage: Some(TokenUsage {
                input_tokens: 50,
                output_tokens: 25,
                cache_creation_input_tokens: 100,
                cache_read_input_tokens: 150,
            }),
        },
    ];

    let session = assemble_session(&messages, TimeDelta::minutes(15)).unwrap();

    assert_eq!(session.input_tokens, 150);
    assert_eq!(session.output_tokens, 75);
    assert_eq!(session.cache_creation_input_tokens, 300);
    assert_eq!(session.cache_read_input_tokens, 450);
}

#[test]
fn assemble_session_ignores_messages_without_usage() {
    let messages = vec![
        ParsedMessage {
            timestamp: "2026-02-03T10:00:00Z".parse().unwrap(),
            cwd: Some("/work/project".to_string()),
            usage: None,
        },
        ParsedMessage {
            timestamp: "2026-02-03T10:05:00Z".parse().unwrap(),
            cwd: Some("/work/project".to_string()),
            usage: Some(TokenUsage {
                input_tokens: 100,
                output_tokens: 50,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }),
        },
        ParsedMessage {
            timestamp: "2026-02-03T10:06:00Z".parse().unwrap(),
            cwd: Some("/work/project".to_string()),
            usage: None,
        },
    ];

    let session = assemble_session(&messages, TimeDelta::minutes(15)).unwrap();

    assert_eq!(session.input_tokens, 100);
    assert_eq!(session.output_tokens, 50);
    assert_eq!(session.cache_creation_input_tokens, 0);
    assert_eq!(session.cache_read_input_tokens, 0);
}
