---
$schema: feature-review.yaml
ready: true
findings: []
human_review: true
human_review_items:
  - |-
    The slow acceptance tests now check the hook, the real validation recipe, and the evidence verifier separately. Independent checks connect those boundaries by comparing report contents, receipt contents, and the command and environment used to start validation. No single test now runs a hook push through the real recipe. This follows the previous review's recommendation and preserves the required behavior within the five-second test budget. The author can inspect this test structure after the review cycle; no decision is needed to complete the repair.
  - |-
    The Windows evidence-equivalence test took 4.848 and 4.968 seconds in the two inspected package runs, close to the five-second limit. The advanced-target tests also have limited margin, at up to 4.695 seconds. Keep these measurements in mind when changing their setup. Both inspected final runs satisfy the limit; this is a performance risk, not an outstanding finding.
human_in_the_loop: null
has_blocked_findings: false
blocked: false
recurrence: false
reviewed_by: codex/gpt-6.1-sol
created: 2026-11-11T06:40:49-08:00
spec: 2026-11-01-rust-pre-push/spec.md
implemented: false
description: 'A **feature** review of `2026-11-01-rust-pre-push/spec.md`'
feature: 2026-11-01-rust-pre-push/review-5.md
previous: 2026-11-01-rust-pre-push/review-4.md
---

# Review 5

The feature is production ready within this specification's Phase 1 scope. Both carried findings are verified fixed, including every remaining timing instance. No new finding or newly discovered sibling remains.

I read the specification, plan, implementation inventory, all four previous reviews, and the repair's Finding Ledger in `log.md`. No previous review contains author rulings. Recommendations for later human review are not treated as exceptions to the specification or test budget.

## Evidence and limits

Fresh verification on macOS:

- `just test-pre-push-hook`: 322 library tests and 82 integration/process tests passed; every test completed below five seconds. This includes the split recipe, publication, verifier, and launcher cases.
- `just check-tier-coverage scripts`: zero stranded tests.
- `python3 scripts/ci/test_local_evidence.py`: 31 tests passed.
- `BISCUIT_REQUIRE_JUST=1 BISCUIT_REQUIRE_JQ=1 BISCUIT_REQUIRE_BASH=1 python3 scripts/ci/test_ci_local.py`: 108 tests passed.
- `git diff --check` for the review and metadata edits: passed.

Windows timing verification uses the raw package-run output, independently inspected during this review, rather than the ledger's completion labels:

| Raw output | Recorded tree | Result | Slowest hook test |
|---|---|---|---|
| `/tmp/cipush-r4/win-full3.log` | `5367625fafea8e352a2c4cfbfdd896213b04efd7` | 908 passed, 1 skipped | Evidence equivalence, 4.848 s |
| `/tmp/cipush-r4/win-orch.log` | `5fe3a3192f7252fe7a8afdd6b2e81d2e22c9ae2c` | 908 passed, 1 skipped | Evidence equivalence, 4.968 s |

These are existing repair executions, not fresh Windows executions by this reviewer. `git diff` against the second recorded tree is empty for `scripts/ci-push/`, both hook binaries, `just/ci-local.just`, `local_evidence.py`, and `observation_notes.py`. Thus the inspected timings apply to the current hook and helper implementation. This does not claim that the entire current repository has qualifying cross-OS CI evidence. Such evidence is CI's responsibility and is not a readiness finding here.

## Previous Finding Verification

