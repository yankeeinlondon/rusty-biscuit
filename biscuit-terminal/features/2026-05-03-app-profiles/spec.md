# Spec: Centralized Terminal App Profiles

**Status:** Unscheduled
**Author:** Claude (Opus 4.7) for Ken Snyder
**Date:** 2026-05-03
**Related:** `biscuit-terminal/features/2026-05-02-l1-l2-tests/spec.md`,
  `biscuit-terminal/features/2026-05-02-apple-terminal/spec.md`

## 1. Why this exists

biscuit-terminal advertises itself as the authority on terminal
capability detection across 13+ emulators. In practice the *static*
facts about each emulator — which graphics protocol it supports,
whether its OSC8 implementation works, what IPC mechanism it exposes,
whether it is single-instance or multi-process, the rounding rule it
uses for image cell math — live as scattered match arms across at
least six modules:

| Static fact | Where it lives today |
|---|---|
| Image protocol per app | `discovery/detection.rs` (`KITTY_TERMINALS` constant + iTerm2 special-cases) |
| OSC52 clipboard support | `discovery/clipboard.rs` (match arm at line 73) |
| Mode 2027 grapheme support | `discovery/mode_2027.rs` (match arm at line 68) |
| OSC color query support | `discovery/osc_queries.rs` (match arms at lines 280, 451) |
| Config file location | `discovery/config_paths.rs` (match arm at line 62) |
| Image rounding rule (ceil vs floor) | `components/terminal_image.rs` (in-method conditionals + monorepo memory) |
| Scroll-compensation policy | `components/terminal_image.rs` (in-method conditionals) |
| OSC8 link support | `Terminal.osc_link_support` (runtime field; no static profile) |
| Italics / underline / color depth | runtime fields populated by mixed env / terminfo / app-name lookups |

There is no single-source-of-truth profile keyed by `TerminalApp`.
Adding a new app — Apple Terminal and Alacritty are imminent (see
related specs), and Foot/mlterm/Contour/Wast already exist in the
enum — means hunting through every match arm to add the missing
case. Adding a *new capability* (e.g. Sixel, recently raised) means
deciding which scattered file owns it. Both are error-prone and
neither is reviewable as a single change.

The L1/L2 testing work also surfaced metadata that biscuit-terminal
does not currently model at all — IPC mechanism, capture fidelity,
single- vs multi-instance behavior, backgroundability. The harness
needs that data to decide what to do per app; today it duplicates
the knowledge in a parallel set of `available()` probes.

A central `TerminalAppProfile` resolves both problems and gives
biscuit-terminal a public API surface that consumers (the harness,
biscuit-tui, downstream CLIs) can query for static facts without
running detection.

### What "static" means here

A profile is the **vendor-determined** behavior of a given emulator
build — facts that do not change between runs on the same version.
*Runtime* values stay on the `Terminal` struct: window size, current
font, locale, whether stdout is a TTY, the result of an OSC color
probe, etc. The profile answers "what is Apple Terminal *capable of*?";
the `Terminal` instance answers "what is the user's session *doing
right now*?".

## 2. Goals

- Introduce a `TerminalAppProfile` type holding all static per-app
  capability facts, exposed as `&'static` data via
  `TerminalApp::profile()`.
- Migrate the scattered match arms in `discovery/{detection,
  clipboard, mode_2027, osc_queries, config_paths}.rs` and the
  rounding/scroll conditionals in `components/terminal_image.rs` to
  consult the profile instead of re-encoding per-app knowledge.
- Add metadata that today is not modeled at all but is observably
  load-bearing: Sixel support, IPC mechanism, capture fidelity,
  backgroundability, instance model, image rounding rule,
  scroll-compensation policy, vendor URL, default config paths.
- Keep the existing `Terminal` instance API stable — the runtime
  `Terminal` struct continues to expose `osc_link_support`,
  `image_support`, `supports_italic`, etc. exactly as today; the
  fields are now populated *from* the profile (with env-driven
  override hooks where detection genuinely varies).
- Give the test harness a single place to ask "does app X have IPC?
  what env var advertises it? does its capture preserve ANSI?".

## 3. Non-goals

- Not a behavioral change to detection. The same TERM/TERM_PROGRAM
  inputs must still produce the same `TerminalApp` and the same
  `Terminal` populated fields.
- Not a public-API breakage of `TerminalApp`. New variants (e.g.
  Sixel-only emulators) and new fields on `Terminal` are additive.
- Not the place to add Sixel *rendering* support. This spec adds the
  Sixel *capability bit* to the profile and `ImageSupport` enum;
  actually emitting Sixel bytes is a separate body of work.
