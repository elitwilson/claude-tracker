---
version: 0.1.0
updated: 2026-02-05
---
# 10b: Time Allocation Transformation

**Parent:** [10-clockify-sync.md](10-clockify-sync.md)\
**Status:** Complete

---

The core algorithm: sessions → ratio-based allocations across a full work day. Pure computation — no DB or HTTP dependencies.

**Input:** A set of sessions for a single day (project name + duration), plus sync config (work_day_start, work_day_end, project_mapping, other_project_id).

**Output:** A list of allocations — one per bucket (mapped project or Other) — each containing a Clockify project ID, start time, and end time. Empty list if zero sessions.

## Integration Points

- **Input:** `Vec<Session>` from `store.query_range()`. Each session carries a `project` path and a `duration`.
- **Config:** `SyncConfig` parsed from the `[sync]` section in config.toml.
- **Output:** `Vec<Allocation>` consumed by the sync loop (10d), which passes each to the HTTP client (10c).
- **No external dependencies.** Pure function: sessions + config → allocations.

## Key Behaviors

- Unmapped projects are summed into an "Other" bucket; each mapped project gets its own bucket
- All tracked time participates in ratio calculation
- Full work day duration is allocated regardless of actual tracked time
- Entries are contiguous from work_day_start; stacking order is arbitrary
- Zero sessions → empty output

## Success Criteria

- [x] Output durations sum to exactly one work day (work_day_end − work_day_start)
- [x] Each entry's duration is proportional to its bucket's share of total tracked time
- [x] Entries are contiguous: each starts where the previous ends; first starts at work_day_start
- [x] Zero sessions → empty output
- [x] No Other entry is produced when all time is mapped

## Important Considerations

- **Project name matching.** Config mapping uses full paths (e.g. `"/Users/etwilson/workdev/tools/claude-tracker"`). Sessions store `project` as a full path, and the config keys match exactly. This is explicit and unambiguous — no risk of name collisions.
- **Rounding.** Ratios are floating point; durations are integer seconds. Naive per-bucket rounding will cause the total to drift from the exact work day duration. The last entry absorbs the remainder to guarantee an exact sum.
- **Work day timestamps.** `work_day_start` / `work_day_end` are local times (e.g. "09:00"). They must be combined with the session date to produce `DateTime<Utc>` values for stacking. Follow the same local→UTC pattern as `Timeframe::boundaries()` in [src/main.rs](../src/main.rs).

## Todo

- [x] Implement ratio calculation: per-project duration → share of work day; unmapped projects aggregated into Other
- [x] Implement sequential stacking: contiguous start/end times from work_day_start; last entry absorbs rounding remainder
- [x] Handle edge case: zero sessions → return empty
- [x] Write tests: ratio math, stacking, single-project, Other aggregation, no-Other-when-all-mapped, zero-session

## Implementation

Implemented in [src/sync.rs:65-153](../src/sync.rs#L65-L153) as `compute_allocations()`.

**Algorithm:**
1. Match session project path directly against config mapping keys (full path)
2. Group sessions into buckets by mapped project ID or "Other"
3. Calculate ratios and allocate full work day proportionally
4. Sort by project_id alphabetically
5. Stack allocations contiguously from work_day_start
6. Last entry absorbs rounding remainder

**Tests:** 8 tests covering all key behaviors in [src/sync/tests.rs](../src/sync/tests.rs)

---

## Reference

- Parent: [10-clockify-sync.md](10-clockify-sync.md)
- Session store (input type): [src/store.rs](../src/store.rs)
- Timeframe boundaries (local→UTC pattern): [src/main.rs](../src/main.rs)
- Sync loop (output consumer): [10d_sync-loop.md](10d_sync-loop.md)
