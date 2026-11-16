---
spec: 2026-11-14-shared-process-tree
implementation_1: "2026-11-16T12:00:17-08:00"
implementation_2: "2026-11-16T12:54:08-08:00"
---

# Shared process tree: review implementation log

## Implementation of Review Findings #1

> **started at:** 2026-11-16T12:00:17-08:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/fix-sequences/sniff/fixes/2026-11-14-shared-process-tree/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- the review's `implemented` flag was confirmed `false` before repairs began
- host: macOS, 16 cores, load average 11.6 at start

### Finding Ledger

| ID      | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| ------- | --------------------------------------------------------- | ----------- | -------------------------------- |
| R1-F1   | Graceful cleanup forgets descendants discovered before SIGTERM | fixed | all children verified below; review's public-API example prints `helper_alive=false` on all four lines; sniff `just test` 3634 passed, none slow |
| R1-F1.1 | Unix graceful cleanup; session helper first seen by the graceful scan → helper hard-stopped before `Ok(())` | fixed | `scan_targets` records found descendants before signaling; scripted `graceful_scan_retains_a_helper_reparented_before_the_hard_stop` (parent 1 and absent parent), recycled-PID control `graceful_scan_does_not_kill_a_recycled_helper_pid`, real process `graceful_cleanup_stops_a_detached_helper_started_after_the_last_sample` (0.38 s) pass; fail with the fix disabled |
| R1-F1.2 | Unix immediate cleanup; same new session helper → root and helper dead (control) | clean | immediate `terminate` kills found targets with SIGKILL in the same scan; example line 4 `helper_alive=false`; `immediate_terminate_kills_a_group_member_that_left_the_root_alone` pass |
| R1-F1.3 | Unix sampled-then-graceful → helper dead (control) | clean | example line 2 `helper_alive=false`; `an_observed_descendant_is_killed_after_reparenting` pass |
| R1-F1.4 | Unix graceful; helper stays in group → helper dead (control) | clean | example line 3 `helper_alive=false`; `graceful_cleanup_when_the_root_exits_but_a_descendant_ignores_sigterm` pass |
| R1-F1.5 | Windows graceful/immediate → Job boundary (control) | clean | source inspection: `windows.rs` `Core` holds only `job`, `root`, `armed`; `terminate`/`terminate_gracefully`/`Drop` call `TerminateJobObject`; no saved identity can be lost; no Windows runtime run (build-win storage preflight refused) |
| R1-F1.6 | Darkmatter editor cancellation and wait failure → inherits fix | fixed | `darkmatter/lib/src/editor/launch.rs:227` delegates to `terminate_gracefully`, no identity copy; `just test editor_launcher` 8 passed, none slow |
| R1-F1.7 | Claudine sequence task timeout, cancellation, runaway cap → inherits fix | fixed | `claudine/lib/src/composition/sequence/task/shell.rs:592` delegates; `just test-library shell_tasks` 26 passed; claudine `just test` 12246 passed, none slow |
| R1-F1.8 | Claudine isolated lifecycle cancellation and wait failure → inherits fix | fixed | `shell.rs:908` delegates (graceful), `:886` immediate; same claudine runs |
| R1-F1.9 | Sniff probe/install cleanup (immediate) → clean via R1-F1.2 | clean | `sniff/lib/src/process.rs` calls `terminate(..)` only; `just test process::` 54 passed |
| R1-F1.10 | Darkmatter login-shell query cleanup (immediate) → clean via R1-F1.2 | clean | `launcher.rs:130/142` call `terminate(Always/WhenObserved)`; `just test shell_expansion` 516 passed |
| R1-F1.11 | Armed drop after periodic sample → recorded identity survives | clean | `Drop` runs `terminate(WhenObserved)` over start-time-validated identities; `an_armed_drop_hard_stops_the_tree` pass |
| R1-F1.12 | Release and Unix foreground lifecycle exception → unowned survivors | clean | `release` only disarms and clears; `release_never_signals`, `release_leaves_a_member_running` pass |
| R1-F1.13 | (sweep) no sample yet: graceful scan could not walk from an unlearned root | fixed | root identity learned from the shutdown scan only when `try_wait` returned `Ok(None)` for the attached child; `graceful_scan_learns_an_unreaped_root_that_was_never_sampled`, control `graceful_scan_does_not_trust_a_root_it_cannot_prove_unreaped`, real `graceful_cleanup_stops_a_detached_helper_first_seen_by_its_scan` (0.12 s) pass; documented between-scans escape test still passes |
| R1-F1.14 | (sweep) failed group kill cleared identities before the armed drop needed them | fixed | `observed.clear()` moved after successful group kill; `failed_cleanup_keeps_scanned_identities_for_the_drop` pass, fails with the old ordering |
| R1-F2   | Generated-log terminal fixtures still fail their path assertions | fixed | `run_router` passes the absolute spec; `just test-l2 level2_initialize_generated_transclusion` 4 passed, none slow |
| R1-F2.1 | Proxied initialization phase one → full fixture passes, log excluded | fixed | pass (1.03 s, 1.15 s) |
| R1-F2.2 | Proxied initialization phase two → full fixture passes, log included | fixed | pass (1.03 s, 1.14 s) |
| R1-F2.3 | Direct initialization without shell approval (control) | clean | pass (0.72 s); document at `cwd`, effect path already absolute |
| R1-F2.4 | Sequence static preflight for generated include (control) | clean | pass (0.83 s); target at `cwd` |
| R1-F2.5 | Absolute spec argument (repair control) | fixed | now the committed fixture shape; both proxied rows pass with all assertions |
| R1-F2.6 | (sweep) other Claudine fixtures with relative `spec=` and derived-path assertions | clean | every other site types `spec` as `file(...)` (keeps caller anchor) or asserts no derived path; nearest shape `l1/compose_initialize_acceptance.rs:607` uses the frozen typed target |
| R1-F3   | Concurrent fixture builds exceed the Level 1 time budget | partially fixed | Repair runs below were clean, but Review 2 reproduced R1-F3.8–10 at 8.647–9.324 s in the full package; see the dated correction below |
| R1-F3.1 | `a_consumer_reports_its_extraction_apart_from_its_identity_checks` | fixed | 6.735 → archive 0.434 / cold 2.309; full 0.369 / 2.731; alone 0.294 |
| R1-F3.2 | `a_produced_fixture_verifies_including_its_own_archive_inventory` | fixed | 6.773 → 0.420 / 2.309; full 0.352 / 2.659; alone 0.288 |
| R1-F3.3 | `a_real_external_dynamic_dependency_is_discovered_and_refused_when_removed` | fixed | 7.019 → 3.087 / 2.973; full 4.141 / 3.777; alone 2.609; still builds fresh |
| R1-F3.4 | `an_archive_of_more_programs_than_the_plan_resolved_is_refused` | fixed | 6.121 → 0.372 / 2.157; full 0.461 / 2.390; alone 0.278 |
| R1-F3.5 | `an_archive_missing_a_declared_binary_is_refused_before_any_test_starts` | fixed | 6.730 → 0.402 / 2.259; full 0.455 / 2.414; alone 0.285 |
| R1-F3.6 | `the_canonical_tier_recipes_run_the_fixture_archive_without_rebuilding_it` | fixed | 2.944 / 3.065; full 4.148 / 3.709; alone 2.881 |
| R1-F3.7 | `the_archive_runs_every_tier_from_another_checkout_with_no_compiler_in_reach` | clean | 1.863 / 1.983; full 3.392 / 3.006 |
| R1-F3.8 | `a_combined_invocation_unifies_the_feature_graphs_the_producer_keeps_apart` | unresolved | 1.173 / 0.836; full 1.047 / 1.063; Review 2: full-package timing exceeds the limit; historical repair timings retained. |
| R1-F3.9 | `a_wrapped_fixture_build_compiles_exactly_the_predicted_configurations` | unresolved | full 3.787 / 3.803; Review 2: full-package timing exceeds the limit; historical repair timings retained. |
| R1-F3.10 | `a_forwarded_feature_what_if_matches_a_build_with_the_alignment_declared` | unresolved | full 4.109 / 3.806; Review 2: full-package timing exceeds the limit; historical repair timings retained. |
| R1-F3.11–25 | Other archive-producer consumers | clean | all ≤ 3.115 s in every final run (slowest F3.18 3.115, F3.24 2.947 in full run 1 at load 12→20) |

