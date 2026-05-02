# Review 6 Implementation Plan

This plan implements every recommendation from
`biscuit-tui/features/2026-04-28-choose-one-improvements/review-6.md` for the
`biscuit-tui` package area. The affected crates are `tui-chrome` in
`biscuit-tui/lib` and `tui-chrome-cli` in `biscuit-tui/cli`.

## Source References

- Spec: `biscuit-tui/features/2026-04-28-choose-one-improvements/spec.md`
- Technical design: `biscuit-tui/features/2026-04-28-choose-one-improvements/tech-design.md`
- Review: `biscuit-tui/features/2026-04-28-choose-one-improvements/review-6.md`

## Current Review Gaps

1. `drive_event_loop_with_hint` and `drive_event_loop_with_chrome` skip every
   `KeyEventKind::Release`, so modifier-only release events never reach
   `ChooseOne` or `ChooseMany`. This can leave hotkey badges stuck visible in
   terminals using keyboard enhancement events.
2. Default Ctrl hotkeys are not implemented for ordinary options. The spec
   requires a plain option like `Red` to have an effective `Ctrl+R` hotkey unless
   the caller explicitly assigns a different hotkey.
3. `question choose-one --list "- Apple\n- Banana"` preserves Markdown bullet
   markers, despite the spec documenting bullet-style list input.

## Phase 1: Deliver Modifier Release Events Through the Runner

### Goal

Make the standalone event loop honor the keyboard protocol contract: modifier
press and release events must reach components so badge visibility can turn on
and off deterministically.

### Likely Files

- `biscuit-tui/lib/src/core/standalone.rs`
- Existing component tests in:
  - `biscuit-tui/lib/src/components/choose_one.rs`
  - `biscuit-tui/lib/src/components/choose_many.rs`

### Implementation Steps

1. Replace the unconditional release-event skip in both event loops:
   - `drive_event_loop_with_hint`
   - `drive_event_loop_with_chrome`
2. Preserve the old behavior for normal key releases if desired, but allow
   modifier-only releases through. A conservative helper is recommended:
   - `fn should_dispatch_key_event(key: &KeyEvent) -> bool`
   - Return `true` for press/repeat events.
   - Return `true` for `KeyEventKind::Release` only when
     `matches!(key.code, KeyCode::Modifier(_))`.
   - Return `false` for non-modifier release events.
3. Keep runner-level Ctrl-C handling before component dispatch for normal Ctrl-C
   press events. Do not map modifier-only release events to Ctrl-C.
4. Update or replace the existing `key_release_events_are_skipped` test so it
   still proves ordinary key releases do not mutate state.

### Test Coverage

Add deterministic runner-level tests in `standalone.rs`:

- A modifier press followed by modifier release reaches the component and causes
  two consumed events before submit.
- The same behavior is covered through `drive_event_loop_with_chrome`.
- Ordinary character release events remain ignored or skipped, so release noise
  does not append characters or force redraws.

The existing component tests for `ChooseOne` and `ChooseMany` modifier release
handling should remain passing; the new runner tests close the integration gap.

### Completion Criteria

- Modifier-only release events are no longer dropped before component handling.
- Runner tests fail on the reviewed implementation and pass after the fix.
- No regression to Ctrl-C, Esc, ignored-event, or redraw behavior.

## Phase 2: Implement Effective Default Ctrl Hotkeys

### Goal

Make default Ctrl hotkeys work consistently for library and CLI callers:
ordinary enabled options receive an effective `HotkeySpec::Ctrl(first_label_char)`
unless an explicit `ChoiceOption::hotkey` is present.

### Likely Files

- `biscuit-tui/lib/src/components/choose.rs`
- `biscuit-tui/lib/src/components/choose_one.rs`
- `biscuit-tui/lib/src/components/choose_many.rs`
- `biscuit-tui/lib/src/components/choice_render.rs`
- `biscuit-tui/lib/src/components/choice_layout.rs`
- `biscuit-tui/cli/src/choice_normalize.rs`
- `biscuit-tui/cli/tests/keyboard_protocol.rs`

