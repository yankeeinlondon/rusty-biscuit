# CI/CD and Releases in Rusty Biscuit

This page records the non-obvious decisions an agent needs before changing the
repository's CI, pre-push hook, or release automation. The live authorities are
`docs/topics/ci-cd.md`, `.github/ci/README.md`, package
`[package.metadata.ci]`, and `.github/ci/environments.json`.

## Affected scope

`scripts/ci/affected_scope.py` is the canonical deterministic calculator for
local and hosted runs. Its package policy is deliberately narrow:

- A package owning changed source receives lint, check, L1, and its declared
  higher tiers.
- An unchanged direct reverse dependency receives compile-check only. Neither
  ordinary dependencies nor transitive reverse dependencies are selected.
- Documentation, manifests, lockfiles, Just recipes, workflow configuration,
  and other CI configuration select no package jobs. CI tooling has compact
  contract tests of its own.
- `workflow_dispatch` is the explicit full-workspace path. Do not turn an
  infrastructure edit or uncertainty into an implicit full run.

The scope job should calculate once and fan out a canonical matrix and policy
document. Downstream jobs consume that output rather than rediscovering scope.

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
- A complete host run is reusable whether it passed or failed. CI omits only
  the host cells for which the receipt supplies terminal outcomes and feeds
  those outcomes into the normal rollup. A failed local outcome must make the
  final verdict fail without preventing the other OS jobs from running.
- An interrupted run, an unavailable required backend, dirty outgoing state,
  or an explicit package override is not complete exact-tree evidence. CI runs
  any cells that are not proven.
- Local evidence may suppress only equivalent L1/L2/check work for the detected
  environment. It never stands in for another OS, browser work, Level 3, or a
  companion suite it did not execute.

These semantics are live as of 2026-09-11. The receipt is version 2, keyed per
`{package, environment, gate}`; `strict` and `warn` both publish a complete run,
passing or failing; `off` is a deprecated alias of `scope-only`. The rollup half
is not: Phases 5 and 6 of `fixes/2026-09-11-cicd-cleanup/plan.md` still owe the
per-area result model that consumes a local-origin cell.

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
publishes no validation outcomes, and excludes no CI cells. It publishes no
standalone *scope* document either: the note ref carries a validation receipt,
and CI recalculates scope.

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
| `strict` | No | Yes after a successful run | Passing outcomes | Omit proven cells |
| `warn` | Yes | Yes | Pass or fail | Omit proven cells; roll up their outcomes |
| `scope-only` | No tests run | Plan resolved and printed | No | Run all |
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
