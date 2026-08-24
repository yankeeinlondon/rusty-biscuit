//! Cross-platform integration tests for desktop notification helpers.
//!
//! Each test exercises the **real** `tokio::process::Command` path of a
//! helper adapter against a stub binary (`stub_dunstify`, `stub_notify_send`,
//! `stub_snoretoast`, `stub_burnttoast`, `stub_terminal_notifier`,
//! `stub_alerter`) that is built alongside the package.
//!
//! Stubs are env-var driven: the test sets variables like `STUB_DUNSTIFY_ID`
//! to control what each stub prints / how it exits, and asserts that the
//! helper observes the contract correctly.
//!
//! The tests are platform-agnostic because helpers only depend on argv,
//! stdin, stdout, exit code, and timeouts — every platform's `tokio::process`
//! layer can drive the same stubs without touching the real notification bus.
//! That is what makes Linux and Windows helper paths *testable* on a macOS
//! development host.
//!
//! ## Stub binary discovery
//!
//! `MESSENGER_STUB_BIN_DIR` is authoritative when set, allowing CI to build
//! the six fixtures once and deliver them to test processes. Local runs first
//! inspect the target directory beside the test executable, then build a
//! missing fixture on demand.

#![cfg(feature = "desktop")]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use crate::dispatch::{NotificationAction, NotificationUrgency};
use crate::provider::desktop::helpers::HelperBackend;
use crate::provider::desktop::helpers::HelperError;
use crate::provider::desktop::helpers::alerter::AlerterHelper;
use crate::provider::desktop::helpers::burnttoast::BurntToastHelper;
use crate::provider::desktop::helpers::dunstify::DunstifyHelper;
use crate::provider::desktop::helpers::notify_send::NotifySendHelper;
use crate::provider::desktop::helpers::snoretoast::SnoreToastHelper;
use crate::provider::desktop::helpers::terminal_notifier::TerminalNotifierHelper;
use crate::provider::desktop::request::DesktopNotificationRequest;

/// Locate the directory holding the compiled stub binaries.
///
/// Cargo runs unit tests from `target/debug/deps/<test-bin>`, while the
/// `[[bin]]` targets land in `target/debug/`. Pop the `deps` segment to get
/// to the bin directory.
fn stub_bin_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("test executable path");
    let mut dir = exe.parent().expect("test executable parent").to_path_buf();
    if dir.file_name().and_then(|name| name.to_str()) == Some("deps") {
        dir.pop();
    }
    dir
}

const STUB_NAMES: &[&str] = &[
    "stub_dunstify",
    "stub_notify_send",
    "stub_snoretoast",
    "stub_burnttoast",
    "stub_terminal_notifier",
    "stub_alerter",
];

static STUB_PATH_CACHE: OnceLock<HashMap<&'static str, PathBuf>> = OnceLock::new();

#[derive(Debug, PartialEq, Eq)]
enum StubResolution {
    Ready(PathBuf),
    BuildRequired(PathBuf),
}

/// Resolve the path to a stub binary, building it on demand for local runs.
///
/// An explicit `MESSENGER_STUB_BIN_DIR` never falls back: a missing fixture is
/// a delivery error. Without it, third-party runners and surgical filters may
/// omit `[[bin]]` targets, so a missing or stale local fixture is rebuilt.
///
/// Resolved paths are cached in a process-wide [`OnceLock`] so that
/// `cargo-nextest` (which runs each test in a fresh process) pays the
/// build cost at most once per process, not once per test.
fn stub_path(name: &str) -> PathBuf {
    let cache = STUB_PATH_CACHE.get_or_init(resolve_stub_paths);
    cache
        .get(name)
        .unwrap_or_else(|| panic!("unknown stub binary `{name}`"))
        .clone()
}

fn resolve_stub_paths() -> HashMap<&'static str, PathBuf> {
    STUB_NAMES
        .iter()
        .map(|name| (*name, resolve_stub_path(name)))
        .collect()
}

fn resolve_stub_path(name: &str) -> PathBuf {
    let path = match stub_resolution(name, &stub_bin_dir()) {
        Ok(StubResolution::Ready(path)) => return path,
        Ok(StubResolution::BuildRequired(path)) => path,
        Err(error) => panic!("{error}"),
    };

    {
        let mut child = std::process::Command::new(env!("CARGO"))
            .args([
                "build",
                "--quiet",
                "--bin",
                name,
                "--features",
                "desktop",
                "-p",
                "messenger",
            ])
            .spawn()
            .expect("failed to invoke cargo to build stub binary");

        let start = Instant::now();
        let timeout = Duration::from_secs(120);
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) => {
                    if start.elapsed() > timeout {
                        let _ = child.kill();
                        panic!(
                            "cargo build for stub binary `{name}` timed out after {timeout:?}. \
                             Build stubs manually with: cargo build --features desktop -p messenger"
                        );
                    }
                    std::thread::sleep(Duration::from_millis(100));
                }
                Err(e) => panic!("failed to wait for cargo build of stub `{name}`: {e}"),
            }
        };
        assert!(
            status.success(),
            "cargo build failed for stub binary {name}"
        );
    }
    assert!(
        path.exists(),
        "stub binary {} not found at {}. \
         Build stubs manually with: cargo build --features desktop -p messenger",
        name,
        path.display()
    );
    path
}

fn stub_executable_name(name: &str, executable_suffix: &str) -> String {
    format!("{name}{executable_suffix}")
}

fn stub_resolution(name: &str, target_dir: &Path) -> Result<StubResolution, String> {
    let file_name = stub_executable_name(name, std::env::consts::EXE_SUFFIX);
    if let Some(explicit_dir) = std::env::var_os("MESSENGER_STUB_BIN_DIR") {
        let path = PathBuf::from(explicit_dir).join(&file_name);
        return if path.is_file() {
            Ok(StubResolution::Ready(path))
        } else {
            Err(format!(
                "MESSENGER_STUB_BIN_DIR fixture `{name}` is missing at {}",
                path.display()
            ))
        };
    }

    let path = target_dir.join(file_name);
    if path.is_file() {
        Ok(StubResolution::Ready(path))
    } else {
        Ok(StubResolution::BuildRequired(path))
    }
}

