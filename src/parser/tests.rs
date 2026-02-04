use super::*;
use chrono::{DateTime, Local, TimeDelta, Utc};

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
        },
        ParsedMessage {
            timestamp: "2026-02-03T10:05:30Z".parse().unwrap(),
            cwd: Some("/Users/etwilson/workdev/project".to_string()),
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

#[test]
fn session_today_is_included() {
    let start: DateTime<Utc> = "2026-02-03T12:00:00Z".parse().unwrap();
    let end: DateTime<Utc> = "2026-02-03T12:30:00Z".parse().unwrap();
    // Derive "today" from the session itself so the test is timezone-agnostic
    let today = start.with_timezone(&Local).date_naive();

    let session = Session {
        start,
        end,
        duration: end - start,
        project: "/test".to_string(),
    };

    assert!(is_today(&session, today));
}

#[test]
fn session_yesterday_is_excluded() {
    let start: DateTime<Utc> = "2026-02-03T12:00:00Z".parse().unwrap();
    let end: DateTime<Utc> = "2026-02-03T12:30:00Z".parse().unwrap();
    // "today" is the day after the session ended — session is fully in the past
    let today = end.with_timezone(&Local).date_naive() + TimeDelta::days(1);

    let session = Session {
        start,
        end,
        duration: end - start,
        project: "/test".to_string(),
    };

    assert!(!is_today(&session, today));
}

#[test]
fn session_spanning_midnight_is_included() {
    let start: DateTime<Utc> = "2026-02-03T23:00:00Z".parse().unwrap();
    let end: DateTime<Utc> = "2026-02-04T01:00:00Z".parse().unwrap();
    // "today" is the end's local date — session should be included
    // because end falls on today, even if start does not
    let today = end.with_timezone(&Local).date_naive();

    let session = Session {
        start,
        end,
        duration: end - start,
        project: "/test".to_string(),
    };

    assert!(is_today(&session, today));
}
