use anyhow::{Context, Result};
use chrono::Local;
use rusqlite::Connection;
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
}

#[cfg(test)]
mod tests;
