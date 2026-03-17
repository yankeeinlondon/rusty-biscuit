# Stream-JSON Implementation Review

This review compares the implementation under `claudine/lib/src/stream/` and `claudine/cli/src/commands/wrap/` against the requirements in `claudine/features/stream-json/spec.md`.

## Findings

1. `[P0]` Codex structured mode never reconstructs the assistant response, so default wrapped Codex runs do not satisfy the stdout contract.

The spec makes Codex's contract explicit: structured mode should be `exec --json --output-last-message <tempfile>`, with the temp file as the primary final-text source (`spec.md:106-110`, `spec.md:157-160`, `spec.md:326-329`). The local Codex research says the same thing (`claudine/docs/non-interactive-sessions/codex-json.md:45-49`, `claudine/docs/non-interactive-sessions/codex-json.md:111-117`). The current wrapper only injects `--json` for Codex structured mode (`claudine/cli/src/commands/wrap/profile.rs:524-527`), and both structured execution paths create the parser without any companion last-message artifact (`claudine/cli/src/commands/wrap/mod.rs:676-693`, `claudine/cli/src/commands/wrap/mod.rs:766-784`). `CodexStreamParser` even documents that `assistant_text` must be supplied externally from `--output-last-message`, but no caller ever does so (`claudine/lib/src/stream/codex.rs:10-12`, `claudine/lib/src/stream/codex.rs:31-32`). In practice, that means normal wrapped Codex runs print no assistant text to `stdout`, and inline composition will write an empty document body on success (`claudine/cli/src/commands/wrap/mod.rs:695-700`).

Recommendation: pair structured Codex mode with a temp file, read it after child exit, and only fall back to stream item text if the file is unavailable or incomplete.

2. `[P1]` Live dispatch is not wired at all; all coarse stream events are dropped.

The spec requires structured parsing to feed coarse events directly into Claudine's library dispatch path (`spec.md:214-243`, `spec.md:251-254`). The parser abstraction is built for that: `StreamEventSink` exposes session, turn, tool, permission, and warning callbacks (`claudine/lib/src/stream/parser.rs:13-25`), and each provider parser invokes those callbacks while parsing (`claudine/lib/src/stream/claude.rs:195-209`, `claudine/lib/src/stream/codex.rs:63-70`, `claudine/lib/src/stream/gemini.rs:152-199`, `claudine/lib/src/stream/kimi.rs:141-177`, `claudine/lib/src/stream/opencode.rs:159-178`, `claudine/lib/src/stream/qwen.rs:141-188`). But both structured wrapper paths instantiate the parser with `NullSink` (`claudine/cli/src/commands/wrap/mod.rs:680-683`, `claudine/cli/src/commands/wrap/mod.rs:770-773`), so none of those events can ever reach dispatch, hooks, or live logging.

Recommendation: replace `NullSink` with a sink that normalizes stream events into Claudine `EventMeta` and calls the library dispatch code directly.

3. `[P1]` Structured capture can hang and it discards provider stderr entirely.

`run_child_stream_capture()` spawns the child with `stderr` piped (`claudine/cli/src/commands/wrap/exec.rs:537-550`) but never drains that pipe before waiting for process exit (`claudine/cli/src/commands/wrap/exec.rs:551-582`). If a provider emits enough stderr output, the child can block on a full pipe and the inline composition path will hang. Even when it does not hang, all provider stderr is lost, so compose cannot surface retry hints, step-failure warnings, or structured diagnostics. That directly conflicts with the spec's compose/error-classification requirements (`spec.md:292-311`) and with the provider-specific stderr requirements for OpenCode and Kimi (`spec.md:345-353`).

Recommendation: drain stderr concurrently in structured capture mode, preserve filtered diagnostics, and fold the captured stderr into the returned summary/error path.

4. `[P1]` The stderr UX in the wrapper does not match the spec's start/warning/completion contract, and `--quiet` is effectively ignored for stream summaries.

