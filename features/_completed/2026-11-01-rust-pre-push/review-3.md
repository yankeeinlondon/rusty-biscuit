---
$schema: feature-review.yaml
ready: false
findings:
  - 'R1-F2 (high): Portable real-recipe tests still exceed the Windows L1 time budget'
  - 'R3-F1 (high): Additional hook tests share the unresolved Windows duration defect'
human_review: true
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: true
reviewed_by: codex/gpt-6.1-sol
created: 2026-11-11T04:05:57-08:00
spec: 2026-11-01-rust-pre-push/spec.md
implemented: false
description: 'A **feature** review of `2026-11-01-rust-pre-push/spec.md`'
feature: 2026-11-01-rust-pre-push/review-3.md
previous: 2026-11-01-rust-pre-push/review-2.md
log: /Volumes/coding/rusty-biscuit/features/2026-11-01-rust-pre-push/log.md
implemented_by: claude/opus
human_review_items:
  - 'R1-F2/R3-F1 (iteration 3 repair): helper processes were not merged (verify+apply+scope-record, record-cells+publish-receipt) and the recipe self-test loop was not consolidated; estimated gain about 0.5 s per push, which would still leave the two-push rows near 7–8 s on Windows. Decide whether to restructure the helper interface in this spec or accept that the remaining Windows L1 floor clears with 2026-11-06-rust-evidence-client and 2026-11-06-rust-ci-planner'
next: 2026-11-01-rust-pre-push/review-4.md
---

# Review 3

The feature is not production ready. The remaining repair is partial: the real-recipe tests now stay within five seconds on macOS, but the repair reports that they still exceed that limit on native Windows. The same problem affects additional tests identified during repair. No new functional defect was found.

The Windows issue is a measured test-duration defect, not missing cross-platform execution evidence. This review independently ran the macOS hook suite and inspected the implementation; Windows durations below are explicitly attributed to the repair measurements in `log.md`, not represented as a fresh Windows run. They cannot establish a successful repair of the five-second requirement.

## Previous Finding Verification