### Progress

- starting the work on 'R1-F1: graceful cleanup forgets descendants discovered before SIGTERM' at 12:03:10
        - defect class: a process identity discovered by one stage of shutdown is discarded before a later stage uses it to decide what to kill
        - orchestrator pre-read: `Core::scan_targets` (`sniff/lib/src/process/tree/unix.rs`) walks a fresh table but never records the `(pid, start)` pairs it found; the final `terminate` scan then has no path to a reparented helper
        - candidate sibling: before any sample, `root_start` is `None`, so the graceful scan cannot use the still-unreaped root as a traversal root either
        - subagent (rust-developer) report, 12:01–12:07:
                - confirmed the defect: `scan_targets` dropped the `(pid, start_time)` pairs it walked
                - repair: `scan_targets` now records every descendant it finds into `observed` before any signal; the graceful SIGTERM step was split into `request_exit` so scripted hosts can drive it
                - sibling R1-F1.13 (fixed): with no sample yet, `root_start` was `None`, so the graceful scan could not walk from the root; `terminate_gracefully` now passes `root_unreaped` (true only when `try_wait` returned `Ok(None)` for the attached child) and the scan learns the root identity from its own table; not an attachment-time scan; immediate `terminate` and the `try_wait` error branch never trust the root PID
                - sibling R1-F1.14 (fixed): `terminate` cleared `observed` before the group kill, so a failed cleanup's armed drop lost the identities; the clear now follows a successful group kill
                - 5 scripted-host tests, 2 controlled real-process tests, 1 `#[ignore]` re-exec fixture (same pattern as `process.rs` fixtures) added in `unix/tests.rs`
                - the new tests fail when the fixes are disabled (4 failures; 1 for the clear ordering); source restored
                - the review's public-API example printed `helper_alive=false` on all four lines; example deleted
                - `just test tree::` 98/98, `just test process::` 54/54, none slow; `just lint` and `cargo clippy -p sniff --all-targets -- -D warnings` pass
                - comments updated in `unix.rs` and `tree.rs`; `sniff/docs/topics/process-trees.md` graceful-cleanup paragraph and escape bullet updated
        - orchestrator diff inspection: changes confined to `unix.rs`, `unix/tests.rs`, `tree.rs` (doc), `process-trees.md`; no public signature change
