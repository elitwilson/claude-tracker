---
version: 0.1.0
updated: 2026-02-04
---
# Feature: Keychain Secret Storage

**Status:** Done\
**Started:** 2026-02-04\
**Finished:** 2026-02-04

---

## Problem

The upcoming Clockify API spike needs an API key. There is no secret storage mechanism in the project. Storing keys in `.env` files or config.toml leaves them as plaintext on disk and risks accidental commits. The project needs a secure, OS-native place to put secrets before the spike begins.

---

## Proposed Solution

Use the `keyring` crate to read/write secrets from the OS keychain (macOS Keychain, Linux secret service, Windows credential store). A `setup` subcommand stores the key interactively; the app reads it back at runtime via the same keyring entry.

- **Service name:** `claude-tracker` (matches the config dir convention already in use)
- **Key name:** `clockify_api_key` (one entry per secret; easy to add more later)
- **Write path:** `claude-tracker setup` — prompts for the key, stores it, confirms. One-time operation.
- **Read path:** A `secrets` module exposes `get_secret(name)` — reads from keyring, returns a clear error if not set. Called by whatever code needs the key (the spike, later).

### Subcommand routing

Currently `main()` has no arg handling — it just starts the TUI. We need to distinguish `claude-tracker` (TUI) from `claude-tracker setup` (store key). Two options:

- **Manual `args` check** — inspect `std::env::args()` for `setup`. Zero new dependencies. Simple, but doesn't scale if more subcommands come.
- **`clap`** — the standard CLI arg parser. Adds a dependency now, but we'll almost certainly need it for the spike (flags, subcommands). Worth pulling in early.

**Decision: `clap`.** The spike will need subcommands and flags anyway — pulling it in here avoids a throwaway args check that we'd replace in the next feature.

### Integration Points

- **`Cargo.toml`** — add `keyring` (and possibly `clap`). If no `clap`, no other dep needed for the prompt — `std::io` suffices.
- **`src/secrets.rs`** (new) — thin wrapper around `keyring`. `store_secret(name, value)` and `get_secret(name)`. Errors via `anyhow`.
- **`src/main.rs`** — subcommand routing before the TUI bootstrap. `setup` path calls `secrets::store_secret`, prints confirmation, exits. Default path is unchanged.

---

## Success Criteria

- [x] `cargo install --path .` produces a binary that can run `claude-tracker setup`
- [x] `setup` prompts for the Clockify API key, stores it in the OS keychain
- [x] A second run of `setup` overwrites the existing key without error
- [x] `secrets::get_secret("clockify_api_key")` returns the stored value
- [x] `secrets::get_secret("clockify_api_key")` returns a clear error if no key has been stored
- [x] No secret is written to disk in the project

---

## Scope

### In Scope

- `keyring` integration: store and retrieve secrets from OS keychain
- `setup` subcommand: interactive prompt → store → confirm
- `secrets` module: `store_secret` / `get_secret`
- Subcommand routing in `main`

### Out of Scope

- Actually calling the Clockify API (that's the spike)
- Multiple secret types or a secret management UI
- Secret rotation or expiry
- `.env` fallback

---

## High-Level Todo

- [x] Decision: `clap` (needed for spike anyway)
- [x] Add `keyring` + `clap` to `Cargo.toml`
- [x] Implement `src/secrets.rs`
- [x] Wire `setup` subcommand into `main.rs`
- [x] Test: store, retrieve, overwrite, missing-key error
- [x] `cargo install --path .` smoke test

---

## Reference

- Previous feature: [notes/07-timeframe-toggle.md](07-timeframe-toggle.md)
- Config pattern (service name, path conventions): [src/main.rs](../src/main.rs) lines 27–80
