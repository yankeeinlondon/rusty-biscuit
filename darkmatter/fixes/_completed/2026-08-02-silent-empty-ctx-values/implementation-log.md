---
implementation_1: "2026-09-18T00:46:57-07:00"
fix: 2026-08-02-silent-empty-ctx-values
plan: plan.md
source_files_during_phase_1:
- darkmatter/lib/tests/ambient_ctx_capture.rs
- darkmatter/lib/src/markdown/compose/context/epoch_spike_tests.rs
- darkmatter/lib/src/markdown/compose/context/mod.rs
docs_updated_during_phase_1:
- darkmatter/fixes/2026-08-02-silent-empty-ctx-values/plan.md
- darkmatter/fixes/2026-08-02-silent-empty-ctx-values/spec.md
- darkmatter/fixes/2026-08-02-silent-empty-ctx-values/implementation-log.md
docs_created_during_phase_1:
- darkmatter/fixes/2026-08-02-silent-empty-ctx-values/surfaces.md
- darkmatter/fixes/2026-08-02-silent-empty-ctx-values/error-taxonomy.md
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
- darkmatter/lib/src/markdown/compose/context/checked.rs
- darkmatter/lib/src/markdown/compose/context/mod.rs
- darkmatter/lib/src/markdown/compose/context/runtime.rs
- darkmatter/lib/src/markdown/compose/context/effective_state.rs
- darkmatter/lib/src/markdown/compose/context/capture/groups.rs
- darkmatter/lib/src/markdown/compose/expression/error.rs
- darkmatter/lib/src/markdown/compose/expression/mod.rs
- darkmatter/lib/src/markdown/compose/expression/ctx.rs
- darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs
- darkmatter/lib/src/markdown/compose/interpolation/fatality_characterization.rs
- darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs
- darkmatter/lib/src/markdown/compose/subtree.rs
- darkmatter/lib/src/markdown/compose/conditions.rs
- darkmatter/lib/tests/ambient_ctx_capture.rs
docs_updated_during_phase_2:
- darkmatter/docs/topics/darkmatter-expressions.md
- darkmatter/fixes/2026-08-02-silent-empty-ctx-values/plan.md
- darkmatter/fixes/2026-08-02-silent-empty-ctx-values/error-taxonomy.md
- darkmatter/fixes/2026-08-02-silent-empty-ctx-values/implementation-log.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2:
- .claude/skills/darkmatter/compose.md
source_files_during_phase_3:
- darkmatter/lib/src/markdown/compose/context/checked.rs
- darkmatter/lib/src/markdown/compose/context/effective_state.rs
- darkmatter/lib/src/markdown/compose/context/options.rs
- darkmatter/lib/src/markdown/compose/context/runtime.rs
- darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs
- darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs
- darkmatter/lib/src/markdown/compose/expression/ctx.rs
- darkmatter/lib/src/markdown/compose/expression/error.rs
- darkmatter/lib/src/markdown/compose/interpolation/fatality_characterization.rs
- darkmatter/lib/src/markdown/compose/conditions.rs
- darkmatter/lib/src/markdown/compose/transclusion/conditions.rs
- darkmatter/lib/src/markdown/compose/transclusion/types.rs
- darkmatter/lib/src/markdown/compose/shell_expansion/types.rs
- darkmatter/lib/src/markdown/compose/shell_blocks/types.rs
- darkmatter/lib/src/markdown/compose/pipeline/phases.rs
- darkmatter/lib/src/markdown/compose/preflight/collect.rs
- darkmatter/lib/src/markdown/compose/inline/interpolation.rs
- darkmatter/lib/src/markdown/errors/blocks.rs
- darkmatter/lib/src/markdown/types.rs
- darkmatter/lib/tests/missing_ctx_capture.rs
- darkmatter/lib/tests/ambient_ctx_capture.rs
- darkmatter/lib/tests/error_snapshots/condition.rs
- darkmatter/lib/tests/error_snapshots/transclusion.rs
- darkmatter/lib/tests/error_snapshots/snapshots/error_snapshots__condition__eval.snap
- darkmatter/lib/tests/error_snapshots/snapshots/error_snapshots__transclusion__condition_eval.snap
- darkmatter/cli/tests/compose_interpolation.rs
docs_updated_during_phase_3:
- darkmatter/docs/topics/darkmatter-expressions.md
- darkmatter/fixes/2026-08-02-silent-empty-ctx-values/plan.md
- darkmatter/fixes/2026-08-02-silent-empty-ctx-values/error-taxonomy.md
- darkmatter/fixes/2026-08-02-silent-empty-ctx-values/implementation-log.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3:
- .claude/skills/darkmatter/compose.md
source_files_during_phase_4:
- darkmatter/lib/src/markdown/compose/context/authority.rs
- darkmatter/lib/src/markdown/compose/context/mod.rs
- darkmatter/lib/src/markdown/compose/context/runtime.rs
- darkmatter/lib/src/markdown/compose/context/options.rs
- darkmatter/lib/src/markdown/compose/context/capture/mod.rs
- darkmatter/lib/src/markdown/compose/context/capture/groups.rs
- darkmatter/lib/src/markdown/compose/context/epoch_spike_tests.rs
- darkmatter/lib/src/markdown/compose/mod.rs
- darkmatter/lib/src/markdown/compose/pipeline/mod.rs
- darkmatter/lib/src/markdown/compose/preflight/collect.rs
- darkmatter/lib/src/markdown/compose/shell_expansion/types.rs
- darkmatter/lib/src/markdown/compose/transclusion/engine.rs
- darkmatter/lib/src/markdown/compose/cache/mod.rs
- darkmatter/lib/src/markdown/compose/cache/runtime.rs
- darkmatter/lib/src/markdown/compose/cache/hashing.rs
- darkmatter/lib/src/markdown/compose/cache/manifest.rs
- darkmatter/lib/src/markdown/compose/cache/store.rs
- darkmatter/lib/tests/request_context_epoch.rs
- darkmatter/lib/tests/ambient_ctx_capture.rs
- darkmatter/lib/tests/shell_block_integration.rs
- darkmatter/cli/src/commands/compose.rs
- claudine/lib/src/invocation_context.rs
- claudine/lib/src/composition/prepare.rs
- claudine/lib/src/system_prompt/prepare.rs
- claudine/cli/src/commands/compose/prep.rs
- claudine/cli/src/commands/wrap/sequence/mod.rs
- claudine/cli/src/commands/wrap/sequence/jit.rs
- claudine/cli/src/commands/wrap/overlay.rs
docs_updated_during_phase_4:
- darkmatter/docs/topics/darkmatter-expressions.md
- darkmatter/fixes/2026-08-02-silent-empty-ctx-values/plan.md
- darkmatter/fixes/2026-08-02-silent-empty-ctx-values/implementation-log.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
- .claude/skills/darkmatter/compose.md
packages:
- darkmatter
human_review: true
human_review_items:
- Phase 4's ContextAuthority shape (CallerSupplied / DarkmatterOwned / CallerExtended) had no author confirmation
  before Phase 5. Phase 5 has now documented it as the public contract in docs/topics/context-variables.md
  and the compose skill. If the shape changes in review, those two documents change with it.
- Rulings R1-R8 (Phase 1) are still provisional. There was no author response across Phases 2-5, and the
  implementation and documentation follow them as recorded.
message_to_agent: 'All 5 phases are implementation-complete and ready for review. verification-matrix.md
  maps spec items #1-#12 to named tests. Do not move the fix to _completed; the author does that after
  review.'
