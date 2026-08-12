---
status: ready-for-merge-commit
phase: receiving-merge
external_ledger: /private/tmp/dm-mega-merge-preflight.CdC0FB
preflight_manifest_sha256: 9c30d5f02b9fe6714b33f05b808988d7a016f668b5a1876a4efe60b0aba3affe
darkmatter_parent: 14dd391f45206d58383ba9d84adbf53c65520534
more_is_more_parent: 0584d8297f57f5eb30b52d03b1241ba55184bb44
merge_base: d672388dd0fed4196295e7f21514cac6fa59f0ae
tested_integration_commit: b6babd517fe3189d1a04ab8abeb0c07ab3be6ea0
receiving_head: 2ddef848d9b0f1b61d01df8dfeaccd01e1f2e99f
incoming_merge_commit: b6babd517fe3189d1a04ab8abeb0c07ab3be6ea0
---

# Darkmatter and More-Is-More Resolution Record

## Current closure state

This record began as a phase-by-phase integration ledger. Statements below
that describe unresolved index entries, prohibited staging, an incomplete
GitNexus refresh, or deferred gates record the state at that historical phase;
they are superseded by the closure evidence at the end of this document.

The original resolutions were staged and committed as tested two-parent merge
`b6babd517fe3189d1a04ab8abeb0c07ab3be6ea0`. That commit has a fresh GitNexus
index, and all required Level 1, Level 2, build, and lint gates passed. It is now
being received by the Darkmatter worktree at
`2ddef848d9b0f1b61d01df8dfeaccd01e1f2e99f`; the receiving merge has no
unmerged index entries and is ready for its final merge commit.

## Phase 0 summary

The three pinned commits and exact merge base were verified. The branch delta is
82 Darkmatter-only and 100 incoming-only commits. The parent
`biscuit-terminal` trees are identical. Sniff discovery, locked Cargo metadata
(72 workspace members), source-worktree status and content identities, control
artifact dispositions, process inventory, host capacity, retained evidence
identities, backup refs, and the isolated target/worktree paths are frozen in
the external ledger.

The exact incoming-parent GitNexus index is available. Refreshing a pinned
Darkmatter-parent index exceeded the non-interactive subprocess limit and was
stopped; newer-Darkmatter impact evidence is labeled approximate and exact
pinned impact remains mandatory before Phase 2 edits. Recorded warnings are
CRITICAL for `effective_for_with_override` and HIGH for
`collect_problems` and `env_disables_baseline_schema`.

## Phase 1 merge inventory

- HEAD: `14dd391f45206d58383ba9d84adbf53c65520534`
- MERGE_HEAD: `0584d8297f57f5eb30b52d03b1241ba55184bb44`
- Actual conflicts: six, exactly matching the reviewed preview.
- Unexpected shared or unmerged paths: none.
- Conflict content edited: no.
- Raw status, unmerged names, and stage entries:
  `/private/tmp/dm-mega-merge-preflight.CdC0FB/phase1-merge-inventory.txt`

## Shared-path records

### P01 — `.claude/skills/darkmatter/SKILL.md`

- Type: content conflict.
- Parent contributions: Darkmatter guidance plus incoming Git/context,
  provider, literal, and semantic-schema guidance.
- Governing requirement: R6.
- Authority boundary: Darkmatter behavior documentation and generated hash.
- Resolution status: Phase 4 working-tree resolution complete. The marker-free
  body retains Darkmatter cleanup/reference guidance and incoming Git context,
  literal, provider, and semantic meta-schema guidance.
- Evidence/follow-up: the merged frontmatter was refreshed and verified with
  Markdown-aware body hash `87f17662fa397abe-c0eb7c8a0924fdd4`. The resolved
  file is staged in tested integration commit `b6babd517`.

### P02 — `.claudine/memory/commits.md`

- Type: content conflict.
- Parent contributions: signing/pinentry safety, hook safety, and incoming
  `--only` plus `-F -` argument ordering.
- Governing requirement: R6.
- Authority boundary: repository commit policy.
- Resolution status: Phase 4 working-tree resolution complete.
- Evidence/follow-up: the marker-free union preserves non-interactive
  signing/pinentry safety, the prohibition on hook bypass, and the existing
  `git commit -F - --only -- <paths>` ordering contract. The resolved file is
  staged in tested integration commit `b6babd517`.

### P03 — `CLAUDE.md`

- Type: content conflict.
- Parent contributions: non-conflicting repository guidance with independently
  generated GitNexus counts.
- Governing requirement: R6 and R9.
- Authority boundary: repository guidance and generated GitNexus metadata.
- Resolution status: Phase 4 working-tree resolution complete using the exact
  Darkmatter-parent count line: 136,293 symbols, 270,769 relationships, and 300
  execution flows.
- Evidence/follow-up: those values were temporary historical placeholders.
  Tested integration commit `b6babd517` was subsequently indexed with 138,356
  symbols, 276,534 relationships, and 300 execution flows. The receiving merge
  retains its generated placeholder until the mandatory post-commit reindex.

### P04 — `darkmatter/cli/tests/level2_code_block_styling.rs`

- Type: content conflict.
- Parent contributions: centralized Level 2 helper versus incoming unique
  build-shim/terminal-discovery coverage.
- Governing requirement: R6.
- Authority boundary: shared real-terminal harness in
  `tests/common/level2.rs`.
- Resolution status: Phase 3 working-tree resolution complete. The resolved
  bytes equal the Darkmatter parent blob `612f893b6ee2a372726adb3f9439525439cce4f4`.
- Evidence/follow-up: A3-01 through A3-05 and T3-01. The incoming file-local
  tmux harness, sentinel loop, fixture writer, and `run_md_in_tmux` copy were
  rejected because the shared helper already contains the build shim and
  terminal-specific tmux path. The file retains
  `#[serial(level2_terminal)]`. The owning `just test-l2` gate later passed,
  and the resolved file is staged in tested integration commit `b6babd517`.

### P05 — `darkmatter/cli/tests/level2_errors.rs`

- Type: content conflict.
- Parent contributions: shared helper/build shim and import changes.
- Governing requirement: R6.
- Authority boundary: Darkmatter CLI Level 2 test support.
- Resolution status: Phase 3 working-tree resolution complete. The resolved
  bytes equal the Darkmatter parent blob `c532cd4ef1fe5547e0686571502e21491a53ea1f`.
- Evidence/follow-up: A3-01 through A3-05 and T3-01. Exactly one
  `use common::level2::md_shim;` remains, ordered before external-crate imports,
  and every invocation therefore uses the Cargo-built binary shim. The owning
  `just test-l2` gate later passed, and the resolved file is staged in tested
  integration commit `b6babd517`.

### P06 — `darkmatter/features/2026-07-15-performance-followup/review-8.md`

- Type: modify/delete conflict; stage 2 retains the Darkmatter version and the
  incoming parent deletes it.
