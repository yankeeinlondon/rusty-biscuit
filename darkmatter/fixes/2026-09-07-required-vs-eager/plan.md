---
total_phases: 5
created: 2026-09-07
phase: 5
agent: codex/default
yolo: true
packages:
  - darkmatter
  - dmls
  - claudine
source_files_during_phase_1:
  - darkmatter/lib/tests/schema_phase_validation.rs
docs_updated_during_phase_1:
  - darkmatter/fixes/2026-09-07-required-vs-eager/plan.md
docs_created_during_phase_1:
  - darkmatter/fixes/2026-09-07-required-vs-eager/baseline.md
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - darkmatter/lib/src/markdown/schemas/about.rs
  - darkmatter/lib/tests/schema_phase_validation.rs
  - darkmatter/cli/tests/schema_about.rs
  - darkmatter/dmls/src/providers/frontmatter.rs
  - darkmatter/dmls/tests/lsp_session.rs
docs_updated_during_phase_2:
  - darkmatter/fixes/2026-09-07-required-vs-eager/plan.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - darkmatter/dmls/tests/lsp_session.rs
docs_updated_during_phase_3:
  - darkmatter/fixes/2026-09-07-required-vs-eager/plan.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - darkmatter/dmls/tests/lsp_session.rs
  - darkmatter/lib/src/markdown/compose/context/options.rs
  - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
  - darkmatter/lib/src/markdown/schemas/triggers/grammar.rs
  - darkmatter/lib/tests/schema_phase_validation.rs
docs_updated_during_phase_4:
  - darkmatter/docs/topics/schema-definition.md
  - darkmatter/docs/inline/schema-validation.md
  - darkmatter/fixes/2026-09-07-required-vs-eager/plan.md
  - claudine/fixes/2026-09-05-inline-flow-and-validations/plan.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4:
  - .claude/skills/darkmatter/schema.md
packages_during_phase_4:
  - darkmatter
  - claudine
source_files_during_phase_5: []
docs_updated_during_phase_5:
  - darkmatter/fixes/2026-09-07-required-vs-eager/plan.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
packages_during_phase_5:
  - darkmatter
  - dmls
  - claudine
source_code:
  - darkmatter/lib/src/markdown/schemas/about.rs
  - darkmatter/lib/src/markdown/schemas/simplified/convert.rs
  - darkmatter/lib/src/markdown/schemas/triggers/grammar.rs
  - darkmatter/lib/src/markdown/compose/context/options.rs
  - darkmatter/lib/tests/schema_phase_validation.rs
  - darkmatter/cli/tests/schema_about.rs
  - darkmatter/dmls/src/providers/frontmatter.rs
  - darkmatter/dmls/tests/lsp_session.rs
documentation:
  - darkmatter/docs/topics/schema-definition.md
  - darkmatter/docs/inline/schema-validation.md
  - darkmatter/fixes/2026-09-07-required-vs-eager/plan.md
  - darkmatter/fixes/2026-09-07-required-vs-eager/baseline.md
  - claudine/fixes/2026-09-05-inline-flow-and-validations/plan.md
  - .claude/skills/darkmatter/schema.md
---

# Execution Plan: Keep `required` and `eager` Independent in DMLS

This plan aligns DMLS authoring behavior and current documentation with
Darkmatter's existing runtime contract. It preserves Darkmatter as the sole
phase-projection authority: `required` controls presence, while `eager`
controls launch-time validation of a supplied value. No parser, converter,
runtime phase engine, public type, or DMLS configuration change is planned
unless a regression test proves an existing implementation defect.

## Phase 1 — Lock the Existing Contract and Regression Surface

- [x] **Task 1.1: Capture repository state and pre-change scope.** Record
  `git status --short` without modifying unrelated work, then run GitNexus
  upstream impact analysis for every existing Rust symbol selected for change.
  Preserve the known HIGH-risk result for `schema_constraint_descriptors()`
  (six direct consumers across Darkmatter CLI/catalog tests and DMLS
  completion/hover) and warn before implementation if any additional HIGH or
  CRITICAL result appears.

- [x] **Task 1.2: Establish focused behavioral baselines.** Run the existing
  Darkmatter required/eager conversion and phase-validation tests, the DMLS
  `eager_schema_fixture_is_clean_and_catalog_driven` LSP test, and the nearby
  schema hover/diagnostic tests. Record which assertions currently fail only
  because they pin obsolete wording and which coverage gaps remain; do not
  change the parser, conversion, or `SchemaPhase` projection to make DMLS tests
  pass.

