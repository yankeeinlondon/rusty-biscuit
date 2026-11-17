---
$schema: feature-review.yaml
ready: false
findings:
  - "R1-F3 (high): concurrent fixture builds exceed the Level 1 time budget"
human_review: false
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: true
reviewed_by: codex/gpt-6.1-sol
created: 2026-11-16T12:48:51-08:00
spec: 2026-11-14-shared-process-tree/spec.md
implemented: true
description: "A **fix** review of `2026-11-14-shared-process-tree/spec.md`"
fix: 2026-11-14-shared-process-tree/review-2.md
previous: 2026-11-14-shared-process-tree/review-1.md
log: /Volumes/coding/wt/rusty-biscuit/fix-sequences/sniff/fixes/2026-11-14-shared-process-tree/log.md
implemented_by: claude/opus
next: 2026-11-14-shared-process-tree/review-3.md
---

# Review 2: shared process-tree ownership

The fix is **not production ready**. Graceful cleanup now retains the helper
identities it needs, and the generated-log terminal fixtures pass. The repair
also removes redundant builds and shares archive fixtures, but three previously
listed compiler-consuming tests still exceed the Level 1 time budget when run
with their package. There are no new findings or decisions needing an author.

## Previous Finding Verification

The previous review and the repair's entire `### Finding Ledger` were read
before checking the implementation. There are no earlier reviews or human
rulings in this directory. The additional instances identified during repair
are included below, rather than assuming the parent completion flags prove them.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-F1: graceful cleanup forgets descendants discovered before SIGTERM | R1-F1.1–12 from Review 1, plus repair-discovered R1-F1.13–14 | verified fixed | All fourteen sites checked below; 54 Sniff process tests pass, including both controlled detached-helper regressions and the injected identity/failure controls; consumer delegations checked | Nothing |
| R1-F2: generated-log terminal fixtures still fail their path assertions | R1-F2.1–5 from Review 1, plus repair sweep R1-F2.6 | verified fixed | Absolute spec argument is present; canonical Claudine L2 recipe runs all four tests successfully with the original path/content assertions | Nothing |
| R1-F3: concurrent fixture builds exceed the Level 1 time budget | R1-F3.1–25 | partially fixed | Full repo-deps run: 919 assertion passes, three timing failures. Archive run: 70 pass, none slow. All three slow tests pass below four seconds alone. Complete instance table below | R1-F3.8, R1-F3.9, R1-F3.10 still contend in the full package |

### R1-F1 instance verification

The Sniff library's [Unix owner](../../lib/src/process/tree/unix.rs) now records
every newly discovered descendant before signaling it. Its final hard cleanup
can therefore still find a helper after the root exits and the helper's parent
changes. The additional root-identity learning is limited to a matching direct
child that `try_wait` proved is still unreaped; it adds no attachment-time scan.
Saved identities survive a failed group kill for the destructor's next attempt.

All tests named here are in Sniff's [Unix owner tests](../../lib/src/process/tree/unix/tests.rs)
unless a consumer package is specified. `just test process::` ran 54 tests,
none at or above five seconds. Controlled processes exercise the public owner;
scripted snapshots additionally pin identity reuse and failure behavior.

