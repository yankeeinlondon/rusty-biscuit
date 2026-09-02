---
total_phases: 5
created: 2026-09-02
phase: 5
agent: codex/default
yolo: true
---

# Execution Plan: Preserve Caller File-Parameter Provenance Across Proxy Preparation

Reference specification: [`spec.md`](spec.md)

## Goal

Preserve each immutable caller override's raw value and launch-time file-resolution
origin until the active document's effective schema selects a file arm, then
materialize eager and lazy caller files before frontmatter interpolation without
changing document-owned anchoring, layer precedence, or shell-approval caching.
Direct, proxy, retry, resume, loop, inline-compose, and sequence/task routes must
produce the same file identity from the same caller record.

## Scope and risk summary

The implementation extends the eager caller projection already present in
Darkmatter; it must not create a second projection path or add a launch-directory
fallback to expression functions. `biscuit_file::FileReference` remains the
authority for parsing, local/remote classification, candidate plans, recursive
status, lexical normalization, and portable presentation.

GitNexus reports **HIGH** upstream risk for Darkmatter's
`resolve_eager_caller_file_overrides` (22 affected symbols) and
`run_with_registry` (21), **HIGH** risk for Claudine's
`prepare_and_run_active_document` (8), and **CRITICAL** risk for changing the
shared `PrepareOptions` struct (35 direct dependents across 11 modules). The
implementation must therefore review depth-1 callers before changing these
symbols and prefer extending the narrower `CallerInputLayers` carrier over
adding unrelated preparation state to `PrepareOptions`. A new impact result of
HIGH or CRITICAL must be reported before the corresponding edit proceeds.

Pre-existing edits are present in both package areas, including the eager
file-anchoring implementation. Preserve them and keep this fix's changes
separable; do not revert, rewrite, or format unrelated work.

## Implementation ownership matrix

| Input layer | Raw-value owner | Resolution origin | Precedence | Lifetime | Caller materialization |
|---|---|---|---|---|---|
| CLI shorthand / `--set` | Immutable invocation caller record, per winning property | Launch `FileResolutionContext` captured once before source resolution | Highest | Invocation-wide across direct, proxy, retry, resume, loop, inline, and sequence entry | Yes, when the active schema selects exactly one supported file arm |
| Document frontmatter | Active document | Active document `SourceContext` | Below overlays and caller overrides | One document preparation | No |
| Schema default | Effective schema / active document | Active document `SourceContext` | Schema-owned default semantics | One document preparation | No |
| `proxy.with` | Immediate handoff overlay | Target's established document/overlay policy | Below explicit caller overrides, above target frontmatter | Immediate target only; dropped on a later hop that omits `with` | No |
| Runtime mutation | Runtime state | Existing runtime-state policy | Existing runtime layer order | Current invocation/runtime epoch | No |
| Sequence/task-authored value | Sequence or task overlay | Existing sequence/task source policy | Existing task/sequence layer order, below explicit caller overrides | Step/task preparation | No |

The raw caller record is never replaced by its native semantic or portable
presentation projection. Interactive collection remains schema/document-owned
and does not acquire CLI provenance merely because its value is merged into
the effective override map.

## Phase 1 — Baseline, ownership matrix, and failing regression

**Outcome:** the exact shipped failure is reproducible at Level 1, and every
input layer and re-entry route has an explicit expected owner and origin before
the shared carriers change.

