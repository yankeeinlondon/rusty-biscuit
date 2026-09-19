# Code Review: CI Python Test Suites

**Date:** 2026-09-15
**Scope:** `scripts/ci/test_*.py` (13 files, 12,677 lines, 607 tests) plus the two
test-only support modules `plan_fixtures.py` (126) and `pending_contracts.py` (90).
**Out of scope:** the modules under test (`affected_scope.py`, `schema.py`,
`local_evidence.py`, and siblings — 8,189 lines), except where a test makes a claim
about them that had to be verified.

**Baseline measured for this review** (macOS, `just`/`jq`/`sniff`/`cargo` all present):

| Suite | Tests | Wall |
|---|---:|---:|
| `test_affected_scope.py` | 174 | 3.9s |
| `test_schema.py` | 69 | 0.06s |
| `test_evidence_reuse.py` | 66 | 43.1s |
| `test_resolved_plan.py` | 64 | 14.6s |
| `test_ci_local.py` | 62 | 62.3s |
| `test_runner_loss.py` | 43 | 0.06s |
| `test_constraints.py` | 39 | 0.74s |
| `test_local_evidence.py` | 20 | 10.6s |
| `test_publish_gaps.py` | 19 | 0.47s |
| `test_reuse_validation.py` | 15 | 0.06s |
| `test_cross_check.py` | 13 | 11.6s |
| `test_build_key.py` | 12 | 0.13s |
| `test_build_baseline_revision.py` | 11 | 5.8s |
| **Total** | **607** | **~2m 45s** |

All 607 pass, none skipped on this host.

---

## 0. Overall assessment

This is unusually good test code. Three properties are worth naming before the
criticism, because several recommendations below are explicitly *not* to trade them
away:

- **Real inputs over mocks.** `test_evidence_reuse` builds real Git repositories
  and attaches real notes; `test_resolved_plan` runs the shipped planner against the
  shipped workspace; `test_ci_local` executes the actual `just` recipe and the actual
  `ci.yml` step scripts; `test_cross_check` runs the real `cross-check.sh` with
  stubbed `ssh`. Fixtures therefore cannot drift from policy the way hand-written
  expectations do.
- **Non-vacuity checks.** Several suites assert that their own corpus still contains
  what they claim to close — `test_at_least_one_real_package_exercises_each_declarative_class`
  (`test_affected_scope.py:3620`), `test_check_args_carry_exactly_the_declared_uncovered_selectors`'s
  `shapes_seen` set (`test_resolved_plan.py:440`), and best of all
  `test_the_assertions_reject_a_fold_that_accepts_failure` (`test_ci_local.py:1961`),
  which re-runs the gate fold against a deliberately widened copy to prove the
  blocking assertions are load-bearing. This is a technique worth spreading, not
  removing.
- **Comments that explain *why*.** The comment density is high but earns its keep:
  most comments record a defect that motivated the fixture (PR #76's seven
  MISSING cells, the nine-comment-line justfile edit that scheduled 72 packages,
  the PowerShell output-stream bug measured on `build-win-native`).

The problems are concentrated in four places: **two suites nothing runs**, **silent
skips**, **a hand-copied plan fixture that exists six times**, and **a handful of
150-line functions in `test_ci_local.py` and `test_cross_check.py`**.

Findings are grouped by kind. Severity: **[H]** act before the next CI change,
**[M]** worth a dedicated pass, **[L]** cleanup.

---

## 1. Completeness gaps

### 1.1 Two suites (23 tests) are executed by nothing — **[H]**

Mapping each suite to every place that invokes it:

| Suite | Hosted CI | `just ci-local` / hook |
|---|---|---|
| `test_schema`, `test_affected_scope`, `test_resolved_plan`, `test_ci_local`, `test_constraints`, `test_publish_gaps` | `ci.yml` | `just/ci-local.just:449`, `.githooks/tests/test-pre-push.sh:2099` |
| `test_runner_loss` | `ci.yml:501` | `just/ci-local.just:449` |
| `test_reuse_validation`, `test_cross_check`, `test_local_evidence`, `test_evidence_reuse` | `ci.yml` | — |
| **`test_build_key`** | **none** | **none** |
| **`test_build_baseline_revision`** | **none** | **none** |

`test_build_key.py` guards what its own docstring calls the repo's single hashing
boundary: *"there is no second digest. A planner that quietly fell back to `hashlib`
when `ci-build` was missing would still emit a plan, and every downstream comparison
against a producer's realized manifest would silently stop meaning anything."* That
is precisely the failure the `2026-09-12-single-os-compile` contract rests on, and
nothing runs the suite that defends it. It also holds the only pinned XXH64 vectors
in the repository (`test_build_key.py:94-97`) — a swapped hashing implementation
would re-key every build in the repo and no gate would notice.

`test_build_baseline_revision.py` guards AC8's measurement baseline. Lower stakes
(the comparison is a one-time acceptance decision), but the same structural problem:
`PublishedRevision` exists to detect drift between the constructor and the id
`baseline-2026-09-12.md` publishes, and it can only detect it if someone runs it.

**Recommendation.** Add both to `ci.yml`'s `ci-tooling` job. `test_build_key` also
belongs in the `just ci-local` self-test loop (`just/ci-local.just:449`) — it is
0.13s warm and the planner will not produce a plan without it.

