---
clarified: "claude/claude-opus-4-8[1m]"
review_iterations: 9
---

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
to expose it.** It ships as a standalone `app_metadata` module now, designed to
nest into `TerminalAppProfile` later (decided — see §5.7).

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

All types live under a new standalone module, `discovery::app_metadata`
(**decided**: ship standalone now, designed to nest into the future
`TerminalAppProfile` — see §5.7). Sketches below are illustrative, not final
signatures.

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
    Json,        // Windows Terminal
    Json5,       // VS Code (JSONC/JSON5-tolerant settings)
    Dconf,       // GNOME Terminal / gsettings (no flat file)
    None,        // no parseable file AND outside the coverage floor (§5.7); not Warp — see §8
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

    /// As `get_config_file`, but also reports WHY this path resolved
    /// (which env var, or which candidate index matched).
    pub fn get_config_file_resolved(&self) -> Option<ResolvedConfig>;
}

pub struct ResolvedConfig {
    pub path: PathBuf,
    pub source: ConfigSource,
}

pub enum ConfigSource {
    EnvVar(&'static str),  // e.g. EnvVar("KITTY_CONFIG_DIRECTORY")
    Candidate(usize),      // e.g. Candidate(0) — index into the OS target's list
}
```

`get_config_file()` stays the simple API (just the `PathBuf`);
`get_config_file_resolved()` is the sibling that carries provenance for richer
`about` output and debugging. The former is a thin projection of the latter
(`.map(|r| r.path)`).

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
| `Toml` / `Json` / `Json5` / `Yaml` | nested table/object key `font` → `size` | alacritty, Windows Terminal, VS Code |
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

**Setting scope for v1 (decided).** Keep **all** locator fields defined on
`SettingLocators` — they are cheap `&'static` pointers and keeping them keeps the
type forward-compatible. But v1 only **guarantees** the 6 core locators (`ipc`,
`font`, `font_size`, `theme`, `background_color`, `opacity`) are populated and
extracted for supported apps. The extended locators are populated
**opportunistically** where the physical location is already known, and are
`None` otherwise — there is **no** v1 requirement to research all 14 extended
settings × every app. An extended locator being `None` is not a coverage gap.

**"What else?" — the extended set beyond the required 6.** The extended set above
covers the settings most consistently present across emulators and most useful in
an `about` report. Deferred (lower value / very app-specific): keybindings,
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
  `::get_config_file()`, `::get_config_file_resolved()`,
  `current_config_os_target()`, `resolve_env_fact()`.
- **Probe-target accessors (sniff-free).** The `app_metadata` library stays
  sniff-free but declares, per app, the probe target the CLI's install detection
  needs: a binary name and/or a macOS bundle id (e.g. `bin_name()` /
  `bundle_id()` accessors, or carry them as fields on the metadata). The library
  only *declares* these strings; it does **not** perform detection. The CLI maps
  the queried app to that probe target and calls sniff (§7.1, §10).

**Metadata keying — keyed directly by `TerminalApp` (decided).** The seed table
is resolved by `match self` over `TerminalApp` variants — no separate
`MetadataKey` type and no stringly-typed `Other(_)` lookup. This requires a
first-class enum variant for every app the metadata represents. Concretely it
adds a **new `TerminalApp::WindowsTerminal` variant**: today `get_terminal_app()`
returns `TerminalApp::Other("Windows Terminal")` for the `WT_SESSION` env var, so
that detector must change to emit the new `WindowsTerminal` variant. That is a
behavior change in detection and gets its own test (assert `WT_SESSION` →
`WindowsTerminal`, not `Other(_)`).

> **Resolved contradiction.** The §8 seed table already lists a Windows Terminal
> row, but under "keyed by `TerminalApp`, `Other(_)` → `None`" that row was
> *unreachable* — no `WindowsTerminal` variant existed, and the detector only ever
> produced `Other("Windows Terminal")`, which keyed to `None`. Adding the variant
> (and re-pointing the detector at it) is what makes the seed row reachable.

**App-coverage floor (acceptance gate).** The metadata seed table must cover
**every app for which the pre-existing `get_terminal_config_path` returns
`Some`**. Today that set includes `VsCode`, `Konsole`, `Foot`, and `Contour`,
none of which yet appear in the §8 seed table — they must be added. The gate is
explicit and testable: assert that **no app regresses `Some -> None`** versus the
pre-existing `get_terminal_config_path` (i.e. for every `TerminalApp` the old
resolver answered `Some` for, the new `get_terminal_config_path` wrapper backed by
the seed table must also answer `Some`). This makes the "thin back-compat wrapper,
behavior preserved" promise above verifiable rather than aspirational.

> **`format: None` is floor-reserved.** Because the floor forbids `Some -> None`,
> `format: None` is reserved for apps that **both** have no parseable file **and**
> are not in the coverage floor (i.e. the legacy resolver already answered `None`
> for them). An app the floor covers must contribute at least one candidate and
> therefore cannot be `None`. This is why Warp is reclassified away from `None`
> below (§8) — it has a real candidate path and is reachable through the wrapper.
>
> **Variants outside the seed table.** Keying by `match self` means any
> `TerminalApp` variant the seed table does not cover (e.g. `Wast`,
> `GnomeTerminal`) simply maps to `None` metadata. That is fine for the floor *as
> long as the legacy resolver also returned `None`* for them — which it does:
> `GnomeTerminal` and `Wast` already resolve to `None` from
> `get_terminal_config_path`, so neither is bound by the floor.

## 6. Value extraction (resolved configuration)

When `get_config_file()` finds a file, `bt about` reads the settings declared in
`SettingLocators`, per `ConfigFormat`:

| Format | v1 extraction | Crate / approach |
|--------|---------------|------------------|
| Toml / Yaml / Json / Json5 | full (dot path → value) | **`biscuit-file`** (normalizes each to `serde_json::Value`) |
| Plist | full | `plist` crate (xml + binary) |
| KittyConf / KeyValue | full (line scan for the flat key; last wins, includes-aware best-effort) | small in-house parser |
| Lua | **locator-only** (report path + note "Lua config; value not extracted") | none in v1 |
| Dconf / None | not applicable (report "no flat config / managed by <system>") | — |

**Structured formats parse via `biscuit-file` (decided).** TOML, YAML, JSON, and
JSON5 are parsed through the in-repo `biscuit-file` crate (path dependency,
`features = ["toml","yaml","json5"]`) — **not** by adding `toml` /
`serde_yaml_ng` / `json-five` directly to biscuit-terminal. `biscuit-file`
re-exports those parsers and normalizes every structured format to a single
`serde_json::Value` (its `Toml` / `Yaml` / `Json5` types each expose
`.as_json_value()` / `.as_json()`). `biscuit-file` is lower-level and does **not**
depend on biscuit-terminal, so there is no dependency cycle.

**Architectural consequence — one shared resolver.** Because every structured
format collapses to one `serde_json::Value`, value extraction over them is a
**single dot-path resolver over `serde_json::Value`**, shared across
TOML/YAML/JSON/JSON5. The per-format extraction logic the old §6 table implied for
these formats collapses into that one resolver. This directly serves the §5.4 goal
(one logical key → a different physical location per format): the *locator* differs
per format, but the *reader* is uniform once the file is normalized to JSON.

**Flat formats stay in-house.** `KittyConf` / `KeyValue` (Kitty, Ghostty, foot)
are not data formats `biscuit-file` handles; they keep the small in-house flat-key
line scanner (no new dependency).

**Plist gets the `plist` crate (sign-off granted).** iTerm2 and Apple Terminal use
the `plist` crate for full extraction (xml + binary). `plist` is the **only
net-new crate added directly to biscuit-terminal**; adding it is a drift task that
must update `docs/dependencies.md` (and any per-area dependencies doc). See the
§10 `cfprefsd` caching note — a freshly-changed value can lag the on-disk file;
surface that as a note in `bt about` output rather than fighting it.

**`serde_json` is already a biscuit-terminal dependency**, so the shared resolver
adds nothing new; structured-format *parsing* arrives entirely via the
`biscuit-file` path dep.

**Value normalization — v1 returns RAW values.** v1 reports the extracted value as
it appears: the literal string for flat formats, or the `serde_json::Value` leaf
rendered as-is for structured formats. There is **no** Color / Enum / Path
normalization in v1. Accordingly, `SettingLocator.value_kind` is **advisory
metadata, not a parsing contract** — it hints at the expected shape for display
but does not coerce or validate the extracted value. (This is what makes §9's
"assert extracted values" testable: the assertion is `extracted == the literal
fixture value`.) Typed normalization is a documented future increment (§12).

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
   (determined by **sniff** — `find_program_with_source`, covering binary on
   `PATH` **plus** macOS `.app` bundle scan (`/Applications` + `~/Applications`)
   **plus** the Windows App Paths registry / install-root walk; reported as
   installed true/false, with the resolved `ExecutableSource` kind available if
   useful); detected version (from env/`--version` is out of scope — env only in
   v1).