| Child ID | Site / shape checked | Evidence and result |
| --- | --- | --- |
| R1-F1.1 | Graceful scan first discovers a session-detached helper | `graceful_scan_retains_a_helper_reparented_before_the_hard_stop` covers parent 1 and absent parent; public `graceful_cleanup_stops_a_detached_helper_started_after_the_last_sample` passes, helper gone |
| R1-F1.2 | Immediate cleanup | `immediate_terminate_kills_a_group_member_that_left_the_root_alone` passes; newly found targets are killed within that scan |
| R1-F1.3 | Previously observed descendant after reparenting | `an_observed_descendant_is_killed_after_reparenting` passes; matching start time retains eligibility |
| R1-F1.4 | Helper stays in the original group and ignores SIGTERM | `graceful_cleanup_when_the_root_exits_but_a_descendant_ignores_sigterm` passes |
| R1-F1.5 | Windows immediate/graceful owner | [Windows owner](../../lib/src/process/tree/windows.rs) uses one retained Job handle and `TerminateJobObject`; there is no descendant-identity list to lose. Structural control, no native-Windows runtime claim |
| R1-F1.6 | Darkmatter editor cancellation / wait failure | [Editor launcher](../../../darkmatter/lib/src/editor/launch.rs) delegates both to shared graceful cleanup; `just test editor_launcher`: 8 pass, none slow |
| R1-F1.7 | Claudine task timeout / cancellation / runaway-output cap | [Task shell](../../../claudine/lib/src/composition/sequence/task/shell.rs) delegates all three to shared graceful cleanup; `just test-library shell_tasks`: 26 pass, none slow |
| R1-F1.8 | Claudine isolated lifecycle cancellation / wait failure | Same file's `terminate_lifecycle_tree` delegates both to shared graceful cleanup; no caller-owned identity state |
| R1-F1.9 | Sniff detection / install cleanup | [Bounded launcher](../../lib/src/process.rs) uses shared immediate termination; process suite passes including timeout, output and inherited-pipe cases |
| R1-F1.10 | Darkmatter login-shell cleanup | [Query launcher](../../../darkmatter/lib/src/markdown/compose/shell_expansion/launcher.rs) uses shared immediate termination; focused launcher run: 4 pass, none slow; final-scan policies and bounded reader wait retained |
| R1-F1.11 | Armed drop with saved observations | `an_armed_drop_hard_stops_the_tree` passes; destructor uses identity-validated immediate cleanup |
| R1-F1.12 | Release / Unix foreground lifecycle exception | `release_never_signals` and `release_leaves_a_member_running` pass; Claudine foreground exception remains direct-child-only as the spec requires |
| R1-F1.13 | Graceful shutdown before any periodic sample | `graceful_scan_learns_an_unreaped_root_that_was_never_sampled` and public `graceful_cleanup_stops_a_detached_helper_first_seen_by_its_scan` pass; `graceful_scan_does_not_trust_a_root_it_cannot_prove_unreaped` and recycled-root controls pass |
| R1-F1.14 | Group kill fails after discovering a helper | `failed_cleanup_keeps_scanned_identities_for_the_drop` passes; saved helper survives in owner state until the drop retries |

`graceful_scan_does_not_kill_a_recycled_helper_pid` also passes, confirming that
the repair retains identities without treating a reused PID as the same helper.
The intentional between-scans escape test remains passing. It is an accepted
limit, not work reopened by this review.

### R1-F2 instance verification

Claudine's [router fixture](../../../claudine/cli/tests/level2/level2_initialize_generated_transclusion.rs)
now passes `staged.fixture.cwd().join(SPEC_ARGUMENT)` through shell quoting.
The production path rules and the expected log location are unchanged.
`just test-l2 level2_initialize_generated_transclusion` ran all four tests
through the real-terminal harness, each in 1.005–1.316 seconds.

| Child ID | Required instance | Verification |
| --- | --- | --- |
| R1-F2.1 | Proxied phase one: log exists beside spec but is excluded from body | Pass, 1.314 s; original assertions retained |
| R1-F2.2 | Proxied phase two: log exists beside spec and is included in body | Pass, 1.316 s; original assertions retained |
| R1-F2.3 | Direct initialization without approval: no effect / provider launch | Pass, 1.005 s |
| R1-F2.4 | Sequence preflight refuses the generated include before effects | Pass, 1.113 s |
| R1-F2.5 | Absolute spec argument repair control | Now the normal fixture input for both passing proxied rows |
| R1-F2.6 | Other relative spec / derived-path fixtures | Repair sweep inspected; typed `file(...)` inputs preserve caller anchoring, including `compose_initialize_acceptance`; no second changed untyped router fixture found |

