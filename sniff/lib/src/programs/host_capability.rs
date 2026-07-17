//! Host capability detection for the install-plan pipeline.
//!
//! See `sniff/features/2026-04-10-program-install-improvements/tech-design.md`
//! for the contract this module implements.

use std::collections::HashSet;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::executable_index::ExecutableIndex;
use crate::os::{LinuxFamily, OsType, detect_linux_distro, detect_os_type};
use crate::performance;
use crate::process::{self, timeouts};
use crate::programs::categories::{InstalledLanguagePackageManagers, InstalledOsPackageManagers};
use crate::programs::enums::{LanguagePackageManager, OsPackageManager};

const PROBE_TIMEOUT: Duration = timeouts::HOST_CAPABILITY;

/// Shared input to `build_install_plan`.
///
/// All fields are injectable so tests can fabricate arbitrary hosts without
/// touching the real machine. `HostCapabilities::default()` returns a
/// "nothing detected" host on the current platform.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostCapabilities {
    pub os_type: OsType,
    pub is_wsl: bool,
    pub has_bash: bool,
    pub os_pkg_mgrs: InstalledOsPackageManagers,
    pub lang_pkg_mgrs: InstalledLanguagePackageManagers,
    pub can_sudo: bool,
    pub default_os_package_manager: Option<OsPackageManager>,
    pub verified_lang_pkg_mgrs: HashSet<LanguagePackageManager>,
    pub npm_global_prefix_writable: Option<bool>,
    pub detected_at: DateTime<Utc>,
}

impl Default for HostCapabilities {
    fn default() -> Self {
        Self {
            os_type: OsType::Other,
            is_wsl: false,
            has_bash: false,
            os_pkg_mgrs: InstalledOsPackageManagers::default(),
            lang_pkg_mgrs: InstalledLanguagePackageManagers::default(),
            can_sudo: false,
            default_os_package_manager: None,
            verified_lang_pkg_mgrs: HashSet::new(),
            npm_global_prefix_writable: None,
            detected_at: Utc::now(),
        }
    }
}

impl HostCapabilities {
    /// Detect cheap host facts (no verification probes).
    ///
    /// Does not touch disk cache; call [`HostCapabilities::load_or_detect`]
    /// for the cached path.
    pub fn detect() -> Self {
        // Eager, PATH-only: the detection below asks the index for ~40 program
        // names, and the lazy index walks PATH once per name. The `_only`
        // variant keeps the lookup surface identical to the lazy index's — the
        // bundle/Windows fallback layers must not start resolving package
        // managers that are absent from PATH.
        let index = ExecutableIndex::build_eager_path_only();
        Self::detect_with_index(&index)
    }

    /// Detect cheap host facts using a pre-built [`ExecutableIndex`].
    ///
    /// Reuses the supplied index for OS- and language-package-manager scans so
    /// PATH is walked only once across the whole detection. Use this when the
    /// caller has already built (or will reuse) an index for other detection
    /// work (e.g. the `messenger info` cold path).
    pub fn detect_with_index(index: &ExecutableIndex) -> Self {
        let os_type = detect_os_type();
        let is_wsl = detect_is_wsl();
        let linux_family = detect_linux_distro().map(|d| d.family);
        let default_pm = default_os_package_manager_for(os_type, linux_family);

        Self {
            os_type,
            is_wsl,
            has_bash: index.find("bash").is_some(),
            os_pkg_mgrs: InstalledOsPackageManagers::new_with_index(index),
            lang_pkg_mgrs: InstalledLanguagePackageManagers::new_with_index(index),
            can_sudo: detect_can_sudo(),
            default_os_package_manager: default_pm,
            verified_lang_pkg_mgrs: HashSet::new(),
            npm_global_prefix_writable: None,
            detected_at: Utc::now(),
        }
    }
}

// ---------------------------------------------------------------------------
// Verification probes
// ---------------------------------------------------------------------------

/// Runs a command with a short timeout and returns its stdout on success.
fn run_probe(program: &str, args: &[&str]) -> Option<String> {
    process::run_for_stdout(program, args, PROBE_TIMEOUT)
}

fn parse_npm_global_list(stdout: &str) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(stdout) else {
        return false;
    };
    value
        .get("dependencies")
        .and_then(|d| d.as_object())
        .is_some_and(|obj| !obj.is_empty())
}

