# Legible CI names: naming evidence and contract

This file is the review artifact for the naming fallback. Phase 1 wrote the
evidence and the frozen contract. Phase 3 adds test outcomes and corroborated
names.

## Decision: automatic matrix labels (the approved fallback)

The spec's ruling `static-matrix-identity` permitted a display-only `label`
matrix field, with a fallback to automatic matrix labels if the label could
not render legibly without losing identity. The plan chose the fallback
without a display experiment, for these reasons from the source:

- `scripts/ci/test_runner_loss.py::job_label` models GitHub's rule: a declared
  `name:` wins over the automatic matrix values. A static job name would hide
  the `(package, gate, environment, runner)` suffix that
  `runner_loss.py::parse_job_name` / `row_cell` read for a lost runner.
- A name built from matrix expressions is prohibited by
  `no_skippable_job_is_labelled_with_an_unresolved_expression` and
  `every_row_expanding_job_carries_a_scalar_guard_and_no_expression_name` in
  `tools/test-toolkit/tests/ci_workflow_contracts.rs`. A skipped job never
  evaluates its matrix, so the raw `${{ … }}` text would be shown.
- `scripts/ci/affected_scope.py::row_sets` emits exactly four row values
  (`package, gate, environment, runner`); the observed names below show all
  four in each automatic suffix. Adding a fifth value would change the row
  schema (`schema.py::ROW_FIELDS`, `.github/ci/schemas/contract.json`,
  `test_resolved_plan.py::RowAdapterOracleTests.ROW_KEYS`).

So `ci.yml`'s `area-ci` and `build`, and `_package-ci.yml`'s `check`, `test`,
`lint`, and `wsl2` keep **no `name:`**. Their suffixes keep stored wording
(`L1`, `wsl2-ubuntu`) and the runner value is repeated
(`ubuntu-latest, ubuntu-latest`). That is an accepted limitation of the
fallback. Descriptive names go on the caller, infrastructure jobs, and steps.

No disposable workflow was created and no CI run was triggered.

## Evidence sources (read-only `gh api`, 2026-11-10)

| Source | Result |
| --- | --- |
| `GET repos/yankeeinlondon/rusty-biscuit/actions/runs/42491863852/jobs` (nightly `schedule` run, 2026-11-08, head `a816f144`) | 222 jobs: 78 failure, 97 skipped, 47 success |
| `GET repos/yankeeinlondon/rusty-biscuit/commits/5a2760470df3cec36fe8668859efd5549ca4dd12/check-runs` (PR head) | 93 check runs |

Both calls succeeded on the first attempt. The plan noted that a separate
job-list read for PR run `41407518248` returned HTTP 502 during planning; it
was not retried, and the commit check-runs list above is the PR evidence.

## Observed names (current, before the rename)

All names below were read from the two sources above, verbatim.

**Native execution (Linux, PR checks):**

```text
area-ci (biscuit-file) / package-ci / test (biscuit-file, L1, ubuntu-latest, ubuntu-latest)
area-ci (biscuit-file) / package-ci / check (biscuit-file, check, ubuntu-latest, ubuntu-latest)
area-ci (biscuit-file) / package-ci / lint (biscuit-file, lint, ubuntu-latest, ubuntu-latest)
area-ci (darkmatter) / package-ci / test (darkmatter, browser, ubuntu-latest, ubuntu-latest)
```

**Native execution (Windows, nightly):**

```text
area-ci (messenger) / package-ci / test (messenger, L1, windows-latest, windows-latest)
```

**Two test levels for one package (PR checks):**

```text
area-ci (claudine) / package-ci / test (claudine-cli, L1, ubuntu-latest, ubuntu-latest)
area-ci (claudine) / package-ci / test (claudine-cli, L2, ubuntu-latest, ubuntu-latest)
```

**WSL2 nesting (nightly):**

```text
area-ci (schematic) / package-ci / wsl2 (schematic-gen, L1, wsl2-ubuntu, windows-latest) / test (wsl2-ubuntu)
area-ci (messenger) / package-ci / wsl2 (messenger, L1, wsl2-ubuntu, windows-latest) / test (wsl2-ubuntu)
```

**Nested area (nightly):**

```text
area-ci (claudine/rendezvous) / package-ci / wsl2 (rendezvous-core, L1, wsl2-ubuntu, windows-latest) / test (wsl2-ubuntu)
area-ci (claudine/rendezvous) / coverage-audit
area-ci (claudine/rendezvous) / accepted-gaps
```

**Empty selection / skipped placeholders (both sources):**

```text
area-ci (darkmatter/dmls) / package-ci            # skipped: the area made no package call (PR)
area-ci (tools) / package-ci                      # skipped: same, nightly
area-ci (research) / package-ci / test            # skipped: a matrix that never expanded
area-ci (messenger) / package-ci / check          # skipped
area-ci (biscuit-file) / package-ci / wsl2        # skipped: WSL2 not scheduled for a PR
```

No skipped name in either source contains `${{`. The one unexpanded
expression seen, `Work counts (${{ matrix.os }})`, belongs to a workflow
outside this feature's scope.

**Infrastructure jobs:**

```text
Check reusable PR validation
Determine affected scope
preflight (ubuntu-latest)
build (ubuntu-latest)
build (windows-latest)
area-drift (planner areas match sniff)
ci-gate
certify-nightly
publish-observations
ci-reporting (advisory)
```

