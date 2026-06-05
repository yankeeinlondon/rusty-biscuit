# Always-Harness: Unify Composition Execution Into a Single Pipeline

## Problem

Claudine's composition system has two execution paths for running an agent:

1. **Harness loop** (`harness_orch.rs`) — used when the target document's frontmatter declares `harness:` properties (pre-checks, post-checks, validation handlers, retry logic). The orchestrator runs the agent inside a loop that evaluates a validation plan after each attempt and can retry, resume, or redirect on failure.

2. **Non-harness path** (`composition/mod.rs` → `execute_without_harness`) — used when the document has no `harness:` block. Runs the agent once, applies inline closure if applicable, emits the summary, and exits. No retries, no validation plan, no handler dispatch.

Both paths duplicate approximately 80% of their logic — structured stream plumbing, `StructuredSummaryDetails` construction, summary emission, inline closure application, stdout rendering, and lifecycle guard management. The remaining 20% diverges in ways that are invisible until a feature gap surfaces in production.

### Concrete Example: final_response Leak

The `2026-06-03-inline-compose-final-response` fix introduced a `final_response` accumulator on `StructuredSummaryDetails` that captures only the output text emitted after the agent's last tool call, filtering out interstitial narration ("Let me read the docs…", "Now I'll write the body…").

The fix was implemented in the non-harness path (`composition/structured.rs:run_structured_branch`) but missed the harness loop path (`harness_orch.rs`). Inline-compose documents that declared `harness:` properties still received the full `assistant_text` accumulation — including all narration — into their body content.

This is not an isolated risk. It is a structural consequence of two parallel pipelines that must be kept in sync manually. Any future feature that touches the agent-result-to-file pipeline (body extraction, response filtering, summary enrichment, closure validation) must be implemented and tested in both paths, and there is no compile-time or test-time guarantee that the two stay converged.

## Proposed Solution

Collapse both paths into a single harness loop execution. When a document has no `harness:` properties, the orchestrator synthesizes a minimal validation plan:

- **Pre-checks:** writability check for inline-compose (already exists as `inline_writability_pre_check`), nothing for compose.
- **Post-checks:** none.
- **Handlers:** none.
- **Max retries:** 1 (the agent runs exactly once).

The harness loop with this degenerate plan is functionally identical to the current non-harness path, except it runs through the same code as every other composition. The `execute_without_harness` function and its caller branch in `composition/mod.rs` are removed entirely.

### Scope

- **In scope:**
  - Refactor `composition/mod.rs` to always route through `run_harness_loop`.
  - Remove `execute_without_harness` and the `CompositionExecutionMode` enum.
  - Remove `composition/structured.rs::run_structured_branch` (its logic moves into the harness loop's per-attempt function or is already present there).
  - Remove `composition/inline_guards.rs::apply_inline_closure` (the harness loop already has its own inline closure path via `inline::try_inline_closure`).
  - Ensure the degenerate harness plan adds zero measurable overhead compared to the current non-harness path.

- **Out of scope:**
  - Changes to the harness validation plan format or handler system.
  - Changes to provider profiles or stream plumbing.
  - Changes to sequence execution (which already delegates to the harness loop per-step).

### Degenerate Plan Construction

A new helper `harness_plan_for_bare_composition` builds a `HarnessPlan` with:

| Field | Value |
|-------|-------|
| `source_path` | The resolved composition file path |
| `pre_checks` | `[inline_writability_pre_check]` for inline, `[]` for compose |
| `post_checks` | `[]` |
| `handlers` | `[]` |
| `max_retries` | `0` (one attempt, no retry) |

The harness loop's existing attempt-count guard (`attempt > max_retries → fail`) means the loop body executes exactly once and returns. No retry logic, no handler dispatch, no plan re-parsing.

### Lifecycle Emission

The current non-harness path manually manages a `LifecycleRunGuard` (start-once, mark-launched, emit success/failure). The harness loop already does this internally. Unification removes the manual guard in favor of the loop's built-in lifecycle management.

### Summary Emission

The non-harness path has a bespoke summary emission path (`summary::emit_composition_summary`). The harness loop calls `policy::emit_stream_summary` with the same inputs. After unification, all composition runs use the harness loop's summary path. If the non-harness path's `defer_section_separator` behavior differs from the harness loop's section-stream emission, the harness loop's approach wins (it is the more carefully managed path).

### Migration Steps

1. **Add convergence tests.** Before any structural changes, add integration tests that assert identical behavior for the same inline-compose invocation with and without a `harness:` block. These tests are the safety net that proves unification does not regress either path.

2. **Extract `harness_plan_for_bare_composition`.** Build a minimal plan from a composition request that has no `harness:` properties. Unit-test it in isolation.

3. **Route all composition through `run_harness_loop`.** In `execute_composition`, replace the `if harness_enabled { … } else { execute_without_harness(…) }` branch with a single `run_harness_loop` call that uses either the parsed harness plan or the synthesized bare plan.

4. **Remove dead code.** Delete `execute_without_harness`, `CompositionExecutionMode`, `run_structured_branch`, `apply_inline_closure` (the `inline_guards.rs` version), and any related match arms.

5. **Verify.** Run the full test suite. The convergence tests from step 1 must continue to pass unchanged.

### Risk

The harness loop is more complex than the non-harness path. Introducing a bug in the degenerate case could affect all composition runs, not just harness-enabled ones. Mitigation:

- The convergence tests from step 1 provide immediate regression detection.
- The degenerate plan is trivial (no checks, no handlers, one attempt) so the loop body reduces to: spawn agent → collect result → apply inline closure if applicable → emit summary. This is exactly what `execute_without_harness` does today.
- If a critical bug is discovered post-merge, reverting to the two-path design is straightforward because the non-harness code is deleted in a single commit (step 4).

### Relationship to Existing Code

The `composition/structured.rs::run_structured_composition` helper (the shared structured-stream runner) is already called by both paths. After unification it remains the shared runner — the harness loop's per-attempt function calls it, and the degenerate plan uses the same call. No duplication is introduced; existing duplication is removed.