- Governing requirement: R6 and R9.
- Authority boundary: historical performance evidence.
- Resolution status: Phase 4 working-tree retention and chain repair complete.
- Evidence/follow-up: Review 8 is retained; Review 7 now points to Review 8;
  Review 8 points to Review 9; Review 9 points to Review 10; and Review 10's
  predecessor is the canonical `review-9.md`. All four remain `ready: false`
  and retain the open quiet-host evidence requirement. The resolution is
  staged in tested integration commit `b6babd517`.

### P07 — `darkmatter/lib/Cargo.toml`

- Type: clean auto-merge requiring semantic audit.
- Parent contributions: Darkmatter benchmark/dev dependency changes plus
  incoming Sniff remote and merge-prediction dependencies.
- Governing requirement: R7.
- Authority boundary: package dependency and benchmark declarations.
- Resolution status: Phase 2 audited; the clean auto-merge needs no corrective
  edit.
- Evidence/follow-up: A2-01, A2-02, T2-01, and G2-01 confirm Sniff `remote`,
  dev-only `git2`, `clean_hot_paths`, one production Git authority, focused
  parity, locked metadata, and restored absent-lock repository state.

### P08 — `darkmatter/lib/src/markdown/schemas/mod.rs`

- Type: clean auto-merge requiring semantic audit.
- Parent contributions: clean-analysis/override/raw-validation seams and
  incoming bounded references/source-aware exports.
- Governing requirement: R7 and R8.
- Authority boundary: Darkmatter schema facade and semantic authority.
- Resolution status: Phase 2 audited; the clean auto-merge needs no corrective
  edit.
- GitNexus: `effective_for_with_override` is CRITICAL in the newer
  Darkmatter index; exact pinned evidence is required before editing.
- Evidence/follow-up: A2-03, T2-02, T2-03, and T2-04 cover override,
  raw/coercing validation, deterministic ordering, shipped reference artifacts,
  bounded cycles/depth, persistence, and passive DMLS source projection.

### P09 — `darkmatter/lib/src/markdown/schemas/validate.rs`

- Type: clean auto-merge requiring semantic audit.
- Parent contributions: schema-clean helper visibility plus incoming nominal
  validator registrations.
- Governing requirement: R7 and R8.
- Authority boundary: shared Darkmatter validator construction.
- Resolution status: Phase 2 audited; the clean auto-merge needs no corrective
  edit.
- GitNexus: `collect_problems` is HIGH with 27 direct schema callers.
- Evidence/follow-up: A2-04 and T2-02 confirm helper visibility, URL-scheme,
  `type-definition`, and `schema` registrations, plus distinct raw/coercing
  validation results.

### P10 — `darkmatter/cli/src/commands/compose.rs`

- Type: clean auto-merge requiring semantic audit.
- Parent contributions: baseline-disable sharing/removal of obsolete bindings
  plus focused provider/approval error classification.
- Governing requirement: R7 and R8.
- Authority boundary: Darkmatter CLI composition orchestration.
- Resolution status: Phase 2 audited; the clean auto-merge needs no corrective
  edit.
- GitNexus: `env_disables_baseline_schema` is HIGH and participates in
  `run_subcommand` and `repair_frontmatter` flows.
- Evidence/follow-up: A2-05, T2-02, and T2-05 confirm clean/compose baseline
  parity, removal of obsolete bindings, focused provider failures, output
  stability, and repeated save/read behavior.

## Phase 2 production authority audit

No production source correction was necessary. The four auto-merged paths
preserve the intended union, so their merge-staged content was left untouched.
The operator did not run `git add`; the explicit task instruction forbids
staging, and no Phase 2 corrective path existed in any case.

### Static and impact evidence

- **A2-01 — manifest authority:** `darkmatter/lib/Cargo.toml` enables Sniff's
  `remote` feature, keeps `git2` under dev-dependencies, and retains the
  `clean_hot_paths` benchmark. Production Darkmatter Git calls resolve through
  `sniff::filesystem::git`; no second production implementation was found.
- **A2-02 — dependency graph:** locked metadata reports 72 workspace members.
  The repository intentionally has no tracked `Cargo.lock`; Cargo generated an
  ignored lock with SHA-256
  `52c2a58dc23331afe1cd82424ea5b295123ed4c1f40275fd35a400709d9ef286`
  during execution, metadata preserved that identity, and the generated file
  was removed to restore the original absent-lock state.
- **A2-03 — schema facade:** the extracted test-module layout, clean and
  override exports, raw-validation entry point, deterministic problem ordering,
  bounded reference graph, source parser/cursor/span/value products, and DMLS
  consumers remain present. The removed inline god-test module was not restored.
- **A2-04 — validator:** clean-analysis helpers remain `pub(super)` and shared;
  URL scheme, `type-definition`, and `schema` keyword validators are registered;
  raw and coercing validation remain separate public observations.
- **A2-05 — compose CLI:** clean and compose share
  `env_disables_baseline_schema`; obsolete approval bindings remain removed;
  focused provider and approval errors retain their classifications and
  terminal rendering.
- **A2-06 — propagation:** context Git values are demand-driven from one
  discovery with independent degradation; literals/indexing/runtime policy
  propagate through frontmatter, body, shell ternaries, and nested subtrees;
  schema authority is singular; DMLS remains catalog-driven and passive; Sniff
  predictions remain caller-anchored, direction-correct, and read-only; and
  Claudine traverses container values without treating object keys as values.
- **A2-07 — impact:** exact incoming-index UIDs were used for
  `SchemaReference` and schema source products. The available newer-Darkmatter
  index reports CRITICAL impact for `effective_for_with_override` (97 upstream
  symbols), HIGH for `collect_problems` (27 direct schema callers), HIGH for
  `env_disables_baseline_schema` (six upstream symbols in `run_subcommand` and
  `repair_frontmatter`), and MEDIUM for raw validation and clean analysis.
  Because no corrective symbol edit was required, these risk warnings did not
  trigger a production change.
- **A2-08 — marker scan:** 100 changed production Rust/manifest paths were
  scanned for exact conflict-marker triples with no unresolved marker. Long
  equals-only lines in the Windows `route print` fixture were investigated and
  confirmed to be fixture content rather than Git conflict markers.

### Requirement-to-test mapping

- **T2-01 — Sniff Git/remote authority:** five default-feature Git tests and
  three `remote`-feature tests passed, covering the `git2` conflict oracle,
  directionality, read-only clean and criss-cross predictions, preferred remote
  ordering, live observation without local mutation, host/provider/credential
  binding, and distinct denial/validation/provider failures.
