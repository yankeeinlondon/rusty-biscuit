---
kind: log
feature: 2026-11-01-rust-pre-push
review: 2026-11-01-rust-pre-push/review-1.md
implementation_1: "2026-11-11T00:47:17-08:00"
implementation_2: "2026-11-11T02:31:09-08:00"
implementation_3: "2026-11-11T04:11:19-08:00"
implementation_4: "2026-11-11T05:39:57-08:00"
---

# Review Repair Log: Rust Pre-push

## Implementation of Review Findings #1

> **started at:** 2026-11-11T00:47:17-08:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/rusty-biscuit/features/2026-11-01-rust-pre-push/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- the review's `implemented` flag was confirmed `false` before repairs started
- the review has no `human_in_the_loop` items and no blocked findings; both findings are repairable in this cycle

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R1-F1 | Checkout reuse relies on stale cleanliness and HEAD snapshots | fixed | All required children fixed or clean; sibling sweep R1-F1.8–R1-F1.11 clean |
| R1-F1.1 | `review_push` initial capture: clean control, initial snapshot appropriate | clean | Snapshot now only records starting identity and an independent withhold reason; it no longer authorizes tree selection or plan reuse |
| R1-F1.2 | `review_update` checkout selection: dirty during interpreter probe must use a detached committed tree | fixed | `review_update` re-reads state via `checkout_departure`; `a_checkout_that_changes_before_tree_selection_is_not_reused::case_1_dirtied` passes (Temporary tree, scope Published for committed tree); failed against pre-fix code |
| R1-F1.3 | `review_update` identity selection: HEAD moved after capture must not reuse checkout | fixed | `…::case_2_head_moved` passes (Materialize(HEAD), Temporary); unreadable HEAD/status cases `case_3`/`case_4` take a conservative temporary tree; all failed pre-fix |
| R1-F1.4 | `run_gates` plan selection: dirty during scope publication must replan and withhold validation | fixed | `run_gates` refreshes `is_dirty()` (unreadable = dirty); `a_checkout_that_changes_before_the_gates_replans_and_publishes_no_validation::case_1_dirtied`/`case_2_unreadable_status` pass: no `BISCUIT_CI_PLAN_IN`, no `record-cells`, `Withheld(FinalDirty)`; failed pre-fix |
| R1-F1.5 | `run_gates` identity selection: HEAD change before gates rejects plan reuse | clean | Now tested: `…::case_3_head_moved` and `case_4_unreadable_head` give `Withheld(HeadChanged)`; passed before and after the fix (existing check) |
| R1-F1.6 | `withhold_reason` final checks: current HEAD and cleanliness | clean | `withhold_reason` reads current HEAD and `is_dirty().unwrap_or(true)` after gates; existing `a_run_that_is_not_the_outgoing_trees_withholds_validation` cases pass |
| R1-F1.7 | Initial dirty/non-HEAD review and gate controls | clean | Existing initially-dirty, non-HEAD, override, and in-place controls pass in the 322-test lib run |
| R1-F1.8 | Sibling: `evidence::publish_scope` on scope-only exits (no final recheck) | clean | Publication reads only staged receipt bytes computed from the refreshed `Target.root`; the checkout is used only when confirmed clean at the commit at selection time (asserted by R1-F1.2) |
| R1-F1.9 | Sibling: `review_context` first-context choice and `record_scope` `review.head` gate | clean | `review.head` is used only as identity (which update carries HEAD, what the receipt binds to), not as tree authority |
| R1-F1.10 | Sibling: helper `verify`/`apply`/`record_scope` working directory | clean | Scripts come from `target.root`; the checkout is only a cwd for repository-level reads with explicit `--base`/`--head` |
| R1-F1.11 | Sibling: `pushes_head` from `update.local_oid == review.head` | clean | Identity only; current HEAD is rechecked before (R1-F1.5) and after (R1-F1.6) gates |
| R1-F2 | Real recipe receipt and retry coverage is excluded from Windows | unresolved | Every required child is fixed or clean and passes on macOS and Windows; one discovered defect remains open: R1-F2.27 (Windows duration over the 5 s L1 budget) |
| R1-F2.1 | `an_advanced_non_main_target_reuses_prior_cells_and_publishes_exact_receipts` compiles on Windows | fixed | Guard removed; passes on macOS and Windows (10.5 s alone, 19.1 s under load) |
| R1-F2.2 | `a_docs_only_follow_up_skips_the_cells_prior_evidence_covers` compiles on Windows | fixed | Guard removed; passes on macOS and Windows (10.5 s alone, 19.3 s under load) |
| R1-F2.3 | `a_follow_up_changing_a_packages_inputs_reruns_its_cell` compiles on Windows | fixed | Guard removed; passes on macOS and Windows (10.7 s alone, 19.1 s under load) |
| R1-F2.4 | `a_cell_whose_prior_evidence_is_a_failure_is_rerun_and_superseded` compiles on Windows | fixed | Guard removed; passes on macOS and Windows (12.3 s alone, 21.8 s under load) |
| R1-F2.5 | `a_stacked_pull_request_records_its_target_tip_as_the_receipt_base` compiles on Windows | fixed | Guard removed; passes on macOS and Windows (5.7 s alone, 10.1 s under load) |
| R1-F2.6 | `a_real_recipe_interruption_withholds_earlier_reports` compiles on Windows | fixed | Guard removed; all three cases pass on macOS and Windows (4.4–4.6 s alone, about 8.3 s under load) |
| R1-F2.7 | Unix signal-injection tests stay platform-specific; portable cancellation coverage retained | clean | Kept Unix-only (`libc::kill`); portable cancellation coverage passes on Windows: `parity::a_gate_exiting_130_never_publishes_staged_evidence`, `process::cancellation_kills_the_whole_tree_and_maps_to_the_signal_exit_code`, R1-F2.6 cases |
| R1-F2.8 | Git Bash and PowerShell launcher tests (clean launcher coverage) | clean | `windows_git_bash_…` and `windows_powershell_…` launcher tests pass on Windows; no change |
| R1-F2.9 | Real recipe consumes fallback interpreter executable and prefix (broken `python3`, selected `py -3` on Windows) | fixed | New `parity::a_real_recipe_runs_its_helpers_under_the_selected_fallback_interpreter`: broken `python3`; fallback `py -3` on Windows, `python` elsewhere; the real recipe ran `constraints.py directory` and `affected_scope.py --apply-to` with the prefix and recorded validation; a `python=(python3)` recipe mutation failed it; passes on macOS (2.3 s) and Windows (5.7 s alone) |
| R1-F2.10–R1-F2.23 | Sibling sweep: every other platform guard in `scripts/ci-push/integration/` and `scripts/ci-push/tests/` (dispatcher launch, zombie reap, process groups, signal handlers, `process_gone`, launcher helpers, `py -3` ordering, `planner_hangs`/`spawn`, byte names and mode bits in `tests/git.rs`, `tests/process.rs`, permission bits in `tests/constraints.rs`/`tests/worktree.rs`, execute bit in `tests/python.rs`, `tests/orchestration.rs`) | clean | Each guards a Unix-only API or has a Windows twin |
| R1-F2.24 | Sibling: `tests/run.rs` `an_unreadable_selection_blocks_before_discovery` guarded portable behavior | fixed | Builds a non-Unicode selection per OS (unpaired surrogate on Windows); passes on macOS and Windows |
| R1-F2.25 | Discovered on Windows: `tests/worktree.rs` detached-worktree test read `first\r\n` | fixed | Shared Git fixture persists `core.autocrlf = false` in `.git/config` (its `-c` never reached production `git worktree add`); passes on Windows |
| R1-F2.26 | Discovered on Windows: `ci-build` `archive::tests::one_owner_tree_shares_a_dependency_compile_without_unifying_features` (`command not found`) | fixed | Test passed `CI_BUILD_BIN=C:\…\ci-build.exe`; Bash looks up a word without `/` on `PATH`; now passes `C:/…`; production never sets the variable; fixed on attempt 1 of 3; passes on Windows (7.45 s alone; it was already over 5 s before this cycle because it runs real Cargo builds) |
| R1-F2.27 | Discovered: real-recipe tests exceed the 5 s L1 budget on Windows | unresolved | Profiled on `build-win`: about 110 process starts per push at 25–35 ms (about 3.7 s floor per push) plus 0.7 s setup; hard-linking stubs removed per-stub Defender scans (serial 73.4 s → 68.9 s), but double-push tests stay about 10.5–12.3 s alone and 19–22 s under load (under nextest's 30 s stop); even single-push tests are 5.7 s alone. Options for the author: seed the first receipt with a stub gate (about 2.2 s each; equivalence to the recipe's plan bytes unproven), batch `jq` in `just/ci-local.just` and `rev-parse` in helpers (about 0.5–0.7 s per push), or accept a Windows-specific budget for real-recipe tests; no timeout overrides were added |

