use anyhow::{Context, Result};
use chrono::{DateTime, Local, NaiveDate, TimeDelta, Utc};
use rusqlite::{Connection, OptionalExtension};
use std::path::Path;

use crate::parser;

pub struct Store {
    conn: Connection,
}

impl Store {
    pub fn new(path: &Path) -> Result<Store> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             CREATE TABLE IF NOT EXISTS sessions (
                 source_path                  TEXT    PRIMARY KEY,
                 project                      TEXT    NOT NULL,
                 date                         TEXT    NOT NULL,
                 start_time                   TEXT    NOT NULL,
                 end_time                     TEXT    NOT NULL,
                 duration_seconds             INTEGER NOT NULL,
                 input_tokens                 INTEGER NOT NULL DEFAULT 0,
                 output_tokens                INTEGER NOT NULL DEFAULT 0,
                 cache_creation_input_tokens  INTEGER NOT NULL DEFAULT 0,
                 cache_read_input_tokens      INTEGER NOT NULL DEFAULT 0
             );
             CREATE TABLE IF NOT EXISTS synced_days (
                 date         TEXT NOT NULL,
                 workspace_id TEXT NOT NULL,
                 PRIMARY KEY (date, workspace_id)
             );
             CREATE TABLE IF NOT EXISTS synced_entries (
                 date              TEXT NOT NULL,
                 workspace_id      TEXT NOT NULL,
                 project_id        TEXT NOT NULL,
                 clockify_entry_id TEXT NOT NULL,
                 PRIMARY KEY (date, workspace_id, project_id)
             );",
        )
        .context("initializing database")?;
        Ok(Store { conn })
    }

    pub fn upsert(&self, source_path: &str, session: &parser::Session) -> Result<()> {
        let date = session
            .start
            .with_timezone(&Local)
            .format("%Y-%m-%d")
            .to_string();
        let start_time = session.start.format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let end_time = session.end.format("%Y-%m-%dT%H:%M:%SZ").to_string();

        self.conn
            .execute(
                "INSERT OR REPLACE INTO sessions (
                     source_path, project, date, start_time, end_time,
                     duration_seconds, input_tokens, output_tokens,
                     cache_creation_input_tokens, cache_read_input_tokens
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                rusqlite::params![
                    source_path,
                    session.project,
                    date,
                    start_time,
                    end_time,
                    session.duration.num_seconds(),
                    session.input_tokens as i64,
                    session.output_tokens as i64,
                    session.cache_creation_input_tokens as i64,
                    session.cache_read_input_tokens as i64,
                ],
            )
            .context("upserting session")?;
        Ok(())
    }

    pub fn query_range(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<parser::Session>> {
        let start_str = start.format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let end_str = end.format("%Y-%m-%dT%H:%M:%SZ").to_string();

        let mut stmt = self.conn.prepare(
            "SELECT project, start_time, end_time, duration_seconds,
                    input_tokens, output_tokens,
                    cache_creation_input_tokens, cache_read_input_tokens
             FROM sessions
             WHERE start_time < ?1 AND end_time >= ?2",
        ).context("preparing query_range")?;

        let rows = stmt.query_map(rusqlite::params![end_str, start_str], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, i64>(6)?,
                row.get::<_, i64>(7)?,
            ))
        }).context("querying sessions")?;

        let mut sessions = Vec::new();
        for row in rows {
            let (project, start_time, end_time, duration_secs, input, output, cache_create, cache_read) = row?;
            sessions.push(parser::Session {
                start: start_time.parse().context("parsing start_time")?,
                end: end_time.parse().context("parsing end_time")?,
                duration: TimeDelta::seconds(duration_secs),
                project,
                input_tokens: input as u64,
                output_tokens: output as u64,
                cache_creation_input_tokens: cache_create as u64,
                cache_read_input_tokens: cache_read as u64,
            });
        }

        Ok(sessions)
    }

    pub fn earliest_session_date(&self) -> Result<Option<NaiveDate>> {
        let result: Option<String> = self.conn.query_row(
            "SELECT start_time FROM sessions ORDER BY start_time ASC LIMIT 1",
            [],
            |row| row.get(0),
        ).optional().context("querying earliest session")?;

        match result {
            Some(start_time_str) => {
                let start_time: DateTime<Utc> = start_time_str.parse()
                    .context("parsing earliest start_time")?;
                Ok(Some(start_time.date_naive()))
            }
            None => Ok(None),
        }
    }

    // --- sync tracking ----------------------------------------------------

    pub fn is_day_synced(&self, date: &str, workspace_id: &str) -> Result<bool> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM synced_days WHERE date = ?1 AND workspace_id = ?2",
            rusqlite::params![date, workspace_id],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    pub fn mark_day_synced(&self, date: &str, workspace_id: &str) -> Result<()> {
        self.conn
            .execute(
                "INSERT INTO synced_days (date, workspace_id) VALUES (?1, ?2)",
                rusqlite::params![date, workspace_id],
            )
            .context("marking day synced")?;
        Ok(())
    }

    pub fn is_entry_synced(&self, date: &str, workspace_id: &str, project_id: &str) -> Result<bool> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM synced_entries WHERE date = ?1 AND workspace_id = ?2 AND project_id = ?3",
            rusqlite::params![date, workspace_id, project_id],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    pub fn mark_entry_synced(&self, date: &str, workspace_id: &str, project_id: &str, clockify_entry_id: &str) -> Result<()> {
        self.conn
            .execute(
                "INSERT INTO synced_entries (date, workspace_id, project_id, clockify_entry_id) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![date, workspace_id, project_id, clockify_entry_id],
            )
            .context("marking entry synced")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