- **T2-02 — schema/compose regression set:** 22 focused Darkmatter tests passed,
  covering the exact raw-versus-coercing regressions, schema override, stable
  diagnostic ordering, every shipped schema classification, a real shipped
  feature-review reference through normal validation, source-aware boundary,
  padded references, cycle/depth errors, all three Git context values and
  degradation, literals/indexes, indexed-file variants, frontmatter/body/shell
  provider parity, policy denial, and focused fatal errors.
- **T2-03 — passive shipped-artifact coverage:** DMLS
  `dsl_requests_spawn_no_processes_and_open_no_sockets` passed through the real
  test binary, while the shipped-schema corpus and feature-review artifact tests
  in T2-02 passed through normal schema invocation paths.
- **T2-04 — persistence:**
  `semantic_arrays_disambiguate_unions_and_survive_two_disk_round_trips` passed,
  exercising two write/read cycles with the semantic array representation.
- **T2-05 — CLI behavior:** six focused clean/compose CLI tests passed for
  absent/empty frontmatter, default versus disabled baseline, explicit override,
  schema replacement, and save repair. Two additional tests passed for repeated
  save/read stability and idempotent output.
- **T2-06 — Claudine downstream:** eight focused tests passed for nested array
  and object traversal, object-key exclusion, undefined-variable validation,
  early/late binding, and semantic-schema classification.

### Broader gates

- **G2-01:** locked metadata succeeded and returned 72 workspace members.
- **G2-02:** `just test` passed 2,642 of 6,084 started Darkmatter library tests
  before the non-interactive 60-second subprocess limit required interruption;
  no failure occurred before interruption. The remaining broad area run is
  deferred to the later validation phase.
- **G2-03:** `just lint` completed the Darkmatter library successfully. The
  Darkmatter CLI all-target lint reached the intentionally unresolved
  `level2_errors.rs` Phase 3 conflict and failed on that marker. No Phase 2
  production lint finding was reported. A follow-up production-binary-only
  Darkmatter CLI Clippy run passed with warnings denied.

## Phase 3 test-conflict and harness audit

Phase 3 changes no product behavior. It resolves two test-only merge conflicts
while preserving the observable Level 2 contracts already represented by the
two parents. No new test was required: the incoming behavior is either already
covered by the shared helper and its integrity tests or is the obsolete local
implementation that R6 explicitly prohibits restoring.

### Static and impact evidence

- **A3-01 — parent resolution:** the working content of
  `level2_code_block_styling.rs` hashes to the stage-2 Darkmatter blob
  `612f893b6ee2a372726adb3f9439525439cce4f4`; `level2_errors.rs` hashes to
  the stage-2 Darkmatter blob `c532cd4ef1fe5547e0686571502e21491a53ea1f`.
  The incoming file-local harness and duplicate import are absent.
- **A3-02 — centralized helper:** within the conflicted code-block test and
  `tests/common/level2.rs`, `SHARED_TMUX_HARNESS`,
  `TMUX_SENTINEL_COUNTER`, `wait_for_tmux_sentinel`, `run_tmux_command`, and
  `run_md_in_tmux` each have one definition, all in the common helper. The
  test imports `run_md_in_tmux` instead of defining a second harness.
- **A3-03 — build shim:** `level2_errors.rs` contains one canonically ordered
  `md_shim` import and all three compose command builders call it. The shared
  helper's `MD_BIN`, `link_or_copy`, and identity check retain macOS, Linux,
  and Windows behavior.
- **A3-04 — resource policy:** every affected real-terminal test remains
  `#[serial(level2_terminal)]`. `darkmatter/justfile` routes all three area
  packages through `_test_l2`; the shared recipe defaults
  `BISCUIT_L2_THREADS` to `1`, brokers one shared pane per backend, and runs
  Nextest serially.
- **A3-05 — authority/impact:** the two resolved files contain no YAML parser,
  schema validator, formatter, Git implementation, terminal detector, HTTP
  client, or remote executor. GitNexus reports LOW risk, zero upstream symbols,
  and zero affected flows for both `md_shim` and `run_md_in_tmux`.

### Phase 5 seam map

The cases below are the pre-execution map required by Phase 3. Phase 5 will
turn these entries into exact Nextest selectors and record results; Level 2
cases must be invoked only through the owning area recipe.

| Phase 5 seam | Existing binaries and concrete cases |
|---|---|
| Schema/meta-schema | `meta_schema_phase1` through `meta_schema_phase6`, `meta_schema_reference_graph`, `meta_schema_repo_schemas`, and `schemas_source_projection`; exact requirements/cases are enumerated in the retained `phase1-test-matrix.md` and `phase2-test-map.md` through `phase7-test-map.md`. DMLS consumers are `lsp_session::{meta_schema_phase1_schema_hover_uses_nominal_type,meta_schema_phase6_shipped_schema_activation_and_current_error,meta_schema_phase7_shipped_schema_provider_path}`. |
| Invalid frontmatter | `biscuit-file` binaries `yaml_corpus`, `yaml_safety`, `yaml_mutation`, and `parse_count`, including `yaml_test_suite_subset_is_release_pinned_and_preserved`, `report_only_and_clean_inputs_stay_byte_identical`, mutation invariants, and zero/candidate parse counts. Darkmatter CLI binaries `clean_frontmatter`, `clean_json`, and `clean_schema` cover exact delimiters, BOM/CRLF/lone-CR/UTF-8 spans, JSON envelopes, fenced-body isolation, schema flags, save, and repeated read/write behavior. |
| Compose/provider | `more_is_more_literals_and_indexes`, `git_context_integration`, `markdown::compose::tests::provider_network`, `frontmatter_shell_expansion`, and transclusion tests cover literal/index variants, demand-driven Git values, nested/frontmatter/body/`$()` propagation, fatal focused-provider errors, and deny-by-default exact-host policy. |
| Sniff Git/remote/credentials | `git_parity`, `merge_conflict_prediction`, `remote_observation`, `remote_resolution`, and `focused_provider` cover the git2 oracle, directionality, before/after snapshots, preferred remotes, query bounds/order, credential isolation, provider authentication, and loopback-only transport fixtures. |
| Reference trust | `markdown::reference::validate::{fresh_seam_uses_snapshot_while_checked_path_rejects_stale_graph,fresh_seam_uses_heading_snapshot_while_checked_path_rejects_stale_headings}`, `markdown::reference::file_tree::ensure_built_is_idempotent`, and `reference_integration::{file_tree_validate_reuses_graph_without_spurious_mismatch,unchanged_child_via_multiple_insertions_passes}` cover fresh, stale, mismatch, changed-child, file-tree, and heading-snapshot seams. |
| Cleanup/formatting | `markdown::cleanup::tests::{reflow,parse_count}`, CLI binaries `clean`, `clean_frontmatter`, `clean_json`, `clean_schema`, and `lsp_session::formatting_is_byte_equivalent_to_library_cleanup` cover default/preserve/fixed width, Unicode/hanging prefixes, opaque boundaries, idempotency, stdout/save, and DMLS parity. |
| DMLS passive behavior | `no_side_effects::dsl_requests_spawn_no_processes_and_open_no_sockets`, `level1_graph_index`, and `lsp_session` cases for completion, hover, diagnostics, document links, graph updates, trigger recovery, and last-good schema state. |
| Claudine downstream | `composition::lifecycle::{tests,executor::tests}`, `composition::schema::classify::semantic_type_tests`, `composition::validate` tests, and `claudine-cli::commands::context::format::tests::semantic_schema_types_render_as_structural_types` cover container traversal, nominal classification, lifecycle validation, rendering, and CLI formatting. |
| Performance/compatibility mechanisms | `hash_directory::{test_hash_directory_includes_vendored_dirs,test_hash_directory_vendored_membership_matches_plain_dir}`, `compose_datetime_capture_never_probes_ntp`, `output::terminal::test_image_renderer_caches_detection`, shell redirection-order/timeout tests, `reference_integration::{graph_ownership_does_not_extend_shell_handler_lifetime,graph_ownership_does_not_extend_preflight_graph_lifetime,nested_recursion,cycle_detection_stops_infinite_recursion}`, cleanup parse-count tests, and Sniff manifest-cache/nested-marker guards retain mechanisms without claiming new benchmark evidence. |
| Terminal boundary | L1 `level2_harness_integrity::{md_shim_resolves_to_cargo_built_binary,assert_shim_resolves_to_built_accepts_valid_link,assert_shim_resolves_to_built_rejects_foreign_link,md_shim_path_is_absolute_temp_dir_link}` plus every case in `level2_code_block_styling` and `level2_errors`. Phase 5 command: `cd darkmatter && just test-l2`; no raw Nextest Level 2 selector is authorized. |

