---
$schema: feature-review.yaml
ready: true
findings: []
closed_on: 2026-09-15
disposition: accepted-risk
human_review: false
reviewed_by: codex/gpt-5.6-sol
created: "2026-09-15T04:52:07-07:00"
spec: _complete/2026-09-13-cicd-redundancies/spec.md
implemented: true
description: "A **fix** review of `2026-09-13-cicd-redundancies/spec.md`"
fix: _complete/2026-09-13-cicd-redundancies/review-7.md
previous: _complete/2026-09-13-cicd-redundancies/review-6.md
---

# Review 7

## Closure decision — 2026-09-15

**Closed by explicit user acceptance of the remaining hosted uncertainty.**
Validations 5 and 7 and their accumulated hosted assertions are deferred, not
passed. No new hosted evidence is claimed. Observe normal CI runs and file focused
defects; reopen this spec only if a core design assumption fails. The `ready` and
`implemented` flags record this accepted disposition, not completion of the probes.
See [Acceptance](acceptance.md#closure-decision--2026-09-15).

## Original review assessment (superseded by the closure decision)

**Not production-ready at review time.** Review 6's unblocked schema-reference finding is
fixed and protected by non-vacuous Rust and Python contracts. Its hosted
GitHub Actions finding did not become unblocked before the latest
implementation and remains the only finding.

## Previous-review disposition

The supplied previous-review path,
`prompts/_reviews/fixes/2026-09-13-cicd-redundancies/review-6.md`, does not exist
in this worktree. The canonical colocated review at
`fixes/_complete/2026-09-13-cicd-redundancies/review-6.md` was reviewed and updated.

Review 6's `## Unblocked Findings` is resolved:

- **Schema reference still reports the baseline as version 2 — closed.**
  `.github/ci/schemas/README.md` now states result version 4 and baseline
  version 3. `the_schema_readme_states_this_tools_versions` binds those claims
  to the compiled Rust constants and the shipped baseline document.
  `SchemaReadmeVersionTests` separately binds the plan, validation-receipt,
  and scope-receipt claims to the Python schema constants. Both contracts fail
  when a required phrase disappears instead of passing vacuously.

Review 6's `## Blocked Findings` did not become unblocked before the latest
implementation. The changes remain uncommitted, no commit or push was
authorized, and `acceptance.md` still marks specification Validations 5 and 7
`HOSTED-PENDING` with no machine-readable hosted run record.

## Unblocked Findings

None.

## Deferred finding — accepted at closure

### High — Required hosted workflow behavior remains unverified

The acceptance record still has no hosted evidence for specification
Validations 5 and 7 (`acceptance.md:164-260`). Local planner, workflow-source,
expression, shell, and renderer fixtures cannot establish GitHub's matrix
expansion, nested reusable-workflow result propagation, artifact handoff,
rendered check names, or `continue-on-error` conclusions.

After commit and push are explicitly authorized, run the controlled hosted
probes from the specification and retain machine-readable job conclusions,
check names, area result-slice contents, the advisory report, and the
`ci-gate` outcome. The run must exercise the accumulated review fixes,
including exclusive reporting modes, cancellation reporting, single-process
lint timing, partial duration and count rendering, the distinction between
absent and genuinely zero counts, result schema version 4, and rejection of a
retained version-3 slice by version. This is a hosted integration-boundary gap,
not a request for additional cross-OS proof or human design review.

## Requirement-to-verification map

This specification changes CI orchestration and Markdown/terminal reporting,
not terminal input encoding or real-terminal rendering. L2 real-terminal and
L3 OS-input tests are not applicable.

| Requirement | Strongest verification inspected | Assessment |
|---|---|---|
| AC1–AC5, AC7, AC10, AC15 | L1 planner, schema, registry, workflow contracts, executable shell fixtures, and `actionlint` | Appropriate and passing. |
| AC6, AC11, AC12, AC14, AC16 | L1 planner, renderer, workflow-source, expression, and shell fixtures | Hosted Actions boundary remains missing; these tests cannot prove the specified hosted orchestration. |
| AC8–AC9 | Native Windows L1 evidence and L1 source contracts recorded in `acceptance.md` | Appropriate tier. Cross-OS evidence itself does not determine readiness. |
| AC12–AC13 duration and count reporting | L1 classifier, serialization, renderer, aggregate, and executable workflow-step tests | Appropriate and passing locally; hosted artifact round-trip remains part of the blocked probe. |
| Review 6 schema-documentation correction | L1 Rust and Python documentation contracts tied to live constants and shipped documents | Appropriate, non-vacuous, and passing. |

## Verification performed

- `just _test repo-deps` with an isolated `CARGO_TARGET_DIR`: **276 passed, 0
  skipped**.
- `just _test test-toolkit` with an isolated `CARGO_TARGET_DIR`: **183 passed,
  2 skipped**. The two skipped Nextest self-verification fixtures are recorded
  as intentional in `acceptance.md`.
- `python3 scripts/ci/test_schema.py`: **64 passed**.
- `just _lint repo-deps` and `just _lint test-toolkit`: passed.
- `actionlint .github/workflows/ci.yml .github/workflows/_package-ci.yml
  .github/workflows/_area-ci.yml .github/workflows/_wsl-ci.yml`: passed.
- `just ci-local --plan`: passed and rendered 20 package cells across
  `biscuit-tui`, `root`, and `tools`, plus two accepted L2 gaps.
- GitNexus was bound to `rusty-biscuit`; its index predates the working-tree
  changes. `detect-changes --scope all` reported 21 files and 86 symbols at
  low risk, with no affected process and no partial or truncated result.
- No full-workspace, L2, L3, push, workflow dispatch, commit, or formatting
  change was performed.
