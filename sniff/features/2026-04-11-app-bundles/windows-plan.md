# Windows App Discovery Implementation Plan


**Goal:** Extend `sniff`'s program detection on Windows beyond `PATH` to cover registry-installed GUI apps (via `App Paths`) and traditional installers (via a shallow walk of `Program Files` / `LocalAppData\Programs`), so that any app Windows can launch via **Win + R** is also found by `sniff`.

**Architecture:** Layered fallback chain inside the existing `ExecutableIndex`. PATH stays authoritative (layer 1), then App Paths registry (layer 2, HKCU > HKLM), then shallow install-root walk (layer 3). All new code is `#[cfg(target_os = "windows")]`; non-Windows builds compile and test unchanged. Two new `ExecutableSource` variants (`WindowsAppPaths`, `WindowsInstallRoot`) propagate through the existing `CategoryDetector` serialization with zero schema churn.

**Tech Stack:** Rust 2024, `winreg` 0.55 for registry access, `windows` 0.62 crate (already present) with the added `Win32_System_Environment` feature for `ExpandEnvironmentStringsW`, existing `rayon`/`thiserror`/`serde` infrastructure.

**Spec:** [`windows-design.md`](./windows-design.md) — read this first.

---

## File Structure

### New files

- `sniff/lib/src/programs/windows_apps.rs` — Windows discovery module (mirrors `macos_bundle.rs`).
  Contains: `WindowsIndex` struct, `build_windows_index()`, `scan_app_paths()`, `scan_install_roots()`, `install_root_dirs()`, `expand_env_vars()`, and private helpers. Entirely gated on `#[cfg(target_os = "windows")]`.
- `sniff/lib/tests/windows_find_program_priority.rs` — Windows-only integration test verifying PATH beats App Paths.
- `sniff/lib/tests/windows_app_paths_orphan.rs` — Windows-only integration test verifying orphaned registry entries are dropped.

### Modified files

- `sniff/lib/Cargo.toml` — add `winreg = "0.55"` and the `Win32_System_Environment` feature to the `windows` crate (Windows target only).
- `sniff/lib/src/programs/mod.rs` — register the new `windows_apps` module behind `#[cfg(target_os = "windows")]`.
- `sniff/lib/src/programs/types.rs` — extend `ExecutableSource` with `WindowsAppPaths` + `WindowsInstallRoot`, add `is_fallback()`, extend `Display`.
- `sniff/lib/src/programs/find_program.rs` — add `windows_index: WindowsIndex` field to `ExecutableIndex`, build it in `build_with_bundles`, and extend `find_with_source()` to consult Windows layers after PATH. Extend the one-off `find_program_with_source()` helper with the same fallback chain.
- `sniff/lib/README.md` — document the Windows fallback chain in the programs section.
- `sniff/docs/sniff-library-architecture.md` — note the Windows index cost model (~40–80 ms) under the existing shared-work discussion.
- `.claude/skills/sniff/programs.md` — add Windows detection notes alongside macOS bundle notes.

### Supporting context

- `sniff/lib/src/programs/macos_bundle.rs` — reference implementation for the parallel macOS side; do **not** modify.
- `sniff/lib/src/test_helpers.rs` — `ENV_MUTEX` + `ScopedEnv` already serialize env-mutating tests; reuse for Windows-only registry-and-env tests (no new `serial_test` dep needed despite what the design doc suggests).
- `sniff/lib/src/programs/schema.rs:479` — `ProgramEntry::installed()` already serializes `source` via `to_string().to_lowercase().replace(' ', "_")`, so the new variants' `Display` output (`"Windows App Paths"` / `"Windows Install Root"`) will auto-produce the correct `"windows_app_paths"` / `"windows_install_root"` strings. No schema change required.

---

## Task 1: Add Windows target and Cargo dependencies

**Goal:** Make the Windows codepath buildable from a macOS dev host via `cargo check --target x86_64-pc-windows-msvc` and add the two new Cargo dependencies the implementation needs.

**Files:**
- Modify: `sniff/lib/Cargo.toml`

- [ ] **Step 1: Install the Windows MSVC target for the toolchain**

Run:

```bash
rustup target list --installed | grep -q x86_64-pc-windows-msvc || rustup target add x86_64-pc-windows-msvc
```

Expected: target already installed or added successfully. If this step fails due to sandbox restrictions, run it manually in an interactive shell before continuing.

- [ ] **Step 2: Add `winreg` and extend the `windows` feature list**

Edit `sniff/lib/Cargo.toml`. Find the existing Windows target block:

```toml
[target.'cfg(target_os = "windows")'.dependencies]
windows = { version = "0.62", features = [
    "Win32_Foundation",
    "Win32_System_Services",
] }
```

Replace it with:

```toml
[target.'cfg(target_os = "windows")'.dependencies]
windows = { version = "0.62", features = [
    "Win32_Foundation",
    "Win32_System_Services",
    "Win32_System_Environment",
] }
winreg = "0.55"
```

- [ ] **Step 3: Verify the workspace still compiles on the host platform**

Run:

```bash
cargo check -p sniff
```

Expected: clean build (no new dependency code yet — just manifest changes).

- [ ] **Step 4: Verify the Windows target cross-check works**

Run:

```bash
cargo check -p sniff --target x86_64-pc-windows-msvc
```

Expected: `winreg` is downloaded and `windows` rebuilds with the new feature. The build should succeed because no new code references the new APIs yet.

- [ ] **Step 5: Commit**

```bash
git add sniff/lib/Cargo.toml
git commit -m "feat(sniff): add winreg + windows env feature for App Paths lookup"
```

---

## Task 2: Extend `ExecutableSource` with Windows variants