#[cfg(test)]
mod stub_resolution_tests {
    use super::*;
    use serial_test::serial;
    use test_toolkit::EnvGuard as TestEnvGuard;

    #[test]
    #[serial(stub_resolution_env)]
    fn explicit_directory_takes_precedence_and_resolves_all_six_stubs() {
        let explicit = tempfile::tempdir().expect("explicit stub directory");
        let target = tempfile::tempdir().expect("target stub directory");
        for name in STUB_NAMES {
            let file_name = stub_executable_name(name, std::env::consts::EXE_SUFFIX);
            std::fs::write(explicit.path().join(&file_name), []).expect("explicit stub fixture");
            std::fs::write(target.path().join(file_name), []).expect("target stub fixture");
        }
        let _env = TestEnvGuard::set_safe("MESSENGER_STUB_BIN_DIR", explicit.path());

        for name in STUB_NAMES {
            assert_eq!(
                stub_resolution(name, target.path()).expect("stub resolves"),
                StubResolution::Ready(
                    explicit
                        .path()
                        .join(stub_executable_name(name, std::env::consts::EXE_SUFFIX))
                )
            );
        }
    }

    #[test]
    fn windows_stub_names_use_the_executable_suffix() {
        assert_eq!(
            stub_executable_name("stub_snoretoast", ".exe"),
            "stub_snoretoast.exe"
        );
    }

    #[test]
    #[serial(stub_resolution_env)]
    fn missing_explicit_fixture_is_an_authoritative_error() {
        let explicit = tempfile::tempdir().expect("explicit stub directory");
        let target = tempfile::tempdir().expect("target stub directory");
        std::fs::write(target.path().join(stub_executable_name("stub_alerter", "")), [])
            .expect("target stub fixture");
        let _env = TestEnvGuard::set_safe("MESSENGER_STUB_BIN_DIR", explicit.path());

        let error = stub_resolution("stub_alerter", target.path())
            .expect_err("the explicit directory is authoritative");

        assert!(error.contains("MESSENGER_STUB_BIN_DIR"));
        assert!(error.contains("stub_alerter"));
        assert!(error.contains(&explicit.path().display().to_string()));
    }

    #[test]
    #[serial(stub_resolution_env)]
    fn missing_target_fixture_remains_eligible_for_local_build_fallback() {
        let target = tempfile::tempdir().expect("target stub directory");
        let _env = TestEnvGuard::remove_safe("MESSENGER_STUB_BIN_DIR");
        let expected = target
            .path()
            .join(stub_executable_name("stub_notify_send", std::env::consts::EXE_SUFFIX));

        assert_eq!(
            stub_resolution("stub_notify_send", target.path()).expect("fallback remains valid"),
            StubResolution::BuildRequired(expected)
        );
    }

    #[test]
    #[serial(stub_resolution_env)]
    fn existing_target_fixture_never_starts_a_nested_build() {
        let target = tempfile::tempdir().expect("target stub directory");
        let _env = TestEnvGuard::remove_safe("MESSENGER_STUB_BIN_DIR");
        let expected = target
            .path()
            .join(stub_executable_name("stub_notify_send", std::env::consts::EXE_SUFFIX));
        std::fs::write(&expected, []).expect("target stub fixture");

        assert_eq!(
            stub_resolution("stub_notify_send", target.path()).expect("fixture resolves"),
            StubResolution::Ready(expected)
        );
    }
}

/// Small RAII guard that removes the env vars it set when dropped.
///
/// Stub binaries read their behaviour out of the parent process's env, so
/// each test that sets vars must clean them up to avoid bleeding state into
/// later tests on the same thread.
struct EnvGuard {
    keys: Vec<&'static str>,
}

