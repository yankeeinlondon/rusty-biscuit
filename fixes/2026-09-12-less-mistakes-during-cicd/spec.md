---
created: 2026-09-12
status: draft
implemented: false
area: repository-ci
depends-on:
  - fixes/2026-09-11-cicd-cleanup/spec.md
related:
  - fixes/2026-09-12-better-cicd-flow/spec.md
  - fixes/2026-09-12-single-os-compile/spec.md
---

# Preventing CI/CD Control-Plane Mistakes

## Status of This Draft

This specification records safeguards proposed after PR #76 exposed several
independent ways for CI configuration to disagree with itself. It describes the
target operating model; it does not claim that the safeguards are implemented.

## Problem

The repository has good individual mechanisms: affected-scope planning,
per-cell local evidence, GitHub Actions producers, Nextest profiles, coverage
audits, a fixed merge gate, and documented OS policies. The recurring failures
have occurred at the boundaries between them:

- version-1 local receipts could not prove the cells needed by a newer tree, so
  CI unexpectedly reran expensive macOS tests;
- producer jobs could appear green while a downstream rollup was the first
  place a test or evidence problem became authoritative;
- a same-head retry preserved only the cells rerun on that attempt and erased
  earlier qualifying evidence;
- `ci-gate` replaced `ci-verdict` in the workflow while the live repository
  ruleset continued waiting forever for `ci-verdict`;
- the OS-aware worker policy assigned public GitHub runners three or four
  workers, while a stale Nextest group silently forced all 2,500+ Claudine CLI
  L1 tests to run with one worker; and
- several of these problems were discoverable only after waiting for a large
  hosted matrix.

These are control-plane defects. Asking reviewers or agents to be more careful
does not solve them because the contradictory state is spread across source
files, generated plans, runtime artifacts, and GitHub repository settings.

## Objective

Make CI/CD mistakes fail quickly, locally, and with one actionable diagnosis.
The system should prove that its plan, workflows, evidence, concurrency policy,
repository settings, and final observed results agree before a pull request is
declared mergeable.

The safeguards below follow one rule: an important CI assumption must be an
executable contract at the boundary where it can drift. Documentation explains
the contract but is not the mechanism enforcing it.

## Safeguards

### 1. One scheduling authority

`scripts/ci/affected_scope.py` and its canonical resolved plan are the only
authority for required work. The plan inventories every
`{package, environment, gate}` cell and assigns exactly one disposition:

- `execute` — hosted CI must run the cell;
- `reuse` — qualifying passing evidence satisfies the cell;
- `accepted-gap` — a governed capability exception applies; or
- invalid — planning fails before a matrix is created.

Workflow matrices must be direct projections of `execute` cells. Coverage
audits, shell scripts, reusable workflows, and package metadata may not
independently recalculate which cells should exist. Area is a presentation
grouping and never a second result identity.

**Example.** A valid macOS receipt covers `claudine-cli / macos-latest / L1`.
The planner retains that cell with `execution: reuse`, while the hosted matrix
contains no row for it. Linux, Windows, and WSL2 cells without evidence remain
`execute`. A downstream reporter consumes the same retained cell rather than
assuming that every absent matrix row is missing evidence.

**Why this helps.** The old flow could remove a job in one projection while a
different consumer still expected its CI artifact. A single plan makes
omission explicit and reviewable: every required cell remains visible, and
there is only one place where evidence or policy may change its disposition.

### 2. Enforce cross-layer invariants as contract tests

Add executable contract tests for relationships that types or workflow syntax
cannot express. At minimum, they must prove:

1. Every planned `execute` cell maps to exactly one producer.
2. Every result-producing matrix row maps back to exactly one planned cell.
3. Every planned `reuse` cell names qualifying evidence and schedules no
   producer.
4. Every `accepted-gap` cell names valid governance and produces one neutral
   check.
5. A failed gate command makes its producer fail; only explicitly advisory
   jobs may use `continue-on-error`.
6. Every blocking top-level job is included in `ci-gate`, and every advisory
   job is excluded.
7. Every required check declared by repository policy is producible by the
   current workflows.
8. Package-wide Nextest groups do not override the OS-aware worker budget
   unless a documented shared-resource exception explicitly permits it.

Tests should derive names and matrix shapes from the shipped configuration.
Fixtures must not copy the expected names by hand, because the fixture and the
workflow can otherwise drift together without observing each other.

**Example.** The workflow renamed the merge job from `ci-verdict` to
`ci-gate`, but no local test compared produced check names with required check
names. A contract test with required context `ci-verdict` and produced context
`ci-gate` must fail with an error such as:

```text
required check has no producer: ci-verdict
produced blocking check is not required: ci-gate
```

