# More is More — High Confidence Plan

## Goal

Restore and strengthen the non-interactive Claudine stderr/stdout experience after the 2026-04-16 composition-rendering unification so users consistently see rich progress, reasoning, tool parameters, warnings, errors, and buffered output without long silent stalls.

## Confidence Basis

This plan is based on the feature spec, the technical review, and a direct check of the current codebase. The following current-state facts are confirmed:

- [`claudine/lib/src/stream/tool_display.rs`](../../lib/src/stream/tool_display.rs) still renders `Bash` via the `command` field only, has no explicit `Task` extractor, and falls back to generic string selection for unknown tool shapes.
- [`claudine/cli/src/commands/wrap/live_semantic_sink.rs`](../../cli/src/commands/wrap/live_semantic_sink.rs) still formats tool lines as `name · summary`, renders warnings through `StatusState::Warning`, and renders errors through `StatusState::Failure` rather than a BlockQuote.
- [`claudine/lib/src/stream/protocol/opencode.rs`](../../lib/src/stream/protocol/opencode.rs) currently has no `Reasoning` variant in `OpenCodeEvent`.
- [`claudine/lib/src/stream/opencode_semantic.rs`](../../lib/src/stream/opencode_semantic.rs) still claims reasoning is out of scope and has no reasoning handler.
- [`claudine/cli/src/commands/wrap/exec.rs`](../../cli/src/commands/wrap/exec.rs) still only flushes `StreamTextRenderer::block_buffer` on markdown boundaries or EOF; the heartbeat path has no renderer-driven idle flush.
- [`claudine/lib/src/stream/semantic.rs`](../../lib/src/stream/semantic.rs) has `SemanticEvent::Error { message, terminal, extra }` with no typed error classification yet.

That means the review’s proposed changes map cleanly to the current implementation and are not stale.

## Scope

In scope:

- Tool call rendering improvements for shell tools and `Task`
- OpenCode reasoning support and thinking-block styling correction
- Typed live error classification and BlockQuote rendering
- Hanging mitigation via idle flushing of buffered assistant text
- Tests and docs needed to lock the behavior down

Out of scope for the first merge unless time remains:

- Broad provider audit beyond OpenCode reasoning
- Heuristic sentence-level early paragraph flushing
- Stalled-stream warning surface

## Implementation Order

### Phase 1 — Tool Call Surface

Files:

- [`claudine/lib/src/stream/tool_display.rs`](../../lib/src/stream/tool_display.rs)
- [`claudine/cli/src/commands/wrap/live_semantic_sink.rs`](../../cli/src/commands/wrap/live_semantic_sink.rs)

Changes:

- Update `extract_tool_summary()` to prepend shell names in summaries for shell-style tools so `Bash` renders as `bash {command}` instead of only `{command}`.
- Add an explicit `Task` extractor with ordered fallback across `description`, `subject`, `prompt`, and `task`.
- Change `render_tool_display()` from `→ Name · summary` to `→ Name(summary)` while preserving existing success/error/pending result handling.

Acceptance criteria:

- Shell tool calls show the invoked shell plus parameters.
- `Task` calls show the actual task body, not an arbitrary first string field.
- Tool result lines keep the same semantic status but adopt the same parentheses format.

Tests:

- `bash_summary_prepends_shell_name`
- `task_extracts_description_first`
- `task_falls_back_to_prompt_when_description_absent`
- `tool_call_renders_with_parentheses_format`

### Phase 2 — Hanging Fix, Required Slice

Files:

- [`claudine/cli/src/commands/wrap/exec.rs`](../../cli/src/commands/wrap/exec.rs)

Changes:

- Add `last_block_growth_at` tracking to `StreamTextRenderer`.
- Add `flush_if_idle()` so buffered markdown can be flushed before EOF.
- Pass the renderer into the heartbeat thread and flush buffered assistant text when it has been idle for at least `HeartbeatPolicy::silence_window`.
- Keep this phase limited to time-based flushing only; defer sentence heuristics and stalled-stream warnings.

Acceptance criteria:

- A dangling final paragraph becomes visible within the silence window even if the provider never closes stdout.
- The next heartbeat never appears above assistant text that was already buffered and old enough to flush.
- Existing partial-line behavior remains intact.

Tests:

- `flush_if_idle_emits_block_after_threshold`
- `flush_if_idle_does_not_emit_when_block_empty`
- `flush_if_idle_resets_growth_clock`
- Integration test for a dangling paragraph reaching captured stdout before the next heartbeat cycle

### Phase 3 — OpenCode Reasoning Parity

Files:

- [`claudine/lib/src/stream/protocol/opencode.rs`](../../lib/src/stream/protocol/opencode.rs)
- [`claudine/lib/src/stream/opencode_semantic.rs`](../../lib/src/stream/opencode_semantic.rs)
- [`claudine/lib/src/stream/thinking.rs`](../../lib/src/stream/thinking.rs)
- [`claudine/cli/src/commands/wrap/live_semantic_sink.rs`](../../cli/src/commands/wrap/live_semantic_sink.rs) for regression coverage only

