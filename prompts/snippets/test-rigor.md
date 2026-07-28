---
kind: snippet
---

## Test Rigor — Level 1 / Level 2 / Level 3 / Browser / Real

Test count is not test rigor. Phrases like "covered by substantial unit and integration tests" are
banned from this review unless you can pair each user-facing requirement with a verification level:

- **Level 1 (in-process / PTY).** Unit tests, plus tests that spawn the binary in a pseudo-TTY and
  feed it manufactured input bytes. Useful and necessary, but does NOT verify the terminal emulator's
  encoder/decoder behaviour — *we* generate those bytes. Cannot catch bugs like "WezTerm does not
  emit bare-modifier press events because we forgot to push `REPORT_ALL_KEYS_AS_ESCAPE_CODES`.

- **Level 2 (run-in-real-terminal with IPC).** Spawn the binary inside an actual terminal emulator
  (WezTerm, Kitty) or multiplexer (tmux), capture the rendered pane text via the terminal's CLI
  (`wezterm cli get-text`, `kitty @ get-text`, `tmux capture-pane`). Verifies that glyphs, widths,
  SGR styling, and scrolling render correctly through the real terminal. Input is still byte-level
  injected via the terminal's CLI, so the terminal's input encoder is NOT exercised.

- **Level 3 (OS keyboard injection).** Real OS keyboard events (`cliclick` on macOS, `xdotool` on
  Linux) injected into the spawned terminal window. The terminal's input encoder fires — this is
  the only level that can verify "what bytes does the terminal actually emit when the user presses
  bare Ctrl?" Required for any UX requirement of the form "when the user holds/presses key X, Y
  happens." Currently env-gated behind `RUN_LEVEL3=1` because focus stability is platform-specific.

- **Browser.** Headless Chrome/Chromium tests via `biscuit-browser-harness`. Assert on computed
  CSS styles, not source substrings or screenshots. Skips cleanly when Chrome is absent; hard-fails
  when `BISCUIT_BROWSER_REQUIRED=1`.

- **Real.** Tests against real external resources (devices, networks, APIs). Always `--ignored`
  unless explicitly opted in via per-package env vars.

When reviewing, for each requirement that asserts user-observable behaviour (modifier-press
visibility, hotkey activation, keybinding behaviour, paste / IME / mouse, scroll on overflow, etc.),
classify the verification level present and call out any mismatch:

- "Spec requires modifier-press to surface badges" + only Level-1 tests = **gap, not "ready"**.
- "Spec requires hotkey chord activation" + Level-2 in tmux but no Level-1 chord-byte test = fine.
- "Spec requires `^X` badges with specific colors" + Level-1 unit tests on style only = needs
  Level-2 capture verifying real-terminal rendering.

Use `test_toolkit::require_level!(Level::L2, harness_available(), "label")` to gate Level-2+ tests
so they skip cleanly when the harness is unavailable, and `BISCUIT_REQUIRED_BACKENDS=tmux` to make a
named backend's absence a hard failure. Prefer it over `BISCUIT_TEST_LEVEL_REQUIRED`, which cannot
express "require tmux but let WezTerm skip". See `.claude/skills/rust-testing/SKILL.md` for the full
decision tree and canonical `just` recipes.

### A passing Level-2 test may never have run

`require_level!` skips by `return`ing, and nextest cannot distinguish that from a test that ran and
asserted nothing — so **every silent skip is counted as a pass**. A tier with no available backend
reports `18 tests run: 18 passed` in 0.138s; the same tier with a backend reports `18 passed` in
13.28s. There is no `0 run`, no warning, and no failure.

Reviewers MUST NOT accept "the Level-2 suite is green" as evidence that Level-2 verification exists.
Require one of: elapsed time consistent with real terminal work, a run under
`BISCUIT_REQUIRED_BACKENDS`, or evidence the test was observed **failing** against the unfixed code.
A test that has only ever been green may never have executed. This is not hypothetical — a `::shell`
hang shipped in darkmatter behind a green Level-2 tier for exactly this reason.

A feature MAY be marked production-ready only when each user-observable requirement has at minimum
the level of verification appropriate for it. Reviewers MUST list any requirement whose strongest
test is at the wrong level under "Findings" with severity at least "high".
