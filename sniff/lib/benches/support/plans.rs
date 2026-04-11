//! Stable `DetectionPlan` builders for benches.
//!
//! These helpers exist so the names and request levels exercised by the
//! benchmark suite stay consistent across runs even when the underlying
//! default presets evolve.

use sniff::DetectionPlan;
use sniff::request::{
    FilesystemRequest, GitRequest, HardwareRequest, NetworkRequest, OsRequest, RepoRequest,
};
use std::path::PathBuf;

/// Bare-minimum plan: every domain skipped.
///
/// Used to measure pure orchestration overhead inside
/// `detect_with_plan`.
pub fn minimal_plan() -> DetectionPlan {
    DetectionPlan::new()
        .without_os()
        .without_hardware()
        .without_network()
        .without_filesystem()
}

/// Cheap "summary" plan across every domain.
///
/// - OS: core identity only (no package managers, no NTP)
/// - Hardware: CPU + memory only (no storage/GPU/audio)
/// - Network: local interfaces only (no WAN IP lookup)
/// - Filesystem: git summary + repo structure, no inventory/docs/formatting
pub fn summary_plan(base_dir: PathBuf) -> DetectionPlan {
    DetectionPlan::new()
        .base_dir(base_dir)
        .os(OsRequest::summary())
        .hardware(HardwareRequest::summary())
        .network(NetworkRequest::interfaces_only())
        .filesystem(
            FilesystemRequest::new()
                .git(GitRequest::summary())
                .repo(RepoRequest::structure())
                .without_docs()
                .without_formatting()
                .without_file_inventory(),
        )
}

/// Full detection plan, anchored at `base_dir`.
///
/// Deliberately pins network to `interfaces_only()` so benchmarks are
/// not dominated by the external HTTP latency of the WAN IP lookup. OS
/// detection still runs at full detail (NTP, locale, package managers)
/// because that is the realistic "full" cost callers pay.
pub fn full_plan(base_dir: PathBuf) -> DetectionPlan {
    DetectionPlan::new()
        .base_dir(base_dir)
        .network(NetworkRequest::interfaces_only())
}