- Not the place to add new emulators (Foot, mlterm, etc.) — those
  variants already exist or are tracked in the apple-terminal spec.
  This spec only structures what we know.
- Not a refactor of the harness. The L1/L2 spec retains its own
  `available()` methods; the harness *may* consult profiles in a
  follow-up but does not depend on this spec.

## 4. Scope of work

### 4.1 New types in `discovery/profile.rs`

A new module owns the profile type and its supporting enums.

```rust
// biscuit-terminal/lib/src/discovery/profile.rs

use std::path::PathBuf;
use crate::discovery::detection::{
    ColorDepth, ImageSupport, TerminalApp, UnderlineSupport,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalAppProfile {
    pub app: TerminalApp,
    pub display_name: &'static str,
    pub vendor_url: &'static str,
    pub platforms: &'static [Platform],

    // Rendering capabilities
    pub graphics: GraphicsCapability,
    pub supports_osc8: TriState,
    pub supports_osc52: bool,
    pub supports_mode_2027: bool,
    pub italics: bool,
    pub underline: UnderlineSupport,
    pub default_color_depth: ColorDepth,

    // Image-rendering policy (consumed by terminal_image.rs)
    pub image_rounding: ImageRounding,
    pub scroll_compensation: ScrollPolicy,

    // IPC / scriptability
    pub ipc: IpcCapability,
    pub instance_model: InstanceModel,

    // Filesystem
    pub config_paths: ConfigPathSet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform { MacOs, Linux, Windows, Bsd }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphicsCapability {
    None,
    Kitty,
    ITerm2,
    Sixel,
    /// Multiple protocols accepted; preference order indicated.
    Multiple { primary: ImageSupport, also: &'static [ImageSupport] },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriState { Yes, No, Quirky }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageRounding { Ceil, Floor }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollPolicy {
    /// Append `\n` after CUD when image overflows bottom margin.
    AppendNewline,
    /// Terminal handles scrolling natively; do not compensate (Ghostty).
    NativeHandled,
    /// Terminal never scrolls (Warp — input always at bottom).
    NoScroll,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcCapability {
    None,
    /// Native CLI shell-out (WezTerm, Kitty).
    Cli {
        tool: &'static str,
        spawn_args: &'static [&'static str],
        send_text_args: &'static [&'static str],
        capture_args: &'static [&'static str],
        required_env: &'static [&'static str],
        capture_preserves_ansi: bool,
    },
    /// macOS AppleScript (Apple Terminal).
    AppleScript { capture_preserves_ansi: bool },
    /// Multiplexer-only (tmux).
    Multiplexer { tool: &'static str, capture_preserves_ansi: bool },
    /// Escape-code based (OSC queries).
    OscOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstanceModel {
    /// One process per window (Alacritty, Kitty, WezTerm).
    MultiProcess,
    /// Single process owns all windows (Apple Terminal, GNOME Terminal).
    SingleProcess,
}

#[derive(Debug, Clone, Copy)]
pub struct ConfigPathSet {
    pub macos: &'static [&'static str],
    pub linux: &'static [&'static str],
    pub windows: &'static [&'static str],
}
```

### 4.2 Profile registry + lookup

```rust
// biscuit-terminal/lib/src/discovery/profile_registry.rs

use super::profile::*;
use super::detection::TerminalApp;

impl TerminalApp {
    /// Returns the static profile for this app, or a synthesized
    /// minimal profile for `TerminalApp::Other`.
    pub fn profile(&self) -> TerminalAppProfile {
        match self {
            TerminalApp::Wezterm        => WEZTERM_PROFILE,
            TerminalApp::Kitty          => KITTY_PROFILE,
            TerminalApp::Alacritty      => ALACRITTY_PROFILE,
            TerminalApp::AppleTerminal  => APPLE_TERMINAL_PROFILE,
            TerminalApp::ITerm2         => ITERM2_PROFILE,
            TerminalApp::Ghostty        => GHOSTTY_PROFILE,
            TerminalApp::Konsole        => KONSOLE_PROFILE,
            TerminalApp::GnomeTerminal  => GNOME_TERMINAL_PROFILE,
            TerminalApp::Foot           => FOOT_PROFILE,
            TerminalApp::Contour        => CONTOUR_PROFILE,
            TerminalApp::Warp           => WARP_PROFILE,
            TerminalApp::Wast           => WAST_PROFILE,
            TerminalApp::VsCode         => VSCODE_PROFILE,
            TerminalApp::Other(_)       => UNKNOWN_PROFILE,
        }
    }
}

const WEZTERM_PROFILE: TerminalAppProfile = TerminalAppProfile {
    app: TerminalApp::Wezterm,  // requires PartialEq + const-able variant
    display_name: "WezTerm",
    vendor_url: "https://wezterm.org/",
    platforms: &[Platform::MacOs, Platform::Linux, Platform::Windows],
    graphics: GraphicsCapability::Kitty,
    supports_osc8: TriState::Yes,
    supports_osc52: true,
    supports_mode_2027: true,
    italics: true,
    underline: UnderlineSupport::all(),
    default_color_depth: ColorDepth::TrueColor,
    image_rounding: ImageRounding::Ceil,
    scroll_compensation: ScrollPolicy::AppendNewline,
    ipc: IpcCapability::Cli {
        tool: "wezterm",
        spawn_args: &["cli", "spawn"],
        send_text_args: &["cli", "send-text", "--no-paste"],
        capture_args: &["cli", "get-text", "--escapes"],
        required_env: &["WEZTERM_UNIX_SOCKET"],
        capture_preserves_ansi: true,
    },
    instance_model: InstanceModel::MultiProcess,
    config_paths: ConfigPathSet {
        macos: &["~/.config/wezterm/wezterm.lua", "~/.wezterm.lua"],
        linux: &["~/.config/wezterm/wezterm.lua", "~/.wezterm.lua"],
        windows: &[r"%USERPROFILE%\.config\wezterm\wezterm.lua"],
    },
};

// ... 12 more constants for the other apps ...
```

