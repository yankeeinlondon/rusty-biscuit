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

The repository does not yet implement all of these semantics: the current
receipt is verified by recalculating scope in CI, and a failed `warn` run
publishes no validation receipt. The current `off` mode exits before calculating
scope; the target design makes it a deprecated `scope-only` alias. Do not
describe that contract as live behavior until the hook, evidence schema,
workflow, rollup, tests, and human documentation land together.

The current receipt's `base` is the branch's merge base. Verification normalizes
the event base with `git merge-base <base> <head>` before comparing identities,
because a PR target may have advanced. The independently computed scope must
still match exactly; normalization never extends a receipt to untested packages
or tiers. Recording continues to require an ancestor base.

## Intentional bypass modes

Prefer a repository-provided **scope-only** mode over `git push --no-verify`
when the goal is to skip local tests and let CI exercise every supported
environment. Scope-only still calculates and publishes exact-tree scope, but
publishes no validation outcomes and excludes no CI cells.

`git push --no-verify` prevents the pre-push hook from executing. It cannot
produce new local scope or validation evidence, so CI must calculate scope and
run every required cell. Reserve it for cases where the hook itself cannot run.

Mode intent is:

| Mode | Push after local failure | Scope evidence | Complete host outcomes | CI host cells |
|---|---:|---:|---:|---|
| `strict` | No | Yes after a successful run | Passing outcomes | Omit proven cells |
| `warn` | Yes | Yes | Pass or fail | Omit proven cells; roll up their outcomes |
| `scope-only` | No tests run | Yes | No | Run all |
| `--no-verify` | Yes; hook does not run | No new evidence | No new evidence | Run all |

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