- [x] **Task 1.3: Turn AC1–AC10 into an implementation test map.** Map syntax,
  strict-mode absence, supplied invalid values, completion detail,
  schema-definition hover, schema-bound instance hover, both array placements,
  array-level `required`, exact LSP ranges, documentation parity, and passive
  behavior to named tests or new cases. Mark the existing four-cell file
  matrix and `schema_phase_validation` coverage as retained regression
  authorities, adding Darkmatter tests only if an acceptance cell is genuinely
  absent.

- [x] **Validation checkpoint 1:** Confirm the baseline demonstrates that
  `string(eager)`, `file(eager)`, `string(required)`, and
  `string(required; eager)` already parse; eager-only absence is allowed by
  Darkmatter; and the intended production edits are limited to the descriptor
  catalog plus DMLS hover metadata inspection. Confirm all tests are passive
  and use temporary workspaces without shell execution, remote access, or host
  window activation.

## Phase 2 — Correct the Catalog Authority and Shared DMLS Metadata

- [x] **Task 2.1: Correct the authoritative `eager` descriptor.** Update the
  `SchemaConstraintDescriptor` in
  `darkmatter/lib/src/markdown/schemas/about.rs` so its description states that
  a supplied value is validated at stabilized launch, eager alone permits
  absence, `required; eager` makes presence mandatory at launch, and eager file
  values/items must resolve to existing files. Update its JSON Schema effect
  to say explicitly that eager does not add the property to the parent's
  `required` list while `file(eager)` still selects the eager file format.
  Preserve `schema_constraint_descriptors()` as the only wording authority.
  **Parallelizable after Phase 1:** this catalog edit can proceed independently
  of Task 2.2, using the specification's fixed wording contract.

- [x] **Task 2.2: Centralize complete constraint-marker inspection in DMLS.** In
  `darkmatter/dmls/src/providers/frontmatter.rs`, introduce or adapt one small
  shared helper that inspects `PropertyAtom::constraints` and
  `PropertyAtom::array_constraints` across every arm returned by `atoms_of`.
  Use it for both `Required` and eager detection so item-level constraints,
  postfix array constraints, and union arms follow identical ownership rules.
  Do not duplicate `project_atom`, call `validate_for_phase`, or synthesize a
  launch/completion phase in DMLS. **Parallelizable after Phase 1:** helper
  implementation and focused unit tests can proceed alongside Task 2.1.

- [x] **Task 2.3: Expose catalog-derived eager metadata in both hover paths.** Use
  the shared constraint inspection and the eager descriptor in
  `meta_schema_definition_hover_body` and `schema_hover_details`. Ensure an
  eager-only definition or instance property displays the catalog description
  without `Required`, while `required; eager` displays both. Preserve existing
  type, literal, enum, default, description, union-selection, and nested-hover
  formatting, including callers of `schema_hover_details` in frontmatter and
  expression/DSL hover.

- [x] **Task 2.4: Add focused provider unit coverage.** Extend the existing
  `frontmatter.rs` unit tests to prove marker detection across union arms,
  `file(eager)[]`, `file[](eager)`, and array-level `required`; assert eager
  prose is taken from `schema_constraint_descriptors()` rather than a DMLS
  string literal. Include eager-only and `required; eager` controls for both
  schema-definition and schema-bound hover bodies.

- [x] **Validation checkpoint 2:** Run the focused Darkmatter catalog tests and
  DMLS provider tests. Confirm all direct consumers identified by the HIGH-risk
  descriptor impact analysis still compile and that completion, schema-about,
  schema-definition hover, and instance hover observe the revised catalog
  wording. Search DMLS production code for a second copy of the eager prose or
  any new `SchemaPhase`/`validate_for_phase` use; both searches must be clean.

## Phase 3 — Prove the Contract Through a Real LSP Session

- [x] **Task 3.1: Update the catalog-driven completion regression.** Replace the
  obsolete exact detail assertion in
  `eager_schema_fixture_is_clean_and_catalog_driven` with the corrected
  descriptor-derived wording. Keep incomplete `string(ea` syntax recoverable
  and assert the returned completion item explicitly says absence is allowed
  unless `required` is also declared.