### Focused commands

- **T3-01 — L1 helper integrity:**
  `cargo nextest run --offline -p darkmatter-cli --test
  level2_harness_integrity --color never` passed all four tests. The initial
  `--locked` form correctly failed because this repository intentionally has
  no tracked lockfile; the offline retry used the generated ignored lockfile.
- **T3-02 — real-terminal seam, deferred to Phase 5:**
  `just test-l2` from the Darkmatter package area. The run must record backend,
  pass/skip counts, and skip reasons.
- **T3-03 — broad Phase 3 gates:** `just test` and `just lint` from the
  Darkmatter package area after conflict-marker removal. Because the operator
  forbids staging, tools that compile from the working tree may run, but Git
  will continue to report the two paths as unmerged until the separate staging
  operation.

### Phase 3 gate results

- **G3-01 — targeted regression:** all four
  `level2_harness_integrity` tests passed: built-binary resolution, valid-link
  acceptance, foreign-link rejection, and absolute temporary shim path.
- **G3-02 — area lint:** `just lint` passed for `darkmatter`,
  `darkmatter-cli`, and `dmls` after a cold-cache attempt was stopped before
  the non-interactive subprocess limit and retried from the populated cache.
- **G3-03 — area tests:** the final warm-cache `just test` attempt passed
  1,956 of 6,084 started Darkmatter tests, with 140 configured skips and no
  failure before interruption. The run was stopped before the non-interactive
  60-second subprocess ceiling; 4,128 Darkmatter tests and the subsequent area
  package runs were therefore not executed at this phase. Later complete
  Phase 4 and Phase 6 gates superseded this incomplete attempt.
- **G3-04 — Level 2:** intentionally deferred to Phase 5 and must be invoked
  through `cd darkmatter && just test-l2`, as required by the shared broker and
  serialization policy.
- **G3-05 — change scope:** GitNexus `detect_changes(scope = all)` reports HIGH
  aggregate merge risk across 837 changed symbols, 198 files, and 12 affected
  processes. That result describes the entire active mega-merge, not the two
  Phase 3 test resolutions; the earlier symbol-specific impact checks remain
  LOW with no upstream callers or affected processes. Final marker scans and
  `git diff --check` passed for every Phase 3 path.

## Phase 4 documentation and policy resolution

Phase 4 changes no runtime behavior or implementation source. Its observable
contracts are the resolved policy text, review-chain reachability, retained
historical evidence, and unchanged support-file unions. No new Rust test is
appropriate for those documentation-only changes.

### Requirement-to-test map

| Requirement | Concrete verification |
|---|---|
| Skill union and deferred generated hash | Parent-delta audit plus marker scan; the Darkmatter-parent hash was recorded as temporary and assigned to Phase 7 for `md hash --save` / `--diff`. |
| Commit safety union | Static assertions retain non-interactive signing/pinentry safety, hook-bypass prohibition, and `git commit -F - --only -- <paths>` ordering. |
| Review 7 → 8 → 9 → 10 and open performance gate | Frontmatter link walk verifies each referenced file exists and each reciprocal edge is canonical; all four reviews remain `ready: false` and retain the quiet-host requirement. |
| Shipped schema/support union | Existing passive corpus tests `shipped_schema_artifacts_validate_through_semantic_keywords` and `repo_root_schemas_all_classify_as_standalone_schemas`; existing normal-path tests `feature_review_resolves_as_a_bare_name_reference` and `feature_review_reference_validates_a_review_document`. The retained `semantic_arrays_disambiguate_unions_and_survive_two_disk_round_trips` case supplies repeated persisted round-trip coverage. |
| Cross-platform workflow policy | Static workflow audit confirms the Sniff matrix still names macOS, Ubuntu, and Windows and the Claudine addition is a distinct macOS/Windows compile-check job rather than a duplicate test job. |
| Historical evidence preservation | Tree comparison against the Darkmatter parent: the invalid-frontmatter subtree has no delta; performance fixtures, raw samples, manifest, and `performance-compliance.md` have no delta; only the intentional Review 7/10 link repairs differ. |

### Support-file audit

- Workflow, testing-strategy, review-schema, prompt, Rust-devops/Sniff skill,
  Darkmatter/DMLS public-doc, and Sniff public-doc unions are byte-identical to
  the incoming parent. No OS leg, hook/signing rule, or validation requirement
  was weakened.
- `feature-review.yaml` and `suggestion-review.yaml` retain the incoming pure
  `$schema` envelopes. Their passive corpus and real shipped-artifact tests are
  already in the merged tree and remain deferred to Phase 5 execution.
- The source-worktree-local `CLAUDE.md` count refresh was not imported. The
  resolved file carries the exact Darkmatter-parent count line until the final
  Phase 7 GitNexus refresh.
- The authorized plan remains in the source worktree for phase-by-phase
  recovery. Every other reviewed dirty control artifact remains external-only.

### Phase 4 path disposition

The following working-tree paths are marker-free and ready for the operator's
separate staging step:

- `.claude/skills/darkmatter/SKILL.md`
- `.claudine/memory/commits.md`
- `CLAUDE.md`
- `darkmatter/features/2026-07-15-performance-followup/review-7.md`
- `darkmatter/features/2026-07-15-performance-followup/review-8.md`
- `darkmatter/features/2026-07-15-performance-followup/review-10.md`

The Darkmatter skill hash and `CLAUDE.md` GitNexus counts remain explicitly
open until Phase 7. Per the operator's no-staging instruction, Git continues to
report the four Phase 4 content/modify-delete index entries plus the two Phase
3 test entries as unmerged even though their working files are resolved.

### Phase 4 verification

- **T4-01 — shipped schema artifacts:** five targeted Darkmatter tests passed:
  `shipped_schema_artifacts_validate_through_semantic_keywords`,
  `repo_root_schemas_all_classify_as_standalone_schemas`,
  `feature_review_resolves_as_a_bare_name_reference`,
  `feature_review_reference_validates_a_review_document`, and
  `semantic_arrays_disambiguate_unions_and_survive_two_disk_round_trips`.
  Together they cover the passive shipped corpus, normal resolution and
  validation of a real shipped review schema, and two persisted write/read
  cycles.
- **G4-01 — area tests:** the unpartitioned `just test` invocation reached
  2,596 of 6,084 Darkmatter library tests with no failure before it was stopped
  at the non-interactive subprocess ceiling. The same owning recipe was then
  run in four deterministic count partitions. Collectively, the partitions
  passed all 6,084 Darkmatter library tests, all 643 selected Darkmatter CLI
  tests, and all 640 selected DMLS tests. Partition counts were
  `1546/176/164`, `1529/168/161`, `1513/153/159`, and `1496/146/156` for
  Darkmatter/Darkmatter CLI/DMLS respectively. The partitioned runs supersede
  the interrupted attempt and omit no selected Level 1 test.
- **G4-02 — area lint:** `just lint` passed for `darkmatter`,
  `darkmatter-cli`, and `dmls`.
- **G4-03 — repository hygiene:** locked Cargo metadata passed. The repository
  still has no tracked `Cargo.lock`, and Cargo introduced no tracked lockfile
  delta. `git diff --check` passed.
- **G4-04 — marker and index audit:** 199 changed text files were scanned. The
  only marker-shaped match is the investigated equals-only `route print`
  fixture in `sniff/lib/src/network/mod.rs`; no `<<<<<<<` or `>>>>>>>` marker
  remains. At this phase, `git ls-files -u` intentionally listed the four Phase
  4 and two Phase 3 entries because the operator then forbade staging; their
  working content was marker-free and prepared for the later staging step.
- **G4-05 — change scope:** GitNexus `detect_changes(scope = all)` reports HIGH
  aggregate merge risk across 835 changed symbols, 198 files, and 12 affected
  processes. This is the expected whole-merge result. Phase 4 itself changes
  documentation and policy only and introduces no changed implementation
  symbol or additional execution-flow risk.

## Unexpected paths

### P11 — `darkmatter/cli/tests/level2_schema_about.rs`

- Type: newly discovered one-sided semantic test overlap.
- Parent contributions: the Darkmatter parent set the legacy `DARK_MODE`
  fixture variable; the incoming parent retained `COLORFGBG`, which is the
  current terminal detector's supported non-interactive color-mode input.
- Governing requirement: R3 and R6.
- Authority boundary: real-terminal schema-about test fixture only; production
  theme selection is unchanged.
- GitNexus: exact upstream impact is LOW. `run_with_sentinel` has one direct
  helper caller and three downstream test callers; `capture_schema_about` has
  the same three direct test callers. No production process is affected.
- Resolution status: Phase 5 corrective test edit restores `COLORFGBG=15;0`
  for dark-terminal fixtures and `COLORFGBG=0;15` for light-terminal fixtures.
- Evidence/follow-up: the original normal-path failure was
  `level2_schema_about_light_terminal_uses_dark_code_theme`, which failed all
  four configured attempts because the YAML example inherited the light page
  background instead of OneHalf Dark. F5-10 reruns the complete owning area
  recipe after this correction.

## Phase 5 focused convergence evidence

### Pre-execution test manifest

Phase 5 changes no product behavior. The merge seams already have targeted
regressions from their owning features; this phase runs those tests against the
combined tree and observes their public outputs and persisted downstream state.
All commands run from the integration repository root unless an area directory
is named explicitly, with `CARGO_TARGET_DIR=/private/tmp/dm-mega-merge-target.EhU7p8`,
`CARGO_BUILD_JOBS=8`, `NEXTEST_TEST_THREADS=8`, `BISCUIT_L2_THREADS=1`, and
`--offline` for focused Nextest commands. `P04` and `P05` Level 2 coverage is
invoked only through the Darkmatter area recipe.

