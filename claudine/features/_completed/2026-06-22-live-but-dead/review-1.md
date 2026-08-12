---
ready: false
agent: codex/default
created: 2026-06-28T08:26:05
implemented: true
---

# Review 1 - Live-but-Dead Guard

## Verdict

Not production ready.

The core OpenCode stderr detector is implemented and has useful Level 1 coverage for the retry-churn algorithm, termination mapping, and configuration parsing. Two production blockers remain: one functional mismatch with the spec's progress reset taxonomy, and one verification-level gap for the new terminal-visible output.

## Findings

### High - stdout-origin progress does not reset the stalled-generation counter

The spec requires any progress-class event to reset the generation count and move `last_progress_at` forward, explicitly including `OutputText`, `Reasoning`, `ToolCall`, `ToolResult`, `FileChange`, and `PlanUpdate`. The implementation only resets inside the OpenCode stderr bridge for bridge-visible events:

- child session start: `claudine/lib/src/stream/logs/opencode/reasoning.rs:802`
- genuine `StepLoop` advance: `claudine/lib/src/stream/logs/opencode/reasoning.rs:942`
- `StepExit`: `claudine/lib/src/stream/logs/opencode/reasoning.rs:965`

The only reset helper is private to `OpenCodeLogBridge` at `claudine/lib/src/stream/logs/opencode/reasoning.rs:1117`, and the shared stdout semantic sink has no path back into that state. As a result, a run can accumulate `LlmCall` churn, then make real stdout progress, and still trip on a later `LlmCall` because the counter and silence clock were not cleared.

This violates acceptance criterion 5 and weakens the false-positive defense. Either reset from a shared progress observer that sees stdout semantic events, or narrow the spec/docs to an explicitly stderr-only contract and prove that stdout progress cannot interleave with this OpenCode retry pattern.

Verification level present: Level 1 tests cover bridge-local resets (`StepLoop`, `StepExit`, subagent lifecycle), but no Level 1 test covers stdout-origin progress resetting the stalled-generation state. Level 1 is sufficient for this logic contract, but it needs to exercise the real producer/sink wiring, not only the private bridge helper.

### High - terminal-visible stalled-generation output lacks Level 2 coverage

The feature adds a user-visible terminal error path: the bridge emits a terminal `SemanticEvent::Error` with label `Stalled Generation`, `SemanticErrorKind::AgentNative`, safe context, and a final summary message. The implementation is unit-tested in `claudine/lib/src/stream/logs/opencode/reasoning.rs:3143` and `claudine/cli/src/commands/wrap/exec/termination.rs:1415`, but I found no `level2_*` test covering `Stalled Generation`, `stalled_generation`, `stall_timeout`, or the live badge/final rendered error path.

Under the review rubric, user-observable terminal rendering requires Level 2 when the acceptance contract includes rendered labels, styling, widths, or captured terminal text. The strongest current verification is Level 1, so the visible output is below the required level.

Add a focused Level 2 tmux/WezTerm capture that runs a fake OpenCode structured-stream fixture, forces the stalled-generation trip without real sleeping, and asserts the captured pane includes the stalled-generation label/message and relevant safe context. Level 3 is not required because this feature does not depend on OS keyboard input encoding.

### Medium - structured `guard_context` omits available OpenCode identity metadata

Acceptance criterion 4 says `guard_context` includes at least `generation_count` and `stall_duration_ms`, and should include OpenCode metadata such as session id, step, agent, provider id, model id, and mode when present. The detector captures that metadata in `StalledGenerationContext` (`claudine/lib/src/stream/logs/opencode/reasoning.rs:78`), and the prose error message renders it (`claudine/cli/src/commands/wrap/exec/termination.rs:951`).

However, the structured `GuardContext` type only has `generation_count` and `stall_duration_ms` for stalled-generation trips (`claudine/lib/src/harness/model.rs:113`, `claudine/lib/src/harness/model.rs:129`), and `early_termination_guard_context` only populates those two fields (`claudine/cli/src/commands/wrap/exec/termination.rs:1088`). Lifecycle consumers therefore cannot branch on the safe OpenCode metadata without parsing the prose message.

This meets the "at least" floor but falls short of the intended structured diagnostics. Consider adding optional stalled-generation identity fields to `GuardContext` and populating them from `StalledGenerationContext`.

## Test Rigor

- Retry-churn trip condition: Level 1 present via helper and ingest tests in `opencode/reasoning.rs` (`four_streamed_generations_past_budget_trip_stalled_generation`, `three_generations_past_budget_do_not_trip`, `four_generations_under_budget_do_not_trip`). Level 1 is appropriate for pure detector logic.
- Progress reset taxonomy: Level 1 partial. Bridge-visible resets are covered; stdout-origin progress events from the spec are not covered and appear unimplemented.
- Long-tool exemption: Level 1 present via `long_tool_shape_never_trips_even_past_budget`. Level 1 is appropriate because the requirement is event classification, not terminal rendering.
- RepeatedStreamError independence: Level 1 present via `repeated_stream_error_is_independent_of_llm_call_churn` and idempotency coverage.
- Termination mapping to `ProcessTermination::Aborted`, `error_kind = "stalled_generation"`, and no `handle_timeout:` path: Level 1 present in `exec/termination.rs`.
- CLI/env/frontmatter duration precedence and `0s` disable: Level 1 present in timeout and harness-plan parsing tests.
- User-visible `Stalled Generation` terminal output and final rendered message: Level 1 only; needs Level 2 real-terminal capture.
- No Level 3 requirements are implied by this feature because it does not specify keyboard, mouse, paste, IME, or terminal input-encoder behavior.

## Verification Run

- Attempted `cargo nextest run -p claudine -p claudine-cli -E 'test(/stalled_generation/) | test(/stall_timeout/) | test(/stalled/)' --no-fail-fast --color never`.
- The command was stopped after about 60 seconds because it was still compiling dependencies and this is a non-interactive review session. No test result should be inferred from that attempt.

## Production Readiness

Not production ready. Fix the progress-reset mismatch and add Level 2 coverage for the new terminal-visible stalled-generation output before marking this feature ready.