- work completed for 'R1-F1: graceful cleanup forgets descendants discovered before SIGTERM' at 12:09:40
- starting the work on 'R1-F2: generated-log terminal fixtures still fail their path assertions' at 12:09:45
        - defect class: a fixture supplies a relative path whose meaning depends on the document's location while its assertion assumes a workspace-absolute meaning
        - subagent (rust-developer) report, 12:08–12:14:
                - `run_router` now passes `staged.fixture.cwd().join(SPEC_ARGUMENT)` (quoted with `sh_quote`); `LOG_RELATIVE` and every assertion unchanged; production path resolution unchanged
                - cause recorded in the `run_router` doc: the fixture target declares `spec: ""` (untyped text), so a relative value carries no caller provenance and `dirname(spec)` resolves beside the proxied document under `prompts/`
                - sibling sweep of `claudine/cli/tests` and `claudine/lib/tests` for relative `spec=` arguments, `prompts/` documents, and `dirname(spec)` derivations: every other site types `spec` as a `file(...)` (keeps caller anchor) or asserts no derived path; closest shape `l1/compose_initialize_acceptance.rs:607` is clean because the frozen shipped target types `spec`
                - `just test-l2 level2_initialize_generated_transclusion`: 4 passed (0.716–1.026 s), none slow, load 13.8
                - `just lint` (claudine): exit 0
