---
created: 2026-09-13
status: proposed
implemented: false
reviewed: false
review_iterations: 0
area: repository-ci
relates_to:
  - fixes/2026-09-11-cicd-cleanup/spec.md
---

# CI Redundancies: One Scheduling Model, One Owner Per Suite, One Report

## Objective

Every unit of CI work should be selected by the resolved plan, owned by exactly
one job, and reported once. Today three legs escape that: two because their
subjects cannot be expressed as plan cells, and one because it duplicates
another job's suites outright. A fourth gap is that a change requiring no tests
produces no account of itself — locally or in CI.

This fix folds the escapes back into the cell model, removes the duplication,
and replaces the advisory summary with a report that states what changed, what
ran, where it ran, and what it cost.

## Observed Problems

Measured 2026-09-13 against `ci.yml` at `08f536b08`.

### P1 — `preflight` and `ci-tooling` run the same eight suites

Both jobs run, on the same commit, in the same run:

```
test_affected_scope.py   test_constraints.py    test_evidence_reuse.py
test_local_evidence.py   test_publish_gaps.py   test_resolved_plan.py
test_reuse_validation.py test_schema.py
```

`test_runner_loss.py` is preflight's alone. `ci_workflow_contracts`,
`ci-rollup`, `ci-plan`, `test_ci_local.py` and the test-audit check are
ci-tooling's alone. Everything else is executed twice.

This is also why a change that schedules no packages still runs a
`preflight (ubuntu-latest)` job: it is not preflighting anything: it is
re-running CI's self-tests under a name that says otherwise. The plan says so
in its own words — `preflight_reason: "no build/test packages affected;
preflight runs on the scope host only"`.

### P2 — `ci-tooling` is a top-level leg because its subjects cannot be cells

`ci-tooling` is gated on a bare boolean, `needs.scope.outputs.ci_tooling`. The
resolved plan carries no cell for it, so it has no
`{package, environment, gate}` identity, no evidence reuse, and no per-input
granularity. Any pull request that touches one `ci_tooling` input pays the
whole leg on **every subsequent push**, including a push that changes only a
document. Its subjects cannot currently be cells:

- `scripts/` declares its own `[workspace]` (package `repo-deps`) and is not a
  member of the root workspace.
- `tools/test-toolkit` is a member but carries
  `[package.metadata.ci] gates = false`, `exclusion-class = "promotion-pending"`,
  expiry `2026-10-31`, whose recorded reason already names this problem: *"63 L1
  tests that currently run in NO CI job. Promotion is blocked on the canonical
  just recipe set (check-canonical) for tools/."*

The tests themselves are not in question — they are load-bearing, and they
caught a real defect in `CI_TOOLING_PREFIXES` on 2026-09-13. What is in question
is the leg.

### P3 — `biscuit-tui-captured-stdout` is a second escape of the same kind

A top-level job calling a dedicated reusable workflow, for exactly one test:
`biscuit-tui/cli/tests/windows_captured_stdout.rs`,
`captured_stdout_receives_only_value_no_tui_bytes`. It is `#[ignore]`d so it
compiles everywhere and runs only when named, and it is invoked with
`cargo test … -- --ignored --nocapture` on a Windows host, bypassing nextest and
the tier filtersets entirely.

`biscuit-tui` is an ordinary package with ordinary tests. One test needing a
Windows host and an attached console is what the **tier system already exists
for**. D12 already moved this leg once — from self-triggering on paths to
scope-selected orchestration. This proposes the last step.

### P4 — a no-test change accounts for itself nowhere

A pure documentation change is correctly scheduled as nothing. Verified at all
three levels:

| change | class | packages | areas | flags |
|---|---|---|---|---|
| `docs/…` (monorepo root) | documentation | 0 | 0 | none |
| `<area>/docs/…` (package area) | documentation | 0 | 0 | none |
| `<package>/README.md` (package) | documentation | 0 | 0 | none |

The scheduling is right. The reporting is absent at both ends: the pre-push
hook prints no account of which documents changed, and CI produces an
`infrastructure summary (advisory)` whose three steps are *Reuse successful PR
validation*, *No package was scheduled*, and *Classify the first actionable
failure* — a navigation aid for bootstrap jobs, not a report. A reviewer
learns what happened by noticing an absence.

