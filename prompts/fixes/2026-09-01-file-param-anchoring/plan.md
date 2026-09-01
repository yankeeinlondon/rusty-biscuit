---
total_phases: 4
created: 2026-09-01
phase: 1
agent: codex/default
yolo: true
---

# Execution Plan: Anchor Eager Caller File Parameters Before Frontmatter Evaluation

Reference specification: [`spec.md`](../../../claudine/fixes/2026-09-01-file-param-anchoring/spec.md)

## Goal

Make every compose surface consume one launch-resolved semantic value for an
eager caller-supplied `file()` parameter before frontmatter expressions run,
while preserving the existing native-versus-portable presentation split,
document-owned eager normalization, and lazy-file anchoring rules.

## Scope and dependency summary

The implementation is centered in Darkmatter's compose pipeline and is then
verified through Claudine's normal preparation and CLI path. GitNexus currently
reports LOW symbol-level risk: `run_with_registry` has two direct callers and
one affected compose process, while the eager-override helpers feed that same
process. The behavioral reach is broader because direct compose, inline
compose, sequences, proxy targets, retries, and downstream Darkmatter callers
all share the pipeline.

Phase 1 establishes the failure and safety baseline. Phase 2 creates the
schema/projection primitives. Phase 3 integrates those primitives into the
pipeline and closes Darkmatter coverage. Phase 4 adds Claudine regression
coverage, documentation, and final repository gates.

## Phase 1 — Baseline, blast radius, and failing regression

**Outcome:** the current split-value defect is reproduced with deterministic
fixtures, and the implementation starts from a reviewed, isolated baseline.

- [ ] Record `git status --short` and preserve all pre-existing worktree changes; keep this fix's diffs limited to the Darkmatter compose/schema/error surfaces, focused tests, Claudine regression/docs, and this plan.
- [ ] Re-run GitNexus upstream impact analysis before editing `run_compose_pipeline_internal`, `run_with_registry`, `resolve_eager_caller_file_overrides`, and `resolve_eager_caller_value`; review every direct caller and stop for user review if any result has risen to HIGH or CRITICAL risk.
- [ ] Add a Darkmatter regression fixture under the existing compose/schema test modules that mirrors the shipped `prompts/plan.md` shape: prompt beneath `prompts/`, eager caller `spec`, `x: "{{ spec }}"`, and `plan: "{{ dirname(spec) + '/plan.md' }}"` with `plan` declared as lazy `file()`.
- [ ] Exercise that fixture with a captured repository-root launch base and with a captured `claudine/` launch base using `spec=fixes/<case>/spec.md`; assert both runs identify the same specification, derive `claudine/fixes/<case>/plan.md`, and never produce `prompts/fixes/<case>/plan.md`.
- [ ] Assert the pre-fix failure specifically: the package-area case currently leaves the raw override visible to frontmatter expressions while body interpolation receives the resolved presentation value. Keep the assertion focused enough that it turns green only when projection moves before pass 1.
- [ ] Capture unchanged-control cases alongside the regression: ordinary strings, lazy caller `file`, absent optional values, document-authored eager files, and keys in `ComposeOptions::exclude_keys` must retain their current values and anchoring.
- [ ] **Validation checkpoint:** run the narrow new tests plus the existing eager-override tests in `darkmatter/lib/src/markdown/compose/schema_validation.rs`; confirm only the new ordering assertion fails and existing scalar, array, repository-boundary, and ambient-CWD cases remain green.

## Phase 2 — Reusable schema assembly and caller projection artifact

**Outcome:** Darkmatter can assemble the effective schema once, project eager
caller inputs with typed failures, and detect unstable eager classification
without performing validation early.

