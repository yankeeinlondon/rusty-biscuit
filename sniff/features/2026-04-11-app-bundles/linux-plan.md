# Linux App Discovery Implementation Plan


**Goal:** Extend `sniff`'s program detection on Linux beyond `PATH` to cover GUI apps registered through freedesktop `.desktop` files (XDG data dirs, Flatpak exports, Snap desktop dirs) plus direct probes of Flatpak/Snap wrapper-bin directories, so that any app a modern Linux desktop's app launcher would show is also found by `sniff`.

**Architecture:** A layered fallback chain inside the existing `ExecutableIndex`. PATH remains authoritative (layer 1). On Linux, a new `LinuxDesktopIndex` is populated at build time by (a) sweeping `.desktop` files under every XDG applications dir plus Flatpak and Snap exports, resolving each entry to a concrete binary, and (b) directly probing the Flatpak/Snap export-bin directories for wrapper scripts that never made it into `$PATH`. Three new `ExecutableSource` variants (`LinuxDesktopEntry`, `LinuxFlatpakBin`, `LinuxSnapBin`) surface the discovery source through the existing `CategoryDetector` serialization with zero schema churn. All new code is `#[cfg(target_os = "linux")]`; non-Linux builds compile and test unchanged.

**Tech Stack:** Rust 2024, hand-rolled freedesktop Desktop Entry tokenizer + parser (no new crate deps), `std::fs::canonicalize` for NixOS symlink dedup, existing `rayon`/`thiserror`/`serde`/`tracing` infrastructure. Cross-checked against `x86_64-unknown-linux-gnu`.

**Spec:** [`linux-design.md`](./linux-design.md) — read this first.

> **Compatibility note with `windows-plan.md`:** Both plans add new variants to `ExecutableSource` and introduce the `is_fallback()` helper. This plan is self-contained: if the Windows plan has already landed, Task 2's `is_fallback()` addition is already done — just add the three Linux variants to the match arms and the test file. If the Windows plan lands later, it will only need to add its own variants to the Linux-flavored `is_fallback()` and `Display` impls. Neither plan must wait for the other.

---

## File Structure

### New files