`TerminalApp` will need either `Other(&'static str)` (breaking) or a
helper `synthesize_other(name: &str)` returning a non-`'static`
profile when the variant carries an owned name. The simpler path: keep
`TerminalApp::Other(String)` and have `profile()` for that variant
return a `Cow<'static, TerminalAppProfile>` (or just construct an
owned `TerminalAppProfile` per call; the hot path is detection-time).

### 4.3 New `ImageSupport::Sixel` variant

The current enum has `None | Kitty | ITerm`. Add `Sixel`. Existing
code that pattern-matches `ImageSupport` must be audited; the
candidate emulators are Foot, Contour, mlterm, xterm-with-sixel.
Detection in `image_support_from_known_terminals` extends with a
Sixel terminals list; the Foot and Contour profiles set
`graphics: GraphicsCapability::Sixel`.

This is the only **breaking** change in this spec — it forces
existing `match` arms over `ImageSupport` to add a `Sixel` case.
Acceptable because the enum is non-exhaustive in spirit (we have
been growing it) and the migration is mechanical.

### 4.4 Migration of scattered match arms

| File | Change |
|---|---|
| `discovery/clipboard.rs:73` | Replace match arm with `app.profile().supports_osc52`. |
| `discovery/mode_2027.rs:68` | Replace match arm with `app.profile().supports_mode_2027`. |
| `discovery/osc_queries.rs:280, 451` | Replace match arms with profile-driven dispatch. The current code asks two different OSC questions (bg color vs cursor color); both reduce to "does this app respond to OSC color queries", which becomes a profile bit (`responds_to_osc_color_queries: bool`). |
| `discovery/config_paths.rs:62` | Replace per-app match with `app.profile().config_paths.<platform>()`. The OS-specific helper functions in this file collapse into the `ConfigPathSet` struct. |
| `discovery/detection.rs:614` (`KITTY_TERMINALS`) | Replace constant + iTerm2 special-case with iteration over all profiles whose `graphics` is `Kitty` / `ITerm2`. |
| `components/terminal_image.rs` (rounding) | Replace `if app == Warp { floor } else { ceil }` with `app.profile().image_rounding`. |
| `components/terminal_image.rs` (scroll) | Replace the Ghostty/Warp special cases with `match app.profile().scroll_compensation`. |

After migration each of these files becomes ~10 lines shorter and the
per-app knowledge it encoded moves into the constant table. The
`Terminal` struct's runtime fields still populate the same way
(detection runs, populates `Terminal` from `app.profile()`); callers
do not change.

### 4.5 Public API additions

- `pub use discovery::profile::{TerminalAppProfile, GraphicsCapability,
  IpcCapability, InstanceModel, ImageRounding, ScrollPolicy, Platform,
  TriState, ConfigPathSet};` from `prelude`.
- `Terminal::profile(&self) -> TerminalAppProfile` convenience that
  delegates to `self.app.profile()`.
- Updated `bt` CLI: when run with no args, the existing capability
  output gains a "Profile" section showing graphics, IPC, instance
  model, vendor URL — sourced from the active app's profile.
  `bt --json` includes the profile under a `"profile"` key.

### 4.6 Test coverage

- One unit test per profile constant verifies internal consistency:
  - `graphics::Kitty` ↔ profile's terminal is in
    `KITTY_TERMINALS` legacy list (via the migration shim until that
    constant is deleted).
  - `ipc::Cli { capture_preserves_ansi }` matches the harness's
    documented behavior (regression net for the L1/L2 spec).