**Goal:** Add `WindowsAppPaths` and `WindowsInstallRoot` variants, update `Display` + `is_app_bundle()`, and introduce a new `is_fallback()` helper. All changes are compile-safe and testable on every platform.

**Files:**
- Modify: `sniff/lib/src/programs/types.rs`
- Test: `sniff/lib/src/programs/types.rs` (existing `mod tests`)

- [ ] **Step 1: Write the failing tests for the new variants**

Add to the `#[cfg(test)] mod tests` block near the existing `test_executable_source_serialize_json` in `sniff/lib/src/programs/types.rs`:

```rust
#[test]
fn test_executable_source_windows_variants_serialize() {
    let app_paths = ExecutableSource::WindowsAppPaths;
    let install_root = ExecutableSource::WindowsInstallRoot;

    assert_eq!(
        serde_json::to_string(&app_paths).unwrap(),
        "\"windows_app_paths\""
    );
    assert_eq!(
        serde_json::to_string(&install_root).unwrap(),
        "\"windows_install_root\""
    );
}

#[test]
fn test_executable_source_windows_variants_deserialize() {
    let ap: ExecutableSource = serde_json::from_str("\"windows_app_paths\"").unwrap();
    let ir: ExecutableSource = serde_json::from_str("\"windows_install_root\"").unwrap();

    assert_eq!(ap, ExecutableSource::WindowsAppPaths);
    assert_eq!(ir, ExecutableSource::WindowsInstallRoot);
}

#[test]
fn test_executable_source_windows_variants_display() {
    assert_eq!(
        ExecutableSource::WindowsAppPaths.to_string(),
        "Windows App Paths"
    );
    assert_eq!(
        ExecutableSource::WindowsInstallRoot.to_string(),
        "Windows Install Root"
    );
}

#[test]
fn test_executable_source_windows_variants_not_app_bundle() {
    assert!(!ExecutableSource::WindowsAppPaths.is_app_bundle());
    assert!(!ExecutableSource::WindowsInstallRoot.is_app_bundle());
}

#[test]
fn test_executable_source_is_fallback() {
    assert!(!ExecutableSource::Path.is_fallback());
    assert!(ExecutableSource::MacOsAppBundle.is_fallback());
    assert!(ExecutableSource::WindowsAppPaths.is_fallback());
    assert!(ExecutableSource::WindowsInstallRoot.is_fallback());
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run:

```bash
cargo test -p sniff programs::types::tests::test_executable_source_windows -- --nocapture
cargo test -p sniff programs::types::tests::test_executable_source_is_fallback -- --nocapture
```

Expected: both commands fail with compile errors ("no variant `WindowsAppPaths`", "no method `is_fallback`").

- [ ] **Step 3: Add the variants and helpers**

In `sniff/lib/src/programs/types.rs`, replace the existing `ExecutableSource` definition and its `impl` block with:

```rust
/// Describes where a program executable was discovered.
///
/// Distinguishes between traditional PATH-based executables, macOS `.app`
/// bundles, and Windows-specific fallback sources (registry App Paths, shallow
/// install-root walk). Non-PATH sources are "fallback" sources — they are
/// consulted only when PATH lookup misses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutableSource {
    /// Found via PATH lookup (traditional executable).
    Path,
    /// Found as a macOS `.app` bundle.
    MacOsAppBundle,
    /// Found via the Windows `App Paths` registry key
    /// (`HKCU|HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths`).
    WindowsAppPaths,
    /// Found via a shallow walk of a Windows install root
    /// (`%ProgramFiles%`, `%ProgramFiles(x86)%`, `%LocalAppData%\Programs`).
    WindowsInstallRoot,
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
            ExecutableSource::WindowsAppPaths => write!(f, "Windows App Paths"),
            ExecutableSource::WindowsInstallRoot => write!(f, "Windows Install Root"),
        }
    }
}
```

- [ ] **Step 4: Run the new tests and verify they pass**

Run:

```bash
cargo test -p sniff programs::types::tests::test_executable_source -- --nocapture
```

Expected: all `test_executable_source_*` tests pass, including the new ones.

- [ ] **Step 5: Run the full types module tests to catch regressions**

Run:

```bash
cargo test -p sniff programs::types::tests
```

Expected: every test in the module passes. The existing serialization tests for `Path` and `MacOsAppBundle` should still pass (the old variants are unchanged).

- [ ] **Step 6: Cross-check the Windows target still compiles**

Run:

```bash
cargo check -p sniff --target x86_64-pc-windows-msvc
```

Expected: clean build.

- [ ] **Step 7: Commit**

```bash
git add sniff/lib/src/programs/types.rs
git commit -m "feat(sniff): add WindowsAppPaths and WindowsInstallRoot executable sources"
```

---

## Task 3: Scaffold `windows_apps` module with `WindowsIndex` type

**Goal:** Create the new module file, register it in `programs/mod.rs` under `#[cfg(target_os = "windows")]`, and stub out `build_windows_index()` returning an empty `WindowsIndex`. Subsequent tasks flesh out each layer.

**Files:**
- Create: `sniff/lib/src/programs/windows_apps.rs`
- Modify: `sniff/lib/src/programs/mod.rs`

- [ ] **Step 1: Create the new module file with empty scans**

Create `sniff/lib/src/programs/windows_apps.rs` with:

```rust
//! Windows application discovery beyond PATH.
//!
//! Provides fallback lookups for Windows programs that are installed but not
//! in PATH, via the App Paths registry key and shallow walks of the standard
//! install-root directories.
//!
//! All public items in this module are gated on `#[cfg(target_os = "windows")]`.
//! Non-Windows builds do not compile this code.

#![cfg(target_os = "windows")]

use std::collections::HashMap;
use std::path::PathBuf;

