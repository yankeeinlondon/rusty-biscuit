---
$schema: feature-review.yaml
ready: true
human_review: false
reviewed_by: codex/gpt-6-sol
created: "2026-09-23T17:23:48-07:00"
spec: 2026-09-22-consolidated-test-binaries-wave-2/spec.md
implemented: false
description: "A **fix** review of `2026-09-22-consolidated-test-binaries-wave-2/spec.md`"
fix: 2026-09-22-consolidated-test-binaries-wave-2/review-4.md
previous: 2026-09-22-consolidated-test-binaries-wave-2/review-3.md
---

# Review 4

**Production-ready.** The sole finding in review 3 is fixed, and this review
found no remaining implementation or test-rigor gap introduced by the wave-2
migration. The prior review has no `Unblocked Findings` or `Blocked Findings`
section and no deferred finding to reassess.

## Previous-review disposition

The supplied `prompts/_reviews/features/2026-09-22-consolidated-test-binaries-wave-2/review-3.md`
path does not exist in this worktree. The colocated
`features/2026-09-22-consolidated-test-binaries-wave-2/review-3.md` is the
previous review and has been linked to this one.

- **Stale test-input scanner comments:** fixed. The module header now says a
  helper reference selects its whole binary. The `Reference` docstring names
  the conditions under which `unit` is absent. Both agree with `_unit`,
  `_has_l1_test`, and `docs/cicd/test-inputs.md`; the edit changes no scanner
  behavior. The focused index and planner regressions pass.
- **Blocked findings:** none. The pending CI producer observations are the
  spec's criterion 10 follow-up, collected from the first ordinary run; they
  are observations rather than a pass/fail gate.

## Requirement-to-verification map

This spec changes test compilation and selection, without adding terminal
input, rendering, keyboard, mouse, paste, IME, or hotkey behavior. Its new
contracts have Level 1 verification. The migrated tests retain their original
behavioral levels; a test's presence in a declared target and a live tier
recipe were checked separately.

| Requirement | Strongest verification inspected | Assessment |
|---|---|---|
| Inherited AC1–3: explicit targets, normalized test identities, tier and feature contracts | Level 1 Cargo metadata check and 30 exact macOS/Linux Nextest-list comparisons | Pass. Ten packages declare 18 test targets in total; all comparison verdicts are `identical`, with no recorded failures. The former zero-test `sniff::fixtures` target is explicitly disposed of as a helper. |
| Inherited AC4: platform conditions and Windows-only reachability | Level 1 attribute checks and recorded native-Windows/WSL2 suite runs | Appropriate migration evidence. The recorded Windows runs include the named `sniff` and `biscuit-tui-cli` Windows-only tests. The two recorded Claudine library failures are outside the moved test sources and are not a cross-OS readiness gate for this review. |
| Inherited AC5–7: unchanged test behavior, snapshots, seeds, and path guards | Level 1 body-diff, snapshot, proptest, and enumerated path-guard reports | Pass in the recorded evidence. All ten body reports have zero nonstructural changes; the attribute and proptest reports have no failures. |
| Inherited AC8: canonical local recipes | Recorded Level 1 and Level 2 area runs; Claudine Level 1 rerun from review 2 | Pass. Review 2 recorded 7,320 passing Claudine tests after the prompt-fixture repair. No Rust test body changed in the review-3 fix. |
| Existing terminal output, colors, widths, and scrolling retained by the move | Level 2 real-terminal pane-capture tests in declared targets and live `test-l2` recipes | Appropriate for rendered output. The migration preserved their identities, tiers, assertions, and recorded Level 2 runs. |
| Existing physical chord activation retained by the move | Level 3 OS-keyboard-injection tests in the declared `biscuit-tui-cli::level3` target and live `test-l3` recipe | Appropriate for terminal input encoding. Four chord tests remain selected. Level 3 is opt-in and was not run in this review, which makes no new chord-behavior claim. |
| Replaced AC9 and inherited AC10: target and executable-size measurements; producer observations | Matched local measurements; pending ordinary-CI observation record | The recorded test executables fell from 136 to 18 and their measured bytes by 68.7%. No package met the edit-latency trigger. Producer observations remain pending by the criterion's procedure. |
| Inherited AC11–12 and new AC4–6: active selectors, layout guard, hazards, workspace count, and no stranded tests | Level 1 selector and layout mutation checks, metadata, exact comparison reports, and current tier-coverage check | Pass. Each package has a declared and selected Level 1 layout guard; all eight affected areas have zero stranded tests. The acceptance record shows 118 workspace integration-test targets, with no package at ten or more. |

## Verification performed

- Checked the review-3 finding against the scanner's active comments and
  implementation. Ran its 41 focused `TestInputIndexTests` and
  `TestInputSelectionTests`: all passed.
- Ran `consolidation.py check-metadata` against all ten migration manifests:
  pass, 18 declared targets.
- Parsed all 30 recorded comparison JSON files: every verdict is `identical`
  and every failure list is empty. Read the attribute, proptest, and snapshot
  reports; none records a failure.
- Ran `just check-tier-coverage` for the eight affected areas: zero stranded
  tests. Inspected the declared test roots, layout guards, and live Level 2
  and Level 3 recipes.
- Checked `git diff --check`: pass. The review-3 implementation changed only
  scanner comments, so this iteration did not rerun the large Rust suites or
  focus a terminal window.

No human design decision or manual test is required for this wave. Criterion
10's producer measurements remain an explicit follow-up after an ordinary CI
run selects these packages.
