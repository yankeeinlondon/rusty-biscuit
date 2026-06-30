# Spec: Terminal App Configuration & Environment Metadata

**Status:** Draft
**Author:** Claude (Opus 4.8, 1M context) for Ken Snyder
**Date:** 2026-06-29
**Related:**
  - `biscuit-terminal/features/2026-05-03-app-profiles/spec.md` (Unscheduled — the central `TerminalAppProfile` this spec slots into)
  - `biscuit-terminal/lib/src/discovery/config_paths.rs` (the single-path resolver this supersedes)
  - `biscuit-terminal/lib/src/discovery/detection/app.rs` (`TerminalApp`, `get_terminal_app()`)
  - `biscuit-terminal/lib/src/discovery/os_detection/` (`OsType`, `is_wsl1()`)

## 1. Why this exists

biscuit-terminal already detects *which* terminal you are in and exposes a
single canonical config path per app via `get_terminal_config_path()`. Three
gaps make that insufficient for the questions consumers actually ask:

1. **Config location is multi-valued and override-driven.** A real terminal
   reads its config from the *first* of several candidate paths, and most honor
   an environment variable (`KITTY_CONFIG_DIRECTORY`, `WEZTERM_CONFIG_FILE`,
   `$XDG_CONFIG_HOME`) that relocates it. Today's resolver returns one hard-coded
   path and ignores overrides, so "where is *this host* actually configured?"
   cannot be answered.

2. **WSL is not modeled.** Under WSL the relevant config usually lives on the
   *Windows* filesystem (reached via `/mnt/c/Users/...`), not the Linux home.
   `OsType` collapses WSL into `Linux`, so the resolver looks in the wrong place.

3. **No structured map of *what is inside* the config or *what the terminal
   exports*.** We know `KITTY_LISTEN_ON` carries kitty's IPC socket and
   `font_size` carries its font size, but that knowledge is tribal (see the
   recent `KITTY_LISTEN_ON` investigation). There is no queryable model of
   "where does app X keep its font size / IPC socket / opacity" or "which env
   var holds app X's pane id".

The `app-profiles` spec already argues for a single `&'static`
`TerminalAppProfile` keyed by `TerminalApp` and explicitly lists *IPC mechanism*
and *config file location* among the facts to centralize — but it remains
unscheduled and unimplemented. **This spec defines the config + environment
slice of that profile concretely and ships it, with a CLI surface (`bt about`)
to expose it.** Whether it lands as part of `TerminalAppProfile` or as a
sibling struct the profile re-exports is an integration detail (§11).

## 2. Goals

- A per-app, `&'static` metadata model describing, for every supported terminal:
  - **Config file candidates broken down by OS target** — `linux`, `macos`,
    `windows`, `wsl1`, `wsl2` — each a **1:M ordered list** (first-existing wins).
  - **Environment variables that relocate the config**, in priority order,
    independent of OS (each tagged as pointing at a *directory* or a *file*).
  - **Dot-notation locators** for where named settings live *inside* the config
    file (IPC, font, font_size, theme, background_color, opacity, and an
    extended set — §5.5), interpreted against the file's declared format.
  - **An environment-fact map** for runtime values the terminal exports
    (PID, window id, pane id, public key, IPC address, session id, … — §5.6).
- A library method `TerminalApp::get_config_file() -> Option<PathBuf>` that
  resolves the **actual** config file in use on this host (env override →
  per-OS-target candidates → first that exists), or `None`.
- A `bt about <app>` CLI subcommand reporting all of the above, plus — when the
  app is installed and its config is found — the host's **actual** resolved
  configuration values. Terminal output by default; `--json` and `--plain`.

## 3. Non-goals

- Replacing the broader `TerminalAppProfile` (graphics protocol, OSC8, image
  rounding, etc.). This spec owns only the *config + environment* facets and is
  designed to nest inside that profile later.
- *Writing* or mutating terminal config files. Read-only.
- Live IPC to the terminal (we report *where* the IPC socket is declared/exported,
  not drive `kitty @` / `wezterm cli`).
