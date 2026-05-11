//! Sanity checks for the bench `DetectionPlan` builders.
//!
//! These guards make sure future edits to `benches/support/plans.rs`
//! still match the claims their docstrings make about each preset.

#[path = "../benches/support/plans.rs"]
mod plans;

// The plans module imports `super::network_fixture`; provide a stub so
// the test crate can include it without pulling in wiremock/tokio.
mod network_fixture {
    pub fn ensure_ready() {}
}

use std::path::PathBuf;

use plans::{full_plan, minimal_plan, summary_plan};

#[test]
fn minimal_plan_disables_every_domain() {
    let plan = minimal_plan();
    assert!(plan.os.is_none(), "minimal_plan should skip OS");
    assert!(plan.hardware.is_none(), "minimal_plan should skip hardware");
    assert!(plan.network.is_none(), "minimal_plan should skip network");
    assert!(
        plan.filesystem.is_none(),
        "minimal_plan should skip filesystem"
    );
}

#[test]
fn summary_plan_is_cheap_but_not_empty() {
    let base = PathBuf::from("/tmp/sniff-bench-summary");
    let plan = summary_plan(base.clone());

    let os = plan.os.as_ref().expect("summary_plan should include os");
    assert!(!os.include_package_managers);
    assert!(!os.include_ntp_status);

    let hw = plan
        .hardware
        .as_ref()
        .expect("summary_plan should include hardware");
    assert!(!hw.include_storage);
    assert!(!hw.include_gpu);
    assert!(!hw.include_audio);

    let net = plan
        .network
        .as_ref()
        .expect("summary_plan should include network");
    assert!(!net.include_wan_ip, "summary should skip WAN IP");

    let fs = plan
        .filesystem
        .as_ref()
        .expect("summary_plan should include filesystem");
    assert!(fs.git.is_some(), "summary should still detect git");
    assert!(fs.repo.is_some(), "summary should still detect repo");
    assert!(!fs.include_file_inventory);
    assert!(!fs.include_formatting);
    assert!(!fs.include_docs);

    let git = fs.git.as_ref().unwrap();
    assert_eq!(git.commit_count, 0, "summary git should be commit-free");

    let repo = fs.repo.as_ref().unwrap();
    assert!(repo.structure_only, "summary repo should be structure-only");

    assert_eq!(plan.base_dir.as_deref(), Some(base.as_path()));
}

#[test]
fn full_plan_exercises_full_network_path() {
    let base = PathBuf::from("/tmp/sniff-bench-full");
    let plan = full_plan(base.clone());

    let net = plan
        .network
        .as_ref()
        .expect("full_plan should include network");
    assert!(
        net.include_wan_ip,
        "full_plan should exercise the WAN IP path, not interfaces_only"
    );

    // OS, hardware, and filesystem must run at their default full detail.
    assert!(plan.os.is_some());
    assert!(plan.hardware.is_some());
    assert!(plan.filesystem.is_some());
    assert_eq!(plan.base_dir.as_deref(), Some(base.as_path()));
}