Read both previous reviews and both finding ledgers in `log.md`. Neither previous review contains author rulings. Their `human_review_items` are recommendations for later review, not author decisions or approved exceptions to testing rules.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
|---|---|---|---|---|
| R1-F1: Checkout reuse relies on stale cleanliness and HEAD snapshots | R1-F1.1–R1-F1.11 | verified fixed | Inspected each decision listed below and reran the public orchestration tests in the 322-test library suite | Nothing |
| R1-F1.1: Initial capture | Starting identity and dirtiness | verified fixed | Starting state remains an independent identity/withhold condition, rather than permission for later checkout reuse | Nothing |
| R1-F1.2: Dirty before tree selection | Dirty during Python selection | verified fixed | `checkout_departure` refreshes status; transition test selects the temporary tree and publishes committed scope | Nothing |
| R1-F1.3: HEAD changes before tree selection | Moved HEAD, unreadable HEAD/status | verified fixed | `review_update` refreshes state after context discovery; all three corresponding transition cases pass | Nothing |
| R1-F1.4: Dirty before gates | Dirty or unreadable status during scope publication | verified fixed | `run_gates` refreshes status; public tests assert no plan input and no validation publication | Nothing |
| R1-F1.5: HEAD changes before gates | Moved or unreadable HEAD | verified fixed | Current identity is checked alongside cleanliness; both transition cases pass | Nothing |
| R1-F1.6: Final publication checks | Dirty or changed HEAD after gates | verified fixed | `withhold_reason` checks current HEAD/status; public final-state cases pass | Nothing |
| R1-F1.7: Starting-state controls | Initially dirty, non-HEAD, override, clean checkout | verified fixed | Library and process controls pass; initial dirtiness still independently withholds validation | Nothing |
| R1-F1.8: Scope-only publication | Publication without gates | verified fixed | `publish_scope` uses the staged committed receipt, independent of mutable checkout state | Nothing |
| R1-F1.9: First-context selection | Context identity and plan capture | verified fixed | `review_context` captures the first eligible context independently of recording success | Nothing |
| R1-F1.10: Helper working directories | Verify, apply, scope-record | verified fixed | Scripts come from the selected committed tree and receive explicit base/head arguments; temporary-tree tests pass | Nothing |
| R1-F1.11: Outgoing HEAD membership | `pushes_head` and later decisions | verified fixed | Membership describes the captured outgoing identity; separate current-state checks guard gate reuse and publication | Nothing |
| R1-F2: Portable real-recipe coverage and duration | R1-F2.1–R1-F2.27, including nine duration children | partially fixed | Windows exclusions remain removed; seeded setup and real-recipe assertions inspected; fresh macOS suite passes without over-budget tests | R1-F2.27 remains open on Windows |
| R1-F2.1–R1-F2.6: Receipt, reuse, retry, stacked-base, interruption tests | Each previously excluded real-recipe test | verified fixed | No Unix compilation guard; declared integration target includes `parity`; current macOS run executes them successfully | Their separate timing requirements remain below |
| R1-F2.7–R1-F2.8: Actual signals and Windows launchers | Unix signal cases, portable cancellation, Git Bash and PowerShell entry points | verified fixed | Platform-specific APIs retain guards; portable cancellation runs; both Windows launcher tests remain declared and enabled there | Nothing; cross-OS proof remains CI's responsibility |
| R1-F2.9: Fallback interpreter consumer | Real recipe consumes executable and argument prefix | verified fixed | Portable test still runs the actual recipe and checks helper calls and validation publication; fresh macOS test passes | Timing tracked below |
| R1-F2.10–R1-F2.23: Other platform guards | Dispatcher, lifecycle, Python, Git bytes/modes, permissions, status construction | verified fixed | Guards cover platform-specific APIs or retain the alternatives checked in review 2; no new coverage exclusion in repair | Nothing |
| R1-F2.24: Non-Unicode selection | Rejection before discovery on both platforms | verified fixed | Portable construction remains in `tests/run.rs`; macOS case passes | Nothing |
| R1-F2.25: Detached-tree line endings | Fixture configuration reaches worktree creation | verified fixed | Fixture persists `core.autocrlf=false`; detached-tree tests pass | Nothing |
| R1-F2.26: Archive coordinator path | Bash receives a Windows executable path with forward slashes | verified fixed | Inspected retained path conversion in `ci-build-archive-tests.rs`; no reversal in this repair | Nothing for this path defect |
| R1-F2.27: Recipe duration | R1-F2.27.1–R1-F2.27.9 | partially fixed | Fresh macOS hook suite: none at or above five seconds; repair ledger explicitly reports each Windows case still over budget | All nine Windows duration instances in the finding below |
| Repair discovery R2-N1: Added real-recipe failure case | Complete failure receipt and duration | partially fixed | New test asserts later gate execution, complete failure, and retained bytes; runs in the fresh integration suite | Windows duration; carried as new sibling R3-F1.1 |
| Repair discovery R2-N2: Stub-gate and launcher durations | Nine named Windows consumers | unresolved | Source confirms they still use the helper pipeline; repair records 5.1–8.2 s measurements | R3-F1.2–R3-F1.10 |
| Repair discovery R2-N3: Workflow-contract anchor | Contract still checks the archive guard after batching | verified fixed | Diff anchors the check on the actual guard `if`, preserving its scope after the earlier assignment moved | Nothing |
| Repair discovery R2-N4: Retired shell-source assertion | Remove obsolete hook-text test without losing retry coverage | verified fixed | Removed old literal assertion; retry overlay and process retry coverage remain and pass | Nothing |

## Unblocked Findings

### R1-F2 (high): Portable real-recipe tests still exceed the Windows L1 time budget

**Authority — spec and review testing rules:** the spec requires the retired hook's behaviors to become portable L1 nextest tests. The review rules explicitly state, “An L1 test over 5 s counts as failing.” Ordinary fixture pushes reach the issue. The repair records successful Windows assertions but continuing durations above that limit; successful exit status does not satisfy the timing requirement.

**Defect class:** acceptance tests repeatedly execute a subprocess-heavy planning and evidence pipeline, exceeding the L1 duration budget on native Windows.

