---
ready: false
agent: ""
model: ""
---

# Feature Review: Choose One Improvements (Review 12)

## Summary

The implementation is substantially complete. I verified the focused suite:

```text
cargo test -p biscuit-tui -p biscuit-tui-cli
```

It passed on this host, including the shell-completion PTY tests and the real-terminal rendering tests. The prior review items around keyboard protocol flags, completion PTY coverage, duplicate hotkey semantics, file/frontmatter sources, padding, active colours, and real-terminal badge checks appear addressed.

I found two remaining issues around `Ctrl+C`: the reusable `ChooseOne` component still does not implement the design's component-level abort contract, and the standalone `Ctrl+C` user-facing behavior is only verified at Level 1.

## Findings

### 1. `ChooseOne::handle_event` still treats `Ctrl+C` as a selectable Ctrl hotkey

Severity: high.

The technical design requires `ChooseOne::handle_event` to handle `Ctrl-C` first and return cancellation before hotkey dispatch. The current component handler starts with modifier badge handling and later dispatches Ctrl hotkeys:

- `biscuit-tui/lib/src/components/choose_one.rs:543`
- `biscuit-tui/lib/src/components/choose_one.rs:614`
- `biscuit-tui/lib/src/components/choose_one.rs:696`

There is no early `Ctrl+C` guard in `ChooseOne::handle_event`. Standalone prompts are protected because `drive_event_loop` intercepts `Ctrl+C` before delegating to the component:

- `biscuit-tui/lib/src/core/standalone.rs:210`
- `biscuit-tui/lib/src/core/standalone.rs:291`

That makes the CLI path exit `130`, but embedded callers using `HandleEvent` directly can still bind `HotkeySpec::Ctrl('c')` and get `EventOutcome::Submitted`. This violates the library-level event order in the design and gives embedded `ChooseOne` a different abort contract from standalone `question choose-one`.

Recommendation: add an early component-level `Ctrl+C` branch before modifier badge and hotkey handling. Add unit coverage proving `ChooseOne::handle_event(Ctrl+C)` returns `EventOutcome::Cancelled` and does not submit an explicit `HotkeySpec::Ctrl('c')` option.

### 2. Standalone `Ctrl+C` exit behavior is only verified at Level 1

Severity: high.

The spec says `Ctrl+C` is the only way to abort `ChooseOne` without returning a value, with exit code `130`. Current coverage for that specific behavior is a PTY test that writes manufactured `\x03` bytes:

- `biscuit-tui/cli/tests/choose_cli.rs:999`

Per the review's test-rigor rubric, that is Level 1. It proves the runner maps byte-level `Ctrl+C` input to exit `130`, but it does not verify the behavior through a real terminal or multiplexer. The real-terminal suite now covers important neighboring requirements:

- Level 2 `Ctrl+Space` badge rendering in tmux: `biscuit-tui/cli/tests/real_terminal_render.rs:115`
- Level 2 WezTerm kitty bare-Ctrl bytes: `biscuit-tui/cli/tests/real_terminal_render.rs:166`
- Level 3 bare-Ctrl badge visibility via cliclick: `biscuit-tui/cli/tests/real_terminal_render.rs:240`
- Level 3 Ctrl+R chord selection: `biscuit-tui/cli/tests/real_terminal_render.rs:344`

There is still no Level 2 or Level 3 assertion for `question choose-one` receiving `Ctrl+C` from a terminal harness and exiting `130`. Because the requirement is user-observable keybinding behavior, Level 1 alone is not enough under the stated review rules.

Recommendation: add a Level 2 tmux or WezTerm test that spawns `question choose-one`, sends `C-c` through the terminal harness, and asserts process exit `130`. A Level 3 `cliclick` Ctrl+C test would also be appropriate if focus stability is acceptable, but Level 2 should be enough for this chord path.

## Verification Notes

- Completion contract: Level 1 script-content tests plus PTY-driven zsh/bash tests are present and passed. The PTY tests cover hotkey-prefix candidates, no hotkey-prefix pollution at empty option position, post-`--` flag completion, and root subcommands.
- Bare modifier badge visibility: Level 2 and Level 3 tests are present and passed on this host. The runner pushes `REPORT_EVENT_TYPES | DISAMBIGUATE_ESCAPE_CODES | REPORT_ALL_KEYS_AS_ESCAPE_CODES` and pops symmetrically.
- Rendering/style requirements: unit buffer tests cover radio/checkbox glyphs, active span width, foreground/background selection, and badge styling; Level 2 tests cover real-terminal rendering smoke and badge visibility.
- Core interaction requirements: `ChooseOne` Enter/Space/Esc and `ChooseMany` Enter/Space semantics have Level 1 unit coverage. Hotkey chord activation has Level 3 coverage for Ctrl+R.

## Production Readiness

Not ready. Most of the feature is in good shape, but the component-level `Ctrl+C` behavior is still not implemented, and the standalone `Ctrl+C` abort requirement is below the requested verification level.