### Progress

- starting the work on 'R1-F1: Checkout reuse relies on stale cleanliness and HEAD snapshots' at 00:47:49
        - defect class: a repository-state snapshot taken early in the push is reused to authorize a later decision (tree source, plan reuse, publication) against a checkout that may have changed since
        - delegated to a foreground rust-developer subagent; it reports log items back to the orchestrator
        - `review.rs`: new `checkout_departure` re-reads HEAD and cleanliness after contexts resolve (after Python selection and remote lookups) and before choosing `TreeSource`; moved, dirty, or unreadable state selects a temporary tree; `initial_dirty` and push-start HEAD checks retained ahead of it
        - `mod.rs::run_gates`: refreshes cleanliness beside the existing HEAD check; dirty or unreadable withholds `BISCUIT_CI_PLAN_IN`, replans, and publishes no validation; `initial_dirty` remains an independent reason
        - `tests/orchestration.rs`: one-shot `Transition` (Dirty, Head, UnreadableHead, UnreadableStatus) on the `Fake`, fired at `select-python` or `note`; two 4-case rstest tests added; 6 of 8 failed against the pre-fix sources (the two HEAD-before-gates cases are passing controls)
        - `docs/topics/ci-cd.md`: states when the checkout is re-read
        - decision: a checkout dirty at push start but clean at selection still uses a temporary tree (conservative; validation is withheld either way); alternative of trusting only the fresh state rejected
        - decision: no new `WithholdReason` variant; mid-push dirtiness reports `FinalDirty` (still dirty) or `NoReviewedComparison` (plan not reused) through the existing order
        - verification: `just _test repo-deps --lib ci_push::` 322/322 passed; `cargo clippy -p repo-deps --lib --tests -- -D warnings` clean; `cd scripts && just lint` clean
- work completed for 'R1-F1: Checkout reuse relies on stale cleanliness and HEAD snapshots' at 01:06:21
- starting the work on 'R1-F2: Real recipe receipt and retry coverage is excluded from Windows' at 01:06:30
        - defect class: platform compilation guards remove the end-to-end consumer of a cross-platform production path (the real Bash recipe consuming the selected interpreter and receipts) from the platforms the port must support
        - delegated to a foreground rust-developer subagent; it reports log items back to the orchestrator
        - first subagent pass (report summarized):
                - fixture now finds Git for Windows' `usr\bin\bash.exe` from the real `git` (never WSL's `System32\bash.exe`), passes `OS` through so the recipe takes its `jq --binary` path, and skips tool directories holding `cygwin1.dll`; the six real-recipe tests lost their `#[cfg(unix)]` guards
                - new `parity::a_real_recipe_runs_its_helpers_under_the_selected_fallback_interpreter`: broken `python3`, fallback `py -3` (stub forwarding only prefixed calls to real Python) on Windows and `python` elsewhere; a mutation of the recipe to `python=(python3)` failed it
                - sweep of every other platform guard in `integration/` and `tests/`: 14 clean (Unix-only APIs or Windows twins), two portable tests fixed (`tests/run.rs` unreadable selection now builds a non-Unicode value per OS; shared Git fixture writes `core.autocrlf = false` so `tests/worktree.rs` passes on Windows)
                - Windows root causes found under load: the rig's Cygwin `jq` crashed with 139 inside Git Bash; recorded both facts in the `os` skill
                - `just _test repo-deps --test ci-push` 68/68 on macOS; lib 322 passed; clippy clean
                - `just cross-check repo-deps --os windows`: 891 passed, 1 failed (`ci-build` `archive::tests::one_owner_tree_shares_a_dependency_compile_without_unifying_features`: `produce-owner.sh: line 34: C:\…\ci-build-run-….exe: command not found`), 1 skipped
                - open after pass 1: real-recipe tests take 8–23 s on Windows (12–13 s alone) against the 5 s L1 budget and nextest's 30 s stop; the `ci-build` Windows failure must be attempted under the cross-OS rule
        - second subagent pass (Windows duration and `ci-build`):
                - profiled one push on `build-win`: about 110 processes (43 git, 22 Python, about 20 `jq`, about 20 coreutils, 5 stubs) at 25–35 ms each; the recipe itself is about 2.1 s of a 4.4 s push
                - `integration/fixture.rs`: every stub is now a hard link to one seed (itself a link to the extracted `ci-push-test-tool` when possible), removing a 0.3 s Defender first-run scan per stub; serial A/B 73.4 s → 68.9 s
                - `ci-build-archive-tests.rs`: `CI_BUILD_BIN` spelled with `/` so Git Bash runs it as a path (R1-F2.26)
                - `os` skill: added the Bash command-word lookup fact and the fresh-`.exe` Defender cost
                - final `just cross-check repo-deps --os windows`: 892 passed, 1 skipped, exit 0
                - 5 s Windows budget not reachable for real-recipe tests without seeding or production spawn reduction; recorded as R1-F2.27 (unresolved) and as a human review item
- work completed for 'R1-F2: Real recipe receipt and retry coverage is excluded from Windows' at 02:27:59 (parent unresolved: R1-F2.27 open)

### Completion Report

- **outcome: partial** — elapsed about 100 minutes (started 2026-11-11T00:47:17-08:00)
- parent findings: total 2 = fixed 1 (R1-F1) + already fixed 0 + blocked 0 + unresolved 1 (R1-F2) + disputed 0
- instance rows: R1-F1.1–R1-F1.11 all fixed or clean; R1-F2.1–R1-F2.26 all fixed or clean; R1-F2.27 unresolved
- verified repairs:
        - R1-F1: tree selection (`review.rs` `checkout_departure`) and gate plan reuse (`mod.rs` `run_gates`) re-read HEAD and cleanliness at the moment of each decision; unreadable state is treated as changed; deterministic public-boundary transition tests prove each site and fail against the pre-fix code
        - R1-F2: the six real-recipe parity tests run on native Windows under Git for Windows' Bash; a new real-recipe fallback-interpreter test proves the recipe consumes the selected executable and prefix (`py -3` on Windows); Windows-only fixture faults (Cygwin `jq`, `autocrlf`) and an unrelated `ci-build` Windows test failure were fixed; a portable sibling test lost its Unix guard
- open items:
        - R1-F2.27 (and therefore parent R1-F2): on the `build-win` rig, real-recipe tests take 5.7–12.3 s alone and up to about 22 s under full-package load, against the 5 s L1 budget (all pass, below nextest's 30 s stop); reason: about 3.7 s of process-start floor per push under Git Bash with Defender on; next action: the author chooses among seeding the first receipt with a stub gate, reducing process spawns in `just/ci-local.just` and the Python helpers, or a Windows-specific budget for real-recipe tests (recorded in the review's `human_review_items`)
