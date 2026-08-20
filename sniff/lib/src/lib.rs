use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Instant;
use tracing::Level;
use tracing::instrument;

pub mod error;
pub mod executable_index;
pub mod filesystem;
pub mod hardware;
pub mod network;
pub mod os;
pub mod package;
pub mod performance;
pub(crate) mod process;
pub mod programs;
#[cfg(feature = "remote")]
pub mod remote;
#[cfg(feature = "network")]
mod credentials;
pub mod request;
pub mod services;

#[cfg(test)]
mod test_helpers;

pub use error::{Result, SniffError};
pub use filesystem::{FilesystemInfo, FilesystemObservation, GitRepositoryIdentity};
pub use hardware::HardwareInfo;
pub use network::NetworkInfo;
pub use performance::PerformanceReport;
pub use programs::{ProgramMetadata, ProgramsInfo};
pub use request::DetectionPlan;

use request::{FilesystemRequest, GitRequest, HardwareRequest, NetworkRequest, OsRequest};

// Re-export key OS types from the os module for convenience.
// The canonical path is `sniff::os::*`.
pub use os::OsInfo;

/// Complete system detection result.
///
/// Contains OS, hardware, network, and filesystem information gathered
/// by the sniff library. All fields are optional to allow partial
/// detection when using flags. The `Default` impl yields an all-`None`
/// instance, which downstream callers can use as a graceful fallback when
/// a detection plan fails (e.g. `detect_with_plan(plan).unwrap_or_default()`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SniffResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os: Option<OsInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hardware: Option<HardwareInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<NetworkInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filesystem: Option<FilesystemInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub performance: Option<PerformanceReport>,
}

/// Planned detection output paired with its reusable filesystem observation.
///
/// The returned observation is the same shared owner supplied by the caller,
/// so later Git queries reuse its discovered repository handle.
#[derive(Debug)]
pub struct ObservedSniffResult {
    /// Detection output.
    pub result: SniffResult,
    /// Reusable filesystem observation.
    pub filesystem_observation: FilesystemObservation,
}

/// Configuration for the detect operation.
///
/// Use the builder pattern to customize detection behavior.
///
/// ## Examples
///
/// ```
/// use sniff::SniffConfig;
/// use std::path::PathBuf;
///
/// let config = SniffConfig::new()
///     .base_dir(PathBuf::from("/some/path"))
///     .skip_network();
/// ```
#[derive(Debug, Clone)]
pub struct SniffConfig {
    /// Base directory for filesystem analysis
    pub base_dir: Option<PathBuf>,
    /// Enable deep git inspection (network operations for remote info)
    pub deep: bool,
    /// Number of recent commits to retrieve (default: 10)
    pub commit_count: usize,
    /// Skip OS detection
    pub skip_os: bool,
    /// Skip hardware detection
    pub skip_hardware: bool,
    /// Skip network detection
    pub skip_network: bool,
    /// Skip filesystem detection
    pub skip_filesystem: bool,
    /// Include structured performance data in the result
    pub include_performance: bool,
}

impl Default for SniffConfig {
    fn default() -> Self {
        Self {
            base_dir: None,
            deep: false,
            commit_count: 10,
            skip_os: false,
            skip_hardware: false,
            skip_network: false,
            skip_filesystem: false,
            include_performance: false,
        }
    }
}

impl SniffConfig {
    /// Create a new configuration with defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the base directory for filesystem analysis.
    pub fn base_dir(mut self, path: PathBuf) -> Self {
        self.base_dir = Some(path);
        self
    }

    /// Enable deep git inspection (fetches remote branch info, checks if behind).
    pub fn deep(mut self, enable: bool) -> Self {
        self.deep = enable;
        self
    }

    /// Set the number of recent commits to retrieve (default: 10).
    pub fn commit_count(mut self, count: usize) -> Self {
        self.commit_count = count;
        self
    }

    /// Skip OS detection.
    pub fn skip_os(mut self) -> Self {
        self.skip_os = true;
        self
    }

    /// Skip hardware detection.
    pub fn skip_hardware(mut self) -> Self {
        self.skip_hardware = true;
        self
    }

    /// Skip network detection.
    pub fn skip_network(mut self) -> Self {
        self.skip_network = true;
        self
    }

    /// Skip filesystem detection.
    pub fn skip_filesystem(mut self) -> Self {
        self.skip_filesystem = true;
        self
    }

    /// Include structured performance data in the result.
    pub fn performance(mut self, enable: bool) -> Self {
        self.include_performance = enable;
        self
    }
}

