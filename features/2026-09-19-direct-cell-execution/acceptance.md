---
kind: acceptance-evidence
feature: 2026-09-19-direct-cell-execution
created: 2026-09-20
plan_phase: 9
status: implementation-complete-ready-for-review
hosted_evidence: none
---

# Acceptance evidence — direct cell execution

Each of the specification's eight acceptance criteria, the named fixture or
artifact that demonstrates it, and the command that reproduces it. Every
command runs from the repository root.

**Everything below is local evidence from macOS on 2026-09-20.** This session
made no push and no hosted run. The claims that need one are listed under
[What is not proven](#what-is-not-proven), with the commands that close them.

Command shorthand used in the tables:

| Shorthand | Command |
|---|---|
| `py <suite> <Class>` | `python3 scripts/ci/<suite>.py <Class>` (a single test: `<Class>.<test>`) |
| `contracts <name>` | `cargo nextest run -p test-toolkit --test ci_workflow_contracts -E 'test(<name>)'` |
| `rollup <name>` | `cargo nextest run -p repo-deps --bin ci-rollup -E 'test(<name>)'` (the tests live in `scripts/ci-rollup-tests.rs`) |

`just _test repo-deps` and `just _test test-toolkit` run every Rust test named
here; `python3 scripts/ci/<suite>.py` runs a whole Python suite.

## AC1 — Every executing cell is dispatched exactly once

> Every executing cell appears exactly once across the shipped dispatch row
> sets, with matching identity and fields. Reused, accepted-gap, prohibited,
> and event-deferred work creates no execution row. Duplicate, dropped, and
> altered rows fail the relevant contracts.

| Claim | Evidence | Reproduce |
|---|---|---|
| The four row sets partition the executing cells, with no duplicate key, for all seven corpus shapes (`pr`, `all`, `all-reused`, `mixed`, `nightly`, `prohibited`, gap-only) | `test_resolved_plan.py::RowAdapterOracleTests::test_the_four_row_sets_partition_the_executing_cells_for_each_corpus_shape`, `…::test_a_wsl2_row_never_appears_in_the_native_test_set`, `…::test_an_area_with_zero_executing_cells_still_appears` | `py test_resolved_plan RowAdapterOracleTests` |
| The area forwards exactly the rows the plan holds, read from the plan as the workflow reads it; a dropped row set is visible as missing coverage (non-vacuity) | `RowSetBoundaryTests::test_the_forwarded_inputs_partition_the_areas_executing_cells`, `…::test_a_dropped_row_set_is_visible_as_missing_coverage` | `py test_resolved_plan RowSetBoundaryTests` |
| Every dispatched row resolves to its own cell; an unknown, duplicated, reused, wrong-runner, unknown-environment, or malformed row is refused with a coded reason | `CellContractTests::test_every_dispatched_row_resolves_to_its_own_cell`, `…::test_a_row_for_a_cell_the_plan_does_not_carry_is_refused`, `…::test_a_duplicated_cell_is_refused_rather_than_taking_the_first`, `…::test_a_row_for_a_reused_cell_is_refused`, `…::test_a_row_dispatched_to_the_wrong_runner_is_refused`, `…::test_a_row_naming_an_unknown_environment_is_refused`, `…::test_a_malformed_row_is_refused_rather_than_defaulted` | `py test_resolved_plan CellContractTests` |
| Rows are derived from the plan only, persist and re-read to the same rows, and applying evidence withdraws the rows it satisfied | `test_affected_scope.py::DirectExecutionOracleTests::test_the_row_adapter_reads_nothing_but_the_plan`, `RowEmissionTests::test_a_persisted_plan_round_trips_to_the_same_rows`, `…::test_applying_evidence_withdraws_the_rows_it_satisfied` | `py test_affected_scope DirectExecutionOracleTests RowEmissionTests` |
| Reused, gap, prohibited, and deferred cells create no row or build demand | `test_resolved_plan.py::ProhibitionTests::test_a_prohibited_environment_schedules_nothing`, `BuildOwnershipTests::test_a_governed_gap_creates_no_consumer_demand`, `…::test_an_all_reused_plan_schedules_no_owner`, `test_affected_scope.py::EventSchedulingTests::test_a_pull_request_plan_carries_no_deferred_cell_build_or_preflight` | `py test_resolved_plan ProhibitionTests BuildOwnershipTests`; `py test_affected_scope EventSchedulingTests` |
| Local observation: this tree's plan (areas `root`, `tools`) has 9 executing cells and 9 unique rows | Phase 7 and Phase 8 logs, "Wave 3: local comparison" | `just ci-local --plan` |

## AC2 — No environment lists; consumers bind rows to the plan

> Hosted area/package workflows accept no independent environment lists.
> Consumers verify row-to-plan binding and do not recalculate scope. Old scope
> receipts fall back cleanly after the schema change; unchanged qualifying
> validation evidence remains reusable under its applicable checks.

| Claim | Evidence | Reproduce |
|---|---|---|
| Each reusable workflow's `workflow_call` inputs are pinned exactly: rows and run scalars only (fails on `main`'s 23-input `_package-ci.yml`) | `ci_workflow_contracts.rs::the_reusable_workflows_accept_only_rows_and_run_scalars`, `…::no_reader_facing_workflow_declares_an_environment_list_input` | `contracts the_reusable_workflows_accept_only_rows_and_run_scalars` |
| No shipped reader (workflows, `just/`, `scripts/ci`, the hook, the root justfile) reads the retired `matrix`/`area_matrix` projection | `…::no_shipped_reader_consumes_a_retired_environment_list_projection`, `test_schema.py::…::test_the_projection_fields_are_what_the_workflow_reads` | `contracts no_shipped_reader_consumes_a_retired_environment_list_projection` |
| The plan admits exactly the dispatch path the workflows implement (`rows`), and a plan claiming `lists` is refused | `…::the_plan_admits_exactly_the_dispatch_path_the_workflows_implement`, `test_affected_scope.py::ExecutionPathTests`, `test_schema.py::…::test_an_area_on_the_environment_list_path_is_refused` | `py test_affected_scope ExecutionPathTests` |
| Consumers resolve their contract from the plan by exact cell key | AC1's `CellContractTests` (the real `cell_contract.contract` over the real rows) | `py test_resolved_plan CellContractTests` |
| An older scope receipt misses once as `scope-schema`; a current one still validates; a receipt carrying the retired keys still validates and still hits through the real `ci.yml` scope step | `test_resolved_plan.py::ScopeReceiptMigrationTests::test_a_version_2_scope_receipt_misses_once_with_scope_schema`, `…::test_a_current_generation_scope_receipt_still_validates`, `test_schema.py::…::test_a_schema_mismatch_is_reported_as_scope_schema`, `…::test_a_receipt_carrying_the_retired_environment_lists_still_validates`, `test_ci_local.py::WorkflowScopeStepTests::test_a_receipt_from_before_the_environment_lists_retired_still_hits` | `py test_resolved_plan ScopeReceiptMigrationTests`; `py test_ci_local WorkflowScopeStepTests` |
| Validation receipts did not move version, so passing evidence stays reusable | `test_schema.py::…::test_version_5_is_the_plans_schema_and_the_receipts_do_not_move`, `…::test_a_complete_pass_is_reusable`, `…::test_an_interrupted_cell_leaves_its_siblings_reusable` | `python3 scripts/ci/test_schema.py` |

