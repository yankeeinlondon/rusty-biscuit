//! Integration tests for the HostCapabilityCache file format and TTL logic.
//!
//! Uses a tempdir via the injectable cache-path entry point so no real home
//! directory is touched.

use std::path::PathBuf;

use chrono::{Duration, Utc};
use sniff::programs::host_capability::{
    HostCapabilities, load_host_capabilities_from, save_host_capabilities_to,
    CACHE_SCHEMA_VERSION,
};

fn tmp_cache_path() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join(".sniff-programs.json");
    (dir, path)
}

#[test]
fn cache_miss_returns_none() {
    let (_dir, path) = tmp_cache_path();
    assert!(load_host_capabilities_from(&path).is_none());
}

#[test]
fn round_trip_preserves_capabilities() {
    let (_dir, path) = tmp_cache_path();
    let host = HostCapabilities::default();
    save_host_capabilities_to(&path, &host).unwrap();
    let loaded = load_host_capabilities_from(&path).expect("cache hit");
    assert_eq!(loaded.os_type, host.os_type);
    assert_eq!(loaded.can_sudo, host.can_sudo);
}

#[test]
fn stale_cache_returns_none() {
    let (_dir, path) = tmp_cache_path();
    let mut host = HostCapabilities::default();
    host.detected_at = Utc::now() - Duration::days(100);
    save_host_capabilities_to(&path, &host).unwrap();
    assert!(load_host_capabilities_from(&path).is_none());
}

#[test]
fn corrupt_cache_returns_none() {
    let (_dir, path) = tmp_cache_path();
    std::fs::write(&path, "this is not json").unwrap();
    assert!(load_host_capabilities_from(&path).is_none());
}

#[test]
fn wrong_schema_version_returns_none() {
    let (_dir, path) = tmp_cache_path();
    let envelope = serde_json::json!({
        "schema_version": CACHE_SCHEMA_VERSION + 1,
        "hostname": "test",
        "os": "linux",
        "is_wsl": false,
        "expires_at": (Utc::now() + Duration::days(30)).to_rfc3339(),
        "capabilities": HostCapabilities::default(),
    });
    std::fs::write(&path, serde_json::to_string(&envelope).unwrap()).unwrap();
    assert!(load_host_capabilities_from(&path).is_none());
}