impl From<SniffConfig> for DetectionPlan {
    fn from(config: SniffConfig) -> Self {
        let git_request = if config.deep {
            GitRequest::deep().commit_count(config.commit_count)
        } else {
            GitRequest::full().commit_count(config.commit_count)
        };

        DetectionPlan {
            base_dir: config.base_dir,
            os: if config.skip_os {
                None
            } else {
                Some(OsRequest::full())
            },
            hardware: if config.skip_hardware {
                None
            } else {
                Some(HardwareRequest::full())
            },
            network: if config.skip_network {
                None
            } else {
                Some(NetworkRequest::full())
            },
            filesystem: if config.skip_filesystem {
                None
            } else {
                Some(FilesystemRequest::new().git(git_request))
            },
            include_performance: config.include_performance,
        }
    }
}

/// Detect system information with default configuration.
///
/// This is a convenience function that calls `detect_with_config`
/// with default settings.
///
/// ## Examples
///
/// ```no_run
/// use sniff::detect;
///
/// let result = detect().unwrap();
/// if let Some(os) = result.os {
///     println!("OS: {}", os.name);
/// }
/// ```
pub fn detect() -> Result<SniffResult> {
    detect_with_config(SniffConfig::default())
}

/// Detect system information with custom configuration.
///
/// ## Examples
///
/// ```no_run
/// use sniff::{detect_with_config, SniffConfig};
/// use std::path::PathBuf;
///
/// let config = SniffConfig::new()
///     .base_dir(PathBuf::from("."))
///     .skip_network();
///
/// let result = detect_with_config(config).unwrap();
/// ```
pub fn detect_with_config(config: SniffConfig) -> Result<SniffResult> {
    detect_with_plan(DetectionPlan::from(config))
}

/// Detect system information according to a detailed plan.
///
/// This is the primary API for callers who need fine-grained control over
/// what gets detected. Use `detect()` for sensible defaults, or module-level
/// functions for expert manual composition.
///
/// ## Examples
///
/// ```no_run
/// use sniff::{detect_with_plan, request::*};
///
/// let plan = DetectionPlan::new()
///     .os(OsRequest::summary())
///     .hardware(HardwareRequest::summary())
///     .without_network()
///     .filesystem(
///         FilesystemRequest::new()
///             .git(GitRequest::summary())
///             .repo(RepoRequest::structure())
///             .without_docs()
///     );
///
/// let result = detect_with_plan(plan).unwrap();
/// ```
///
/// ## Notes
///
/// Performance-enabled plans reuse a collector already installed on the
/// calling thread. This lets a composed request include work performed before
/// or after detection in one report while preserving the standalone result
/// snapshot.
#[instrument(skip(plan), fields(
    os = plan.os.is_some(),
    hw = plan.hardware.is_some(),
    net = plan.network.is_some(),
    fs = plan.filesystem.is_some(),
    perf = plan.include_performance,
))]
pub fn detect_with_plan(plan: DetectionPlan) -> Result<SniffResult> {
    detect_with_plan_inner(plan, None)
}

/// Detect according to a plan using a pre-observed Git filesystem context.
///
/// When filesystem detection is requested, the plan's base directory must
/// match [`FilesystemObservation::observed_root`]. A plan without an explicit
/// base uses that observed root and therefore does not read the ambient current
/// directory. No additional Git discovery occurs.
pub fn detect_with_plan_and_filesystem_observation(
    plan: DetectionPlan,
    filesystem_observation: FilesystemObservation,
) -> Result<ObservedSniffResult> {
    let result = detect_with_plan_inner(plan, Some(&filesystem_observation))?;
    Ok(ObservedSniffResult {
        result,
        filesystem_observation,
    })
}