The repair correctly replaces the first recipe execution in four follow-up tests with a stub gate. That gate still goes through `ci-push` to produce real receipts. Its generated reports are checked byte for byte against real-recipe runs; the follow-up runs still execute the real recipe. Separating the real-recipe failure case preserves coverage. These are reasonable internal testing choices and require no author confirmation.

The remaining problem is the helper pipeline, not missing recipe portability. The repair's Windows measurements remain over five seconds, even for single-push tests. The following table preserves every required child ID from review 2. Sites are in repo-deps, [integration/parity.rs](../../scripts/ci-push/integration/parity.rs).

| Child ID | Site | Shape tested | Observed result | Expected result |
|---|---|---|---|---|
| R1-F2.27.1 | `an_advanced_non_main_target_reuses_prior_cells_and_publishes_exact_receipts` | Seeded first push, advanced target, real-recipe follow-up | Fresh macOS 3.439 s; repair Windows 11.3–15.1 s | Same exact-receipt/reuse checks within 5 s |
| R1-F2.27.2 | `a_docs_only_follow_up_skips_the_cells_prior_evidence_covers` | Seeded evidence, docs-only follow-up | macOS within budget; repair Windows 14.8–15.7 s, 8.3 s alone | Same skip/receipt checks within 5 s |
| R1-F2.27.3 | `a_follow_up_changing_a_packages_inputs_reruns_its_cell` | Seeded evidence, changed package inputs | macOS within budget; repair Windows 14.5–15.0 s | Same invalidation checks within 5 s |
| R1-F2.27.4 | `a_cell_whose_prior_evidence_is_a_failure_is_rerun_and_superseded` | Seeded failure, passing retry, public verification | macOS within budget; repair Windows 16.1–17.8 s | Same newest-evidence checks within 5 s |
| R1-F2.27.5 | `a_stacked_pull_request_records_its_target_tip_as_the_receipt_base` | Single push, target-tip base, retained reports | Fresh macOS 2.184 s; repair Windows 8.3–9.9 s, 5.8–6.2 s alone | Same base/identity/report checks within 5 s |
| R1-F2.27.6 | `a_real_recipe_interruption_withholds_earlier_reports`, malformed manifest | Single incomplete run | macOS within budget; repair Windows 7.4–7.5 s | Preserve refusal within 5 s |
| R1-F2.27.7 | Same function, exit 130 | Single interrupted run | macOS within budget; repair Windows 7.2–7.6 s | Preserve refusal within 5 s |
| R1-F2.27.8 | Same function, recorded 143 | Interruption recorded in manifest | macOS within budget; repair Windows 7.3–7.6 s | Preserve refusal within 5 s |
| R1-F2.27.9 | `a_real_recipe_runs_its_helpers_under_the_selected_fallback_interpreter` | Broken first candidate, recipe consumes fallback | Fresh macOS 2.252 s; repair Windows 9.9–10.1 s | Preserve executable/prefix checks within 5 s |

**Recommended repair:** retain the assertions and remove repeated helper work. The repair explicitly left duplicate `rev-parse` and `notes list` calls untouched because later features replace the helpers. Removing repeated reads within the existing Python implementation does not port the planner, create a second evidence client, or change its decisions; it is a smaller repair within scope. Profile that route before declaring the existing scope incapable of meeting the budget. Where setup dominates a two-push test, further separate receipt setup from the operation being measured only with an independent equivalence check. Do not raise the budget, add retries, move these tests to a different tier, disable Windows, or require implementing the later Rust ports.

A wholesale helper redesign would be disproportionate to the harm. Keep any helper changes to measured redundant work and run the existing Python contract tests. Verify ordinary owning-suite concurrency, rather than only individual tests.

### R3-F1 (high): Additional hook tests share the unresolved Windows duration defect

**Authority — spec and review testing rules:** the spec places the hook's process and launcher acceptance tests in L1 on all supported systems; the five-second L1 rule applies to these tests too. The repair's ordinary Windows package runs identify the same over-budget behavior in additional consumers. This is a sibling of R1-F2's duration defect, not a new requirement for cross-platform proof.

**Defect class:** additional consumers of the same subprocess-heavy hook pipeline exceed the L1 duration budget.