fn parse_cargo_install_list(stdout: &str) -> bool {
    // `cargo install --list` prints one crate header per line ending in ':'.
    stdout.lines().any(|line| line.trim_end().ends_with(':'))
}

fn probe_npm_verified() -> bool {
    run_probe("npm", &["ls", "-g", "--depth=0", "--json"])
        .as_deref()
        .map(parse_npm_global_list)
        .unwrap_or(false)
}

fn probe_pnpm_verified() -> bool {
    run_probe("pnpm", &["ls", "-g", "--depth=0", "--json"])
        .as_deref()
        .map(parse_npm_global_list) // same shape
        .unwrap_or(false)
}

fn probe_bun_verified() -> bool {
    run_probe("bun", &["pm", "ls", "-g"])
        .as_deref()
        .map(|s| s.lines().any(|l| !l.trim().is_empty()))
        .unwrap_or(false)
}

fn probe_yarn_verified() -> bool {
    run_probe("yarn", &["global", "list", "--json"])
        .as_deref()
        .map(|s| s.lines().any(|l| l.trim().starts_with('{')))
        .unwrap_or(false)
}

fn probe_cargo_verified() -> bool {
    run_probe("cargo", &["install", "--list"])
        .as_deref()
        .map(parse_cargo_install_list)
        .unwrap_or(false)
}

fn detect_verified_lang_pkg_mgrs(
    lang_pkg_mgrs: &InstalledLanguagePackageManagers,
) -> HashSet<LanguagePackageManager> {
    use rayon::prelude::*;

    type ProbeFn = fn() -> bool;
    let candidates: [(LanguagePackageManager, ProbeFn); 5] = [
        (LanguagePackageManager::Npm, probe_npm_verified),
        (LanguagePackageManager::Pnpm, probe_pnpm_verified),
        (LanguagePackageManager::Yarn, probe_yarn_verified),
        (LanguagePackageManager::Bun, probe_bun_verified),
        (LanguagePackageManager::Cargo, probe_cargo_verified),
    ];

    // Each probe has its own timeout and spends most of its time waiting on
    // a child process, so parallelism here collapses worst-case latency from
    // `5 × PROBE_TIMEOUT` to roughly `1 × PROBE_TIMEOUT`.
    let collector = performance::current_collector();
    candidates
        .par_iter()
        .filter_map(|(pm, probe)| {
            let _worker = performance::pooled_worker(collector.as_ref());
            if lang_pkg_mgrs.is_installed(*pm) && probe() {
                Some(*pm)
            } else {
                None
            }
        })
        .collect()
}

fn detect_npm_global_prefix_writable() -> Option<bool> {
    let prefix = run_probe("npm", &["prefix", "-g"])?;
    let path = std::path::Path::new(prefix.trim());
    if !path.exists() {
        return Some(false);
    }
    let marker = path.join(".sniff-writable-check");
    match std::fs::File::create(&marker) {
        Ok(_) => {
            let _ = std::fs::remove_file(&marker);
            Some(true)
        }
        Err(_) => Some(false),
    }
}

impl HostCapabilities {
    /// Detect host facts plus verification probes.
    ///
    /// This runs global-list commands for each installed language package
    /// manager and checks whether the npm global prefix is user-writable. Each
    /// probe has a 2-second timeout and its failure mode is "unverified", not
    /// fatal. Call the cheaper [`HostCapabilities::detect`] when you don't
    /// need these extra signals.
    pub fn detect_with_verification() -> Self {
        let mut host = Self::detect();
        host.verified_lang_pkg_mgrs = detect_verified_lang_pkg_mgrs(&host.lang_pkg_mgrs);
        host.npm_global_prefix_writable = detect_npm_global_prefix_writable();
        host
    }
}

// ---------------------------------------------------------------------------
// On-disk capability cache
// ---------------------------------------------------------------------------

/// Current on-disk schema version for the host capability cache.
///
/// Increment this constant whenever the `HostCapabilityCacheFile` structure
/// changes in a backward-incompatible way. A version mismatch causes
/// [`load_host_capabilities_from`] to discard the old file and return `None`.
pub const CACHE_SCHEMA_VERSION: u32 = 1;