- `sniff/lib/src/programs/linux_desktop.rs` — Linux discovery module (mirrors `macos_bundle.rs` and the Windows plan's `windows_apps.rs`).
  Contains: `LinuxDesktopIndex` struct, `build_linux_index()`, `desktop_search_dirs()`, `scan_desktop_entries()`, `scan_flatpak_bins()`, `scan_snap_bins()`, `parse_desktop_entry()`, `tokenize_exec()`, `extract_binary_from_exec()`, `DESKTOP_ID_ALIASES`, and private helpers. Entirely gated on `#[cfg(target_os = "linux")]`.
- `sniff/lib/tests/linux_desktop_discovery.rs` — Linux-only integration test against a synthetic XDG tree.
- `sniff/lib/tests/linux_flatpak_wrapper.rs` — Linux-only integration test for the Flatpak wrapper fallback.
- `sniff/lib/tests/linux_nixos_dedup.rs` — Linux-only integration test for symlink canonicalization dedup.
- `sniff/lib/tests/linux_priority_over_xdg_system.rs` — Linux-only integration test for user-dir precedence over system dirs.

### Modified files

- `sniff/lib/src/programs/mod.rs` — register the new `linux_desktop` module behind `#[cfg(target_os = "linux")]` and re-export the discovery source variants (already re-exported via `pub use types::*`).
- `sniff/lib/src/programs/types.rs` — extend `ExecutableSource` with `LinuxDesktopEntry`, `LinuxFlatpakBin`, `LinuxSnapBin`, add `is_fallback()` if not already present, and extend `Display`.
- `sniff/lib/src/programs/find_program.rs` — add a `linux_index: LinuxDesktopIndex` field to `ExecutableIndex`, build it in `build_with_bundles`, and extend `find_with_source()` to consult the Linux layers after PATH. Extend the one-off `find_program_with_source()` helper with the same chain.
- `sniff/lib/README.md` — document the Linux fallback chain alongside the macOS section.
- `sniff/docs/sniff-library-architecture.md` — note the Linux index cost model (~20–60 ms warm) under the shared-work discussion.
- `.claude/skills/sniff/programs.md` — add Linux detection notes alongside the macOS bundle notes.

### Supporting context

- `sniff/lib/src/programs/macos_bundle.rs` — reference implementation for the parallel macOS side; do **not** modify.
- `sniff/lib/src/programs/find_program.rs` — the existing `is_executable()` helper already implements the correct Unix "regular file + `0o111` bit set" semantics. Reuse it rather than re-implementing.
- `sniff/lib/src/test_helpers.rs` — `ENV_MUTEX` + `ScopedEnv` already serialize env-mutating tests. Reuse for all Linux tests that touch `XDG_DATA_HOME`, `XDG_DATA_DIRS`, or `HOME`.
- `sniff/lib/src/programs/schema.rs:479` — `ProgramEntry::installed()` serializes `source` via `to_string().to_lowercase().replace(' ', "_")`, so the new variants' `Display` output (`"Linux Desktop Entry"`, `"Linux Flatpak Bin"`, `"Linux Snap Bin"`) will auto-produce the correct `"linux_desktop_entry"` / `"linux_flatpak_bin"` / `"linux_snap_bin"` strings. No schema change required.

---

## Task 1: Verify Linux cross-compile target and Cargo baseline

**Goal:** Make the Linux codepath buildable (via `cargo check`) from a macOS dev host and confirm the Cargo.toml baseline is sound. The design doc recommends the `freedesktop-desktop-entry` crate; this plan intentionally uses a hand-rolled parser instead (simpler, zero dependency churn, ~200 lines as the design doc itself notes). No new Cargo dependencies are added.

**Files:**
- Verify only: `sniff/lib/Cargo.toml`

- [ ] **Step 1: Install the Linux GNU target if missing**

Run:

```bash
rustup target list --installed | grep -q x86_64-unknown-linux-gnu || rustup target add x86_64-unknown-linux-gnu
```

Expected: target already installed or added successfully. If this step fails due to sandbox restrictions, run it manually in an interactive shell before continuing.

- [ ] **Step 2: Verify the workspace still compiles on the host**

Run:

```bash
cargo check -p sniff
```

Expected: clean build.

- [ ] **Step 3: Verify the Linux target cross-check works**

Run:

```bash
cargo check -p sniff --target x86_64-unknown-linux-gnu
```

Expected: clean check (type-check only, no linker invocation needed). On macOS this is the primary "does Linux code compile?" signal throughout the plan. `cargo build` is not expected to succeed without a linux-gnu sysroot; **use `cargo check` for every Linux cross-validation step in this plan**.

- [ ] **Step 4: No commit for this task**

This task only validates tooling; nothing has changed.

---

## Task 2: Extend `ExecutableSource` with Linux variants

**Goal:** Add `LinuxDesktopEntry`, `LinuxFlatpakBin`, and `LinuxSnapBin` variants, update `Display`, and introduce `is_fallback()`. All changes are compile-safe and testable on every platform.

**Files:**
- Modify: `sniff/lib/src/programs/types.rs`
- Test: `sniff/lib/src/programs/types.rs` (existing `mod tests`)

- [ ] **Step 1: Write the failing tests for the new variants**

Add to the `#[cfg(test)] mod tests` block in `sniff/lib/src/programs/types.rs`, near the existing `test_executable_source_serialize_json`:

```rust
#[test]
fn test_executable_source_linux_variants_serialize() {
    let desktop = ExecutableSource::LinuxDesktopEntry;
    let flatpak = ExecutableSource::LinuxFlatpakBin;
    let snap = ExecutableSource::LinuxSnapBin;

    assert_eq!(
        serde_json::to_string(&desktop).unwrap(),
        "\"linux_desktop_entry\""
    );
    assert_eq!(
        serde_json::to_string(&flatpak).unwrap(),
        "\"linux_flatpak_bin\""
    );
    assert_eq!(
        serde_json::to_string(&snap).unwrap(),
        "\"linux_snap_bin\""
    );
}

#[test]
fn test_executable_source_linux_variants_deserialize() {
    let d: ExecutableSource =
        serde_json::from_str("\"linux_desktop_entry\"").unwrap();
    let f: ExecutableSource =
        serde_json::from_str("\"linux_flatpak_bin\"").unwrap();
    let s: ExecutableSource =
        serde_json::from_str("\"linux_snap_bin\"").unwrap();

    assert_eq!(d, ExecutableSource::LinuxDesktopEntry);
    assert_eq!(f, ExecutableSource::LinuxFlatpakBin);
    assert_eq!(s, ExecutableSource::LinuxSnapBin);
}

#[test]
fn test_executable_source_linux_variants_display() {
    assert_eq!(
        ExecutableSource::LinuxDesktopEntry.to_string(),
        "Linux Desktop Entry"
    );
    assert_eq!(
        ExecutableSource::LinuxFlatpakBin.to_string(),
        "Linux Flatpak Bin"
    );
    assert_eq!(
        ExecutableSource::LinuxSnapBin.to_string(),
        "Linux Snap Bin"
    );
}

#[test]
fn test_executable_source_linux_variants_not_app_bundle() {
    assert!(!ExecutableSource::LinuxDesktopEntry.is_app_bundle());
    assert!(!ExecutableSource::LinuxFlatpakBin.is_app_bundle());
    assert!(!ExecutableSource::LinuxSnapBin.is_app_bundle());
}

#[test]
fn test_executable_source_is_fallback() {
    assert!(!ExecutableSource::Path.is_fallback());
    assert!(ExecutableSource::MacOsAppBundle.is_fallback());
    assert!(ExecutableSource::LinuxDesktopEntry.is_fallback());
    assert!(ExecutableSource::LinuxFlatpakBin.is_fallback());
    assert!(ExecutableSource::LinuxSnapBin.is_fallback());
}

#[test]
fn test_executable_source_roundtrip_linux_variants() {
    for source in [
        ExecutableSource::LinuxDesktopEntry,
        ExecutableSource::LinuxFlatpakBin,
        ExecutableSource::LinuxSnapBin,
    ] {
        let json = serde_json::to_string(&source).unwrap();
        let back: ExecutableSource = serde_json::from_str(&json).unwrap();
        assert_eq!(source, back);
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run:

```bash
cargo test -p sniff programs::types::tests::test_executable_source_linux -- --nocapture
cargo test -p sniff programs::types::tests::test_executable_source_is_fallback -- --nocapture
```

Expected: both commands fail with compile errors (`no variant LinuxDesktopEntry`, `no method is_fallback`).

- [ ] **Step 3: Add the variants and helpers**

In `sniff/lib/src/programs/types.rs`, replace the existing `ExecutableSource` definition (currently the `#[serde(rename_all = "snake_case")] pub enum ExecutableSource { Path, MacOsAppBundle, }` block near line 43–50) and its `impl` block (near line 52–67) and its `Display` impl (near line 69–76) with:

```rust
/// Describes where a program executable was discovered.
///
/// Distinguishes between traditional PATH-based executables, macOS `.app`
/// bundles, and Linux-specific fallback sources (XDG `.desktop` files,
/// Flatpak export wrappers, Snap export wrappers). Non-PATH sources are
/// "fallback" sources — they are consulted only when PATH lookup misses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutableSource {
    /// Found via PATH lookup (traditional executable).
    Path,
    /// Found as a macOS `.app` bundle.
    MacOsAppBundle,
    /// Found via a freedesktop `.desktop` file under an XDG applications
    /// directory, a Flatpak export, or a Snap desktop directory.
    LinuxDesktopEntry,
    /// Found via a direct probe of
    /// `/var/lib/flatpak/exports/bin/` or
    /// `~/.local/share/flatpak/exports/bin/` wrapper scripts.
    LinuxFlatpakBin,
    /// Found via a direct probe of `/snap/bin/` wrapper scripts.
    LinuxSnapBin,
}

impl ExecutableSource {
    /// Returns `true` if this source is a macOS app bundle.
    #[must_use]
    pub fn is_app_bundle(&self) -> bool {
        matches!(self, Self::MacOsAppBundle)
    }

    /// Returns `true` if this source is a non-PATH fallback source.
    ///
    /// Callers that want to know "did we find this through something other
    /// than the standard PATH?" should use this instead of pattern matching
    /// against every fallback variant.
    #[must_use]
    pub fn is_fallback(&self) -> bool {
        !matches!(self, Self::Path)
    }
}

impl std::fmt::Display for ExecutableSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecutableSource::Path => write!(f, "PATH"),
            ExecutableSource::MacOsAppBundle => write!(f, "macOS App Bundle"),
            ExecutableSource::LinuxDesktopEntry => {
                write!(f, "Linux Desktop Entry")
            }
            ExecutableSource::LinuxFlatpakBin => write!(f, "Linux Flatpak Bin"),
            ExecutableSource::LinuxSnapBin => write!(f, "Linux Snap Bin"),
        }
    }
}
```

> **If the Windows plan already landed:** `is_fallback()` will already exist and `ExecutableSource` will already have `WindowsAppPaths` / `WindowsInstallRoot` variants. In that case, leave the existing variants and `is_fallback()` alone — just add the three new Linux variants to the enum body, the `Display` match, and the tests.

- [ ] **Step 4: Update the existing `test_executable_source_pattern_matching` to cover the new variants**

The existing test at the bottom of the `tests` module exhaustively matches every variant inside a helper `fn describe_source`. Extend it so it still compiles:

```rust
#[test]
fn test_executable_source_pattern_matching() {
    fn describe_source(source: ExecutableSource) -> &'static str {
        match source {
            ExecutableSource::Path => "path",
            ExecutableSource::MacOsAppBundle => "bundle",
            ExecutableSource::LinuxDesktopEntry => "linux_desktop",
            ExecutableSource::LinuxFlatpakBin => "linux_flatpak",
            ExecutableSource::LinuxSnapBin => "linux_snap",
        }
    }

    assert_eq!(describe_source(ExecutableSource::Path), "path");
    assert_eq!(describe_source(ExecutableSource::MacOsAppBundle), "bundle");
    assert_eq!(
        describe_source(ExecutableSource::LinuxDesktopEntry),
        "linux_desktop"
    );
    assert_eq!(
        describe_source(ExecutableSource::LinuxFlatpakBin),
        "linux_flatpak"
    );
    assert_eq!(
        describe_source(ExecutableSource::LinuxSnapBin),
        "linux_snap"
    );
}
```

- [ ] **Step 5: Run the new tests and verify they pass**

Run:

```bash
cargo test -p sniff programs::types::tests::test_executable_source -- --nocapture
```

Expected: all `test_executable_source_*` tests pass, including the new ones.

- [ ] **Step 6: Run the full types module tests to catch regressions**

Run:

```bash
cargo test -p sniff programs::types::tests
```

Expected: every test in the module passes. The existing serialization tests for `Path` and `MacOsAppBundle` still pass (the old variants are unchanged).

- [ ] **Step 7: Cross-check the Linux target still compiles**

Run:

```bash
cargo check -p sniff --target x86_64-unknown-linux-gnu
```

Expected: clean build.

- [ ] **Step 8: Commit**

```bash
git add sniff/lib/src/programs/types.rs
git commit -m "feat(sniff): add Linux fallback variants to ExecutableSource"
```

---

## Task 3: Scaffold `linux_desktop` module with empty `LinuxDesktopIndex`

**Goal:** Create the new module file, register it in `programs/mod.rs` under `#[cfg(target_os = "linux")]`, and stub out `build_linux_index()` returning an empty index. Subsequent tasks flesh out each layer.

**Files:**
- Create: `sniff/lib/src/programs/linux_desktop.rs`
- Modify: `sniff/lib/src/programs/mod.rs`

- [ ] **Step 1: Create the new module file**

Create `sniff/lib/src/programs/linux_desktop.rs` with:

```rust
//! Linux application discovery via freedesktop `.desktop` files and
//! Flatpak/Snap wrapper probes.
//!
//! Scans XDG data dirs, Flatpak exports, and Snap desktop dirs for
//! `.desktop` files, resolves them to runnable binaries, and also directly
//! probes the Flatpak and Snap wrapper-bin directories. Provides a fallback
//! index for the "find apps not in PATH" use case on Linux desktops where
//! Flatpak/Snap installations often leave `$PATH` empty for GUI apps.
//!
//! All public items in this module are gated on
//! `#[cfg(target_os = "linux")]`. Non-Linux builds do not compile this code.

#![cfg(target_os = "linux")]

use std::collections::HashMap;
use std::path::PathBuf;

/// Builds the Linux-specific fallback index.
///
/// Performs three scans once:
/// 1. `.desktop` files across XDG data dirs, Flatpak exports, and Snap
///    desktop dirs (parsed into binary paths).
/// 2. Flatpak wrapper binaries under
///    `/var/lib/flatpak/exports/bin/` and
///    `~/.local/share/flatpak/exports/bin/`.
/// 3. Snap wrapper binaries under `/snap/bin/`.
///
/// Returns a fully-populated `LinuxDesktopIndex` ready for O(1) lookups.
///
/// ## Cost
///
/// Typical warm-cache cost: 20–60 ms serial for 100–500 `.desktop` files
/// across 6–8 directories. Called once per `ExecutableIndex::build()` on
/// Linux.
pub(super) fn build_linux_index() -> LinuxDesktopIndex {
    LinuxDesktopIndex {
        desktop_entries: scan_desktop_entries(),
        flatpak_bins: scan_flatpak_bins(),
        snap_bins: scan_snap_bins(),
    }
}

/// Linux-specific fallback index populated by [`build_linux_index`].
///
/// Three HashMaps keyed by binary basename (lowercase-preserving). Checked
/// in priority order (`desktop_entries` → `flatpak_bins` → `snap_bins`)
/// after PATH by
/// [`crate::programs::find_program::ExecutableIndex::find_with_source`].
#[derive(Debug, Default, Clone)]
pub(super) struct LinuxDesktopIndex {
    /// Map keyed by binary basename, desktop-id stem, and Flatpak app-id,
    /// built from the XDG + Flatpak + Snap `.desktop` sweep.
    pub desktop_entries: HashMap<String, PathBuf>,
    /// Map keyed by wrapper-script basename from the Flatpak export-bin
    /// directories.
    pub flatpak_bins: HashMap<String, PathBuf>,
    /// Map keyed by wrapper-script basename from `/snap/bin/`.
    pub snap_bins: HashMap<String, PathBuf>,
}

/// Placeholder — populated in Task 8.
fn scan_desktop_entries() -> HashMap<String, PathBuf> {
    HashMap::new()
}

/// Placeholder — populated in Task 9.
fn scan_flatpak_bins() -> HashMap<String, PathBuf> {
    HashMap::new()
}

/// Placeholder — populated in Task 9.
fn scan_snap_bins() -> HashMap<String, PathBuf> {
    HashMap::new()
}
```

- [ ] **Step 2: Register the module in `programs/mod.rs`**

In `sniff/lib/src/programs/mod.rs`, find the existing list of `pub mod ...` declarations near line 95-110. After the `pub mod macos_bundle;` line, add:

```rust
#[cfg(target_os = "linux")]
pub(crate) mod linux_desktop;
```

- [ ] **Step 3: Host-side check**

Run:

```bash
cargo check -p sniff
cargo test -p sniff programs:: --lib
```

Expected: clean build and all existing tests still pass (the new module is absent on non-Linux, so nothing changes).

- [ ] **Step 4: Linux cross-check**

Run:

```bash
cargo check -p sniff --target x86_64-unknown-linux-gnu
```

Expected: clean build. The placeholder functions satisfy the signatures consumed by `build_linux_index`.

- [ ] **Step 5: Commit**

```bash
git add sniff/lib/src/programs/linux_desktop.rs sniff/lib/src/programs/mod.rs
git commit -m "feat(sniff): scaffold linux_desktop module with empty LinuxDesktopIndex"
```

---

## Task 4: Implement `desktop_search_dirs()`

**Goal:** Implement the XDG base-dir resolution plus unconditional Flatpak and Snap directory probing. Pure function (no I/O), drivable with `ScopedEnv`.

**Files:**
- Modify: `sniff/lib/src/programs/linux_desktop.rs`

- [ ] **Step 1: Add the function at the bottom of `linux_desktop.rs`**

Append to `sniff/lib/src/programs/linux_desktop.rs`:

```rust
/// Returns the list of directories to sweep for `.desktop` files, in
/// priority order (lowest priority first — higher-priority dirs should
/// overwrite earlier ones when a binary name collides).
///
/// Order:
/// 1. `$XDG_DATA_DIRS` entries (default `/usr/local/share:/usr/share`)
///    with `/applications` appended.
/// 2. `$XDG_DATA_HOME/applications` (default `~/.local/share/applications`).
/// 3. `/var/lib/flatpak/exports/share/applications` (system Flatpak).
/// 4. `~/.local/share/flatpak/exports/share/applications` (user Flatpak).
/// 5. `/var/lib/snapd/desktop/applications` (Snap).
///
/// Flatpak and Snap directories are always included, regardless of whether
/// they appear in any env var — this is the whole point of the Linux
/// fallback chain. Duplicates are filtered while preserving order.
fn desktop_search_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();

    // XDG system dirs (lowest priority).
    let xdg_data_dirs = std::env::var("XDG_DATA_DIRS")
        .unwrap_or_else(|_| "/usr/local/share:/usr/share".to_string());
    for dir in xdg_data_dirs.split(':') {
        if dir.is_empty() {
            continue;
        }
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

    // Flatpak exports — always probed, regardless of env.
    dirs.push(PathBuf::from(
        "/var/lib/flatpak/exports/share/applications",
    ));
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        dirs.push(home.join(".local/share/flatpak/exports/share/applications"));
    }

    // Snap.
    dirs.push(PathBuf::from("/var/lib/snapd/desktop/applications"));

    // Dedupe while preserving the insertion order defined above.
    let mut seen: std::collections::HashSet<PathBuf> =
        std::collections::HashSet::new();
    dirs.retain(|d| seen.insert(d.clone()));

    dirs
}
```

- [ ] **Step 2: Add unit tests for `desktop_search_dirs()`**

At the bottom of `sniff/lib/src/programs/linux_desktop.rs`, create the tests module if it does not yet exist and add:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{ENV_MUTEX, ScopedEnv};

    #[test]
    fn desktop_search_dirs_includes_flatpak_and_snap_when_env_empty() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let mut env = ScopedEnv::new();
        env.remove("XDG_DATA_HOME");
        env.remove("XDG_DATA_DIRS");
        env.remove("HOME");

        let dirs = desktop_search_dirs();

        // System defaults
        assert!(dirs.contains(&PathBuf::from("/usr/local/share/applications")));
        assert!(dirs.contains(&PathBuf::from("/usr/share/applications")));

        // Flatpak system + snap always present
        assert!(
            dirs.contains(&PathBuf::from(
                "/var/lib/flatpak/exports/share/applications"
            ))
        );
        assert!(dirs.contains(&PathBuf::from(
            "/var/lib/snapd/desktop/applications"
        )));

        // HOME-dependent dirs should be absent when HOME is unset
        let has_user_flatpak = dirs.iter().any(|d| {
            d.to_string_lossy()
                .ends_with(".local/share/flatpak/exports/share/applications")
        });
        assert!(
            !has_user_flatpak,
            "user Flatpak dir must be skipped when HOME is unset"
        );
    }

    #[test]
    fn desktop_search_dirs_respects_xdg_data_home() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let mut env = ScopedEnv::new();
        env.set("XDG_DATA_HOME", "/custom/xdg-data");
        env.set("XDG_DATA_DIRS", "/custom/a:/custom/b");
        env.remove("HOME");

        let dirs = desktop_search_dirs();

        assert!(dirs.contains(&PathBuf::from("/custom/xdg-data/applications")));
        assert!(dirs.contains(&PathBuf::from("/custom/a/applications")));
        assert!(dirs.contains(&PathBuf::from("/custom/b/applications")));
    }

    #[test]
    fn desktop_search_dirs_derives_user_dir_from_home_when_xdg_home_missing() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let mut env = ScopedEnv::new();
        env.remove("XDG_DATA_HOME");
        env.set("HOME", "/tmp/home");

        let dirs = desktop_search_dirs();

        assert!(dirs.contains(&PathBuf::from("/tmp/home/.local/share/applications")));
        assert!(dirs.contains(&PathBuf::from(
            "/tmp/home/.local/share/flatpak/exports/share/applications"
        )));
    }

    #[test]
    fn desktop_search_dirs_deduplicates() {
        let _lock = ENV_MUTEX.lock().unwrap();
        let mut env = ScopedEnv::new();
        env.set("XDG_DATA_DIRS", "/usr/share:/usr/share:/usr/share");
        env.remove("XDG_DATA_HOME");
        env.remove("HOME");

        let dirs = desktop_search_dirs();

        let count = dirs
            .iter()
            .filter(|d| d == &&PathBuf::from("/usr/share/applications"))
            .count();
        assert_eq!(count, 1, "duplicate XDG entries must be collapsed");
    }
}
```