- [x] Record `git status --short` for `darkmatter/`, `claudine/`, and this fix directory; identify the pre-existing eager-anchoring diffs and constrain all subsequent review to this fix's additive changes.
- [x] Re-run GitNexus upstream impact analysis for `ComposeOptions`, `resolve_eager_caller_file_overrides`, `run_with_registry`, `CallerInputLayers`, `PrepareOptions`, `canonical_compose_options`, `harness_prepare_options`, and `prepare_and_run_active_document`; review every depth-1 dependent and record any HIGH/CRITICAL warning before editing.
- [x] Write an implementation ownership matrix covering CLI shorthand/`--set`, document frontmatter, schema defaults, `proxy.with`, runtime mutation, and sequence/task-authored values; pin raw-value owner, origin context, precedence, lifetime, and whether the layer is eligible for this fix's caller materialization.
- [x] Add a focused Darkmatter regression around the existing caller-projection tests using a router-style eager `spec` and target-style lazy `spec`; prove the lazy target currently evaluates `frontmatter(spec, 'review_iterations')` against the wrong document anchor while the eager router succeeds.
- [x] Add a non-interactive Claudine Level 1 process fixture using the repository's fake-provider conventions and the shipped `prompts/implement.md` → `prompts/_implement/implement-suggestions.md` route. Launch from a package area with `spec=fixes/<case>/spec.md`, record the prepared target and provider invocation, and assert the pre-fix target fails before provider launch.
- [x] Capture control assertions with the regression: caller precedence over conflicting `proxy.with`, immediate-target-only overlay lifetime, unchanged ordinary strings, unchanged document-authored lazy files, and no process-CWD recapture after invocation capture.
- [x] **Parallelizable:** while the two regressions are being built, inventory the existing retry/resume, multi-hop proxy, inline-compose, sequence JIT, direct/proxy equivalence, and ambient-CWD test helpers to select reusable fixtures rather than introducing a second orchestration harness.
- [x] **Validation checkpoint:** run only the new Darkmatter and Claudine regression cases with nextest-compatible package recipes; confirm they fail for lost caller origin (not setup, provider discovery, terminal focus, or an unrelated schema error) and that all control assertions remain green.

## Phase 2 — Immutable caller records and origin-sensitive identity

**Outcome:** raw caller values and their stable per-property origins are modeled
as one invocation input, survive cloning/re-entry, and participate in every
prepared-state identity that can reuse parameter-derived results.

- [x] Introduce a focused Darkmatter caller-input record/map beside `ComposeOptions::set_overrides`, keyed by winning property and containing the unmodified raw JSON value plus an immutable `FileResolutionContext` (or stable equivalent origin identity); do not copy document-owned, default, `proxy.with`, or sequence/task-owned values into this map.
- [x] Add builder/accessor support that lets callers install raw overrides and matching origin records together while retaining compatibility for library callers that supply only `set_overrides`; document the compatibility behavior without inferring caller origin from the active document.
- [x] Extend the exhaustive `ComposeOptions` field-classification encoder so graph identity and compose-cache identity include canonical raw caller records and exact native origin identity. Bump the appropriate versioned identity domain if required, and prove equal raw values from two origins hash differently while clones remain stable.
- [x] Keep the invocation-wide exact-command shell approval cache on its existing command/policy identity; add a regression assertion showing caller-origin changes do not split or otherwise alter approval-cache reuse.
- [x] Extend Claudine's `CallerInputLayers` as the invocation-wide owner of the caller records, and update `from_options`/`apply_to` so canonical preparation can round-trip them without replacing raw values with materialized or presentation values.
- [x] Capture the launch `FileResolutionContext` once when CLI shorthand and `--set` are parsed, construct one record per explicit caller property, and ensure interactive schema collection does not silently relabel collected/document-owned values as CLI caller input.
- [x] Update all explicit struct initializers and compatibility constructors surfaced by the `CallerInputLayers`/`ComposeOptions` changes; retain current precedence and avoid broad `PrepareOptions` changes unless the reviewed depth-1 callers demonstrate that the narrower carrier cannot satisfy the contract.
- [x] **Parallelizable after the record shape is fixed:** add unit tests for per-property origins, raw-value immutability, clone/round-trip behavior, two-origin cache isolation, absence/null preservation, and the exclusion of document, default, proxy, runtime, and sequence/task layers.
- [x] **Validation checkpoint:** run focused Darkmatter options/cache tests and Claudine caller-input/preparation-service tests; inspect hashes and prepared options to prove raw value + origin survive independently and no precedence or approval-cache behavior changed.

## Phase 3 — Schema-selected eager/lazy materialization in Darkmatter

**Outcome:** the existing pre-interpolation caller projection selects the exact
schema arm, materializes supported eager and lazy caller files through
`FileReference`, and carries distinct raw, semantic, and presentation
projections through both schema passes.

