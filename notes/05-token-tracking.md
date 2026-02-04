---
version: 0.1.0
updated: 2026-02-04
---
# Feature: Token Usage Tracking

**Status:** In Progress\
**Started:** 2026-02-04\
**Completed:** —

---

## Problem

We track time spent per project, but token usage — the other axis of Claude Code consumption — is invisible. Every assistant message in the JSONL already carries full usage data (`input_tokens`, `output_tokens`, cache fields). We're scanning these files on every refresh and discarding that data.

---

## Proposed Solution

Wire token usage through the existing scan → parse → aggregate → render pipeline. No new data sources, no storage changes — just extracting and displaying data we're already reading.

### Integration Points

- **parser.rs** — `ParsedMessage` gains optional `TokenUsage`; `Session` gains token totals; `assemble_session` sums usage across messages
- **main.rs** — `ProjectSummary` gains token fields; `aggregate_sessions` sums them; `render` displays them per row and in the daily total

### Key Behaviors

- Every `assistant` message's `message.usage` is parsed and summed into the session total
- **Input** in the UI = `input_tokens` + `cache_creation_input_tokens` + `cache_read_input_tokens` (the three fields are additive, not nested)
- **Output** in the UI = `output_tokens`
- All four raw fields stored in `Session` for future use; UI collapses to two numbers
- Large token counts formatted compactly (e.g. `3.2k`, `142k`)
- Per-project rows show input and output alongside time
- Daily total line includes aggregate input and output

### Target Layout

```
claude-tracker  ⠹

  tlf-gql-api          19m    3.2k in    150 out
  scripts               4m     800 in     42 out
  claude-workflows     63m    142k in    8.1k out
  claude-tracker      156m     89k in    5.2k out  ← now

  Today: 242m  (4h 2m)  235k in  13.5k out
```

---

## Success Criteria

- [ ] Token usage is extracted from all assistant messages during parsing
- [ ] Per-project token totals update live alongside time totals
- [ ] Input and output displayed clearly in each project row
- [ ] Daily total includes token aggregates
- [ ] Messages without usage (user messages, or assistant messages missing the field) don't break anything
- [ ] Numbers format compactly for large values

---

## Scope

### In Scope

- Parsing `message.usage` from assistant messages
- Summing tokens per session and per project
- Displaying input/output in the live dashboard
- Compact number formatting

### Out of Scope

- Cost calculations (cache vs. regular input rates differ — store raw fields for that later)
- Per-message token detail
- Token usage in any persistence layer (doc 06)
- Historical token data

---

## Important Considerations

- **Only assistant messages have usage.** `ParsedMessage.usage` is `Option<TokenUsage>`. User messages parse cleanly with `None`.
- **Some assistant messages may lack usage.** Handle as zero, don't error.
- **"Input" is three additive fields.** Confirmed from real data: `input_tokens` is the non-cached portion; `cache_creation_input_tokens` and `cache_read_input_tokens` are separate. Total input = sum of all three. The nested `cache_creation.ephemeral_*` fields are subsets of `cache_creation_input_tokens` — skip them.
- **Number formatting.** Token counts can reach hundreds of thousands per day. Need compact display to fit terminal width.
- **Column alignment.** Two new columns per row. Time, tokens, and `← now` marker all need to coexist cleanly at varying terminal widths.

---

## High-Level Todo

- [ ] Extend `ParsedMessage` with optional `TokenUsage` struct
- [ ] Extend `Session` with token fields; update `assemble_session` to sum them
- [ ] Extend `ProjectSummary` and `aggregate_sessions` for tokens
- [ ] Add compact number formatting utility
- [ ] Update `render` — per-project rows and daily total
- [ ] Tests: token parsing, summing, formatting

---

## Notes & Context

### 2026-02-04 - Token field mapping

The JSONL `message.usage` object contains four token fields plus `service_tier`:

- `input_tokens` — base input (non-cached)
- `cache_creation_input_tokens` — input that created new cache entries
- `cache_read_input_tokens` — input served from existing cache
- `output_tokens` — output

Confirmed additive from real data: one assistant message had `input_tokens: 3`, `cache_creation_input_tokens: 3826`, `cache_read_input_tokens: 14484`. If `input_tokens` included the cache fields it would be 18k+, not 3.

The nested `cache_creation` object (`ephemeral_5m_input_tokens`, `ephemeral_1h_input_tokens`) is a breakdown of `cache_creation_input_tokens` — doesn't add to totals. Skipped.

### 2026-02-04 - Why input/output only in UI

Full token breakdown (cache hit rate, creation vs. read) is interesting for cost optimization but adds visual noise to what's primarily a "how much am I using" dashboard. Store all four fields for future analysis; display the two numbers that map to usage.

---

## Reference

- Previous feature: [notes/04-refresh-loop.md](04-refresh-loop.md)
- Parser: [src/parser.rs](../src/parser.rs)
- View model & render: [src/main.rs](../src/main.rs)
- Token field source: `message.usage` on assistant messages in `~/.claude/projects/<project>/<uuid>.jsonl`