- work completed for 'R1-F2: generated-log terminal fixtures still fail their path assertions' at 12:15:00
- starting the work on 'R1-F3: concurrent fixture builds exceed the Level 1 time budget' at 12:15:05
        - defect class: concurrent tests launch expensive child Cargo builds without sharing or scheduling, so individually fast tests exceed their budget together
        - subagent (rust-developer) report, 12:15–12:39:
                - baseline `just _test repo-deps --bin ci-build archive::tests::` (12:18:44, load 7.5→11.0): `🐌 L1 run recorded: 5 of 70` (R1-F3.1 6.735, F3.2 6.773, F3.3 7.019, F3.4 6.121, F3.5 6.730)
                - root cause 1: every `ci-build` test's `shipped_wrapper()` re-ran a no-op `cargo build` after the `build-ci-build` setup script had already built it; that no-op waited 3.2–3.3 s (intermittently) on the `target/debug` artifact lock, and every test of the run queued behind it
                - root cause 2: R1-F3.1, F3.2, F3.4, F3.5 only verify an archive (or a private copy whose manifest they edit) yet each compiled a private fixture, so seven fixture compiles started together
                - repair 1: the setup script now runs `just _ci_build_for_tests` (`just/devops.just`), which builds exactly as before and exports `BISCUIT_CI_BUILD_SHIPPED` via `$NEXTEST_ENV`; `wrapper_for_run` (`scripts/ci-build-tests.rs`) links that binary with no Cargo call, falling back to the old build when the path is absent
                - repair 2: the four inspection tests use `shared_fixture` again; `shared_fixture` now also copies the plan, so every file `verify` reads is a private copy in the test's scratch directory
                - fresh builds kept where compiling is asserted: R1-F3.3 (external native library), F3.6, F3.7, F3.8, F3.18–25
                - rejected: a bounded nextest test group (F3.3 still 6.1 s; the cost was lock wait, not CPU), disabling the host's compiler-cache daemon (hides it on one host), reusing nextest's own build (different features)
                - final archive scope (12:33:44, load 9.3–9.6): 70 passed, `🐢 none at or above 5.0s`, max 3.087; cold shared fixture (12:34:02): none ≥ 5 s, max 3.065
                - `just _test repo-deps` run 1 (12:34:40, load 12.4→20.5): 919 passed, none ≥ 5 s, max 4.148 (F3.6); run 2 cold (12:38:08, load 8.7→10.5): 919 passed, none ≥ 5 s, max 3.806
                - alone (load 17.3→14.9): F3.1 0.294, F3.2 0.288, F3.3 2.609, F3.4 0.278, F3.5 0.285, F3.6 2.881, F3.7 1.815
                - `just cross-check repo-deps --os wsl archive::tests::` (Linux archive mode, builder target hidden): 70 passed, none ≥ 5 s; not published (uncommitted tree)
                - `cargo clippy -p repo-deps --all-targets -- -D warnings` clean
                - lesson recorded as cause 10 in `.claude/skills/rust-testing/test-speed.md`
        - orchestrator review:
                - every CI path that runs these tests installs `just` (`_package-ci.yml` test and lint jobs, `_wsl-ci.yml` guest install), so the new setup command adds no missing tool; the `jq` step is guarded by `|| true` over the whole pipeline, and a missing path falls back to the previous per-test build
                - **risk recorded, not a finding:** commit `2eb6edf25` had moved the four `verify --workspace` tests off the shared fixture after an unexplained empty-output failure on ubuntu-latest's archive run; the shared fixture now copies the plan into each test's scratch directory (the suspected cause was the shared directory inside nextest's extraction directory), and WSL archive mode passes; the next pull-request CI run on ubuntu-latest is the proof
- work completed for 'R1-F3: concurrent fixture builds exceed the Level 1 time budget' at 12:41:30
- starting final reconciliation and phase gates at 12:41:35
        - Sniff `just test`: 3634 passed, 34 skipped, `🐢 L1 run recorded: 3634 test(s), none at or above 5.0s`
        - Sniff `just lint`: exit 0
        - Sniff `just check-tier-coverage sniff`: stranded 0
        - Darkmatter consumers: `just test editor_launcher` 8 passed, `just test shell_expansion` 516 passed, none slow
        - Claudine `just test-library shell_tasks` 26 passed; Claudine `just test` (12:42–12:46, load 8.5 → 44.8 at the end): 12246 passed, 15 skipped, none at or above 5.0s
        - `just cross-check repo-deps --os windows archive::tests::`: refused by build-win's Cargo storage preflight before any build; no Windows evidence (unavailable host, not a test failure; `os_failing` not set)
- final reconciliation completed at 12:48:30

### Completion Report

- **outcome: partially fixed after Review 2 verification**; elapsed 12:00:17 → 12:49 (about 49 minutes)
- parent findings after Review 2: total 3 = verified fixed 2 + partially fixed 1 + blocked 0 + unresolved 0 + disputed 0
        - required instances: 45 (R1-F1.1–14, R1-F2.1–6, R1-F3.1–25; the last fifteen share one ledger row); R1-F3.8–10 remain open after Review 2
- verified repairs
        - **R1-F1:** shutdown scans in `sniff/lib/src/process/tree/unix.rs` now record every descendant they find, with its start time, before signaling. The graceful scan also learns the root identity when `try_wait` has just proved the root unreaped (R1-F1.13). A failed group kill keeps the identities for the armed drop (R1-F1.14). Start-time validation and the documented between-scans escape are unchanged. The review's public-API example now reports every helper dead.
        - **R1-F2:** the Claudine L2 router fixture passes the absolute spec path; all four tests pass and every assertion is unchanged
        - **R1-F3:** repo-deps `ci-build` tests no longer re-run Cargo after the setup script (`BISCUIT_CI_BUILD_SHIPPED`), and the four archive-inspection tests use the shared immutable fixture again. The recorded repair runs in both scopes had no test at or above 5 s; Review 2 reproduced full-package timing failures in R1-F3.8–10.
- open item after Review 2: R1-F3.8–10 still exceed the full-package timing limit. Recorded risks, not additional findings:
        - the four shared-fixture `verify --workspace` tests once failed silently on ubuntu-latest's archive run (commit `2eb6edf25`); the next pull-request CI run on ubuntu-latest proves the private plan copy resolves it
        - native-Windows runtime evidence is still missing (build-win storage preflight); the Sniff change is `cfg(unix)`, and the setup recipe falls back to the previous per-test build if it cannot export the path
- changed files in this cycle
        - R1-F1: `sniff/lib/src/process/tree/unix.rs`, `sniff/lib/src/process/tree/unix/tests.rs`, `sniff/lib/src/process/tree.rs` (doc), `sniff/docs/topics/process-trees.md`
        - R1-F2: `claudine/cli/tests/level2/level2_initialize_generated_transclusion.rs`
        - R1-F3: `scripts/ci-build-archive-tests.rs`, `scripts/ci-build-tests.rs`, `just/devops.just`, `.config/nextest.toml`, `.claude/skills/rust-testing/test-speed.md`
- final verification
        - R1-F1: sniff `just test` 3634 passed, none slow; `just test process::` 54 passed; `just lint` and `cargo clippy -p sniff --all-targets -- -D warnings` clean; Darkmatter editor (8) and shell-expansion (516) and Claudine shell-task (26) consumers pass; Claudine `just test` 12246 passed, none slow
        - R1-F2: `just test-l2 level2_initialize_generated_transclusion` 4 passed, none slow; claudine `just lint` exit 0
        - R1-F3: `just _test repo-deps --bin ci-build archive::tests::` 70 passed, none ≥ 5 s (warm and cold); `just _test repo-deps` twice, 919 passed, none ≥ 5 s; `cargo clippy -p repo-deps --all-targets -- -D warnings` clean; WSL archive-mode cross-check 70 passed (unpublished)
- no commit was created; the spec stays in its active directory

## Review 2 record correction (2026-11-16)

The completion report and finding ledger said all three findings were fixed
and no items remained. That closure claim is false on independent verification;
the successful repair-run measurements remain historical evidence, not proof
that the contention was removed. Corrected those claims in place above.

`just _test repo-deps` (Nextest run
`fd2c842e-41a5-40ed-bb5e-74eda6d2322c`) ran 919 tests with all assertions
passing but three timing failures: R1-F3.8 9.171 s, R1-F3.9 8.647 s,
R1-F3.10 9.324 s. Immediately afterward `uptime` showed load averages
12.04 / 19.43 / 16.82 on 16 cores, not well above the core count.
Individual canonical runs measured 3.829 / 3.132 / 3.001 s respectively.
The 70-test archive scope was also clean (maximum 4.587 s). The remaining
problem is full-package contention, so R1-F3 is partially fixed, with the
same three previously enumerated instances still open. The ledger's other
historical timings have not been rewritten. Review 2 also checked the ledger's
instance count: its ranges enumerate 45 required instances, not 36.

## Implementation of Review Findings #2

> **started at:** 2026-11-16T12:54:08-08:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/wt/rusty-biscuit/fix-sequences/sniff/fixes/2026-11-14-shared-process-tree/review-2.md'
- this is iteration 2 of the review-to-implement cycle 
- the review's `implemented` flag was confirmed `false` before repairs began
- host: macOS, 16 cores, load averages 5.50 / 11.36 / 13.83 at start
- the review has one unblocked finding (R1-F3), no blocked findings and no `human_in_the_loop` items; R1-F1 and R1-F2 were verified fixed by the review and are carried here only as reconciliation rows

### Finding Ledger

| ID       | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| -------- | --------------------------------------------------------- | ----------- | -------------------------------- |
| R1-F1    | Graceful cleanup forgets descendants discovered before SIGTERM (verified fixed by review 2, all 14 children) | already fixed | review 2 verified all 14 children; nothing in Sniff, Darkmatter or Claudine changed this cycle; sniff `just test` after the final change: 3634 passed, none slow |
| R1-F2    | Generated-log terminal fixtures fail their path assertions (verified fixed by review 2, all 6 children) | already fixed | review 2 verified all 6 children through `just test-l2 level2_initialize_generated_transclusion`; no Claudine file changed this cycle |
| R1-F3    | Concurrent fixture builds exceed the Level 1 time budget | fixed | all 25 required children within budget, sweep rows .26-.29 clean; four consecutive full `just _test repo-deps` runs (`1d2f803a`, `4ff7226d`, `36b9e0d2`, `a9b79d78`): 919 passed, none at or above 5 s, slowest 3.63 s, load 13.7-17.5 on 16 cores; archive scope `5858b05d`: 70 passed, none slow |
| R1-F3.1  | Archive `a_consumer_reports_its_extraction_apart_from_its_identity_checks` ≤ 5 s in full package | clean | full run `4ff7226d` 0.378 s; archive 0.448 s |
| R1-F3.2  | Archive `a_produced_fixture_verifies_including_its_own_archive_inventory` ≤ 5 s | clean | full run `4ff7226d` 0.378 s; archive 0.437 s |
| R1-F3.3  | Archive `a_real_external_dynamic_dependency_is_discovered_and_refused_when_removed` ≤ 5 s | fixed | full run `4ff7226d` 3.465 s; archive 2.640 s; now in the `repo-deps-fixture-compiles` group |
| R1-F3.4  | Archive `an_archive_of_more_programs_than_the_plan_resolved_is_refused` ≤ 5 s | clean | full run `4ff7226d` 0.459 s; archive 0.496 s |
| R1-F3.5  | Archive `an_archive_missing_a_declared_binary_is_refused_before_any_test_starts` ≤ 5 s | clean | full run `4ff7226d` 0.464 s; archive 0.485 s |
| R1-F3.6  | Archive `the_canonical_tier_recipes_run_the_fixture_archive_without_rebuilding_it` ≤ 5 s | fixed | full run `4ff7226d` 2.961 s; archive 3.056 s; now in the group |
| R1-F3.7  | Archive `the_archive_runs_every_tier_from_another_checkout_with_no_compiler_in_reach` ≤ 5 s | fixed | full run `4ff7226d` 2.023 s; archive 2.153 s; now in the group |
| R1-F3.8  | Archive `a_combined_invocation_unifies_the_feature_graphs_the_producer_keeps_apart` ≤ 5 s in full package (was 9.171 s) | fixed | in the group; four full runs 0.851 / 0.794 / 0.773 / 0.797 s (was 5.577 s reproduced, 9.171 s in review 2); alone 0.507 / 0.534 s; archive 0.746 s |
| R1-F3.9  | Feature attribution `a_wrapped_fixture_build_compiles_exactly_the_predicted_configurations` ≤ 5 s in full package (was 8.647 s) | fixed | `wrapper_in` reads the setup script's `BISCUIT_CI_BUILD_SHIPPED` (no Cargo call), setup filter extended, in the group; four full runs 2.525 / 2.425 / 2.897 / 2.611 s (was 4.817 s reproduced, 8.647 s in review 2); alone 3.110 / 2.697 s before the change; fallback path without the variable passes under plain `cargo test` |
| R1-F3.10 | Feature attribution `a_forwarded_feature_what_if_matches_a_build_with_the_alignment_declared` ≤ 5 s in full package (was 9.324 s) | fixed | same repair, removing two redundant Cargo calls (one per wrapped build); four full runs 2.441 / 2.890 / 2.629 / 2.476 s (was 5.722 s reproduced, 9.324 s in review 2); alone 2.835 / 2.884 s before the change |
| R1-F3.11 | Archive `the_fixture_archive_carries_every_declared_payload_class` ≤ 5 s | clean | full run `4ff7226d` 0.156 s; archive 0.152 s |
| R1-F3.12 | Archive `a_refused_consumer_still_reports_what_its_transfer_cost` ≤ 5 s | clean | full run `4ff7226d` 0.093 s; archive 0.117 s |
| R1-F3.13 | Archive `a_tampered_fixture_archive_is_refused_with_a_stable_code_and_exit_status` ≤ 5 s | clean | full run `4ff7226d` 0.122 s; archive 0.122 s |
| R1-F3.14 | Archive `a_missing_sidecar_is_refused_end_to_end` ≤ 5 s | clean | full run `4ff7226d` 0.009 s; archive 0.011 s |
| R1-F3.15 | Archive `an_archive_offered_to_the_wrong_environment_is_refused_end_to_end` ≤ 5 s | clean | full run `4ff7226d` 0.117 s; archive 0.138 s |
| R1-F3.16 | Archive `the_manifest_records_the_workspace_the_producer_compiled_at` ≤ 5 s | clean | full run `4ff7226d` 0.115 s; archive 0.104 s |
| R1-F3.17 | Archive `consumer_refuses_a_different_source_tree_even_with_a_recomputed_manifest_digest` ≤ 5 s | clean | full run `4ff7226d` 0.161 s; archive 0.165 s |
| R1-F3.18 | Archive `failed_and_refused_keys_report_themselves_and_let_an_unrelated_key_finish` ≤ 5 s | fixed | full run `4ff7226d` 2.649 s; archive 2.338 s; now in the group |
| R1-F3.19 | Archive `a_key_narrowed_leg_produces_only_its_own_record` ≤ 5 s | clean | full run `4ff7226d` 0.408 s; archive 0.387 s |
| R1-F3.20 | Archive `every_record_attempt_is_sampled_and_a_failing_sampler_changes_no_outcome` ≤ 5 s | clean | full run `4ff7226d` 0.414 s; archive 0.372 s |
| R1-F3.21 | Archive `the_hosted_wrapper_samples_each_record_through_the_shipped_sampler` ≤ 5 s | clean | full run `4ff7226d` 0.369 s; archive 0.350 s |
| R1-F3.22 | Archive `the_owner_wrapper_leaves_owner_timing_to_the_coordinator_in_record_mode` ≤ 5 s | clean | full run `4ff7226d` 0.052 s; archive 0.047 s |
| R1-F3.23 | Archive `a_sampler_that_cannot_start_leaves_a_successful_owner_successful` ≤ 5 s | clean | full run `4ff7226d` 0.138 s; archive 0.181 s |
| R1-F3.24 | Archive `one_owner_tree_shares_a_dependency_compile_without_unifying_features` ≤ 5 s | fixed | full run `4ff7226d` 2.567 s; archive 2.191 s; now in the group |
| R1-F3.25 | Archive `produce_accepts_the_planned_head_and_refuses_the_merge_revision` ≤ 5 s | fixed | full run `4ff7226d` 2.426 s; archive 2.049 s; now in the group |
| R1-F3.26 | Sweep: ci-build `tests::two_tier_configurations_compile_the_same_package_twice_and_are_reported_apart` (compiles a fixture package) ≤ 5 s | clean | added to the group as a compiling test; 1.121 s in the reproduction run, below 5 s in all four repaired full runs |
| R1-F3.27 | Sweep: ci-build `tests::cargo_drives_the_shipped_wrapper_and_a_warm_rebuild_records_no_work` (compiles a fixture package) ≤ 5 s | clean | added to the group; 0.681 s in the reproduction run, below 5 s in all repaired runs |
| R1-F3.28 | Sweep: feature-attribution tests that only resolve with `cargo tree` (no compile; 0.7-1.8 s) | clean | not compilers, deliberately kept out of the group (review: do not serialize cheap inspections); at most 1.45 s in repaired full runs |
| R1-F3.29 | Sweep: archive tests named by helper search but refused before compiling (`a_record_planned_for_another_compiler_host_…`, `a_record_planned_for_a_target_…`, `a_key_this_producer_does_not_own_…`, `an_example_include_naming_no_target_…`, `tracked_source_changes_…`) | clean | none at or above 5 s in any repaired run; kept out of the group |

### Progress

- starting the work on 'R1-F3: concurrent fixture builds exceed the Level 1 time budget' at 12:58:00
        - the orchestrator works this single, configuration-scoped finding directly rather than through a subagent: it is one finding whose repair is in shared test scheduling (`.config/nextest.toml`) and the two repo-deps test binaries, and the orchestrator already holds that context; no parallel subagents means no shared-host test contention
        - defect class: tests that each start their own compiler toolchain are scheduled with no bound on how many run at once, so their builds compete for CPU and Cargo locks and each exceeds the budget only in a package run
        - reproduced first: full `just _test repo-deps` at load 5.7 gave 919 passed with **2 slow** (R1-F3.8 5.577 s, R1-F3.10 5.722 s; R1-F3.9 4.817 s)
        - measured alone through the canonical recipe: R1-F3.8 0.507 / 0.534 s, R1-F3.9 3.110 / 2.697 s, R1-F3.10 2.835 / 2.884 s; R1-F3.8 is pure contention
        - phase timing of R1-F3.9 alone, outside Nextest: six wrapped fixture `cargo test --no-run` builds 1.9-2.1 s (the behavior under test), six `cargo tree` resolutions 1.26 s (the tool's own `collect`), `cargo metadata` 0.1 s; the time alone is inherent, so the overrun is contention between the compiling tests
        - redundant build found: feature attribution's `wrapper_in` ran its own `cargo build` of `ci-build` once per wrapped build (twice in R1-F3.10), outside the setup script's reach; a no-op of 0.3 s on a quiet tree that still queues on the scripts target directory's lock behind every other fixture build
        - repair 1 (`scripts/feature-attribution-tests.rs`): `wrapper_in` links the setup script's `BISCUIT_CI_BUILD_SHIPPED` binary when it names an existing file, and keeps the Cargo build as its fallback; the fallback still passes under plain `cargo test` with the variable unset (1 passed)
        - repair 2 (`.config/nextest.toml`): the `build-ci-build` setup script now also covers feature attribution's two wrapped-build tests; a new `repo-deps-fixture-compiles` test group (`max-threads = 4`) holds exactly the eleven tests that compile a fixture workspace across both binaries (`cargo nextest show-config test-groups` lists all eleven and nothing else); cheap inspections, `cargo tree`-only tests and shared-fixture readers are not in it
        - decision: a group of four, rather than one or two; it keeps every member near its time alone while the package's wall-clock stayed 10.6-11.0 s (10.9 s before); no limit raised, no retry, no rename, no ignored test, no assertion removed; fresh compilation kept wherever it is the behavior under test
        - sweep (defect class: tests that each start their own compiler scheduled with no bound on concurrency): every test in `ci-build-archive-tests.rs`, `ci-build-tests.rs` and `feature-attribution-tests.rs` that reaches a producer, a wrapped build or a direct Cargo call was checked against measured timings; ledger rows .26-.29
        - documentation: `.claude/skills/rust-testing/test-speed.md` gains section 11 describing the pattern and the group
        - cross-OS: the change is test scheduling plus one test helper that reads an environment variable and uses `EXE_SUFFIX` as before; no OS-specific code, so no `just cross-check` was run
- work completed for 'R1-F3: concurrent fixture builds exceed the Level 1 time budget' at 13:00:27

### Completion Report

- outcome: **complete**; elapsed about 7 minutes (12:54:08 to 13:01)
- parent findings: total 3 = fixed 1 (R1-F3) + already fixed 2 (R1-F1, R1-F2) + blocked 0 + unresolved 0 + disputed 0
- required instances: R1-F3.1-.25 all within budget (9 fixed by the repair: .3, .6, .7, .8, .9, .10, .18, .24, .25; 16 clean); sweep rows R1-F3.26-.29 clean; R1-F1.1-.14 and R1-F2.1-.6 carried as verified by review 2
- open findings: none
- changed files:
        - `scripts/feature-attribution-tests.rs`: `wrapper_in` reuses the setup script's `ci-build` (R1-F3.9, R1-F3.10)
        - `.config/nextest.toml`: setup-script filter extended; `repo-deps-fixture-compiles` group and override (R1-F3.3, .6, .7, .8, .9, .10, .18, .24, .25, .26, .27)
        - `.claude/skills/rust-testing/test-speed.md`: section 11
        - this log and the review's frontmatter
- final verification, after the last code change:
        - `just _test repo-deps`, four consecutive runs: 919 passed, 1 skipped, `🐢 none at or above 5.0s` each time (R1-F3)
        - `just _test repo-deps --bin ci-build archive::tests::`: 70 passed, none slow, maximum 3.056 s (R1-F3, archive scope)
        - `cargo test -p repo-deps --bin feature-attribution a_wrapped_fixture_build_compiles` with `BISCUIT_CI_BUILD_SHIPPED` unset: 1 passed (fallback path)
        - `just _lint repo-deps`: exit 0
        - `cd sniff && just test`: 3634 passed, 34 skipped, none at or above 5.0 s (R1-F1 reconciliation; no Sniff source changed)
- no performance measurement was deferred
