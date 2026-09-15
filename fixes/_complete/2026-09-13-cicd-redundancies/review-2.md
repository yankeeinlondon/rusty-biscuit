---
$schema: feature-review.yaml
ready: false
findings:
  - title: The advisory reuse report makes a run-level CLEAR claim
    priority: high
  - title: The reporting modes overlap and cancellations are misclassified
    priority: high
  - title: Required hosted workflow behavior remains unverified
    priority: high
  - title: Sub-second lint measurements are reported as unavailable
    priority: medium
human_review: false
reviewed_by: codex/gpt-5.6-sol
created: "2026-09-14T19:06:25-07:00"
spec: _complete/2026-09-13-cicd-redundancies/spec.md
implemented: true
implemented_by: claude/opus
log: fixes/_complete/2026-09-13-cicd-redundancies/log.md
description: "A **fix** review of `2026-09-13-cicd-redundancies/spec.md`"
fix: _complete/2026-09-13-cicd-redundancies/review-2.md
previous: _complete/2026-09-13-cicd-redundancies/review-1.md
next: _complete/2026-09-13-cicd-redundancies/review-3.md
---

# Review 2

**Not production-ready.** The implementation lands the main ownership and
scheduling architecture, and its focused L1 suites are strong. The new
`ci-reporting` path nevertheless violates its advisory contract in two visible
ways, loses a real class of lint-duration measurements, and has not been
exercised at the hosted GitHub boundary that defines several of the
specification's user-observable requirements.

This review changes only this review artifact and the requested
`review_iterations` metadata. Missing cross-OS proof and human approval do not
determine `ready`.

## Previous-review disposition

The supplied previous-review path,
`prompts/_reviews/fixes/2026-09-13-cicd-redundancies/review-1.md`, does not
exist in this worktree, any sibling rusty-biscuit worktree searched, or this
branch's Git history. No other `review-1.md` exists for this fix. Consequently,
there is no `## Unblocked Findings` or `## Blocked Findings` list that can be
truthfully checked or updated, and the requested `next` / `implemented`
frontmatter edit could not be made without inventing a missing historical
document.

The revised specification does preserve three explicit review corrections,
and all three are reflected in the implementation:

- the cleanup specification is a direct dependency rather than a loose
  relationship;
- the Windows captured-stdout test is ordinary Windows-only L1 evidence, not
  L2;
- change inventory and report measurements are recognized as new schema data,
  rather than incorrectly assumed to exist already.

This is evidence for those recorded corrections, not a substitute for the
missing previous review's finding disposition.

## Findings

### High — The advisory reuse report makes a run-level CLEAR claim

The specification says `ci-reporting` applies no merge policy and must not
claim that a pull request is mergeable independently of `ci-gate`
(`spec.md:292-296`). The reused-validation branch instead begins its report
with `**CLEAR**` and explains why the gate is green
(`.github/workflows/ci.yml:608-622`). That is a run-level green verdict emitted
by the job whose defining contract is to report evidence without judging it.

The existing workflow contract at
`tools/test-toolkit/tests/ci_workflow_contracts.rs:2299-2319` only bans the old
literal `No gate reported a failure`; it does not reject equivalent verdict
language. The `ci-rollup` report test correctly bans `CLEAR —`, but it exercises
only the Rust renderer, not the reuse-mode shell in `ci.yml`.

Make reuse mode state only the neutral facts required by section 6: this run
executed no package cell, and the linked prior run is authoritative. Remove
`CLEAR` and any conclusion derived from the folded job results. Extend the
workflow contract to reject the same policy/verdict vocabulary already banned
from the Rust advisory renderer.

### High — The reporting modes overlap and cancellations are misclassified

`ci-reporting` claims that exactly one of its three modes speaks
(`.github/workflows/ci.yml:566-569`). Mode 2 runs when scope succeeds, but the
mode-3 step is guarded only by `reuse != 'true'` (`:623-675`), so every normal
successful scoped run renders both the full report and a second “Jobs outside
the rollup / No bootstrap stage failed” section. The modes are not exclusive.

The same step promises a “failed or cancelled bootstrap” report, but its loop
matches only the literal `failure` (`:682-703`). If validation or scope is
`cancelled`, it prints `No bootstrap stage failed`, directly contradicting the
job state and specification section 6.3 (`spec.md:274-276`). The source-only
test at `ci_workflow_contracts.rs:993-1015` checks that labels exist; it neither
executes the branch nor tests success, failure, and cancellation outcomes.

Guard mode 3 on the absence of successful scope (while preserving the reuse
branch), treat both `failure` and `cancelled` as actionable, and add an
executable shell fixture covering all three mutually exclusive modes. A
successful scope must emit mode 2 only; cancelled validation/scope must name
that state rather than claim no bootstrap failure.

### High — Required hosted workflow behavior remains unverified

The acceptance record still marks Validation 5 and Validation 7
`HOSTED-PENDING` (`acceptance.md:28-30`, `:169-179`): no run of the shipped
nested workflows proves package-owned tooling failures reach `area-ci` and
`ci-gate`, `ci-reporting` failure stays advisory, zero matrices resolve with
the intended labels, area artifacts aggregate correctly, or a follow-up
documentation change preserves and reports reused cells. These are GitHub
Actions behaviors, not Rust/Python calculations.

