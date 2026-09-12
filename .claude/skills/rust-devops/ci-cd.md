# CI/CD and Releases in Rusty Biscuit

This page records the non-obvious decisions an agent needs before changing the
repository's CI, pre-push hook, or release automation. The live authorities are
`docs/topics/ci-cd.md`, `.github/ci/README.md`, package
`[package.metadata.ci]`, and `.github/ci/environments.json`.

## Affected scope

`scripts/ci/affected_scope.py` is the canonical deterministic calculator for
local and hosted runs. Its package policy is deliberately narrow:

- A package owning changed source receives lint, L1, and its declared higher
  tiers — and `check` only when it declares `example` or `bench` targets. The
  L1 build already compiles the `lib`, `bin`, and `test` kinds, so a separate
  compile job exists solely for the kinds no test gate produces: one check
  cell per native environment, running `cargo check -p <pkg>` with explicit
  `--examples`/`--benches` selectors (`check_args`), never `--all-targets`.
  `_wsl-ci.yml` takes `archive-args` (package and features only) so those
  selectors cannot reach the guest's archive build. Every cell
  records `target_kinds` and `compile_coverage_from`, and an archive-only
  environment names the runner that built its archive rather than claiming to
  have compiled anything.
- An unchanged direct reverse dependency is **reported by name** in the plan's
  `reverse_dependencies` and selected nowhere: no area, no job, no result cell.
  It used to receive a compile-check entry, which presented an untested area as
  a green top-level result (PR #76). Where — if anywhere — that seam gets
  compiled is Open Question 1 and is unruled; `dependent_seam` is the optional
  package field that would carry it. Neither ordinary dependencies nor
  transitive reverse dependencies are selected.
- Documentation, manifests, lockfiles, Just recipes, workflow configuration,
  and other CI configuration select no package jobs. CI tooling has compact
  contract tests of its own.
- `workflow_dispatch` is the explicit full-workspace path. Do not turn an
  infrastructure edit or uncertainty into an implicit full run.

The scope job calculates once and emits **one** canonical resolved plan:
selected areas with a reason each, the packages contributing to each, and one
`{package, environment, gate}` cell per unit of work carrying its execution
(`execute`/`reuse`), origin, state, evidence, and governance.
`schema.validate_resolved_plan` is its contract and
`.github/ci/schemas/contract.json` is the field list Rust tooling asserts
against. Downstream jobs consume that document rather than rediscovering scope.
Package remains the stored identity everywhere; **area is a derived grouping**,
computed from the manifest directory with the same rule as
`sniff repo package-area` and kept honest by a drift contract rather than by a
committed mapping file.

## Local scope and validation evidence

Parallel test workers default to `max(1, logical_cores - 2)` locally. CI uses
all logical cores on runners with four or fewer, otherwise `logical_cores - 2`.
The policy preserves capacity on developer and larger shared hosts without
crippling small CI runners. It controls test concurrency, not CPU affinity or a
guaranteed reservation. Shared-resource L2 stays serial; isolated suites opt in
through `l2-parallel-self-spawn`, with explicit `BISCUIT_L2_THREADS` overriding
the default.

`_test_threads` in `just/devops.just` detects CI using `CI=true`,
`GITHUB_ACTIONS=true`, or nonempty `BISCUIT_CI_ENVIRONMENT`; the `ci` Nextest
profile alone leaves the local budget in effect. L1, sanity, and real-resource
recipes preserve explicit `NEXTEST_TEST_THREADS` and otherwise export this
default. Direct local Nextest runs use `.config/nextest.toml`'s
`test-threads = -2`. Cargo build-job limits are unchanged. Existing CI-profile
groups still cap Claudine L1 at four, Claudine CLI L1 at one, and Sniff L1 on
Windows at one, even when the overall budget is larger. See the
[central policy](../../../docs/topics/ci-cd.md#layer-1--local-pre-push-hook).

Keep two claims distinct:

- **Scope evidence** identifies what the deterministic calculator selected for
  an exact `{base, head, tree, schema}` tuple.
- **Validation evidence** records the per-package, per-environment, per-tier
  outcomes of a complete local run against that scope.

When changing the current evidence implementation, preserve these agreed
semantics:

- A matching local scope receipt is authoritative. CI reuses it; a missing,
  stale, malformed, or mismatched receipt makes CI calculate scope itself.
  Live as of 2026-09-11: the hook publishes it on `refs/notes/ci-local/scope`
  in every mode, before any gate, from the committed `base..head` path set;
  `ci.yml` runs `local_evidence.py scope-verify` before the planner and
  reports `scope source` in its summary with the miss code on a fallback.
- A complete host run is reusable whether it passed or failed. CI omits only
  the host cells for which the receipt supplies terminal outcomes and feeds
  those outcomes into the normal rollup. A failed local outcome must make the
  final verdict fail without preventing the other OS jobs from running.
- An interrupted run, an unavailable required backend, dirty outgoing state,
  or an explicit package override is not complete exact-tree evidence. CI runs
  any cells that are not proven.
- Local evidence may suppress only the equivalent L1/L2/browser cells it
  actually measured. `lint` and `check` stage no JUnit report, so they can
  never come from a local receipt and are always CI-origin — do not expect a
  local-origin lint cell. A receipt is keyed by environment, so it never stands
  in for another OS, for Level 3, or for a companion suite it did not execute.

These semantics are live as of 2026-09-11. The receipt is version 2, keyed per
`{package, environment, gate}`; `strict` and `warn` both publish a complete run,
passing or failing; `off` is a deprecated alias of `scope-only`. A receipt's
`host.report_dir` is where the hook retained the run's JUnit reports —
`$BISCUIT_CI_EVIDENCE_DIR/<head sha>/<environment>/`, root default
`~/.rusty-biscuit/ci-evidence` — copied there before the receipt exists;
`record-cells` refuses an empty or non-retaining directory rather than
inventing one.

The rollup consumes that evidence. `ci-rollup rollup --plan` reads the resolved
execution plan, so a cell a receipt satisfied is reported as a completed
local-origin result with its counts, duration, and the notes ref behind it —
not as `MISSING`. Result documents are `schema_version: 3` and are refused
across generations; the baseline keeps its own version 2. `--area` on `rollup`
and `verdict` narrows a document to one area's slice (cells, scope, scheduled
set, and accepted evidence together), and `summarize` folds slices into a view
that applies no policy.

Two cases worth knowing before reading a result:

- **A cell the plan reused and CI also executed reports the execution.** The two
  documents can disagree; when they do, the result with a report behind it is
  the honest one and the disagreement is stated in the cell's reasons. A reused
  cell is therefore not a guarantee that no job ran for it.
- **`lint` and `check` cells have no JUnit walker behind them.** Their state
  comes from the producer status, which is why a lint job that never reported is
  `MISSING` rather than absent. Any scheduling change must keep uploading those
  statuses.

`ci.yml` fans out one caller identity per selected AREA (`area-ci`, over the
planner's `scheduled_areas`) into `_area-ci.yml`, which fans out the area's
packages into `_package-ci.yml`, which delegates the WSL2 cell to
`_wsl-ci.yml` — four levels including the caller, GitHub's maximum, with no
margin for another. Each area's own `rollup` job (`if: always()`, behind that
area's producers) runs `ci-rollup rollup --area` and `verdict --area` and
narrows `runner_loss.py attribute --package` to its own packages: a failure
blocks its own area and no other, and another area's baseline entry cannot
excuse it. Area is a grouping, not an identity: the only stored name carrying it
is the per-area slice `ci-results-<slug>`, with `/` spelled `--`, because
GitHub rejects `/` in an artifact name.

**An all-reused area must still fan out.** If a receipt covers every cell an
area owns and the area then dropped out of `scheduled_areas`, no
`ci-results-<slug>` slice would be written and that area's local-origin results
would be reported nowhere. The planner builds the matrix from each package's
*declared* gates rather than its executing cells, which is what gets this
right; nothing turns red when it breaks, so it is pinned by a fixture.

Two rules the presentation depends on, both cheap to break:

- A job that can be skipped as a whole carries **no `name:`**. GitHub does not
  evaluate the matrix context for a skipped job, so a `name:` holding
  `${{ matrix.… }}` reaches the Checks tab as raw expression text. Omitting it
  makes the label the job id when skipped and `job-id (matrix values)` when it
  runs. The package half of the identity comes from the caller, because a
  called workflow's jobs render as `<caller job name> / <called job name>`.
  That composite label is a **parsed contract**, not just presentation:
  `runner_loss.py` reads `area-ci (<area>) / <package> / <gate> (<env>)` from
  the tail to attribute a dead runner's cell. Phase 6's renaming broke all six
  producer labels at once and nothing turned red, because every fixture spelled
  the names by hand. `test_runner_loss.py` now derives them from the shipped
  workflows instead, and runs in `just ci-local`'s self-test loop.
- Advisory jobs carry `continue-on-error: true`. The merge gate the repository
  is moving to folds the run's conclusion, so an advisory job that could fail
  would become a merge blocker.

## The merge gate today

`ci.yml`'s `ci-verdict` job is **still the single required context** in ruleset
`protect-your-bacon` (19747338). It is transitional: it duplicates the area
rollups' judgement over the whole run, reading the same plan, policy,
environment table, artifact patterns, and baseline, so the two cannot reach
opposing verdicts while both exist. Which mechanism replaces it — a required
workflow, or a policy-free fold of the run conclusion — is Open Question 3 and
is unruled.

Removing the job is **not separable** from moving the required context: delete
it first and every PR waits on a check that never reports. The complete change
set, when the ruling lands, is the `ci-verdict` job in `ci.yml`, the
`NON_PRODUCER_JOBS` entry in `scripts/ci/runner_loss.py`, the advisory summary's
closing line, `just ci-diff`'s `gh run download -n ci-results`,
`.claudine/scripts/ci-watchdog.ts`'s `ci-verdict` job lookup, and the two
pending fixtures `no_standalone_global_verdict_job_remains` and
`the_verdict_consumers_are_rewired_when_the_job_goes` — which must be deleted
together. The last two consumers are not in the specification's checklist.

The run conclusion is already a faithful conjunction: exactly one job
(`ci.yml:summary`) carries `continue-on-error: true`, asserted as an exact set
over all four reader-facing workflows. `reuse_validation.py` and
`release-plz.yml` already key on a completed, successful `ci` run, which is the
signal the migration makes authoritative, so neither needs rewiring.

## Governed policy gaps

A tier a package owns tests for that an environment cannot host is governed
**once**, in `.github/ci/environments.json`, as a capability object carrying
`available: false` plus `reason`, `owner`, `expiry`, and optionally `closes` —
the tracked work that ends the gap. A plain `false` is an *ungoverned* absence.

- A governed, unexpired gap is a distinct machine-readable **`ACCEPTED GAP`**
  state: neither a pass nor a test failure, and it does not block. The rollup
  renders its owner, expiry, policy entry, `closes` link, and revocation
  instructions where a reader sees the cell.
- An absent, incomplete, or expired acceptance is a blocking `POLICY GAP`.
- The state is decided by the planner before the run and is **never inferred
  from a GitHub cancellation conclusion**. A real failure outranks it.
- Do not baseline a policy gap in `ci-baseline.toml`; a baselined entry is only
  accepted against a `FAIL`, so it would not work anyway.

A `gates = false` package owns no plan cells at all. Its governed
`NOT SCHEDULED` entries come from the resolved-package policy document, which is
the only place its owner, class, and expiry live — that document cannot be
deleted without moving the exclusion metadata into the plan first.

A receipt's `base` is the branch's merge base, and verification normalizes the
event base with `git merge-base <base> <head>` before comparing, because a PR
target may have advanced. Recording still requires an ancestor base.

A receipt from an **older head** is reusable only when the cell's gate-input
identity is unchanged — the `git ls-tree` entries of the tested package's build
closure (dev-dependencies included, and the lockfile) plus that gate's global
inputs. Verification recomputes that over both trees rather than trusting the
identity the receipt stored. `schema_version: 1` notes are exact-tree,
pass-only, whole-environment, never upgraded in place, and render their
measurements as `not recorded (v1 receipt)`.

A gate that staged no JUnit report is recorded `partial`: it has an exit code
and nothing to attribute it to, so it is published and refused for reuse rather
than credited as a tested cell. That is how a compile failure stays a CI job.

## Execution constraints before a push

An instruction such as "WSL was already run; do not run it again" also applies
when a push would automatically schedule WSL. A request to repush retains that
constraint. Check the final resolved matrix, including each package's `wsl`
flag, against every active constraint before triggering CI. Checking only that
macOS disappeared is insufficient when WSL must also be excluded.

The planner takes a verified **per-cell** result set (`--accepted-cells`). An
accepted cell has its execution omitted and stays a cell with local origin, so
the rollup expects a local-origin result for it instead of reporting `MISSING`;
suppressing the matrix entry while leaving the policy expecting a CI result was
the PR #76 seven-cell regression.

`local_evidence.py verify --cells` produces that set by reading **every**
environment's notes ref reachable from the outgoing head, so a macOS receipt
from this push and a prior WSL receipt combine in one run. Each refusal carries
a code from `schema.REJECTIONS` and is published with the plan.

`scripts/cross-check.sh` publishes a `wsl2-ubuntu` receipt only when its WSL leg
ran the outgoing head's exact tree on a clean remote worktree with no test
filter; every other run prints why it published nothing. It ships the
developer's local tree, uncommitted work included, so most of its runs test a
tree no head names.

**Record the restriction, do not remember it.** `BISCUIT_CI_CONSTRAINTS_DIR`
names a store of `{environment, gate?, reason, owner, expiry, repository?,
branch?}` records. `just ci-local --plan` and the pre-push hook enforce them;
CI never reads them, so a constraint can only stop a push. Where the store lives
by default is Open Question 2 and is unruled — `constraints.default_directory()`
is empty until it is.

If prior evidence cannot be reused or CI cannot express the requested
exclusions, resolve that limitation before pushing. Preserve the restriction
while explaining what is missing; do not silently substitute a new test run or
fabricate current-head evidence. These are execution constraints, distinct from
whether a package must support the environment.

## Intentional bypass modes

Prefer a repository-provided **scope-only** mode over `git push --no-verify`
when the goal is to skip local tests and let CI exercise every supported
environment. Scope-only resolves and prints the plan — so a recorded execution
constraint is still enforced and the run is still reviewable — but runs no gate,
publishes no validation outcomes, and excludes no CI cells. It does publish the
standalone *scope* receipt, so CI takes the committed scope from it on an exact
`{base, head, tree}` match and recalculates only on a miss.

`git push --no-verify` prevents the pre-push hook from executing and produces
no new evidence. It does not invalidate already-published matching receipts:
CI still verifies them and can omit their covered environment. If strict
validation was run separately on the exact clean outgoing head and its receipt
was published and verified, the branch transfer can use `--no-verify` without
repeating that validation. Otherwise, the absence of qualifying evidence leaves
the corresponding CI cells scheduled. The flag itself excludes no environment.

Mode intent is:

| Mode | Push after local failure | Scope evidence | Complete host outcomes | CI host cells |
|---|---:|---:|---:|---|
| `strict` | No | Yes, before the gates | Pass or fail | Omit proven cells |
| `warn` | Yes | Yes, before the gates | Pass or fail | Omit proven cells; roll up their outcomes |
| `scope-only` | No tests run | Yes; plan resolved and printed | No | Run all |
| `--no-verify` | Yes; hook does not run | No new evidence | No new evidence | Existing valid receipts still apply |

Do not implement a failing local-evidence job as an upstream dependency that
causes the remaining matrix to skip. Represent local outcomes through the same
status/rollup contract as hosted producers, or otherwise ensure every remaining
OS continues before the final verdict fails.

## Release contract

Release-plz is the sole version/tag/changelog authority:

1. A successful `ci` run on `main` triggers a non-canceling release-plz job for
   that exact validated commit. It opens or updates a draft release PR.
2. Merging a PR labeled `release` triggers publication. Ordinary pushes do not
   publish releases.
3. `publish = false` means no crate is published to crates.io. Git tags and
   GitHub releases are the current package release channel; the specialized
   integration workflow may attach its own cross-compiled release assets.

Do not introduce a second version authority, publish from a feature branch, or
silently turn on crates.io. A new registry, installer generator, signing path,
or binary matrix is a separate design decision whose credentials, target
coverage, checksums, and rollback behavior must be explicit.

## Verification boundaries

- Package tests use Nextest and canonical `just` recipes; load the
  `rust-testing` skill before changing their gates or tiers.
- OS evidence is environment-specific; load the `os` skill before changing
  platform matrices or claiming an environment cannot be exercised locally.
- CI and release workflow changes require their compact contract suites and
  `actionlint`; they do not justify running every package.
- Preserve the pinned toolchain. Required CI follows `rust-toolchain.toml`;
  floating stable and nightly belong only to their advisory workflows.