## AC3 — All-reused and gap-only areas keep a blocking audit

> All-reused and gap-only areas create no execution-workflow call but retain
> their blocking audit, area slice, and summary. Neutral gap checks remain
> visible. Unexpectedly skipped execution with required cells cannot pass.

| Claim | Evidence | Reproduce |
|---|---|---|
| An all-reused area skips execution but keeps its audit, slice, and gap publisher | `ci_workflow_contracts.rs::an_all_reused_area_skips_execution_but_keeps_its_audit_slice_and_publisher`, `…::every_selected_area_owns_an_always_coverage_audit` | `contracts an_all_reused_area_skips_execution` |
| All-reused and gap-only areas dispatch nothing and are still judged by the real audit | `test_completion.py::AuditEndToEndTests::test_an_all_reused_area_dispatches_nothing_and_is_still_judged`, `…::test_a_gap_only_area_dispatches_nothing_and_is_still_judged` | `py test_completion AuditEndToEndTests` |
| A skipped producer call over executing cells is missing coverage; over none it is not | `…::test_a_skipped_producer_over_executing_cells_is_missing_coverage`, `ci-rollup-tests.rs::a_skipped_producer_call_blocks_only_over_executing_cells` | `rollup a_skipped_producer_call_blocks_only_over_executing_cells` |
| An all-reused area is still scheduled and owns a result slice | `test_affected_scope.py::AllReusedAreaFanOutTests`, `test_resolved_plan.py::ResultCompletenessTests::test_an_area_with_all_test_cells_reused_still_owns_a_result_slice` | `py test_affected_scope AllReusedAreaFanOutTests` |
| Neutral gap checks: only the area's accepted gaps are published, as neutral, by the only job holding `checks: write` | `test_publish_gaps.py` (19), `ci_workflow_contracts.rs::the_gap_publisher_is_the_only_job_holding_checks_write`, `ci-rollup-tests.rs::an_accepted_gap_is_neither_a_pass_nor_a_failure_and_does_not_block`, `…::a_real_failure_outranks_an_accepted_gap` | `python3 scripts/ci/test_publish_gaps.py` |

