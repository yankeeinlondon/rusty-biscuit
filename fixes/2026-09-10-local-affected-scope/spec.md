---
status: absorbed
absorbed-by: fixes/2026-09-11-cicd-cleanup/spec.md
absorbed-on: 2026-09-12
created: 2026-09-10
area: repo
packages: []
depends-on:
  - fixes/2026-08-06-cicd/spec.md
  - fixes/2026-08-30-ci-preflight-local-parity/spec.md
---

# Local affected scope and host-result reuse

> **Absorbed 2026-09-12.** This specification was never implemented as its
> own fix. Its scope is owned by `fixes/2026-09-11-cicd-cleanup/spec.md`
> (Rulings, B0), which deliberately supersedes three of its decisions and
> audits the rest in `fixes/2026-09-11-cicd-cleanup/absorption-audit-2026-09-12.md`.
> The text below is unchanged and is read through that audit.

## Summary

The pre-push hook must make the local machine the normal producer of the
affected CI scope. That scope is deterministic and cheaper to calculate on an
already-warm development host than after allocating a GitHub runner. CI reuses
a matching exact-tree scope as its scheduling authority and calculates scope
itself only when local evidence is absent or does not match the event.

Local test results are a separate claim. A complete host run is reusable
whether it passes or fails. CI must omit the corresponding completed host
cells, run every remaining environment, and feed both passing and failing local
outcomes into the normal final verdict. A known local failure must not consume
another hosted runner merely to reproduce the same failure, and it must not
prevent the other operating systems from producing evidence.

The intentional no-test path is a new `scope-only` hook mode. It publishes the
scope but no test results, so CI runs the affected work on every supported
environment. `git push --no-verify` remains the emergency escape hatch: Git
does not invoke the hook, so that invocation cannot produce new evidence and CI
falls back safely.

## Problem

PR #75 established the first useful boundary:

- local and hosted runs use `scripts/ci/affected_scope.py`;
- source-changed packages receive lint, L1, and hostable L2 locally;
- unchanged direct reverse dependencies receive compile-check only;
- a clean successful local run publishes an OS-specific Git note; and
- CI recalculates scope and omits the matching environment when the note equals
  its independently derived expectation.

That implementation still wastes work in four ways.

First, CI always allocates a runner and repeats the deterministic scope
calculation, including repository checkout, toolchain setup, and Cargo metadata,
even when the exact scope was already calculated locally.

Second, local evidence is pass-only. In `warn` mode a complete failing host run
allows the branch push but publishes no reusable outcome, so CI allocates the
same host environment and rediscovers a failure already known locally.

Third, `off` exits before calculating scope. A developer who wants every OS to
run in CI must currently discard the cheap local scheduling result as well as
the expensive local tests.

Fourth, the receipt excludes an entire environment rather than recording
individual completed cells. That cannot distinguish an executed failure from
an interrupted run, an unavailable backend, or partial coverage. The safe
fallback is therefore coarser and more expensive than necessary.

## Existing scope policy remains unchanged

This fix changes where scheduling evidence is produced and how completed local
outcomes are reused. It does not widen the affected-package algorithm.

- A package owning changed source receives its configured full validation.
- An unchanged direct reverse dependency receives compile-check only.
- Dependencies of the changed package and transitive reverse dependencies are
  not selected.
- Documentation, manifests, lockfiles, Just recipes, workflow configuration,
  and other CI configuration select no package jobs. CI tooling retains its
  compact contract tests.
- `workflow_dispatch` remains the explicit full-workspace operation.

## Evidence model

Scope and validation are distinct versioned documents even if their transport
shares implementation code.

### Scope evidence

Scope evidence states what the canonical calculator selected for one immutable
comparison. Its identity is:

```text
{schema version, base commit, head commit, head tree, scope document}
```

The scope document is the complete canonical output consumed by the workflow,
including the package matrix, source packages, direct reverse dependencies,
resolved package policy, environment routing, flags, change class, and job
estimate. A short list of package names is not sufficient because CI must not
run Cargo metadata or reinterpret package policy merely to reconstruct the
matrix.

