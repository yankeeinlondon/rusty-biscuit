---
total_phases: 5
created: 2026-09-16
phase: 1
agent: opencode/zai-coding-plan/glm-5.3
yolo: 'true'
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

# Plan: A missing runtime value must not render as nothing

## Work Summary

### The work required

Darkmatter's 2026-08-02 regression let a known `ctx.*` variable reach composition
without its owning capture group in the request snapshot, silently rendering an
empty string. The immediate repair (root ambient upgrade plus
`lib/tests/ambient_ctx_capture.rs`) closed the root-document case only. This
plan implements the full specification
(`fixes/2026-08-02-silent-empty-ctx-values/spec.md`): a typed, checked
`ctx.*` lookup at the composition boundary that hard-fails composition when a
known variable's owning group was never captured, across every expression
surface and the entire transclusion graph, with cache identity that follows
context growth.

The work concentrates in five areas:

1. **Checked lookup core** — a typed four-state classification
   (value present including typed null/empty; owning group not captured; group
   captured but cataloged key missing from the projection; unknown key) driven
   exclusively by the existing authorities
   (`context_variable_descriptors()`, `ContextGroup::for_key`,
   `ComposeContext::capture_requirements()`). Today `EffectiveState::get_context_value`
   (`context/effective_state.rs`) collapses all four states into
   `Option<Value>` and then into `""`.
2. **Fatal typed diagnostics** — new `ExpressionError` variants that are
   authoring-fatal in lenient mode (the `is_authoring_fatal()` /
   `fatality_characterization.rs` mechanism), carrying variable path, owning
   group, source identity/span, and caller guidance; surfaced through the typed
   `MarkdownError` path so the CLI exits nonzero without a stringly
   `Transform` error.
3. **Transclusion request epoch** — a graph-shared, monotonically growing
   request context. Darkmatter-owned ambient composition extends a newly
   reachable child's missing groups from retained request evidence (same
   anchor, same environment) before the child's first expression stage;
   caller-supplied contexts stay frozen and fail with the missing-capture
   error. The current root-only `upgrade_ambient_context_for`
   (`context/options.rs`) and per-child `options.clone()` in
   `transclusion/engine.rs` do not provide this.
4. **Cache identity timing** — the child compose-cache key in
   `transclusion/engine.rs` (~line 1444) is built from a parent-phase
   `PhaseStateIdentity` captured before any child-introduced group exists. The
   handoff must happen before the child's cache key is computed, the context
   hash must include every group affecting the cached result, and lookup/write
   must share one finalized identity.
5. **Hardening and documentation** — the twelve verification scenarios from the
   spec as automated tests, performance guards (no discovery for documents
   that name no discovery-backed group; no L1 test above the five-second
   slow-test threshold), and the documentation/comment revisions including the
   stale "captured on demand during expression evaluation" claims in
   `ComposeOptions::new()` / `capture_minimal()` doc comments.

### What successful completion looks like

- Composing `repo_root={{ ctx.repo_root }}` with a caller-supplied context that
  omitted the Repo group returns a typed fatal error naming `ctx.repo_root`,
  `Repo`, and the source document — through frontmatter interpolation (both
  passes), body interpolation, `when=`/page-block conditions, and `$()`
  branching — never a partially composed document, never an empty string.
- A captured `null`/`""`/`[]` value (`ctx.branch` outside a repo, `ctx.gpu`
  with no device) still renders with existing semantics and emits no
  missing-capture diagnostic; `PartialRuntimeCapture` behavior is unchanged;
  unknown names like `ctx.oss` keep the existing unknown-variable diagnostic
  and suggestion path, with no duplicate "group not captured" emission.
- A root with no host-context reference whose local or nested remote child
  introduces `{{ ctx.os }}` composes correctly under ambient options, with one
  captured OS projection reused by every later read in the graph; the same
  graph under a minimal caller-supplied context fails with the named
  missing-group error and performs no ambient fallback.
- Cache regression tests prove a child-only context value change invalidates
  the parent's cached output, and cache hits cannot bypass the
  missing-group contract.
- A document graph naming no discovery-backed `ctx.*` group captures none
  (existing constructor test stays green), and `just test` + `just lint` pass
  in the darkmatter package area with no L1 test above the slow-test
  threshold.