- A registry-completeness test that iterates every `TerminalApp`
  variant and asserts `app.profile().app == app` (catches missing
  match arms after enum additions).
- A migration parity test: for every emulator currently handled by
  the scattered match arms, the new profile-driven path returns the
  same answer as the old match.

## 5. File layout

```
biscuit-terminal/lib/src/discovery/
├── profile.rs                  # NEW — TerminalAppProfile + enums
├── profile_registry.rs         # NEW — per-app constants + lookup
├── detection.rs                # Smaller: drops KITTY_TERMINALS const
├── clipboard.rs                # Smaller: profile-driven
├── mode_2027.rs                # Smaller: profile-driven
├── osc_queries.rs              # Smaller: profile-driven
└── config_paths.rs             # Smaller: ConfigPathSet driven
```

## 6. Acceptance criteria

A reviewer can mark this work complete when:

1. **Profile module exists.** `discovery/profile.rs` defines
   `TerminalAppProfile` and its supporting enums; `profile_registry.rs`
   holds one `const` per known terminal (12 named + 1 `UNKNOWN`).
2. **Lookup works.** `TerminalApp::profile()` is implemented and tested
   for every variant including `Other`.
3. **Sixel variant added.** `ImageSupport::Sixel` exists; Foot and
   Contour profiles use it; downstream `match` arms compile.
4. **Scattered arms migrated.** None of `clipboard.rs`,
   `mode_2027.rs`, `osc_queries.rs`, `config_paths.rs`,
   `terminal_image.rs` contains a match arm enumerating per-app names
   for *static* facts. Runtime detection branches (env probing) may
   remain.
5. **Behavioral parity.** The migration parity test passes — every
   pre-existing per-app answer is unchanged.
6. **Public API.** `TerminalAppProfile` is re-exported through
   `biscuit_terminal::prelude`; `bt` and `bt --json` surface profile
   data.
7. **Documentation.** The biscuit-terminal skill (SKILL.md) gains a
   "Terminal app profiles" section pointing readers to
   `TerminalApp::profile()` as the static-facts entry point.

## 7. Risks & open questions

- **`ImageSupport::Sixel` is an enum addition.** Downstream `match`
  arms over `ImageSupport` will fail to compile until they add a
  `Sixel` case. Mitigation: do the migration in the same PR that
  adds the variant; the workspace is the only consumer.
- **`TerminalApp::Other(String)` and `const` profiles.** The `Other`
  variant carries a runtime `String`, so the match arm in
  `profile()` cannot return a `&'static TerminalAppProfile`. Either:
  (a) `profile()` returns an owned `TerminalAppProfile` everywhere
  (cheap — it's `Copy`-able since all fields are `&'static` /
  primitives), or (b) profiles are returned as `Cow`. (a) is simpler
  and recommended.
- **OSC8 Quirky cases.** Apple Terminal silently swallows OSC8
  wrappers; iTerm2 honors them; Warp displays them but rendering is
  inconsistent across versions. The `TriState::Quirky` variant lets
  us record "supported in some sense but consult vendor docs" without
  forcing a binary answer. Callers that need a strict yes/no should
  check for `TriState::Yes`.
- **Per-version drift.** Profiles describe the *current* released
  build. When Ghostty stabilizes its IPC, Warp ships an Lua API,
  etc., we update the profile. There is no per-version matrix; this
  is a deliberate simplification.
- **Profile is not a config.** Users cannot override the profile —
  it is vendor-determined data, not user preference. User overrides
  continue to flow through `TerminalBuilder` and env vars on the
  runtime `Terminal` struct.
- **Harness coupling.** The L1/L2 harness has its own `available()`
  probes today. A follow-up could rewrite them in terms of
  `IpcCapability::Cli { required_env, .. }`. Out of scope here so
  the L1/L2 work doesn't block on this refactor.

## 8. Sequencing suggestion

1. **§4.1** Define `TerminalAppProfile` and its supporting enums.
   Compile-only; no callers yet.
2. **§4.2** Implement `TerminalApp::profile()` with the 13 profile
   constants. Add the registry-completeness test.
3. **§4.3** Add `ImageSupport::Sixel`. Fix downstream `match` arms.
4. **§4.4** Migrate the five scattered files one at a time, each
   with a parity test before the match arm is removed.
5. **§4.5** Wire `Terminal::profile()`, prelude re-exports, `bt`
   surface.
6. **§4.6** Polish: SKILL.md update, doc comments, deprecation
   notes on any internal helpers that the migration retired.

Each step is independently reviewable; only step 3 (Sixel addition)
is a forced workspace-wide compile break, and it is intentionally
small.