**Example.** `.config/nextest.toml` defined
`claudine-cli-ci-l1 = { max-threads = 1 }` while `_test_threads` assigned all
four cores on a public Linux or Windows runner. The contract must reject the
package-wide cap unless it is represented as an approved resource-specific
exception.

**Why this helps.** These failures become deterministic seconds-long test
failures instead of ambiguous behavior discovered after a 30-minute matrix.
The diagnostic names both conflicting authorities, so the developer fixes the
boundary rather than investigating healthy tests.

### 3. Use staged migrations for required checks

A required check name is a compatibility contract with live GitHub settings.
It must never be renamed or removed in one atomic source change. Required-check
migrations use this sequence:

1. Add the new check while continuing to publish the old check as a
   compatibility alias.
2. Observe both checks completing successfully on a real pull request.
3. Change the live ruleset to require the new check.
4. Read the ruleset back and verify that the old check is no longer required
   and the new one is required.
5. Remove the compatibility alias in a later pull request.

The compatibility check must consume the same already-computed gate result; it
must not rerun tests or implement a second verdict policy.

**Example.** When introducing `ci-gate`, the workflow should temporarily have
published both `ci-gate` and `ci-verdict`, with `ci-verdict` mirroring the
completed gate. PR #76 would then have remained mergeable while the
`protect-your-bacon` ruleset was changed. Only after the ruleset read-back
showed `ci-gate` should the alias have been deleted.

**Why this helps.** GitHub evaluates branch protection outside the workflow.
Dual publication makes the transition backward compatible across that external
boundary and prevents an `Expected` check that can never start. Separating
addition, settings migration, and removal also gives each step an observable
rollback point.

### 4. Treat live GitHub settings as versioned CI state

Repository rulesets, required contexts, bypass actors, and relevant Actions
settings are part of the CI system even though they do not live in the Git
tree. Add a minimal desired-state document under `.github/ci/` and a read-only
auditor that compares it with GitHub's live configuration.

The auditor must report at least:

- required checks present in GitHub but absent from desired state;
- desired required checks absent from GitHub;
- required checks that no current workflow can produce;
- blocking workflow checks that are not required;
- unexpected changes to bypass policy; and
- the repository, ruleset identifier, observed revision or timestamp, and
  authentication failure when live state cannot be read.

The desired-state document is the reviewable authority. Applying it to GitHub
remains an explicit administrative action; the ordinary pull-request token
must not be granted mutation rights merely to make the audit convenient.

**Example.** Desired state names `ci-gate`, while ruleset
`protect-your-bacon` still names `ci-verdict`. The auditor reports both sides
of the drift before the obsolete workflow job is removed.

**Example.** A developer accidentally grants an admin bypass while changing
the required check. The read-only audit reports the bypass-policy difference
instead of silently treating the easier merge as success.

**Why this helps.** External settings stop being undocumented memory. A code
review can see the intended policy, automation can detect drift, and inability
to inspect GitHub is reported as unknown rather than mistaken for agreement.

### 5. Provide a fast synthetic validation command

Add a canonical `just ci-contract` command that validates the CI control plane
without compiling package code or waiting for hosted runners. Its target is a
clean-machine runtime under one minute.

It should use small synthetic plans and artifacts to exercise:

- affected-scope selection and plan schema validation;
- mixed `execute`, `reuse`, and `accepted-gap` cells across macOS, Linux,
  native Windows, and WSL2;
- plan-to-matrix bijection;
- passing, failing, skipped, cancelled, and missing producer results;
- cumulative same-head evidence retries;
- producer truthfulness and advisory-job boundaries;
- `ci-gate` folding;
- required-check desired-state consistency; and
- OS-aware worker-budget resolution.

The pre-push hook and CI-tooling job run this exact command. They must not
maintain separate lists of component tests.

**Example.** A fixture starts with eight passing macOS evidence cells, then
simulates a retry of one failed cell. The resulting receipt must still contain
all eight passing cells and retain every referenced report. This reproduces the
same-head retry defect without executing thousands of Rust tests.

**Example.** A four-core CI fixture resolves Claudine CLI L1 to four workers; a
three-core macOS fixture resolves it to three. A Sniff Windows fixture may
resolve to one only through its explicit shared-network exception.

**Why this helps.** Developers get high-signal feedback before a push, and the
same assertions run again in CI. Synthetic fixtures make rare failure states
cheap to reproduce and allow exhaustive combinations that would be wasteful or
impossible to induce reliably on real runners.

### 6. Reconcile the plan after every hosted run

Every run produces a machine-readable completeness report satisfying:

```text
resolved required cells
    == qualifying reused cells
     + governed accepted gaps
     + completed producer cells
```

The sets must be disjoint and keyed by exact
`{package, environment, gate}` identity. Reconciliation also reports duplicate
producers, unplanned results, missing artifacts, rejected evidence, and any
producer whose GitHub conclusion contradicts its recorded cell result.

