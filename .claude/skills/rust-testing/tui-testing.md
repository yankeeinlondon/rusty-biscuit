# TUI Testing

Part of the `rust-testing` skill; [SKILL.md](SKILL.md) is the index.

## Separate rendering from event handling

- **Rendering:** draw into `ratatui::backend::TestBackend` and snapshot the
  buffer when layout matters, or assert a few strings when it does not. Cover a
  normal width and a narrow width that proves the code does not panic, as in
  `claudine/cli/src/commands/config_tui/tabs/services.rs`
  (`services_render_handles_narrow_terminal`).
- **Events:** call key handlers (`handle_key`) and pure reducers directly and
  assert state, not rendered output. Keep modal handlers thin and delegating to
  reducers; test overview/detail transitions and the modal stack at app level.

## Never spawn an interactive TUI from a test

A subprocess inherits the test's controlling terminal. When it calls
`crossterm::enable_raw_mode()` it changes termios on that **shared** tty;
raw mode clears `OPOST`, so the parent's `\n` stops producing a carriage return
and nextest's progress bar or any spinner redraws in the wrong column (lines
staircase rightward, or stack without overwriting).

So a test like "run `question choose-one … --flag-x` and expect exit 1" only
passes where there is no controlling tty (CI). From a developer's shell the
prompt really runs: it hangs on `event::read` or corrupts the parent's output.
Cover flag plumbing instead with:

- `Cli::try_parse_from(...)` unit tests,
- `--help` snapshots for flag visibility,
- the writer-seam `run_with_writer` pattern (see the `biscuit-tui` skill;
  example in `biscuit-tui/cli/src/commands/boolean_switch.rs`).

## Guard interactive binaries against headless runs

A binary that enters raw mode must refuse to start when **both** stdout and
stderr are not terminals, before touching `/dev/tty`. Checking both keeps
`FOO=$(question …)` working (stdout piped, stderr on the tty) while rejecting the
test-harness and pipeline shape. Reference: the guard in
`biscuit-tui/lib/src/core/standalone/mod.rs`.
