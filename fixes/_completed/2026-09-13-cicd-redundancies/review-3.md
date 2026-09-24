---
$schema: feature-review.yaml
ready: false
findings:
  - title: Lint timing compares separate-process clocks and fails the mandatory L1 suite
    priority: high
  - title: Required hosted workflow behavior remains unverified
    priority: high
  - title: Environment durations silently omit unmeasured cells
    priority: medium
human_review: false
reviewed_by: codex/gpt-5.6-sol
created: "2026-09-14T20:46:27-07:00"
spec: _complete/2026-09-13-cicd-redundancies/spec.md
implemented: true
implemented_by: claude/opus
log: fixes/_complete/2026-09-13-cicd-redundancies/log.md
description: "A **fix** review of `2026-09-13-cicd-redundancies/spec.md`"
fix: _complete/2026-09-13-cicd-redundancies/review-3.md
previous: _complete/2026-09-13-cicd-redundancies/review-2.md
next: _complete/2026-09-13-cicd-redundancies/review-4.md
---

# Review 3

**Not production-ready.** The neutral reuse report and mutually exclusive
reporting modes from review 2 are now implemented correctly, but the lint-clock
fix makes the required `test-toolkit` L1 suite fail intermittently on macOS and
can publish negative durations. The required hosted GitHub Actions validation
also remains undone. A separate reporting gap presents partial environment
duration totals as though they were complete.

## Previous-review disposition

The supplied previous-review path,
`prompts/_reviews/fixes/2026-09-13-cicd-redundancies/review-2.md`, does not exist
in this worktree. The canonical colocated review at
`fixes/_complete/2026-09-13-cicd-redundancies/review-2.md` was reviewed and updated.

Review 2 contains neither an `## Unblocked Findings` nor an
`## Blocked Findings` section; its four findings are listed under `## Findings`
and in frontmatter. Their dispositions are:

- **The advisory reuse report makes a run-level CLEAR claim — closed.** The
  reuse branch now reports only the successful prior validation and the fact
  that this run executed no package cell. The workflow contract applies the
  renderer's policy-vocabulary exclusions to the shell-authored report too.
- **The reporting modes overlap and cancellations are misclassified — closed.**
  Mode 3 is the negation of modes 1 and 2, cancellation is actionable, and an
  executable Bash fixture plus an exhaustive guard test cover the state space.
- **Required hosted workflow behavior remains unverified — open and blocked.**
  The acceptance record explicitly says the requested hosted evidence still
  does not exist.
- **Sub-second lint measurements are reported as unavailable — not closed.**
  Presence is now modeled independently with `Option<f64>`, but the replacement
  clock is not reliable across the two processes used to read it and fails its
  own new regression.

No prior blocked finding became unblocked before this implementation. The
hosted finding remains blocked on an explicitly authorized commit and push.

## Unblocked Findings

### High — Lint timing compares separate-process clocks and fails the mandatory L1 suite

The lint step reads `time.monotonic()` in two independent Python processes and
subtracts the first process's value from the second
(`.github/workflows/_package-ci.yml:664-687`). Its comment claims those values
are safely comparable on every supported platform. They were not comparable in
this review run: the shipped executable regression
`the_lint_step_measures_a_sub_second_command_instead_of_recording_zero` recorded
`duration_s=-0.001` and failed at
`tools/test-toolkit/tests/ci_workflow_contracts.rs:1946`.

This is both a product defect and a test-stability defect. A negative duration
can enter the producer artifact and render in the advisory report, and the
ordinary macOS L1 cell for `test-toolkit` can fail depending on the clock offset
observed by two short-lived processes. The implementation log's earlier pass
does not make a nondeterministic test acceptable.

Measure both endpoints in one process that owns the `just _lint` child and
propagates its exit status, or use another single-process timing boundary. Keep
the existing regression, but require a finite, non-negative measurement and
exercise both a successful and a failing lint child so timing never masks the
gate result.

### Medium — Environment durations silently omit unmeasured cells

`render_environment_report` collects only present cell durations and reports
their sum whenever at least one cell was measured
(`scripts/ci-rollup.rs:4250-4275`). If an environment has two test cells and one
has `duration_s: None`—for example, a missing producer or legacy reused
evidence—the row prints the other cell's duration as the environment duration
without saying the aggregate is partial.

That violates the specification's requirement that unavailable measurements
be shown as unavailable with a reason, rather than silently manufacturing a
complete-looking value. Add a mixed measured/unmeasured regression. The row
should either render the duration as `not recorded` with the missing package or
cell named, or explicitly label the numeric sum as partial.

## Blocked Findings

### High — Required hosted workflow behavior remains unverified

Review 2 required a controlled GitHub Actions run for AC6, AC11, AC12, AC14,
and AC16. The current acceptance record still marks Validations 5 and 7
`HOSTED-PENDING` and states that no machine-readable record exists
(`acceptance.md:164-202`). Local planner, YAML, expression, Bash, and rollup
tests cannot establish GitHub's matrix expansion, nested reusable-workflow
result propagation, artifact handoff, Checks-tab labels, or
`continue-on-error` conclusions.

Complete the specification's hosted Validation 5 and 7 probes after commit and
push are explicitly authorized. Retain job conclusions, rendered check names,
area result-slice contents, the advisory report, and the `ci-gate` outcome.
This is an integration-boundary gap, not a request for cross-OS proof or human
design review.

## Requirement-to-verification map

This specification changes CI orchestration and reporting, not terminal input
or rendering. L2 real-terminal and L3 OS-input tests are therefore inapplicable.

| Requirement | Strongest verification inspected | Assessment |
|---|---|---|
| AC1–AC5, AC7, AC10, AC15 | L1 planner, schema, registry, workflow contracts, and executable shell fixtures | Appropriate and passing in `repo-deps`; review 2's ownership conclusions remain supported. |
| AC6, AC11, AC12, AC14, AC16 | L1 planner/renderer/workflow fixtures | Hosted Actions boundary remains missing; not sufficient for the specified orchestration behavior. |
| AC8–AC9 | Native Windows L1 record and L1 source contracts from the acceptance record | Appropriate tier; additional cross-OS proof does not determine readiness. |
| AC12–AC13 duration reporting | L1 Rust renderer tests plus executable workflow-step fixture | Renderer unit cases pass, but the workflow fixture fails and mixed measurement availability is untested and incorrect. |

## Verification performed

- `actionlint .github/workflows/ci.yml .github/workflows/_package-ci.yml`:
  passed.
- `just _test repo-deps` with an isolated `CARGO_TARGET_DIR`: **261 passed**.
- `just _test test-toolkit` with an isolated `CARGO_TARGET_DIR`: **181 passed,
  1 failed, 2 skipped**. The failure recorded `duration_s=-0.001` in the new
  lint timing contract.
- Initial runs against the shared `target/` failed before tests because cached
  `.rmeta` files were not writable. Isolated-target reruns separated that host
  condition from the real test result.
- GitNexus was refreshed for this worktree; pre-review change analysis reported
  10 files / 34 symbols, no affected process, and low risk. The timing failure
  was established by executable evidence rather than inferred from graph risk.
- No full-workspace, L2, L3, push, workflow dispatch, ruleset mutation, or
  commit was performed.
