---
version: 0.1.0
updated: 2026-02-04
---
# 10c: Clockify HTTP Client

**Parent:** [10-clockify-sync.md](10-clockify-sync.md)\
**Status:** Todo

---

POST time entries to Clockify. Takes an allocation from the transformation (10b) and pushes it. Returns the created entry ID for the idempotency tracker (10a).

## Integration Points

- **Input:** A single allocation (Clockify project ID, start, end) plus workspace_id from config. Description is always "Development".
- **Auth:** `X-Api-Key` header with raw API key value (no Bearer prefix). Retrieved via `secrets::get_secret("clockify_api_key")`.
- **Endpoint:** `POST https://api.clockify.me/api/v1/workspaces/{workspaceId}/time-entries`
- **Output:** `Result<String>` — the Clockify entry ID on success. Errors propagate to caller.
- **HTTP client:** `ureq` (already in Cargo.toml).

## Key Behaviors

- One allocation → one POST → one entry ID returned
- Description is always "Development" (static)
- Times sent in UTC ISO 8601 format (e.g. `2026-02-04T15:00:00Z`)
- No retry logic — errors propagate as `Result` to the sync loop

## Success Criteria

- [ ] POST creates a time entry visible in the Clockify UI
- [ ] Returns the created entry ID on success
- [ ] Returns a clear error on bad project ID, expired/invalid key, or network failure

## Important Considerations

- **Request shape is proven.** See [09-clockify-spike.md](09-clockify-spike.md) for the exact working request body and response format. Follow it exactly — do not guess.
- **`description` field.** Sending "Development" as a static string. If Clockify rejects the POST, this is the first thing to check.
- **Times are already UTC.** The transformation (10b) produces UTC timestamps. Pass through directly — no conversion needed here.

## Todo

- [ ] Implement POST to `/api/v1/workspaces/{workspaceId}/time-entries`
- [ ] Use description "Development"; revisit if Clockify rejects
- [ ] Return created entry ID on success
- [ ] Test: POST a single entry, verify it appears in Clockify
- [ ] Test: error handling — bad project ID, expired key, network failure

---

## Reference

- Parent: [10-clockify-sync.md](10-clockify-sync.md)
- Spike (proven request shape): [09-clockify-spike.md](09-clockify-spike.md)
- Secret reading: [src/secrets.rs](../src/secrets.rs)
- Idempotency tracker: [10a_idempotency-design.md](10a_idempotency-design.md)