- [ ] **Step 3: Host-side check**

Run:

```bash
cargo check -p sniff
```

Expected: clean build. The new module is `#[cfg(target_os = "linux")]`, so macOS does not compile any of it.

- [ ] **Step 4: Linux cross-check**

Run:

```bash
cargo check -p sniff --target x86_64-unknown-linux-gnu
cargo check -p sniff --target x86_64-unknown-linux-gnu --tests
```

Expected: clean build for both the library and the tests.

- [ ] **Step 5: Commit**

```bash
git add sniff/lib/src/programs/linux_desktop.rs
git commit -m "feat(sniff): resolve Linux desktop search dirs from XDG + Flatpak + Snap"
```

---

## Task 5: Implement `tokenize_exec()`

**Goal:** Implement the freedesktop Exec-string tokenizer. The spec is explicit that this is **not** POSIX shell quoting: only double quotes are honored, `\\`/`\"`/`` \` ``/`\$` are the allowed escape sequences, and field codes `%f`/`%F`/`%u`/`%U`/`%i`/`%c`/`%k` are stripped while `%%` collapses to `%`.

**Files:**
- Modify: `sniff/lib/src/programs/linux_desktop.rs`

- [ ] **Step 1: Add the tokenizer at the bottom of `linux_desktop.rs` (above the `#[cfg(test)]` block)**

Append:

```rust
/// Tokenizes a freedesktop `Exec=` string.
///
/// This is explicitly **not** POSIX shell quoting. Per the freedesktop
/// Desktop Entry specification:
///
/// - Unquoted whitespace separates tokens.
/// - Double quotes (`"..."`) group a single token.
/// - Inside double quotes, the only valid escapes are `\\`, `\"`,
///   `` \` ``, and `\$`. Any other `\X` is technically an error; we
///   forgive it and emit `X`.
/// - Single quotes have no special meaning.
/// - Field codes (`%f`, `%F`, `%u`, `%U`, `%i`, `%c`, `%k`, `%d`, `%D`,
///   `%n`, `%N`, `%v`, `%m`) are stripped when they appear as a standalone
///   token. `%%` collapses to `%` (wherever it appears).
///
/// Returns `None` when the string is empty after trimming. Callers should
/// treat that as "no binary resolvable from this Exec".
fn tokenize_exec(exec: &str) -> Option<Vec<String>> {
    // First pass: collapse `%%` to `%` while respecting nothing else —
    // the spec says `%%` is literal regardless of quoting context.
    let mut pre: String = String::with_capacity(exec.len());
    let mut chars = exec.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%' {
            if chars.peek() == Some(&'%') {
                chars.next();
                pre.push('%');
                continue;
            }
            // Field code: consume the following character (the code
            // letter) and emit a sentinel so the tokenizer can drop the
            // whole token later. We use `\x01{letter}` which is unreachable
            // in a valid Exec string.
            if let Some(&next) = chars.peek() {
                chars.next();
                pre.push('\x01');
                pre.push(next);
                continue;
            }
            // Trailing lone `%` — emit as-is.
            pre.push('%');
            continue;
        }
        pre.push(c);
    }

    // Second pass: quote-aware splitter.
    let mut tokens: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = pre.chars().peekable();

    while let Some(c) = chars.next() {
        match (c, in_quotes) {
            ('"', _) => {
                in_quotes = !in_quotes;
            }
            ('\\', true) => {
                // Freedesktop escape inside quotes: `\\`, `\"`, `` \` ``, `\$`.
                if let Some(&next) = chars.peek() {
                    chars.next();
                    current.push(next);
                } else {
                    // Dangling backslash at EOS inside quotes — emit it
                    // literally. Matches the behavior of a forgiving parser.
                    current.push('\\');
                }
            }
            (c, false) if c.is_whitespace() => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            (c, _) => current.push(c),
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }

    // Drop field-code tokens (those that are *only* `\x01X`).
    tokens.retain(|t| !(t.len() == 2 && t.starts_with('\x01')));

    if tokens.is_empty() {
        None
    } else {
        Some(tokens)
    }
}
```

- [ ] **Step 2: Add unit tests to the existing `tests` module**

Append inside the existing `#[cfg(test)] mod tests` block in the same file:

```rust
#[test]
fn tokenize_exec_plain() {
    let tokens = tokenize_exec("/usr/bin/firefox").unwrap();
    assert_eq!(tokens, vec!["/usr/bin/firefox".to_string()]);
}

#[test]
fn tokenize_exec_with_field_code() {
    let tokens = tokenize_exec("/usr/bin/firefox %u").unwrap();
    assert_eq!(tokens, vec!["/usr/bin/firefox".to_string()]);
}

#[test]
fn tokenize_exec_quoted_argument_with_space() {
    let tokens = tokenize_exec(r#"echo "hello world""#).unwrap();
    assert_eq!(
        tokens,
        vec!["echo".to_string(), "hello world".to_string()]
    );
}

#[test]
fn tokenize_exec_escaped_quote() {
    let tokens = tokenize_exec(r#"echo "foo \"bar\" baz""#).unwrap();
    assert_eq!(
        tokens,
        vec!["echo".to_string(), "foo \"bar\" baz".to_string()]
    );
}

#[test]
fn tokenize_exec_escaped_backslash() {
    let tokens = tokenize_exec(r#"echo "a\\b""#).unwrap();
    assert_eq!(tokens, vec!["echo".to_string(), "a\\b".to_string()]);
}

#[test]
fn tokenize_exec_percent_percent() {
    let tokens = tokenize_exec("printf %%s").unwrap();
    assert_eq!(tokens, vec!["printf".to_string(), "%s".to_string()]);
}

#[test]
fn tokenize_exec_strips_all_field_codes() {
    // Every field code listed in the spec must be dropped as a token.
    let tokens = tokenize_exec("cmd %f %F %u %U %i %c %k %d %D %n %N %v %m")
        .unwrap();
    assert_eq!(tokens, vec!["cmd".to_string()]);
}

#[test]
fn tokenize_exec_env_prefix() {
    let tokens = tokenize_exec("env FOO=bar /usr/bin/foo").unwrap();
    assert_eq!(
        tokens,
        vec![
            "env".to_string(),
            "FOO=bar".to_string(),
            "/usr/bin/foo".to_string()
        ]
    );
}

#[test]
fn tokenize_exec_empty_returns_none() {
    assert!(tokenize_exec("").is_none());
    assert!(tokenize_exec("   ").is_none());
}
```

- [ ] **Step 3: Host-side check**

Run:

```bash
cargo check -p sniff
```

Expected: clean build.

- [ ] **Step 4: Linux cross-check (run tests too)**

Run:

```bash
cargo check -p sniff --target x86_64-unknown-linux-gnu --tests
```

Expected: clean build.

- [ ] **Step 5: Commit**

```bash
git add sniff/lib/src/programs/linux_desktop.rs
git commit -m "feat(sniff): add freedesktop Exec tokenizer for .desktop parsing"
```

---

## Task 6: Implement `extract_binary_from_exec()`

**Goal:** Convert the tokenized Exec command into a resolvable binary `PathBuf`, handling the `env`-prefix, Flatpak wrapper, and PATH-relative resolution with Flatpak/Snap export-bin fallback. Conservative: return `None` on ambiguity.

**Files:**
- Modify: `sniff/lib/src/programs/linux_desktop.rs`

- [ ] **Step 1: Add the binary resolver immediately after `tokenize_exec`**

Append to `sniff/lib/src/programs/linux_desktop.rs` (above the `#[cfg(test)]` block):

```rust
/// Resolves a freedesktop `Exec=` string to a runnable binary path.
///
/// Strategy:
/// 1. Tokenize with [`tokenize_exec`].
/// 2. Skip leading `env [VAR=value ...]` wrapper and take the first
///    non-assignment token as the binary.
/// 3. If the binary token is `flatpak` / `/usr/bin/flatpak`, walk forward
///    to find the Flatpak app-id (first non-flag token) and try to resolve
///    `/var/lib/flatpak/exports/bin/<app-id>` or the user equivalent. If
///    neither exists, fall through and return the `flatpak` binary itself.
/// 4. If the binary token is absolute, stat it; return on success.
/// 5. Otherwise resolve via PATH extended with Flatpak and Snap export bins.
///
/// Returns `None` when no candidate is resolvable. Callers drop the entry
/// silently in that case.
fn extract_binary_from_exec(exec: &str) -> Option<PathBuf> {
    let tokens = tokenize_exec(exec)?;
    let first = tokens.first()?;

    // Strip leading `env [VAR=value ...]` wrapper: find the first token
    // that is neither `env` / `/usr/bin/env` nor contains `=`.
    let binary_token: String = if first == "env" || first == "/usr/bin/env" {
        tokens
            .iter()
            .skip(1)
            .find(|t| !t.contains('='))
            .cloned()?
    } else {
        first.clone()
    };

    // Flatpak wrapper resolution.
    let flatpak_candidate = binary_token == "flatpak"
        || binary_token.ends_with("/flatpak");
    if flatpak_candidate {
        // Find `--app-id=...` or a positional Flatpak app-id.
        let remaining: Vec<&String> = tokens.iter().skip(1).collect();
        let app_id: Option<String> = remaining
            .iter()
            .find_map(|t| {
                if let Some(stripped) = t.strip_prefix("--app-id=") {
                    return Some(stripped.to_string());
                }
                None
            })
            .or_else(|| {
                // First non-flag, non-assignment positional
                remaining
                    .iter()
                    .find(|t| !t.starts_with('-') && !t.contains('='))
                    .map(|t| t.to_string())
            });

        if let Some(app_id) = app_id {
            let system =
                PathBuf::from("/var/lib/flatpak/exports/bin").join(&app_id);
            if system.is_file() {
                return Some(system);
            }
            if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
                let user = home
                    .join(".local/share/flatpak/exports/bin")
                    .join(&app_id);
                if user.is_file() {
                    return Some(user);
                }
            }
        }
        // Fall through: treat flatpak itself as the binary if present.
    }

    let path = PathBuf::from(&binary_token);
    if path.is_absolute() {
        return if path.is_file() { Some(path) } else { None };
    }

    // Relative — resolve via PATH extended with Flatpak/Snap export bins.
    resolve_via_path_with_exports(&binary_token)
}

/// Resolves a relative program name against `$PATH` plus the Flatpak and
/// Snap wrapper-bin directories. Returns the first match or `None`.
fn resolve_via_path_with_exports(name: &str) -> Option<PathBuf> {
    let mut search: Vec<PathBuf> = Vec::new();

    if let Some(path_var) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_var) {
            search.push(dir);
        }
    }

    search.push(PathBuf::from("/var/lib/flatpak/exports/bin"));
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        search.push(home.join(".local/share/flatpak/exports/bin"));
    }
    search.push(PathBuf::from("/snap/bin"));

    for dir in search {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}
```