- [x] **Task 3.2: Add a strict-mode absence discriminator.** Through the normal
  DMLS initialize/open/publish-diagnostics path in
  `darkmatter/dmls/tests/lsp_session.rs`, enable schema strict mode and open one
  temporary document that omits an eager-only property and an adjacent
  `required; eager` property. Assert no `dm.schema.missing_required` names the
  eager-only property, exactly one such diagnostic names the required control,
  and its LSP range is stable. Do not use default mode as the semantic proof;
  default mode suppresses all missing-required diagnostics.

- [x] **Task 3.3: Add supplied-value diagnostic coverage.** In a real LSP
  session, supply an invalid eager-only scalar or file value and assert the
  shared Darkmatter validator's existing typed diagnostic code and exact value
  range. Add a valid or absent neighboring property as a control, and verify
  the range does not expand to the `$schema` block or property definition.
  Keep any file fixture local to the temporary workspace.

- [x] **Task 3.4: Add end-to-end hover coverage.** Request hover from both a
  SimplifiedSchema definition and a schema-bound instance for eager-only and
  `required; eager` properties. Assert eager-only hover contains the exact
  descriptor description and omits the standalone `Required` marker, while the
  combined form contains both. Include nested and union-backed properties as
  needed to protect existing path selection and source ranges.

- [x] **Task 3.5: Add array-placement LSP coverage.** Exercise
  `file(eager)[]`, `file[](eager)`, and an array-level `required` declaration in
  schema-definition and schema-bound instance hover. Assert both eager
  placements disclose timing without claiming presence, and postfix
  `required` displays `Required`. Keep exact definition and instance ranges in
  the fixture assertions.

- [x] **Validation checkpoint 3:** Run the focused `dmls` nextest cases and
  confirm AC1–AC8 through the actual server protocol: valid syntax is clean,
  strict-mode absence distinguishes eager from required, invalid supplied
  values retain typed/ranged diagnostics, completion is catalog-driven, and
  both hover surfaces cover item constraints, postfix constraints, and union
  arms. Confirm the tests never focus or drive an editor window.

## Phase 4 — Remove Current Documentation and Comment Drift

- [x] **Task 4.1: Correct the public schema-definition topic.** Update
  `darkmatter/docs/topics/schema-definition.md` so the universal-constraint
  prose and phase matrix match the four-cell contract; eager-only is optional,
  `required` alone may be absent at launch but is mandatory at completion, and
  `required; eager` is mandatory at both seams. Correct nested-property,
  union-hoisting, and `file(eager)[]` versus `file[](eager)` ownership prose so
  eager never implies presence.

- [x] **Task 4.2: Correct inline validation documentation and the Darkmatter
  skill.** In `darkmatter/docs/inline/schema-validation.md`, change the stale
  explicit-null sentence so eager-only `null` is treated as absence, while
  `required; eager` rejects it at launch and `required` rejects it at
  completion. Update `.claude/skills/darkmatter/schema.md` with the same phase
  matrix and array ownership semantics while preserving the passive-validation
  and eager-file preparation boundaries. **Parallelizable after Phase 2:** the
  two documents can be updated independently once descriptor wording is final.

- [x] **Task 4.3: Correct active upstream planning drift without rewriting
  history.** Update the still-active
  `claudine/fixes/2026-09-05-inline-flow-and-validations/plan.md` wherever it
  says launch requires eager, completion requires required-or-eager, or
  optional eager is impossible. Record the now-authoritative independent-axis
  ruling and adjust active acceptance/test mappings whose names or prose pin
  the superseded model. Leave completed specifications, completed plans,
  reviews, and other historical records unchanged.

- [x] **Task 4.4: Correct only behavior-adjacent stale comments.** Rewrite the
  comment above `type_fragment` in
  `darkmatter/lib/src/markdown/schemas/simplified/convert.rs` to describe eager
  as universal timing metadata with file-specific existence semantics, not a
  presence constraint. Correct the trigger-grammar comment in
  `darkmatter/lib/src/markdown/schemas/triggers/grammar.rs` so rejection is
  attributed to phase metadata and eager file existence being invalid in
  passive matching. Do not alter their behavior or perform unrelated comment
  cleanup.

- [x] **Validation checkpoint 4:** Search all active code, public docs, active
  plans, and `.claude/skills/darkmatter` for the retired claims that eager is
  required, is equivalent to `required; eager`, controls property presence, or
  causes eager-only `null` to fail. Confirm only intentionally historical
  records retain earlier wording. Compare every current surface against the
  descriptor and the four-cell matrix in `spec.md`.

## Phase 5 — Run Package Gates and Verify Scope

