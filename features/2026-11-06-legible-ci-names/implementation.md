# Legible CI names: naming evidence and contract

This file is the review artifact for the naming fallback. Phase 1 wrote the
evidence and the frozen contract. Phase 3 adds test outcomes. The R1-F2
repair adds the display check (run 42963148323) that settles the fallback and
the rendering pattern of the renamed names.

## Decision: automatic matrix labels (the approved fallback)

The spec's ruling `static-matrix-identity` permitted a display-only `label`
matrix field, with a fallback to automatic matrix labels "if adding it loses
identity in any rendered name". The plan first chose the fallback from the
source alone, on a false premise: it read
`scripts/ci/test_runner_loss.py::job_label` ("a declared `name:` wins") as
GitHub's rule and concluded that a static job name would hide the
`(package, gate, environment, runner)` suffix. Review finding R1-F2 required
the bounded display check; it ran as run 42963148323 (next section) and
settled the question by observation:

- A static `name:` does **not** hide the suffix. GitHub appends the matrix
  values to it: `Run tests (claudine-cli, L1, ubuntu-latest, ubuntu-latest)`.
  The premise was wrong, and `job_label` has been corrected to model this.
- A `label` placed first does lead the suffix, but GitHub truncates each
  job-name segment at 100 characters (97 plus `...`). Every labeled segment
  for `rendezvous-core` (15 characters) lost its runner, and so did the
  statically named one for `schematic-gen` (13); the WSL2 rows under a static
  name lost part of the environment too:
  `test (rendezvous-core: run Level 1 tests on Linux, rendezvous-core, L1, ubuntu-latest, ubuntu-lat...`.
  34 of the 78 workspace packages are longer than 15 characters (longest
  `unfolded-integration-helper`, 27). The spec's own example,
  `Package work (schematic-gen: run Level 1 tests on WSL2, schematic-gen, L1, wsl2-ubuntu, windows-latest)`,
  is 103 characters.

The fallback condition is therefore established: adding the label loses
identity (the runner, and sometimes part of the environment) in rendered
names. The fallback is retained; no `label` field is added. Unlabeled rows
stay well under the cap: the longest possible segment is
`test (unfolded-integration-helper, browser, windows-latest, windows-latest)`
(75 characters, computed, not observed); that package's WSL2 delegation
would read `wsl2 (unfolded-integration-helper, L1, wsl2-ubuntu, windows-latest)`
(67).

Two further facts keep the choice, independent of truncation:

- A name built from matrix expressions is prohibited by
  `no_skippable_job_is_labelled_with_an_unresolved_expression` and
  `every_row_expanding_job_carries_a_scalar_guard_and_no_expression_name` in
  `tools/test-toolkit/tests/ci_workflow_contracts.rs`. A skipped job never
  evaluates its matrix, so the raw `${{ … }}` text would be shown.
