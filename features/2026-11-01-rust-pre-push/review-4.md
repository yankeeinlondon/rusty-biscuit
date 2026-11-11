---
$schema: feature-review.yaml
ready: false
findings:
  - 'R1-F2 (high): Portable real-recipe tests still exceed the Windows L1 time budget'
  - 'R3-F1 (high): Three additional hook tests still exceed the Windows L1 time budget'
human_review: true
human_review_items:
  - 'Iteration 4 replaced the one hook-push-through-the-real-recipe test with pinned boundaries (real recipe run directly, modeled gate in hook tests, real-helper seeding), because ci-push plus the real recipe costs about 5.6 s on native Windows; the alternative is one ~6 s end-to-end test over the L1 budget'
  - 'Windows margins are thin: seeded_evidence_is_what_a_hook_push_records measured 4.85–4.97 s, the advanced-target tests 4.56–4.70 s'
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: true
reviewed_by: codex/gpt-6.1-sol
created: 2026-11-11T05:34:56-08:00
spec: 2026-11-01-rust-pre-push/spec.md
implemented: true
implemented_by: claude/opus
log: /Volumes/coding/rusty-biscuit/features/2026-11-01-rust-pre-push/log.md
description: 'A **feature** review of `2026-11-01-rust-pre-push/spec.md`'
feature: 2026-11-01-rust-pre-push/review-4.md
previous: 2026-11-01-rust-pre-push/review-3.md
next: 2026-11-01-rust-pre-push/review-5.md
---

# Review 4

The feature is not production ready: both carried findings are partially repaired. Seven previously slow Windows consumers now meet the five-second L1 limit; twelve still do not. No new finding or newly discovered sibling is reported.

I read the spec, implementation and repair logs, and all three earlier reviews. None contains a `Human rulings` section. The repair's suggestion to defer the remaining cost to later Rust ports is not an author ruling or an accepted exception.

## Evidence and limits

Freshly ran `just test-pre-push-hook`: 322 library tests and 69 integration/process tests passed on macOS, with no test at or above five seconds. The fresh library run includes the checkout-transition cases and public JSON validation matrix. `python3 scripts/ci/test_local_evidence.py` passed 31 tests, including the eight batching controls. Recipe verification is recorded below.

Independently inspected the raw Windows owning-suite output at `/tmp/cipush-prof/winfull3.log`, rather than treating the repair ledger's completion labels as proof. It contains 893 passes, one skip, and the individual durations reproduced below. These are existing measurements, not a fresh Windows run. The log identifies tested tree `c9750b66665c5510ba86e1b34c15af191d4b2580` and explicitly refuses to publish an observation because the local head tree differed. Consequently this is diagnostic timing evidence for the repair, not qualifying current-tree CI evidence. Current source inspection and fresh macOS executions confirm the named cases and assertions remain present. Missing current-tree cross-OS proof is not a finding.

## Previous Finding Verification

