---
updated: 2026-02-03
---
# Backlog

Ideas and future work, not yet planned.

---

## Idle timeout for session duration

**Problem:** `assemble_session` computes duration as `end - start` (wall clock span).
If you stay in a conversation across a long break, that break inflates the total.

**Idea:** Walk consecutive message timestamps. Sum only the deltas that fall below a
configurable idle threshold. Anything above = gap, not counted. Clock resumes on the
next message.

- Threshold is configurable (e.g. default 5 min)
- Changes `Session.duration` from wall-clock span → active time
- Fix lives in `parser::assemble_session` — the rest of the pipeline (aggregation,
  display) doesn't need to change