impl EnvGuard {
    fn set(pairs: &[(&'static str, &str)]) -> Self {
        let mut keys = Vec::with_capacity(pairs.len());
        for (key, value) in pairs {
            // SAFETY: tests run sequentially within the helper modules below
            // (each uses `serial_test::serial`) so env-var manipulation does
            // not race with other threads.
            unsafe {
                std::env::set_var(key, value);
            }
            keys.push(*key);
        }
        Self { keys }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for key in &self.keys {
            // SAFETY: same single-threaded invariant as `set`.
            unsafe {
                std::env::remove_var(key);
            }
        }
    }
}

fn notice_request() -> DesktopNotificationRequest {
    DesktopNotificationRequest {
        title: "Hello".into(),
        body: Some("World".into()),
        subtitle: None,
        app_name: "Messenger".into(),
        icon: None,
        image: None,
        silent: false,
        category: None,
        urgency: NotificationUrgency::Normal,
        timeout_ms: None,
        replace_id: None,
        group_id: None,
        actions: Vec::new(),
        progress: None,
        badge_count: None,
        replace_helper_hint: None,
    }
}

fn interactive_request(actions: Vec<NotificationAction>) -> DesktopNotificationRequest {
    let mut request = notice_request();
    request.actions = actions;
    request
}

fn dunstify_helper(path: &Path) -> DunstifyHelper {
    DunstifyHelper::new(path.to_path_buf(), true)
}

fn notify_send_helper(path: &Path) -> NotifySendHelper {
    NotifySendHelper::new(path.to_path_buf(), Some((0, 8, 3)))
}

fn snoretoast_helper(path: &Path) -> SnoreToastHelper {
    let helper = SnoreToastHelper::new(
        path.to_path_buf(),
        "RustyBiscuit.MessengerTests".to_string(),
    );
    // Skip AppID registration for send-path tests; dedicated registration
    // coverage below exercises the real `-install` shell-out.
    helper.mark_app_id_registered();
    helper
}

fn burnttoast_helper(path: &Path) -> BurntToastHelper {
    let helper = BurntToastHelper::new(
        path.to_path_buf(),
        "RustyBiscuit.MessengerTests".to_string(),
    );
    // Skip the per-process AppID registration shell-out so the stub only has
    // to handle the actual send invocation.
    helper.mark_app_id_registered();
    helper
}

fn terminal_notifier_helper(path: &Path) -> TerminalNotifierHelper {
    TerminalNotifierHelper::new(path.to_path_buf())
}

fn alerter_helper(path: &Path) -> AlerterHelper {
    AlerterHelper::new(path.to_path_buf())
}

fn alerter_helper_with_timeout(path: &Path, timeout_ms: u64) -> AlerterHelper {
    let mut helper = AlerterHelper::new(path.to_path_buf());
    helper.notice_timeout_ms = timeout_ms;
    helper
}

mod dunstify_stub {
    use super::*;
    use serial_test::serial;

    #[tokio::test]
    #[serial(dunstify)]
    async fn success_returns_id_and_helper_name() {
        let stub = stub_path("stub_dunstify");
        let _env = EnvGuard::set(&[("STUB_DUNSTIFY_ID", "123")]);
        let helper = dunstify_helper(&stub);
        let receipt = helper.send(&notice_request()).await.unwrap();
        assert_eq!(receipt.notification_id, "123");
        assert_eq!(helper.name().to_string(), "dunstify");
    }

    #[tokio::test]
    #[serial(dunstify)]
    async fn interactive_records_action_metadata() {
        let stub = stub_path("stub_dunstify");
        let _env = EnvGuard::set(&[("STUB_DUNSTIFY_ID", "9"), ("STUB_DUNSTIFY_ACTION", "ok")]);
        let helper = dunstify_helper(&stub);
        let request = interactive_request(vec![NotificationAction {
            id: "ok".into(),
            label: "OK".into(),
        }]);
        let receipt = helper.send(&request).await.unwrap();
        assert_eq!(receipt.notification_id, "9");
        assert_eq!(
            receipt.metadata.get("activation_type").map(String::as_str),
            Some("action"),
        );
        assert_eq!(
            receipt.metadata.get("activation_key").map(String::as_str),
            Some("ok"),
        );
    }

    #[tokio::test]
    #[serial(dunstify)]
    async fn replace_returns_replaced_metadata() {
        let stub = stub_path("stub_dunstify");
        let _env = EnvGuard::set(&[("STUB_DUNSTIFY_ID", "42")]);
        let helper = dunstify_helper(&stub);
        let receipt = helper.replace("42", &notice_request()).await.unwrap();
        assert_eq!(receipt.notification_id, "42");
        assert_eq!(
            receipt.metadata.get("replaced").map(String::as_str),
            Some("42"),
        );
    }

    #[tokio::test]
    #[serial(dunstify)]
    async fn nonzero_exit_maps_to_exited_error() {
        let stub = stub_path("stub_dunstify");
        let _env = EnvGuard::set(&[("STUB_DUNSTIFY_EXIT", "7")]);
        let helper = dunstify_helper(&stub);
        let result = helper.send(&notice_request()).await;
        let error = result.expect_err("expected dunstify stub to exit nonzero");
        assert!(matches!(error, HelperError::Exited { status: 7, .. }));
        assert!(error.is_fallback_eligible());
    }

    #[tokio::test]
    #[serial(dunstify)]
    async fn empty_stdout_propagates_parse_error() {
        let stub = stub_path("stub_dunstify");
        let _env = EnvGuard::set(&[("STUB_DUNSTIFY_STDOUT_OVERRIDE", "")]);
        let helper = dunstify_helper(&stub);
        let result = helper.send(&notice_request()).await;
        assert!(matches!(result, Err(HelperError::Parse(_))));
    }

    #[tokio::test]
    #[serial(dunstify)]
    async fn notice_only_timeout_maps_to_timeout_error() {
        let stub = stub_path("stub_dunstify");
        let _env = EnvGuard::set(&[("STUB_DUNSTIFY_SLEEP_MS", "4000")]);
        let helper = dunstify_helper(&stub);
        let result = helper.send(&notice_request()).await;
        let error = result.expect_err("expected dunstify stub to timeout");
        assert!(
            matches!(error, HelperError::Timeout { timeout_ms: 3000 }),
            "got {error:?}"
        );
    }
}

mod notify_send_stub {
    use super::*;
    use serial_test::serial;

    #[tokio::test]
    #[serial(notify_send)]
    async fn success_records_id() {
        let stub = stub_path("stub_notify_send");
        let _env = EnvGuard::set(&[("STUB_NOTIFY_SEND_ID", "777")]);
        let helper = notify_send_helper(&stub);
        let receipt = helper.send(&notice_request()).await.unwrap();
        assert_eq!(receipt.notification_id, "777");
        assert_eq!(helper.name().to_string(), "notify_send");
    }

    #[tokio::test]
    #[serial(notify_send)]
    async fn replace_returns_replaced_metadata() {
        let stub = stub_path("stub_notify_send");
        let _env = EnvGuard::set(&[("STUB_NOTIFY_SEND_ID", "55")]);
        let helper = notify_send_helper(&stub);
        let receipt = helper.replace("55", &notice_request()).await.unwrap();
        assert_eq!(receipt.notification_id, "55");
        assert_eq!(
            receipt.metadata.get("replaced").map(String::as_str),
            Some("55"),
        );
    }

    #[tokio::test]
    #[serial(notify_send)]
    async fn nonzero_exit_maps_to_exited_error() {
        let stub = stub_path("stub_notify_send");
        let _env = EnvGuard::set(&[("STUB_NOTIFY_SEND_EXIT", "1")]);
        let helper = notify_send_helper(&stub);
        let result = helper.send(&notice_request()).await;
        let error = result.expect_err("expected notify-send stub to exit nonzero");
        assert!(matches!(error, HelperError::Exited { status: 1, .. }));
        assert!(error.is_fallback_eligible());
    }

    #[tokio::test]
    #[serial(notify_send)]
    async fn empty_stdout_propagates_parse_error() {
        let stub = stub_path("stub_notify_send");
        let _env = EnvGuard::set(&[("STUB_NOTIFY_SEND_STDOUT_OVERRIDE", "")]);
        let helper = notify_send_helper(&stub);
        let result = helper.send(&notice_request()).await;
        assert!(matches!(result, Err(HelperError::Parse(_))));
    }

    #[tokio::test]
    #[serial(notify_send)]
    async fn notice_only_timeout_maps_to_timeout_error() {
        let stub = stub_path("stub_notify_send");
        let _env = EnvGuard::set(&[("STUB_NOTIFY_SEND_SLEEP_MS", "6000")]);
        let helper = notify_send_helper(&stub);
        let result = helper.send(&notice_request()).await;
        let error = result.expect_err("expected notify-send stub to timeout");
        assert!(
            matches!(error, HelperError::Timeout { timeout_ms: 5000 }),
            "got {error:?}"
        );
    }
}

mod snoretoast_stub {
    use super::*;
    use serial_test::serial;

    #[tokio::test]
    #[serial(snoretoast)]
    async fn notice_only_uses_replace_id_and_records_dismissed() {
        let stub = stub_path("stub_snoretoast");
        let _env = EnvGuard::set(&[("STUB_SNORETOAST_EXIT", "1")]);
        let helper = snoretoast_helper(&stub);
        let mut request = notice_request();
        request.replace_id = Some("snore-1".into());
        let receipt = helper.send(&request).await.unwrap();
        assert_eq!(receipt.notification_id, "snore-1");
        assert_eq!(
            receipt.metadata.get("activation_type").map(String::as_str),
            Some("dismissed"),
        );
    }

    #[tokio::test]
    #[serial(snoretoast)]
    async fn interactive_recovers_action_id_via_label() {
        let stub = stub_path("stub_snoretoast");
        let _env = EnvGuard::set(&[
            ("STUB_SNORETOAST_STDOUT", "Confirm"),
            ("STUB_SNORETOAST_EXIT", "0"),
        ]);
        let helper = snoretoast_helper(&stub);
        let request = interactive_request(vec![NotificationAction {
            id: "confirm".into(),
            label: "Confirm".into(),
        }]);
        let receipt = helper.send(&request).await.unwrap();
        assert_eq!(
            receipt.metadata.get("activation_type").map(String::as_str),
            Some("action"),
        );
        assert_eq!(
            receipt.metadata.get("activation_key").map(String::as_str),
            Some("confirm"),
        );
    }

    #[tokio::test]
    #[serial(snoretoast)]
    async fn replace_round_trips_id() {
        let stub = stub_path("stub_snoretoast");
        let _env = EnvGuard::set(&[("STUB_SNORETOAST_EXIT", "1")]);
        let helper = snoretoast_helper(&stub);
        let receipt = helper.replace("toast-9", &notice_request()).await.unwrap();
        assert_eq!(receipt.notification_id, "toast-9");
        assert_eq!(
            receipt.metadata.get("replaced").map(String::as_str),
            Some("toast-9"),
        );
    }

    #[tokio::test]
    #[serial(snoretoast)]
    async fn exit_four_propagates_exited_error() {
        let stub = stub_path("stub_snoretoast");
        let _env = EnvGuard::set(&[
            ("STUB_SNORETOAST_STDOUT", "boom"),
            ("STUB_SNORETOAST_EXIT", "4"),
        ]);
        let helper = snoretoast_helper(&stub);
        let result = helper.send(&notice_request()).await;
        let error = result.expect_err("expected SnoreToast stub to fail");
        assert!(matches!(error, HelperError::Exited { status: 4, .. }));
    }

    #[tokio::test]
    #[serial(snoretoast)]
    async fn notice_only_timeout_maps_to_timeout_error() {
        // SnoreToast notice-only timeout is 5000ms; sleep for 6000ms to trigger it.
        let stub = stub_path("stub_snoretoast");
        let _env = EnvGuard::set(&[("STUB_SNORETOAST_SLEEP_MS", "6000")]);
        let helper = snoretoast_helper(&stub);
        let result = helper.send(&notice_request()).await;
        let error = result.expect_err("expected SnoreToast stub to timeout");
        assert!(
            matches!(error, HelperError::Timeout { timeout_ms: 5000 }),
            "got {error:?}"
        );
    }

    #[tokio::test]
    #[serial(snoretoast)]
    async fn interactive_request_does_not_timeout() {
        let stub = stub_path("stub_snoretoast");
        let _env = EnvGuard::set(&[
            ("STUB_SNORETOAST_SLEEP_MS", "2000"),
            ("STUB_SNORETOAST_STDOUT", "Confirm"),
            ("STUB_SNORETOAST_EXIT", "0"),
        ]);
        let helper = snoretoast_helper(&stub);
        let request = interactive_request(vec![NotificationAction {
            id: "confirm".into(),
            label: "Confirm".into(),
        }]);
        let receipt = helper.send(&request).await.unwrap();
        assert_eq!(
            receipt.metadata.get("activation_type").map(String::as_str),
            Some("action"),
        );
        assert_eq!(
            receipt.metadata.get("activation_key").map(String::as_str),
            Some("confirm"),
        );
    }

    #[tokio::test]
    #[serial(snoretoast)]
    async fn app_id_registration_runs_install_before_send() {
        let stub = stub_path("stub_snoretoast");
        let log = tempfile::NamedTempFile::new().unwrap();
        let log_path = log.path().to_string_lossy().into_owned();
        let _env = EnvGuard::set(&[("STUB_SNORETOAST_ARGV_LOG", log_path.as_str())]);
        let helper = SnoreToastHelper::new(
            stub.to_path_buf(),
            "RustyBiscuit.MessengerTests".to_string(),
        );
        assert!(!helper.app_id_registered());

        let mut request = notice_request();
        request.replace_id = Some("registered-snore".into());
        let receipt = helper.send(&request).await.unwrap();

        assert!(helper.app_id_registered());
        assert_eq!(receipt.notification_id, "registered-snore");
        let log = std::fs::read_to_string(log.path()).unwrap();
        let lines = log.lines().collect::<Vec<_>>();
        assert_eq!(
            lines.len(),
            2,
            "expected install and send argv lines: {log}"
        );
        let install = lines[0].split('\t').collect::<Vec<_>>();
        assert_eq!(install[0], "-appID");
        assert_eq!(install[1], "RustyBiscuit.MessengerTests");
        assert_eq!(install[2], "-install");
        assert_eq!(install[3], "RustyBiscuit.MessengerTests.lnk");
        assert_eq!(install[4], stub.to_string_lossy());
        assert_eq!(install[5], "RustyBiscuit.MessengerTests");
        assert!(!lines[1].contains("-install"));
    }

    #[tokio::test]
    #[serial(snoretoast)]
    async fn oversized_png_is_dropped_but_send_still_succeeds() {
        // Step 4.1: a 2048×2048 PNG exceeds Windows toast limits. The helper
        // must drop the image from argv, annotate the receipt with
        // `dropped=image_too_large`, and still complete the send — image
        // failures degrade gracefully rather than aborting the notification.
        use std::io::Write;

        let stub = stub_path("stub_snoretoast");
        let _env = EnvGuard::set(&[("STUB_SNORETOAST_EXIT", "1")]);

        let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
        bytes.extend_from_slice(&13u32.to_be_bytes());
        bytes.extend_from_slice(b"IHDR");
        bytes.extend_from_slice(&2048u32.to_be_bytes());
        bytes.extend_from_slice(&2048u32.to_be_bytes());
        bytes.extend_from_slice(&[8, 6, 0, 0, 0]);
        bytes.extend_from_slice(&[0, 0, 0, 0]);

        let temp = tempfile::Builder::new().suffix(".png").tempfile().unwrap();
        let mut file = temp.reopen().unwrap();
        file.write_all(&bytes).unwrap();
        file.flush().unwrap();

        let helper = snoretoast_helper(&stub);
        let mut request = notice_request();
        request.image = Some(temp.path().to_path_buf());
        request.replace_id = Some("oversize-1".into());

        let receipt = helper.send(&request).await.unwrap();
        assert_eq!(receipt.notification_id, "oversize-1");
        assert_eq!(
            receipt.metadata.get("dropped").map(String::as_str),
            Some("image_too_large"),
        );
    }
}

mod burnttoast_stub {
    use super::*;
    use serial_test::serial;

    #[tokio::test]
    #[serial(burnttoast)]
    async fn parses_action_activation() {
        let stub = stub_path("stub_burnttoast");
        let _env = EnvGuard::set(&[(
            "STUB_BURNTTOAST_JSON",
            r#"{"activationType":"action","activationKey":"ok"}"#,
        )]);
        let helper = burnttoast_helper(&stub);
        let request = interactive_request(vec![NotificationAction {
            id: "ok".into(),
            label: "OK".into(),
        }]);
        let receipt = helper.send(&request).await.unwrap();
        assert_eq!(
            receipt.metadata.get("activation_type").map(String::as_str),
            Some("action"),
        );
        assert_eq!(
            receipt.metadata.get("activation_key").map(String::as_str),
            Some("ok"),
        );
    }

    #[tokio::test]
    #[serial(burnttoast)]
    async fn parses_reply_activation() {
        let stub = stub_path("stub_burnttoast");
        let _env = EnvGuard::set(&[(
            "STUB_BURNTTOAST_JSON",
            r#"{"activationType":"reply","replyText":"hi there"}"#,
        )]);
        let helper = burnttoast_helper(&stub);
        let receipt = helper.send(&notice_request()).await.unwrap();
        assert_eq!(
            receipt.metadata.get("activation_type").map(String::as_str),
            Some("reply"),
        );
        assert_eq!(
            receipt.metadata.get("reply_text").map(String::as_str),
            Some("hi there"),
        );
    }

    #[tokio::test]
    #[serial(burnttoast)]
    async fn missing_marker_falls_back_to_dismissed() {
        let stub = stub_path("stub_burnttoast");
        // No JSON env var → stub emits no marker.
        let helper = burnttoast_helper(&stub);
        let receipt = helper.send(&notice_request()).await.unwrap();
        assert_eq!(
            receipt.metadata.get("activation_type").map(String::as_str),
            Some("dismissed"),
        );
    }

    #[tokio::test]
    #[serial(burnttoast)]
    async fn nonzero_exit_propagates_exited_error() {
        let stub = stub_path("stub_burnttoast");
        let _env = EnvGuard::set(&[("STUB_BURNTTOAST_EXIT", "9")]);
        let helper = burnttoast_helper(&stub);
        let result = helper.send(&notice_request()).await;
        let error = result.expect_err("expected pwsh stub to exit nonzero");
        assert!(matches!(error, HelperError::Exited { status: 9, .. }));
    }

    #[tokio::test]
    #[serial(burnttoast)]
    async fn notice_only_timeout_maps_to_timeout_error() {
        // BurntToast notice-only timeout is 10000ms; sleep for 11000ms to trigger it.
        let stub = stub_path("stub_burnttoast");
        let _env = EnvGuard::set(&[("STUB_BURNTTOAST_SLEEP_MS", "11000")]);
        let helper = burnttoast_helper(&stub);
        let result = helper.send(&notice_request()).await;
        let error = result.expect_err("expected BurntToast stub to timeout");
        assert!(
            matches!(error, HelperError::Timeout { timeout_ms: 10000 }),
            "got {error:?}"
        );
    }

    #[tokio::test]
    #[serial(burnttoast)]
    async fn interactive_request_does_not_timeout() {
        let stub = stub_path("stub_burnttoast");
        let _env = EnvGuard::set(&[
            ("STUB_BURNTTOAST_SLEEP_MS", "2000"),
            (
                "STUB_BURNTTOAST_JSON",
                r#"{"activationType":"action","activationKey":"ok"}"#,
            ),
        ]);
        let helper = burnttoast_helper(&stub);
        let request = interactive_request(vec![NotificationAction {
            id: "ok".into(),
            label: "OK".into(),
        }]);
        let receipt = helper.send(&request).await.unwrap();
        assert_eq!(
            receipt.metadata.get("activation_type").map(String::as_str),
            Some("action"),
        );
        assert_eq!(
            receipt.metadata.get("activation_key").map(String::as_str),
            Some("ok"),
        );
    }

    #[tokio::test]
    #[serial(burnttoast)]
    async fn app_id_registration_succeeds_before_send() {
        // Do NOT call mark_app_id_registered — let the real registration path run.
        let stub = stub_path("stub_burnttoast");
        let log = tempfile::NamedTempFile::new().unwrap();
        let log_path = log.path().to_string_lossy().into_owned();
        let _env = EnvGuard::set(&[
            ("STUB_BURNTTOAST_JSON", r#"{"activationType":"dismissed"}"#),
            ("STUB_BURNTTOAST_STDIN_LOG", log_path.as_str()),
        ]);
        let helper = BurntToastHelper::new(
            stub.to_path_buf(),
            "RustyBiscuit.MessengerTests".to_string(),
        );
        assert!(!helper.app_id_registered());
        let receipt = helper.send(&notice_request()).await.unwrap();
        assert!(helper.app_id_registered());
        assert_eq!(
            receipt.metadata.get("activation_type").map(String::as_str),
            Some("dismissed"),
        );
        let log = std::fs::read_to_string(log.path()).unwrap();
        let scripts = log.split("-----SCRIPT-----").collect::<Vec<_>>();
        assert!(
            scripts
                .get(1)
                .is_some_and(|script| script.contains("New-BTAppId")),
            "registration script did not include New-BTAppId: {log}",
        );
        assert!(
            scripts
                .get(2)
                .is_some_and(|script| script.contains("Submit-BTNotification")),
            "send script did not include Submit-BTNotification: {log}",
        );
    }
}

mod terminal_notifier_stub {
    use super::*;
    use serial_test::serial;

    #[tokio::test]
    #[serial(terminal_notifier)]
    async fn success_uses_group_as_id_when_supplied() {
        let stub = stub_path("stub_terminal_notifier");
        let helper = terminal_notifier_helper(&stub);
        let mut request = notice_request();
        request.group_id = Some("build-alerts".into());
        let receipt = helper.send(&request).await.unwrap();
        assert_eq!(receipt.notification_id, "build-alerts");
    }

    #[tokio::test]
    #[serial(terminal_notifier)]
    async fn replace_records_replaced_metadata() {
        let stub = stub_path("stub_terminal_notifier");
        let helper = terminal_notifier_helper(&stub);
        let receipt = helper.replace("legacy-7", &notice_request()).await.unwrap();
        assert_eq!(receipt.notification_id, "legacy-7");
        assert_eq!(
            receipt.metadata.get("replaced").map(String::as_str),
            Some("legacy-7"),
        );
    }

    #[tokio::test]
    #[serial(terminal_notifier)]
    async fn nonzero_exit_propagates_exited_error() {
        let stub = stub_path("stub_terminal_notifier");
        let _env = EnvGuard::set(&[("STUB_TERMINAL_NOTIFIER_EXIT", "2")]);
        let helper = terminal_notifier_helper(&stub);
        let result = helper.send(&notice_request()).await;
        let error = result.expect_err("expected terminal-notifier stub to fail");
        assert!(matches!(error, HelperError::Exited { status: 2, .. }));
    }

    #[tokio::test]
    #[serial(terminal_notifier)]
    async fn notice_only_timeout_maps_to_timeout_error() {
        // terminal-notifier timeout is 5000ms; sleep for 6000ms to trigger it.
        let stub = stub_path("stub_terminal_notifier");
        let _env = EnvGuard::set(&[("STUB_TERMINAL_NOTIFIER_SLEEP_MS", "6000")]);
        let helper = terminal_notifier_helper(&stub);
        let result = helper.send(&notice_request()).await;
        let error = result.expect_err("expected terminal-notifier stub to timeout");
        assert!(
            matches!(error, HelperError::Timeout { timeout_ms: 5000 }),
            "got {error:?}"
        );
    }
}

mod alerter_stub {
    use super::*;
    use serial_test::serial;

    #[tokio::test]
    #[serial(alerter)]
    async fn parses_action_clicked() {
        let stub = stub_path("stub_alerter");
        let _env = EnvGuard::set(&[
            ("STUB_ALERTER_TYPE", "actionClicked"),
            ("STUB_ALERTER_VALUE", "ok"),
        ]);
        let helper = alerter_helper(&stub);
        let request = interactive_request(vec![NotificationAction {
            id: "ok".into(),
            label: "OK".into(),
        }]);
        let receipt = helper.send(&request).await.unwrap();
        assert_eq!(
            receipt.metadata.get("activation_type").map(String::as_str),
            Some("action"),
        );
        assert_eq!(
            receipt.metadata.get("activation_key").map(String::as_str),
            Some("ok"),
        );
    }

    #[tokio::test]
    #[serial(alerter)]
    async fn parses_replied_with_value_as_reply_text() {
        let stub = stub_path("stub_alerter");
        let _env = EnvGuard::set(&[
            ("STUB_ALERTER_TYPE", "replied"),
            ("STUB_ALERTER_VALUE", "see you soon"),
        ]);
        let helper = alerter_helper(&stub);
        let receipt = helper.send(&notice_request()).await.unwrap();
        assert_eq!(
            receipt.metadata.get("activation_type").map(String::as_str),
            Some("reply"),
        );
        assert_eq!(
            receipt.metadata.get("reply_text").map(String::as_str),
            Some("see you soon"),
        );
    }

    #[tokio::test]
    #[serial(alerter)]
    async fn parses_closed_as_dismissed() {
        let stub = stub_path("stub_alerter");
        let _env = EnvGuard::set(&[("STUB_ALERTER_TYPE", "closed")]);
        let helper = alerter_helper(&stub);
        let receipt = helper.send(&notice_request()).await.unwrap();
        assert_eq!(
            receipt.metadata.get("activation_type").map(String::as_str),
            Some("dismissed"),
        );
    }

    #[tokio::test]
    #[serial(alerter)]
    async fn invalid_json_propagates_parse_error() {
        let stub = stub_path("stub_alerter");
        let _env = EnvGuard::set(&[("STUB_ALERTER_STDOUT_OVERRIDE", "not-json\n")]);
        let helper = alerter_helper(&stub);
        let result = helper.send(&notice_request()).await;
        assert!(matches!(result, Err(HelperError::Parse(_))));
    }

    #[tokio::test]
    #[serial(alerter)]
    async fn interactive_request_does_not_timeout() {
        // Alerter has no timeout for interactive requests. Sleeping for 2s
        // must not trigger a timeout error.
        let stub = stub_path("stub_alerter");
        let _env = EnvGuard::set(&[
            ("STUB_ALERTER_SLEEP_MS", "2000"),
            ("STUB_ALERTER_TYPE", "actionClicked"),
            ("STUB_ALERTER_VALUE", "ok"),
        ]);
        let helper = alerter_helper(&stub);
        let request = interactive_request(vec![NotificationAction {
            id: "ok".into(),
            label: "OK".into(),
        }]);
        let receipt = helper.send(&request).await.unwrap();
        assert_eq!(
            receipt.metadata.get("activation_type").map(String::as_str),
            Some("action"),
        );
        assert_eq!(
            receipt.metadata.get("activation_key").map(String::as_str),
            Some("ok"),
        );
    }

    #[tokio::test]
    #[serial(alerter)]
    async fn notice_only_timeout_maps_to_timeout_error() {
        // Use a very short timeout so the suite stays fast; we only need to
        // verify that the helper surfaces HelperError::Timeout.
        let stub = stub_path("stub_alerter");
        let _env = EnvGuard::set(&[("STUB_ALERTER_SLEEP_MS", "200")]);
        let helper = alerter_helper_with_timeout(&stub, 100);
        let result = helper.send(&notice_request()).await;
        let error = result.expect_err("expected alerter stub to timeout");
        assert!(
            matches!(error, HelperError::Timeout { timeout_ms: 100 }),
            "got {error:?}"
        );
    }
}

mod backend_fallback {
    //! Higher-level checks that route stubs through `LinuxBackend` and
    //! `WindowsBackend` to verify helper election + fallback annotation.
    //!
    //! Each test runs the platform backend with explicitly-supplied helper
    //! instances pointing at the stub binaries — no native delivery is
    //! attempted on success because the elected helper succeeds.

    use super::*;
    use crate::provider::desktop::LinuxDesktopConfig;
    use crate::provider::desktop::MacOsDesktopConfig;
    use crate::provider::desktop::WindowsDesktopConfig;
    use crate::provider::desktop::backend::DesktopBackend;
    use crate::provider::desktop::linux::LinuxBackend;
    use crate::provider::desktop::macos::MacOsBackend;
    use crate::provider::desktop::windows::WindowsBackend;
    use serial_test::serial;

    #[tokio::test]
    #[serial(dunstify)]
    async fn linux_backend_routes_through_dunstify_stub() {
        let dunstify_path = stub_path("stub_dunstify");
        let _env = EnvGuard::set(&[("STUB_DUNSTIFY_ID", "linux-99")]);

        let helpers: Vec<Arc<dyn HelperBackend>> = vec![Arc::new(dunstify_helper(&dunstify_path))];
        let backend = LinuxBackend::with_helpers(LinuxDesktopConfig::default(), helpers);

        let receipt = backend.send(notice_request()).await.unwrap();
        assert_eq!(receipt.notification_id, "linux-99");
        assert_eq!(
            receipt.metadata.get("helper_used").map(String::as_str),
            Some("dunstify"),
        );
        assert!(!receipt.metadata.contains_key("helper_fallbacks"));
    }

    #[tokio::test]
    #[serial(dunstify, notify_send)]
    async fn linux_backend_falls_through_to_notify_send_when_dunstify_fails() {
        let dunstify_path = stub_path("stub_dunstify");
        let notify_path = stub_path("stub_notify_send");
        let _env = EnvGuard::set(&[
            ("STUB_DUNSTIFY_EXIT", "2"),
            ("STUB_NOTIFY_SEND_ID", "ns-42"),
        ]);

        let helpers: Vec<Arc<dyn HelperBackend>> = vec![
            Arc::new(dunstify_helper(&dunstify_path)),
            Arc::new(notify_send_helper(&notify_path)),
        ];
        let backend = LinuxBackend::with_helpers(LinuxDesktopConfig::default(), helpers);

        let receipt = backend.send(notice_request()).await.unwrap();
        assert_eq!(receipt.notification_id, "ns-42");
        assert_eq!(
            receipt.metadata.get("helper_used").map(String::as_str),
            Some("notify_send"),
        );
        let fallbacks = receipt
            .metadata
            .get("helper_fallbacks")
            .map(String::as_str)
            .unwrap_or("");
        assert!(fallbacks.contains("dunstify"));
    }

    #[tokio::test]
    #[serial(snoretoast)]
    async fn windows_backend_routes_through_snoretoast_stub() {
        let snore_path = stub_path("stub_snoretoast");
        let _env = EnvGuard::set(&[("STUB_SNORETOAST_EXIT", "1")]);

        let config = WindowsDesktopConfig {
            app_id: Some("RustyBiscuit.MessengerTests".into()),
            ..WindowsDesktopConfig::default()
        };
        let helpers: Vec<Arc<dyn HelperBackend>> = vec![Arc::new(snoretoast_helper(&snore_path))];
        let backend = WindowsBackend::with_helpers(config, helpers);

        let mut request = notice_request();
        request.replace_id = Some("toast-room".into());
        let receipt = backend.send(request).await.unwrap();
        assert_eq!(receipt.notification_id, "toast-room");
        assert_eq!(
            receipt.metadata.get("helper_used").map(String::as_str),
            Some("snore_toast"),
        );
    }

    #[tokio::test]
    #[serial(snoretoast, burnttoast)]
    async fn windows_backend_falls_through_to_burnttoast_when_snoretoast_fails() {
        let snore_path = stub_path("stub_snoretoast");
        let burnt_path = stub_path("stub_burnttoast");
        let _env = EnvGuard::set(&[
            ("STUB_SNORETOAST_EXIT", "4"),
            ("STUB_SNORETOAST_STDOUT", "boom"),
            ("STUB_BURNTTOAST_JSON", r#"{"activationType":"dismissed"}"#),
        ]);

        let config = WindowsDesktopConfig {
            app_id: Some("RustyBiscuit.MessengerTests".into()),
            ..WindowsDesktopConfig::default()
        };
        let helpers: Vec<Arc<dyn HelperBackend>> = vec![
            Arc::new(snoretoast_helper(&snore_path)),
            Arc::new(burnttoast_helper(&burnt_path)),
        ];
        let backend = WindowsBackend::with_helpers(config, helpers);

        let receipt = backend.send(notice_request()).await.unwrap();
        assert_eq!(
            receipt.metadata.get("helper_used").map(String::as_str),
            Some("burnt_toast"),
        );
        let fallbacks = receipt
            .metadata
            .get("helper_fallbacks")
            .map(String::as_str)
            .unwrap_or("");
        assert!(fallbacks.contains("snore_toast"));
    }

    #[tokio::test]
    #[serial(dunstify, notify_send)]
    async fn linux_backend_dunstify_timeout_falls_through_to_notify_send() {
        let dunstify_path = stub_path("stub_dunstify");
        let notify_path = stub_path("stub_notify_send");
        // Dunstify notice-only timeout is 3000ms; sleep for 4000ms to trigger it.
        let _env = EnvGuard::set(&[
            ("STUB_DUNSTIFY_SLEEP_MS", "4000"),
            ("STUB_NOTIFY_SEND_ID", "ns-timeout"),
        ]);

        let helpers: Vec<Arc<dyn HelperBackend>> = vec![
            Arc::new(dunstify_helper(&dunstify_path)),
            Arc::new(notify_send_helper(&notify_path)),
        ];
        let backend = LinuxBackend::with_helpers(LinuxDesktopConfig::default(), helpers);

        let receipt = backend.send(notice_request()).await.unwrap();
        assert_eq!(receipt.notification_id, "ns-timeout");
        assert_eq!(
            receipt.metadata.get("helper_used").map(String::as_str),
            Some("notify_send"),
        );
        let fallbacks = receipt
            .metadata
            .get("helper_fallbacks")
            .map(String::as_str)
            .unwrap_or("");
        assert!(fallbacks.contains("dunstify"));
    }

    #[tokio::test]
    #[serial(terminal_notifier)]
    async fn macos_backend_routes_through_terminal_notifier_stub() {
        let tn_path = stub_path("stub_terminal_notifier");
        let helpers: Vec<Arc<dyn HelperBackend>> =
            vec![Arc::new(terminal_notifier_helper(&tn_path))];
        let backend = MacOsBackend::with_helpers(MacOsDesktopConfig::default(), helpers);

        let mut request = notice_request();
        request.group_id = Some("mac-group".into());
        let receipt = backend.send(request).await.unwrap();
        assert_eq!(receipt.notification_id, "mac-group");
        assert_eq!(
            receipt.metadata.get("helper_used").map(String::as_str),
            Some("terminal_notifier"),
        );
    }

    #[tokio::test]
    #[serial(terminal_notifier, alerter)]
    async fn macos_backend_falls_through_to_alerter_when_terminal_notifier_fails() {
        let tn_path = stub_path("stub_terminal_notifier");
        let alerter_path = stub_path("stub_alerter");
        // Notice-only request so terminal-notifier scores 80 and is elected first.
        let _env = EnvGuard::set(&[
            ("STUB_TERMINAL_NOTIFIER_EXIT", "1"),
            ("STUB_ALERTER_TYPE", "closed"),
        ]);

        let helpers: Vec<Arc<dyn HelperBackend>> = vec![
            Arc::new(terminal_notifier_helper(&tn_path)),
            Arc::new(alerter_helper(&alerter_path)),
        ];
        let backend = MacOsBackend::with_helpers(MacOsDesktopConfig::default(), helpers);

        let receipt = backend.send(notice_request()).await.unwrap();
        assert_eq!(
            receipt.metadata.get("helper_used").map(String::as_str),
            Some("alerter"),
        );
        let fallbacks = receipt
            .metadata
            .get("helper_fallbacks")
            .map(String::as_str)
            .unwrap_or("");
        assert!(fallbacks.contains("terminal_notifier"));
    }
}