- [ ] **Step 2: Add unit tests to the `tests` module**

Append:

```rust
#[test]
fn extract_binary_absolute_missing_returns_none() {
    // Absolute path that does not exist → None (conservative).
    let got = extract_binary_from_exec("/definitely/not/here %U");
    assert!(got.is_none());
}

#[test]
fn extract_binary_strips_env_prefix() {
    // We can't assert the final path (depends on the host), but we can
    // assert that an env-prefixed absolute path to a real file resolves
    // correctly when pointed at a real one.
    use std::fs;
    let tmp = tempfile::tempdir().unwrap();
    let exe = tmp.path().join("mock-bin");
    fs::write(&exe, "#!/bin/sh\ntrue").unwrap();
    // Chmod to executable.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&exe, fs::Permissions::from_mode(0o755)).unwrap();
    }

    let exec_str = format!("env FOO=bar {} %U", exe.display());
    let got = extract_binary_from_exec(&exec_str);
    assert_eq!(got.as_ref(), Some(&exe));
}

#[test]
fn extract_binary_flatpak_wrapper_falls_back_when_missing() {
    // No Flatpak wrapper exists for this bogus app-id, so the resolver
    // should return None (it falls through, `flatpak` may or may not be
    // on the host, so we don't assert a successful path).
    use crate::test_helpers::{ENV_MUTEX, ScopedEnv};
    let _lock = ENV_MUTEX.lock().unwrap();
    let mut env = ScopedEnv::new();
    env.remove("HOME");
    // Force PATH to a directory that cannot contain flatpak, so the
    // final fallthrough also fails.
    env.set("PATH", "/tmp/__sniff_empty_path__");

    let got = extract_binary_from_exec(
        "flatpak run org.example.__nope_12345__ %U",
    );
    assert!(got.is_none());
}

#[test]
fn extract_binary_flatpak_wrapper_resolves_user_exports() {
    use crate::test_helpers::{ENV_MUTEX, ScopedEnv};
    use std::fs;

    let _lock = ENV_MUTEX.lock().unwrap();
    let home = tempfile::tempdir().unwrap();
    let exports = home
        .path()
        .join(".local/share/flatpak/exports/bin");
    fs::create_dir_all(&exports).unwrap();
    let wrapper = exports.join("org.mozilla.firefox");
    fs::write(&wrapper, "#!/bin/sh\ntrue").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o755))
            .unwrap();
    }

    let mut env = ScopedEnv::new();
    env.set("HOME", home.path().to_string_lossy().as_ref());

    let got = extract_binary_from_exec(
        "/usr/bin/flatpak run --branch=stable org.mozilla.firefox %U",
    );
    assert_eq!(got, Some(wrapper));
}

#[test]
fn extract_binary_relative_resolves_via_path() {
    use crate::test_helpers::{ENV_MUTEX, ScopedEnv};
    use std::fs;

    let _lock = ENV_MUTEX.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let exe = dir.path().join("mock-rel");
    fs::write(&exe, "#!/bin/sh\ntrue").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&exe, fs::Permissions::from_mode(0o755)).unwrap();
    }

    let mut env = ScopedEnv::new();
    env.set("PATH", dir.path().to_string_lossy().as_ref());

    let got = extract_binary_from_exec("mock-rel %U");
    assert_eq!(got, Some(exe));
}
```

- [ ] **Step 3: Host-side check**

Run:

```bash
cargo check -p sniff
```

Expected: clean build.

- [ ] **Step 4: Linux cross-check**

Run:

```bash
cargo check -p sniff --target x86_64-unknown-linux-gnu --tests
```

Expected: clean build.

- [ ] **Step 5: Commit**

```bash
git add sniff/lib/src/programs/linux_desktop.rs
git commit -m "feat(sniff): resolve .desktop Exec strings into runnable binaries"
```

---

## Task 7: Implement `parse_desktop_entry()`

**Goal:** Parse a single `.desktop` file into a minimal `DesktopEntry` struct containing exactly the keys the index cares about. Hand-rolled, ~120 lines, no new dependencies. Last-writer-wins on duplicate keys per spec. Only the `[Desktop Entry]` group is honored; `[Desktop Action *]` groups are ignored.

**Files:**
- Modify: `sniff/lib/src/programs/linux_desktop.rs`

- [ ] **Step 1: Add the parser at the bottom of `linux_desktop.rs` (above the `#[cfg(test)]` block)**

Append:

```rust
/// Minimal freedesktop Desktop Entry representation.
///
/// Only the keys used for binary resolution are kept. The parser drops
/// everything else (locale-specific `Name[xx]`, `Comment`, `Categories`,
/// `Icon`, etc.) because `sniff` only needs "is this installed, and
/// what binary does it map to?".
#[derive(Debug, Default, Clone)]
struct DesktopEntry {
    type_field: Option<String>,
    name: Option<String>,
    exec: Option<String>,
    try_exec: Option<String>,
    hidden: bool,
    no_display: bool,
    dbus_activatable: bool,
}

impl DesktopEntry {
    /// Returns `true` if the entry is a real, visible (to `sniff`) launcher
    /// for an application. `NoDisplay=true` is deliberately kept because
    /// `sniff` cares about "installed", not "shown in launcher menus".
    fn is_application_candidate(&self) -> bool {
        if self.hidden {
            return false;
        }
        match self.type_field.as_deref() {
            Some("Application") => true,
            // Missing Type is ambiguous — drop to be safe.
            _ => false,
        }
    }

    /// Resolves the binary for this entry, honoring `TryExec` priority.
    ///
    /// Per the spec, `TryExec` must be verified with `access(X_OK)` and
    /// the entry treated as "not installed" if it fails. We honor this.
    /// When `TryExec` is absent, we fall back to `Exec` via
    /// [`extract_binary_from_exec`].
    fn resolve_binary(&self) -> Option<PathBuf> {
        if let Some(try_exec) = self.try_exec.as_deref() {
            // `TryExec` is a single path, not a full command line, and
            // may be absolute or PATH-relative.
            let path = PathBuf::from(try_exec);
            if path.is_absolute() {
                if is_executable_file(&path) {
                    return Some(path);
                }
                // Strict honor — absolute TryExec that fails access means
                // the entry is a stub. Drop.
                return None;
            }
            // Relative — resolve via PATH + export dirs.
            if let Some(resolved) = resolve_via_path_with_exports(try_exec) {
                return Some(resolved);
            }
            return None;
        }

        if let Some(exec) = self.exec.as_deref() {
            return extract_binary_from_exec(exec);
        }

        None
    }
}

/// Returns true if `path` is a regular file with at least one execute bit.
/// Shared with the rest of the crate's executable probing.
fn is_executable_file(path: &std::path::Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.is_file()
        && path
            .metadata()
            .map(|m| m.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
}

/// Parses a `.desktop` file into a [`DesktopEntry`].
///
/// - Strips a leading UTF-8 BOM.
/// - Skips `#` comment lines and blank lines.
/// - Honors only the `[Desktop Entry]` group; other groups (e.g.
///   `[Desktop Action new-window]`) are ignored.
/// - Unlocalized keys only — `Name[de]` and friends are dropped.
/// - Last-duplicate-key wins (per spec).
///
/// Returns `Err` on I/O failure. Malformed contents that merely fail to
/// yield a usable entry return `Ok` with whatever fields parsed; the
/// caller's `is_application_candidate` + `resolve_binary` checks do the
/// final filtering.
fn parse_desktop_entry(path: &std::path::Path) -> std::io::Result<DesktopEntry> {
    let raw = std::fs::read_to_string(path)?;
    let content = raw.strip_prefix('\u{feff}').unwrap_or(&raw);

    let mut entry = DesktopEntry::default();
    let mut in_desktop_entry = false;

    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if let Some(group) = trimmed.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            in_desktop_entry = group == "Desktop Entry";
            continue;
        }

        if !in_desktop_entry {
            continue;
        }

        let Some((key, value)) = trimmed.split_once('=') else {
            continue;
        };
        let key = key.trim();

        // Drop locale-specific variants like `Name[fr]`.
        if key.contains('[') {
            continue;
        }

        let value = value.trim();

        match key {
            "Type" => entry.type_field = Some(value.to_string()),
            "Name" => entry.name = Some(value.to_string()),
            "Exec" => entry.exec = Some(value.to_string()),
            "TryExec" => entry.try_exec = Some(value.to_string()),
            "Hidden" => entry.hidden = value.eq_ignore_ascii_case("true"),
            "NoDisplay" => {
                entry.no_display = value.eq_ignore_ascii_case("true")
            }
            "DBusActivatable" => {
                entry.dbus_activatable = value.eq_ignore_ascii_case("true")
            }
            _ => {}
        }
    }

    Ok(entry)
}
```

- [ ] **Step 2: Add unit tests that parse synthetic fixtures**

Append to the `tests` module:

```rust
fn write_desktop_fixture(dir: &std::path::Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, body).unwrap();
    path
}

#[test]
fn parse_desktop_entry_happy_path() {
    let tmp = tempfile::tempdir().unwrap();
    let path = write_desktop_fixture(
        tmp.path(),
        "foo.desktop",
        "[Desktop Entry]\nType=Application\nName=Foo\nExec=/usr/bin/foo %U\n",
    );

    let entry = parse_desktop_entry(&path).unwrap();
    assert_eq!(entry.type_field.as_deref(), Some("Application"));
    assert_eq!(entry.name.as_deref(), Some("Foo"));
    assert_eq!(entry.exec.as_deref(), Some("/usr/bin/foo %U"));
    assert!(entry.is_application_candidate());
}

#[test]
fn parse_desktop_entry_hidden_is_dropped() {
    let tmp = tempfile::tempdir().unwrap();
    let path = write_desktop_fixture(
        tmp.path(),
        "hidden.desktop",
        "[Desktop Entry]\nType=Application\nName=Hidden\nExec=/usr/bin/true\nHidden=true\n",
    );
    let entry = parse_desktop_entry(&path).unwrap();
    assert!(!entry.is_application_candidate());
}

