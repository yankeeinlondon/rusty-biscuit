# Phase 6.0 Discovery — Parser Struct Ground Truth (2026-07-11)

Field-by-field comparison of the 8 stream parser state structs and their
`finish()` implementations, run before the 6a–6c extractions. This is the
evidence base for the re-scoped Phase 6 design (helpers by delegation, no
`ParserShared` state unification) and the 6d driver decision.

## State-struct intersection

Same name **and** type in all 8 structs (only 8 fields):
`sink`, `session_id`, `model`, `assistant_text`, `provider_status`,
`is_error`, `error_kind`, `error_message`.

Same name, **split types** (the crux of the aborted attempt's "materially
different state types"):

| field | `Option<T>` | plain `T` |
|---|---|---|
| `token_usage` | claude, codex, kimi, qwen, gemini | opencode, pi, antigravity |
| `cost_usd` | claude, codex, kimi, qwen, gemini | opencode, pi, antigravity |
| `num_turns` | claude, qwen, gemini | codex, opencode, kimi, pi, antigravity |

All 7 line-oriented parsers (not Antigravity): `line_num`, `tool_calls`.
4+ but not all: `duration_ms` (7, not pi), `raw_summary` (claude, codex,
qwen, gemini), `tool_uses` (5, with three different value types).

Genuinely provider-specific: claude (`api_key_source`, `duration_api_ms`,
`rate_limit`, `terminal_error_emitted`, `session_started`,
`pre_init_hook_buffer`, `pending_tool_use`), codex (`permission_prompts`,
`user_input_prompts`, typed `tool_items`), kimi (`pending_text`,
`context_usage`, `pending_tool_call`, `next_pending_slot`,
`pending_thinking`, `pending_thinking_kind`, vestigial
`prompt_status_seen`), gemini (`pending_text`), antigravity (`buffer`,
`emitted`, `has_usage`). Qwen is a proper subset of gemini; pi is
distinguished by absence.

**Conclusion:** a shared state struct would fit only 8 of ~15 fields and
fight three Option-vs-plain splits — delegation to free functions was the
right shape. Verdict on the original `ParserShared` plan: correctly
abandoned; the duplication never lived in the state, it lived in the
helpers.

## Helper deltas absorbed in 6a

- `base_extra`: 6 parsers take `(raw_kind)`; claude takes `()` (raw_kind
  added by its `extra_with`); antigravity has no line-oriented surface.
  Codex layers a conditional `session_id` key (kept local).
- `emit_provider_extension`: 7 byte-identical copies except kimi's debug
  message said "unknown envelope shape" — unified to the structured
  `provider` field + one message (tracing-only change, no event payload
  difference).
- `emit_malformed_warning`: 6 parsers redundantly re-inserted `line_num`
  after `base_extra` had already added it (same key/value — output
  identical); claude did not. Shared version inserts once.

## `finish()` (6b)

All 8 shared the same assembly idiom: full 22-field
`StreamExecutionSummary` literal (most fields hardcoded `None`) +
`derive_badges` stamp. Replaced by per-parser literals carrying **only
populated fields** + `..Default::default()`, with
`common::finish_summary(provider, summary)` stamping `provider` + badges.
Provider-specific pre-steps stay local: kimi's flushes +
`let _ = prompt_status_seen`, gemini's `flush_pending_text`, opencode/pi's
`has_usage` gates and `trace_parser_finish`, antigravity's buffered
re-emit + synthesized error.

## `classify_error` (6c)

The cascade skeleton was identical (lowercase kind → buckets, lowercase
message → buckets, `AgentNative` fallthrough) with per-provider keyword
and *ordering* deltas: gemini checks Configuration before ApiRemote in the
kind branch; codex/opencode/qwen/gemini run a second ApiRemote pass after
Interrupted; antigravity checks auth keywords first in the message branch;
pi/antigravity match `"abort"` where the others require `"aborted"`.
Replaced by `common::ErrorKeywords` **ordered bucket tables** (order = the
behavior contract, encoded as data) + `classify_error_by_keywords`. Kimi's
JSON-RPC numeric-code match stays local and falls through to its table.

## 6d decision — generic `feed_line` driver: NOT implemented

Post-6a the residual per-parser skeleton is the 6-line prologue
(`line_num += 1` / trim / empty-check) plus a fallback arm that is now
pure delegation. The remainder of each `feed_line` is the typed-enum
dispatch — the part that genuinely differs — and two parsers deviate
structurally (kimi's `KimiEnvelope::classify_str` + synthetic `raw_kind`;
gemini's pre-dispatch `flush_pending_text`). A generic driver would save
~14 lines × 6 parsers while adding generic machinery plus per-provider
hooks for those deviations. **C5 closes at 6c**; the driver is demoted to
the Nice-to-Have tier in `review.md`.
