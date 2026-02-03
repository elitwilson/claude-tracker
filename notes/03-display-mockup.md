---
version: 0.1.0
updated: 2026-02-03
---
# Feature: Display Mockup

**Status:** In Progress\
**Started:** 2026-02-03\
**Completed:** TBD

---

## Problem

The data layer is proven and working. The next question: does the grouped, project-level summary layout actually look and feel right in a real terminal UI? We need to see it rendered before committing to the refresh/watch architecture that sits on top of it.

---

## Proposed Solution

A one-shot ratatui app that scans real data, aggregates it by project, and renders the target layout. No refresh loop — just render, wait for `q`, exit. The mockup becomes the rendering skeleton that the live refresh layer will be built on top of later.

### Integration Points

- Consumes output from `scanner::find_session_files` and `parser::*` (existing data layer)
- Introduces `ratatui` + `crossterm` as dependencies
- Adds a new aggregation step: `Vec<Session>` → grouped project summaries (view model)
- `main.rs` gets rewritten: current one-shot println loop becomes a ratatui app

### Target Layout

```
claude-tracker  ↻ --

  tlf-gql-api          19m
  scripts               4m
  claude-workflows     63m
  make-it-so-cli       16m
  claude-tracker      153m  ← now

  Today: 255m  (4h 15m)
```

- Project names: last path segment only
- One row per project, time right-aligned
- `← now` on the project with the most recent activity
- `↻ --` in the header (static for mockup; becomes refresh indicator later)
- Grand total at bottom in both minutes and h:m
- `q` to quit

### Key Behaviors

- Scans and parses session data (reuses existing data layer, one-shot)
- Aggregates sessions by project: sums duration, tracks last activity timestamp
- Renders layout into ratatui alternate screen
- Marks most-recently-active project with `← now`
- Exits cleanly on `q`, restoring terminal

---

## Success Criteria

- [ ] `cargo run` opens alternate screen with the target layout
- [ ] Projects are grouped correctly with accurate total durations
- [ ] Most-recently-active project is marked
- [ ] Layout looks right against real data (eyeball check)
- [ ] `q` exits cleanly, terminal is restored

---

## Scope

### In Scope

- Aggregation logic: sessions → project summaries
- ratatui rendering of the target layout
- One-shot data load (no refresh, no file watching)
- `q` to quit

### Out of Scope

- Refresh / live updates (next plan)
- File watching / event-driven re-scan (next plan)
- The `↻` indicator actually ticking (placeholder only)
- Any interaction beyond `q` to quit
- Styling / colors (get the layout right first)

---

## Important Considerations

- **ratatui redraws the full screen each frame.** Don't think in terms of updating individual elements — think in terms of "given this state, what does the screen look like?" The render function is a pure description.
- **Alternate screen buffer.** ratatui swaps the terminal to a clean screen on start and restores on exit. If the app panics without cleanup, the terminal can be left in a bad state. Use `crossterm`'s cleanup hooks.
- **Project name collisions.** Last-path-segment shortening could collide (two projects named `api` in different parent dirs). Not a problem with current real data, but worth noting. Out of scope to solve now.
- **The 0m sessions.** These still exist in the aggregated totals. A project with only 0m sessions will show as 0m. Acceptable for now.
- **View model lives in main.rs for now.** The aggregation (sessions → project summaries) is simple enough to keep inline. Don't extract a module unless it gets complex.

---

## High-Level Todo

- [ ] Add `ratatui` and `crossterm` dependencies
- [ ] Define project summary aggregation (group by project, sum durations, track last activity)
- [ ] Rewrite `main.rs`: scan → aggregate → ratatui render loop
- [ ] Implement the target layout as a ratatui render function
- [ ] Verify against real data, eyeball the layout
- [ ] Confirm clean exit on `q`

---

## Notes & Context

### 2026-02-03 - Why ratatui for the mockup, not just println

Using ratatui now (not just for the final live version) because the mockup becomes the rendering skeleton. Later plans add the refresh loop and file watching on top — they don't rewrite the rendering. Using println for the mockup would mean rewriting everything when we add the TUI anyway.

### 2026-02-03 - UX design decisions

Layout sketch was iterated in conversation before this plan was written. Key calls: project names shortened to last segment, grouped with per-project totals, most-recent marked with `← now`, grand total at bottom. These came from the spike plan's stated goals (today's total broken down by project) shaped into a concrete layout.

---

## Reference

- Spike plan: [notes/01-spike-plan.md](01-spike-plan.md)
- Data layer: [notes/02-session-scanner.md](02-session-scanner.md)
- Existing data layer code: `src/scanner.rs`, `src/parser.rs`
- ratatui: https://docs.rs/ratatui
