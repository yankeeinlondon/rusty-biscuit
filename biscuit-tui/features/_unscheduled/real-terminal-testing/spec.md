# Real-Terminal Testing Utility

> Status: **Initial spec — unscheduled.** Captures the design intent so we don't lose the lessons learned while solving Level-3 testing for `biscuit-tui`. Implementation is a separate effort.

## Why this exists

Testing a CLI's behaviour inside a *real* terminal emulator — not a PTY mock, not `ratatui::TestBackend`, not raw bytes piped through a multiplexer — is the only way to catch a whole class of regressions: glyph width, SGR fidelity, cursor placement, scroll handling, and (most importantly) what the terminal's input encoder actually emits when the user presses a key. We have a Test Rigor model for this in [`cli-best-practices.md`](../../.claude/skills/cli/cli-best-practices.md):

| Level | Mechanism |
|---|---|
| 1 | In-process / PTY with manufactured input |
| 2 | Spawn binary inside a real terminal, capture rendered text via terminal CLI |
| 3 | OS-level keyboard injection into a spawned terminal window |

Solving Levels 2 and 3 *just for WezTerm on macOS* in `biscuit-tui` took about 30 iterations across one long debugging session. Most of the time was spent re-discovering things the terminal's docs don't tell you: how to target a specific window when the developer has many of the same terminal already open, why programmatic app activation fails silently, which permissions are needed where, why `set-window-title` is silently overridden, and so on.

**That work shouldn't be redone every time another CLI in this monorepo (or another project entirely) wants Level-2/3 verification.** The biscuit-tui harness is a working reference, but it's WezTerm-specific, monolithic, and tangled up with biscuit-tui-specific helpers. We need to extract a reusable utility that:

1. Handles every reasonable terminal a developer might have installed.
2. Detects available terminals at runtime and skips cleanly when missing.
3. Documents OS permission requirements once instead of at every call site.
4. Offers a single coherent API regardless of which terminal underlies the test.
5. Makes Level-3 hard limitations (e.g. macOS bare-modifier injection) explicit and pointed at the right Level-2 fallback.

## Target users

- CLI authors in this monorepo (`biscuit-tui`, `homelab`, `claudine`, `model-citizen`, `queue`, etc.) who want Level-2/3 coverage for terminal-rendering or input-encoding behaviour.
- External Rust CLI projects that take a dependency on the published crate.
- Library authors of TUI frameworks (ratatui, biscuit-terminal, darkmatter) who want to verify rendering parity across terminals.

Not in scope: replacing unit tests, replacing PTY tests, or providing a generic GUI testing framework.

## Target terminals

Initial implementations:

| Terminal | macOS | Linux | Windows | Notes |
|---|---|---|---|---|
| **WezTerm** | ✓ | ✓ | ✓ | Reference implementation already exists in biscuit-tui |
| **Kitty** | ✓ | ✓ | — | `kitten @ ls`/`focus-window`/`launch` — solid remote-control surface |
| **iTerm2** | ✓ | — | — | Python API is the canonical scripting surface; AppleScript is partial |
| **Apple Terminal** | ✓ | — | — | AppleScript only; minimal pane introspection |
| **Alacritty** | ✓ | ✓ | ✓ | No remote control; spawn-only, capture via OS screenshot or stdout redirection |
| **Ghostty** | ✓ | ✓ | — | Has IPC but limited; treat as remote-control-light initially |
| **tmux** | ✓ | ✓ | ✓ | Multiplexer, not a terminal — included because `tmux capture-pane -e` is the most portable Level-2 capture path on any host |

Stretch goals: Hyper, Warp, Windows Terminal, Konsole, GNOME Terminal, foot, Rio.

A given test run uses **whatever's installed**. The harness skips missing terminals cleanly; it never `#[ignore]`s a test for tooling availability.

## Core abstractions

The library exposes a `TerminalHarness` trait. Each supported terminal is a separate type implementing it. A facade type chooses an available implementation at runtime when the caller doesn't care which.