Every required child from review 3 is listed below. Earlier R1-F1 was already verified fixed in review 3 and is retained as a regression control. “Verified fixed” for the seven timing rows means their inspected implementation preserves the assertions and the raw Windows repair run meets the limit; it does not claim a fresh Windows run on this tree.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
|---|---|---|---|---|
| R1-F1: Checkout reuse uses stale snapshots | R1-F1.1–R1-F1.11: tree selection, gate reuse, final publication, starting controls, scope, first context, helper roots, outgoing identity | verified fixed | `review.rs::checkout_departure`, refreshed state in `run_gates`, final `withhold_reason`; fresh public transition/control cases pass | Nothing; no regression found |
| R1-F2: Portable real-recipe coverage and duration | R1-F2.1–R1-F2.27, including nine timing children | partially fixed | Portable targets and original assertions retained; fresh macOS cases pass; Windows rows below | Nine timing instances |
| R1-F2.1–R1-F2.6: Original recipe consumers | Receipt, reuse, retry, stacked base and three interruption shapes | verified fixed | No portable-behavior Unix exclusion; declared integration target executes these cases on macOS; Windows raw output executes them too | Duration remains separately below |
| R1-F2.7–R1-F2.8: Signals and launchers | Actual Unix signals, portable cancellation, Git Bash and PowerShell entry points | verified fixed | Platform guards match platform APIs; portable cancellation passes; both Windows launchers appear in raw output | PowerShell duration separately below |
| R1-F2.9: Recipe fallback consumer | Real recipe consumes selected executable and prefix | verified fixed | `parity.rs` keeps real-recipe fallback assertions; fresh case passes | Duration separately below |
| R1-F2.10–R1-F2.23: Other platform guard controls | Dispatcher, lifecycle, probes, bytes/modes, permissions and status construction | verified fixed | Guards remain scoped to platform APIs or have portable counterparts; fresh hook suite passes | Nothing |
| R1-F2.24: Non-Unicode selection | Rejection before discovery on each platform | verified fixed | `tests/run.rs` retains platform-specific invalid-string construction; public refusal test passes | Nothing |
| R1-F2.25: Detached-tree line endings | Persist fixture autocrlf setting | verified fixed | Fixture writes `core.autocrlf=false`; detached-tree tests pass | Nothing |
| R1-F2.26: Archive coordinator executable path | Bash command word uses forward slashes on Windows | verified fixed | `ci-build-archive-tests.rs` retains explicit conversion; no reversal | Nothing |
| R1-F2.27: Recipe duration | R1-F2.27.1–R1-F2.27.9 | partially fixed | Fresh macOS cases within limit; raw Windows output remains over limit | All nine rows below |
| R1-F2.27.1: `an_advanced_non_main_target_reuses_prior_cells_and_publishes_exact_receipts` | Advanced target; seeded evidence, real-recipe follow-up | partially fixed | Assertions retained; fresh macOS passes; raw Windows 10.806 s | Windows duration |
| R1-F2.27.2: `a_docs_only_follow_up_skips_the_cells_prior_evidence_covers` | Docs-only follow-up skips covered cells | partially fixed | Assertions retained; fresh macOS passes; raw Windows 11.561 s | Windows duration |
| R1-F2.27.3: `a_follow_up_changing_a_packages_inputs_reruns_its_cell` | Changed package inputs invalidate prior evidence | partially fixed | Assertions retained; fresh macOS passes; raw Windows 11.811 s | Windows duration |
| R1-F2.27.4: `a_cell_whose_prior_evidence_is_a_failure_is_rerun_and_superseded` | Passing retry supersedes failure; public verifier | partially fixed | Assertions retained; fresh macOS passes; raw Windows 13.256 s | Windows duration |
| R1-F2.27.5: `a_stacked_pull_request_records_its_target_tip_as_the_receipt_base` | Target-tip base and retained reports | partially fixed | Assertions retained; fresh macOS passes; raw Windows 7.126 s | Windows duration |
| R1-F2.27.6: `a_real_recipe_interruption_withholds_earlier_reports::case_1_malformed_manifest` | Malformed manifest refuses publication | partially fixed | Assertions retained; fresh macOS passes; raw Windows 5.950 s | Windows duration |
| R1-F2.27.7: `a_real_recipe_interruption_withholds_earlier_reports::case_2_exit_130` | Exit 130 refuses publication | partially fixed | Assertions retained; fresh macOS passes; raw Windows 5.825 s | Windows duration |
| R1-F2.27.8: `a_real_recipe_interruption_withholds_earlier_reports::case_3_recorded_143` | Recorded interruption refuses publication | partially fixed | Assertions retained; fresh macOS passes; raw Windows 6.145 s | Windows duration |
| R1-F2.27.9: `a_real_recipe_runs_its_helpers_under_the_selected_fallback_interpreter` | Real recipe consumes fallback executable and prefix | partially fixed | Assertions retained; fresh macOS passes; raw Windows 8.121 s | Windows duration |
| R3-F1: Additional hook consumers | R3-F1.1–R3-F1.13 | partially fixed | Seven named consumers meet limit; three do not; controls retained | R3-F1.1, R3-F1.2, R3-F1.9 |
| R3-F1.1: `parity::a_real_recipe_gate_failure_records_a_complete_failure` | Complete failure; later gate and retained reports | partially fixed | Source assertions retained; raw Windows 7.939 s; fresh macOS counterpart passes | Windows duration |
| R3-F1.2: `parity::a_blocked_strict_failure_publishes_its_note_but_ci_reruns_the_cell` | Strict failure publication and CI rerun | partially fixed | Source assertions retained; raw Windows 5.850 s; fresh macOS counterpart passes | Windows duration |
| R3-F1.3: `parity::a_broken_python3_hands_the_fallback_to_every_helper_and_the_gates` | Broken first interpreter and propagated fallback | verified fixed | Source assertions retained; raw Windows 4.358 s; fresh macOS counterpart passes | Nothing |
| R3-F1.4: `parity::a_branch_creating_push_records_the_remote_main_tip_and_says_so` | Branch creation against remote main | verified fixed | Source assertions retained; raw Windows 4.335 s; fresh macOS counterpart passes | Nothing |
| R3-F1.5: `parity::a_complete_warn_failure_publishes_its_note` | Complete warn failure publishes | verified fixed | Source assertions retained; raw Windows 3.920 s; fresh macOS counterpart passes | Nothing |
| R3-F1.6: `parity::a_record_in_the_default_store_binds_a_fresh_session` | Default constraint store across sessions | verified fixed | Source assertions retained; raw Windows 3.535 s; fresh macOS counterpart passes | Nothing |
| R3-F1.7: `parity::a_published_receipt_names_retained_readable_reports` | Receipt points at retained reports | verified fixed | Source assertions retained; raw Windows 4.048 s; fresh macOS counterpart passes | Nothing |
| R3-F1.8: `parity::a_failed_validation_note_publication_keeps_the_durable_reports` | Publication failure preserves reports | verified fixed | Source assertions retained; raw Windows 3.786 s; fresh macOS counterpart passes | Nothing |
| R3-F1.9: `launcher::windows_powershell_pushes_through_the_launcher_to_the_real_ci_push` | PowerShell auto and strict pushes through actual launcher | partially fixed | Source assertions retained; raw Windows 7.340 s; Windows-only case remains declared | Windows duration |
| R3-F1.10: `launcher::the_real_ci_push_reviews_an_auto_push_without_gates_and_a_strict_push_with_them` | Direct auto and strict pushes through actual launcher | verified fixed | Source assertions retained; raw Windows 4.785 s; fresh macOS counterpart passes | Nothing |
| R3-F1.11: Other parity and Git Bash consumers | Remaining fixture shapes and launcher | verified fixed | Raw Windows Git Bash 4.692 s; remaining parity controls at most 3.019 s; fresh macOS counterparts pass | Nothing |
| R3-F1.12: Dispatcher, probe and supervision controls | Forwarding, bounded probe and child ownership | verified fixed | These sites do not run the planning/evidence helper pipeline; fresh process cases pass | Nothing |
| R3-F1.13: Public orchestration controls | Injected-services decision tests | verified fixed | No external planning/evidence helper starts; fresh 322-test library run passes | Nothing |
| R2-N1: Extracted real-recipe failure coverage | Same instance now R3-F1.1 | partially fixed | Complete failure and retained-report assertions remain; raw Windows 7.939 s | Carried once under R3-F1.1 |
| R2-N2: Additional helper consumers | Same inventory now R3-F1.2–R3-F1.10 | partially fixed | Per-instance verification above | R3-F1.2 and R3-F1.9 |
| R2-N3: Workflow-contract anchor | Actual archive guard remains checked | verified fixed | Contract still anchors the guard `if`, independent of moved assignment | Nothing |
| R2-N4: Retired shell assertion | Remove obsolete assertion; retain retry coverage | verified fixed | Old hook-text assertion absent; public and process retry checks remain | Nothing |