- [x] Generalize `CallerProjection` and `prepare_caller_projection` from eager-only classification to a schema-selected caller-file artifact that records each consumed property's selected mode, native semantic identity, portable presentation value, and diagnostic evidence while leaving the raw caller record untouched.
- [x] Replace `value_has_eager_schema`/first-matching-arm behavior with the same exactly-one-applicable schema selection used by normal validation for scalar properties, arrays, property unions, and root unions; ambiguous and zero-match unions must return control to normal schema validation without guessing a file arm.
- [x] For selected local eager files, parse with `FileReference::new`, resolve through the caller record's captured context, require the existing eager success condition, and retain the established malformed/no-match diagnostic code with raw, base/origin, attempted candidate, and selected-path evidence.
- [x] For selected non-recursive local lazy files, call `FileReference::candidate_plan` with the caller origin, choose and lexically normalize the first ordered candidate without probing for existence, and project its native absolute path plus `biscuit_file::to_portable_string` presentation.
- [x] Detect recursive lazy references through `FileReference::class()` and fail with typed parameter-binding guidance to declare `file(eager)`; classify HTTP(S) through the typed remote-target API so lazy remote identities remain remote and never enter a local candidate plan or filesystem probe.
- [x] Skip absent and explicit-null caller properties, ordinary strings, excluded DM1/lifecycle keys, defaults, and document-owned values; preserve invalid-optional, requiredness, nullability, and schema-default authority in their existing validation stages.
- [x] Install semantic values before frontmatter interpolation pass 1, reuse the same projection artifact through post-shell/pass-2 validation and `EffectiveStateBuilder::with_presentation_values`, and make repeated installation idempotent without re-resolving against the active document.
- [x] Extend classification-drift checks to eager↔lazy/non-file changes after pass 1, including trigger-selected schemas, and fail closed before a shell command runs or a whole-value template can leak into a provider prompt.
- [x] Keep `expression::resolve_arg` and all read-side functions document-relative; add negative tests proving the implementation introduces no global launch fallback and document-authored references still resolve from the active `SourceContext`.
- [x] Preserve direct/proxy typed diagnostic equivalence: extend the existing `FileReferenceDiagnostic` detail only where needed for raw/origin/candidate evidence, keep existing diagnostic codes, and verify Claudine's transparent error selection does not replace the target error with a generic proxy bootstrap failure.
- [x] **Parallelizable after the materializer API is fixed:** split Level 1 test work across (a) scalar/array/union selection, (b) lazy local/recursive/remote behavior, (c) null/default/non-caller controls, (d) pass-2 drift and idempotence, and (e) cross-platform native-versus-portable path assertions.
- [x] **Validation checkpoint:** run the focused Darkmatter compose/schema/error/cache suites. Prove two caller origins select distinct identities, eager/lazy redeclaration changes validation but not origin, later reads own lazy missing-file failures, remote lazy values cause no local probe, and Windows native paths correspond to `/`-portable presentation without manual separator replacement.

## Phase 4 — Canonical Claudine propagation and route equivalence

**Outcome:** every direct and fresh/reused preparation path carries the same
immutable caller record, while other input layers retain their established
origins and lifetimes.

