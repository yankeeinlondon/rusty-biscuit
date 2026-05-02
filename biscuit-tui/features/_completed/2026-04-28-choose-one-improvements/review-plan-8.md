# Review 8 Implementation Plan

## Goal

Address every finding in `review-8.md` for the ChooseOne improvements feature and leave the `biscuit-tui` package area with passing focused tests, passing full package tests, and no clippy warnings/errors.

This plan covers:

- Forced `--hotkey-badges` modes (`never`, `always`, `ctrl`, `alt`) being silently overwritten by transient modifier-only press/release events and chord-fallback writes in both `ChooseOne` and `ChooseMany`.
- Hotkey parsing in `cli/src/choice_normalize.rs` silently truncating multi-character specs such as `CTRL+RED` -> `Ctrl('r')`, and accepting empty suffixes such as `ALT+`.

The two findings are independent in code surface (library components vs. CLI parser) but are tightly coupled in user-visible "hotkey" behavior, so each gets its own implementation phase followed by a single shared verification phase.

## Phase 1 - Make `--hotkey-badges` Override Authoritative

### Scope

Fix the public `--hotkey-badges never|always|ctrl|alt` contract so an explicit override mode cannot be mutated by:

- Modifier-only `KeyCode::Modifier` press/release events (the `modifier_only_mode` path).
- Chord-fallback writes triggered by `KeyModifiers::CONTROL` / `KeyModifiers::ALT` on any other key event.

The override must persist for the lifetime of the state, exactly as documented at `biscuit-tui/lib/src/components/choose_one.rs:165` and `biscuit-tui/lib/src/components/choose_many.rs:154`.

### Implementation Steps

1. Update `biscuit-tui/lib/src/components/choose_one.rs`.
   - Add a new field to `ChooseOneState<V>`:
     - `hotkey_display_override: Option<HotkeyDisplayMode>`
   - Initialize it to `None` in `ChooseOneState::new` (alongside the existing `hotkey_display` and `hotkey_display_deadline` fields near line ~130).
   - Change `with_hotkey_display(mut self, mode: HotkeyDisplayMode) -> Self` (line ~175) so it:
     - Sets `self.hotkey_display_override = Some(mode);`
     - Sets `self.hotkey_display = mode;`
     - Sets `self.hotkey_display_deadline = None;`
     - Returns `self`.
   - Add a small private helper (or inline check) so that `hotkey_display_override.is_some()` can be queried as "is forced" from the event handler.
   - Update `current_hotkey_display(&self, now: Instant)` (line ~210) to short-circuit and return `override` when present, ignoring the deadline path entirely.
   - In `impl<V: Clone + PartialEq> HandleEvent for ChooseOne<V>::handle_event` (line ~496):
     - In the `modifier_only_mode` branch (lines ~502-515), do NOT mutate `state.hotkey_display` or `state.hotkey_display_deadline` when `state.hotkey_display_override.is_some()`. Still return `EventOutcome::Consumed` so terminals that only emit modifier-only events do not leak the keypress to other bindings.
     - In the chord-fallback branch (lines ~520-526), wrap both the `CONTROL` and `ALT` arms in a `state.hotkey_display_override.is_none()` guard so a forced mode is never overwritten and no fallback deadline is armed.

2. Update `biscuit-tui/lib/src/components/choose_many.rs` with the same model.
   - Add `hotkey_display_override: Option<HotkeyDisplayMode>` to `ChooseManyState<V>` and initialize to `None`.
   - Update `with_hotkey_display` (line ~156) to set the override, mirror the mode into `hotkey_display`, and clear the deadline.
   - Update `current_hotkey_display` to short-circuit on override.
   - In `impl<V: Clone + PartialEq> HandleEvent for ChooseMany<V>::handle_event` (line ~512):
     - Guard the `modifier_only_mode` mutations (lines ~516-528) with the override check.
     - Guard the chord-fallback writes (lines ~533-539) with the override check.