A `build (<runner>)` job is a build owner preparing test archives; the suffix
is the owner's runner, not an execution environment. The name stays automatic
because `runner_loss.py::parse_build_job_name` reads it.

**Accepted-gap checks (PR checks, neutral):**

```text
accepted gap: darkmatter / darkmatter / L2 (macos-latest)
accepted gap: darkmatter / darkmatter / browser (macos-latest)
accepted gap: darkmatter / darkmatter / L2 (ubuntu-latest)
```

## The frozen contract (applied in Phase 2)

### Workflow titles and job names

A dash means "keep automatic naming"; the job keeps no `name:` key.

| Workflow | Title | Job identifier → displayed name |
| --- | --- | --- |
| `ci.yml` | `Continuous integration` | `validation` → `Check prior pull request validation`; `scope` → `Choose required work`; `preflight` → `Verify build prerequisites`; `build` → —; `area-ci` → —; `area-drift` → `Verify package area mapping`; `ci-gate` → `ci-gate`; `certify-nightly` → `Publish nightly coverage evidence`; `publish-observations` → `Publish test observations`; `ci-reporting` → `Report CI results (advisory)` |
| `_area-ci.yml` | `Package area checks` | `package-ci` → `Run package work`; `accepted-gaps` → `Publish accepted capability gaps`; `coverage-audit` → `Check required results` |
| `_package-ci.yml` | `Package checks` | `check`, `test`, `lint`, `wsl2` → — |
| `_wsl-ci.yml` | `WSL2 tests` | `wsl` → `Run guest tests` |

### Step renames

| Workflow | Existing step name | Replacement |
| --- | --- | --- |
| `_package-ci.yml` | `cargo check (declared kinds no test gate compiles)` | `Check compilation of declared targets` |
| `_package-ci.yml` | `Tests` | `Run planned tests` |
| `_package-ci.yml` | `Companion suites` | `Run companion test suites` |
| `_package-ci.yml` | `Lint` | `Lint source` |
| `_package-ci.yml` | `Companion suites (lint)` | `Run companion lint suites` |
| `_package-ci.yml`, `_wsl-ci.yml` | `Certify this cell's completeness` | `Verify all expected results reported` |
| `_wsl-ci.yml` | `L1 tests from the archive` | `Run Level 1 tests from the archive` |
| `_wsl-ci.yml` | `Upload wsl2-ubuntu L1 JUnit` | `Upload WSL2 Level 1 test results` |
| `_wsl-ci.yml` | `Guest post-mortem` | `Collect guest diagnostics` |
| `_wsl-ci.yml` | `Host post-mortem` | `Collect host diagnostics` |
| `_area-ci.yml` | `Enforce coverage completeness and exceptions` | `Verify required results and policy exceptions` |

`Compile unchanged dependents` stays verbatim. Every existing step name in the
left column was confirmed present in its workflow on 2026-11-10.

### Accepted-gap check name

`ACCEPTED GAP: <package>: <action> on <environment>`, with actions `L1` →
`run Level 1 tests`, `L2` → `run Level 2 tests`, `browser` →
`run browser tests`, `lint` → `lint source`, `check` → `check compilation`, and
environments `ubuntu-latest` → `Linux`, `macos-latest` → `macOS`,
`windows-latest` → `Windows`, `wsl2-ubuntu` → `WSL2`. An unknown gate or
environment is refused before anything is published.

## Projected names (not yet observed)

These follow from the contract and GitHub's documented rendering. They are
**projections**, to be corroborated against a run after the change merges.

```text
area-ci (research) / Run package work / test (research, L1, ubuntu-latest, ubuntu-latest)
area-ci (research) / Run package work / test (research, L2, ubuntu-latest, ubuntu-latest)
area-ci (schematic) / Run package work / wsl2 (schematic-gen, L1, wsl2-ubuntu, windows-latest) / Run guest tests
area-ci (claudine/rendezvous) / Check required results
area-ci (claudine/rendezvous) / Publish accepted capability gaps
area-ci (tools) / Run package work                 # skipped: empty selection
area-ci (research) / Run package work / test       # skipped matrix, bare identifier
ACCEPTED GAP: darkmatter: run Level 2 tests on macOS
ACCEPTED GAP: darkmatter: run browser tests on macOS
```

Two projections are uncertain and do not affect any reader:

- `preflight` is a matrix job over `os`. With the static name it renders
  either `Verify build prerequisites` or `Verify build prerequisites
  (ubuntu-latest)`, depending on whether GitHub appends matrix values to a
  static name. It owns no result cell.
- Whether the PR Checks tab shows the reusable workflow's caller prefix
  exactly as the Actions job list does. Both observed sources above agree for
  today's names.

## Protected structure baseline

`workflow_structure.py` (beside this file) prints the four workflows with
their presentation removed: the workflow title, every job's `name:`, and every
step's `name:`. Everything else stays, including artifact `name` fields under
`with:`, commands, guards, `needs`, matrices, permissions, runners, and
timeouts. YAML comments are not part of the parsed structure.

`workflow-structure-baseline.json` is its output before the rename. To compare
after the rename, from the repository root:

```sh
python3 features/2026-11-06-legible-ci-names/workflow_structure.py \
  | diff features/2026-11-06-legible-ci-names/workflow-structure-baseline.json -
```

An empty diff means no protected property changed.