   The plain `which`/`PATH` check is insufficient for macOS GUI bundles (iTerm2,
   Apple Terminal, Warp have no reliable PATH binary), which is why install
   detection is delegated to sniff. This is a **CLI-only** concern — see §10 for
   why the sniff dependency lives in `biscuit-terminal/cli` and not the library.
2. **Config files** — a `Table`: per OS target, the ordered candidate templates,
   with the **active** host's OS target marked; the `location_env` overrides; and
   the **resolved active file** from `get_config_file_resolved()` (or "none
   found"), shown **with its provenance** — e.g. "resolved via
   `$KITTY_CONFIG_DIRECTORY`" or "candidate #1".
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
    "resolved_source": { "candidate": 0 },
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

### 7.3 `--plain` (new global flag) and styling unification

Add `--plain` alongside the existing global `--json` on `Args`. It forces
`ColorDepth::None` so every renderable component emits no SGR/OSC escapes —
required because `bt about` is the first reporting subcommand whose output is
likely piped to `grep`/`diff`. `--plain` applies to all subcommands, not just
`about`.

**Styling unification is in scope (decided).** The CLI today has **two
independent styling systems**: the render-tree / `ColorDepth` path used by
components, **and** a legacy `CliStyles` struct (`cli/src/types.rs`) of
hardcoded raw ANSI literals emitted directly via `println!`. `CliStyles::detect()`
is consumed at **5 sites** — `output.rs` (both `print_pretty`/content-analysis and
the default terminal-metadata dump), `commands/graph.rs`, `commands/mermaid.rs`,
and `commands/shared.rs`. As originally specified ("`--plain` = force
`ColorDepth::None`"), `--plain` would **not** suppress escapes on those 5 legacy
paths, because they never consult `ColorDepth`.

Therefore this spec's scope now **includes migrating all 5 `CliStyles` call sites
to render-tree components and deleting the `CliStyles` struct entirely**, leaving a
single `ColorDepth`-governed styling path. Consequences:

- `--plain` forces `ColorDepth::None` and is now correct for **all** subcommands
  (default report, graph, mermaid, content-analysis, about), because there is only
  one styling path to govern.
- **Precedence.** `--plain` suppresses color **unconditionally**, overriding
  `FORCE_COLOR`. Absent `--plain`, the existing `NO_COLOR` / `FORCE_COLOR` /
  TTY-detection logic stands (continue to honor `NO_COLOR`).
- `--plain` + `--json` is allowed (`--json` is already escape-free; `--plain` is a
  no-op there).

**Tension with surgical-change discipline, acknowledged.** Retiring `CliStyles` is
a deliberate, in-scope refactor for this feature rather than incidental cleanup. It
is justified because `--plain` cannot be globally honest while two styling systems
coexist — a partial migration would leave `--plain` silently lying on the legacy
paths.

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
| Warp | (real on-disk format — see note) | `warp_config_path(&home, os)` | same | — |
| GNOME Terminal | Dconf | n/a | none — managed by dconf/gsettings; **empty candidate list** | — |
| VS Code | Json | `~/Library/Application Support/Code/User/settings.json` | `$XDG_CONFIG_HOME/Code/User/settings.json` | — |
| Konsole | KeyValue | n/a | `$XDG_CONFIG_HOME/konsolerc` (+ profiles) | — |
| Foot | KeyValue | n/a | `$XDG_CONFIG_HOME/foot/foot.ini` | — |
| Contour | Yaml | `~/.config/contour/contour.yml` | `$XDG_CONFIG_HOME/contour/contour.yml` | — |

The four rows above (`VS Code`, `Konsole`, `Foot`, `Contour`) are required by the
app-coverage floor in §5.7: each already returns `Some` from the pre-existing
`get_terminal_config_path`, so each must be present in the seed table.

**IMPLEMENTATION NOTE — GNOME Terminal (Dconf, metadata-only).** GNOME Terminal is
modeled as `format: Dconf` carrying env/locator metadata but **no config-file
extraction**: its candidate list is deliberately **empty**, so `get_config_file()`
returns `None` and `bt about` reports "config managed by dconf/gsettings (no flat
file)". This does not violate the coverage floor (§5.7) because the pre-existing
`get_terminal_config_path` already returns `None` for `GnomeTerminal` — it is not
floor-bound, so the empty candidate list is legal. This is the v1 use of the
`Dconf` `ConfigFormat` variant (i.e. `Dconf` is a USED variant, not reserved).
Konsole is unaffected: it stays `KeyValue` (`konsolerc`) with a real candidate.

**IMPLEMENTATION NOTE — Warp.** The pre-existing `get_terminal_config_path`
already returns `Some(warp_config_path(&home, os))` for `Warp`
(`config_paths.rs`), so the app-coverage floor (§5.7) binds Warp: it must keep a
candidate so the seed-table-backed wrapper still answers `Some` — **no carve-out,
the floor stays absolute.** Warp therefore keeps that candidate path in the seed
table and is **reclassified from `format: None` to its real on-disk format.** The
implementer must confirm what `warp_config_path(&home, os)` actually points at and
whether that file is parseable, then pick the matching `ConfigFormat`. If it turns
out genuinely unparseable, it degrades to **locator-only** per §6 — but it must
still contribute a candidate so the floor holds. (This supersedes the earlier
"cloud/managed; no flat file" framing, which is incorrect given the resolver
already hands back a path.)

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
- **L1 value extraction.** Fixture config files per format (kitty.conf, TOML, YAML,
  JSON/JSON5, plist, key=value) → assert each extracted value **equals the literal
  fixture value** (v1 returns RAW values; `value_kind` is advisory, so there is no
  type-coercion to assert). Lua fixture → assert locator-only with the documented
  note. Structured formats route through `biscuit-file` → shared
  `serde_json::Value` dot-path resolver, so one resolver test covers
  TOML/YAML/JSON/JSON5.
- **Detection behavior change.** Assert `WT_SESSION` now yields
  `TerminalApp::WindowsTerminal` (not `Other("Windows Terminal")`).
- **App-coverage floor.** Assert no app regresses `Some -> None`: for every
  `TerminalApp` the pre-existing `get_terminal_config_path` answered `Some`
  (includes `VsCode`, `Konsole`, `Foot`, `Contour`), the seed-table-backed wrapper
  also answers `Some`.
- **CLI integration (`assert_cmd` + `insta`).** `bt about kitty --plain` snapshot
  (escape-free), `--json` schema/round-trip, fuzzy match, invalid app → exit 2,
  default-to-current behavior (env-driven). `NO_COLOR=1` for stable snapshots.
- **`--plain` must cover a previously-`CliStyles`-rendered path.** The escape-free
  assertion must target output that was legacy-styled before this feature — e.g.
  the **default `bt` metadata report** and/or content-analysis — not only a
  renderable-component path, otherwise the test validates the wrong half of the
  migration. Migrating `output.rs` will churn existing L2/snapshot expectations for
  the default report; those snapshots must be regenerated/updated as part of this
  feature.
- **Install detection (CLI concern).** Install detection lives in the CLI via
  sniff, so it is exercised there, not in the library. A light test suffices —
  e.g. a known-present binary resolves (non-`NotFound` `ExecutableSource`) and a
  bogus name resolves to `NotFound`. It does **not** require the terminal harness.
- No L2/terminal harness needed for the new resolution logic — this feature reads
  files/env, it does not drive a live terminal. `--plain`'s escape-stripping is
  asserted at L1 via `ColorDepth::None`.
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
- **Dependencies (decided).** The only net-new crate added **directly** to
  biscuit-terminal is `plist` (sign-off granted) for binary/xml plist extraction.
  Structured-format parsing (TOML/YAML/JSON/JSON5) arrives via a **path dependency
  on the in-repo `biscuit-file`** crate (`features = ["toml","yaml","json5"]`),
  which re-exports the underlying parsers and normalizes to `serde_json::Value`;
  no `toml` / `serde_yaml_ng` / `json-five` is added to biscuit-terminal directly.
  `serde_json` is already a biscuit-terminal dependency. Adding `plist` and the
  `biscuit-file` path dep is a drift task: update `docs/dependencies.md` and the
  per-area dependencies doc alongside the code change.
- **Install detection (CLI-only sniff dep).** `bt about`'s "installed?" signal is
  determined by `sniff::programs::find_program_with_source(name) ->
  ExecutableSource` (`Path | MacOsBundle | WindowsAppPaths | WindowsInstallRoot |
  NotFound`), covering PATH, the macOS `/Applications` + `~/Applications` bundle
  scan, and the Windows App Paths registry + install-root walk. The `sniff` path
  dependency (on `sniff/lib`) is added to **`biscuit-terminal/cli` ONLY — never to
  `biscuit-terminal/lib`.** Rationale: `sniff/lib` pulls a heavy tree (gix, rayon,
  sysinfo, windows, git2, core-foundation); "installed?" is a report concern, and
  loading that tree into the rendering library would violate the heavy-dep
  discipline. Binaries can afford the weight; the library cannot. There is no
  dependency cycle — `sniff/lib` does not depend on biscuit-terminal (only
  `sniff/cli` does). Adding the `sniff` path dep to `biscuit-terminal/cli` is a
  drift task: update `docs/dependencies.md` and the per-area dependencies doc
  alongside the code change.
- **Integration detail — two distinct `TerminalApp` enums.** `sniff::programs`
  has its **own** `TerminalApp` enum, separate from biscuit-terminal's
  `TerminalApp`; they are not the same type. The CLI bridges by binary name (the
  library-declared probe target, §5.7) or by mapping between the two enums — do
  not assume they unify.

## 11. Resolved questions (ledger)

**All open questions are resolved** — none remain. Each former question has been
folded into the body; this section is a pure ledger of the decisions.

Resolved in this revision (folded into the body):

- **Dconf/gsettings modeling** → GNOME Terminal is modeled as `format: Dconf`,
  metadata-only with an **empty candidate list** (`get_config_file()` → `None`,
  reported as "config managed by dconf/gsettings (no flat file)"); it is not
  floor-bound since the legacy resolver already returned `None` for it. This makes
  `Dconf` a USED v1 variant. Konsole stays `KeyValue` (`konsolerc`) unchanged
  (§6, §8).
- **`bt about` with no arg** → **decided: default to the current terminal** via
  `get_terminal_app()` (§2 / §7). An invalid *explicit* app remains a usage error
  (exit 2) listing valid names. No longer a recommendation — settled.
- **Resolution provenance** → **decided: expose it.** `get_config_file()` stays the
  simple `Option<PathBuf>` API; a sibling `get_config_file_resolved() ->
  Option<ResolvedConfig>` adds the resolved path plus a `ConfigSource` discriminator
  (`EnvVar(name)` or `Candidate(index)`). Surfaced in the §7.1 "Config files"
  section and the §7.2 `--json` `resolved_source` field (§5.3, §7).
- **Home of the metadata** → standalone `app_metadata` module now, designed to nest
  into `TerminalAppProfile` later; keyed directly by `TerminalApp` (§5, §5.7).
- **Setting set scope for v1** → all locator fields kept on the type; only the 6
  core locators guaranteed populated/extracted, extended set opportunistic (§5.5).
- **Lua / wezterm value extraction** → locator-only in v1; `wezterm show-config` /
  Lua sandbox deferred (§6, §12).
- **New dependencies** → `plist` added directly (sign-off granted); structured
  formats via the `biscuit-file` path dep; no `toml`/`serde_yaml_ng`/`json-five`
  added to biscuit-terminal directly (§6, §10).
- **Warp vs the app-coverage floor** → Warp keeps its existing
  `warp_config_path` candidate so the floor stays absolute (no carve-out), and is
  reclassified off `format: None` to its real on-disk format; `format: None` is
  now reserved for apps that are both unparseable and outside the floor (§5.7, §8).
- **Installed-detection** → determined by `sniff` (`find_program_with_source`,
  covering PATH + macOS bundle + Windows registry), with the `sniff` dependency in
  `biscuit-terminal/cli` only — the library stays sniff-free and merely declares
  per-app probe targets (§5.7, §7.1, §9, §10).

## 12. Out of scope / future

- Writing/patching config files.
- Live IPC drive (`kitty @`, `wezterm cli`) — only declaration/exposure is modeled.
- **Lua / wezterm value extraction** (`wezterm show-config` shell-out or a Lua
  sandbox); v1 is locator-only (§6). `defaults`/`cfprefsd` write-through.
- **Typed value normalization.** v1 returns RAW extracted values; coercing values
  into `Color` / `Enum` / `Path` per `SettingLocator.value_kind` (today advisory
  only) is a deferred increment (§6).
- Capability facets (graphics protocol, OSC8, rounding) — owned by the broader
  `TerminalAppProfile`.
- A `bt about --all` cross-app matrix (natural follow-up once per-app works).
