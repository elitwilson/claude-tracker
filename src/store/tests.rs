use super::*;
use anyhow::Result;
use chrono::{DateTime, Local, TimeDelta, Utc};
use tempfile::tempdir;

fn make_session(start: &str, end: &str, duration_secs: i64) -> parser::Session {
    parser::Session {
        start: start.parse().unwrap(),
        end: end.parse().unwrap(),
        duration: TimeDelta::seconds(duration_secs),
        project: "/work/test".to_string(),
        input_tokens: 0,
        output_tokens: 0,
        cache_creation_input_tokens: 0,
        cache_read_input_tokens: 0,
    }
}

#[test]
fn schema_creates_sessions_table() -> Result<()> {
    let dir = tempdir()?;
    let store = Store::new(&dir.path().join("test.db"))?;

    let count: i64 = store.conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='sessions'",
        [],
        |row| row.get(0),
    )?;

    assert_eq!(count, 1);
    Ok(())
}

#[test]
fn upsert_is_idempotent() -> Result<()> {
    let dir = tempdir()?;
    let store = Store::new(&dir.path().join("test.db"))?;
    let session = make_session("2026-02-04T10:00:00Z", "2026-02-04T10:30:00Z", 1800);

    store.upsert("abc123/session-1.jsonl", &session)?;
    store.upsert("abc123/session-1.jsonl", &session)?;

    let count: i64 = store.conn.query_row(
        "SELECT COUNT(*) FROM sessions",
        [],
        |row| row.get(0),
    )?;

    assert_eq!(count, 1);
    Ok(())
}

#[test]
fn upsert_overwrites_active_session() -> Result<()> {
    let dir = tempdir()?;
    let store = Store::new(&dir.path().join("test.db"))?;

    let initial = make_session("2026-02-04T10:00:00Z", "2026-02-04T10:30:00Z", 1800);
    store.upsert("abc123/session-1.jsonl", &initial)?;

    let updated = make_session("2026-02-04T10:00:00Z", "2026-02-04T10:45:00Z", 2700);
    store.upsert("abc123/session-1.jsonl", &updated)?;

    let (end_time, duration): (String, i64) = store.conn.query_row(
        "SELECT end_time, duration_seconds FROM sessions WHERE source_path = ?1",
        ["abc123/session-1.jsonl"],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    assert_eq!(end_time, "2026-02-04T10:45:00Z");
    assert_eq!(duration, 2700);
    Ok(())
}

#[test]
fn date_column_for_midnight_spanning_session() -> Result<()> {
    let dir = tempdir()?;
    let store = Store::new(&dir.path().join("test.db"))?;

    // 24h apart — guaranteed different local dates in any timezone.
    // Verifies that `date` is derived from start_time, not end_time.
    let session = make_session("2026-02-03T12:00:00Z", "2026-02-04T12:00:00Z", 86400);
    store.upsert("abc123/session-1.jsonl", &session)?;

    let date: String = store.conn.query_row(
        "SELECT date FROM sessions WHERE source_path = ?1",
        ["abc123/session-1.jsonl"],
        |row| row.get(0),
    )?;

    let expected = session.start.with_timezone(&Local).format("%Y-%m-%d").to_string();
    assert_eq!(date, expected);
    Ok(())
}

// --- query_range ---------------------------------------------------------

#[test]
fn query_range_returns_sessions_overlapping_range() -> Result<()> {
    let dir = tempdir()?;
    let store = Store::new(&dir.path().join("test.db"))?;

    let session = parser::Session {
        start: "2026-02-04T10:00:00Z".parse().unwrap(),
        end: "2026-02-04T10:30:00Z".parse().unwrap(),
        duration: TimeDelta::seconds(1800),
        project: "/work/test".to_string(),
        input_tokens: 100,
        output_tokens: 50,
        cache_creation_input_tokens: 200,
        cache_read_input_tokens: 300,
    };
    store.upsert("proj/session-1.jsonl", &session)?;

    let start: DateTime<Utc> = "2026-02-04T00:00:00Z".parse().unwrap();
    let end: DateTime<Utc> = "2026-02-05T00:00:00Z".parse().unwrap();
    let results = store.query_range(start, end)?;

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].project, "/work/test");
    assert_eq!(results[0].start, session.start);
    assert_eq!(results[0].end, session.end);
    assert_eq!(results[0].duration, TimeDelta::seconds(1800));
    assert_eq!(results[0].input_tokens, 100);
    assert_eq!(results[0].output_tokens, 50);
    assert_eq!(results[0].cache_creation_input_tokens, 200);
    assert_eq!(results[0].cache_read_input_tokens, 300);
    Ok(())
}

