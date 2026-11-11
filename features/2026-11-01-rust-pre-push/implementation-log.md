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
source_files_during_phase_2:
  - scripts/Cargo.toml
  - scripts/lib.rs
  - scripts/ci-plan.rs
  - scripts/ci-plan-render.rs
  - scripts/ci-plan-tests.rs
  - scripts/ci-change-inventory.rs
  - scripts/ci-push-test-tool.rs
  - scripts/ci-push/mod.rs
  - scripts/ci-push/documents.rs
  - scripts/ci-push/json.rs
  - scripts/ci-push/constraints.rs
  - scripts/ci-push/git.rs
  - scripts/ci-push/worktree.rs
  - scripts/ci-push/process.rs
  - scripts/ci-push/python.rs
  - scripts/ci-push/tests/mod.rs
  - scripts/ci-push/tests/constraints.rs
  - scripts/ci-push/tests/git.rs
  - scripts/ci-push/tests/worktree.rs
  - scripts/ci-push/tests/process.rs
  - scripts/ci-push/tests/python.rs
  - scripts/ci-push/integration/main.rs
  - scripts/ci-push/integration/fixture.rs
  - scripts/ci-push/integration/process.rs
  - scripts/ci-push/integration/python.rs
  - scripts/ci/constraints.py
  - scripts/ci/test_constraints.py
  - scripts/ci/fixtures/pre-push/constraints/parity.json
