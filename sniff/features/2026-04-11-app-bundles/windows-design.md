# Windows App Discovery Design

**Date:** 2026-04-11
**Status:** Draft
**Area:** sniff/lib (`programs` module)
**Related:** [linux-design.md](./linux-design.md), existing macOS implementation at `sniff/lib/src/programs/macos_bundle.rs`

## Context

`sniff` currently discovers installed programs by scanning `PATH` and (on macOS)
`/Applications` + `~/Applications`. On Windows the PATH-only strategy misses a
significant class of legitimately-installed applications: GUI apps that ship
with per-user installers and never extend the machine PATH, MSIX / Store apps
reached via execution aliases, and Office / Adobe products that register via
registry keys rather than PATH. This document designs a Windows-specific
fallback chain that matches user expectations for "is X installed?", i.e. it
answers *yes* for anything that Windows itself can launch via **Win + R** or
`start <name>`.

## Research Summary

Authoritative findings from the accompanying research report:

- **App Paths** (`HKLM|HKCU\SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\<exe>`)
  is the documented, `ShellExecuteEx`-consulted registry key that maps an
  executable name to a fully qualified path. It is the direct analog of the
  macOS bundle scan.
- **`CreateProcess` does NOT consult App Paths** — only the Shell API does.
  Because `which`/`CreateProcess` are PATH-bound, augmenting `sniff` with App
  Paths is a semantic extension, not a semantic change: we begin resolving what
  Win + R resolves.
- **Precedence:** `ShellExecuteEx` reads **HKCU first, HKLM second**. Both hives
  are world-readable; no admin required.
- **WOW6432Node is not involved** — on Windows 7+ `App Paths` is shared across
  32/64-bit views. The `WOW6432Node\...\App Paths` tree should NOT be checked.
- **Values of interest:** the `(Default)` value holds the full path (may lack
  the `.exe` suffix — `ShellExecuteEx` appends it). The optional `Path` value
  holds a semicolon-separated directory list prepended to the child's `PATH`
  (often `REG_EXPAND_SZ` containing `%ProgramFiles%\...`).
- **Environment expansion is the caller's responsibility.** `winreg` reads
  `REG_EXPAND_SZ` verbatim. We must call `ExpandEnvironmentStringsW` (or use an
  equivalent expander) before we `stat` the path.