fn detect_with_plan_inner(
    plan: DetectionPlan,
    filesystem_observation: Option<&FilesystemObservation>,
) -> Result<SniffResult> {
    let started = Instant::now();
    let base = match (&plan.base_dir, filesystem_observation, plan.filesystem.is_some()) {
        (Some(base), Some(observation), true) if base != observation.observed_root() => {
            return Err(SniffError::SystemInfo {
                domain: "filesystem",
                message: format!(
                    "filesystem observation for '{}' cannot seed plan rooted at '{}'",
                    observation.observed_root().display(),
                    base.display()
                ),
            });
        }
        (Some(base), _, _) => base.clone(),
        (None, Some(observation), true) => observation.observed_root().to_path_buf(),
        (None, _, _) => std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
    };
    let collector = plan.include_performance.then(|| {
        performance::current_collector()
            .unwrap_or_else(performance::PerformanceCollector::new_shared)
    });

    let mut result = performance::with_current_collector(collector.clone(), || {
        let detect_os = |req: &OsRequest| {
            performance::with_current_collector(collector.clone(), || {
                let _span = tracing::info_span!("detect_os").entered();
                let started = Instant::now();
                let result = os::detect_os_with_request(req);
                performance::record_logged_stage("detect.os", started.elapsed(), Level::INFO);
                result
            })
        };
        let detect_hardware = |req: &HardwareRequest| {
            performance::with_current_collector(collector.clone(), || {
                let _span = tracing::info_span!("detect_hardware").entered();
                let started = Instant::now();
                let result = hardware::detect_hardware_with_request(req);
                performance::record_logged_stage("detect.hardware", started.elapsed(), Level::INFO);
                result
            })
        };
        let detect_network = |req: &NetworkRequest| {
            performance::with_current_collector(collector.clone(), || {
                let _span = tracing::info_span!("detect_network").entered();
                let started = Instant::now();
                let result = network::detect_network_with_request(req);
                performance::record_logged_stage("detect.network", started.elapsed(), Level::INFO);
                result
            })
        };
        let detect_filesystem = |req: &FilesystemRequest| {
            performance::with_current_collector(collector.clone(), || {
                let _span = tracing::info_span!("detect_filesystem").entered();
                let started = Instant::now();
                let result = match filesystem_observation {
                    Some(observation) => {
                        filesystem::detect_filesystem_with_observation(&base, req, observation)
                    }
                    None => filesystem::detect_filesystem_with_request(&base, req),
                };
                performance::record_logged_stage(
                    "detect.filesystem",
                    started.elapsed(),
                    Level::INFO,
                );
                result
            })
        };

        #[cfg(windows)]
        let (os, hardware, network, filesystem) = (
            plan.os.as_ref().map(detect_os).transpose(),
            plan.hardware.as_ref().map(detect_hardware).transpose(),
            plan.network.as_ref().map(detect_network).transpose(),
            plan.filesystem.as_ref().map(detect_filesystem).transpose(),
        );

        // Full Windows domains use overlapping host APIs that may fail-fast the
        // process when invoked concurrently. Other platforms retain parallel
        // detection because their system interfaces are safe to overlap.
        #[cfg(not(windows))]
        let (os, hardware, network, filesystem) = std::thread::scope(|s| {
            let os_handle = plan.os.as_ref().map(|req| {
                s.spawn(move || detect_os(req))
            });
            let hw_handle = plan.hardware.as_ref().map(|req| {
                s.spawn(move || detect_hardware(req))
            });
            let net_handle = plan.network.as_ref().map(|req| {
                s.spawn(move || detect_network(req))
            });
            let fs_handle = plan.filesystem.as_ref().map(|req| {
                s.spawn(move || detect_filesystem(req))
            });

            let os = os_handle.map(|h| h.join().unwrap()).transpose();
            let hardware = hw_handle.map(|h| h.join().unwrap()).transpose();
            let network = net_handle.map(|h| h.join().unwrap()).transpose();
            let filesystem = fs_handle.map(|h| h.join().unwrap()).transpose();

            (os, hardware, network, filesystem)
        });

        let result = SniffResult {
            os: os?,
            hardware: hardware?,
            network: network?,
            filesystem: filesystem?,
            performance: None,
        };
        performance::record_logged_stage("detect.total", started.elapsed(), Level::INFO);
        Ok::<_, SniffError>(result)
    })?;

    if let Some(collector) = collector {
        result.performance = Some(collector.snapshot(started.elapsed()));
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[serial_test::serial]
    fn test_detect_returns_result() {
        let original_dir = std::env::current_dir().unwrap();
        let temp_dir = tempfile::TempDir::new().unwrap();
        std::env::set_current_dir(temp_dir.path()).unwrap();
        let result = detect();
        std::env::set_current_dir(original_dir).unwrap();
        assert!(result.is_ok());
    }

    #[test]
    fn test_skip_hardware_returns_none() {
        // Skip filesystem + network too (not asserted here) to avoid a monorepo scan.
        let config = SniffConfig::new()
            .skip_hardware()
            .skip_filesystem()
            .skip_network();
        let result = detect_with_config(config).unwrap();
        assert!(result.hardware.is_none());
    }

    #[test]
    fn test_skip_network_returns_none() {
        let config = SniffConfig::new()
            .skip_network()
            .skip_filesystem()
            .skip_hardware();
        let result = detect_with_config(config).unwrap();
        assert!(result.network.is_none());
    }

    #[test]
    fn test_skip_filesystem_returns_none() {
        let config = SniffConfig::new()
            .skip_filesystem()
            .skip_network()
            .skip_hardware();
        let result = detect_with_config(config).unwrap();
        assert!(result.filesystem.is_none());
    }

    #[test]
    fn test_config_builder_pattern() {
        let config = SniffConfig::new()
            .base_dir(PathBuf::from("."))
            .deep(true)
            .skip_network();

        assert!(config.base_dir.is_some());
        assert!(config.deep);
        assert!(config.skip_network);
    }

    #[test]
    fn test_detect_with_base_dir() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        // Filesystem only: skip os/hardware/network, which this test does not assert.
        let plan = DetectionPlan::new()
            .base_dir(temp_dir.path().to_path_buf())
            .without_os()
            .without_hardware()
            .without_network();
        let result = detect_with_plan(plan).unwrap();
        assert!(result.filesystem.is_some());
    }

    #[test]
    fn test_detect_with_plan_reuses_filesystem_observation() {
        let dir = tempfile::tempdir().unwrap();
        git2::Repository::init(dir.path()).unwrap();
        let observation = FilesystemObservation::discover(dir.path());
        let plan = DetectionPlan::new()
            .without_os()
            .without_hardware()
            .without_network()
            .filesystem(
                FilesystemRequest::new()
                    .git(GitRequest::identity())
                    .without_repo()
                    .without_docs()
                    .without_formatting()
                    .without_file_inventory(),
            )
            .performance(true);

        let observed = detect_with_plan_and_filesystem_observation(plan, observation).unwrap();
        let filesystem = observed.result.filesystem.unwrap();
        let performance = observed.result.performance.unwrap();

        assert!(filesystem.git.is_some());
        assert!(
            observed
                .filesystem_observation
                .repository_identity()
                .unwrap()
                .is_some()
        );
        assert_eq!(
            performance
                .counters
                .get(performance::counters::GIT_DISCOVERIES)
                .copied()
                .unwrap_or(0),
            0
        );
        assert!(performance.stages.contains_key("detect.filesystem"));
    }

    // Regression test: OS should be skipped when skip_os is set
    // Bug: When using --filesystem flag, OS section was still displayed
    #[test]
    fn test_skip_os_returns_none() {
        // Skip filesystem too (not asserted here) to avoid a monorepo scan.
        let config = SniffConfig::new().skip_os().skip_filesystem();
        let result = detect_with_config(config).unwrap();
        assert!(result.os.is_none(), "OS should be None when skip_os is set");
    }

    // Regression test: OS should be present by default
    #[test]
    fn test_os_present_by_default() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let config = SniffConfig::new().base_dir(temp_dir.path().to_path_buf());
        let result = detect_with_config(config).unwrap();
        assert!(result.os.is_some(), "OS should be Some by default");
    }

    // Regression test: Combining skip_os with other sections should work correctly
    #[test]
    fn test_skip_os_with_filesystem_only() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let config = SniffConfig::new()
            .base_dir(temp_dir.path().to_path_buf())
            .skip_os()
            .skip_hardware()
            .skip_network();
        let result = detect_with_config(config).unwrap();
        assert!(result.os.is_none(), "OS should be None when skipped");
        assert!(
            result.hardware.is_none(),
            "Hardware should be None when skipped"
        );
        assert!(
            result.network.is_none(),
            "Network should be None when skipped"
        );
        assert!(
            result.filesystem.is_some(),
            "Filesystem should be Some when not skipped"
        );
    }

    // Regression test: Multiple skip flags including OS
    #[test]
    fn test_multiple_skip_flags_including_os() {
        let config = SniffConfig::new()
            .skip_os()
            .skip_hardware()
            .skip_network()
            .skip_filesystem();
        let result = detect_with_config(config).unwrap();
        assert!(result.os.is_none());
        assert!(result.hardware.is_none());
        assert!(result.network.is_none());
        assert!(result.filesystem.is_none());
    }

    #[test]
    fn test_detect_with_plan_skips_sections() {
        let plan = DetectionPlan::new()
            .without_os()
            .without_hardware()
            .without_network()
            .without_filesystem();
        let result = detect_with_plan(plan).unwrap();
        assert!(result.os.is_none());
        assert!(result.hardware.is_none());
        assert!(result.network.is_none());
        assert!(result.filesystem.is_none());
    }

    #[test]
    fn test_sniff_config_to_detection_plan() {
        let config = SniffConfig::new()
            .skip_os()
            .skip_network()
            .deep(true)
            .commit_count(5);
        let plan = DetectionPlan::from(config);
        assert!(plan.os.is_none());
        assert!(plan.hardware.is_some());
        assert!(plan.network.is_none());
        let fs = plan.filesystem.unwrap();
        let git = fs.git.unwrap();
        assert_eq!(git.commit_count, 5);
        assert!(git.refresh_remote_tracking);
    }
}
