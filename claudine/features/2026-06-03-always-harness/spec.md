---
status: ready for planning and implementation
reviewed: true
---

# Always-Harness: Unify Composition Execution Into a Single Pipeline

## Problem

Claudine's composition system has two execution paths for running an agent:

1. **Harness loop** (`harness_orch.rs`) - used when the target document's effective frontmatter declares harness properties such as `pre_checks`, `post_checks`, `timeout`, `step_timeout`, or handlers. The orchestrator runs the agent inside a loop that evaluates a validation plan after each attempt and can retry, resume, redirect, or deviate on failure.

2. **Non-harness path** (`composition/mod.rs` -> `execute_without_harness`) - used when the document has no harness properties. It runs the agent once, applies inline closure when applicable, emits the summary, and exits. No validation plan or handler dispatch exists on this path.

Both paths duplicate approximately 80% of their logic: structured stream plumbing, `StructuredSummaryDetails` construction, summary emission, inline closure application, stdout rendering, timeout wiring, performance collection, and lifecycle guard management. The remaining 20% diverges in ways that are invisible until a feature gap surfaces in production.

### Concrete Example: `final_response` Leak

The `2026-06-03-inline-compose-final-response` fix introduced a `final_response` accumulator on `StructuredSummaryDetails` that captures only the output text emitted after the agent's last tool call, filtering out interstitial narration such as "Let me read the docs..." or "Now I'll write the body...".

The fix was implemented in the non-harness path (`composition/structured.rs::run_structured_branch`) but missed the harness loop path (`harness_orch.rs`). Inline-compose documents that declared harness properties still received the full `assistant_text` accumulation, including process narration, into their body content.

This is not an isolated risk. It is a structural consequence of two parallel pipelines that must be kept in sync manually. Any future feature that touches the agent-result-to-file pipeline - body extraction, response filtering, summary enrichment, closure validation, timeout reporting, lifecycle reporting, or performance metrics - must be implemented and tested in both paths, and there is no compile-time or test-time guarantee that the two stay converged.

## Proposed Solution

Collapse both paths into a single harness-loop execution path for every non-dry-run `compose` and `inline-compose` invocation.

When a document has no harness properties, the composition layer uses a **bare harness plan**: the same `HarnessPlan` shape produced by `parse_harness_plan`, but with no author-declared validations or handlers. For inline composition, the existing system-owned `inline_writability_pre_check` is still inserted before execution. For direct composition, the plan has no checks.

The harness loop with a bare plan is functionally equivalent to the current non-harness path:

- no author pre-checks for direct compose,
- one system-owned writability pre-check for inline compose,
- no post-checks,
- no handlers,
- no handler-driven retries,
- one provider launch,
- inline closure through `wrap/inline.rs::try_inline_closure`,
- summary emission through the harness summary path.

The `execute_without_harness` function and its caller branch in `composition/mod.rs` are removed entirely.

### Reader's Note

An earlier draft described adding `max_retries: 0` to `HarnessPlan`. That does not match the current contract: `HarnessPlan` has no retry-limit field, and `run_harness_loop` only consults `DEFAULT_MAX_RETRIES` when a handler has resolved a failure into another attempt. A bare plan has no handlers, so no retry path exists. The design therefore keeps retry limits out of `HarnessPlan` and relies on handler absence to guarantee one launch.

## Scope

### In Scope

- Refactor `composition/mod.rs` so every non-dry-run composition routes through `run_harness_loop`.
- Use the parsed harness plan when harness properties exist and a synthesized bare plan when they do not.
- Remove `execute_without_harness` and the `CompositionExecutionMode` enum.
- Remove `composition/structured.rs::run_structured_branch` once its logic has moved into or is already covered by the harness attempt path.
- Remove `composition/inline_guards.rs::apply_inline_closure`; inline closure must consistently flow through `wrap/inline.rs::try_inline_closure`.
- Preserve direct compose and inline-compose CLI behavior unless this spec explicitly calls out an intentional behavior change.
- Update docs and comments that still describe separate harness and non-harness execution paths.