The owning producer remains authoritative for an ordinary test or compile
failure. Reconciliation fails only for structural incompleteness or
contradiction, avoiding a second red check for one test failure. Its rendered
summary must distinguish `PASS`, `FAIL`, `REUSED`, `ACCEPTED GAP`, `MISSING`,
and `UNPLANNED` rather than collapsing them into green/red.

**Example.** A macOS test is omitted because of local evidence, all Linux and
Windows producers pass, and two governed L2 gaps are neutral. Reconciliation
shows the reused macOS cell and the two accepted gaps explicitly; it does not
expect a nonexistent macOS artifact.

**Example.** A workflow condition accidentally drops a planned Windows cell.
No producer artifact exists, so reconciliation reports that exact cell as
`MISSING` and blocks even if every job GitHub happened to schedule is green.

**Why this helps.** Green jobs prove only that scheduled work succeeded. This
audit proves that all required work was either executed, reused with evidence,
or governed. It catches silent omissions while preserving one clear owner for
ordinary failures.

### 7. Make CI migrations small and independently observable

CI control-plane changes should be split at compatibility boundaries. A pull
request may introduce the compatibility needed for the next step, but should
not simultaneously replace planning, evidence format, workflow projection,
presentation, merge-gate identity, and unrelated product behavior.

Recommended migration units are:

1. Add schemas, synthetic fixtures, and compatibility readers.
2. Add the new producer or check beside the old path.
3. Switch one consumer to the new path and observe a hosted canary.
4. Change external repository settings and verify them by read-back.
5. Remove the old path after at least one green real run.

When a change affects global CI inputs and therefore selects a wide matrix,
first run a `workflow_dispatch` canary with a deliberately small synthetic or
package scope. The canary proves orchestration and naming; it does not replace
the final affected-package run.

**Example.** The evidence migration should first teach readers to accept both
receipt versions, then publish version 2, then verify per-cell reuse on a small
plan, and only later remove version-1 support. Combining those steps makes a
rejection indistinguishable from a planner, publisher, or reader defect.

**Example.** The `ci-verdict` to `ci-gate` transition belongs in a gate-only
migration with dual publication. It should not share its observation window
with a rollup rewrite or a product change whose failures obscure the gate.

**Why this helps.** Smaller migrations reduce the number of plausible causes
when something fails, preserve a working rollback path, and let hosted evidence
validate one changed boundary at a time. Compatibility steps allow progress
without forcing one large, high-risk cutover.

## Operating Workflow

For every CI/CD change:

1. State the single control-plane boundary being changed.
2. Run GitNexus impact analysis for edited symbols and confirm unresolved
   dynamic or configuration consumers with text search.
3. Add or update the contract that would have failed before the change.
4. Run `just ci-contract` locally.
5. Preview the exact resolved plan and hosted matrix before pushing.
6. Use a small hosted canary when workflow orchestration or check identity
   changes.
7. Inspect the post-run completeness report and live-settings audit.
8. Remove compatibility code only in a later change with observed evidence.

## Acceptance Criteria

1. One canonical plan inventories every required cell and is the sole source
   of hosted execution matrices.
2. Contract tests detect missing, duplicate, unplanned, reused, and governed
   cells across all supported environments.
3. Required-check migrations cannot remove the old producer before live
   settings have been verified on the new context.
4. A read-only audit detects drift between desired and live GitHub repository
   policy, including an unproducible required context.
5. `just ci-contract` completes without package compilation, runs the same
   control-plane suite locally and in CI, and meets its one-minute target on a
   clean supported development host.
6. Every hosted run publishes an exact, disjoint reconciliation of planned,
   reused, accepted-gap, and executed cells.
7. CI-profile concurrency cannot reduce a package below the OS-aware worker
   budget without an explicit, tested resource-specific exception.
8. At least one fixture reproduces each PR #76 failure described in this
   specification and fails when its corresponding safeguard is removed.
9. CI migration documentation defines compatibility, observation, external
   settings, rollback, and removal as separate rollout stages.

## Non-Goals

- Eliminating `coverage-audit`; it remains necessary for structural
  completeness until every producer self-validates all of its own evidence.
- Treating WSL2 as native Windows or allowing evidence from one environment to
  satisfy another.
- Automatically mutating repository rulesets from ordinary pull-request CI.
- Replacing real OS execution with synthetic tests.
- Optimizing package build or test performance beyond enforcing the agreed
  worker-budget policy.

## Open Questions

- Which checked-in desired-state format should represent repository rulesets
  without duplicating GitHub-specific fields that do not affect merge policy?
- Where should the authenticated live-settings audit run so forks and local
  contributors receive a clear `UNAVAILABLE` result without gaining settings
  credentials?
- How narrow should the first hosted canary be while still exercising reusable
  workflows and the final fixed-name gate exactly as a pull request does?
