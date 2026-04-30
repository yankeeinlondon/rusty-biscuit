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
//! `cargo test -p messenger --features desktop` builds every `[[bin]]` in
//! the package whose `required-features` are satisfied. The compiled stubs
//! sit alongside the test binary in the target directory; we navigate up
//! from the test executable to locate them.

#![cfg(feature = "desktop")]

use std::path::{Path, PathBuf};
use std::sync::Arc;

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
    let mut dir = exe
        .parent()
        .expect("test executable parent")
        .to_path_buf();
    if dir.file_name().and_then(|name| name.to_str()) == Some("deps") {
        dir.pop();
    }
    dir
}

/// Resolve the path to a stub binary, building it on demand if missing.
///
/// `cargo test` does build every `[[bin]]` whose `required-features` match
/// the active feature set, but third-party runners (e.g. cargo-nextest with
/// `--no-fail-fast`) and surgical filters (`--lib`) can skip that step. As
/// a safety net we shell out to `cargo build --bin <name>` when the file is
/// missing; the build is a no-op when the bin is already up to date.
fn stub_path(name: &str) -> PathBuf {
    let mut path = stub_bin_dir();
    path.push(if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    });
    if !path.exists() {
        let status = std::process::Command::new(env!("CARGO"))
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
            .status()
            .expect("failed to invoke cargo to build stub binary");
        assert!(status.success(), "cargo build failed for stub binary {name}");
    }
    assert!(
        path.exists(),
        "stub binary {} not found at {}",
        name,
        path.display()
    );
    path
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
    SnoreToastHelper::new(path.to_path_buf(), "RustyBiscuit.MessengerTests".to_string())
}

