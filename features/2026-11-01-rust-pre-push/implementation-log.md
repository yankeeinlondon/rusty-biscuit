---
spec: "/Volumes/coding/rusty-biscuit/features/2026-11-01-rust-pre-push/spec.md"
plan: "features/2026-11-01-rust-pre-push/plan.md"
implemented_by: "claude/opus"
started_phase: "1"
source_files_during_phase_1:
  - Cargo.lock
  - scripts/Cargo.toml
  - scripts/lib.rs
  - scripts/ci-push.rs
  - scripts/ci-push/mod.rs
  - scripts/ci-push/host.rs
  - scripts/ci-push/json.rs
  - scripts/ci-push/mode.rs
  - scripts/ci-push/updates.rs
  - scripts/ci-push/documents.rs
  - scripts/ci-push/tests/mod.rs
  - scripts/ci-push/tests/documents.rs
  - scripts/ci-push/tests/run.rs
  - scripts/ci-push/tests/updates.rs
  - scripts/ci/fixtures/pre-push/constraints/minimal.json
  - scripts/ci/fixtures/pre-push/constraints/scoped.json
  - scripts/ci/fixtures/pre-push/plans/plan-macos-executing.json
  - scripts/ci/fixtures/pre-push/plans/plan-macos-two-packages.json
  - scripts/ci/fixtures/pre-push/plans/plan-wsl-absent.json
  - scripts/ci/fixtures/pre-push/plans/plan-wsl-executing.json
  - scripts/ci/fixtures/pre-push/plans/plan-wsl-reused.json
  - scripts/ci/fixtures/pre-push/pull-requests/none.json
  - scripts/ci/fixtures/pre-push/pull-requests/two-open.json
docs_updated_during_phase_1:
  - docs/dependencies.md
docs_created_during_phase_1:
  - scripts/ci/fixtures/pre-push/README.md
skills_files_updated_during_phase_1: []
packages:
  - repo-deps
decisions_for_review:
  - |-
    gix version. The plan says `gix = "=0.84.0"` "following sniff/worktree's manifests"; those manifests now pin `=0.90.0`, so ci-push uses `=0.90.0` (a second gix version would double the build and split types with sniff). Alternative: honor 0.84 literally, at the cost of two gix trees. Redo: one manifest line.
  - |-
    libgit2-sys ceiling. Not in the plan. Adding `git2` beside `sniff` (whose `hyperpolyglot` links pcre2-sys) reproduces the MSVC duplicate-symbol link failure sniff, tree-hugger, and darkmatter already guard against, so repo-deps carries the same `libgit2-sys = ">=0.18.3, <0.18.6"` pin. Alternative: none safe on native Windows until upstream fixes the bundled symbols.
  - |-
    What a "valid" ref-update line is. The plan asks for "valid full refs". Git writes the pushed source as the user spelled it (`HEAD`, `HEAD~1`, an object id, or `(delete)`), so only the destination must be a valid `refs/...` name; the source must merely be present. Object ids must be lowercase hex of the repository's hash length (Git never writes uppercase). A line that is not UTF-8 is malformed and blocks the push. Alternatives: also validate the source as a ref (would block legitimate `git push origin HEAD~1:refs/heads/x`), or accept uppercase ids/non-UTF-8 lines (lenient, no known producer). Redo: `updates.rs::parse_updates` and its tests.
  - |-
    Constraint-record and plan-view shape validation landed in Phase 1 (`documents.rs`), not with the matching in Phase 2. The plan's Phase 1 asks for "strict document validators" while Phase 2 owns `constraints.rs`. Phase 2 should call `parse_constraint_record` / `parse_constraint_plan` rather than re-validate. The minimal plan view now requires nonempty `environment`/`gate`/`execution`, where Python `read_plan` accepts any string including empty; Phase 2 must make the Python reader match. Redo if reversed: move the two functions.
  - |-
    `run` blocks every reviewable push for now. Review arrives in Phase 3; until then `ci_push::run` returns exit 1 with `ReviewUnavailable` for any branch update, so the unwired binary can never pass a push unreviewed. The shell hook is unchanged and still owns the real flow. Alternative: return success (unsafe if someone wires the binary early). Redo: Phase 3 replaces that branch.
  - |-
    `PushRequest` carries the raw `RUSTY_BISCUIT_PRE_PUSH` / `..._AREAS` values and `run` validates them, rather than the binary passing an already-parsed mode. This makes "invalid mode blocks before discovery" provable at the public `run` boundary. `HostServices` (production services) lives in the library, not the binary, so the real discovery boundary is tested in-process.
  - |-
    Fixture scope. The plan lists fixtures for constraints, plans, PR output, path inventories, and evidence scenarios. Phase 1 adds the first three, which its tests read. Path-inventory and evidence-scenario fixtures are left to Phases 2-4, whose tests define their shape; a fixture with no reader cannot be checked against the format it claims to carry. The five plan fixtures are copied from `.githooks/tests/fixtures/` so they survive the shell suite's retirement; the shell copies (and `affected_scope_stub.py`) go with that suite in Phase 4. Negative cases are in-memory one-field edits of a control, not stored files.
  - |-
    No CLI-spawn wiring test in Phase 1. `scripts/` has no integration-test target, so no test can resolve the `ci-push` executable (`CARGO_BIN_EXE_*` exists only there), and the binary loads no JSON document itself yet. Its wiring is tested when Phase 4 adds the launcher/hook tests, which need such a target anyway.
