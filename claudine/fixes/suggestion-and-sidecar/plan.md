---
total_phases: 7
created: 2026-09-03
phase: 7
agent: codex/default
yolo: "true"
source_files_during_phase_1: []
docs_updated_during_phase_1:
  - claudine/fixes/suggestion-and-sidecar/plan.md
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - claudine/cli/src/completion/operation_file.rs
  - claudine/cli/src/completion/operation_file/recovery_tests.rs
docs_updated_during_phase_2:
  - claudine/fixes/suggestion-and-sidecar/plan.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - darkmatter/lib/src/markdown/schemas/advisory.rs
  - darkmatter/lib/src/markdown/schemas/mod.rs
  - darkmatter/lib/src/markdown/schemas/resolve.rs
docs_updated_during_phase_3:
  - claudine/fixes/suggestion-and-sidecar/plan.md
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - darkmatter/lib/src/markdown/compose/context/report.rs
  - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
  - darkmatter/lib/src/markdown/compose/schema_validation.rs
  - darkmatter/lib/src/markdown/compose/tests/frontmatter.rs
  - darkmatter/lib/src/markdown/compose/transclusion/types.rs
  - darkmatter/lib/src/markdown/schemas/mod.rs
  - darkmatter/lib/src/markdown/schemas/tests/mod.rs
  - darkmatter/lib/src/markdown/schemas/triggers/assemble.rs
docs_updated_during_phase_4:
  - claudine/fixes/suggestion-and-sidecar/plan.md
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - claudine/cli/tests/compose_schema_cli.rs
  - darkmatter/cli/src/commands/schema/validate.rs
  - darkmatter/cli/tests/schema_validate.rs
  - darkmatter/dmls/src/diagnostics/frontmatter.rs
  - darkmatter/dmls/src/overlay/schema.rs
  - darkmatter/dmls/tests/lsp_session.rs
docs_updated_during_phase_5:
  - claudine/fixes/suggestion-and-sidecar/plan.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
source_files_during_phase_6: []
docs_updated_during_phase_6:
  - claudine/docs/topics/completions/shell-completions.md
  - claudine/fixes/suggestion-and-sidecar/plan.md
  - darkmatter/docs/inline/schema-validation.md
  - darkmatter/docs/topics/schema-definition.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6:
  - .claude/skills/claudine/completions/shell-completions.md
  - .claude/skills/darkmatter/schema.md
source_files_during_phase_7: []
docs_updated_during_phase_7:
  - claudine/fixes/suggestion-and-sidecar/plan.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7: []
packages_during_phase_7:
  - darkmatter
  - darkmatter-cli
  - dmls
  - claudine-cli
packages:
  - darkmatter
  - darkmatter-cli
  - dmls
  - claudine-cli
source_code:
  - claudine/cli/src/completion/operation_file.rs
  - claudine/cli/src/completion/operation_file/recovery_tests.rs
  - darkmatter/lib/src/markdown/schemas/advisory.rs
  - darkmatter/lib/src/markdown/schemas/mod.rs
  - darkmatter/lib/src/markdown/schemas/resolve.rs
  - darkmatter/lib/src/markdown/compose/context/report.rs
  - darkmatter/lib/src/markdown/compose/pipeline/mod.rs
  - darkmatter/lib/src/markdown/compose/schema_validation.rs
  - darkmatter/lib/src/markdown/compose/tests/frontmatter.rs
  - darkmatter/lib/src/markdown/compose/transclusion/types.rs
  - darkmatter/lib/src/markdown/schemas/tests/mod.rs
  - darkmatter/lib/src/markdown/schemas/triggers/assemble.rs
  - claudine/cli/tests/compose_schema_cli.rs
  - darkmatter/cli/src/commands/schema/validate.rs
  - darkmatter/cli/tests/schema_validate.rs
  - darkmatter/dmls/src/diagnostics/frontmatter.rs
  - darkmatter/dmls/src/overlay/schema.rs
  - darkmatter/dmls/tests/lsp_session.rs
documentation:
  - claudine/fixes/suggestion-and-sidecar/plan.md
  - claudine/docs/topics/completions/shell-completions.md
  - darkmatter/docs/inline/schema-validation.md
  - darkmatter/docs/topics/schema-definition.md
spec: ./spec.md
---

# Suggestion Recovery and Schema Sidecar Advisory Execution Plan

This plan implements the two fixes defined in [`spec.md`](spec.md): make the
bounded repository suggestion walk tolerate per-entry failures, and surface a
single typed advisory when a referenced YAML sidecar strongly resembles a bare
SimplifiedSchema but lacks a supported envelope.

## Success criteria