### Out of Scope

- Changing the user-authored harness validation format.
- Adding per-document retry-limit syntax.
- Changing handler action semantics.
- Changing provider profiles or stream parsing beyond the wiring needed to use the existing harness attempt path.
- Changing sequence orchestration semantics. `sequence` already delegates to composition per step; it should benefit from this refactor without gaining a new sequence-specific execution model.
- Moving `--dry-run` into the harness launch loop.

## Bare Plan Construction

Add a small helper for bare plan construction. It may live in the library harness layer if tests or future callers need it there, or in the CLI composition layer if it is purely orchestration glue. The helper must return a normal `HarnessPlan`:

| Field | Direct `compose` | `inline-compose` |
|-------|------------------|------------------|
| `source_path` | resolved composition file path | resolved composition file path |
| `timeout` | `None` | `None` |
| `step_timeout` | `None` | `None` |
| `timeout_warn` | `None` | `None` |
| `step_timeout_warn` | `None` | `None` |
| `pre_checks` | `[]` | `[inline_writability_pre_check(source_path)]` |
| `post_checks` | `[]` | `[]` |
| `handlers` | empty table | empty table |
| `programmatic_handler` | `None` | `None` |

The helper must not add a retry field. One-attempt behavior follows from the empty handler table.

If implementation can reuse `parse_harness_plan` against empty effective frontmatter without a dedicated helper, that is acceptable, but tests must still pin the resulting bare-plan shape for direct and inline composition.

## Execution Semantics

### Dry Run

`--dry-run` remains a pre-launch composition feature. It must continue to return before `run_harness_loop` is called, while still performing the existing preflight work that dry-run promises:

- Darkmatter composition and shell expansion,
- schema validation,
- shell approval,
- harness plan parsing when harness frontmatter exists,
- inline writability validation when applicable,
- provider and model resolution,
- dry-run stdout/stderr rendering.

The dry-run output split remains unchanged: composed body to stdout, finalized frontmatter and metadata to stderr, and no source-file mutation.

### Lifecycle

The outer `LifecycleRunGuard` in `execute_composition_request` must be defused before calling `run_harness_loop` for both parsed and bare plans. The harness loop becomes the single lifecycle owner for non-dry-run composition.

Pre-launch failures still emit blocked/failure lifecycle signals according to the existing guard contract. Provider-launched failures still emit failure. Successful runs emit success once.

### Provider Failure Exit Codes

The refactor must preserve the non-harness contract that a provider process exit is surfaced as that provider's exit code. This is especially important for direct `compose`, shell scripts, and loop orchestration that interpret non-zero provider status.

Design decision: unhandled provider failure inside the harness loop should emit the existing failure report and lifecycle failure signal, then return `Ok((outcome.exit_code, perf))` rather than converting the provider exit into a generic `eyre` error. Validation failures, shell-audit failures, malformed plans, and inline-closure validation failures remain structured Claudine errors because they are Claudine-side failures rather than provider process status.

This is an intentional tightening of the harness contract, not an accidental side effect. It aligns parsed-harness and bare-plan composition with the established CLI expectation that provider exit status is observable by callers.

### Timeout and Warning Resolution

Bare plans have no frontmatter timeout values, so timeout resolution must continue to behave like today's non-harness path:

1. CLI `--timeout` / `--step-timeout`,
2. env-var defaults,
3. built-in defaults.

Parsed harness plans keep the existing precedence:

1. CLI flags,
2. frontmatter `timeout` / `step_timeout`,
3. env-var defaults,
4. built-in defaults.

Bare plans do not support `timeout_warn` or `step_timeout_warn` because those fields are frontmatter-only harness properties. The periodic prompt-scoped timing header still appears for all non-dry-run composition runs.

### Summary Emission

The non-harness path has a bespoke summary emission path (`summary::emit_composition_summary`). The harness loop calls the policy summary emitter with the same structured details. After unification, all non-dry-run composition runs use the harness loop's summary path.

