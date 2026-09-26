//! `wt list`'s PR badges through the real binary, with every request sent to
//! a local proxy stub: a fresh store makes no request, a stalled request stops
//! at its deadline and falls back to the stored badges with their age, and a
//! refused connection shows the table without badges and stores nothing.
//!
//! Timing bounds live in `perf_pr_request.rs`; these tests check behavior.

mod perf_support;

use std::fs;
use std::process::Output;
use std::time::Duration;

use perf_support::{MixedFixture, ProxyStub, stage_from_perf};
use serial_test::serial;

fn list(fixture: &MixedFixture, proxy: &ProxyStub) -> (Output, String) {
    let output = fixture
        .wt_command_via(proxy)
        .args(["list", "--perf"])
        .env("NO_COLOR", "1")
        .output()
        .expect("wt list should run");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(output.status.success(), "wt list failed:\n{stderr}");
    (output, stderr)
}

fn row<'a>(stderr: &'a str, needle: &str) -> &'a str {
    stderr
        .lines()
        .find(|line| line.contains(needle) && line.contains('│'))
        .unwrap_or_else(|| panic!("no table row with {needle:?}:\n{stderr}"))
}

/// Removes a store written into the real user cache (Windows only; see
/// `MixedFixture::pr_store`).
struct RemoveOnDrop(std::path::PathBuf);

impl Drop for RemoveOnDrop {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

#[test]
#[serial]
fn a_fresh_pr_store_makes_no_request_and_shows_its_badges() {
    let fixture = MixedFixture::new().with_github_origin();
    let _cleanup = RemoveOnDrop(fixture.pr_store());
    fixture.seed_pr_store(Duration::from_secs(10), 99, "divergent-0");
    let proxy = ProxyStub::hanging();

    let (_, stderr) = list(&fixture, &proxy);

    assert_eq!(proxy.connections(), 0, "a store younger than 60 s skips the request");
    assert!(row(&stderr, "divergent-0").contains("PR #99"), "{stderr}");
    assert!(!stderr.contains("PRs as of"), "fresh results need no age line:\n{stderr}");
}

#[test]
#[serial]
fn a_stalled_pr_request_stops_at_its_deadline_and_shows_stored_badges_with_their_age() {
    let fixture = MixedFixture::new().with_github_origin();
    let _cleanup = RemoveOnDrop(fixture.pr_store());
    fixture.seed_pr_store(Duration::from_secs(12 * 60 + 5), 99, "divergent-0");
    let stored = fs::read(fixture.pr_store()).expect("seeded store");
    let proxy = ProxyStub::hanging();

    let (_, stderr) = list(&fixture, &proxy);

    assert!(proxy.connections() >= 1, "a 12-minute-old store is refreshed");
    assert!(row(&stderr, "divergent-0").contains("PR #99"), "{stderr}");
    assert!(stderr.contains("PRs as of 12 min ago"), "{stderr}");
    let waited = stage_from_perf(&stderr, "pr gather").expect("pr gather stage");
    assert!(waited < Duration::from_secs(2), "the 300 ms deadline bounds the wait, got {waited:?}");
    assert_eq!(fs::read(fixture.pr_store()).expect("store"), stored, "a failure is never stored");
}

#[test]
#[serial]
fn with_the_network_down_the_table_shows_without_badges_and_nothing_is_stored() {
    let fixture = MixedFixture::new().with_github_origin();
    let _cleanup = RemoveOnDrop(fixture.pr_store());
    let proxy = ProxyStub::refusing();

    let (_, stderr) = list(&fixture, &proxy);

    assert!(row(&stderr, "divergent-0").contains("divergent-0"));
    assert!(!stderr.contains("PR #"), "{stderr}");
    assert!(!stderr.contains("PRs as of"), "{stderr}");
    assert!(!fixture.pr_store().exists(), "an unavailable answer is not an empty list");

    // With results stored earlier, the same outage shows them with their age.
    fixture.seed_pr_store(Duration::from_secs(5 * 60), 99, "divergent-0");
    let (_, stderr) = list(&fixture, &proxy);
    assert!(row(&stderr, "divergent-0").contains("PR #99"), "{stderr}");
    assert!(stderr.contains("PRs as of 5 min ago"), "{stderr}");
}