- Explicit operation-file misses retain sorted, repository-relative basename
  suggestions despite broken walk entries, while preserving the 20,000-visit
  budget, five-result cap, symlink containment, and primary diagnostic shape.
- Darkmatter continues interpreting bare YAML maps as raw JSON Schema, but a
  conservative classifier emits one stable
  `dm.schema.missing_simplified_envelope` advisory through library validation,
  composition, the `md` CLI, Claudine, and DMLS.
- All behavior is covered by platform-independent L1 tests, with a Unix-only
  real dangling-symlink regression, and the package-area and cross-platform
  gates in AC7 pass.

## Phase 1: Baseline, impact analysis, and contract fixtures

**Goal:** Establish current behavior and the exact change boundaries before
editing shared symbols.

- [x] Run GitNexus upstream impact analysis for every symbol that will be edited, at minimum `collect_repository_suggestions`, `repository_basename_suggestions`, `parse_yaml_referenced_file`, `ResolvedSchema`, `EffectiveSchema`, `ValidationReport`, `DarkmatterSchemas::validate`, `ComposeReport`, `run_with_registry`, `SchemaBundle`, DMLS `diagnostics`, and `run_validate`; record direct callers, affected flows, and risk, and stop for user review before implementation if any result is HIGH or CRITICAL.
- [x] Run the narrow existing L1 tests around `claudine/cli/src/completion/operation_file/recovery_tests.rs`, Darkmatter schema resolution/composition, `darkmatter/cli/tests/schema_validate.rs`, and DMLS schema diagnostics to capture a green baseline and confirm the current “any walk error clears matches” and “bare map produces no warning” behaviors.
- [x] Add or identify reusable fixtures for a valid bare simplified-looking map, each excluded raw-JSON/custom-vocabulary shape from AC5, repeated schema validation, a root union/reference merge, and a suggestion iterator that can prove whether item 20,001 was consumed.
- [x] Confirm the implementation preserves the passive schema boundary: classification uses only the YAML value already read and `parse_yaml_schema`, with no second read, import/file resolution, network access, mutation, or reinterpretation as SimplifiedSchema.

**Validation checkpoint:** Baseline tests pass; impact results and target files
are documented; each new fixture maps to AC2, AC4, or AC5.

### Phase 1 evidence

#### Impact boundary

GitNexus was run upstream to depth three with tests included. The two suggestion
symbols and two shared schema/composition entry points require review before
their later implementation phases because their transitive risk is elevated.
No implementation code was changed in this phase.

| Target | Direct callers / constructors | Affected flows | Risk |
| --- | --- | --- | --- |
| `collect_repository_suggestions` | `repository_basename_suggestions` | none indexed | HIGH: 6 symbols across completion, commands, and prep |
| `repository_basename_suggestions` | `recover_operation_file` | none indexed | CRITICAL: 12 symbols across completion, compose, commands, prep, and harness orchestration |
| `parse_yaml_referenced_file` | `load_schema_from_path_in_context` | none indexed | LOW: 3 symbols in schema resolution |
| `ResolvedSchema` | constructed by raw/YAML parsing, root-union resolution, standalone resolution, and root-aware YAML resolution | none indexed | UNKNOWN: the Rust struct node is present in the relation graph but is not addressable by the impact API |
| `EffectiveSchema` | constructed by `effective_for_with_override`; consumed by schema validation, completion, cleanup, CLI assignment, and DMLS overlay code | covered by the validation/composition entry points below | UNKNOWN: impact resolved the impl node rather than the data type |
| `ValidationReport` | constructed by `DarkmatterSchemas::validate` and `EffectiveSchema::validate_instance` | covered by the validation consumers below | UNKNOWN: the Rust struct node is present but is not addressable by the impact API |
| `DarkmatterSchemas::validate` | 75 direct callers, including schema cleanup, expression validation, `md` validation, and library/integration tests | none indexed | CRITICAL: 112 symbols across five modules |
| `EffectiveSchema::validate` | no direct callers indexed | none indexed | LOW |
| `ComposeReport` | created and merged throughout compose, transclusion, page-block, link, and shell stages | covered by `run_with_registry` and compose pipeline analysis | UNKNOWN: impact resolved the impl node rather than the data type |
| `run_with_registry` | `run` and `Markdown::run_compose_pipeline_internal` | `run_compose_pipeline` and `run_compose_pipeline_internal` | HIGH: 21 symbols across compose, context, and transclusion |
| `SchemaBundle` | constructed by DMLS `assemble`; consumed by overlay and frontmatter diagnostics | DMLS overlay/diagnostic path | UNKNOWN: the Rust struct node is present but is not addressable by the impact API |
| DMLS frontmatter `diagnostics` | `providers::frontmatter::diagnostics`, then provider dispatch | DMLS diagnostics publication | UNKNOWN: this function is absent from the current GitNexus symbol index |
| schema CLI `run_validate` | `commands::run_subcommand` | `run_subcommand` | LOW: 3 symbols across command dispatch |