### Implementation Steps

1. Add one shared helper for the effective hotkey. Prefer placing it near the
   choice model types so rendering, layout, and state construction share one
   rule, for example:
   - `ChoiceOption::effective_hotkey() -> Option<HotkeySpec>`
   - or `effective_hotkey(option: &ChoiceOption<_>) -> Option<HotkeySpec>`
2. Effective hotkey rule:
   - Disabled options return `None`.
   - Explicit `option.hotkey` wins.
   - Otherwise choose the first usable character from the option label and return
     `HotkeySpec::Ctrl(char.to_ascii_lowercase())`.
   - Ignore labels that have no character suitable for a hotkey.
3. Use the helper when building `ChooseOneState` Ctrl/Alt maps. This should make
   `Ctrl+R` submit a plain `Red` option.
4. Apply the same effective-hotkey behavior to `ChooseMany` dispatch. Current
   `ChooseMany` uses a legacy plain-letter map; update it so Ctrl/Alt chords
   route through effective hotkeys without changing the existing plain-letter
   fallback unless tests prove it is intentionally removed elsewhere.
5. Use the helper in `choice_render.rs` for badge width and badge rendering so
   default Ctrl badges appear when Ctrl or Alt is held.
6. Use the helper in `choice_layout.rs` wherever badge width participates in
   horizontal packing. This prevents default badges from rendering beyond the
   measured item width.
7. Review `choice_normalize.rs` after the library-level helper is in place.
   Avoid materializing default hotkeys only in the CLI; library callers must get
   identical behavior. CLI explicit prefixes and `--numeric-hot-keys` should
   still set `ChoiceOption::hotkey` and therefore override the default.
8. If duplicate effective hotkeys occur, keep the current map behavior unless
   existing normalization rejects duplicates before options reach the library:
   the first enabled option wins for library state maps. Do not introduce a new
   runtime error in the library for this review fix.

### Test Coverage

Add or update library tests:

- `ChooseOneState::new` on plain `Red`, `Green` exposes effective Ctrl mappings
  for `r` and `g`.
- `ChooseOne` submits `Red` on `Ctrl+R` with no explicit hotkey.
- `ChooseOne` renders a Ctrl badge for plain `Red` while Ctrl is held.
- Explicit Alt hotkeys still override the default Ctrl mapping and render as Alt
  badges.
- Disabled options do not receive default Ctrl hotkeys.
- `ChooseMany` toggles or selects according to its existing semantics when an
  effective default Ctrl hotkey is pressed, and explicit Alt hotkeys still work.
- Horizontal layout tests include default hotkey badge width in packing.

Add or update CLI/integration tests:

- `keyboard_protocol.rs` should use plain `choose-one Red Green Blue` and still
  assert that a bare Ctrl press reveals badges. This makes the PTY coverage
  exercise the spec-required default hotkeys instead of only explicit hotkeys.
- Existing prefix and `--numeric-hot-keys` tests must continue to pass.

### Completion Criteria

- Plain options have effective Ctrl hotkeys in both component behavior and
  rendering.
- Explicit Ctrl/Alt hotkeys retain priority.
- CLI callers using positional, `--csv`, `--list`, `--rows`, stdin, or file
  sources receive default Ctrl hotkeys without CLI-specific duplication.

## Phase 3: Strip Markdown Bullets in `--list` Sources

### Goal

Make `--list` match the documented spec example by accepting simple Markdown
bullet and numbered-list lines while preserving plain line input.

### Likely Files

- `biscuit-tui/cli/src/option_sources.rs`
- Possibly `biscuit-tui/docs/components/choose_one.md` or CLI docs only if they
  are found to contradict the chosen behavior.

### Implementation Steps