- Exhaustive setting extraction for formats we cannot statically parse (Lua, binary
  plists) — those degrade to locator-only with a note (§6).

## 4. Terminology

- **OS target** — the resolution bucket for config locations: one of `Linux`,
  `MacOS`, `Windows`, `Wsl1`, `Wsl2`. Distinct from `OsType` (which carries BSDs,
  illumos, etc. but no WSL split). Derived from `OsType` + `is_wsl1()` /
  `is_wsl2()`; BSD/illumos/unknown map to `Linux` (XDG-style) for resolution.
- **Metadata** — vendor-determined, version-stable facts (`&'static`): candidate
  paths, env-var names, dot locators. Answers "where *would* X be".
- **Resolved value** — what *this host* actually has: the existing config file,
  the parsed setting values, the live env values. Answers "what *is* X here".

## 5. Library design

All types live under a new module, proposed `discovery::app_metadata` (final home
TBD per §11). Sketches below are illustrative, not final signatures.

### 5.1 OS target resolution

```rust
pub enum ConfigOsTarget { Linux, MacOS, Windows, Wsl1, Wsl2 }

/// Maps the running host to a single config-resolution bucket.
/// Windows host → Windows; macOS → MacOS; Linux+is_wsl1 → Wsl1;
/// Linux+is_wsl2 → Wsl2; other Linux/BSD/illumos/unknown → Linux.
pub fn current_config_os_target() -> ConfigOsTarget;
```

`is_wsl2()` is new (companion to the existing `is_wsl1()`): WSL2 is a real Linux
kernel, detected via `/proc/version` containing `microsoft`/`WSL2` and/or the
`WSL_INTEROP` / `WSL_DISTRO_NAME` env vars without the WSL1 signature.

**Why WSL1 and WSL2 are separate buckets.** For *config location* they often
share candidates (both reach the Windows-side terminal config through `/mnt/c`),
but keeping them distinct lets a terminal diverge where it must, and WSL1 vs WSL2
genuinely differ for the *IPC/env* facets (AF_UNIX socket interop and the
`WSL_INTEROP` surface differ). Modeling both now avoids a breaking change later.

### 5.2 Config metadata

```rust
pub struct ConfigMetadata {
    pub format: ConfigFormat,
    pub locations: OsConfigLocations,         // 1:M per OS target
    pub location_env: &'static [ConfigLocationEnv], // priority order
    pub settings: SettingLocators,
}

pub enum ConfigFormat {
    KittyConf,   // flat "key value" lines
    KeyValue,    // "key = value" (ghostty, foot/ini-ish)
    Toml,        // alacritty
    Yaml,        // contour, legacy alacritty.yml
    Lua,         // wezterm — not statically parseable (locator-only)
    Plist,       // iTerm2, Apple Terminal (xml or binary)
    Json,        // Windows Terminal, VS Code
    Dconf,       // GNOME Terminal / gsettings (no flat file)
    None,        // app has no user-editable flat config (e.g. Warp)
}

pub struct OsConfigLocations {
    pub linux:   &'static [ConfigCandidate],
    pub macos:   &'static [ConfigCandidate],
    pub windows: &'static [ConfigCandidate],
    pub wsl1:    &'static [ConfigCandidate],
    pub wsl2:    &'static [ConfigCandidate],
}

pub struct ConfigCandidate {
    /// Path template with portable tokens expanded at resolve time:
    /// `~`, `$HOME`, `$XDG_CONFIG_HOME`, `$APPDATA`, `$LOCALAPPDATA`,
    /// `$USER`, and the WSL helper `$WIN_HOME` (the Windows user profile
    /// surfaced under /mnt/c). Unknown/empty tokens drop the candidate.
    pub template: &'static str,
    pub note: Option<&'static str>,
}

pub struct ConfigLocationEnv {
    pub var: &'static str,
    pub kind: ConfigEnvKind, // Dir | File
    pub note: Option<&'static str>,
}
```