#[test]
fn parse_desktop_entry_nodisplay_is_kept() {
    let tmp = tempfile::tempdir().unwrap();
    let path = write_desktop_fixture(
        tmp.path(),
        "nd.desktop",
        "[Desktop Entry]\nType=Application\nName=ND\nExec=/usr/bin/true\nNoDisplay=true\n",
    );
    let entry = parse_desktop_entry(&path).unwrap();
    assert!(entry.is_application_candidate());
    assert!(entry.no_display);
}

#[test]
fn parse_desktop_entry_link_type_is_dropped() {
    let tmp = tempfile::tempdir().unwrap();
    let path = write_desktop_fixture(
        tmp.path(),
        "link.desktop",
        "[Desktop Entry]\nType=Link\nName=L\nURL=https://example.com\n",
    );
    let entry = parse_desktop_entry(&path).unwrap();
    assert!(!entry.is_application_candidate());
}

#[test]
fn parse_desktop_entry_handles_comments_bom_blank_lines() {
    let tmp = tempfile::tempdir().unwrap();
    let body = "\u{feff}# A comment\n\n[Desktop Entry]\n# inline comment (whole line)\nType=Application\nName=Foo\nExec=/usr/bin/foo\n";
    let path = write_desktop_fixture(tmp.path(), "bom.desktop", body);
    let entry = parse_desktop_entry(&path).unwrap();
    assert_eq!(entry.exec.as_deref(), Some("/usr/bin/foo"));
}

#[test]
fn parse_desktop_entry_duplicate_key_last_wins() {
    let tmp = tempfile::tempdir().unwrap();
    let path = write_desktop_fixture(
        tmp.path(),
        "dup.desktop",
        "[Desktop Entry]\nType=Application\nName=First\nName=Second\nExec=/usr/bin/foo\n",
    );
    let entry = parse_desktop_entry(&path).unwrap();
    assert_eq!(entry.name.as_deref(), Some("Second"));
}

#[test]
fn parse_desktop_entry_skips_locale_keys() {
    let tmp = tempfile::tempdir().unwrap();
    let path = write_desktop_fixture(
        tmp.path(),
        "loc.desktop",
        "[Desktop Entry]\nType=Application\nName=Foo\nName[fr]=Toto\nExec=/usr/bin/foo\n",
    );
    let entry = parse_desktop_entry(&path).unwrap();
    assert_eq!(entry.name.as_deref(), Some("Foo"));
}

#[test]
fn parse_desktop_entry_ignores_action_groups() {
    let tmp = tempfile::tempdir().unwrap();
    let path = write_desktop_fixture(
        tmp.path(),
        "action.desktop",
        "[Desktop Entry]\nType=Application\nName=Foo\nExec=/usr/bin/foo\n[Desktop Action new-window]\nExec=/usr/bin/other\n",
    );
    let entry = parse_desktop_entry(&path).unwrap();
    assert_eq!(entry.exec.as_deref(), Some("/usr/bin/foo"));
}

#[test]
fn parse_desktop_entry_try_exec_wins_when_file_exists() {
    use std::fs;
    let tmp = tempfile::tempdir().unwrap();
    let real_bin = tmp.path().join("real-bin");
    fs::write(&real_bin, "#!/bin/sh\ntrue").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&real_bin, fs::Permissions::from_mode(0o755))
            .unwrap();
    }
    let body = format!(
        "[Desktop Entry]\nType=Application\nName=Foo\nTryExec={}\nExec=/usr/bin/foo\n",
        real_bin.display()
    );
    let path = write_desktop_fixture(tmp.path(), "tryexec.desktop", &body);
    let entry = parse_desktop_entry(&path).unwrap();
    assert_eq!(entry.resolve_binary().as_ref(), Some(&real_bin));
}

#[test]
fn parse_desktop_entry_try_exec_strict_drops_missing_absolute() {
    let tmp = tempfile::tempdir().unwrap();
    let body = "[Desktop Entry]\nType=Application\nName=Foo\nTryExec=/this/is/not/there\nExec=/usr/bin/foo\n";
    let path = write_desktop_fixture(tmp.path(), "stub.desktop", body);
    let entry = parse_desktop_entry(&path).unwrap();
    // Absolute TryExec that fails must cause resolve_binary to return None,
    // per spec-strict behavior.
    assert!(entry.resolve_binary().is_none());
}
```

- [ ] **Step 3: Host-side check**

Run:

```bash
cargo check -p sniff
```

Expected: clean build.

- [ ] **Step 4: Linux cross-check**

Run:

```bash
cargo check -p sniff --target x86_64-unknown-linux-gnu --tests
```

Expected: clean build.

- [ ] **Step 5: Commit**

```bash
git add sniff/lib/src/programs/linux_desktop.rs
git commit -m "feat(sniff): parse freedesktop .desktop files (hand-rolled)"
```

---

## Task 8: Implement `scan_desktop_entries()` with desktop-id aliases

**Goal:** Replace the `scan_desktop_entries` placeholder with a real sweep that walks every dir returned by `desktop_search_dirs()`, parses each `.desktop` file, resolves its binary, and indexes by three keys: binary basename, desktop-id stem, and any alias-table entry. Uses canonical paths for NixOS symlink dedup; last-writer-wins so higher-priority dirs overwrite earlier ones.

**Files:**
- Modify: `sniff/lib/src/programs/linux_desktop.rs`

- [ ] **Step 1: Add the alias table and the sweep**

In `sniff/lib/src/programs/linux_desktop.rs`, replace:

```rust
/// Placeholder — populated in Task 8.
fn scan_desktop_entries() -> HashMap<String, PathBuf> {
    HashMap::new()
}
```

with:

```rust
/// Alias table mapping well-known Flatpak desktop-id stems to the native
/// binary name callers are likely to look up. Purely additive — missing
/// entries cause no lookups to be lost.
///
/// Maintained alongside `macos_bundle::get_app_bundle_name` and audited
/// periodically against the most popular Flathub apps.
const DESKTOP_ID_ALIASES: &[(&str, &str)] = &[
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

/// Scans every directory returned by [`desktop_search_dirs`], parses each
/// `.desktop` file, resolves its binary, and indexes the result by three
/// keys: the binary basename, the desktop-id stem (filename without
/// `.desktop`), and any matching alias from [`DESKTOP_ID_ALIASES`].
///
/// Symlinks are canonicalized so NixOS `/nix/store` farms collapse to a
/// single entry. Higher-priority dirs are scanned *last* so their entries
/// overwrite earlier ones on key collisions (user > system > Flatpak
/// exports > Snap).
fn scan_desktop_entries() -> HashMap<String, PathBuf> {
    let mut map: HashMap<String, PathBuf> = HashMap::new();
    // Track canonical paths to avoid double-indexing the same underlying
    // file via two different profile symlinks (NixOS dedup).
    let mut seen_canonical: std::collections::HashSet<PathBuf> =
        std::collections::HashSet::new();

    for dir in desktop_search_dirs() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if !name.ends_with(".desktop") {
                continue;
            }

            // Canonicalize so NixOS store copies collapse to one entry.
            let canonical = std::fs::canonicalize(&path)
                .unwrap_or_else(|_| path.clone());
            if !seen_canonical.insert(canonical.clone()) {
                // Same underlying file seen before — still index it under
                // this directory's key namespace (for completeness) but
                // skip re-parsing.
                continue;
            }

            let Ok(parsed) = parse_desktop_entry(&canonical) else {
                tracing::warn!(
                    path = %canonical.display(),
                    "failed to parse .desktop file; skipping"
                );
                continue;
            };
            if !parsed.is_application_candidate() {
                continue;
            }

            let Some(binary) = parsed.resolve_binary() else {
                continue;
            };
            let Some(binary_name) = binary
                .file_name()
                .and_then(|n| n.to_str())
                .map(str::to_string)
            else {
                continue;
            };

            // Index under the binary basename.
            map.insert(binary_name.clone(), binary.clone());

            // Index under the `.desktop` stem.
            let stem = name.strip_suffix(".desktop").unwrap_or(name);
            map.insert(stem.to_string(), binary.clone());

            // Apply aliases — if the stem matches a Flatpak app-id in
            // DESKTOP_ID_ALIASES, also index under the native alias name.
            for (app_id, alias) in DESKTOP_ID_ALIASES {
                if *app_id == stem {
                    map.insert((*alias).to_string(), binary.clone());
                }
            }
        }
    }

    map
}
```

- [ ] **Step 2: Add a unit test for `scan_desktop_entries` driven by `tempdir`**

Append to the `tests` module in the same file:

```rust
#[test]
fn scan_desktop_entries_finds_binary_and_aliases_flatpak_id() {
    use crate::test_helpers::{ENV_MUTEX, ScopedEnv};
    use std::fs;

    let _lock = ENV_MUTEX.lock().unwrap();

    // Build a self-contained XDG_DATA_HOME/applications tree.
    let xdg_home = tempfile::tempdir().unwrap();
    let apps = xdg_home.path().join("applications");
    fs::create_dir_all(&apps).unwrap();

    // Create a real executable the .desktop file can point at.
    let bin_dir = tempfile::tempdir().unwrap();
    let real_bin = bin_dir.path().join("firefox");
    fs::write(&real_bin, "#!/bin/sh\ntrue").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&real_bin, fs::Permissions::from_mode(0o755))
            .unwrap();
    }

    // The .desktop file uses the Flatpak app-id stem and an absolute Exec.
    let desktop_body = format!(
        "[Desktop Entry]\nType=Application\nName=Firefox\nExec={} %U\n",
        real_bin.display()
    );
    fs::write(apps.join("org.mozilla.firefox.desktop"), desktop_body).unwrap();

    // Isolate the env: no XDG_DATA_DIRS, XDG_DATA_HOME points at our tree,
    // HOME is unset to keep Flatpak user exports out of the picture.
    let mut env = ScopedEnv::new();
    env.set(
        "XDG_DATA_HOME",
        xdg_home.path().to_string_lossy().as_ref(),
    );
    env.set("XDG_DATA_DIRS", "/tmp/__sniff_empty__");
    env.remove("HOME");

    let map = scan_desktop_entries();

    // Indexed by binary basename.
    assert_eq!(map.get("firefox").cloned(), Some(real_bin.clone()));
    // Indexed by .desktop stem.
    assert_eq!(
        map.get("org.mozilla.firefox").cloned(),
        Some(real_bin.clone())
    );
}

