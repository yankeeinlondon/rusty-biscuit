# Linux App Discovery Design

**Date:** 2026-04-11
**Status:** Draft
**Area:** sniff/lib (`programs` module)
**Related:** [windows-design.md](./windows-design.md), existing macOS implementation at `sniff/lib/src/programs/macos_bundle.rs`

## Context

`sniff` currently discovers installed programs on Linux by scanning `PATH`
only. For CLI-heavy dev environments this is fine — package managers drop
binaries into `/usr/bin` and shells start with a sane `PATH`. But for GUI
applications, and for anything installed via Flatpak or Snap, the PATH-only
strategy misses a growing share of the desktop: Firefox on Ubuntu 22.04+ is a
snap wrapper, half the apps on Fedora Silverblue are Flatpaks whose bin
exports only land in `PATH` for login shells, and KDE Plasma / XFCE
non-login sessions routinely ship environments where `/var/lib/flatpak/exports/bin`
is absent from `$PATH`. A Linux equivalent of the macOS bundle fallback should
find every GUI application that a modern Linux desktop would show in its app
launcher.

This document designs that fallback using freedesktop `.desktop` files — the
standard, stable, well-documented source of truth for Linux application
registration.

## Research Summary

Authoritative findings from the accompanying research report:

- **The [Freedesktop Desktop Entry Specification](https://specifications.freedesktop.org/desktop-entry/latest/)
  is the correct primitive.** Version 1.5 (2020) has been stable since 1.4
  (only `SingleMainWindow` was added). Treat it as a frozen target.
- **`.desktop` files are simple INI-format UTF-8 text**, grouped by `[Desktop
  Entry]` headers, with no multi-line values and no continuations. Parseable
  with a ~200-line hand-rolled tokenizer or — the recommended choice — the
  `freedesktop-desktop-entry` crate.
- **Canonical search paths** come from the XDG Base Directory Specification:
  `$XDG_DATA_HOME/applications/` and each `$XDG_DATA_DIRS/applications/`
  entry. Defaults resolve to:
  - `~/.local/share/applications/`
  - `/usr/local/share/applications/`
  - `/usr/share/applications/`
- **`TryExec` is the highest-signal key.** When present (~30-50% of real-world
  files), it is a single absolute-or-PATH-relative executable path, with no
  field codes. The spec requires implementations to verify it with
  `access(X_OK)` before treating the entry as runnable. `sniff` can use it as
  the canonical "what binary does this launcher map to?" signal.
- **`Exec` is the universal key** (`~99% coverage), but it contains field
  codes (`%f`, `%F`, `%u`, `%U`, `%i`, `%c`, `%k`, `%%`), custom quoting
  rules, and distro-specific wrappers. To recover the binary:
  1. Tokenize respecting `"..."` quoting with `\` escapes (NOT POSIX shell
     semantics — the spec is explicit).
  2. Take the first token.
  3. If absolute, stat it.
  4. If relative, resolve via `PATH` extended with Flatpak/Snap export dirs.
  5. If it is `flatpak` / `/usr/bin/flatpak` / `/snap/bin/snap`, walk forward
     to find the real app id and wrapper binary.
- **Filter keys:**
  - `Hidden=true` — equivalent to the file not existing. **Always skip.**
  - `NoDisplay=true` — runnable but not menu-visible. **Keep** (sniff cares
    about "installed", not "visible in the launcher").
  - `OnlyShowIn` / `NotShowIn` — DE-specific menu filtering. **Ignore**.
  - `Terminal=true` — needs a terminal wrapper. Informational only.
- **`Type=Application` is the only relevant type**. `Link` and `Directory`
  entries are for MIME handlers and menu hierarchy, not launchable binaries.
- **Flatpak exports its own .desktop files explicitly** into:
  - `/var/lib/flatpak/exports/share/applications/` (system)
  - `~/.local/share/flatpak/exports/share/applications/` (user)
  Plus wrapper binaries at:
  - `/var/lib/flatpak/exports/bin/` (system)
  - `~/.local/share/flatpak/exports/bin/` (user)
  **These dirs are NOT reliably on `$PATH` or `$XDG_DATA_DIRS`** — the
  propagation is via `/etc/profile.d/flatpak-bindir.sh`, which only sources
  in login shells. Non-login shells, systemd user services, and GUI-spawned
  terminals routinely see empty flatpak paths. This is the single highest-
  value gap a Linux fallback can close.
- **Snap is similar but simpler.** `.desktop` files at
  `/var/lib/snapd/desktop/applications/`, wrappers at `/snap/bin/`. The `snap`
  command inspects `argv[0]` of its symlinks. `/snap/bin` is usually in PATH
  (shipped via a similar profile script), but defensive probing is cheap.
- **AppImage has no formal standard.** Users drop `.AppImage` files into
  `~/Applications`, `~/AppImages`, `~/bin`, `~/.local/bin`, `~/Downloads`.
  Optional helpers (`appimaged`, `AppImageLauncher`) create `.desktop` entries
  under `~/.local/share/applications/appimagekit_*.desktop`. **Out of scope
  for v1** — AppImages without launcher integration are unreachable anyway,
  and integrated ones are already covered by the XDG sweep.
- **NixOS requires symlink canonicalization.** `.desktop` files live in
  `/nix/store/<hash>-<name>/share/applications/`, linked from
  `/run/current-system/sw/share/applications/` (system) and
  `~/.nix-profile/share/applications/` (user). The store hashes change on
  every rebuild, so `sniff` must **deduplicate by canonical path** but
  **cache by the stable profile path** (not the store path).
- **Distribution-specific quirks:**
  - **Fedora Silverblue / Kinoite** — rpm-ostree + Flatpak-first. Without
    Flatpak export probing, `sniff` finds almost nothing.
  - **Ubuntu 22.04+** — Firefox/Chromium/Thunderbird are snaps. The
    `firefox.desktop` file in `/usr/share/applications` often launches
    `/snap/bin/firefox`.
  - **Arch / Debian** — `desktop-file-utils` runs `update-desktop-database`
    on install/remove, producing `mimeinfo.cache`. Not useful for enumeration
    (only indexes entries with `MimeType=`), but its mtime is a cheap
    invalidation signal for caching.
- **Typical file count** on a desktop system: 100-500 `.desktop` files.
  Full parse cost: ~20 ms serial with a hand parser, ~60 ms with
  `freedesktop-desktop-entry`. Well within budget.
- **Rust crate choice:** `freedesktop-desktop-entry` (pop-os, MPL-2.0,
  actively maintained) handles XDG path resolution, Flatpak exports, BOM
  stripping, locale matching, and `PathSource` classification out of the
  box. Alternative: hand-rolled parser (~200 lines, zero deps). The crate
  wins on correctness and maintenance; the hand parser wins on dependency
  footprint. **Recommendation: use the crate** — sniff already carries
  heavier deps (`git2`, `sysinfo`, `walkdir`) and the correctness value is
  high.

## Goals

1. Find every GUI application the user's Linux desktop would show in its app
   launcher, including Flatpak and Snap installations that are absent from
   non-login-shell `$PATH`.
2. Preserve PATH as the highest-priority source; the Linux sweep is a strict
   fallback.
3. Match the existing `ExecutableIndex` "scan once, lookup many" shape so
   all eight program categories amortize the Linux scan cost.
4. Zero impact on non-Linux builds — all new code is
   `#[cfg(target_os = "linux")]`.
5. Thread-safe, panic-free. Absence is normal; malformed files must not
   crash the scan.
6. Handle NixOS symlink farms correctly (canonicalize, dedupe).

## Non-Goals

- **AppImage enumeration.** Only the `.desktop` files that `appimaged` /
  `AppImageLauncher` create are covered (via the XDG sweep). Raw `.AppImage`
  file scanning of home directories is out of scope.
- **Menu hierarchy / category classification.** We care about "is this binary
  installed?", not "which section of the Activities overview does it live
  under?"
- **D-Bus activation resolution.** If `DBusActivatable=true` and there is no
  `Exec` / `TryExec`, we cannot produce a runnable binary path and the entry
  is skipped. A future D-Bus introspection pass could close this gap, but it
  is rarely the *only* way to reach an app.
- **Icon / metadata caching.** We extract binary paths only. No thumbnail
  indexing.
- **BSDs.** FreeBSD/DragonFly may follow the freedesktop spec, but they are
  not part of sniff's support matrix. `#[cfg(target_os = "linux")]` only.

## Detection Strategy

Four layers, executed in priority order, with results cached in a single
`LinuxDesktopIndex` that feeds `ExecutableIndex::build()`.

### Layer 1 — PATH (existing)

Unchanged. Scans every directory in `$PATH` for executable files.
Already catches:

- System package installs (`/usr/bin`, `/usr/local/bin`)
- User-scope installs (`~/.local/bin`, `~/bin`, `~/.cargo/bin`, `~/.npm-global/bin`)
- `/snap/bin/` when the snapd profile script fired
- Flatpak export bin dirs when `/etc/profile.d/flatpak-bindir.sh` fired
- Nix profile bins (`~/.nix-profile/bin`, `/run/current-system/sw/bin`)

### Layer 2 — XDG applications sweep

Enumerate every `.desktop` file under:

- `$XDG_DATA_HOME/applications/` (default `~/.local/share/applications/`)
- Each `$XDG_DATA_DIRS/applications/` entry (default
  `/usr/local/share/applications/` and `/usr/share/applications/`)
- **Flatpak exports** (always probed explicitly, regardless of env vars):
  - `~/.local/share/flatpak/exports/share/applications/`
  - `/var/lib/flatpak/exports/share/applications/`
- **Snap exports** (always probed explicitly):
  - `/var/lib/snapd/desktop/applications/`

Algorithm:

```rust
#[cfg(target_os = "linux")]
fn scan_desktop_entries() -> HashMap<String, PathBuf> {
    use std::collections::HashMap;
    let mut map: HashMap<String, PathBuf> = HashMap::new();

    let search_dirs = desktop_search_dirs();

    // Iterate lowest priority first; higher-priority dirs later overwrite
    // collisions. Order: system < local-system < user < flatpak-system <
    // flatpak-user < snap. (Snap and Flatpak do not overlap with XDG dirs.)
    for dir in &search_dirs {
        let Ok(entries) = std::fs::read_dir(dir) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else { continue };
            if !name.ends_with(".desktop") { continue; }

            // Resolve symlinks (NixOS store dedup).
            let canonical = std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone());

            let Ok(parsed) = parse_desktop_entry(&canonical) else { continue };
            if parsed.hidden || parsed.type_ != DesktopType::Application { continue; }

            let Some(binary) = parsed.resolve_binary() else { continue };
            let Some(binary_name) = binary.file_name().and_then(|n| n.to_str()) else { continue };

            // Last writer wins: higher-priority dirs are scanned last.
            map.insert(binary_name.to_string(), binary.clone());

            // Also index by the desktop-id stem for lookups like
            // `org.mozilla.firefox` → firefox binary.
            let stem = name.strip_suffix(".desktop").unwrap_or(name);
            map.insert(stem.to_string(), binary);
        }
    }

    map
}
```

Key helpers:

```rust
#[cfg(target_os = "linux")]
fn desktop_search_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    // XDG system dirs (lowest priority).
    let xdg_data_dirs = std::env::var("XDG_DATA_DIRS")
        .unwrap_or_else(|_| "/usr/local/share:/usr/share".to_string());
    for dir in xdg_data_dirs.split(':') {
        if dir.is_empty() { continue; }
        dirs.push(PathBuf::from(dir).join("applications"));
    }

    // XDG user dir.
    let xdg_data_home = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share"))
        });
    if let Some(home) = xdg_data_home {
        dirs.push(home.join("applications"));
    }

    // Flatpak exports (always probed, regardless of env).
    dirs.push(PathBuf::from("/var/lib/flatpak/exports/share/applications"));
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        dirs.push(home.join(".local/share/flatpak/exports/share/applications"));
    }

    // Snap.
    dirs.push(PathBuf::from("/var/lib/snapd/desktop/applications"));

    // Deduplicate while preserving order (sets don't preserve insertion).
    let mut seen = std::collections::HashSet::new();
    dirs.retain(|d| seen.insert(d.clone()));
    dirs
}
```

### Layer 3 — Direct Flatpak/Snap export-bin probe

Even without a `.desktop` file, `sniff` should probe the Flatpak and Snap
wrapper-bin directories directly. This catches apps whose `.desktop` file
was removed or whose export-bin propagation failed:

- `/var/lib/flatpak/exports/bin/`
- `~/.local/share/flatpak/exports/bin/`
- `/snap/bin/`

Each directory is scanned the same way PATH entries are scanned (see
`find_program.rs::is_executable`). Matches are recorded with
`ExecutableSource::LinuxFlatpakBin` / `LinuxSnapBin` where appropriate.

This layer exists because `.desktop` files are the primary signal but not
the only one — a broken or uninstalled launcher with a surviving wrapper
should still resolve.

### Layer 4 — AppImage directory scan (deferred)

Out of scope for v1. Notes for future work:

- Scan `~/Applications/`, `~/AppImages/`, `~/Downloads/` for `*.AppImage`
  files.
- Verify magic bytes: ELF header at offset 0 + AppImage type-2 marker
  `0x41 0x49 0x02` at offset `0x08` (type 1 is `0x41 0x49 0x01`).
- Extract embedded `.desktop` metadata via `--appimage-extract-appinfo`
  (requires running the AppImage, which we should NOT do in a detector).
- Better approach: if the user runs `appimaged` or `AppImageLauncher`, the
  generated `.desktop` files land in `~/.local/share/applications/` and
  Layer 2 picks them up for free.

## Parsing .desktop Files

Responsibilities:

1. Strip UTF-8 BOM if present.
2. Skip `#` comment lines and blank lines.
3. Parse `[Group]` headers; only `[Desktop Entry]` matters.
4. Parse `key=value` within the group. Respect case-sensitive key names
   (`Name` ≠ `name`). Last duplicate key wins per spec.
5. Extract fields we care about: `Type`, `Name`, `Exec`, `TryExec`, `Hidden`,
   `NoDisplay`, `DBusActivatable`.
6. Do NOT apply locale resolution to `Name` — unlocalized `Name=` is the
   canonical identifier we surface.

### Recovering the binary from `Exec`

```rust
#[cfg(target_os = "linux")]
fn extract_binary_from_exec(exec: &str) -> Option<PathBuf> {
    let tokens = tokenize_exec(exec)?;
    let first = tokens.first()?;

    // Skip leading `env VAR=value` wrappers.
    let binary_token = if first == "env" || first == "/usr/bin/env" {
        tokens.iter().skip(1).find(|t| !t.contains('=')).cloned()?
    } else {
        first.clone()
    };

    // Flatpak wrapper: `flatpak run <app-id>` — fall back to the
    // flatpak-exports wrapper script which has a predictable path.
    if binary_token == "flatpak" || binary_token.ends_with("/flatpak") {
        // Find --app-id / positional <app-id> in remaining tokens.
        if let Some(app_id) = tokens.iter().skip(1).find(|t| !t.starts_with('-')) {
            let wrapper = PathBuf::from("/var/lib/flatpak/exports/bin").join(app_id);
            if wrapper.is_file() { return Some(wrapper); }
            if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
                let user = home.join(".local/share/flatpak/exports/bin").join(app_id);
                if user.is_file() { return Some(user); }
            }
        }
        // Fall through to treating `flatpak` itself as the binary.
    }

    // Snap: `Exec=/snap/bin/foo %U` — the token is already a real path.
    // Nothing special needed.

    let path = PathBuf::from(&binary_token);
    if path.is_absolute() {
        if path.is_file() { return Some(path); }
        return None;
    }

    // Relative — resolve via PATH + export bins.
    resolve_via_path_with_exports(&binary_token)
}
```

`tokenize_exec` implements the freedesktop quoting rules — explicitly *not*
POSIX shell. A minimal tokenizer:

1. Walk characters.
2. If outside quotes: whitespace is a separator.
3. `"` enters quote mode.
4. In quote mode: `\\` produces `\`, `\"` produces `"`, `` \` `` produces
   `` ` ``, `\$` produces `$`. All other `\X` is an error (per spec) but
   we forgive and emit `X`.
5. After tokenization, strip standalone `%f`/`%F`/`%u`/`%U`/`%i`/`%c`/`%k`
   tokens from the remainder. `%%` collapses to `%`.

Deliberately conservative: we prefer returning `None` (and letting the entry
drop out of the index) over returning a wrong path.

## Type Additions

Extend `ExecutableSource` with Linux-specific variants:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutableSource {
    Path,
    MacOsAppBundle,
    WindowsAppPaths,        // from windows-design.md
    WindowsInstallRoot,     // from windows-design.md
    LinuxDesktopEntry,      // NEW — found via XDG .desktop sweep
    LinuxFlatpakBin,        // NEW — found via flatpak exports/bin probe
    LinuxSnapBin,           // NEW — found via /snap/bin probe
}
```

Serialization strings: `"linux_desktop_entry"`, `"linux_flatpak_bin"`,
`"linux_snap_bin"`. Additive; existing clients tolerate unknown strings via
the existing `BoolOrEntry` + `IgnoredAny` pattern in `CategoryDetector`.

`is_app_bundle()` continues to return `false` for the Linux variants — they
are PATH-substitute sources, not bundles. Callers that want "is this a
non-PATH source?" use `is_fallback()` (proposed in the Windows design and
shared across platforms).

## Module Layout

New file: `sniff/lib/src/programs/linux_desktop.rs`. Parallel to
`macos_bundle.rs`. Public surface:

```rust
//! Linux application discovery via freedesktop .desktop files.
//!
//! Scans XDG data dirs, Flatpak exports, and Snap desktop dirs for
//! `.desktop` files and resolves them to runnable binaries. Provides a
//! fallback index for the macOS-style "find apps not in PATH" use case.

use std::collections::HashMap;
use std::path::PathBuf;

#[cfg(target_os = "linux")]
pub(super) fn build_linux_index() -> LinuxDesktopIndex {
    LinuxDesktopIndex {
        desktop_entries: scan_desktop_entries(),
        flatpak_bins: scan_flatpak_bins(),
        snap_bins: scan_snap_bins(),
    }
}

#[cfg(target_os = "linux")]
#[derive(Debug, Default, Clone)]
pub(super) struct LinuxDesktopIndex {
    /// Map keyed by binary name, desktop-id stem, and Flatpak app-id.
    pub desktop_entries: HashMap<String, PathBuf>,
    /// Map keyed by basename of Flatpak export-bin wrappers.
    pub flatpak_bins: HashMap<String, PathBuf>,
    /// Map keyed by basename of /snap/bin symlinks.
    pub snap_bins: HashMap<String, PathBuf>,
}
```

Non-Linux builds omit the module entirely via `#[cfg(target_os = "linux")]`
in `programs/mod.rs`.

## ExecutableIndex Integration

Single unified integration surface alongside the Windows extension:

```rust
#[derive(Debug, Clone)]
pub struct ExecutableIndex {
    path_executables: HashMap<String, PathBuf>,

    #[cfg(target_os = "macos")]
    bundle_executables: HashMap<String, PathBuf>,

    #[cfg(target_os = "windows")]
    windows_index: windows_apps::WindowsIndex,

    #[cfg(target_os = "linux")]
    linux_index: linux_desktop::LinuxDesktopIndex,
}

impl ExecutableIndex {
    #[instrument(skip_all)]
    pub fn build() -> Self {
        // ... existing PATH scan ...

        Self {
            path_executables,
            #[cfg(target_os = "macos")]
            bundle_executables: build_bundle_index(),
            #[cfg(target_os = "windows")]
            windows_index: windows_apps::build_windows_index(),
            #[cfg(target_os = "linux")]
            linux_index: linux_desktop::build_linux_index(),
        }
    }

    pub fn find_with_source(&self, program: &str) -> Option<(PathBuf, ExecutableSource)> {
        // Layer 1: PATH
        if let Some(p) = self.path_executables.get(program) {
            return Some((p.clone(), ExecutableSource::Path));
        }

        // Layer 2: macOS bundles
        #[cfg(target_os = "macos")]
        if let Some(p) = self.bundle_executables.get(program) {
            return Some((p.clone(), ExecutableSource::MacOsAppBundle));
        }

        // Layer 3a/b: Windows (see windows-design.md)
        #[cfg(target_os = "windows")]
        {
            if let Some(p) = self.windows_index.app_paths.get(program) {
                return Some((p.clone(), ExecutableSource::WindowsAppPaths));
            }
            if let Some(p) = self.windows_index.install_roots.get(program) {
                return Some((p.clone(), ExecutableSource::WindowsInstallRoot));
            }
        }

        // Layer 3c/d/e: Linux fallbacks
        #[cfg(target_os = "linux")]
        {
            if let Some(p) = self.linux_index.desktop_entries.get(program) {
                return Some((p.clone(), ExecutableSource::LinuxDesktopEntry));
            }
            if let Some(p) = self.linux_index.flatpak_bins.get(program) {
                return Some((p.clone(), ExecutableSource::LinuxFlatpakBin));
            }
            if let Some(p) = self.linux_index.snap_bins.get(program) {
                return Some((p.clone(), ExecutableSource::LinuxSnapBin));
            }
        }

        None
    }
}
```

Ordering rationale: `.desktop` entries have the richest metadata and are the
most authoritative signal; export-bin probing is a catch-all for cases where
the `.desktop` file is missing or malformed.

`find_program_with_source()` (the non-index entry point) gets the same
treatment but without the build-once optimization. For one-off lookups the
per-query scan cost is acceptable given typical `.desktop` file counts.

## Well-Known Name Aliases

`sniff` indexes every discovered binary by three keys:

1. The binary's own basename (e.g. `firefox`).
2. The `.desktop` file's stem (e.g. `firefox` or `org.mozilla.firefox`).
3. Any alternate binary name the program's category-enum metadata declares
   (already used by `CategoryDetector::new_with_index`).

To reduce false negatives we maintain a small alias table for programs with
divergent flatpak / native naming:

```rust
#[cfg(target_os = "linux")]
const DESKTOP_ID_ALIASES: &[(&str, &str)] = &[
    // flatpak desktop-id → canonical binary
    ("org.mozilla.firefox", "firefox"),
    ("com.google.Chrome", "google-chrome"),
    ("com.google.Chrome", "google-chrome-stable"),
    ("org.chromium.Chromium", "chromium"),
    ("com.visualstudio.code", "code"),
    ("com.vscodium.codium", "codium"),
    ("org.wezfurlong.wezterm", "wezterm"),
    ("com.valvesoftware.Steam", "steam"),
    ("org.videolan.VLC", "vlc"),
    ("com.discordapp.Discord", "discord"),
    ("com.slack.Slack", "slack"),
    ("com.spotify.Client", "spotify"),
    ("md.obsidian.Obsidian", "obsidian"),
    ("org.telegram.desktop", "telegram-desktop"),
    ("com.github.tchx84.Flatseal", "flatseal"),
];
```

The alias table is small and maintained alongside the existing
`macos_bundle::get_app_bundle_name` mapping. When a `.desktop` file's stem
matches an alias entry, we additionally index the binary under the alias
target. This is a pure augmentation — no lookups are lost if the table is
incomplete.

## Dependencies

Cargo.toml additions (Linux target only):

```toml
[target.'cfg(target_os = "linux")'.dependencies]
freedesktop-desktop-entry = "0.8"
```

Rationale:

- **Correctness.** The crate handles BOMs, quoting edge cases, locale matching,
  and the Flatpak `PathSource` classification. Re-implementing these in sniff
  is possible but adds test burden.
- **Maintenance.** Pop-os actively maintains it; the crate is used by
  COSMIC, elementary, and the broader Rust desktop ecosystem.
- **License.** MPL-2.0 — compatible with sniff's AGPL-3.0-only distribution.
- **Transitive footprint.** Adds `bstr`, `memchr`, `unicase`, `xdg` — all
  small, stable, and already transitively pulled in by other dependencies.

If the maintenance footprint ever becomes a concern, the alternative is a
~200-line hand-rolled parser. The parsing logic is isolated behind
`parse_desktop_entry()` to make swapping implementations mechanical.

## Performance and Caching

- **Scan cost:** ~20-60 ms on a warm cache for 100-500 `.desktop` files
  across 6-8 search directories. Cold cache (first read after boot) can
  spike to 200-500 ms on spinning disks but is negligible on NVMe and VMs.
- **No parallelism needed.** The scan is O(files) with cheap per-file work;
  Rayon overhead exceeds the gain at this scale. Sequential `read_dir` is
  disk-locality-friendly.
- **Deduplication on NixOS:** symlinks resolve to `/nix/store/<hash>-...`
  which we canonicalize. Two profile paths pointing at the same store entry
  produce one index entry. Dedup is by canonical path, not by desktop-id.
- **Future caching:** the Windows design notes an opportunity to persist
  the platform-specific fallback index in `HostCapabilityCacheFile`. Same
  applies here. Invalidation signal: `mtime` of any scanned directory, plus
  `mimeinfo.cache` mtime as a coarse "something changed" hint. Deferred.

## Testing Strategy

### Unit tests (runnable everywhere via `#[cfg]`-gated conditionals)

- `ExecutableSource` serialization roundtrip for the three new variants.
- Serialization strings match exactly (`"linux_desktop_entry"`, etc.).

### Linux-only unit tests

- `tokenize_exec()` over the full grammar:
  - `"foo bar"` with quoted spaces
  - `"foo \"bar\" baz"` with escaped quotes
  - `\\` → `\`
  - `%%` → `%`
  - Stray `%f`, `%U`, `%i`, `%c`, `%k` stripping
  - `env FOO=bar /usr/bin/foo` → `/usr/bin/foo`
- `extract_binary_from_exec()`:
  - Absolute path happy path
  - Flatpak wrapper (`/usr/bin/flatpak run org.mozilla.firefox @@u %u @@`)
  - Snap wrapper (`/snap/bin/firefox %U`)
  - Relative path resolution via synthetic `PATH` (with `ScopedEnv`)
- `desktop_search_dirs()` returns Flatpak + Snap dirs even when `HOME` and
  `XDG_DATA_DIRS` are unset.
- `parse_desktop_entry()` against synthetic fixtures:
  - Minimal happy case with `Exec` only
  - `TryExec` + `Exec`, confirms `TryExec` wins
  - `Hidden=true` → entry skipped
  - `NoDisplay=true` → entry kept
  - `Type=Link` → entry skipped
  - Comments, blank lines, BOM
  - Duplicate keys (last wins)
  - Multi-group file with `[Desktop Action new-window]` — only `[Desktop Entry]`
    is honored

### Integration tests (`sniff/lib/tests/`)

- `linux_desktop_discovery.rs`: creates a tempdir containing synthetic XDG
  data dirs with fake `.desktop` files and a fake `HOME`, uses `ScopedEnv`
  to override `XDG_DATA_HOME` / `XDG_DATA_DIRS` / `HOME`, runs
  `scan_desktop_entries()`, asserts the expected binary names appear.
- `linux_flatpak_wrapper.rs`: synthetic Flatpak export tree under a tempdir,
  with a desktop file whose `Exec` references the flatpak runner. Verifies
  the wrapper script is resolved as the binary.
- `linux_nixos_dedup.rs`: creates two tempdir symlinks pointing at the same
  underlying `.desktop` file, verifies the index contains exactly one entry.
- `linux_priority_over_xdg_system.rs`: same binary defined in both
  `~/.local/share/applications/` (user) and `/usr/share/applications/`
  (system), verifies user wins.

### Real-system smoke tests (`#[ignore]`, opt-in)

- `#[ignore]` tests that run `scan_desktop_entries()` against the real host
  and assert at least one well-known entry (e.g. `firefox` or
  `org.mozilla.firefox`) is present. Gated behind
  `RUN_LINUX_REAL_SCAN=1` for CI debugging.

### CI coverage

- The existing `ubuntu-latest` runner already compiles the Linux target.
  Unit tests run by default; integration tests run by default on Linux;
  real-system smoke tests stay ignored.
- Fedora Silverblue / NixOS coverage is out of scope for automated CI; a
  manual validation checklist is attached to the rollout plan.

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Malformed `.desktop` files crashing the scan | Medium | High | Per-file `Result` handling; parse errors drop the entry silently and emit a `tracing::warn!`. No `unwrap()`. |
| `Exec` tokenizer missing an edge case | High | Low | Return `None` on ambiguity — the entry is dropped, but other entries are unaffected. Fuzz-test the tokenizer with `proptest`. |
| Flatpak exports not propagating on a distro we don't know about | Medium | Medium | Direct probe of the well-known Flatpak paths does NOT depend on env; we always check them. |
| Symlink loops during canonicalization | Low | Medium | `std::fs::canonicalize` handles loops via `ELOOP`; on error we fall back to the original path. |
| NixOS store path changing between runs | High | Low | We store the canonical path in the index but *key* the index by binary name — rebuilds transparently pick up new store paths on the next run. No persistent cache by store path. |
| Large `.desktop` file counts (>1000) on heavily customized systems | Low | Medium | Scan is O(N) with small per-file constant; 1000 files is still <200 ms. Parallelize later if needed. |
| `freedesktop-desktop-entry` crate becoming unmaintained | Low | Low | Parsing is isolated behind one function; swap to hand-rolled parser if needed. |
| False positives from `DBusActivatable` entries lacking `Exec` | Medium | Low | Drop entries with neither `Exec` nor `TryExec` — a future D-Bus pass can add them back. |
| User has a chroot / sandbox with a partial `/usr/share/applications` | Low | Low | Missing dirs are skipped; scan continues. |

## Open Questions

1. **Should we surface `.desktop` metadata beyond the binary path?**
   The parse already has `Name`, `GenericName`, `Comment`, `Icon`,
   `Categories`. Exposing these would enable richer `sniff programs --json`
   output. Out of scope for v1; revisit after v1 lands.
2. **Should the Linux index participate in `HostCapabilities` caching?**
   Same question as the Windows design. Cost is low enough to defer.
3. **Should `TryExec` validation be strict?** The spec requires
   `access(X_OK)`-checking `TryExec` and hiding the entry if it fails. For
   "is this installed?" semantics, a failing `TryExec` means the entry is
   a stub and should be skipped. **Recommendation: honor it strictly.**
4. **Alias table maintenance.** The `DESKTOP_ID_ALIASES` table will drift.
   Should we pull from the existing `programs::enums` metadata instead?
   Each category enum already declares `binary_name` and
   `alternate_binary_names`; teaching `scan_desktop_entries()` to consult
   these would let us reuse the existing source of truth. Recommended for a
   v1.1 refactor.
5. **Do we want to index Desktop Actions?** Many `.desktop` files ship
   `[Desktop Action new-window]` etc. with their own `Exec` lines. These are
   typically the same binary with different flags; indexing them adds noise.
   Defer.

## Rollout Plan

1. **Phase 1** — `ExecutableSource` extension (three new Linux variants).
   Pure additive change, shared with the Windows extension PR if landing in
   parallel.
2. **Phase 2** — `linux_desktop.rs` module: `desktop_search_dirs()`,
   `tokenize_exec()`, `extract_binary_from_exec()`, `parse_desktop_entry()`.
   Unit-tested in isolation against synthetic fixtures.
3. **Phase 3** — `scan_desktop_entries()` end-to-end. Integration tests
   against tempdir fixtures.
4. **Phase 4** — Flatpak / Snap export-bin probe layers.
5. **Phase 5** — `ExecutableIndex` integration and
   `find_program_with_source()` parity. Integration tests against a
   real Ubuntu / Fedora runner.
6. **Phase 6** — Alias table seeded from real-world desktop-ids; research
   merge with `programs::enums` metadata deferred to v1.1.
7. **Phase 7** — Documentation: update `sniff/lib/README.md`,
   `.claude/skills/sniff/programs.md`, and
   `sniff/docs/sniff-library-architecture.md`.
8. **Phase 8** — Manual validation on three reference systems:
   Ubuntu 22.04 (snap-heavy), Fedora Silverblue (flatpak-only), and NixOS
   unstable (symlink farms). Attach a validation checklist to the PR.

Each phase is independently mergeable. Phases 1-4 do not require a real
Linux desktop thanks to tempdir fixtures; phase 8 is the manual real-system
pass.

## References

- [Freedesktop Desktop Entry Specification 1.5](https://specifications.freedesktop.org/desktop-entry/latest/)
- [Exec variables / field codes](https://specifications.freedesktop.org/desktop-entry/1.5/exec-variables.html)
- [Desktop entry recognized keys](https://specifications.freedesktop.org/desktop-entry/1.5/recognized-keys.html)
- [XDG Base Directory Specification](https://specifications.freedesktop.org/basedir-spec/latest/)
- [Desktop Menu Specification](https://specifications.freedesktop.org/menu-spec/menu-spec-latest.html)
- [Flatpak conventions](https://docs.flatpak.org/en/latest/conventions.html)
- [Snap Execution Environment](https://github.com/canonical/snapd/wiki/Snap-Execution-Environment)
- [pop-os/freedesktop-desktop-entry](https://github.com/pop-os/freedesktop-desktop-entry)
- [Arch Wiki — Desktop entries](https://wiki.archlinux.org/title/Desktop_entries)
- [Debian manpage — update-desktop-database](https://manpages.debian.org/testing/desktop-file-utils/update-desktop-database.1.en.html)
- [NixOS Discourse — Where are .desktop files located?](https://discourse.nixos.org/t/where-are-desktop-files-located/17391)