/// Builds the Windows-specific fallback index.
///
/// Scans `App Paths` (HKLM + HKCU) and the canonical install-root directories
/// once, returning a `WindowsIndex` ready for O(1) lookups. Both `app_paths`
/// and `install_roots` maps use lowercased binary-name keys and include the
/// `.exe`-suffixed form alongside the bare stem.
///
/// ## Cost
///
/// Typical warm-cache cost: 40–80 ms serial. Called once per
/// `ExecutableIndex::build()` on Windows.
pub(super) fn build_windows_index() -> WindowsIndex {
    WindowsIndex {
        app_paths: scan_app_paths(),
        install_roots: scan_install_roots(),
    }
}

/// Windows-specific fallback index populated by [`build_windows_index`].
///
/// Two HashMaps keyed by lowercased binary name. Checked in priority order
/// (`app_paths` before `install_roots`) after PATH by
/// [`crate::programs::find_program::ExecutableIndex::find_with_source`].
#[derive(Debug, Default, Clone)]
pub(super) struct WindowsIndex {
    /// Name → path map built from the App Paths registry key.
    pub app_paths: HashMap<String, PathBuf>,
    /// Name → path map built from a shallow walk of install roots.
    pub install_roots: HashMap<String, PathBuf>,
}

/// Placeholder — populated in a later task.
fn scan_app_paths() -> HashMap<String, PathBuf> {
    HashMap::new()
}

/// Placeholder — populated in a later task.
fn scan_install_roots() -> HashMap<String, PathBuf> {
    HashMap::new()
}
```

- [ ] **Step 2: Register the module in `programs/mod.rs`**

In `sniff/lib/src/programs/mod.rs`, find the list of `pub mod ...` declarations near line 95-110. After the `pub mod macos_bundle;` line, add:

```rust
#[cfg(target_os = "windows")]
pub(crate) mod windows_apps;
```

- [ ] **Step 3: Run host-platform checks**

Run:

```bash
cargo check -p sniff
cargo test -p sniff programs:: --lib
```

Expected: clean build and all existing tests still pass (the new module is absent on non-Windows, so nothing changes).

- [ ] **Step 4: Run the Windows cross-check**

Run:

```bash
cargo check -p sniff --target x86_64-pc-windows-msvc
```

Expected: clean build. `winreg` is now a direct dep but is not yet used — that is intentional; adding the `extern` reference in the next task will exercise it.

- [ ] **Step 5: Commit**

```bash
git add sniff/lib/src/programs/windows_apps.rs sniff/lib/src/programs/mod.rs
git commit -m "feat(sniff): scaffold windows_apps module with empty WindowsIndex"
```

---

## Task 4: Implement `expand_env_vars` via `ExpandEnvironmentStringsW`

**Goal:** Build the env-variable expander used by both scan layers. This touches only the new module and has no dependencies on other tasks, so it can be reviewed in isolation.

**Files:**
- Modify: `sniff/lib/src/programs/windows_apps.rs`

- [ ] **Step 1: Add the expander function at the bottom of `windows_apps.rs`**

Append to `sniff/lib/src/programs/windows_apps.rs`:

```rust
/// Expands Windows environment variables (`%Name%`) using `ExpandEnvironmentStringsW`.
///
/// Falls back to the raw input when the API call fails or when no variables
/// are present. Unknown variables are left untouched (the Win32 API preserves
/// them verbatim), which lets downstream `is_file()` checks drop stale paths.
///
/// ## Implementation note
///
/// We call the native Win32 API rather than a Rust-level expansion crate so
/// that edge cases like `%ProgramFiles(x86)%` (parentheses) and `%SystemRoot%`
/// (kernel-known but not always in process env) behave exactly like every
/// other Windows consumer. The `windows` crate's `Win32_System_Environment`
/// feature is enabled in `Cargo.toml`.
fn expand_env_vars(input: &str) -> String {
    use windows::Win32::System::Environment::ExpandEnvironmentStringsW;
    use windows::core::HSTRING;

    let wide = HSTRING::from(input);

    // First call with `None` returns the required buffer size in wide chars,
    // including the terminating null.
    let required = unsafe { ExpandEnvironmentStringsW(&wide, None) };
    if required == 0 {
        return input.to_string();
    }

    let mut buf = vec![0u16; required as usize];
    let written = unsafe { ExpandEnvironmentStringsW(&wide, Some(&mut buf)) };
    if written == 0 {
        return input.to_string();
    }

    // `written` includes the null terminator — trim it.
    let end = (written as usize).saturating_sub(1);
    String::from_utf16_lossy(&buf[..end])
}
```

- [ ] **Step 2: Add a Windows-only unit test beneath the function**

Append to the same file:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_env_vars_resolves_system_root() {
        // %SystemRoot% is always set on Windows and points at something
        // like `C:\Windows`. Expansion must rewrite it and the resulting
        // path must exist.
        let expanded = expand_env_vars("%SystemRoot%\\System32");
        assert!(
            !expanded.contains('%'),
            "expected expansion, got {expanded}"
        );
        assert!(
            std::path::Path::new(&expanded).is_dir(),
            "expected {expanded} to exist"
        );
    }

    #[test]
    fn expand_env_vars_preserves_unknown_variable() {
        // Unknown variables are left verbatim by Win32 — we inherit that
        // behavior, and the downstream `is_file()` check drops them.
        let input = "%__sniff_definitely_not_set_1234__%\\bin";
        let expanded = expand_env_vars(input);
        assert_eq!(expanded, input);
    }

    #[test]
    fn expand_env_vars_passes_through_plain_string() {
        let plain = r"C:\tools\foo.exe";
        assert_eq!(expand_env_vars(plain), plain);
    }

    #[test]
    fn expand_env_vars_handles_empty_string() {
        assert_eq!(expand_env_vars(""), "");
    }
}
```