The spec calls for start summaries, immediate warnings/errors, and a completion summary, with a distinct `--quiet` behavior (`spec.md:168-210`). The library already has formatter helpers for exactly that contract (`claudine/lib/src/stream/stderr.rs:14-139`). The wrapper bypasses them and emits only a post-run prose block via `emit_stream_summary()` (`claudine/cli/src/commands/wrap/mod.rs:846-877`). That function explicitly ignores the `quiet` flag (`claudine/cli/src/commands/wrap/mod.rs:851-853`), so `--quiet` does not actually reduce stream-summary verbosity. On top of that, parsers send malformed-line notices, rate-limit warnings, context pressure warnings, and step-failure warnings through `StreamEventSink::on_warning`, but because the wrapper uses `NullSink` those warnings are dropped before they ever reach `stderr` (`claudine/lib/src/stream/claude.rs:189-197`, `claudine/lib/src/stream/kimi.rs:137-146`, `claudine/lib/src/stream/opencode.rs:159-173`, `claudine/lib/src/stream/parser.rs:25`, `claudine/cli/src/commands/wrap/mod.rs:680-683`, `claudine/cli/src/commands/wrap/mod.rs:770-773`).

Recommendation: use the `stream/stderr.rs` helpers as the one source of truth, emit start/warning/completion summaries from the live sink, and make `--quiet`/`--silent` control those paths centrally.

5. `[P1]` Codex metadata parsing is behind the documented event vocabulary, so even the control-plane side of the integration is incomplete.

The local Codex research uses `thread.started` in its example stream and explicitly notes broader event coverage including item lifecycle events (`claudine/docs/non-interactive-sessions/codex-json.md:69-93`). The parser only recognizes `thread.created`, `turn.started`, `turn.completed`, error events, and tool events (`claudine/lib/src/stream/codex.rs:148-178`). It does not accept `thread.started`, and it ignores `item.completed`/agent-message events entirely. That means the documented example would fail to populate `session_id`, and there is no assistant-text fallback available from the stream if `--output-last-message` is missing or incomplete.

Recommendation: align the Codex parser with the documented `thread.started`/item lifecycle vocabulary and keep a minimal stream-text fallback for robustness.

6. `[P2]` The synthetic reporting event drops provider-specific compact summary data that the parsers already collected.

The spec recommends carrying safe compact provider summary data such as `raw_summary`, plus any provider-specific fields needed for observability (`spec.md:262-277`). Several parsers already accumulate this data, including Claude, Codex, Gemini, and Qwen (`claudine/lib/src/stream/claude.rs:178-186`, `claudine/lib/src/stream/codex.rs:104-107`, `claudine/lib/src/stream/gemini.rs:138`, `claudine/lib/src/stream/qwen.rs:115-117`). But `summary_to_event_meta()` only writes model, token usage, cost, duration, exit code, provider status, and tool count (`claudine/lib/src/stream/reporting.rs:45-93`). `raw_summary`, `rate_limit`, and `context_usage` never make it into the emitted JSONL event.

Recommendation: keep the existing core reporting fields, but also persist a compact provider summary object so rate-limit state, Kimi context pressure, and provider stop metadata remain queryable.

## Additional Suggestions

- Gemini parsing should actually correlate `tool_result` back to `tool_use` via `tool_id`, not just count calls. The spec requires that correlation (`spec.md:333-338`), and the local Gemini research explains why it matters (`claudine/docs/non-interactive-sessions/gemini-json.md:38-39`, `claudine/docs/non-interactive-sessions/gemini-json.md:52-67`). The current parser never stores a `tool_id` map (`claudine/lib/src/stream/gemini.rs:190-199`).
- Reconcile the Qwen parser with the local Qwen hook design. The design doc models headless output events as `system`, `assistant`, and `result` (`claudine/docs/hook-designs/qwen-cli.md:467-540`), but the parser only accepts `message` and `assistant_message` as assistant-text events (`claudine/lib/src/stream/qwen.rs:165-176`). If the hook design is authoritative, the parser will miss the documented `assistant` event type.
- Add wrapper-level tests that exercise the actual structured execution path, not just per-provider parser units. I did not find tests around `run_child_stream`, `run_child_stream_capture`, or `emit_stream_summary`, and the spec's acceptance criteria depend on those end-to-end behaviors (`spec.md:485-497`).