source_files_during_phase_5:
- darkmatter/lib/tests/request_context_epoch.rs
- darkmatter/lib/tests/reference_integration.rs
- darkmatter/lib/src/markdown/compose/conditions.rs
- darkmatter/lib/src/markdown/compose/context/options.rs
docs_updated_during_phase_5:
- darkmatter/docs/topics/context-variables.md
- darkmatter/docs/topics/caching.md
- darkmatter/fixes/2026-08-02-silent-empty-ctx-values/plan.md
- darkmatter/fixes/2026-08-02-silent-empty-ctx-values/implementation-log.md
docs_created_during_phase_5:
- darkmatter/fixes/2026-08-02-silent-empty-ctx-values/verification-matrix.md
skills_files_updated_during_phase_5:
- .claude/skills/darkmatter/compose.md
source_code:
- darkmatter/lib/tests/ambient_ctx_capture.rs
- darkmatter/lib/src/markdown/compose/context/mod.rs
- darkmatter/lib/src/markdown/compose/context/checked.rs
- darkmatter/lib/src/markdown/compose/context/runtime.rs
- darkmatter/lib/src/markdown/compose/context/effective_state.rs
- darkmatter/lib/src/markdown/compose/context/capture/groups.rs
- darkmatter/lib/src/markdown/compose/expression/error.rs
- darkmatter/lib/src/markdown/compose/expression/mod.rs
- darkmatter/lib/src/markdown/compose/expression/ctx.rs
- darkmatter/lib/src/markdown/compose/interpolation/evaluator.rs
- darkmatter/lib/src/markdown/compose/interpolation/fatality_characterization.rs
- darkmatter/lib/src/markdown/compose/frontmatter_interpolation.rs
- darkmatter/lib/src/markdown/compose/subtree.rs
- darkmatter/lib/src/markdown/compose/conditions.rs
- darkmatter/lib/src/markdown/compose/context/options.rs
- darkmatter/lib/src/markdown/compose/frontmatter_shell_expansion.rs
- darkmatter/lib/src/markdown/compose/transclusion/conditions.rs
- darkmatter/lib/src/markdown/compose/transclusion/types.rs
- darkmatter/lib/src/markdown/compose/shell_expansion/types.rs
- darkmatter/lib/src/markdown/compose/shell_blocks/types.rs
- darkmatter/lib/src/markdown/compose/pipeline/phases.rs
- darkmatter/lib/src/markdown/compose/preflight/collect.rs
- darkmatter/lib/src/markdown/compose/inline/interpolation.rs
- darkmatter/lib/src/markdown/errors/blocks.rs
- darkmatter/lib/src/markdown/types.rs
- darkmatter/lib/tests/missing_ctx_capture.rs
- darkmatter/lib/tests/error_snapshots/condition.rs
- darkmatter/lib/tests/error_snapshots/transclusion.rs
- darkmatter/lib/tests/error_snapshots/snapshots/error_snapshots__condition__eval.snap
- darkmatter/lib/tests/error_snapshots/snapshots/error_snapshots__transclusion__condition_eval.snap
- darkmatter/cli/tests/compose_interpolation.rs
- darkmatter/lib/src/markdown/compose/context/authority.rs
- darkmatter/lib/src/markdown/compose/context/capture/mod.rs
- darkmatter/lib/src/markdown/compose/mod.rs
- darkmatter/lib/src/markdown/compose/pipeline/mod.rs
- darkmatter/lib/src/markdown/compose/transclusion/engine.rs
- darkmatter/lib/src/markdown/compose/cache/mod.rs
- darkmatter/lib/src/markdown/compose/cache/runtime.rs
- darkmatter/lib/src/markdown/compose/cache/hashing.rs
- darkmatter/lib/src/markdown/compose/cache/manifest.rs
- darkmatter/lib/src/markdown/compose/cache/store.rs
- darkmatter/lib/tests/request_context_epoch.rs
- darkmatter/lib/tests/shell_block_integration.rs
- darkmatter/cli/src/commands/compose.rs
- claudine/lib/src/invocation_context.rs
- claudine/lib/src/composition/prepare.rs
- claudine/lib/src/system_prompt/prepare.rs
- claudine/cli/src/commands/compose/prep.rs
- claudine/cli/src/commands/wrap/sequence/mod.rs
- claudine/cli/src/commands/wrap/sequence/jit.rs
- claudine/cli/src/commands/wrap/overlay.rs
- darkmatter/lib/tests/reference_integration.rs
documentation:
- darkmatter/fixes/2026-08-02-silent-empty-ctx-values/plan.md
- darkmatter/fixes/2026-08-02-silent-empty-ctx-values/spec.md
- darkmatter/fixes/2026-08-02-silent-empty-ctx-values/implementation-log.md
- darkmatter/fixes/2026-08-02-silent-empty-ctx-values/surfaces.md
- darkmatter/fixes/2026-08-02-silent-empty-ctx-values/error-taxonomy.md
- darkmatter/docs/topics/darkmatter-expressions.md
- darkmatter/docs/topics/context-variables.md
- darkmatter/docs/topics/caching.md
- darkmatter/fixes/2026-08-02-silent-empty-ctx-values/verification-matrix.md
completed_phase: '5'
implemented: true
---

# Implementation Log: A missing runtime value must not render as nothing

## Phase 1

### Baseline (untouched tree, before any Phase 1 change)

- HEAD `914ac2188`; `just test` (darkmatter): **7842 passed, 7 skipped, 7 flagged slow** (68.6s wall). `just lint`: green (includes zed-dmls wasm32-wasip2 check).
- The nextest slow-test threshold is `slow-timeout.period = "5s"` (`.config/nextest.toml`); the 7 slow flags are pre-existing.

### Repro characterization (Work-group A)

Added three pre-change baseline tests to `lib/tests/ambient_ctx_capture.rs`, reusing its purpose-built fixture repository (no rusty-biscuit walk):

