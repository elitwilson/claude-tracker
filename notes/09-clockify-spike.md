---
version: 0.1.0
updated: 2026-02-04
---
# Spike: Clockify API Time Entry

**Status:** Complete\
**Started:** 2026-02-04\
**Completed:** 2026-02-04

---

## Goal

Answer one question: **Can we create a Clockify time entry using a personal API key?**

If yes, capture the exact auth mechanism, request shape, and IDs needed. If no, understand why and what the alternative is.

---

## What We're Proving

- Personal API key auth is sufficient to hit the Clockify API
- We can POST a time entry to a known workspace + project
- We understand the request/response shape well enough to wire real data later

---

## Approach

Proven via curl against the live API. The `sync` subcommand was not needed — a single POST confirmed auth, endpoint, and body shape in one shot. IDs were discovered via the network tab (workspace) and `GET /workspaces/{id}/projects` (project list).

---

## IDs Used

- Workspace ID: `5ff748b4abb0e16bed500885` (Scientific Programming and Innovation)
- Project ID: `65b2d73e06de527a7ed67403` (NPR)
- Test entry: 2026-02-04T15:00:00Z → 15:30:00Z, description "spike test entry"

---

## What We Keep After the Spike

- `ureq` dependency
- The working request structure (headers, endpoint, body shape)
- Knowledge of required IDs and where they live in the API

## What We Throw Away

- The hardcoded values
- The `sync` subcommand itself (becomes a stub or is rewritten for the real feature)

---

## Success Criteria

- [x] A time entry was created visible in the Clockify UI
- [x] We know the exact auth header format that worked
- [x] We know the request body shape well enough to document it

---

## Findings

**Auth:** `X-Api-Key` header with the raw API key value. No Bearer prefix, no encoding.

**Endpoint:** `POST https://api.clockify.me/api/v1/workspaces/{workspaceId}/time-entries`

**Request body (minimum that worked):**
```json
{
  "projectId": "...",
  "start": "2026-02-04T15:00:00Z",
  "end": "2026-02-04T15:30:00Z",
  "description": "spike test entry"
}
```

**Response:**
```json
{
  "id": "6983e379c582c61564974391",
  "description": "spike test entry",
  "projectId": "65b2d73e06de527a7ed67403",
  "userId": "6239e4df3e89a17fb524819a",
  "billable": true,
  "timeInterval": {
    "start": "2026-02-04T15:00:00Z",
    "end": "2026-02-04T15:30:00Z",
    "duration": "PT30M"
  },
  "workspaceId": "5ff748b4abb0e16bed500885"
}
```

**Open question:** Whether `description` is required or optional — not tested without it.

**Project listing:** `GET /workspaces/{workspaceId}/projects` returns all projects. Each has `id`, `name`, `archived`, etc.

---

## Scope

### In Scope

- Auth: personal API key → Clockify API
- One POST: create a single hardcoded time entry
- Capturing the working request/response for future reference

### Out of Scope

- Reading from our session store
- Mapping claude-tracker projects → Clockify projects
- Idempotency or duplicate detection
- Error handling beyond "did it work"
- Deleting or updating time entries

---

## Dependencies

- `ureq` — synchronous HTTP client (new dep, to add)
- `serde_json` — already in Cargo.toml
- `secrets::get_secret("clockify_api_key")` — already built ([src/secrets.rs](../src/secrets.rs))

---

## High-Level Todo

- [x] Gather workspace + project IDs
- [x] Prove auth + POST via curl
- [x] Document the working request shape here
- [x] Clean up: delete the test entry from Clockify

---

## Reference

- Previous feature: [notes/08-keychain-secret-storage.md](08-keychain-secret-storage.md)
- Secret reading: [src/secrets.rs](../src/secrets.rs)
