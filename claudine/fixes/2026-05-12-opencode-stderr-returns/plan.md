---
created: 2026-05-12
phases: 6
start_phase: 3
source_files_during_phase_1:
  - claudine/cli/src/commands/wrap/profile/opencode.rs
  - claudine/lib/src/stream/logs/opencode/reasoning.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - claudine/lib/src/stream/logs/opencode/errors.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - claudine/lib/src/stream/logs/opencode/reasoning.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - claudine/lib/src/stream/providers/opencode.rs
  - claudine/lib/src/stream/logs/opencode/reasoning.rs
  - claudine/cli/tests/wrap_commands.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5: []
docs_updated_during_phase_5:
  - claudine/docs/topics/timeouts.md
  - claudine/docs/research/agent-cli/opencode.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .claude/skills/claudine/SKILL.md
  - .claude/skills/claudine/opencode-event-sources.md
source_files_during_phase_6:
  - claudine/lib/tests/opencode_stderr_lifecycle.rs
  - claudine/lib/tests/fixtures/logs/opencode-subagent-lifecycle.txt
docs_updated_during_phase_6: []
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
packages:
  - claudine
---

# Plan: OpenCode Stderr as a First-Class Event Source

This plan implements the "Dual-Source Contract" for OpenCode integration, promoting the structured stderr log stream to a first-class activity signal. This ensures Claudine correctly observes progress during OpenCode's long "DONE-only" NDJSON silence windows, preventing false-positive hangs.

## Phase 1: Infrastructure & Configuration

Goal: Enable the necessary log volume and implement initial noise filtering.

- [x] **Bump Log Level:** Modify `claudine/cli/src/commands/wrap/profile/opencode.rs` to use `--log-level INFO` for the structured stream path.
- [x] **Implement Bus Filtering:** In `claudine/lib/src/stream/logs/opencode/reasoning.rs`, implement aggressive filtering for `service=bus` lines in the `ingest` or `handle_structured` path.
- [x] **Validate Heartbeat:** Verify that byte-heartbeat (`last_byte_at`) continues to refresh on filtered lines (as a backstop) while semantic events only flow for non-filtered lines.

## Phase 2: Log Classification Extension

Goal: Teach the log parser to recognize the lifecycle events needed for semantic promotion.

- [x] **Extend Classification Enum:** Update `LogClassification` in `claudine/lib/src/stream/logs/opencode/events.rs` to include variants for:
    - `BootBanner`
    - `SessionCreated { id: String, parent_id: Option<String> }`
    - `LlmCall { provider_id: String, model_id: String, mode: String, is_stream: bool }`
    - `StepLoop { session_id: String, step: u32 }`
    - `StepExit { session_id: String }`
    - `PermissionEvaluated { permission: String, pattern: String, action: String }`
    - `HttpResponse { method: String, url: String, status: u16, duration_ms: u64 }`
- [x] **Implement Classification Logic:** Update the classification logic (likely in `claudine/lib/src/stream/logs/opencode/errors.rs`) to detect these patterns based on the `service` tag and adjacent fields.
- [x] **Unit Tests for Classifiers:** Add tests to `events.rs` or `errors.rs` verifying that the specific log lines documented in the spec are correctly classified.

## Phase 3: Semantic Event Promotion

Goal: Flow classified log records into the `SemanticEventSink`.

- [x] **Implement Bridge Handlers:** In `claudine/lib/src/stream/logs/opencode/reasoning.rs`, add handler methods (e.g., `on_session_created`, `on_llm_call`) that convert classifications into `SemanticEvent` variants.
- [x] **Map Classifications to Events:**
    - `SessionCreated` (no `parent_id`) → `SemanticEvent::SessionStart`
    - `SessionCreated` (with `parent_id`) → `SemanticEvent::SubagentStart`
    - `LlmCall` → `SemanticEvent::Info` (message: "llm_call_start", extra: { provider, model, mode })
    - `StepLoop` → `SemanticEvent::Info` (message: "step_loop", extra: { session_id, step })
    - `StepExit` → `SemanticEvent::Info` (message: "exiting_loop", extra: { session_id })
    - `PermissionEvaluated` → `SemanticEvent::Info` (message: "permission_evaluated")
    - `HttpResponse` → `SemanticEvent::Info` (message: "http_response")
- [x] **Subagent Stop Detection:** Implement logic to detect `StepExit` for a child session and emit `SemanticEvent::SubagentStop`.

## Phase 4: Integration & Refactoring

Goal: Harmonize the two streams and update metrics/summary logic.

- [x] **Remove NDJSON Synthesis:** In `claudine/lib/src/stream/providers/opencode.rs`, remove the atomic/synthesized `SubagentStart`/`SubagentStop` from `handle_tool_use_completed`.
- [x] **Verify Watchdog Updates:** Ensure `LiveMetricsState::observe_event` correctly refreshes `last_event_at` for the new `Info` events emitted from stderr.
- [x] **Enrich End-of-Run Summary:** Update `StreamExecutionSummary` logic to capture the primary provider/model from the first `mode=primary` LLM call observed on stderr.
- [x] **Deduplication:** Implement logic to ensure that if both streams report the same logical event (e.g., subagent stop), the sink/metrics layer handles it gracefully (first arrival wins, second enriches).

## Phase 5: Documentation

Goal: Ensure the dual-source requirement is never forgotten.

- [x] **Update Architecture Docs:** Add a section to `.claude/skills/claudine/SKILL.md` explaining the "DONE-only" NDJSON limitation and the stderr signal requirement.
- [x] **Create Technical Guide:** Create `.claude/skills/claudine/opencode-event-sources.md` with the full signal-to-event mapping table.
- [x] **Update Timeout Guidelines:** Update `claudine/docs/topics/timeouts.md` to clarify how OpenCode's `step_timeout` interacts with both streams.
- [x] **Research Doc Update:** Ensure the "DONE-only" rule is prominent at the top of `claudine/docs/research/agent-cli/opencode.md`.

## Phase 6: Validation & Acceptance

Goal: Verify the fix with tests and real-world scenarios.

- [x] **Regression Test with Fixture:** Create a regression test in `claudine/lib` using a captured stderr fixture (parent + subagent) to verify the sequence of `SubagentStart`/`Stop` events. — `claudine/lib/tests/opencode_stderr_lifecycle.rs` ingests `claudine/lib/tests/fixtures/logs/opencode-subagent-lifecycle.txt` through `OpenCodeLogBridge` and asserts paired `SubagentStart`/`SubagentStop` per child plus cross-stream dedup behavior.
- [x] **Watchdog Breach Test:** Verify that `step_timeout` still fires correctly if BOTH streams go silent for the full budget. — covered by `watchdog_stream_idle_timeout_after_tool_call_hang` in `claudine/cli/tests/wrap_commands.rs`: the fake opencode binary emits no stderr structured log records and stops emitting stdout after `tool_start`, then idles forever; the wrapper still raises the `step_timeout` breach.
- [ ] **Manual Run:** Perform a `just commit` run with OpenCode and verify:
    - Continuous progress updates in the renderer during subagent work.
    - Correct primary model/provider in the final summary.
    - No false-positive hangs.

    > **Status:** deferred — this acceptance check requires a live OpenCode binary and an interactive session, which is not available in this non-interactive worktree.
