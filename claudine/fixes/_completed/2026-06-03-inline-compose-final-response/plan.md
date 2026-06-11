---
phases: 5
created: 2026-06-10
start_phase: 1
agent: open_code/kimi-for-coding/k2p6
yolo: true
---

# Inline Compose Final Response Capture — Execution Plan

## Overview

Add a provider-agnostic final-response accumulator at the normalized semantic layer so `inline-compose` writes only the agent's closing response (output text after the last tool call) into the document body. Interstitial process narration between tool calls must not leak into the artifact.

---

## Phase 1: Model and Semantic Layer Changes

**Goal:** Add the final-response accumulator and wire it into the live semantic sink.

- [ ] Add `final_response: String` field to `StructuredSummaryDetails` in `cli/src/commands/wrap/policy.rs`
- [ ] Add `push_final_response(&mut self, text: &str)` method that appends a text chunk to the accumulator
- [ ] Add `reset_final_response(&mut self)` method that clears the accumulator
- [ ] Update `LiveSemanticSink::on_semantic_event` in `cli/src/commands/wrap/live_semantic_sink/mod.rs`:
  - On `SemanticEvent::ToolCall { .. }`, call `details.reset_final_response()`
  - On `SemanticEvent::OutputText { text, .. }`, call `details.push_final_response(text)`
  - Ignore `Reasoning`, `ToolResult`, metadata, status, errors, and provider extension events for the accumulator
- [ ] Do **not** change `StreamExecutionSummary::assistant_text` semantics

**Validation checkpoint:**
- [ ] `cargo test -p claudine-cli live_semantic_sink::tests::final_response_keeps_only_text_after_last_tool_call` passes

---

## Phase 2: Non-Harness Inline-Compose Integration

**Goal:** Use the final-response accumulator as the body source for structured `inline-compose`.

- [ ] Update `run_structured_branch` in `cli/src/commands/wrap/composition/structured.rs`:
  - When `is_inline == true`, select the body source in this order:
    1. If `result.details.final_response.trim()` is non-empty, use `result.details.final_response`
    2. Otherwise, if the provider has a documented post-hoc final-message recovery path and `result.assistant_text` is non-empty, use `result.assistant_text` (Codex fallback)
    3. Otherwise, treat as empty and let the existing empty-output failure behavior trigger
  - When `is_inline == false` (direct `compose`), continue using `result.assistant_text` verbatim
- [ ] Ensure `CompositionStreamResult` carries `details` so the closure code can access `final_response`

**Validation checkpoint:**
- [ ] `cargo test -p claudine-cli` for the structured composition module passes

---

## Phase 3: Harness Inline-Compose Integration

**Goal:** Ensure the harness loop uses the same final-response source as the non-harness structured path.

- [ ] Update harness orchestration in `cli/src/commands/wrap/harness_orch.rs`:
  - After structured stream completion, read `details.final_response` from the shared `summary_details`
  - When `prompt_mode == HarnessPromptMode::Inline`, apply the same three-rule fallback:
    1. Non-empty `final_response` → use it
    2. Empty accumulator + Codex-style recovery with non-empty `assistant_text` → use `assistant_text`
    3. Otherwise empty → existing failure behavior
  - When `prompt_mode == HarnessPromptMode::Compose`, continue using `summary.assistant_text`
- [ ] Verify `build_attempt_outcome` in `lib/src/harness/runtime.rs` does not change semantics (it should continue using `summary.assistant_text` for the harness attempt outcome)

**Validation checkpoint:**
- [ ] `cargo test -p claudine-cli` for harness orchestration module passes

---

## Phase 4: Testing

**Goal:** Cover the accumulator, interstitial narration exclusion, fallback behavior, and harness/non-harness convergence.

### 4.1 Unit Tests (Semantic Sink)

- [ ] Add `final_response_keeps_only_text_after_last_tool_call` test (or verify existing):
  - Feed `OutputText("narration") -> ToolCall -> OutputText("more narration") -> ToolCall -> OutputText("final")`
  - Assert `final_response == "final"`
- [ ] Add `reasoning_does_not_append_to_final_response` test:
  - Feed `OutputText("hello ") -> Reasoning("pondering") -> OutputText("world")`
  - Assert `final_response == "hello world"`

### 4.2 Integration Tests (Inline-Compose)

- [ ] Add integration test for structured provider with interstitial narration:
  - Stub a stream that emits `OutputText`, `ToolCall`, `OutputText`, `ToolCall`, final `OutputText`
  - Run inline-compose and assert only the final text appears in the document body
- [ ] Add integration test for Codex post-hoc fallback:
  - Simulate a provider where `final_response` is empty but `assistant_text` contains post-hoc recovered text
  - Assert the post-hoc text is written to the body
- [ ] Add integration test for empty final response after tool call:
  - Emit `OutputText("narration") -> ToolCall` with no subsequent `OutputText`
  - Assert the source file is **not** mutated and the existing empty-output error path fires

### 4.3 Convergence Test

- [ ] Add or extend test that verifies harness and non-harness `inline-compose` produce identical replacement bodies for the same structured stream event sequence

**Validation checkpoint:**
- [ ] All new tests pass: `cargo test -p claudine-cli`
- [ ] Existing `inline-compose`, `compose`, and `sequence` tests continue passing

---

## Phase 5: Documentation

**Goal:** Update user-facing composition docs to describe the closure contract.

- [ ] Update `claudine/docs/topics/composition.md` under the Inline Composition closure behavior section to state:
  - The replacement body is the agent's **final response only**
  - "Final response" means output text emitted after the agent's **last tool call**
  - Interstitial narration is dropped
  - Providers with post-hoc final-message recovery (e.g. Codex) can supply that final message directly
  - Legacy non-structured stdout capture is **not** part of this guarantee
- [ ] Verify the docs do not imply full coverage for legacy non-structured providers

**Validation checkpoint:**
- [ ] Doc changes reviewed for accuracy against the implemented behavior

---

## Definition of Done

- [ ] Acceptance criteria 1–10 from the functional specification pass
- [ ] `cargo test -p claudine-cli` passes (all new and existing tests)
- [ ] Composition documentation updated and accurate
- [ ] No changes to `assistant_text` semantics for direct `compose`, summaries, logs, or terminal rendering
- [ ] Legacy Goose behavior is unchanged
- [ ] `cargo check` and `cargo clippy` pass for touched crates