#### Requirement-to-test map and fixtures

| Requirement | Concrete public observation | Fixture / targeted test |
| --- | --- | --- |
| AC2 entry errors are local and budgeted | first/middle errors retain later sorted matches; exactly 20,000 iterator items are consumed; item 20,001 is not polled; an in-budget match survives exhaustion | replace the old walk-error and budget characterizations with a reusable `CountingIterator` backed by `Arc<AtomicUsize>` and exact `SuggestionEntry` values in `operation_file/recovery_tests.rs` |
| AC2 recovery state remains coherent | rendered suggestions and `err.detail.suggestions` are identical while code, candidates, and no-match detail remain unchanged | extend the existing explicit-no-match recovery fixture using the original `./docs/unifi/protect.md` input and repository-relative `protect.md` matches |
| AC3 real walker containment and recovery | a dangling `.aaa/broken -> missing` entry does not hide `zzz/access.md`; directory symlink escapes remain absent | Unix-only temp-repository test plus the existing Unix/Windows directory-symlink tests |
| AC4 bare-map advisory | valid composition and validation retain raw JSON Schema output and emit one advisory after one or two validation passes | shared sidecar body `source_marker: string(required)\nspec: 'file(eager; required)'\ncaller_spec: 'file(eager; required)'`; existing `yaml_without_schema_key_is_treated_as_json_schema` remains the unchanged-output characterization |
| AC4 root merge and deduplication | duplicate references collapse to one `(kind, path)` advisory and distinct paths are sorted through a root union | temp directory with two bare-map sidecars and a root union containing repeated string references; existing `root_union_string_arm_records_referenced_file` anchors referenced-file retention |
| AC5 false-positive exclusions | no advisory and unchanged validity/output for every excluded representation | table-driven corpus: `type: object` plus `properties`, `format: string`, `title: string(required)`, `$comment: string`, `$custom: string`, `x-custom: string`, sole-root `$schema:` envelope, `kind: schema` plus `types:`, whole-file scalar reference, mixed non-string value, and invalid simplified type string |
| AC6 consumer projections | the same identity/path/message is observable in library, pretty/JSON CLI, Claudine normal/silent, and DMLS consumer-range outputs | later L1 integration fixtures use the shared bare sidecar through each normal invocation path; DMLS also exercises closed-sidecar and cache invalidation |

All schema fixtures use native and quoted YAML scalar spellings where those
representations are semantically relevant. Persistence-driven behavior is not
changed by this fix, so a read/write/read round trip is not applicable; DMLS's
dependency-cache invalidation test supplies the required repeated-read boundary.

#### Green baseline and passive boundary

The following L1 baselines passed on macOS:

- Claudine repository suggestion characterizations: 7 passed, including the
  current `repository_suggestion_walk_error_returns_empty` and budget behavior.
- Claudine explicit recovery propagation: 1 passed.
- Darkmatter raw-YAML fallback, root-union reference retention, and composition
  schema validation: 3 passed.
- `darkmatter-cli` `schema_validate` integration binary: 33 passed.
- DMLS `frontmatter_schema_intelligence`: 1 passed.
- Full Claudine package-area L1 gate: 6,712 passed, 11 tier-gated tests skipped.
- Full Claudine package-area lint gate: transport/error guards, documentation
  guard, formatting checks, and Clippy passed for all five area packages.

The future classifier belongs only in `parse_yaml_referenced_file`, after the
already-read YAML value fails standalone-envelope recognition and before raw
JSON Schema fallback. It may call the existing passive `parse_yaml_schema`
parser on that in-memory value. It must not read the sidecar again, resolve
imports or file references, access the network, mutate content, or cause the
value to be interpreted as SimplifiedSchema.

## Phase 2: Repair repository suggestion collection

**Goal:** Make entry failures local to one visit without weakening containment
or bounded-work guarantees. This phase is independent of Phases 3–5 and may be
implemented in parallel with them.