All required instances from review 4 and its repair ledger are accounted for below. The source sites are in repo-deps: [parity tests](../../scripts/ci-push/integration/parity.rs), [fixture boundaries](../../scripts/ci-push/integration/evidence_fixture.rs), and [launcher tests](../../scripts/ci-push/integration/launcher.rs). Windows durations in this table come from `win-orch.log` unless stated otherwise. Every portable counterpart also passed the fresh macOS hook run.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
|---|---|---|---|---|
| R1-F1: Checkout reuse uses stale snapshots | R1-F1.1–R1-F1.11: initial state, tree selection, gate reuse, final publication, starting controls, scope, first context, helper roots, outgoing identity | verified fixed | `review_update` calls `checkout_departure` after context discovery; `run_gates` refreshes HEAD/status; `withhold_reason` checks final state. Starting dirtiness remains independently disqualifying. First-context plans and scope use committed identities. Fresh public transition and control tests pass. | Nothing |
| R1-F2: Portable real-recipe coverage and duration | R1-F2.1–R1-F2.27, including all nine timing children below | verified fixed | Portable targets remain enabled; inspected assertion mapping and independent boundary checks; fresh macOS tests and both raw Windows package runs pass within budget. | Nothing |
| R1-F2.1–R1-F2.6: Original recipe consumers | Receipt, reuse, retry, stacked base, and three interruption shapes | verified fixed | Recipe output is checked directly for pass, complete failure, malformed manifest, exit 130, and recorded 143; hook tests separately check receipts, withholding, reuse and bases. No portable-behavior Unix exclusion. | Nothing |
| R1-F2.7–R1-F2.8: Signals and launchers | Unix signals, portable cancellation, Git Bash and PowerShell | verified fixed | Real signals remain guarded by platform API; portable cancellation remains selected. Both Windows shells execute separate auto/strict hook pushes in the raw runs. | Nothing |
| R1-F2.9: Recipe fallback consumer | Selected executable and argument prefix | verified fixed | Real recipe receives fallback and prefix, checks each helper invocation, and never returns to `python3`; hook launch contract separately checks selection and prefix bytes. | Nothing |
| R1-F2.10–R1-F2.23: Other platform controls | Dispatcher, lifecycle, probes, Git bytes/modes, permissions and status construction | verified fixed | Platform guards remain scoped to platform APIs or portable alternatives. Declared process modules and fresh library/process tests pass; no new exclusion introduced by the repair. | Nothing |
| R1-F2.24: Non-Unicode selection | Refusal before discovery on both platforms | verified fixed | Platform-specific invalid-string construction remains in `tests/run.rs`; public refusal test passes. | Nothing |
| R1-F2.25: Detached-tree line endings | Persist `core.autocrlf=false` in fixture | verified fixed | Fixture configuration remains persisted; detached-tree and masked-change controls pass. | Nothing |
| R1-F2.26: Archive executable path | Forward slashes in Bash command word on Windows | verified fixed | `ci-build-archive-tests.rs` retains the explicit path conversion; repair does not reverse it. | Nothing |
| R1-F2.27: Recipe duration | R1-F2.27.1–R1-F2.27.9 | verified fixed | Each acceptance question retains its assertions at the public boundary listed below; all replacement tests are below five seconds in both inspected final Windows runs. | Nothing |
| R1-F2.27.1: Advanced target reuse/publication | Reuse, exact base/identity/remote receipts, retained reports | verified fixed | Reuse half 4.670 s; publication half 4.563 s; real recipe skip control 3.405 s. Tests check advanced remote base, covered alpha cell, executing beta cell, receipt identity and retained bytes. | Nothing |
| R1-F2.27.2: Documentation follow-up | Prior pass, skipped covered cell, only executed cells recorded | verified fixed | Hook-fed plan 4.260 s; verifier/recording 4.374 s; real-recipe skip 3.405 s. Seed-to-hook receipt equality and retained-report checks 4.968 s. | Nothing |
| R1-F2.27.3: Changed package inputs | Refuse incompatible prior evidence and execute cell | verified fixed | Verifier asserts `gate-inputs-changed` and no accepted alpha cell, 2.886 s; real recipe pass control asserts alpha execution, 3.281 s. | Nothing |
| R1-F2.27.4: Failed prior evidence and retry | Complete failure, refusal, newest passing retry wins | verified fixed | Failure/refusal 2.902 s; newest-pass verifier 3.980 s; real recipe execution and complete-failure controls 3.281/3.316 s. Accepted result names the follow-up commit. | Nothing |
| R1-F2.27.5: Stacked pull request receipt | Target-tip base, scope identity and retained reports | verified fixed | Hook test 4.313 s asserts parent tip rather than merge base, both cells, scope identity and retained reports; recipe's matching output checked independently. | Nothing |
| R1-F2.27.6: Malformed manifest | Refuse publication of earlier reports | verified fixed | Hook refusal 2.783 s; actual recipe output/error control 3.351 s. Checks interruption marker, manifest, status and no validation note. | Nothing |
| R1-F2.27.7: Exit 130 | Interruption cannot publish earlier reports | verified fixed | Hook refusal 2.756 s; actual recipe exit/marker/report control 3.138 s. | Nothing |
| R1-F2.27.8: Recorded 143 | Manifest interruption cannot publish earlier reports | verified fixed | Hook refusal 2.788 s; actual recipe exit/marker/report control 3.584 s. | Nothing |
| R1-F2.27.9: Real recipe fallback | Executable/prefix reaches recipe helpers | verified fixed | Direct recipe 3.546 s; hook launch contract 3.172 s checks argv, cwd, staged paths, selected fallback, NUL-delimited prefix and a single failed `python3` probe. | Nothing |
| R3-F1: Additional hook consumers | R3-F1.1–R3-F1.13 | verified fixed | Three remaining slow consumers repaired; all previously clean siblings retain assertions and remain within budget in both raw Windows runs. | Nothing |
| R3-F1.1: Complete recipe failure | Failure receipt, later gate, retained reports | verified fixed | Hook receipt/retention 4.271 s; direct complete-failure recipe 3.316 s checks both gates start and stages the modeled reports/status. | Nothing |
| R3-F1.2: Strict failure publication and CI retry | Publish diagnostic failure, refuse reuse in CI | verified fixed | Publication half 3.839 s; CI verifier half 2.841 s. Real helpers produce notes, and verifier rejects failed cells. | Nothing |
| R3-F1.3: Broken first Python candidate | Propagate fallback through helpers/gates | verified fixed | 4.029 s; helper-call inventory and gate environment assertions retained. | Nothing |
| R3-F1.4: Branch creation | Remote-main base and explanation | verified fixed | 3.613 s; scope base and diagnostic assertions retained. | Nothing |
| R3-F1.5: Warn failure | Publish complete diagnostic failure | verified fixed | 4.006 s; remote complete-failure note and successful warn outcome asserted. | Nothing |
| R3-F1.6: Default constraint store | Enforcement across fresh sessions | verified fixed | 3.698 s; own repository record blocks, unrelated repository record does not. | Nothing |
| R3-F1.7: Retained readable reports | Receipt names durable reports | verified fixed | 3.980 s; retained contents and readable receipt paths asserted. | Nothing |
| R3-F1.8: Failed note publication | Durable reports survive failure | verified fixed | 3.746 s; no note published, reports retained, successful gate status preserved. | Nothing |
| R3-F1.9: PowerShell launcher | Separate auto and strict pushes | verified fixed | Actual PowerShell-launched Git: auto 3.659 s, strict 3.748 s. Each has its own fixture; remote result and gate count checked. Shared launcher forwarding controls remain. | Nothing |
| R3-F1.10: Direct launcher | Separate auto and strict pushes | verified fixed | Auto 2.692 s, strict 2.513 s; same mode assertions through actual Git hooks and real binary. | Nothing |
| R3-F1.11: Other parity/Git Bash consumers | Remaining fixture shapes and Windows shell entry | verified fixed | Git Bash auto/strict 2.553/2.497 s; inspected every hook test's raw duration, none at or above five seconds. | Nothing |
| R3-F1.12: Dispatcher/probe/supervision controls | Forwarding, bounded probes, child ownership | verified fixed | These do not start the planning/evidence pipeline. Fresh process cases and Windows counterparts pass; no timing regression. | Nothing |
| R3-F1.13: Public orchestration controls | Injected-service decisions | verified fixed | Fresh 322-test library suite passes; decisions remain tested without helper startup cost. | Nothing |
| R2-N1: Extracted failure coverage | Same instance as R3-F1.1 | verified fixed | Complete-failure recipe and hook assertions checked above. | Nothing |
| R2-N2: Additional helper consumers | Same inventory as R3-F1.2–R3-F1.10 | verified fixed | Every named consumer checked above, including both split launcher modes. | Nothing |
| R2-N3: Workflow-contract anchor | Actual archive guard remains checked | verified fixed | Contract continues to anchor the guard `if`, independently of the moved assignment. | Nothing |
| R2-N4: Retired shell assertion | Remove obsolete assertion; preserve retry coverage | verified fixed | Obsolete shell-text assertion remains absent; public retry-overlay and process/helper retry checks remain. | Nothing |