/// 90-day TTL for cached host capabilities.
const CACHE_TTL: chrono::Duration = chrono::Duration::days(90);

/// On-disk envelope wrapping a [`HostCapabilities`] snapshot.
///
/// The envelope stores metadata needed to validate the cache on next load:
/// schema version for forward-compatibility gating, hostname for
/// cross-machine invalidation (future use), OS summary fields for quick
/// inspection, and an `expires_at` timestamp computed from `detected_at +
/// CACHE_TTL`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostCapabilityCacheFile {
    pub schema_version: u32,
    pub hostname: String,
    pub os: OsType,
    pub is_wsl: bool,
    pub expires_at: DateTime<Utc>,
    pub capabilities: HostCapabilities,
}

/// Returns the default cache path: `~/.sniff-programs.json`.
///
/// Returns `None` when the home directory cannot be resolved — callers should
/// skip caching and run a live detection in that case.
pub fn default_cache_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".sniff-programs.json"))
}

/// Loads capabilities from a cache file if the cache is fresh, the schema
/// version matches, and the recorded hostname matches the current host.
///
/// Returns `None` on cache miss, stale entry, corrupt JSON, schema drift, or
/// hostname mismatch (e.g. a synced home directory moved between machines).
/// This function never panics.
pub fn load_host_capabilities_from(path: &Path) -> Option<HostCapabilities> {
    let bytes = std::fs::read(path).ok()?;
    let envelope: HostCapabilityCacheFile = serde_json::from_slice(&bytes).ok()?;
    if envelope.schema_version != CACHE_SCHEMA_VERSION {
        return None;
    }
    if envelope.expires_at < Utc::now() {
        return None;
    }
    // A synced home directory (e.g. iCloud/Dropbox) may carry the cache file
    // across machines. Trusting it blindly leads to plans built against the
    // wrong package managers, so we invalidate on any mismatch.
    let current_hostname = sysinfo::System::host_name().unwrap_or_default();
    if envelope.hostname != current_hostname {
        return None;
    }
    Some(envelope.capabilities)
}

/// Saves capabilities atomically to `path` using a write-then-rename strategy.
///
/// The file is written to a `.json.tmp` sibling, fsynced, then renamed over
/// the target so readers never observe a partial write. On Unix, the final
/// file is chmod'd to 0o600 to prevent other users from reading host metadata.
///
/// ## Errors
///
/// Returns an `io::Error` if the parent directory cannot be created, the temp
/// file cannot be written, or the rename fails.
pub fn save_host_capabilities_to(path: &Path, host: &HostCapabilities) -> std::io::Result<()> {
    let envelope = HostCapabilityCacheFile {
        schema_version: CACHE_SCHEMA_VERSION,
        hostname: sysinfo::System::host_name().unwrap_or_default(),
        os: host.os_type,
        is_wsl: host.is_wsl,
        expires_at: host.detected_at + CACHE_TTL,
        capabilities: host.clone(),
    };

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let tmp = path.with_extension("json.tmp");
    {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(
            &serde_json::to_vec_pretty(&envelope)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?,
        )?;
        f.sync_all()?;
    }
    std::fs::rename(&tmp, path)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    }

    Ok(())
}

impl HostCapabilities {
    /// Returns cached capabilities if fresh, otherwise detects and writes a
    /// new cache file.
    ///
    /// Errors writing the cache are silently ignored so callers always get a
    /// usable result even when the home directory is read-only.
    pub fn load_or_detect() -> Self {
        let path = default_cache_path();
        if let Some(ref p) = path
            && let Some(host) = load_host_capabilities_from(p)
        {
            performance::increment_counter("programs.host_capability.cache_hits", 1);
            return host;
        }
        performance::increment_counter("programs.host_capability.cache_misses", 1);
        let host = Self::detect();
        if let Some(ref p) = path {
            let _ = save_host_capabilities_to(p, &host);
        }
        host
    }