- [x] Update `collect_repository_suggestions` in `claudine/cli/src/completion/operation_file.rs` so every yielded `Ok` or `Err` consumes one of exactly 20,000 visits, errors are skipped, item 20,001 is never polled, and sorted/deduplicated matches found inside the budget are retained up to `MAX_SUGGESTIONS`.
- [x] Keep root validation in `repository_basename_suggestions` before walker construction; map walker errors, failed `strip_prefix`, and failed symlink `metadata` to skipped budgeted entries while retaining `follow_links(false)` and all existing ignore/filter rules.
- [x] Update the function and module documentation that currently says budget exhaustion returns no suggestions so it states the actual invariant: unusable roots return none, entry failures are skipped, and budget exhaustion returns matches accumulated within the bound.
- [x] Replace `repository_suggestion_walk_error_returns_empty` with synthetic first-error and middle-error tests that prove later matches survive and errors count against the budget.
- [x] Replace the budget regression with an instrumented boundary test proving exactly 20,000 items are visited, item 20,001 is not consumed, and an in-budget match survives exhaustion; retain empty-iterator, non-directory-root, sorting, deduplication, five-result cap, and exact-basename coverage.
- [x] Add a `#[cfg(unix)]` temp-repository test with `.aaa/broken -> missing` sorting before `zzz/access.md`; assert the real repository seam returns `zzz/access.md`. Keep the existing Unix and Windows directory-symlink escape tests unchanged.
- [x] Extend the recovery-level assertion to verify the rendered suggestion list and `err.detail.suggestions` use the same sorted, capped list while the diagnostic code, attempted candidates, and no-match fields remain unchanged.

**Validation checkpoint:** Run the targeted Claudine completion/recovery L1
tests. AC1–AC3 pass on macOS; the synthetic tests contain no Unix-only APIs and
compile on Windows and Linux.

### Phase 2 evidence

- Entry-local recovery is covered by
  `repository_suggestion_first_error_keeps_later_match` and
  `repository_suggestion_middle_error_keeps_later_match`. Both failed against
  the prior collector and pass after the fix.
- The exact work bound and retained-result contract are covered by
  `repository_suggestion_budget_visits_exactly_the_bound_and_retains_matches`;
  its instrumented iterator panics if item 20,001 is polled and includes an
  error plus a match at visit 20,000. Empty input remains covered by
  `repository_suggestion_empty_iterator_returns_empty`.
- The real walker regression is covered on Unix by
  `repository_suggestions_skip_earlier_dangling_symlink`; the existing Unix and
  Windows directory-symlink containment tests remain unchanged.
- The original `./docs/unifi/protect.md` miss is covered by
  `recovery_enriches_explicit_no_match_without_selecting_suggestion`, which
  asserts sorted five-result capping, the rendered list, structured
  `err.detail.suggestions`, code, no-match disposition, and attempted candidate
  evidence.
- The focused Nextest selection passed 11 tests. The full Claudine `just test`
  gate passed 6,714 tests with 11 expected higher-tier skips, and `just lint`
  passed all guards, formatting checks, and Clippy checks across the five area
  packages. No pre-existing failures were observed. Persistence is unchanged,
  so a read/write/read round trip is not applicable.

## Phase 3: Introduce the typed schema advisory at resolution

**Goal:** Create one authoritative advisory and attach it conservatively at the
raw-JSON-Schema fallback seam.

- [x] Define a small public typed `SchemaAdvisory` and kind/code identity in `darkmatter/lib/src/markdown/schemas/` with source `darkmatter.schema`, code `dm.schema.missing_simplified_envelope`, referenced `PathBuf`, and the prescribed message naming both `$schema:` and `kind: schema` + `types:` envelopes.
- [x] Add one named Draft 2020-12 keyword classifier in `schemas/resolve.rs` covering the core, applicator, validation, metadata, format, and unevaluated vocabularies; also reserve every root key beginning with `$` or `x-`.
- [x] Add a passive helper that returns the advisory only when the root is a non-empty map, every value is a string, the complete map succeeds under `parse_yaml_schema`, and none of its keys is recognized or reserved.
- [x] Invoke the helper in `parse_yaml_referenced_file` only after `parse_standalone_schema_document` returns `None` and before raw-schema fallback; continue returning the same raw JSON Schema and validity semantics.
- [x] Add an advisory collection to `ResolvedSchema`; update every constructor and root-union/reference merge to sort and deduplicate by `(kind, path)` while preserving existing import, example, origin, and referenced-file behavior.
- [x] Add resolver tests for the positive bare-map case and every AC5 negative case: object/properties JSON Schema, scalar `format`, `title`, and `$comment` schemas, `$` and `x-` vocabularies, pure and kinded SimplifiedSchema envelopes, whole-file scalar references, and a mixed/invalid bare map.
- [x] Add merge tests showing duplicate references produce one advisory and multiple advisory paths are deterministic, sorted, and retained through root unions.