| Evidence | Package(s) | Tier | Exact selector or recipe | Observable invariant |
|---|---|---|---|---|
| F5-01 | `darkmatter`, `dmls` | L1 | `package(darkmatter) & binary(/^meta_schema_(phase1|phase3|phase4|phase5|phase6|reference_graph|repo_schemas)$/)`, `package(darkmatter) & binary(schemas_source_projection)`, and exact `dmls::lsp_session` cases `meta_schema_phase1_schema_hover_uses_nominal_type`, `meta_schema_phase6_shipped_schema_activation_and_current_error`, `meta_schema_phase7_shipped_schema_provider_path` | Meta-schema exports, nominal validators, raw/coercing separation, references/cycles/depth, source projection, shipped artifacts, and passive DMLS consumption all survive the merge. |
| F5-02 | `biscuit-file`, `darkmatter-cli` | L1 | exact binaries `yaml_corpus`, `yaml_safety`, `yaml_mutation`, `parse_count`, `clean_frontmatter`, `clean_json`, and `clean_schema` | Original malformed YAML corpus inputs, representation/line-ending/BOM/UTF-8 variants, report-only and repair behavior, JSON envelopes, trigger isolation, parse counts, and repeated save/read behavior retain their public bytes and values. |
| F5-03 | `darkmatter` | L1 | exact binaries `more_is_more_literals_and_indexes`, `git_context_integration`; library test paths under `markdown::compose::tests::provider_network`, `markdown::compose::frontmatter_shell_expansion`, and nested transclusion runtime-policy cases | Literal/index variants, indexed-file fallbacks, demand-driven Git context, nested/frontmatter/body/`$()` propagation, fatal provider errors, and deny-by-default exact-host policy remain additive. |
| F5-04 | `sniff` | L1 | exact binaries `git_parity`, `merge_conflict_prediction`, `remote_observation`, `remote_resolution`, and `focused_provider`, with feature `remote` | Conflict prediction matches the git2 oracle without mutation; remote/provider selection, bounds, ordering, credentials, and loopback-only policy remain isolated and deterministic. |
| F5-05 | `darkmatter` | L1 | exact library cases `fresh_seam_uses_snapshot_while_checked_path_rejects_stale_graph`, `fresh_seam_uses_heading_snapshot_while_checked_path_rejects_stale_headings`, `ensure_built_is_idempotent`; exact `reference_integration` cases `file_tree_validate_reuses_graph_without_spurious_mismatch` and `unchanged_child_via_multiple_insertions_passes` | Trusted-fresh graph and heading snapshots skip redundant work while checked/stale/mismatched/changed-child paths still reject invalid reuse. |
| F5-06 | `darkmatter`, `darkmatter-cli`, `dmls` | L1 | library paths under `markdown::cleanup::tests::reflow` and `markdown::cleanup::tests::parse_count`; exact CLI binaries `clean`, `clean_frontmatter`, `clean_json`, `clean_schema`; exact DMLS case `formatting_is_byte_equivalent_to_library_cleanup` | Default/preserve/fixed-width cleanup, Unicode width, hanging prefixes, opaque regions, idempotency, stdout/save, and DMLS formatting remain byte-equivalent. |
| F5-07 | `dmls` | L1 | exact binaries `no_side_effects`, `level1_graph_index`, and `lsp_session` | Passive requests spawn no process/open no socket, and completion, hover, diagnostics, links, graph updates, LSP sessions, and last-good recovery remain observable through the normal protocol path. |
| F5-08 | `claudine`, `claudine-cli` | L1 | library paths under `composition::lifecycle`, `composition::schema::classify`, and `composition::validate`; exact CLI case `commands::context::format::tests::semantic_schema_types_render_as_structural_types` | Container traversal, nominal-schema classification/validation, lifecycle state, rendering, and CLI formatting remain coherent downstream. |
| F5-09 | `darkmatter`, `darkmatter-cli`, `sniff` | L1 | exact `hash_directory` binary; exact tests for no-NTP capture, terminal-query caching, shell redirection/timeouts, graph ownership, recursion/cycle limits, cleanup parse counts, and Sniff manifest-cache/nested-marker guards | Retained performance/compatibility mechanisms execute without making new benchmark claims. |
| F5-10 | Darkmatter area (`darkmatter`, `darkmatter-cli`, `dmls`) | L2 | `cd darkmatter && just test-l2` | The centralized helper/build shim and single terminal discovery preserve stable code-block/error rendering; backend skips remain explicit. |
| F5-11 | repository discovery | Read-only | `sniff repo packages --json`, `sniff repo package-areas --json`, `sniff repo package-dependencies --json`, plus GitNexus change/impact reconciliation | Phase 6 package, area, recipe, and conditional scope is frozen from the merged tree rather than inferred from directories. |

### Focused results

- **F5-01 — schema/meta-schema:** 53 of 53 selected tests passed across nine
  binaries; 83 non-selected tests were skipped by the exact filter. This
  includes the shipped schema corpus, nominal validators, native/quoted/source
  representation variants, recursion boundaries, reference cycles, two
  persisted semantic-array write/read round trips, and the three normal-path
  DMLS schema consumers.
- **F5-02 — invalid frontmatter:** 98 of 98 selected tests passed across seven
  binaries with no skips. The run includes the pinned YAML Test Suite subset,
  mutation/property guards, zero/candidate parse counts, exact BOM/CRLF/lone-CR
  and UTF-8 coordinate behavior, byte-preserving report-only paths, repair and
  error envelopes, trigger isolation, idempotency, and repeated save/read
  behavior.
- **F5-03 — compose/provider:** 53 of 53 selected tests passed across three
  binaries; three were classified slow and none failed. The results cover the
  exact literal/index regressions, every Git context state and independent
  degradation, frontmatter/body/`$()` provider parity, focused fatal failures,
  exact-host denial before contact, and nested/interpolated remote transclusion
  runtime propagation.
- **F5-04 — Sniff Git/remote:** 170 of 170 selected tests passed across five
  binaries with no skips. The suite includes git2 parity, caller-direction
  conflict prediction, before/after read-only repository-state checks,
  preferred remote resolution, complete-domain ordering before truncation,
  provider/flavor bounds, host-bound credential isolation, and loopback-only
  transport fixtures.
- **F5-05 — reference trust:** 23 of 23 selected tests passed across the
  library and `reference_integration` binaries. Fresh graph/heading snapshots
  and idempotent `FileTree` reuse passed alongside the negative matrix for
  stale roots, edited/missing/unreadable children, option/source mismatches,
  volatile targets, recreated runtime ownership, and transclusion-only graphs.
- **F5-06 — cleanup/formatting:** 196 of 196 selected tests passed across six
  binaries. The matrix covers preserve/default/fixed-width modes, wide Unicode
  and nested hanging prefixes, hard breaks, fences, code, tables, HTML,
  Darkmatter directives and shell bodies, parse counts, idempotency, CLI
  stdout/save/repeated reads, frontmatter repair composition, and byte-equivalent
  DMLS formatting.
- **F5-07 — DMLS passive behavior:** 91 of 91 selected tests passed across
  `no_side_effects`, `level1_graph_index`, and the complete `lsp_session`
  binary. Completion, hover, diagnostics, links, graph invalidation, watcher
  and rescan paths, malformed-edit last-good recovery, protocol lifecycle,
  rename/format actions, and the process/socket side-effect guard all passed.
- **F5-08 — Claudine downstream:** 14 of 14 exact tests passed across the
  Claudine library and CLI binary. The selected matrix traverses nested array
  and object values, excludes object keys, distinguishes early/late binding,
  rejects undefined container values, preserves the `doc.err` escape hatch,
  classifies nominal semantic types structurally, and renders those types
  through the CLI formatter.
- **F5-09 — retained mechanism guards:** 56 of 56 selected tests passed across
  five binaries. Directory-hash membership, no-NTP capture, cached terminal
  discovery, shell redirection ordering/timeouts, graph ownership and
  recursion/cycle limits, cleanup parse counts, manifest single-read caching,
  and nested-marker traversal guards all passed. This is mechanism evidence,
  not a new performance measurement or claim.
- **F5-10 — Darkmatter Level 2:** the first owning-recipe run passed all 18
  Darkmatter tests and 67 of 69 executed Darkmatter CLI tests, then
  `level2_schema_about_light_terminal_uses_dark_code_theme` failed all four
  configured attempts and canceled one later case. P11 records the LOW-risk,
  test-only fixture correction. The complete `just test-l2` rerun brokered
  WezTerm, tmux, and Apple Terminal resources and passed 18 Darkmatter, 69
  Darkmatter CLI, and 3 DMLS tests (90 total) with zero selected skips. The
  centralized helper/build shim, code-block/error rendering, both explicit
  color-mode fixtures, and live Neovim semantic-token repaint paths passed.