3. CLI wiring.
   - `biscuit-tui/cli/src/commands/choose_one.rs` and `biscuit-tui/cli/src/commands/choose_many.rs` already call `with_hotkey_display` when `resolve_hotkey_badges` returns `Some` (per `common_choose.rs:194`); no CLI-flag mapping changes are needed.
   - Confirm the doc comment at `lib/src/components/choose_one.rs:165` and the matching comment at `choose_many.rs:154` still describe the new behavior accurately. They already say "lifetime-forcing"; leave them and make the code match.

### Tests to Add or Adjust

In `biscuit-tui/lib/src/components/choose_one.rs` (in the existing `mod tests` near `with_hotkey_display_forces_mode_and_clears_deadline` at line ~2139), add:

- `with_hotkey_display_hidden_survives_ctrl_modifier_press`
  - Build state with `with_hotkey_display(HotkeyDisplayMode::Hidden)`.
  - Send a `KeyCode::Modifier(ModifierKeyCode::LeftControl)` press event (or whatever `modifier_only_mode` recognizes).
  - Assert `state.hotkey_display() == HotkeyDisplayMode::Hidden` and `state.hotkey_display_deadline.is_none()`.
  - Also assert `state.current_hotkey_display(Instant::now()) == HotkeyDisplayMode::Hidden`.
- `with_hotkey_display_hidden_survives_chord_fallback`
  - Build state with `with_hotkey_display(HotkeyDisplayMode::Hidden)`.
  - Send a `KeyEvent` with `KeyModifiers::CONTROL` and a printable char (e.g. `'a'`).
  - Assert `state.hotkey_display() == HotkeyDisplayMode::Hidden`, `state.hotkey_display_deadline.is_none()`.
- `with_hotkey_display_ctrl_held_survives_modifier_release`
  - Build state with `with_hotkey_display(HotkeyDisplayMode::CtrlHeld)`.
  - Send a `KeyCode::Modifier(...)` release event.
  - Assert `state.hotkey_display() == HotkeyDisplayMode::CtrlHeld`.
- `with_hotkey_display_alt_held_survives_modifier_release`
  - Same shape as above but with `AltHeld` and an Alt modifier release.
- `with_hotkey_display_ctrl_held_not_overwritten_by_alt_event`
  - Build state with `with_hotkey_display(HotkeyDisplayMode::CtrlHeld)`.
  - Send a `KeyEvent` carrying `KeyModifiers::ALT`.
  - Assert `state.hotkey_display() == HotkeyDisplayMode::CtrlHeld` (override wins, no flip to `AltHeld`).
- Preserve the existing auto-mode coverage:
  - Ensure a separate test still proves auto mode (no override) does transition to `CtrlHeld` on Ctrl press and back to `Hidden` on release, and that chord-fallback still arms a deadline that decays through `current_hotkey_display(now)`.

Mirror the same set of tests in `biscuit-tui/lib/src/components/choose_many.rs` against `ChooseManyState`.

### Focused Verification

Run:

```bash
cargo test -p tui-chrome hotkey
cargo test -p tui-chrome choose_one::tests
cargo test -p tui-chrome choose_many::tests
```

If exact module filters differ, fall back to `cargo test -p tui-chrome` and read output.

## Phase 2 - Reject Empty and Multi-Character Hotkey Specs

### Scope

Make `parse_hotkey_spec` and the surrounding normalization layer enforce the spec's single-character contract for `CTRL+`, `ALT+`, and `OPT+` prefixes, both for object-source `hotkey` fields and for bracketed CLI prefixes such as `[CTRL+R] Red`.

### Implementation Steps