**Validation checkpoint:** Targeted Darkmatter resolver tests pass; snapshots or
assertions prove the JSON Schema output is unchanged and classification performs
no additional I/O.

### Phase 3 evidence

- `bare_simplified_map_remains_raw_json_schema_and_reports_advisory` uses the
  motivating native/quoted YAML map verbatim, asserts the stable kind, source,
  code, path, and both envelope remedies, and proves the unchanged raw schema
  still validates unrelated input.
- `bare_map_advisory_excludes_json_schema_envelopes_and_invalid_maps` covers
  object/properties JSON Schema; scalar `format`, `title`, and `$comment`;
  reserved `$` and `x-` vocabularies; both SimplifiedSchema envelopes; a
  whole-file scalar reference; a mixed native scalar map; and an invalid
  simplified type string.
- `root_union_sorts_and_deduplicates_schema_advisories` covers repeated and
  distinct referenced paths through both an inline root union and a referenced
  standalone root-union envelope. Both products retain one advisory per
  `(kind, path)` in deterministic path order.
- The unchanged `shipped_schema_corpus_is_passively_classified_without_resolution`
  corpus test and the normal public validation-path tests
  `feature_review_resolves_as_a_bare_name_reference` and
  `feature_review_reference_validates_a_review_document` pass against shipped
  artifacts. Persistence is unchanged, so a read/write/read round trip is not
  applicable.
- Focused resolver selection: 5 passed. Darkmatter `just test`: 7,620 passed,
  50 higher-tier tests skipped. Darkmatter `just lint`: passed for the library,
  CLI, and DMLS. Claudine `just test`: 6,715 passed, 11 higher-tier tests
  skipped. Claudine `just lint`: all guards and all five package checks passed.
  No pre-existing failures were observed.
- GitNexus post-change detection against `HEAD^` reports low risk and no
  affected execution flows. `git diff --check` is clean. No formatting command
  was run. A separate workspace process committed the three Darkmatter source
  files as `82bf58acd` during verification; this implementation session did not
  stage or commit files.

## Phase 4: Propagate advisories through validation and composition

**Goal:** Carry the resolver’s semantic advisory through shared library products
without reparsing or duplicate emission.

- [x] Add advisories to `EffectiveSchema` beside dependency paths and thread them through `DarkmatterSchemas::effective_for` assembly, baseline/reference merges, and all no-schema/effective-schema construction paths.
- [x] Add advisories to `ValidationReport`; ensure `DarkmatterSchemas::validate` and every `EffectiveSchema::validate*` variant return the effective advisory set without changing `valid`, `problems`, or `pending` semantics.
- [x] Audit all `ValidationReport` struct literals and public docs/tests so the new field is initialized consistently and downstream callers do not synthesize advisories independently.
- [x] Map schema advisories to `ComposeWarning` at the schema-validation stage, preserving the stable code/source information where the warning model permits it and using the root document as the consumer identity.
- [x] Deduplicate schema advisories when first-pass/post-shell validation and transclusion reports merge so one referenced path produces exactly one warning per root `ComposeReport`; do not globally collapse unrelated existing warnings.
- [x] Add library tests proving one warning for one and two schema-validation passes, no duplicate after transclusion/report merging, no warning for AC5 fixtures, and unchanged composed document content and validation outcome.

**Validation checkpoint:** Targeted Darkmatter schema and compose L1 tests pass;
AC4 and the library portion of AC6 are observable from `ValidationReport` and
`ComposeReport` without reparsing.

### Phase 4 evidence

- `bare_sidecar_advisory_reaches_every_validation_entry_point` covers
  `DarkmatterSchemas::validate` and all five `EffectiveSchema::validate*`
  variants with the motivating native/quoted YAML map. It asserts unchanged
  validity, problems, and pending state plus the typed source, code, path, and
  both envelope remedies.
- `bare_sidecar_advisory_survives_baseline_and_reference_assembly` and the
  no-schema assertion cover baseline/reference merging and the vacuous report.
  `excluded_bare_sidecar_shapes_remain_advisory_free_through_validation`
  carries the successful AC5 corpus through the public validation path; the
  whole-file scalar and invalid-type error shapes remain covered at the Phase 3
  passive resolver seam.
- `bare_sidecar_composes_with_one_typed_warning_in_one_validation_pass` and
  `bare_sidecar_composes_unchanged_with_one_typed_warning_across_two_passes`
  cover normal composition and trigger-enabled repeat validation while
  asserting unchanged document content and frontmatter.
- `schema_advisory_is_not_duplicated_when_transclusion_reports_merge` and
  `transcluded_schema_advisory_uses_root_document_as_consumer` cover report
  merging and root identity. `compose_report_merge_keeps_duplicate_non_schema_warnings`
  proves the deduplication does not collapse unrelated warning kinds.
