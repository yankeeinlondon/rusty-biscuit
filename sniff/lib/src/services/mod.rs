//! System service detection and enumeration.
//!
//! This module provides functionality for detecting the init system (systemd, launchd,
//! runit, etc.) and listing system services with their running state.
//!
//! ## Supported Init Systems
//!
//! - **systemd** (Linux): Most common on modern Linux distributions
//! - **launchd** (macOS): Apple's service management framework
//! - **OpenRC**: Used by Gentoo, Alpine, and others
//! - **runit**: Lightweight init used by Void Linux and others
//! - **Windows SCM**: Windows Service Control Manager
//!
//! ## Examples
//!
//! ```no_run
//! use sniff::services::{ServiceManager, ServiceState};
//!
//! let manager = ServiceManager::detect();
//! println!("Init system: {}", manager.init_system);
//!
//! // List all running services
//! let services = manager.services_detailed(ServiceState::Running);
//! for svc in services {
//!     println!("{}: PID {:?}", svc.name, svc.pid);
//! }
//! ```

use serde::{Deserialize, Serialize};
use std::{
    env, fmt, fs,
    path::{Path, PathBuf},
};
use tracing::{debug, instrument};

mod launchd;
mod openrc;
mod runit;
mod systemd;
mod windows_scm;

/// Units per enrichment subprocess when a backend queries many services at once.
///
/// This bounds command-line length, nothing else — a backend must not treat it as
/// a limit on how many services it reports. Sized well under the smallest
/// `ARG_MAX` sniff targets even with long unit names.
pub(crate) const ENRICHMENT_CHUNK: usize = 128;

// =============================================================================
// Host OS Detection
// =============================================================================

/// Host operating system type for service detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HostOs {
    Linux,
    Macos,
    Windows,
    Other,
}

impl fmt::Display for HostOs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HostOs::Linux => write!(f, "Linux"),
            HostOs::Macos => write!(f, "macOS"),
            HostOs::Windows => write!(f, "Windows"),
            HostOs::Other => write!(f, "Other"),
        }
    }
}

/// Detect the current host OS.
pub fn detect_host_os() -> HostOs {
    #[cfg(target_os = "linux")]
    {
        HostOs::Linux
    }
    #[cfg(target_os = "macos")]
    {
        HostOs::Macos
    }
    #[cfg(target_os = "windows")]
    {
        HostOs::Windows
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        HostOs::Other
    }
}

// =============================================================================
// Init System Detection
// =============================================================================

/// Init system / service manager type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InitSystem {
    // Linux init/service managers
    Systemd,
    Runit,
    S6,
    Dinit,
    BusyboxInit,
    /// Container minimal init (tini/dumb-init as PID 1)
    ContainerMinimalInit,
    OpenRc,
    Upstart,

    // Non-Linux "init-like" managers
    /// macOS launchd
    Launchd,
    /// Windows Service Control Manager
    WindowsScm,

    Unknown,
}

impl fmt::Display for InitSystem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            InitSystem::Systemd => "systemd",
            InitSystem::Runit => "runit",
            InitSystem::S6 => "s6",
            InitSystem::Dinit => "dinit",
            InitSystem::BusyboxInit => "busybox-init",
            InitSystem::ContainerMinimalInit => "container-minimal-init",
            InitSystem::OpenRc => "openrc",
            InitSystem::Upstart => "upstart",
            InitSystem::Launchd => "launchd",
            InitSystem::WindowsScm => "windows-scm",
            InitSystem::Unknown => "unknown",
        };
        f.write_str(s)
    }
}

/// Evidence gathered during init system detection.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InitEvidence {
    /// PID 1 process "comm" (best-effort). Usually available on Linux.
    pub pid1_comm: Option<String>,
    /// Best-effort resolved path to /proc/1/exe (Linux).
    pub pid1_exe: Option<PathBuf>,
    /// High-signal facts used in classification.
    pub hints: Vec<String>,
    /// Extra context for caller logs/UI.
    pub notes: Vec<String>,
}

impl InitEvidence {
    fn new() -> Self {
        Self::default()
    }

    fn hint(&mut self, msg: impl Into<String>) {
        self.hints.push(msg.into());
    }

    fn note(&mut self, msg: impl Into<String>) {
        self.notes.push(msg.into());
    }
}