    /// As [`HostCapabilities::load_or_detect`], but reuses the supplied
    /// [`ExecutableIndex`] on a cache miss so PATH is scanned only once across
    /// the surrounding detection work.
    pub fn load_or_detect_with_index(index: &ExecutableIndex) -> Self {
        let path = default_cache_path();
        if let Some(ref p) = path
            && let Some(host) = load_host_capabilities_from(p)
        {
            performance::increment_counter("programs.host_capability.cache_hits", 1);
            return host;
        }
        performance::increment_counter("programs.host_capability.cache_misses", 1);
        let host = Self::detect_with_index(index);
        if let Some(ref p) = path {
            let _ = save_host_capabilities_to(p, &host);
        }
        host
    }

    /// As [`HostCapabilities::load_or_detect`], but uses
    /// [`HostCapabilities::detect_with_verification`] on a cache miss.
    ///
    /// When `force_refresh` is `true`, the cache is bypassed entirely and a
    /// fresh detection is written back to disk.
    pub fn load_or_detect_with_verification(force_refresh: bool) -> Self {
        let path = default_cache_path();
        if force_refresh {
            performance::increment_counter("programs.host_capability.cache_forced_refreshes", 1);
        }
        if !force_refresh
            && let Some(ref p) = path
            && let Some(host) = load_host_capabilities_from(p)
        {
            performance::increment_counter("programs.host_capability.cache_hits", 1);
            return host;
        }
        performance::increment_counter("programs.host_capability.cache_misses", 1);
        let host = Self::detect_with_verification();
        if let Some(ref p) = path {
            let _ = save_host_capabilities_to(p, &host);
        }
        host
    }
}

// ---------------------------------------------------------------------------
// Default OS package manager
// ---------------------------------------------------------------------------

/// Returns the OS package manager that should be considered the "default" for
/// the given host. Linux delegates to the distro family; non-Linux uses the
/// hard-coded canonical manager.
///
/// Returns `None` when the OS has no known default (e.g. BSDs, unknown Linux
/// family) — the plan builder falls through to the alternative-OS-PM bucket.
pub fn default_os_package_manager_for(
    os: OsType,
    linux_family: Option<LinuxFamily>,
) -> Option<OsPackageManager> {
    match os {
        OsType::MacOS => Some(OsPackageManager::Brew),
        OsType::Windows => Some(OsPackageManager::Winget),
        OsType::Linux => match linux_family {
            Some(LinuxFamily::Debian) => Some(OsPackageManager::Apt),
            Some(LinuxFamily::RedHat) => Some(OsPackageManager::Dnf),
            Some(LinuxFamily::Arch) => Some(OsPackageManager::Pacman),
            Some(LinuxFamily::SUSE) => None, // zypper not modelled yet
            Some(LinuxFamily::NixOS) => Some(OsPackageManager::Nix),
            _ => None,
        },
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// WSL detection
// ---------------------------------------------------------------------------

fn detect_is_wsl() -> bool {
    if !cfg!(target_os = "linux") {
        return false;
    }
    let version = std::fs::read_to_string("/proc/version").unwrap_or_default();
    let osrelease = std::fs::read_to_string("/proc/sys/kernel/osrelease").unwrap_or_default();
    proc_markers_indicate_wsl(&version, &osrelease)
}

/// Pure decision over the WSL marker files' contents.
///
/// Empty strings stand in for a missing or unreadable `/proc/version` or
/// `/proc/sys/kernel/osrelease`, so an absent `/proc` yields `false` — the
/// non-WSL (native Linux) fallback.
fn proc_markers_indicate_wsl(version: &str, osrelease: &str) -> bool {
    [version, osrelease].iter().any(|contents| {
        let lower = contents.to_lowercase();
        lower.contains("microsoft") || lower.contains("wsl")
    })
}

// ---------------------------------------------------------------------------
// Sudo detection
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default)]
struct SudoProbes {
    group_membership: bool,
    sudo_n_true: bool,
}

/// Pure decision function over the probe results.
///
/// First positive signal wins; if none fire, returns `false`.
fn decide_can_sudo(probes: &SudoProbes) -> bool {
    probes.group_membership || probes.sudo_n_true
}

#[cfg(unix)]
fn probe_group_membership() -> bool {
    // `id -Gn` prints space-separated group names for the current user.
    let Some(groups) = run_probe("id", &["-Gn"]) else {
        return false;
    };
    groups
        .split_whitespace()
        .any(|g| matches!(g, "wheel" | "sudo" | "admin"))
}

#[cfg(not(unix))]
fn probe_group_membership() -> bool {
    false
}