Token expansion respects the same XDG fallbacks as today's `config_dir()`
(`$XDG_CONFIG_HOME` else `~/.config`). The `$WIN_HOME` token resolves the
Windows profile path from inside WSL (via `/mnt/c/Users/$USER`, or the
`USERPROFILE` translated through `wslpath` when available; best-effort, candidate
dropped if unresolvable).

### 5.3 `get_config_file()` resolution

```rust
impl TerminalApp {
    pub fn metadata(&self) -> Option<&'static TerminalAppMetadata>;
    pub fn config_metadata(&self) -> Option<&'static ConfigMetadata>;

    /// The config file THIS host is actually using, or None.
    pub fn get_config_file(&self) -> Option<PathBuf>;
}
```

Algorithm:
1. For each `location_env` var in order: if set and non-empty, expand. `File`
   kind → that path; `Dir` kind → join the app's canonical config filename.
   If the resulting path **exists**, return it.
2. Else pick the candidate list for `current_config_os_target()`, expand each
   template in order, return the **first that exists**.
3. Else `None`.

Existence-checked by design: the question is "what is in use", not "what is the
default". (Contrast the existing `get_terminal_config_path`, which returns the
default path unconditionally — kept as a thin back-compat wrapper, §5.7.)

> Known limitation: flag-based overrides (`kitty --config foo`,
> `alacritty --config-file foo`) are invisible to an out-of-process query. Env
> overrides are the discoverable surface; this is documented, not solved.

### 5.4 Why dot-notation, and how it resolves per format

A single logical key (e.g. `font_size`) maps to different physical locations per
format. The locator is a **dot path** interpreted against `ConfigFormat`:

| Format | Dot path `font.size` means | Example app |
|--------|----------------------------|-------------|
| `Toml` / `Json` / `Yaml` | nested table/object key `font` → `size` | alacritty, Windows Terminal |
| `Plist` | nested dict keys (via plist crate) | iTerm2 |
| `KittyConf` / `KeyValue` | a **flat** key; dots are literal/decorative — the metadata gives the real flat key (e.g. `font_size`) | kitty, ghostty |
| `Lua` | logical pointer only (`config.font_size`); value extraction not attempted (§6) | wezterm |

So the locator struct carries the dot path **and** the metadata declares the
format, giving a generic reader enough to resolve (or knowingly decline). For
flat formats the "dot path" is simply the flat key the reader looks up verbatim.

### 5.5 Setting locators

```rust
pub struct SettingLocators {
    // v1 core (the spec's required set)
    pub ipc: Option<SettingLocator>,
    pub font: Option<SettingLocator>,
    pub font_size: Option<SettingLocator>,
    pub theme: Option<SettingLocator>,
    pub background_color: Option<SettingLocator>,
    pub opacity: Option<SettingLocator>,
    // v1 extended (proposed additions — see "what else?")
    pub foreground_color: Option<SettingLocator>,
    pub cursor_color: Option<SettingLocator>,
    pub cursor_style: Option<SettingLocator>,
    pub selection_colors: Option<SettingLocator>,
    pub color_scheme: Option<SettingLocator>,   // named palette/scheme
    pub bold_font: Option<SettingLocator>,
    pub italic_font: Option<SettingLocator>,
    pub line_height: Option<SettingLocator>,
    pub window_padding: Option<SettingLocator>,
    pub scrollback_lines: Option<SettingLocator>,
    pub shell_program: Option<SettingLocator>,  // shell/command launched
}

pub struct SettingLocator {
    pub path: &'static str,         // dot path (see §5.4)
    pub value_kind: ValueKind,      // String | Number | Color | Bool | Path | Enum
    pub note: Option<&'static str>,
}
```

**"What else?" — proposed beyond the required 6.** The extended set above covers
the settings most consistently present across emulators and most useful in an
`about` report. Deferred (lower value / very app-specific): keybindings,
ligatures/font-features, tab-bar style, bell, blink rate, cursor blink, window
decorations, working-directory inheritance. `Option<…>` means a setting that an
app simply does not expose is `None`, not an error.