The L1 planner, workflow-source, and shell-fragment tests are valuable and all
pass, but they manufacture inputs or parse YAML text. They cannot establish
GitHub's matrix expansion, reusable-workflow result propagation, artifact
availability across nested jobs, Checks-tab labels, or `continue-on-error`
conclusions. This is the wrong strongest boundary for AC6, AC11, AC12, AC14,
and AC16. Terminal L2/L3 would not help; the required integration boundary is
a controlled hosted Actions run.

Complete the specification's hosted Validation 5 and 7 probes and retain a
machine-readable record of job conclusions, check names, result-slice
contents, and `ci-gate` outcome. Native Windows/JUnit proof is cross-OS evidence
and does not determine readiness here; this finding concerns the workflow
orchestration product itself.

### Medium — Sub-second lint measurements are reported as unavailable

The lint step records elapsed time with Bash's integer `SECONDS`
(`_package-ci.yml:663-672`). A valid command completing in less than one second
therefore writes `duration_s=0`. Producer status preserves that zero
(`:729-744`), but `ci-rollup` stores duration as `u64` and renders every zero as
`not recorded (the producer recorded no command duration)`
(`scripts/ci-rollup.rs:740-768`, `:2343-2345`, `:4311-4320`). A measured fast
command is thus indistinguishable from a command that never ran.

This violates the distinction in section 6 between an available command
duration and an unavailable measurement with a reason (`spec.md:282-290`). It
also makes the code comment that producer durations are “never `0`” false.

Record fractional elapsed time from a monotonic clock, or carry duration as an
optional value through the result cell so presence is independent of numeric
value. Add a regression with a recorded zero/sub-second duration and a separate
missing-duration case; the former must render a measured duration and the
latter `not recorded`.

## Requirement-to-verification map

L1 below includes in-process tests and hermetic subprocess/shell fixtures. This
fix introduces no terminal-emulator input or OS keyboard/mouse contract, so L2
and L3 are inapplicable. GitHub-hosted integration is a separate boundary and
is the only appropriate proof for Actions graph and artifact behavior.

| Requirement | Strongest verification inspected | Assessment |
|---|---|---|
| AC1–AC3: one owner/scheduler, workspace migration, gating promotion | L1 planner, metadata, and workflow contracts | Appropriate and passing. |
| AC4–AC5: complete suite registry, triggers, outcomes | L1 registry, runner subprocess, schema, and rollup tests | Appropriate and passing; per-suite failures are independently recorded. |
| AC6: documentation follow-up reuse | L1 planner/rollup fixtures | Logic passes; hosted follow-up remains unverified. |
| AC7: retired jobs/workflow/recipe | L1 workflow and source-corpus contracts | Appropriate and passing. |
| AC8–AC9: Windows console test is normal bounded L1 | Native Windows L1 evidence and negative probe recorded in `acceptance.md`; L1 source contracts | Appropriate tier. Cross-OS rerun is external evidence and does not determine this verdict. |
| AC10: plan inventory/schema/renderers | L1 Python schema/planner and Rust terminal/Markdown renderer tests | Appropriate and passing. |
| AC11: documentation-only local/CI report and zero fan-out | L1 local-plan and report-renderer fixtures | Content logic passes; actual GitHub zero-matrix/report behavior is hosted-pending. |
| AC12: consolidated report fields and origins | L1 Rust renderer and workflow-source contracts | Renderer passes; reuse-mode verdict and hosted aggregation gaps remain. |
| AC13: unavailable measurements are not fabricated | L1 Rust/Python tests | Companion cases pass; lint presence is incorrectly inferred from `duration_s > 0`. |
| AC14: prompt honest zero-matrix resolution | L1 planner/YAML contracts | Static shape passes; GitHub presentation/timing remains hosted-pending. |
| AC15: policy-free `ci-gate` fold | L1 executable shell fixture plus workflow contracts | Appropriate and passing for success/skipped/failure/cancelled. |
| AC16: all-reused area still emits a slice | L1 planner/rollup/workflow-source fixtures | Logic passes; nested hosted artifact handoff remains unverified. |

## Verification performed

- Ran all ten registered Python CI suites through their canonical counts
  wrapper: **562 passed**.
- Ran `just _test repo-deps` in an isolated Cargo target: **257 passed**.
- Ran `just _test test-toolkit` in an isolated Cargo target: **179 passed, 2
  pre-existing ignored fixtures skipped**.
- Ran `tools/test-audit` typecheck and Vitest: **189 passed, 29 pre-existing
  compatibility fixtures skipped**.
- Ran `actionlint`, regenerated the schema contract byte-identically, and ran
  `just ci-local --plan`; all succeeded.
- The first `repo-deps` attempt encountered pre-existing non-writable files in
  the shared `target/debug/deps`; the isolated-target rerun passed without
  deleting or changing the user's cache.
- No full-workspace suite, L2/L3 suite, push, workflow dispatch, ruleset write,
  or commit was performed.

`just gitnexus` confirmed the local index matches `d14beb34a`, but the
GitNexus CLI registry did not expose this worktree to `detect-changes` even by
absolute path. That tool failure was not treated as low-risk evidence; the
review used the complete `main...HEAD` diff, direct caller/reader searches, and
the focused executable suites above.