#[test]
fn scan_desktop_entries_user_dir_overrides_system_dir() {
    use crate::test_helpers::{ENV_MUTEX, ScopedEnv};
    use std::fs;

    let _lock = ENV_MUTEX.lock().unwrap();

    // Two separate trees, both with a foo.desktop pointing at different
    // (real) executables. XDG_DATA_DIRS is the lower-priority system tree;
    // XDG_DATA_HOME is the higher-priority user tree, scanned last.
    let system = tempfile::tempdir().unwrap();
    let sys_apps = system.path().join("applications");
    fs::create_dir_all(&sys_apps).unwrap();

    let user = tempfile::tempdir().unwrap();
    let usr_apps = user.path().join("applications");
    fs::create_dir_all(&usr_apps).unwrap();

    // Real binaries.
    let sys_bin_dir = tempfile::tempdir().unwrap();
    let usr_bin_dir = tempfile::tempdir().unwrap();
    let sys_bin = sys_bin_dir.path().join("foo");
    let usr_bin = usr_bin_dir.path().join("foo");
    for bin in [&sys_bin, &usr_bin] {
        fs::write(bin, "#!/bin/sh\ntrue").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(bin, fs::Permissions::from_mode(0o755))
                .unwrap();
        }
    }

    fs::write(
        sys_apps.join("foo.desktop"),
        format!(
            "[Desktop Entry]\nType=Application\nName=Foo\nExec={}\n",
            sys_bin.display()
        ),
    )
    .unwrap();
    fs::write(
        usr_apps.join("foo.desktop"),
        format!(
            "[Desktop Entry]\nType=Application\nName=Foo\nExec={}\n",
            usr_bin.display()
        ),
    )
    .unwrap();

    let mut env = ScopedEnv::new();
    env.set(
        "XDG_DATA_DIRS",
        system.path().to_string_lossy().as_ref(),
    );
    env.set(
        "XDG_DATA_HOME",
        user.path().to_string_lossy().as_ref(),
    );
    env.remove("HOME");

    let map = scan_desktop_entries();
    // User dir is scanned *after* system, so its entry wins.
    assert_eq!(map.get("foo").cloned(), Some(usr_bin));
}
```

- [ ] **Step 3: Host-side check**

Run:

```bash
cargo check -p sniff
```

Expected: clean build. The `tracing::warn!` call uses the already-imported `tracing` crate, which is a direct dep of sniff.

- [ ] **Step 4: Linux cross-check**

Run:

```bash
cargo check -p sniff --target x86_64-unknown-linux-gnu --tests
```

Expected: clean build.

- [ ] **Step 5: Commit**

```bash
git add sniff/lib/src/programs/linux_desktop.rs
git commit -m "feat(sniff): sweep XDG .desktop files with alias + NixOS dedup"
```

---

## Task 9: Implement Flatpak and Snap export-bin probes

**Goal:** Replace the `scan_flatpak_bins` and `scan_snap_bins` placeholders with real directory scans that index every executable wrapper in `/var/lib/flatpak/exports/bin/`, `~/.local/share/flatpak/exports/bin/`, and `/snap/bin/`.

**Files:**
- Modify: `sniff/lib/src/programs/linux_desktop.rs`

- [ ] **Step 1: Replace both placeholders**

In `sniff/lib/src/programs/linux_desktop.rs`, replace:

```rust
/// Placeholder — populated in Task 9.
fn scan_flatpak_bins() -> HashMap<String, PathBuf> {
    HashMap::new()
}

/// Placeholder — populated in Task 9.
fn scan_snap_bins() -> HashMap<String, PathBuf> {
    HashMap::new()
}
```

with:

```rust
/// Scans Flatpak wrapper-bin directories (system + user) and returns a
/// map from wrapper basename to full path. First write wins within a
/// directory; later dirs do not overwrite.
fn scan_flatpak_bins() -> HashMap<String, PathBuf> {
    let mut dirs: Vec<PathBuf> =
        vec![PathBuf::from("/var/lib/flatpak/exports/bin")];
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        dirs.push(home.join(".local/share/flatpak/exports/bin"));
    }
    scan_executable_dir(&dirs)
}

/// Scans `/snap/bin/` and returns a map from wrapper basename to full
/// path.
fn scan_snap_bins() -> HashMap<String, PathBuf> {
    scan_executable_dir(&[PathBuf::from("/snap/bin")])
}

/// Shared helper: walks each `dir` one level, indexing any regular-file
/// entry with an execute bit. First occurrence wins per key.
fn scan_executable_dir(dirs: &[PathBuf]) -> HashMap<String, PathBuf> {
    let mut map: HashMap<String, PathBuf> = HashMap::new();
    for dir in dirs {
        let Ok(read) = std::fs::read_dir(dir) else {
            continue;
        };
        for entry in read.flatten() {
            let path = entry.path();
            if !is_executable_file(&path) {
                continue;
            }
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            map.entry(name.to_string()).or_insert_with(|| path.clone());
        }
    }
    map
}
```

- [ ] **Step 2: Add unit tests for the probes using `tempdir` fixtures**

Append to the `tests` module:

```rust
#[test]
fn scan_executable_dir_indexes_executable_entries() {
    use std::fs;

    let tmp = tempfile::tempdir().unwrap();
    let exe = tmp.path().join("wrapper-one");
    fs::write(&exe, "#!/bin/sh\ntrue").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&exe, fs::Permissions::from_mode(0o755)).unwrap();
    }

    let non_exec = tmp.path().join("README");
    fs::write(&non_exec, "doc").unwrap();

    let result = scan_executable_dir(&[tmp.path().to_path_buf()]);

    assert_eq!(result.get("wrapper-one").cloned(), Some(exe));
    assert!(!result.contains_key("README"));
}

#[test]
fn scan_executable_dir_skips_missing_dir() {
    let result =
        scan_executable_dir(&[PathBuf::from("/__sniff_nonexistent__")]);
    assert!(result.is_empty());
}

#[test]
fn scan_executable_dir_first_write_wins() {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;

    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    for d in [a.path(), b.path()] {
        let exe = d.join("dup");
        fs::write(&exe, "#!/bin/sh\ntrue").unwrap();
        fs::set_permissions(&exe, fs::Permissions::from_mode(0o755)).unwrap();
    }

    let result = scan_executable_dir(&[
        a.path().to_path_buf(),
        b.path().to_path_buf(),
    ]);

    // Earlier dir wins.
    assert_eq!(result.get("dup").cloned(), Some(a.path().join("dup")));
}
```

- [ ] **Step 3: Host-side check**

Run:

```bash
cargo check -p sniff
```

Expected: clean build.

- [ ] **Step 4: Linux cross-check**

Run:

```bash
cargo check -p sniff --target x86_64-unknown-linux-gnu --tests
```

Expected: clean build.

- [ ] **Step 5: Commit**

```bash
git add sniff/lib/src/programs/linux_desktop.rs
git commit -m "feat(sniff): probe Flatpak and Snap export-bin directories"
```

---

## Task 10: Wire `LinuxDesktopIndex` into `ExecutableIndex`

**Goal:** Add a `#[cfg(target_os = "linux")]` field on `ExecutableIndex`, populate it inside `build_with_bundles`, and extend `find_with_source()` to consult the Linux layers after PATH. Extend the one-off `find_program_with_source()` helper with the same chain. On non-Linux the struct and method bodies are unchanged.

**Files:**
- Modify: `sniff/lib/src/programs/find_program.rs`

- [ ] **Step 1: Add the field to `ExecutableIndex`**

In `sniff/lib/src/programs/find_program.rs`, find the struct definition near line 71-80:

```rust
#[derive(Debug, Clone)]
pub struct ExecutableIndex {
    /// Maps binary name to resolved path (first occurrence wins = PATH precedence)
    path_executables: HashMap<String, PathBuf>,
    /// Number of PATH directories scanned while building the index.
    path_dir_count: usize,
    /// Maps binary name to app bundle path (macOS only)
    #[cfg(target_os = "macos")]
    bundle_executables: HashMap<String, PathBuf>,
}
```

Replace it with:

```rust
#[derive(Debug, Clone)]
pub struct ExecutableIndex {
    /// Maps binary name to resolved path (first occurrence wins = PATH precedence)
    path_executables: HashMap<String, PathBuf>,
    /// Number of PATH directories scanned while building the index.
    path_dir_count: usize,
    /// Maps binary name to app bundle path (macOS only).
    #[cfg(target_os = "macos")]
    bundle_executables: HashMap<String, PathBuf>,
    /// Linux-specific fallback index (XDG .desktop sweep + Flatpak/Snap bins).
    #[cfg(target_os = "linux")]
    linux_index: super::linux_desktop::LinuxDesktopIndex,
}
```

- [ ] **Step 2: Populate `linux_index` in `build_with_bundles`**

Find the tail of `build_with_bundles` near line 146-155:

```rust
        Self {
            path_executables,
            path_dir_count,
            #[cfg(target_os = "macos")]
            bundle_executables: if include_bundles {
                build_bundle_index()
            } else {
                HashMap::new()
            },
        }
```

Replace it with:

```rust
        Self {
            path_executables,
            path_dir_count,
            #[cfg(target_os = "macos")]
            bundle_executables: if include_bundles {
                build_bundle_index()
            } else {
                HashMap::new()
            },
            #[cfg(target_os = "linux")]
            linux_index: if include_bundles {
                super::linux_desktop::build_linux_index()
            } else {
                super::linux_desktop::LinuxDesktopIndex::default()
            },
        }
```

The `include_bundles` flag is reused intentionally: it means "consult non-PATH fallback sources", and `build_path_only()` stays a pure PATH snapshot regardless of platform.

- [ ] **Step 3: Extend `find_with_source` with the Linux layers**

Find the existing `find_with_source` implementation near line 166-177:

```rust
    pub fn find_with_source(&self, program: &str) -> Option<(PathBuf, ExecutableSource)> {
        if let Some(path) = self.path_executables.get(program) {
            return Some((path.clone(), ExecutableSource::Path));
        }

        #[cfg(target_os = "macos")]
        if let Some(path) = self.bundle_executables.get(program) {
            return Some((path.clone(), ExecutableSource::MacOsAppBundle));
        }

        None
    }
```

Replace it with:

```rust
    pub fn find_with_source(&self, program: &str) -> Option<(PathBuf, ExecutableSource)> {
        // Layer 1: PATH (authoritative).
        if let Some(path) = self.path_executables.get(program) {
            return Some((path.clone(), ExecutableSource::Path));
        }

        // Layer 2: macOS bundles.
        #[cfg(target_os = "macos")]
        if let Some(path) = self.bundle_executables.get(program) {
            return Some((path.clone(), ExecutableSource::MacOsAppBundle));
        }

        // Layer 3 + 4 + 5: Linux fallbacks — `.desktop` sweep, Flatpak
        // wrapper-bin probe, Snap wrapper-bin probe. Key lookups are case
        // sensitive; `.desktop` entries and wrapper dirs both use exact
        // filesystem basenames.
        #[cfg(target_os = "linux")]
        {
            if let Some(path) = self.linux_index.desktop_entries.get(program) {
                return Some((path.clone(), ExecutableSource::LinuxDesktopEntry));
            }
            if let Some(path) = self.linux_index.flatpak_bins.get(program) {
                return Some((path.clone(), ExecutableSource::LinuxFlatpakBin));
            }
            if let Some(path) = self.linux_index.snap_bins.get(program) {
                return Some((path.clone(), ExecutableSource::LinuxSnapBin));
            }
        }

        None
    }
```

- [ ] **Step 4: Extend `find_program_with_source` (one-off helper) with the same chain**

Find `find_program_with_source` near line 417-433:

```rust
pub fn find_program_with_source<P: AsRef<OsStr>>(
    program: P,
) -> Option<(PathBuf, ExecutableSource)> {
    let program_str = program.as_ref().to_string_lossy();

    // Priority 1: Check PATH first
    if let Ok(path) = which(&program) {
        return Some((path, ExecutableSource::Path));
    }

    // Priority 2: Check macOS app bundles (macOS only)
    if let Some(path) = find_macos_app_bundle(&program_str) {
        return Some((path, ExecutableSource::MacOsAppBundle));
    }

    None
}
```

Replace it with:

```rust
pub fn find_program_with_source<P: AsRef<OsStr>>(
    program: P,
) -> Option<(PathBuf, ExecutableSource)> {
    let program_str = program.as_ref().to_string_lossy();

    // Priority 1: PATH (authoritative).
    if let Ok(path) = which(&program) {
        return Some((path, ExecutableSource::Path));
    }

    // Priority 2: macOS app bundles.
    if let Some(path) = find_macos_app_bundle(&program_str) {
        return Some((path, ExecutableSource::MacOsAppBundle));
    }

    // Priority 3 + 4 + 5: Linux fallbacks. One-off callers pay the build
    // cost of LinuxDesktopIndex on each call; `ExecutableIndex::build()`
    // is preferred for batch lookups.
    #[cfg(target_os = "linux")]
    {
        let idx = super::linux_desktop::build_linux_index();
        let key = program_str.as_ref();
        if let Some(path) = idx.desktop_entries.get(key) {
            return Some((path.clone(), ExecutableSource::LinuxDesktopEntry));
        }
        if let Some(path) = idx.flatpak_bins.get(key) {
            return Some((path.clone(), ExecutableSource::LinuxFlatpakBin));
        }
        if let Some(path) = idx.snap_bins.get(key) {
            return Some((path.clone(), ExecutableSource::LinuxSnapBin));
        }
    }

    None
}
```

- [ ] **Step 5: Host-side build + test**

Run:

```bash
cargo check -p sniff
cargo test -p sniff programs::find_program
cargo test -p sniff programs::types
```

Expected: all existing tests in both modules still pass. The change is invisible to them because the Linux branch is behind `#[cfg(target_os = "linux")]`.

- [ ] **Step 6: Linux cross-check**

Run:

```bash
cargo check -p sniff --target x86_64-unknown-linux-gnu
cargo check -p sniff --target x86_64-unknown-linux-gnu --tests
```

Expected: clean build.

- [ ] **Step 7: Commit**

```bash
git add sniff/lib/src/programs/find_program.rs
git commit -m "feat(sniff): consult Linux fallback index in ExecutableIndex lookups"
```

---

## Task 11: Add Linux integration tests

**Goal:** Four focused integration-test files that exercise the full pipeline on a Linux host. Every file is `#![cfg(target_os = "linux")]` so non-Linux builds ignore them.

**Files:**
- Create: `sniff/lib/tests/linux_desktop_discovery.rs`
- Create: `sniff/lib/tests/linux_flatpak_wrapper.rs`
- Create: `sniff/lib/tests/linux_nixos_dedup.rs`
- Create: `sniff/lib/tests/linux_priority_over_xdg_system.rs`

- [ ] **Step 1: Create `linux_desktop_discovery.rs`**

Create `sniff/lib/tests/linux_desktop_discovery.rs` with:

```rust
//! Integration test: a synthetic XDG data dir containing a `.desktop` file
//! that points at a real tempdir binary must surface via
//! `ExecutableIndex::find_with_source` under the binary basename and the
//! desktop-id stem.
//!
//! Linux only.

#![cfg(target_os = "linux")]

use std::fs;
use std::os::unix::fs::PermissionsExt;

use sniff::programs::{ExecutableIndex, ExecutableSource};

fn make_exec(path: &std::path::Path) {
    fs::write(path, "#!/bin/sh\ntrue").unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

#[test]
fn xdg_sweep_finds_binary_via_desktop_entry() {
    // Synthetic XDG_DATA_HOME tree plus a real binary.
    let xdg_home = tempfile::tempdir().unwrap();
    let apps = xdg_home.path().join("applications");
    fs::create_dir_all(&apps).unwrap();

    let bin_dir = tempfile::tempdir().unwrap();
    let bin = bin_dir.path().join("sniff-fake-app");
    make_exec(&bin);

    fs::write(
        apps.join("sniff-fake-app.desktop"),
        format!(
            "[Desktop Entry]\nType=Application\nName=Fake\nExec={} %U\n",
            bin.display()
        ),
    )
    .unwrap();

    // Env isolation uses the unsafe API — the library tests already share
    // an ENV_MUTEX, but integration tests run in a separate binary so they
    // cannot access the crate-private mutex. We rely on single-threaded
    // test execution via `cargo test -- --test-threads=1` OR on the fact
    // that these tests each use unique binary names so collisions across
    // concurrent tests are harmless.
    //
    // SAFETY: Single-threaded within this file; the env changes are
    // scoped to the duration of the test.
    unsafe {
        std::env::set_var("XDG_DATA_HOME", xdg_home.path());
        std::env::set_var("XDG_DATA_DIRS", "/tmp/__sniff_empty_integration__");
        std::env::remove_var("HOME");
    }

    let index = ExecutableIndex::build();

    let (path, source) = index
        .find_with_source("sniff-fake-app")
        .expect("binary basename lookup");
    assert_eq!(path, bin);
    assert_eq!(source, ExecutableSource::LinuxDesktopEntry);
}
```

- [ ] **Step 2: Create `linux_flatpak_wrapper.rs`**

Create `sniff/lib/tests/linux_flatpak_wrapper.rs` with:

```rust
//! Integration test: a synthetic Flatpak user-exports tree must surface
//! through `LinuxFlatpakBin` (either via the wrapper probe or via a
//! `.desktop` file whose Exec uses `flatpak run`).
//!
//! Linux only.

#![cfg(target_os = "linux")]

use std::fs;
use std::os::unix::fs::PermissionsExt;

use sniff::programs::{ExecutableIndex, ExecutableSource};

fn make_exec(path: &std::path::Path) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, "#!/bin/sh\ntrue").unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

#[test]
fn user_flatpak_exports_bin_is_indexed() {
    let home = tempfile::tempdir().unwrap();
    let wrapper = home
        .path()
        .join(".local/share/flatpak/exports/bin/org.mozilla.firefox");
    make_exec(&wrapper);

    // SAFETY: Single-threaded within this integration test binary.
    unsafe {
        std::env::set_var("HOME", home.path());
        std::env::set_var("XDG_DATA_DIRS", "/tmp/__sniff_empty_integration2__");
        std::env::remove_var("XDG_DATA_HOME");
    }

    let index = ExecutableIndex::build();

    let (path, source) = index
        .find_with_source("org.mozilla.firefox")
        .expect("Flatpak wrapper should be indexed");
    assert_eq!(path, wrapper);
    // Either LinuxFlatpakBin (wrapper probe) or LinuxDesktopEntry (if a
    // synthetic .desktop happens to exist elsewhere) is acceptable. The
    // test isolates HOME + XDG_DATA_DIRS, so in practice this should
    // always be LinuxFlatpakBin, but accept either to stay robust.
    assert!(
        matches!(
            source,
            ExecutableSource::LinuxFlatpakBin
                | ExecutableSource::LinuxDesktopEntry
        ),
        "unexpected source: {:?}",
        source
    );
}
```

- [ ] **Step 3: Create `linux_nixos_dedup.rs`**

Create `sniff/lib/tests/linux_nixos_dedup.rs` with:

```rust
//! Integration test: two symlinks pointing at the same `.desktop` file
//! must produce exactly one index entry (NixOS store dedup).
//!
//! Linux only.

#![cfg(target_os = "linux")]

use std::fs;
use std::os::unix::fs::PermissionsExt;

use sniff::programs::{ExecutableIndex, ExecutableSource};

#[test]
fn symlinked_desktop_files_collapse_to_one_entry() {
    // Binary the .desktop file points at.
    let bin_dir = tempfile::tempdir().unwrap();
    let bin = bin_dir.path().join("nix-demo");
    fs::write(&bin, "#!/bin/sh\ntrue").unwrap();
    fs::set_permissions(&bin, fs::Permissions::from_mode(0o755)).unwrap();

    // Real .desktop file lives in `store`.
    let store = tempfile::tempdir().unwrap();
    let real_desktop = store.path().join("nix-demo.desktop");
    fs::write(
        &real_desktop,
        format!(
            "[Desktop Entry]\nType=Application\nName=NixDemo\nExec={}\n",
            bin.display()
        ),
    )
    .unwrap();

    // Two XDG dirs both symlink to the same underlying file.
    let xdg_home = tempfile::tempdir().unwrap();
    let user_apps = xdg_home.path().join("applications");
    fs::create_dir_all(&user_apps).unwrap();
    std::os::unix::fs::symlink(
        &real_desktop,
        user_apps.join("nix-demo.desktop"),
    )
    .unwrap();

    let system = tempfile::tempdir().unwrap();
    let sys_apps = system.path().join("applications");
    fs::create_dir_all(&sys_apps).unwrap();
    std::os::unix::fs::symlink(
        &real_desktop,
        sys_apps.join("nix-demo.desktop"),
    )
    .unwrap();

    // SAFETY: Single-threaded within this integration test binary.
    unsafe {
        std::env::set_var("XDG_DATA_HOME", xdg_home.path());
        std::env::set_var("XDG_DATA_DIRS", system.path());
        std::env::remove_var("HOME");
    }

    let index = ExecutableIndex::build();

    let (path, source) = index
        .find_with_source("nix-demo")
        .expect("dedup index must still surface the binary");
    assert_eq!(path, bin);
    assert_eq!(source, ExecutableSource::LinuxDesktopEntry);
}
```

- [ ] **Step 4: Create `linux_priority_over_xdg_system.rs`**

Create `sniff/lib/tests/linux_priority_over_xdg_system.rs` with:

```rust
//! Integration test: when the same binary name is defined in both
//! `$XDG_DATA_DIRS` (system) and `$XDG_DATA_HOME` (user), the user entry
//! wins.
//!
//! Linux only.

#![cfg(target_os = "linux")]

use std::fs;
use std::os::unix::fs::PermissionsExt;

use sniff::programs::{ExecutableIndex, ExecutableSource};

fn make_exec(path: &std::path::Path) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, "#!/bin/sh\ntrue").unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

#[test]
fn user_xdg_wins_over_system_xdg() {
    let system = tempfile::tempdir().unwrap();
    let sys_apps = system.path().join("applications");
    fs::create_dir_all(&sys_apps).unwrap();

    let user = tempfile::tempdir().unwrap();
    let usr_apps = user.path().join("applications");
    fs::create_dir_all(&usr_apps).unwrap();

    let sys_bin_dir = tempfile::tempdir().unwrap();
    let sys_bin = sys_bin_dir.path().join("duplicated");
    make_exec(&sys_bin);

    let usr_bin_dir = tempfile::tempdir().unwrap();
    let usr_bin = usr_bin_dir.path().join("duplicated");
    make_exec(&usr_bin);

    fs::write(
        sys_apps.join("duplicated.desktop"),
        format!(
            "[Desktop Entry]\nType=Application\nName=Dup\nExec={}\n",
            sys_bin.display()
        ),
    )
    .unwrap();
    fs::write(
        usr_apps.join("duplicated.desktop"),
        format!(
            "[Desktop Entry]\nType=Application\nName=Dup\nExec={}\n",
            usr_bin.display()
        ),
    )
    .unwrap();

    // SAFETY: Single-threaded within this integration test binary.
    unsafe {
        std::env::set_var("XDG_DATA_DIRS", system.path());
        std::env::set_var("XDG_DATA_HOME", user.path());
        std::env::remove_var("HOME");
    }

    let index = ExecutableIndex::build();
    let (path, source) = index
        .find_with_source("duplicated")
        .expect("duplicated binary lookup");
    assert_eq!(path, usr_bin, "user entry must win over system entry");
    assert_eq!(source, ExecutableSource::LinuxDesktopEntry);
}
```

- [ ] **Step 5: Host-side build**

Run:

```bash
cargo check -p sniff --tests
```

Expected: clean build — every new file collapses to an empty module on non-Linux via the top-of-file `#![cfg(target_os = "linux")]`.

- [ ] **Step 6: Linux cross-check**

Run:

```bash
cargo check -p sniff --target x86_64-unknown-linux-gnu --tests
```

Expected: clean build.