- [ ] **Step 3: Host check (should stay compile-clean on macOS)**

Run:

```bash
cargo check -p sniff
```

Expected: success — the module is entirely `#[cfg(target_os = "windows")]`, so macOS does not compile the new code.

- [ ] **Step 4: Windows cross-compile**

Run:

```bash
cargo check -p sniff --target x86_64-pc-windows-msvc
```

Expected: clean build. This is the primary signal that the `ExpandEnvironmentStringsW` import resolves.

- [ ] **Step 5: Commit**

```bash
git add sniff/lib/src/programs/windows_apps.rs
git commit -m "feat(sniff): add Windows env-var expansion via ExpandEnvironmentStringsW"
```

---

## Task 5: Implement `scan_app_paths` (registry layer)

**Goal:** Replace the `scan_app_paths` placeholder with a real enumeration of HKLM + HKCU `App Paths` subkeys, with HKCU winning ties (matching `ShellExecuteEx` precedence).

**Files:**
- Modify: `sniff/lib/src/programs/windows_apps.rs`

- [ ] **Step 1: Replace the `scan_app_paths` placeholder**

In `sniff/lib/src/programs/windows_apps.rs`, replace:

```rust
/// Placeholder — populated in a later task.
fn scan_app_paths() -> HashMap<String, PathBuf> {
    HashMap::new()
}
```

with:

```rust
/// Scans the `App Paths` registry key in HKLM and HKCU.
///
/// Returns a name → path map where entries from HKCU take precedence over
/// HKLM (matching `ShellExecuteEx`'s resolution order). Every key is
/// lowercased and each entry is written under both the bare stem and the
/// `.exe`-suffixed name so that lookups for `"chrome"` and `"chrome.exe"`
/// both succeed. Orphaned entries whose target file does not exist are
/// filtered out.
fn scan_app_paths() -> HashMap<String, PathBuf> {
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ};

    const APP_PATHS: &str =
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths";

    let mut map: HashMap<String, PathBuf> = HashMap::new();

    // HKLM is scanned first; HKCU overwrites on collision, matching
    // `ShellExecuteEx`'s "HKCU wins" rule.
    for hive in [HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER] {
        let Ok(root) = RegKey::predef(hive).open_subkey_with_flags(APP_PATHS, KEY_READ) else {
            continue;
        };

        for subkey_name in root.enum_keys().flatten() {
            let Ok(subkey) = root.open_subkey_with_flags(&subkey_name, KEY_READ) else {
                continue;
            };

            // Default value = full path (possibly with %EnvVars%). Missing => skip.
            let raw: String = match subkey.get_value::<String, _>("") {
                Ok(v) => v,
                Err(_) => continue,
            };

            let expanded = expand_env_vars(&raw);
            let path = PathBuf::from(expanded);

            // Drop orphaned registry entries.
            if !path.is_file() {
                continue;
            }

            let key_lower = subkey_name.to_ascii_lowercase();
            // Insert under both the `.exe` variant and the bare stem.
            if let Some(stem) = key_lower.strip_suffix(".exe") {
                map.insert(stem.to_string(), path.clone());
                map.insert(key_lower, path);
            } else {
                map.insert(format!("{key_lower}.exe"), path.clone());
                map.insert(key_lower, path);
            }
        }
    }

    map
}
```

- [ ] **Step 2: Add a Windows-only unit test for the HKCU precedence rule**

Append to the existing `#[cfg(test)] mod tests` block in the same file:

```rust
#[test]
fn scan_app_paths_honors_hkcu_precedence_over_hklm() {
    // This test writes to HKCU (no admin needed) under a unique subkey,
    // asserts the scan picks it up, then cleans up. It is serialized via
    // the shared env mutex to avoid clobbering parallel tests that
    // mutate global state.
    use crate::test_helpers::ENV_MUTEX;
    use std::fs;
    use tempfile::NamedTempFile;
    use winreg::RegKey;
    use winreg::enums::{HKEY_CURRENT_USER, KEY_ALL_ACCESS};

    let _lock = ENV_MUTEX.lock().unwrap();

    // Create a real file the registry entry can point at.
    let tmp = NamedTempFile::new().unwrap();
    let tmp_path = tmp.path().to_path_buf();
    // Ensure the suffix is `.exe` so `is_file()` + the stem/suffix logic
    // both apply cleanly.
    let exe_path = tmp_path.with_extension("exe");
    fs::copy(&tmp_path, &exe_path).unwrap();

    const TEST_KEY: &str =
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\sniff_test_unique_12345.exe";

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hkcu.create_subkey(TEST_KEY).unwrap();
    key.set_value("", &exe_path.to_string_lossy().to_string())
        .unwrap();

    let result = scan_app_paths();

    // Cleanup before asserting so a failing assert still leaves a clean
    // registry.
    let _ = hkcu.delete_subkey_all(TEST_KEY);
    let _ = fs::remove_file(&exe_path);

    let got = result
        .get("sniff_test_unique_12345")
        .cloned()
        .expect("HKCU entry should surface in the scan map");
    assert_eq!(got, exe_path);

    // Both the bare stem and the .exe key should resolve.
    assert_eq!(
        result.get("sniff_test_unique_12345.exe").cloned(),
        Some(exe_path)
    );
}

#[test]
fn scan_app_paths_filters_orphaned_entries() {
    use crate::test_helpers::ENV_MUTEX;
    use winreg::RegKey;
    use winreg::enums::HKEY_CURRENT_USER;

    let _lock = ENV_MUTEX.lock().unwrap();

    const TEST_KEY: &str =
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\sniff_test_orphan_99999.exe";

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hkcu.create_subkey(TEST_KEY).unwrap();
    // Point at a path that cannot exist.
    key.set_value(
        "",
        &r"C:\__sniff_orphan_definitely_missing__\nope.exe".to_string(),
    )
    .unwrap();

    let result = scan_app_paths();
    let _ = hkcu.delete_subkey_all(TEST_KEY);

    assert!(
        !result.contains_key("sniff_test_orphan_99999"),
        "orphaned entries must be filtered out"
    );
}
```