- [x] Thread the caller records from `run_composition_inner` through `prepare_and_run_active_document`, canonical `PrepareOptions` assembly, `PreparedComposition::input_layers`, and `HarnessPromptState` without rebuilding origins from `SourceContext`, `current_dir`, or a proxied target.
- [x] Update `harness_prepare_options` so overlay/runtime layering changes only the effective `set_overrides` map: explicit CLI caller records remain invocation-wide, `proxy.with` remains immediate-target-only, and runtime state is never tagged with the CLI launch origin.
- [x] Verify proxy adoption replaces document identity and the immediate overlay only; a second hop without `with` must retain caller records while dropping the first hop's overlay, and caller overrides must continue to outrank a conflicting overlay.
- [x] Verify retry and resume fresh reads rebuild materialization from the retained raw caller value + origin against the current target schema; loop/same-plan reuse retains the installed semantic identity without re-anchoring or ambient recapture.
- [x] Thread the same caller records through inline-compose and sequence entry/JIT paths. Keep sequence/task params and reserved `state`/`previous`/`next` overlays on their existing origin policy even when a CLI caller file parameter is present in the same preparation.
- [x] Extend the shipped-workflow Level 1 fixture to prove the target reads `review_iterations`, derives `review`, `log`, and optional `design` beside the original specification, reaches the fake provider, and matches direct invocation of `implement-suggestions.md` from identical immutable caller inputs.
- [x] Add focused process cases for proxied retry, proxied resume, a second proxy hop without forwarding, caller-versus-`proxy.with` precedence, inline-compose routing, sequence-task routing, mixed CLI/task-authored file values, and process-CWD mutation after capture. Keep all tests non-interactive and avoid terminal/browser focus.
- [x] Assert route-equivalent failures as well as successes: direct and proxied malformed/eager-missing/lazy-read-missing cases must expose the same diagnostic code, raw spelling, caller origin/base, and selected candidate evidence.
- [x] **Parallelizable after carrier wiring compiles:** build the retry/resume/multi-hop matrix independently from the inline/sequence/mixed-origin matrix, using the existing fake-provider and canonical-preparation helpers.
- [x] **Validation checkpoint:** run focused Claudine library and CLI Level 1 tests with nextest. Confirm every route reaches (or refuses) provider launch identically, no test opens/focuses a terminal or browser, and the exact area-relative shipped command now succeeds.

## Phase 5 — Documentation, impact review, and completion gates

**Outcome:** authoritative and portable documentation describe the same
ownership/materialization contract, and all required package and repository
gates pass with an expected change surface.

- [x] Update `claudine/docs/topics/composition.md` to replace the eager-only caller anchoring description with immutable per-property caller origin, target-schema eager/lazy selection, fresh/reuse re-entry behavior, remote/recursive lazy rules, and the separation of raw, semantic, and presentation projections.
- [x] Apply the same content to `.claude/skills/claudine/composition.md` and update `.claude/skills/claudine/SKILL.md` only if its architecture summary needs a matching wording change; compare the authoritative topic and portable snapshot to prevent drift.
- [x] Review and update affected Darkmatter compose/schema docs and behavior comments. Remove stale eager-only claims, retain the documented interpolation/validation ordering, and avoid narrating implementation details that the types already express.
- [x] Run `just test` and `just lint` from `darkmatter/`, then `just test` and `just lint` from `claudine/`; fix only failures attributable to this work and rerun each failed gate to green.
- [x] Run `just test darkmatter claudine` and `just ci-local darkmatter claudine` from the repository root; record macOS results and review all touched path logic for Windows and Linux compilation/identity semantics even where this host cannot execute those targets.
- [x] Run GitNexus `detect_changes` against `main` and inspect changed symbols/execution flows. Confirm the surface is limited to caller-input identity, Darkmatter schema-selected projection, Claudine canonical propagation, tests, docs, and this plan; investigate any unexpected flow before completion.
- [x] Recheck every acceptance criterion against named tests, verify no Level 2/Level 3 coverage was added, verify no terminal output path changed, and confirm no raw caller record was persisted into Markdown or a Darkmatter content hash.
- [x] **Final validation checkpoint:** rerun the exact `compose prompts/implement.md spec=fixes/<case>/spec.md -y --codex` workflow with a controlled fake provider, compare its prepared target to direct invocation, and capture the full gate summary and any host-only test limitations for handoff.

## Implementation record

Implementation completed on 2026-09-02 without changing the specification's
active lifecycle state. The process regression suite uses the existing fake
provider and covers the shipped router, direct/proxy equivalence, multi-hop
overlay lifetime, proxied retry, inline-compose, and post-capture CWD stability.
Existing retry/resume entry tests and sequence JIT/task tests exercise the same
typed carrier for the remaining re-entry routes, avoiding a second process
orchestration harness.

Validation passed on macOS with package-local `just test` and `just lint`, the
combined root `just test darkmatter claudine`, all 22 gates in
`just ci-local darkmatter claudine`, and Claudine's Windows cross-check. Linux
was reviewed through platform-neutral `Path`/`FileReference` handling but was
not executed on this host. No Level 2 or Level 3 tests were added.
