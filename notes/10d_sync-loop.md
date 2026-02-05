---
version: 0.1.0
updated: 2026-02-05
---
# 10d: Sync Loop

**Parent:** [10-clockify-sync.md](10-clockify-sync.md)\
**Status:** Complete\
**Depends on:** 10a, 10b, 10c

---

The orchestration layer. Iterates over unsynced workdays, transforms sessions into allocations, POSTs them, and marks days complete. Ties 10a (tracking), 10b (transformation), and 10c (HTTP) together.

## How It Works

For each workday (M-F) in the sync date range, up through yesterday:

1. Check `synced_days` — skip if day is already complete (10a)
2. Pull sessions for that day via `store.query_range()`
3. If zero sessions, skip without marking synced
4. Transform sessions → allocations (10b)
5. For each allocation: check `synced_entries` (10a); if not already posted, POST via 10c; record returned entry ID in `synced_entries`
6. After all allocations for the day succeed, mark day complete in `synced_days` (10a)

## Design Decisions

**Sync date range:** Start from the earliest session date in the DB. Sync everything that's not already synced. Simple and complete — no configuration needed.

**Error handling:** Stop on first error. Simpler to reason about, and because the sync is idempotent, subsequent runs will fill in gaps after the error is resolved.

## Integration Points

- **10a** — check and record sync state (day-level and entry-level)
- **10b** — transform sessions → allocations
- **10c** — POST each allocation, receive entry ID
- **`store.query_range()`** — pull sessions per day
- **`main.rs`** — wired as `claude-tracker sync` subcommand alongside existing `setup`

## Success Criteria

- [x] All unsynced workdays in the date range are processed
- [x] Already-synced days are skipped (zero duplicate POSTs)
- [x] Zero-session days are skipped without being marked synced
- [x] A partial failure on a day is recoverable on next run (per 10a)
- [x] Open questions (date range, error handling) are decided and documented

## Important Considerations

- **This is the only layer that knows about date ranges and weekday filtering.** 10a, 10b, 10c all operate on a single day. The loop is responsible for iterating, skipping weekends, and determining which days need syncing.
- **Today is excluded.** The work day isn't over yet — end boundary is yesterday.
- **Time zone boundary for "a day."** Sessions are stored in UTC. A workday is defined by local work_day_start/end. The loop must convert local day boundaries to UTC before calling `query_range()` — same pattern as `Timeframe::boundaries()` in [src/main.rs](../src/main.rs).

## Implementation

- [x] Implemented `run_sync()` in [src/sync.rs](../src/sync.rs)
- [x] Added `Store::earliest_session_date()` to find sync start date
- [x] Added `is_weekday()` helper for Mon-Fri filtering
- [x] Wired as `claude-tracker sync` subcommand in [src/main.rs](../src/main.rs)
- [x] Unit test for weekday helper in [src/sync/tests.rs](../src/sync/tests.rs)
- [x] Integration test scaffolded (marked `#[ignore]`, requires manual testing)

## Key Implementation Details

- **Date iteration:** Loops from earliest session date to yesterday, incrementing by one day
- **Weekday filtering:** Uses chrono's `Weekday` enum to check Mon-Fri
- **Idempotency:** Checks both day-level (`is_day_synced`) and entry-level (`is_entry_synced`) before POSTing
- **Progress output:** Prints real-time progress for each day synced and summary at end
- **Error handling:** Stops on first error (subsequent runs will resume from where it left off)

---

## Reference

- Parent: [10-clockify-sync.md](10-clockify-sync.md)
- Idempotency: [10a_idempotency-design.md](10a_idempotency-design.md)
- Transformation: [10b_transformation.md](10b_transformation.md)
- HTTP client: [10c_clockify-http-client.md](10c_clockify-http-client.md)
- Session store: [src/store.rs](../src/store.rs)
- Subcommand wiring: [src/main.rs](../src/main.rs)