1. Update `biscuit-tui/cli/src/choice_normalize.rs::parse_hotkey_spec` (line ~113).
   - Replace the three `let ch = rest.chars().next()?;` lines (lines ~117, ~121, ~125) with a helper that returns `Some(ch)` only when `rest` contains exactly one Unicode scalar value, else returns `None`. Suggested shape:

     ```rust
     fn single_char(rest: &str) -> Option<char> {
         let mut iter = rest.chars();
         let ch = iter.next()?;
         if iter.next().is_some() {
             return None;
         }
         Some(ch)
     }
     ```

   - Apply it inside each of the three modifier arms:

     ```rust
     if let Some(rest) = upper.strip_prefix("CTRL+") {
         let ch = single_char(rest)?;
         return Some(HotkeySpec::Ctrl(ch.to_ascii_lowercase()));
     }
     // Same shape for ALT+ and OPT+.
     ```

   - Result: `parse_hotkey_spec("CTRL+R")` -> `Some(HotkeySpec::Ctrl('r'))`, but `parse_hotkey_spec("CTRL+RED")`, `parse_hotkey_spec("CTRL+AB")`, `parse_hotkey_spec("ALT+")`, `parse_hotkey_spec("OPT+")` all return `None`.
   - Preserve case-insensitive modifier matching (`upper`) and ASCII-lowercase normalization for the hotkey character.

2. Decide bracketed-prefix failure behavior.
   - Inspect `extract_hotkey` (line ~92) and the surrounding label-normalization path (lines ~63 onward) and the object-hotkey path around line ~332.
   - Object-source `hotkey` strings already turn `parse_hotkey_spec` `None` into `NormalizeError::InvalidHotkey { spec, option }` at line ~335; that path now correctly fires for `CTRL+RED`, `ALT+`, and `OPT+AB` with no extra changes.
   - For bracketed prefixes in labels (the `extract_hotkey` path), prefer treating a recognized modifier prefix with an invalid suffix (`[CTRL+RED] Red`, `[ALT+] Red`) as an `InvalidHotkey` error rather than silently leaving the bracket text in the label. Concretely:
     - If `extract_hotkey` currently returns `(None, original_str)` when `parse_hotkey_spec` is `None`, change the call site (or `extract_hotkey` itself) to differentiate "no recognized modifier prefix at all" from "recognized modifier prefix with invalid suffix". The simplest local fix:
       - Inside `extract_hotkey`, when the bracket contents start (case-insensitively) with `CTRL+`, `ALT+`, or `OPT+`, but `parse_hotkey_spec` returns `None`, propagate that as a sentinel. Either change `extract_hotkey`'s signature to `Result<(Option<HotkeySpec>, &str), InvalidHotkeyKind>` or have the caller re-check the prefix and emit `NormalizeError::InvalidHotkey { spec, option }` with `spec = bracket_contents.to_string()` and `option = raw_label.to_string()`.
     - Keep unrelated bracket text (e.g., `[note] Red`) as ordinary label content; only modifier-prefixed brackets become errors.
   - Keep the error variant unchanged (`NormalizeError::InvalidHotkey { spec, option }`) so existing CLI error-rendering does not need to change.

3. Keep parser symmetry between bracketed and object sources.
   - Both code paths must end up calling the now-strict `parse_hotkey_spec` (directly or indirectly) so the wire formats stay consistent, as documented at `choice_normalize.rs:111`.

### Tests to Add or Adjust

In the existing `#[cfg(test)] mod tests` in `biscuit-tui/cli/src/choice_normalize.rs` (where `parse_hotkey_spec_canonical_forms` already lives at line ~448, and the existing invalid-hotkey coverage is around line ~868):

- Parser unit tests:
  - `assert_eq!(parse_hotkey_spec("CTRL+AB"), None);`
  - `assert_eq!(parse_hotkey_spec("CTRL+RED"), None);`
  - `assert_eq!(parse_hotkey_spec("ALT+"), None);`
  - `assert_eq!(parse_hotkey_spec("OPT+"), None);`
  - `assert_eq!(parse_hotkey_spec("CTRL+"), None);`
  - Regression: keep `parse_hotkey_spec("CTRL+R") == Some(HotkeySpec::Ctrl('r'))`, `parse_hotkey_spec("ctrl+x") == Some(HotkeySpec::Ctrl('x'))`, `parse_hotkey_spec("ALT+B") == Some(HotkeySpec::Alt('b'))`, `parse_hotkey_spec("OPT+B") == Some(HotkeySpec::Alt('b'))`, `parse_hotkey_spec("nope") == None`.

