---
version: 0.1.0
updated: 2026-02-04
---
# Feature: Timeframe Toggle

**Status:** Complete\
**Started:** 2026-02-04\
**Completed:** 2026-02-04

---

## Problem

The TUI only shows today's data. The DB (added in doc 06) has the full history, but nothing queries it. A developer checking in on a quiet morning — or reviewing a week of work — has no way to see past usage. The early exit on "no sessions today" compounds this: if today is empty, the TUI never starts and historical data is completely inaccessible.

---

## Proposed Solution

Add a timeframe toggle (`t` key) that cycles the existing project table through three windows: Today, Last 7 days, Last 30 days. The table structure, columns, and aggregation logic are identical across all three — only the data source and labels change.

- **Today:** After scan + upsert, reads from the DB via `query_range` with today's boundaries. Same code path as the other two windows.
- **Last 7 days / Last 30 days:** After the scan + upsert completes (keeping the DB current), the view reads from the DB via a range query instead of using the scan output.
- **Early exit removed.** The TUI always starts. An empty window shows an empty table with zero totals; the user can toggle to a window that has data.

### Integration Points

- **store.rs** — new `query_range` method. Takes UTC start/end boundaries, returns `Vec<Session>` via an overlap check on `start_time`/`end_time`. The existing `aggregate_sessions` consumes the result unchanged.
- **main.rs** — `Timeframe` enum + cycle logic on `t` press. `load_sessions` becomes pure ingestion (scan + upsert, no return). All timeframes read via `store.query_range` after each refresh. `render` receives the current timeframe label. Totals line and footer update accordingly. Early exit removed. `is_today` removed.

### Key Behaviors

- `t` cycles: Today → Last 7 days → Last 30 days → Today (wraps)
- "Last 7 days" = today + 6 days back. "Last 30 days" = today + 29 days back. Both inclusive of today.
- Date boundaries are local-time concepts. Rust computes start-of-first-day and start-of-tomorrow in local time, converts to UTC, passes as SQL params. The query itself is simple: `WHERE start_time < ?end AND end_time >= ?start`
- The green dot (most recent activity) works unchanged — marks the project with the latest `end_time` in the current window
- Refresh cycle: scan → upsert → query_range(current timeframe) → aggregate → render
- `t` press triggers an immediate re-query (does not wait for next refresh tick)

### Timeframe boundaries

| Window       | range_start (local)          | range_end (local)    |
|--------------|------------------------------|----------------------|
| Today        | start of today               | start of tomorrow    |
| Last 7 days  | start of (today − 6 days)    | start of tomorrow    |
| Last 30 days | start of (today − 29 days)   | start of tomorrow    |

---

## Success Criteria

- [x] `t` key cycles through Today / Last 7 days / Last 30 days
- [x] Each window shows correct aggregated data from the DB
- [x] Midnight-spanning sessions appear in both adjacent days' windows
- [x] Empty window renders cleanly (empty table, zero totals)
- [x] Early exit removed: TUI starts even with no today sessions
- [x] Footer hint updated to show `t`
- [x] Totals label reflects current window ("Today:" / "Last 7 days:" / "Last 30 days:")

---

## Scope

### In Scope

- Timeframe toggle (`t` key, 3 windows)
- `Store::query_range` — overlap query on start_time/end_time
- Remove early exit; empty-state rendering
- UI label updates (totals line, footer)

### Out of Scope

- Arbitrary date range selection (future)
- Trend or sparkline visualization (future)
- Per-day breakdown within a range (future)
- Cost calculations from token fields (doc 06 out of scope, still deferred)

---

## Important Considerations

- **`date` column not used for range queries.** The `date` column is derived from `start_time` only and has the midnight-spanning inconsistency noted in doc 06. Range queries use `start_time`/`end_time` overlap instead. The `date` column is left alone — still written by the upsert, not relied upon here.
- **Today reads from the DB.** All three windows use `query_range`. `load_sessions` becomes pure ingestion (scan + upsert, no filter/return). The `is_today` filter is removed — dead code once Today queries the DB. This eliminates the two-path divergence risk.
- **Empty state in render.** Removing the early exit means `render` must handle empty `summaries`. The table height constraint (`summaries.len() + 3`) becomes 3 (borders + header only). All totals are zero. ratatui's Table widget handles an empty row list — no special case needed.
- **`t` press is a view change, not a disk refresh.** It does not reset the spinner or re-scan files. It re-queries the DB with the new range and re-renders immediately.

---

## High-Level Todo

- [x] Add `Timeframe` enum and cycle logic
- [x] Add `Store::query_range` — overlap query on start_time/end_time
- [x] Wire timeframe into event loop: `t` press, immediate re-query, label updates
- [x] Remove early exit; verify empty state renders correctly
- [x] Restructure `load_sessions` → pure ingestion (scan + upsert, no filter/return)
- [x] Remove `is_today` + its tests
- [x] Update footer hint
- [x] Tests: query_range correctness, midnight-spanning, empty range, timeframe cycling

---

## Notes & Context

### 2026-02-04 - Why toggle, not a separate view

Early in planning this was framed as a "history view" you navigate into. The toggle won.
The table structure is identical across all three windows — same columns, same aggregation,
same green dot logic. A second view would add layout code, view-state management, and
navigation for no information gain. The toggle is one enum, one key, one label swap.

### 2026-02-04 - Why overlap query, not the `date` column

The `date` column was added in doc 06 as a convenience for the today filter. It's derived
from `start_time` only. A session spanning midnight (start yesterday, end today) is stored
as yesterday but appears in today's TUI via `is_today` (which checks both start and end).
Querying by `date` for a range would miss or double-count these sessions. The overlap check
on `start_time`/`end_time` is the correct general solution and avoids touching the schema.

### 2026-02-04 - Token column definitions settled before this feature

Doc 06 stored all four token types. The TUI previously lumped them into one "Input" column
via `total_input()` (input + cache_creation + cache_read), which inflated the number
dramatically due to cache reads. Fixed before this feature: Input = input_tokens +
cache_creation_input_tokens (novel input), Cache = cache_read_input_tokens. The DB is
unchanged — all four fields are available for future cost calculations.

### 2026-02-04 - Why Today also queries the DB

Originally planned as two paths: Today uses live-scan output, 7d/30d query the DB.
Reversed during planning. The scan + upsert keeps the DB current on every refresh
regardless of timeframe. Querying the DB for Today after upsert gives identical results
to filtering scan output, with one fewer code path. `is_today` becomes dead code and is
removed. An mtime-based incremental scan optimization was considered and explicitly
deferred — Rust handles the brute-force full scan without measurable cost at any
realistic session file count.

---

## Reference

- Previous feature: [notes/06-session-persistence.md](06-session-persistence.md)
- Store: [src/store.rs](../src/store.rs)
- Main (TUI + event loop): [src/main.rs](../src/main.rs)
- Parser (Session struct): [src/parser.rs](../src/parser.rs)
