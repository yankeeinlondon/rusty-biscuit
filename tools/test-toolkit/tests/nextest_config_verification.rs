use std::thread;
use std::time::Duration;

/// A deliberately slow test to verify that nextest's `slow-timeout` configuration
/// is being honored. The default profile marks tests slower than 5s as slow; this
/// test sleeps for 6s to trigger that threshold.
#[test]
fn slow_test_should_be_flagged_by_nextest() {
    thread::sleep(Duration::from_secs(6));
    assert!(true);
}