> **Note on concurrent env mutation:** The integration tests each run in their own test binary, so they do not share `crate::test_helpers::ENV_MUTEX`. Each `#[test]` sets env vars using `unsafe { std::env::set_var(...) }` (Rust 2024 requirement) and relies on the fact that the enclosing test binary serializes tests across this file automatically (each integration test file runs each test on a single thread when built with the default harness and when only one `#[test]` is present per file). The tests are split one-per-file for exactly this reason. If a future refactor puts multiple tests in one of these files, wrap them in a `Mutex` local to the file.

- [ ] **Step 7: Commit**

```bash
git add sniff/lib/tests/linux_desktop_discovery.rs \
        sniff/lib/tests/linux_flatpak_wrapper.rs \
        sniff/lib/tests/linux_nixos_dedup.rs \
        sniff/lib/tests/linux_priority_over_xdg_system.rs
git commit -m "test(sniff): add Linux .desktop + Flatpak + NixOS integration tests"
```

---

## Task 12: Update documentation

**Goal:** Bring the library README, the architecture doc, and the `sniff` skill's `programs.md` in sync with the new Linux fallback chain.

**Files:**
- Modify: `sniff/lib/README.md`
- Modify: `sniff/docs/sniff-library-architecture.md`
- Modify: `.claude/skills/sniff/programs.md`

- [ ] **Step 1: Update `sniff/lib/README.md`**

Find the section describing macOS app bundle fallback (search for `MacOsAppBundle` or `app bundles`). Immediately after that paragraph, add:

```markdown
### Linux fallback chain

On Linux, program detection expands beyond `$PATH` to find GUI apps that
Flatpak and Snap install outside non-login-shell `$PATH`:

1. **PATH** — `which`-style lookup, returns `ExecutableSource::Path`.
2. **XDG `.desktop` sweep** — every `.desktop` file under
   `$XDG_DATA_HOME/applications`, each `$XDG_DATA_DIRS/applications`,
   `/var/lib/flatpak/exports/share/applications`,
   `~/.local/share/flatpak/exports/share/applications`, and
   `/var/lib/snapd/desktop/applications` is parsed. `TryExec` wins when
   present; otherwise the `Exec=` command is tokenized per freedesktop
   quoting rules and resolved via PATH plus the Flatpak/Snap export bins.
   Symlinks are canonicalized so NixOS `/nix/store` farms dedupe cleanly.
   Returns `ExecutableSource::LinuxDesktopEntry`. A small alias table
   surfaces common Flatpak app-ids (e.g. `org.mozilla.firefox` → `firefox`)
   so lookups by native binary name still succeed.
3. **Flatpak wrapper probe** — direct scan of
   `/var/lib/flatpak/exports/bin/` and
   `~/.local/share/flatpak/exports/bin/`. Catches Flatpak installs whose
   `.desktop` file is missing. Returns `ExecutableSource::LinuxFlatpakBin`.
4. **Snap wrapper probe** — direct scan of `/snap/bin/`. Returns
   `ExecutableSource::LinuxSnapBin`.

The combined Linux scan costs ~20–60 ms on a warm filesystem. It runs once
inside `ExecutableIndex::build()`, so all eight program-detection
categories amortize the cost.
```

- [ ] **Step 2: Update `sniff/docs/sniff-library-architecture.md`**

Find the section that documents the shared-work highlights or cost model for programs detection. Append:

```markdown
- **Linux fallback index.** On Linux, `ExecutableIndex::build()` also
  populates a `LinuxDesktopIndex` holding three HashMaps:
  `desktop_entries` (from the freedesktop `.desktop` sweep across XDG data
  dirs, Flatpak exports, and Snap desktop dirs), `flatpak_bins` (from a
  direct probe of the system + user Flatpak export-bin dirs), and
  `snap_bins` (from `/snap/bin`). Warm-cache cost: ~20–60 ms serial for
  100–500 `.desktop` files, inside the existing build-once budget.
  Symlink canonicalization dedupes NixOS store entries. Non-Linux builds
  do not compile this code.
```

- [ ] **Step 3: Update `.claude/skills/sniff/programs.md`**

Find the "macOS App Bundle Fallback" section (starts around line 34). After it, add:

```markdown
## Linux Fallback Chain

PATH lookup with `.desktop` + Flatpak/Snap wrapper fallback:

```rust
use sniff::programs::find_program_with_source;

let (path, source) = find_program_with_source("firefox");
match source {
    ExecutableSource::Path => { /* Found in PATH */ }
    ExecutableSource::LinuxDesktopEntry => { /* Found via .desktop sweep */ }
    ExecutableSource::LinuxFlatpakBin => { /* Found under flatpak/exports/bin */ }
    ExecutableSource::LinuxSnapBin => { /* Found under /snap/bin */ }
}
```

Searches, in order:

1. `$PATH`
2. `$XDG_DATA_HOME/applications/*.desktop` and each
   `$XDG_DATA_DIRS/applications/*.desktop`
3. `/var/lib/flatpak/exports/share/applications/*.desktop` and the user
   equivalent under `$HOME`
4. `/var/lib/snapd/desktop/applications/*.desktop`
5. `/var/lib/flatpak/exports/bin/` and user equivalent
6. `/snap/bin/`

Aliases resolve common Flatpak app-ids to native binary names (e.g.
`org.mozilla.firefox` → `firefox`). Symlinks are canonicalized so NixOS
store copies dedupe automatically.
```

If the file currently lists `ExecutableSource` variants, append
`LinuxDesktopEntry`, `LinuxFlatpakBin`, and `LinuxSnapBin` with one-line
descriptions.

- [ ] **Step 4: Run doctests / lints across the affected areas**

Run:

```bash
cargo check -p sniff
cargo fmt --package sniff
cargo doc -p sniff --no-deps
```

Expected: no warnings, no broken intra-doc links.

- [ ] **Step 5: Commit**

```bash
git add sniff/lib/README.md sniff/docs/sniff-library-architecture.md .claude/skills/sniff/programs.md
git commit -m "docs(sniff): document Linux .desktop + Flatpak + Snap fallback chain"
```

---

## Task 13: (Optional) Add `ubuntu-latest` Linux integration-test run to CI

**Goal:** Ensure the new integration tests actually execute on CI. The existing sniff CI already builds on `ubuntu-latest`, but the new integration tests need `cargo test` (not just `cargo check`) to run and they need to run *after* a real PATH has been set up on the runner. This task is optional — the implementation is complete without it.

**Files:**
- Modify: the existing GitHub Actions workflow that runs `sniff` tests (likely `.github/workflows/sniff.yml` or the root `ci.yml`; verify before editing).

- [ ] **Step 1: Locate the existing sniff test job**

Run:

```bash
ls .github/workflows/
```

Then open whichever file currently runs `cargo test -p sniff` on `ubuntu-latest`.

- [ ] **Step 2: Confirm `ubuntu-latest` is already in the matrix**

If yes, nothing to change in the matrix — only confirm the job runs `cargo test -p sniff`, which will pick up the new integration tests automatically.

- [ ] **Step 3: Add a Linux-specific smoke job if one doesn't already exist**

If the current job only runs `cargo check` on Linux, add a new step (or promote the existing one) to run:

```yaml
- name: sniff integration tests (Linux)
  if: runner.os == 'Linux'
  run: cargo test -p sniff --tests
```

- [ ] **Step 4: Commit**

```bash
git add .github/workflows/
git commit -m "ci(sniff): run sniff integration tests on ubuntu-latest"
```

---

## Verification Checklist

Before declaring the plan complete, confirm all of the following from the repo root:

- [ ] `cargo check -p sniff` — clean on macOS
- [ ] `cargo check -p sniff --target x86_64-unknown-linux-gnu` — clean Linux cross-check
- [ ] `cargo check -p sniff --target x86_64-unknown-linux-gnu --tests` — clean Linux test build
- [ ] `cargo test -p sniff programs::types` — new serialization / display / `is_fallback` tests pass
- [ ] `cargo test -p sniff programs::find_program` — existing PATH + macOS tests unaffected
- [ ] `cargo test -p sniff programs::linux_desktop` — new unit tests pass (host runs a no-op because the module is cfg-gated; this command is a no-op on macOS but a real run on Linux CI)
- [ ] `cargo fmt --package sniff` — no formatting drift
- [ ] `cargo doc -p sniff --no-deps` — no broken intra-doc links
- [ ] `just test` (from `sniff/`) — full sniff package area test suite green
- [ ] Linux CI job — all four new integration tests (`linux_desktop_discovery`, `linux_flatpak_wrapper`, `linux_nixos_dedup`, `linux_priority_over_xdg_system`) pass on `ubuntu-latest`

---

## Notes for the Implementing Engineer

1. **PATH remains authoritative.** The Linux layers are *strict* fallbacks — never shuffle PATH below them, and never rewrite the PATH scan. The existing `is_executable` helper in `find_program.rs` already implements correct Unix semantics; do not re-implement it in `linux_desktop.rs`. (This plan deliberately copies a tiny `is_executable_file` helper into the new module to keep it self-contained, but a later refactor can promote it to `crate::programs::find_program::is_executable` if visibility allows.)
2. **Do NOT apply lowercase normalization to the Linux lookup key.** Unlike Windows (which lowercases because the registry + filesystem are case-insensitive), Linux filesystem names are case-sensitive. The `LinuxDesktopIndex` keys are exact basenames, and callers passing `Firefox` vs `firefox` must get different results. The `scan_desktop_entries` function already honors this.
3. **No new crate dependencies.** The design doc recommends `freedesktop-desktop-entry = "0.8"`, but this plan implements a hand-rolled parser to keep the change self-contained and to avoid adding a workspace-wide dep for a single module. If a future refactor wants to swap in the crate, the parser is fully isolated behind `parse_desktop_entry()` and `tokenize_exec()` — swap should be mechanical.
4. **`TryExec` is honored strictly.** The design doc's Open Question #3 recommends this: a `TryExec` that fails `is_file()` + execute-bit check drops the entry. Do not weaken this to "fall back to `Exec`" — the spec says the entry is a stub and must be skipped.
5. **NixOS dedup is by canonical path, not desktop-id.** Two profile paths that symlink to the same store entry produce one index entry. The index stores the *resolved binary*, not the `.desktop` file path, so rebuilds that change the store hash transparently update the index on the next scan.
6. **Integration tests use `unsafe { std::env::set_var(...) }` deliberately.** Rust 2024 marks `std::env::set_var` unsafe because it is not thread-safe. Library tests use `crate::test_helpers::ENV_MUTEX` to serialize, but integration tests live in separate binaries and cannot access the crate-private mutex. To sidestep this cleanly, the four integration test files each contain **exactly one `#[test]` function**. Do not add a second `#[test]` to any of them without introducing a local `Mutex<()>` guard.
7. **AppImages are out of scope.** Do not attempt to enumerate `~/Applications/*.AppImage`, `~/Downloads/*.AppImage`, etc. The only AppImages we catch are those that `appimaged`/`AppImageLauncher` has registered under `~/.local/share/applications/` — those are already covered by Layer 2 for free.
8. **D-Bus activation is out of scope.** Entries with `DBusActivatable=true` but no `Exec`/`TryExec` are dropped. A future pass could introspect the service file; not now.
9. **Scan cost budget:** 20–60 ms per `ExecutableIndex::build()` on Linux (warm cache). If you find yourself tempted to add deeper directory walks or more directories, retune first and verify against the benchmark harness under `sniff/benches/` before merging.
10. **Alias table maintenance.** `DESKTOP_ID_ALIASES` will drift over time. The design doc's Open Question #4 suggests reusing the existing `programs::enums` metadata (each category enum declares `binary_name` and `alternate_binary_names`). Do **not** take that on in this PR — keep the scope to what `linux-design.md` covers and file a follow-up for v1.1.
