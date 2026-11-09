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
created: 2026-11-08T18:34:35-08:00
spec: 2026-11-06-proven-push-planner-cell/spec.md
implemented: false
description: 'A **fix** review of `2026-11-06-proven-push-planner-cell/spec.md`'
fix: 2026-11-06-proven-push-planner-cell/review-2.md
previous: 2026-11-06-proven-push-planner-cell/review-1.md
---

# Review 2

The fix is production ready. The previous malformed-producer finding is verified fixed, including every additional instance in the repair ledger. No new findings or decisions requiring an author ruling were identified.

## Previous Finding Verification

The required instances below come from review-1.md and the repair log's Finding Ledger. Completion claims were checked against the validator, its downstream readers, and executable regressions. There are no earlier open findings or human rulings.

| Finding ID and title | Required instances | Outcome | Verification evidence | What remains |
| --- | --- | --- | --- | --- |
| R1-F1 — Malformed producer records escape the shared validator as exceptions | All children R1-F1.1–R1-F1.14 | verified fixed | All 13 `DependentCheckSchedulingTests` pass; guards inspected in `schema.py`; independent 926-case mutation sweep raises no exceptions | Nothing |
| R1-F1.1 — Missing producer guest list | Producer `for` absent | verified fixed | Producer lookup requires exact record keys; producer-shape regression returns the missing-key problem | Nothing |
| R1-F1.2 — Unhashable producer name | List and object names | verified fixed | Membership requires a string; both shapes return coded problems through validator and overlay | Nothing |
| R1-F1.3 — Producer field shapes | Missing/null/number/empty names; malformed guest-list scalars, empty list and invalid elements | verified fixed | `test_malformed_producer_records_are_coded_problems` checks the positive fixture and each invalid shape's diagnostic | Nothing |
| R1-F1.4 — Optional producer collection | Absent, null, number, object, empty list and non-record elements | verified fixed | Absence remains allowed; present invalid shapes return producer-collection problems | Nothing |
| R1-F1.5 — Cell/environment relationship | Valid producer builds; absent and producer-only cell environments, including reused cells | verified fixed | Four existing validator regressions pass; relationship check is independent of cell execution state | Nothing |
| R1-F1.6 — Evidence overlay | Malformed producer shapes | verified fixed | Shared `assert_refused` checks contextual `RuntimeError` against the validator's first coded problem for every malformed case | Nothing |
| R1-F1.7 — Carried-plan CLI | Missing `for`; list name | verified fixed | Actual CLI subprocesses exit nonzero, report the coded reason, emit no stdout and create no output plan; neither raw exception appears | Nothing |
| R1-F1.8 — Environment table identities | Unhashable environment names; null/number collection | verified fixed | Both environment-name sets admit only strings and guard collection type; name substitutions pass the rejection regression | Nothing |
| R1-F1.9 — Deferred/proven identities | Unhashable names | verified fixed | Both fields share the guarded membership loop; proven-name substitutions exercise it | Nothing |
| R1-F1.10 — Area/package identities | Area name, package name and package area as lists/objects | verified fixed | Explicit identity errors or guarded lookup; all three mutation sites return coded problems | Nothing |
| R1-F1.11 — Dependent identities | Reverse-dependency and compiled-dependent elements | verified fixed | Shared string-list validation reports invalid elements before guarded package membership; both sites tested | Nothing |
| R1-F1.12 — Cell identities | Package, environment and gate; package backend lookup and skip-policy identity set | verified fixed | All three substitutions tested; backend lookup reuses guarded package result and skip-policy keys require string triples | Nothing |
| R1-F1.13 — Dispatch-row identities | Package/environment/gate duplicate key | verified fixed | Row package and environment substitutions tested; all three components must be strings before duplicate-key membership | Nothing |
| R1-F1.14 — Build identities and references | Key, package, producer, compatible environments, consumer pairs and cell build reference | verified fixed | Build regressions exercise each ledger site; consumer ordering requires string pairs and reference lookup requires a string | Nothing |

The additional identity sites were repaired during the previous cycle's class sweep. They are verified repairs, not newly discovered siblings or partially repaired findings in this review.

## Unblocked Findings

None.

## Blocked Findings

None.

## Spec verification

