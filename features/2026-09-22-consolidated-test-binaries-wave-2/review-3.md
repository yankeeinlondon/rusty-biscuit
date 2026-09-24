---
$schema: feature-review.yaml
ready: false
findings:
  - title: The test-input scanner still documents the retired helper scope
    priority: low
human_review: false
reviewed_by: codex/gpt-6-sol
created: "2026-09-23T17:06:40-07:00"
spec: 2026-09-22-consolidated-test-binaries-wave-2/spec.md
implemented: true
implemented_by: claude/opus
log: features/2026-09-22-consolidated-test-binaries-wave-2/implementation-log.md
description: "A **fix** review of `2026-09-22-consolidated-test-binaries-wave-2/spec.md`"
fix: 2026-09-22-consolidated-test-binaries-wave-2/review-3.md
previous: 2026-09-22-consolidated-test-binaries-wave-2/review-2.md
next: 2026-09-22-consolidated-test-binaries-wave-2/review-4.md
---

# Review 3

**Not production-ready.** Both review-2 findings are fixed and the migration checks pass. Active scanner comments still state the old CI narrowing contract, so the inherited requirement that active documentation be accurate is not yet met.

## Review-2 findings

| Finding | Result |
|---|---|
| CI input narrowing drops Level 1 callers of a shared helper | Fixed. A helper reference now selects its whole test binary when that binary contains an L1 test. The planner regression changes `docs/guide.md`, schedules one Linux L1 cell, and proves the narrowed filter selects the sibling `captured::uses_guide` caller. All 41 focused index and selection tests pass. |
| The acceptance record still describes a failed local suite | Fixed. Its summary and section 8 now distinguish the failed sweep from the successful Claudine rerun, without the damaged sentence or orphaned text. |

Review 2 has no **Blocked Findings** section or deferred finding to reassess.

## Findings

### Low — The test-input scanner still documents the retired helper scope

The module header in `scripts/ci/test_inputs.py:23-25` says that a reference in a helper names its module. The review-2 fix correctly changed `_unit` to name the entire binary because a caller may live in another module. The `Reference` docstring at `:152-156` also says `unit` is `None` exactly when `product` is true, although `_unit` returns `None` for non-L1 tests and targets whose features CI does not enable, and `scan` clears binary-wide units when the binary has no L1 test. These comments now contradict the working implementation and the updated `docs/cicd/test-inputs.md`. Correct the two comments; the code and tests do not need another behavioral change. This is an active documentation gap against inherited acceptance criterion 11.

## Requirement-to-verification map

The spec changes how test programs are built and selected; it introduces no new terminal input or rendering behavior. The migrated tests retain their original assertions and verification levels.

| Requirement | Strongest verification | Assessment |
|---|---|---|
| AC1–3: target declarations, identities, tiers, and feature contracts | Level 1 Cargo metadata reconciliation and exact Nextest listing comparisons | Pass. All ten manifests reconcile with Cargo: 136 old targets map to 18 declared targets. All 30 recorded macOS/Linux comparisons are identical with zero failures. |
| AC4: platform conditions and Windows-only test reachability | Level 1 attribute checks and recorded native-Windows/WSL2 suite runs | Appropriate migration evidence. The named Windows-only modules ran. The two recorded Claudine library failures are outside the moved test sources; cross-OS proof does not determine this review's readiness. |
| AC5–7: bodies, snapshots, and path-sensitive guards | Level 1 body and snapshot checks, guard scans, and local suite records | Pass in the recorded evidence: all ten body diffs report zero nonstructural lines. |
| AC8: canonical local recipes | Level 1 `just test` and Level 2 `just test-l2` results in the acceptance record; review 2 reran Claudine L1 successfully | Pass. No migrated test was renamed or added in the review-2 fix. |
| Existing badge colors, terminal rendering, and scrolling retained by the move | Level 2 real-terminal pane captures, including the badge SGR and image-scroll tests | Appropriate for visible terminal output. Recorded Level 2 runs pass; the migration's identity and body checks preserve these tests. |
| Existing physical hotkey chord behavior retained by the move | Level 3 OS-keyboard-injection tests in the declared `biscuit-tui-cli::level3` target and live `test-l3` recipe | Appropriate test level. Four `level3_chord_select` tests remain selected; they were not rerun in this review because they require an opt-in foreground window. The migration makes no new hotkey behavior claim. |
| AC9–10: target/byte measurements and producer observations | Level 1 matched measurements; pending ordinary CI producer table | The recorded executable bytes fall 68.7%. CI producer observations remain pending by the spec's stated procedure, with no extra CI run requested. |
| AC11: active selector and documentation updates | Level 1 selector checks and direct reading of active docs | **Gap:** the scanner comments above describe the old helper rule. |
| AC12 and new AC4–6: layout guard, hazards, workspace count, and tier reachability | Level 1 mutation checks, metadata, and `check-tier-coverage` | Pass. The eight affected areas report zero stranded tests, and the acceptance record reports no package with ten or more integration-test targets. |

## Verification performed

- Ran the 41 focused `TestInputIndexTests` and `TestInputSelectionTests`: pass.
- Ran all `scripts/ci/test_*.py` suites: 1,200 tests, pass.
- Reran `consolidation.py check-metadata` against all ten migration manifests: pass, 18 declared targets.
- Reran `just check-tier-coverage` for all eight affected areas: zero stranded tests.
- Parsed all 30 recorded comparison reports: every verdict is `identical`, with an empty failures list.
- Checked `git diff --check`: pass. No Level 3 test was launched or terminal window focused.

No human decision is needed. The remaining fix is documentation only; the spec stays at implementation complete, ready for another review after that correction.