## Test Coverage Recommendations

The current test suite appears to over-index on unit parsing and argument shaping, while under-testing the real structured wrapper execution path. That explains how the suite can remain green even with contract-level gaps.

### Current bias in coverage

- Parser unit tests exist across `claudine/lib/src/stream/*.rs`, for example Codex in `claudine/lib/src/stream/codex.rs`.
- Wrapper CLI tests cover passthrough flags, environment setup, and generic quiet/silent behavior in `claudine/cli/tests/wrap_commands.rs`.
- Formatter/reporting helpers are tested in `claudine/lib/src/stream/stderr.rs` and `claudine/lib/src/stream/reporting.rs`.

What is mostly missing is coverage for:

- `claudine/cli/src/commands/wrap/exec.rs::run_child_stream`
- `claudine/cli/src/commands/wrap/exec.rs::run_child_stream_capture`
- `claudine/cli/src/commands/wrap/mod.rs::emit_stream_summary`
- the actual end-to-end structured wrapper contract described in `spec.md`

### Recommended additions

1. Add end-to-end wrapper tests per provider for default structured mode.

   Use fake provider scripts that emit realistic stream lines and assert:

   - `stdout` contains only assistant text
   - `stderr` contains Claudine summaries, not raw stream envelopes
   - one synthetic summary event is written

   This is the class of test that would have caught the broken Codex default path.

2. Add a Codex-specific integration test for `--output-last-message`.

   The current Codex parser unit test manually sets `assistant_text` before `finish()`, which hides the integration gap. Add a wrapper-level test that proves:

   - structured Codex mode adds `--json` and `--output-last-message`
   - the temp file is read back after child exit
   - final assistant text reaches `stdout`
   - inline composition updates the destination body with that text

3. Add structured-run tests for `--quiet` and `--silent`.

   The current quiet/silent coverage is focused on preflight wrapper output. Add tests that run a structured provider stream and verify:

   - default mode shows start/warning/completion summaries
   - `--quiet` reduces that to warnings plus one compact completion line
   - `--silent` suppresses all Claudine-generated stream summaries

4. Add malformed-line and partial-failure regression tests.

   Use fake provider streams that include:

   - one malformed JSON line between valid events
   - a structured warning event
   - a structured error plus non-zero exit

   Assert the wrapper still completes, the warning/error reaches `stderr`, and the synthetic summary event is still written.

5. Add structured capture tests for inline composition and document update flows.

   These should verify:

   - the updated file body contains assistant text only
   - provider noise does not leak into the document
   - provider-classified failures do not overwrite the target file
   - retry/error metadata remains available to callers

6. Add stderr-heavy child-process tests for `run_child_stream_capture()`.

   Use a fake provider that writes enough stderr to fill a pipe and then exits. Assert the wrapper does not hang. This specifically protects against deadlock risk in the capture path.

7. Add sink/dispatch tests once live dispatch is wired.

   The parsers already emit coarse events through `StreamEventSink`. Add tests with a recording sink at the wrapper boundary that verify:

   - session start
   - turn start / turn complete / turn error
   - before tool / after tool
   - warning propagation

   This would catch a regression where the wrapper accidentally drops everything into `NullSink`.

8. Add spec-driven contract tests as a table-driven suite.

   Encode the acceptance criteria from `spec.md` as executable tests, including:

   - default wrapped non-interactive run -> assistant text only on `stdout`
   - explicit `--output text` -> provider text contract preserved
   - explicit `--output json` -> raw provider JSON preserved
   - explicit `--output stream` -> raw provider stream preserved
   - one synthetic summary event written exactly once

   This keeps the implementation aligned to the feature spec rather than only to helper-level behavior.

## Test Status

- I started `just test` in `claudine/`, but it did not complete within the review window, so the findings above are based on source inspection rather than a completed package test run.
