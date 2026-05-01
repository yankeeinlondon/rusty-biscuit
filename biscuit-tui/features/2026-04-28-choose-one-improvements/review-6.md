---
ready: false
agent: ""
model: ""
---

# Feature Review: Choose One Improvements (Review 6)

## Summary

The implementation is broad and most core behavior is covered by unit and CLI tests: ChooseOne ESC restore-submit, ChooseMany Enter semantics, padding, sort, source parsing, rendering, and ordinary completions are in place. I found two production-blocking gaps in the hotkey/modifier work and one smaller CLI source mismatch.

## Findings

### 1. Modifier release events never reach the widgets in the standalone runner

The design requires modifier-only press/release events to drive badge visibility while a modifier is held: real `KeyEventKind::Press`/`Release` events should set `hotkey_display` to `CtrlHeld`/`AltHeld` and then back to `Hidden` on release.

The component handlers implement the release branch, but `drive_event_loop_with_hint` and `drive_event_loop_with_chrome` drop all `KeyEventKind::Release` events before calling `handle_event`. In a Kitty-protocol terminal, a bare Ctrl press can set `hotkey_display = CtrlHeld` with no fallback deadline, but the release event is skipped by the runner, leaving badges stuck on until another event changes state.

Evidence:
- `biscuit-tui/lib/src/core/standalone.rs:203` and `biscuit-tui/lib/src/core/standalone.rs:285` skip release events.
- `biscuit-tui/lib/src/components/choose_one.rs:502` through `biscuit-tui/lib/src/components/choose_one.rs:513` expects release events to clear `hotkey_display`.
- `tech-design.md:354` explicitly requires press/release events to set the held mode.

Recommendation: let modifier-only release events pass through to components, or add a runner-level exception for `KeyCode::Modifier(_)` releases. Add a deterministic `drive_event_loop` test that sends modifier press then release and asserts the state redraws with badges hidden.

### 2. The default Ctrl hotkey association from the spec is not implemented

The spec says all hotkeys are Ctrl/Alt chords, Ctrl is the default association, and its example states that an option without `.with_hotkey(...)` sets up `CTRL+R`. Current state construction only builds Ctrl/Alt chord maps from explicit `ChoiceOption::hotkey`; options without explicit hotkeys only get the legacy plain-letter map.

That means a library option like `ChoiceOption::new("red", "Red", "red")` does not respond to `Ctrl+R` and does not render a badge when Ctrl is held. The CLI also leaves ordinary positional/CSV/list options with `hotkey: None` unless the caller uses `[CTRL+X]` or `--numeric-hot-keys`.

Evidence:
- `spec.md:117` through `spec.md:130` defines default Ctrl association and shows `Red` without an explicit hotkey.
- `biscuit-tui/lib/src/components/choose_one.rs:106` through `biscuit-tui/lib/src/components/choose_one.rs:107` builds separate legacy and explicit maps.
- `biscuit-tui/lib/src/components/choose_one.rs:823` through `biscuit-tui/lib/src/components/choose_one.rs:845` only inserts `ChoiceOption::hotkey` entries into Ctrl/Alt maps.
- `biscuit-tui/lib/src/components/choice_render.rs:42` through `biscuit-tui/lib/src/components/choice_render.rs:49` renders no badge when `option.hotkey` is `None`.

Recommendation: decide the source of the default chord key, then materialize it as `HotkeySpec::Ctrl` during state/input construction or render/dispatch through a unified effective-hotkey helper. Cover both library and CLI paths with tests: plain `Red` should submit on `Ctrl+R`, and holding Ctrl should show its badge.

### 3. `--list` preserves Markdown bullets from the documented example

The spec’s `--list` example is `question choose-one --list "- Apple\n- Banana\n- Cherry"`, but `parse_list` only splits non-empty lines and does not strip bullet markers. The displayed labels and returned values become `- Apple`, `- Banana`, etc.

Evidence:
- `spec.md:181` through `spec.md:184` documents the bullet-style `--list` form.
- `biscuit-tui/cli/src/option_sources.rs:174` through `biscuit-tui/cli/src/option_sources.rs:179` preserves the line text unchanged.

Recommendation: either strip Markdown bullet/numbered prefixes for `--list`, or update the docs/spec-facing CLI reference to use non-bulleted lines. Given the feature spec, parsing bullets is the better user-facing behavior.

## Test Coverage Notes

The default test suite passes:

`cargo test -p tui-chrome -p tui-chrome-cli -- --skip completions_shell --skip keyboard_protocol`

The existing direct component tests cover modifier release, but the runner-level path contradicts them by skipping releases. The PTY keyboard tests are gated and also spawn `choose-one "Red" "Green" "Blue"` without explicit hotkeys, so they do not currently prove the spec’s badge behavior for real hotkeys.

## Production Readiness

Not ready for production. The core choice behavior is mostly solid, but the hotkey feature still has contract-level gaps in default chord assignment and modifier release handling.