| Test | Pins today's behavior | Later phase flips it to |
|---|---|---|
| `baseline_ambient_root_resolves_the_regression_input` | ambient root renders `repo_root`, `os`, `today` identical to a full capture (the 2026-08-02 root repair holds) | stays green (positive control) |
| `baseline_caller_supplied_minimal_context_renders_uncaptured_groups_empty` | `new_with_context(capture_minimal())` composes **successfully** with `repo_root=` and `os=` empty, `today` non-empty | Phase 3: typed fatal error naming `ctx.repo_root`, `Repo`, source (verification #1) |
| `baseline_ambient_child_first_reference_renders_empty` | root with no ctx ref + child `os={{ ctx.os }}` under `ComposeOptions::new()` renders `os=` empty; a full capture renders it | Phase 4: ambient extension renders the full-capture value (verification #7) |

All use the spec's verbatim failing input (`repo_root={{ ctx.repo_root }}|os={{ ctx.os }}|today={{ ctx.today }}`) where applicable. All 5 tests in the binary pass (~0.4s).

Gotcha: a file-based root needs `.with_source_file(root)` or transclusion fails with `MissingSourceContext`.

### Surface survey (Spike B, Work-group A)

- Wrote `surfaces.md`, which lists 15 evaluation surfaces plus the lookup implementations. An Explore subagent ran the survey; I spot-checked the key claims against source (`rewrite.rs:165`, `phases.rs:327-410`, `engine.rs:1420-1530`, `hashing.rs:103-160`, `effective_state.rs:228-300`, `frontmatter_shell_expansion.rs:1536`, `expression/ctx.rs:40`, `error.rs:372`).
- Findings that change the plan's assumptions:
  1. **Tolerated transclusion swallows child errors.** `phases.rs:376-410` turns non-structural child failures into a notice plus a warning unless `fail_fast`. New errors must be structural there (R2 refinement).
  2. **Stringly wrappers.** `ConditionError` (page-block and transclusion `when=`) and `ShellExpansionError::ParseDirective` (`$()` ternary) flatten `ExpressionError` into strings, so they need a typed `#[source]`.
  3. **Pre-flight runs before the root upgrade.** Pre-flight passes (#13a/b) and `reference/graph.rs` `when=` evaluate against the non-upgraded context, so they must tolerate missing-capture (R3 extension).
  4. **Bare-name fallback is never captured.** `when="repo"` resolves through `EffectiveState::get`, but no requirement scan ever captures a bare name's group. This became new Ruling R8, flagged for the author.
  5. **`EvaluationLookup` has no error channel.** The taxonomy adds a defaulted `get_checked`.
  6. **`fixed_for_testing` is malformed under R7.** It marks DateTime captured but projects 11 of 37 keys; `fixed_for_testing_with` inserts values for unmarked groups. Phase 2 must repair these fixtures first.

### Error taxonomy (Work-group A)

- Wrote `error-taxonomy.md`. It defines `CtxLookupOutcome { Present, NotCaptured, ProjectionMissing, Unknown }` (crate-private).
- It adds public `ExpressionError::{ContextNotCaptured, ContextProjectionInvariant} { key, group: ContextGroup }`. Both are authoring-fatal, and source identity comes from the existing `MarkdownError::Interpolation` wrapper. `ContextGroup` is already publicly re-exported.
- It covers the `get_checked` trait channel, the wrapper table, and the `with_projection_key_removed` test hook.

### Epoch spike (Spike A, Work-group B)

- **Decision** (recorded in `plan.md` under "Spike A Decision"): a request-epoch cell (`Arc<RwLock<ComposeContext>>` plus a version counter) owned by `PipelineRuntime`, shared by `clone_for_child`.
  - Authority lives in a `ContextAuthority` enum on `ComposeOptions`.
  - The hook sits in `render_markdown_transclusion` before the cache key: memoized load, then set overlay, then `for_document` requirements, then ensure.
  - Cache identity direction: the subtree context-group closure is recorded with the result, and identity is hashed over those groups only.
- Rejected alternatives: a cell in `ComposeOptions` (aliasing across cloned requests, fingerprint), and a preflight-graph eager closure as the authority (optional, pre-evaluates `when=` on the non-upgraded context, blind to dynamic and remote children).
- PoC, test-only (`lib/src/markdown/compose/context/epoch_spike_tests.rs`, registered `#[cfg(test)]` in `context/mod.rs`):
  - `shared_cell_extends_once_and_every_sibling_reads_the_same_projection`: 32 Rayon workers, exactly one extension, identical `repo_root` for all, and the pre-extension snapshot unchanged (copy-on-write).
  - `phase_hoisted_context_hash_cannot_see_a_child_introduced_group`: the hoisted hash is identical for two requests whose child-only `repo_root` differs; post-extension hashes differ.
  - A third draft test (whether `context_hash` ignores the captured-group set) was **removed**: its assertion was conditional and could pass vacuously. A captured group's `null` keys are present in the map, so values already imply the marker while projections are complete. `surfaces.md` was softened to say so.
  - Replaced a literal `"/request/repo"` assertion with a non-empty check so Windows path spelling cannot break it.

### Rulings

- R1–R7 are adopted provisionally with evidence; R8 (bare-name fallback) is proposed. All are recorded in `plan.md` under "Ruling Decisions".
- The session was non-interactive, so no author confirmation was possible: `human_review: true`.

### Gates after Phase 1 changes

- `just test` (darkmatter): **7847 passed, 7 skipped, 1 flagged slow** (baseline 7842 plus 5 new tests). No new test is slow; the new tests run in ≤0.4s.
- `just lint`: green. `cargo clippy -p darkmatter --tests`: no warnings.
- `just test-l2` was not run: no terminal or browser surface was touched.
- Cross-OS: not run remotely. The new tests reuse the existing cross-platform git fixture, use temp dirs, and make no path-string comparisons.
- No production code changed, so GitNexus impact analysis was not required. The only non-test edit is a `#[cfg(test)] mod` declaration.

### Requirement → test mapping (Phase 1 scope)

| Phase 1 requirement | Test / artifact |
|---|---|
| Repro through ambient `ComposeOptions::new()` (verbatim input) | `ambient_ctx_capture::baseline_ambient_root_resolves_the_regression_input` |
| Repro through a minimal caller-supplied context | `ambient_ctx_capture::baseline_caller_supplied_minimal_context_renders_uncaptured_groups_empty` |
| Child-first reference (transclusion half of the defect) | `ambient_ctx_capture::baseline_ambient_child_first_reference_renders_empty` |
| Spike A sibling visibility and once-only capture | `context::epoch_spike_tests::shared_cell_extends_once_and_every_sibling_reads_the_same_projection` |
| Spike A pre-extension hash mismatch | `context::epoch_spike_tests::phase_hoisted_context_hash_cannot_see_a_child_introduced_group` |
| Surface inventory | `surfaces.md` |
| Error contract | `error-taxonomy.md` |

## Phase 2

### Starting point

- Phase 1's `human_review` items (R1–R8, Spike A) had no visible author response. The plan allows Phase 2 to proceed on the provisional rulings, so this phase implements them as recorded.
- GitNexus upstream impact before editing:
  - `EffectiveState::get_context_value`: LOW, 2 direct callers.
  - `EvaluationLookup`: HIGH, 17 direct / 71 total, including Claudine's `SourceExpressionLookup`.
  - `expression::evaluate`: HIGH, 177 impacted.
  - `ComposeContext::fixed_for_testing`: CRITICAL, 378 test-only callers.
  - Mitigation: the trait change is an additive defaulted method. Compose behavior for uncaptured groups is unchanged (staged; see the decision below). The fixture change only adds `null` projections and marks groups captured. The full suite and a Claudine `cargo check --tests` confirm it.

### Decision: the uncaptured-group error is staged, not yet live

The plan contradicts itself here. The "evaluator checked path" task maps `NotCaptured` to the new error. The Phase 2 checkpoint requires "no existing behavior change observable from documents — fatality wiring is Phase 3", and the Phase 1 baseline tests (flipped in Phases 3/4) pin today's empty output. Enforcing `NotCaptured` now would also break real CLI composition before Phase 3's tolerance work: pre-flight ternary discovery (#13b) runs against the non-upgraded context. Resolution:

- The full checked path exists and is exercised by tests.
- `ProjectionMissing` → `ContextProjectionInvariant` is **live** everywhere. It cannot fire on a legitimate capture, and the completeness tests prove that.
- `NotCaptured` → `ContextNotCaptured` is gated by the crate-private `context::checked::COMPOSE_NOT_CAPTURED_POLICY = NotCapturedPolicy::Legacy`, which returns the pre-classification answer. Tests force `Fatal` via `EffectiveState::get_checked_with`. Phase 3 flips the constant and deletes the policy.

### What landed

- `context/checked.rs`: `CtxLookupOutcome { Present, NotCaptured, ProjectionMissing, Unknown }` with one classifier (`classify`), `into_checked`, and `into_checked_bare_name` (R8), plus `NotCapturedPolicy` and the staged constant.
- `ContextGroup::projected_keys()` (`capture/groups.rs`): one key table, date/time aliases included. `group_for_key` now derives from it, so there is no second list.
- `ComposeContext::classify_ctx_key` (`runtime.rs`).
- Fixture repair (R7):
  - `fixed_for_testing` projects every DateTime key (`null` for unset).
  - `fixed_for_testing_with` marks each inserted key's group captured and projects `null` for that group's other keys.
  - New `#[cfg(test)] with_projection_key_removed` hook.
- `ExpressionError::{ContextNotCaptured, ContextProjectionInvariant} { key, group }` are public, and both are `is_authoring_fatal()`.
- `EvaluationLookup::get_checked` is defaulted to `Ok(self.get(path))`. `evaluate`'s `Expr::Variable` and the `Evaluator::eval` variable fast path both read through it.
- Overrides:
  - `EffectiveState`: R1 group check before authored `ctx`; present values keep the authored-then-projection merge order.
  - `ResolvingLookup` and `LayeredLookup`: globals keep precedence, then delegate.
  - `FrontmatterSeedState`: effective values.
  - `ShortcutLookup`.
  - `CtxLookup`: new public `resolve_ctx_checked`. `resolve_ctx` keeps its `Option` contract, and lazy per-group capture is unchanged.

### Silent-path audit (Work-group D)

Each compose stage and the lookup it reads `ctx.*` through:

| Stage (surfaces.md #) | Evaluation entry | Lookup → checked? |
|---|---|---|
| FM interpolation pass 1/2 (#1, #2) | `Evaluator::eval` / `eval_json` → `evaluate` | `FrontmatterSeedState::get_checked` ✔ |
| Page-block `when=` (#5) | `conditions::evaluate_condition` → `evaluate` | `ResolvingLookup` → `EffectiveState::get_checked` ✔ |
| Transclusion `when=` (#6) | same | same ✔ |
| Body interpolation, directive targets (#7) | `Evaluator::eval` fast path / `evaluate` | `ResolvingLookup` ✔ |
| `$()` ternary condition/branches (#8) | `Evaluator` over `FrontmatterSeedState` | ✔ |
| Bare-name fallback (#9) | `EffectiveState::get_checked` → `into_checked_bare_name`; `ShortcutLookup` → `resolve_ctx_checked` | ✔ |
| Local/remote child compose (#10, #11) | child re-runs #1–#9 | ✔ |
| Standalone `evaluate_condition_against` (#12) | `evaluate` over `ShortcutLookup` | `CtxLookup::resolve_ctx_checked` ✔ |
| Pre-flight passes (#13a–c) | frontmatter/`compose_with` paths above | ✔ (tolerance is Phase 3) |
| Subtree compose (#15) | `LayeredLookup::get_checked` | ✔ |
| `md graph` `when=` (#14) | `ResolvingLookup` | ✔ (`unwrap_or(false)`) |

No compose evaluation reads `ctx.*` through the unchecked `get` anymore. The remaining unchecked reads, recorded for Phase 3:

- `Evaluator::eval_value` swallows errors to `Null`. It has no production callers (evaluator unit tests only).
- `Evaluator::presentation_value` is consulted before the lookup. Presentation values are keyed by frontmatter properties, so only an authored `ctx` presentation value could shadow a read.
- After a successful `get_checked`, an object value is re-read through the unchecked `get_string` for name coercion. The check has already passed at that point.
- `is_valid_context_variable` still uses `get_context_value`. It feeds only the unknown-name warning walker, which is unchanged per R6.
- `EffectiveState::get_context_value` step 3 (legacy date fields) is unreachable under `Fatal`. Under `Legacy` it is reachable only for a snapshot without DateTime captured.
- Claudine's `SourceExpressionLookup` still uses the unchecked `CtxLookup::resolve_ctx`. Claudine's `compose_failed_code` falls through to `composition.expression_invalid` for the new variants. This is cross-package and belongs to Phase 3 typed-cause work.

### Requirement → test mapping

| Requirement | Test(s) |
|---|---|
| Present incl. typed `null`/`""`/`[]`/`{}` | `context::checked::tests::captured_typed_null_and_empty_values_are_present` |
| Known key, uncaptured group → `ContextNotCaptured` (Fatal) / legacy (staged) | `…::a_known_key_of_an_uncaptured_group_is_not_captured` |
| Captured group, removed key → internal invariant (both policies, shipped lookup) | `…::a_captured_group_missing_a_cataloged_key_is_an_internal_invariant` |
| Unknown key keeps unchecked path; authored custom `ctx` keys still resolve | `…::an_unknown_key_keeps_the_unchecked_lookup` |
| Date/time aliases owned by DateTime | `…::date_time_aliases_classify_under_their_group` |
| R1 authored `ctx.os` cannot mask uncaptured OS; still overrides when captured | `…::an_authored_ctx_value_cannot_mask_an_uncaptured_group` |
| R8 bare-name fallback | `…::a_bare_name_fallback_is_undefined_when_its_group_is_uncaptured` |
| Only evaluated operands checked (ternary, `\|\|`) | `…::unevaluated_operands_raise_nothing` |
| Interpolation fast path surfaces typed, authoring-fatal error | `…::the_interpolation_fast_path_surfaces_the_typed_error` |
| Staged compose policy = no document-observable change | `…::compose_lookups_keep_legacy_not_captured_answers_while_staged`, plus Phase 1 `ambient_ctx_capture::baseline_*` (still green) |
| Frontmatter-pass lookup uses same classification | `…::the_frontmatter_seed_lookup_classifies_through_the_same_path` |
| Fatality matrix: both kinds fatal on Body/FrontmatterWholeValue × fail_fast | `interpolation::fatality_characterization::fatality_matrix_is_locked` (2 new cases), `body_context_not_captured_is_fatal_in_lenient_mode`, `body_context_projection_invariant_is_fatal_in_lenient_mode` |
| Standalone `CtxLookup` parity + captures only reached groups | `expression::ctx::tests::a_capture_missing_a_cataloged_key_is_an_internal_invariant`, `…::checked_resolution_captures_only_the_groups_it_reaches` |
| Public condition lookup (incl. bare name) parity | `conditions::tests::shortcut_lookup_reports_a_projection_invariant_failure` |
| Key table ⇔ catalog agreement | `context::capture::groups::tests::every_projected_key_is_a_generated_descriptor` (reverse direction already pinned by `every_generated_descriptor_maps_to_one_group_or_explicit_alias`) |
| Real captures project every key (presence, absence, missing evidence) | `ambient_ctx_capture::a_repository_capture_projects_every_key_of_each_captured_group`, `…outside_any_repository…`, `…without_supplied_evidence…` |

- **Mutation check:** renaming the `os_version` insert in `host.rs` made all three completeness tests fail with `omitted cataloged keys ["os_version"]`. `host.rs` was restored from backup, and `git diff` of it is empty.
- **Corpus and end-to-end requirements:** no parser, schema, template, or shipped artifact changed, and nothing is persisted, so no corpus or round-trip test applies. The end-to-end path (real `ComposeOptions` composition) is covered by the unchanged Phase 1 `ambient_ctx_capture` tests. Document-level fatal end-to-end tests belong to Phase 3, when the error becomes observable.

### Gates

- `just test` (darkmatter): **7867 passed, 7 skipped, 1 flagged slow**. That is 7847 plus 20 new tests. The slow test is the pre-existing `rendering::test_compose_cleanup_preserves_nested_lists_inside_blockquotes`, and every new test runs under 0.4s.
- `just lint`: green, including the zed-dmls wasm32-wasip2 check. `cargo clippy -p darkmatter --tests`: no warnings.
- Downstream: `cargo check -p claudine --tests` compiles. Claudine tests were not run: its behavior is unchanged under the staged policy.
- `just test-l2`: not run. No terminal or browser surface changed.
- Cross-OS: not run remotely. The change has no OS-specific code or path comparisons, and the new integration tests use temp dirs and the existing cross-platform git fixture. The OS/hardware/GPU capture they call is already exercised on every CI OS by `every_catalog_variable_survives_ambient_options`.
- No pre-existing failures were encountered.

## Phase 3

### Starting point and impact analysis

- Phase 2's `human_review` items (R1–R8) still had no visible author response. This phase implements them as recorded, and Phase 3 is what makes them observable.
- Flipping `COMPOSE_NOT_CAPTURED_POLICY` to `Fatal` first, as a probe, broke only 4 L1 tests:
  - the Phase 2 staging pin;
  - one frontmatter unit test whose fixture never captured Repo;
  - the two Phase 1 baselines.

  The child-first baseline showed the child's error swallowed into a `_Could not transclude_` notice. This confirmed that the structural classification was needed.
- GitNexus upstream impact:
  - `evaluate_ternary_condition`: HIGH (3 shell-expansion processes).
  - `directive_reachable_pipelines`, `interpolate_branch_text`, `evaluate_value_branch`: LOW.
  - `ConditionError` and `ShellExpansionError`: UNKNOWN, so I confirmed by text search. Consumers are Darkmatter-internal, and Claudine only constructs `ParseDirective`.

### What landed

- **Policy flip.** `NotCapturedPolicy`, `COMPOSE_NOT_CAPTURED_POLICY`, `EffectiveState::get_checked_with`, and the fatality matrix's `EnforcingLookup` were deleted. `into_checked` raises `ContextNotCaptured`, and the matrix uses plain `ResolvingLookup`.
- **Typed causes** (spec §2, surfaces #5, #6, #8):
  - `ConditionError::Eval` and `TransclusionError::ConditionEval` now carry `#[source] cause: Box<ExpressionError>` instead of `message`. Two error snapshots changed their hint to `length(): items not found`.
  - New `ShellExpansionError::ExpressionEvaluation` for `$()` ternary interpolation and evaluation failures.
- **Cause-chain helpers.** `ExpressionError::is_missing_runtime_context()` and `MarkdownError::missing_runtime_context()` are public. The chain walk also downcasts `Box<ExpressionError>`, because wrappers box their cause.
- **Structural transclusion** (#10, #11). `run_transclusion_phase` treats any error whose chain holds either variant as structural, so a child is never replaced by a notice.
- **Source identity.** Body interpolation errors now go through `with_on_disk_source`, so a transcluded child's error names `child.md`. `interpolation_block` has a dedicated arm for both variants with a "Defined in:" link. The on-disk YAML excerpt is now shown only for frontmatter keys.
- **Pre-flight (Ruling 3):**
  - **#13a/#13b.** `scan_one_frontmatter` reads `ComposeOptions::upgraded_for(markdown)`, a borrowed or cloned view with the ambient upgrade the compose pass will apply. Before this, ambient `compose_preflight` of a `$()` branch using `{{ ctx.os }}` would fail.
  - **Best-effort interpolation.** `FrontmatterInterpolationReport` records the first missing-capture key error. When the dynamic-shape guard then fires, pre-flight returns that error instead of the misleading `DynamicCommandShape`. That key fails compose-pass frontmatter interpolation unconditionally, so the verdict is the same.
  - **#13c.** A new crate-private `ComposeOptions::defer_missing_runtime_context` is folded into `options_hash`. It is set only on the discovery inline compose. A `DeferrableLookup` in body interpolation answers `ContextNotCaptured` with `null`, but never `ContextProjectionInvariant`. Discovery composes without page blocks, so without this it would fail on content a false `::block` removes.
  - **Removed during the phase.** I first added a per-branch skip in `directive_reachable_pipelines`. It proved unreachable, because a raw `{{` value trips the dynamic-shape guard first, so I removed it.
- **Doc drift fixed** (behavior changed in this phase):
  - `ComposeContext::capture_minimal` and `ComposeOptions::new` no longer claim uncaptured groups are "captured on demand during expression evaluation".
  - `evaluate_ternary_condition` and `interpolate_branch_text` docs name the new variant.

### Decisions recorded

- **`$()` ternary branches are both evaluated.** The compose pass prepares and interpolates *both* branches before evaluating the condition, because every command must be allowlisted. So a `$()` branch reading an uncaptured group fails even when unselected. That is an evaluated reference, not a scanner false positive. `{{ a ? b : c }}` ternaries still short-circuit.
- **Interim regression accepted (child-first reference).** A transcluded child that is the first to name a group now fails loudly instead of rendering empty. Phase 4's request epoch fixes this. Every production caller is affected, not only ambient `ComposeOptions::new()`:
  - The **CLI** passes `new_with_context(capture_for_document(root))`, which the current flag treats as caller-supplied.
  - So does **Claudine**. Its `composition/prepare.rs`, `sequence/preflight`, and `system_prompt/prepare.rs` use root-only or empty `capture_for_*`.
- **Consequence observed.** 4 Claudine tests now fail, all through `prompts/_os.md` (`{{ ctx.os }}`) transcluded by the shipped `implement-plan` prompts. This is the real-world form of the bug: this very session's prompt rendered "a host that runs \"\"".
  - `claudine composition::schema::tests::shipped_implement_plan_preserves_supplied_commit_message_in_preflight_command`
  - `…::shipped_implement_plan_prepares_with_unset_optional_commit_message`
  - `claudine-cli::shipped_prompt_contract shipped_implement_plan_launches_without_a_sibling_spec`
  - `claudine-cli::compose_caller_file_provenance shipped_implement_router_prefers_an_unimplemented_review_over_the_completed_plan`

  I did not pull Phase 4's ownership, epoch, and cache-key work forward piecemeal. Flagged for human review.

### Requirement → test mapping

| Requirement (spec verification / plan task) | Test(s) |
|---|---|
| #1 typed error naming variable, group, source | `missing_ctx_capture::body_reference_to_an_uncaptured_group_names_variable_group_and_source`; verbatim input: `…::the_regression_input_fails_instead_of_rendering_empty_values` (both `fail_fast`), `ambient_ctx_capture::caller_supplied_minimal_context_fails_instead_of_rendering_uncaptured_groups_empty` |
| #2 frontmatter pass 1 (whole value + mixed text × `fail_fast`) | `…::frontmatter_interpolation_fails_in_whole_value_and_mixed_text_forms` |
| #2 frontmatter pass 2 | `…::frontmatter_interpolation_pass_two_fails` |
| #2 page-block condition (typed `PageBlock` wrapper) | `…::page_block_condition_fails_with_a_typed_cause` |
| #2 transclusion `when=` (typed `Transclusion` wrapper) | `…::transclusion_condition_fails_with_a_typed_cause` |
| #2 `$()` ternary condition, value branch, interpolated condition/branch | `…::frontmatter_shell_ternary_condition_and_branch_fail_with_a_typed_cause` |
| #2 recursive transclusion (lenient + strict, nested) never becomes a notice | `…::a_transcluded_child_reading_an_uncaptured_group_fails_the_parent` |
| Only evaluated references fail (unchosen `{{ }}` ternary, `\|\|`/`&&` short-circuit, `{{{ }}}` literal, false `::block`, short-circuited condition) | `…::unevaluated_references_do_not_fail_when_the_group_was_never_captured` |
| #3/#4 captured null/empty (no evidence → partial capture) renders, no missing-capture diagnostic | `…::a_captured_group_without_evidence_renders_its_typed_projection` |
| #5 unknown key: one unknown warning, no missing-capture | `…::an_unknown_key_warns_once_and_is_not_a_missing_capture` |
| §7 same reference in two files = two issues, own source | `…::the_same_reference_in_two_files_reports_each_files_own_source` |
| #6 malformed projection fails composition (body + condition, lenient) | `context::checked::tests::composition_fails_on_a_malformed_projection` |
| Best-effort/pre-flight tolerance; the compose pass owns the verdict | `…::preflight_tolerates_a_body_missing_capture_that_the_compose_pass_owns`, `…::ambient_preflight_discovers_a_ternary_branch_that_reads_runtime_context`, `…::preflight_reports_a_missing_capture_instead_of_a_dynamic_command_shape` |
| CLI exits nonzero naming variable and group (`CliProcessFixture`) | `darkmatter-cli::compose_interpolation::test_compose_uncaptured_context_group_exits_nonzero_naming_variable_and_group` |
| Policy flip at lookup level | `context::checked::tests::compose_lookups_raise_not_captured` (replaces the staging pin), fatality matrix now over `ResolvingLookup` |
| Interim child-first behavior pinned (Phase 4 flips) | `ambient_ctx_capture::ambient_child_first_reference_fails_until_the_request_context_extends` |

- **Mutation checks.** Each check disabled one fix, and the named tests failed. Both files were restored from backups and re-verified.
  - Structural classification: `a_transcluded_child…`.
  - Body on-disk source: `body_reference…`, `the_same_reference_in_two_files…`.
  - `defer_missing_runtime_context`: `preflight_tolerates…`.
  - Missing-capture-over-dynamic-shape: `preflight_reports…`.
  - `upgraded_for`: `ambient_preflight_discovers…`.
- **Corpus and end-to-end.** No parser, schema, or template grammar changed, and nothing is persisted. The CLI test is the end-to-end path. The shipped-artifact effect shows up in Claudine's shipped-prompt tests above.
- **CLI test trigger.** It uses an expression that *produces* `{{ ctx.os }}` (`{{ '{' + '{ ctx' + '.os }' + '}' }}`). A requirement scan cannot see that reference, so the test stays valid after Phase 4. I verified the rendered block manually with the built `md`: the header "runtime context not captured", the variable and group, and "Defined in:". The child-first case names `child.md`.

### Gates

- `just test` (darkmatter): **7884 passed, 7 skipped, 1 slow**. The slow test is the pre-existing `rendering::test_compose_cleanup_preserves_nested_lists_inside_blockquotes`. That is 7867 plus 17 new tests; every new test is under 1s.
- `just lint`: green, including the zed-dmls wasm32-wasip2 check. `cargo clippy -p darkmatter -p darkmatter-cli --tests`: no warnings.
- Downstream: `cargo check -p claudine --tests` compiles. `just test` in claudine: 7015 passed and **4 failed** (listed above; caused by this phase, interim until Phase 4).
- **Storage incident.** The first full run hit `No space left on device`: the APFS container had 169 MiB free across Data (2.6 TB) and coding (1.3 TB). The default `scripts/sweep.sh` freed nothing. `SWEEP_MAX_SIZE=20GB scripts/sweep.sh <this worktree>` capped this worktree's own `target/`, freeing 49.28 GiB. Nothing outside this worktree was touched. The host-wide shortage is not caused by this session.
- `just test-l2` was not run: no real-terminal or browser surface changed. The status-block change is covered at L1.
- **Cross-OS.** Not run remotely. The new tests are portable:
  - They use temp dirs, `Path::ends_with` / `file_name`, and supplied-evidence or DateTime-only contexts, with no host discovery.
  - `echo` is only pre-approved and is never executed.
  - The CLI assertion avoids path text, which can wrap on long Windows temp paths.
- GitNexus `detect-changes --scope all` covers the whole branch: HIGH risk, 13 affected flows (compose pipeline, frontmatter shell expansion, transclusion, directive targets), as expected for this fix.

## Phase 4

### Starting point

- Phase 3's `human_review` items had no visible author response. This phase answers item (b) in code: the CLI's and Claudine's own captures are extendable. Details are under "Decisions" below.
- GitNexus upstream impact before editing:
  - `run_compose_pipeline_internal`: LOW, 1 flow.
  - `render_markdown_transclusion`: LOW, 2.
  - `upgrade_ambient_context_for`: LOW, 2.
  - UNKNOWN (confirmed by text search): `PhaseStateIdentity` is used only in `phases.rs`/`engine.rs`; `get_or_compute_compose` has one production caller in `engine.rs`; `upgraded_for` is used only in `collect.rs`; `try_persistent_read_compose` is private; `clone_for_child` has LOW per-axis risk.
- Disk: 22 GiB free on `/Volumes/coding` at start.

### Decisions

- **Ruling 4 is three-state, not two.** Claudine's contexts come from `capture_with_evidence` over its own retained launch evidence (`InvocationContext::capture_launch_context`). Darkmatter can't grow them by ambient discovery without mixing observation epochs, and freezing them keeps the shipped prompts broken. The public `ContextAuthority` therefore has three states:
  - `CallerSupplied`: frozen. The default for `new_with_context` and `with_context`.
  - `DarkmatterOwned`: `ComposeContext::extend_ambient`. The default for `ComposeOptions::new()`.
  - `CallerExtended(Arc<dyn ContextExtension>)`.
- **Who opts in.**
  - The `md` CLI sets `DarkmatterOwned` on its `capture_for_document(launch_dir)` context.
  - Claudine implements `ContextExtension` for `InvocationContext` and `DocumentEpoch` by delegating to `extend_launch_context`. `DocumentEpoch::extend_launch_context` now returns `bool`.
  - Claudine installs the authority in `canonical_compose_options`, system-prompt compose, prep pre-flight, sequence pre-flight, JIT template pre-flight, and the overlay. The ambient-fallback branches use `DarkmatterOwned`.
- **`with_context` freezes.** It used to leave the ambient flag set, so `new().with_context(pinned)` could replace a pinned snapshot wholesale.
- **Epoch mechanism (Spike A, refined).** `RequestContextEpoch` (an `RwLock<Option<ComposeContext>>`) lives on `PipelineRuntime` and is shared by `clone_for_child`. The root seeds it after `extend_context_for`.
  - A child's context is `context_for_source(parent, for_document(child after set overlay), authority)`. That is the parent snapshot plus the child's own missing groups, adopted from the epoch; the epoch is grown under the write lock only when the authority allows.
  - The child's group set therefore depends only on its path from the root, not on sibling order, so its cache key is deterministic. The spike's "hash the whole epoch" churn goes away without a two-level key.
  - A group is captured at most once per request.
- **Root extension is now an extension, not a recapture.** `extend_context_for` keeps the constructor's environment and date/time and adds only the missing groups at the retained anchor. The old upgrade re-ran `capture_for_document` and replaced both.
- **Cache identity.**
  - The child key's `context_hash` is taken over the child's snapshot after handoff. The hoisted phase hash is reused when `is_same_snapshot`, which keeps Finding 35.1's fast path.
  - A grandchild can read a group the child's key never saw. `ComposeResult` and `ComposedDocumentManifest` therefore record the subtree's context-group closure (`PipelineRuntime::context_groups`, propagated by `merge_child` and by cache hits) plus `context_groups_hash`.
  - On a persistent read, `RequestContextClosure` grows the epoch to the closure when the authority allows, then compares hashes. A mismatch, or a closure a frozen request can't cover, returns `None`. That is a miss, never `Stale`, so neither the `Fallback` nor the `Forced`/`Optimistic` modes can bypass the missing-capture contract.
  - The new manifest fields are required serde fields, so pre-existing entries fail to deserialize and are misses. No `CACHE_VERSION` bump was needed.
  - Run-local hits need no closure check, because one request has one epoch.
- **Pre-flight.** `collect_recursive` applies `extended_for(markdown)` per document and passes the extended options to children, which mirrors the compose-pass group sets. `scan_one_frontmatter` no longer extends on its own.

### Known limitations (logged, not fixed)

- Pre-flight uses a separate `PipelineRuntime`, so it has no shared epoch. An ambient child extension is discovered once in pre-flight and again in compose, which the root already was before this phase. Evidence-backed extensions (Claudine) are deterministic.
- Pre-existing, out of scope: `PipelineRuntime::merge_child` merges only transclusion stats, so compose `dependencies` recorded while resolving a nested child never reach the parent's `ComposeResult`. Context groups now propagate.
- A `CallerExtended` extension is fingerprinted by kind only (`fingerprint_tag`), in the graph identity.

### Requirement → test mapping

| Requirement (spec verification / plan task) | Test(s) |
|---|---|
| #7 ambient: a child and a nested grandchild first read `ctx.os`, render the full-capture value, same value for both | `ambient_ctx_capture::ambient_child_first_reference_renders_the_full_capture_value` (flipped from Phase 3's interim pin) |
| #7 one capture per request, reused by siblings (Rayon) and a nested child | `request_context_epoch::siblings_and_nested_children_read_one_capture_of_a_child_introduced_group`, `context::authority::tests::concurrent_sources_capture_a_group_once_and_read_one_projection` (32 workers) |
| #7 remote child handoff | `request_context_epoch::a_remote_child_first_naming_a_group_reads_the_request_capture` (wiremock) |
| #7 lazy at reachability (false `when=` child never captured) | `request_context_epoch::a_group_named_only_by_an_unreachable_child_is_never_captured` |
| Group set independent of sibling order (stable identity) | `context::authority::tests::a_source_sees_its_parent_groups_plus_its_own_whatever_the_request_holds` |
| #8 frozen context fails naming variable, group, and child; `new_with_context` and `new().with_context` | `request_context_epoch::a_frozen_context_fails_when_a_child_first_names_a_group`, `ambient_ctx_capture::ambient_child_first_reference_renders_the_full_capture_value` (`capture_minimal` arm) |
| #8 no fallback capture (epoch not grown) | `context::authority::tests::a_frozen_authority_never_grows_the_request_context` |
| #9 changed child-only value, persistent cache (write/read/overwrite/read, plus a still-hitting third run) | `request_context_epoch::persistent_cache::a_changed_child_only_value_invalidates_the_cached_child` |
| #9 grandchild-only value (closure is the only guard) | `…::a_changed_grandchild_only_value_invalidates_the_cached_parent` |
| #9 cache hit cannot bypass the contract (Strict/Fallback/Optimistic/Forced) | `…::a_persistent_entry_cannot_bypass_a_frozen_missing_capture` |
| #10 root extension removed → named failure | `context::authority::tests::a_pipeline_without_the_root_extension_fails_with_the_named_group` |
| Pre-flight sees the child's grown context; frozen reports the missing capture | `request_context_epoch::preflight_discovers_a_child_branch_reading_a_child_introduced_group` |
| Closure hash semantics | `cache::hashing::tests::context_groups_hash_follows_only_the_recorded_groups_values` |
| Persisted group names | `context::capture::groups::tests::every_group_name_is_unique_and_round_trips` |
| Shipped-artifact end-to-end (Claudine prompts transcluding `prompts/_os.md`) | claudine `composition::schema::tests::shipped_implement_plan_*` (2), claudine-cli `shipped_prompt_contract::shipped_implement_plan_launches_without_a_sibling_spec`, `compose_caller_file_provenance::shipped_implement_router_prefers_an_unimplemented_review_over_the_completed_plan`: all red after Phase 3, green now |

- **Mutation checks** (each file restored from backup and re-verified):
  1. Closure check disabled in `try_persistent_read_compose`: `a_changed_grandchild_only…` served stale `leaf=one-1`; `a_persistent_entry_cannot_bypass…` returned cached output to a frozen request.
  2. Handoff removed (child gets `state.context().clone()`) in both local and remote arms: 6 of 7 epoch tests failed with `ContextNotCaptured { key: "repo_root", group: Repo }` naming the child source. That is a named failure, not an empty-output mismatch (verification #10, handoff half).
  3. Per-document `extended_for` removed from `collect_recursive`: the new pre-flight test and Phase 3's `ambient_preflight_discovers_a_ternary_branch_that_reads_runtime_context` failed.
- **Removed:** `context/epoch_spike_tests.rs` (Phase 1 PoC, untracked). Its own doc said Phase 4 replaces it; the equivalent assertions now run against the real cell in `context::authority::tests`.
- **Comment drift fixed:**
  - `lib/tests/shell_block_integration.rs` still claimed an uncaptured group "is still captured on demand" (stale since Phase 3).
  - "root upgrade" wording in `ambient_ctx_capture.rs`.
  - The `ComposeOptions` field and `new`/`with_context` docs.
  - `darkmatter-expressions.md` ("never captures a group on demand" now names reachability and authority).
  - The compose skill: "Known gap" replaced with the authority/epoch/cache contract.

### Gates

- `just test` (darkmatter): **7896 passed, 7 skipped**, no slow flags (Phase 3: 7884). Every new test finishes in under 0.4s.
- `just lint` (darkmatter): green, including the zed-dmls wasm32-wasip2 check. `cargo clippy -p darkmatter -p darkmatter-cli -p claudine -p claudine-cli --tests`: no warnings.
- `just test` (claudine, the Phase 3 gate): **7019 passed, 9 skipped, 0 failed**. The 4 Phase 3 regressions are fixed.
- No other workspace package constructs `ComposeOptions` (checked with `rg`).
- GitNexus `detect-changes --scope all` covers the whole branch (Phases 1–4, darkmatter + Claudine): 50 files, 174 symbols, 30 affected flows, risk **critical**. That is expected: the compose spine plus Claudine's preparation paths. The mitigation is the full darkmatter and Claudine suites.
- `just test-l2`: not run. No terminal or browser surface changed.
- Cross-OS: not run remotely. No `cfg` or path-comparison code changed. The new tests use temp dirs, supplied evidence (no host discovery except the existing fixture-repo ambient test), substring checks on file names and evidence labels, and the existing wiremock `127.0.0.1` pattern. CI covers Linux, Windows, and WSL.
- Disk: 58 GiB free at the end.

## Phase 5

### Starting point

- Phase 4's `human_review` items had no visible author response. Phase 5 documents the three-state `ContextAuthority` as implemented and carries the confirmation item forward.
- No Phase 5 production-code behavior changed. The work is tests, comments, and documentation, so impact analysis was not needed for a symbol edit. GitNexus `detect-changes` was still run as a final gate.

### What landed

- **#11 graph guard.** `request_context_epoch::a_graph_naming_no_discovery_backed_group_captures_none` composes a root, a `when="ctx.year > 2000"` child, and a nested child. The graph contains:
  - frontmatter interpolation;
  - DateTime-only `ctx.*`;
  - an unknown `ctx.oss`;
  - a literal `{{{ ctx.repo_root }}}`.

  A counting `CallerExtended` authority then observes zero growth, on both a cold and a warm persistent cache (the warm run exercises the closure check). Every authority kind grows through the same `context_for_source`/`ensure` handoff, so this stands in for ambient discovery without probing the host.
  - **Mutation:** adding `{{ ctx.os }}` to the nested child failed the test with `cold: a group was captured for a graph that names none`. The file was restored from backup.
- **#12 public entry point.** `conditions::tests::evaluate_condition_against_keeps_unknown_names_falsy_and_skips_unreached_groups` checks, through the public function, that an unknown name is falsy rather than an error, that a short-circuited known name is not read, and that a DateTime read resolves. My first draft also asserted `ctx.oss == null`. That was wrong: an undefined name is not equal to `null` in this engine, so I removed the line; it was unrelated to the spec item.
- **Verification matrix.** `verification-matrix.md` maps #1–#12 to named tests, confirmed with `rg`, and summarizes the mutation evidence from every phase.
- **Documentation.**
  - `docs/topics/context-variables.md`: "Timing in Compose" now covers the five required points.
    - `ComposeOptions` is the authority, with a three-state `ContextAuthority` table.
    - Growth happens only at root and child reachability, from retained evidence, once per request.
    - Frozen contexts never fall back to ambient discovery and fail with `ContextNotCaptured`.
    - Captured null/empty differs from an uncaptured group, and each other absence state keeps its own diagnostic.
    - Cache identity follows the context.
  - `docs/topics/caching.md`:
    - The "Context hash" section was stale before this fix: it claimed only `today`, `yesterday`, and `tomorrow` plus env were hashed. It now lists what the code hashes: all values minus the volatile keys, plus env.
    - New sections cover the finalized post-extension child hash, the context closure hash, and the closure check. A closure mismatch is a miss in every freshness mode, and pre-closure manifests are misses.
    - The key model, "Current Status", and the compose read-flow steps were updated to match.
- **Comment drift fixed.** Both comments still claimed on-demand capture during evaluation:
  - `context/options.rs`: the doc on `new_captures_no_discovery_derived_group`.
  - `lib/tests/reference_integration.rs`: the fixture comment. Its context is frozen, so it now says so.
- **Skill.** `.claude/skills/darkmatter/compose.md`:
  - Corrected the manifest field names (`context_closure_groups`/`context_closure_hash`).
  - Added the captured-absence rule and the #11 guard test.
  - Added pointers to the two user-facing topics.

### Not changed (recorded)

- `claudine/lib/src/composition/mod.rs:195` says groups are "still captured on demand during evaluation". That comment describes Claudine's `ResolutionContext` fallback. The `ResolutionContext` carries no `ctx` values, and Claudine's standalone lookups use the lazily capturing `CtxLookup`, so the comment may still be accurate for that path. It is in another package and was not verified to be drift, so I left it for review.
- `expression/ctx.rs` `resolve_ctx_checked` ("captured on demand") and the `evaluate_condition_against` notes ("capture is lazy") are accurate. The standalone path does capture lazily.

### Requirement → test mapping (Phase 5)

| Requirement | Test(s) |
|---|---|
| #11 graph-level: no discovery-backed group captured (root, gated child, nested child, unknown and literal `ctx.*`, cold and warm persistent cache) | `request_context_epoch::a_graph_naming_no_discovery_backed_group_captures_none` |
| #11 constructor level (unchanged) | `context::options::tests::new_captures_no_discovery_derived_group` |
| #11 no L1 test above 5 s | full `just test` run, slowest 3.396 s (`rendering::test_compose_cleanup_preserves_nested_lists_inside_blockquotes`, pre-existing) |
| #12 public entry point: unknown name falsy, short-circuit skips a known group, reached group resolves | `conditions::tests::evaluate_condition_against_keeps_unknown_names_falsy_and_skips_unreached_groups` |
| #1–#12 full mapping | `verification-matrix.md` |
| Generated `ctx.*` catalog block in `context-variables.md` still matches (the prose was edited outside the markers) | `context_variables_doc_matches_generated_catalog` (in the suite, green) |

- **Corpus and end-to-end.** No parser, schema, template, or shipped artifact changed in Phase 5, and nothing new is persisted. The #11 test runs through the real `Markdown::compose_with` path, with a real persistent cache round trip (write on the cold run, read on the warm run).

### Gates

- `just test` (darkmatter): **7898 passed, 7 skipped, 0 failed**. That is Phase 4's 7896 plus 2 new tests. No test is flagged slow, and the slowest took 3.4 s.
- `just lint` (darkmatter): green, including the zed-dmls wasm32-wasip2 check.
- GitNexus `detect-changes --scope all` covers the whole branch (Phases 1–5): 53 files, 194 symbols, 30 affected flows, risk **critical**. That matches Phase 4 plus this phase's test, comment, and doc files. Phase 5 changed no public `ComposeContext`, `EffectiveState`, or `ComposeOptions` shape, so Claudine was not re-tested. Its Phase 4 run was green: 7019 passed.
- `just test-l2`: not run. No terminal or browser surface changed.
- Cross-OS: not run remotely. The new tests use temp dirs, supplied evidence (no host discovery), and a relative `Path::new(".")` for DateTime-only standalone evaluation, with no path-text assertions. CI covers Linux, Windows, and WSL.
- No pre-existing failures were encountered.

## Implementation of Review Findings #1

> **started at:** 2026-09-18T00:46:57-07:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/feat-dark-fixes/darkmatter/fixes/2026-08-02-silent-empty-ctx-values/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- starting the work on 'Persistent cache hits discard child partial-capture diagnostics' at 00:47:40
        - discovered: the code path the finding names no longer exists. Commit `8fc370dae` (2026-09-17 00:44, after this review was written at 2026-09-16 22:26) removed semantic-result persistence; `RunLocalCache` (`lib/src/markdown/compose/cache/runtime.rs`) is now memory-only and the fresh/stale persistent read paths that built `ComposeResult` with `ComposeReport::new()` were deleted. A cache root now persists no composed child, so a warm run recomposes and re-derives the child's warnings.
        - discovered: the one remaining cache that can serve a child without recomposing it is the run-local single-flight slot. It stores the child's full `ComposeResult` (including `report`), and `render_markdown_transclusion` merges `cached.report` into the parent on a hit as well as on a miss (`transclusion/engine.rs`, `report.merge(cached.report.clone())`), so a run-local hit already replays the child's `PartialRuntimeCapture` warnings without duplicating them relative to an uncached run.
        - design decision: no production code change. Persisting the report, or regenerating warnings on a hit, would re-introduce the persistence the content-policy fix deleted; the cleanest design is the current one (memory-only results that carry their report). The finding is closed by regression tests that pin the contract on both cache surfaces. No serialization format changed, so no version bump.
        - tests added (`lib/tests/request_context_epoch.rs`): `persistent_cache::a_child_only_partial_capture_warns_identically_on_cold_and_warm_runs` (root names no discovery-backed group; child's first `ctx.branch` read is captured from unavailable evidence via a new `UnavailableEvidence` extension; cold and warm runs against one cache root report the same `Partial runtime capture for git` warnings, count and content, and persist nothing) and `a_run_local_hit_replays_a_child_only_partial_capture_warning` (same child transcluded twice; asserts a run-local hit occurred and the warnings equal a `CacheAccessMode::Off` run).
        - test strengthened (`lib/tests/missing_ctx_capture.rs`): `a_captured_group_without_evidence_renders_its_typed_projection` now asserts the report's `Partial runtime capture ...` warnings equal the context's `PartialRuntimeCapture` diagnostics, once each and in order.
        - mutation check: replacing `report.merge(cached.report.clone())` with a discard made both new cache tests fail; the source was restored byte-for-byte (`cmp` against a backup) and re-verified.
        - docs: `verification-matrix.md` row 4 now names what is actually asserted and lists the two cache tests; a Review-1 entry was added under "Mutation evidence". No `///`/`//!`, skill, or `darkmatter/docs` contract changed (behavior unchanged). Frontmatter left alone (no per-review-cycle pattern).
        - gates: targeted `request_context_epoch` + `missing_ctx_capture` 26/26 passed. `just test` (darkmatter, `--no-fail-fast`): 8099 passed, 14 skipped, 9 timed out; the 9 are `markdown::compose::tests::provider_network::*` wiremock tests (an earlier run timed out `remote_fetch::persistent_cache_tests::*` instead) under host load average ~20-24 from other sessions. All 55 `provider_network` + `persistent_cache_tests` tests passed on rerun at `-j 2`. None touch files changed here. `just lint`: green, including the zed-dmls wasm32-wasip2 check.
        - impact: no library function was edited, so no GitNexus pre-edit impact applies; Claudine not re-tested (no public type changed).
        - orchestrator verification: `git log` confirms `8fc370dae` (2026-09-17T00:44) removed the persistent compose-result read paths after the review was written (2026-09-16T22:26); the remaining `ComposeReport::new()` sites in `cache/runtime.rs` are all inside unit-test fixtures, not cache-hit paths
        - resolution: fixed by tests only — the defect's code path no longer exists, and the new cold/warm and run-local-hit regressions pin warning preservation so it cannot reappear
- work completed for 'Persistent cache hits discard child partial-capture diagnostics' at 01:06:30
- starting the work on 'Body failures omit the available authored location' at 01:06:30
        - impact (GitNexus, upstream): `interpolate_text` HIGH (47 impacted, 28 direct: frontmatter interpolation, frontmatter shell expansion, directive targets, body stage); `interpolation_error` MEDIUM (48, 2 direct); `rewrite_directive_targets` LOW (1: body `run_stage`); `interpolation_block` LOW (4); `with_on_disk_source` and `run_inline_pre_operation` UNKNOWN (text search: 3 and 1 callers, all in `compose/pipeline/`). Mitigation for the HIGH risk: `interpolate_text` keeps its signature and behavior byte-for-byte as a thin wrapper; only the body stage calls the new located entry point.
        - discovered: body offsets from the engine are in the frame of the text it scanned, which is NOT the file: (a) the body is the parsed remainder after frontmatter (lines rejoined with `\n`); (b) text replacement, page blocks, and directive-target rewriting can edit it before interpolation runs; (c) rescans after depth 0 see replacement output; (d) `full_source_context_for_errors` reconstructs text from the current (possibly edited) body, so its line numbers can drift from disk. `Markdown` already retains the exact loaded text (`LoadedSource::text`), which is the authoritative on-disk frame.
        - design decision: the engine reports the failing expression's byte span only for depth-0 (authored-candidate) expressions; rescans report none. At the body-stage boundary the span is accepted as authored only when the scanned text up to the end of the expression is byte-identical (after line-ending normalization) to the loaded document's body prefix; the line is then the frontmatter-aware file line from `extract_frontmatter_block`. Anything unproven keeps the existing file-identity-only presentation (omit, never guess). The located form is a new `SourceRef::OnDiskLine { context, line }` whose context content is the loaded text, so the rendered excerpt and line agree with disk. `MarkdownError::Interpolation`'s shape is unchanged, so Claudine constructors are untouched.
        - files changed: `lib/src/markdown/compose/interpolation/rewrite.rs` (new crate-private `LocatedInterpolationError` + `interpolate_text_located`; `interpolate_text` is now a wrapper with identical behavior), `interpolation/mod.rs` (re-exports), `compose/directive_targets.rs` (`rewrite_directive_targets` locates a whole-value `::file`/`::code`/`::url` target failure at its span), `compose/inline/interpolation.rs` (`anchor_authored_failure` + `authored_line` proof), `lib/src/markdown/mod.rs` (`Markdown::loaded_source_context_for_errors`), `lib/src/markdown/types.rs` (public `SourceRef::OnDiskLine { context, line }`, `MarkdownError::with_authored_line`, docs), `lib/src/markdown/errors/blocks.rs` (renders the file link, the line, and a `>`-marked numbered excerpt for `OnDiskLine` in the file-reference and missing-context blocks)
        - Claudine: no Claudine source constructs or matches `SourceRef` exhaustively (only `SourceRef::Effective` constructions in tests), and `MarkdownError::Interpolation`'s fields are unchanged, so no Claudine edit was needed
        - tests added: `missing_ctx_capture::a_body_failure_reports_its_authored_file_line_after_frontmatter` (CRLF, four frontmatter lines, earlier successful expressions; asserts `OnDiskLine` file + line 9, and rendered file name, `Expression at line: 9`, and the `>`-marked excerpt line), `…::a_transcluded_child_failure_reports_the_child_file_and_line` (child file, child line 6), `…::a_generated_expression_is_not_reported_at_an_authored_line` (a `{{{ … }}}`-literal frontmatter value substituted into the body and failing on rescan stays file-only `OnDisk`, key `None`, no rendered line), `…::an_expression_after_text_an_earlier_stage_removed_is_not_given_a_line` (a removed page block before the expression: file-only, not the drifted line); unit: `rewrite::tests::located_failure_spans_the_authored_expression`, `…::located_failure_from_replacement_output_has_no_span`, `inline::interpolation::tests::authored_line_*` (4). Two existing tests now assert `OnDiskLine` and the line.
        - mutation checks: replacing the depth-0 guard with an unconditional span failed `located_failure_from_replacement_output_has_no_span`; removing the prefix proof failed `authored_line_rejects_a_rewritten_prefix` and `…_a_different_expression_at_the_same_offset`. Both sources restored byte-for-byte (`cmp`).
        - docs: `error-taxonomy.md` (Review 1 bullet on `OnDiskLine`), `verification-matrix.md` row 1, `.claude/skills/darkmatter/errors.md` (line-number frame trap: `full_source_context_for_errors` vs loaded text)
        - gate: `just test --no-fail-fast`: 8117 passed, 14 skipped, 1 timed out (`persistent_cache_disabled::a_warm_cache_root_never_replays_composed_local_output`, load average ~35). A first fail-fast run had instead timed out 10 `remote_fetch::persistent_cache_tests::*`. Rerun at `-j 2` of that binary plus every `persistent_cache_tests`/`provider_network` test: 60/60 passed.
        - first `just lint`: clippy `result_large_err` on the new error pair (fixed by boxing its `MarkdownError`) and two `unnecessary_lazy_evaluations` (`then_some`)
        - gates after the clippy fixes: darkmatter `just lint` green (clippy + zed-dmls wasm32-wasip2 check); targeted rerun of `missing_ctx_capture` plus every interpolation/directive-target/`interpolation_block` test: 411/411 passed
        - Claudine (public `SourceRef` gained a variant): `just lint` green. `just test --no-fail-fast`: 7031 passed, 9 skipped, 5 failed. None is caused by this change: `shipped_prompt_contract::*` (2) and `shipped_prompt_route_drift::*` (1) fail on the uncommitted `prompts/` edits from other work (e.g. an untracked `prompts/_add/add-expressions.md` whose `description` contains `{{ … }}`), and `composition::schema::tests::shipped_implement_plan_*` (2) fail with `LifecycleObjectDataThroughInterpolationPositional` on the modified `prompts/_implement/implement-plan.md`. No Claudine test references the rendered missing-context block.
        - GitNexus `detect-changes --scope all`: reports `critical`, but the dirty tree holds 514 changed files from other sessions and the CLI summary is truncated, so it is not a clean check for this change alone
        - deferred: none. CLI process tests were not added; the rendered block is the same `status_block` the CLI prints, and it is asserted at the library boundary
- work completed for 'Body failures omit the available authored location' at 02:24:37

### Successful Completion

The implementation of review cycle 1 has completed successfully in 1h 38m. During this implementation all 2 review findings were evaluated to see if they could be fixed as a part of this implementation cycle: 2 were fixed, 0 were deferred (see reasons below):

- no findings were deferred
- 'Persistent cache hits discard child partial-capture diagnostics' was closed with regression tests only: the persistent compose-result read path it named was removed by `8fc370dae` after the review was written, and the remaining run-local cache already replays the child report
- 'Body failures omit the available authored location' was fixed in the library: body failures now carry a proven authored line (`SourceRef::OnDiskLine`) and fall back to file-only when the line cannot be proven
- caveats for the reviewer:
        - under host load (load average 20–35), `just test` had wiremock timeouts in `provider_network::*`, `remote_fetch::persistent_cache_tests::*`, and one `persistent_cache_disabled` test; all passed when rerun at `-j 2`
        - Claudine `just test` has 5 failures from other sessions' uncommitted `prompts/` edits (`shipped_prompt_contract::*`, `shipped_implement_plan_*`), which this change did not cause; Claudine `just lint` is green
        - no cross-OS rig run: the changes are line-ending-normalized (a CRLF case is tested) and use temp-dir fixtures without host discovery, so CI covers Linux, Windows, and WSL

The files changed during this cycle are:

- `darkmatter/lib/src/markdown/compose/interpolation/rewrite.rs`
- `darkmatter/lib/src/markdown/compose/interpolation/mod.rs`
- `darkmatter/lib/src/markdown/compose/directive_targets.rs`
- `darkmatter/lib/src/markdown/compose/inline/interpolation.rs`
- `darkmatter/lib/src/markdown/mod.rs`
- `darkmatter/lib/src/markdown/types.rs`
- `darkmatter/lib/src/markdown/errors/blocks.rs`
- `darkmatter/lib/tests/missing_ctx_capture.rs`
- `darkmatter/lib/tests/request_context_epoch.rs`
- `darkmatter/fixes/2026-08-02-silent-empty-ctx-values/error-taxonomy.md`
- `darkmatter/fixes/2026-08-02-silent-empty-ctx-values/verification-matrix.md`
- `.claude/skills/darkmatter/errors.md`
