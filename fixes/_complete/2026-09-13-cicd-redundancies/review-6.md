---
$schema: feature-review.yaml
ready: false
findings:
  - title: Schema reference still reports the baseline as version 2
    priority: medium
  - title: Required hosted workflow behavior remains unverified
    priority: high
human_review: false
reviewed_by: codex/gpt-5.6-sol
created: "2026-09-15T04:36:54-07:00"
spec: _complete/2026-09-13-cicd-redundancies/spec.md
implemented: true
implemented_by: claude/opus
log: fixes/_complete/2026-09-13-cicd-redundancies/log.md
description: "A **fix** review of `2026-09-13-cicd-redundancies/spec.md`"
fix: _complete/2026-09-13-cicd-redundancies/review-6.md
previous: _complete/2026-09-13-cicd-redundancies/review-5.md
next: _complete/2026-09-13-cicd-redundancies/review-7.md
---

# Review 6

**Not production-ready.** Review 5's result-schema finding is fixed: result
documents now identify as version 4, both read paths reject another generation
before deserializing its cells, and a non-vacuous regression proves that
ordering. The required hosted GitHub Actions validation remains blocked, and
one schema reference still contradicts the shipped baseline version.

## Previous-review disposition

The supplied previous-review path,
`prompts/_reviews/fixes/2026-09-13-cicd-redundancies/review-5.md`, does not exist
in this worktree. The canonical colocated review at
`fixes/_complete/2026-09-13-cicd-redundancies/review-5.md` was reviewed and updated.

Review 5's `## Unblocked Findings` is resolved:

- **Result schema changes still identify themselves as version 3 — closed.**
  `RESULT_SCHEMA_VERSION` is now 4. `parse_rollup` first deserializes only the
  version, rejects incompatible generations, and only then deserializes the
  complete result. The regression fixture contains a deliberately invalid
  version-3 cell and proves the command returns the version-4 migration rather
  than the otherwise-triggered Serde type error. The result and baseline
  version-independence test continues to pin result 4 against baseline 3.

Review 5's `## Blocked Findings` did not become unblocked before the latest
implementation. The changes remain uncommitted, no push was authorized, and
the acceptance record still marks specification Validations 5 and 7
`HOSTED-PENDING` with no machine-readable hosted run record.

## Unblocked Findings

### Medium — Schema reference still reports the baseline as version 2

The result-schema paragraph in `.github/ci/schemas/README.md:34-37` now
correctly reports result schema version 4, but the same sentence says the
baseline is version 2. The shipped `.github/ci/ci-baseline.toml:40`,
`BASELINE_SCHEMA_VERSION` in `scripts/ci-rollup.rs:68-71`, the schema
independence regression, and every other updated CI reference identify the
baseline as version 3. The implementation log also claims this document was
updated with the baseline stated separately as 3.

Correct the schema reference to baseline version 3 and add or extend the
documentation contract so this live version inventory cannot drift from the
Rust constants and shipped documents again. This is documentation drift in the
schema authority updated by this iteration, not a historical record.

## Blocked Findings

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
| Result schema migration introduced by review 5 | L1 version-probe, incompatible-cell, migration-diagnostic, and independent-version tests | Appropriate and passing. The live schema reference still misstates the baseline version. |

## Verification performed

- `just _test repo-deps` with an isolated `CARGO_TARGET_DIR`: **275 passed, 0
  skipped**.
- `just _test test-toolkit` with an isolated `CARGO_TARGET_DIR`: **183 passed,
  2 skipped**. The two skipped Nextest self-verification fixtures are recorded
  as intentional in `acceptance.md`.
- `just _lint repo-deps` and `just _lint test-toolkit`, each with an isolated
  `CARGO_TARGET_DIR`: passed.
- `actionlint .github/workflows/ci.yml .github/workflows/_package-ci.yml
  .github/workflows/_area-ci.yml .github/workflows/_wsl-ci.yml`: passed.
- `just ci-local --plan`: passed and rendered 20 package cells across
  `biscuit-tui`, `root`, and `tools`, plus two accepted L2 gaps.
- GitNexus was bound to `rusty-biscuit`; its committed index predates the
  working-tree changes. Indexed upstream impact for `reject_old_schema` and
  `load_rollup` is low, with no affected execution process. Its
  `RESULT_SCHEMA_VERSION` result was `UNKNOWN`, so text search confirmed every
  use in result construction, validation, and tests. `detect-changes --scope
  all` reported 20 indexed files and 73 symbols at low risk, with no affected
  process and no partial or truncated result.
- No full-workspace, L2, L3, push, workflow dispatch, ruleset mutation, commit,
  or formatting command was performed.
