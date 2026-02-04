---
version: 0.1.0
updated: 2026-02-03
---
# Feature: Session Scanner & JSONL Parser

**Status:** Complete\
**Started:** 2026-02-03\
**Completed:** 2026-02-03

---

## Problem

The spike needs a reliable way to find and parse Claude Code transcript files to extract session timing and project data. This is the core data layer — everything downstream (live display, historical summaries) builds on it.

---

## Proposed Solution

A Rust module that scans `~/.claude/projects/` for today's session files, parses their JSONL transcripts, and produces structured session data: start time, end time, duration, and project path. Output for this phase is a stdout summary — no TUI yet.

### Integration Points

- Input: `~/.claude/projects/` filesystem layout
- Output: structured session data to stdout (will become the data source for TUI in next phase)
- No external services or network

### Key Behaviors

- Scans all subdirectories under `~/.claude/projects/`
- Matches only `<uuid>.jsonl` files — skips `agent-*` files and bare UUID directories
- Parses JSONL line-by-line; only cares about `user` and `assistant` message types
- Extracts `timestamp` and `cwd` from those messages
- Session duration = sum of consecutive-message gaps below the idle threshold (gaps ≥ threshold are idle and excluded)
- Project identity = `cwd` from the first user/assistant message
- Filters to sessions with activity today (local calendar day)
- Skips files with no user/assistant messages (effectively empty sessions)

---

## Success Criteria

- [x] Finds all of today's session files across all project directories
- [x] Correctly skips agent files and directories
- [x] Parses timestamps and project paths from JSONL accurately
- [x] Calculates session durations correctly
- [x] Prints a readable session summary to stdout
- [x] Handles edge cases gracefully: empty sessions, malformed/unparseable lines

---

## Scope

### In Scope

- File discovery under `~/.claude/projects/`
- JSONL parsing for `user` and `assistant` message types
- Session duration calculation
- Project identification via `cwd`
- "Today" filter using local calendar day
- Stdout output

### Out of Scope

- Live file watching / tailing (next phase — that's the TUI plan)
- TUI display (next phase)
- Token usage extraction (future)
- `sessions-index.json` parsing (not universal; possible optimization later)
- Anything beyond today's sessions

---

## Important Considerations

- **Timestamps are UTC** (`2026-02-03T17:36:56.582Z`). "Today" means local calendar day — convert to local before the date boundary check.
- **Empty session files exist.** Some JSONL files contain only `file-history-snapshot` / `queue-operation` lines. Skip these cleanly — don't error on them.
- **This is foundation code.** The session struct and scanning logic will be reused when the TUI layer is added. Keep the data layer cleanly separated from output concerns, but don't abstract prematurely for it.
- Files are `0600` owner-only — no permission issues since we're the owner.

---

## High-Level Todo

- [x] Add dependencies: `serde`, `serde_json`, `chrono`, `anyhow`
- [x] Define data structures: session message types (serde), parsed Session output
- [x] Implement file scanner: find `<uuid>.jsonl` files, skip agent files and dirs
- [x] Implement JSONL parser: extract timestamps and cwd from user/assistant messages
- [x] Implement duration calculation and today-filter
- [x] Wire together: scan → parse → print
- [x] Validate against real transcript data
- [x] Verify edge cases

---

## Notes & Context

### 2026-02-03 - Investigation findings

- `sessions-index.json` exists in only 2 of 11 project dirs — can't rely on it
- `agent-*.jsonl` = sidechains spawned within a parent session; share the parent's `sessionId`; skip to avoid double-counting time
- UUID directories contain only `tool-results`; irrelevant here
- Every `user`/`assistant` message has top-level `timestamp` (ISO 8601 UTC) and `cwd`
- `assistant` messages include `message.usage` with token counts — noted for future use, out of scope now
- Small files (~472 bytes) are sessions where nothing actually happened — only snapshot messages, no user activity

### 2026-02-03 - Real-data validation

Ran against actual `~/.claude/projects/` — 17 sessions, 262m total across 4 projects. Output matched expectations for a full coding day. Notable: several sessions show 0m duration (start == end within the same minute). These are real — single-message sessions where the user opened Claude Code but the conversation was very short. They survive the parser cleanly (not filtered as empty, since they do have user/assistant messages) and show up as 0m. Acceptable for now; worth revisiting if the TUI needs to handle them differently.

### 2026-02-04 - Idle timeout added to duration calculation

Duration was originally `end - start` (wall clock). Long conversations with idle gaps
(e.g. lunch break mid-session) inflated totals visibly. Changed `assemble_session` to
walk consecutive message gaps and exclude any gap ≥ a configurable idle threshold
(default 15 min). Threshold is loaded from `~/.config/claude-tracker/config.toml`.
Rest of the pipeline (aggregation, display) was unchanged.

### 2026-02-03 - Why data layer first

Three options were considered for the first spike phase: (A) data layer only, (B) full end-to-end in one plan, (C) TUI skeleton with mock data. Chose A. Parsing reliability is the core bet of the spike — if we can't do that, the display doesn't matter. It's also the most reusable piece as foundation for the real tool.

---

## Reference

- Spike plan: [notes/01-spike-plan.md](01-spike-plan.md)
- Template: [notes/00-feature-plan-template.md](00-feature-plan-template.md)
- Transcript layout: `~/.claude/projects/<project-dir>/<uuid>.jsonl`
- Message types of interest: `user`, `assistant` (both have `timestamp`, `cwd` at top level)
