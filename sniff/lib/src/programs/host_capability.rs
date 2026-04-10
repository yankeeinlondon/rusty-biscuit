//! Host capability detection for the install-plan pipeline.
//!
//! See `sniff/features/2026-04-10-program-install-improvements/tech-design.md`
//! for the contract this module implements.

use std::collections::HashSet;
use std::process::{Command, Stdio};
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::os::{LinuxFamily, OsType, detect_linux_distro, detect_os_type};
use crate::programs::enums::{LanguagePackageManager, OsPackageManager};
use crate::programs::pkg_mngrs::{
    InstalledLanguagePackageManagers, InstalledOsPackageManagers,
};

const PROBE_TIMEOUT: Duration = Duration::from_secs(2);

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
    /// (added in a later task) for the cached path.
    pub fn detect() -> Self {
        let os_type = detect_os_type();
        let is_wsl = detect_is_wsl();
        let linux_family = detect_linux_distro().map(|d| d.family);
        let default_pm = default_os_package_manager_for(os_type, linux_family);

        Self {
            os_type,
            is_wsl,
            has_bash: detect_has_bash(),
            os_pkg_mgrs: InstalledOsPackageManagers::new(),
            lang_pkg_mgrs: InstalledLanguagePackageManagers::new(),
            can_sudo: detect_can_sudo(),
            default_os_package_manager: default_pm,
            verified_lang_pkg_mgrs: HashSet::new(),
            npm_global_prefix_writable: None,
            detected_at: Utc::now(),
        }
    }
}

fn detect_has_bash() -> bool {
    which::which("bash").is_ok()
}

// ---------------------------------------------------------------------------
// Verification probes
// ---------------------------------------------------------------------------

/// Runs a command with a short timeout and returns its stdout on success.
fn run_probe(program: &str, args: &[&str]) -> Option<String> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => {
                use std::io::Read;
                let mut out = String::new();
                if let Some(mut stdout) = child.stdout.take() {
                    let _ = stdout.read_to_string(&mut out);
                }
                return Some(out);
            }
            Ok(Some(_)) => return None,
            Ok(None) => {}
            Err(_) => return None,
        }
        if start.elapsed() >= PROBE_TIMEOUT {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
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
    stdout
        .lines()
        .any(|line| line.trim_end().ends_with(':'))
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
    let mut verified = HashSet::new();

    if lang_pkg_mgrs.is_installed(LanguagePackageManager::Npm) && probe_npm_verified() {
        verified.insert(LanguagePackageManager::Npm);
    }
    if lang_pkg_mgrs.is_installed(LanguagePackageManager::Pnpm) && probe_pnpm_verified() {
        verified.insert(LanguagePackageManager::Pnpm);
    }
    if lang_pkg_mgrs.is_installed(LanguagePackageManager::Yarn) && probe_yarn_verified() {
        verified.insert(LanguagePackageManager::Yarn);
    }
    if lang_pkg_mgrs.is_installed(LanguagePackageManager::Bun) && probe_bun_verified() {
        verified.insert(LanguagePackageManager::Bun);
    }
    if lang_pkg_mgrs.is_installed(LanguagePackageManager::Cargo) && probe_cargo_verified() {
        verified.insert(LanguagePackageManager::Cargo);
    }

    verified
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
    if let Ok(version) = std::fs::read_to_string("/proc/version") {
        let lower = version.to_lowercase();
        if lower.contains("microsoft") || lower.contains("wsl") {
            return true;
        }
    }
    if let Ok(osrelease) = std::fs::read_to_string("/proc/sys/kernel/osrelease") {
        let lower = osrelease.to_lowercase();
        if lower.contains("microsoft") || lower.contains("wsl") {
            return true;
        }
    }
    false
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
    use std::process::Command;

    // `id -Gn` prints space-separated group names for the current user.
    let Ok(output) = Command::new("id").arg("-Gn").output() else {
        return false;
    };
    if !output.status.success() {
        return false;
    }
    let groups = String::from_utf8_lossy(&output.stdout);
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
    use std::process::Command;

    let Ok(output) = Command::new("sudo").args(["-n", "true"]).output() else {
        return false;
    };
    output.status.success()
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
}
