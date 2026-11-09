---
implementation_1: "2026-11-08T18:18:54-08:00"
---

# Log

## Implementation of Review Findings #1

> **started at:** 2026-11-08T18:18:54-08:00

- this implementation is attempting to implement _all_ of the review findings found in '/Volumes/coding/rusty-biscuit/fixes/2026-11-06-proven-push-planner-cell/review-1.md'
- this is iteration 1 of the review-to-implement cycle
- the review's `implemented` flag was already `false`; it was left `false` before repairs began
- the review has one parent finding (R1-F1, medium), no `human_in_the_loop` items, and no blockers
- the orchestrator performed the single small repair directly rather than through a subagent: one finding in one validator, with no parallel work to fan out

### Finding Ledger

| ID      | Finding or required instance/consumer and expected result | Disposition | Verification evidence or blocker |
| ------- | --------------------------------------------------------- | ----------- | -------------------------------- |
| R1-F1   | Malformed producer records escape the shared validator as exceptions | fixed | all children below fixed or clean; the class sweep found 17 raising sites (2 named, 15 siblings), all repaired and each pinned by a subtest that fails on the pre-repair validator |
| R1-F1.1 | Producer `for` absent → existing missing-key problem, no `KeyError` | fixed | the lookup now admits only records with exactly `{name, for}` and a string name; `test_malformed_producer_records_are_coded_problems[for absent]` → `exactly 'name' and 'for'` problem |
| R1-F1.2 | Producer `name` `[]`/`{}` → invalid-name problem, no `TypeError` | fixed | scheduled-membership check guarded by `isinstance(str)`; subtests `name []`, `name {}`, `name ['ubuntu-latest']`, `name [3]` → `unknown-environment: … name is …` |
| R1-F1.3 | Producer field shapes → coded problems | clean | same test: name absent, null, `3`, `""`; for absent, null, `3`, `{}`, `""`, `[]`, `["wsl2-ubuntu", 3]`, `[3]`, self-guest `["ubuntu-latest"]`, all refused with coded problems; unedited control validates |
| R1-F1.4 | Producer collection absent/null/number/object/empty/non-record elements | clean | same test: absence yields no producer problem; null, `3`, `{}`, `[]`, `[3]`, `["ubuntu-latest"]`, `[[]]` refused with `resolved plan producing_environments` problems |
| R1-F1.5 | Cell/environment relationship check | clean | existing `test_the_validator_accepts_the_planned_documents`, `…refuses_a_cell_on_an_environment_the_plan_lacks`, `…on_a_producer_only_environment`, `…reused_cell_on_an_absent_environment` still pass |
| R1-F1.6 | Evidence overlay → contextual `RuntimeError` with coded problem | fixed | every malformed case asserts `apply_accepted_cells` raises `RuntimeError("the plan to apply evidence to is invalid: <first coded problem>")`; unedited nightly overlay preserves `producing_environments` |
| R1-F1.7 | CLI carried-plan path → nonzero exit, coded reason, no publication | fixed | `test_the_cli_refuses_a_carried_plan_with_a_malformed_producer` spawns `affected_scope.py --apply-to … --plan-out …` for `for` absent and a list `name`: nonzero exit, coded reason on stderr, no `KeyError`/`TypeError`, empty stdout, no plan file |
| R1-F1.8 | Sibling: environment table `name` unhashable (`scheduled` set, `_build_records` environment names) | fixed | both sets now admit only string names; also `environments` null/number no longer crash the `scheduled` comprehension; subtest `environment table name` |
| R1-F1.9 | Sibling: `deferred_environments`/`proven_environments` name unhashable | fixed | membership guarded; subtest `proven environment name` (deferred shares the same loop) |
| R1-F1.10 | Sibling: area record `area` and package record `package` used as dict keys | fixed | non-string identity reported as `malformed-receipt` and skipped; package `area` lookup guarded; subtests `area record area`, `package record package`, `package record area` |
| R1-F1.11 | Sibling: `reverse_dependencies` and `dependent_seam.dependents` membership in packages | fixed | membership guarded (`_str_list` already reports the type); subtests `reverse dependency`, `compiled dependent` |
| R1-F1.12 | Sibling: cell `package`/`environment`/`gate` lookups and skip-policy identity set | fixed | package lookup computed once with type guard (also feeds `_cell_backends`); skip-policy set admits only string triples; subtests `cell package`, `cell environment`, `cell gate` |
| R1-F1.13 | Sibling: dispatch-row duplicate key | fixed | non-string row package reported; duplicate check runs only on string triples; subtests `row package`, `row environment` |
| R1-F1.14 | Sibling: build `key`, `package`, `producer`, `compatible_environments`, consumer pairs, and cell `build` reference | fixed | non-string key reported then skipped; other lookups guarded; subtests `build key`, `build package`, `build producer`, `build compatible environment`, `build consumer environment`, `cell build reference` |

