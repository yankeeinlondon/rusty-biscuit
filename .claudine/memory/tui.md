---
description: captures knowledge learned while implementing TUI functionality
last_updated: 2026-05-01
---
# TUI Memory

## Crossterm

### Keyboard Enhancement Flags (Kitty Protocol)

Pushing `KeyboardEnhancementFlags::REPORT_EVENT_TYPES | DISAMBIGUATE_ESCAPE_CODES` is **not enough** to receive bare-modifier press/release events. Most kitty-aware terminals (notably WezTerm) only emit modifier events as part of a chord under those two flags alone — holding bare `Ctrl` produces no event. **`REPORT_ALL_KEYS_AS_ESCAPE_CODES` must also be pushed** for press/release of standalone modifier keys to be reported.

The push/pop pair must be symmetric: only pop the flags when the push succeeded. crossterm returns `Ok` from the `execute!(PushKeyboardEnhancementFlags(...))` invocation when the terminal acknowledged the request — store that boolean and only call `PopKeyboardEnhancementFlags` if it's `true`.

### `Ctrl+<key>` arrives as the *control-coded byte*, NOT `Char(<key>) + CONTROL`

This is a cross-platform encoding contract that every legacy / VT-style terminal honors: pressing Ctrl subtracts `0x40` from the key code. Examples:

| Chord | Legacy byte | crossterm reports |
|---|---|---|
| `Ctrl+Space` (`0x20 - 0x40`) | `0x00` (NUL) | `KeyCode::Char('\0') + CONTROL` |
| `Ctrl+H` | `0x08` (BS) | `KeyCode::Backspace` |
| `Ctrl+I` | `0x09` (TAB) | `KeyCode::Tab` |
| `Ctrl+J` / `Ctrl+M` | `0x0a` / `0x0d` | `KeyCode::Enter` |
| `Ctrl+@` | `0x00` (NUL) | same as `Ctrl+Space` |

Under kitty keyboard protocol with `DISAMBIGUATE_ESCAPE_CODES`, the same chord arrives in the "intuitive" form: `KeyCode::Char(' ') + CONTROL`. **A matcher for `Ctrl+Space` must accept both forms** — `KeyCode::Char(' ')` AND `KeyCode::Char('\0')` (with CONTROL set) — otherwise users on any non-kitty path press the chord, the bytes reach the binary, and the handler silently drops the event. Same holds for any `Ctrl+<symbol-or-control-char>` chord across macOS, Linux, and Windows.

A spec-compliant test for `Ctrl+Space` MUST send the `\0`-form event and assert the handler fires.

### `KeyCode::Modifier` events

A bare-modifier press under kitty protocol arrives as `KeyEvent { code: KeyCode::Modifier(ModifierKeyCode::LeftControl), kind: Press, modifiers: KeyModifiers::CONTROL, ... }`. Match on `KeyCode::Modifier(_)` to detect bare-modifier events specifically — don't filter on `modifiers.contains(CONTROL)` alone, that fires for chords too.

### Should-dispatch filter

Press and Repeat events should always be dispatched. Release events should usually be filtered out except when the released key is a modifier — modifier releases are how the badge-display state machine knows to clear:

```rust
fn should_dispatch_key_event(key: &KeyEvent) -> bool {
    match key.kind {
        KeyEventKind::Press | KeyEventKind::Repeat => true,
        KeyEventKind::Release => matches!(key.code, KeyCode::Modifier(_)),
    }
}
```

## Ratatui

### `Modifier::DIM` is unreliable

Do **not** use `Modifier::DIM` to indicate de-emphasised text — it renders inconsistently across terminals. WezTerm's default theme makes DIM nearly invisible against a dark background, which made a "DIM-on-bright orange" treatment look identical to bold-on-bright. To distinguish a "held" from "not-held" badge state without DIM, use:

- A darker BG shade of the same family colour (e.g. `Color::Indexed(166)` for dim orange vs `Color::Indexed(208)` for held orange).
- Plus `Modifier::BOLD` only on the held state.

### White on yellow is illegible

`Color::White` foreground on a bright yellow background (`Color::Indexed(220)`) is very low contrast — the text reads as a yellow blur on most terminal themes. **Use `Color::Black` foreground on yellow** badge backgrounds. White stays correct on orange.

Lesson: define family-aware FG constants per-BG-color rather than a single global FG. e.g. `BADGE_FG_ON_ORANGE = Color::White`, `BADGE_FG_ON_YELLOW = Color::Black`.

### Buffer cells default to `bg = Some(Color::Reset)`

Once a styled span is rendered into a `ratatui::Buffer`, the background of an "unstyled" cell shows up as `Some(Color::Reset)`, not `None`. Test assertions like `assert_eq!(style.bg, None)` for "no background fill" are wrong; use `matches!(style.bg, None | Some(Color::Reset))`.

