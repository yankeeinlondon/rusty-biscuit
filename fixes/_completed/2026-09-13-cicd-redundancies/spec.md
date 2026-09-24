---
created: 2026-09-13
status: complete
closed_on: 2026-09-15
implemented: true
reviewed: true
reviewed_by: codex/gpt-5.6-sol
reviewed_on: 2026-09-13
review_iterations: 7
area: repository-ci
depends-on:
  - fixes/2026-09-11-cicd-cleanup/spec.md
---

# CI Redundancies: One Scheduling Model, One Owner Per Suite, One Consolidated Report

## Closure decision — 2026-09-15

Closed by explicit user decision with the remaining hosted validation uncertainty
accepted. Validations 5 and 7 and the accumulated hosted assertions in Review 7
are **DEFERRED**, not passed. The implementation and existing local evidence are
accepted as sufficient to close this spec; no new hosted evidence is claimed.

Observe the first normal implementation CI run and a subsequent documentation-only
change. Report observed defects as focused fixes; reopen this work only if a core
design assumption fails. Passive observation does not guarantee coverage of
forced reporting failures, cancellation, or the other controlled probe cases.
See [Acceptance](acceptance.md#closure-decision--2026-09-15) for the disposition.

## Objective

Every build, lint, test, and companion-suite invocation must be selected by the
resolved plan, owned by exactly one package cell, and reported once. The
orchestration jobs that calculate scope, verify runner prerequisites, fold the
merge gate, and render the advisory report are not cells because they produce
no package evidence.

Today three test paths escape that rule: `preflight` duplicates eight suites
from `ci-tooling`; `ci-tooling` itself is selected by a bare flag rather than
package cells; and the Biscuit TUI Windows console test bypasses the package
matrix and Nextest. A fourth gap is that a change requiring no package work
produces no useful account of itself locally or in CI.

This fix assigns every retained suite to a package, removes the duplicate and
specialized jobs, makes reuse visible with the repository's established origin
vocabulary, and replaces the current advisory navigation aid with one
consolidated run report. It does not move policy out of area coverage audits or
the merge decision out of `ci-gate`.

## Relationship to the CI Cleanup Specification

This specification directly depends on
`fixes/2026-09-11-cicd-cleanup/spec.md`. It preserves that specification's
resolved-plan authority, package-keyed evidence, area-only grouping, per-area
coverage audits, governed-gap behavior, and policy-free `ci-gate` fold.

> **Reader's note (review, 2026-09-13).** The original draft used
> `relates_to`, but implementation assumes the prior plan schema, area fan-out,
> local-evidence overlay, and `ci-gate` architecture are already present. This
> is therefore a direct dependency, not merely related work.

## Observed Problems

Measured 2026-09-13 against `ci.yml` at `08f536b08`.

### P1 — `preflight` and `ci-tooling` run the same eight suites

Both jobs run, on the same commit, in the same workflow run:

```text
test_affected_scope.py   test_constraints.py    test_evidence_reuse.py
test_local_evidence.py   test_publish_gaps.py   test_resolved_plan.py
test_reuse_validation.py test_schema.py
```

`test_runner_loss.py` is preflight's alone. `ci_workflow_contracts`,
`ci-rollup`, `ci-plan`, `test_ci_local.py`, and the test-audit check are
ci-tooling's alone. The duplicated eight execute twice without producing
different evidence.

This is also why a change that schedules no packages still runs a
`preflight (ubuntu-latest)` job. The plan says
`preflight_reason: "no build/test packages affected; preflight runs on the
scope host only"`, but the job is primarily re-running CI's self-tests rather
than establishing a precondition for scheduled package work.

### P2 — CI tooling has owners, but its owners are not schedulable

`ci-tooling` is gated on `needs.scope.outputs.ci_tooling`. The resolved plan
carries no cell for that work, so it has no `{package, environment, gate}`
identity, no area-owned result, and no cell-granular evidence.

The existing package model already provides most of the required shape:

- `scripts/Cargo.toml` defines package `repo-deps`, but its nested
  `[workspace]` keeps it outside the root workspace and gives it a separate
  lockfile.
- `tools/test-toolkit` is a root workspace member with a complete canonical
  recipe set in `tools/justfile`, but its manifest still declares
  `gates = false` under an obsolete `promotion-pending` record.
- `[package.metadata.ci.tests].companion-suites` already assigns a non-Cargo
  suite to a Cargo package and makes a skipped or failed companion downgrade
  the owning cell. The implementation is currently hard-coded for only
  `homelab-frontend` and records only one companion outcome.

The suites are load-bearing: they caught a real defect in
`CI_TOOLING_PREFIXES` on 2026-09-13. The defect is their top-level scheduling
shape, not their existence.

### P3 — `biscuit-tui-captured-stdout` bypasses the normal Windows L1 cell

The top-level job calls a dedicated reusable workflow for one test:
`biscuit-tui/cli/tests/windows_captured_stdout.rs` /
`captured_stdout_receives_only_value_no_tui_bytes`. The test is `#[ignore]`d
and invoked with `cargo test ... -- --ignored --nocapture`, bypassing Nextest,
the canonical recipe, JUnit staging, local evidence, and the area coverage
audit.

The test does not require an external terminal harness. It self-allocates a
Windows console, rewires its handles, injects input, and fails when it cannot
prove that precondition. Under the repository's tier contract, that makes it a
Windows-only L1 integration test, not L2: operating system is expressed by
`#![cfg(windows)]`, while L2 is reserved for a real external terminal/PTY
harness. Classifying it as L2 would strand it behind the governed
`windows-latest` L2 capability gap.

> **Reader's note (review, 2026-09-13).** The original draft said the tier
> system should select this test but did not select a tier. This review rules
> L1 explicitly. That is an intentional correction to the older test docs that
> treated `#[cfg(windows)] + #[ignore]` as the appropriate automation shape.

### P4 — a no-test change accounts for itself nowhere

A pure documentation change is correctly scheduled as no package work at all
three ownership levels:

| change | class | packages | areas | flags |
|---|---|---:|---:|---|
| `docs/...` at repository root | documentation | 0 | 0 | none |
| `<area>/docs/...` | documentation | 0 | 0 | none |
| `<package>/README.md` | documentation | 0 | 0 | none |

The scheduling is right. The reporting is absent at both ends: the pre-push
hook does not name the documents it evaluated, and the current
`infrastructure summary (advisory)` is a bootstrap navigation aid rather than a
change-and-execution report. A reviewer learns what happened by noticing an
absence.

The resolved plan currently carries only the coarse `change_class`; it does
not carry the changed paths or category counts needed to produce the requested
report. Raw JUnit manifests and producer statuses also do not currently record
lint command duration or machine-readable counts for every companion suite.
The original draft's statement that all report data already exists was
therefore incomplete.

### P5 — an empty matrix looks pending longer than necessary

`area-ci` with no scheduled areas renders as **Waiting for pending jobs** while
its prerequisites run and resolves to `skipped` only afterward. The outcome is
correct, but the presentation implies undiscovered work. A dynamically scoped
job cannot resolve before `scope`; the enforceable requirement is that it
resolve immediately after `scope`, without waiting for an unnecessary
documentation-only preflight.

## Required Behavior

### 1. One scheduling model for package work

Every build, lint, test, and companion suite is selected from the canonical
resolved plan as a `{package, environment, gate}` cell. No top-level job may be
selected by a bare change-classification flag to run package-owned work.

`ci_tooling` is removed from the plan flags and workflow outputs. Tooling input
paths select the package that owns the affected suite through an explicit,
tested path-to-owner rule. This is selection within the existing plan, not a
second scheduler and not an implicit full-workspace trigger.

### 2. CI's own suites become package-owned

`repo-deps` becomes a root workspace member by removing its nested
`[workspace]`, adding `scripts` to the root workspace, and retiring
`scripts/Cargo.lock`. Its package area is `root`, following the existing
manifest-directory derivation rule.

`test-toolkit` becomes gating now that `tools/justfile` already provides the
canonical recipe set named by its exclusion record. The
`promotion-pending` record is removed in the same change.

Suite ownership is:

| owner package | ordinary L1 suite | companion suite |
|---|---|---|
| `repo-deps` | the complete Nextest suite, including `ci-rollup` and `ci-plan` tests | every `scripts/ci/test_*.py` suite, including `test_ci_local.py` and `test_runner_loss.py` |
| `test-toolkit` | the complete Nextest suite, including `ci_workflow_contracts` | `tools/test-audit` typecheck and Vitest suite |

The companion-suite mechanism is generalized from its one hard-coded outcome
to a closed set of named suites. Each declared suite has one canonical recipe,
one owning package, one CI environment, and one machine-readable outcome with
counts and command duration. The producer status carries outcomes per suite;
one suite's success cannot hide another suite's failure or skip. Unknown suite
names, missing recipes, absent results, and duplicate owners fail contract
validation.

The tooling trigger table names each suite's real inputs. At minimum:

- source files under `scripts/**` select `repo-deps` through normal package
  ownership, while its manifest, canonical runner, and other suite inputs are
  explicit triggers for the same owner;
- `.github/ci/**` selects `repo-deps` for the Python CI contracts;
- `.github/workflows/**` selects `test-toolkit` for workflow contracts;
- `tools/test-audit/**`, `pnpm-lock.yaml`, and `pnpm-workspace.yaml` select
  `test-toolkit` for its test-audit companion;
- changes to a suite or its canonical runner select its owner.

The contract tests pin this mapping so an input cannot silently stop selecting
the suite that verifies it. A shared input may select both owners when both
suites consume it.

Companion suites remain CI-origin on their declared environment unless the
local-evidence schema is deliberately extended later. Their presence must not
make unrelated L1 cells non-reusable. If the current implementation attaches a
companion to L1, only the environment that must run that companion is
non-reusable; Rust-only L1 cells in other environments retain normal reuse.

### 3. One owner per suite

`preflight` retains only prerequisites for scheduled package work: pinned
toolchain initialization, required tool availability, Cargo metadata, and
`just check-canonical`. It runs no Python, Rust, or TypeScript test suite.

When the plan contains no package work, `preflight_os` is empty and preflight
skips. When package work exists, its current OS-breadth policy remains in
force. The package fan-out continues to depend on preflight for work that will
actually execute.

A contract test validates suite identities and owners, rather than comparing
fragile shell-command strings. Every registered suite must have exactly one
owner and may be invoked only by that owner's package job.

### 4. The Windows console test becomes ordinary L1 evidence

`captured_stdout_receives_only_value_no_tui_bytes` remains
`#![cfg(windows)]`, loses `#[ignore]`, and runs under the canonical
`biscuit-tui-cli` L1 recipe with `terminal-tests` enabled. It therefore appears
in the `biscuit-tui-cli/windows-latest/L1` JUnit and producer-status artifacts,
can be reused only as Windows evidence, and is audited by the Biscuit TUI area.

The dedicated `test-windows-captured-stdout` recipe,
`biscuit-tui-windows-captured-stdout.yml`, and the top-level caller job are
removed. The test must continue to fail loudly if it cannot establish and
prove the console precondition. Its fixed readiness sleeps are replaced with a
bounded readiness observation or bounded retry loop; moving it into the normal
parallel suite must not add a timing-only race or steal terminal/browser focus.

### 5. The resolved plan carries one change inventory

The scope calculator classifies its already-supplied changed paths once and
stores a normalized, sorted, repository-relative inventory in the resolved
plan. The inventory is exhaustive and separates at least `configuration`,
`documentation`, `source`, and `other`, with per-category and total counts.
Renames are represented without double-counting one logical changed path. A
manual full-scope run records that it has no diff inventory rather than
inventing changed files.

Both the local renderer and hosted report consume this inventory. Neither
re-runs package selection or independently reclassifies paths. Documentation
paths are listed by name; full diff bodies are not embedded because they can be
unbounded and duplicate the review UI.

Adding this required plan field bumps the resolved-plan schema and generated
contract. Older scope receipts are rejected with the existing `scope-schema`
reason and cause one fresh calculation; they are never upgraded in place.
Validation receipts remain reusable only where their existing cell and
gate-input checks still qualify.

### 6. `ci-reporting` replaces the advisory summary

The current `summary` job is replaced by `ci-reporting`. It runs with
`if: always()` and `continue-on-error: true`, after `scope`, `area-ci`, and
`ci-gate` have resolved. It has three explicit modes:

1. A reused whole-PR validation links the authoritative prior run and states
   that this run executed no package cells.
2. A successful scope calculation reads the resolved plan and existing
   `ci-results-<area-slug>` slices, using `ci-rollup summarize` (or its shared
   typed model) for aggregation. It does not parse raw artifacts into a second
   result model.
3. A failed or cancelled bootstrap reports the first actionable
   infrastructure failure, preserving the useful behavior of the old summary
   without making a run-level policy claim.

For a normal scoped run the report renders:

- the change inventory and direct/reverse-dependency package sets;
- per-environment test counts, including machine-recorded companion counts;
- lint command duration, explicitly identified as the Linux-only `ci` result;
- per-environment test duration and the exact origin vocabulary already used
  by the plan: `ci`, `local`, or `prior-local`;
- cells whose measurements are unavailable, as `not recorded` with a reason.

The literal origin `cicd` is not introduced. `check` and `lint` remain
CI-origin because local receipts have no JUnit evidence for them. Command
duration, not runner setup/queue duration, is recorded in producer status;
existing result-cell `duration_s` remains the report's normalized field.

`ci-reporting` applies no baseline, accepted-gap, missing-cell, or merge policy.
Per-area coverage audits remain the only policy readers, and `ci-gate` remains
the only merge authority. The report may state that no package tests were
required; it must not claim that a pull request is mergeable independently of
`ci-gate` and other required checks.

### 7. A documentation-only change accounts for itself locally and in CI

When the plan selects no package cells:

- the pre-push hook renders the categorized change inventory, names the
  changed documents, and states that no package tests are required;
- `ci-reporting` renders the same inventory and statement;
- preflight and package matrices skip as soon as scope resolves;
- `ci-gate` accepts those skipped jobs under its existing fold.

This is an affirmative successful scheduling decision, never a warning,
failure, accepted gap, or fabricated passing test result. Terminal output is
rendered through a `TerminalRenderable` component (preferably the existing
`ci-plan` typed renderer); the GitHub summary renders the same typed data as
Markdown.

### 8. An unscheduled matrix resolves promptly and honestly

Matrix jobs are guarded by a scalar plan output before matrix expansion. With
zero entries they resolve to `skipped` immediately after `scope`, and no
matrix-expression display name is attached to a skippable job. Contract tests
cover documentation-only, whole-run reuse, and zero-executing-cell plans.

The specification does not require an impossible pre-scope state: GitHub
cannot know a dynamic matrix is empty before the scope dependency completes.

## Design Decisions

- **D1 — the suites stay; duplicate and specialized jobs go.** Each named suite
  remains load-bearing and receives an explicit owner.
- **D2 — `repo-deps` joins the root workspace.** Its nested workspace was
  introduced with the original utility package and has no recorded deliberate
  lockfile-isolation contract. Membership gives the planner, Nextest profile,
  target directory, and lockfile one authority. Teaching every consumer about
  secondary workspaces would add a second package-discovery model; assigning
  its tests to another package would falsify result identity.
- **D3 — use the existing companion-suite seam.** Python and TypeScript suites
  remain non-Cargo companions of the package that owns their contract. This
  preserves package identity without pretending they are Rust tests or leaving
  them as top-level exceptions.
- **D4 — the Windows console test is L1.** The test constructs and proves its
  own Windows resource and needs no optional external harness. L2 would turn
  required evidence into an accepted Windows capability gap.
- **D5 — reuse is visible and uses exact origins.** `local` and `prior-local`
  remain distinct, and hosted execution remains `ci`. Silent absence and a new
  alias such as `cicd` are both rejected.
- **D6 — `ci-gate` policy is unchanged, not byte-identical.** Removing two
  top-level blocking jobs necessarily removes their names from `needs` and the
  fold input. Their failures now reach `ci-gate` through `area-ci`. The fold
  algorithm and accepted conclusions (`success` and `skipped`) do not change.
- **D7 — reporting aggregates; it does not judge.** Existing per-area result
  slices are the machine evidence. `ci-reporting` presents them and preserves
  infrastructure navigation without running another verdict.
- **D8 — schema evolution is explicit.** The change inventory requires a plan
  schema bump and a generated-contract update. A temporary scope-receipt miss
  is safer than accepting a document that cannot support the report.
- **D9 — no new persistent store.** The plan gains data the calculator already
  receives, producer status gains measurements it already observes, and the
  report consumes existing run artifacts. No area-keyed, reporting-only, or
  long-lived store is introduced.

## Open Questions

None. Review resolved the four draft questions as D2, the already-complete
`tools/justfile` prerequisite, Required Behavior 5, and Required Behavior 6.

## Implementation Boundaries

- No change to required package environments or governed capability gaps.
- No weakening of the Biscuit TUI console-precondition assertion and no new
  focus-taking test behavior.
- No area-keyed result, artifact, baseline, or evidence identity.
- No second scope calculator, package-discovery mechanism, verdict, or
  persisted reporting store.
- No full-workspace selection merely because CI tooling changed; suite inputs
  select their owner packages only.
- The `ci-gate` fold semantics and `protect-your-bacon` required context remain
  unchanged; only retired top-level inputs leave its static dependency list.
- Comment and documentation passes accompany each behavioral change.
  `.github/ci/README.md`, `docs/topics/ci-cd.md`, the `rust-devops`,
  `rust-testing`, and `os` skills, and stale Biscuit TUI test/reproduction docs
  are updated with the workflow and tier changes.
- Root workspace membership and lockfile changes are reflected in dependency
  documentation where required; no dependency version is changed merely to
  promote `repo-deps`.

## Acceptance Criteria

1. No Python, Rust, or TypeScript test suite is invoked by both `preflight` and
   another job; preflight contains prerequisite checks only.
2. `repo-deps` is a root workspace member using the root lockfile and canonical
   Nextest configuration; `scripts/Cargo.lock` and its nested `[workspace]` no
   longer exist.
3. `test-toolkit` gates normally and its now-satisfied
   `promotion-pending` exclusion is removed.
4. Every suite in the ownership table has exactly one registered owner,
   canonical recipe, trigger set, selected cell, and machine-readable outcome;
   missing, skipped, duplicated, or unknown companion suites fail a contract
   test.
5. A change touching only `scripts/` selects `repo-deps`; a change touching
   only `tools/test-audit/` selects `test-toolkit`; a change touching neither
   tooling owner nor its declared inputs selects neither package.
6. A subsequent documentation-only push on a pull request whose tooling cells
   already have qualifying evidence reuses eligible cells, reruns only
   non-reusable companion work still required by the pull request diff, and
   reports both outcomes explicitly.
7. `ci-tooling` and `biscuit-tui-captured-stdout` no longer exist as top-level
   jobs; `biscuit-tui-windows-captured-stdout.yml` and the dedicated just recipe
   are removed.
8. The Windows captured-stdout test is discovered and executed by the normal
   `biscuit-tui-cli/windows-latest/L1` Nextest cell, appears in that cell's
   JUnit, and fails if its console precondition cannot be established.
9. The Windows test uses bounded readiness observation/retry rather than a
   fixed sleep and never opens or focuses a terminal or browser window.
10. The plan schema carries the categorized change inventory; plan validation,
    generated schema contracts, scope-receipt rejection fixtures, and both
    renderers agree on its shape.
11. A documentation-only change at each ownership level names the documents
    locally and in CI, states that no package tests are required, creates zero
    package/preflight executions, and leaves the merge decision to `ci-gate`.
12. `ci-reporting` renders direct and reverse dependencies, per-environment
    counts and test durations, Linux lint duration, companion results, and
    exact `ci`/`local`/`prior-local` origins without applying policy.
13. Missing measurements render as unavailable with a reason; they are never
    emitted as zero, `ci`, or pass.
14. A zero-entry area matrix resolves as skipped immediately after scope and
    never displays an unresolved matrix expression.
15. `ci-gate` still accepts only `success` and `skipped`, blocks `failure` and
    `cancelled`, reads no plan/artifact/baseline, and remains the required
    `ci-gate` context. Its dependency list contains no retired top-level job.
16. All-reused areas still fan out far enough to produce their area result
    slices, so `ci-reporting` includes reused cells rather than losing them.

## Validation and Rollout

1. Add pending contract fixtures for suite ownership, tooling path-to-owner
   selection, no-test preflight, Windows L1 discovery, plan change inventory,
   companion result completeness, and prompt empty-matrix resolution. Prove
   each fixture fails against the current implementation for its intended
   reason before promotion.
2. Run the compact Python CI suites, `repo-deps` and `test-toolkit` L1 suites,
   test-audit check, workflow contracts, schema-generation drift check,
   `actionlint`, and the root `just ci-local --plan` preview. Do not substitute
   a full workspace test run for this dependency-derived scope.
3. Validate the root-workspace migration from both repository root and
   `scripts/` so Cargo uses one lockfile, target directory, and Nextest config
   from either working directory.
4. Exercise documentation-only fixtures at repository, area, and package
   levels and verify byte-equivalent inventory data reaches the terminal and
   Markdown renderers.
5. Use a hosted run on this fix's branch to verify job ownership, artifact
   collection, report aggregation, advisory failure behavior, and prompt
   zero-matrix resolution. Verify a failed package-owned tooling suite blocks
   through `area-ci` and `ci-gate` while a reporting failure does not.
6. Obtain native `windows-latest` runtime evidence for the captured-stdout L1
   test. A macOS-to-Windows GNU cross-check is compile evidence only and does
   not satisfy this criterion.
7. Follow with a documentation-only commit or equivalent fixture branch to
   prove eligible prior cells are reported as reused and no suite is silently
   dropped. Older scope receipts may miss once with `scope-schema`; that is the
   expected migration behavior.
8. Update `.github/ci/README.md` and `docs/topics/ci-cd.md` in the same change.
   No new `docs/cicd/` document or persistent report store is added.
