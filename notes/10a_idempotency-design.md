---
version: 0.1.0
updated: 2026-02-04
---
# 10a: Idempotency Design

**Parent:** [10-clockify-sync.md](10-clockify-sync.md)\
**Status:** Decided — pending implementation\
**Prerequisite for:** 10c

---

How `sync` tracks which days have been uploaded. Running `sync` multiple times must not create duplicates.

## Decisions

- **Key:** (date, workspace_id) composite.
- **Tables:** Two tables in SQLite:
  - `synced_days` (date, workspace_id) — day-complete marker. Written only after all entries for the day POST successfully.
  - `synced_entries` (date, workspace_id, project_id, clockify_entry_id) — per-entry audit trail. Written as each POST succeeds.
- **Partial failure recovery:** On retry, skip days already in `synced_days`. For days not yet complete, skip individual entries already in `synced_entries` to avoid duplicate POSTs.
- **Zero-session days:** Skipped entirely. Not marked synced.

## Integration Points

- **`src/store.rs`** — two new tables (`synced_days`, `synced_entries`) created in `Store::new()` alongside the existing `sessions` table. New methods on `Store`: `is_day_synced`, `is_entry_synced`, `mark_entry_synced`, `mark_day_synced`.
- **`src/sync.rs`** — the sync loop (10d) calls these methods around each POST.

## Success Criteria

- [ ] Re-running sync on an already-synced day produces zero Clockify POSTs
- [ ] Each posted entry's Clockify ID is stored in `synced_entries`
- [ ] A day is marked complete in `synced_days` only after all entries for that day POST successfully
- [ ] A partially-failed day retries on next sync; already-posted entries are skipped, remaining entries are attempted

## Important Considerations

- **Two-stage commit order matters.** Entry rows in `synced_entries` are written per-POST as each succeeds. The `synced_days` row is written last, only after all entries for the day succeed. If the process crashes between, the day is incomplete — retry skips already-posted entries via `synced_entries` and attempts the rest. Writing `synced_days` first would silently lose any entries that hadn't posted yet.
- **Composite PK on `synced_entries` is (date, workspace_id, project_id).** A given project can only have one entry per day. This is the per-entry duplicate guard on retry.

## Todo

- [ ] Add `synced_days` and `synced_entries` tables to `Store::new()` in store.rs
- [ ] Implement Store methods: `is_day_synced`, `is_entry_synced`, `mark_entry_synced`, `mark_day_synced`
- [ ] Test: sync a day, verify marked complete; re-run, verify no POSTs
- [ ] Test: partial failure — verify retry skips already-posted entries, completes the rest

---

## Reference

- Parent: [10-clockify-sync.md](10-clockify-sync.md)
- Session store (where tables live): [src/store.rs](../src/store.rs)
- Sync loop (calls into this): [10d_sync-loop.md](10d_sync-loop.md)