## Unblocked Findings

### R1-F2 (high): Portable real-recipe tests still exceed the Windows L1 time budget

**Authority — spec, Tests and acceptance criterion 4, plus review testing rules:** the hook behavior must run as portable L1 nextest tests; “An L1 test over 5 s counts as failing.” Ordinary fixture pushes still exceed the limit. Functional success is preserved, but the timing requirement remains unmet.

**Defect class:** repeated process startup in the real-recipe acceptance path exceeds the L1 budget on native Windows.

Sites below are in repo-deps, [integration/parity.rs](../../scripts/ci-push/integration/parity.rs). This is the complete nine-instance inventory from review 3. The common expected result is the same assertions with each test completing below five seconds under ordinary owning-suite concurrency.

| Child ID | Site | Shape tested | Observed Windows result | Expected result |
|---|---|---|---|---|
| R1-F2.27.1 | `an_advanced_non_main_target_reuses_prior_cells_and_publishes_exact_receipts` | Advanced target; seeded evidence, real-recipe follow-up | Assertions pass, 10.806 s | Same behavior, below 5 s |
| R1-F2.27.2 | `a_docs_only_follow_up_skips_the_cells_prior_evidence_covers` | Docs-only follow-up skips covered cells | Assertions pass, 11.561 s | Same behavior, below 5 s |
| R1-F2.27.3 | `a_follow_up_changing_a_packages_inputs_reruns_its_cell` | Changed package inputs invalidate prior evidence | Assertions pass, 11.811 s | Same behavior, below 5 s |
| R1-F2.27.4 | `a_cell_whose_prior_evidence_is_a_failure_is_rerun_and_superseded` | Passing retry supersedes failure; public verifier | Assertions pass, 13.256 s | Same behavior, below 5 s |
| R1-F2.27.5 | `a_stacked_pull_request_records_its_target_tip_as_the_receipt_base` | Target-tip base and retained reports | Assertions pass, 7.126 s | Same behavior, below 5 s |
| R1-F2.27.6 | `a_real_recipe_interruption_withholds_earlier_reports::case_1_malformed_manifest` | Malformed manifest refuses publication | Assertions pass, 5.950 s | Same behavior, below 5 s |
| R1-F2.27.7 | `a_real_recipe_interruption_withholds_earlier_reports::case_2_exit_130` | Exit 130 refuses publication | Assertions pass, 5.825 s | Same behavior, below 5 s |
| R1-F2.27.8 | `a_real_recipe_interruption_withholds_earlier_reports::case_3_recorded_143` | Recorded interruption refuses publication | Assertions pass, 6.145 s | Same behavior, below 5 s |
| R1-F2.27.9 | `a_real_recipe_runs_its_helpers_under_the_selected_fallback_interpreter` | Real recipe consumes fallback executable and prefix | Assertions pass, 8.121 s | Same behavior, below 5 s |