## AC4 — Consumers keep archive, provisioning, identity, and build sharing

> Native macOS, Linux, Windows, and WSL2 consumers preserve archive
> verification, runtime provisioning, canonical gate behavior, artifact
> identity, and retry attribution. Multiple consumers share one build;
> unrelated build failures do not suppress them. Check/lint and dependent
> compile behavior are unchanged.

| Claim | Evidence | Reproduce |
|---|---|---|
| Archive consumers verify first, never compile, never fall back to a build, and report their planned key and realized digest | `ci_workflow_contracts.rs::an_archive_consumer_verifies_first_and_never_reaches_a_compiler`, `…::an_archive_miss_never_enters_a_fallback_build`, `…::a_cell_with_no_build_record_refuses_instead_of_compiling_in_place`, `…::every_archive_consumer_reports_its_planned_key_and_realized_digest`, `…::an_archive_consumer_hands_native_programs_native_paths` | `just _test test-toolkit` |
| Runtime provisioning reaches each row: native prerequisites, runner tools, Node, toolchain, backends, companions | `test_resolved_plan.py::CellContractTests::test_a_guest_row_takes_the_linux_native_prerequisites`, `ci_workflow_contracts.rs::messenger_stub_runner_tool_reaches_native_and_wsl2_execution`, `…::the_wsl_leg_provisions_jq_and_forwards_the_slow_test_contract`, `test_affected_scope.py::PackageExecutionTests::test_toolchain_environments_follow_the_declaration_and_the_capability` | `py test_resolved_plan CellContractTests`; `py test_affected_scope PackageExecutionTests` |
| Canonical gate behavior: producers invoke the canonical recipes | `…::the_reusable_workflow_invokes_the_canonical_recipes`, `test_completion.py::ShippedSelectionTests::test_every_shipped_tier_expression_is_canonical` | `contracts the_reusable_workflow_invokes_the_canonical_recipes` |
| Artifact identity stays `{package, environment, gate}` | `…::artifact_names_carry_the_environment_not_the_runner_label`, `…::every_producer_uploads_a_completion_artifact` | `contracts artifact_names_carry_the_environment_not_the_runner_label` |
| One build serves many consumers, including the WSL2 guest | `test_resolved_plan.py::BuildOwnershipTests::test_l1_and_browser_consumers_share_one_compatible_linux_key`, `…::test_one_linux_build_feeds_native_linux_and_the_wsl2_guest`, `ci_workflow_contracts.rs::the_wsl2_guest_consumes_the_same_build_as_native_linux` | `py test_resolved_plan BuildOwnershipTests` |
| An unrelated build failure suppresses nobody | `ci_workflow_contracts.rs::the_area_fan_out_waits_for_the_owner_without_being_cancelled_by_it`, `ci-rollup-tests.rs::a_reused_cell_consumes_no_build_and_cannot_be_blocked_by_one` | `contracts the_area_fan_out_waits_for_the_owner` |
| Check/lint and the dependent-compile half are unchanged | `…::unchanged_dependents_are_compiled_inside_the_changed_packages_linux_check`, `…::lint_and_check_labels_identify_their_environment`, `test_resolved_plan.py::CellContractTests::test_a_check_row_carries_its_dependent_seam_and_no_recipe`, `test_affected_scope.py::DependentSeamTests` | `py test_affected_scope DependentSeamTests` |
| Retry attribution | See AC7 | — |

