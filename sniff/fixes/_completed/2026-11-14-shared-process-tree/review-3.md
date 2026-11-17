---
$schema: feature-review.yaml
ready: true
findings: []
human_review: false
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: false
reviewed_by: codex/gpt-6.1-sol
created: 2026-11-16T13:01:30-08:00
spec: 2026-11-14-shared-process-tree/spec.md
implemented: false
description: "A **fix** review of `2026-11-14-shared-process-tree/spec.md`"
fix: 2026-11-14-shared-process-tree/review-3.md
previous: 2026-11-14-shared-process-tree/review-2.md
---

# Review 3: shared process-tree ownership

The fix is **production ready**. All previous findings are verified fixed,
including the three compiler fixtures that remained slow in Review 2. No new
findings, blocked work, or author decisions were identified.

## Previous Finding Verification

Read both earlier reviews and both repair ledgers before verification. Neither
earlier review contains human rulings. The completion flags were treated as
claims: the tables below record source checks and independent executions.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-F1: graceful cleanup forgets descendants discovered before SIGTERM | R1-F1.1–14 | verified fixed | Unix implementation and all consumer delegations inspected; 54 Sniff process tests, 8 editor tests, 4 shell-query tests and 26 shell-task tests pass; instance checks below | Nothing |
| R1-F2: generated-log terminal fixtures still fail their path assertions | R1-F2.1–6 | verified fixed | Absolute spec argument and retained assertions checked; all four tests pass through the canonical real-terminal recipe | Nothing |
| R1-F3: concurrent fixture builds exceed the Level 1 time budget | R1-F3.1–25, plus repair sweep R1-F3.26–29 | verified fixed | Full repo-deps run: 919 pass, none at or above 5 s; archive run: 70 pass, none at or above 5 s; all rows checked below | Nothing |

### Process-tree repair: R1-F1

In Sniff's [Unix owner](../../lib/src/process/tree/unix.rs), `scan_targets`
retains discovered identities before either shutdown stage can signal them.
Only a matching, unreaped direct child permits learning a previously unknown
root identity during graceful shutdown. Traversal and individual signals use
validated saved identities; failed hard group cleanup leaves those identities
available to the destructor. Successful cleanup disarms the owner.

`cd sniff && just test process::` independently ran 54 tests, all passing,
with none at or above five seconds. This includes controlled subprocess
regressions through the public owner and scripted identity/failure cases.

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

The table's named tests were rerun in this review. Consumer runs were also
repeated: `just test editor_launcher` (8), `just test shell_expansion::launcher`
(4), and `just test-library shell_tasks` (26). All passed below five seconds.
The Windows row is source verification, not a native-Windows execution claim.
No code in this repair changes that backend. Cross-OS runtime evidence remains
CI's responsibility and is not an open finding under this review's rules.

### Terminal-fixture repair: R1-F2

The Claudine [router fixture](../../../claudine/cli/tests/level2/level2_initialize_generated_transclusion.rs)
passes the absolute staged spec path through shell quoting. Its log-location
and body assertions remain intact. The typed-file controls retain their
original path anchoring; production path resolution was not changed.

| Child ID | Required instance | Verification |
| --- | --- | --- |
| R1-F2.1 | Proxied phase one creates the log beside the spec and excludes it from the body | Canonical L2 run passes, 1.322 s |
| R1-F2.2 | Proxied phase two includes the created log | Canonical L2 run passes, 1.323 s |
| R1-F2.3 | Direct initialization without approval produces no effect | Canonical L2 run passes, 0.996 s |
| R1-F2.4 | Sequence preflight refuses a generated include before effects | Canonical L2 run passes, 1.101 s |
| R1-F2.5 | Absolute spec argument repair control | Present in the shared router helper used by both passing proxied cases |
| R1-F2.6 | Other relative spec and derived-path fixtures | Typed `file(...)` controls preserve caller anchoring; inspected the neighboring `compose_initialize_acceptance` fixture; no additional untyped router instance found |

`cd claudine && just test-l2 level2_initialize_generated_transclusion` ran all
four CLI tests. The recipe's subsequent generator pass selects zero matching
tests; that does not replace or obscure the four CLI executions.

### Compiler-fixture repair: R1-F3

The feature-attribution test helper now links the binary supplied by the setup
script rather than launching redundant Cargo builds. The setup filter covers
both wrapped-build tests. The Nextest group limits the eleven actual compiler
fixtures across the two binaries to four concurrent tests; cheap inspections
and metadata-only queries remain outside it. Fresh builds remain where the
assertions measure compilation. No deadline, tier, assertion, or retry policy
was weakened.