fn burnttoast_helper(path: &Path) -> BurntToastHelper {
    let helper = BurntToastHelper::new(path.to_path_buf(), "RustyBiscuit.MessengerTests".to_string());
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

mod dunstify_stub {
    use super::*;
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    async fn success_returns_id_and_helper_name() {
        let stub = stub_path("stub_dunstify");
        let _env = EnvGuard::set(&[("STUB_DUNSTIFY_ID", "123")]);
        let helper = dunstify_helper(&stub);
        let receipt = helper.send(&notice_request()).await.unwrap();
        assert_eq!(receipt.notification_id, "123");
        assert_eq!(helper.name().to_string(), "dunstify");
    }

    #[tokio::test]
    #[serial]
    async fn interactive_records_action_metadata() {
        let stub = stub_path("stub_dunstify");
        let _env = EnvGuard::set(&[
            ("STUB_DUNSTIFY_ID", "9"),
            ("STUB_DUNSTIFY_ACTION", "ok"),
        ]);
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
    #[serial]
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
    #[serial]
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
    #[serial]
    async fn empty_stdout_propagates_parse_error() {
        let stub = stub_path("stub_dunstify");
        let _env = EnvGuard::set(&[("STUB_DUNSTIFY_STDOUT_OVERRIDE", "")]);
        let helper = dunstify_helper(&stub);
        let result = helper.send(&notice_request()).await;
        assert!(matches!(result, Err(HelperError::Parse(_))));
    }
}

mod notify_send_stub {
    use super::*;
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    async fn success_records_id() {
        let stub = stub_path("stub_notify_send");
        let _env = EnvGuard::set(&[("STUB_NOTIFY_SEND_ID", "777")]);
        let helper = notify_send_helper(&stub);
        let receipt = helper.send(&notice_request()).await.unwrap();
        assert_eq!(receipt.notification_id, "777");
        assert_eq!(helper.name().to_string(), "notify_send");
    }

    #[tokio::test]
    #[serial]
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
    #[serial]
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
    #[serial]
    async fn empty_stdout_propagates_parse_error() {
        let stub = stub_path("stub_notify_send");
        let _env = EnvGuard::set(&[("STUB_NOTIFY_SEND_STDOUT_OVERRIDE", "")]);
        let helper = notify_send_helper(&stub);
        let result = helper.send(&notice_request()).await;
        assert!(matches!(result, Err(HelperError::Parse(_))));
    }
}

mod snoretoast_stub {
    use super::*;
    use serial_test::serial;

    #[tokio::test]
    #[serial]
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
    #[serial]
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
    #[serial]
    async fn replace_round_trips_id() {
        let stub = stub_path("stub_snoretoast");
        let _env = EnvGuard::set(&[("STUB_SNORETOAST_EXIT", "1")]);
        let helper = snoretoast_helper(&stub);
        let receipt = helper
            .replace("toast-9", &notice_request())
            .await
            .unwrap();
        assert_eq!(receipt.notification_id, "toast-9");
        assert_eq!(
            receipt.metadata.get("replaced").map(String::as_str),
            Some("toast-9"),
        );
    }

    #[tokio::test]
    #[serial]
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
    #[serial]
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
    #[serial]
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
    #[serial]
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
    #[serial]
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
    #[serial]
    async fn nonzero_exit_propagates_exited_error() {
        let stub = stub_path("stub_burnttoast");
        let _env = EnvGuard::set(&[("STUB_BURNTTOAST_EXIT", "9")]);
        let helper = burnttoast_helper(&stub);
        let result = helper.send(&notice_request()).await;
        let error = result.expect_err("expected pwsh stub to exit nonzero");
        assert!(matches!(error, HelperError::Exited { status: 9, .. }));
    }
}

mod terminal_notifier_stub {
    use super::*;
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    async fn success_uses_group_as_id_when_supplied() {
        let stub = stub_path("stub_terminal_notifier");
        let helper = terminal_notifier_helper(&stub);
        let mut request = notice_request();
        request.group_id = Some("build-alerts".into());
        let receipt = helper.send(&request).await.unwrap();
        assert_eq!(receipt.notification_id, "build-alerts");
    }

    #[tokio::test]
    #[serial]
    async fn replace_records_replaced_metadata() {
        let stub = stub_path("stub_terminal_notifier");
        let helper = terminal_notifier_helper(&stub);
        let receipt = helper
            .replace("legacy-7", &notice_request())
            .await
            .unwrap();
        assert_eq!(receipt.notification_id, "legacy-7");
        assert_eq!(
            receipt.metadata.get("replaced").map(String::as_str),
            Some("legacy-7"),
        );
    }

    #[tokio::test]
    #[serial]
    async fn nonzero_exit_propagates_exited_error() {
        let stub = stub_path("stub_terminal_notifier");
        let _env = EnvGuard::set(&[("STUB_TERMINAL_NOTIFIER_EXIT", "2")]);
        let helper = terminal_notifier_helper(&stub);
        let result = helper.send(&notice_request()).await;
        let error = result.expect_err("expected terminal-notifier stub to fail");
        assert!(matches!(error, HelperError::Exited { status: 2, .. }));
    }
}

mod alerter_stub {
    use super::*;
    use serial_test::serial;

    #[tokio::test]
    #[serial]
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
    #[serial]
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
    #[serial]
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
    #[serial]
    async fn invalid_json_propagates_parse_error() {
        let stub = stub_path("stub_alerter");
        let _env = EnvGuard::set(&[("STUB_ALERTER_STDOUT_OVERRIDE", "not-json\n")]);
        let helper = alerter_helper(&stub);
        let result = helper.send(&notice_request()).await;
        assert!(matches!(result, Err(HelperError::Parse(_))));
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
    use crate::provider::desktop::WindowsDesktopConfig;
    use crate::provider::desktop::backend::DesktopBackend;
    use crate::provider::desktop::linux::LinuxBackend;
    use crate::provider::desktop::windows::WindowsBackend;
    use serial_test::serial;

    #[tokio::test]
    #[serial]
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
    #[serial]
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
    #[serial]
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
    #[serial]
    async fn windows_backend_falls_through_to_burnttoast_when_snoretoast_fails() {
        let snore_path = stub_path("stub_snoretoast");
        let burnt_path = stub_path("stub_burnttoast");
        let _env = EnvGuard::set(&[
            ("STUB_SNORETOAST_EXIT", "4"),
            ("STUB_SNORETOAST_STDOUT", "boom"),
            (
                "STUB_BURNTTOAST_JSON",
                r#"{"activationType":"dismissed"}"#,
            ),
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
}