- no performance measurement was deferred for host load; `deferred_perf_measurement` is not set
- noted drift, left as the record it is: row 1 of the parity inventory in `implementation-log.md` still says "(Unix only)"; this ledger supersedes it
- changed files:
        - `scripts/ci-push/review.rs`, `scripts/ci-push/mod.rs`, `scripts/ci-push/tests/orchestration.rs`, `docs/topics/ci-cd.md` (R1-F1)
        - `scripts/ci-push/integration/evidence_fixture.rs`, `scripts/ci-push/integration/parity.rs`, `scripts/ci-push/integration/fixture.rs` (R1-F2.1–R1-F2.9, R1-F2.27 partial reduction)
        - `scripts/ci-push/tests/run.rs` (R1-F2.24), `scripts/ci-push/tests/git.rs` (R1-F2.25), `scripts/ci-build-archive-tests.rs` (R1-F2.26)
        - `.claude/skills/os/windows.md`, `.claude/skills/os/SKILL.md` (Windows facts learned: Cygwin `jq` under Git Bash, persisted `autocrlf`, Bash command-word lookup, fresh-`.exe` Defender cost)
- final verification:
        - `just test repo-deps` (macOS): 905 run, 905 passed, 1 skipped; 3 slow, of which `parity::a_cell_whose_prior_evidence_is_a_failure_is_rerun_and_superseded` (5.08 s under load) passed alone in 4.22 s (load-sensitive); the two `ci-build` slow tests are unchanged by this cycle — verifies R1-F1, R1-F2 on macOS
        - `just lint`: 28 areas passed, 0 failed; `cargo clippy -p repo-deps --tests --all-features -- -D warnings`: clean
        - `just check-tier-coverage scripts`: 0 stranded
        - `just cross-check repo-deps --os windows` (by the second subagent, final run): 892 passed, 1 skipped, exit 0 — verifies R1-F2 rows and R1-F2.24–R1-F2.26 on Windows; spec `os_failing` not set

## Implementation of Review Findings #2