- Object-source normalization tests (mirroring the existing `Err(NormalizeError::InvalidHotkey { .. })` style at line ~878):
  - Object hotkey field `"CTRL+AB"` -> `Err(NormalizeError::InvalidHotkey { spec: "CTRL+AB", option: <label> })`.
  - Object hotkey field `"ALT+"` -> `Err(NormalizeError::InvalidHotkey { .. })`.
  - Object hotkey field `"OPT+AB"` -> `Err(NormalizeError::InvalidHotkey { .. })`.

- Bracketed-prefix normalization tests:
  - `[CTRL+AB] Red` -> `Err(NormalizeError::InvalidHotkey { .. })`.
  - `[ALT+] Red` -> `Err(NormalizeError::InvalidHotkey { .. })`.
  - `[CTRL+R] Red` continues to normalize successfully into a label `"Red"` with `HotkeySpec::Ctrl('r')`.
  - `[note] Red` continues to be treated as a normal label (no error, no hotkey).

If introducing a `Result` return on `extract_hotkey` would cascade into many call sites, prefer a localized check at the single label-normalization call site rather than rewriting the helper signature.

### Focused Verification

Run:

```bash
cargo test -p tui-chrome-cli choice_normalize
cargo test -p tui-chrome-cli hotkey
```

## Phase 3 - Full Package Verification and Lint Cleanup

### Scope

Prove the whole `biscuit-tui` package area is clean after Phase 1 and Phase 2.

### Required Verification Commands

Run from the repository root:

```bash
cargo test -p tui-chrome -p tui-chrome-cli
cargo clippy -p tui-chrome -p tui-chrome-cli --all-targets -- -D warnings
cargo test -p tui-chrome -p tui-chrome-cli
```

The first command must pass with zero failing tests. The second must pass with zero warnings (since `-D warnings` upgrades them to errors). The third reruns the full non-gated package tests after any clippy-induced edits to prove lint cleanup did not regress behavior.

### Optional Gated Verification

Review 8 explicitly noted that the gated PTY suites already pass:

```bash
RUN_PTY_TESTS=1 cargo test -p tui-chrome-cli --test keyboard_protocol -- --nocapture
RUN_SHELL_TESTS=1 cargo test -p tui-chrome-cli --test completions_shell -- --nocapture
```

Re-run them only as a sanity check that Phase 1 and Phase 2 did not perturb PTY behavior. Failures here are out of scope for this review unless they were caused by changes in this plan.

### Lint Expectations

- Fix all clippy warnings/errors in the `biscuit-tui` package area, even if they predate this change.
- Do not suppress lints unless the suppression is narrowly scoped and locally justified.
- After lint fixes, rerun the full non-gated package tests.

### Documentation Expectations

No README or component documentation update is required:

- The `with_hotkey_display` doc comments at `choose_one.rs:165` and `choose_many.rs:154` already document forced/lifetime-locking behavior; Phase 1 makes the code match.
- The hotkey-spec contract at `spec.md:117` and `spec.md:133` already specifies `CTRL+key` / `ALT+key` as single-character; Phase 2 enforces it.

## Completion Criteria

The review is complete when:

- `review-8.md` finding 1 is resolved: forced `--hotkey-badges` modes (`never`, `always`, `ctrl`, `alt`) survive both modifier-only press/release events and chord-fallback events for both `ChooseOneState` and `ChooseManyState`, with new tests covering each survival case.
- `review-8.md` finding 2 is resolved: `parse_hotkey_spec` rejects empty and multi-character suffixes for `CTRL+`, `ALT+`, and `OPT+`; both object-source `hotkey` fields and bracketed `[CTRL+...] Label` prefixes surface `NormalizeError::InvalidHotkey` for invalid specs, with new tests covering each case.
- `cargo test -p tui-chrome -p tui-chrome-cli` passes.
- `cargo clippy -p tui-chrome -p tui-chrome-cli --all-targets -- -D warnings` passes.
- The post-lint `cargo test -p tui-chrome -p tui-chrome-cli` rerun passes.
