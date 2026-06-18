---
ready: false
agent: ""
model: ""
---

# Review: Level 1 / Level 2 Test Feature

## Findings

### High: Level-2 suite fails with real WezTerm available

- Requirement: prose styling must be verified through a real terminal, including SGR color output.
- Current strongest verification: intended Level 2 in `cli/tests/level2_prose_styling.rs`.
- Problem: `cargo test -p biscuit-terminal-cli --test level2_prose_styling` fails on this machine when WezTerm is available. `level2_prose_emits_sgr_in_real_terminal` captures:

  ```text
  ken@shazam ~ % bt prose "<red>x</red>"
  x
  ken@shazam ~ %
  ```

  with no `\x1b[31m` or `\x1b[91m`, so the assertion at `biscuit-terminal/cli/tests/level2_prose_styling.rs:203` fails. That means the feature is not production-ready even before considering coverage gaps.

### High: WezTerm image tests do not verify user-visible image rendering

- Requirement: `bt image` must render image protocol output in real terminals and prove cursor row / scroll behaviour through the terminal display path.
- Current strongest verification: nominal Level 2, but assertions are mostly debug-text checks.
- Problem: `level2_image_renders_in_wezterm` runs `bt image --debug` and only checks for `--- image debug ---` plus `Wezterm` in plain text (`biscuit-terminal/cli/tests/level2_image.rs:60`). `level2_image_scroll_compensation_at_bottom_margin` only checks for the diagnostic string `SCROLL needed` (`level2_image.rs:119`). `level2_warp_uses_floor_rounding` only checks that debug output mentions `Warp` and `floor=` (`level2_image.rs:155`).
- Why this is a gap: these tests can pass if the image protocol bytes are missing, if the real terminal did not render the image, if CUD row advancement is wrong, or if scroll compensation does not visibly place following text below the image. The spec required Level-2 verification of rendered image bytes / cursor row math, not debug diagnostics.

### High: Cursor placement test does not prove the sentinel lands below the rendered image

- Requirement: after image rendering, subsequent output must appear directly below the rendered image.
- Current strongest verification: nominal Level 2.
- Problem: `level2_cursor_lands_below_rendered_image` only asserts that `SENTINEL_BELOW_IMAGE` exists somewhere and that debug output contains `cursor AFTER:` (`biscuit-terminal/cli/tests/level2_cursor_and_hygiene.rs:72`). It never compares the sentinel row against the image block height or the debug before/after rows.
- Why this is a gap: the test would pass if the sentinel appeared on the wrong row, including overlapping the image, as long as the shell echoed it somewhere in the captured pane.

### High: Diagram Level-2 coverage is far below the acceptance criteria

- Requirement: at least one Level-2 test per diagram subcommand verifies rendered output contains image-protocol bytes, or fallback under tmux.
- Current strongest verification: Level 2 for only `flowchart` and `pie-chart`, plus one tmux fallback for `flowchart`.
- Missing subcommands: `quadrant`, `git-graph`, `bar-chart`, `line-chart`, `timeline`, `state-diagram`, `erd`, and `graph-expression`.
- Additional problem: the WezTerm flowchart and width tests assert metadata JSON from `--meta` (`biscuit-terminal/cli/tests/level2_diagrams.rs:44` and `level2_diagrams.rs:100`), not image protocol bytes or rendered pane geometry. `level2_inverse_flag_changes_background_in_capture` compares cache filenames (`level2_diagrams.rs:123`), not captured rendered bytes or visible output.

### High: `TerminalHarness::spawn` was not adapted to the shell model

- Requirement: the shared harness `spawn` method creates a fresh login shell and tests use `send_text("bt ...\n")`.
- Current implementation: each harness added a separate `spawn_shell`, but the trait method still spawns the requested program directly:
  - `biscuit-test-harness/src/wezterm.rs:253`
  - `biscuit-test-harness/src/kitty.rs:110`
  - `biscuit-test-harness/src/tmux.rs:132`
