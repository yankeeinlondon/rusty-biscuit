---
ready: false
agent: ""
model: ""
---

# Review: Level 1 / Level 2 Test Feature

## Findings

### High: WezTerm SGR Level-2 test still fails in a real terminal

- Requirement: prose SGR styling must be verified through a real terminal.
- Expected verification level: Level 2.
- Current strongest verification: Level 2 in `biscuit-terminal/cli/tests/level2_prose_styling.rs`.
- Problem: `cargo test -p biscuit-terminal-cli --test level2_prose_styling` fails on this machine when WezTerm is available. `level2_prose_emits_sgr_in_real_terminal` asserts for `\x1b[31m` or `\x1b[91m` at `biscuit-terminal/cli/tests/level2_prose_styling.rs:51`, but both captures contain only:

  ```text
  ken@shazam ~ % bt prose "<red>x</red>"
  x
  ken@shazam ~ %
  ```

  No SGR bytes are present in either capture. This is either a real rendering/capture incompatibility in the WezTerm path or the test is asserting against the wrong WezTerm capture surface. Either way, the feature cannot be marked ready while a required Level-2 test fails under an available real terminal.

### High: Diagram width test does not verify pane-column width

- Requirement: `level2_diagram_width_respects_pane_columns` must prove `--width 50%` respects the actual pane columns.
- Expected verification level: Level 2, because this is terminal-rendered image geometry.
- Current strongest verification: Level 2 process execution, but only metadata success.
- Problem: the test sends `bt pie-chart --meta --width 50% "A: 1"` and asserts only that `"filename"` and `"render_time_ms"` appear and that the fenced fallback did not fire (`biscuit-terminal/cli/tests/level2_diagrams.rs:331` and `biscuit-terminal/cli/tests/level2_diagrams.rs:347`). That proves the diagram rendered, not that the image block width was 50% of the real pane.
- Impact: a regression that ignores percentage width, uses host `$COLUMNS`, or always renders full width would still pass.

### High: Acceptance criterion 3 is not met for all public discovery functions

- Requirement: every public function in `discovery/osc_queries.rs`, `discovery/clipboard.rs`, `discovery/mode_2027.rs`, and `discovery/cursor_position.rs` has at least one Level-1 PTY test that asserts behaviour, not just “doesn't panic.”
- Current strongest verification: Level 1 covers the main default paths, but not every public function.
- Missing Level-1 PTY coverage includes:
  - `osc52_support`, `set_clipboard_with_target`, `clear_clipboard`, and `get_clipboard` in `biscuit-terminal/lib/src/discovery/clipboard.rs:63`, `:161`, `:190`, and `:224`.
  - `bg_color_with_timeout`, `text_color_with_timeout`, and `cursor_color_with_timeout` in `biscuit-terminal/lib/src/discovery/osc_queries.rs:376`, `:384`, and `:392`.
  - `cursor_position_with_timeout` in `biscuit-terminal/lib/src/discovery/cursor_position.rs:39`.
  - `supports_mode_2027` in `biscuit-terminal/lib/src/discovery/mode_2027.rs:61`.
- The existing Level-1 files verify the important default behaviours, for example OSC colors in `level1_osc_queries.rs:55`, clipboard write in `level1_clipboard.rs:12`, and mode enable/disable in `level1_mode_2027.rs:12`. They do not satisfy the spec's “every public function” wording.

### Medium: Shell detection can choose non-POSIX shells, but Level-2 tests send POSIX syntax

- Requirement: the harness shell model should be reliable for shell-driven CLI tests.
- Problem: `detect_shell()` prefers `$SHELL` whenever the executable exists (`biscuit-test-harness/src/lib.rs:212`). The Level-2 tests then send POSIX shell syntax such as `export TERM_PROGRAM=WarpTerminal`, `unset TERM_PROGRAM`, and inline assignment `NO_COLOR=1 bt ...` (`biscuit-terminal/cli/tests/level2_image.rs:265`, `:330`, `biscuit-terminal/cli/tests/level2_prose_styling.rs:99`). These commands are not portable to shells like fish.
- Impact: developers with a non-POSIX login shell can get false failures unrelated to terminal behaviour. Prefer selecting `bash`/`sh` for test shells, or have the harness expose a shell-agnostic `send_command_with_env` helper.

## Verification Matrix

| Requirement | Expected level | Strongest present | Status |
|---|---:|---:|---|
| OSC 10/11/12 parsed replies | Level 1 | Level 1 PTY | OK for default APIs |
| OSC timeout variants | Level 1 | Unit/default-path coverage only | Gap |
| OSC52 clipboard write | Level 1 | Level 1 PTY | OK for `set_clipboard` |
| Other public clipboard APIs | Level 1 | Unit/default-path coverage only | Gap |
| Mode 2027 enable/disable bytes | Level 1 | Level 1 PTY | OK |
| `supports_mode_2027` heuristic in PTY | Level 1 | Unit/default-path coverage only | Gap |
| Cursor DSR query and CPR parsing | Level 1 | Level 1 PTY | OK for default API |
| Cursor timeout variant | Level 1 | Default-path coverage only | Gap |
| `Terminal::new()` in PTY | Level 1 | Level 1 PTY | OK |
| Prose SGR in WezTerm | Level 2 | Level 2, failing | Gap |
| Prose SGR in Kitty | Level 2 | Level 2 | OK |
| OSC8 hyperlink in real terminal | Level 2 | Level 2 | OK |
| NO_COLOR in real terminal | Level 2 | Level 2 | OK |
| Diagram image rendering per subcommand | Level 2 | Kitty protocol bytes + WezTerm metadata | Mostly OK |
| Diagram width respects pane columns | Level 2 | Metadata success only | Gap |
| Image protocol / cursor / scroll branches | Level 2 | Level 2 | OK |
| Dir unicode width alignment | Level 2 | Level 2 geometry assertion | OK |

## Test Commands Run

```text
cargo test -p biscuit-test-harness
cargo test -p biscuit-terminal --example discovery_probe --test level1_osc_queries --test level1_clipboard --test level1_mode_2027 --test level1_cursor --test level1_terminal_init
cargo test -p biscuit-terminal-cli --test level2_prose_styling --test level2_image --test level2_diagrams --test level2_cursor_and_hygiene
```

Results:

- `biscuit-test-harness`: passed.
- Targeted Level-1 biscuit-terminal tests: passed.
- Level-2 cursor/hygiene, diagram, and image tests: passed on this machine.
- Level-2 prose styling failed in `level2_prose_emits_sgr_in_real_terminal`.

## Recommendation

Do not mark this ready yet. Fix or replace the WezTerm SGR assertion so the required Level-2 prose test passes against a real WezTerm capture, strengthen the diagram width test to assert actual captured geometry, and either add Level-1 PTY coverage for the remaining public discovery functions or narrow the acceptance criterion to the public behaviours that genuinely require PTY verification.