- **F5-11 — frozen Phase 6 scope:** merged-tree Sniff discovery reports 74
  package records and 32 package areas; locked Cargo metadata remains the
  workspace authority at 72 members. Dependency projection and the HIGH
  aggregate GitNexus result (840 changed symbols, 199 files, 12 affected
  processes) retain the four minimum gate areas: `biscuit-file` (`biscuit-file`,
  `biscuit-file-cli`), `sniff` (`sniff`, `sniff-cli`), `darkmatter`
  (`darkmatter`, `darkmatter-cli`, `dmls`, with DMLS covered by the merged area
  recipe), and `claudine` (all members enumerated by its merged area recipe).
  The parent `biscuit-terminal` subtrees are byte-identical, no Phase 5 edit
  touched that area, and its terminal consumers passed F5-10, so its conditional
  Phase 6 gate is not activated. The only Phase 5 source correction is P11's
  LOW-risk Darkmatter CLI test fixture; its complete owning Level 2 recipe
  passed after the change.

### Required area gates after the Phase 5 correction

- **G5-01 — complete Level 1 suite:** the Darkmatter area's `just test` recipe
  passed in four deterministic count partitions, using the frozen isolated
  target directory and offline dependency resolution. Collectively the runs
  passed all 6,084 `darkmatter`, 643 `darkmatter-cli`, and 640 `dmls` tests.
  Partition counts were `1546/176/164`, `1529/168/161`, `1513/153/159`, and
  `1496/146/156`; the skipped counts shown by each run are precisely the tests
  assigned to the other count partitions, so no selected Level 1 test was
  omitted.
- **G5-02 — lint:** the Darkmatter area's `just lint` recipe passed for
  `darkmatter`, `darkmatter-cli`, and `dmls` after P11's test-fixture change.
- **G5-03 — command correction:** an initial `just lint --offline --color
  never` invocation exited before running a lint because the area recipe does
  not accept forwarded arguments. The canonical `just lint` invocation then
  passed; this was command-shape noise, not a source, test, or lint failure.

## Phase 6 scoped macOS area gates

All commands ran serially in the integration worktree with
`CARGO_TARGET_DIR=/private/tmp/dm-mega-merge-target.EhU7p8`,
`CARGO_BUILD_JOBS=8`, `NEXTEST_TEST_THREADS=8`,
`BISCUIT_L2_THREADS=1`, and `CARGO_TERM_COLOR=never`. Build, Level 1, and
Level 2 recipes received `--locked --offline --color never`; lint recipes do
not accept Cargo arguments and instead ran with `CARGO_NET_OFFLINE=true`.
Every recorded command exited 0 in the final tree.

| Area / working directory | Exact commands | Final result | Logs |
|---|---|---|---|
| `biscuit-file` / `biscuit-file` | `just build --locked --offline --color never`; `just test --locked --offline --color never`; `just test-l2 --locked --offline --color never`; `just lint` | Build and lint passed. L1 passed 624 library and 61 CLI tests; four tier-filtered skips. L2 reported its canonical intentional no-op. | `phase6-biscuit-file-{build,test,test-l2,lint}.log` |
| `sniff` / `sniff` | same four area commands | Build and lint passed. L1 passed 1,634 library and 769 CLI tests; six tier-filtered skips. L2 passed 2 tests with 772 non-L2 skips. | `phase6-sniff-build.log`, `phase6-sniff-test-rerun-6.log`, `phase6-sniff-test-l2.log`, `phase6-sniff-lint.log` |
| `darkmatter` / `darkmatter` | same four area commands | Build and lint passed for `darkmatter`, `darkmatter-cli`, and `dmls`. L1 passed 6,084 + 643 + 640 tests; 214 tier-filtered skips. L2 passed 18 + 69 + 3 tests; non-L2 tests were filtered by the owning recipe. | `phase6-darkmatter-{build,test,test-l2,lint}.log` |
| `claudine` / `claudine` | same four area commands | Build and lint passed for all five area packages. Final L1 passed 21 catalog-types, 3,411 library, 47 contract, 1,907 CLI, and 90 generator tests; 170 tier-filtered skips. Final L2 passed all 131 selected CLI tests. | `phase6-claudine-build-final.log`, `phase6-claudine-test-final.log`, `phase6-claudine-test-l2-final.log`, `phase6-claudine-lint-final.log` |

`biscuit-terminal` was not activated: Phase 5 proved the parent subtrees are
byte-identical, no corrective edit touched the area, and the downstream
Darkmatter terminal seam passed its complete L2 recipe twice (Phase 5 and
Phase 6).

### Requirement-to-test mapping and corrective evidence

Phase 6 changes no intended product behavior. It exposed test-environment,
generated-artifact, and current-lint drift while running the normal public
entry points. Corrections retained the original observable assertions:

- **Sniff repository discovery:** the original aggregate, filesystem,
  git-status, default JSON, structure JSON, and library detection tests now run
  from committed temporary repositories or explicit temporary base paths.
  This preserves every public JSON/text and section assertion while preventing
  unrelated, prunable `/private/tmp/dmbench` registrations from becoming test
  inputs. No test was added. The 29 updated cases are the four library cases
  `test_detect_returns_result`, `test_detect_with_base_dir`,
  `test_os_present_by_default`, and `test_skip_os_with_filesystem_only`; the
  integration case `test_detect_with_custom_base_dir`; the topology case
  `aggregate_structure_child_includes_monorepo_topology`; ten
  `repo_aggregate_json_*`/aggregate-output cases; eleven filesystem,
  base-directory, structure, and git-status CLI cases; and
  `test_no_subcommand_with_json_outputs_json` plus
  `test_repo_subcommand_json_output`. Targeted runs passed 27 cases in
  `phase6-sniff-targeted-hermeticity-7.log` and the final two in
  `phase6-sniff-targeted-hermeticity-8.log`; the complete area L1/L2/lint gates
  then passed. All fixtures use `tempfile`, `PathBuf`, and `current_dir`, with
  no platform-specific separator assumption.
- **Claudine shipped dispatch inventory:** both merge parents carried five
  stale source-line references. The canonical
  `CLAUDINE_UPDATE_INVENTORY=1 cargo nextest run --locked --offline --color
  never -p claudine-cli --test dispatch_inventory -E
  'test(dispatch_inventory_matches_committed_file)'` path refreshed only those
  line numbers. The same test then passed without the bless environment, and
  passed again through two complete L1 reads of the real shipped artifact.
  This gives the required read/write/read verification without changing the
  scanner schema or inventory membership. Logs are
  `phase6-claudine-dispatch-inventory-{bless,targeted}.log`.
- **Claudine real-terminal expressions report:** the exact original input
  remains `claudine context --expressions` in a 65-column tmux pane. The merged
  catalog is 772 lines at that width, so the stale 480-row viewport could no
  longer observe the list it asserts. The pane height is now 820; the public
  marker, hanging-indent, and one-cell right-margin assertions are unchanged.
  The targeted L2 case passed, followed by the complete 131-test L2 gate.