Scope evidence is independent of an operating system and belongs under one
dedicated Git-notes ref, separate from OS validation receipts. Canonical JSON
serialization and a closed schema make byte-for-byte fixtures stable.

### Validation evidence

Validation evidence states what actually completed on one detected environment
for that exact scope identity. Each reusable outcome is keyed at the same
granularity as CI and the rollup:

```text
{package, environment, gate or tier}
```

Each record carries at least:

- terminal result (`success` or `failure`);
- command exit code;
- duration;
- whether the command executed to completion;
- a bounded failure summary and failing test identities when Nextest provides
  them; and
- enough provenance for the report to label the producer as local evidence.

An executed test failure is complete evidence. Cancellation, interruption,
missing required tooling, an unavailable required backend, malformed staging,
or failure before the gate starts is incomplete evidence and cannot suppress
that CI cell.

Lint may be recorded for diagnostics, but it suppresses a hosted lint cell only
when the scope policy explicitly declares the local invocation equivalent to
that cell. A macOS Clippy run must not silently stand in for an Ubuntu-targeted
lint job. The same equivalence rule applies to checks and higher tiers.

Full JUnit payloads are not required in Git notes. The evidence format must be
bounded. The CI importer may project validated local outcomes into the existing
producer-status/JUnit staging contract or extend the rollup with a first-class
local producer, but it must not create a second verdict path.

## Requirements

### R1 — calculate committed scope before local tests

For an ordinary push of the current `HEAD`, the hook calculates scope from the
outgoing commit objects and their merge base with `origin/main` before running
local gates. It uses the committed `base..head` path set, not unstaged or
untracked files, for the publishable scope document.

The existing developer validation may continue to include staged, unstaged,
and untracked paths so local feedback covers work still present in the
worktree. That wider working scope is not evidence about the immutable outgoing
tree. Validation outcomes are publishable only when the executed tree and the
outgoing tree are demonstrably identical.

A dirty worktree therefore prevents validation reuse but does not prevent the
hook from publishing committed scope evidence.

### R2 — publish scope independently of validation

The hook publishes canonical scope evidence as soon as it has a valid outgoing
head, base, and scope document. Publication does not depend on tests passing,
on a host environment being recognized, or on a validation mode that runs
tests.

Failure to publish evidence does not falsely block or shrink CI: the branch may
follow the selected hook mode, while CI detects the missing receipt and
calculates scope normally. Diagnostics distinguish calculation failure from
publication failure.

An explicit package/area override may affect local developer testing but must
not replace canonical committed scope. The canonical scope receipt may still
be published; results from the overridden selection are not reusable unless
they prove every cell the canonical scope requires.

### R3 — matching local scope is authoritative in CI

The scope job fetches the canonical repository's scope-notes ref and validates:

- supported evidence and scope schema versions;
- exact event head commit;
- exact head tree;
- exact event comparison base; and
- structural validity of the complete scope document.

When all checks pass, the job emits that document directly as the workflow
matrix and policy artifact. It does not run `affected_scope.py`, Cargo metadata,
or a second package-policy resolution.

When any check fails, CI calculates scope from the event's exact base and head
using the checked-out implementation, exactly as it does today. A base branch
advance intentionally invalidates older local scope. `workflow_dispatch`
ignores local scope and remains the explicit full run.

The Git-notes ref is only a cache/evidence transport. External forks and actors
without write access cannot populate the canonical ref and naturally take the
fallback path. The repository already executes contributor-controlled workflow
and scope code; this change does not claim Git notes are a new security
boundary.

### R4 — publish every complete host outcome

After local gates finish, the hook publishes validation evidence for every
completed reusable cell, including failures. It must not reduce the document to
one environment-level Boolean.

Publishing occurs before a failing `strict` invocation returns nonzero. This is
useful when the developer subsequently makes an intentional `--no-verify` push
of the same unchanged commit: the bypass creates no evidence, but the earlier
completed hook run already did. A first invocation using `--no-verify` has no
new receipt and takes the normal CI fallback.

