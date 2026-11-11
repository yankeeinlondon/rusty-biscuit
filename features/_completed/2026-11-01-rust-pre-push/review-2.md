---
$schema: feature-review.yaml
ready: false
findings:
  - 'R1-F2 (high): Real-recipe coverage is portable but still exceeds the L1 time budget'
human_review: true
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: true
reviewed_by: codex/gpt-6.1-sol
created: 2026-11-11T02:28:14-08:00
spec: 2026-11-01-rust-pre-push/spec.md
implemented: false
description: 'A **feature** review of `2026-11-01-rust-pre-push/spec.md`'
feature: 2026-11-01-rust-pre-push/review-2.md
previous: 2026-11-01-rust-pre-push/review-1.md
log: /Volumes/coding/rusty-biscuit/features/2026-11-01-rust-pre-push/log.md
implemented_by: claude/opus
human_review_items:
  - 'R1-F2.27: on native Windows every ci-push push test, including stub-gate tests that skip the recipe, is still 5.8–18 s because the Python evidence and planner helpers start about 22 Python and 43 git processes per push. The spec forbids porting them here. Decide whether the Windows 5 s budget waits for 2026-11-06-rust-evidence-client and 2026-11-06-rust-ci-planner, or whether helper call reductions move into this spec.'
  - 'R1-F2.27.1–.4: the first push of each two-push test is seeded by a stub gate that stages the same reports the real recipe stages; the real recipe runs on the second push; assert_retained_reports pins the equivalence. Confirm this test layout.'
next: 2026-11-01-rust-pre-push/review-3.md
---

# Review 2

The feature is not production ready. The checkout-state defect is fixed, and the real-recipe tests now compile on Windows. One carried finding remains: these L1 tests exceed the repository's five-second limit. No author decision is needed to continue repair. No new functional finding was identified.

## Previous Finding Verification