These requirements concern structured planning and workflow conditions. Level 1 verification is appropriate throughout; no requirement depends on terminal rendering, keyboard encoding or a real device.

| Requirement | Verification reviewed |
| --- | --- |
| A proven push has no cells, builds, dispatch rows, scheduled areas or preflight runners, while retaining proof and dependency facts | Level 1 proven-push fixture checks the planner and projections |
| Pull requests and unproven pushes retain exactly one Linux dependent check and do not select the unchanged consumer | Level 1 fixture exercises both events and checks dispatch, package and area projections |
| Example/benchmark coverage combines with dependent coverage | Level 1 example fixture checks one combined non-reusable check and no proven-push cells; examples and benchmarks share the uncovered-target branch |
| Nightly Linux is an archive producer without Linux cells | Level 1 nightly fixture checks Windows/WSL2 cells and both archive consumer records; `calculate_scope` passes cell-bearing environments to `package_cells` |
| Accepted local L1 evidence cannot satisfy a scheduled dependent check | Level 1 fixture checks both calculated-plan and evidence-overlay paths, including successful L1 reuse beside an executing check |
| Invalid cell environments fail shared validation, including reused cells and carried plans | Level 1 validator and overlay cases; valid producer-only builds remain accepted |
| Invalid calculated plans reach neither output file nor stdout | Level 1 CLI-entry test asserts the intended validation diagnostic and no publication |
| Failed contract resolution skips uploads with empty names and creates no completion; later failures still upload named status | Level 1 workflow contracts plus cell-contract refusal tests; inspected seven guarded uploads, success-gated completion creation and unchanged required resolution steps |
| Scheduling guidance agrees with behavior | Reviewed `CLAUDE.md`, `docs/topics/ci-cd.md`, the rust-devops guidance and changed function comments: execution is mandatory only for a scheduled dependent check |

The validation runs before a calculated plan returns and before carried evidence is applied. The relationship check rejects invalid cells rather than deleting them. Schema version remains 7; no format or evidence identity changed. Pull request proof verification remains outside the repaired scheduling branch and was not weakened.

## Input robustness

The default for malformed producer metadata is a coded validation problem; the evidence overlay refuses execution and the CLI publishes no plan. The only absence exception is the optional `producing_environments` collection: omission means no producer-only environments. An explicitly empty collection is invalid.

The focused tests cover the producer name and guest-list validators with absent, null, wrong scalar/container type, empty value, mixed invalid elements and wholly invalid elements, alongside valid nightly controls. The producer collection is separately checked for absence and invalid present shapes. The CLI exercises one missing required field and one wrong-type field, checking diagnostics and no publication.

The additional identity guards preserve existing type errors rather than silently filtering malformed documents into valid plans. I independently replaced every scalar leaf with a list and an object in pull-request, nightly and proven-push planner fixtures: all 926 calls returned without exceptions. This establishes exception containment, not complete type validation of every unrelated field. The in-process rejection tests separately establish that the repaired identities produce coded errors.

The JSON loader is unchanged. Direct `schema.loads_strict` checks refused duplicate keys, trailing garbage, a second document and an empty document. No new loader or field default was introduced. Inspection of the changed paths found no load-bearing parse converted to a permissive default.

## Verification results

- `python3 -m unittest test_affected_scope.DependentCheckSchedulingTests -v`: 13 tests passed in 1.978 seconds, including all repair regressions and the original scheduling acceptance cases.
- `python3 scripts/ci/suite_runner.py test_schema.py`: 203 tests passed.
- The full affected-scope suite's repair-run result is recorded in `log.md`: 491 passed. This review reran its focused class rather than repeating the unrelated full suite.

- `python3 scripts/ci/suite_runner.py test_resolved_plan.py`: 103 tests passed in 78.460 seconds, including the cell-contract tests.
- `just test test-toolkit`: 484 tests passed, 4 skipped. The upload-guard regression passed in 0.023 seconds. Three unchanged tests exceeded five seconds; no new or repaired test was among them. Cargo also reported a nonfatal permission warning while cleaning its global registry cache; the recipe exited successfully.

`actionlint` is unavailable on this host, as in the first review; no hosted CI run was started merely to inspect expression behavior. Additional OS evidence is left to CI and does not block readiness under this review's rules.

## Recurrence

None. R1-F1 and every required instance are verified fixed; no recurring defect is reported.