/// Result of init system detection with evidence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectInitResult {
    pub init: InitSystem,
    pub evidence: InitEvidence,
}

/// Detect the init system for the given OS.
pub fn detect_init(os: HostOs) -> InitSystem {
    detect_init_with_evidence(os).init
}

/// Detect the init system with evidence for debugging.
pub fn detect_init_with_evidence(os: HostOs) -> DetectInitResult {
    match os {
        HostOs::Linux => detect_init_linux_with_evidence(),
        HostOs::Macos => {
            let mut ev = InitEvidence::new();
            ev.note(
                "On macOS host systems, PID 1 is launchd (init system is not typically variable).",
            );
            DetectInitResult {
                init: InitSystem::Launchd,
                evidence: ev,
            }
        }
        HostOs::Windows => {
            let mut ev = InitEvidence::new();
            ev.note("Windows does not have a Unix-like PID 1 init; service management is provided by the Service Control Manager (SCM).");
            DetectInitResult {
                init: InitSystem::WindowsScm,
                evidence: ev,
            }
        }
        HostOs::Other => DetectInitResult {
            init: InitSystem::Unknown,
            evidence: InitEvidence::new(),
        },
    }
}

fn detect_init_linux_with_evidence() -> DetectInitResult {
    let mut ev = InitEvidence::new();

    // --- 1) PID 1 truth (best signal) ---
    let comm = read_trimmed("/proc/1/comm");
    if let Some(ref c) = comm {
        ev.pid1_comm = Some(c.clone());
        ev.hint(format!("Read /proc/1/comm = {c:?}"));
    } else {
        ev.hint("Failed to read /proc/1/comm".to_string());
    }

    let exe = fs::read_link("/proc/1/exe")
        .map_err(|e| {
            debug!(error = %e, path = "/proc/1/exe", "could not read service file");
            e
        })
        .ok()
        .and_then(|p| canonicalish(p).ok());

    if let Some(ref p) = exe {
        ev.pid1_exe = Some(p.clone());
        ev.hint(format!("Read /proc/1/exe -> {:?}", p));
    } else {
        ev.hint("Failed to read /proc/1/exe".to_string());
    }

    let comm_s = comm.as_deref().unwrap_or("");
    let exe_path = exe.as_deref().unwrap_or_else(|| Path::new(""));

    // Helper: match on PID 1 comm / exe
    let classified = if comm_s == "systemd" || path_ends_with_component(exe_path, "systemd") {
        ev.hint("Matched systemd via PID 1 comm/exe".to_string());
        Some(InitSystem::Systemd)
    } else if comm_s == "runit-init" || path_ends_with_component(exe_path, "runit-init") {
        ev.hint("Matched runit via PID 1 comm/exe".to_string());
        Some(InitSystem::Runit)
    } else if comm_s == "s6-svscan" || path_ends_with_component(exe_path, "s6-svscan") {
        ev.hint("Matched s6 via PID 1 comm/exe".to_string());
        Some(InitSystem::S6)
    } else if comm_s == "dinit" || path_ends_with_component(exe_path, "dinit") {
        ev.hint("Matched dinit via PID 1 comm/exe".to_string());
        Some(InitSystem::Dinit)
    } else if path_ends_with_component(exe_path, "busybox")
        || (comm_s == "init" && exe_path.to_string_lossy().contains("busybox"))
    {
        ev.hint("Matched busybox init via PID 1 comm/exe".to_string());
        Some(InitSystem::BusyboxInit)
    } else if comm_s == "tini"
        || comm_s == "dumb-init"
        || path_ends_with_component(exe_path, "tini")
        || path_ends_with_component(exe_path, "dumb-init")
    {
        ev.hint("Matched container-minimal-init (tini/dumb-init) via PID 1 comm/exe".to_string());
        Some(InitSystem::ContainerMinimalInit)
    } else {
        None
    };

    if let Some(init) = classified {
        return DetectInitResult { init, evidence: ev };
    }

    // --- 2) Runtime hints (weaker signals) ---
    if Path::new("/run/systemd/system").is_dir() {
        ev.hint("Found /run/systemd/system directory".to_string());
        return DetectInitResult {
            init: InitSystem::Systemd,
            evidence: ev,
        };
    }

    // Note: This is *just* a hint; tools can be installed without being used as init.
    if has_in_path("rc-status") {
        ev.hint("Found rc-status in PATH (OpenRC tooling present)".to_string());
        return DetectInitResult {
            init: InitSystem::OpenRc,
            evidence: ev,
        };
    }

    if has_in_path("initctl") {
        ev.hint("Found initctl in PATH (Upstart tooling present)".to_string());
        return DetectInitResult {
            init: InitSystem::Upstart,
            evidence: ev,
        };
    }

    ev.note("No decisive match from PID 1 comm/exe or runtime hints.");
    DetectInitResult {
        init: InitSystem::Unknown,
        evidence: ev,
    }
}