```rust
pub trait TerminalHarness {
    /// Probes the host for tooling + permissions. Cheap, idempotent.
    fn available() -> Availability where Self: Sized;

    /// Spawns `program` with `args` in a fresh window/pane. Returns
    /// when the process is running and the first frame has rendered.
    fn spawn(&mut self, program: &str, args: &[&str]) -> io::Result<()>;

    /// Sends raw bytes to the spawned pane's stdin (input encoder bypassed).
    /// For Level-2 tests that pipe escape sequences directly.
    fn send_text(&mut self, bytes: &[u8]) -> io::Result<()>;

    /// Captures rendered pane content. `raw` includes ANSI sequences;
    /// `plain` strips them.
    fn capture(&mut self) -> io::Result<CapturedFrame>;

    /// Brings the spawned window to the OS-level keyWindow position so
    /// subsequent OS keyboard injection lands in it. Returns the window's
    /// click-anchor screen coords on success.
    ///
    /// On platforms / terminals where this isn't possible (e.g. Alacritty
    /// without remote control, or non-macOS hosts), returns `Ok(None)`.
    fn focus_spawned_pane(&self) -> io::Result<Option<ScreenPoint>>;
}
```

A separate `KeyboardInjector` trait handles Level-3 OS-level events:

```rust
pub trait KeyboardInjector {
    fn click_at(&self, point: ScreenPoint) -> io::Result<()>;
    fn click_then_press(&self, point: ScreenPoint, modifier: Modifier) -> io::Result<()>;
    fn click_then_chord(&self, point: ScreenPoint, modifier: Modifier, key: char) -> io::Result<()>;
    fn release_modifier(&self, modifier: Modifier) -> io::Result<()>;
    fn type_text(&self, text: &str) -> io::Result<()>;
    fn press_key(&self, key: NamedKey) -> io::Result<()>;
}
```

Why split? Different OSes use different injectors (cliclick on macOS, xdotool on Linux). The terminal harness focuses on terminal interaction; the injector focuses on synthesising keyboard events. A test composes them.

### Capability matrix

Terminals differ on what they can do via remote control. The library exposes capabilities so tests skip gracefully:

```rust
pub struct Capabilities {
    pub spawn_isolated_window: bool,    // can it spawn a separate window?
    pub get_text: bool,                  // can it return rendered pane content?
    pub get_text_with_escapes: bool,     // does that content include ANSI?
    pub send_text_to_stdin: bool,        // can we inject input bytes?
    pub set_window_title: bool,          // can we stamp identity?
    pub focus_specific_window: bool,     // can we raise a *specific* window?
    pub query_window_geometry: bool,     // can we get screen coords?
}
```

Indicative matrix — to be refined during implementation:

| | Spawn | Get text | + escapes | Send text | Set title | Focus specific | Geometry |
|---|---|---|---|---|---|---|---|
| WezTerm | ✓ | ✓ | ✓ | ✓ | ✓ tab title | needs AXRaise + click | via System Events |
| Kitty | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ via `focus-window --match` | via OS API |
| iTerm2 | ✓ | partial | partial | partial | ✓ | ✓ via Python API | via Python API |
| Terminal.app | ✓ | partial | — | partial | ✓ | partial | partial |
| Alacritty | ✓ (CLI args) | — | — | — | ✓ via CLI flag | needs OS-level AXRaise | via System Events only |
| Ghostty | ✓ | ✓ partial | partial | partial | partial | partial | partial |
| tmux | ✓ session | ✓ | ✓ | ✓ | ✓ window/pane | via tmux only (intra-app) | — |

A test that needs `get_text_with_escapes` skips Alacritty cleanly; one that needs `focus_specific_window` may still pass via Kitty or WezTerm even when Alacritty is the only thing on PATH.

## OS-level concerns

These are platform-specific and live in their own modules so each gets dedicated documentation.

### macOS

Distilled from the biscuit-tui debugging session — see [`cliclick.md`](../../.claude/skills/cli/cliclick.md) for the long form.

**Permissions chain:**

| Permission | Granted to | Required for |
|---|---|---|
| Accessibility | The app hosting `cargo test` (e.g. iTerm2) | All synthetic events; AXRaise; AX queries |
| Apple Events / Automation | Same | `tell application "X" to activate` cross-app calls |
| Input Monitoring | Same | Some lower-level CGEvent taps |

The harness must surface clear error messages naming the missing permission and the System Settings path that grants it.

**Window targeting:**