The required instances below come from review-1.md and the repair's `Finding Ledger` in log.md. Verification used the current source and a fresh `just test-pre-push-hook` run. Windows durations below are explicitly attributed to the repair log; this review did not rerun that host. There are no earlier reviews or human rulings to carry forward.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
|---|---|---|---|---|
| R1-F1: Checkout reuse relies on stale cleanliness and HEAD snapshots | R1-F1.1–R1-F1.11 | verified fixed | Fresh public-orchestration tests pass; inspected tree, gate, and publication decisions individually below | Nothing |
| R1-F1.1: Initial capture | Starting identity and cleanliness | verified fixed | `review_push` retains starting identity and dirtiness; later authorization now reads current state | Nothing |
| R1-F1.2: Dirty before tree selection | Dirtied during interpreter selection | verified fixed | `checkout_departure` reads status; `a_checkout_that_changes_before_tree_selection_is_not_reused::case_1_dirtied` proves temporary-tree planning and committed scope publication | Nothing |
| R1-F1.3: HEAD changes before tree selection | Moved HEAD; unreadable HEAD/status | verified fixed | Same public-boundary test's cases 2–4 pass; current HEAD/status failures select a temporary tree | Nothing |
| R1-F1.4: Dirty before gates | Dirtied or unreadable status during scope publication | verified fixed | `run_gates` refreshes status; `a_checkout_that_changes_before_the_gates_replans_and_publishes_no_validation` cases 1–2 assert no plan input and no validation recording | Nothing |
| R1-F1.5: HEAD changes before gates | Moved or unreadable HEAD | verified fixed | Same test's cases 3–4 pass; current identity rejects plan reuse | Nothing |
| R1-F1.6: Final publication checks | Dirty or changed HEAD after gates | verified fixed | `withhold_reason` reads current identity/status; final-state test cases pass | Nothing |
| R1-F1.7: Starting-state controls | Initially dirty, non-HEAD, selection override, clean checkout | verified fixed | Existing orchestration and process controls pass; original dirtiness independently withholds validation | Nothing |
| R1-F1.8: Scope-only publication | Publication without a post-gate check | verified fixed | `publish_scope` consumes staged committed receipt bytes; dirty selection case publishes scope from a temporary tree | Nothing |
| R1-F1.9: First-context selection | Receipt identity and comparison selection | verified fixed | `review_context` captures only the first eligible context; identity is distinct from refreshed tree authorization | Nothing |
| R1-F1.10: Helper working directories | Verify, apply, scope-record | verified fixed | `review_context`/`record_scope` obtain scripts from `target.root` and pass explicit base/head; transition test checks the planner's temporary-tree path | Nothing |
| R1-F1.11: Outgoing HEAD membership | `pushes_head` and downstream consumers | verified fixed | Membership remains tied to the captured outgoing identity; pre-gate and final checks separately reject a moved checkout | Nothing |
| R1-F2: Real recipe receipt and retry coverage is excluded from Windows | R1-F2.1–R1-F2.27 | partially fixed | Portable recipe tests and fallback consumer are now enabled; fresh suite independently reproduces the outstanding duration defect | R1-F2.27 only |
| R1-F2.1: Advanced target receipt reuse | Real recipe, two pushes | verified fixed | Unix guard removed; real-recipe fixture supplies Windows Bash; macOS process test passes | Duration tracked separately below |
| R1-F2.2: Documentation-only follow-up | Prior-cell reuse | verified fixed | Guard removed; recipe and receipt assertions pass | Duration tracked separately below |
| R1-F2.3: Changed package inputs | Prior-cell invalidation | verified fixed | Guard removed; recipe and receipt assertions pass | Duration tracked separately below |
| R1-F2.4: Failed-cell retry | Failed evidence superseded by passing evidence | verified fixed | Guard removed; public helper verification asserts newest passing evidence; test passes | Duration tracked separately below |
| R1-F2.5: Stacked pull request | Target tip recorded as base | verified fixed | Guard removed; receipt base and scope identity assertions pass | Duration tracked separately below |
| R1-F2.6: Recipe interruption | Malformed manifest, exit 130, recorded 143 | verified fixed | All three manufactured-outcome cases are portable and pass | Duration tracked separately below |
| R1-F2.7: Actual Unix signals | Three signal-injection tests and portable cancellation counterpart | verified fixed | Unix-only calls remain guarded; portable cancellation and manufactured-interruption tests pass | Nothing |
| R1-F2.8: Windows launchers | Git Bash and PowerShell-launched Git | verified fixed | Both platform-specific hook tests remain declared; source invokes actual Git with the compiled fixture tools | Nothing; OS execution is CI evidence |
| R1-F2.9: Fallback consumer | Broken first Python candidate and executable/prefix consumption in real recipe | verified fixed | New portable test passes; Windows branch selects `py -3`, checks prefix on each logged recipe call, verifies both actual helpers ran, and checks validation publication | Duration tracked separately below |
| R1-F2.10–R1-F2.23: Other platform guards | Dispatcher, process lifecycle, launcher/Python helpers, byte names, permissions, platform status construction | verified fixed | Inspected platform guards: remaining exclusions use platform-specific APIs or have Windows alternatives; no additional recipe path is excluded | Nothing |
| R1-F2.24: Non-Unicode selection | Portable rejection before discovery | verified fixed | Unguarded test uses invalid UTF-8 on Unix and an unpaired surrogate on Windows; macOS case passes | Nothing |
| R1-F2.25: Detached-tree line endings | Fixture configuration reaches production worktree creation | verified fixed | Shared Git fixture persists `core.autocrlf=false`; detached-tree tests pass | Nothing |
| R1-F2.26: Archive coordinator executable path | Bash invocation of a Windows executable | verified fixed | `produce_shared_deps` passes a forward-slash executable path to `CI_BUILD_BIN`, removing Bash's command-word lookup ambiguity | Nothing for this path defect |
| R1-F2.27: Recipe test duration | Every real-recipe parity test | unresolved | Fresh nextest run: failed-cell retry takes 5.064 s; repair log reports 5.7–12.3 s alone and up to about 22 s under Windows package load | Reduce work while preserving assertions and portable recipe coverage |

## Unblocked Findings

### R1-F2 (high): Real-recipe coverage is portable but still exceeds the L1 time budget

