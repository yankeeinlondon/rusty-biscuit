---
ready: false
---

# Review 1

## Findings

### 1. CLI choose commands never enable fuzzy search

Severity: High

The spec says `choose-one` and `choose-many` should show the search prompt and filter the list as soon as the user types alphanumeric characters. The library implementation supports this only when `ChoiceInput::filter_enabled` is true, but the CLI builders never set it:

- `biscuit-tui/cli/src/commands/choose_one.rs:142`
- `biscuit-tui/cli/src/commands/choose_many.rs:170`

As a result, `question choose-one Apple Banana` still uses the legacy first-letter hotkey path. Typing `B` selects/jumps to Banana instead of opening `/ B` and filtering. `choose-many` has the same issue. This also means the designed `--no-filter` opt-out is absent, but the more important bug is that the default CLI behavior does not meet the feature spec.

Suggested fix: after building every `ChoiceInput` source path, call `.with_filter_enabled(true)` for both choose subcommands, and add a shared `--no-filter` flag if the design’s compatibility escape hatch is still desired. Add CLI-level tests that inspect the state passed into `run_with_writer`, or add the designed synthetic event runner, so this cannot regress.

### 2. Integration tests do not validate successful CLI output for most new behavior

Severity: Medium

The design calls for integration tests using `QUESTION_TEST_AUTOSUBMIT` to verify successful stdout for stdin, positional args, delimiter values, exit codes, and bulk selection. That synthetic event path is not implemented, and the main integration tests intentionally assert failure after parsing because there is no TTY:

- `biscuit-tui/cli/tests/choose_cli.rs:5`
- `biscuit-tui/cli/tests/choose_cli.rs:19`
- `biscuit-tui/cli/tests/choose_cli.rs:61`

This is too light for the requested production bar. For example, `delimiter_separates_label_and_value` never proves that selecting `Apple:1` emits `1`; it only proves clap accepted the flag and then the command failed later. The PTY tests cover a few keystroke paths, but they are gated behind `QUESTION_INTERACTIVE_PTY=1` and return early by default:

- `biscuit-tui/cli/tests/choose_cli.rs:421`
- `biscuit-tui/cli/tests/choose_cli.rs:446`
- `biscuit-tui/cli/tests/choose_cli.rs:471`

Suggested fix: implement the test event injection described in the design, or expose a test-only command runner seam that can feed synthetic `crossterm::Event`s through the real CLI path. Then add green-path tests for stdin, positionals, delimiter output, selected defaults, filter typing, Esc, Ctrl+C, and Ctrl+A/Ctrl+D.

## Notes

I ran:

- `cargo test -p tui-chrome-cli --test choose_cli`
- `cargo test -p tui-chrome typing_letter_opens_filter`

Both passed. The passing tests do not cover the missing CLI filter wiring, which is why this review marks the feature as not ready for production.