Independent full-package run `80885378-bd98-4c9d-b0a0-f110393e7a89`
(`just _test repo-deps`): **919 passed, 1 skipped, zero tests at or above
five seconds**, 11.047 s execution. Host load immediately afterward was
10.12 / 11.19 / 12.50 on 16 cores, so no overloaded-host allowance was used.
The two feature-attribution fixtures previously failing took 2.967 and
2.704 s. The archive-only run `188c81b9-32f2-458a-969a-34724ef98345`
ran 70 passing tests in 6.434 s, with none at or above five seconds.

Every original archive instance passed below five seconds in the full run.
The table adds individual archive-scope measurements; feature-attribution
rows use their full-package timings. Every row expects its assertions to
hold within the five-second budget.

| Child ID | Required site / shape | Verification |
| --- | --- | --- |
| R1-F3.1 | Archive `a_consumer_reports_its_extraction_apart_from_its_identity_checks`; private copy | Archive 0.435 s; full package below 5 s; clean |
| R1-F3.2 | Archive `a_produced_fixture_verifies_including_its_own_archive_inventory`; private copy | Archive 0.436 s; full package below 5 s; clean |
| R1-F3.3 | Archive `a_real_external_dynamic_dependency_is_discovered_and_refused_when_removed`; fresh native-library build | Archive 3.325 s; full package below 5 s; clean |
| R1-F3.4 | Archive `an_archive_of_more_programs_than_the_plan_resolved_is_refused`; edited private manifest | Archive 0.395 s; full package below 5 s; clean |
| R1-F3.5 | Archive `an_archive_missing_a_declared_binary_is_refused_before_any_test_starts`; edited private manifest | Archive 0.406 s; full package below 5 s; clean |
| R1-F3.6 | Archive `the_canonical_tier_recipes_run_the_fixture_archive_without_rebuilding_it`; fresh build | Archive 2.878 s; full package below 5 s; clean |
| R1-F3.7 | Archive `the_archive_runs_every_tier_from_another_checkout_with_no_compiler_in_reach`; fresh build | Archive 1.953 s; full package below 5 s; clean |
| R1-F3.8 | Archive `a_combined_invocation_unifies_the_feature_graphs_the_producer_keeps_apart`; combined Cargo build | Archive 1.257 s; full package below 5 s; clean |
| R1-F3.9 | Feature attribution `a_wrapped_fixture_build_compiles_exactly_the_predicted_configurations`; fresh instrumented build | Full package 2.967 s; clean |
| R1-F3.10 | Feature attribution `a_forwarded_feature_what_if_matches_a_build_with_the_alignment_declared`; original and aligned builds | Full package 2.704 s; clean |
| R1-F3.11 | Archive `the_fixture_archive_carries_every_declared_payload_class`; shared fixture | Archive 0.101 s; full package below 5 s; clean |
| R1-F3.12 | Archive `a_refused_consumer_still_reports_what_its_transfer_cost`; shared fixture | Archive 0.121 s; full package below 5 s; clean |
| R1-F3.13 | Archive `a_tampered_fixture_archive_is_refused_with_a_stable_code_and_exit_status`; private corrupted payload | Archive 0.137 s; full package below 5 s; clean |
| R1-F3.14 | Archive `a_missing_sidecar_is_refused_end_to_end`; private missing sidecar | Archive 0.130 s; full package below 5 s; clean |
| R1-F3.15 | Archive `an_archive_offered_to_the_wrong_environment_is_refused_end_to_end`; incompatible environment | Archive 0.131 s; full package below 5 s; clean |
| R1-F3.16 | Archive `the_manifest_records_the_workspace_the_producer_compiled_at`; shared fixture | Archive 0.137 s; full package below 5 s; clean |
| R1-F3.17 | Archive `consumer_refuses_a_different_source_tree_even_with_a_recomputed_manifest_digest`; private edited source | Archive 0.149 s; full package below 5 s; clean |
| R1-F3.18 | Archive `failed_and_refused_keys_report_themselves_and_let_an_unrelated_key_finish`; custom build | Archive 2.750 s; full package below 5 s; clean |
| R1-F3.19 | Archive `a_key_narrowed_leg_produces_only_its_own_record`; custom build | Archive 0.357 s; full package below 5 s; clean |
| R1-F3.20 | Archive `every_record_attempt_is_sampled_and_a_failing_sampler_changes_no_outcome`; custom producer | Archive 0.284 s; full package below 5 s; clean |
| R1-F3.21 | Archive `the_hosted_wrapper_samples_each_record_through_the_shipped_sampler`; custom producer | Archive 0.308 s; full package below 5 s; clean |
| R1-F3.22 | Archive `the_owner_wrapper_leaves_owner_timing_to_the_coordinator_in_record_mode`; custom producer | Archive 0.049 s; full package below 5 s; clean |
| R1-F3.23 | Archive `a_sampler_that_cannot_start_leaves_a_successful_owner_successful`; custom producer | Archive 0.197 s; full package below 5 s; clean |
| R1-F3.24 | Archive `one_owner_tree_shares_a_dependency_compile_without_unifying_features`; fresh builds | Archive 2.370 s; full package below 5 s; clean |
| R1-F3.25 | Archive `produce_accepts_the_planned_head_and_refuses_the_merge_revision`; custom build | Archive 2.008 s; full package below 5 s; clean |
| R1-F3.26 | `two_tier_configurations_compile_the_same_package_twice_and_are_reported_apart` | Full package 0.839 s; included in compiler group |
| R1-F3.27 | `cargo_drives_the_shipped_wrapper_and_a_warm_rebuild_records_no_work` | Full package 0.479 s; included in compiler group |
| R1-F3.28 | Feature-attribution metadata / `cargo tree` queries | Full package all below 5 s; kept outside compiler group |
| R1-F3.29 | Producer cases rejected before compilation | Full package all below 5 s; kept outside compiler group |