## Unblocked Findings

### R1-F3 (high): concurrent fixture builds exceed the Level 1 time budget

**Authority: defect.** The review's explicit “A Slow Test Is a Failing Test”
contract makes an L1 test over five seconds fail on a host without substantial
overload. The ordinary `just _test repo-deps` invocation still reaches three
previously listed tests at 8.647–9.324 seconds. These are normal package-test
inputs, not a manufactured payload or an expanded production requirement.

**Defect class:** compiler-consuming fixtures contend when run together, so
tests that are individually within budget exceed it in the full package.

The repair correctly removes a redundant Cargo invocation using the setup
script's binary path, and four archive inspection tests now reuse an immutable
fixture with private copies of their workspace, plan, and manifest. Those
improvements are verified. They do not resolve all instances of the earlier
finding: the combined-build and both feature-attribution fixtures remain slow.

The full run was Nextest `fd2c842e-41a5-40ed-bb5e-74eda6d2322c`: 919 assertion
passes, **three timing failures**, 14.546 seconds of execution. Immediately
afterward `uptime` reported load averages 12.04 / 19.43 / 16.82 on 16 cores;
this is not well above the core count and does not qualify for the overloaded
host allowance. Each slow test was then measured alone through the canonical
recipe. No production or fixture changes were made during these measurements.

The table covers every original instance. “Archive” means a subsequent
`just _test repo-deps --bin ci-build archive::tests::` run: all 70 pass,
none slow, maximum 4.587 seconds. “Full” means the full 919-test run above;
all rows other than .8–10 were below five seconds. For clean rows the archive
timing is shown as the more compact individual evidence. All rows expect their
assertions to hold and their concurrent runtime to remain at most five seconds.