## AC5 — Producers prove completeness

> Producer fixtures cover missing and malformed reports, missing tests, extra
> filters, duplicate identities, retries, explicit ignored/approved skips,
> expired approvals, empty suites, required backend failure, companion
> failure, upload failure, and cancellation. A missing test cannot use a skip
> approval to pass. Canonical tier exclusions do not create false missing-test
> failures.

All in `test_completion.py::CompletionValidatorOracleTests` unless noted
(`py test_completion CompletionValidatorOracleTests`).

| Case | Test |
|---|---|
| Missing report / malformed report | `test_a_missing_report_fails_validation`, `test_a_malformed_report_fails_validation` |
| Missing test, and a skip approval cannot excuse it | `test_a_missing_expected_test_fails_and_a_skip_approval_cannot_excuse_it` |
| Unexpected identity | `test_an_unexpected_identity_fails_validation` |
| Extra filter | `test_an_extra_filter_that_narrows_the_selection_is_refused`; `ShippedSelectionTests::test_an_ad_hoc_narrowing_of_a_shipped_expression_is_caught`; `ManifestCrossCheckTests::test_run_ignored_is_refused_because_it_moves_the_expected_set` |
| Duplicate identities | `test_duplicate_identities_fail_while_distinct_binaries_stay_distinct` |
| Retries | `test_retries_normalize_to_one_final_outcome_without_hiding_a_failure` |
| Explicit ignored / approved skip / expired approval | `test_an_explicitly_ignored_test_is_expected_but_not_required`, `test_an_observed_skip_with_an_unexpired_approval_completes`, `test_an_observed_skip_with_an_expired_approval_fails` |
| Empty suite | `test_an_empty_expected_set_without_a_plan_recorded_reason_fails` |
| Required backend failure | `test_a_required_backend_proof_absent_fails_and_present_passes`; `ManifestCrossCheckTests::test_a_producer_provisioned_for_other_backends_is_refused` |
| Companion failure | `test_a_failed_companion_suite_fails_the_cell`, `test_a_companion_only_cell_fails_when_its_declared_suite_did_not_complete`; `ManifestCrossCheckTests::test_a_producer_that_ran_other_companions_is_refused` |
| Canonical tier exclusion / `cfg`-excluded test is not missing | `test_a_canonical_tier_exclusion_is_not_a_missing_test` |
| Upload failure | `ci_workflow_contracts.rs::a_failed_required_upload_fails_the_job`, `…::every_producer_uploads_a_completion_artifact`; `test_runner_loss.py::FailureStageTests::test_an_upload_failure_after_passing_tests_is_not_a_test_failure` |
| Cancellation | `ci_workflow_contracts.rs::cancellation_keeps_failure_path_publication_best_effort`; `ci-rollup-tests.rs::a_cancelled_job_is_not_an_accepted_gap` |
| A refusal leaves no stale record | `CompletionRecordContractTests::test_a_refusal_removes_a_record_an_earlier_validation_left` |
| The shipped recipe's manifest is accepted end to end | `ShippedRecipeEndToEndTests::test_the_shipped_recipe_emits_a_manifest_the_validator_accepts` (needs `cargo-nextest`) |

## AC6 — The audit rejects unproven completion and legacy bypass

> Audit fixtures reject absent or mismatched completion records and
> artifacts, invalid reuse, invalid gaps, and partial rerun evidence. Legacy
> evidence cannot silently bypass checks removed from the new execution path.

All in `scripts/ci-rollup-tests.rs` (`just _test repo-deps`).