#[test]
fn query_range_includes_session_starting_before_range() -> Result<()> {
    let dir = tempdir()?;
    let store = Store::new(&dir.path().join("test.db"))?;

    // Starts 30m before range, ends 30m into range
    let session = make_session("2026-02-03T23:30:00Z", "2026-02-04T00:30:00Z", 3600);
    store.upsert("proj/session-1.jsonl", &session)?;

    let start: DateTime<Utc> = "2026-02-04T00:00:00Z".parse().unwrap();
    let end: DateTime<Utc> = "2026-02-05T00:00:00Z".parse().unwrap();
    let results = store.query_range(start, end)?;

    assert_eq!(results.len(), 1);
    Ok(())
}

#[test]
fn query_range_includes_session_ending_after_range() -> Result<()> {
    let dir = tempdir()?;
    let store = Store::new(&dir.path().join("test.db"))?;

    // Starts 30m before range end, ends 30m after range end
    let session = make_session("2026-02-04T23:30:00Z", "2026-02-05T00:30:00Z", 3600);
    store.upsert("proj/session-1.jsonl", &session)?;

    let start: DateTime<Utc> = "2026-02-04T00:00:00Z".parse().unwrap();
    let end: DateTime<Utc> = "2026-02-05T00:00:00Z".parse().unwrap();
    let results = store.query_range(start, end)?;

    assert_eq!(results.len(), 1);
    Ok(())
}

#[test]
fn query_range_excludes_session_entirely_outside_range() -> Result<()> {
    let dir = tempdir()?;
    let store = Store::new(&dir.path().join("test.db"))?;

    // Session entirely on Feb 3, query is Feb 4
    let session = make_session("2026-02-03T10:00:00Z", "2026-02-03T10:30:00Z", 1800);
    store.upsert("proj/session-1.jsonl", &session)?;

    let start: DateTime<Utc> = "2026-02-04T00:00:00Z".parse().unwrap();
    let end: DateTime<Utc> = "2026-02-05T00:00:00Z".parse().unwrap();
    let results = store.query_range(start, end)?;

    assert_eq!(results.len(), 0);
    Ok(())
}

#[test]
fn query_range_midnight_spanning_session_in_both_adjacent_days() -> Result<()> {
    let dir = tempdir()?;
    let store = Store::new(&dir.path().join("test.db"))?;

    // Spans midnight: starts Feb 3 23:30, ends Feb 4 00:30
    let session = make_session("2026-02-03T23:30:00Z", "2026-02-04T00:30:00Z", 3600);
    store.upsert("proj/session-1.jsonl", &session)?;

    let feb3: DateTime<Utc> = "2026-02-03T00:00:00Z".parse().unwrap();
    let feb4: DateTime<Utc> = "2026-02-04T00:00:00Z".parse().unwrap();
    let feb5: DateTime<Utc> = "2026-02-05T00:00:00Z".parse().unwrap();

    let results = store.query_range(feb3, feb4)?;
    assert_eq!(results.len(), 1, "session visible from Feb 3");

    let results = store.query_range(feb4, feb5)?;
    assert_eq!(results.len(), 1, "session visible from Feb 4");
    Ok(())
}

#[test]
fn query_range_empty_range_returns_empty() -> Result<()> {
    let dir = tempdir()?;
    let store = Store::new(&dir.path().join("test.db"))?;

    // Session on Feb 4, query is Feb 5 — nothing there
    let session = make_session("2026-02-04T10:00:00Z", "2026-02-04T10:30:00Z", 1800);
    store.upsert("proj/session-1.jsonl", &session)?;

    let start: DateTime<Utc> = "2026-02-05T00:00:00Z".parse().unwrap();
    let end: DateTime<Utc> = "2026-02-06T00:00:00Z".parse().unwrap();
    let results = store.query_range(start, end)?;

    assert_eq!(results.len(), 0);
    Ok(())
}