- [ ] Extract the effective-schema builder and trigger-registry discovery currently embedded in `schema_validation::run_with_registry` into a crate-private preparation seam that accepts the existing `ComposeOptions`, source path, frontmatter snapshot, and reusable trigger registry.
- [ ] Make the preparation seam preserve the current baseline-schema merge, document `$schema` handling, request-scoped `FileResolutionContext`, `file_ref_fallback_dir`, deferred-verdict behavior, and single trigger-discovery walk; do not add a second filesystem scan or recapture ambient CWD.
- [ ] Introduce a transient pipeline-owned caller projection artifact in `darkmatter/lib/src/markdown/compose/schema_validation.rs` containing native values, portable presentation values, and the set/classification of present caller overrides typed as eager `darkmatter-file` properties.
- [ ] Refactor `resolve_eager_caller_file_overrides` and `resolve_eager_caller_value` to populate that artifact for top-level scalar, array, property-union, root-union, baseline-schema, document-schema, and applicable trigger-schema shapes while ignoring lazy/string/document-owned/absent/DM1-excluded values.
- [ ] Resolve projected values through `biscuit_file::FileReference` and the already captured trusted caller base. Return the existing typed malformed/missing file-reference failure, including launch-base provenance, instead of swallowing it or converting it to an unstructured message.
- [ ] Add a crate-private pre-interpolation operation that builds the initial effective schema, computes the artifact, and installs only its native values into working frontmatter. Keep coercion, optional binding materialization, validation verdicts, and document-owned eager normalization in their existing validation stage.
- [ ] Add a typed phase-instability error that records the affected property and explains that caller eager-file typing must be stable before frontmatter interpolation; wire it through `MarkdownError`/`BlockError` and Claudine's existing diagnostic discovery and code-selection path without flattening the source error.
- [ ] Compare the eager classification of every present caller override whenever the effective schema is reassembled. Fail closed on eager-to-non-eager or non-eager-to-eager drift; do not rerun interpolation or any read-side expression/warning work.
- [ ] **Parallelizable after the artifact shape is fixed:** add helper-level tests for scalar/array/union classification, baseline plus document schemas, DM1 exclusions, idempotent projection, native versus portable values, and typed malformed/missing failures.
- [ ] **Validation checkpoint:** run the targeted schema-validation and error-rendering tests. Verify projection is side-effect free except for explicitly installing caller native values, trigger discovery occurs once, and classification drift reports the named property through a typed diagnostic.

## Phase 3 — Compose pipeline integration and Darkmatter contract coverage

**Outcome:** the projection artifact is created once before interpolation pass
1, reused by both schema passes, and consumed once by effective-state
presentation.

- [ ] In `Markdown::run_compose_pipeline_internal`, create and apply the caller projection immediately after `prepare_frontmatter_for_compose` installs caller overrides and before `frontmatter_interpolation::interpolate_frontmatter` pass 1 builds its dependency graph.
- [ ] Change `schema_validation::run_with_registry` to consume the prepared schema/projection state instead of clearing presentation values and independently resolving raw overrides; retain its current coercion, optional binding, validation filtering, verdict, and document-owned normalization responsibilities.
- [ ] Reuse the same artifact during post-shell schema revalidation, reassembling only the effective trigger match needed for the stability comparison. Ensure shell expansion and interpolation pass 2 cannot change the caller anchor, duplicate registry discovery, or reinterpret installed absolute values as document-authored.
- [ ] Pass the artifact's presentation map to `EffectiveStateBuilder::with_presentation_values` exactly once. Verify whole-value, inline, static member, and array-index body interpolation use portable values while frontmatter/path expressions retain native semantic values.
- [ ] Update the stage-order documentation and behavior comments in `pipeline/mod.rs` and `schema_validation.rs` to describe the pre-interpolation eager-caller projection prelude while preserving the public interpolation → validation → shell → interpolation-pass-2 contract.
- [ ] Expand Darkmatter Level 1 coverage for identical root/package launch results, body/frontmatter semantic agreement, eager arrays and union arms, unchanged lazy and document-authored behavior, malformed/missing launch-anchored diagnostics, pass-2 reuse, and projection idempotence.
- [ ] Add a trigger test where interpolation changes a present caller property's eager classification and assert failure occurs before any frontmatter shell command executes; include an execution sentinel proving the shell side effect did not run and raw `{{ ... }}` syntax did not leak.
- [ ] Add platform-neutral assertions for `/`-normalized presentation and derived paths, plus `#[cfg(windows)]` coverage proving semantic frontmatter uses native absolute Windows paths. Keep macOS/Linux expectations identical and avoid platform-specific path literals outside gated assertions.
- [ ] **Parallelizable after pipeline wiring:** split the negative-semantics matrix, shell/trigger tests, and Windows-specific assertions across independent test cases while one owner updates the core pipeline to avoid merge conflicts.
- [ ] **Validation checkpoint:** from `darkmatter/`, run `just test` and `just lint`; all existing and new Level 1 tests must pass. Do not run Level 2/3 suites because the change does not involve terminal/browser rendering, focus, or input encoding.

