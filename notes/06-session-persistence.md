---
version: 0.2.0
updated: 2026-02-04
---
# Feature: Session Persistence

**Status:** Planning\
**Started:** 2026-02-04\
**Completed:** —

---

## Problem

Claude's session JSONL files in `~/.claude/projects/` are internal implementation details — not guaranteed to persist long-term. The tracker scans them on every refresh and discards everything at shutdown. There is no durable record of past usage, making it impossible to answer questions like "how much time did I spend on project X this week?" or to aggregate usage across days.

---

## Proposed Solution

Persist every scanned session into a local SQLite database via idempotent upsert on each refresh cycle. The upsert is wired synchronously into the existing scan loop — no new threads, channels, or loading state. The scan already blocks the main thread every 2 seconds; adding a handful of SQLite upserts doesn't change observable behavior.

### Integration Points

- **scanner.rs** — already finds all session files; unchanged
- **parser.rs** — already produces `Session` structs; unchanged
- **main.rs** — minimal change: `load_sessions` gains a `&Store` parameter. No new threads, flags, or state.
- **store.rs** _(new)_ — SQLite connection lifecycle, schema, upsert logic

### Key Behaviors

- `main` owns the `Store` (and its `Connection`) for its lifetime. Passes a reference into `load_sessions`.
- On every refresh cycle, `load_sessions` scans **all** session files, parses, upserts all, then filters to today and returns that subset to the TUI. Upsert before filter — historical files get persisted even on a day with no today sessions.
- Upsert keyed by relative file path (`<hash>/<session-id>.jsonl`). Scanner returns absolute paths; `strip_prefix(projects_dir)` produces the key before upsert. Rescanning an unchanged file is a no-op.
- Active sessions (files Claude is still writing to) get their row overwritten with the latest parsed state on each scan — naturally stabilizes once the session closes.
- Early-exit on empty today is unchanged: it fires after the upsert, so all historical files are already persisted before the check.

### Schema

```sql
CREATE TABLE IF NOT EXISTS sessions (
    source_path TEXT PRIMARY KEY,                          -- <hash>/<session-id>.jsonl
    project TEXT NOT NULL,
    date TEXT NOT NULL,                                    -- local calendar date (YYYY-MM-DD)
    start_time TEXT NOT NULL,                              -- ISO 8601 UTC
    end_time TEXT NOT NULL,                                -- ISO 8601 UTC
    duration_seconds INTEGER NOT NULL,
    input_tokens INTEGER NOT NULL DEFAULT 0,
    output_tokens INTEGER NOT NULL DEFAULT 0,
    cache_creation_input_tokens INTEGER NOT NULL DEFAULT 0,
    cache_read_input_tokens INTEGER NOT NULL DEFAULT 0
);
```

---

## Success Criteria

- [ ] All session files are persisted to `~/.config/claude-tracker/sessions.db` on each scan
- [ ] Repeated scans of the same files produce no duplicate rows (idempotent)
- [ ] Active sessions update correctly on subsequent scans; rows stabilize once the session closes
- [ ] TUI display and refresh behavior is unchanged from the user's perspective
- [ ] DB file and table are created automatically on first run
- [ ] Midnight-spanning sessions: `date` column and today-filter behavior are consistent (test defines the contract)

---

## Scope

### In Scope

- SQLite persistence of all parsed session data
- Synchronous upsert in the existing scan loop
- Full scan + idempotent upsert on every refresh cycle
- `rusqlite` with bundled SQLite (no system dependency)

### Out of Scope

- Querying or displaying historical data in the TUI (next feature)
- Background worker thread / async scanning (future optimization — only if profiling shows the synchronous upsert is a bottleneck)
- Data retention or cleanup policies
- Cost calculations from token fields
- Schema versioning or migration tooling (premature until we have a second schema)

---

## Important Considerations

- **Active session convergence.** A `.jsonl` file still being written to parses differently on each scan. The upsert overwrites unconditionally. This is correct: the row always reflects latest known state and stabilizes once Claude closes the session. `duration_seconds` and token totals may tick upward between scans.
- **`date` column vs `is_today` semantics.** `is_today` returns true if *either* start or end falls on the given date. But `date` is derived from `start_time` only. A session spanning midnight will show in today's TUI but be stored as yesterday. This is a real inconsistency for future date-based queries. A test will define the contract; the decision on how to resolve it can follow.
- **`source_path` derivation.** Scanner returns absolute paths. The upsert key must be the relative path under `projects/`. Produce it with `strip_prefix(projects_dir)` before calling the upsert.
- **Idle threshold applied before persist.** `assemble_session` (idle-gap exclusion) runs before the upsert, same as it does now for the TUI. `duration_seconds` in the DB already has idle time subtracted.

---

## High-Level Todo

- [ ] Add `rusqlite` dependency (bundled feature)
- [ ] Create `store` module: connection init, schema creation, upsert function
- [ ] Refactor `load_sessions`: scan → parse → upsert all (via store) → filter today → return
- [ ] Tests: idempotent upsert, schema creation, active-session update, midnight-spanning date behavior

---

## Notes & Context

### 2026-02-04 - Why SQLite over flat files

Claude's JSONL files are not a durable archive. SQLite gives us a single-file durable store that's also queryable. Future features (history views, cross-day project aggregation) will query this DB directly — that would be painful with per-day JSON snapshots or append-only flat files.

### 2026-02-04 - Why synchronous upsert, not a background thread

The scan already blocks the main thread every 2 seconds. At this data volume (dozens to low hundreds of rows), SQLite upserts add low single-digit milliseconds to an already-blocking operation. A background thread would add channels, a `scan_in_flight` flag, shutdown coordination, and a loading state — complexity that solves no observable problem. If profiling ever shows the upsert is a bottleneck, threading can be added as an optimization pass. It's not a foundational requirement.

### 2026-02-04 - Idempotency key: relative file path

Each `.jsonl` file in `~/.claude/projects/<hash>/<session-id>.jsonl` maps to exactly one session. The relative path under `projects/` is stable and unique. It's the upsert key: rescanning a persisted file is a no-op (or an overwrite with identical data). If Claude deletes the source file, the row stays in SQLite — that's the whole point of this feature. Note: the scanner returns absolute paths; `strip_prefix(projects_dir)` is required to produce the key.

### 2026-02-04 - Why full scan on every refresh, not incremental

Incremental scanning (tracking which files are new or modified) adds state and complexity. The full scan is already what the TUI does every 2 seconds — it's cheap for this data volume. The upsert cost for dozens to low hundreds of rows is negligible in SQLite. Revisit only if profiling shows a bottleneck.

---

## Reference

- Previous feature: [notes/05-token-tracking.md](05-token-tracking.md) — references this as "doc 06" in its Out of Scope
- Scanner: [src/scanner.rs](../src/scanner.rs)
- Parser + Session struct: [src/parser.rs](../src/parser.rs)
- Current event loop + config path: [src/main.rs](../src/main.rs)
- Config dir (where `sessions.db` will live): `~/.config/claude-tracker/`