### P5 — an empty matrix reads as a hung job

`area-ci` with zero scheduled areas renders in the Actions graph as
**"Waiting for pending jobs"** and resolves to `skipped` only at the end. The
outcome is correct; the presentation says the opposite of what is true for the
duration of the run.

## Required Behavior

### 1. One scheduling model

Every unit of CI work is selected from the resolved plan as a
`{package, environment, gate}` cell. No top-level job may be gated on a bare
scope flag as its own unit of work.

Consequently `ci_tooling` ceases to be a scheduling flag. It may remain as a
*change-classification* input if the calculator needs it to widen selection, but
it must not gate a job.

### 2. CI's own suites become owned packages

`repo-deps` and `test-toolkit` become schedulable like any other package, so
their suites are selected by scope, reuse local evidence, and skip when their
inputs did not change. `test-toolkit`'s `promotion-pending` exclusion is retired
by this fix rather than expiring unattended on 2026-10-31.

### 3. One owner per suite

Each suite named in P1 runs in exactly one job per run. `preflight` keeps only
what is genuinely a precondition for other work — toolchain and tooling
availability, canonical recipe validation — and stops re-running CI's
self-tests.

A contract test asserts no suite command appears in two top-level job
definitions.

### 4. The Windows console test becomes a cell

`captured_stdout_receives_only_value_no_tui_bytes` is selected by tier and
environment like every other test, on a Windows cell of `biscuit-tui-cli`.
`biscuit-tui-windows-captured-stdout.yml` and the top-level job that calls it are
removed. The test's runtime precondition proof — that it fails loudly when the
console contract does not hold — is preserved exactly; that assertion is the
evidence, and nothing here weakens it.

### 5. A change report replaces the advisory summary

`summary` is replaced by `ci-reporting`, which always runs and reports:

- **Change summary** — counts of configuration files, documents, and source
  files changed; packages impacted directly; packages impacted as reverse
  dependencies.
- **Test counts** per OS, when any testing ran.
- **Lint timing**, noting that lint is Linux-only and absent entirely for a
  change that schedules none.
- **Test timing** per OS, each carrying a **`local` or `cicd` flag** recording
  where the cell was actually executed.

The `local`/`cicd` flag is the point: when the pre-push hook satisfies a cell,
CI currently just does not run it, and nobody can see that this is what
happened. Reuse must be visible as a stated outcome, not inferred from an
absence.

`ci-reporting` is advisory and never gates: `ci-gate` remains the policy-free
fold, and nothing here changes which job the merge rule names.

The data already exists. The resolved plan carries `change_class`, the package
and reverse-dependency sets, and `preflight_reason`. Per-cell counts, durations
and outcomes come from `manifest.jsonl` and `status-<package>-<job>[-<environment>]/status.json`.

### 6. A documentation change accounts for itself, locally and in CI

When a change schedules no tests, both ends say so affirmatively and name what
changed:

- The **pre-push hook** reports the documentation changes it evaluated and
  states that no tests are required, at all three levels — monorepo root,
  package area, and package.
- **`ci-reporting`** renders the same change summary and states that the pull
  request is ready to merge with no tests required.

Neither end may present this as a warning, a skip, or an error. It is the
expected outcome for the change class.

### 7. An unscheduled matrix reads as unscheduled

A matrix job with zero entries presents as skipped or neutral from the start of
the run, not as pending work.

## Design Decisions

- **D1 — the tests stay; the legs go.** Nothing in P1–P3 argues that a suite is
  unnecessary. Each is load-bearing. The defect is the scheduling shape.
- **D2 — reuse must be visible.** An outcome that says "reused from local
  evidence, 248 tests, 35.5 s, macOS" is strictly better than a job that did not
  appear. Silent absence is indistinguishable from a scheduling bug, which is
  the failure mode this repository has already hit more than once.
- **D3 — `ci-gate` is untouched.** This fix changes what runs and what is
  reported, never what blocks. The required context stays `ci-gate` and its fold
  stays policy-free.
