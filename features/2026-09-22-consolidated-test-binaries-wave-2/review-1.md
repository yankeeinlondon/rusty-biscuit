---
$schema: feature-review.yaml
ready: false
findings:
  - title: The Claudine Level 1 area gate still fails
    priority: high
  - title: CI input narrowing assumes a level2 binary contains no Level 1 tests
    priority: medium
  - title: Two consolidated crate roots describe the wrong test tier
    priority: low
human_review: false
reviewed_by: codex/gpt-6-sol
created: "2026-09-23T16:13:34-07:00"
spec: 2026-09-22-consolidated-test-binaries-wave-2/spec.md
implemented: true
next: 2026-09-22-consolidated-test-binaries-wave-2/review-2.md
implemented_by: claude/opus
log: features/2026-09-22-consolidated-test-binaries-wave-2/implementation-log.md
description: "A **fix** review of `2026-09-22-consolidated-test-binaries-wave-2/spec.md`"
fix: 2026-09-22-consolidated-test-binaries-wave-2/review-1.md
---

# Review 1

**Not production-ready against the written acceptance criteria.** The migration's target, feature, identity, and tier checks are strong and pass. The Claudine area's required Level 1 recipe remains red. Its failures predate this migration and occur in `claudine-cli`, outside the ten packages, but acceptance criterion 8 says the recipe passes *in every migrated package area*. The acceptance record calls that criterion met despite recording the failure.

## Findings

### High — The Claudine Level 1 area gate still fails

The inherited acceptance criterion 8 requires `just test` to pass in every migrated package area (`2026-09-21-consolidated-test-binaries/spec.md`, criterion 8). `acceptance.md:215` records two Claudine failures but marks the section met. I reran `just test claudine` on this tree: 7,631 passed, 2 failed, 11 skipped; exit 100. Both failures are `claudine-cli::l1 shipped_prompt_route_drift::{fixture_body_matches_the_shipped_body, shipped_implement_prompts_have_not_drifted_from_their_fixture}`. The acceptance record attributes them to a changed shipped prompt whose fixture and hash pin were not refreshed (`acceptance.md:230-242`). No migrated `claudine` or `claudine-gen` test failed.

The migration did not cause these failures, but a failing required recipe cannot be recorded as a pass. Refresh the prompt fixture and hash pin in the owning change, then rerun the Claudine area gate. If baseline parity was the intended criterion, amend the spec and acceptance record explicitly before calling criterion 8 met. This finding concerns local acceptance, not a demand for new cross-OS proof.

### Medium — CI input narrowing assumes a level2 binary contains no Level 1 tests

`scripts/ci/test_inputs.py:251-265` returns no narrowed L1 unit for every test binary whose name starts with `level2`. This migration puts `windows_captured_stdout`, an ordinary L1 test, in `biscuit-tui-cli::level2` (`biscuit-tui/cli/tests/level2/main.rs:16-17`). It also keeps 49 unmarked L1 tests in `biscuit-terminal-cli::level2` under neutral `prose_cells` and `diagrams` aliases (`biscuit-terminal/cli/tests/level2/main.rs:12-22`). The binary-name premise is therefore false in live targets. A tracked repository input read added to one of these L1 tests would silently get no narrowed CI cell.

The committed test-input probes show no existing tracked non-source read lost by this move, so this is a forward correctness risk, not a demonstrated missed cell today. Remove the binary-name shortcut and decide L1 reachability from test paths or actual Nextest listings; add a mixed-tier binary regression. The package manifest's `required-features` must still be honored.

### Low — Two consolidated crate roots describe the wrong test tier

`biscuit-tui/cli/tests/level2/main.rs:1-5` calls all its modules Level 2 and says each kept its old name, although it declares the L1 `windows_captured_stdout` module and aliases `real_terminal_render` to `terminal_render`. `biscuit-terminal/cli/tests/level2/main.rs:1-5` similarly calls all modules Level 2, although `prose_cells` and `diagrams` contain 49 L1 tests and are aliases. The code and tier comparisons agree; the newly added comments drifted. Describe the binary's feature contract and say that tier selection follows the test path.

## Requirement-to-verification map

This feature changes test compilation and discovery, not terminal input or rendering behavior. Its new requirements need Level 1 metadata, listing, and source-layout proof. Existing L2 rendering and L3 physical-keyboard tests keep their original behavioral roles; moving them does not substitute a lower verification level. No browser, mouse, paste, or IME behavior is introduced.

| Requirement | Strongest verification inspected | Result |
|---|---|---|
| AC1–3: declared targets, exact identities, tier and feature contracts | Level 1 Cargo metadata reconciliation and exact Nextest-list comparisons for every supported feature set on macOS and Linux | Pass: 136 old targets map to 18 declared targets; all 30 comparison files say `identical` with zero failures. |
| AC4: platform conditions and reachability | Level 1 attribute checks and recorded native-Windows and WSL2 suite runs, including named Windows-only tests | Appropriate evidence. The two native-Windows `claudine` lib failures recorded in acceptance are outside the moved tests and do not determine this readiness verdict. |
| AC5–7: structural bodies, snapshots, and path-based guards | Level 1 body-diff, snapshot/proptest checks, enumerated guard scans, and `INSTA_UPDATE=no` suite records | Pass in the inspected evidence; no nonstructural body edit is reported. |
| AC8: canonical suites and existing user-facing tests | Level 1 `just test` and Level 2 real-terminal suite records; existing Level 3 OS-keyboard tests remain declared under `terminal-tests` and the live `test-l3` recipe | **Fail:** Claudine's Level 1 area recipe is red. Level 2 is recorded green; Level 3 remains opt-in and was not launched because it takes desktop focus. The migration makes no new bare-modifier or hotkey claim. |
| AC9–10: performance and producer observations | Level 1 matched target/byte measurements; ordinary producer observations pending | Target executables fell from 136 to 18 and measured bytes fell 68.7%. No package met the spec's edit-latency trigger. AC10 is deliberately pending until an ordinary CI run and is not a readiness gate. |
| AC11: active selectors and documentation | Level 1 consumer sweep and direct recipe/override inspection | Exact Nextest overrides now match one test each. New crate-root tier comments need correction as above. |
| AC12 and new AC4–6: layout gate, hazards, workspace count, no stranded tests | Level 1 in-binary layout guards, metadata, mutation checks, and live `check-tier-coverage` in all eight areas | Pass: all eight areas report zero stranded tests; no package has ten or more integration-test targets. The CI narrowing heuristic needs the follow-up above. |

## Verification performed

- Read both specifications, the migration rulings and manifests, the acceptance record, test roots, selector code, and the recorded macOS/Linux comparisons.
- Ran `consolidation.py check-metadata` for all ten manifests: pass, 18 declared targets.
- Ran the toolkit's 17 mutation checks: all detected their mutants.
- Ran `just check-tier-coverage` for all eight affected areas: zero stranded tests in each.
- Ran `just test claudine`: exit 100 with the two prompt-drift failures above.
- Inspected the committed Level 2 suite records and Level 3 target/recipe without opening or focusing a terminal window.

The ordinary CI producer observations remain pending by design. No extra CI run is needed for this review. Human review is not required to resolve these findings: the gate, CI heuristic, and comments have actionable code or documentation fixes.