**Authority — spec and review testing rules:** the spec's Tests section requires these parity behaviors to be L1 nextest tests on every supported OS. The supplied testing rules state, “An L1 test over 5 s counts as failing.” The existing fixture repositories and normal full-suite execution reach this problem; no unusual input is needed. This is the remaining R1-F2.27 instance from the repair ledger, not a claim that Windows compilation guards remain.

**Defect class:** repeated end-to-end setup and helper subprocess work make portable L1 acceptance tests exceed their execution budget.

In repo-deps, [the real-recipe parity tests](../../scripts/ci-push/integration/parity.rs:186) repeatedly invoke the full hook and Bash recipe to establish evidence before testing a follow-up. The failed-cell retry test also starts a final public evidence-verification helper. The portability repair removed the guards correctly but retained this expensive work. Hard-linking fixture executables reduces scanning overhead; it does not remove the repeated hook/helper execution.

The complete bounded class is the seven functions using `stage_ci_local_harness`, with three generated interruption cases. Each uses the same real recipe and helper boundary. The table keeps the ledger's open child R1-F2.27 and assigns its concrete sites sub-IDs for the next repair. All functional assertions passed in this review's macOS suite. Windows timings are the repair log's measurements, not fresh measurements by this reviewer.

| Child ID | Site in `integration/parity.rs` | Shape tested | Observed result | Expected result |
|---|---|---|---|---|
| R1-F2.27.1 | `an_advanced_non_main_target_reuses_prior_cells_and_publishes_exact_receipts` | Two pushes, target advances | macOS 4.041 s; logged Windows 10.5 s alone / 19.1 s loaded | Same receipt/reuse assertions within 5 s |
| R1-F2.27.2 | `a_docs_only_follow_up_skips_the_cells_prior_evidence_covers` | Two pushes, docs-only follow-up | macOS passes; logged Windows 10.5 s / 19.3 s | Same skip and receipt assertions within 5 s |
| R1-F2.27.3 | `a_follow_up_changing_a_packages_inputs_reruns_its_cell` | Two pushes, changed package input | macOS 4.518 s; logged Windows 10.7 s / 19.1 s | Same invalidation assertions within 5 s |
| R1-F2.27.4 | `a_cell_whose_prior_evidence_is_a_failure_is_rerun_and_superseded` | Two pushes and final verify | **Fresh macOS 5.064 s**; logged Windows 12.3 s / 21.8 s | Same retry/newest-evidence assertions within 5 s |
| R1-F2.27.5 | `a_stacked_pull_request_records_its_target_tip_as_the_receipt_base` | One real-recipe push | macOS 2.212 s; logged Windows 5.7 s / 10.1 s | Same base/identity assertions within 5 s |
| R1-F2.27.6 | `a_real_recipe_interruption_withholds_earlier_reports`, malformed-manifest case | One interrupted recipe | Functional pass; logged Windows 4.4–4.6 s alone, about 8.3 s loaded across these cases | Preserve completion refusal within 5 s under suite load |
| R1-F2.27.7 | Same function, exit-130 case | One interrupted recipe | Functional pass; same logged duration range | Preserve interruption refusal within 5 s under suite load |
| R1-F2.27.8 | Same function, recorded-143 case | One interrupted recipe | Functional pass; same logged duration range | Preserve interruption refusal within 5 s under suite load |
| R1-F2.27.9 | `a_real_recipe_runs_its_helpers_under_the_selected_fallback_interpreter` | Broken Python candidate, fallback consumed by recipe | Functional pass; logged Windows 5.7 s alone | Preserve executable/prefix and helper assertions within 5 s |

The remaining orchestration and stub-gate process tests are clean controls for this class: they do not run this real-recipe setup, and none exceeded five seconds in the fresh hook suite. Platform launcher and actual signal tests remain separate requirements and must stay enabled in their appropriate environments.