| Case | Test |
|---|---|
| Absent completion record | `an_executing_cell_without_a_completion_record_blocks_the_verdict`, `a_green_check_status_needs_its_completion_record` |
| Mismatched record: revision, build key, cell, build, unplanned cell | `a_completion_record_bound_to_another_revision_blocks`, `a_completion_record_bound_to_another_build_key_blocks`, `a_completion_record_that_does_not_describe_its_cell_blocks`, `a_completion_record_must_name_exactly_the_planned_build`, `a_completion_record_for_an_unplanned_cell_blocks` |
| Mismatched artifacts: absent inventory, uncertified report, incomplete record | `a_green_status_with_an_absent_report_inventory_blocks`, `a_staged_report_the_record_does_not_certify_blocks`, `an_incomplete_completion_record_blocks_green_junit` |
| Malformed record | `a_malformed_completion_record_is_a_tool_error_not_a_verdict` |
| Partial rerun evidence | `a_completion_record_from_another_run_blocks_and_an_earlier_attempt_does_not`, `a_rerun_that_publishes_both_attempts_keeps_the_failure` |
| Invalid reuse | `a_check_the_plan_reused_through_its_l1_receipt_expects_no_producer_status`; `test_schema.py::…::test_a_reused_cell_must_name_its_evidence`, `…::test_a_reused_cell_that_no_evidence_may_satisfy_is_refused` |
| Invalid gaps | `an_expired_accepted_gap_blocks_its_area`, `an_ungoverned_gap_is_never_accepted_however_the_plan_labels_it`, `an_accepted_gap_state_with_no_policy_entry_blocks` |
| Legacy evidence keeps its own checks; the new path's checks apply by version | `the_expected_manifest_diff_applies_to_legacy_cells_only`, `a_certified_cells_skips_are_reported_while_stale_approvals_still_block`, `a_version_two_expected_manifest_is_refused_with_its_owner_named` |
| Another generation's documents are refused, never read as legacy | `a_version_four_result_document_is_refused_rather_than_read_as_legacy`, `a_plan_from_another_schema_generation_is_refused_rather_than_audited` |

## AC7 — Attribution, sibling isolation, and the unchanged gate

> Runner-loss fixtures derive actual job labels from the new workflows and
> identify native and WSL2 cells. Matrix failures do not cancel siblings;
> producer failures reach the unchanged `ci-gate` without duplicate policy
> failures. Producer permissions remain read-only.

| Claim | Evidence | Reproduce |
|---|---|---|
| Labels are derived from the shipped workflows; every producer job parses to the status it uploads | `test_runner_loss.py::JobNameCorpusTests::test_every_producer_job_parses_to_the_status_it_uploads`, `…::test_every_status_uploading_job_is_covered`, `DirectExecutionAttributionTests::test_the_shipped_workflows_declare_the_row_set_inputs` | `py test_runner_loss JobNameCorpusTests DirectExecutionAttributionTests` |
| A lost native or WSL2 row resolves to exactly one cell | `DirectExecutionAttributionTests::test_a_four_token_native_row_label_resolves_to_exactly_one_cell`, `…::test_a_four_token_wsl2_row_label_resolves_to_exactly_one_cell`, `JobNameCorpusTests::test_a_native_and_a_guest_row_of_one_package_are_two_cells` | as above |
| Matrix failures do not cancel siblings | `ci_workflow_contracts.rs::every_matrix_in_the_reader_facing_workflows_keeps_fail_fast_false`, `…::no_test_row_job_stages_behind_another_test_row_job` | `contracts every_matrix_in_the_reader_facing_workflows_keeps_fail_fast_false` |
| `ci-gate` is the unchanged policy-free fold of exactly the blocking jobs | `…::ci_gate_is_the_single_required_check`, `…::ci_gate_needs_exactly_the_surviving_blocking_jobs`, `…::every_top_level_job_is_either_folded_by_ci_gate_or_the_advisory_report`, `…::only_ci_gate_makes_a_run_level_claim` | `contracts ci_gate` |
| A producer failure is not doubled by a red audit check | `test_runner_loss.py::ClassifyTests::test_a_coverage_audit_failure_vetoes_a_runner_loss_retry`; `ci-rollup-tests.rs::a_producer_failure_downgrades_a_green_report`; the audit's `--producers` rule, Phase 6 log | `py test_runner_loss ClassifyTests` |
| Producer tokens stay read-only; only `accepted-gaps` holds `checks: write` | `ci_workflow_contracts.rs::the_gap_publisher_is_the_only_job_holding_checks_write` | `contracts the_gap_publisher_is_the_only_job_holding_checks_write` |