| Child ID | Site / fixture shape tested | Observed result | Expected result |
| --- | --- | --- | --- |
| R1-F3.1 | Archive `a_consumer_reports_its_extraction_apart_from_its_identity_checks`; private copy | Archive 0.430 s; full below 5 s | Same; clean |
| R1-F3.2 | Archive `a_produced_fixture_verifies_including_its_own_archive_inventory`; private copy | Archive 0.416 s; full below 5 s | Same; clean |
| R1-F3.3 | Archive `a_real_external_dynamic_dependency_is_discovered_and_refused_when_removed`; fresh native-library build | Full 4.124 s; archive 3.197 s | Same; clean |
| R1-F3.4 | Archive `an_archive_of_more_programs_than_the_plan_resolved_is_refused`; edited private manifest | Archive 0.413 s; full below 5 s | Same; clean |
| R1-F3.5 | Archive `an_archive_missing_a_declared_binary_is_refused_before_any_test_starts`; edited private manifest | Archive 0.392 s; full below 5 s | Same; clean |
| R1-F3.6 | Archive `the_canonical_tier_recipes_run_the_fixture_archive_without_rebuilding_it`; fresh build | Full 4.071 s; archive 3.535 s | Same; clean |
| R1-F3.7 | Archive `the_archive_runs_every_tier_from_another_checkout_with_no_compiler_in_reach`; fresh build | Archive 2.594 s; full below 5 s | Same; clean |
| R1-F3.8 | Archive `a_combined_invocation_unifies_the_feature_graphs_the_producer_keeps_apart`; combined Cargo build | **Full 9.171 s**; alone 3.829 s; archive 4.587 s | At most 5 s in full package; unresolved |
| R1-F3.9 | Feature attribution `a_wrapped_fixture_build_compiles_exactly_the_predicted_configurations`; fresh instrumented build | **Full 8.647 s**; alone 3.132 s | At most 5 s in full package; unresolved |
| R1-F3.10 | Feature attribution `a_forwarded_feature_what_if_matches_a_build_with_the_alignment_declared`; original and aligned builds | **Full 9.324 s**; alone 3.001 s | At most 5 s in full package; unresolved |
| R1-F3.11 | Archive `the_fixture_archive_carries_every_declared_payload_class`; shared fixture | Archive 0.107 s; full below 5 s | Same; clean |
| R1-F3.12 | Archive `a_refused_consumer_still_reports_what_its_transfer_cost`; shared fixture | Archive 0.124 s; full below 5 s | Same; clean |
| R1-F3.13 | Archive `a_tampered_fixture_archive_is_refused_with_a_stable_code_and_exit_status`; private corrupted payload | Archive 0.149 s; full below 5 s | Same; clean |
| R1-F3.14 | Archive `a_missing_sidecar_is_refused_end_to_end`; private missing sidecar | Archive 0.129 s; full below 5 s | Same; clean |
| R1-F3.15 | Archive `an_archive_offered_to_the_wrong_environment_is_refused_end_to_end`; incompatible environment | Archive 0.118 s; full below 5 s | Same; clean |
| R1-F3.16 | Archive `the_manifest_records_the_workspace_the_producer_compiled_at`; shared fixture | Archive 0.119 s; full below 5 s | Same; clean |
| R1-F3.17 | Archive `consumer_refuses_a_different_source_tree_even_with_a_recomputed_manifest_digest`; private edited source | Archive 0.149 s; full below 5 s | Same; clean |
| R1-F3.18 | Archive `failed_and_refused_keys_report_themselves_and_let_an_unrelated_key_finish`; custom build | Archive 2.316 s; full below 5 s | Same; clean |
| R1-F3.19 | Archive `a_key_narrowed_leg_produces_only_its_own_record`; custom build | Archive 0.342 s; full below 5 s | Same; clean |
| R1-F3.20 | Archive `every_record_attempt_is_sampled_and_a_failing_sampler_changes_no_outcome`; custom producer | Archive 0.260 s; full below 5 s | Same; clean |
| R1-F3.21 | Archive `the_hosted_wrapper_samples_each_record_through_the_shipped_sampler`; custom producer | Archive 0.279 s; full below 5 s | Same; clean |
| R1-F3.22 | Archive `the_owner_wrapper_leaves_owner_timing_to_the_coordinator_in_record_mode`; custom producer | Archive 0.043 s; full below 5 s | Same; clean |
| R1-F3.23 | Archive `a_sampler_that_cannot_start_leaves_a_successful_owner_successful`; custom producer | Archive 0.185 s; full below 5 s | Same; clean |
| R1-F3.24 | Archive `one_owner_tree_shares_a_dependency_compile_without_unifying_features`; fresh builds | Archive 2.152 s; full below 5 s | Same; clean |
| R1-F3.25 | Archive `produce_accepts_the_planned_head_and_refuses_the_merge_revision`; custom build | Archive 2.438 s; full below 5 s | Same; clean |

The sweep ran every L1 test in repo-deps, not just the original slow rows.
The remaining archive tests were clean too; the slow real-package archive test
remains outside L1 by its existing declared tier. No assertions were removed.

**Recommended smallest repair:** keep the setup-path and shared-fixture fixes.
Measure the child-build waits in the three remaining sites, then remove any
redundant build or give the actual compiler-consuming tests a bounded shared
Nextest concurrency group across both binaries. The affected sources are
repo-deps' [combined-build test](../../../scripts/ci-build-archive-tests.rs:2581)
and [feature-attribution build tests](../../../scripts/feature-attribution-tests.rs:684).
Keep fresh compilation where it is the behavior under test. Verify both archive
and full-package scopes; a clean archive-only run does not prove the two
binaries coexist within budget. Do not raise the limit, retry, rename tiers,
ignore tests, or serialize unrelated cheap inspections. This needs no public
API or author decision.

## Blocked Findings

None.

## Recurrence

R1-F3 repeats Review 1's compiler-fixture contention class. Instances .8–10
were already enumerated there and retained in the repair ledger. These are
**unchanged required instances**, not newly discovered siblings, and there is
no evidence that the repair introduced their contention. Earlier successful
repair runs do not supersede this independent failing full-package run. The
current review carries all 25 original instances, including the clean ones;
no additional machinery or broader containment requirement is being requested.