- [ ] **Step 3: Host-side check (macOS): still compile-clean**

Run:

```bash
cargo check -p sniff
```

Expected: clean build — the new code path is still entirely `#[cfg(target_os = "windows")]`.

- [ ] **Step 4: Windows cross-compile**

Run:

```bash
cargo check -p sniff --target x86_64-pc-windows-msvc
cargo check -p sniff --tests --target x86_64-pc-windows-msvc
```

Expected: clean build for both the library and the tests. `--tests` is important because it exercises the new unit-test block.

- [ ] **Step 5: Commit**

```bash
git add sniff/lib/src/programs/windows_apps.rs
git commit -m "feat(sniff): scan Windows App Paths registry with HKCU precedence"
```

---

## Task 6: Implement `scan_install_roots` (shallow install-root walk)

**Goal:** Replace the `scan_install_roots` placeholder with a shallow one-level walk of `%ProgramFiles%`, `%ProgramFiles(x86)%`, and `%LocalAppData%\Programs`, capturing every `*.exe` at the root of each first-level child.

**Files:**
- Modify: `sniff/lib/src/programs/windows_apps.rs`

- [ ] **Step 1: Add the install-root helpers**

In `sniff/lib/src/programs/windows_apps.rs`, replace the current `scan_install_roots` placeholder with:

```rust
/// Returns the canonical Windows install roots in priority order.
///
/// Order is deliberate: `%ProgramFiles%` → `%ProgramFiles(x86)%` →
/// `%LocalAppData%\Programs`. Any unset env var is skipped (we do not
/// fabricate a path). The priority matters for the "first write wins"
/// rule inside [`scan_install_roots`].
fn install_root_dirs() -> Vec<PathBuf> {
    let mut v = Vec::with_capacity(3);
    for var in ["ProgramFiles", "ProgramFiles(x86)"] {
        if let Ok(path) = std::env::var(var) {
            v.push(PathBuf::from(path));
        }
    }
    if let Ok(lad) = std::env::var("LocalAppData") {
        v.push(PathBuf::from(lad).join("Programs"));
    }
    v
}

/// Walks the supplied roots one level deep, indexing every `.exe` found at
/// the root of each first-level child directory.
///
/// Extracted as a pure function (no env reads) so unit tests can drive it
/// with a `tempdir()` fixture. `scan_install_roots` is a thin wrapper that
/// simply calls this with `install_root_dirs()`.
///
/// ## Rules
///
/// - Only one directory level is walked — nested helpers (`<App>\bin\`,
///   `<App>\resources\`) are intentionally ignored to keep false-positives
///   low and the cost capped.
/// - "First write wins" — entries earlier in `roots` beat later entries
///   for the same lowercased key.
/// - Every key is stored under both the `.exe`-suffixed form and the bare
///   stem.
fn walk_install_roots(roots: &[PathBuf]) -> HashMap<String, PathBuf> {
    let mut map: HashMap<String, PathBuf> = HashMap::new();

    for root in roots {
        let Ok(children) = std::fs::read_dir(root) else {
            continue;
        };
        for child in children.flatten() {
            let child_path = child.path();
            if !child_path.is_dir() {
                continue;
            }

            let Ok(entries) = std::fs::read_dir(&child_path) else {
                continue;
            };
            for entry in entries.flatten() {
                let p = entry.path();
                if !p.is_file() {
                    continue;
                }
                let Some(name) = p.file_name().and_then(|n| n.to_str()) else {
                    continue;
                };
                let lower = name.to_ascii_lowercase();
                if !lower.ends_with(".exe") {
                    continue;
                }

                // First write wins — higher-priority roots come first.
                let stem = lower.trim_end_matches(".exe").to_string();
                map.entry(lower.clone()).or_insert_with(|| p.clone());
                map.entry(stem).or_insert(p);
            }
        }
    }

    map
}

/// Scans the standard Windows install roots and returns a name → path map.
///
/// Equivalent to `walk_install_roots(&install_root_dirs())`. See those two
/// functions for the contract.
fn scan_install_roots() -> HashMap<String, PathBuf> {
    walk_install_roots(&install_root_dirs())
}
```

- [ ] **Step 2: Add unit tests driven by `tempdir()`**

Append to the existing `#[cfg(test)] mod tests` block:

```rust
#[test]
fn walk_install_roots_indexes_exe_at_child_root() {
    use std::fs;

    let tmp = tempfile::tempdir().unwrap();
    let app_dir = tmp.path().join("MockApp");
    fs::create_dir_all(&app_dir).unwrap();

    let exe = app_dir.join("mock.exe");
    fs::write(&exe, b"fake").unwrap();

    let result = walk_install_roots(&[tmp.path().to_path_buf()]);

    assert_eq!(result.get("mock.exe").cloned(), Some(exe.clone()));
    assert_eq!(result.get("mock").cloned(), Some(exe));
}

#[test]
fn walk_install_roots_ignores_nested_bin_directory() {
    use std::fs;

    let tmp = tempfile::tempdir().unwrap();
    let app_dir = tmp.path().join("DeepApp");
    let bin_dir = app_dir.join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    // Only nested — root has no exe.
    fs::write(bin_dir.join("hidden.exe"), b"x").unwrap();

    let result = walk_install_roots(&[tmp.path().to_path_buf()]);

    assert!(
        !result.contains_key("hidden"),
        "nested exes must not be indexed"
    );
}

#[test]
fn walk_install_roots_first_write_wins_across_roots() {
    use std::fs;

    let high = tempfile::tempdir().unwrap();
    let low = tempfile::tempdir().unwrap();

    fs::create_dir_all(high.path().join("Dup")).unwrap();
    fs::create_dir_all(low.path().join("Dup")).unwrap();

    let high_exe = high.path().join("Dup").join("dup.exe");
    let low_exe = low.path().join("Dup").join("dup.exe");
    fs::write(&high_exe, b"H").unwrap();
    fs::write(&low_exe, b"L").unwrap();

    let result = walk_install_roots(&[
        high.path().to_path_buf(),
        low.path().to_path_buf(),
    ]);

    assert_eq!(
        result.get("dup").cloned(),
        Some(high_exe),
        "earlier root wins the key"
    );
}

#[test]
fn walk_install_roots_skips_missing_root() {
    let result = walk_install_roots(&[PathBuf::from(r"C:\__sniff_nope__")]);
    assert!(result.is_empty());
}

#[test]
fn install_root_dirs_reads_env_vars() {
    use crate::test_helpers::{ENV_MUTEX, ScopedEnv};

    let _lock = ENV_MUTEX.lock().unwrap();
    let mut env = ScopedEnv::new();
    env.set("ProgramFiles", r"C:\pf");
    env.set("ProgramFiles(x86)", r"C:\pf86");
    env.set("LocalAppData", r"C:\Users\test\AppData\Local");

    let dirs = install_root_dirs();
    assert_eq!(dirs.len(), 3);
    assert_eq!(dirs[0], PathBuf::from(r"C:\pf"));
    assert_eq!(dirs[1], PathBuf::from(r"C:\pf86"));
    assert_eq!(
        dirs[2],
        PathBuf::from(r"C:\Users\test\AppData\Local\Programs")
    );
}
```

- [ ] **Step 3: Host-side check**

Run:

```bash
cargo check -p sniff
```

Expected: clean build — the new code is still entirely `#[cfg(target_os = "windows")]`.

- [ ] **Step 4: Windows cross-compile (lib + tests)**

Run:

```bash
cargo check -p sniff --target x86_64-pc-windows-msvc
cargo check -p sniff --tests --target x86_64-pc-windows-msvc
```

Expected: clean build.

- [ ] **Step 5: Commit**

```bash
git add sniff/lib/src/programs/windows_apps.rs
git commit -m "feat(sniff): shallow-walk Windows install roots for exe discovery"
```

---

## Task 7: Wire `WindowsIndex` into `ExecutableIndex`

**Goal:** Add a `#[cfg(target_os = "windows")]` field on `ExecutableIndex`, populate it inside `build_with_bundles`, and extend `find_with_source()` to consult the Windows layers after PATH. On non-Windows the struct and method bodies are unchanged.

**Files:**
- Modify: `sniff/lib/src/programs/find_program.rs`

- [ ] **Step 1: Add the field to `ExecutableIndex`**

In `sniff/lib/src/programs/find_program.rs`, find the struct definition near line 71-80 that currently reads:

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
    /// Windows-specific fallback index (App Paths + install-root walk).
    #[cfg(target_os = "windows")]
    windows_index: super::windows_apps::WindowsIndex,
}
```

- [ ] **Step 2: Populate `windows_index` in `build_with_bundles`**

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
            #[cfg(target_os = "windows")]
            windows_index: if include_bundles {
                super::windows_apps::build_windows_index()
            } else {
                super::windows_apps::WindowsIndex::default()
            },
        }
```

The `include_bundles` flag is reused intentionally: it means "fallback sources" broadly, and `build_path_only()` stays a pure PATH snapshot.