// =============================================================================
// Service Types
// =============================================================================

/// Filter for listing services by state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ServiceState {
    #[default]
    All,
    Initializing,
    Running,
    Stopped,
}

impl ServiceState {
    /// Check if a service with the given state matches this filter.
    fn matches(&self, running: Option<bool>) -> bool {
        match self {
            ServiceState::All => true,
            ServiceState::Running => running == Some(true),
            ServiceState::Stopped => running == Some(false),
            ServiceState::Initializing => running.is_none(),
        }
    }
}

/// Information about a system service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Service {
    /// Service name/label
    pub name: String,
    /// Process ID if running
    pub pid: Option<u32>,
    /// Whether the service is currently running
    pub running: bool,
    /// Exit status or last status code (init-system specific)
    pub status: Option<i32>,
}

// =============================================================================
// Service Manager
// =============================================================================

/// Aggregated service manager information for a host.
///
/// This struct holds the detected init system and evidence used for detection,
/// providing a unified view of the host's service management infrastructure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceManager {
    /// The detected init system
    pub init_system: InitSystem,
    /// Evidence gathered during detection (for debugging/logging)
    pub evidence: InitEvidence,
    /// The host OS used for detection
    pub host_os: HostOs,
}

impl ServiceManager {
    /// Detect the service manager for the current host.
    #[instrument(skip_all)]
    pub fn detect() -> Self {
        let host_os = detect_host_os();
        let result = detect_init_with_evidence(host_os);
        Self {
            init_system: result.init,
            evidence: result.evidence,
            host_os,
        }
    }

    /// Get services filtered by state, returning just service names.
    ///
    /// This is a convenience wrapper around `services_detailed()`.
    pub fn services(&self, state: ServiceState) -> Vec<String> {
        self.services_detailed(state)
            .into_iter()
            .map(|s| s.name)
            .collect()
    }

    /// Get detailed service information filtered by state.
    ///
    /// Returns a list of `Service` structs with name, PID, running status, etc.
    /// The implementation varies by init system:
    /// - **launchd** (macOS): Parses `launchctl list`
    /// - **systemd** (Linux): Parses `systemctl list-units --type=service`
    /// - **OpenRC**: Parses `rc-status`
    /// - **runit**: Scans `/var/service/` or `SVDIR`
    /// - **Windows SCM**: Enumerates services via `EnumServicesStatusExW`.
    ///   On Windows the `status` field holds the raw SCM current-state code.
    /// - **Others**: Returns empty list (not yet implemented)
    pub fn services_detailed(&self, state: ServiceState) -> Vec<Service> {
        let all_services = match self.init_system {
            InitSystem::Launchd => launchd::list_launchd_services(),
            InitSystem::Systemd => systemd::list_systemd_services(),
            InitSystem::OpenRc => openrc::list_openrc_services(),
            InitSystem::Runit => runit::list_runit_services(),
            InitSystem::WindowsScm => windows_scm::list_windows_scm_services(),
            _ => Vec::new(),
        };

        all_services
            .into_iter()
            .filter(|s| state.matches(Some(s.running)))
            .collect()
    }
}

impl Default for ServiceManager {
    fn default() -> Self {
        Self::detect()
    }
}

// =============================================================================
// Convenience Functions
// =============================================================================

/// Get service names for the current host.
pub fn host_services(state: ServiceState) -> Vec<String> {
    ServiceManager::detect().services(state)
}

/// Get detailed service information for the current host.
pub fn host_services_detailed(state: ServiceState) -> Vec<Service> {
    ServiceManager::detect().services_detailed(state)
}

