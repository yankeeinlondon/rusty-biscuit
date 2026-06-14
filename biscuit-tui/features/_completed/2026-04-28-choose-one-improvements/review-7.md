---
ready: false
agent: ""
model: ""
---

# Feature Review: Choose One Improvements (Review 7)

## Summary

The previously identified blockers have mostly been addressed in the normal unit/integration surface: default Ctrl hotkeys now exist, modifier release events can reach widgets, and `--list` now strips simple Markdown list markers. The default package tests pass.

However, the feature design explicitly says the completion and keyboard-modifier claims cannot be marked production-ready without the corresponding PTY tests passing. Those gated tests fail when enabled, and I also found a smaller but real CLI/API override bug around `--hotkey-badges`.

## Findings

### 1. Required PTY verification gates fail when enabled

The design makes PTY-driven shell completion and keyboard-protocol tests a production readiness gate: completion claims must be verified in real `zsh` and `bash`, keyboard-modifier claims must exercise the real `prepare_terminal` sequence, and the feature must not be marked production-ready without those tests passing.

Running those gates currently fails:

```text
RUN_PTY_TESTS=1 cargo test -p biscuit-tui-cli --test keyboard_protocol -- --nocapture
```

Result: 0 passed, 4 failed.

- The dumb-terminal tests fail before launch because `expectrl::spawn` is given a command string prefixed with `TERM=dumb ...`; this is treated as an executable name rather than a shell assignment in this environment.
- The normal keyboard-protocol tests then hit I/O errors writing Ctrl bytes, which means the test is not proving bare Ctrl or chord fallback behavior under the real PTY path.

Evidence:
- `biscuit-tui/features/2026-04-28-choose-one-improvements/tech-design.md:510` through `:529` defines the PTY readiness gates.
- `biscuit-tui/cli/tests/keyboard_protocol.rs:71` through `:77` spawns the ordinary command via a formatted string.
- `biscuit-tui/cli/tests/keyboard_protocol.rs:163` through `:169` prefixes `TERM=dumb` directly into the spawned command string.

Shell completion verification also fails:

```text
RUN_SHELL_TESTS=1 cargo test -p biscuit-tui-cli --test completions_shell -- --nocapture
```

Result: 0 passed, 7 failed.

Most failures come from all tests sharing the same process-id temp directory and then calling `create_dir` for the same `fpath` / `bash_completion.d` path in parallel.

Evidence:
- `biscuit-tui/cli/tests/completions_shell.rs:19` through `:23` creates a temp root keyed only by process id.
- `biscuit-tui/cli/tests/completions_shell.rs:62` through `:67` creates the same `fpath` subdirectory per zsh test.
- `biscuit-tui/cli/tests/completions_shell.rs:211` through `:216` creates the same `bash_completion.d` subdirectory per bash test.

Recommendation: fix the harness first so each test has a unique tempdir and shell commands are spawned through a reliable shell invocation or `Command`-style API with environment variables set explicitly. Then rerun both gated suites and keep them enabled in CI or in a documented production-readiness job.

### 2. `--hotkey-badges never/always/ctrl/alt` is not actually forced for the lifetime of the prompt

The CLI exposes `--hotkey-badges`, and its docs say `never` hides badges entirely while `always` keeps them visible for the prompt lifetime. `resolve_hotkey_badges` maps those modes into `with_hotkey_display`, whose rustdoc also says it bypasses modifier detection and forces the value for the lifetime of the state.

But both `ChooseOne::handle_event` and `ChooseMany::handle_event` still overwrite `state.hotkey_display` on modifier-only press/release and Ctrl/Alt chord fallback. That means:

- `--hotkey-badges never` can start hidden, then a Ctrl/Alt press makes badges visible anyway.
- `--hotkey-badges always`, `ctrl`, or `alt` can be cleared back to hidden by a modifier release event.

Evidence:
- `biscuit-tui/cli/src/commands/common_choose.rs:159` through `:167` documents the public flag behavior.
- `biscuit-tui/cli/src/commands/common_choose.rs:200` through `:205` maps `Never`/`Always` into forced display modes.
- `biscuit-tui/lib/src/components/choose_one.rs:165` through `:178` says `with_hotkey_display` is lifetime-forcing.
- `biscuit-tui/lib/src/components/choose_one.rs:502` through `:526` overwrites that state on modifier events and chord fallback.
- `biscuit-tui/lib/src/components/choose_many.rs:531` through `:539` has the same fallback overwrite path.

Recommendation: represent forced badge mode separately from transient badge mode, e.g. `hotkey_display_override: Option<HotkeyDisplayMode>`, and have event handlers skip transient mutations when an override is present. Add tests for both components proving `Hidden` stays hidden after Ctrl/Alt press and `CtrlHeld` stays visible after modifier release.

### 3. Hotkey parsing accepts multi-character specs by silently truncating

The spec describes `[CTRL+{char}]`, `[ALT+{char}]`, and `[OPT+{char}]` as single-key assignments. The parser currently takes only the first character after the modifier prefix and ignores the rest, so `CTRL+RED` becomes `Ctrl('r')` instead of being rejected as an invalid hotkey.

Evidence:
- `biscuit-tui/features/2026-04-28-choose-one-improvements/spec.md:117` through `:133` defines hotkeys as a modifier plus one character.
- `biscuit-tui/cli/src/choice_normalize.rs:113` through `:126` uses `rest.chars().next()?` and does not check that the rest contains exactly one character.

Recommendation: require exactly one character after `CTRL+`, `ALT+`, or `OPT+`, then route invalid object hotkeys through the existing `InvalidHotkey` error. Add normalization tests for `CTRL+AB`, `ALT+`, and bracketed prefix equivalents.

## Test Coverage Notes

Passed:

```text
cargo test -p biscuit-tui -p biscuit-tui-cli
```

Failed gated verification:

```text
RUN_PTY_TESTS=1 cargo test -p biscuit-tui-cli --test keyboard_protocol -- --nocapture
RUN_SHELL_TESTS=1 cargo test -p biscuit-tui-cli --test completions_shell -- --nocapture
```

The default test command is not enough to satisfy this feature's design gates because those PTY tests return early unless their environment variables are set.

## Production Readiness

Not ready for production. The core widget behavior is in much better shape, but the feature's own required PTY verification is currently failing, and one public hotkey-badge override mode is not honored after input events.
