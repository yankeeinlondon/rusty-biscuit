---
$schema: feature-review.yaml
ready: false
findings:
  - title: CI input narrowing drops Level 1 callers of a shared helper
    priority: high
  - title: The acceptance record still describes a failed local suite
    priority: low
human_review: false
reviewed_by: codex/gpt-6-sol
created: "2026-09-23T16:46:00-07:00"
spec: 2026-09-22-consolidated-test-binaries-wave-2/spec.md
implemented: true
next: 2026-09-22-consolidated-test-binaries-wave-2/review-3.md
implemented_by: claude/opus
log: features/2026-09-22-consolidated-test-binaries-wave-2/implementation-log.md
description: "A **fix** review of `2026-09-22-consolidated-test-binaries-wave-2/spec.md`"
fix: 2026-09-22-consolidated-test-binaries-wave-2/review-2.md
previous: 2026-09-22-consolidated-test-binaries-wave-2/review-1.md
---

# Review 2

**Not production-ready.** The three findings from review 1 were addressed, but the new test-input reachability check can omit a Level 1 test that reads a changed repository file through a shared helper. The prior review has no separate blocked-findings section or deferred finding to reassess.

## Prior findings

| Review 1 finding | Result |
|---|---|
| Claudine Level 1 area gate | Fixed. I reran `just test` in `claudine/`: 7,320 passed, 9 skipped, exit 0. The prompt fixture and hash pin now match the shipped prompt. |
| Binary-name shortcut in CI input narrowing | Removed. `_unit` now checks the test path and the target's CI-enabled features. The 38 targeted Python index and selection tests pass. The new reachability defect below remains. |
| Wrong tier descriptions in two crate roots | Fixed. Both `terminal-tests` crate roots now identify their Level 1 modules and explain path-based tier selection. |

## Findings

### High — CI input narrowing drops Level 1 callers of a shared helper

`scripts/ci/test_inputs.py:751-757` decides that a module-wide file reference is unreachable from Level 1 when every test *inside that module* has a non-Level-1 name. It does not account for a Level 1 test in another module calling the helper. The new pruning then changes the reference's `unit` to `None` at `:710-713`, and `select_test_inputs` ignores it at `scripts/ci/affected_scope.py:4394-4397`. A change to the referenced file schedules no test cell for that package.

I reproduced this with a consolidated `level2` binary containing `render::guide()`, which reads `repo_root().join("docs/guide.md")`, a `render::level2_uses_guide` test, and a sibling Level 1 `captured::uses_guide` test that calls `render::guide()`. `scan` reports the guide reference with `product=False, unit=None`. The Level 1 caller is compiled and selected by the ordinary L1 tier, but a guide-only change would not run it. The new regression tests cover helpers with only local Level 2 callers and helpers with no tests; neither covers a helper with a Level 1 caller outside its module.

Keep the static index conservative when callers are unknown. Preserve a package or binary Level 1 unit for shared helpers, or establish caller reachability before discarding one. Add a planner regression in which changing the guide schedules the sibling Level 1 test, and verify the resulting filter selects at least one test. This is a CI-scope correctness gap; it does not require another CI matrix cell.

### Low — The acceptance record still describes a failed local suite

`acceptance.md:230-253` has a damaged sentence: the old “one failure is the baseline's” text runs into the new “baseline's two failures are fixed” text, and `er the sweep.` remains after the passing result. Its opening summary still says local-suite failures remain, although the Claudine Level 1 gate now passes. Repair the record so criterion 8 describes the old failed sweep and the successful rerun without contradictory status text.

## Requirement-to-verification map

This feature consolidates test binaries; it does not add terminal key, paste, mouse, IME, rendering, or scrolling behavior. Existing Level 2 rendering tests and Level 3 OS-keyboard tests retain their purpose and declarations. Moving those tests creates no new user-facing behavior to prove at a different level.

| Requirement | Strongest verification reviewed | Result |
|---|---|---|
| AC1–3: declared targets, identities, tiers, and feature contracts | Level 1 Cargo metadata and exact Nextest listing comparisons on macOS and Linux, recorded for all ten packages | Pass: 136 old targets map to 18 declared targets, with 30 identical comparisons. |
| AC4: platform conditions and Windows-only reachability | Level 1 attribute checks and recorded native-Windows and WSL2 runs | Appropriate migration evidence; the named Windows-only `sniff` and `biscuit-tui-cli` tests ran. Two unrelated Claudine library tests failed on native Windows, as recorded in acceptance. Cross-OS results are not a readiness gate here. |
| AC5–7: bodies, snapshots, path guards | Level 1 body diffs, snapshot mappings, and guard checks recorded in acceptance | Pass in the reviewed evidence; no nonstructural body changes were reported. |
| AC8: canonical local suites | Level 1 `just test` rerun for Claudine; recorded Level 1, Level 2, and lint runs for the other seven areas | Claudine now passes 7,320 tests. Existing Level 2 tests run in real terminal or multiplexer harnesses without taking focus. Level 3 is opt-in and was not rerun; no new physical-key behavior is claimed. |
| AC9–10: target/byte measurements and producer observations | Level 1 matched measurements; producer observation table | The measured target and executable reductions are recorded. Ordinary producer observations remain pending by design and are not a reason for this readiness verdict. |
| AC11–12 and new AC4–6: selectors, layout, hazards, workspace count, and no stranded tests | Level 1 selector checks, in-binary layout guards, metadata, mutation checks, and eight area tier-coverage checks recorded in review 1 | Existing migration checks pass, but the review-1 CI-input fix introduced the shared-helper omission above. |

## Verification performed

- Read the wave-2 and inherited wave-1 specifications, review 1, acceptance record, implementation log, and changed code and tests.
- Ran the 38 `TestInputIndexTests` and `TestInputSelectionTests`: all passed.
- Ran `just test` in `claudine/`: 7,320 passed, 9 skipped, exit 0.
- Ran an isolated scanner reproduction of a shared helper with a sibling Level 1 caller: the reference has no narrowed unit.
- Checked `git diff --check`: no whitespace errors.

No human decision is needed for these findings; both have direct fixes. The spec remains at implementation complete, ready for another review after the CI-scope issue is corrected.