The two parent findings are fully repaired, rather than partially repaired with a new sibling left over. Splitting the direct and Git Bash launcher cases was the repair's class sweep; those replacements are checked here, not introduced as new findings.

## Unblocked Findings

None.

## Blocked Findings

None. No author decision is required to repair anything remaining.

## Recurrence

No open finding recurs. The duration class carried from reviews 1–4 is closed by the complete instance verification above. The checkout-state repair remains intact.

## Specification and test verification

All requirements below concern process, Git, filesystem, or structured-data behavior. L1 is the appropriate verification level; the specification adds no terminal-rendering or keyboard-encoder requirement that calls for L2 or L3.

| Requirement | Verification level and evidence | Assessment |
|---|---|---|
| Parse the complete update list; reject malformed input; accept CRLF; skip non-branch/deletion updates | L1 parser and public orchestration tests plus binary stdin checks | Passing |
| Main, open pull request and provisional contexts; both states of simultaneous base updates; failed GitHub lookup blocks | L1 public decisions and Git fixture process tests | Passing |
| Auto/strict/warn/scope-only/off and invalid settings | L1 public outcomes and actual hook pushes; ordinary failure distinguished from interruption | Passing |
| Committed outgoing tree, path set and temporary-tree cleanup | L1 Git tree/diff tests, actual detached worktrees, transition cases, cancellation and Unix signals | Passing |
| Evidence-applied constraints; renamed branch identities; selection affects local gates only | L1 shared Rust/Python fixture parity and public orchestration | Passing |
| Python 3.10+, executed fallback probes, timeout, clear missing-interpreter error, propagated executable/prefix | L1 real executable probes, hook launch contract and real recipe helper calls | Passing |
| Receipt eligibility, first HEAD context, retry, durable reports, advisory publication failure | L1 public outcomes, real helper/notes fixtures, recipe output equivalence, verifier refusal/newest-pass checks | Passing |
| Thin Cargo launcher and dispatcher; argument/stdin/status forwarding and invoking-worktree selection | L1 actual Git hook invocation with explicit Cargo argument checks; native Windows Bash and PowerShell cases present in raw runs | Passing |
| Portable L1 coverage and five-second test budget | Declared `ci-push` target, live recipe, fresh macOS run and inspected Windows package runs | Previous timing findings closed |
| Bash-suite retirement, case mapping, source-input ownership, docs | Implementation inventory, target/recipe declarations and README/topic/testing/skill inspection | No remaining gap found |