`ipc` deserves a note: it is the locator for the IPC mechanism *as declared in
the config* (e.g. kitty `allow_remote_control` + `listen_on`, wezterm
`unix_domains`). The *live* IPC address is an env fact (§5.6,
`ipc_address`) — the two together explain the `KITTY_LISTEN_ON` situation: config
says "listen", env says "here".

### 5.6 Environment-fact map

```rust
pub struct EnvFactMap {
    pub pid:            &'static [&'static str], // ["KITTY_PID"]
    pub window_id:      &'static [&'static str], // ["KITTY_WINDOW_ID"]
    pub pane_id:        &'static [&'static str], // ["WEZTERM_PANE"]
    pub public_key:     &'static [&'static str], // ["KITTY_PUBLIC_KEY"]
    // proposed additions ("what else is commonly exposed?")
    pub ipc_address:    &'static [&'static str], // ["KITTY_LISTEN_ON","WEZTERM_UNIX_SOCKET","ALACRITTY_SOCKET"]
    pub session_id:     &'static [&'static str], // ["ITERM_SESSION_ID","WT_SESSION"]
    pub config_dir:     &'static [&'static str], // ["KITTY_CONFIG_DIRECTORY"]
    pub resources_dir:  &'static [&'static str], // ["GHOSTTY_RESOURCES_DIR","KITTY_INSTALLATION_DIR"]
    pub version:        &'static [&'static str], // ["TERM_PROGRAM_VERSION"]
    pub profile:        &'static [&'static str], // ["ITERM_PROFILE"]
}
```

Each fact is an **ordered candidate list** (first set wins) because some facts
have aliases or vary by version. A helper resolves a fact against the live
environment:

```rust
pub fn resolve_env_fact(candidates: &[&str]) -> Option<(/*var*/ String, /*value*/ String)>;
```

Live env facts are only *meaningful* when the app being queried is the current
terminal; `bt about` flags this in output rather than implying otherwise.

### 5.7 Public API & back-compat

- `get_terminal_config_path(app)` / `get_terminal_config_paths(app)` are kept,
  reimplemented as thin wrappers: `_paths` returns the current OS target's
  expanded candidate templates; `_path` returns the first. Behavior for existing
  callers is preserved (default path, no existence check).
- New surface: `TerminalApp::metadata()`, `::config_metadata()`,
  `::get_config_file()`, `current_config_os_target()`, `resolve_env_fact()`.
- Seed data: a `const`/`LazyLock` table keyed by `TerminalApp`, one entry per
  supported app. `Other(_)` → `None`.

## 6. Value extraction (resolved configuration)

When `get_config_file()` finds a file, `bt about` reads the settings declared in
`SettingLocators`, per `ConfigFormat`:

| Format | v1 extraction | Crate / approach |
|--------|---------------|------------------|
| Toml / Yaml / Json | full (dot path → value) | `toml`, `serde_yaml_ng`, `serde_json` |
| Plist | full | `plist` crate (xml + binary) |
| KittyConf / KeyValue | full (line scan for the flat key; last wins, includes-aware best-effort) | small in-house parser |
| Lua | **locator-only** (report path + note "Lua config; value not extracted") | none in v1 |
| Dconf / None | not applicable (report "no flat config / managed by <system>") | — |

This is the central honesty of the spec: **metadata is always reportable; value
extraction is best-effort and format-bounded.** A locator with no extractable
value still shows *where* to look. Lua extraction (evaluating `wezterm.lua` in a
sandbox or via `wezterm show-config`) is a future option (§12), explicitly out of
v1.

## 7. CLI: `bt about <app>`

```
bt about [APP] [--json] [--plain] [-v]
```