Evidence is attached to the exact outgoing tree and base. A new commit, amended
commit, changed base, dirty test run, interrupted hook, or incomplete required
cell cannot inherit an earlier result accidentally.

### R5 — define the hook modes around evidence

`RUSTY_BISCUIT_PRE_PUSH` has these supported values:

| Mode | Local behavior | Push behavior | Published evidence |
|---|---|---|---|
| `strict` | Calculate scope; run required local gates | Block on any local failure | Scope plus every complete pass/fail outcome |
| `warn` | Calculate scope; run required local gates | Always allow after the run | Scope plus every complete pass/fail outcome |
| `scope-only` | Calculate scope; run no local gates | Allow | Scope only |

`strict` remains the default.

The existing `off` spelling becomes a deprecated alias for `scope-only` and
prints a migration message. It must not retain its current scope-discarding
behavior. This preserves existing developer configuration without maintaining
a second semantic mode.

`git push --no-verify` remains outside the mode table because Git does not
execute the hook. Documentation recommends `scope-only` when the hook can run
and reserves `--no-verify` for a broken or unavailable hook.

### R6 — exclude completed cells, not an entire environment blindly

CI validates every available OS receipt matching the chosen scope identity and
removes only cells carrying complete terminal outcomes. A single incomplete
cell remains scheduled even when other cells from the same environment are
reused.

This permits more than one trusted host receipt for the same tree without
special-casing one preferred environment. Duplicate receipts for the same cell
must agree; conflicting evidence is invalid for that cell and schedules it in
CI.

Local evidence may cover only an equivalent environment and invocation. It
never substitutes macOS for Linux, WSL2 for native Windows, or ordinary L1/L2
for browser, Level 3, real-resource, or companion work that did not execute.

### R7 — local failures join the normal verdict without stopping other OSes

A locally failed completed cell is visible in the GitHub run and makes the
final `ci-verdict` fail. It must not be modeled as a failed prerequisite that
causes remaining package jobs to skip. Linux, native Windows, macOS, and WSL2
cells not covered by local evidence continue independently, with the final
verdict waiting for them.

The report identifies each reused cell as local evidence, shows pass or fail,
names its package/environment/tier, and includes the bounded diagnostic carried
by the receipt. It must not claim that a hosted runner executed that cell.

The rollup remains the single blocking authority. Local result import produces
the same cell semantics as hosted producer status: PASS, FAIL, incomplete, and
missing evidence cannot be conflated.

### R8 — preserve fail-safe behavior

All evidence errors add work rather than remove it. These cases calculate scope
or schedule the affected cell normally:

- no note exists;
- the note ref cannot be fetched;
- JSON or schema validation fails;
- base, head, tree, or schema identity differs;
- a scope document is incomplete;
- a validation receipt conflicts with scope;
- a cell is interrupted or only partially executed;
- required failure detail exceeds the bounded format and cannot be represented
  safely; or
- the workflow cannot import local results into the rollup.

No catch-all error handler may convert an unverifiable receipt into an
environment exclusion.

### R9 — keep the bootstrap cheap and observable

GitHub still needs a small bootstrap job to check out the event, fetch notes,
validate identity, and emit a dynamic matrix. The valid-receipt path must avoid
Rust toolchain installation, Cargo metadata, and affected-scope recalculation
unless one of those is demonstrably required merely to validate the receipt.

The job summary reports:

- `local` or `CI fallback` as the scope source;
- the fallback reason without exposing secrets;
- matching validation environments;
- reused pass/fail cells;
- cells retained because evidence was incomplete; and
- the resulting hosted job estimate.

### R10 — update contracts and active documentation together

Implementation updates all current authorities in the same change:

- `.githooks/pre-push` and `.githooks/tests/test-pre-push.sh`;
- `just/ci-local.just` and any structured local-result staging helper;
- `scripts/ci/local_evidence.py` and its unit tests;
- `scripts/ci/affected_scope.py` only if canonical serialization or matrix
  filtering requires it;
- `.github/workflows/ci.yml`, workflow contract tests, and `ci-rollup`;
- `.github/ci/README.md`, `docs/topics/ci-cd.md`, the root README, and
  `docs/testing-strategy.md`; and