- Why this matters: future tests using the trait contract documented in `biscuit-test-harness/src/lib.rs:56` will bypass the shell model, PATH setup, and prompt-readiness logic. The implementation and spec now disagree.

### Medium: Layout and unicode-width tests are too weak for the behaviours they name

- `level2_columns_word_wrap_in_pane` only checks that the long word appears and that the pane has more than one line (`biscuit-terminal/cli/tests/level2_prose_styling.rs:185`). A shell prompt alone can satisfy the multiple-line condition; the test does not verify expected wrap rows or column boundaries.
- `level2_dir_command_unicode_widths_in_capture` only checks that CJK/emoji filenames and tree glyphs appear (`biscuit-terminal/cli/tests/level2_cursor_and_hygiene.rs:154`). It does not verify column alignment or consistent pre-name padding, despite the requirement being unicode width alignment.

## Verification Matrix

| Requirement | Expected level | Strongest present | Status |
|---|---:|---:|---|
| OSC 10/11/12 parsed replies | Level 1 | Level 1 PTY | OK |
| OSC52 emits to TTY | Level 1 | Level 1 PTY | OK |
| Mode 2027 enable/disable bytes | Level 1 | Level 1 PTY | OK |
| Cursor DSR query and CPR parsing | Level 1 | Level 1 PTY | OK |
| `Terminal::new()` in PTY | Level 1 | Level 1 PTY | OK |
| Prose SGR in real terminal | Level 2 | Level 2, but failing in WezTerm | Gap |
| OSC8 hyperlink in real terminal | Level 2 | Level 2 WezTerm/Kitty | Mostly OK |
| NO_COLOR in real terminal | Level 2 | Level 2 WezTerm | OK |
| Image protocol bytes and render path | Level 2 | Level 2 Kitty; WezTerm uses debug text | Gap |
| Image scroll compensation | Level 2 | Debug prediction only | Gap |
| Warp floor rounding branch | Level 2-ish via spoofed env | Debug text only | Gap |
| Diagram image rendering per subcommand | Level 2 | Partial; most subcommands missing | Gap |
| Diagram width respects pane columns | Level 2 | Metadata success only | Gap |
| Inverse changes rendered capture | Level 2 | Cache filename comparison | Gap |
| Cursor lands below image | Level 2 | Sentinel existence only | Gap |
| Dir unicode width alignment | Level 2 | Filename existence only | Gap |

## Test Commands Run

```text
cargo test -p biscuit-test-harness
cargo test -p biscuit-terminal --test level1_osc_queries --test level1_clipboard --test level1_mode_2027 --test level1_cursor --test level1_terminal_init
cargo test -p biscuit-terminal-cli --test level2_prose_styling --test level2_image --test level2_diagrams --test level2_cursor_and_hygiene
cargo check -p biscuit-tui-cli --tests
```

Results:

- `biscuit-test-harness`: passed.
- Level-1 biscuit-terminal tests: passed.
- `biscuit-tui-cli --tests`: passed.
- Level-2 biscuit-terminal CLI tests: failed in `level2_prose_emits_sgr_in_real_terminal`.

## Recommendations

1. Fix the WezTerm SGR capture problem first. Either the CLI is not emitting color in that environment, or `wezterm cli get-text --escapes` is not the right capture surface for SGR verification. Do not mark this ready while the real-terminal test fails.
2. Replace debug/meta assertions for image and diagram rendering with assertions on the captured real-terminal rendering contract: protocol bytes where the terminal CLI exposes them, rendered pane geometry / sentinel row placement where it does not.
3. Add Level-2 diagram tests for every diagram subcommand named in the spec.
4. Make `TerminalHarness::spawn` follow the shell model, or change the trait documentation/spec so callers cannot accidentally use the old direct-spawn path.
5. Strengthen layout assertions to compare exact captured rows, columns, and padding rather than checking for mere text presence.
