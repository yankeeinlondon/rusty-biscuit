---
$schema: feature-review.yaml
ready: false
findings:
  - title: Unavailable Rust test counts are reported as numeric zero
    priority: medium
  - title: Required hosted workflow behavior remains unverified
    priority: high
human_review: false
reviewed_by: codex/gpt-5.6-sol
created: "2026-09-15T03:40:50-07:00"
spec: _complete/2026-09-13-cicd-redundancies/spec.md
implemented: true
implemented_by: claude/opus
log: fixes/_complete/2026-09-13-cicd-redundancies/log.md
description: "A **fix** review of `2026-09-13-cicd-redundancies/spec.md`"
fix: _complete/2026-09-13-cicd-redundancies/review-4.md
previous: _complete/2026-09-13-cicd-redundancies/review-3.md
next: _complete/2026-09-13-cicd-redundancies/review-5.md
---

# Review 4

**Not production-ready.** Review 3's lint-clock and partial-duration defects
are fixed and their executable regressions pass. The required hosted GitHub
Actions validation remains blocked, and the consolidated report still renders
unavailable Rust test counts as numeric zero instead of labeling the missing
measurement.

## Previous-review disposition

The supplied previous-review path,
`prompts/_reviews/fixes/2026-09-13-cicd-redundancies/review-3.md`, does not exist
in this worktree. The canonical colocated review at
`fixes/_complete/2026-09-13-cicd-redundancies/review-3.md` was reviewed and updated.

Review 3's `## Unblocked Findings` are resolved:

- **Lint timing compares separate-process clocks and fails the mandatory L1
  suite — closed.** The workflow now starts and stops `time.monotonic()` in the
  same Python process that owns the `just _lint` child. The extracted workflow
  body was exercised with successful and failing children and preserves their
  exit codes. The formerly failing `test-toolkit` L1 suite now passes all 183
  selected tests.
- **Environment durations silently omit unmeasured cells — closed.** A mixed
  environment now renders a labeled partial duration and names the unmeasured
  `{package}/{tier}` cells. Separate regressions cover complete, wholly
  unmeasured, mixed, and bounded-list cases.

Review 3's `## Blocked Findings` contains one item. It did not become unblocked
before the latest implementation: no commit or push was authorized, and the
acceptance record still marks hosted Validations 5 and 7 `HOSTED-PENDING`.

## Unblocked Findings

### Medium — Unavailable Rust test counts are reported as numeric zero

`render_environment_report` unconditionally sums `cell.counts.total` and emits
the resulting integer (`scripts/ci-rollup.rs:4250-4266`). A missing producer or
a reused version-1 receipt has no count measurement, but both carry
`Counts::default()`. The report therefore prints `0` for a wholly unmeasured
environment and silently understates a mixed measured/unmeasured environment.
The area summary repeats the same unconditional sum
(`scripts/ci-rollup.rs:4392-4404`).

This contradicts Required Behavior 6 and AC13: unavailable cell measurements
must render as `not recorded` with a reason, never as zero. The existing test
named `an_environment_no_producer_reported_renders_not_recorded_rather_than_zero`
actually requires a row containing `tests = 0`; only its duration column is
unavailable (`scripts/ci-rollup-tests.rs:3833-3849`). The version-1 receipt
fixture separately proves that its default counts are not measurements
(`scripts/ci-rollup-tests.rs:3011-3027`).

Model count availability explicitly or derive it from reliable cell evidence,
then apply the same complete/partial/unavailable treatment used for durations.
Add regressions for a wholly missing environment, a mixed measured/unmeasured
environment, a version-1 reused cell, and the per-area aggregate. A genuine
executed suite that selected zero tests must remain distinguishable from an
absent measurement.

## Blocked Findings

### High — Required hosted workflow behavior remains unverified

The acceptance record still has no hosted evidence for specification
Validations 5 and 7 (`acceptance.md:164-228`). Local planner, workflow-source,
shell, and renderer fixtures cannot establish GitHub's matrix expansion,
nested reusable-workflow result propagation, artifact handoff, rendered check
names, or `continue-on-error` conclusions.

After commit and push are explicitly authorized, run the controlled hosted
probes from the specification and retain machine-readable job conclusions,
check names, area result-slice contents, the advisory report, and the
`ci-gate` outcome. The hosted run must also exercise the single-process lint
timer on each hosted image and the partial-duration rendering added in this
iteration. This remains an integration-boundary gap, not a request for
additional cross-OS proof or human design review.

## Requirement-to-verification map

This specification changes CI orchestration and reporting rather than terminal
input or terminal rendering. L2 real-terminal and L3 OS-input tests are not
applicable.

| Requirement | Strongest verification inspected | Assessment |
|---|---|---|
| AC1–AC5, AC7, AC10, AC15 | L1 planner, schema, registry, workflow contracts, executable shell fixtures, and `actionlint` | Appropriate and passing; review 3's conclusions remain supported. |
| AC6, AC11, AC12, AC14, AC16 | L1 planner, renderer, workflow-source, and shell fixtures | Hosted Actions boundary remains missing; these local tests cannot prove the specified hosted orchestration. |
| AC8–AC9 | Native Windows L1 evidence and L1 source contracts recorded in `acceptance.md` | Appropriate tier. Cross-OS evidence itself does not determine readiness. |
| AC12–AC13 duration reporting | L1 Rust renderer tests and an executable workflow-step fixture | Appropriate and passing, including mixed availability and failing-child status propagation. |
| AC12–AC13 test-count reporting | L1 renderer inspection and existing missing/v1 fixtures | Incorrect: unavailable counts render as numeric zero, and mixed aggregates are not labeled partial. |

## Verification performed

- `actionlint .github/workflows/ci.yml .github/workflows/_package-ci.yml
  .github/workflows/_area-ci.yml .github/workflows/_wsl-ci.yml`: passed.
- `just _test repo-deps` with an isolated `CARGO_TARGET_DIR`: **265 passed, 0
  skipped**.
- `just _test test-toolkit` with an isolated `CARGO_TARGET_DIR`: **183 passed,
  2 skipped**. The two skipped Nextest self-verification fixtures are recorded
  as pre-existing in `acceptance.md`.
- `just ci-local --plan`: passed and rendered 21 planned cells across
  `biscuit-tui`, `root`, and `tools`.
- GitNexus was bound to `rusty-biscuit`; its committed index matches `HEAD` but
  is stale for the working-tree implementation. Upstream impact for
  `render_environment_report` is low: `render_report` → `cmd_summarize` →
  `run`, with no indexed execution process.
- No full-workspace, L2, L3, push, workflow dispatch, ruleset mutation, or
  commit was performed.