The bounded sweep follows this cost through every real-recipe caller, stub-gate consumer and launcher; R3-F1 below includes the clean siblings. Injected-service orchestration cannot incur this helper startup cost. The fresh macOS suite passes within budget, so this is the remaining Windows portion of the original finding.

**Recommended repair:** retain a focused end-to-end recipe control and the existing receipt/reuse assertions, then reduce redundant setup and execution in the slow tests. Prefer separating the acceptance questions at their existing public boundaries: the real recipe's gate/report contract, the hook's receipt/publication contract, and the verifier's retry decision. Keep an independent real-recipe equivalence check where setup is substituted. Measure those focused cases under owning-suite load. Small batching inside existing helpers remains permitted, but a wholesale planner or evidence-client port would be disproportionate and is outside this spec. Do not raise timeouts, rename tiers, skip Windows or drop assertions.

The repair has already reduced Git reads and recipe lookups. Repeating the same micro-optimization recommendation without changing the test's work is unlikely to remove the remaining two-push cost. Internal fixture and measurement structure is reversible and requires no author decision.

### R3-F1 (high): Three additional hook tests still exceed the Windows L1 time budget

**Authority — spec, Tests and acceptance criterion 4, plus the five-second L1 rule:** hook process and launcher acceptance cases must satisfy the same L1 contract. Ordinary strict-failure and PowerShell fixture pushes still exceed it.