- [x] **Task 5.1: Run focused regression suites.** Execute the named Darkmatter
  phase/conversion tests and the complete DMLS LSP regression group first so a
  failure can be attributed to catalog, hover, diagnostics, or range behavior.
  Re-run any non-vacuity controls used by the new fixtures and confirm each
  would fail under the old eager-implies-required wording or marker detection.

- [x] **Task 5.2: Run the Darkmatter package-area gates.** From `darkmatter/`,
  run `just test` and `just lint`. Do not run `cargo fmt`. No L2 or browser gate
  is required because this change uses protocol-level L1 sessions and has no
  terminal, GUI, or browser behavior.

- [x] **Task 5.3: Run the downstream runtime-contract gate.** From the repository
  root, run `just test claudine` and verify Claudine's launch/completion tests
  remain green. Treat any changed runtime phase result as a regression because
  this fix is limited to DMLS authoring metadata, LSP coverage, documentation,
  and comments.

- [x] **Task 5.4: Verify acceptance and changed scope.** Check off an AC1–AC10
  evidence table with exact passing test names and documentation/code
  locations. Run GitNexus `detect_changes(scope: "all")`, confirm affected
  symbols and processes are limited to the planned catalog and DMLS hover
  surface, and review `git diff --check` plus `git diff --stat`. Separate or
  remove any unrelated behavior, formatting, generated artifacts, or
  historical-record edits.

- [x] **Validation checkpoint 5:** All focused tests, the Darkmatter-area
  `just test` and `just lint` gates, and root `just test claudine` pass; completion and
  both hover paths use the single catalog descriptor; DMLS contains no runtime
  phase fork; documentation and active planning agree with the semantic
  matrix; tests remain passive and headless; and the final diff contains only
  the expected Darkmatter, DMLS, skill, public documentation, and active-plan
  files.

### Phase 5 evidence

#### Gates

| Gate | Command | Result |
|---|---|---|
| Focused regression group | `BISCUIT_TEST_FILTER='binary(schema_phase_validation) + binary(schema_about) + test(/eager/) + test(/required/) + test(/schema_hover/) + test(/meta_schema/)' just test` (`darkmatter/`) | 198 run, 198 passed |
| DMLS package | `BISCUIT_TEST_FILTER='package(dmls)' just test` | 657 run, 657 passed |
| Darkmatter area L1 | `just test` (`darkmatter/`) | 7704 run, 7704 passed, 51 skipped |
| Darkmatter area lint | `just lint` (`darkmatter/`) | clean, incl. `zed-dmls` wasm32-wasip2 |
| Downstream runtime contract | `just test claudine` (repo root) | 7080 run, 7080 passed, 13 skipped |

`just test claudine` was run as
`env -u MODEL -u CLAUDINE_INTERACTIVE -u CLAUDINE_PID -u CLAUDINE_SESSION_ID …`
after `cargo build -p darkmatter-cli --bin md`. Both are host-environment
prerequisites of that gate, not consequences of this fix: an ambient `MODEL`
is read by the target-launch overlay tests as a real overlay, and
`inline_compose_writes_hash_that_passes_md_diff` shells out to `target/debug/md`
which claudine's own test graph never builds.

Claudine's runtime phase verdicts are unchanged — `inline_launch_allows_absent_eager_but_rejects_present_invalid_eager`,
`inline_launch_succeeds_with_required_non_eager_properties_absent`,
`eager_without_required_is_still_required_at_completion`, and
`status_report_for_inline_mode_defers_required_non_eager_gaps` all pass.

#### Non-vacuity controls

Each control was applied to production code, measured, and reverted.

| Control | Production edit | Newly failing tests |
|---|---|---|
| Old descriptor wording | `about.rs` eager `description`/`json_schema_effect` restored to the retired text | `eager_is_universal_in_catalog_and_round_trips`, `public_docs_and_skill_describe_required_and_eager_as_independent_axes`, `schema_about_renders_the_shipped_eager_descriptor_verbatim`, `eager_schema_fixture_is_clean_and_catalog_driven` |
| Item-level-only marker detection | `declares_constraint` stops chaining `atom.array_constraints` | `constraint_markers_are_detected_at_item_and_array_level_and_across_union_arms`, `both_hover_surfaces_disclose_array_placements_without_claiming_presence`, `eager_and_required_hover_independently_on_both_surfaces` |
| Eager implies presence | `convert.rs` matches `Constraint::Required \| Constraint::Eager => required = true` | `strict_mode_diagnoses_absent_required_eager_but_never_absent_eager_only` plus 5 Darkmatter phase tests |