#[cfg(unix)]
fn probe_sudo_n_true() -> bool {
    // `-n` makes sudo fail rather than prompt, and the helper's null stdin means a
    // sudo that ignores `-n` still cannot block on a terminal read.
    run_probe("sudo", &["-n", "true"]).is_some()
}

#[cfg(not(unix))]
fn probe_sudo_n_true() -> bool {
    false
}

fn detect_can_sudo() -> bool {
    // On native Windows we never claim sudo; WSL is detected as Linux and
    // goes through the Unix probes.
    if cfg!(all(windows, not(target_env = "gnu"))) {
        return false;
    }
    decide_can_sudo(&SudoProbes {
        group_membership: probe_group_membership(),
        sudo_n_true: probe_sudo_n_true(),
    })
}

#[cfg(test)]
mod sudo_tests {
    use super::*;

    #[test]
    fn group_membership_wins() {
        assert!(decide_can_sudo(&SudoProbes {
            group_membership: true,
            sudo_n_true: false,
        }));
    }

    #[test]
    fn sudo_n_true_wins() {
        assert!(decide_can_sudo(&SudoProbes {
            group_membership: false,
            sudo_n_true: true,
        }));
    }

    #[test]
    fn no_signals_returns_false() {
        assert!(!decide_can_sudo(&SudoProbes::default()));
    }
}

#[cfg(test)]
mod default_pm_tests {
    use super::*;
    use crate::os::LinuxFamily;

    #[test]
    fn debian_maps_to_apt() {
        assert_eq!(
            default_os_package_manager_for(OsType::Linux, Some(LinuxFamily::Debian)),
            Some(OsPackageManager::Apt)
        );
    }

    #[test]
    fn redhat_maps_to_dnf() {
        assert_eq!(
            default_os_package_manager_for(OsType::Linux, Some(LinuxFamily::RedHat)),
            Some(OsPackageManager::Dnf)
        );
    }

    #[test]
    fn arch_maps_to_pacman() {
        assert_eq!(
            default_os_package_manager_for(OsType::Linux, Some(LinuxFamily::Arch)),
            Some(OsPackageManager::Pacman)
        );
    }

    #[test]
    fn macos_maps_to_brew() {
        assert_eq!(
            default_os_package_manager_for(OsType::MacOS, None),
            Some(OsPackageManager::Brew)
        );
    }

    #[test]
    fn windows_maps_to_winget() {
        assert_eq!(
            default_os_package_manager_for(OsType::Windows, None),
            Some(OsPackageManager::Winget)
        );
    }

    #[test]
    fn unknown_linux_family_returns_none() {
        assert_eq!(
            default_os_package_manager_for(OsType::Linux, Some(LinuxFamily::Other)),
            None
        );
    }

    #[test]
    fn linux_without_family_returns_none() {
        assert_eq!(default_os_package_manager_for(OsType::Linux, None), None);
    }
}

#[cfg(test)]
mod verification_tests {
    use super::*;

    #[test]
    fn parse_npm_global_list_finds_entries() {
        let json = r#"{"dependencies":{"typescript":{"version":"5.0.0"}}}"#;
        assert!(parse_npm_global_list(json));
    }

    #[test]
    fn parse_npm_global_list_handles_empty() {
        let json = r#"{"dependencies":{}}"#;
        assert!(!parse_npm_global_list(json));
    }

    #[test]
    fn parse_npm_global_list_handles_malformed() {
        assert!(!parse_npm_global_list("not json"));
    }

    #[test]
    fn parse_cargo_install_list_finds_entries() {
        let output = "bat v0.24.0:\n    bat\nripgrep v14.1.0:\n    rg\n";
        assert!(parse_cargo_install_list(output));
    }