- `tell application "X" to activate` is window-ambiguous when the app already owns multiple windows. macOS picks the most-recently-`keyWindow`, which is rarely the spawned test window.
- The reliable pattern is **stamp a unique identifier on the spawned window, then AXRaise that specific window** via System Events, then click into it via the OS-level injector. The click is what actually transfers `keyWindow`.
- `AXFocusedWindow` ≠ `AXMain`. Probe `AXFocusedWindow` in diagnostics — that's the actual `keyWindow` where keyboard events go.

**Parent-app vs spawned-app:**

If `cargo test` runs in WezTerm and the harness spawns more WezTerm windows, parent and child belong to the same `NSApplication` and compete for `keyWindow`. The harness wrapper must detect a same-app parent (`$TERM_PROGRAM`) and relaunch cargo inside a different terminal app via `osascript` before the test invokes the harness.

**Multi-monitor:**

System Events returns global screen coordinates; with a 2× 5K display arrangement (5120 px wide), x-coordinates above 4000 are valid. The harness must NOT treat large coordinates as off-screen failures.

**Bare-modifier injection limitation:**

cliclick (and AppleScript System Events `key down`) cannot synthesise the `flagsChanged` event type macOS routes bare-modifier presses through. **Bare-modifier Level-3 tests are structurally blocked on macOS until someone writes a `core_graphics::event`-based injector.** The harness should:
- Provide a chord-injection API that works (`click_then_chord(point, Ctrl, 'r')`).
- Provide a bare-modifier-press API that documents the limitation and points the caller at the Level-2 raw-bytes fallback (send the kitty-protocol bytes directly via `send_text`).
- Optionally: implement the `core_graphics::event` flagsChanged injector as a `flagschanged` feature flag for users who want to commit to it.

### Linux

- Wayland vs X11 is a real fork: `xdotool` is X11-only; Wayland needs `ydotool` (or `wtype`) and may require root or a separate daemon. The harness probes both and picks one.
- Per-terminal CLIs largely "just work" without the macOS permissions dance. The biggest unknown is whether terminals raise specific windows reliably under Wayland.

### Windows

- Out of scope for the initial implementation.
- Sketch: WezTerm CLI works; tmux doesn't exist; cliclick equivalent would be `User32.SendInput` via the `windows` crate; Windows Terminal has an IPC API but limited.

## Diagnostic helpers

The biscuit-tui session showed how much time goes into "did the focus transfer? did the click land? did the binary even see the bytes?". The library should ship first-class diagnostic helpers:

```rust
pub struct FocusReport {
    pub frontmost_app: String,
    pub focused_window_title: Option<String>,
    pub click_target: Option<ScreenPoint>,
}

pub fn current_focus() -> io::Result<FocusReport>;

/// Sends a single non-modifier key (e.g. arrow-down) and returns
/// whether *any* keyboard input is reaching the spawned binary.
/// Differentiates "harness broken" from "modifier-specific issue."
pub fn baseline_key_delivery_check(harness: &mut dyn TerminalHarness) -> bool;
```

Tests that fail at Level 3 should reach for `current_focus()` first — that one call resolves about 70% of failures by showing whether the click landed in the right app and window.

## Test classification

The library doesn't dictate test design, but it offers a `TestLevel` enum + helper macros so tests can self-document:

```rust
#[test_level(1)]  // PTY / unit
#[test_level(2)]  // Real terminal capture
#[test_level(3)]  // OS keyboard injection
```

These macros gate execution behind `RUN_LEVEL2=1` / `RUN_LEVEL3=1` and emit consistent skip messages. Level-1 tests always run.

## CLI front-end

Distribute alongside a CLI (`tterm-test` or similar) for ad-hoc local invocation:

```sh
tterm-test list-available           # which terminals does this host support?
tterm-test capabilities wezterm     # what can WezTerm do here?
tterm-test focus-probe              # current frontmost app + focused window
tterm-test spawn wezterm -- question choose-one Red Green Blue
tterm-test inject chord ctrl+r --target last-spawned
```

The CLI is a thin wrapper over the library — useful for poking at things during test development, debugging permission problems, and demoing the API.

## Naming and packaging

Working name: **`tterm-test`** (terminal-test). Open to alternatives.

Package layout follows the monorepo convention:

```
tterm-test/
├── lib/        # core abstractions, terminal implementations
├── cli/        # `tterm-test` binary
└── docs/       # per-terminal nuances, OS-level concerns, recipes
```

