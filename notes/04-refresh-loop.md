---
version: 0.1.0
updated: 2026-02-04
---
# Feature: Refresh Loop

**Status:** In Progress\
**Started:** 2026-02-04\
**Completed:** —

---

## Problem

The display mockup renders a one-shot snapshot of today's sessions. Real usage requires live updates — as you work in Claude Code, the tracker should show accumulating time without manual restarts. The current app scans once, renders, and waits for `q`. It needs a refresh loop.

---

## Proposed Solution

Add a timer-based refresh loop that re-scans session data periodically and re-renders the display. Include an animated spinner in the header to show the app is alive, and support `r` to force an immediate refresh.

### Integration Points

- Extends `main.rs` event loop (currently just listens for `q`)
- Reuses existing scan/parse/aggregate pipeline unchanged
- Header rendering gains animated spinner (replaces static `↻ --`)

### Key Behaviors

- **Periodic refresh**: Re-scan and re-render every N seconds (configurable, default 2s)
- **Animated spinner**: Braille dots (`⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏`) cycle in the header to indicate liveness
- **Manual refresh**: `r` key triggers immediate re-scan
- **Non-blocking event loop**: Spinner animates smoothly between refreshes
- **`q` to quit**: Unchanged from mockup

### Target Layout

```
claude-tracker  ⠹

  tlf-gql-api          19m
  scripts               4m
  claude-workflows     63m
  make-it-so-cli       16m
  claude-tracker      156m  ← now

  Today: 258m  (4h 18m)
```

The spinner character cycles continuously. When a refresh completes, data updates in place.

---

## Success Criteria

- [ ] App refreshes data automatically every N seconds
- [ ] Spinner animates smoothly in the header (no stuttering)
- [ ] `r` triggers immediate refresh
- [ ] Time totals update as new session activity occurs
- [ ] `q` still exits cleanly
- [ ] No visible flicker or glitching during refresh

---

## Scope

### In Scope

- Timer-based periodic refresh
- Animated braille spinner in header
- `r` key for manual refresh
- Configurable refresh interval

### Out of Scope

- File watching / event-driven refresh (future enhancement)
- Refresh interval configuration UI (config file only)
- Any new display features (layout is frozen from mockup)
- Colors or styling changes

---

## Important Considerations

- **Event loop timing**: The spinner should animate faster than the data refresh (e.g., spinner ticks every 100ms, data refreshes every 2s). This requires a tick-based event loop, not just blocking on input.
- **ratatui double-buffering**: Refreshing the full screen every tick is fine — ratatui diffs and only updates changed cells. No manual optimization needed.
- **Scan cost**: Re-scanning all session files every 2s is cheap for the current data volume. If this becomes a problem later, file watching is the fix (out of scope now).
- **Spinner state**: The spinner frame index is UI state, not data state. Keep it separate from the session data model.

---

## High-Level Todo

- [ ] Refactor event loop to tick-based (support both input events and timer ticks)
- [ ] Add spinner state and render animated spinner in header
- [ ] Add periodic data refresh on configurable interval
- [ ] Add `r` key handler for manual refresh
- [ ] Verify smooth animation and clean data updates
- [ ] Test with real session activity (watch totals increment)

---

## Notes & Context

### 2026-02-04 - Timer vs file watching

Considered file watching (notify crate) vs simple timer polling. Timer is simpler and sufficient for MVP — scanning a handful of local JSONL files every 2 seconds is negligible. Architecture keeps refresh trigger separate from display logic, so file watching can be swapped in later without touching render code.

### 2026-02-04 - Spinner choice

Braille dots (`⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏`) chosen for smooth 10-frame animation. Alternatives considered: `|/-\` (too simple), block characters (too chunky). Braille gives a nice "working" feel without being distracting.

---

## Reference

- Previous feature: [notes/03-display-mockup.md](03-display-mockup.md)
- Data layer: [notes/02-session-scanner.md](02-session-scanner.md)
- ratatui event handling: https://docs.rs/ratatui (see examples for tick-based loops)