- **Notable gaps in App Paths coverage:**
  - **VS Code** does not register `Code.exe` (tracked since 2017 in
    microsoft/vscode#37807). User-scope install lives at
    `%LocalAppData%\Programs\Microsoft VS Code\`.
  - **MSIX / Store apps** (Windows Terminal, WSL, `winget`, Store Python)
    register as **App Execution Aliases** under
    `%LocalAppData%\Microsoft\WindowsApps\`. This directory is pre-prepended
    to the user's PATH, so the existing PATH scan already catches them — but
    they appear as zero-byte NT reparse points, which we must tolerate.
- **Secondary install roots worth a shallow walk:**
  `%ProgramFiles%`, `%ProgramFiles(x86)%`, `%LocalAppData%\Programs\`. Each can
  be walked one directory deep, looking for `<name>.exe` at the root of every
  first-level child.
- **Uninstall keys** (`...\CurrentVersion\Uninstall\*`) are a lower-value
  fallback: they're indexed by GUID, not by binary name, and many entries omit
  `InstallLocation`. `DisplayIcon` is often a usable binary path (stripped of
  a trailing `,0`), but scanning thousands of entries is expensive and the
  match quality is fuzzy. Out of scope for the first iteration.
- **Rust crate choice:** [`winreg`](https://docs.rs/winreg) is the pragmatic
  pick — safe, thin wrapper, handles UTF-16 and type discriminators, no extra
  `unsafe`. The already-present `windows` crate can read the registry via
  `Win32_System_Registry`, but the call sites are ~4× more verbose. Gate
  `winreg` on `#[cfg(target_os = "windows")]` so non-Windows builds are
  unaffected.

## Goals

1. Find every Windows-installed program that `start <name>` or **Win + R**
   would find.
2. Preserve PATH as the highest-priority source (matches `CreateProcess`
   semantics and keeps existing callers stable).
3. Match the existing `ExecutableIndex` "scan once, lookup many" shape so all
   eight program categories amortize the Windows scan cost.
4. Zero impact on non-Windows builds — all new code is `#[cfg(target_os = "windows")]`.
5. Thread-safe, panic-free, rewrite-friendly. Never `unwrap()` against the
   registry; absence is normal.

## Non-Goals

- **Uninstall-key scraping.** Possible future work; not needed to hit the
  90%+ coverage target from App Paths + install-root walk.
- **Display-name fuzzy matching.** `sniff` resolves by binary name. A
  "find-by-human-name" API is a separate concern.
- **MSIX package enumeration via `Windows.Management.Deployment.PackageManager`.**
  The `WindowsApps` alias dir already covers the invocation surface; full MSIX
  metadata would require WinRT bindings and is orthogonal to program
  detection.
- **Running on Windows < 10.** App Paths is shared-view only on Windows 7+,
  and sniff's support matrix is Windows 10/11.

## Detection Strategy

Five layers, executed in priority order, with results cached in a single
`WindowsAppIndex` that feeds `ExecutableIndex::build()`.

### Layer 1 — PATH (existing)

Unchanged. Scans every directory in `$env:PATH` for executable files, subject
to the existing `PATHEXT` filter. This already catches:

- Anything installed by `choco`, `scoop`, `winget` (their shim dirs are in
  PATH).
- `cargo`, `rustup`, `.dotnet\tools`, `npm`, `pnpm`, `yarn`, `bun`.
- The `%LocalAppData%\Microsoft\WindowsApps\` alias directory (Store apps).
- Anything the user explicitly added.

### Layer 2 — App Paths registry scan

Enumerate **both** hives and build a name → path map. HKCU entries take
precedence over HKLM entries of the same name (matching `ShellExecuteEx`).

Algorithm:

```rust
#[cfg(target_os = "windows")]
fn scan_app_paths() -> HashMap<String, PathBuf> {
    use winreg::RegKey;
    use winreg::enums::*;

    const APP_PATHS: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths";
    let mut map: HashMap<String, PathBuf> = HashMap::new();

    // Scan LOW-priority first (HKLM), then overwrite with HIGH-priority (HKCU).
    for hive in [HKEY_LOCAL_MACHINE, HKEY_CURRENT_USER] {
        let Ok(root) = RegKey::predef(hive).open_subkey_with_flags(APP_PATHS, KEY_READ) else {
            continue;
        };

        for subkey_name in root.enum_keys().flatten() {
            let Ok(subkey) = root.open_subkey_with_flags(&subkey_name, KEY_READ) else {
                continue;
            };

            // Default value = full path. Missing => skip.
            let raw: String = match subkey.get_value("") {
                Ok(v) => v,
                Err(_) => continue,
            };

            let expanded = expand_env_vars(&raw);
            let path = PathBuf::from(expanded);

            // Verify the target still exists — orphaned entries are common
            // after uninstall.
            if !path.is_file() {
                continue;
            }

            // Index under both the bare name and the .exe variant so lookups
            // for "chrome" and "chrome.exe" both succeed.
            let key_lower = subkey_name.to_ascii_lowercase();
            map.insert(key_lower.clone(), path.clone());
            if let Some(stem) = key_lower.strip_suffix(".exe") {
                map.insert(stem.to_string(), path);
            } else {
                // Subkey lacked .exe — index under the .exe form too.
                map.insert(format!("{key_lower}.exe"), path);
            }
        }
    }
    map
}
```

Notes:

- HKLM is scanned *first* and HKCU *second* precisely so that the HashMap
  `insert` overwrites HKLM entries with HKCU, matching `ShellExecuteEx` order.
- All subkey names are lowercased on insert to match the registry's
  case-insensitive semantics and sniff's lookup convention (lookups are
  case-sensitive as written today, so both the source and query are normalized
  to lowercase at the Windows-specific layer only — the generic layer is
  untouched).
- `path.is_file()` is a cheap syscall and filters orphaned entries.
- `open_subkey_with_flags(..., KEY_READ)` is unnecessary on most Windows
  builds but is explicit about read-only intent.

### Layer 3 — Install-root shallow walk

For every canonical install root we walk one directory deep. For each child
`<RootApp>\`, we index every `*.exe` found at the root of that child. This is
the fallback that covers VS Code (`%LocalAppData%\Programs\Microsoft VS Code\Code.exe`),
Cursor, Signal, Slack, Obsidian, most Electron apps, and every "Program Files\FooSoft\foo.exe"
traditional installer.

Roots (resolved from environment variables, skipped if unset):

| Variable | Typical value |
|---|---|
| `%ProgramFiles%` | `C:\Program Files` |
| `%ProgramFiles(x86)%` | `C:\Program Files (x86)` |
| `%LocalAppData%` → `...\Programs\` | `C:\Users\<user>\AppData\Local\Programs` |

Walk depth: exactly one level. We do **not** recurse into `<App>\bin\`,
`<App>\resources\`, or nested vendor folders — that would blow up cost and
produce false positives (many apps ship uninstallers, crash handlers, and
helper binaries alongside the main executable). Real-world verification:
~95% of user-installed Windows apps place the main launcher at the root of
their install directory.

```rust
#[cfg(target_os = "windows")]
fn scan_install_roots() -> HashMap<String, PathBuf> {
    let roots = install_root_dirs();
    let mut map = HashMap::new();

    for root in roots {
        let Ok(children) = std::fs::read_dir(&root) else { continue };
        for child in children.flatten() {
            let child_path = child.path();
            if !child_path.is_dir() { continue; }

            let Ok(entries) = std::fs::read_dir(&child_path) else { continue };
            for entry in entries.flatten() {
                let p = entry.path();
                if !p.is_file() { continue; }
                let Some(name) = p.file_name().and_then(|n| n.to_str()) else { continue };
                let lower = name.to_ascii_lowercase();
                if !lower.ends_with(".exe") { continue; }

                // First write wins (parent roots have priority).
                map.entry(lower.clone()).or_insert_with(|| p.clone());
                if let Some(stem) = lower.strip_suffix(".exe") {
                    map.entry(stem.to_string()).or_insert(p);
                }
            }
        }
    }
    map
}

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
```

Walk ordering matters because of the `or_insert_with` "first wins" rule — we
traverse `ProgramFiles` → `ProgramFiles(x86)` → `%LocalAppData%\Programs` so
system-wide installs beat user-scope ones for the same binary name. This also
matches how Windows itself ranks duplicate entries.

### Layer 4 — Non-rebuild of WindowsApps alias directory

The `%LocalAppData%\Microsoft\WindowsApps\` alias directory contains zero-byte
reparse points for Store apps. Because Windows itself injects it into the user
PATH, our existing PATH-scan pass picks it up — but currently
`is_executable()` calls `path.is_file()`, and NT reparse points report
successfully as files with size zero. We must verify that the existing
`is_executable()` path-scan branch actually includes these entries.

**Action item:** verify that `find_program::is_executable()` on Windows tolerates
the reparse points and does not reject them based on size. If it does, a small
tweak is needed to include them. No new layer is introduced — this is a
correctness check on Layer 1.

### Layer 5 — Uninstall keys (deferred / future work)

Not in this iteration. Design notes captured for future reference:

- Enumerate `HKLM\...\Uninstall\*`, `HKLM\WOW6432Node\...\Uninstall\*`,
  `HKCU\...\Uninstall\*`.
- For each subkey, read `DisplayName`, `InstallLocation`, `DisplayIcon`.
- Derive binary from `DisplayIcon` by stripping trailing `,<index>` and
  verifying existence.
- Use as a "find-by-display-name" API, not as part of binary lookup.

If we ever add a `sniff programs find "Adobe Photoshop"` command, this is
where it lives.

## Environment Variable Expansion

Win32 has a dedicated API, `ExpandEnvironmentStringsW`, that handles edge cases
the naive `shellexpand` crate misses (e.g. `%ProgramFiles(x86)%` contains
parentheses; `%SystemRoot%` is sometimes not in the process env but known to
the kernel). We should call the native API via the already-present `windows`
crate:

```rust
#[cfg(target_os = "windows")]
fn expand_env_vars(input: &str) -> String {
    use windows::core::{HSTRING, PWSTR};
    use windows::Win32::System::Environment::ExpandEnvironmentStringsW;

    let wide = HSTRING::from(input);
    // Query required buffer size first (including null terminator).
    let required = unsafe { ExpandEnvironmentStringsW(&wide, None) };
    if required == 0 {
        return input.to_string();
    }
    let mut buf = vec![0u16; required as usize];
    let written = unsafe {
        ExpandEnvironmentStringsW(&wide, Some(&mut buf))
    };
    if written == 0 {
        return input.to_string();
    }
    // Trim the null terminator.
    let slice = &buf[..(written as usize).saturating_sub(1)];
    String::from_utf16_lossy(slice)
}
```

The `windows` crate is already in `sniff/lib/Cargo.toml`; we only need to add
the `Win32_System_Environment` feature. This avoids pulling in a second
expansion crate.

## Type Additions

Extend `ExecutableSource` with two new variants:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutableSource {
    Path,
    MacOsAppBundle,
    WindowsAppPaths,     // NEW — found via registry App Paths key
    WindowsInstallRoot,  // NEW — found via one-level walk of Program Files / LocalAppData\Programs
}
```

Serialization strings: `"windows_app_paths"` and `"windows_install_root"`.
This is an additive change. Existing serialized data (the repo has tests
asserting the exact strings for `Path` and `MacOsAppBundle`) is unaffected.
The existing `CategoryDetector` deserializer already tolerates unknown keys
via `BoolOrEntry` + `IgnoredAny`, so older clients reading newer JSON degrade
gracefully (the program will still deserialize as installed, just without a
recognizable source string).

`ExecutableSource::is_app_bundle()` is kept returning `false` for both new
Windows variants — they are *not* app bundles in the macOS sense; they are
fallback-located PATH replacements. Callers that want "is this a non-PATH
source?" should use a new helper:

```rust
impl ExecutableSource {
    #[must_use]
    pub fn is_fallback(&self) -> bool {
        !matches!(self, Self::Path)
    }
}
```

Display impl:

```rust
impl std::fmt::Display for ExecutableSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Path => write!(f, "PATH"),
            Self::MacOsAppBundle => write!(f, "macOS App Bundle"),
            Self::WindowsAppPaths => write!(f, "Windows App Paths"),
            Self::WindowsInstallRoot => write!(f, "Windows Install Root"),
        }
    }
}
```

## Module Layout

New file: `sniff/lib/src/programs/windows_apps.rs`. Parallel to the existing
`macos_bundle.rs` module. Public surface:

```rust
//! Windows application discovery beyond PATH.
//!
//! Provides fallback lookups for Windows programs that are installed but not
//! in PATH, via the App Paths registry key and shallow walks of the standard
//! install-root directories.

use std::collections::HashMap;
use std::path::PathBuf;

/// Builds the Windows-specific fallback index.
///
/// Scans App Paths (HKLM + HKCU) and the canonical install-root directories
/// once, returning two HashMaps keyed by lowercased binary name. Both maps
/// include entries with and without the `.exe` suffix so that lookups for
/// `"chrome"` and `"chrome.exe"` both resolve.
#[cfg(target_os = "windows")]
pub(super) fn build_windows_index() -> WindowsIndex {
    WindowsIndex {
        app_paths: scan_app_paths(),
        install_roots: scan_install_roots(),
    }
}

#[cfg(target_os = "windows")]
#[derive(Debug, Default, Clone)]
pub(super) struct WindowsIndex {
    pub app_paths: HashMap<String, PathBuf>,
    pub install_roots: HashMap<String, PathBuf>,
}
```

On non-Windows builds the `WindowsIndex` type and scan functions are absent
entirely — `#[cfg(target_os = "windows")]` at the module-level in
`programs/mod.rs`:

```rust
#[cfg(target_os = "windows")]
pub mod windows_apps;
```

## ExecutableIndex Integration

`ExecutableIndex` gains a `#[cfg]`-gated field and a lookup extension:

```rust
#[derive(Debug, Clone)]
pub struct ExecutableIndex {
    path_executables: HashMap<String, PathBuf>,

    #[cfg(target_os = "macos")]
    bundle_executables: HashMap<String, PathBuf>,

    #[cfg(target_os = "windows")]
    windows_index: windows_apps::WindowsIndex,
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
        }
    }

    pub fn find_with_source(&self, program: &str) -> Option<(PathBuf, ExecutableSource)> {
        // Normalize once up-front for Windows — other platforms keep their
        // existing case semantics.
        #[cfg(target_os = "windows")]
        let program = program.to_ascii_lowercase();
        #[cfg(target_os = "windows")]
        let program = program.as_str();

        // Layer 1: PATH
        if let Some(p) = self.path_executables.get(program) {
            return Some((p.clone(), ExecutableSource::Path));
        }

        // Layer 2: macOS bundles
        #[cfg(target_os = "macos")]
        if let Some(p) = self.bundle_executables.get(program) {
            return Some((p.clone(), ExecutableSource::MacOsAppBundle));
        }

        // Layer 3: Windows App Paths (HKCU winner already baked in)
        #[cfg(target_os = "windows")]
        if let Some(p) = self.windows_index.app_paths.get(program) {
            return Some((p.clone(), ExecutableSource::WindowsAppPaths));
        }

        // Layer 4: Windows install-root walk
        #[cfg(target_os = "windows")]
        if let Some(p) = self.windows_index.install_roots.get(program) {
            return Some((p.clone(), ExecutableSource::WindowsInstallRoot));
        }

        None
    }
}
```

Ordering rationale: PATH first (matches `CreateProcess` and existing behavior),
then App Paths (the documented Shell API fallback), then install-root walk
(catches the VS Code-style gap). The walk runs *after* App Paths so that apps
that registered properly get the authoritative registry path, not a
potentially stale filesystem guess.

`find_program_with_source()` (the non-index entry point) gets the same
treatment but without the build-once optimization — it calls the underlying
helpers on every invocation. The helpers are written to be cheap enough that
this is acceptable for one-off lookups.

## Parallelism and Caching

- `build_windows_index()` is called once per `ExecutableIndex::build()`, from
  the existing single build thread. App Paths scan is effectively O(number of
  registered apps) — ~50-200 on a typical machine, ~10 ms serial.
- Install-root walk is O(number of first-level children across three roots)
  — ~100-500 dirs, each with a cheap `read_dir`, ~30-60 ms on a warm cache.
- **Total Windows-side cost: 40-80 ms** on a warm filesystem. This is well
  within the existing "build the index once, do O(1) lookups" budget.
- No host-capability cache integration is needed — `HostCapabilities` is
  persisted separately, and the Windows index rebuilds on every `sniff` run.
  If we want to persist it later, it slots naturally into the existing
  `HostCapabilityCacheFile`.

Threading: `RegKey` from `winreg` is `Send` but not `Sync`. All registry
handles are opened and closed inside `scan_app_paths()`, so the returned
`HashMap` is trivially movable between threads. No `Arc<Mutex<...>>` needed.

## Dependencies

Cargo.toml additions (Windows target only):

```toml
[target.'cfg(target_os = "windows")'.dependencies]
windows = { version = "0.62", features = [
    "Win32_Foundation",
    "Win32_System_Services",
    "Win32_System_Environment",  # NEW — for ExpandEnvironmentStringsW
] }
winreg = "0.55"                   # NEW — App Paths registry access
```

Both crates are widely used, zero-`unsafe` at the call sites we need, and add
no footprint for non-Windows builds. `winreg` 0.55 is the current release
line (as of April 2026); pin to a minor version per workspace convention.

## Testing Strategy

Test categories, each written to be safe to run on non-Windows hosts:

### Unit tests (runnable everywhere)

- `ExecutableSource` serialization roundtrip for the two new variants.
- `ExecutableSource::is_fallback()` truth table.
- `expand_env_vars()` happy paths (via a pure-Rust stub on non-Windows targets).

### Windows-only unit tests (`#[cfg(target_os = "windows")]`)

- `scan_app_paths()` returns a non-empty map on a CI Windows runner (smoke
  test; asserts that at least one well-known key — `chrome.exe` or
  `msedge.exe` — resolves).
- `scan_app_paths()` honors HKCU precedence. Test setup writes a fake entry
  to `HKCU\Software\Microsoft\Windows\CurrentVersion\App Paths\sniff_test.exe`
  pointing to a tempfile, confirms it appears in the map, then cleans up. The
  `serial_test` crate (already a workspace dep) serializes against other tests
  that touch the registry.
- `scan_install_roots()` on a synthetic root: `tempdir()` populated with
  `<dir>/MockApp/mock.exe` and a `ScopedEnv`-overridden `%ProgramFiles%`.
- `ExpandEnvironmentStringsW` roundtrip for `%SystemRoot%\System32`.

### Integration tests (`sniff/lib/tests/`)

- `windows_find_program_priority.rs`: creates a fixture with the same program
  present in both PATH and App Paths, confirms PATH wins.
- `windows_app_paths_orphan.rs`: writes an App Paths entry pointing to a
  nonexistent path, confirms it does not appear in the final map.

### CI coverage

- The existing CI matrix runs on `ubuntu-latest` and `macos-latest` only.
  Windows tests will initially compile-check via `cargo check --target
  x86_64-pc-windows-msvc` on the existing runners (using `cross` or `rustup
  target add`). A follow-up ticket adds a `windows-latest` job to the matrix.
- Guard any test that touches the real registry behind `#[ignore]` by default
  and a `RUN_WINDOWS_REGISTRY_TESTS=1` env flag to keep local developer runs
  side-effect-free.

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Orphaned App Paths entries after uninstall | High | Low | `path.is_file()` check post-expansion drops them silently. |
| `REG_EXPAND_SZ` with unknown env var | Medium | Low | `ExpandEnvironmentStringsW` leaves unknown vars untouched; `is_file()` filters the result. |
| Installer races during scan (rare) | Low | Low | Accept stale data for this run; next `sniff` rebuilds the index. |
| Install-root walk false positives (uninstallers, helpers) | Medium | Medium | Only index the root directory of each first-level child, not `bin/` or `resources/`. Document the "first-level walk" rule clearly. |
| `winreg` crate vulnerabilities or unmaintained status | Low | Medium | 0.55 is actively maintained; fallback path is to switch to `windows` crate direct calls, which is mechanical. |
| Long file names (NTFS `\\?\` paths) | Low | Low | `PathBuf` handles these transparently; we never parse paths ourselves. |
| Slow `read_dir` on pathological install trees | Low | Low | One-level walk caps the fan-out; a single pathological directory is at worst a few ms. |
| Cross-user visibility into HKCU | N/A | N/A | Running as user `A` only sees `A`'s HKCU by design — this is correct. |

## Open Questions

1. **Should the Windows index participate in `HostCapabilities` caching?**
   `HostCapabilities` persists across runs; adding the Windows index would let
   us skip the 40-80 ms scan on subsequent invocations. Deferred to a follow-up
   once we validate the baseline cost budget is acceptable.
2. **Do we want a configurable "install-root" list?** Some users install to
   `D:\Apps\` or similar. Environment variables do not cover this. A future
   `SniffConfig::windows_install_roots` field could extend the walk. Not
   required for v1.
3. **Should we also index `%ProgramData%` subdirectories?** Chocolatey and
   some MSI installers place binaries under `%ProgramData%\*\bin\`. These are
   typically picked up via PATH (`%ProgramData%\chocolatey\bin` in PATH), so
   the marginal value is low. Defer.
4. **`PATHEXT` beyond `.exe`?** The install-root walk only indexes `.exe`
   files. `.cmd`, `.bat`, `.ps1` launchers are possible but rare at
   install-root level, and PATH already covers shim-heavy install patterns.
   Revisit if a real-world miss surfaces.

## Rollout Plan

1. **Phase 1** — `ExecutableSource` extension (two new variants + `is_fallback()`).
   Pure additive change. Ship with tests.
2. **Phase 2** — `windows_apps.rs` module implementing `scan_app_paths()` and
   the env expansion helper. Unit-tested in isolation.
3. **Phase 3** — `scan_install_roots()` with tempdir tests.
4. **Phase 4** — `ExecutableIndex` integration and `find_program_with_source()`
   parity. Integration tests.
5. **Phase 5** — Documentation: update `sniff/lib/README.md`,
   `programs.md` skill doc, and `sniff/docs/sniff-library-architecture.md`.
6. **Phase 6** — CI: add `windows-latest` to the GitHub Actions matrix for
   the `sniff` package area.

Each phase is independently mergeable. Phases 1-5 do not require a Windows
development machine thanks to `#[cfg]`-gating and CI-side compile checks.

## References

- [Application Registration — Microsoft Learn](https://learn.microsoft.com/en-us/windows/win32/shell/app-registration)
- [Registry Keys Affected by WOW64 — Microsoft Learn](https://learn.microsoft.com/en-us/windows/win32/winprog64/shared-registry-keys)
- [CreateProcessW — Microsoft Learn](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-createprocessw)
- [ExpandEnvironmentStringsW — Microsoft Learn](https://learn.microsoft.com/en-us/windows/win32/api/processenv/nf-processenv-expandenvironmentstringsw)
- [winreg crate](https://docs.rs/winreg)
- [windows crate `Win32_System_Environment`](https://docs.rs/windows/latest/windows/Win32/System/Environment/index.html)
- [microsoft/vscode#37807 — Code.exe App Paths gap](https://github.com/microsoft/vscode/issues/37807)