**Caveat, and the reason this is not a one-line fix:** `build_key.helper_command()`
falls back to `cargo run --manifest-path scripts/Cargo.toml` when no `ci-build`
binary exists (`build_key.py:65-78`), so on a cold runner `test_build_key.py`
compiles a Rust crate inside what presents as a 0.13s unit suite. See **Spike 3**.

### 1.2 Tool-gated suites skip silently, and in CI — **[H]**

There are 21 skip guards across six files: 6 platform guards (`win32`/`nt`, which are
legitimate), 2 history guards (`BASE_AVAILABLE`), and **13 that gate on a host tool**.
Exactly one of the 13 refuses to skip under CI:

```python
# test_ci_local.py:1267 — the only guard that does this
if os.environ.get("CI"):
    raise AssertionError(message)
raise unittest.SkipTest(message)
```

Its comment states the principle correctly: *"a skip there would be a green cell
that verified nothing."* Every other guard violates it. The most consequential:

- **AC15 is unenforced in CI.** `test_every_workspace_members_area_matches_sniff`
  (`test_resolved_plan.py:300`), `test_the_planners_area_matches_sniff_for_every_layout_the_repo_uses`
  (`:273`), and `test_the_area_universe_is_a_subset_of_sniffs` (`:335`) are all
  `@unittest.skipUnless(shutil.which("sniff"), ...)`. `ci-tooling` runs on
  `ubuntu-latest` and installs only `nextest` and `just`, so **sniff is never
  present and all three always skip**. `just/ci-local.just:444-447` acknowledges
  this ("only runs where sniff is installed — a developer host, not a hosted
  runner") but the consequence is that the planner's area mapping — which CLAUDE.md
  names as a load-bearing invariant — is verified only when a developer happens to
  have `sniff` on PATH.
- **`ArchiveInventoryClosureTests` turns a planner regression into a skip.**
  `test_affected_scope.py:3571`: `if result.returncode != 0: raise unittest.SkipTest(...)`.
  A planner that cannot resolve a full-scope plan is the loudest possible failure,
  and this converts it to green.
- **`DependentSeamFixtureTests`** (`test_affected_scope.py:2604`) skips without
  `cargo`. **Five classes** — `CiLocalTests`, `ThreadPolicyTests`,
  `L1ThreadForwardingTests`, `PlanSurfaceTests`, `PlanFedRunTests` — skip without
  `just`+`jq`, and `NativeProvisioningTests` without `bash`+`jq`; that is ~40 tests
  behind tool guards in `test_ci_local.py` alone.

**Recommendation.** Introduce one shared guard and use it everywhere:

```python
def require_tool(name: str, reason: str) -> None:
    """Skip on a developer host, fail under CI, when a required tool is absent."""
```

Then either install `sniff` in `ci-tooling` or move the sniff-drift contracts to a
job that does. A contract whose enforcement depends on an unmanaged developer
PATH is not enforced.

### 1.3 No Python linter exists — **[M]**

There is no `ruff`, `flake8`, `pyproject.toml`, or `setup.cfg` anywhere in the repo,
and no Python lint step in any workflow or justfile. Yet 8 of the 13 files carry
`# noqa: E402` markers, so a linter was clearly intended at some point.

The immediate cost is small but measurable — unused imports survive:

- `test_affected_scope.py:17-57` imports `package_area`, `parse_lockfile`, and
  `derive_build_records`; none is used.
- `test_resolved_plan.py:12` imports `copy`; unused.

The real cost is that none of the findings in §4 below would have survived a lint
gate, and the 20,866-line Python surface has no equivalent of the Rust side's
`just lint` with `-D warnings`.

**Recommendation.** Add `ruff check scripts/ci` to the `ci-tooling` job and to the
package-area `just lint` convention. Start with `E`, `F`, `I` and expand.

### 1.4 `pending_contracts.py` is dead, and mis-specified — **[L]**

`@pending` is used by nothing except its own harness tests in
`test_schema.py:896-931`. Every contract it was written to hold landed in Phase 4
(`test_evidence_reuse.py:5-6`: *"none of them is pending any more"*). 90 lines of
module plus 3 tests are kept alive solely by each other.

It is also narrower than its docstring claims. The docstring promises three
outcomes, the second being *"fails, message does not contain it → fail, the
fixture's setup broke."* But the implementation only catches `AssertionError`
(`pending_contracts.py:69`). A pending fixture for a not-yet-existing field — the
single most common shape — raises `KeyError` or `AttributeError`, which propagates
as a bare error rather than the promised diagnostic. The `target()` helper at
`test_evidence_reuse.py:38` exists precisely to convert one such case
(`AttributeError`) into a legible message, which is the same gap solved locally.

**Recommendation.** Either delete the module and its three harness tests, or keep
it and widen the `except` to `Exception` (with `ContractLanded` re-raised first).
Keeping is defensible — the pattern is genuinely good and will be wanted for the
next multi-phase fix — but not as-is, because the first user to reach for it will
hit the `KeyError` gap on their first fixture.

---

## 2. Correctness and hygiene defects

### 2.1 Six `mkdtemp` calls never clean up — **[M]**

`test_affected_scope.py` uses `TemporaryDirectory` correctly in 21 places and
`tempfile.mkdtemp()` with no cleanup in six:

| Line | Site | Directories leaked per run |
|---|---|---|
| 1036 | `test_a_missing_capability_is_rejected` | 1 |
| 1062 | `test_an_ungoverned_capability_expiry_fails` | 1 |
| **1229** | `BuildContractTests.write` | **~14** (called by every refusal test) |
| 1600 | `ShadowWorkspaceTests` | 1 |
| **2086** | `MatrixLimitTests` | 1, containing **257 package manifests** |
| 2996 | `CompanionRecipeCheckTests.justfile_root` | 3 |

Roughly 21 directories per run, forever. On a CI runner that is noise; on a
developer machine running `just ci-local` dozens of times a day it is not, and
`MatrixLimitTests` leaves a 257-entry tree each time.

**Recommendation.** Mechanical: replace each with
`self.addCleanup(tempfile.TemporaryDirectory().cleanup)` or the `with` form already
used everywhere else in the same file.

### 2.2 Four files cannot be imported by a test runner — **[M]**

Nine files begin with:

```python
sys.path.insert(0, str(Path(__file__).resolve().parent))
```

Four do not: `test_affected_scope.py`, `test_local_evidence.py`,
`test_reuse_validation.py`, `test_runner_loss.py`. They import `affected_scope`,
`schema`, `local_evidence`, `runner_loss` as top-level modules, which resolves only
because `python3 scripts/ci/test_x.py` puts the script's own directory on
`sys.path`. There is no `__init__.py` and no `conftest.py`.

Consequence: `python3 -m unittest discover scripts/ci`, `pytest scripts/ci`, and any
IDE test runner all fail on those four with `ModuleNotFoundError`. Every invocation
site in the repo spells the full filename, which is why this has never been noticed.

**Recommendation.** Add a `scripts/ci/conftest.py` (or the same `sys.path.insert`
prologue) so the directory is importable uniformly, and consider a `just ci-tests`
recipe that discovers rather than enumerating filenames — which would also have
caught §1.1.

### 2.3 `report()` declares a list default it never accepts — **[L]**

```python
# test_evidence_reuse.py:127
def report(self, directory: Path, record: dict, *, failures: list[str] = ()) -> None:
```

The annotation says `list[str]`; the default is a tuple. Harmless today (only
iterated and `len()`-ed), but it is a type lie in a file that otherwise annotates
carefully.

### 2.4 `CrossCheckHarness` asserts, despite saying it does not — **[L]**

```python
# test_cross_check.py:182
"""Runs the real script with stubbed `ssh`/`scp`; owns no assertions."""
```

`replay_prelude` at `:313` calls
`self.assertIn(marker, remote, "the prelude must end by checking out one revision")`.
Per CLAUDE.md's drift rule the code is right: that assertion is a genuine
precondition of the replay and belongs there. **Fix the docstring**, e.g. *"Runs the
real script with stubbed `ssh`/`scp`. Asserts only the preconditions its own
machinery needs; the contracts live in the subclasses."*

### 2.5 A stale docstring describes a bridge that became the interface — **[L]**

```python
# test_affected_scope.py:96-101
"""The legacy `scope.json` shape the workflow still reads.
...the cases below that assert the package matrix and the rollup policy list go
through this projection until Phases 5 and 6 move their consumers onto cells."""
```

`fixes/2026-09-11-cicd-cleanup` is now in `fixes/_completed/`; Phases 5 and 6 are
done. `legacy_scope_document` was not removed — it is the permanent projection
`ci.yml` and `just/ci-local.just` read (`affected_scope.py:3263`). The comment tells
a future reader the projection is temporary when it is not.

Again the code is right and the comment is wrong. It is worth asking separately
whether the *name* `legacy_` is now misleading for a live interface — but that is a
production-code question, not a test one.

---

## 3. DRY

### 3.1 The resolved plan is hand-written six times — **[H]**

The single largest duplication in the suite. A complete resolved-plan document —
`schema_version`, `base`, `head`, `change_class`, `full_scope`, `full_scope_gates`,
`areas`, `packages`, `source_packages`, `reverse_dependencies`, `environments`,
`cells`, `builds`, `accepted_evidence`, `policy_gaps`, `prohibited_cells`,
`job_estimate`, `preflight_os`, `preflight_reason`, `flags` — is spelled out
literally in:

| File | Function | Lines |
|---|---|---|
| `test_schema.py:24` | `plan()` | 64 |
| `test_evidence_reuse.py:153` | `EvidenceFixture.plan()` | 72 |
| `test_ci_local.py:404` | `PlanSurfaceTests.resolved_plan()` | 131 |
| `test_ci_local.py:755` | `PlanFedRunTests.fed_plan()` | 101 |
| `test_local_evidence.py:149` | `ScopeReceiptTests.setUp()` | 115 |
| `test_cross_check.py:90` | `planner_stub()` | 89 |

The `environments` sub-block is byte-identical in five of the six:

```python
"environments": [
    {
        "name": name,
        "runner": "windows-latest" if name == "wsl2-ubuntu" else name,
        "native_key": "ubuntu-latest" if name == "wsl2-ubuntu" else name,
        "capabilities": {
            "tmux": name in ("ubuntu-latest", "macos-latest"),
            "headless_browser": name == "ubuntu-latest",
            "node_pnpm": name == "ubuntu-latest",
            "archive_only": name == "wsl2-ubuntu",
        },
    }
    for name in schema.ENVIRONMENTS
],
```

`test_affected_scope.py:3453`'s `environments_for_tests()` is a seventh variant —
richer (it carries `build` contracts) and deliberately separate, so it is not part of
this consolidation.

This is the exact problem `plan_fixtures.py` was created to solve. Its docstring
(`plan_fixtures.py:3-8`) reads: *"`test_evidence_reuse`, `test_ci_local`, and the
`just ci-local` stub planner all assemble a resolved plan literally... hand-writing
them in three places would guarantee they drift from the cells they claim to
serve."* The module then extracts only `builds`. The observation applies with equal
force to the other nineteen fields; the extraction stopped one field short.

**Cost of the status quo, measured:** schema version 3 added `builds`. Five of the
six fixtures needed a mechanical edit; `plan_fixtures.attach_builds` absorbed it for
three. Version 4 will require six edits again. Note `test_evidence_reuse.py:216` and
`test_ci_local.py:528/848` all write `"builds": []` before calling `attach_builds`,
a dead placeholder that exists only because the split is mid-way.

**Recommendation.** Extend `plan_fixtures.py` into the single fixture constructor:

```python
def plan(**overrides) -> dict:
    """A valid resolved plan, with every field at a neutral default."""

def cell(**overrides) -> dict:
    """One result cell, defaulted to an executing L1 on ubuntu-latest."""

def environments() -> list[dict]:
    """The four-environment table, capability-accurate, contract-free."""
```

`attach_builds` then becomes an internal step of `plan()` rather than something
every caller must remember. Each of the six sites collapses to a handful of
`overrides`, and the diff for schema version 4 becomes one file.

**Ergonomic cost:** low, and it *improves* legibility — today
`PlanSurfaceTests.resolved_plan` is 131 lines in which perhaps 20 carry the test's
actual intent, and the reader has to diff it mentally against the other five to find
them.

### 3.2 Two independent build-record derivations in test code — **[M]**

`test_schema.py:167` `builds_for(cells)` and `plan_fixtures.py:67` `attach_builds(plan)`
compute the same thing — the build records a cell set demands — by different rules,
with different key generators (`test_schema.BUILD_KEY = "0123456789abcdef"`, a single
constant, versus `plan_fixtures._key`, a function of `{package, producer}`). Both
claim in their docstrings to mirror `affected_scope`'s derivation. Two mirrors of one
production rule will not stay identical.

Fold `builds_for` into `plan_fixtures` as part of §3.1.

### 3.3 Four hand-rolled YAML readers — **[M]**

The stdlib has no YAML parser, so the suites read workflow files by indentation:

| Location | Reads |
|---|---|
| `test_ci_local.py:1020` `workflow_step_script` | one step's `run: \|` body |
| `test_ci_local.py:1057` `workflow_job_run_steps` | every `run:` step of a job |
| `test_runner_loss.py:414` `jobs_in` + `:440` `job_label` + `:453` `status_artifact` | job blocks, labels, artifact names |
| `test_build_baseline_revision.py:238` `_run_body` | one step's `run:` scalar |

Each hardcodes the layout independently — `workflow_step_script` assumes
six/eight/ten-space indents, `workflow_job_run_steps` assumes two/six/eight/ten and
strips six then four, `jobs_in` regexes `^  ([A-Za-z0-9_-]+):$`, `_run_body` assumes
eight/ten. `workflow_job_run_steps` is honest about it (`:1061-1064`), which does not
make four copies safer.

They are also fragile in a specific way: a re-indent, a flow-style mapping, an
anchor, or a `run: >-` folded scalar silently changes what they extract. Some fail
loudly (`workflow_step_script` raises when it walks past the step), others do not —
`workflow_job_run_steps` would simply return fewer steps, and
`WorkflowScopeStepTests` would then execute a shorter job and still pass.

**Recommendation.** One shared `scripts/ci/workflow_reading.py` used by all four
sites, with an explicit fail-loud contract. Whether to keep the indentation
heuristic or vendor a parser is **Spike 1**.

### 3.4 `test_ci_local.py` builds the same temp root three times — **[M]**

`run_recipe` (`:79`, 125 lines), `run_plan` (`:571`, 72), and `run_fed` (`:857`, 86)
each independently: make `bin/` and `scripts/ci/`, `shutil.copyfile(RECIPE, ...)`,
write `policy.just` from `thread_policy_recipe()`, write the same
`'red := ""\ngreen := ""\nreset := ""\nimport "ci-local.just"\n'` justfile,
`shutil.copyfile(CONSTRAINTS, ...)`, write the seven self-test stub files, write
Python stub executables with `chmod(0o755)`, call `clean_policy_environment()` and
`relocate_home()`, and pop the same list of `BISCUIT_*` variables.

**Recommendation.** One `CiLocalSandbox` context manager owning the scaffolding;
the three functions keep only the parts that differ (which stubs, which flags, what
to capture). This is also the prerequisite for §4.1.

### 3.5 `test_affected_scope.py` repeats its `setUp` fourteen times — **[M]**

Fourteen `setUp` methods, all containing `seed_build_inputs(self.root)`; 21 calls to
`package_ci_policy`, 16 of them with the identical
`runner_labels={"ubuntu-latest", "windows-latest", "macos-latest"}` literal. The
bodies differ only in the package list and the `resolve.nodes` graph.

**Recommendation.** A `SyntheticWorkspace` base class taking a package/edge
description:

```python
class SyntheticWorkspace(unittest.TestCase):
    """A temporary root with `cargo metadata` and CI policy for a declared graph."""
    PACKAGES: dict[str, str] = {}          # name -> relative manifest
    EDGES: dict[str, list[str]] = {}       # name -> direct dependencies
```

Fourteen `setUp` bodies become fourteen pairs of class attributes, and the
`workspace_packages_from` / `ci_policy` plumbing lives once.

### 3.6 The self-test suite list is hardcoded in four places — **[L]**

`just/ci-local.just:449`, `.githooks/tests/test-pre-push.sh:2099`,
`test_ci_local.py:123`, and `test_ci_local.py:877` each spell out
`test_schema.py test_affected_scope.py test_resolved_plan.py test_ci_local.py
test_constraints.py test_publish_gaps.py test_runner_loss.py`.

This one fails loudly rather than silently (a suite added to the recipe but not to
the stub lists makes the recipe call a missing file, failing the test), so it is
low severity — but it is four edits for one change, and it is adjacent to §1.1: had
the list been derived from a manifest, the two orphaned suites would have been
visible.

---

## 4. Clarity and self-documentation

The user's stated bar: *functions should carry a high-level comment of their utility
without dipping into specifics that will drift.* The suites mostly clear it. The
exceptions cluster in the largest functions.

### 4.1 Five functions are too large to read — **[M]**

| Lines | Location | Function |
|---:|---|---|
| 142 | `test_ci_local.py:1276` | `WorkflowScopeStepTests.run_step` |
| 131 | `test_ci_local.py:404` | `PlanSurfaceTests.resolved_plan` |
| 125 | `test_ci_local.py:79` | `CiLocalTests.run_recipe` |
| 118 | `test_cross_check.py:186` | `CrossCheckHarness.ship` |
| 115 | `test_local_evidence.py:149` | `ScopeReceiptTests.setUp` |
| 101 | `test_ci_local.py:755` | `PlanFedRunTests.fed_plan` |
| 89 | `test_cross_check.py:90` | `planner_stub` |

`resolved_plan`, `fed_plan`, `planner_stub`, and `ScopeReceiptTests.setUp` are
resolved by §3.1 — they are large because they are inlined fixture documents.

The other three are genuinely doing too much. `run_step` in particular has a
19-line docstring describing seven keyword arguments, then in one body: seeds a Git
repository, writes two Python trampolines with injected faults, copies
`rust-toolchain.toml`, writes a `rustup` counting stub, attaches scope notes,
publishes validation receipts, assembles eight environment variables with
event-shape conditionals, executes every job step in sequence honoring
`continue-on-error`, and finally validates the resulting plan. Its four boolean/
string fault parameters (`verifier_failure`, `overlay_failure`, `expect_failure`,
`validation_receipt`) are the tell — each is a branch the caller pushed down into
the helper.

**Recommendation.** Decompose along the seams the docstring already names:

```python
def seed_step_fixture(root) -> StepFixture:
    """A repository, trampolines, and stubs, ready for the scope job's steps."""

def arm_fault(fixture, *, verifier=None, overlay=False) -> None:
    """Make one evidence-processing tool fail, for the paths that must survive it."""

def run_job_steps(fixture, event, **event_shape) -> StepRun:
    """Execute the scope job's run steps in order, one shell each, as GitHub does."""
```

Note the docstring style: each says *what the thing is for*, none names a variable,
a step title, or an indentation depth. That is the target register.

### 4.2 A test named for one thing asserting seven — **[M]**

`test_workspace_excluded_zed_extension_selects_dmls_companion`
(`test_affected_scope.py:3280`, 47 lines, 17 assertions) checks: the scope selection;
`_package-ci.yml`'s `runner-tools` condition; that its `lint:` job mentions three
specific strings; the schema version, commit regex, sha256 regex, and URL
construction of `.github/ci/zed-extension.json`; four properties of
`darkmatter/justfile`'s `check-zed` recipe; and five required strings in
`rust-latest-stable.yml`.

Only the first sentence of the name is true. When it fails, the name tells the
reader nothing about which of the five files moved.

Same shape, smaller: `test_the_broken_consumer_fails_inside_the_changed_packages_own_check`
(13 assertions) and `test_each_unix_host_is_shipped_the_producer_consumer_sequence`
(13 assertions across 3 OSes).

**Recommendation.** Split into `test_the_zed_extension_selects_dmls`,
`test_the_packager_pin_is_well_formed`, `test_the_lint_job_verifies_the_pinned_packager`,
`test_the_check_zed_recipe_does_not_install_its_own_target`.

### 4.3 A hand-rolled soft-assert accumulator — **[M]**

`test_messenger_policy_and_matrix_contract_are_promoted` (`test_affected_scope.py:3193`,
78 lines) builds `missing: list[str]` through 14 `if ...: missing.append(...)`
statements, then ends with `self.assertEqual([], missing, f"... {', '.join(missing)}")`.

This reimplements `subTest` badly: no per-case isolation, the failure message is a
comma-joined string a human has to parse, and the 15-element `forwarding_contract`
list of booleans at `:3238-3262` collapses fourteen distinct workflow invariants into
one `if not all(...)` that names none of them on failure.

**Recommendation.** `with self.subTest(aspect=...)` per invariant.

### 4.4 Out-parameter dictionaries — **[L]**

Three helpers return data by mutating a caller-supplied dict:

```python
self.run_recipe(reports=reports)        # test_ci_local.py:242
result = self.run_plan(capture=captured) # test_ci_local.py:539
```

Both then read `reports["gate_backends"]` / `captured["plan"]`. The pattern also
appears as the `receipts: dict` + `remember()` closure idiom, repeated verbatim in
five tests (`test_ci_local.py:1567, 1776, 1854, 1868, 1882`).

**Recommendation.** Return a small dataclass or named tuple. `run_fed` at `:936`
already does exactly this (returns a dict of `calls`/`planner`/`stdout`/`written_plan`/`plan`)
and reads far better — make it the house style. The `remember` closure becomes
`fixture.scope_receipt_text` on the returned object.

### 4.5 Helpers that fight their callers — **[L]**

**`ci_policy`** (`test_affected_scope.py:129`) maps `_` to `-` in keyword names, so
any field whose real name contains an underscore is inexpressible. Callers work
around it inline, producing the mixed idiom that appears ~8 times:

```python
ci_policy(gates=False, reason="x", owner="@o",
          **{"exclusion-class": "promotion-pending", "expiry": "2027-01-31"})
```

The `**{...}` spread defeats the helper's only purpose. Take a plain dict.

**`plan_package`** (`test_affected_scope.py:3403`) silently rewrites a field the
caller did not ask about:

```python
if "check_args" not in overrides and "package" in overrides:
    record["check_args"] = f"-p {overrides['package']}"
```

A reader of a call site cannot tell what `check_args` will be without reading the
helper. Make it explicit, or derive it unconditionally.

### 4.6 Definition order — **[L]**

`test_affected_scope.py` calls `workspace_packages_from` at line 159 and defines it
at 3538; `environments_for_tests` is called from line 174 and defined at 3453;
`plan_package` from 934, defined at 3403. There is a `# --- helpers ---` banner at
3400, which is the right instinct applied 3,200 lines late. Worse, one test class
(`ArchiveInventoryClosureTests`, 3547) sits *below* that banner, so the file's own
organizing rule is already broken.

**Recommendation.** Move the helper block to the top of the file (after the imports),
which is where every other suite in the directory puts it.

---

## 5. Brittleness

### 5.1 309 substring assertions over prose — **[M]**

Across the 13 files there are 309 `assertIn`/`startswith` assertions, a large share
of them against human-readable error text produced by the module under test:

```python
self.assertTrue(any("has no consumer" in p for p in problems), problems)
self.assertTrue(any("not among its own compatible_environments" in p for p in problems), problems)
self.assertIn("also compiles 2 unchanged dependent(s)", cell["selection_reason"])
```

`test_schema.py` alone uses the `any(... in problem for problem in problems)` idiom
38 times.

This is a deliberate and largely sound trade — the messages *are* the product for a
human reading a failed plan validation, and the fixtures double as documentation of
what a refusal says. But it means every wording improvement is a test edit, and
**the tests cannot distinguish "the right rule fired" from "some rule fired whose
message happens to contain this substring."** `test_a_reused_cell_may_not_reference_a_build`
(`test_schema.py:598`) and `test_a_build_attached_to_lint_or_check_is_refused` (`:586`)
already assert the *same* substring for two different rules.

`schema.py` already has a rejection-code vocabulary (`schema.REJECTIONS`,
`schema.SCOPE_REJECTIONS`) and several fixtures assert against it correctly
(`test_evidence_reuse.py:375`, `test_local_evidence.py:373`).

**The codes are not missing — they are too coarse.** All 53 problem literals in
`schema.py` carry a code, but **45 of the 53 share one code, `malformed-receipt`**
(the rest: `unknown-package` ×3, `scope-malformed` ×2, `unknown-environment`,
`missing-receipt`, `conflicting-evidence`). `malformed-receipt` currently spans
everything from "area has no selection reason" to "build key has two owners" to
"counts.total disagrees with its parts". A test that wants to pin one of those 45
rules has no choice but to match on the prose tail, which is exactly why the two
build-reference tests collide.

**Recommendation.** See **Spike 2** — this is the one finding where I would measure
before changing.

### 5.2 A magic count — **[M]**

```python
# test_affected_scope.py:3259
package_ci.count("contains(fromJSON(inputs.gates), 'test')") == 4,
```

Buried inside the 15-element `forwarding_contract` boolean list, so when it breaks
the message is `messenger promotion contract is incomplete:
check/native-L1/WSL2 feature-argument forwarding` — which names neither the count
nor the file. Adding a legitimate fifth test-gated job breaks it, and the reader
has no way to know that from the failure.

**Recommendation.** Assert the set of *jobs* that carry the condition, not the
number of occurrences, and give it its own test.

### 5.3 Import-time subprocesses — **[M]**

```python
# test_ci_local.py:1180
STEP_BASH = resolve_step_bash()
# test_build_baseline_revision.py:74
BASE_AVAILABLE = _base_revision_is_available()
```

`resolve_step_bash` walks every `PATH` entry plus four well-known prefixes and runs
`<candidate> -c 'printf ...'` on each existing one, with a 10-second timeout apiece
(`bash_version`, `:1118`). On a host with several `bash` installations this is
several subprocesses before a single test is collected, and on a pathological host
(an NFS-mounted `PATH` entry, a hung binary) module import blocks for up to 10
seconds per candidate. `BASE_AVAILABLE` runs `git cat-file` at import.

Both also make the module un-importable for introspection (documentation tooling,
`--collect-only`) without side effects.

**Recommendation.** `functools.cache`d functions called from `setUpClass`, not
module-level constants.

### 5.4 A hardcoded 21-element dependency list — **[L], defend**

`test_sniff_change_selects_exact_direct_dependents` (`test_affected_scope.py:3328`)
pins sniff's exact direct dependents and, on mismatch, prints the computed list
paste-ready into the fixture. The comment calls it *"deliberate friction: this exact
list makes a human acknowledge a change to sniff's direct dependents."*

This will break on every dependency-edge change and that is the point. **Keep it.**
Noted here only so a future reader does not "fix" it into a `assertGreater(len(...), 0)`.
The paste-ready hint is a good pattern — worth copying to the other hand-maintained
corpus lists (`test_every_status_uploading_job_is_covered`,
`test_the_shipped_table_declares_one_contract_per_producer`).

### 5.5 `test_reuse_validation.py` mixes two mocking styles — **[L]**

`self.api` is constructed with a dispatching `side_effect` lambda (`:37`), but three
tests replace it with a positional list (`:91`, `:106`, `:121`). The list form makes
the test depend on call *order and count* rather than on what was asked:

```python
self.assertEqual(self.api.call_count, 2)   # :95
self.assertIn(f"head_sha={HEAD}", self.api.call_args_list[1].args[0])  # :54
```

Adding a harmless extra query (a retry, a pagination follow-up) breaks tests that are
not about call counts. Prefer the dispatching form throughout and assert on the
endpoints requested, not their index.

### 5.6 `test_build_key.py` can compile a Rust crate — **[L]**

`test_the_cli_and_the_module_agree_on_one_input` (`:132`) and the five `DigestTests`
call `build_key.helper_command()`, which falls back to
`cargo run --manifest-path scripts/Cargo.toml` when no `ci-build` binary is present.
On this host, warm, the suite is 0.13s; cold, it is a Rust build inside what
presents as a unit suite, with no timeout on the `subprocess.run` at `:139`.

Relevant to §1.1: this is likely *why* the suite was never wired into CI.

---

## 6. Proposed spikes

Four questions where I would measure rather than assert. Each is scoped to a day or
less and produces a decision, not a refactor. Each has a full plan beside this file:

| Spike | Plan | Decides |
|---|---|---|
| 1 | [`spike-1-workflow-yaml-reader.md`](spike-1-workflow-yaml-reader.md) | whether `scripts/ci` takes a PyYAML dependency |
| 2 | [`spike-2-rejection-code-granularity.md`](spike-2-rejection-code-granularity.md) | whether to add `PLAN_REJECTIONS` and subdivide `malformed-receipt` |
| 3 | [`spike-3-build-key-cold-cost.md`](spike-3-build-key-cold-cost.md) | how `test_build_key.py` is wired into CI |
| 4 | [`spike-4-sniff-drift-in-ci.md`](spike-4-sniff-drift-in-ci.md) | where AC15 is enforced |

The summaries below are the short form; the plans carry the steps, the results
tables, and the decision thresholds.

### Spike 1 — Should the workflow readers use a real YAML parser?

**Question.** §3.3 identifies four independent indentation-based readers. Should the
consolidated reader keep the heuristic, or should `scripts/ci` take a PyYAML
dependency?

**Why it is open.** The suites currently have *zero* third-party Python
dependencies, which is why `ci-tooling` needs no install step beyond `just` and
`nextest` and why the hook can run them on any developer machine. That is a real
asset. Against it: the heuristic readers cannot see anchors, flow mappings, or
folded scalars, and `workflow_job_run_steps` degrades silently rather than loudly.

**Method.**
1. Write `workflow_reading.py` with the current heuristic behind one API; port all
   four call sites; confirm 607 still pass.
2. Write a second implementation over PyYAML behind the same API.
3. Build a mutation corpus from the real workflows: re-indent a job, convert one
   `run: |` to `run: >-`, introduce a YAML anchor, convert one `with:` to flow
   style, reorder step keys. Run both implementations against each mutant.
4. Record, per mutant: heuristic reads the same / reads differently / raises;
   parser ditto.

**Decides.** Keep the heuristic if it raises on every mutant it cannot handle (loud
degradation is acceptable; silent is not). Take the dependency if it reads any
mutant differently without raising.

**Ships either way:** one reader instead of four, and the mutation corpus as a
permanent regression fixture.

### Spike 2 — Should `malformed-receipt` be subdivided?

**Question.** §5.1: 309 prose-substring assertions, ~100 of them against
`validate_resolved_plan` output. The codes already exist but 45 of 53 refusals share
`malformed-receipt`, so a test cannot name the rule it is pinning without matching
prose. Would a finer code vocabulary make the suite less brittle without making the
failures less useful to a human reading a CI log?

**Why it is open.** The messages are the product for a human, and `test_schema.py`'s
fixtures double as the specification of what each refusal says. A naive move to
codes would lose that. `schema.REJECTIONS` already proves the repo can have both —
the receipt validator emits `code: human text` and the tests assert on the code — so
the question is not whether but how finely, and whether the recipients of these
strings (`ci-rollup`, the area coverage audit, the hook) branch on the code today.

**Method.**
1. Inventory every distinct refusal `validate_resolved_plan` can emit, and every
   substring the suite asserts against it. Count collisions — cases where one
   substring is asserted for two different rules. One is already confirmed:
   `test_schema.py:594` and `:613` both assert
   `"only an executing L1, L2, or browser cell"` — once for the lint/check rule,
   once for the reused-cell rule — so neither test can tell which fired.
2. Prototype `code: text` for the build-record rules only (~20 refusals,
   `BuildRecordValidationTests`). Keep one test per rule asserting the exact human
   text, so the prose stays specified; move the other ~19 to code assertions.
3. Measure: lines changed, and whether any previously-passing test now fails
   (a collision that was masking a wrong rule firing).

**Decides.** If step 1 finds three or more collisions, roll codes out across the
whole plan validator. If it finds zero, the prose assertions are precise enough and
the cost is not justified — record that and stop.

### Spike 3 — What does `test_build_key.py` actually cost on a cold runner?

**Question.** §1.1 says the suite must be in CI; §5.6 says it may compile Rust.
Which, and how long?

**Method.** On a clean `ubuntu-latest` container with the pinned toolchain and no
`scripts/target`: time `python3 scripts/ci/test_build_key.py`. Repeat with
`ci-build` prebuilt (the `ci-tooling` job already runs `rustup show`, and other
steps may already have built `scripts/`).

**Decides.**
- Under ~30s cold → add to `ci-tooling` and to the `just ci-local` self-test loop
  unconditionally.
- Over that → add a `ci-build` build step to `ci-tooling` before it, or split the
  suite: `HelperResolutionTests` + `CanonicalizationTests` (no binary needed) run
  everywhere, `DigestTests` runs where the binary exists.

Either way, add a `timeout=` to the `subprocess.run` at `test_build_key.py:139`, the
only unbounded subprocess in the directory.

### Spike 4 — Can the sniff-drift contract run on a hosted runner?

**Question.** §1.2: AC15 is verified only where `sniff` happens to be installed,
which is never in CI.

**Why it is open.** The contract is deliberately "ask the other tool" rather than
"compare against a second copy of the rule" (`test_resolved_plan.py:58-64`), so
stubbing `sniff` would defeat it entirely. Installing it means building a workspace
member inside the CI-tooling job, and `test_every_workspace_members_area_matches_sniff`
spawns 73 `sniff` subprocesses through a `ThreadPoolExecutor` (measured here: 14.5s
wall, **85s system time**).

**A cheaper query exists, and it asks a different question.** Inverting the loop —
`sniff repo package-areas --json` once, then `sniff repo packages --package-area <A>
--json` per area — builds the whole package→area map in 33 calls instead of 73.
Measured here: **2.75s wall / 1.7s system, against 14.5s / 85s** for the current
`ThreadPoolExecutor` fan-out.

But the inverted map **loses `biscuit-test-harness` entirely**: it is absent from the
area universe, which is precisely the divergence `SNIFF_SELF_INCONSISTENT`
(`test_resolved_plan.py:55`) exists to document. The cheap query answers "which
packages does sniff list under this area", not "what area does sniff detect for this
directory" — and the second is the one AC15 makes authoritative. So the optimization
is not free; it narrows the contract.

**Method.**
1. Time `cargo build -p sniff-cli --release` on `ubuntu-latest` from a warm cache.
2. Decide whether the inverted query is an acceptable substitute, a useful *second*
   assertion alongside the per-directory one, or a semantic loss to reject.
3. Prototype `ci-tooling` with sniff installed; measure total job delta.

**Decides.**
- Delta under ~2 minutes → install sniff in `ci-tooling`, delete the `skipUnless`,
  and AC15 becomes a real gate.
- Otherwise → move the three sniff contracts to a scheduled/nightly job that builds
  sniff once, and make the skip in `ci-tooling` **explicit about where the contract
  is enforced instead** — a skip message naming the job that does enforce it is
  honest; a bare `"requires sniff"` is not.

---

## 7. Prioritized actions

**Before the next CI change**

1. Wire `test_build_key.py` and `test_build_baseline_revision.py` into `ci-tooling`
   (§1.1, gated on **Spike 3** for the former).
2. Add `require_tool()` — fail under `CI`, skip on a developer host — and apply it to
   all thirteen tool guards (§1.2).
3. Fix `ArchiveInventoryClosureTests.setUpClass` so a planner that cannot resolve a
   plan fails rather than skips (`test_affected_scope.py:3571`).

**Next dedicated pass**

4. Extend `plan_fixtures.py` to own the whole plan document; collapse six hand-written
   fixtures and fold in `test_schema.builds_for` (§3.1, §3.2).
5. Extract `CiLocalSandbox`; decompose `run_step`, `run_recipe`, `ship` (§3.4, §4.1).
6. Add `ruff` to `ci-tooling` and to `just lint`; clear the four unused imports (§1.3).
7. Replace the six leaking `mkdtemp` calls (§2.1); add `conftest.py` (§2.2).
8. Split `test_workspace_excluded_zed_extension_selects_dmls_companion` and replace
   the `missing`/`forwarding_contract` accumulators with `subTest` (§4.2, §4.3, §5.2).

**Cleanup**

9. `SyntheticWorkspace` base class for `test_affected_scope.py`'s fourteen `setUp`s (§3.5).
10. Move the helper block to the top of `test_affected_scope.py` (§4.6).
11. Fix the two drifted docstrings (§2.4, §2.5) and the `failures: list[str] = ()`
    annotation (§2.3).
12. Replace out-parameter dicts with returned records (§4.4); fix `ci_policy` and
    `plan_package` (§4.5).
13. Move `STEP_BASH` / `BASE_AVAILABLE` out of module scope (§5.3).
14. Decide `pending_contracts.py`: delete, or widen its `except` (§1.4).

**Explicitly do not change**

- The real-input fixture strategy. It is why these suites catch what they catch.
- The non-vacuity checks, especially `test_the_assertions_reject_a_fold_that_accepts_failure`.
- `test_sniff_change_selects_exact_direct_dependents`'s hardcoded list and its
  paste-ready failure hint (§5.4).
- The comment density. The comments record defects, not implementations.