## Unblocked Findings

None.

## Blocked Findings

None.

## Recurrence

None in this review. R1-F3 was the unchanged-instance recurrence reported in
Review 2; its remaining instances .8–10 now pass. This review found neither
a regression of that repair nor an additional sibling requiring repair.

## Spec coverage and verification levels

The shared API, Unix and Windows backends, migrated callers, tests, canonical
process-tree topic and linked skill/documentation were inspected against the
spec. Accepted scope exclusions remain intact: detached children, Claudine
provider-session supervision and the Unix foreground lifecycle exception.

| Requirement | Appropriate verification | Evidence |
| --- | --- | --- |
| Configure / attach, group versus session, hooks, fast exit, foreign-group rejection | L1 | Sniff process tests pass; attachment precedes readers; failed attachment kills and waits for direct child |
| Deferred sampling, identity validation before traversal, zero attachment scans | L1 | Scripted-host scheduling and recycled-root/descendant cases pass; first sample remains due at 250 ms |
| Immediate and graceful termination, root exits first, discovered detached helpers, failed-cleanup backstop | L1 | Public controlled-process tests and scripted cases pass; all consumer paths inspected |
| Release, armed drop, cleanup disarming and bounded reader settlement | L1 | Release survivor fixture and armed-drop controls pass; repeated cleanup is a no-op; inherited-pipe and deliberate between-sample escape cases pass |
| Windows suspension, assign-before-resume, handle ownership, release and failure paths | Native-Windows L1 | Source and declared tests inspected: suspension combined once, Job assigned before resume, handles owned immediately, setup failures retain caller cleanup; no new Windows runtime result claimed |
| Editor cancellation, resistant child, GUI-client survival, arguments and streams | L1 | 8 editor-launcher tests pass, maximum 2.085 s |
| Editor terminal foreground input and restoration after interruption | L2 | `cd darkmatter && just test-l2 level2_edit_action`: 2 pass, 0.860 / 0.885 s |
| Login-shell isolation, output cap and deadline cleanup | L1 plus existing terminal evidence | 4 launcher tests pass; session configuration inspected; earlier recorded terminal evidence retained |
| Claudine task timeout, cancellation, background cleanup, caps and original error categories | L1 | 26 shell-task tests pass, maximum 2.080 s; lifecycle immediate/graceful delegation inspected |
| Generated-log fixture path behavior in a terminal | L2 | All four generated-transclusion tests pass with unchanged assertions |
| Local-only API and consumer compatibility | Dependency/source inspection plus recorded builds | Process owner uses local OS APIs; no new dependency or HTTP feature requirement; earlier no-feature and consumer-build evidence remains applicable |
| Duplicate removal and documentation | Source/document inspection | No consumer-local owner, Job or descendant tracker; terminal handoff remains caller-owned; canonical topic documents lifecycle, cleanup, release and accepted escape/reuse limits |
| Compiler-fixture runtime | Concurrent L1 package run | All 919 tests pass below 5 s; 70-test archive scope also passes below 5 s |

The new Unix regressions are library unit tests selected in the executed L1
process suite. The re-execution helper is explicitly spawned by its controlling
tests; its ignored marker does not hide acceptance coverage. Consolidated
terminal targets declare the modules exercised above. No physical-key encoder
behavior was added; L3 injection is unnecessary for this change.

No file-format or configuration reader was introduced or changed by this fix
or its repairs; the input robustness matrix does not apply. The test helper's
compiler-artifact lookup is tool-output selection, not a configuration loader.

## Implementation records

The latest repair ledger's completion claim is supported by the independent
full-package and archive runs. Earlier failing timings remain documented as
historical results; the latest repair supersedes them without rewriting that
history. No additional false behavioral statement in either implementation
record was verified, so no in-place correction was needed in this iteration.
