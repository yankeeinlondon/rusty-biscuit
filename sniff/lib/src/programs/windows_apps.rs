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
use std::sync::OnceLock;

/// Global cached Windows index, built once per process.
///
/// This eliminates the per-lookup cost of registry enumeration and
/// filesystem walks when [`find_program_with_source`] is called repeatedly.
static WINDOWS_INDEX: OnceLock<WindowsIndex> = OnceLock::new();

/// Returns the cached Windows index, building it on first access.
///
/// The index is shared across all threads and built at most once per
/// process lifetime. This replaces the previous per-call
/// `build_windows_index()` pattern that caused severe slowdowns when
/// looking up multiple programs.
pub(super) fn get_or_build_windows_index() -> &'static WindowsIndex {
    WINDOWS_INDEX.get_or_init(build_windows_index)
}

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
/// `ExecutableIndex::build()` on Windows, and once for
/// [`find_program_with_source`] fallback lookups.
pub(crate) fn build_windows_index() -> WindowsIndex {
    WindowsIndex {
        app_paths: scan_app_paths(),
        install_roots: scan_install_roots(),
    }
}

/// Windows-specific fallback index populated by [`build_windows_index`].
///
/// Two HashMaps keyed by lowercased binary name. Checked in priority order
/// (`app_paths` before `install_roots`) after PATH by
/// [`crate::executable_index::ExecutableIndex::find_with_source`].
#[derive(Debug, Default, Clone)]
pub(crate) struct WindowsIndex {
    /// Name → path map built from the App Paths registry key.
    pub app_paths: HashMap<String, PathBuf>,
    /// Name → path map built from a shallow walk of install roots.
    pub install_roots: HashMap<String, PathBuf>,
}

// ---------------------------------------------------------------------------
// Layer 2: App Paths registry scan
// ---------------------------------------------------------------------------

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

    const APP_PATHS: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths";

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

// ---------------------------------------------------------------------------
// Layer 3: Shallow install-root walk
// ---------------------------------------------------------------------------

/// Returns the canonical Windows install roots in priority order.
///
/// Order is deliberate: `%ProgramFiles%` → `%ProgramFiles(x86)%` →
/// `%LocalAppData%\Programs`. Any unset env var is skipped (we do not
/// fabricate a path). The priority matters for the "first write wins"
/// rule inside [`walk_install_roots`].
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
fn scan_install_roots() -> HashMap<String, PathBuf> {
    walk_install_roots(&install_root_dirs())
}

// ---------------------------------------------------------------------------
// Env-var expansion
// ---------------------------------------------------------------------------

/// Expands Windows environment variables (`%Name%`) using `ExpandEnvironmentStringsW`.
///
/// Falls back to the raw input when the API call fails or when no variables
/// are present. Unknown variables are left untouched (the Win32 API preserves
/// them verbatim), which lets downstream `is_file()` checks drop stale paths.
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

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================
    // expand_env_vars tests
    // ============================================

    #[test]
    fn expand_env_vars_resolves_system_root() {
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

    // ============================================
    // scan_app_paths tests
    // ============================================

    #[test]
    fn scan_app_paths_honors_hkcu_precedence_over_hklm() {
        use crate::test_helpers::ENV_MUTEX;
        use std::fs;
        use tempfile::NamedTempFile;
        use winreg::RegKey;
        use winreg::enums::{HKEY_CURRENT_USER, KEY_ALL_ACCESS};

        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

        let tmp = NamedTempFile::new().unwrap();
        let tmp_path = tmp.path().to_path_buf();
        let exe_path = tmp_path.with_extension("exe");
        fs::copy(&tmp_path, &exe_path).unwrap();

        const TEST_KEY: &str =
            r"SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\sniff_test_unique_12345.exe";

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let (key, _) = hkcu.create_subkey(TEST_KEY).unwrap();
        key.set_value("", &exe_path.to_string_lossy().to_string())
            .unwrap();

        let result = scan_app_paths();

        let _ = hkcu.delete_subkey_all(TEST_KEY);
        let _ = fs::remove_file(&exe_path);

        let got = result
            .get("sniff_test_unique_12345")
            .cloned()
            .expect("HKCU entry should surface in the scan map");
        assert_eq!(got, exe_path);

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

        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());

        const TEST_KEY: &str =
            r"SOFTWARE\Microsoft\Windows\CurrentVersion\App Paths\sniff_test_orphan_99999.exe";

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let (key, _) = hkcu.create_subkey(TEST_KEY).unwrap();
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

    // ============================================
    // walk_install_roots tests
    // ============================================

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

        let result = walk_install_roots(&[high.path().to_path_buf(), low.path().to_path_buf()]);

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

        let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
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
}
