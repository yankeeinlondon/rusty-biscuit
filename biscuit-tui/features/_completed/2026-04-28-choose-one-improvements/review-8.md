---
ready: false
agent: ""
model: ""
---

# Feature Review: Choose One Improvements (Review 8)

## Summary

The major Review 7 verification blockers have been fixed: the default package tests pass, and both gated PTY suites now pass when enabled. I found two remaining correctness issues in hotkey handling. Both are small, but they affect public CLI/library behavior, so I would not mark this production-ready yet.

## Findings

### 1. `--hotkey-badges never/always/ctrl/alt` is still mutable after the prompt starts

The CLI documents `--hotkey-badges always` as visible for the prompt lifetime and `never` as hidden entirely. The state builder also documents `with_hotkey_display` as forcing the display mode for the lifetime of the state.

However, both choice event handlers still write directly into the same `hotkey_display` field for modifier-only events and Ctrl/Alt chord fallback. That means a forced mode can be overwritten:

- `--hotkey-badges never` starts hidden, but pressing Ctrl/Alt can show badges.
- `--hotkey-badges always`, `ctrl`, or `alt` can be cleared to hidden by a modifier release event.
- A forced `ctrl` display can be changed to `alt` by an Alt event, and vice versa.

Evidence:

- `biscuit-tui/cli/src/commands/common_choose.rs:159` documents the public `--hotkey-badges` behavior.
- `biscuit-tui/cli/src/commands/common_choose.rs:194` maps non-auto modes into a forced `with_hotkey_display` call.
- `biscuit-tui/lib/src/components/choose_one.rs:165` documents `with_hotkey_display` as lifetime-forcing.
- `biscuit-tui/lib/src/components/choose_one.rs:502` mutates `state.hotkey_display` on modifier press/release.
- `biscuit-tui/lib/src/components/choose_one.rs:520` mutates it again for chord fallback.
- `biscuit-tui/lib/src/components/choose_many.rs:154` gives `ChooseMany` the same forced-mode contract.
- `biscuit-tui/lib/src/components/choose_many.rs:516` and `:533` have the same mutation paths.

Recommendation: split forced and transient badge state, for example `hotkey_display_override: Option<HotkeyDisplayMode>` plus `transient_hotkey_display`. Event handlers should only mutate the transient state when there is no override. Add tests for both components proving `Hidden` stays hidden after Ctrl/Alt press and `CtrlHeld`/`AltHeld` survives modifier release.

### 2. Hotkey parsing still silently truncates multi-character specs

The spec defines hotkeys as `CTRL+key` or `ALT+key`, with examples using a single character. The parser accepts the first character after the modifier and ignores the rest, so `CTRL+RED` becomes `Ctrl('r')` instead of being rejected.

This affects both bracketed CLI prefixes and object/file sources because object-supplied `hotkey` fields route through the same parser. It can hide configuration mistakes and create accidental duplicate hotkeys that are hard for callers to diagnose.

Evidence:

- `biscuit-tui/features/2026-04-28-choose-one-improvements/spec.md:117` defines hotkeys as `CTRL+key` / `ALT+key`.
- `biscuit-tui/features/2026-04-28-choose-one-improvements/spec.md:133` shows the CLI shorthand `[CTRL+R] Red`.
- `biscuit-tui/cli/src/choice_normalize.rs:113` parses hotkeys.
- `biscuit-tui/cli/src/choice_normalize.rs:116` through `:126` uses `rest.chars().next()?` without checking that exactly one character remains.
- `biscuit-tui/cli/src/choice_normalize.rs:868` has invalid-hotkey coverage, but only for unsupported prefixes, not multi-character or empty supported prefixes.

Recommendation: require exactly one character after `CTRL+`, `ALT+`, or `OPT+`. Return `InvalidHotkey` for object fields like `CTRL+RED`, `ALT+`, and `OPT+AB`; for bracketed prefixes, either reject during normalization or leave the prefix intact and surface a clear invalid hotkey error. Add tests for canonical single-char specs, empty specs, and multi-character specs.

## Test Results

Passed:

```text
cargo test -p biscuit-tui -p biscuit-tui-cli
RUN_PTY_TESTS=1 cargo test -p biscuit-tui-cli --test keyboard_protocol -- --nocapture
RUN_SHELL_TESTS=1 cargo test -p biscuit-tui-cli --test completions_shell -- --nocapture
```

## Production Readiness

Not ready for production. The broad feature surface and required PTY verification are in good shape, but the hotkey badge override contract and hotkey parser need correction before release.