### `strip_ansi` must handle ESC-intermediate-final

Naive ESC-stripper that treats every escape sequence as `ESC + 1 byte` will leave the *final* byte of a 3-byte sequence behind. WezTerm emits `ESC ( B` (designate ASCII as G0) liberally around styled regions; a naive stripper turns it into stray `B` characters that masquerade as text in captured output. Implement ECMA-48 §5.4: `ESC (intermediate 0x20-0x2F)* (final 0x30-0x7E)`.

### Layout for inside-the-border footer rows

When a component needs to render its own footer (legend, status, etc.) **inside** the FrameChrome border, the component itself must reserve the row from its inner-area `Rect` before passing it to the renderer. Painting in the standalone runner *after* the chrome has already drawn lands the footer outside the border.

```rust
// Inside the widget's `render(rect, ...)` impl:
let mut list_area = rect;
let legend_y = if want_legend && list_area.height >= 2 {
    let y = list_area.bottom().saturating_sub(1);
    list_area = Rect::new(list_area.x, list_area.y, list_area.width, list_area.height - 1);
    Some(y)
} else { None };
// ... render options into the now-shorter list_area
if let Some(y) = legend_y { /* paint legend at y */ }
```

## Terminal Output

### macOS intercepts `Ctrl+Space`

On a default macOS install, `Ctrl+Space` is bound to "Select previous input source" at the OS level. The chord never reaches the terminal — your binary will see nothing. Provide an alternative chord (e.g. `Ctrl+/`) for users who can't or won't disable the OS shortcut.

### `enable_kitty_keyboard = true` in WezTerm is necessary but not sufficient

This setting only authorises WezTerm to honour application requests for the kitty keyboard protocol. The application still has to push the right `KeyboardEnhancementFlags` (see Crossterm section). Both pieces are required for bare-modifier reporting.

## Testing TUIs

### Test rigor: Level 1 / Level 2 / Level 3

Test count is not test rigor. A feature with hundreds of unit tests can still ship with a glaring user-visible bug if none of the tests exercise the right layer. Three levels:

1. **Level 1** — unit tests + PTY (`expectrl`) with manufactured input bytes. Verifies binary's parsing, state, rendering. Does NOT prove the terminal's encoder/decoder works — *you* generate the bytes.
2. **Level 2** — spawn binary inside a real terminal/multiplexer (`wezterm cli`, `kitty @`, `tmux`) and capture the rendered pane text. Verifies real terminal rendering. Input still byte-injected via the terminal's CLI.
3. **Level 3** — OS-level keyboard injection (`cliclick` macOS, `xdotool` Linux) into the spawned terminal window. The terminal's *input encoder* fires.

A "when the user holds modifier X, behaviour Y" requirement covered only by Level 1 is **not production-ready** — the terminal's encoder is structurally invisible to Level 1.

### cliclick + bare modifier keys is unreliable on macOS

`CGEventCreateKeyboardEvent` events for bare-modifier keys often fail to propagate through `flagsChanged` to AppKit applications. Symptoms: the chord injection (`kd:ctrl t:r ku:ctrl`) works perfectly because the modifier flag rides along with the letter `keyDown`, but a bare `kd:ctrl` alone produces no observable effect in WezTerm's pane.

**Workaround**: for verifying that the binary correctly handles bare-modifier kitty bytes, send the literal escape sequence through `wezterm cli send-text` (Level 2) instead of relying on cliclick (Level 3):

```rust
harness.send_text(b"\x1b[57442;1u")?;  // kitty: bare LeftControl press
std::thread::sleep(Duration::from_millis(200));
let frame = harness.capture()?;
let _ = harness.send_text(b"\x1b[57442;1:3u");  // release
assert!(frame.plain.contains("^R"));
```

This proves the binary's handling end-to-end through real terminal rendering. It does NOT prove the terminal *emits* those bytes when a real keyboard is pressed (only a real keyboard test can prove that).

### Capture during the modifier hold, not after

If the binary clears its display state on modifier release (which it should), a `hold_modifier(800ms)` helper that internally sequences press → sleep → release leaves *nothing observable* by the time `capture()` runs. Split `key_down` and `key_up` and capture between them:

```rust
cliclick::key_down("ctrl")?;
std::thread::sleep(Duration::from_millis(300));  // let event round-trip
let frame = harness.capture()?;
let _ = cliclick::key_up("ctrl");  // release before any panic to avoid stuck modifier
```

### Always release modifiers via `let _ =` before any potential panic

A panicking test that left Ctrl/Alt held would leave the user's machine in a stuck-modifier state. Wrap the release in `let _ = ...` so it runs even on assertion failure (or use a guard struct with `Drop`).