The boundary split is adequate: the real recipe's reports, manifest, interruption marker, exit status and returned plan are compared with the modeled gate outcome for every used outcome; the hook's launch contract checks the actual arguments/environment; seeded evidence is compared byte for byte with a hook-produced note and its retained reports. This preserves independent checks at each connection without requiring one test to pay for the whole chain twice.

The launcher retains explicit `local-tools` feature selection and Cargo freshness checking. No alternate binary selection, new evidence client, planner port, schema change, or dedicated OS workflow was introduced.

## Input robustness

The production JSON readers use one strict loader and shared validators in [documents.rs](../../scripts/ci-push/documents.rs) and [json.rs](../../scripts/ci-push/json.rs). Invalid document or field shapes produce located errors and block review before gates or publication. Exceptions remain explicit: empty pull-request/cell arrays mean no entries; absent or empty optional constraint strings mean wildcards; absent constraint-view `state` supports older plans, while explicit null is rejected.

The fresh public-API tests cover control fixtures, absent/null/wrong-type/empty values, mixed and wholly invalid collection elements, required-field wiring, positive integers and calendar dates. The shared loader is checked for duplicate keys, trailing garbage or a second value, comments, trailing commas and empty documents. Public orchestration/process tests connect malformed pull-request output, plans and constraint records to blocked execution/publication. No production loader was added by this repair.

The requested source search found no load-bearing `serde(default)` or silently filtered element conversion in these readers. Optional `state` distinguishes absence explicitly. Date parse failures become located errors; HEAD/status conversion failures deny reuse/publication; receipt parsing with `.ok()` derives only a display count. Validated branch-update contracts guard branch defaults. There is no remaining robustness finding.