- **`APP`** — optional; fuzzy-matched against `TerminalApp` variants
  (exact → prefix → contains, mirroring claudine's provider matching). Omitted →
  the **currently detected** terminal (`get_terminal_app()`). Invalid → usage
  error (exit 2) listing valid names.
- Renders with biscuit-terminal components (`Section`, `Table`, `Prose`,
  `UnorderedList`) — never hand-rolled escapes.

### 7.1 Report sections (default / terminal output)

1. **Identity** — display name; is this the current terminal?; installed?
   (binary on `PATH` via `which`); detected version (from env/`--version` is out
   of scope — env only in v1).
2. **Config files** — a `Table`: per OS target, the ordered candidate templates,
   with the **active** host's OS target marked; the `location_env` overrides; and
   the **resolved active file** from `get_config_file()` (or "none found").
3. **Settings** — a `Table` of `Setting | Dot path | Value`. `Value` is the
   extracted host value when the file was found and the format is parseable; else
   `—` with a short reason (e.g. "Lua", "no config file").
4. **Environment facts** — a `Table` of `Fact | Candidate vars | Live value`.
   Live value shown only when the app is the current terminal; otherwise
   `(not current terminal)`.

Section ordering is fixed; sections with no data render a one-line empty state
rather than vanishing.

### 7.2 `--json`

Single JSON object on STDOUT, e.g.:

```jsonc
{
  "app": "Kitty",
  "is_current": true,
  "installed": true,
  "os_target": "MacOS",
  "config": {
    "format": "KittyConf",
    "location_env": [{ "var": "KITTY_CONFIG_DIRECTORY", "kind": "dir" }],
    "candidates": { "macos": ["~/.config/kitty/kitty.conf"], "linux": ["..."], "...": [] },
    "resolved_file": "/Users/ken/.config/kitty/kitty.conf",
    "settings": { "font_size": { "path": "font_size", "value": "13.0" },
                  "ipc":       { "path": "allow_remote_control", "value": "no" },
                  "opacity":   { "path": "background_opacity", "value": null } }
  },
  "env": { "pid": { "vars": ["KITTY_PID"], "value": "19716" },
           "ipc_address": { "vars": ["KITTY_LISTEN_ON"], "value": null } }
}
```

`null` value = locator known but no host value (absent key, or non-current app).
STDOUT is JSON-only in this mode; diagnostics to STDERR.

### 7.3 `--plain` (new global flag)

Add `--plain` alongside the existing global `--json` on `Args`. It forces
`ColorDepth::None` so every renderable component emits no SGR/OSC escapes —
required because `bt about` is the first reporting subcommand whose output is
likely piped to `grep`/`diff`. Honors `NO_COLOR`; `FORCE_COLOR=1` overrides TTY
detection for the default colored path. `--plain` + `--json` is allowed
(`--json` already escape-free; `--plain` is a no-op there). `--plain` applies to
all subcommands, not just `about`.

## 8. Seed data (representative, not exhaustive)

| App | Format | macOS candidate(s) | Linux candidate(s) | Location env |
|-----|--------|--------------------|--------------------|--------------|
| Kitty | KittyConf | `~/.config/kitty/kitty.conf` | `$XDG_CONFIG_HOME/kitty/kitty.conf` | `KITTY_CONFIG_DIRECTORY` (dir) |
| WezTerm | Lua | `$XDG_CONFIG_HOME/wezterm/wezterm.lua`, `~/.wezterm.lua` | same | `WEZTERM_CONFIG_FILE` (file) |
| Alacritty | Toml | `$XDG_CONFIG_HOME/alacritty/alacritty.toml`, `~/.alacritty.toml` | same (+ legacy `.yml`) | `$XDG_CONFIG_HOME` (dir) |
| Ghostty | KeyValue | `~/Library/Application Support/com.mitchellh.ghostty/config`, `$XDG_CONFIG_HOME/ghostty/config` | `$XDG_CONFIG_HOME/ghostty/config` | — |
| iTerm2 | Plist | `~/Library/Preferences/com.googlecode.iterm2.plist` | n/a | — |
| Apple Terminal | Plist | `~/Library/Preferences/com.apple.Terminal.plist` | n/a | — |
| Windows Terminal | Json | n/a | n/a (windows/wsl) | — |
| Warp | None | (cloud/managed; no flat file) | (same) | — |
| GNOME Terminal | Dconf | n/a | dconf/gsettings (no file) | — |

Windows Terminal (windows + wsl1/wsl2 targets) candidate:
`$LOCALAPPDATA\Packages\Microsoft.WindowsTerminal_*\LocalState\settings.json`;
from WSL the same under `$WIN_HOME/AppData/Local/...`. The `*` package-id glob
is a candidate-expansion concern (first match wins).

## 9. Testing strategy

- **L1 (bulk).** Resolution logic is pure given injected inputs. Test
  `get_config_file()` and `current_config_os_target()` with a fabricated `HOME`
  / `XDG_CONFIG_HOME` / `APPDATA` and an injected `ConfigOsTarget` + a temp dir
  containing (or not) candidate files: env-override-wins, first-existing-wins,
  none-found, WSL `/mnt/c` mapping. Use `EnvGuard` + `#[serial]` for env mutation.
- **L1 value extraction.** Fixture config files per format (kitty.conf, TOML,
  plist, key=value) → assert extracted values; Lua fixture → assert locator-only
  with the documented note.
- **CLI integration (`assert_cmd` + `insta`).** `bt about kitty --plain` snapshot
  (escape-free), `--json` schema/round-trip, fuzzy match, invalid app → exit 2,
  default-to-current behavior (env-driven). `NO_COLOR=1` for stable snapshots.
- No L2/terminal harness needed — this feature reads files/env, it does not drive
  a live terminal. `--plain`'s escape-stripping is asserted at L1 via
  `ColorDepth::None`.
- Must compile and resolve correctly on macOS, Windows, Linux (CI matrix);
  WSL paths are unit-tested via injected target since CI lacks WSL.

## 10. Cross-platform considerations

- **WSL.** `$WIN_HOME` resolution via `/mnt/c/Users/$USER` (and `wslpath -u`
  when present); candidates that fail to resolve are dropped, not errored.
- **Windows.** `$APPDATA` / `$LOCALAPPDATA`; package-id globbing for Windows
  Terminal; path separators handled by `PathBuf`.
- **Plist.** macOS `defaults`-managed plists may be binary; the `plist` crate
  reads both. Preference caching (`cfprefsd`) means a freshly-changed value can
  lag the on-disk file — note in output, do not fight it.
- **No new heavy deps without sign-off** — `plist` and `toml` are the likely
  additions; confirm against `docs/dependencies.md` before adding (§11).

## 11. Open questions (need Ken's input)

1. **Home of the metadata.** Fold into the unscheduled `TerminalAppProfile`
   (schedule that first), or ship as a standalone `app_metadata` module the
   profile later re-exports? Recommendation: standalone now, designed to nest.
2. **Setting set scope for v1.** Confirm the 6 required + which of the proposed
   extended set (§5.5) ship in v1 vs deferred.
3. **Lua / wezterm value extraction.** Locator-only in v1 (recommended), or
   invest in `wezterm show-config` shell-out / a Lua sandbox now?
4. **Dconf/gsettings & Warp.** Model GNOME Terminal/Konsole (dconf) and Warp
   (no file) as `format: Dconf/None` metadata-only, or exclude from v1?
5. **`bt about` with no arg.** Default to the current terminal (recommended) or
   require an explicit app?
6. **New dependencies.** OK to add `plist` (+ confirm `toml`/`serde_yaml_ng`
   already present) for value extraction?
7. **Should `get_config_file()` also expose the *reason* it resolved** (which env
   var or which candidate index) for richer `about` output and debugging?

## 12. Out of scope / future

- Writing/patching config files.
- Live IPC drive (`kitty @`, `wezterm cli`) — only declaration/exposure is modeled.
- Lua config evaluation; `defaults`/`cfprefsd` write-through.
- Capability facets (graphics protocol, OSC8, rounding) — owned by the broader
  `TerminalAppProfile`.
- A `bt about --all` cross-app matrix (natural follow-up once per-app works).
