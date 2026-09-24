---
$schema: feature-review.yaml
ready: true
human_review: true
human_review_items:
  - |-
    Confirm whether the four machine-readable migration manifests should remain the sole authoritative mapping of old test programs to their consolidated targets.

    - **A — Keep the manifests as the sole authority (recommended).** Every automated comparison reads the same mapping, avoiding a second record that can drift.
    - **B — Require an additional hand-reviewed mapping.** This adds an independent review of more than 3,800 identities, but creates a second source that must be maintained.

    Select A or B. No implementation change is needed for A.
reviewed_by: codex/gpt-5.6-sol
created: "2026-09-22T13:59:13-07:00"
spec: 2026-09-21-consolidated-test-binaries/spec.md
implemented: false
description: "A **fix** review of `2026-09-21-consolidated-test-binaries/spec.md`"
fix: 2026-09-21-consolidated-test-binaries/review-5.md
previous: 2026-09-21-consolidated-test-binaries/review-4.md
---

# Review 5

**Production-ready.** Review 4's Unicode-identifier defect is implemented and
verified at both the parser and whole-layout boundaries. I found no remaining
functional, correctness, performance, or test-rigor gap introduced by this
specification.

Review 4 had no formal `Unblocked Findings` or `Blocked Findings` sections. Its
one high-priority implementation finding is closed. No blocked implementation
finding became actionable before the last implementation: the migration-
manifest authority question remains a human design decision, and ordinary CI
producer measurements remain intentionally pending until a qualifying run.

## Previous-review disposition

The supplied previous-review path,
`prompts/_reviews/features/2026-09-21-consolidated-test-binaries/review-4.md`,
does not exist in this worktree. The canonical colocated review at
`features/2026-09-21-consolidated-test-binaries/review-4.md` was reviewed and
updated.

- **Unicode macro identifiers:** closed. The scanner now uses Rust-compatible
  XID start/continue predicates for macro names, module names, visibility
  boundaries, and raw-string prefix boundaries. Regressions cover Unicode
  macro definitions and invocations at parser and whole-layout levels.
- **Migration-manifest authority:** still requires the human decision recorded
  in frontmatter. It does not block implementation readiness.
- **Ordinary CI producer observations:** still pending by specification. AC10
  prohibits triggering a run solely to collect them and makes them observations,
  not a pass/fail gate.

## Requirement-to-verification map

This feature changes compilation and test discovery, not terminal input,
rendering, keyboard, mouse, paste, IME, or hotkey behavior. Its new behavior is
therefore correctly verified at Level 1. Existing L2/L3/browser tests retain
their original responsibility for behavior inside the moved modules.

| Requirement | Strongest verification inspected | Assessment |
|---|---|---|
| AC1–AC3 — explicit targets, identities, tiers, and feature contracts | L1 metadata reconciliation and exact recorded Nextest-list comparisons | Appropriate and passing. The 139/74/53/38 former targets map to exactly 4/5/2/2 declared targets. |
| AC4 — platform conditions and reachability | L1 attribute audits, macOS/Linux identity comparisons, native-Windows suite runs, and WSL2 archive runs | Appropriate and complete under the author's narrowed Windows criterion. |
| AC5–AC7 — structural edits, snapshots, and path guards | L1 mechanical reports, source-difference review, and snapshot/path mappings | Appropriate and passing. |
| AC8 — canonical suites and resource contracts | Existing L1/L2/browser migration evidence; unchanged L3 and real-provider gates | Appropriate. Consolidation changes discovery and process boundaries, not terminal encoding or rendering. |
| AC9 — pilot performance guardrail | Matched clean-build and edit-loop measurements | Appropriate and within the specified split threshold. |
| AC10 — producer observations | No qualifying ordinary CI run yet | Pending by design and not a readiness gate. |
| AC11 — selector and process-isolation documentation | L1 active-document sweep and direct inspection | Appropriate and passing. |
| AC12 — reject stray roots and undeclared modules | L1 shared-walker tests plus all four live package guards | Appropriate and passing, including Unicode macro definition and invocation regressions. |

## Verification performed

- Read the specification, review 4, implementation log, acceptance record,
  shared layout walker, its regressions, dependency changes, and live consumers.
- Bound GitNexus to `rusty-biscuit`. Its upstream impact result for
  `layout_violations` was `UNKNOWN`; source search confirmed the four live
  package consumers rather than treating the empty graph result as safety
  evidence.
- `tools/` `just test`: 345 passed, 2 expected tier-filter skips.
- `tools/` `just lint`: passed.
- `claudine/` `just test-cli test_placement::`: 12 passed.
- `darkmatter/` `just test test_layout::`: 2 passed.
- `biscuit-terminal/` `just test test_layout::`: 1 passed.
- `acceptance/metadata-check.py`: passed for all four packages.
- `selfproof/mutation-check.py`: all eight mutants were detected.

No L2, L3, browser, or GUI-backed test was launched in this iteration. The
review-4 implementation changes only the L1 structural scanner; this review
does not replace the earlier recorded evidence for the moved higher-level
tests.

## Non-blocking observation

`acceptance.md` still says the shared layout walker has 12 unit tests; the
current focused run lists 16. This stale count does not change the acceptance
evidence or production-readiness verdict, but should be corrected when the
acceptance record is next maintained.

## Production readiness

Ready for production. The implementation satisfies the specification and the
prior review finding, with verification at the appropriate level for every
new user-observable requirement. Human confirmation of manifest authority and
future ordinary-CI measurements remain explicit external follow-ups.