The bounded sweep includes all real-recipe consumers, all stub-gate parity consumers, and actual launcher consumers of `Evidence`. Review 2 incorrectly called the latter clean controls based on macOS durations alone. The repair discovered their Windows cost and added another real-recipe test; those sites need explicit repair inventory now.

| Child ID | Site | Shape tested | Observed result | Expected result |
|---|---|---|---|---|
| R3-F1.1 | `parity::a_real_recipe_gate_failure_records_a_complete_failure` | Failing first gate, successful later gate, complete retained failure | Fresh macOS passes within budget; repair Windows 9.6–9.8 s | Same receipt/report assertions within 5 s |
| R3-F1.2 | `parity::a_blocked_strict_failure_publishes_its_note_but_ci_reruns_the_cell` | Strict failure and subsequent evidence decision | Repair Windows 7.1–8.2 s | Same behavior within 5 s |
| R3-F1.3 | `parity::a_broken_python3_hands_the_fallback_to_every_helper_and_the_gates` | Broken Python candidate, helper/gate propagation | Repair Windows 5.6–7.0 s | Same fallback assertions within 5 s |
| R3-F1.4 | `parity::a_branch_creating_push_records_the_remote_main_tip_and_says_so` | Branch creation, remote-main base | Repair Windows 5.6–6.8 s | Same base assertions within 5 s |
| R3-F1.5 | `parity::a_complete_warn_failure_publishes_its_note` | Complete gate failure in warn mode | Repair Windows 5.5–6.8 s | Same publication assertions within 5 s |
| R3-F1.6 | `parity::a_record_in_the_default_store_binds_a_fresh_session` | Default constraint store across sessions | Repair Windows 5.3 s | Same enforcement assertions within 5 s |
| R3-F1.7 | `parity::a_published_receipt_names_retained_readable_reports` | Receipt points at durable reports | Repair Windows 5.1–5.3 s | Same retained-report assertions within 5 s |
| R3-F1.8 | `parity::a_failed_validation_note_publication_keeps_the_durable_reports` | Failed publication, retained reports | Repair Windows 5.1 s | Same retention assertions within 5 s |
| R3-F1.9 | `launcher::windows_powershell_pushes_through_the_launcher_to_the_real_ci_push` | PowerShell-launched Git and actual launcher | Repair Windows 7.3 s | Same forwarding/status assertions within 5 s |
| R3-F1.10 | `launcher::the_real_ci_push_reviews_an_auto_push_without_gates_and_a_strict_push_with_them` | Actual launcher, auto and strict pushes | Repair Windows 5.2 s | Same mode assertions within 5 s |
| R3-F1.11 | Other parity tests and Git Bash launcher | Remaining ordinary fixture shapes | Fresh macOS suite within budget; no additional over-budget Windows consumer named in the repair ledger | Preserve coverage; check durations in final Windows owning-suite run |
| R3-F1.12 | Dispatcher-only, Python probe, process supervision tests | Forwarding, bounded probe, child lifecycle | Fresh macOS cases within budget; these tests do not execute the planning/evidence Python pipeline | Clean for this repeated-helper class |
| R3-F1.13 | Public orchestration tests | Same decisions with injected services | Fresh 322-test library suite passes within budget; no real helper pipeline is started | Clean for this class |

The Windows observations are the repair's recorded measurements. The fresh macOS reproductions use the same unedited fixture tests and confirm preserved behavior, but cannot validate Windows timing. Row R3-F1.11 is deliberately not claimed as independently verified within budget on Windows.

**Recommended repair:** include these consumers in the same shared helper-cost repair as R1-F2, then inspect every hook test's duration in the owning suite. Keep real launcher invocations and focused real-helper coverage. A separate optimization layer per test would add more maintenance than this issue warrants.

## Blocked Findings

None. No approved exception or changed prerequisite blocks repair. The spec forbids porting the remaining planner/evidence implementation here; it does not forbid removing duplicate reads in their existing implementation. That route preserves the spec's decisions and is the recommended next step. Neither moving work between features nor relaxing the L1 budget is requested.

## Recurrence

R1-F2 repeats the unresolved duration instance from review 2, originally recorded as R1-F2.27 during the first repair. The macOS portion is repaired; the Windows portion is unchanged and remains open.