- The focused Nextest selection passed all 8 tests. Persistence is unchanged,
  so a read/write/read round trip is not applicable.
- Darkmatter `just test` passed 7,628 tests with 50 expected higher-tier skips;
  `just lint` passed for `darkmatter`, `darkmatter-cli`, and `dmls`.
- Downstream Claudine `just test` passed 6,715 tests with 11 expected
  higher-tier skips; `just lint` passed all guards and all five package checks.
- `sniff repo package-dependencies darkmatter --plain` confirmed the direct
  Darkmatter consumers; the Darkmatter area gate covered its CLI and DMLS, and
  the explicit downstream Claudine area gates covered `claudine`,
  `claudine-cli`, and `claudine-gen`. GitNexus detected one affected compose
  pipeline at medium aggregate risk. `git diff --check` is clean. No
  pre-existing failures were observed, and no formatting command was run.

## Phase 5: Project the advisory to CLI, Claudine, and DMLS consumers

**Goal:** Expose the same advisory identity and message on every required
surface. After Phase 4, the three task groups below are parallelizable.

- [x] **[Parallel: `md` CLI]** Extend `darkmatter/cli/src/commands/schema/validate.rs`’s validated outcome to retain `ValidationReport.advisories`; render advisories with `TerminalRenderable`/`Prose` in normal pretty output and serialize a structured `warnings` array in JSON output, while preserving `valid`, `problems`, exit codes, and documented `--quiet` success-output suppression.
- [x] **[Parallel: `md` CLI]** Add `darkmatter/cli/tests/schema_validate.rs` coverage for pretty and JSON output, stable code/source/path/message, valid exit status despite the warning, and quiet-mode suppression; no L2 terminal test is needed.
- [x] **[Parallel: Claudine]** Add an ordinary compose/inline-compose L1 CLI test using a bare sidecar and assert Darkmatter’s existing compose-warning path renders the advisory once; add a `--silent` assertion proving normal Claudine warning suppression, with no Claudine-specific schema parsing or message construction.
- [x] **[Parallel: DMLS]** Retain the advisory through `darkmatter/dmls/src/overlay/schema.rs`’s `SchemaBundle`, and add a dedicated projection in `darkmatter/dmls/src/diagnostics/frontmatter.rs` using source `darkmatter.schema`, code `dm.schema.missing_simplified_envelope`, warning severity, and the consuming Markdown document’s `$schema` value range.
- [x] **[Parallel: DMLS]** Ensure the advisory is emitted only for the consuming document, never duplicated onto the sidecar buffer, and does not require the referenced sidecar to be open.
- [x] **[Parallel: DMLS]** Add DMLS L1 tests for exact code/source/severity/range, one-warning deduplication, closed-sidecar behavior, AC5 suppression, and dependency-cache invalidation when the referenced sidecar changes between advisory and non-advisory content.

**Validation checkpoint:** AC6 passes independently on the library, `md` pretty,
`md --format json`, Claudine normal/silent, and DMLS surfaces; all report the
same path and semantic identity without changing validity or process status.

### Phase 5 evidence

- `schema_validate_pretty_reports_bare_sidecar_advisory_without_failing`,
  `schema_validate_json_reports_structured_bare_sidecar_advisory`, and
  `schema_validate_quiet_suppresses_bare_sidecar_advisory` cover the motivating
  native/quoted YAML sidecar through the compiled `md` binary. They assert the
  stable source, code, resolved path, message and both remedies, unchanged
  validity/problems, exit status 0, and quiet suppression. The existing
  `schema_validate_legacy_json_output_is_byte_identical` test also proves that
  advisory-free JSON remains byte-identical; `warnings` is additive only when
  findings exist.
- `compose_and_inline_bare_sidecar_advisory_render_once_and_silent_suppresses_it`
  drives the compiled Claudine binary and a real provider process stub through
  ordinary compose and inline-compose preparation. It proves the shared
  Darkmatter warning path renders the referenced path and advisory message
  exactly once on both surfaces, while `--silent` suppresses it without any
  Claudine-specific schema parsing or message construction; inline-compose
  still persists the provider result.
- `bare_sidecar_advisory_projects_to_consumer_and_tracks_dependency_changes`
  exercises an in-memory LSP session. It asserts the exact code, source,
  warning severity, consuming `$schema` value range, consumer-only ownership,
  closed-sidecar behavior, and a bare → enveloped → bare disk
  read/write/read cycle that clears and restores the advisory through cache
  invalidation.