## Requirement coverage and verification levels

The review inspected the shared owner, migrated launch/wait/cleanup paths,
platform tests, canonical topic, and repair changes. The earlier full-area
implementation runs remain historical evidence; the table distinguishes this
review's execution from source inspection and recorded evidence.

| Requirement | Appropriate level | Verification |
| --- | --- | --- |
| Configure, attach, group/session modes, fast exit, foreign-group rejection | L1 | Sniff process suite passes; ownership is checked before readers are started |
| Deferred sampling, identity-safe traversal, zero attachment scans | L1 | Fake-host scan counters and recycled-PID controls pass; first sample still due at 250 ms |
| Immediate/graceful cleanup, root exits first, detached helpers, release, armed drop, failed cleanup backstop | L1 | Sniff's 54 process tests pass, including public controlled helpers and new shutdown-scan controls |
| Known unobserved Unix escape and bounded pipe settlement | L1 | Existing escape and inherited-pipe cases pass; accepted limit retained in canonical documentation |
| Windows assignment before resume; setup, resume, release and cleanup failures | L1 on native Windows | Source/test inspection: suspension flag combined once, Job assigned before thread resume, handles owned immediately; existing injected failure and controlled-process tests remain declared. No new runtime evidence claimed; cross-OS proof remains CI's responsibility |
| Darkmatter editor arguments, streams, cancellation, SIGTERM resistance, external GUI survival | L1 | 8 editor-launcher tests pass, none slow; callers preserve their release/warning policy |
| Darkmatter editor foreground access and restoration | L2 | `just test-l2 level2_edit_action`: both real-terminal tests pass (0.854 / 0.928 s), verifying input and restoration after interruption |
| Darkmatter login-shell session, cleanup and output bounds | L1 plus existing terminal coverage | Shared-owner delegation and session setup inspected; `just test shell_expansion::launcher`: 4 pass (0.025–0.345 s), none slow; previous recorded terminal coverage remains applicable |
| Claudine timeout, cancellation, background descendants, output caps, error categories | L1 | 26 shell-task tests pass, none slow; isolated lifecycle path shares the corrected owner |
| Generated-log composition inside the terminal | L2 | 4 generated-transclusion tests pass through canonical real-terminal recipe |
| Local API without HTTP dependencies | Compile / dependency inspection | No dependency change in repair; earlier no-feature Sniff build and no-reqwest graph remain applicable; owner uses local OS APIs only |
| Duplicate removal and canonical documentation | Source/document inspection | Consumers delegate group/session setup, descendant tracking and Job work to Sniff; detached-child and foreground/provider exceptions retained; topic covers operations, errors, lifecycle and limits |
| Required test runtime | L1 concurrent package run | R1-F3 remains red despite 919 assertion passes |

No new file-format/configuration reader or field validator was introduced by
these changes, so the input robustness matrix does not apply. Reading Cargo's
compiler-artifact output to locate the setup build is a tool-output lookup,
not a configuration language change. No requirement here depends on the
terminal's physical keyboard encoder, so L3 injection is unnecessary.

The new Unix tests compile under the existing library test target, and the
unprefixed regression names run in L1. The ignored re-execution helper is
explicitly spawned by those tests; it is not an ignored acceptance test.
`just check-tier-coverage sniff` reports zero stranded tests.

## Implementation record corrections

Corrected the repair record `log.md` in place: R1-F3 is partially fixed,
instances .8–10 remain unresolved, and the completion summary no longer claims
all findings are closed. Added a dated note with the full-run ID, individual
timings and load reading. Preserved the earlier successful repair-run timings
as historical observations. Also corrected the count of individually required
instances from 36 to 45. These factual record corrections are not additional
findings or human-review items. The spec's behavior was left unchanged.