**Defect class:** other consumers of the same process-heavy planning/evidence path exceed the L1 budget. This finding retains its original ID; no new sibling was found.

Sites are in repo-deps, [parity.rs](../../scripts/ci-push/integration/parity.rs) and [launcher.rs](../../scripts/ci-push/integration/launcher.rs). The sweep includes clean sites so the next repair does not treat them as new findings.

| Child ID | Site | Shape tested | Observed Windows result | Expected result |
|---|---|---|---|---|
| R3-F1.1 | `parity::a_real_recipe_gate_failure_records_a_complete_failure` | Complete failure; later gate and retained reports | Still over budget: 7.939 s, assertions pass | Same assertions, below 5 s |
| R3-F1.2 | `parity::a_blocked_strict_failure_publishes_its_note_but_ci_reruns_the_cell` | Strict failure publication and CI rerun | Still over budget: 5.850 s, assertions pass | Same assertions, below 5 s |
| R3-F1.3 | `parity::a_broken_python3_hands_the_fallback_to_every_helper_and_the_gates` | Broken first interpreter and propagated fallback | Clean: 4.358 s, assertions pass | Same assertions, below 5 s |
| R3-F1.4 | `parity::a_branch_creating_push_records_the_remote_main_tip_and_says_so` | Branch creation against remote main | Clean: 4.335 s, assertions pass | Same assertions, below 5 s |
| R3-F1.5 | `parity::a_complete_warn_failure_publishes_its_note` | Complete warn failure publishes | Clean: 3.920 s, assertions pass | Same assertions, below 5 s |
| R3-F1.6 | `parity::a_record_in_the_default_store_binds_a_fresh_session` | Default constraint store across sessions | Clean: 3.535 s, assertions pass | Same assertions, below 5 s |
| R3-F1.7 | `parity::a_published_receipt_names_retained_readable_reports` | Receipt points at retained reports | Clean: 4.048 s, assertions pass | Same assertions, below 5 s |
| R3-F1.8 | `parity::a_failed_validation_note_publication_keeps_the_durable_reports` | Publication failure preserves reports | Clean: 3.786 s, assertions pass | Same assertions, below 5 s |
| R3-F1.9 | `launcher::windows_powershell_pushes_through_the_launcher_to_the_real_ci_push` | PowerShell auto and strict pushes through actual launcher | Still over budget: 7.340 s, assertions pass | Same assertions, below 5 s |
| R3-F1.10 | `launcher::the_real_ci_push_reviews_an_auto_push_without_gates_and_a_strict_push_with_them` | Direct auto and strict pushes through actual launcher | Clean: 4.785 s, assertions pass | Same assertions, below 5 s |
| R3-F1.11 | Other parity tests and Git Bash launcher | Remaining fixture shapes | Clean: Git Bash 4.692 s; other parity controls at most 3.019 s | Preserve coverage and budget |
| R3-F1.12 | Dispatcher, Python probe and process supervision | Child forwarding and lifetime | Clean for this class: no planning/evidence pipeline | Preserve coverage |
| R3-F1.13 | Public orchestration | Injected-service decisions | Clean by construction: no helper process startup; fresh suite passes | Preserve coverage |

**Recommended repair:** repair R3-F1.1 and R3-F1.2 alongside R1-F2. For R3-F1.9, split the combined auto/strict launcher scenario into independent tests, each with its own fixture, while retaining argument/stdin forwarding and real PowerShell-launched Git. The helper currently performs two pushes in one test; the two modes are independent acceptance questions. Keep test names in L1 and ensure each split case is declared and selected. This recommendation changes internal test structure, not the launcher's Cargo freshness policy. Source inspection shows Cargo is a compiled forwarding stand-in in this fixture; its measured cost should not be described as a real Cargo build/freshness check.

## Blocked Findings

None. These are ordinary implementation repairs within the existing scope. Deferring the timing requirement to another spec or accepting a different limit would change scope or policy, but neither is required by the recommended repair. No human-only verification is needed.

## Recurrence