- **Claudine proxy lifecycle:** current Clippy flagged the existing typed
  `CompositionError` boundary and a nested conditional. GitNexus reported HIGH
  impact (five symbols, two composition processes) before edits. The correction
  is behavior-neutral: the conditional is an equivalent let-chain and the
  shared typed error remains unboxed under narrowly reasoned lint allowances.
  `proxy_target_preflight_approves_frontmatter_shell_and_rematerializes`
  passed the original whitelisted frontmatter-shell input and its downstream
  materialized prompt assertion. The real-terminal cases
  `level2_lifecycle_proxy_target_harness_plan_failure_routes_blocked_finalize_with_err`
  and
  `level2_lifecycle_proxy_target_lifecycle_parse_failure_routes_blocked_finalize_with_err`
  passed their provider-not-launched, persisted event, and typed
  blocked/finalize-state assertions. The complete Claudine L1/L2/lint gates
  then passed in the final tree.

### Failures, skips, and integrity audit

- The initial Sniff L1 run failed only when ambient corrupt/prunable benchmark
  worktree registrations were discovered. Several intermediate targeted
  compile/test attempts exposed fixture-path and helper-visibility mistakes;
  those attempts are retained in the ledger and the final targeted plus broad
  gates pass.
- Claudine's first L1 run found the pre-existing dispatch-inventory drift. Its
  first L2 run found the stale expressions viewport. Its first lint run found
  the three current-Clippy diagnostics described above. Each owning correction
  was impact-audited and followed by targeted and complete downstream gates.
- `level2_dry_run_not_installed_renders_yellow_dim_in_tmux` missed its target
  row on the first attempt in both successful complete L2 reruns and passed on
  retry; Nextest records it as one unrelated pre-existing flake. All final
  selected tests passed. Other reported skips are intentional tier filters or
  the `biscuit-file` L2 no-op.
- `Cargo.lock` remained byte-identical at SHA-256
  `52c2a58dc23331afe1cd82424ea5b295123ed4c1f40275fd35a400709d9ef286`.
  No snapshot, fixture, or baseline changed. The only generated documentation
  delta is the reviewed five-line dispatch inventory refresh. `git diff
  --check` passed.
- Process checks after every area and at closeout found no competing Cargo,
  rustc, or Nextest process. Long-lived DMLS processes, another Codex session,
  and one pre-existing orphan test tmux session were observed but did not use
  the integration target or run a competing gate; none was modified.
- Cross-platform review found no new OS-specific production behavior or path
  construction. The Sniff fixtures use Rust path APIs, the tmux-only assertion
  remains under `cfg(unix)`, and repository CI retains macOS, Ubuntu, and
  Windows coverage. Native Windows/Linux and Level 3 execution remain outside
  this phase's completion gate as specified.
- Final read-only GitNexus `detect_changes(scope = all)` reports the expected
  HIGH aggregate merge scope: 869 changed symbols across 205 files and 13
  affected processes. The increase from the Phase 5 snapshot is attributable
  to the reviewed Phase 6 Sniff test isolation and Claudine corrective edits;
  the newly visible Claudine harness process is covered by the focused proxy
  tests and complete downstream area gates. Phase 7 still owns the one final
  index refresh and parent/`main` reconciliation.

## Phase 7 integration closure

Phase 7 introduced no runtime behavior change and required no new regression
test. The Darkmatter-aware hash path updated the merged skill frontmatter to
`87f17662fa397abe-c0eb7c8a0924fdd4`; the immediate semantic-diff verification
exited successfully. The operator subsequently staged all reviewed resolutions
and created two-parent merge commit
`b6babd517fe3189d1a04ab8abeb0c07ab3be6ea0`, whose parents are the pinned
Darkmatter and more-is-more commits. Its unmerged index is empty.

All required affected-area build, Level 1, Level 2, and lint gates passed. The
authoritative Darkmatter-area results are:

- `darkmatter` Level 1: 6,084 passed, 48 slow, 5 flaky, 140 skipped.
- `darkmatter-cli` Level 1: 643 passed, 13 slow, 1 flaky, 71 skipped.
- `dmls` Level 1: 640 passed.
- Level 2: 18 `darkmatter`, 69 `darkmatter-cli`, and 3 `dmls` tests passed.
- `darkmatter`, `darkmatter-cli`, and `dmls` lint gates passed with no warning
  or error.

The six Level 1 flakes all passed under the configured Nextest retries and are
recorded in `merge-report.md` for separate reliability follow-up. They do not
represent unresolved merge failures. Native Windows/Linux and Level 3 runs
remain outside the approved completion gate and are not claimed.

### Fresh integration index

GitNexus completed a fresh index for tested integration commit `b6babd517` at
`2026-07-22T03:58:59.820Z`: 6,567 files, 138,356 symbols, 276,534
relationships, 3,722 communities, and 300 execution flows. Change detection
reported LOW all-scope risk for the sole generated `CLAUDE.md` delta and
CRITICAL aggregate branch risk versus `main` across 5,953 changed symbols, 857
files, and 78 affected processes. The latter reflects the intentionally broad
long-lived branch merge; the scoped gates above cover its affected package
areas.

This completed refresh supersedes the earlier failed, time-bounded GitNexus
attempts recorded in historical phases. The integration commit, its test
evidence, and its index are therefore a complete handoff artifact.

## Receiving merge closure

The Darkmatter worktree started the receiving merge from
`2ddef848d9b0f1b61d01df8dfeaccd01e1f2e99f` with `b6babd517` as
`MERGE_HEAD`; their merge base is original Darkmatter parent `14dd391f`. The
receiving merge has no unmerged entries. Its one conflict was generated
`CLAUDE.md` GitNexus metadata and was resolved with the receiving placeholder,
because the file must be regenerated after every commit.

Before this record and `merge-report.md` were refreshed, a tree comparison
against `b6babd517` found only 11 target-side control or documentation paths:
`.gitignore`, `CLAUDE.md`, seven mega-merge planning/evidence documents, and two
prompt documents. These two closure records add two more documentation-only
differences. No Rust source, test, manifest, schema, snapshot, workflow, or
runtime configuration differs from the tested integration commit.

The full cached `git diff --check` reports inherited whitespace in evidence and
prompt artifacts already committed in `b6babd517`. Relative to that tested
commit, the only pre-existing warning outside these updated records is a blank
line at end of `more-is-more-log.md`; it is retained to avoid changing tested
incoming evidence. The closure-record edits introduce no whitespace warning.

The receiving merge is ready for its final merge commit. After that commit,
the operator must run `node .gitnexus/run.cjs analyze`, perform final change
detection against `main`, and confirm the expected generated `CLAUDE.md` delta.
No final receiving commit, push, tag, source-branch deletion, or worktree
deletion is performed by this documentation update.