### `RUN_LEVEL3=1` env-gate, not `#[ignore]`

`#[ignore]` makes Level-3 tests opaque (they show as ignored in every output line forever). Env-gating with `RUN_LEVEL3=1` lets the test body print a clear `skipping: requires X` line and return `ok` so the test is visibly part of the suite but only attempts the work when the developer opts in. macOS focus is a shared global resource; running cliclick during a normal `cargo test` causes random failures from focus thrash.

### Skip semantics across levels

Each harness's `available()` probe should test for the binary on `$PATH` plus any required env (`WEZTERM_UNIX_SOCKET`, `KITTY_LISTEN_ON`). Tests that depend on it print `skipping: requires <X>` to stderr and return — no `#[ignore]`, no spurious failures on contributor machines that lack the tooling.

### Test assertions must match the right thing

The original kitty-modifier PTY test in this repo asserted `has_badge OR contains("Ctrl") OR contains("CTRL")`. The OR-fallbacks made the test pass even when no badge rendered, because incidental text in a help hint contained "Ctrl". A regression in the badge code would have been invisible. Lesson: when an assertion has an `||` branch that could fire on unrelated text, the test is lying. Tighten to the actual feature.

### Per-terminal harness pattern

`cli/tests/common/real_terminal/` defines a `TerminalHarness` trait with `spawn / send_text / capture` and per-terminal implementations:

- `WezTermHarness` — `wezterm cli spawn --new-window`, `wezterm cli send-text`, `wezterm cli get-text --escapes`. Requires `WEZTERM_UNIX_SOCKET` set.
- `KittyHarness` — `kitty @ launch`, `kitty @ send-text --from-file /dev/stdin`, `kitty @ get-text --ansi`. Requires `KITTY_LISTEN_ON` set.
- `TmuxHarness` — `tmux new-session -d`, `tmux send-keys -l` for literal bytes, `tmux send-keys C-space` for symbolic chord names, `tmux capture-pane -p -e`. Most portable; runs headlessly.

For Level-3, an additional `cliclick` helper module wraps `kd:`, `ku:`, `kp:`, and `t:`. Add a `key_down` / `key_up` pair separate from `hold_modifier` so callers can capture between press and release.

### tmux + `Command::arg` rejects `\x00`

`tmux send-keys -l` with a NUL byte argument is rejected by Rust's `std::process::Command::arg` — you cannot pass nul bytes through the argv layer. Use `tmux send-keys C-space` (no `-l`) for control chords; tmux's key-name translation handles the byte mapping internally.

### Don't trust the chrome to leave room for footers

When testing layouts with optional border + optional footer + optional help-hint, write tests that exercise short heights (3, 4, 5 rows) — each addition compresses the available area and a careless layout can clip rendering or panic on subtraction. The `saturating_sub` / `if height >= N` guards matter.

## Other

### Spec language: "duplicate" is ambiguous without scope

A spec line "reject duplicate hotkeys at parsing time" can read in many ways. When the spec uses a category word ("duplicates", "collisions", "conflicts"), it MUST say *what counts*. The spec for this feature should have included an explicit table of which collision shapes error and which don't, rather than letting an implementer pick a stricter reading. The right rule here: only **explicit-vs-explicit** collisions error; everything else either has no hotkey at all (plain labels) or never collides.

### Modifier-press emphasis and the discoverability problem

Spec said: "when the user holds Ctrl, all hotkeys show; Ctrl-bound ones are emphasised; Alt-bound ones are de-emphasised." But on terminals where bare-modifier events don't actually arrive, holding Ctrl produces no visible change — the user has no way to know hotkeys exist. Mitigations:

1. An always-visible **legend** ("key") row inside the prompt that shows what each colour means. Implemented in `lib/src/components/choice_render.rs::hotkey_legend_line()` — only renders when at least one option has an explicit hotkey. Render it inside the FrameChrome border by reserving the bottom row of the widget's inner area, NOT by appending after chrome rendering.
2. A **portable `Ctrl+Space` / `Alt+Space` chord** that toggles the corresponding emphasis mode — same observable effect as holding the bare modifier. macOS by default binds `Ctrl+Space` to "Select previous input source" so users may need to disable the OS shortcut for it to reach the terminal.

### `wezterm cli activate-pane` plus `osascript activate WezTerm` is the right focus dance

For Level-3 tests, focusing the spawned pane requires both:
1. `wezterm cli activate-pane --pane-id N` — selects the pane within WezTerm.
2. `osascript -e 'tell application "WezTerm" to activate'` — raises WezTerm to the front of the macOS window stack.

Then a 300-400 ms `sleep` so the WindowServer settles focus before injecting keys.