    #[test]
    fn parse_cargo_install_list_handles_empty() {
        assert!(!parse_cargo_install_list(""));
        assert!(!parse_cargo_install_list("\n\n"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_reports_no_sudo_and_no_default_pm() {
        let host = HostCapabilities::default();
        assert!(!host.can_sudo);
        assert!(host.default_os_package_manager.is_none());
        assert!(host.verified_lang_pkg_mgrs.is_empty());
        assert!(host.npm_global_prefix_writable.is_none());
    }

    #[test]
    fn detect_returns_current_os_type() {
        let host = HostCapabilities::detect();
        assert_eq!(host.os_type, detect_os_type());
    }

    #[test]
    fn detect_records_timestamp_near_now() {
        let before = Utc::now();
        let host = HostCapabilities::detect();
        let after = Utc::now();
        assert!(host.detected_at >= before);
        assert!(host.detected_at <= after);
    }

    /// `detect()` asks the index for every OS and language package-manager
    /// name; a lazy index would walk PATH once per name.
    #[test]
    fn detect_traverses_path_through_the_shared_eager_index() {
        HostCapabilities::detect();
        assert!(
            crate::executable_index::eager_path_cache_is_populated(),
            "detect() must build an eager PATH index so PATH is traversed once"
        );
    }

    /// Routing `detect()` through the eager builder changes work, not results.
    #[test]
    fn detect_agrees_with_a_lazy_path_only_index() {
        let lazy = HostCapabilities::detect_with_index(&ExecutableIndex::build_path_only());
        let host = HostCapabilities::detect();

        assert_eq!(host.has_bash, lazy.has_bash);
        assert_eq!(host.os_pkg_mgrs, lazy.os_pkg_mgrs);
        assert_eq!(host.lang_pkg_mgrs, lazy.lang_pkg_mgrs);
        assert_eq!(
            host.default_os_package_manager,
            lazy.default_os_package_manager
        );
    }

    #[test]
    fn proc_markers_indicate_wsl_matches_microsoft_and_wsl_tokens() {
        // Representative WSL2 /proc/version and a WSL osrelease.
        assert!(proc_markers_indicate_wsl(
            "Linux version 5.15.0-microsoft-standard-WSL2 (oe-user@oe-host)",
            ""
        ));
        assert!(proc_markers_indicate_wsl("", "5.15.90.1-microsoft-standard-WSL2"));
        // Marker can live in either file; matching is case-insensitive.
        assert!(proc_markers_indicate_wsl("WSL", ""));
    }

    #[test]
    fn proc_markers_indicate_wsl_false_for_native_linux_or_missing_proc() {
        // A stock Linux kernel string carries no WSL marker.
        assert!(!proc_markers_indicate_wsl(
            "Linux version 6.8.0-generic (builder@host)",
            "6.8.0-generic"
        ));
        // Empty strings stand in for missing /proc files -> native Linux fallback.
        assert!(!proc_markers_indicate_wsl("", ""));
    }
}

#[cfg(test)]
mod cache_tests {
    use super::*;
    use tempfile::tempdir;

    fn write_envelope(path: &Path, envelope: &HostCapabilityCacheFile) {
        let bytes = serde_json::to_vec(envelope).unwrap();
        std::fs::write(path, bytes).unwrap();
    }

    fn sample_envelope(hostname: &str) -> HostCapabilityCacheFile {
        let host = HostCapabilities::default();
        HostCapabilityCacheFile {
            schema_version: CACHE_SCHEMA_VERSION,
            hostname: hostname.to_string(),
            os: host.os_type,
            is_wsl: host.is_wsl,
            expires_at: host.detected_at + CACHE_TTL,
            capabilities: host,
        }
    }

    #[test]
    fn load_returns_capabilities_when_hostname_matches() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("cache.json");
        let current = sysinfo::System::host_name().unwrap_or_default();
        write_envelope(&path, &sample_envelope(&current));
        let loaded = load_host_capabilities_from(&path);
        assert!(
            loaded.is_some(),
            "expected cache hit with matching hostname"
        );
    }

    #[test]
    fn load_returns_none_when_hostname_differs() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("cache.json");
        // Use a hostname that the current machine will never match.
        let envelope = sample_envelope("this-hostname-should-never-match-ever-xyz123");
        write_envelope(&path, &envelope);
        let loaded = load_host_capabilities_from(&path);
        assert!(loaded.is_none(), "expected cache miss on hostname mismatch");
    }

    #[test]
    fn save_then_load_round_trips() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("cache.json");
        let host = HostCapabilities::default();
        save_host_capabilities_to(&path, &host).unwrap();
        let loaded = load_host_capabilities_from(&path);
        assert!(
            loaded.is_some(),
            "expected cache written by save_host_capabilities_to to load back"
        );
    }
}