R1-F2 repeats the unresolved Windows duration class from reviews 1–3. R3-F1 repeats the same class explicitly inventoried in review 3; its seven repaired consumers are distinguished above from the three remaining instances. These are partial repairs, not checkout-state regressions or newly discovered siblings.

The boundary remains the existing hook L1 consumers and their shared helper calls. Repeated profiling has reduced redundant operations without meeting the limit for twelve cases. Further machinery to protect increasingly synthetic inputs would not be justified; focus on the work each existing acceptance test needs to perform.

## Spec and Test Verification

| Requirement | Strongest relevant verification inspected | Assessment |
|---|---|---|
| Complete ref parsing, malformed/CRLF input and skipped refs | L1 public parser/orchestration and binary process cases | Fresh suite passes |
| Main, PR and provisional contexts; simultaneous base changes; fail-closed lookup | L1 public results and fixture repositories | Fresh suite passes |
| Mode selection, ordinary failure and interruption | L1 public outcomes, child exit statuses, portable cancellation and Unix signals | Fresh suite passes |
| Committed-tree planning and cleanup | L1 state transitions, actual detached trees and child supervision | Prior checkout-state defect stays fixed |
| Changed paths, repository/branch identity and evidence-applied constraints | L1 Git fixtures and shared constraint fixtures | Fresh suite passes |
| Interpreter version, bounded probe, fallback and propagation into recipe | L1 executable probes and real-recipe consumer | Functional pass; carried duration defect |
| Scope, validation, retry, report retention and advisory publication failure | L1 public orchestration and actual Python helpers/recipe | Functional pass; carried duration defect |
| Thin launcher and worktree-aware dispatcher | L1 actual Git hook invocation, compiled Cargo forwarder, Windows entry-point source and raw output | Appropriate level; PowerShell duration remains |
| Retired shell suite parity, test ownership and documentation | Implementation parity inventory; Cargo targets, just recipes, source-input declarations; README, topic/testing docs and skills | No additional gap found |

These requirements concern process, Git and filesystem behavior. No terminal rendering, modifier encoder or focused keyboard requirement calls for L2/L3. Cross-OS CI proof itself is excluded from readiness findings.

The review inspected the production batching changes: only immutable object IDs/tree expressions are cached, keyed by working directory; mutable names use the existing resolver. Missing objects and refused batches fall back to the original reads. Batched `ls-tree` output is sorted and deduplicated as before. Recipe running-cell matching remains exact-line matching, and the combined `jq` emits the same decisions. The fresh Python batching tests exercise real fixture repositories and compare results against the original Git operations. No additional functional defect was found.

## Input Robustness

No production configuration loader or field validator was added by this repair. The hook retains one strict JSON loader and shared validators in `documents.rs`/`json.rs`. Default invalid shapes produce a located error and block review before gates or publication. Existing exceptions remain: empty PR/cell arrays mean no entries; absent or empty optional constraint strings are wildcards; absent constraint-view `state` supports older plans, while null is rejected.

The fresh public document tests exercise absent, null, wrong whole-field types, wrong collection elements and empty values through shared validators, with per-field wiring controls; loader cases reject duplicate keys, trailing content, comments, trailing commas and empty documents. Public orchestration and process cases prove refusal before work for invalid helper documents. Source search found no load-bearing `serde(default)`, silent element filtering or permissive defaulting in these readers. Date parse `.ok()?` propagates to a located error. The new batch-process output is internal Git protocol output, not a new configuration format; fallback cases are covered by the Python tests.

## Verification Commands

- `just test-pre-push-hook`: 322 library and 69 process tests passed; no test at or above five seconds on macOS.
- `python3 scripts/ci/test_local_evidence.py`: 31 passed.
- `BISCUIT_REQUIRE_JUST=1 BISCUIT_REQUIRE_JQ=1 BISCUIT_REQUIRE_BASH=1 python3 scripts/ci/test_ci_local.py`: 108 passed.
- `just check-tier-coverage scripts`: zero stranded tests.
- `git diff --check`: passed before review-file edits; checked again afterward.