- The compose/context documentation states the authority boundary
  (`ComposeOptions` is the context authority; ambient growth only from
  retained evidence within one request; caller-supplied contexts never fall
  back to ambient discovery; captured null/empty differs from uncaptured;
  cache identity is finalized after requirements are known).

Out of scope (per spec): capture-cost reduction, mentioned-vs-consumed group
narrowing, changing null/empty projections, unknown-name severity policy, DMLS
runtime capture, aggregate fatal-error collection.

---

## Phase 1 — Rulings, Survey, and Baseline

Phase 1 removes ambiguity and de-risks the two structural unknowns (the
graph-shared epoch mechanism and the full inventory of evaluator surfaces)
before any production code changes.

### Necessary Rules

The implementation needs explicit rulings on the following. Each ruling should
be confirmed with the author (or resolved by spike evidence) and recorded in
this file before Phase 2 begins; the spec text cited supports the proposed
answer, but each is an observable behavior or contract decision that must not
be made silently.

- **Ruling 1 — user-authored `ctx` values cannot mask an uncaptured group.**
  A known key whose owning group is absent from `capture_requirements()` fails
  even when the document's frontmatter authors a value for that key under a
  `ctx:` mapping. The captured-group check consults `capture_requirements()`
  before the merged `data["ctx"]` map. When the group *is* captured, the
  existing merge/override policy is untouched (spec §3: "user values must not
  falsify the runtime snapshot's captured group evidence").
- **Ruling 2 — fatality in lenient mode.** Both new errors (missing capture and
  capture-projection invariant) are authoring-fatal on every compose surface:
  they are never downgraded to `ComposeWarning`, never suppressed by
  `fail_fast: false`, and never leave the literal `{{ … }}` in place — the same
  treatment `ExpressionError::UnknownFunction` receives today
  (`interpolation/fatality_characterization.rs`).
- **Ruling 3 — internal passes tolerate the error.** Condition-blind and
  best-effort passes (pre-flight shell approval collection via
  `interpolate_frontmatter_best_effort`, shell-command discovery, deferred
  schema verdicts) swallow per-expression missing-capture errors so the
  terminal compose pass owns the verdict, matching how those passes already
  tolerate other per-key evaluation failures. Rationale: the approval set must
  stay a faithful superset; only the terminal pass's evaluation is authoritative.
- **Ruling 4 — context ownership is two-state, not the current flag.**
  `context_is_ambient_default` conflates "Darkmatter may extend this context"
  with "the constructor installed it". After the root upgrade the flag clears,
  which would freeze children out of extension. Replace with an explicit
  ownership distinction: **caller-supplied (frozen authority)** vs
  **Darkmatter-owned (extendable from retained evidence within the request)**.
  The root document's upgrade remains the first capture of a Darkmatter-owned
  epoch; children extend it.
- **Ruling 5 — what "retained request evidence" means for ambient extension.**
  Proposed: a new same-epoch ambient extension (e.g.
  `ComposeContext::extend_ambient(required)`) that captures only the missing
  groups anchored on the snapshot's retained `anchor()`, derives Agent values
  from the snapshot's retained environment map (mirroring
  `extend_with_evidence`'s Agent handling), never overwrites existing values,
  and never re-reads CWD. A fresh point-of-use capture that re-anchors or
  re-snapshots the environment is rejected (spec §Design decision 3,
  "Alternatives rejected" — uncoordinated laziness). Confirm whether the
  retained Git handle must also be shared across extensions (accepted answer:
  yes where cheaply possible, since `ContextCapture::new` already exists per
  capture; a re-`discover` on the same anchor is acceptable only if the spike
  shows sharing is impractical).
- **Ruling 6 — evaluation-driven, not scan-driven.** The fatal diagnostic is
  raised only by actually-evaluated expressions (evaluator variable
  resolution). The existing AST walker (`collect_context_warnings`) must NOT
  grow a missing-group warning — that would flag unchosen ternary branches and
  short-circuited operands (spec §4). `ContextRequirements` scanning may
  conservatively overcapture; it never diagnoses.
- **Ruling 7 — projection-invariant reachability.** The "group captured but
  cataloged key missing" invariant is checked on the same lookup path
  (compose and standalone `CtxLookup`), and its test requires a test-only way
  to build a malformed snapshot (cataloged group marked captured with a key
  surgically removed from `values`). Confirm the test hook shape
  (e.g. a `#[cfg(test)]`/`pub(crate)` constructor next to
  `ComposeContext::fixed_for_testing`).

### Ruling Decisions (recorded in Phase 1)

The author could not be consulted (non-interactive session), so each ruling
below is settled by spec text plus the Phase 1 survey and spike. Every ruling
is **adopted provisionally** and flagged for author confirmation through
`human_review`. Phase 2 may proceed on these decisions unless the author
overrides one. Evidence lives in `surfaces.md`, `error-taxonomy.md`, and
`context/epoch_spike_tests.rs`.

- **R1: adopted.** The group check consults `capture_requirements()` before the
  merged `data["ctx"]` (`EffectiveState::get_context_value` reads user `ctx`
  first today, `effective_state.rs:290`). When the group is captured, the
  existing merge order is untouched.
- **R2: adopted, with one refinement found by the survey.** "Same treatment as
  `UnknownFunction`" is not enough on its own. Tolerated transclusion
  resolution (`pipeline/phases.rs:376-410`) turns any non-structural child
  error into a notice plus a warning when `fail_fast` is false, and today that
  already happens to a child's `UnknownFunction`. A `MarkdownError` whose cause
  chain holds either new variant must be classified **structural** there, so
  it propagates instead of yielding a partially composed document (spec §2).
  Conditions (#5/#6) and `$()` ternaries (#8) are already always fatal, but
  their wrappers are stringly and need a typed source (`error-taxonomy.md`).
- **R3: adopted, and extended to #13b and #14.** Pre-flight ternary branch
  discovery (`frontmatter_shell_expansion.rs:1464`, hard-coded
  `fail_fast=true`) and the reference-graph `when=` (`reference/graph.rs:354`,
  already `.unwrap_or(false)`) both run against the **non-upgraded** context,
  so they must tolerate missing-capture too. The shell-command-discovery inline
  compose (#13c) receives the root upgrade but inherits `?`; it must tolerate
  per-expression missing-capture the same way.
- **R4: adopted.** Replace `context_is_ambient_default: bool` with a two-state
  `ContextAuthority { CallerSupplied, DarkmatterOwned }` on `ComposeOptions`,
  preserved across the root upgrade. `options_hash` currently folds the bool
  in (`options.rs:2275`); fold the enum in the same way.
- **R5: adopted, with a Git-handle answer.** The new
  `ComposeContext::extend_ambient(&mut self, required)` is the discovery-backed
  twin of `extend_with_evidence` (`runtime.rs:354`). It works as follows:
  - It captures only missing groups on the retained `anchor()`.
  - It derives values from the retained env map, never `std::env::vars()`.
  - It uses the anchor as the invocation cwd, exactly as
    `capture_runtime_context_for_requirements` does today.
  - It never repopulates DateTime and never overwrites.
  - It resets the `overrides` memo.

  **Git handle:** sharing is impractical. `GitRepo` lives inside one
  `ContextCapture::new` call (`capture/snapshot.rs:231`) and is not `Sync`
  (Git identity and file changes already share it serially for that reason).
  A `GitRepo::discover` on the same anchor per extension is accepted.
  Extensions happen at most once per group per request, and discovery is the
  cheap part (`.git` lookup).
- **R6: adopted.** `collect_context_warnings` (`interpolation/evaluator.rs:365`)
  gains nothing. Unknown keys classify as `Unknown`, which keeps today's `None`,
  so the two diagnostics cannot both fire.
- **R7: adopted.** A `#[cfg(test)] pub(crate)` hook,
  `ComposeContext::with_projection_key_removed`, sits next to
  `fixed_for_testing`. **Phase 2 prerequisite found:** `fixed_for_testing`
  marks DateTime captured but projects only 11 of the group's 37 `KEYS`
  (`capture/datetime.rs:4`), and `fixed_for_testing_with` inserts values for
  groups it never marks captured. Under R7 both are malformed snapshots, so the
  test fixtures must be repaired (project every key of each marked group; mark
  the owning group of every inserted key) before the checked lookup lands. Real
  captures also need a per-group projection-completeness test, including
  failure/absence evidence, so R7 cannot fire on a legitimate capture.
- **R8 (new, survey-found): bare-name ctx fallback.**
  `EffectiveState::get` (`effective_state.rs:235`) and `ShortcutLookup::get`
  (`conditions.rs:368`) resolve a bare `when="repo"` as `ctx.repo` when `repo`
  is not a frontmatter key. `ContextRequirements` scans only literal
  `ctx.KEY`, so a bare name never causes its group to be captured.
  **Proposed:** a bare name is primarily a document variable, not an
  unambiguous context reference. When its cataloged group *is* captured it goes
  through the checked classification (so R7 still fires). When the group is
  *not* captured it resolves as an undefined name, the same as any other
  missing bare name, and raises no missing-capture error.
  **Rejected alternative:** making it fatal would turn an absent optional
  frontmatter key that happens to share a ctx name (`model`, `agent`, `os`,
  `branch`, `packages`) into a hard compose failure. No shipped artifact uses
  the bare ctx fallback (`rg 'when="repo"'` hits only design docs).
  **Needs author confirmation.**

### Spikes

- **Spike A — graph-shared request epoch (time-boxed).** Prototype the
  mechanism by which a context extension performed while resolving one
  transclusion child becomes visible to later siblings, grandchildren, and the
  child's own cache-key computation, under the Rayon-resolved transclusion
  phase. Candidate mechanisms to evaluate: an `Arc<RwLock<ComposeContext>>`
  request cell carried by `ComposeOptions`; a request-scoped cell on
  `PipelineRuntime` (note `clone_for_child` shares only some fields); or
  discovering the reachable requirement closure eagerly before phase open.
  Constraints to verify in the spike: monotonic growth only (no overwrites),
  one anchor, caller-supplied contexts excluded, `EffectiveState` snapshots
  already built remain valid (values they read were checked at their own
  stage), and the extension happens before any cache identity that can reuse
  the child's output is finalized. Deliverable: chosen mechanism plus a
  proof-of-concept test showing sibling visibility and pre-extension hash
  mismatch; record the decision and its trade-offs in this plan file.
- **Spike B — expression-surface and fatality matrix survey.** Produce the
  authoritative inventory of every surface that evaluates `ctx.*` and how
  errors flow from each today: frontmatter interpolation passes 1 and 2
  (note: these build their lookup directly over `&ComposeContext`, not
  `EffectiveState`), schema-adjacent composed values, page-block conditions,
  transclusion `when=` conditions, body interpolation, `$()` ternary
  condition/branches, bare-name `ctx` fallback (`when="repo"`), recursive
  local and remote transclusion, standalone `evaluate_condition_against`, and
  the internal best-effort passes from Ruling 3. For each: the lookup object
  in scope, whether `fail_fast` gates fatality today, and which
  `MarkdownError` wrapper carries failures. Deliverable: a surface matrix
  checked into the fix directory (`surfaces.md`) that Phase 3 implements
  against and the fatality characterization tests extend.


### Spike A Decision (recorded in Phase 1)

**Chosen mechanism: a request-epoch cell owned by `PipelineRuntime`.** The
authority stays on `ComposeOptions`.

- **The cell.** It holds `Arc<RwLock<ComposeContext>>` plus an epoch version
  counter. It is shared through `PipelineRuntime::clone_for_child`
  (`shell_expansion/types.rs:1372`), the same way `cache` and `remote_fetch`
  are already request-scoped and shared. The root seeds it from
  `options.context()` right after `upgrade_ambient_context_for`.
- **Extension.** A check under a read lock, then check-and-extend under a
  write lock, so concurrent Rayon siblings capture a group **exactly once**
  and every later reader observes that one projection. Extension is
  copy-on-write (`Arc::make_mut`), so an already-built `EffectiveState`
  keeps the snapshot its stage was checked against. Both properties are proven
  by `shared_cell_extends_once_and_every_sibling_reads_the_same_projection`.
- **Authority.** `DarkmatterOwned` contexts extend via `extend_ambient` (R5).
  `CallerSupplied` contexts never extend; the child runs on the frozen
  snapshot and fails in the checked lookup.
- **Hook location: the engine, before the cache key.** It is not at the start
  of `run_compose_pipeline_internal`, because the key must be finalized after
  extension. In `render_markdown_transclusion`, do the following before the
  `persistent_ctx`/`cache_key` construction (`engine.rs:1444`):
  1. Load the child with the memoized `runtime.load_markdown(path)`. The
     closure already makes this call, so it costs nothing extra.
  2. Apply the `set` overlay. Overlay values can contain `ctx.*`.
  3. Compute `ContextRequirements::for_document(child)` and ensure the cell.
  4. Hand the child the cell's snapshot as `child_options` context.

  Remote children (`engine.rs:1105`) do the same after fetch; they are not
  cached through `get_or_compute_compose`.
- **Finding 35.1 fast path.** Keep `PhaseStateIdentity` hoisted. Recompute
  the context component only when the cell's epoch version differs from the
  version captured with the phase identity.
- **Cache identity (Phase 4 direction).** Hashing after the child's *own*
  extension is not sufficient, because a grandchild can introduce a group
  after the child's key is taken. Hashing the whole current epoch is
  order-dependent under Rayon, which churns persistent keys. Direction:
  - Record the subtree's context-group closure (own requirements ∪ folded
    children's recorded closures) on the `ComposeResult` and persistent
    manifest.
  - Compute the context component of the identity over **those groups'
    values only**.
  - On lookup, first ensure the epoch covers the recorded closure: ambient
    extends, caller-supplied fails with `ContextNotCaptured`, so a hit cannot
    bypass the contract. Then compare.
  - Write with the same closure-restricted identity.

  `phase_hoisted_context_hash_cannot_see_a_child_introduced_group` shows the
  current defect: the hoisted hash is identical for two requests whose
  child-only `repo_root` differs, while post-extension hashes differ.
- **Rejected: a cell inside `ComposeOptions`.** `ComposeOptions` is a public
  `Clone` value that is fingerprinted by `options_hash`. Interior mutability
  would silently alias the epoch across independent requests built by cloning
  one options value, and the fingerprint would have to exclude it.
- **Rejected: an eager closure from the preflight graph as the authority.**
  `preflight_graph` is optional (attached by the CLI, absent from plain
  `compose_with`). It pre-evaluates `when=` against the non-upgraded context,
  and it cannot see dynamic `{{ }}` targets or remote children before fetch.
  It remains a possible optimization to pre-extend known children.

### Tasks

#### Work-group A — survey and baseline (concurrent with Work-group B)

- [x] **surface matrix survey**
  - Execute Spike B; write `fixes/2026-08-02-silent-empty-ctx-values/surfaces.md`
    with the per-surface inventory described above.
  - Flag any surface whose lookup cannot reach `capture_requirements()` today
    (e.g. the frontmatter-pass lookup over raw `ComposeContext`) — these need
    adapter work in Phase 2.
- [x] **repro characterization**
  - Add a temporary (or permanent, if it becomes verification #1) test that
    composes `repo_root={{ ctx.repo_root }}|os={{ ctx.os }}|today={{ ctx.today }}`
    through ambient `ComposeOptions::new()` inside the existing
    `lib/tests/ambient_ctx_capture.rs` fixture and through a minimal
    caller-supplied context, capturing today's silent-empty behavior as the
    pre-change baseline.
  - Record the baseline suite state: `just test` and `just lint` green before
    any change.
- [x] **error taxonomy note**
  - Draft the exact new `ExpressionError` variant names and fields (variable
    path, owning group, source identity/span, caller guidance; plus the
    internal-invariant variant) and the checked-outcome enum shape, so Phases
    2–3 implement one agreed contract. Keep it in the fix directory
    (`error-taxonomy.md`), one page maximum.

#### Work-group B — epoch mechanism spike (concurrent with Work-group A)

- [x] **epoch mechanism spike**
  - Execute Spike A; land the proof-of-concept (may be `#[ignore]`-gated or
    behind the eventual API) and record the decision in this plan.
  - Include in the evaluation: where the child requirement handoff hook lives
    (start of `run_compose_pipeline_internal` vs the engine before
    `get_or_compute_compose`), and how the child's cache identity is computed
    after extension while preserving the Finding 35.1 hoisting for the
    unchanged-epoch fast path.

**Validation checkpoint (Phase 1):** rulings R1–R7 recorded with decisions;
`surfaces.md` and `error-taxonomy.md` exist; spike A decision recorded; `just
test` and `just lint` green on the untouched tree (baseline recorded).

---

## Phase 2 — Checked `ctx` Lookup Core

Introduce the four-state classification and thread it to the boundary types,
without yet changing fatality on any surface. Prerequisite: Phase 1 rulings
(even a preliminary version if work-group A/B results are recorded in the same
session). Run GitNexus impact analysis (`impact` upstream) on
`EffectiveState::get`, `ComposeContext::get`, and `EvaluationLookup` before
editing — these are hub symbols.

### Tasks

#### Work-group C — core classification (serial spine; everything else waits on this)

- [x] **checked-outcome enum**
  - Add the typed outcome (per `error-taxonomy.md`), e.g.
    `CtxLookupOutcome::{Value, UncapturedGroup(group), MissingProjectionKey(group), Unknown}`
    in the context module, classified from the authorities only:
    `context_variable_descriptors()` for known-ness, `ContextGroup::for_key`
    for ownership, `capture_requirements()` for captured-ness, and the
    projected values map for presence (typed `null`/`""`/`[]`/`{}` are
    `Value`). No second key list anywhere (spec §1).
  - Implement a checked accessor on `ComposeContext` (usable by both
    `EffectiveState` and the frontmatter-pass lookup over raw
    `ComposeContext`) and on `EffectiveState` (covering the merged
    `data["ctx"]` precedence rule from Ruling 1: group check first, user
    values only consulted for a captured group).
- [x] **expression error variants**
  - Add the two `ExpressionError` variants from the taxonomy note and mark
    both authoring-fatal via `is_authoring_fatal()`. They carry the full
    message inputs (variable path like `ctx.repo_root`, group like `Repo`,
    caller guidance: capture the document's `ContextRequirements` or build
    options from an appropriately captured `ComposeContext`) so no stage has
    to reconstruct them.
  - Extend the fatality characterization matrix
    (`interpolation/fatality_characterization.rs`) with the new kinds across
    every surface × fail_fast cell (fatality expected everywhere per Ruling 2).
- [x] **evaluator checked path**
  - Route `ctx.*` variable resolution (including the bare-name fallback
    resolved against the ctx namespace) through the checked outcome during
    evaluation, mapping `UncapturedGroup`/`MissingProjectionKey` to the new
    errors while `Value` and `Unknown` keep today's behavior exactly.
  - The mechanism must work for every `EvaluationLookup` implementor used by
    composition (e.g. a trait method with a default that derives from `get`,
    overridden by `EffectiveState`/`ResolvingLookup` and the frontmatter-pass
    lookup). Unknown keys still flow to the existing AST-walker diagnostic;
    no walker change (Ruling 6).

#### Work-group D — parity and unit tests (concurrent after the C spine lands)

- [x] **standalone ctx parity**
  - Update `expression/ctx.rs` `CtxLookup` to obey the same classification:
    after demand-capturing a group, a cataloged key still absent from the
    merged cache is the projection-invariant failure, not a silent `None`
    (spec §The defect / verification #12). Keep its lazy per-group capture and
    the "captures only groups actually reached" property.
- [x] **classification unit tests**
  - Unit tests pinning all four states: value present including typed
    null/empty; known key with uncaptured group; captured group with a
    surgically removed key (test hook per Ruling 7) failing as internal
    invariant; unknown key. Include a test that a user-authored `ctx.os`
    value does not satisfy the uncaptured-OS-group lookup (Ruling 1), and
    that it still merges/overrides when the group is captured.
- [x] **silent-path audit**
  - Grep/graph audit that no compose stage reads `ctx.*` through the unchecked
    `Option<Value>` path anymore (the legacy field-by-field fallback in
    `EffectiveState::get_context_value` step 3 must not be reachable from
    compose stages; the unchecked public accessors may remain for
    compatibility per spec §1).

**Validation checkpoint (Phase 2):** `just test` (new classification tests
green, no existing behavior change observable from documents — fatality wiring
is Phase 3); `just lint` green; audit note recorded (list of stages and the
lookup each now uses).

---

## Phase 3 — Fatal Diagnostics on Every Surface

Make the checked outcome fatal wherever composition evaluates expressions, per
the surface matrix from Phase 1. Prerequisite: Phase 2 complete.

### Tasks

#### Work-group E — frontmatter-family surfaces (concurrent with Work-group F)

- [x] **frontmatter passes fatal**
  - Frontmatter interpolation passes 1 and 2 raise the missing-capture error
    as a typed `MarkdownError` (carrying the `ExpressionError` cause and the
    on-disk source context/span), fatal regardless of `fail_fast` (Ruling 2).
  - Schema-adjacent composed values on the frontmatter surface get the same
    treatment per the surface matrix.
- [x] **best-effort tolerance**
  - `interpolate_frontmatter_best_effort` and the other internal passes named
    in Ruling 3 swallow the new errors per-key exactly as they swallow other
    evaluation failures today; add tests pinning that the terminal pass, not
    the collector, reports the error.

#### Work-group F — body-family surfaces (concurrent with Work-group E)

- [x] **body and conditions fatal**
  - Body interpolation (`interpolation/rewrite.rs`), page-block conditions,
    transclusion `when=` conditions, and `$()` ternary condition/branches
    raise the new errors fatally on every path where they evaluate a `ctx.*`
    reference (surfaces without `EffectiveState` in scope get the adapter
    identified in Phase 1).
  - Only evaluated expressions fail: add tests that an unchosen ternary
    branch, a short-circuited operand, an interpolation literal
    (`{{{ ctx.gpu }}}`), and content removed before its expression stage
    (false `::block`) do not raise — including when the group was never
    captured (verification #2's negative space, spec §4).
- [x] **typed cause and rendering**
  - Verify library callers receive the typed cause (not string-only
    `MarkdownError::Transform`) and the CLI exits nonzero through the existing
    typed compose-error rendering; add a CLI integration test using
    `cli/tests/common/fixture.rs`'s `CliProcessFixture` asserting nonzero exit
    and a message naming the variable and group.
- [x] **diagnostic non-duplication**
  - Tests proving an expression never emits both "unknown context variable"
    and "group not captured" (unknown keys have no owning group), and that
    repeated evaluation/rescan of the same authored expression does not
    duplicate the diagnostic; the same bad reference authored in two files
    remains two issues with their own source identities (spec §7,
    verification #5).

**Validation checkpoint (Phase 3):** verification items #1, #2, #3, #5, #6
from the spec hold as automated tests (caller-supplied minimal context fails
typed on each surface; captured null/empty renders with no extra diagnostic;
unknown-key path unchanged; malformed projection fails as internal invariant);
`just test` and `just lint` green.

---

## Phase 4 — Transclusion Epoch and Cache Identity

Cover the full transclusion graph with one request epoch and make cache
identity follow context growth. Prerequisite: Phase 3 complete plus Spike A's
chosen mechanism. This is the highest-risk phase; run impact analysis on
`run_compose_pipeline_internal`, `PhaseStateIdentity`, and the engine's
`get_or_compute_compose` path before editing.

### Tasks

#### Work-group G — epoch spine (serial; H depends on it)

- [x] **ownership split**
  - Implement Ruling 4's ownership distinction on `ComposeOptions`/
  `ComposeContext`: caller-supplied contexts are frozen; Darkmatter-owned
    contexts (ambient default, and post-root-upgrade) are extendable. The
    root upgrade in `run_compose_pipeline` becomes the epoch's first capture
    and the ownership survives it (replace the `context_is_ambient_default`
    clearing semantics; keep `new_captures_no_discovery_derived_group`
    green).
- [x] **graph-shared epoch**
  - Land the Spike A mechanism: one request-scoped, monotonically growing
    context visible across the Rayon-resolved graph. Implement the ambient
    extension per Ruling 5 (`extend_ambient`-style: missing groups only,
    retained anchor, retained environment, no CWD read, no overwrites).
  - Hook the requirement handoff so each newly reachable source's requirements
    are ensured against the epoch before that source's first expression stage
    — local children, remote children, and nested children; conditional and
    dynamic transclusions extend lazily at reachability, not eagerly
    (verification #7).
- [x] **caller-frozen failure**
  - With a caller-supplied minimal context, a child's first request for a
    missing group fails with the named missing-capture error and performs no
    ambient fallback capture — assert no discovery side effects (verification
    #8; the error itself comes from the Phase 2/3 checked path, this task
    proves no fallback intercepts it).

#### Work-group H — cache identity (after G lands; tests below concurrent)

- [x] **cache key timing**
  - Move the child compose-cache identity so the child's requirement
    handoff/extension happens before the cache key is computed in
    `transclusion/engine.rs`; the context hash used for lookup and write is
    the same finalized post-extension identity (spec §6). Preserve the
    Finding 35.1 hoisting for the unchanged-epoch fast path (e.g. epoch
    version/dirty check rather than unconditional rehash).
  - Prove a cache hit cannot bypass the missing-group contract: a cached
    child result is only reachable under an identity that already includes
    the groups its output depends on.
- [x] **cache regression tests**
  - Verification #9: change a child-only context value between runs and prove
    the parent cannot reuse stale composed output; cover both the run-local
    and persistent (`--cache-root`-style) cache paths as applicable.
  - Verification #10: mutation-style tests showing that removing the root
    ambient upgrade or the transclusion requirement handoff produces a named
    missing-group failure rather than an empty-output mismatch (e.g. driving
    the pipeline the way `run_compose_pipeline` does, minus the upgrade).

**Validation checkpoint (Phase 4):** verification items #7, #8, #9, #10 hold
as automated tests; one captured value is reused for every later read in the
graph (same value asserted across sibling and nested reads); `just test` and
`just lint` green.

---

## Phase 5 — Hardening, Documentation, and Drift Cleanup

Close the verification matrix, guard performance, and reconcile all
documentation with the new behavior. Prerequisites: Phases 2–4 complete.

### Tasks

#### Work-group I — verification sweep (concurrent with Work-group J)

- [x] **matrix reconciliation**
  - Walk the spec's twelve verification items against the test suite; every
    item maps to at least one named test (items #1–#6 land in Phase 3, #7–#10
    in Phase 4; confirm #11 and #12 here — #12's standalone-path selectivity
    and classification from Phase 2's parity work).
  - Close any gaps found; record the item → test mapping in the fix directory
    (`verification-matrix.md`).
- [x] **performance guards**
  - Verification #11: a document graph with no discovery-backed context
    references still captures no discovery-backed group (extend the existing
    constructor-level assertion to the graph level — e.g. captured-groups
    assertion after composing a root+child graph), and the full darkmatter L1
    suite has no test above the established five-second slow-test threshold
    (run the suite and check timings; keep any new heavy fixture inside the
    `ambient_ctx_capture.rs` purpose-built-fixture pattern).

#### Work-group J — documentation and drift (concurrent with Work-group I)

- [x] **compose docs update**
  - Update `docs/topics/context-variables.md` (and any compose/context docs it
    links) to state the five required points: `ComposeOptions` is the context
    authority; ambient context grows only from retained evidence within one
    request; caller-supplied contexts never fall back to ambient discovery;
    captured null/empty differs from an uncaptured group; cache identity is
    finalized after the requirements for its result are known.
  - Revise the stale comments claiming `ComposeOptions::new()` values are
    captured "on demand during expression evaluation"
    (`context/options.rs::new`, `context/runtime.rs::capture_minimal`) to
    describe the actual root/transclusion handoff and fixed-snapshot
    behavior; sweep for any other drifted comment per the repo's
    comment-drift rule in the same change.
- [x] **caching docs update**
  - Update `docs/topics/caching.md` (and cache-adjacent docs) for the
    finalized-identity rule and what a context hash now includes.
- [x] **skill drift update**
  - Update `.claude/skills/darkmatter/compose.md` (Demand-Driven Runtime
    Context Evidence section) to match the new boundary, per the repo's drift
    maintenance rule.
- [x] **final gates**
  - Full `just test`, `just test-l2` (only if implementation touched a
    terminal/browser surface — not expected per spec), and `just lint` green;
  run GitNexus `detect_changes` (scope `all`) and review the affected
    processes/summary; confirm no unexpected blast radius beyond darkmatter
    (check downstream consumers such as Claudine if any public
    `ComposeContext`/`EffectiveState` surface changed shape).

**Validation checkpoint (Phase 5):** verification matrix complete (#1–#12);
documentation and skill updated; performance guards green; all recipes green;
detect_changes reviewed. The fix is implementation-complete and ready for
review (the author moves it to `_completed` — never the agent).