Sits at the same level as `biscuit-tui`, `biscuit-terminal`, etc.

## Out of scope (initial)

- Capturing terminal *pixel* output (screenshots, OCR) — text-level capture is enough for behaviour verification.
- Cross-host orchestration (running tests against terminals on a different machine).
- Mobile / TouchID / FaceID prompts during permission grants.
- Per-test recording of full session video.
- Integration with `cargo-nextest` retry policies for terminal-flake — leave it to the user.

## Lessons baked into this spec

These come directly from the biscuit-tui Level-3 debugging session and represent traps the library should make impossible (or at least obvious):

1. **A "focus failure" is rarely a focus failure.** It's typically: (a) wrong window targeted, (b) parent-app ate the events, (c) a permission isn't granted, (d) the binary isn't bound to the test's chord. The library's diagnostic helpers should surface (a)–(d) directly so the user doesn't chase phantoms.
2. **Title stamping is fragile.** Each terminal has its own title-propagation rules: WezTerm derives OS title from tab title (with most users' configs); Kitty respects `--title`; Alacritty respects `--title`. The library encapsulates this so callers just say "stamp this window with X."
3. **Click-to-focus beats activate-to-focus across apps.** AXRaise + click is the only reliable way on macOS to make a specific window of an app the `keyWindow` from a different parent app.
4. **Batched event injection beats sequential calls.** Multiple separate cliclick processes leave focus-drift windows. The library's `click_then_chord(...)` does it in one cliclick invocation.
5. **`--test-threads=1` is mandatory** for tests that each spawn their own GUI window. The library's macros / helpers should surface this requirement loudly.
6. **A "plain key delivers" baseline test is the single most useful Level-3 diagnostic.** Always include one in any Level-3 suite — it localises any failure to either the harness layer (broken) or the modifier/chord layer (config issue).
7. **Bare-modifier Level-3 testing on macOS is currently unsolvable from userspace.** Don't pretend otherwise. Mark it `#[ignore]`, point at the Level-2 raw-bytes fallback, document the future fix path (`core_graphics` flagsChanged injector).

## Open questions

To resolve before implementation:

1. **Crate name and scope.** `tterm-test`? `terminal-harness`? Sit inside this monorepo or publish standalone? If standalone, dependency policy?
2. **Sync vs async API.** Most existing test harnesses are sync. A `tokio`-based async API would let tests run several harnesses in parallel — useful for cross-terminal verification but adds complexity. Probably sync-first with async as a feature flag.
3. **How tightly to model capabilities.** The capability struct above is one approach. Another is per-method `Result<_, NotSupported>`. Trade-off: ergonomic API vs explicit upfront skip.
4. **Snapshot integration.** Should we ship a wrapper over `insta` for capture-and-snapshot? Or stay framework-agnostic?
5. **Linux Wayland fallback.** Worth implementing day-one or defer until someone needs it?
6. **`core_graphics` flagsChanged injector.** Build it as part of the initial work or leave for a follow-up? The question is whether bare-modifier Level-3 on macOS is a launch blocker or a known-limitation.
7. **Terminal version pinning.** Some features only exist in recent terminal versions (e.g. `kitten @ focus-window --match` matchers, WezTerm's `set-tab-title` behaviour). Should the library probe versions and adapt? Document minimums?

## References

- [`biscuit-tui/cli/tests/common/real_terminal/`](../../../cli/tests/common/real_terminal/) — current reference implementation for WezTerm + Kitty + tmux harnesses, plus cliclick injector and the iTerm2 relaunch trick in `biscuit-tui/justfile`.
- [`.claude/skills/cli/cli-best-practices.md`](../../../../.claude/skills/cli/cli-best-practices.md) — Test Rigor / Level 1-2-3 model, multi-window WezTerm targeting, permissions chain, diagnostic recipes.
- [`.claude/skills/cli/cliclick.md`](../../../../.claude/skills/cli/cliclick.md) — cliclick deep dive: gotchas, WezTerm-specific behaviour, bare-modifier limitation.
- [WezTerm CLI](https://wezterm.org/cli/cli/index.html), [Kitty remote control](https://sw.kovidgoyal.net/kitty/remote-control/), [iTerm2 Python API](https://iterm2.com/python-api/).
