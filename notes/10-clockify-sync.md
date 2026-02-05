---
version: 0.1.0
updated: 2026-02-04
---
# Feature: Clockify Sync

**Status:** Planning\
**Started:** 2026-02-04

---

## Problem

Clockify requires daily time entries per project, but logging them manually is tedious and error-prone. Claude-tracker already captures granular session data (project, start, end, duration) in SQLite. The gap is getting that data into Clockify in the shape it expects: hours per project per day, not raw IDE sessions.

---

## Proposed Solution

A `claude-tracker sync` subcommand that reads session data from the local DB, transforms it into normalized daily allocations, and POSTs them to Clockify. Idempotent — run it whenever, it fills in what's missing.

### The transformation

Raw session durations are a proxy for relative time spent, not actual billable hours. The sync layer does not upload raw durations. For each workday:

1. Pull all sessions for that day from SQLite
2. Sum duration per project. Unmapped projects are aggregated into an "Other" bucket.
3. Compute each bucket's share of total tracked time (ratios only)
4. Allocate the full work day duration across buckets using those ratios
5. Stack entries sequentially from work day start (order is arbitrary)
6. POST one entry per bucket to Clockify (mapped project ID or other_project_id)

Days with zero sessions are skipped and not marked synced.

Example: 3h tracked on NPR, 1h on claude-tracker, 1h on unmapped projects → ratios 3:1:1 → Clockify gets 4.8h NPR, 1.6h claude-tracker, 1.6h Other (8h work day).

### Config

New fields in `config.toml`:

```toml
[sync]
workspace_id = "..."
other_project_id = "..."   # Clockify project ID for unmapped projects
work_day_start = "09:00"   # local time
work_day_end = "17:00"     # local time

[sync.project_mapping]
# local project full path → Clockify project ID
"/Users/etwilson/workdev/tools/claude-tracker" = "65b2d73e06de527a7ed67403"
```

Work day is M-F only. Start/end are local times; conversion to UTC happens at POST time.

### Idempotency

`sync` tracks uploaded days in SQLite via two tables:
- `synced_days` (date, workspace_id) — day-complete marker. Written only after all entries for the day succeed.
- `synced_entries` (date, workspace_id, project_id, clockify_entry_id) — per-entry audit trail.

On retry, complete days are skipped. Incomplete days skip already-posted entries (via `synced_entries`) to avoid duplicates. See [10a](10a_idempotency-design.md) for details.

### Integration Points

- **`config.toml`** — new `[sync]` section with workspace_id, work day bounds, project mapping
- **`src/store.rs`** — `query_range()` already exists; reused to pull sessions per day. New tables: `synced_days`, `synced_entries` for idempotency tracking.
- **`src/secrets.rs`** — `get_secret("clockify_api_key")` already built
- **`src/main.rs`** — new `sync` subcommand route alongside existing `setup`
- **`src/sync.rs`** (new) — transformation (10b), HTTP client (10c), sync loop (10d)
- **`Cargo.toml`** — add `ureq`

### Key Behaviors

- Unmapped projects are aggregated into an "Other" bucket (configured via `other_project_id`); all tracked time participates in ratio calculation
- Full work day duration is always allocated, regardless of how much time was actually tracked
- Entries stacked sequentially from work day start; order is arbitrary
- Description: "Development" (static string)
- M-F only; weekends skipped

---

## Success Criteria

- [ ] `claude-tracker sync` creates Clockify entries for each mapped project on unsynced workdays
- [ ] Allocated hours per day sum to exactly one work day duration
- [ ] Running `sync` twice does not create duplicate entries
- [ ] Unmapped projects are aggregated into Other; days with zero sessions are skipped
- [ ] Config changes (new mapping, different work day hours) take effect on next sync

---

## Scope

### In Scope

- 1:1 project mapping via config.toml
- Ratio-based time allocation normalized to work day duration
- Idempotent on-demand sync
- Sequential entry stacking from work day start

### Out of Scope

- Automatic or scheduled sync
- Deleting or updating existing Clockify entries
- Multi-workspace support
- Granular session-level sync to Clockify

---

## Important Considerations

- **Idempotency mechanism decided.** Two-table design with partial-failure recovery. See [10a](10a_idempotency-design.md).
- **Unmapped projects** are aggregated into an "Other" bucket mapped to `other_project_id`. Days with zero sessions are skipped and not marked synced.
- **`description` field** — Set to "Development". If Clockify rejects it, revisit.
- **Time zone handling** — work day start/end are local times. Sessions in SQLite are stored in UTC. Boundary conversion must be correct or days will bleed into each other.

---

## High-Level Todo

- [x] Decide idempotency mechanism
- [ ] Extend config.toml: workspace_id, other_project_id, work_day_start/end, project_mapping
- [ ] Add `ureq` to Cargo.toml
- [x] Implement transformation: sessions → ratio-based daily allocations
- [ ] Implement sync: read config + DB, transform, POST to Clockify
- [ ] Wire `sync` subcommand into main.rs
- [ ] Test: single day sync, verify entries appear in Clockify
- [ ] Test: re-run sync, verify no duplicates
- [ ] Test: multiple mapped projects, ratios are correct

---

## Notes & Context

### 2026-02-04 - Design decisions

- **Project mapping is manual 1:1.** Clockify project organization (e.g. "NPR" vs "NPR Modernization") is the user's concern, not ours. We map what we're told to map.
- **Raw durations are intentionally not uploaded.** Claude-tracker is a proxy for how time is spent, not a stopwatch. The ratio-based transformation is the core insight — it lets the user skip manual logging without exposing granular tracking to anyone looking at Clockify.
- **Idempotency deferred from initial design.** Important enough to get right, not important enough to block the rest of the plan. Decide before writing sync loop code.

---

## Reference

- Sub-docs: [10a — Idempotency](10a_idempotency-design.md) · [10b — Transformation](10b_transformation.md) · [10c — HTTP Client](10c_clockify-http-client.md) · [10d — Sync Loop](10d_sync-loop.md)
- Spike (auth + API shape proven): [notes/09-clockify-spike.md](09-clockify-spike.md)
- Previous feature: [notes/08-keychain-secret-storage.md](08-keychain-secret-storage.md)
- Secret reading: [src/secrets.rs](../src/secrets.rs)
- Session store: [src/store.rs](../src/store.rs)
- Config: [src/main.rs](../src/main.rs)
