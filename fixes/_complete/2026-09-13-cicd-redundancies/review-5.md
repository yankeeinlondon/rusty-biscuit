---
$schema: feature-review.yaml
ready: false
findings:
  - title: Result schema changes still identify themselves as version 3
    priority: medium
  - title: Required hosted workflow behavior remains unverified
    priority: high
human_review: false
reviewed_by: codex/gpt-5.6-sol
created: "2026-09-15T04:13:01-07:00"
spec: _complete/2026-09-13-cicd-redundancies/spec.md
implemented: true
implemented_by: claude/opus
log: fixes/_complete/2026-09-13-cicd-redundancies/log.md
description: "A **fix** review of `2026-09-13-cicd-redundancies/spec.md`"
fix: _complete/2026-09-13-cicd-redundancies/review-5.md
previous: _complete/2026-09-13-cicd-redundancies/review-4.md
next: _complete/2026-09-13-cicd-redundancies/review-6.md
---

# Review 5

**Not production-ready.** Review 4's unavailable-count finding is fixed and
the relevant owner suites pass. The result document changed incompatibly
without advancing its schema version, and the specification's required hosted
GitHub Actions validation remains blocked.

## Previous-review disposition

The supplied previous-review path,
`prompts/_reviews/fixes/2026-09-13-cicd-redundancies/review-4.md`, does not exist
in this worktree. The canonical colocated review at
`fixes/_complete/2026-09-13-cicd-redundancies/review-4.md` was reviewed and updated.

Review 4's `## Unblocked Findings` is resolved:

- **Unavailable Rust test counts are reported as numeric zero — closed.**
  `Cell.counts` now represents availability explicitly. Missing and unreadable
  reports and version-1 reused evidence carry no count, while an executed suite
  that selected zero tests carries a measured zero. Environment and area
  aggregates label partial and wholly unavailable measurements, and regressions
  cover each required boundary.

Review 4's `## Blocked Findings` contains one item. It did not become unblocked
before the latest implementation: no commit or push was authorized, and the
acceptance record still marks hosted Validations 5 and 7 `HOSTED-PENDING`.

## Unblocked Findings

### Medium — Result schema changes still identify themselves as version 3

The serialized `Cell` contract now permits `counts` and `duration_s` to be
absent and represents duration as a fractional number
(`scripts/ci-rollup.rs:485-504`), but `RESULT_SCHEMA_VERSION` remains 3
(`scripts/ci-rollup.rs:55-63`). The version-3 reader on `main` requires
`counts: Counts`; it therefore rejects a new missing-measurement cell before
the explicit generation guard runs. Both files claim schema version 3 even
though one cannot read the other.

This defeats the result format's stated contract that documents from another
generation are refused by their version (`scripts/ci-rollup.rs:4561-4588`) and
can break cross-run tools that consume retained area slices. Advance the result
schema to version 4, update the migration diagnostic and documentation, and add
a fixture proving that version-3 results are rejected by version before their
cell fields are interpreted. Keep the baseline schema version independent.

## Blocked Findings

### High — Required hosted workflow behavior remains unverified

The acceptance record still has no hosted evidence for specification
Validations 5 and 7 (`acceptance.md:164-251`). Local planner, workflow-source,
shell, and renderer fixtures cannot establish GitHub's matrix expansion,
nested reusable-workflow result propagation, artifact handoff, rendered check
names, or `continue-on-error` conclusions.

After commit and push are explicitly authorized, run the controlled hosted
probes from the specification and retain machine-readable job conclusions,
check names, area result-slice contents, the advisory report, and the
`ci-gate` outcome. The run must also exercise the accumulated review fixes:
exclusive reporting modes, cancellation reporting, single-process lint timing,
partial duration and count rendering, and the distinction between absent and
genuinely zero test counts. This is an integration-boundary gap, not a request
for additional cross-OS proof or human design review.

## Requirement-to-verification map

This specification changes CI orchestration and reporting rather than terminal
input or terminal rendering. L2 real-terminal and L3 OS-input tests are not
applicable.

| Requirement | Strongest verification inspected | Assessment |
|---|---|---|
| AC1–AC5, AC7, AC10, AC15 | L1 planner, schema, registry, workflow contracts, executable shell fixtures, and `actionlint` | Appropriate and passing. |
| AC6, AC11, AC12, AC14, AC16 | L1 planner, renderer, workflow-source, and shell fixtures | Hosted Actions boundary remains missing; these tests cannot prove the specified hosted orchestration. |
| AC8–AC9 | Native Windows L1 evidence and L1 source contracts recorded in `acceptance.md` | Appropriate tier. Cross-OS evidence itself does not determine readiness. |
| AC12–AC13 duration and count reporting | L1 classifier, serialization, renderer, aggregate, and executable workflow-step tests | Behavior is covered and passing, but the serialized result generation is mislabeled as version 3. |

## Verification performed

- `actionlint .github/workflows/ci.yml .github/workflows/_package-ci.yml
  .github/workflows/_area-ci.yml .github/workflows/_wsl-ci.yml`: passed.
- `just _test repo-deps` with an isolated `CARGO_TARGET_DIR`: **273 passed, 0
  skipped**.
- `just _test test-toolkit` with an isolated `CARGO_TARGET_DIR`: **183 passed,
  2 skipped**. The two skipped Nextest self-verification fixtures are recorded
  as pre-existing in `acceptance.md`.
- All ten registered compact Python CI suites passed through
  `scripts/ci/suite_runner.py`.
- `tools/test-audit` `just check`: TypeScript typecheck passed; Vitest reported
  **189 passed, 29 declared compatibility skips**.
- `python3 scripts/ci/schema.py` regenerated the checked-in plan contract
  byte-identically.
- `just _lint repo-deps` and `just _lint test-toolkit`, each with an isolated
  `CARGO_TARGET_DIR`: passed.
- `just ci-local --plan`: passed and rendered 20 planned cells across
  `biscuit-tui`, `root`, and `tools`, plus two accepted L2 gaps.
- GitNexus was bound to `rusty-biscuit`. Its committed index matches `HEAD` but
  is stale for the working tree. Indexed upstream impact for
  `render_environment_report` is low: `render_report` → `cmd_summarize` →
  `run`, with no indexed execution process.
- No full-workspace, L2, L3, push, workflow dispatch, ruleset mutation, or
  commit was performed.