message_to_agent: |-
  Phase 1 built the `ci-push` foundation in `scripts/` (package `repo-deps`): `[lib] path = "lib.rs"` gating `pub mod ci_push` (from `scripts/ci-push/mod.rs`) on `local-tools`, and `[[bin]] ci-push`. Tests live in `scripts/ci-push/tests/` as lib unit tests (`ci_push::tests::{documents,updates,run}`); run them with `cargo nextest run -p repo-deps --lib -E 'test(/^ci_push::/)'` (126 tests, under 1 s).
  Reuse, do not duplicate: `json::load` is the one strict JSON loader (duplicate keys, trailing content, empty documents refused) and `json::{required_text, optional_wildcard_text, required_array, positive_integer, calendar_date}` are the shared field validators. `documents.rs` already has `parse_constraint_record` (full shape, unknown fields refused, expiry as `chrono::NaiveDate`) and `parse_constraint_plan` (Python `read_plan` view; `state: None` only when absent): Phase 2's `constraints.rs` should do matching/store/expiry on top of these. The expiry yesterday/today/tomorrow cases belong with `expired()` in Phase 2. Python `constraints.py` still has the old lenient reader; Phase 2 must tighten it to the same rules (including nonempty cell strings in `read_plan`).
  `PushServices` has one method so far (`object_hash`); add boundaries there as they are needed. `run` currently ends any reviewable push with `BlockReason::ReviewUnavailable` (exit 1); Phase 3 replaces that branch with review. Remote-URL fallback via sniff is not wired yet (`PushRequest.remote_url` is `None` when Git gave none).
  gix is `=0.90.0` (sniff's pin), not 0.84. `libgit2-sys` carries the `<0.18.6` ceiling for MSVC.
  Gates: `scripts/` has no justfile; run `just _lint repo-deps` and `just test repo-deps` from the repository root (`just lint` in `scripts/` lints every area). The Windows cross-check of Phase 1 built and linked but could not execute tests: build-win's Cargo volume was below the 50 GiB preflight. Re-run `just cross-check repo-deps --os windows` once space is available.
  Path-inventory and evidence-scenario fixtures were not created; add them under `scripts/ci/fixtures/pre-push/` when their consumers are written. The parity table in this log has 72 Bash rows plus dispatcher row D1, all `pending`.
---

# Implementation Log for 2026-11-01-rust-pre-push (5 phases)

## Parity Inventory

Inventoried at the start of Phase 1 (2026-11-10) from
`.githooks/tests/test-pre-push.sh` at commit `71f6d40e4` (its last change):
**72** `run_test` registrations (the spec's 71 is its historical count), plus
the separate dispatcher contract `.githooks/tests/test-pre-push-dispatcher.sh`
(row D1). This table is the parity record: it stays here after the shell
suites are deleted. A Rust name may be regrouped into a submodule when the
test is written (Phase 4); the row is then updated to the final path and its
result.

| # | Bash case (`run_test` description) | Original function | Assigned Rust test | Result |
| - | --- | --- | --- | --- |
| 1 | real recipe interruption withholds earlier evidence | `test_a_real_recipe_interruption_withholds_earlier_reports` | `ci_push::tests::parity::a_real_recipe_interruption_withholds_earlier_reports` | pending |
| 2 | advanced non-main target reuses and publishes exact receipts | `test_an_advanced_non_main_target_reuses_prior_cells_and_publishes_exact_receipts` | `ci_push::tests::parity::an_advanced_non_main_target_reuses_prior_cells_and_publishes_exact_receipts` | pending |
| 3 | scope-only runs no gate but resolves scope | `test_scope_only_runs_no_gate_but_still_resolves_scope` | `ci_push::tests::parity::scope_only_runs_no_gate_but_still_resolves_scope` | pending |
| 4 | off is a deprecated alias of scope-only | `test_off_mode_is_a_deprecated_alias_of_scope_only` | `ci_push::tests::parity::off_mode_is_a_deprecated_alias_of_scope_only` | pending |
| 5 | invalid mode exits 1 with helpful message | `test_invalid_mode_exits_one` | `ci_push::tests::parity::invalid_mode_exits_one` | pending |
| 6 | warn + passing → exit 0 | `test_warn_passing_tests_exits_zero` | `ci_push::tests::parity::warn_passing_tests_exits_zero` | pending |
| 7 | warn + failing → exit 0 and mentions --no-verify | `test_warn_failing_tests_exits_zero_with_no_verify_hint` | `ci_push::tests::parity::warn_failing_tests_exits_zero_with_no_verify_hint` | pending |
| 8 | strict + passing → exit 0 | `test_strict_passing_tests_exits_zero` | `ci_push::tests::parity::strict_passing_tests_exits_zero` | pending |
| 9 | strict + failing → propagate exit and mention --no-verify | `test_strict_failing_tests_exits_nonzero_with_no_verify_hint` | `ci_push::tests::parity::strict_failing_tests_exits_nonzero_with_no_verify_hint` | pending |
| 10 | unset default runs no gate when no CI run starts | `test_unset_default_runs_no_gate_for_a_push_that_starts_no_run` | `ci_push::tests::parity::unset_default_runs_no_gate_for_a_push_that_starts_no_run` | pending |
| 11 | explicit auto behaves as the default | `test_explicit_auto_is_the_default` | `ci_push::tests::parity::explicit_auto_is_the_default` | pending |
| 12 | unset default is strict for a push to main | `test_unset_default_is_strict_for_a_push_to_main` | `ci_push::tests::parity::unset_default_is_strict_for_a_push_to_main` | pending |
| 13 | unset default is strict with an open pull request | `test_unset_default_is_strict_for_a_branch_with_an_open_pull_request` | `ci_push::tests::parity::unset_default_is_strict_for_a_branch_with_an_open_pull_request` | pending |
| 14 | default path calls 'just pre-push' with no selection | `test_default_delegates_scope_to_pre_push` | `ci_push::tests::parity::default_delegates_scope_to_pre_push` | pending |
| 15 | run_hook forwards no selection without an override | `test_run_hook_forwards_no_selection_without_an_override` | `ci_push::tests::parity::run_hook_forwards_no_selection_without_an_override` | pending |
| 16 | RUSTY_BISCUIT_PRE_PUSH_AREAS override is forwarded verbatim | `test_areas_override_is_passed_through` | `ci_push::tests::parity::areas_override_is_passed_through` | pending |
| 17 | a failing warn run reaches the publication block | `test_a_failing_warn_run_reaches_the_publication_block` | `ci_push::tests::parity::a_failing_warn_run_reaches_the_publication_block` | pending |
| 18 | interrupted gate withholds staged evidence | `test_a_gate_exiting_130_never_publishes_staged_evidence` | `ci_push::tests::parity::a_gate_exiting_130_never_publishes_staged_evidence` | pending |
| 19 | interrupted hook withholds staged evidence | `test_an_interrupted_hook_never_publishes_staged_evidence` | `ci_push::tests::parity::an_interrupted_hook_never_publishes_staged_evidence` | pending |
| 20 | publication requires a clean, exact, unoverridden tree | `test_publication_requires_a_clean_exact_tree_and_no_override` | `ci_push::tests::parity::publication_requires_a_clean_exact_tree_and_no_override` | pending |
| 21 | a persisted prohibition blocks the push | `test_prohibition_blocks_the_push` | `ci_push::tests::parity::prohibition_blocks_the_push` | pending |
| 22 | a prohibition refusal states its reason | `test_prohibition_is_explained` | `ci_push::tests::parity::prohibition_is_explained` | pending |
| 23 | an expired prohibition does not block | `test_expired_prohibition_does_not_block` | `ci_push::tests::parity::expired_prohibition_does_not_block` | pending |
| 24 | a reused prohibited cell does not block | `test_a_reused_prohibited_cell_does_not_block` | `ci_push::tests::parity::a_reused_prohibited_cell_does_not_block` | pending |
| 25 | an absent prohibited environment does not block | `test_an_absent_prohibited_environment_does_not_block` | `ci_push::tests::parity::an_absent_prohibited_environment_does_not_block` | pending |
| 26 | an executing prohibited cell blocks with its reason | `test_an_executing_prohibited_cell_blocks_with_its_reason` | `ci_push::tests::parity::an_executing_prohibited_cell_blocks_with_its_reason` | pending |
| 27 | the constraint is decided in scope-only mode too | `test_the_constraint_is_decided_in_scope_only_mode_too` | `ci_push::tests::parity::the_constraint_is_decided_in_scope_only_mode_too` | pending |
| 28 | an unreadable plan blocks | `test_an_unreadable_plan_blocks` | `ci_push::tests::parity::an_unreadable_plan_blocks` | pending |
| 29 | a failed plan resolution blocks and publishes no scope | `test_a_failed_plan_resolution_blocks_and_publishes_no_scope` | `ci_push::tests::parity::a_failed_plan_resolution_blocks_and_publishes_no_scope` | pending |
| 30 | the plan resolution is handed no constraint store | `test_the_plan_resolution_is_handed_no_constraint_store` | `ci_push::tests::parity::the_plan_resolution_is_handed_no_constraint_store` | pending |
| 31 | a constraint for another branch does not block | `test_a_constraint_for_another_branch_does_not_block` | `ci_push::tests::parity::a_constraint_for_another_branch_does_not_block` | pending |
| 32 | the plan fixtures are valid resolved plans | `test_the_plan_fixtures_are_valid_resolved_plans` | `ci_push::tests::parity::the_plan_fixtures_are_valid_resolved_plans` | pending |
| 33 | a published receipt names retained, readable reports | `test_a_published_receipt_names_retained_readable_reports` | `ci_push::tests::parity::a_published_receipt_names_retained_readable_reports` | pending |
| 34 | reports that cannot be retained publish no receipt | `test_reports_that_cannot_be_retained_publish_no_receipt` | `ci_push::tests::parity::reports_that_cannot_be_retained_publish_no_receipt` | pending |
| 35 | a blocked strict failure publishes its note; CI reruns it | `test_a_blocked_strict_failure_publishes_its_note_but_ci_reruns_the_cell` | `ci_push::tests::parity::a_blocked_strict_failure_publishes_its_note_but_ci_reruns_the_cell` | pending |
| 36 | a complete warn failure publishes its note | `test_a_complete_warn_failure_publishes_its_note` | `ci_push::tests::parity::a_complete_warn_failure_publishes_its_note` | pending |
| 37 | scope-only publishes committed scope for a push to main | `test_scope_only_publishes_committed_scope_for_a_push_to_main` | `ci_push::tests::parity::scope_only_publishes_committed_scope_for_a_push_to_main` | pending |
| 38 | the published scope receipt binds this plan generation | `test_the_published_scope_receipt_binds_this_plan_generation` | `ci_push::tests::parity::the_published_scope_receipt_binds_this_plan_generation` | pending |
| 39 | committed scope declares deletions, omits rename sources | `test_the_committed_scope_declares_deletions_and_omits_rename_sources` | `ci_push::tests::parity::the_committed_scope_declares_deletions_and_omits_rename_sources` | pending |
| 40 | a feature push records the PR base, not its previous tip | `test_a_feature_branch_push_records_the_pull_request_base_not_its_previous_tip` | `ci_push::tests::parity::a_feature_branch_push_records_the_pull_request_base_not_its_previous_tip` | pending |
| 41 | a branch-creating push records the remote's main tip, says so | `test_a_branch_creating_push_records_the_remote_main_tip_and_says_so` | `ci_push::tests::parity::a_branch_creating_push_records_the_remote_main_tip_and_says_so` | pending |
| 42 | a dirty strict run publishes scope but no validation | `test_a_dirty_strict_run_publishes_scope_but_no_validation_receipt` | `ci_push::tests::parity::a_dirty_strict_run_publishes_scope_but_no_validation_receipt` | pending |
| 43 | a failed scope publication is named; exit code unchanged | `test_a_failed_scope_publication_is_named_and_leaves_the_exit_code_alone` | `ci_push::tests::parity::a_failed_scope_publication_is_named_and_leaves_the_exit_code_alone` | pending |
| 44 | a committed change masked by an unstaged revert is reviewed | `test_a_committed_change_masked_by_an_unstaged_revert_is_still_reviewed` | `ci_push::tests::parity::a_committed_change_masked_by_an_unstaged_revert_is_still_reviewed` | pending |
| 45 | an unstaged policy edit never reaches the plan or receipt | `test_an_unstaged_policy_edit_never_reaches_the_committed_plan_or_receipt` | `ci_push::tests::parity::an_unstaged_policy_edit_never_reaches_the_committed_plan_or_receipt` | pending |
| 46 | a clean checkout is planned in place | `test_a_clean_checkout_is_planned_in_place` | `ci_push::tests::parity::a_clean_checkout_is_planned_in_place` | pending |
| 47 | a push that does not carry HEAD reviews the pushed revision | `test_a_push_that_does_not_carry_head_reviews_the_pushed_revision` | `ci_push::tests::parity::a_push_that_does_not_carry_head_reviews_the_pushed_revision` | pending |
| 48 | a deletion-only push triggers nothing and is not reviewed | `test_a_deletion_only_push_triggers_nothing_and_is_not_reviewed` | `ci_push::tests::parity::a_deletion_only_push_triggers_nothing_and_is_not_reviewed` | pending |
| 49 | a tag-only push in strict mode runs no gate | `test_a_tag_only_push_in_strict_mode_runs_no_gate` | `ci_push::tests::parity::a_tag_only_push_in_strict_mode_runs_no_gate` | pending |
| 50 | a prohibition on the pushed branch blocks, whatever is checked out | `test_a_prohibition_on_the_pushed_branch_blocks_whatever_is_checked_out` | `ci_push::tests::parity::a_prohibition_on_the_pushed_branch_blocks_whatever_is_checked_out` | pending |
| 51 | a prohibition on the checked-out branch does not bind another push | `test_a_prohibition_on_the_checked_out_branch_does_not_bind_another_branch_push` | `ci_push::tests::parity::a_prohibition_on_the_checked_out_branch_does_not_bind_another_branch_push` | pending |
| 52 | every pushed branch is planned with its own path set | `test_every_pushed_branch_is_planned_with_its_own_path_set` | `ci_push::tests::parity::every_pushed_branch_is_planned_with_its_own_path_set` | pending |
| 53 | ref updates are reviewed in the order Git supplies them | `test_ref_updates_are_reviewed_in_the_order_git_supplies_them` | `ci_push::tests::parity::ref_updates_are_reviewed_in_the_order_git_supplies_them` | pending |
| 54 | the repository identity is the remote being pushed to | `test_the_repository_identity_is_the_remote_being_pushed_to` | `ci_push::tests::parity::the_repository_identity_is_the_remote_being_pushed_to` | pending |
| 55 | a renamed refspec binds a record under either branch name | `test_a_renamed_refspec_binds_a_record_under_either_branch_name` | `ci_push::tests::parity::a_renamed_refspec_binds_a_record_under_either_branch_name` | pending |
| 56 | deletions and tags among branch updates are skipped by name | `test_deletions_and_tags_among_branch_updates_are_skipped_by_name` | `ci_push::tests::parity::deletions_and_tags_among_branch_updates_are_skipped_by_name` | pending |
| 57 | a stacked pull request is planned against its target's tip | `test_a_stacked_pull_request_is_planned_against_its_target_branch_tip` | `ci_push::tests::parity::a_stacked_pull_request_is_planned_against_its_target_branch_tip` | pending |
| 58 | a pull request into main from a restoring child selects nothing | `test_a_pull_request_into_main_from_a_restoring_child_selects_nothing` | `ci_push::tests::parity::a_pull_request_into_main_from_a_restoring_child_selects_nothing` | pending |
| 59 | an advanced target records its exact remote comparison | `test_an_advanced_target_branch_is_read_from_the_remote_and_records_exact_scope` | `ci_push::tests::parity::an_advanced_target_branch_is_read_from_the_remote_and_records_exact_scope` | pending |
| 60 | no open pull request on GitHub → a provisional plan | `test_a_github_branch_with_no_open_pull_request_gets_a_provisional_plan` | `ci_push::tests::parity::a_github_branch_with_no_open_pull_request_gets_a_provisional_plan` | pending |
| 61 | a record in the default store binds a fresh session | `test_a_record_in_the_default_store_binds_a_fresh_session` | `ci_push::tests::parity::a_record_in_the_default_store_binds_a_fresh_session` | pending |
| 62 | two open pull requests from one head are each reviewed | `test_two_open_pull_requests_from_one_head_are_each_reviewed` | `ci_push::tests::parity::two_open_pull_requests_from_one_head_are_each_reviewed` | pending |
| 63 | a GitHub remote without gh blocks and names it | `test_a_github_remote_without_gh_blocks_and_names_it` | `ci_push::tests::parity::a_github_remote_without_gh_blocks_and_names_it` | pending |
| 64 | a failing gh blocks and names the failure | `test_a_failing_gh_blocks_and_names_the_failure` | `ci_push::tests::parity::a_failing_gh_blocks_and_names_the_failure` | pending |
| 65 | a non-GitHub remote never asks gh | `test_a_non_github_remote_never_asks_gh` | `ci_push::tests::parity::a_non_github_remote_never_asks_gh` | pending |
| 66 | a target updated in the same push is reviewed at its incoming revision | `test_a_target_updated_in_the_same_push_is_reviewed_at_its_incoming_revision` | `ci_push::tests::parity::a_target_updated_in_the_same_push_is_reviewed_at_its_incoming_revision` | pending |
| 67 | a simultaneous push with nothing prohibited passes in either order | `test_a_simultaneous_push_with_no_prohibited_work_passes_in_either_order` | `ci_push::tests::parity::a_simultaneous_push_with_no_prohibited_work_passes_in_either_order` | pending |
| 68 | a pull request into a branch this push deletes is refused | `test_a_pull_request_into_a_branch_this_push_deletes_is_refused` | `ci_push::tests::parity::a_pull_request_into_a_branch_this_push_deletes_is_refused` | pending |
| 69 | a docs-only follow-up skips the cells prior evidence covers | `test_a_docs_only_follow_up_skips_the_cells_prior_evidence_covers` | `ci_push::tests::parity::a_docs_only_follow_up_skips_the_cells_prior_evidence_covers` | pending |
| 70 | a follow-up changing a package's inputs reruns its cell | `test_a_follow_up_changing_a_packages_inputs_reruns_its_cell` | `ci_push::tests::parity::a_follow_up_changing_a_packages_inputs_reruns_its_cell` | pending |
| 71 | a cell whose prior evidence is a failure is rerun, superseded | `test_a_cell_whose_prior_evidence_is_a_failure_is_rerun_and_superseded` | `ci_push::tests::parity::a_cell_whose_prior_evidence_is_a_failure_is_rerun_and_superseded` | pending |
| 72 | a stacked pull request records its target tip as the base | `test_a_stacked_pull_request_records_its_target_tip_as_the_receipt_base` | `ci_push::tests::parity::a_stacked_pull_request_records_its_target_tip_as_the_receipt_base` | pending |
| D1 | dispatcher selects each linked worktree's hook (`.githooks/tests/test-pre-push-dispatcher.sh`, one script-level contract) | _(whole script)_ | `ci_push::tests::dispatcher::the_dispatcher_runs_each_linked_worktrees_own_hook` | pending |

## Spec Departures

Recorded here rather than in the spec, which stays a snapshot of what was
decided.

1. **Malformed constraint records fail closed (both readers).** The spec says
   no constraint schema change is intended. `constraints.py`'s `Constraint`
   accepts any truthy value for a required field and `str()`s it, reads an
   optional `branch: 7` or `gate: null` through `.get`, and so silently
   accepts numbers/collections for text or exempts a record whose `branch` is
   malformed. Both readers now apply one validator: required nonempty strings
   `environment`, `reason`, `owner`, `expiry`; optional strings `gate`,
   `repository`, `branch` (absent or `""` = wildcard, as before); explicit
   `null`, any non-string, and unknown fields are errors; `expiry` is a real
   `YYYY-MM-DD` date (no basic or week-date spellings), binding through that
   day. Canonical valid records and matching are unchanged; only inputs that
   were already wrong change outcome (they now bind as malformed). The store is
   unversioned, so no plan/receipt schema version moves; Phase 5 records this
   in `docs/cicd/schema-versions.md`. The Rust shape validator landed in
   Phase 1 (`parse_constraint_record`); the Python reader changes with the
   matching port in Phase 2.
2. **HEAD's first-context plan is separate state from scope recording.** The
   shell's `record_scope_evidence` returns before copying the plan when HEAD's
   first base is all zeros, so the gates of a branch-creating push to `main`
   replan instead of consuming the reviewed plan's identity. The port captures
   HEAD's first context's raw and evidence-applied plans before attempting
   scope recording, for every base. Scope recording (which an all-zero or
   non-ancestor base still cannot produce) and gate input selection become
   separate state; an all-zero base still cannot bind a receipt or validation
   comparison, so its gates replan for feedback. Implemented in Phase 3.

## Phase 1

### Freeze parity

- Parity inventory above: 72 Bash rows + dispatcher row D1, every row
  `pending`. Phase 1 tests already exercise the decision half of some rows
  (row 4 `off` alias, row 5 invalid mode, rows 50-51 deletion-/tag-only
  pushes) in `ci_push::tests::run`, but none of them crosses the gate or
  publication boundary the Bash case asserted, so they stay `pending`.
- Shared fixtures: `scripts/ci/fixtures/pre-push/` with `plans/` (the five
  `.githooks/tests/fixtures/plan-*.json` resolved plans, copied byte for byte
  so they outlive the shell suite), `constraints/minimal.json` (the shell
  suite's `write_prohibition` record) and `constraints/scoped.json` (every
  optional field), `pull-requests/two-open.json` and `none.json` (`gh pr list
  --json number,baseRefName` output), and a README. Every negative test edits
  exactly one field of a control in memory, so no mutated copies are stored.
  Path-inventory and evidence-scenario fixtures are left to the phases that
  read them (see `decisions_for_review`).
- Spec departures recorded above.

### Build foundation

- `scripts/Cargo.toml`: `[lib] path = "lib.rs"`; `[[bin]] ci-push`
  (`required-features = ["local-tools"]`); under `local-tools` the optional
  `chrono`, `dirs`, `git2` (no default features), `gix =0.90.0` (sha1,
  revision, status, blob-diff, dirwalk, excludes, parallel), `libgit2-sys`
  ceiling, `tempfile`, Unix `libc`, Windows `windows-sys` (Foundation,
  Security, Console, JobObjects, Threading); dev-dependencies
  `biscuit-test-harness` and `test-toolkit`. `Cargo.lock` gains edges only,
  no new package.
- `scripts/lib.rs`: gates `pub mod ci_push` on `local-tools` (a `[lib]`
  cannot declare `required-features`), so `--no-default-features` builds an
  empty library.
- `scripts/ci-push/mod.rs`: `PushRequest`, `PushServices`, `PushOutcome`,
  `Diagnostic`, `BlockReason`, `WithholdReason`, `ReviewContext`,
  `ContextSource`, `PythonCommand`, and `run`. `run` validates mode and
  selection before discovery, discovers the object format, parses the whole
  update list, names each skipped update, passes a push with nothing to
  review, and **blocks** any reviewable push with `ReviewUnavailable` until
  Phase 3 wires review in.
- `scripts/ci-push/mode.rs`: `PushMode::parse` (absent/empty = auto, `off` =
  deprecated scope-only, everything else including padding/case an error) and
  `parse_selection` (ASCII-whitespace names, no expansion; non-UTF-8 blocks).
- `scripts/ci-push/updates.rs`: `parse_updates` (LF/CRLF/no final newline,
  blank lines skipped but counted, exactly four tokens, lowercase hex OIDs of
  the repository's hash length, destination a valid full ref, duplicate
  destination malformed with both line numbers, non-UTF-8 line malformed) and
  `RefUpdate` classification (deletion, non-branch, branch names, event).
- `scripts/ci-push/json.rs`: the one strict loader (duplicate-key visitor,
  trailing content, empty document; serde_json already refuses comments and
  trailing commas) and the shared field validators.
- `scripts/ci-push/documents.rs`: `parse_pull_requests`,
  `parse_resolved_plan` (keeps the whole document), `parse_constraint_plan`
  (Python `read_plan`'s minimal view; absent `state` = `None`, present
  null/wrong type error), `parse_constraint_record`.
- `scripts/ci-push/host.rs`: `HostServices` (sniff discovery, gix object
  format). Lives in the library so the real discovery boundary is tested in
  process.
- `scripts/ci-push.rs`: reads args (empty name = `origin`, empty URL = none),
  env, and all of stdin, then prints `run`'s diagnostics and exits with its
  status. Not yet launched by the hook.

### Requirement-to-test mapping (all in `repo-deps` lib, L1)

| Requirement | Tests (`ci_push::tests::…`) |
| --- | --- |
| Loader syntax policy (duplicate key, comment, trailing comma, trailing garbage, second document, empty, whitespace), once | `documents::the_loader_refuses_*` |
| Required nonempty text, full matrix (absent, null, number, bool, array, object, `""`) | `documents::required_text_refuses_every_shape_but_a_nonempty_string` |
| Wiring: `reason`/`owner`/`expiry`, plan `base`/`head`, cell `package`/`environment`/`gate`/`execution`/`state`, PR `baseRefName`, constraint-view fields | `documents::every_required_constraint_field_is_wired_to_required_text`, `the_plan_comparison_is_wired_to_required_text`, `every_plan_cell_identity_field_is_wired_to_required_text`, `a_pull_request_base_is_wired_to_required_text`, `the_constraint_view_is_wired_to_the_array_and_text_validators` |
| Calendar date (invalid day/month, basic, week date, unpadded, time, padding; leap day) | `documents::expiry_must_be_a_real_yyyy_mm_dd_date`, `a_leap_day_is_a_calendar_date` |
| Optional wildcard text, full matrix through `gate`; wiring `repository`/`branch` | `documents::an_optional_field_is_a_wildcard_when_absent_or_empty`, `an_optional_field_refuses_null_and_every_non_string`, `every_optional_constraint_field_is_wired_to_the_wildcard_validator`, `the_scoped_control_record_reads_every_optional_field` |
| Unknown constraint field, non-object documents | `documents::a_constraint_record_refuses_an_unknown_field`, `a_constraint_record_must_be_an_object`, `a_resolved_plan_must_be_an_object` |
| Required arrays (absent, null, whole wrong, one wrong, all wrong, `[]`) | `documents::the_plan_cells_must_be_an_array_of_cells`, `one_wrong_cell_among_valid_cells_is_not_filtered_out`, `an_empty_cells_array_is_a_plan_with_no_cells`, `the_pull_request_list_is_wired_to_the_array_validator`, `an_empty_pull_request_list_means_no_open_pull_request` |
| Positive integer PR number (absent, null, 0, negative, float, string, bool) | `documents::a_pull_request_number_is_a_positive_integer` |
| Optional `state` in the constraint view (absent vs null vs wrong type vs `""`) | `documents::the_constraint_view_reads_absent_state_as_no_override`, `the_constraint_view_refuses_a_present_but_invalid_state` |
| Controls and fixture validity, unknown plan/gh fields kept/tolerated | `documents::the_constraint_control_record_parses`, `every_plan_fixture_is_a_valid_resolved_plan_that_keeps_unread_fields`, `the_executing_plan_reads_its_cells_in_order`, `the_pull_request_control_lists_each_number_and_base`, `a_pull_request_tolerates_fields_gh_adds` |
| Located diagnostics | `documents::a_document_error_names_the_document_and_the_field`, `updates::a_second_update_of_one_destination_is_malformed`, `run::a_malformed_line_blocks_the_whole_push_before_review` |
| Update framing (LF, CRLF, no final newline, blank lines, any ASCII whitespace, empty input) | `updates::every_line_ending_gives_the_same_updates`, `blank_and_whitespace_only_lines_are_ignored_but_counted`, `fields_may_be_separated_by_any_ascii_whitespace`, `empty_input_is_an_empty_push` |
| Malformed update lines (field count, OID length/case/hex/hash kind, short or invalid destination, non-UTF-8, duplicate destination) | `updates::a_malformed_line_names_its_line_number`, `a_crlf_malformed_line_is_quoted_without_its_line_ending`, `a_non_utf8_line_is_malformed`, `a_second_update_of_one_destination_is_malformed` |
| Classification (deletions, tags, notes skipped; renamed refspec; non-branch sources; event) | `updates::only_a_branch_update_is_reviewed`, `a_renamed_refspec_keeps_both_branch_names`, `a_source_that_is_not_a_branch_has_no_local_branch`, `main_triggers_a_push_and_every_other_branch_a_pull_request` |
| Mode before discovery; `off` deprecated; selection | `run::every_mode_spelling_is_read`, `off_is_a_deprecated_alias_of_scope_only`, `an_invalid_mode_blocks_before_discovery`, `a_readable_selection_never_blocks`, `an_unreadable_selection_blocks_before_discovery` (Unix) |
| Whole list before review; empty / skipped-only push passes; reviewable never passes unreviewed | `run::a_malformed_line_blocks_the_whole_push_before_review`, `a_push_of_only_deletions_and_tags_names_each_skip_and_passes`, `a_branch_update_is_never_passed_unreviewed`, `every_mode_spelling_is_read` |
| Real discovery boundary (sniff + gix on a temp repository) | `run::the_host_reads_a_repository_object_format_and_decides_a_tag_push`, `run::a_discovery_failure_is_an_error` |

Fidelity: F1 in-process at the library's public API for every decision;
F2 for the discovery boundary (a real Git repository on disk). No CLI spawn
in Phase 1: the binary is not reached by the hook yet and loads no document
itself (see `decisions_for_review`).

Code-smell sweep over the new parsers: no `#[serde(default)]`, no
`Option<T>` deserialization, no `filter_map`/`unwrap_or_default`/`.ok()` on a
load-bearing value. Display-only uses: `ModeError` renders an invalid value
with `unwrap_or_default().to_string_lossy()`; `parse_calendar_date`'s
`.parse().ok()?` runs only after every byte was checked to be a digit.

### Gates and evidence (macOS host, 2026-11-10)

- Validation checkpoint: `cargo check -p repo-deps --no-default-features
  --features local-tools --bin ci-push --lib`: clean, no warnings.
- Tier isolation: `cargo check -p repo-deps --no-default-features --bin
  ci-rollup --lib` and `--features build-tools --bin ci-plan --bin ci-build
  --lib` both pass; `cargo tree --no-default-features` lists only anyhow,
  quick-xml, serde, serde_json, toml (no gix/git2/sniff/chrono/tempfile/dirs).
- Focused: `cargo nextest run -p repo-deps --lib -E 'test(/^ci_push::/)'`:
  126 passed, 0.27 s.
- `just _lint repo-deps` (clippy `--all-targets -D warnings`): pass. Note
  `scripts/` has no justfile, so `just lint` from `scripts/` falls through to
  the root's all-areas lint; it was stopped and the package recipe used.
- `just test repo-deps`: 639 passed (2 slow), 1 skipped (pre-existing, not a
  `ci_push` test), exit 0.
- `just check-tier-coverage repo-deps`: nothing stranded.
- Scope calculator over exactly this phase's files (the worktree also holds
  unrelated uncommitted changes from other work): `packages: []`,
  `change_class: documentation`. Gates were run anyway because this phase lists
  source files.
- Dependents: no package depends on `repo-deps`; no dependent tests to run.
- Cross-OS (`just cross-check repo-deps --os windows`), one attempt: the
  native MSVC build of all 9 binaries **linked** (the `libgit2-sys` ceiling
  holds; no LNK2005), the archive was produced, and the L1 listing found 635
  tests. Execution then stopped in the rig's Cargo storage preflight (47.3 GiB
  free, 50 GiB required; automatic reclaim freed nothing). This is a rig
  capacity condition, not a test failure; `BISCUIT_BUILD_MIN_FREE_GIB=0` is an
  emergency-only override and was not used. Windows **test execution is
  unproven** for Phase 1; `os_failing` is not set because no test failed.
  The Windows lib-test link printed a `linker_messages` warning (`Creating
  library …repo_deps-….lib and object ….exp`), which is MSVC reporting that the
  test executable exports symbols; it is informational and does not affect
  clippy.