Changes:

- Add a typed `Reasoning` variant to `OpenCodeEvent`.
- Add `OpenCodeReasoning` resolution helpers for top-level and nested text shapes.
- Route OpenCode reasoning into `SemanticEvent::Reasoning`.
- Remove the stale module-doc statement that OpenCode reasoning is not exposed.
- Change `render_thinking_block()` to use the wider `▌ ` border so thinking matches the established system-prompt and agent-prompt visual style.

Acceptance criteria:

- OpenCode reasoning no longer falls through `ProviderExtension`.
- Thinking text renders in the dedicated Thinking section as a gray BlockQuote with the wider border.
- Existing providers already using `SemanticEvent::Reasoning` remain unaffected.

Tests:

- `opencode_reasoning_top_level_text_deserializes`
- `opencode_reasoning_nested_part_text_resolves`
- `reasoning_event_emits_semantic_reasoning`
- `reasoning_with_empty_text_emits_nothing`
- `block_quote_uses_wider_border_character`

### Phase 4 — Typed Error Classification And Live Error Rendering

Files:

- [`claudine/lib/src/stream/semantic.rs`](../../lib/src/stream/semantic.rs)
- Provider semantic parsers that emit `SemanticEvent::Error`
- [`claudine/cli/src/commands/wrap/live_semantic_sink.rs`](../../cli/src/commands/wrap/live_semantic_sink.rs)
- [`claudine/cli/src/output/error_report.rs`](../../cli/src/output/error_report.rs)

Changes:

- Introduce `SemanticErrorKind` with `Configuration`, `AgentNative`, `ApiRemote`, `Interrupted`, and `Unknown`.
- Add `kind` to `SemanticEvent::Error` with `#[serde(default)]` for replay compatibility.
- Add one `classify_error()` helper per provider parser near its current `handle_error()` path.
- Replace live sink error rendering from `StatusState::Failure` to a BlockQuote renderer with label and color derived from `SemanticErrorKind`.
- Add `From<SemanticErrorKind> for AgentErrorCategory` so typed live error kinds can align with the end-of-run error-report surface.

Acceptance criteria:

- Warnings continue to use `StatusState::Warning`.
- Errors render with a colored `▌ ` BlockQuote rather than a single failure status line.
- Replay fixtures without `kind` still deserialize successfully as `Unknown`.
- Dispatch behavior remains keyed off `terminal`; `kind` is classificatory metadata, not a new dispatch switch.

Tests:

- `error_kind_round_trips_through_serde`
- `error_kind_default_is_unknown_when_field_missing`
- Provider-specific classification tests for at least configuration and remote/native cases
- `error_event_renders_blockquote_with_red_border`
- `interrupted_error_renders_blockquote_with_yellow_border`
- `configuration_error_renders_blockquote_with_orange_border`

### Phase 5 — Documentation And Optional Polish

Files:

- [`claudine/docs/topics/composition.md`](../../docs/topics/composition.md)
- [`claudine/cli/README.md`](../../cli/README.md)
- [`.claude/skills/claudine/SKILL.md`](../../../.claude/skills/claudine/SKILL.md)

Required doc updates:

- Document the richer tool rendering shape.
- Document idle flushing of buffered assistant output.
- Document typed live error rendering and classification.
- Document that OpenCode reasoning now uses the shared thinking surface.

Optional follow-up work if Phases 1-4 land cleanly:

- Add sentence-level early flush for non-fenced prose.
- Add a stalled-stream warning after repeated heartbeat cycles with no provider activity.
- Audit Gemini and Qwen protocol parsers for the same reasoning-variant gap.

## Verification Plan

Run focused checks after each phase rather than waiting for one large end-to-end pass:

1. `cargo test -p claudine tool_display`
2. `cargo test -p claudine live_semantic_sink`
3. `cargo test -p claudine opencode_semantic`
4. `cargo test -p claudine semantic_fidelity`
5. `cargo test -p claudine exec`
6. `cargo test -p claudine`

If there is an existing fixture or log replay path for wrapped non-interactive sessions, replay at least:

- one shell-heavy session with `Bash` and `Task`
- one OpenCode session containing `"type":"reasoning"`
- one session with a provider error
- one artificially stalled stream that leaves a paragraph unterminated

## Delivery Strategy

Recommended merge strategy:

1. Merge Phase 1 and Phase 2 together if the branch needs quick user-visible recovery.
2. Merge Phase 3 next because it is narrowly scoped and low risk.
3. Merge Phase 4 separately because it changes the shared semantic event model and provider parsers.
4. Merge docs and any optional polish last.

This keeps the most visible regressions moving first while isolating the highest-schema-risk change.