If the old `defer_section_separator` behavior differs from the harness loop's section-stream emission, the harness loop's approach wins, provided inline-closure validation messages remain readable and are not split by a stray section separator. Tests should assert the intended order rather than snapshotting incidental blank-line differences.

### Inline Closure

Inline closure must use the structured `final_response` captured after the last tool call. It must not fall back to accumulated `assistant_text` except for providers that explicitly recover a post-hoc final message through their supported mechanism.

Closure remains before post-check evaluation in the harness loop so file-state checks observe the final rewritten document.

## Migration Steps

1. **Add convergence tests.** Before structural changes, add tests that compare the same `compose` and `inline-compose` invocations with and without minimal harness frontmatter. The inline case must include interstitial assistant narration before a tool call and final body content after the last tool call; both variants must write the same final body.

2. **Add or pin bare-plan construction.** Unit-test the direct and inline bare-plan shapes. Confirm there is no retry field and that inline mode gets exactly one system-owned `HasWritePermission` pre-check.

3. **Preflight both parsed and bare plans.** Keep template shell approval unchanged. Run harness shell approval against the parsed or bare plan. Remove the separate non-harness inline permission branch after inline writability is represented by the bare plan.

4. **Route all composition through `run_harness_loop`.** Build `HarnessPromptState` for both direct and inline composition and pass the same base args, env, child cwd, structured output settings, noise filters, stream verbosity, dispatch context, timeout CLI values, and materialized prompt data that the current harness branch receives.

5. **Preserve result surfaces.** Ensure provider failure exit codes, lifecycle signals, performance collection, prompt timing, summary emission, and inline closure behavior match the contracts above.

6. **Remove dead code.** Delete `execute_without_harness`, `CompositionExecutionMode`, `run_structured_branch`, the obsolete `inline_guards` closure wrapper, and any legacy branch modules or match arms that have no remaining call sites.

7. **Clean documentation drift.** Update `claudine/docs/topics/composition.md`, `claudine/docs/topics/non-interactive-sessions.md`, `claudine/docs/topics/execution-flow.md`, `claudine/docs/pipeline.md`, and the Claudine skill docs if they still describe separate harness and non-harness execution paths.

8. **Verify.** Run the new convergence tests, targeted existing composition tests, `cargo check -p claudine -p claudine-cli --color=never`, and the claudine-area test recipe that matches current repo convention.

## Acceptance Criteria

- `rg -n "execute_without_harness|CompositionExecutionMode|run_structured_branch|inline_guards|non-harness path|without harness" claudine/cli/src claudine/docs claudine/.claude/skills` returns no live-code or current-doc references to the removed path.
- Bare direct compose and parsed-harness direct compose both execute through `run_harness_loop`.
- Bare inline compose and parsed-harness inline compose both execute through `run_harness_loop`.
- Inline body replacement uses final response only in both bare and parsed-harness runs.
- A provider process that exits non-zero preserves that exit code at the CLI boundary.
- Dry-run does not launch the provider and does not mutate inline source files.
- Lifecycle notifications fire once per run.
- Timeout precedence remains CLI > frontmatter > env > built-in for parsed harness plans and CLI > env > built-in for bare plans.
- Targeted convergence and composition tests pass.

## Risk

The harness loop is more complex than the non-harness path. Introducing a bug in the bare-plan case could affect every composition run, not just harness-enabled ones.

Mitigations:

- Add convergence tests before removing the old path.
- Keep the bare plan trivial and handler-free.
- Preserve dry-run outside the provider launch loop.
- Preserve provider failure exit codes explicitly rather than relying on generic error propagation.
- Delete the old path only after the harness route compiles and tests pass, so the final diff clearly separates migration from cleanup.

## Open Questions

None. The reviewed design decisions are:

- Do not add `max_retries` to `HarnessPlan`; handler absence guarantees one attempt for bare plans.
- Keep dry-run outside `run_harness_loop`.
- Preserve provider exit codes through the harness loop.
- Use the harness summary and inline-closure paths as the single source of truth after unification.