- `bare_sidecar_advisory_deduplicates_root_union_and_excludes_raw_json_schema`
  proves repeated reference arms publish one warning and the representative
  `type: object` + `properties` AC5 raw-JSON shape remains warning-free. The
  complete passive AC5 corpus and shipped-schema normal-path tests from Phase 3
  also passed in the full Darkmatter gate.
- Focused red/green evidence: before implementation, the two `md` projection
  tests and both DMLS tests failed because the advisory was absent; the quiet
  test passed under the old output, and the Claudine test exposed the already
  wired message but no separate implementation gap. After implementation all
  six targeted tests passed. The first full Darkmatter run found one induced
  legacy-JSON compatibility failure; conditional warning serialization fixed
  it, and the targeted compatibility test passed before rerunning the area.
- Darkmatter `just lint` passed for `darkmatter`, `darkmatter-cli`, and `dmls`;
  `just test` passed all 7,633 L1 tests with 50 expected higher-tier skips.
  Claudine `just lint` passed all guards and five package checks; `just test`
  passed all 6,716 L1 tests with 11 expected higher-tier skips. No pre-existing
  failures were observed, and no L2/L3/browser tests were required.
- Sniff dependency inspection confirmed the implementation surface is limited
  to `darkmatter-cli`, `dmls`, and downstream `claudine-cli`; both affected
  package-area gates were run. GitNexus change detection reports the expected
  composition pipeline as the sole affected indexed flow at medium aggregate
  risk. `git diff --check` is clean. No formatting, staging, or commit command
  was run, and no Claudine skill update was warranted because Claudine's
  existing warning policy and architecture did not change.

## Phase 6: Update authoring documentation and skill guidance

**Goal:** Put the corrected contracts where schema and Claudine authors look.
This phase can begin in parallel with Phase 5 once Phase 3’s public names and
message are stable.

- [x] Update `darkmatter/docs/topics/schema-definition.md` to state that referenced YAML is SimplifiedSchema only under the sole-root `$schema:` envelope or `kind: schema` + `types:` envelope; a qualifying bare map remains raw JSON Schema and receives the advisory.
- [x] Correct the older disambiguation wording in `darkmatter/docs/inline/schema-validation.md` and add the warning behavior and both remedies.
- [x] Update `.claude/skills/darkmatter/schema.md` with the same authoritative envelope, raw-fallback, and conservative-warning contract.
- [x] Update `claudine/docs/topics/completions/shell-completions.md` and `.claude/skills/claudine/completions/shell-completions.md` to clarify that unreadable/error entries are skipped within the work budget rather than aborting advisory suggestions.
- [x] Review touched `//!`, `///`, and inline comments against the changed behavior; remove or correct drift while avoiding unrelated comment cleanup.
- [x] Run `md hash <file> --save` only for edited Markdown files that already contain a managed `hash:` property, then inspect each diff to confirm only the expected hash metadata changed.

**Validation checkpoint:** Documentation agrees with D1–D6, examples use the two
actual envelopes, and all managed hashes verify through Darkmatter.

### Phase 6 evidence

- The schema-definition topic, inline authoring guide, and Darkmatter schema
  skill now identify only the sole-root `$schema:` and `kind: schema` +
  `types:` standalone envelopes. They state that qualifying bare maps remain
  raw JSON Schema, retain their prior validity and exit status, and emit the
  conservative `dm.schema.missing_simplified_envelope` advisory with both
  remedies.
- The Claudine completions topic and its skill snapshot now document that the
  advisory basename walk visits at most 20,000 entries, skips and budgets
  per-entry failures, retains in-budget matches, and still returns nothing for
  an unusable repository root.
- The touched implementation comments were reviewed against D1–D6. They
  already describe the corrected behavior, so no source-comment cleanup was
  needed. None of the six edited Markdown files carries a managed `hash:`
  property, so `md hash <file> --save` was not applicable and no hash metadata
  changed.
- The focused L1 requirement map passed 7 tests: the first-error, middle-error,
  exact-budget, and real Unix dangling-symlink recovery regressions; the bare
  sidecar resolution advisory; the passive shipped-schema corpus; and the
  compiled Claudine normal/silent advisory path. These are existing Phase 2–5
  tests because Phase 6 changes documentation only; it adds no runtime behavior
  requiring a new red test or persistence round trip.
- From `claudine/`, `just test` passed all 6,716 L1 tests with 11 expected
  higher-tier skips, and `just lint` passed the diagnostic/documentation guards,
  formatting checks, and Clippy checks for all five area packages. No
  pre-existing or skipped-in-scope failures were observed. No L2, L3, browser,
  formatting, staging, or commit operation was run.

