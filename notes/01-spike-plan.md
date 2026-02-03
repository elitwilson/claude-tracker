---
version: 0.1.0
updated: 2026-02-03
---
# Feature: Claude Code Session Watcher (Spike)
**Status:** Concluded\
**Started:** 2026-02-03\
**Concluded:** 2026-02-03

---

## Problem
A significant portion of coding work happens through Claude Code, but that time isn't tracked anywhere. Manually starting/stopping timers is friction nobody wants. We need passive, automatic time tracking for Claude Code sessions, and we want to explore what other useful data might be available in the transcripts.

---

## Proposed Solution
Build a simple Rust CLI (`claude-watch`) that:
- Watches Claude Code transcript files in real-time
- Extracts session data (timestamps, project/directory info)
- Displays a live terminal dashboard showing:
  - Current active session with elapsed time
  - Today's total time broken down by project
  - Simple refresh/update indicator

This is a **spike/POC** to prove we can effectively tail and parse Claude transcripts in real-time before building the full solution.

### Key Behaviors
- Automatically detects active Claude Code sessions
- Updates display in real-time as the session progresses
- Groups session time by project/directory
- Runs as a standalone CLI tool

---

## Success Criteria
- [ ] Can run `claude-watch` and see current session time ticking up — deferred to real CLI
- [x] Shows accurate project identification
- [x] Displays today's total time across all sessions
- [ ] Updates display in real-time — deferred to real CLI
- [x] Proves we can parse the JSONL transcript format reliably

---

## Scope

### In Scope
- Real-time transcript tailing
- Basic time tracking (session start, elapsed time)
- Project identification from transcript data
- Simple terminal UI that refreshes
- Today's sessions only (in-memory, no persistence)

### Out of Scope
- Token usage tracking (future)
- Devlog generation (future)
- Historical session data/persistence
- Multiple simultaneous sessions
- Configuration files
- Fancy TUI with graphs/charts

---

## Important Considerations
- This is a spike - code quality matters less than proving the concept works
- Focus on "can we do this effectively" not "is this production-ready"
- The transcript format is known/provided - we don't need to reverse-engineer it
- If the spike works, this becomes foundation for full time tracking + devlog system

---

## High-Level Todo
- [x] Set up Rust project with basic CLI structure
- [x] Implement JSONL transcript parsing
- [ ] Build file watcher/tailer for real-time updates — deferred to real CLI
- [x] Extract time and project data from transcript
- [ ] Create simple terminal display that updates — deferred to real CLI
- [x] Test with actual Claude Code session
- [x] Document what worked / what didn't

---

## Notes & Context

### 2026-02-03 - Spike conclusion

The spike's core question: "Can we reliably parse Claude transcripts to extract session timing?" Answer: yes. Validated against real data — 17 sessions, 262m, 4 projects, all edge cases handled cleanly (agent files skipped, empty sessions skipped, malformed lines skipped, 0m sessions survive gracefully).

The two items deferred (file watching, terminal refresh) are standard tooling problems — `notify` for watching, clear-and-reprint or a TUI crate for display. They were never the risky part. The risky part was the parsing, and that's proven.

Decision: conclude the spike. The data layer (scanner.rs, parser.rs) becomes foundation for the real CLI. File watching and TUI are planned as new features against that foundation.

### 2026-02-03 - Initial Planning
Core insight: Don't overthink implementation details in planning. The spike's job is to answer "can we build a CLI that watches Claude transcripts effectively?" If yes, we build the real thing. If no, we learned something valuable.

---

## Reference
- Claude Code transcript format (JSONL with message types: `user`, `assistant`, `file-history-snapshot`, `queue-operation`)
- Example CLI output target provided in initial brainstorm