- **D4 — no new store.** Reporting reads existing artifacts. If a number is not
  already recorded, it is reported as absent rather than invented.

## Open Questions

### OQ1 — How does `repo-deps` become schedulable?

`scripts/` declares its own `[workspace]`. Three shapes:

- **Option A — make it a root workspace member.** Most uniform; it then behaves
  exactly like every other package. Cost: it joins the root lockfile and
  resolution graph, which was presumably why it was kept separate.
- **Option B — teach the calculator about secondary workspaces.** Keeps the
  manifests separate; adds a concept, and every consumer of the plan must
  understand it.
- **Option C — leave `repo-deps` out of the cell model and run its suites inside
  the `test-toolkit` cell.** Cheapest; couples two unrelated subjects and makes
  one package's results cover another's.

Needs Ken's ruling. Option A is the recommendation unless the separate lockfile
is deliberate.

### OQ2 — Does promoting `test-toolkit` require `check-canonical` first?

Its exclusion record names the canonical just recipe set for `tools/` as the
blocker. Is that still true, and is it in scope here or a prerequisite?

### OQ3 — What does the documentation report actually render?

"A visual diff, or at least a summary" spans two different renderers. In a
terminal hook a rich diff is available. In a GitHub job summary it is Markdown.
Options: a per-file changed-line summary in both; a rendered diff locally and a
file list in CI; or a rendered diff in both, accepting the size cost on large
documentation changes.

### OQ4 — Does `ci-reporting` replace `summary` or sit beside it?

The advisory summary's *Classify the first actionable failure* step is a real
navigation aid for the bootstrap jobs no area coverage audit covers. Absorb that
into `ci-reporting`, or keep both jobs with distinct responsibilities?

## Implementation Boundaries

- No change to `ci-gate`, its fold, or the `protect-your-bacon` ruleset.
- No change to which environments are required for any package.
- No weakening of the `biscuit-tui` console precondition assertion.
- No new persisted store; reporting reads existing plan and artifact data.
- Comment and documentation passes accompany each behavioral change, per repo
  policy. `.github/ci/README.md` and the `rust-devops` skill are updated in the
  same change as the workflow edits.

## Acceptance Criteria

1. No suite command appears in two top-level job definitions in `ci.yml`; a
   contract test enforces this and fails against the current file.
2. `preflight` runs no CI self-test suite that another job owns; its remaining
   steps are preconditions for other work.
3. A change touching only `scripts/` schedules that package's cells and nothing
   else; a change touching neither `scripts/` nor `tools/` schedules neither.
4. A second push to a pull request that changed a `ci_tooling` input, where that
   push changes only a document, re-runs none of those suites and reports them
   as reused.
5. `ci-tooling` and `biscuit-tui-captured-stdout` no longer exist as top-level
   jobs; `biscuit-tui-windows-captured-stdout.yml` is deleted.
6. The Windows captured-stdout test runs as a `biscuit-tui-cli` cell on a
   Windows environment, and still fails loudly when its console precondition
   does not hold.
7. A documentation-only change at each of the three levels produces: a pre-push
   report naming the changed documents and stating no tests are required, a CI
   report saying the same, zero test jobs, and a mergeable pull request.
8. `ci-reporting` renders change counts, per-OS test counts, lint timing, and
   per-OS test timing, each timing carrying a `local` or `cicd` flag; a cell
   satisfied by local evidence appears with its actual counts and duration and
   is labelled `local`.
9. A run with zero scheduled areas shows the area matrix as skipped or neutral
   from the start, never as pending.
10. `ci-gate` behavior is byte-identical before and after; the required context
    is unchanged.

## Validation and Rollout

1. Contract tests for AC1, AC2 and AC9 land first and fail against the current
   workflow, proving they are not vacuous.
2. Fixture changes covering AC3, AC4 and AC7 exercise the calculator without a
   hosted run.
3. AC5, AC6 and AC8 need a hosted run on this fix's own branch. AC6 needs a
   native Windows result specifically.
4. The `test-toolkit` exclusion record is removed in the same change that
   promotes it, not left to expire.
5. `docs/cicd/` gains no new document; this fix's outcome is recorded in
   `.github/ci/README.md`, which already owns the run-shape description.