## Phase 7: Full regression and cross-platform gates

**Goal:** Prove package-wide correctness, platform portability, and scoped
change impact before handoff.

- [x] From `darkmatter/`, run `just test` and `just lint`; confirm library, CLI, and DMLS L1 suites pass without invoking L2/L3 terminal or browser windows.
- [x] From `claudine/`, run `just test` and `just lint`; confirm the real dangling-symlink fixture runs on the Unix host and Claudine warning policy remains intact.
- [x] From the repository root, run `just cross-check claudine-cli --host windows` and `just cross-check claudine-cli --host linux`; verify the production collector and platform-independent budget/error tests compile for both targets.
- [x] Re-run focused AC1–AC6 tests and inspect pretty/JSON diagnostic fixtures for deterministic path ordering, five-result capping, one advisory per root document, unchanged validity, and unchanged exit codes.
- [x] Run GitNexus `detect_changes(scope: "compare", base_ref: "main")`; confirm only the expected suggestion-recovery, schema-resolution/propagation, consumer-output, test, documentation, and skill surfaces are affected, and investigate any unexpected execution flow before handoff.
- [x] Review `git diff --check` and `git status --short`; confirm no unrelated user changes were modified, no generated artifacts are stale, and no `cargo fmt` or commit was performed unless separately requested.

**Final checkpoint:** AC1–AC7 are all satisfied, both package areas pass L1 and
lint, Windows/Linux cross-checks pass, and the change-impact report matches the
planned scope.

### Phase 7 evidence

- Darkmatter `just test` passed 7,633 L1 tests with 50 expected higher-tier
  skips; `just lint` passed for `darkmatter`, `darkmatter-cli`, and `dmls`.
  Claudine `just test` passed 6,716 L1 tests with 11 expected higher-tier
  skips; `just lint` passed its diagnostic/documentation guards and all five
  package checks. The Claudine generation-drift tests passed, so no generated
  artifact is stale. No L2, L3, or browser window was invoked.
- The native Windows cross-check passed all 2,069 selected `claudine-cli` L1
  tests with 8 skips. The Linux cross-check passed all 2,431 selected L1 tests
  with 9 skips, including the Unix production dangling-symlink regression and
  compiled Claudine advisory route. The first local launcher attempt exposed
  two host-only conditions: macOS Bash 3.2 cannot parse the script's
  associative array, and parallel invocations share a temporary patch name.
  Selecting Homebrew Bash and serializing the documented commands resolved
  both without a repository change. The first Linux run then found a
  pre-existing non-writable shared Cargo cache entry; rerunning with an
  isolated target directory passed. Windows emitted one unrelated
  platform-specific unused-import warning in
  `compose_caller_file_provenance.rs`; the warning did not fail the test gate,
  and the local warning-denying lint gate was clean.
- The focused AC1–AC6 Nextest filter passed 30 tests. It covered exact basename
  matching, deterministic sorting, five-result capping, first/middle entry
  errors, the exact 20,000-visit boundary, root/no-hit behavior, directory
  symlink containment, the real Unix dangling symlink, and the original
  `./docs/unifi/protect.md` recovery state. Schema coverage included the
  positive and complete AC5 classifier corpus, deterministic root-union
  deduplication, passive shipped artifacts through their normal path, every
  validation entry point, one- and two-pass composition, transclusion merging,
  `md` pretty/JSON/quiet output, advisory-free JSON compatibility, Claudine
  normal/silent behavior, and DMLS consumer ownership plus its
  bare-to-enveloped-to-bare read/write/read cycle. No Phase 7 behavior changed,
  so no new test was added in this phase; the exact test names remain recorded
  in the Phase 2–5 evidence above.
- GitNexus `detect_changes(scope: "compare", base_ref: "main")` reported a
  critical aggregate because the long-lived feature branch already differs
  from `main` in 300 files and 1,944 indexed symbols. The follow-up
  uncommitted-worktree analysis isolated this fix to 21 files and 81 indexed
  symbols at medium risk, with one affected flow:
  `Run_compose_pipeline_internal → Index`. Its only changed step is the planned
  composition entry point; no unexpected phase execution flow was found.
- `git diff --check` and a whitespace check of the untracked plan are clean.
  Final status matches the inherited worktree plus this plan update; the
  unrelated pre-existing `claudine/fixes/2026-09-03-tts-not-finishing/`
  directory remains untouched. No `cargo fmt`, staging, or commit operation
  was performed. Phase 7 changed documentation only, and the frontmatter now
  records empty Phase 7 source/created-doc/skill lists plus the cumulative
  source and documentation inventories.