### Work

- starting the work on 'R1-F1 malformed producer records' at 18:19:20
        - defect class: an identity value from a carried document (a name or key) is used as a dictionary key or set member before the validator has established that it is a string, so a list or object escapes as `TypeError`/`KeyError` instead of a coded problem
        - sweep method: a throwaway script replaced every scalar leaf of four real planner outputs (nightly, pull request, proven push, pull request with reused L1) with `[]` and `{}` and recorded each exception site in `validate_resolved_plan` and its helpers
                - 17 raising sites found: the 2 the review named (producer `for` lookup, producer `name` membership) plus 15 siblings in `_rows`, area/package records, reverse dependencies, dependent seam, cell identity, skip-policy identities, environment table names, deferred/proven names, `_build_records`, `_build_consumers`, and `_build_references`
                - after the repair the same sweep raises nowhere, and `environments` null/number no longer crashes the `scheduled` comprehension
        - repair: type guards at each lookup, and two new coded problems for non-string area and package record identities; no new parsing layer, no document-format change, schema version unchanged
                - the producer lookup now admits only records that passed the exact `{name, for}` shape check and carry a string name
                - the cell's package record is now looked up once with a type guard and the result passed to `_cell_backends`, so a later lookup cannot reintroduce the crash
        - tests added to `DependentCheckSchedulingTests`: `test_malformed_producer_records_are_coded_problems`, `test_non_name_identities_are_coded_problems_not_exceptions` (19 sites × `[]`/`{}`), and `test_the_cli_refuses_a_carried_plan_with_a_malformed_producer`; every in-process case also goes through `apply_accepted_cells`
                - load-bearing check: with the pre-repair `schema.py` swapped in, every new subtest (26 distinct labels, including both CLI cases) errors; with the repair, all 13 tests in the class pass in 0.48 s
        - checked but outside this class (nothing raises, indexes, or tests membership): the same sweep shows the validator accepts `[]`/`{}` in free-form or table-copied fields such as `environments[].runner`, `environments[].build.*`, `flags.*`, `preflight_reason`, and `cells[].evidence.*`
                - that is a type-strictness question about fields this spec does not touch, not an exception escape, so it is recorded here rather than repaired
        - the Python suites need no lint recipe: `ruff` is not installed and the repo defines no Python lint recipe; `py_compile` of both edited files passes
- work completed for 'R1-F1 malformed producer records' at 18:27:00

### Completion Report

- **complete**: started 18:18:54, finished 18:34:12 (about 15 minutes)
- parent findings: total 1 = fixed 1 + already fixed 0 + blocked 0 + unresolved 0 + disputed 0
        - instance rows: R1-F1.1, .2, .6, .7 fixed; .3, .4, .5 clean; sweep siblings R1-F1.8 to R1-F1.14 fixed
- open findings: none
- verified repairs: the shared resolved-plan validator now returns coded problems, never `KeyError`/`TypeError`, for every unhashable or missing identity found by a leaf-by-leaf sweep of real planner output; the evidence overlay and the `--apply-to` CLI surface the first coded problem and publish nothing
- changed files
        - `scripts/ci/schema.py`: type guards at 17 lookup and membership sites; producer lookup restricted to shape-valid records
        - `scripts/ci/test_affected_scope.py`: three regression tests in `DependentCheckSchedulingTests`
        - this log
- final verification (macOS host)
        - `python3 suite_runner.py test_affected_scope.py`: 491 tests OK in 671 s (R1-F1, all rows; includes the 3 new tests)
        - `python3 suite_runner.py test_schema.py`: 203 OK; `test_resolved_plan.py`: 103 OK (shared validator consumers)
        - `test_completion.py`, `test_reuse_validation.py`, `test_runner_loss.py`: OK
        - `test_hosted_observations.py`: 44 setup errors from `git commit` in temporary fixture repositories (host signing key expired, as the implementation log recorded); with `commit.gpgsign=false` injected only through `GIT_CONFIG_*` for that run: OK. This is unrelated to the repair
        - `py_compile` of both edited files: OK; no Python linter is installed or configured (`ruff` absent)
        - no Rust or workflow file changed in this cycle, so `just test test-toolkit`, Clippy, and `actionlint` were not rerun
- no performance measurement was required; `deferred_perf_measurement` is not set
- no local OS test rigs are configured (`BUILD_LINUX`, `BUILD_WIN`, `BUILD_WSL`, `BUILD_MACOS` unset); cross-OS evidence is left to CI