- [ ] **Step 3: Extend `find_with_source` with the two Windows layers**

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
        // Layer 1: PATH (authoritative — matches `CreateProcess`).
        if let Some(path) = self.path_executables.get(program) {
            return Some((path.clone(), ExecutableSource::Path));
        }

        // Layer 2: macOS bundles.
        #[cfg(target_os = "macos")]
        if let Some(path) = self.bundle_executables.get(program) {
            return Some((path.clone(), ExecutableSource::MacOsAppBundle));
        }

        // Layers 3 + 4: Windows App Paths registry, then shallow install-root walk.
        // Both maps are lowercased at build time; normalize the lookup key to
        // match so that `code` and `Code` both resolve.
        #[cfg(target_os = "windows")]
        {
            let key = program.to_ascii_lowercase();
            if let Some(path) = self.windows_index.app_paths.get(&key) {
                return Some((path.clone(), ExecutableSource::WindowsAppPaths));
            }
            if let Some(path) = self.windows_index.install_roots.get(&key) {
                return Some((path.clone(), ExecutableSource::WindowsInstallRoot));
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

    // Priority 3 + 4: Windows fallback layers. One-off callers pay the build
    // cost of the WindowsIndex on each call; `ExecutableIndex::build()` is
    // preferred for batch lookups.
    #[cfg(target_os = "windows")]
    {
        let key = program_str.to_ascii_lowercase();
        let idx = super::windows_apps::build_windows_index();
        if let Some(path) = idx.app_paths.get(&key) {
            return Some((path.clone(), ExecutableSource::WindowsAppPaths));
        }
        if let Some(path) = idx.install_roots.get(&key) {
            return Some((path.clone(), ExecutableSource::WindowsInstallRoot));
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

Expected: all existing tests in both modules still pass. None of them reference the Windows variants, so the change should be invisible to them.

- [ ] **Step 6: Windows cross-compile**

Run:

```bash
cargo check -p sniff --target x86_64-pc-windows-msvc
cargo check -p sniff --tests --target x86_64-pc-windows-msvc
```

Expected: clean build.

- [ ] **Step 7: Commit**

```bash
git add sniff/lib/src/programs/find_program.rs
git commit -m "feat(sniff): consult Windows fallback index in ExecutableIndex lookups"
```

---

## Task 8: Add Windows integration tests

**Goal:** Two focused integration-test files that exercise the full pipeline on a Windows host. Both are `#[cfg(target_os = "windows")]` so macOS builds ignore them.

**Files:**
- Create: `sniff/lib/tests/windows_find_program_priority.rs`
- Create: `sniff/lib/tests/windows_app_paths_orphan.rs`

- [ ] **Step 1: Create the priority test**

Create `sniff/lib/tests/windows_find_program_priority.rs` with:

```rust
//! Integration test: PATH must win over App Paths for the same program.
//!
//! Runs on Windows only; on other platforms the file compiles to an empty
//! module so the test binary still links.

#![cfg(target_os = "windows")]

use sniff::programs::{ExecutableIndex, ExecutableSource};

/// Any exe on the default Windows install that lives in PATH. `cmd.exe`
/// ships under `%SystemRoot%\System32`, which is always in PATH, and is
/// also not typically registered under App Paths — which means the test
/// only meaningfully tells us "PATH branch returned Path source".
#[test]
fn path_wins_over_fallbacks_for_cmd() {
    let index = ExecutableIndex::build();
    let (_, source) = index
        .find_with_source("cmd")
        .or_else(|| index.find_with_source("cmd.exe"))
        .expect("cmd.exe should always be on a Windows host");
    assert_eq!(source, ExecutableSource::Path);
}
```

- [ ] **Step 2: Create the orphan test**

Create `sniff/lib/tests/windows_app_paths_orphan.rs` with:

```rust
//! Integration test: an App Paths registry entry that points at a
//! nonexistent target must not appear in the built index.
//!
//! Writes to `HKCU` only (no admin required). Cleans up after itself even
//! on assertion failure.

#![cfg(target_os = "windows")]

use sniff::programs::ExecutableIndex;
use winreg::enums::HKEY_CURRENT_USER;
use winreg::RegKey;

const TEST_KEY: &str =
    r"SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\sniff_integration_orphan.exe";

struct OrphanGuard;

impl Drop for OrphanGuard {
    fn drop(&mut self) {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let _ = hkcu.delete_subkey_all(TEST_KEY);
    }
}

#[test]
fn orphaned_hkcu_entry_is_filtered() {
    let _guard = OrphanGuard;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hkcu.create_subkey(TEST_KEY).unwrap();
    key.set_value(
        "",
        &r"C:\__sniff_nowhere__\never_existed.exe".to_string(),
    )
    .unwrap();

    let index = ExecutableIndex::build();
    assert!(
        index.find_with_source("sniff_integration_orphan").is_none(),
        "orphan should not resolve"
    );
    assert!(
        index
            .find_with_source("sniff_integration_orphan.exe")
            .is_none(),
        "orphan should not resolve under .exe form either"
    );
}
```

- [ ] **Step 3: Declare `winreg` as a dev-dep for Windows integration tests**

The library crate already depends on `winreg` on Windows, so it is transitively visible to integration tests. Verify by cross-compiling the integration test target:

```bash
cargo check -p sniff --tests --target x86_64-pc-windows-msvc
```

Expected: clean build. If the test file fails to find `winreg`, add a Windows-only dev-dep to `sniff/lib/Cargo.toml`:

```toml
[target.'cfg(target_os = "windows")'.dev-dependencies]
winreg = "0.55"
```

and re-run the cross-check until it is clean.

- [ ] **Step 4: Host-side build (macOS)**

Run:

```bash
cargo check -p sniff --tests
```

Expected: clean build — both files collapse to empty modules on non-Windows.

- [ ] **Step 5: Commit**

```bash
git add sniff/lib/tests/windows_find_program_priority.rs sniff/lib/tests/windows_app_paths_orphan.rs sniff/lib/Cargo.toml
git commit -m "test(sniff): add Windows integration tests for PATH priority + orphan filtering"
```

---

## Task 9: Update documentation

**Goal:** Bring the library README, the architecture doc, and the `sniff` skill's `programs.md` in sync with the new Windows fallback chain.

**Files:**
- Modify: `sniff/lib/README.md`
- Modify: `sniff/docs/sniff-library-architecture.md`
- Modify: `.claude/skills/sniff/programs.md`

- [ ] **Step 1: Update `sniff/lib/README.md`**

Find the section describing macOS app bundle fallback (search for `MacOsAppBundle` or `app bundles`). Immediately after that paragraph, add:

```markdown
### Windows fallback chain

On Windows the executable search expands beyond PATH to cover registry-installed
GUI apps and traditional installers:

1. **PATH** — `CreateProcess`-compatible, returns `ExecutableSource::Path`.
2. **App Paths registry** — `HKCU` then `HKLM`
   (`SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths`). HKCU wins ties to
   match `ShellExecuteEx`. Environment variables in the target path are
   expanded via `ExpandEnvironmentStringsW` and orphaned entries (whose target
   no longer exists) are dropped. Returns `ExecutableSource::WindowsAppPaths`.
3. **Install-root walk** — one directory deep under `%ProgramFiles%`,
   `%ProgramFiles(x86)%`, and `%LocalAppData%\Programs`. Returns
   `ExecutableSource::WindowsInstallRoot`. Catches VS Code-style
   user-scope installers that never register with App Paths.

The combined Windows scan costs 40–80 ms on a warm filesystem. It runs once
inside `ExecutableIndex::build()`, so the eight program-detection categories
amortize the cost.
```

- [ ] **Step 2: Update `sniff/docs/sniff-library-architecture.md`**

Find the section that documents the shared-work highlights or cost model for programs detection. Append a bullet / paragraph:

```markdown
- **Windows fallback index.** On Windows, `ExecutableIndex::build()` also
  populates a `WindowsIndex` holding two HashMaps: `app_paths` (from a
  HKLM+HKCU registry walk) and `install_roots` (from a one-level directory
  walk of `Program Files`, `Program Files (x86)`, and `LocalAppData\Programs`).
  Warm-cache cost: 40–80 ms serial, inside the existing build-once budget.
  Non-Windows builds do not compile this code.
```

- [ ] **Step 3: Update `.claude/skills/sniff/programs.md`**

If the file discusses macOS bundle fallback, add a parallel "Windows fallback" subsection summarizing the same three-layer chain. If the file currently lists `ExecutableSource` variants, append `WindowsAppPaths` and `WindowsInstallRoot` with one-line descriptions.

- [ ] **Step 4: Run doctests / lints across the affected areas**

Run:

```bash
cargo check -p sniff
cargo fmt --package sniff
cargo doc -p sniff --no-deps
```

Expected: no warnings, no broken intra-doc links. `cargo doc` is the backstop for broken rustdoc links introduced by the README update if any links reference internal items.

- [ ] **Step 5: Commit**

```bash
git add sniff/lib/README.md sniff/docs/sniff-library-architecture.md .claude/skills/sniff/programs.md
git commit -m "docs(sniff): document Windows App Paths fallback chain"
```

---

## Task 10: Add `windows-latest` to the CI matrix

**Goal:** Run the Windows integration tests automatically on every push. This task is optional and independently mergeable — the implementation is complete without it.

**Files:**
- Modify: the existing GitHub Actions workflow that runs `sniff` tests (likely `.github/workflows/sniff.yml` or the root `ci.yml`; verify before editing).

- [ ] **Step 1: Locate the existing sniff test job**

Run:

```bash
ls .github/workflows/
```

Then open whichever file currently runs `cargo test -p sniff` on `ubuntu-latest` / `macos-latest`.

- [ ] **Step 2: Add `windows-latest` to the matrix**

Inside the `strategy.matrix.os` list (or equivalent), add `windows-latest`. Example edit:

```yaml
strategy:
  matrix:
    os: [ubuntu-latest, macos-latest, windows-latest]
```

Verify the job's `run:` steps are compatible with PowerShell (backslashes, command escaping). If the job uses `just`, ensure the relevant recipe also runs under `bash` on Windows runners (the runner's Git Bash is fine).

- [ ] **Step 3: Push a branch and watch the run**

Run:

```bash
git push -u origin HEAD
```

Monitor the `windows-latest` job to confirm the new integration tests pass.

- [ ] **Step 4: Commit**

```bash
git add .github/workflows/
git commit -m "ci(sniff): run tests on windows-latest"
```

---

## Verification Checklist

Before declaring the plan complete, confirm all of the following from the repo root:

- [ ] `cargo check -p sniff` — clean on macOS
- [ ] `cargo check -p sniff --target x86_64-pc-windows-msvc` — clean Windows cross-compile
- [ ] `cargo check -p sniff --tests --target x86_64-pc-windows-msvc` — clean Windows test build
- [ ] `cargo test -p sniff programs::types` — new serialization / display / `is_fallback` tests pass
- [ ] `cargo test -p sniff programs::find_program` — existing PATH + macOS tests unaffected
- [ ] `cargo fmt --package sniff` — no formatting drift
- [ ] `cargo doc -p sniff --no-deps` — no broken intra-doc links
- [ ] `just test` (from `sniff/`) — full sniff package area test suite green
- [ ] Windows CI job (if Task 10 was taken) — `windows_find_program_priority` and `windows_app_paths_orphan` both pass

---

## Notes for the Implementing Engineer

1. **Do not touch the existing PATH-scan case handling.** The design is explicit that PATH is layer 1 and unchanged. The new Windows layers normalize lookup keys to lowercase *only inside the Windows branch* of `find_with_source` — do not propagate that lowercasing to the generic code.
2. **The `serial_test` crate is NOT in this workspace** despite what the design doc suggests. Use the existing `crate::test_helpers::ENV_MUTEX` pattern (see `sniff/lib/src/test_helpers.rs`) to serialize any test that mutates env vars or the registry.
3. **Registry tests must be side-effect-safe.** Always pair `hkcu.create_subkey(...)` with a `Drop` guard or an unconditional `delete_subkey_all` *before* the assertion that could panic. The registry is shared process state; a leaked key can break future runs.
4. **`WindowsIndex` is `pub(super)` on purpose.** Callers go through `ExecutableIndex::find_with_source`; they should not touch the internal HashMaps directly. Do not widen the visibility without a concrete reason.
5. **Do not add `WindowsApps` alias-dir scanning.** The design explicitly defers it because the existing PATH scan already catches `%LocalAppData%\Microsoft\WindowsApps\*` (it is in the user PATH by default). If you hit a reparse-point edge case during Task 8, document it and open a follow-up issue rather than expanding scope.
6. **`include_bundles` on `build_path_only()` now controls the Windows index too.** This is the intended generalization: "no fallback sources" means exactly that, regardless of platform.
7. **Cost budget:** 40–80 ms per `ExecutableIndex::build()` on Windows. If you find yourself tempted to add deeper directory walks, retune first and verify against the benchmark harness under `sniff/benches/` before merging.