The DMLS hover unit tests deliberately survive the first control: they compare
hover prose to `schema_constraint_descriptors()` at runtime, which is the AC6
property being asserted. `supplied_invalid_eager_values_are_diagnosed_on_the_value_range`
is a coverage test for AC3 rather than an axis discriminator; it is non-vacuous
because it panics when the typed diagnostic is absent and pins exact ranges.

#### AC1–AC10 evidence

| AC | Evidence |
|---|---|
| AC1 | `eager_is_universal_in_catalog_and_round_trips` (`lib/tests/schema_phase_validation.rs:336`), `eager_controls_validation_timing_and_required_controls_presence` (`:28`), `eager_schema_fixture_is_clean_and_catalog_driven` (`dmls/tests/lsp_session.rs:3624`) |
| AC2 | `strict_mode_diagnoses_absent_required_eager_but_never_absent_eager_only` (`dmls/tests/lsp_session.rs:3715`) — strict mode on, eager-only clean, `required; eager` control diagnosed once with a pinned range |
| AC3 | `supplied_invalid_eager_values_are_diagnosed_on_the_value_range` (`dmls/tests/lsp_session.rs:3783`) — `dm.schema.type_mismatch` on the scalar and on the nested file value, never the `$schema` block |
| AC4 | `eager_and_required_hover_independently_on_both_surfaces` (`dmls/tests/lsp_session.rs:3913`), `union_arms_and_nested_properties_disclose_eager_without_presence` (`:3974`), `schema_definition_hover_separates_eager_timing_from_required_presence` / `schema_bound_instance_hover_separates_eager_timing_from_required_presence` (`dmls/src/providers/frontmatter.rs:3374`, `:3395`) |
| AC5 | `eager_schema_fixture_is_clean_and_catalog_driven`; descriptor at `lib/src/markdown/schemas/about.rs:179` |
| AC6 | `eager_hover_prose_is_taken_from_the_darkmatter_descriptor_catalog` (`dmls/src/providers/frontmatter.rs:3355`); `grep -rn "stabilized launch" dmls/src/` is empty; single helper `eager_hover_line` feeds both hover bodies |
| AC7 | `grep -rn "SchemaPhase\|validate_for_phase\|project_atom" dmls/src/` is empty |
| AC8 | `file_lazy_eager_required_matrix` (`lib/src/markdown/schemas/simplified/convert.rs:1626`), `eager_array_placement_owns_items_or_property` (`lib/tests/schema_phase_validation.rs:163`), `array_level_required_owns_presence_independently_of_array_level_eager` (`:175`), `required_eager_is_mandatory_at_both_phases_including_explicit_null` (`:197`); DMLS `HOVER_MATRIX` covers `file(eager)[]`, `file[](eager)`, `file[](required)`, `file[](required; eager)` |
| AC9 | `public_docs_and_skill_describe_required_and_eager_as_independent_axes` (`lib/tests/schema_phase_validation.rs:439`) asserts both public topics and `.claude/skills/darkmatter/schema.md` against `RETIRED_EAGER_CLAIMS`, the descriptor, *and* re-derives the four-cell matrix from the real validator; `shipped_schema_and_trigger_corpus_parses_passively` (`:549`) is the passive shipped-artifact corpus and `real_shipped_inline_schema_uses_normal_resolution_and_phase_path` (`:637`) the end-to-end shipped-artifact path |
| AC10 | `dsl_requests_spawn_no_processes_and_open_no_sockets` (`dmls/tests/no_side_effects.rs`); every new LSP fixture writes into `tempfile::tempdir()` and drives `ClientFixture` over stdio only |

#### Changed scope

`detect_changes(scope: "all")` reports **risk low, 0 affected processes**. The
changed symbols are exactly `SCHEMA_CONSTRAINT_DESCRIPTORS`, the DMLS hover
helpers (`declares_constraint`, `is_required`, `is_eager`, `eager_hover_line`,
`meta_schema_definition_hover_body`, `schema_hover_details`), the new and
rewritten tests, and the three comment/doc-comment sites (`type_fragment`,
`enforce_match_safe`, `ComposeOptions::with_schema_phase`).

`git diff --check` is clean for this fix's files. Two pre-existing trailing
whitespace hits live in `claudine/docs/shared-resources/agent-skills/agent-skills.md`,
which belongs to unrelated in-flight work in this worktree and was left
untouched.