> **started at:** 2026-11-11T02:31:09-08:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/rusty-biscuit/features/2026-11-01-rust-pre-push/review-2.md'
- this is iteration 2 of the review-to-implement cycle
- the review's `implemented` flag was confirmed `false` before repairs started
- the review has no `human_in_the_loop` items and no blocked findings; its one finding (R1-F2, carried) is repairable in this cycle
- the review's IDs are retained: the carried parent is R1-F2, its open child R1-F2.27, and the review's sub-IDs R1-F2.27.1–R1-F2.27.9

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R1-F2 | Real-recipe coverage is portable but still exceeds the L1 time budget | unresolved | macOS: every required child is within 5 s in the owning suite; native Windows: every child is still 7–16 s (R1-F2.27 open); see the completion report |
| R1-F2.27 | Every real-recipe parity test runs within 5 s under suite load, with portable recipe coverage and assertions preserved | unresolved | macOS fixed (worst 4.07 s in `just test repo-deps`); Windows still over budget; the remaining floor is about 22 Python and 43 `git` process starts per push in the Python evidence/planner helpers, which the spec assigns to `2026-11-06-rust-evidence-client` and `2026-11-06-rust-ci-planner` and forbids porting here |
| R1-F2.27.1 | `an_advanced_non_main_target_reuses_prior_cells_and_publishes_exact_receipts`: same receipt/reuse assertions within 5 s | unresolved | macOS 3.95 → 3.44 s (first push seeded via `seed_gate`, second push on the real recipe; all receipt/reuse assertions kept); Windows 15.1 / 11.3 s |
| R1-F2.27.2 | `a_docs_only_follow_up_skips_the_cells_prior_evidence_covers`: same skip/receipt assertions within 5 s | unresolved | macOS 3.99 → 3.24 s (seeded first push; first-push receipt assertions kept); Windows 14.8 / 15.7 s, 8.3 s alone |
| R1-F2.27.3 | `a_follow_up_changing_a_packages_inputs_reruns_its_cell`: same invalidation assertions within 5 s | unresolved | macOS 4.04 → 3.30 s; Windows 14.5 / 15.0 s |
| R1-F2.27.4 | `a_cell_whose_prior_evidence_is_a_failure_is_rerun_and_superseded`: same retry/newest-evidence assertions within 5 s | unresolved | macOS 5.06 → 4.07 s (first push seeded; final `local_evidence.py verify` kept because no Rust verifier exists; real-recipe failure-receipt coverage moved to new row R2-N1); Windows 16.1 / 17.8 s |
| R1-F2.27.5 | `a_stacked_pull_request_records_its_target_tip_as_the_receipt_base`: same base/identity assertions within 5 s | unresolved | macOS 2.07 s (now also pins `assert_retained_reports`, the seed's equivalence check); Windows 9.9 / 8.3 s, 5.8–6.2 s alone (6.45–6.78 s before the recipe `jq` batching) |
| R1-F2.27.6 | `a_real_recipe_interruption_withholds_earlier_reports`, malformed-manifest case | unresolved | macOS 1.56 s; Windows 7.5 / 7.4 s |
| R1-F2.27.7 | Same function, exit-130 case | unresolved | macOS 1.54 s; Windows 7.6 / 7.2 s |
| R1-F2.27.8 | Same function, recorded-143 case | unresolved | macOS 1.52 s; Windows 7.6 / 7.3 s |
| R1-F2.27.9 | `a_real_recipe_runs_its_helpers_under_the_selected_fallback_interpreter`: executable/prefix and helper assertions within 5 s | unresolved | macOS 2.13 s (still drives the real recipe); Windows 10.1 / 9.9 s |
| R2-N1 | Discovered (class member, new): `a_real_recipe_gate_failure_records_a_complete_failure` keeps real-recipe ordinary-failure receipt coverage moved out of R1-F2.27.4 | unresolved | macOS 1.94 s, asserts retained reports byte for byte; Windows 9.6 / 9.8 s |
| R2-N2 | Sweep: stub-gate parity tests and launcher tests on Windows (`a_blocked_strict_failure_publishes_its_note_but_ci_reruns_the_cell` 7.1–8.2 s, `a_broken_python3_hands_the_fallback_to_every_helper_and_the_gates` 5.6–7.0 s, `a_branch_creating_push_records_the_remote_main_tip_and_says_so` 5.6–6.8 s, `a_complete_warn_failure_publishes_its_note` 5.5–6.8 s, `a_record_in_the_default_store_binds_a_fresh_session` 5.3 s, `a_published_receipt_names_retained_readable_reports` 5.1–5.3 s, `a_failed_validation_note_publication_keeps_the_durable_reports` 5.1 s, `launcher::windows_powershell_pushes_through_the_launcher_to_the_real_ci_push` 7.3 s, `launcher::the_real_ci_push_reviews_an_auto_push_without_gates_and_a_strict_push_with_them` 5.2 s) | unresolved | The review called these clean controls from macOS evidence; on Windows they share the same per-push helper process floor without the recipe, which shows the remaining cost is in the helpers, not the recipe; all ≤ 2.4 s on macOS |
| R2-N3 | Sweep: other Rust consumers of the real recipe | fixed | Only `tools/test-toolkit/tests/ci_workflow_contracts.rs` (`no_guard_consumer_derives_its_own_source_scope`) reads the recipe text; its anchor moved to the guard's `if` line after the `jq` batching; fails against the new recipe without the fix; 228/228 pass |
| R2-N4 | Discovered (sibling stale-test class): `scripts/ci/test_ci_local.py::PrePushEvidenceContractTests` asserted text of the retired Bash hook in `.githooks/pre-push` and failed on this branch | fixed | Removed the test and its `PRE_PUSH` constant (spec: remove obsolete tests in the same change); the behavior is owned by `scripts/ci-push/evidence.rs` (`--prior-receipt`, retained report overlay) and covered by `tests/orchestration.rs::retry_reports_overlay_the_durable_directory` and the parity retry tests; `test_ci_local.py` 108/108 OK |

### Progress

- starting the work on 'R1-F2: Real-recipe coverage is portable but still exceeds the L1 time budget' at 02:31:31
        - defect class: acceptance tests that drive an end-to-end subprocess pipeline repeat setup and helper process work (including duplicate metadata queries and parsing in the code under test), so each test's wall time exceeds the L1 budget on slower process-start hosts
        - sibling scope (review's bounded class): the seven functions that call `stage_ci_local_harness` in `scripts/ci-push/integration/parity.rs` (nine cases, R1-F2.27.1–R1-F2.27.9); the stub-gate process tests are controls
        - delegated to a foreground rust-developer subagent; it reports log items back to the orchestrator
        - subagent report, grouped:
                - discovery: a fixture push on macOS is CPU-bound (1.3 s CPU in 1.45 s wall), with 29 `git`, 21 `jq`, and 20 Python starts per push; helpers: `local_evidence.py verify` about 0.13 s with 8 `git` children (4 `notes list`, one per environment), `scope-record` 0.08 s, `record-cells` 0.12 s, `publish-receipt` 0.19 s; the recipe itself is about 0.55 s
                - discovery: on macOS the first exec of `target/debug/ci-push` after any Cargo command costs about 0.6 s (new inode, signature check), and `/usr/bin/git` is the `xcrun` launcher (about 12 ms against 5 ms); both recorded in `.claude/skills/os/macos.md`
                - production change: `just/ci-local.just` reads its scope facts with one `jq` and each package's six plan facts with one `jq` (previously nine separate calls), and counts packages in Bash; a replayed push produced byte-identical output; Windows A/B on R1-F2.27.5 alone: 6.45–6.78 s → 5.83–6.20 s; macOS about −0.05 s
                - edge: a malformed plan with no `.cells` now fails before that package's gates instead of after its unreachable `check` step; it fails either way
                - decision: the first push of R1-F2.27.1–R1-F2.27.4 is seeded with `Evidence::seed_gate` (a stub `just pre-push` that copies the plan back through a new `copy_files` behavior in `ci-push-test-tool` and stages the same `recipe_reports` the real recipe stages); the second push still runs the real recipe; equivalence is held by the shared report generator, by `assert_retained_reports` in two real-recipe tests, by ci-push recording from the review's own plan, and by `test_ci_local.py`'s plan write-back test; rejected: calling the helpers directly (duplicates ci-push's arguments), copying a pre-built on-disk fixture (absolute paths in remote config and notes), hand-forged notes (drift)
                - decision: R1-F2.27.4's real-recipe ordinary-failure receipt coverage moved to the new single-push test `a_real_recipe_gate_failure_records_a_complete_failure`; the final `verify` subprocess is kept because no Rust verifier exists and an in-process note read would not prove the verifier prefers the newer evidence
                - decision: the harness's planner stub runs the real planner in-process (`runpy`), one Python start per projection as in production
                - rejected: precompiling fixture helpers (0.08 s per push, reverted); a production `PYTHONPYCACHEPREFIX` (adds a compile per push on warm checkouts); dropping the recipe's self-test loop from the harness (tests would stop running the real recipe text); removing duplicate `rev-parse`/`notes list` calls in the Python helpers (small gain; that code is being replaced by the follow-up specs)
                - `ci_workflow_contracts.rs` anchor moved after the recipe edit (R2-N3)
                - `just cross-check repo-deps --os windows`: 893 passed, 1 skipped, exit 0; 34 tests at or above 5 s on that run; per-case Windows timings are in the ledger
                - discovery: on Windows, stub-gate parity and launcher tests are also 5.1–8.2 s (R2-N2), so the remaining floor is the Python helper chain, not the recipe
        - orchestrator reconciliation:
                - reviewed the recipe diff: one `jq` per package; an empty plan prints nothing so every fact is empty and `guard_planned` defaults to `false`, matching the old `-s` guards
                - reviewed `seed_gate`/`assert_retained_reports`: the seed's reports come from the same generator, checked byte for byte against a real-recipe run
                - fixed R2-N4 myself: removed the stale hook-text test from `scripts/ci/test_ci_local.py` (108/108 OK)
                - started the `scripts` area's `just test`, found it runs the whole workspace (58,296 tests), stopped it, and ran the package-scoped `just test repo-deps` the previous cycle used
- work completed for 'R1-F2: Real-recipe coverage is portable but still exceeds the L1 time budget' at 04:05:07 (parent unresolved: Windows duration open)

### Completion Report

- **outcome: partial** — elapsed about 94 minutes (02:31:09 → about 04:05)
- parent findings: total 1 = fixed 0 + already fixed 0 + blocked 0 + unresolved 1 (R1-F2) + disputed 0
- instance rows: R1-F2.27 and R1-F2.27.1–R1-F2.27.9 unresolved (each within 5 s on macOS, over 5 s on native Windows); R2-N1 and R2-N2 unresolved (Windows); R2-N3 and R2-N4 fixed
- verified repairs:
        - on macOS, every real-recipe parity case is now within budget in the owning suite (worst R1-F2.27.4 at 4.07 s, down from 5.06 s); every receipt, reuse, skip, invalidation, retry, base, interruption, and executable/prefix assertion is kept; real-recipe-to-receipt coverage remains in R1-F2.27.5, R1-F2.27.6–R1-F2.27.9, R2-N1, and the second push of R1-F2.27.1–R1-F2.27.4
        - production `just/ci-local.just` no longer starts a `jq` per plan fact (about 0.6 s per push saved on Windows)
        - the stale `test_ci_local.py` hook-text test that failed on this branch was removed
- open items:
        - R1-F2 / R1-F2.27 / R1-F2.27.1–R1-F2.27.9 / R2-N1 / R2-N2: native Windows durations of 7–18 s (5.8 s minimum alone) against the 5 s L1 budget
                - reason: every push through `ci-push` starts about 22 Python and 43 `git` processes in the evidence and planner helpers at 25–35 ms each under Windows Defender; stub-gate tests that never run the recipe are also over 5 s, so no further test-layout change reaches the budget; the spec says not to port those helpers here ("Do not build a second evidence store client or port the planner here"), and the review rules out raising the budget, retries, tier renames, or skipping Windows
                - next action: the author decides whether this spec's Windows L1 budget waits on `2026-11-06-rust-evidence-client` / `2026-11-06-rust-ci-planner` (which remove these process starts) or whether helper call-count reductions should be pulled into this spec; recorded in the review's `human_review_items`
- unrelated over-budget tests seen on macOS (not this class; already slow before this cycle and not changed by it): `ci-build` `archive::tests::a_real_external_dynamic_dependency_is_discovered_and_refused_when_removed` (5.14 s) and `archive::tests::the_canonical_tier_recipes_run_the_fixture_archive_without_rebuilding_it` (7.57 s), both of which run real Cargo builds; `test-toolkit` `ci_workflow_contracts::the_shipped_planner_emits_plans_the_guards_reader_accepts` (5.45 s)
- no performance measurement was deferred for host load; `deferred_perf_measurement` is not set; the Windows cross-check passed, so the spec's `os_failing` is not set
- changed files:
        - `just/ci-local.just` (R1-F2.27, production `jq` batching)
        - `scripts/ci-push/integration/evidence_fixture.rs`, `scripts/ci-push/integration/parity.rs`, `scripts/ci-push-test-tool.rs` (R1-F2.27.1–R1-F2.27.9, R2-N1)
        - `tools/test-toolkit/tests/ci_workflow_contracts.rs` (R2-N3)
        - `scripts/ci/test_ci_local.py` (R2-N4)
        - `.claude/skills/os/macos.md`, `.claude/skills/os/SKILL.md`, `.claude/skills/os/windows.md` (host facts learned)
- final verification (macOS unless noted):
        - `just test repo-deps`: 906 run, 906 passed, 1 skipped, 2 slow (the unrelated `ci-build` tests above); parity cases 1.52–4.07 s — verifies R1-F2.27.1–R1-F2.27.9 and R2-N1 on macOS
        - `BISCUIT_REQUIRE_JUST=1 BISCUIT_REQUIRE_JQ=1 BISCUIT_REQUIRE_BASH=1 python3 scripts/ci/test_ci_local.py`: 108 tests OK — verifies the recipe change and R2-N4
        - `cargo nextest run -p test-toolkit --test ci_workflow_contracts`: 228 passed, 1 slow (unrelated, above) — verifies R2-N3
        - `cd scripts && just lint`: 28 areas passed, 0 failed; `cargo clippy -p repo-deps -p test-toolkit --tests --all-features -- -D warnings`: clean
        - `just check-tier-coverage scripts`: 0 stranded
        - `just cross-check repo-deps --os windows` (subagent, after its final change): 893 passed, 1 skipped, exit 0 — functional pass on Windows; durations remain over budget as recorded
        - the `scripts` area's `just test` was stopped after about 16,000 of 58,296 workspace tests (all passing up to that point) because it runs the whole workspace; `just test repo-deps` is the gate used instead

## Implementation of Review Findings #3

> **started at:** 2026-11-11T04:11:19-08:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/rusty-biscuit/features/2026-11-01-rust-pre-push/review-3.md'
- this is iteration 3 of the review-to-implement cycle 
- the review's `implemented` flag was confirmed `false` before repairs started
- the review has no `human_in_the_loop` items and no blocked findings; it explicitly rules that removing redundant work inside the existing Python helpers is within this spec's scope, which removes the iteration-2 open question about porting
- R1-F2 and R3-F1 share one defect class and one repair (the review asks for them to be repaired together); they are worked by one foreground subagent and reconciled per row

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R1-F2 | Portable real-recipe tests still exceed the Windows L1 time budget | unresolved | every child is within 5 s on macOS (worst 3.35 s); all nine children remain over 5 s on native Windows (5.83–13.26 s in the final owning-suite run), down 0.4–2.4 s from iteration 2; remaining floor is process starts the production pipeline performs (see completion report) |
| R1-F2.27.1 | `an_advanced_non_main_target_reuses_prior_cells_and_publishes_exact_receipts`: same exact-receipt/reuse checks within 5 s on Windows | unresolved | Windows 15.1/11.3 (iter 2) → 10.81 s; macOS 2.81 s; assertions unchanged |
| R1-F2.27.2 | `a_docs_only_follow_up_skips_the_cells_prior_evidence_covers`: same skip/receipt checks within 5 s | unresolved | Windows 14.8–15.7 → 11.56 s; macOS 2.84 s |
| R1-F2.27.3 | `a_follow_up_changing_a_packages_inputs_reruns_its_cell`: same invalidation checks within 5 s | unresolved | Windows 14.5–15.0 → 11.81 s; macOS 2.84 s |
| R1-F2.27.4 | `a_cell_whose_prior_evidence_is_a_failure_is_rerun_and_superseded`: same newest-evidence checks within 5 s | unresolved | Windows 16.1–17.8 → 13.26 s; macOS 3.35 s |
| R1-F2.27.5 | `a_stacked_pull_request_records_its_target_tip_as_the_receipt_base`: same base/identity/report checks within 5 s | unresolved | Windows 8.3–9.9 → 7.13 s; macOS 1.79 s |
| R1-F2.27.6 | `a_real_recipe_interruption_withholds_earlier_reports`, malformed manifest: refusal within 5 s | unresolved | Windows 7.4–7.5 → 5.95 s; macOS 1.24 s |
| R1-F2.27.7 | Same function, exit 130: refusal within 5 s | unresolved | Windows 7.2–7.6 → 5.83 s; macOS 1.18 s |
| R1-F2.27.8 | Same function, recorded 143: refusal within 5 s | unresolved | Windows 7.3–7.6 → 6.14 s; macOS 1.31 s |
| R1-F2.27.9 | `a_real_recipe_runs_its_helpers_under_the_selected_fallback_interpreter`: executable/prefix checks within 5 s | unresolved | Windows 9.9–10.1 → 8.12 s; macOS 1.94 s |
| R3-F1 | Additional hook tests share the unresolved Windows duration defect | unresolved | 7 of 10 named consumers fixed on Windows; R3-F1.1, R3-F1.2, R3-F1.9 remain over 5 s |
| R3-F1.1 | `parity::a_real_recipe_gate_failure_records_a_complete_failure`: receipt/report assertions within 5 s | unresolved | Windows 9.6–9.8 → 7.94 s; macOS 1.61 s |
| R3-F1.2 | `parity::a_blocked_strict_failure_publishes_its_note_but_ci_reruns_the_cell`: within 5 s | unresolved | Windows 7.1–8.2 → 5.85 s; macOS 2.09 s |
| R3-F1.3 | `parity::a_broken_python3_hands_the_fallback_to_every_helper_and_the_gates`: within 5 s | fixed | Windows 5.6–7.0 → 4.36 s (owning suite); macOS 1.42 s; assertions unchanged |
| R3-F1.4 | `parity::a_branch_creating_push_records_the_remote_main_tip_and_says_so`: within 5 s | fixed | Windows 5.6–6.8 → 4.33 s; macOS 1.35 s |
| R3-F1.5 | `parity::a_complete_warn_failure_publishes_its_note`: within 5 s | fixed | Windows 5.5–6.8 → 3.92 s; macOS 1.17 s |
| R3-F1.6 | `parity::a_record_in_the_default_store_binds_a_fresh_session`: within 5 s | fixed | Windows 5.3 → 3.54 s; macOS 1.12 s |
| R3-F1.7 | `parity::a_published_receipt_names_retained_readable_reports`: within 5 s | fixed | Windows 5.1–5.3 → 4.05 s; macOS 1.09 s |
| R3-F1.8 | `parity::a_failed_validation_note_publication_keeps_the_durable_reports`: within 5 s | fixed | Windows 5.1 → 3.79 s; macOS 1.16 s |
| R3-F1.9 | `launcher::windows_powershell_pushes_through_the_launcher_to_the_real_ci_push`: within 5 s | unresolved | Windows 7.3 → 7.34 s (Windows-only test; includes Cargo freshness check and PowerShell start) |
| R3-F1.10 | `launcher::the_real_ci_push_reviews_an_auto_push_without_gates_and_a_strict_push_with_them`: within 5 s | fixed | Windows 5.2 → 4.79 s (close to the limit; ~10% run-to-run noise); macOS 1.65 s |
| R3-F1.11 | Other parity tests and Git Bash launcher: coverage preserved, durations checked in the final Windows owning-suite run | clean | final Windows owning-suite run: Git Bash launcher 4.69 s, every other parity test ≤ 3.02 s; macOS ≤ 0.91 s |
| R3-F1.12 | Dispatcher-only, Python probe, process supervision tests (control: no planning/evidence pipeline) | clean | control: Windows all ≤ 1.1 s; these tests start no planning/evidence helper |
| R3-F1.13 | Public orchestration tests (control: injected services) | clean | control: 322/322 library tests, none slow; injected services, no helper pipeline |

### Progress

- starting the work on 'R1-F2 + R3-F1: hook L1 tests exceed the Windows time budget through the shared helper pipeline' at 04:14:00
        - defect class: every push through the hook starts many redundant `git` and Python processes in the planning/evidence helper chain, so any test that performs a fixture push pays a per-push process floor that exceeds the L1 budget on hosts with expensive process starts
        - delegated to one foreground rust-developer subagent (R1-F2 and R3-F1 together, as the review asks); it reported log items back to the orchestrator
        - subagent report, grouped:
                - profile method: temporary child-process logging in `ci-push`, a `sitecustomize.py` that logged every helper's Python and `git` starts plus `cProfile`, Bash xtrace with `EPOCHREALTIME` on the recipe, and timed fixture setup steps; all instrumentation was removed afterward
                - Windows profile, one test alone, before → after: real-recipe push 5.14 → 4.58 s; stub-gate push 2.27 → about 1.6 s; fixture setup 0.7–1.0 → 0.4–0.6 s; helper `git` children 24 → 13 per push (about 20 ms each instead of 30 ms); `just pre-push` step 2.09 → 1.80 s; parallel-suite load factor about 1.4×
                - production change (`scripts/ci/local_evidence.py`): `prefetch_revisions` resolves several names with one `git cat-file --batch-check`; `revision()` answers remembered immutable names (full object IDs, alone or peeled to `^{tree}`), and everything else still goes through `git rev-parse --verify`; `_resolve` skips `git notes list` for notes refs that do not exist; `record_scope`/`record_cells` prefetch head, base, and tree; `gate_input_identity` lists paths in batches of 64 with one `ls-tree` and relists path by path when a batch is refused
                - production change (`scripts/ci/observation_notes.py`): `publish()` prefetches commit, tree, and notes ref, and skips `git notes show` when the ref is absent
                - production change (`just/ci-local.just`): the running-cells `jq` and the cell-listing `jq` merged into one tagged `jq` (same lines, same order); `cell_runs_here` uses a Bash exact-line match instead of `grep -qxF`
                - test change (`scripts/ci-push/integration/evidence_fixture.rs`): the fixture commits only the helpers `ci-push` runs plus their import closure; the Windows fixture `PATH` names Git's `bin` and `usr\bin` instead of the `cmd\git.exe` launcher (one fewer process per Git call); the harness planner stub uses `runpy.run_module` with bytecode caching (writes to the ignored `scripts/ci/__pycache__/`)
                - new tests: `scripts/ci/test_local_evidence.py::BatchedGitReadTests` (8 tests: prefetch equals `rev-parse` including absent refs; remembered names need no further Git call and mutable names are never remembered; unknown IDs fall through to `rev-parse`; only existing notes refs are listed in review and bootstrap; batched identity equals the per-path union at batch sizes 1, 2, 64; a refused batch is relisted path by path)
                - decision: prefetch with fallback rather than a long-lived `git cat-file` process; a persistent child would hold its directory open, which blocks Windows temp-directory cleanup
                - decision: check notes-ref existence rather than parsing notes trees in Python, which would amount to a second notes reader
                - decision: did not merge helper processes (verify+apply+scope-record; record-cells+publish-receipt); estimated −0.5 s per push under load, but it rewrites the ci-push↔helper step boundaries the orchestration tests and parity inventory rely on, and the review warns against a wholesale redesign; recorded in the review's `human_review_items`
                - rejected: running the recipe's self-test suites in one process (changes their isolation in production); forging seed evidence for the two-push tests (a second evidence client; would save only about 0.6 s); merging the build-key `jq` calls (changes malformed-plan behavior); changing `platform`/argument-parser setup (they feed recorded fields and help output)
                - remaining floor on a real-recipe push (Windows, alone, about 4.0 s inside `ci-push`): recipe 1.8 s (11 self-test Python starts ≈ 0.5 s, about 16 `jq`/coreutils/stub starts, planner projection 0.1–0.25 s, `just`/Bash startup); evidence helpers about 1.3 s (Python start ≈ 45 ms each, imports recompiled under `PYTHONDONTWRITEBYTECODE`, a 50–100 ms WMI query from `platform` in two helpers); note transport about 0.45 s (fetch, push, ls-remote, push); two-push tests also pay a seed push of about 1.6 s
                - skill update: `.claude/skills/os/windows.md` (Python helper process cost, batching Git reads) and a symptom row in `.claude/skills/os/SKILL.md`
        - orchestrator reconciliation:
                - reviewed the Python diff: remembered entries are restricted to full object IDs (immutable), keyed by directory; any name not answered falls back to the old `rev-parse` call, so answers are unchanged; `ls-tree` over a union of pathspecs lists the same entries as per-path calls, then sorts and dedupes them as before
                - reviewed the recipe diff: the tagged single `jq` emits the listing lines in the original order, and the running-cell membership test is an exact-line match equivalent to `grep -qxF`
                - Windows durations in the ledger come from the subagent's final `just cross-check repo-deps --os windows` run (893 passed, 1 skipped, exit 0), not from a fresh orchestrator run; macOS checks were rerun by me (below)
- work completed for 'R1-F2 + R3-F1: hook L1 tests exceed the Windows time budget through the shared helper pipeline' at 05:34:30 (both parents unresolved: Windows duration remains open for 12 rows)

### Completion Report

- **outcome: partial**; elapsed about 83 minutes (04:11:19 → 05:34)
- parent findings: total 2 = fixed 0 + already fixed 0 + blocked 0 + unresolved 2 (R1-F2, R3-F1) + disputed 0
- instance rows: fixed 7 (R3-F1.3–R3-F1.8, R3-F1.10); clean 3 (R3-F1.11–R3-F1.13); unresolved 12 (R1-F2.27.1–R1-F2.27.9, R3-F1.1, R3-F1.2, R3-F1.9)
- verified repairs:
        - helper `git` work per push nearly halved (24 → 13 starts) with byte-identical answers; the recipe starts fewer `jq`/`grep` processes; fixture setup and the Windows Git launcher hop removed from test cost
        - seven previously over-budget Windows consumers are now within 5 s in the owning suite; every remaining row dropped 0.4–2.4 s; no assertion or case was dropped, and no budget, retry, tier, or skip workaround was used
- open items:
        - R1-F2.27.1–R1-F2.27.4 (Windows 10.8–13.3 s): two-push tests, each paying a seed push plus a real-recipe push
        - R1-F2.27.5, R1-F2.27.6–R1-F2.27.8, R1-F2.27.9, R3-F1.1, R3-F1.2 (Windows 5.8–8.1 s): single-push tests whose remaining cost is process starts the production pipeline performs
        - R3-F1.9 (Windows 7.34 s): PowerShell-launched Git through the real launcher, including Cargo's freshness check
        - reason: the measured redundant work is removed; what remains is one Python start per helper step, the recipe's self-test and `jq` processes, and note transport; the next lever (merging helper processes, about −0.5 s per push) would not bring the two-push rows under 5 s and changes the ci-push↔helper interface
        - next action: author decision recorded in the review's `human_review_items`: restructure helper processes in this spec, or accept that the Windows floor clears with `2026-11-06-rust-evidence-client` and `2026-11-06-rust-ci-planner`
- not related to this class, over 5 s and unchanged by this cycle: macOS `ci-build` `archive::tests::a_real_external_dynamic_dependency_is_discovered_and_refused_when_removed` (5.19 s) and `archive::tests::the_canonical_tier_recipes_run_the_fixture_archive_without_rebuilding_it` (7.66 s); on Windows, 12 `ci-build` `archive::tests` (5.3–20.3 s) and 2 `feature-attribution` tests (11.0, 11.5 s); all of them run real Cargo builds
- no performance measurement was deferred for host load; `deferred_perf_measurement` is not set; the Windows cross-check passed functionally, so the spec's `os_failing` is not set
- changed files this cycle:
        - `scripts/ci/local_evidence.py`, `scripts/ci/observation_notes.py`, `just/ci-local.just` (R1-F2, R3-F1: production redundant-work removal)
        - `scripts/ci-push/integration/evidence_fixture.rs` (R1-F2, R3-F1: fixture setup and Windows Git path)
        - `scripts/ci/test_local_evidence.py` (new `BatchedGitReadTests`)
        - `.claude/skills/os/windows.md`, `.claude/skills/os/SKILL.md` (host facts learned)
        - `features/2026-11-01-rust-pre-push/review-3.md` (metadata), this log
- final verification (macOS, orchestrator, after the last change):
        - `just test repo-deps`: 906 run, 906 passed, 1 skipped, 2 slow (the unrelated `ci-build` tests above); no hook test slow (R1-F2.27.*, R3-F1.* on macOS)
        - `BISCUIT_REQUIRE_JUST=1 BISCUIT_REQUIRE_JQ=1 BISCUIT_REQUIRE_BASH=1 python3 scripts/ci/test_ci_local.py`: OK (recipe changes)
        - `python3 scripts/ci/test_local_evidence.py`, `test_evidence_reuse.py`, `test_hosted_observations.py`, `test_cross_check.py`: all OK (helper changes, new batched-read tests)
        - `cd scripts && just lint`: 0 failed
        - `just check-tier-coverage scripts`: 0 stranded
        - `git diff --check`: clean
- final verification (subagent):
        - `just cross-check repo-deps --os windows` after its final change: 893 passed, 1 skipped, exit 0; per-row durations in the ledger
        - `cargo nextest run -p test-toolkit --test ci_workflow_contracts`: 228 passed, 1 slow (unrelated `the_shipped_planner_emits_plans_the_guards_reader_accepts`, 5.38 s)
        - `python3 scripts/ci/test_affected_scope.py`: OK

## Implementation of Review Findings #4

> **started at:** 2026-11-11T05:39:57-08:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/rusty-biscuit/features/2026-11-01-rust-pre-push/review-4.md'
- this is iteration 4 of the review-to-implement cycle 
- the review's `implemented` flag was confirmed `false` before repairs started
- the review has no `human_in_the_loop` items, no blocked findings, and no `Human rulings`; it rules that restructuring the slow tests (separating acceptance questions at their public boundaries, substituting setup with an independent real-recipe equivalence check, splitting the PowerShell auto/strict scenario) is in scope and needs no author decision, which answers the iteration-3 open question
- R1-F2 and R3-F1 share one defect class and one repair; they are worked by one foreground subagent and reconciled per row

### Finding Ledger

| ID | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| --- | --- | --- | --- |
| R1-F2 | Portable real-recipe tests still exceed the Windows L1 time budget | fixed | all nine children within 5 s on Windows in two full owning-suite runs (subagent's final run and the orchestrator's confirmation run); assertions moved to the recipe, hook, and verifier boundaries, none dropped; boundaries pinned by equivalence tests |
| R1-F2.27.1 | `an_advanced_non_main_target_reuses_prior_cells_and_publishes_exact_receipts`: same assertions, below 5 s on Windows | fixed | split: reuse half `an_advanced_non_main_target_reuses_prior_cells_and_publishes_exact_receipts` (Windows 4.67 s, macOS 1.36 s), publication half `an_advanced_non_main_target_publishes_exact_receipts` (4.56 s), recipe skip `the_real_recipe_skips_a_cell_its_plan_reuses` (3.41 s); previously 10.81 s |
| R1-F2.27.2 | `a_docs_only_follow_up_skips_the_cells_prior_evidence_covers`: same assertions, below 5 s | fixed | hook plan feed `a_docs_only_follow_up_skips_the_cells_prior_evidence_covers` 4.26 s; recorded cells `a_docs_only_follow_up_records_only_the_cells_that_ran` 4.37 s; first-push assertions in `seeded_evidence_is_what_a_hook_push_records` 4.97 s (thin margin, see completion report); recipe skip test 3.41 s; previously 11.56 s |
| R1-F2.27.3 | `a_follow_up_changing_a_packages_inputs_reruns_its_cell`: same assertions, below 5 s | fixed | `gate-inputs-changed` refusal asserted on `local_evidence.py verify` output in the same test, 2.89 s; executing cell through the real recipe `the_real_recipe_stages_the_modeled_outcome_of_a_fed_plan::case_1_pass` 3.28 s; previously 11.81 s |
| R1-F2.27.4 | `a_cell_whose_prior_evidence_is_a_failure_is_rerun_and_superseded`: same assertions, below 5 s | fixed | `failed-cell` refusal and complete-failure receipt in the same test 2.90 s; newest-wins verifier check `a_rerun_cells_newest_pass_supersedes_its_prior_failure` 3.98 s; `run    alpha/L1` in the direct recipe cases; previously 13.26 s |
| R1-F2.27.5 | `a_stacked_pull_request_records_its_target_tip_as_the_receipt_base`: same assertions, below 5 s | fixed | same test with a modeled gate (outcome pinned to the real recipe by `the_real_recipe_stages…::case_1_pass`), 4.31 s; previously 7.13 s |
| R1-F2.27.6 | `a_real_recipe_interruption_withholds_earlier_reports::case_1_malformed_manifest`: below 5 s | fixed | hook refusal `a_recipe_interruption_withholds_earlier_reports::case_1_malformed_manifest` 2.78 s; real-recipe manifest message `the_real_recipe_stages…::case_3_malformed_manifest` 3.35 s; previously 5.95 s |
| R1-F2.27.7 | `…::case_2_exit_130`: below 5 s | fixed | `a_recipe_interruption_withholds_earlier_reports::case_2_exit_130` 2.76 s; `the_real_recipe_stages…::case_4_exit_130` 3.14 s; previously 5.83 s |
| R1-F2.27.8 | `…::case_3_recorded_143`: below 5 s | fixed | `a_recipe_interruption_withholds_earlier_reports::case_3_recorded_143` 2.79 s; `the_real_recipe_stages…::case_5_recorded_143` 3.58 s; previously 6.14 s |
| R1-F2.27.9 | `a_real_recipe_runs_its_helpers_under_the_selected_fallback_interpreter`: below 5 s | fixed | same name, now runs the real recipe directly with the fallback interpreter and prefix, 3.55 s; hook-side start pinned by `the_gates_are_started_as_the_direct_recipe_runs_start_them` (argv, cwd, staging names, `BISCUIT_CI_PYTHON`, NUL-terminated prefix, `python3` probed once) 3.17 s; previously 8.12 s; decision recorded in `human_review_items` |
| R3-F1 | Three additional hook tests still exceed the Windows L1 time budget | fixed | the three open children are within 5 s; the seven previously fixed consumers stay within 5 s; controls unchanged |
| R3-F1.1 | `parity::a_real_recipe_gate_failure_records_a_complete_failure`: below 5 s | fixed | renamed `a_recipe_gate_failure_records_a_complete_failure` (modeled gate) 4.27 s; the real recipe's complete-failure staging `the_real_recipe_stages…::case_2_complete_failure` 3.32 s; previously 7.94 s |
| R3-F1.2 | `parity::a_blocked_strict_failure_publishes_its_note_but_ci_reruns_the_cell`: below 5 s | fixed | publication half kept under the same name 3.84 s; CI-rerun half `ci_reruns_a_cell_whose_published_evidence_is_a_complete_failure` 2.84 s; previously 5.85 s |
| R3-F1.3 | `parity::a_broken_python3_hands_the_fallback_to_every_helper_and_the_gates`: stays below 5 s | clean | unchanged; 4.03 s |
| R3-F1.4 | `parity::a_branch_creating_push_records_the_remote_main_tip_and_says_so`: stays below 5 s | clean | unchanged; under 3.84 s (not in the top 12) |
| R3-F1.5 | `parity::a_complete_warn_failure_publishes_its_note`: stays below 5 s | clean | unchanged; 4.01 s |
| R3-F1.6 | `parity::a_record_in_the_default_store_binds_a_fresh_session`: stays below 5 s | clean | unchanged; under 3.84 s |
| R3-F1.7 | `parity::a_published_receipt_names_retained_readable_reports`: stays below 5 s | clean | unchanged; 3.98 s |
| R3-F1.8 | `parity::a_failed_validation_note_publication_keeps_the_durable_reports`: stays below 5 s | clean | unchanged; under 3.84 s |
| R3-F1.9 | `launcher::windows_powershell_pushes_through_the_launcher_to_the_real_ci_push`: split auto/strict, each below 5 s, forwarding retained | fixed | `windows_powershell_auto_push_reaches_the_real_ci_push_through_the_launcher` 3.66 s and `windows_powershell_strict_push_…` 3.75 s, each with its own fixture and real PowerShell-launched Git; previously 7.34 s |
| R3-F1.10 | `launcher::the_real_ci_push_reviews_an_auto_push_without_gates_and_a_strict_push_with_them`: stays below 5 s | fixed | split the same way (sweep: same two-push shape): `the_real_ci_push_reviews_an_auto_push_without_gates` 2.69 s, `the_real_ci_push_runs_the_gates_of_a_strict_push` 2.51 s; macOS 1.57/1.56 s; previously 4.79 s |
| R3-F1.11 | Other parity tests and Git Bash launcher: coverage and budget preserved | clean | Git Bash launcher also split (same helper): `windows_git_bash_{auto,strict}_push_…` 2.55/2.50 s; every other parity test at most 3.84 s |
| R3-F1.12 | Dispatcher, Python probe, process supervision: coverage preserved | clean | unchanged; at most 1.08 s on Windows |
| R3-F1.13 | Public orchestration (injected services): coverage preserved | clean | unchanged; library tests pass in `just test repo-deps` (919 passed) |
| R1-F1 | Regression control: checkout reuse uses fresh snapshots (fixed in iteration 2) | clean | not touched this cycle; the eight `checkout_that_changes_*` orchestration cases pass on macOS and Windows |

### Progress
- starting the work on 'R1-F2 + R3-F1: hook L1 tests exceed the Windows time budget' at 05:42:00
        - defect class: a single L1 test asks several acceptance questions at once (a seed push plus a follow-up push, or a hook push plus the real recipe, or an auto push plus a strict push), so on native Windows it pays more process starts than 5 s allows; no single helper is at fault
        - delegated to one foreground rust-developer subagent (both findings share the class); it reported log items back to the orchestrator
        - subagent report, grouped:
                - measurements (build-win, full `repo-deps` run, 12 logical CPUs): a stub-gate `ci-push` push costs about 3.0 s without publication and 3.8 s with it; the real `ci-local` recipe run directly costs 2.6–3.3 s; seeding prior evidence through the real helpers in one interpreter costs about 0.8 s; so ci-push plus the real recipe has a floor of about 5.6 s and cannot be an L1 test on Windows
                - restructure: each question is asked at its own public boundary, as the review recommends: the real recipe's gate/report contract by running the real recipe directly (`the_real_recipe_stages_the_modeled_outcome_of_a_fed_plan` × 5, `the_real_recipe_skips_a_cell_its_plan_reuses`, `a_real_recipe_runs_its_helpers_under_the_selected_fallback_interpreter`); the hook's receipt/publication contract with a gate that stages what the recipe stages (`Evidence::model_gate`); the verifier's retry decision over evidence seeded through the real `record-cells` and `publish-receipt` helpers (`Evidence::record_evidence`)
                - pins between boundaries: every modeled gate outcome equals what the real recipe stages (five recipe cases); `seeded_evidence_is_what_a_hook_push_records` shows the seeded note is byte-identical to a hook push's note and the retained reports match; `the_gates_are_started_as_the_direct_recipe_runs_start_them` shows ci-push starts the gates as the direct runs do (argv, cwd, staging names, interpreter and NUL-terminated prefix)
                - launcher: the shared auto+strict helper was split into one push per test for every launch it serves (Direct, Git Bash, PowerShell): six tests
                - fixture: helper bytecode is compiled once inside the Python version check the fixture already makes (`PYTHONDONTWRITEBYTECODE` stops writing bytecode, not reading it; the folder is git-ignored)
                - renames: `a_real_recipe_interruption_withholds_earlier_reports` → `a_recipe_interruption_withholds_earlier_reports`; `a_real_recipe_gate_failure_records_a_complete_failure` → `a_recipe_gate_failure_records_a_complete_failure` (they no longer run the real recipe; a `real_` segment would also have left L1)
                - skills: `.claude/skills/os/windows.md` (per-push cost budget on build-win, bytecode reads); `.claude/skills/rust-devops/ci-cd.md` (its "seven `#[cfg(unix)]` parity tests" sentence was already stale; now describes the boundary split)
                - discovery: `just cross-check … -E 'test(...)'` breaks on the parentheses (already recorded in the os skill's build-hosts page); call `scripts/cross-check.sh` with a positional filter
                - Windows runs: run 1 still had five tests over 5 s (fallback end-to-end 7.07, advanced 6.36, docs-only 5.95, prior failure 5.52, equivalence 5.08); run 2 three marginal (5.01–5.11); run 3 (final) 908 passed, hook tests at most 4.85 s
        - orchestrator reconciliation:
                - read the restructured tests: each old assertion has a home (mapping in the ledger); the equivalence checks assert independently expected facts (byte equality of the note, exact retained-report trees, exact gate argv/env), not only "ran"
                - decision accepted, recorded for the author: no single test now runs a push through ci-push into the real recipe; the chain of pinned boundaries replaces it (the alternative, one ~6 s end-to-end test, would violate the L1 budget)
                - updated `implementation-log.md` parity rows 1, 2, 69–72 and the Windows launcher note to the new test names (and dropped their stale "(Unix only)" tags, removed in iteration 2)
                - confirmation run `just cross-check repo-deps --os windows` (tree `5fe3a319…`): 908 passed, 1 skipped; every hook test below 5 s; worst `seeded_evidence_is_what_a_hook_push_records` 4.97 s (4.85 s in the subagent's run)
                - left the 4.97 s test as it is: its question needs both a seed and a real hook push; trimming either removes the equivalence it proves; the thin margin is recorded as a risk below and in `human_review_items`
- work completed for 'R1-F2 + R3-F1: hook L1 tests exceed the Windows time budget' at 06:40:00

### Completion Report

- **outcome: complete**; elapsed about 61 minutes (05:39:57 → 06:41)
- parent findings: total 2 = fixed 2 (R1-F2, R3-F1) + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
- instance rows: fixed 13 (R1-F2.27.1–R1-F2.27.9, R3-F1.1, R3-F1.2, R3-F1.9, R3-F1.10 — the last split by the sweep although it was within budget); clean 10 (R3-F1.3–R3-F1.8, R3-F1.11–R3-F1.13, R1-F1 regression control)
- verified repairs:
        - every previously over-budget hook test is below 5 s on native Windows in two independent full `repo-deps` runs; no timeout, tier rename, skip, retry, or dropped assertion was used
        - the real recipe, the hook's publication, and the verifier are each tested at their own boundary, and three equivalence tests tie the boundaries together
- risks the next review should know:
        - thin Windows margins: `seeded_evidence_is_what_a_hook_push_records` 4.85–4.97 s, `an_advanced_non_main_target_reuses_prior_cells_and_publishes_exact_receipts` 4.67–4.70 s, `an_advanced_non_main_target_publishes_exact_receipts` 4.56 s
        - no single test pushes through ci-push into the real recipe (author decision item)
- open items: none
- not related to this class, over 5 s and unchanged: macOS `ci-build` `archive::tests::a_real_external_dynamic_dependency_is_discovered_and_refused_when_removed` (5.23 s) and `archive::tests::the_canonical_tier_recipes_run_the_fixture_archive_without_rebuilding_it` (7.63 s); on Windows 13 `ci-build` `archive::tests` and 2 `feature-attribution` tests; all run real Cargo builds
- no performance measurement was deferred; `deferred_perf_measurement` is not set; the Windows cross-check passed, so the spec's `os_failing` is not set
- changed files this cycle:
        - `scripts/ci-push/integration/parity.rs`, `scripts/ci-push/integration/evidence_fixture.rs`, `scripts/ci-push/integration/launcher.rs` (R1-F2, R3-F1)
        - `.claude/skills/os/windows.md`, `.claude/skills/rust-devops/ci-cd.md` (host facts learned; stale skill sentence)
        - `features/2026-11-01-rust-pre-push/implementation-log.md` (parity rows renamed), `review-4.md` (metadata), this log
- final verification (orchestrator, macOS, after the last code change):
        - `just test repo-deps`: 919 run, 919 passed, 1 skipped; 2 slow (the unrelated `ci-build` tests above); slowest hook test 1.56 s (R1-F2.*, R3-F1.*)
        - `cargo clippy -p repo-deps --all-targets -- -D warnings`: clean (lint; the `scripts` folder has no area justfile, so `just lint` there runs the root lint, which does not cover `repo-deps`)
        - `just check-tier-coverage scripts`: 0 stranded (new and split tests are selected by L1)
        - `BISCUIT_REQUIRE_JUST=1 BISCUIT_REQUIRE_JQ=1 BISCUIT_REQUIRE_BASH=1 python3 scripts/ci/test_ci_local.py`: 108 OK (recipe contract, unchanged this cycle)
        - `python3 scripts/ci/test_local_evidence.py`: 31 OK
        - `git diff --check`: clean
- final verification (Windows):
        - `just cross-check repo-deps --os windows` (orchestrator): 908 passed, 1 skipped, exit 0; per-row durations in the ledger
        - subagent's final run: 908 passed, 1 skipped; hook tests at most 4.85 s