- `scripts/ci/affected_scope.py::row_sets` emits exactly four row values
  (`package, gate, environment, runner`). A fifth value would change the row
  schema (`schema.py::ROW_FIELDS`, which `validate_resolved_plan` enforces as
  an exact key set, `.github/ci/schemas/contract.json`,
  `test_resolved_plan.py::RowAdapterOracleTests.ROW_KEYS`, and
  `test_affected_scope.py`'s `assertEqual(list(schema.ROW_FIELDS), list(row))`).

So `ci.yml`'s `area-ci` and `build`, and `_package-ci.yml`'s `check`, `test`,
`lint`, and `wsl2` keep **no `name:`**. For the package jobs and `build` a
static name would keep the suffix but replace the job id that
`runner_loss.py` matches (`GATE_SEGMENT`, `WSL_DELEGATION_ROW`,
`BUILD_JOB_SEGMENT`). `area-ci` has no such reader: a static name would
render `<name> (claudine)`; it stays automatic so the area's entry keeps the
`area-ci (<directory>)` spelling the frozen contract and every recorded run
use. The suffixes keep stored wording (`L1`, `wsl2-ubuntu`) and the runner
value is repeated (`ubuntu-latest, ubuntu-latest`). That is an accepted
limitation of the fallback. Descriptive names go on the caller,
infrastructure jobs, and steps.

The chosen templates:

| Job | Expanded | Skipped |
| --- | --- | --- |
| package matrix job (`check`, `test`, `lint`) | `area-ci (<area dir>) / Run package work / <job id> (<package>, <gate>, <environment>, <runner>)` | `area-ci (<area dir>) / Run package work / <job id>` |
| WSL2 delegation | `area-ci (<area dir>) / Run package work / wsl2 (<package>, <gate>, wsl2-ubuntu, <runner>) / Run guest tests` | `area-ci (<area dir>) / Run package work / wsl2` |
| area with no executing rows | — | `area-ci (<area dir>) / Run package work` |
| static-named matrix job (`preflight`, `certify-nightly`) | `<name> (<matrix value>)` | `<name>` |
| accepted gap (check run posted by `publish_gaps.py`) | `ACCEPTED GAP: <package>: <action> on <environment words>` | — |

## Display check (run 42963148323)

A disposable workflow, not production CI, run once to learn how GitHub
renders the names. No full-scope run was triggered.

| Fact | Value |
| --- | --- |
| Run | 42963148323, <https://github.com/yankeeinlondon/rusty-biscuit/actions/runs/42963148323> |
| Head | `9315984f9b38ce8a646e78fe3fbbe02ace9c4b79`, a signed orphan commit on a scratch ref (`display-check/legible-ci-names`), deleted after the run |
| Event | `push` to the scratch ref |
| Workflows | `display-check/zz-display-check.yml`, `zz-display-area.yml`, `zz-display-package.yml`, `zz-display-wsl.yml` (beside this file); every job only echoes |
| Records | `display-check/jobs.json` (Actions jobs list) and `display-check/check-runs.json` (commit check runs), trimmed to name, status, conclusion |
| Totals | 33 jobs and 33 check runs; the two name lists are identical |

The nesting mirrors production: `zz-display-check.yml`'s unnamed `area-ci`
fans out over `["schematic", "claudine", "claudine/rendezvous", "tools"]`
and calls `zz-display-area.yml`, whose `package-ci` (`Run package work`)
runs for an area with rows and `package-ci-empty` (`Run package work for
empty selection`) runs only for the empty `tools` area. Each calls
`zz-display-package.yml`, which declares six variants, each guarded by a
scalar input comparison like production:

| Variant | Job id | `name:` | Rows | Shape |
| --- | --- | --- | --- | --- |
| V1 | `test` | none | labeled (`label` first) | `test (<label>, <package>, <gate>, <environment>, <runner>)` |
| V2 | `test-static` | `Run tests` | labeled | `Run tests (<label>, …)` |
| V3 | `test-static-nolabel` | `Run tests` | unlabeled | `Run tests (<package>, <gate>, <environment>, <runner>)` |
| V4 | `test-auto-nolabel` | none | unlabeled | `test-auto-nolabel (<package>, …)`, today's production shape |
| V5 | `wsl2` | none | labeled WSL2, calls `zz-display-wsl.yml` | `wsl2 (<label>, …) / Run guest tests` |
| V6 | `wsl2-static` | `Run WSL2 tests` | labeled WSL2, calls `zz-display-wsl.yml` | `Run WSL2 tests (<label>, …) / Run guest tests` |

Labels: `<package>: run Level 1 tests on Linux`, `run Level 2 tests on
Linux`, `run Level 1 tests on WSL2`. Rows: `schematic-gen` L1 native and
WSL2; `claudine-cli` L1 and L2 native, no WSL2; `rendezvous-core` L1 native
and WSL2; `tools` none.

### Every observed name, verbatim

Character counts are of the whole name. `[truncated]` marks a segment GitHub
cut at 100 characters; `[skipped]` marks a job that did not run. All other
jobs succeeded.

Planner and area level:

```text
Plan display check
area-ci (tools) / Run package work                                    [skipped]
area-ci (claudine) / Run package work for empty selection             [skipped]
area-ci (claudine/rendezvous) / Run package work for empty selection  [skipped]
area-ci (schematic) / Run package work for empty selection            [skipped]
```

V1, no name, labeled:

```text
area-ci (claudine) / Run package work / test (claudine-cli: run Level 1 tests on Linux, claudine-cli, L1, ubuntu-latest, ubuntu-latest)
area-ci (claudine) / Run package work / test (claudine-cli: run Level 2 tests on Linux, claudine-cli, L2, ubuntu-latest, ubuntu-latest)
area-ci (schematic) / Run package work / test (schematic-gen: run Level 1 tests on Linux, schematic-gen, L1, ubuntu-latest, ubuntu-latest)
area-ci (claudine/rendezvous) / Run package work / test (rendezvous-core: run Level 1 tests on Linux, rendezvous-core, L1, ubuntu-latest, ubuntu-lat...   [truncated]
area-ci (tools) / Run package work for empty selection / test                                                                                            [skipped]
```

V2, static name, labeled:

```text
area-ci (claudine) / Run package work / Run tests (claudine-cli: run Level 1 tests on Linux, claudine-cli, L1, ubuntu-latest, ubuntu-latest)
area-ci (claudine) / Run package work / Run tests (claudine-cli: run Level 2 tests on Linux, claudine-cli, L2, ubuntu-latest, ubuntu-latest)
area-ci (schematic) / Run package work / Run tests (schematic-gen: run Level 1 tests on Linux, schematic-gen, L1, ubuntu-latest, ubuntu-la...           [truncated]
area-ci (claudine/rendezvous) / Run package work / Run tests (rendezvous-core: run Level 1 tests on Linux, rendezvous-core, L1, ubuntu-latest, ubunt...  [truncated]
area-ci (tools) / Run package work for empty selection / Run tests                                                                                        [skipped]
```

V3, static name, unlabeled:

```text
area-ci (claudine) / Run package work / Run tests (claudine-cli, L1, ubuntu-latest, ubuntu-latest)
area-ci (claudine) / Run package work / Run tests (claudine-cli, L2, ubuntu-latest, ubuntu-latest)
area-ci (schematic) / Run package work / Run tests (schematic-gen, L1, ubuntu-latest, ubuntu-latest)
area-ci (claudine/rendezvous) / Run package work / Run tests (rendezvous-core, L1, ubuntu-latest, ubuntu-latest)
area-ci (tools) / Run package work for empty selection / Run tests    [skipped]
```

The `tools` area shows `… / Run tests` twice, once for V2 and once for V3;
the two skipped names are identical and cannot be told apart.

V4, no name, unlabeled (today's production shape):

```text
area-ci (claudine) / Run package work / test-auto-nolabel (claudine-cli, L1, ubuntu-latest, ubuntu-latest)
area-ci (claudine) / Run package work / test-auto-nolabel (claudine-cli, L2, ubuntu-latest, ubuntu-latest)
area-ci (schematic) / Run package work / test-auto-nolabel (schematic-gen, L1, ubuntu-latest, ubuntu-latest)
area-ci (claudine/rendezvous) / Run package work / test-auto-nolabel (rendezvous-core, L1, ubuntu-latest, ubuntu-latest)
area-ci (tools) / Run package work for empty selection / test-auto-nolabel    [skipped]
```

V5, no name, labeled WSL2, nested call:

```text
area-ci (schematic) / Run package work / wsl2 (schematic-gen: run Level 1 tests on WSL2, schematic-gen, L1, wsl2-ubuntu, windows-latest) / Run guest tests
area-ci (claudine/rendezvous) / Run package work / wsl2 (rendezvous-core: run Level 1 tests on WSL2, rendezvous-core, L1, wsl2-ubuntu, windows-latest) / Run guest tests
area-ci (claudine) / Run package work / wsl2                                [skipped]
area-ci (tools) / Run package work for empty selection / wsl2               [skipped]
```

V6, static name, labeled WSL2, nested call:

```text
area-ci (schematic) / Run package work / Run WSL2 tests (schematic-gen: run Level 1 tests on WSL2, schematic-gen, L1, wsl2-ubuntu, windows... / Run guest tests            [truncated]
area-ci (claudine/rendezvous) / Run package work / Run WSL2 tests (rendezvous-core: run Level 1 tests on WSL2, rendezvous-core, L1, wsl2-ubuntu, win... / Run guest tests  [truncated]
area-ci (claudine) / Run package work / Run WSL2 tests                      [skipped]
area-ci (tools) / Run package work for empty selection / Run WSL2 tests     [skipped]
```

That is 33 names: 1 + 4 + 5 + 5 + 5 + 5 + 4 + 4.

### Answers

- **(a) Does a static `name:` hide the matrix suffix?** No. On an expanded
  matrix job GitHub appends the values to it (V2, V3, V6):
  `Run tests (claudine-cli, L1, ubuntu-latest, ubuntu-latest)`.
- **(b) What does a skipped job show?** Its static name, or its job id when
  unnamed; never values or a raw expression. A skipped reusable-workflow
  caller (`uses:`) shows only its own name, with nothing under it
  (`area-ci (tools) / Run package work`).
- **(c) Does a `label` lead the suffix?** Yes, when it is the row's first
  key: values render in row key order (V1, V2, V5, V6).
- **(d) Is there a length limit?** Yes, per segment: each part between
  ` / ` is cut at 100 characters (97 plus `...`). Whole names are not cut
  (the longest observed is 169 characters). Labeled segments exceeded the
  cap for `rendezvous-core` (15 characters) in every variant and for
  `schematic-gen` (13) under a static name, losing the runner; V6 also lost
  part of the environment (`windows...`, `win...`). No unlabeled segment came
  near it (longest observed 69 characters).
- **(e) WSL2 nesting** renders `<caller> (<suffix>) / Run guest tests`; the
  guest job's static name gets no suffix because it is not a matrix job.
- **(f) Nested area** renders `area-ci (claudine/rendezvous)`; the `/` in the
  directory is not the ` / ` segment separator.
- **(g) Two levels of one package** are distinct (`claudine-cli, L1, …` and
  `claudine-cli, L2, …`) in every variant.

**Pull-request checks list.** No pull request could be opened (`gh` was not
authenticated for that), so the PR Checks tab was not observed. The commit
check-runs API, which feeds that tab, returned the same 33 names as the jobs
list. The PR UI frames each one as `<workflow title> / <check-run name>
(<event>)`; that framing is GitHub's and adds nothing the names lack.

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

These are the declared `name:` values. Two of the named jobs are matrix jobs,
and GitHub appends the matrix value when a leg runs (display check answer
(a)): `preflight` renders `Verify build prerequisites (<os>)`, for example
`Verify build prerequisites (ubuntu-latest)`, and `certify-nightly` renders
`Publish nightly coverage evidence (<environment>)`. Skipped as a whole, each
shows the bare name. Neither owns a result cell; their true labels classify
exactly as `preflight (ubuntu-latest)` did (no cell, not a build owner, not a
judge), which `test_static_named_matrix_jobs_render_a_suffix_and_stay_unclassified`
in `scripts/ci/test_runner_loss.py` pins.

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

## Renamed names: composed, with the rendering pattern observed

These names are composed from the declared names and the templates above. No
production run has carried them yet, because the change has not been pushed.
What is no longer a projection is GitHub's rendering pattern: display check
run 42963148323 observed every job-name shape below at the same nesting depth,
with only the job id and package differing.

```text
area-ci (research) / Run package work / test (research, L1, ubuntu-latest, ubuntu-latest)
area-ci (research) / Run package work / test (research, L2, ubuntu-latest, ubuntu-latest)
area-ci (schematic) / Run package work / wsl2 (schematic-gen, L1, wsl2-ubuntu, windows-latest) / Run guest tests
area-ci (claudine/rendezvous) / Check required results
area-ci (claudine/rendezvous) / Publish accepted capability gaps
area-ci (tools) / Run package work                 # skipped: empty selection
area-ci (research) / Run package work / test       # skipped matrix, bare identifier
Verify build prerequisites (ubuntu-latest)
ACCEPTED GAP: darkmatter: run Level 2 tests on macOS
ACCEPTED GAP: darkmatter: run browser tests on macOS
```

Observed versus inferred, line by line:

- The two `test (…)` lines follow V4 exactly
  (`area-ci (claudine) / Run package work / test-auto-nolabel (claudine-cli, L1, ubuntu-latest, ubuntu-latest)`
  and its `L2` sibling): unnamed, unlabeled `include:` rows, one caller level.
- The `wsl2 (…) / Run guest tests` line follows V5's shape with an
  unlabeled row. V5 was observed only with labeled rows; that the suffix then
  starts at `package` is inferred from V3 and V4, which show unlabeled rows
  rendering their four values in key order.
- The two area-level lines are static names on non-matrix jobs one level
  under `area-ci (<dir>)`, the shape of the observed
  `area-ci (claudine) / Run package work for empty selection`.
- The two skipped lines follow the observed `area-ci (tools) / Run package
  work` and `area-ci (tools) / Run package work for empty selection / test`.
- `Verify build prerequisites (ubuntu-latest)` is inferred from V2 and V3 (a
  static name on an expanded `include:` matrix gains its values) and from the
  pre-rename observation `preflight (ubuntu-latest)` for this keyed `os`
  matrix. No run showed this exact string.
- The accepted-gap names are check runs created through the Checks API by
  `scripts/ci/publish_gaps.py`, not job names, so the display check does not
  cover them. The publisher sets the whole `name`; the longest possible is
  `ACCEPTED GAP: unfolded-integration-helper: run Level 1 tests on Windows`
  (71 characters), under both the 100-character job-name segment cut and the
  255-character limit `publish_gaps.py::NAME_LIMIT` enforces. Whether the
  Checks API applies the 100-character cut to API-created runs was not
  observed; at 71 characters it cannot matter today.

The earlier uncertainty about `preflight`'s suffix is settled (it has one).
The PR Checks tab was not observed directly; the commit check-runs API that
feeds it returned names identical to the jobs list.

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

## Phase 3: verification and outcome

### Departure from the preferred design

The spec preferred a display-only `label` matrix field. The fallback was used
instead, and after review finding R1-F2 it rests on observation: display check
run 42963148323 showed GitHub cutting each name segment at 100 characters, so
a leading label loses the runner (and sometimes part of the environment) for
package names of 13 to 15 characters and up. The cell-carrying matrix jobs
keep automatic labels, so their suffixes show stored values (`L1`,
`wsl2-ubuntu`) and repeat the runner. Plain words are
on the caller (`Run package work`), the infrastructure jobs, the steps, and the
accepted-gap checks. `docs/topics/ci-cd.md`, "Reading job and check names",
explains the suffix values to a reader.

### Test outcomes (2026-11-10, macOS host)

| Check | Result |
| --- | --- |
| `python3 scripts/ci/test_runner_loss.py` | 72 OK |
| `python3 scripts/ci/test_reuse_validation.py` | 52 OK |
| `python3 scripts/ci/test_publish_gaps.py` | 29 OK |
| `python3 scripts/ci/test_resolved_plan.py` | 103 OK |
| `python3 scripts/ci/test_schema.py` | 203 OK |
| `python3 scripts/ci/test_ci_local.py` | 104 run, 1 failure, which predates this feature (below) |
| `just test repo-deps test-toolkit` | 1045 passed, 5 skipped |
| `darkmatter` doc-corpus tests that read the edited docs | 2 passed |
| `workflow_structure.py` diff against the baseline | empty |

The one failure,
`NativeProvisioningTests.test_the_build_owner_installs_only_its_own_records_closure`,
comes from six `run: >-` steps in `ci.yml`'s `build` job, added by commit
`212070aba` (build-storage diagnostics). `scripts/ci/workflow_reading.py`
refuses folded scalars on purpose. This feature does not touch those steps.

### Preserved execution properties

- The structure diff is empty: commands, `with:` values (including artifact
  names), identifiers, guards, `needs`, matrices, permissions, runners, and
  timeouts all match the Phase 1 baseline.
- No job or step `name:` in the four workflows contains `${{`.
- `ci-gate` is still the identifier and the displayed name.
- No commit or uncommitted change of this feature touches
  `scripts/ci/schema.py` or `.github/ci/schemas/`, so `ROW_FIELDS`, the row
  field list, and the plan schema version are unchanged by it.
- The only display-name readers are `runner_loss.py` and
  `reuse_validation.py::AUDIT_JOB`; no third was added.

### Observed versus projected

Every name under "Observed names" was read from a production run before the
rename. Every name under "Display check" was read from run 42963148323. The
names under "Renamed names" have not appeared in a production run yet, because
the change has not been pushed; their rendering pattern is observed and each
line's basis is stated there. Phase 3 itself made no new API reads; the
display check was run afterwards for R1-F2. The first pull-request run after
the merge can confirm the exact strings.