docs_updated_during_phase_2:
  - scripts/ci/fixtures/pre-push/README.md
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - scripts/ci-push.rs
  - scripts/ci-push/mod.rs
  - scripts/ci-push/review.rs
  - scripts/ci-push/evidence.rs
  - scripts/ci-push/host.rs
  - scripts/ci-push/tests/mod.rs
  - scripts/ci-push/tests/run.rs
  - scripts/ci-push/tests/orchestration.rs
  - scripts/ci-push/integration/main.rs
  - scripts/ci-push/integration/push.rs
  - just/ci-local.just
  - scripts/ci/test_ci_local.py
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
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
  - |-
    Fixture executable location. The plan asks for a compiled `ci-push-test-tool` but leaves open how tests find it. It is a `[[bin]]` (local-tools) plus a new integration test target `ci-push` (`scripts/ci-push/integration/`), because Cargo publishes `CARGO_BIN_EXE_*` (what `bin_exe!` reads, and what makes nextest archives work) only to integration tests. Process and Python tests live there; decision tests stay lib unit tests. Alternatives: locate the binary next to `current_exe()` or rebuild it with Cargo from the test, as `ci-build`'s tests do (fragile under archives, slower). Redo: move the integration tests and drop the `[[test]]` entry.
  - |-
    Shared renderer lives in the library. `ci-plan-render.rs` is `repo_deps::ci_plan_render` (build-tools), so `ci-push` (local-tools) calls it in-process. Its 26 tests moved from the `ci-plan` bin harness to the library (same names, new binary id). `ci-rollup` keeps its own `#[path]` include of `ci-change-inventory.rs` because it must not link the library's build-tools half. Alternative: include the render file in both binaries by `#[path]` (two compiled copies). Redo: move one `mod` line and the test module back.
  - |-
    Rename detection uses Git's built-in defaults (50%, no copies, limit 1000, empty files paired), not the user's `diff.renames` config; the shell inherited the user's config. Hosted CI uses the defaults, so the local plan now matches CI's. gix's similarity estimate can differ from Git's on pairs right at the 50% threshold or on ties between identical files. Alternative: `Options::from_configuration` (one line) to honor user config. Redo: that line plus its test.
  - |-
    Children that outlive their parent are killed. After a supervised child exits, anything left in its process group (Unix) or Job Object (Windows) is killed too; on Windows kill-on-close already did this, and Unix now matches it. A gate cannot leave a daemon running. Pipe readers get 500 ms after the tree ends, so a process that escaped the group cannot hang the hook (`Completed.incomplete` is set). Alternative: let descendants live on Unix (inconsistent with Windows, can hang on held pipes). Redo: `process.rs` reaping.
  - |-
    Windows details in `process.rs`: the thread-resume functions (Tool Help API) are declared locally rather than enabling the `Win32_System_Diagnostics_ToolHelp` windows-sys feature; a failed Job Object assignment (some SSH sessions forbid nested jobs) runs the child without tree ownership instead of failing; PATH lookup on Windows accepts only `name.exe` (no `.bat`/`.cmd`, which re-parse arguments through cmd.exe). Alternatives: add the feature to `scripts/Cargo.toml`; fail closed when the job cannot be assigned. Redo: local edits in `process.rs`/`python.rs`.
  - |-
    The two constraint readers now agree on two edge cases the shared validators did not decide: the unknown field reported is the alphabetically first one, and year `0000` is not a calendar date (Python's `date` cannot represent it). Both are surgical edits to Phase 1's `documents.rs`/`json.rs`. Redo: revert those two lines and drop the parity cases.
  - |-
    Session boundary shape. The plan's `run(request, services)` stays, but the review, gates, and publication cross a second trait, `PushSession`, reached through `PushServices::session(request, body)` (a callback, so the production session can hold `git::Remote`, which borrows the repository and its runner, for the whole invocation and keep its per-branch tip cache and fetch-once rule). `PushSession` exposes operations (remote tip, gh listing, run one Python helper, run the gates, publish the scope note, OS environment, home, today, cancellation), and the library builds every argument list, so the in-process fake in `tests/orchestration.rs` asserts exact helper argv. Alternative: a lower-level trait (raw process/git primitives) or a higher one (one method per Python step). Redo: `mod.rs` trait, `host.rs`, and the fake.
  - |-
    Exit statuses. Kept the shell's: a review failure (contexts, trees, helpers, Python missing) exits 2, a constraint refusal or an unreadable reviewed plan or store exits 1, an ordinary strict gate failure exits with the gate's status (clamped to 1-127), and an interruption exits 130/143/129 for the received signal, else the gate's own status when it is 128-255, else 130 (the gates' `interrupted` marker with an ordinary status). The marker now makes the run an interruption in every mode; the shell only withheld evidence for it and let `warn` pass. Redo: `BlockReason::exit_code`, `finish`.
  - |-
    HEAD's first context when several updates push HEAD. The shell recorded scope for the first context of EVERY update carrying HEAD, so the last such update won. The port takes the first update's first context only, as the plan's "other contexts never replace the first context" says. Redo: one condition in `review::review_context`.
  - |-
    Withholding order and two new checks. Reasons are checked in this order: interrupted (signal, exit >= 128, or marker), push does not carry HEAD, selection override, dirty at start, HEAD moved, dirty after the gates, incomplete output (a process outside the gates' tree held their pipes), no reviewed nonzero comparison, no local plan, empty manifest; then unsupported OS and retention failure. HEAD moving before the gates also makes them replan rather than consume the reviewed plan. A gate that cannot start (no `just`) is an ordinary failure with status 127, as the shell's missing command was, so `warn` lets it through. Redo: `evidence::withhold_reason`, `mod::run_gates`.
  - |-
    The reviewed plan is validated twice. After `--apply-to`, the reviewed plan is parsed with the full resolved-plan validator (identity fields of every cell) and then with the constraint reader's view; either failure refuses the push as a constraint failure (exit 1), matching the shell's `constraints.py check` on an unreadable plan. Alternative: only the constraint view (what the shell checked). Redo: one call in `review::review_context`.
  - |-
    Python selection happens once, before the first review, for every reviewable push including `scope-only`; a missing interpreter blocks with exit 2 and one message naming each candidate. Helper stderr is captured and relayed after the helper ends (not streamed), so it can be quoted in a failure. Redo: `review::review_push`, `Flow::helper`.
  - |-
    Presentation. Every diagnostic is written to stderr as it happens (the binary no longer prints a list at the end). Only the constraint refusal uses a terminal component (`UnorderedList`, items added literally so file paths are never parsed as markup); other messages are plain text, and the shell's red ANSI is not reproduced. The plan's rule 17 asks for `Prose`/`UnorderedList` for actionable diagnostics; Prose markup would need escaping of paths and user text, which biscuit-terminal does not offer for strings. Redo: `host::report`.
  - |-
    An unreadable remote configuration. When Git gives no URL and the remote's URL cannot be read, the identity stays unknown (matching every repository-scoped constraint record, the conservative side) rather than blocking. Redo: one `.ok()` in `host.rs`.
  - |-
    ci-local interpreter bridge (subagent decisions). `BISCUIT_CI_PYTHON_ARGS_FILE` without `BISCUIT_CI_PYTHON` is an error ("set both or neither"), because a prefix such as `-3` belongs to the launcher it was chosen with; `BISCUIT_CI_PYTHON` must be an executable regular file; a truncated final prefix record exits 2. The self-test suite `test_ci_local.py`'s own `tool()` helper launched `python3` and now uses `sys.executable`. Redo: the variable block at the top of the `ci-local` recipe.
message_to_agent: |-
  Phase 3 wired the whole flow into `ci_push::run`: review (`review.rs`), scope publication and validation (`evidence.rs`), gates (`mod.rs::run_gates`/`finish`), and the production `HostServices`/`HostSession` (`host.rs`). `ci-push.rs` installs the cancellation handlers and emits diagnostics live. `.githooks/pre-push` is still the shell hook: nothing launches `ci-push` yet (Phase 4 cuts over).
  Where the tests are: decision rows run in-process against the fake session in `scripts/ci-push/tests/orchestration.rs` (45 test functions, 66 cases: ledger order, contexts, gh failures, constraints, trees, gates, withholding, publication). Many parity-table rows map onto those directly; reuse its `Fake` (fields `fail`, `signal_during`, `gates`, `plans`, `gh`, ...) rather than writing another. A real-process end to end is `scripts/ci-push/integration/push.rs`: a git2-built checkout with a bare remote, stand-in `scripts/ci/affected_scope.py`/`local_evidence.py` committed in the fixture tree and run by the real selected Python, and the compiled `just` stub; it is the pattern for hermetic parity tests. `HostSession` resolves python/just/gh/git from the process `PATH`, so tests set `PATH`, `HOME`/`USERPROFILE`, `BISCUIT_CI_CONSTRAINTS_DIR`, and `BISCUIT_CI_EVIDENCE_DIR` on the spawned `ci-push`.
  The ci-local bridge is done (`BISCUIT_CI_PYTHON` + NUL-delimited `BISCUIT_CI_PYTHON_ARGS_FILE`, tests in `SelectedInterpreterTests` of `scripts/ci/test_ci_local.py`; that suite needs `BISCUIT_REQUIRE_JUST=1 BISCUIT_REQUIRE_JQ=1` to run its real-recipe classes and takes ~5 min in full).
  No docs changed in Phase 3 (Phase 5 owns them), but `docs/topics/ci-cd.md` must describe the new `BISCUIT_CI_PYTHON*` contract of `just ci-local`.
  Gates: `just test repo-deps` (821), `just _lint repo-deps`, `cargo nextest run -p test-toolkit --test ci_workflow_contracts` (it reads ci-local.just). See "## Phase 3" for the Windows cross-check result.
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

## Phase 2

### Wiring before concurrency (orchestrator)

- `scripts/Cargo.toml`: `[[bin]] ci-push-test-tool` (`ci-push-test-tool.rs`,
  local-tools) and `[[test]] ci-push` (`ci-push/integration/main.rs`,
  local-tools). An integration target is the only kind for which Cargo sets
  `CARGO_BIN_EXE_*`, so it is the only portable (archive-safe) way to locate
  the fixture executable through `biscuit_test_harness::bin_exe!`; Phase 4's
  launcher tests need the same target.
- `scripts/ci-push/mod.rs`: `pub mod constraints, git, process, python,
  worktree` (stubs), so the four Wave 2 tasks own disjoint files.
- `scripts/lib.rs`: `#[cfg(feature = "build-tools")] pub mod ci_plan_render`
  (`ci-plan-render.rs`), so `ci-plan` (build-tools) and `ci-push`
  (local-tools) share one renderer without `ci-push` spawning `ci-plan`.
- `scripts/ci-push/tests/mod.rs`: test modules for each new adapter.

Wave 2 ran as four concurrent subagents with disjoint file ownership; each
verified only its own slice, and the gates below ran once afterwards.

### Manage processes

- `scripts/ci-push/process.rs`: `ChildCommand` builder (native argv, null
  stdin by default, concurrent pipe drain, optional inherit, capture limit,
  deadline, cancellation token). `repository_independent()` removes the six
  `GIT_*` pinning variables; `clear_plan_input()` removes
  `BISCUIT_CI_PLAN_IN`; `GIT_TERMINAL_PROMPT=0`, `PYTHONDONTWRITEBYTECODE=1`,
  `PYTHONUTF8=1` are always set. Unix: own process group, the received signal
  forwarded to the group, 1 s grace, then `SIGKILL`. Windows: created
  suspended, assigned to a kill-on-close Job Object, then resumed
  (`CREATE_NEW_PROCESS_GROUP` so console Ctrl-C reaches only `ci-push`).
  `install_cancellation_handlers()` (`sigaction` INT/TERM/HUP storing the
  signal; `ctrlc` on Windows), `exit_code_for_cancellation()` 130/143/129,
  `is_interruption()` (exit >= 128 or killed by signal).
- `scripts/ci-push/python.rs`: `PythonCommand::select` / `select_with`
  (`python3`, Windows-only `py -3`, `python`; >= 3.10; 2 s probe, injectable
  for tests; one error naming every attempt), `.command()`,
  `.gate_environment(dir)` writing the NUL-delimited prefix file and returning
  `BISCUIT_CI_PYTHON` / `BISCUIT_CI_PYTHON_ARGS_FILE`.
- `scripts/ci-push-test-tool.rs`: the fixture executable (dispatch by file
  stem or `--scenario`; argv/env/cwd/stdin JSON-line log; exit codes, output,
  Python version answers, hangs, grandchild with readiness marker). Copied,
  never symlinked; `.exe` on Windows.
- Tests: integration target `ci-push` (22 tests, `process::*`, `python::*`)
  and lib `ci_push::tests::{process,python}` (34). Real-signal tests signal a
  fixture child that installs the production handlers, never the test
  process. A mutation check (process-group ownership removed) failed both
  tree-cleanup tests.

### Adapt repositories

- `scripts/ci-push/git.rs`: `Repository::discover` (sniff, then gix), HEAD
  identity, dirtiness (sniff), `peel_to_commit`/`has_commit`/`is_ancestor`,
  refs and notes reads, `changed_paths(base, head)` with Git's default rename
  detection, and `ChangedPaths::arguments_file()` byte-identical to
  `diff_scope.py`. Non-UTF-8 path is a located `NonPortablePath` error (the
  rule is compiled on every OS so it is tested here). `github_repo_of` port.
  Transport: `TransportCommand::{ls_remote, fetch, push}` only, behind a
  `CommandRunner` trait (`DirectRunner` default) so Phase 3 routes it through
  `process.rs`. `Remote` caches tips per branch and fetches at most once.
  `attach_note` writes the scope note through git2 (bytes match
  `git notes add -F`, including its whitespace cleanup).
- `scripts/ci-push/worktree.rs`: `RuntimeWorktree::create` (git2, uniquely
  named temporary branch removed once the worktree is detached at the commit),
  `remove()` / `Drop` touching only this invocation's worktree, record and
  ref; `CleanupFailure` lists each retained artifact with its exact removal
  command.
- Tests: lib `ci_push::tests::{git,worktree}` (43), real repositories in temp
  dirs with `git diff --name-status -z` and `diff_scope.py` as oracles.

### Validate constraints

- `scripts/ci-push/constraints.rs`: `repository_identity`, `store_segments`,
  `store_root(home)`, `default_directory`, `resolve_directory(explicit,
  variable, default)`, `load` (recursive, path-sorted; missing or non-directory
  store empty; unreadable file or directory blocks; malformed records bind
  everywhere), `Constraint::{applies_to, expired, describe, is_malformed}`,
  `select`, `read_plan` (through `parse_constraint_plan`), `unsatisfied`.
  Built on Phase 1's `parse_constraint_record` / `parse_constraint_plan`.
  Two surgical edits to shared validators so both readers agree: the unknown
  field named is the alphabetically first one, and year `0000` is refused
  (Python's `date` has no year 0).
- `scripts/ci/constraints.py`: duplicate-key-rejecting loader, required
  nonempty strings, optional wildcard strings (null/non-string error),
  unknown fields error, strict `YYYY-MM-DD` expiry, strict `read_plan` (cells
  array; nonempty `environment`/`gate`/`execution`; `state` nonempty when
  present). Command surface unchanged.
- `scripts/ci/fixtures/pre-push/constraints/parity.json`: shared cases (URL
  spellings, wildcard fields, expiry yesterday/today/tomorrow, recursive
  stores, duplicate keys, malformed and unknown-field records, every cell
  outcome); `every_identity_case_in_the_parity_fixture_agrees` and
  `every_store_case_in_the_parity_fixture_agrees` (Rust) and the matching
  `test_constraints.py` tests run every case with an injected date.
- Tests: lib `ci_push::tests::constraints` (14); `test_constraints.py` 48.

### Share presentation

- `scripts/ci-plan-render.rs` (`repo_deps::ci_plan_render`, build-tools):
  `render_document(bytes, term)`; owns the `Plan` types, renderer, and the
  `ci-change-inventory.rs` include (ci-rollup keeps its own include, since it
  cannot link the library's build-tools half). `scripts/ci-plan.rs` is the
  thin CLI; output, stderr, and exit codes compared byte for byte against the
  old binary for all five plan fixtures, three widths, and four error cases.
- The 26 ci-plan tests moved from `repo-deps::bin/ci-plan tests::<name>` to
  lib `ci_plan_render::tests::<name>` (names unchanged) plus
  `render_document_matches_rendering_the_parsed_plan` and
  `render_document_rejects_malformed_json`.
- Stale commentary fixed: the module now says build-tools; the moved spec
  link uses the `2026-09-11-cicd-cleanup` form; the `[lib]` comment in
  `scripts/Cargo.toml` names both modules.

### Requirement-to-test mapping (Phase 2)

| Requirement | Tests |
| --- | --- |
| Native argv, null stdin, concurrent drain, bounded capture, inherit | `ci-push` integration `process::arguments_reach_the_child_as_separate_native_values`, `stdin_is_null_so_the_child_reads_end_of_file`, `large_stdout_and_stderr_are_drained_concurrently`, `a_capture_limit_keeps_the_head_and_still_drains_the_rest`, `inherited_output_is_not_captured` |
| Narrow sanitization | `process::a_repository_independent_child_loses_only_the_pinning_variables`, `pinning_variables_and_the_plan_input_stay_unless_removal_is_asked_for` |
| Tree ownership, deadline, cancellation, exit codes, interruption | `process::a_deadline_kills_the_whole_tree_and_keeps_the_output_read_so_far`, `cancellation_kills_the_whole_tree_and_maps_to_the_signal_exit_code`, `a_handled_signal_ends_the_supervised_tree_with_the_signal_exit_code` (Unix ×3), `a_child_exit_of_128_or_more_is_an_interruption` (×3), `a_pipe_held_outside_the_owned_tree_is_abandoned_after_a_grace` (Unix) |
| Python selection and fallback | `python::the_first_candidate_that_proves_itself_is_selected`, `a_broken_python3_falls_back_to_python`, `an_old_python_is_rejected_and_named_in_the_one_error`, `a_failed_probe_reports_its_status_and_stderr`, `a_probe_that_does_not_answer_is_killed_at_the_injected_timeout`, `the_selected_prefix_starts_every_python_invocation`, `py_dash_3_is_tried_after_python3_and_before_python` (Windows); lib `ci_push::tests::python::*` (version parsing, prefix bytes, PATH lookup, all missing) |
| Committed two-tree diff and arguments file | `ci_push::tests::git::renames_with_and_without_edits_match_git`, `a_copy_is_an_addition_of_its_destination`, `a_deletion_is_changed_and_deleted`, `an_empty_file_is_renamed_by_identity_as_git_renames_it`, `a_renamed_directory_reports_each_file_and_a_file_replaced_by_a_directory_is_deleted`, `a_type_change_and_a_mode_change_are_changes`, `spaces_and_unicode_survive_verbatim`, `newlines_and_non_utf8_bytes_in_names_survive_on_unix`, `the_arguments_file_matches_diff_scope_py`, `an_empty_diff_still_ends_its_arguments`, `a_path_windows_cannot_carry_is_a_located_error` |
| HEAD, objects, ancestry, dirtiness | `git::head_identity_on_a_branch_detached_and_in_a_linked_worktree`, `an_unborn_head_and_a_directory_outside_any_repository_are_errors`, `peeling_reports_missing_objects_and_non_commits`, `ancestry_follows_history_not_order`, `dirtiness_counts_untracked_files_but_not_ignored_ones` |
| Remote tips, notes, GitHub URL spellings | `git::remote_tips_are_read_from_the_remote_fetched_once_and_cached`, `an_advertised_tip_still_missing_after_one_fetch_is_unfetchable`, `a_scope_note_matches_git_notes_add_and_is_pushed`, `github_remotes_are_recognized_exactly_as_the_shell_did` |
| Runtime worktrees and owned cleanup | `ci_push::tests::worktree::*` (7) |
| Constraint matching, store, expiry, parity | `ci_push::tests::constraints::*` (14) and `test_constraints.py` (48) over `constraints/parity.json` |
| Shared renderer | `ci_plan_render::tests::render_document_matches_rendering_the_parsed_plan`, `render_document_rejects_malformed_json`, plus the 26 moved ci-plan tests |

Fidelity: F2 for processes (real child processes of a compiled fixture),
repositories (real Git repositories, Git CLI as oracle), and the constraint
store (real directories); F1 for the constraint and renderer decisions.

Code-smell sweep of the new parsers: no `#[serde(default)]`. Remaining
`unwrap_or_default` / `.ok()` uses are display-only or end in a located
error: `Remote`'s unknown URL is `""` only in messages; `parse_ls_remote`
returning `None` becomes the `Unreadable` error; `parse_version`'s `.ok()`
becomes a rejected candidate; the worktree name's `unwrap_or_default` reads a
tempfile name that is always alphanumeric; the cleanup report's
`Repository::open(..).ok()` only decides whether to list the temporary
branch (if the repository cannot be opened the report still lists the
directories).

Formatting: no `cargo fmt`. Two subagents ran `rustfmt` on their own new
files only (`process.rs`, `python.rs`, `ci-push-test-tool.rs`, integration
files, `git.rs`, `worktree.rs`, their tests); no pre-existing file was
reformatted.

### Gates and evidence (macOS host, 2026-11-10)

- Focused, per task: `cargo nextest run -p repo-deps --test ci-push` 22/22;
  lib `ci_push::tests::(process|python)` 34/34, `(git|worktree)` 43/43,
  `(constraints|documents)` 90/90; ci-plan/renderer 28/28.
- Python companions (Python 3.12.5, `scripts/ci/suite_runner.py`):
  `test_constraints.py` 48 OK, `test_ci_local.py` 104 OK,
  `test_affected_scope.py` 492 OK (the two suites that read `constraints.py`).
- `just _lint repo-deps`: pass. `just check-tier-coverage repo-deps`: nothing
  stranded.
- Tier isolation: `cargo check -p repo-deps --no-default-features --bin
  ci-rollup --lib` and `--features build-tools --bin ci-plan --bin ci-build
  --lib`: both clean.
- `just test repo-deps`: 754 passed (1 slow: the pre-existing
  `ci-build archive::tests::the_canonical_tier_recipes_run_the_fixture_archive_without_rebuilding_it`,
  7.4 s), 1 skipped (pre-existing, not `ci_push`), exit 0.
- Scope calculator over this phase's working-tree files: `packages:
  ["repo-deps", "test-toolkit"]`, `change_class: package`. test-toolkit is
  selected as a changed test input (its `ci_workflow_contracts` tests read
  `scripts/Cargo.toml`): `cargo nextest run -p test-toolkit --test
  ci_workflow_contracts` 228/228. No package depends on repo-deps.
- Cross-OS: `just cross-check repo-deps --os linux` (build-linux): 754
  passed, 1 skipped. `just cross-check repo-deps --os windows` (build-win), one
  attempt: the native MSVC build of the library, `ci-push`,
  `ci-push-test-tool`, and the `ci-push` integration target compiled and
  linked and the archive was produced; execution then stopped at the rig's
  Cargo storage preflight (46.9 GiB free, 50 GiB required), the same capacity
  condition as Phase 1. No test failed, so `os_failing` is not set, but
  Windows **execution is unproven** for Phase 2 (Job Object, suspended
  resume, `py -3`, worktree cleanup). The emergency override was not used.
- Note on how Wave 2 ran: the four subagents were started in the background
  rather than the foreground; the orchestrator waited for every one to finish
  and verified the constraints slice itself (its agent stopped without a final
  report, with its work complete and green) before running the gates.

## Phase 3

Wave 3 ran the "Bridge interpreter" task as one subagent (owning only
`just/ci-local.just` and `scripts/ci/test_ci_local.py`) while the
orchestrator wrote the Rust review; Wave 4 (gates, then publication) was
written sequentially by the orchestrator. The gates below ran once, after
every change was in.

### Review contexts (`scripts/ci-push/review.rs`)

- `review_push` reads HEAD and the checkout's cleanliness once, selects
  Python once (a missing interpreter blocks with exit 2 before any review),
  then reviews every branch update in Git's order. `pushes_head` counts only
  reviewed branch updates.
- `resolve_contexts` ports `resolve_trigger_contexts`: main uses its
  supplied previous tip (all zeros → `--all`; a tip absent after one fetch of
  `main` blocks); other branches list open pull requests with `gh` on a
  GitHub URL (missing `gh`, a failing `gh`, and unreadable output each block
  with their own message; an unrecognized github.com spelling fails closed),
  one context per pull request in `gh`'s order, current tip before a distinct
  incoming revision of a base this push also writes; a base this push deletes
  blocks; a base this push creates has only its incoming state; no open pull
  request or a non-GitHub URL reviews provisional contexts against the
  remote's `main`. Remote tips go through the session's one `git::Remote`, so
  each branch is listed and fetched at most once per invocation.
- The checkout is the reviewed tree only for HEAD on a clean checkout;
  otherwise a temporary detached worktree is materialized for the update and
  released after its last context, on success and on failure.
- `review_context` per context, in a `context-N/` staging directory:
  planner (`--event`, `--plan-out`, `--base`, `--head`, then `--all` or the
  Rust-written NUL-delimited `--arguments-file`; no constraint store) →
  `local_evidence.py verify --cells` → `affected_scope.py --apply-to` →
  render (display only) → full resolved-plan validation → constraint check
  (store from `BISCUIT_CI_CONSTRAINTS_DIR` or `<home>/.rusty-biscuit/...`,
  matched under the pushed URL and the remote plus renamed local branch name;
  expired records are reported; malformed records bind). Helper failures name
  the revision, base, exact command, exit status, and stderr.
- HEAD's first context (first update carrying HEAD, first context) is
  captured as `FirstContext { base, raw_plan, review_plan }` before scope
  recording, for every base including all zeros (spec departure 2), then
  `scope-record` runs for a nonzero base; a refused recording is advisory and
  leaves the captured plan usable by the gates.
- The ledger is `PushOutcome::reviews` (`ContextReview`: context, trigger,
  tree source, branch names, status), appended as each context starts and
  marked `Passed` when it ends.

### Bridge interpreter (`just/ci-local.just`, subagent)

- The `ci-local` recipe builds one `python` argv array: `BISCUIT_CI_PYTHON`
  plus the NUL-delimited prefix read with `IFS= read -r -d ''` from
  `BISCUIT_CI_PYTHON_ARGS_FILE`, else `python3` (standalone default, with its
  old "python3 is required" check). A truncated final record exits 2; an args
  file without an interpreter is an error. All nine `python3` calls in the
  recipe's path (constraints directory, `--apply-to` ×2, `--all` planner,
  `diff_scope.py`, working-tree planner, `verify`, self-test loop,
  `companion_suites.py`) use the array; the separate `ci-evidence` recipe is
  untouched.
- `test_ci_local.py`'s `tool()` helper launched `python3` itself and now
  uses `sys.executable`.
- New `SelectedInterpreterTests` (each puts a `python3` stub that logs and
  exits 9 first on `PATH`): fed-plan path and working-tree replan path both
  run every helper through a recording `BISCUIT_CI_PYTHON` with a prefix
  containing a space, the broken `python3` never launches; truncated args file
  fails and runs nothing; empty args file adds no prefix; no variables keeps
  `python3`. Four of the five fail against the pre-change recipe.

### Run gates (`scripts/ci-push/mod.rs`)

- After every review passes: scope publication, then a cancellation check,
  then `auto` → `strict` when a reviewed update starts a run, otherwise "No
  gate runs" and exit 0; `scope-only` exits 0.
- `run_gates` sets `BISCUIT_CI_SCOPE_OUT`, `BISCUIT_CI_PLAN_OUT`,
  `BISCUIT_CI_REPORTS_OUT`, and `BISCUIT_CI_PLAN_IN` (only for a clean,
  unoverridden checkout whose HEAD has not moved and whose first context has a
  nonzero base) exactly once; the host session clears an inherited
  `BISCUIT_CI_PLAN_IN`, removes the six repository-pinning variables, adds
  `BISCUIT_CI_PYTHON`/`_ARGS_FILE`, splits the selection into separate
  arguments, and lets the gates' output reach the terminal.
- `finish`: a signal, a gate status of 128 or more, or the gates'
  `interrupted` marker blocks in every mode with the signal's (or the gate's)
  status; an ordinary failure blocks `strict` with its status and passes
  `warn` with the `--no-verify` hint.

### Publish evidence (`scripts/ci-push/evidence.rs`)

- `publish_scope`: attach (git2) and push (Git CLI) HEAD's receipt on
  `refs/notes/ci-local/scope` after the full review; failures are advisory.
- `publish_validation`: the withholding order recorded in
  `decisions_for_review`; then the CI environment (`ci_environment`: macOS,
  Windows, WSL2 by a Microsoft kernel, other Linux), retention of the staged
  reports into `<evidence root>/<head>/<environment>` (overlaying a retry's
  directory; a relative root resolves against the checkout), `record-cells`
  against the first context's raw plan and base, and `publish-receipt`
  (status 0 published, 4 recorded locally, anything else advisory). A
  retention failure keeps the staging directory and prints its path.

### Requirement-to-test mapping (Phase 3)

All lib tests are `ci_push::tests::orchestration::*` (F1: in-process at
`run`, every session boundary faked and recorded); the end to end is the
`ci-push` integration target's `push::*` (F2: real repository, bare remote,
real Python, real Git transport, compiled `just` stub, the `ci-push` binary).

| Requirement | Tests |
| --- | --- |
| Update order, skipped updates, own path set per update | `updates_are_reviewed_in_git_order_and_skipped_updates_are_not` |
| Main push base, fetch-once failure, all-zero full scope | `a_push_to_main_is_compared_with_the_remote_previous_tip_and_starts_a_run`, `a_remote_main_tip_that_cannot_be_fetched_blocks`, `a_branch_creating_push_of_main_plans_full_scope_and_its_gates_replan` |
| Pull-request contexts, multi-base, created/deleted base | `every_open_pull_request_is_reviewed_in_gh_order`, `a_base_this_push_also_updates_is_reviewed_at_both_states`, `a_base_created_by_the_same_push_has_only_its_incoming_state`, `a_push_deleting_an_open_pull_requests_base_is_refused` |
| gh missing / failing / unreadable / invalid number, every mode | `a_pull_request_lookup_that_fails_blocks_every_mode` (4 cases × 3 modes) |
| GitHub URL spellings, provisional plans | `an_unrecognized_github_url_fails_closed`, `a_non_github_remote_never_asks_gh_and_gets_a_provisional_plan`, `a_github_branch_with_no_open_pull_request_gets_a_provisional_plan`, `a_provisional_base_that_cannot_be_read_blocks` |
| Planner arguments, arguments file, no constraint store, plan threading | `the_planner_gets_the_committed_arguments_file_and_no_constraint_store`; F2 `push::a_strict_push_to_main_reviews_publishes_scope_and_hands_the_gates_the_reviewed_plan` (real gix diff bytes) |
| Helper failures name revision/command; later-context failure publishes nothing, runs no gate | `a_helper_failure_blocks_naming_the_revision_and_the_command` (3), `a_later_context_failure_publishes_nothing_and_runs_no_gate` |
| Python missing | `python_that_cannot_be_found_blocks_before_any_review` |
| Committed trees (non-HEAD, dirty, clean in place, released on failure) | `a_revision_that_is_not_the_clean_checkout_is_reviewed_in_a_temporary_tree` (2), `the_clean_checkout_is_planned_in_place`, `a_temporary_tree_is_released_when_its_review_fails` |
| Interruption during review | `a_signal_during_review_blocks_with_its_status_and_runs_nothing_after` |
| Constraints: every mode, reused/absent, expired, renamed refspec, repository identity, malformed, unreadable plan | `an_executing_prohibited_cell_blocks_every_mode` (4), `a_reused_or_absent_prohibited_environment_does_not_block` (2), `an_expired_prohibition_is_reported_and_does_not_block`, `a_renamed_refspec_binds_a_record_under_either_branch_name` (3), `the_repository_identity_is_the_remote_being_pushed_to` (2), `a_malformed_record_binds_and_names_its_field`, `an_unreadable_reviewed_plan_blocks_even_with_an_empty_store` |
| Scope publication timing, dirty checkout, advisory failure, refused recording | `scope_is_published_after_every_review_and_before_auto_decides`, `a_dirty_checkout_still_publishes_its_committed_scope`, `a_failed_scope_publication_is_named_and_leaves_the_exit_code_alone`, `a_refused_scope_recording_still_feeds_the_reviewed_plan_to_the_gates`; F2 note on the bare remote |
| Gate input and validation | `a_clean_head_feeds_its_reviewed_plan_to_the_gates_and_publishes_validation`, `an_override_or_dirty_checkout_replans_and_publishes_no_validation` (2), `a_push_that_does_not_carry_head_reviews_its_revision_and_withholds_validation`; F2 (`BISCUIT_CI_PLAN_IN` replaces an inherited value, interpreter variables, no `GIT_DIR`) |
| Strict/warn/interruption | `a_strict_failure_publishes_its_complete_evidence_then_blocks`, `warn_lets_an_ordinary_failure_through_and_still_publishes_it`, `an_interrupted_run_blocks_even_in_warn_and_withholds_validation` (3 × 2 modes) |
| Withholding reasons | `a_run_that_is_not_the_outgoing_trees_withholds_validation` (4), `a_host_without_a_ci_environment_withholds_validation` |
| Retention, overlay, relative root, retention failure, publication statuses | `retry_reports_overlay_the_durable_directory`, `a_relative_evidence_root_resolves_against_the_checkout`, `reports_that_cannot_be_retained_publish_no_receipt_and_stay_readable`, `a_publication_failure_is_advisory_and_keeps_the_reports` (3) |
| OS → environment | `the_environment_follows_the_host_os` |
| ci-local interpreter bridge | `SelectedInterpreterTests` (5) in `scripts/ci/test_ci_local.py` |

`tests/run.rs`'s `a_branch_update_is_never_passed_unreviewed` (which pinned
Phase 1's `ReviewUnavailable` stub) is replaced by
`a_branch_update_opens_a_review_session_after_reporting_every_skip`.

Input robustness: Phase 3 adds no new loader. It wires the existing ones:
`gh` output through `parse_pull_requests` (control: `TWO_OPEN`; invalid
shapes: an object instead of an array, `number: 0`), the reviewed plan
through `parse_resolved_plan` then `parse_constraint_plan` (control: every
plan fixture; invalid: `cells: 7`), and constraint records through
`constraints::load` (control: `minimal`-shaped record; invalid: `branch: 7`).
Smell sweep of the new code: `.ok()` appears only where a failure takes the
conservative branch and is commented (unreadable remote URL → unknown
identity; undetectable OS → no environment, validation withheld; HEAD
unreadable before the gates → replan); `is_dirty().unwrap_or(true)` withholds
on a failed status read; the receipt's cell count is display-only.

### Gates and evidence (macOS host, 2026-11-10)

- Focused: `cargo nextest run -p repo-deps --lib -E 'test(/^ci_push::/)'`
  283 passed; `cargo nextest run -p repo-deps --test ci-push` 23 passed (the
  new end to end takes ~1.1 s).
- `test_ci_local.py` (subagent, `BISCUIT_REQUIRE_JUST=1
  BISCUIT_REQUIRE_JQ=1`, Python 3.12): 109 ran, 109 passed, 0 skipped.
- `just _lint repo-deps`: pass. `just check-tier-coverage repo-deps`: nothing
  stranded.
- `just test repo-deps`: 821 passed (1 slow: the pre-existing
  `ci-build archive::tests::the_canonical_tier_recipes_run_the_fixture_archive_without_rebuilding_it`,
  7.7 s), 1 skipped (pre-existing).
- Scope calculator over the working tree (which still holds Phase 2's
  uncommitted files as well): `packages: ["repo-deps", "test-toolkit"]`,
  `change_class: package`. test-toolkit's `ci_workflow_contracts` reads
  `just/ci-local.just`: `cargo nextest run -p test-toolkit --test
  ci_workflow_contracts` 228 passed. No package depends on repo-deps.
- Cross-OS: `just cross-check repo-deps --os linux` (build-linux): 821
  passed, 1 skipped, including the new `push::*` end to end with the rig's
  real Python and Git. `just cross-check repo-deps --os windows` (build-win),
  one attempt: the MSVC build compiled and linked every target and the L1
  listing found 808 tests, then execution stopped at the rig's Cargo storage
  preflight (46.8 GiB free, 50 GiB required; the 80 GB artifact sweep freed
  nothing), the same capacity condition as Phases 1 and 2. No test failed, so
  `os_failing` is not set, but Windows **execution is still unproven** for
  Phases 1–3 (Job Object, `py -3`, worktree release, the new host session and
  end to end). The emergency override was not used.
- Formatting: no `cargo fmt` or `rustfmt` was run; the lint recipe's
  check-only formatter passed.