1. Update `parse_list` to normalize each non-empty line before converting it to
   `RawOption`.
2. Strip common Markdown markers after leading indentation:
   - `- Apple`
   - `* Apple`
   - `+ Apple`
   - `1. Apple`
   - `1) Apple`
3. Preserve non-list lines exactly after existing CR handling and whitespace
   policy. Do not strip hyphens from legitimate values such as `--flag` or
   `north-east`; require a marker followed by whitespace.
4. Decide whether `parse_rows` should inherit the same behavior because it calls
   `parse_list`. If rows are intended to be raw line/value rows, split the
   common helper so `--rows` remains unchanged. The review only requires
   `--list`; keep `--rows` raw unless the existing tests/spec already expect
   shared behavior.
5. Confirm stdin behavior. Since stdin currently calls `parse_list`, it will get
   the same bullet-stripping behavior unless deliberately split. That is likely
   acceptable because stdin is list-like.

### Test Coverage

Add `option_sources.rs` unit tests:

- `parse_list` strips dash, star, and plus bullets.
- `parse_list` strips ordered markers with `.` and `)`.
- `parse_list` leaves plain lines unchanged.
- `parse_list` does not strip non-marker hyphens such as `--flag`.
- If `parse_rows` is kept raw, add a test proving `parse_rows("Red::apple")`
  still preserves the line for later delimiter handling.

Add a CLI-level test if the existing CLI tests include non-interactive source
normalization:

- A command path using `--list "- Red\n- Green"` should build labels/values
  without the leading bullet. If the CLI cannot submit non-interactively without
  a PTY helper, the unit tests are sufficient for this review item.

### Completion Criteria

- The documented `--list "- Apple\n- Banana\n- Cherry"` form displays and
  returns `Apple`, `Banana`, and `Cherry`.
- Existing `--rows`, `--csv`, file, Markdown-frontmatter, hotkey-prefix, and
  delimiter behavior does not regress.

## Phase 4: Package-Wide Validation and Lint Cleanup

### Goal

Prove the whole `biscuit-tui` package area is test-clean and lint-clean after
the review fixes.

### Required Commands

Run from the repository root:

```bash
just -f biscuit-tui/justfile check
cargo test -p tui-chrome -p tui-chrome-cli -- --skip completions_shell --skip keyboard_protocol
cargo test -p tui-chrome -p tui-chrome-cli
just -f biscuit-tui/justfile lint
```

If `completions_shell` or `keyboard_protocol` fail because the local environment
lacks a required shell, PTY capability, or keyboard-protocol support, record the
failure and rerun the deterministic subset:

```bash
cargo test -p tui-chrome -p tui-chrome-cli -- --skip completions_shell --skip keyboard_protocol
```

Lint errors anywhere in `tui-chrome` or `tui-chrome-cli` must be fixed, even if
they predate these review changes. After lint fixes, rerun:

```bash
cargo test -p tui-chrome -p tui-chrome-cli -- --skip completions_shell --skip keyboard_protocol
just -f biscuit-tui/justfile lint
```

### Completion Criteria

- All deterministic tests pass.
- Gated shell/PTY tests either pass or have a clearly recorded environmental
  reason for not passing.
- `just -f biscuit-tui/justfile lint` passes with no warnings or errors for both
  `tui-chrome` and `tui-chrome-cli`.
- No unrelated workspace packages are modified to satisfy this review.

## Developer Handoff Notes

- Keep changes scoped to the three reviewed issues. The feature implementation
  is otherwise broad and already covered by tests.
- Prefer shared helpers over duplicating effective-hotkey logic in the CLI,
  rendering, layout, and state construction.
- Be careful with the existing legacy plain-letter hotkey map. This review asks
  for Ctrl as the default chord association; it does not explicitly ask to remove
  legacy plain-letter behavior.
- The final developer summary should include changed files, tests added, exact
  validation commands run, and any gated tests that could not be executed in the
  local environment.
