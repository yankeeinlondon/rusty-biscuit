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

use super::network_fixture;

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
/// Exercises the real full-network path, including the WAN IP lookup.
/// To keep bench wall-clock off public internet latency, the bench
/// process installs a wiremock-backed endpoint via
/// [`network_fixture::ensure_ready`] before this plan runs — callers
/// must invoke that helper first (the top-level `register_all` in
/// `perf.rs` does). OS detection runs at full detail (NTP, locale,
/// package managers) because that is the realistic cost callers pay.
pub fn full_plan(base_dir: PathBuf) -> DetectionPlan {
    // Cheap idempotent guard: if a direct caller forgot to prime the
    // fixture we still point WAN IP detection at wiremock rather than
    // the public internet.
    network_fixture::ensure_ready();
    DetectionPlan::new()
        .base_dir(base_dir)
        .network(NetworkRequest::full())
}