R3-F1.1 is a new sibling introduced by extracting real-recipe failure coverage during repair. R3-F1.2–R3-F1.10 are siblings the earlier sweep missed: review 2 should have included stub-gate and launcher consumers when following the cost into the shared helper pipeline. They are not regressions of the checkout-state repair. The tables now include the full named repair inventory and the remaining control categories. Both findings should be repaired together.

This class has expanded through measurement of existing consumers and one extracted test, rather than through new production protection machinery. Its boundary remains concrete: existing hook L1 tests and their shared planning/evidence calls. Do not expand it into the later Rust helper ports.

## Spec and Test Verification

Fresh verification on this checkout:

- `just test-pre-push-hook`: 322 library tests and 69 integration/process tests passed. Neither invocation recorded an L1 test at or above five seconds.
- `BISCUIT_REQUIRE_JUST=1 BISCUIT_REQUIRE_JQ=1 BISCUIT_REQUIRE_BASH=1 python3 scripts/ci/test_ci_local.py`: 108 tests passed, exercising the recipe after batching.
- `git diff --check`: passed.
- `just check-tier-coverage scripts`: zero stranded tests. The `ci-push` integration target declares the process modules, and its feature requirement is enabled by the package test configuration.
- Source review confirms the recipe batches plan facts without changing the gate selection conditions, and the obsolete shell-hook assertion is removed.

| Requirement | Verification level and inspected evidence | Assessment |
|---|---|---|
| Complete update parsing; malformed/CRLF input; skipped refs | L1 parser/public orchestration and binary wiring tests | Passing |
| Main/PR/provisional contexts; simultaneous base changes; fail-closed review | L1 public orchestration plus Git fixture process tests | Passing |
| Mode resolution, ordinary failure versus interruption | L1 public results, real exit statuses, cancellation and Unix signal tests | Passing |
| Committed-tree selection and checkout transitions | L1 state-transition tests, actual detached worktrees and cleanup | Previous defect verified fixed |
| Changed paths and evidence-applied constraints | L1 Git fixtures, shared Rust/Python constraint fixtures and public orchestration | Passing |
| Python version/fallback/timeout and helper propagation | L1 real executable probes and real-recipe consumer | Functional assertions pass; duration findings remain |
| Scope/validation eligibility, retry, retained reports, advisory publication failure | L1 public orchestration and actual helper/recipe receipts | Functional assertions pass; duration findings remain |
| Thin launcher, dispatcher, argument/stdin/status forwarding, worktree selection | L1 actual Git hook invocation; both native-Windows entry points remain enabled | Appropriate level; duration finding includes launcher consumers |
| Shell suite retirement, parity inventory, owning targets and docs | Implementation parity mapping, target/recipe declarations, README/topic/testing/skill inspection | No additional gap found |

These requirements concern process, Git, and filesystem behavior. No terminal-rendering or keyboard-encoder acceptance criterion requires L2/L3. Cross-OS execution evidence itself is not a readiness finding.

## Input Robustness

The production readers retain one strict JSON loader and shared field validators. Default invalid input produces a located error, blocks review before gates, and publishes nothing. Exceptions are defined: empty PR/cell arrays mean no entries; absent or empty optional constraint strings are wildcards; absent constraint-view `state` supports older plans, while null is rejected.

The fresh library suite executes the controls and shared-validator matrix in `tests/documents.rs`: absent, null, wrong whole-field type, wrong collection elements, and empty values; field wiring; positive integers; calendar dates; duplicate keys; trailing content; comments; trailing commas; and empty documents. Public orchestration/process cases establish refusal before execution/publication for malformed PR output, plans, and constraints. There is no newly added production loader in the repair.

The requested source search found no load-bearing `serde(default)` or filtered element coercion in these readers. Date `.ok()?` calls propagate to located errors. HEAD `.ok()` calls fail closed at authorization checks. Receipt `.ok()` only derives a diagnostic count. Ref/selection defaults are guarded by their validated input contracts. The new test-tool `copy_files` setting is optional fixture behavior; absence intentionally means no copies, and wrong types fail deserialization. No additional robustness finding is warranted.