## AC8 — Budgets, the trial, and the suites

> Offline full-workspace and nightly plans satisfy matrix, nesting, and output
> budgets. The focused root-area trial matches the plan without duplicate gate
> runs. The Python suites, `ci_workflow_contracts`, rollup and hook suites, and
> actionlint pass using canonical repository recipes where available.

| Claim | Evidence | Reproduce |
|---|---|---|
| Real full-workspace, push, and nightly plans fit every matrix and byte ceiling | `test_affected_scope.py::CapacityGuardTests::test_the_real_full_workspace_and_nightly_plans_fit_every_ceiling`; measurements in `spikes/s3-capacity.md` § "Phase 7 re-measurement" | `py test_affected_scope CapacityGuardTests`; `python3 features/2026-09-19-direct-cell-execution/spikes/capacity.py` |
| The guard refuses before dispatch, whole, emitting nothing; the aggregate budget refuses many small areas | `CapacityGuardTests::test_an_over_budget_area_fails_before_dispatch_and_emits_nothing`, `…::test_the_combined_scope_output_budget_refuses_many_small_areas`, `DirectExecutionOracleTests::test_an_over_limit_row_set_fails_planning_with_a_named_error` | `py test_affected_scope CapacityGuardTests` |
| The chain stays within four levels | `ci_workflow_contracts.rs::the_reusable_workflow_chain_stays_within_githubs_four_levels` | `contracts the_reusable_workflow_chain_stays_within_githubs_four_levels` |
| The focused trial matches the plan without duplicate gate runs | **Not proven: needs a push.** Local comparison only (Phase 7 log): 9 executing cells = 9 rows; `just ci-local` 13/13 gates. | See [What is not proven](#what-is-not-proven) |
| The suites and actionlint pass | [Validation sweep](#validation-sweep-phase-9-2026-09-20) | as listed there |

## Validation sweep (Phase 9, 2026-09-20)

Run on macOS (this host) against the final working tree, after Phase 9's
documentation-drift fix. All green; no test was skipped beyond the
pre-existing skips named below.

| Gate | Command | Result |
|---|---|---|
| Python contract suites (13) | `python3 scripts/ci/test_<suite>.py`, each | 905 tests, all `OK` |
| &nbsp;&nbsp;`test_affected_scope` | | 282 |
| &nbsp;&nbsp;`test_schema` | | 133 |
| &nbsp;&nbsp;`test_resolved_plan` | | 96 |
| &nbsp;&nbsp;`test_ci_local` | | 92 |
| &nbsp;&nbsp;`test_evidence_reuse` | | 76 |
| &nbsp;&nbsp;`test_runner_loss` | | 50 |
| &nbsp;&nbsp;`test_completion` | | 49 |
| &nbsp;&nbsp;`test_constraints` | | 39 |
| &nbsp;&nbsp;`test_local_evidence` | | 23 |
| &nbsp;&nbsp;`test_reuse_validation` | | 21 |
| &nbsp;&nbsp;`test_publish_gaps` | | 19 |
| &nbsp;&nbsp;`test_cross_check` | | 13 |
| &nbsp;&nbsp;`test_build_key` | | 12 |
| `repo-deps` (incl. `ci-rollup` tests) | `just _test repo-deps` | 445 passed, 1 skipped (pre-existing) |
| `test-toolkit` (incl. `ci_workflow_contracts`, 149) | `just _test test-toolkit` | 228 passed, 2 skipped (pre-existing) |
| Lint | `just _lint repo-deps`, `just _lint test-toolkit` | clean, 0 warnings |
| Pre-push hook suite | `env -u CDPATH .githooks/tests/test-pre-push.sh` | 67 passed, 0 failed |
| actionlint | `actionlint .github/workflows/{ci,_area-ci,_package-ci,_wsl-ci}.yml` | clean |
| Plan on this tree | `just ci-local --plan` | 9 rows across `root` and `tools`, one per executing cell; both WSL2 cells `omit`/`accepted-gap`; 6 build records on 3 owners; 0 approved skips |

`env -u CDPATH` is needed only because this host's shell exports `CDPATH`
(recorded in `.claude/skills/os/macos.md`). `just ci-local` (the full local
gate run) was last run in Phase 8, 13/13 gates green; Phase 9 changed no code
it exercises.

Linux, Windows, and WSL2 were not run in Phase 9. The only code Phase 9
changed is a Rust string contract over documentation text, which is
platform-neutral. Phase 8 ran the 13 Python suites on Linux in Docker (see
its "OS considerations").

## What is not proven

Nothing below was claimed by this implementation. Each item needs a push or a
hosted run, which the implementing sessions were not allowed to make.

1. **The hosted trial (Phase 7, Wave 3; AC8's "trial matches the plan").** One
   ordinary pull-request run on this branch. The commands and the acceptance
   conditions are in the implementation log, Phase 7, "The hosted trial". In
   short:

   ```sh
   export GIT_TERMINAL_PROMPT=0
   git push -u origin fix/archive-path-sites
   gh pr create --draft --base main --fill           # or reuse the open PR
   RUN=$(gh run list --workflow ci.yml --branch fix/archive-path-sites --limit 1 --json databaseId --jq '.[0].databaseId')
   gh run watch "$RUN" --exit-status
   gh run view "$RUN" --json jobs --jq '.jobs[] | [.name, .conclusion] | @tsv'
   gh run download "$RUN" --name ci-resolved-plan --dir trial/plan
   gh run download "$RUN" --pattern 'completion-*' --dir trial/completion
   gh run download "$RUN" --pattern 'ci-results-*' --dir trial/slices
   ```

   Accept when the plan's executing cells equal the `completion-*`
   directories, every area slice shows `completion` without `problems` on
   every executing cell, the accepted-gap cells appear as neutral check runs,
   and `ci-gate` is green with exactly one gate run for the head SHA.
2. **S1 — hosted job labels** (`spikes/s1-labels.md`, status
   `deferred-requires-push`). The same trial run answers questions 1 and 2
   (four-token `include:`-only labels; a scalar-guarded skipped job shows its
   bare id). Question 3, the `… / wsl2 / test (wsl2-ubuntu)` label under a
   once-per-area call, needs a run with an **executing** WSL2 row. This
   branch's two WSL2 cells are accepted gaps, so no WSL2 row runs here under
   any label. The first later change that touches a package with an executing
   WSL2 cell closes it; record the answer in `s1-labels.md`.
3. **The observed-cells comparison against a hosted run.** Part of item 1.
   Locally the observed macOS cells match the plan (Phases 7 and 8).
4. **The WSL2 guest's completion record on a real guest.** Proven only by
   contracts and by the Phase 5 guest-provisioning tests. Closed by the same
   later run as S1 question 3.
5. **Windows and WSL2 local runs.** Not exercised in any phase. The changed code is
   the Python planner, workflow YAML, Rust string contracts, and a bash/jq
   recipe; see each phase's "OS considerations". `push` to `main` adds the
   Windows leg and the nightly schedule adds WSL2.

## Known gaps and follow-ups (not blocking)

- **`execution_path` is redundant.** It admits only `rows`. Removing it bumps
  the plan schema from v5 to v6 for no behavior change, so it was left for an
  optional follow-up (Phase 8 log, "Decisions worth reading").
- **`local_evidence.py`'s version-1 path still reads `scope["matrix"]`.** It
  verifies version-1 notes already in the wild, from a caller-supplied legacy
  document; no shipped caller uses it.
- **`package_cells` raises `KeyError`** for an L2 backend with no capability
  entry. Unreachable with shipped artifacts, because `load_environments`
  refuses such a table.
- **build-linux lock.** Phase 8 found build-linux held by a six-day-old lock
  (`nightly-reward-spike`, owner `reward-20260914-c3e60d0`). Someone should
  check whether it is stale.