/// Detect services and return a complete `ServicesInfo` structure.
#[instrument(skip_all)]
pub fn detect_services() -> ServicesInfo {
    let manager = ServiceManager::detect();
    let services = manager.services_detailed(ServiceState::All);
    ServicesInfo {
        init_system: manager.init_system,
        host_os: manager.host_os,
        evidence: manager.evidence,
        services,
    }
}

/// Complete services information for serialization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServicesInfo {
    /// The detected init system
    pub init_system: InitSystem,
    /// The host OS
    pub host_os: HostOs,
    /// Evidence gathered during detection
    pub evidence: InitEvidence,
    /// List of all services
    pub services: Vec<Service>,
}

// =============================================================================
// Helper Functions
// =============================================================================

fn read_trimmed(path: &str) -> Option<String> {
    fs::read_to_string(path)
        .map_err(|e| {
            debug!(path = %path, error = %e, "could not read service file");
            e
        })
        .ok()
        .map(|s| s.trim().to_string())
}

/// Like canonicalize(), but doesn't fail just because resolution is restricted.
/// If canonicalize fails, returns the original path.
fn canonicalish(p: PathBuf) -> std::io::Result<PathBuf> {
    match p.canonicalize() {
        Ok(c) => Ok(c),
        Err(_) => Ok(p),
    }
}

fn path_ends_with_component(path: &Path, component: &str) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|n| n == component)
        .unwrap_or(false)
}

