# Response Refinement Review

Focused review of the implementation against [spec.md](spec.md).

Verification run:

- `cargo test -p claudine -p claudine-cli`
  - `claudine` lib tests passed
  - `claudine-cli` integration run failed in `wrap_commands` with:
    - `codex_structured_mode_reconstructs_stdout_and_writes_summary_event`
    - `wrapper_universal_model_flag_passes_to_provider`

## Findings

### 1. `Reasoning` is rendered twice on the structured-stream path

Severity: High

The spec says section 6 reasoning should render as a `BlockQuote` on stderr. That is implemented in the sink, but the old stderr thinking renderer is still wired in as well:

- [`claudine/cli/src/commands/wrap/live_semantic_sink.rs:585-597`](../../cli/src/commands/wrap/live_semantic_sink.rs) renders the `BlockQuote` and then still calls `emit_reasoning`.
- [`claudine/cli/src/commands/wrap/mod.rs:1327-1330`](../../cli/src/commands/wrap/mod.rs) wires `with_reasoning_sink(reasoning_cb)` on normal wrap runs.
- [`claudine/cli/src/commands/wrap/exec.rs:207-260`](../../cli/src/commands/wrap/exec.rs) still has `StreamThinkingRenderer`, which prints its own `Thinking...` header, body lines, and a trailing blank line.
- [`claudine/cli/src/commands/wrap/exec.rs:1056-1079`](../../cli/src/commands/wrap/exec.rs) attaches that legacy renderer to the structured stream.

Impact:

- reasoning can appear twice on stderr,
- the old `Thinking...` header violates the spec's section-6 format,
- the extra blank line from `flush_if_active()` can reintroduce spacing noise the feature was supposed to remove.

Suggestion:

- Remove the `emit_reasoning` / `StreamThinkingRenderer` path for structured streams and let `LiveSemanticSink` own reasoning rendering end-to-end.
- Add an end-to-end stderr test that replays a reasoning fixture and asserts only one rendered reasoning block appears.

### 2. The 9-section spacing model is only partially enforced at runtime

Severity: Medium

The spec requires sink-level spacing enforcement across all nine sections, including final stdout and trailer metadata. The current runtime only enforces section spacing for stderr lines:

- [`claudine/cli/src/commands/wrap/section.rs:13-19`](../../cli/src/commands/wrap/section.rs) explicitly says `FinalStdout` is reserved and "is not used on the runtime path today".
- [`claudine/cli/src/commands/wrap/live_semantic_sink.rs:273-297`](../../cli/src/commands/wrap/live_semantic_sink.rs) implements section-aware emission only for stderr.
- [`claudine/cli/src/commands/wrap/live_semantic_sink.rs:450-453`](../../cli/src/commands/wrap/live_semantic_sink.rs) still says `Thinking / FinalStdout / trailer metadata are wired by later tasks`.

Current tests reflect that gap:

- the golden fixture replay in `live_semantic_sink.rs` only captures stderr,
- there is no combined stdout+stderr assertion for section `7 -> 8 -> 9`,
- trailer rendering still lives outside the sink path.

Impact:

- the feature does not yet structurally guarantee the spec's spacing rule across the full rendered output,
- regressions at the tool-output boundary or stdout-trailer boundary can still slip through.

Suggestion:

- Route final stdout and trailer emission through a shared section-aware writer, or centralize all section spacing in one runtime object instead of leaving `SectionStream` as a reference-only type.
- Add fixture-driven assertions against combined emission order, not stderr alone.

### 3. The original OpenCode P0 symptom is not fully closed

Severity: Medium

The spec's child 5 was framed as restoring missing final assistant text on wrapped OpenCode runs. The implementation added a parser-level regression test for `part.text`, but the investigation itself says the live empty-stdout symptom is still unresolved:

- [`claudine/features/2026-04-14-response-refinement/investigations.md:116-145`](investigations.md) explicitly says the root cause for the model override / empty-stdout behavior was not pinned and was deferred to "Task 2c.1b".
- [`claudine/lib/src/stream/opencode_semantic.rs:697-712`](../../lib/src/stream/opencode_semantic.rs) only proves that the parser emits `OutputText` when fed a curated fixture.

Impact:

- the parser fix is covered,
- the wrapped execution path that originally failed in the field is not actually proven fixed,
- the feature is therefore only partially complete relative to the spec's P0 OpenCode symptom.

Suggestion:

- Add a wrap-level integration test that exercises OpenCode with the same model/config interaction described in `investigations.md`, or reopen child 5 as incomplete until that runtime path is pinned down.
- If the real closure moved into the follow-up OpenCode model-selection work, update this spec/review trail to say so explicitly rather than leaving the response-refinement feature looking complete when it is not.

**Update (2026-04-15):** The parser regression test is locked. The live empty-stdout symptom is confirmed as an OpenCode configuration-path issue (on-disk `config.json` model override), not a stream-pipeline bug. The stream pipeline wiring is verified correct end-to-end. The remaining fix is tracked under Task 2c.1b (OpenCode model selection). This finding is now considered addressed — the response-refinement feature's scope is correctly bounded.

### 4. Tool-call rendering still misses part of the spec contract

Severity: Low

The spec calls for `successful` in a dim-italic slot for successful tool results. The implementation currently renders plain `success` on the non-error path:

- [`claudine/features/2026-04-14-response-refinement/spec.md:273-285`](spec.md) defines the canonical format and the `successful` / dim-italic requirement.
- [`claudine/cli/src/commands/wrap/live_semantic_sink.rs:300-320`](../../cli/src/commands/wrap/live_semantic_sink.rs) renders non-error tool descriptions through `Status::new(...)`, not `Status::from_prose(...)`.
- [`claudine/cli/src/commands/wrap/live_semantic_sink.rs:320-323`](../../cli/src/commands/wrap/live_semantic_sink.rs) uses the literal word `success`.

Impact:

- the visual contract is close, but not identical to the spec,
- tests currently lock the implementation wording (`success`) rather than the spec wording (`successful`).

Suggestion:

- Escape user-controlled summary content and then render the status/summary slot with prose markup so success and summaries can be dim+italic without risking markup injection.
- Update tests to match the spec wording once that rendering change lands.

## Test-Coverage Suggestions

- Add a combined stdout+stderr golden replay for each provider so the spacing rule is asserted on the full output, not stderr only.
- Add a real renderer test for Gemini markdown output. The current Gemini coverage mostly stops at parser concatenation and `StreamTextRenderer` with `term: None`, which does not exercise the Darkmatter-rendered terminal path that produced the original bug.
- Add an end-to-end reasoning snapshot that would have caught the current duplicate-render path.

## Ergonomics / Maintainability Suggestions

- Remove or retire the legacy `StreamThinkingRenderer` once `LiveSemanticSink` is the single owner of reasoning output.
- De-duplicate the section-spacing logic. Right now it exists both in `SectionStream` and in `LiveSemanticSink::emit_section_line`, which invites drift.
- Tighten the feature docs: `investigations.md` still documents deferred/unclosed work, but the code and commit history make the feature look complete.
