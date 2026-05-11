//! Phase 6 / Test Coverage #5: `clip service status` reports stopped
//! when no clipper service is running.

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn clip_service_status_reports_stopped_with_empty_runtime_dir() {
    let runtime = tempfile::tempdir().expect("tempdir");
    let mut cmd = Command::cargo_bin("clip").expect("locate clip binary");
    cmd.args(["service", "status"])
        .env("CLIP_RUNTIME_DIR", runtime.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("not running"));
}
