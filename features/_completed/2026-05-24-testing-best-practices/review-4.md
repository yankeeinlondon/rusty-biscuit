---
ready: false
agent: codex
model: ""
---

# Review 4

## Findings

### High - Alt hotkey chord behavior is claimed as Level-3 verified but has no Level-3 test

The updated production-readiness note says chord presses, including `Ctrl+R`, `Alt+R`, and similar chords, are "fully verified end-to-end with Level-3 OS keyboard injection" (`biscuit-tui/docs/components/choose_one.md:88`). The active Level-3 coverage does not support that claim. `biscuit-tui/cli/tests/real_terminal_render.rs` contains `level3_wezterm_ctrl_r_chord_selects_red` for a real OS-injected `Ctrl+R` chord, but there is no corresponding Level-3 test for `Alt+R`, `Alt+B`, `Alt+Space`, or any Alt chord. A targeted search of `biscuit-tui/cli/tests` and `biscuit-tui/lib/src` only found Level-1/unit coverage and parsing/completion tests for Alt hotkeys, not an OS-injected terminal-path assertion.

Requirement verification level: hotkey activation is user-observable keypress behavior, so Level 3 is required by this review rubric. Current strongest verification for Alt-specific hotkey behavior appears to be Level 1, while the docs now claim Level 3. The existing `Ctrl+R` Level-3 test is useful, but it does not prove that the terminal/OS path encodes and forwards Alt chords correctly.

Recommended fix: either add an active `level3_wezterm_alt_*` test that injects an Alt chord into a real WezTerm pane and verifies the expected selection/submission or narrow the production-readiness note to say only the tested Ctrl chord path is Level-3 verified. If the supported production claim includes `Alt+Space` badge toggling, that needs its own Level-3 verification too.

## Test Rigor Notes

- The previous root canonical validator hang is fixed. `timeout 65s just check-canonical` completed successfully across all 17 curated areas.
- The previous bare-Ctrl mismatch is now handled honestly: the docs mark end-to-end bare-Ctrl visibility as best-effort / not production-verified on macOS, and the ignored Level-3 test documents why the active Level-2 raw-byte test is not a substitute for OS keyboard injection.
- Ctrl badge styling still has appropriate Level-2 real-terminal rendering coverage through tmux capture.

## Verification Performed

- Read `features/2026-05-24-testing-best-practices/spec.md`, `plan.md`, and `review-3.md`.
- Inspected the changed files in `.github/workflows/fuzz-nightly.yml`, `justfile`, `biscuit-tui/cli/tests/real_terminal_render.rs`, and `biscuit-tui/docs/components/choose_one.md`.
- Ran `timeout 65s just check-canonical` successfully.
- Ran `cargo test --color=never -p test-toolkit --lib --no-run` successfully.
- Ran `git diff --check` successfully.
- Searched for Alt-specific Level-3 coverage with `rg -n "level3_.*alt|Alt\\+|ALT\\+|M-" biscuit-tui/cli/tests biscuit-tui/lib/src -S`.

## Production Readiness

Not ready for production. The previous high-severity issues are substantially addressed, but the implementation now overstates Level-3 coverage for Alt chord behavior. Under the requested rigor rubric, that mismatch must stay a high-severity finding until the claim is narrowed or the missing Level-3 test is added.