## Phase 4 — Claudine end-to-end proof, documentation, and final gates

**Outcome:** Claudine's real planning workflow proves the shared fix from both
launch directories, the contract is documented, and both package areas pass
their required gates.

- [ ] Add a Claudine Level 1 CLI regression (prefer a focused `claudine/cli/tests/file_param_anchoring.rs`) that stages the actual `prompts/plan.md` schema/expression shape and a real specification beneath `claudine/fixes/<case>/spec.md`.
- [ ] Run the staged workflow through `claudine compose ... --dry-run` once from the repository root with `spec=claudine/fixes/<case>/spec.md` and once from `claudine/` with `spec=fixes/<case>/spec.md`; assert both complete outputs instruct saving to `claudine/fixes/<case>/plan.md`, not merely that `prompts/` is absent.
- [ ] Add a Claudine diagnostic regression proving dynamic eager-classification drift retains its typed identity, property name, diagnostic code, and actionable message through `CompositionError`, `as_diagnostic`, effective selection, and terminal rendering; do not add ad hoc ANSI output or a Claudine-side path resolver.
- [ ] Confirm direct compose, inline-compose preparation, sequence/task preparation, proxy-target preparation, retry, and resume continue to route through the corrected shared Darkmatter pipeline; add new route-specific tests only if an audited route bypasses that pipeline.
- [ ] **Parallelizable with Claudine tests:** update `claudine/docs/composition.md` and `.claude/skills/claudine/composition.md` together to state that eager caller file parameters are launch-resolved before frontmatter expressions, native semantics and portable presentation remain distinct, and document-authored/lazy references remain source-relative.
- [ ] Review every changed symbol's `///`, `//!`, and inline comments for behavioral drift; delete or update only comments made inaccurate by this fix, and leave unrelated cleanup out of scope.
- [ ] From `claudine/`, run `just test` and `just lint`; from `darkmatter/`, rerun `just test` and `just lint`. Confirm no Level 2 or Level 3 test is needed and no terminal/browser window is launched.
- [ ] Run the two manual `--dry-run` reproduction commands from the specification against the checkout and compare their full target-path lines; both must name the same plan beside the specification.
- [ ] Run GitNexus `detect_changes` with `scope: "compare"` and `base_ref: "main"`; review changed symbols and affected execution flows, and reconcile any scope outside Darkmatter composition/schema diagnostics plus Claudine tests/docs before handoff.
- [ ] Inspect the final diff and report the exact gates run, cross-platform coverage added, and any host-limited Windows checks. Do not run `cargo fmt` and do not commit unless separately requested.

## Completion criteria

The work is complete when caller eager-file overrides are projected once from
the captured launch context before frontmatter pass 1; all later stages reuse
that artifact; unstable dynamic typing fails with a typed diagnostic before
shell/provider execution; lazy and document-owned semantics remain unchanged;
the root and package-area planning workflows derive the same target; and both
Darkmatter and Claudine Level 1 and lint gates pass.