/// Cross-platform `command -v` style check.
/// On Windows, respects PATHEXT (e.g., .EXE, .BAT, .CMD).
fn has_in_path(exe_name: &str) -> bool {
    let path_var = match env::var_os("PATH") {
        Some(p) => p,
        None => return false,
    };

    #[cfg(windows)]
    {
        let pathext = env::var_os("PATHEXT").unwrap_or(".EXE;.BAT;.CMD;.COM".into());
        let exts: Vec<String> = pathext
            .to_string_lossy()
            .split(';')
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.trim().to_ascii_lowercase())
            .collect();

        let name_has_ext = Path::new(exe_name).extension().is_some();

        for dir in env::split_paths(&path_var) {
            if name_has_ext {
                if dir.join(exe_name).is_file() {
                    return true;
                }
            } else {
                for ext in &exts {
                    let cand = dir.join(format!("{exe_name}{ext}"));
                    if cand.is_file() {
                        return true;
                    }
                }
            }
        }
        false
    }

    #[cfg(not(windows))]
    {
        env::split_paths(&path_var).any(|dir| dir.join(exe_name).is_file())
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_host_os_detection() {
        let os = detect_host_os();
        #[cfg(target_os = "macos")]
        assert_eq!(os, HostOs::Macos);
        #[cfg(target_os = "linux")]
        assert_eq!(os, HostOs::Linux);
        #[cfg(target_os = "windows")]
        assert_eq!(os, HostOs::Windows);
    }

    #[test]
    fn test_init_system_display() {
        assert_eq!(InitSystem::Systemd.to_string(), "systemd");
        assert_eq!(InitSystem::Launchd.to_string(), "launchd");
        assert_eq!(InitSystem::WindowsScm.to_string(), "windows-scm");
        assert_eq!(InitSystem::Runit.to_string(), "runit");
        assert_eq!(InitSystem::S6.to_string(), "s6");
        assert_eq!(InitSystem::Dinit.to_string(), "dinit");
        assert_eq!(InitSystem::BusyboxInit.to_string(), "busybox-init");
        assert_eq!(
            InitSystem::ContainerMinimalInit.to_string(),
            "container-minimal-init"
        );
        assert_eq!(InitSystem::OpenRc.to_string(), "openrc");
        assert_eq!(InitSystem::Upstart.to_string(), "upstart");
        assert_eq!(InitSystem::Unknown.to_string(), "unknown");
    }

    #[test]
    fn test_detect_init_macos() {
        let result = detect_init_with_evidence(HostOs::Macos);
        assert_eq!(result.init, InitSystem::Launchd);
        assert!(!result.evidence.notes.is_empty());
    }

    #[test]
    fn test_detect_init_windows() {
        let result = detect_init_with_evidence(HostOs::Windows);
        assert_eq!(result.init, InitSystem::WindowsScm);
        assert!(!result.evidence.notes.is_empty());
    }

    #[test]
    fn test_detect_init_other() {
        let result = detect_init_with_evidence(HostOs::Other);
        assert_eq!(result.init, InitSystem::Unknown);
    }

    #[test]
    fn test_detect_init_returns_valid_system() {
        let init = detect_init(detect_host_os());
        // Just verify it returns something (platform-dependent)
        let _ = init.to_string();
    }

    #[test]
    fn test_service_manager_detect() {
        let sm = ServiceManager::detect();
        // Verify all fields are populated
        let _ = sm.init_system.to_string();
        let _ = &sm.evidence;
        let _ = sm.host_os;
    }

    #[test]
    fn test_service_manager_default() {
        let sm = ServiceManager::default();
        // Should be same as detect()
        let sm2 = ServiceManager::detect();
        assert_eq!(sm.init_system, sm2.init_system);
        assert_eq!(sm.host_os, sm2.host_os);
    }

    #[test]
    fn test_service_manager_on_current_platform() {
        let sm = ServiceManager::detect();

        #[cfg(target_os = "macos")]
        {
            assert_eq!(sm.host_os, HostOs::Macos);
            assert_eq!(sm.init_system, InitSystem::Launchd);
        }

        #[cfg(target_os = "windows")]
        {
            assert_eq!(sm.host_os, HostOs::Windows);
            assert_eq!(sm.init_system, InitSystem::WindowsScm);
        }

        // Linux is more complex - could be various init systems
        #[cfg(target_os = "linux")]
        {
            assert_eq!(sm.host_os, HostOs::Linux);
            // Just verify it's a valid init system
            let _ = sm.init_system.to_string();
        }
    }

    #[test]
    fn test_services_does_not_panic() {
        let sm = ServiceManager::detect();
        // Should not panic regardless of init system
        let _ = sm.services(ServiceState::All);
        let _ = sm.services(ServiceState::Running);
        let _ = sm.services(ServiceState::Stopped);
        let _ = sm.services(ServiceState::Initializing);
    }

    #[test]
    fn test_services_detailed_does_not_panic() {
        let sm = ServiceManager::detect();
        let services = sm.services_detailed(ServiceState::All);
        // Verify Service struct fields are accessible
        for service in services {
            let _ = &service.name;
            let _ = service.pid;
            let _ = service.running;
            let _ = service.status;
        }
    }

    #[test]
    fn test_host_services_function() {
        // Should not panic
        let services = host_services(ServiceState::All);
        let _ = services;
    }

    #[test]
    fn test_host_services_detailed_function() {
        // Should not panic
        let services = host_services_detailed(ServiceState::All);
        let _ = services;
    }

    #[test]
    fn test_detect_services_function() {
        let info = detect_services();
        let _ = info.init_system.to_string();
        let _ = info.host_os.to_string();
        let _ = &info.evidence;
        let _ = &info.services;
    }

    #[test]
    fn test_service_state_matches() {
        // All matches everything
        assert!(ServiceState::All.matches(Some(true)));
        assert!(ServiceState::All.matches(Some(false)));
        assert!(ServiceState::All.matches(None));

        // Running only matches Some(true)
        assert!(ServiceState::Running.matches(Some(true)));
        assert!(!ServiceState::Running.matches(Some(false)));
        assert!(!ServiceState::Running.matches(None));

        // Stopped only matches Some(false)
        assert!(!ServiceState::Stopped.matches(Some(true)));
        assert!(ServiceState::Stopped.matches(Some(false)));
        assert!(!ServiceState::Stopped.matches(None));

        // Initializing only matches None
        assert!(!ServiceState::Initializing.matches(Some(true)));
        assert!(!ServiceState::Initializing.matches(Some(false)));
        assert!(ServiceState::Initializing.matches(None));
    }

    #[test]
    fn test_service_struct() {
        let service = Service {
            name: "test-service".to_string(),
            pid: Some(1234),
            running: true,
            status: Some(0),
        };
        assert_eq!(service.name, "test-service");
        assert_eq!(service.pid, Some(1234));
        assert!(service.running);
        assert_eq!(service.status, Some(0));
    }

    #[test]
    fn test_services_info_serialization() {
        let info = detect_services();
        let json = serde_json::to_string(&info).expect("should serialize");
        let _: ServicesInfo = serde_json::from_str(&json).expect("should deserialize");
    }
}