**Recommended repair:** reduce redundant setup and subprocess calls, then verify the whole owning suite under ordinary concurrency. Start with repeated metadata queries and recipe parsing identified by the existing profile, since eliminating duplicate work also improves actual pushes. For follow-up tests, move independently asserted receipt-creation setup out of the expensive follow-up operation where this preserves the case-by-case parity contract; keep focused actual recipe-to-receipt coverage and the executable/prefix consumer test. Do not replace all receipt assertions with invocation logs, raise the budget, add retries, rename tests into a slower tier, or disable Windows coverage. This is a test-layout and implementation-efficiency choice within the existing scope, so it does not require an author ruling. A large new evidence abstraction would cost more than this defect warrants.

## Blocked Findings

None. The repair log's suggestion that the author choose a timing strategy does not establish a blocker. Reducing duplicate work is authorized; relaxing the five-second rule is unnecessary.

## Recurrence

R1-F2 carries forward review-1.md's incomplete portable real-recipe coverage repair. The original compilation exclusions are fixed. The remaining duration problem was identified during that repair as R1-F2.27 and was left unresolved; the fresh macOS run confirms it still exists. This is an unchanged open repair instance, not a newly discovered sibling of the checkout-state defect. The table above now enumerates every real-recipe consumer to repair together. R1-F1 does not recur.

## Spec and Test Verification

`just test-pre-push-hook` completed successfully on macOS: **322 library tests and 68 integration/process tests passed**. Nextest nevertheless reported the 5.064-second L1 test listed above. Passing exit status does not waive that testing rule. No production source was edited for this review.

| User-facing requirement | Strongest applicable verification inspected | Assessment |
|---|---|---|
| Parse complete updates, reject malformed lines, accept CRLF, skip non-branch/deletion updates | L1 public orchestration and parsing tests, plus malformed/empty process wiring cases | Passing |
| Review every PR context, simultaneous base updates, provisional plans; fail closed on review errors | L1 public service-boundary tests and fixture-repository process cases | Passing |
| Resolve auto/strict/warn/scope-only/off and preserve failure/interruption precedence | L1 public orchestration, real subprocess statuses and Unix signals | Passing |
| Review committed trees, refresh checkout identity/status, clean temporary trees | L1 deterministic state-transition tests and actual Git worktree/process cleanup tests | Passing |
| Compute committed changed paths and enforce evidence-applied constraints | L1 Git fixtures, shared Python/Rust constraint fixtures, prohibited/reused-cell orchestration | Passing |
| Select supported Python with bounded probes and propagate executable/prefix to helpers | L1 probe and real-helper process tests; portable real-recipe consumer test | Functionally passing; timing remains open |
| Preserve receipt eligibility, reports, retry behavior, and advisory publication failures | L1 public orchestration and actual helper/recipe receipt tests | Functionally passing; timing remains open |
| Thin launcher and worktree-aware dispatcher preserve Git arguments, stdin, status, and checkout | L1 actual Git hook invocation; Windows-specific Git Bash/PowerShell tests remain compiled there | Appropriate level; macOS controls pass |
| Register tests and retire old shell workflow; update current documentation | Declared integration target, root recipe selection, embedded launcher inputs, docs/source inspection | No additional gap found |

These are process, Git, and file-behavior requirements. They do not need a terminal emulator or OS keyboard injection, so L2/L3 absence is not a gap. Native Windows execution evidence remains CI's responsibility and is not itself a readiness finding.

## Input Robustness

The readers share one strict JSON loader. Default invalid-shape behavior is a located error; a review failure runs no gates and publishes nothing. Exceptions remain intentional: empty PR/cell arrays mean no entries; absent or empty optional constraint strings mean wildcards; an absent constraint-view `state` supports older plans, while null is rejected.

Inspected and reran the public-parser tests in `tests/documents.rs`: controls, absent/null/wrong-type/empty text fields, positive integers, calendar dates, collection roots and elements, field wiring, duplicate keys, trailing content, comments, trailing commas, and empty documents. The public orchestration suite additionally checks unreadable plans and malformed constraints prevent execution/publication. No additional matrix defect was found.

The requested search for permissive parsing found no load-bearing `serde(default)` or element-filtering coercion in these readers. Date `.ok()?` calls propagate failure to a located date error. Remote/ref defaults are constrained by explicit identity and reviewed-ref contracts; receipt cell counting in `evidence.rs` is diagnostic, not authorization. The portability repair adds no new configuration loader.