- the `rust-devops` and `os` skills.

Historical completed specifications remain historical and are not rewritten.

## Acceptance criteria

1. A normal clean `strict` push calculates affected scope once locally,
   publishes it, and CI uses the identical canonical scope without invoking
   the calculator or Cargo metadata.
2. Source changes select only their owning packages for full validation and
   direct reverse dependencies for compile-check; infrastructure-only changes
   do not acquire package jobs.
3. A missing, stale, malformed, inaccessible, or base-mismatched scope receipt
   makes CI calculate the correct scope and never reduces scheduled work.
4. A dirty worktree may publish committed scope but publishes no validation
   claim for the outgoing tree.
5. A passing `strict` or `warn` run omits only the completed equivalent host
   cells and contributes visible passing outcomes to the final rollup.
6. A complete failing `warn` run allows the push, omits the completed failing
   host cells, runs all remaining OS cells, and produces a failing final verdict
   naming the local failures.
7. A complete failing `strict` run publishes its scope and terminal outcomes
   before blocking the branch push. A later `--no-verify` push of the unchanged
   tree can reuse those receipts.
8. An interrupted or incomplete host run does not suppress the unfinished
   cells. Completed sibling cells may still be reused when independently
   represented and validated.
9. `scope-only` publishes canonical scope, runs no tests, excludes no OS cells,
   and allows the push.
10. `off` behaves as a deprecated alias for `scope-only` and prints a migration
    message; `strict` remains the default.
11. A first-time `git push --no-verify` publishes nothing. With no pre-existing
    exact-tree receipt, CI calculates scope and runs every affected cell.
12. A local failure does not short-circuit Linux, macOS, native Windows, or WSL2
    jobs still required by the scope.
13. Multiple matching OS receipts can be reused together; conflicting receipts
    fail safe at cell granularity.
14. `workflow_dispatch` ignores all local evidence and performs the explicit
    full-workspace run.
15. The CI summary identifies scope provenance, fallback reasons, reused local
    cells, retained incomplete cells, and the reduced hosted job estimate.
16. Hook tests, evidence tests, affected-scope tests, rollup tests, workflow
    contract tests, and `actionlint` cover pass, fail, incomplete, stale,
    conflicting, scope-only, and no-receipt paths.

## Verification plan

- Unit-test canonical scope receipt creation and validation with exact and
  mismatched base/head/tree/schema identities.
- Prove non-vacuously that the valid receipt path does not invoke Cargo or the
  affected-scope calculator, while every invalid path does.
- Exercise the hook with fake `just`, Git, evidence publisher, and OS discovery
  commands for all supported modes and exit outcomes.
- Exercise a complete failing `warn` run and assert that the branch push is
  allowed, the host cells are absent from the hosted matrix, other environments
  remain scheduled, and the final rollup is red from local evidence.
- Exercise interruption and missing-backend cases and assert the unproven cells
  remain scheduled.
- Exercise dirty state and explicit selection to prove neither can produce
  false exact-tree validation evidence.
- Exercise two matching environment receipts and one conflicting receipt to
  prove cell-level merging and fail-safe fallback.
- Run `python3 scripts/ci/test_affected_scope.py`,
  `python3 scripts/ci/test_local_evidence.py`, the pre-push hook suite, the
  `ci-rollup` Nextest suite, workflow contract tests, and
  `actionlint .github/workflows/ci.yml`.
- Use `just ci-local --dry-run` fixtures for source-only, reverse-dependency,
  documentation-only, workflow-only, and explicit full-scope cases.

## Out of scope

- Changing the affected-package algorithm or adding reverse-dependency tests.
- Treating dependencies or transitive reverse dependencies as affected.
- Replacing Git notes with an external evidence service.
- Cryptographic attestation beyond the repository's existing Git/GitHub access
  model.
- Reusing one operating system's results for another environment.
- Making browser, Level 3, real-resource, or unexecuted companion tests locally
  reusable.
- Changing PR-validation reuse after merge, release-plz, compiler caching, or
  package publication.
- Intercepting or redefining Git's `--no-verify` behavior.
