use super::*;
use anyhow::Result;
use chrono::{Local, TimeDelta};
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
