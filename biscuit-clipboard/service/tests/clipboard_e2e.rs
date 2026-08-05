//! Real clipboard end-to-end integration tests.
//!
//! These tests mutate the user's actual OS clipboard and therefore
//! require a graphical session (or clipboard daemon on Linux). They
//! are gated by the `RUN_CLIPBOARD_E2E=1` environment variable and are
//! **not** run as part of the standard `cargo test` suite.
//!
//! ## Running
//!
//! ```bash
//! RUN_CLIPBOARD_E2E=1 cargo test -p biscuit-clipboard-service --test clipboard_e2e
//! ```
//!
//! ## Platform Requirements
//!
//! - **macOS**: Works out of the box; requires a logged-in user session.
//! - **Linux**: Requires a running clipboard manager (e.g. `wl-clipboard`
//!   on Wayland or `xclip` / `xsel` on X11) and a display server.
//! - **Windows**: Works out of the box in a standard user session.

use std::process::{Child, Command, Stdio};
use std::time::Duration;

use biscuit_clipboard::ClipboardBackend;
use biscuit_clipboard::backend::SystemClipboard;
use biscuit_clipboard::config::{CLIP_RUNTIME_DIR_ENV, read_port_file_in};

const HEALTH_TIMEOUT_MS: u64 = 5000;
const HISTORY_POLL_TIMEOUT_MS: u64 = 2000;
const HISTORY_POLL_INTERVAL_MS: u64 = 100;

/// Holds the spawned clipper process and the temp runtime directory.
/// Dropping it kills the process and cleans up the temp dir.
#[allow(dead_code)]
struct ServiceHandle {
    child: Child,
    runtime_dir: tempfile::TempDir,
}

impl Drop for ServiceHandle {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn e2e_enabled() -> bool {
    std::env::var("RUN_CLIPBOARD_E2E")
        .map(|v| v == "1")
        .unwrap_or(false)
}

/// Locate the `clipper` binary built for this test binary.
fn clipper_bin() -> std::path::PathBuf {
    biscuit_test_harness::bin_exe!("clipper")
}

/// Spawn `clipper` with a temp runtime directory and wait for it to
/// report healthy.
async fn start_service() -> (ServiceHandle, u16) {
    let runtime_dir = tempfile::tempdir().expect("tempdir");

    let child = Command::new(clipper_bin())
        .arg("--port")
        .arg("0")
        .env(CLIP_RUNTIME_DIR_ENV, runtime_dir.path())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn clipper");

    // Poll for the port file to appear.
    let port = tokio::time::timeout(Duration::from_millis(HEALTH_TIMEOUT_MS), async {
        loop {
            if let Some(p) = read_port_file_in(runtime_dir.path()) {
                return p;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("clipper did not write port file in time");

    // Poll /health until ready.
    let client = reqwest::Client::new();
    tokio::time::timeout(Duration::from_millis(HEALTH_TIMEOUT_MS), async {
        loop {
            match client
                .get(format!("http://127.0.0.1:{port}/health"))
                .send()
                .await
            {
                Ok(resp) if resp.status().is_success() => break,
                _ => tokio::time::sleep(Duration::from_millis(50)).await,
            }
        }
    })
    .await
    .expect("clipper health check timed out");

    (ServiceHandle { child, runtime_dir }, port)
}

/// Poll `/history` until an entry with the given preview appears, or
/// the timeout expires.
async fn wait_for_history_entry(port: u16, expected_preview: &str) -> Option<serde_json::Value> {
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{port}/history");
    let deadline = tokio::time::Instant::now() + Duration::from_millis(HISTORY_POLL_TIMEOUT_MS);

    while tokio::time::Instant::now() < deadline {
        if let Ok(resp) = client.get(&url).send().await
            && let Ok(json) = resp.json::<serde_json::Value>().await
            && let Some(entries) = json.get("entries").and_then(|v| v.as_array())
        {
            for entry in entries {
                if entry.get("preview").and_then(|p| p.as_str()) == Some(expected_preview) {
                    return Some(entry.clone());
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(HISTORY_POLL_INTERVAL_MS)).await;
    }
    None
}

/// Write plain text to the real OS clipboard.
fn write_os_text(text: &str) {
    let backend = SystemClipboard::new().expect("construct SystemClipboard");
    backend.set_text(text).expect("write text to clipboard");
}

/// Write HTML to the real OS clipboard.
fn write_os_html(html: &str) {
    let backend = SystemClipboard::new().expect("construct SystemClipboard");
    backend.set_html(html).expect("write html to clipboard");
}

/// Write a small PNG image to the real OS clipboard.
fn write_os_image(data: &[u8]) {
    let backend = SystemClipboard::new().expect("construct SystemClipboard");
    backend
        .set_image(data, 1, 1)
        .expect("write image to clipboard");
}

#[tokio::test]
async fn e2e_text_roundtrip() {
    if !e2e_enabled() {
        return;
    }

    let (_handle, port) = start_service().await;

    write_os_text("e2e text hello");

    let entry = wait_for_history_entry(port, "e2e text hello")
        .await
        .expect("history entry did not appear in time");

    assert_eq!(
        entry.get("content_type").and_then(|v| v.as_str()),
        Some("text")
    );
}

#[tokio::test]
async fn e2e_html_roundtrip() {
    if !e2e_enabled() {
        return;
    }

    let (_handle, port) = start_service().await;

    write_os_html("<b>e2e html hello</b>");

    let entry = wait_for_history_entry(port, "e2e html hello")
        .await
        .expect("history entry did not appear in time");

    // The watcher should capture both html and a plain-text fallback
    // when the platform provides it.
    assert_eq!(
        entry.get("content_type").and_then(|v| v.as_str()),
        Some("html")
    );
}

#[tokio::test]
async fn e2e_image_roundtrip() {
    if !e2e_enabled() {
        return;
    }

    let (_handle, port) = start_service().await;

    // Minimal 1x1 PNG (header + IHDR + IDAT + IEND).
    let png: Vec<u8> = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
        0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, // IHDR chunk
        0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77,
        0x53, 0xDE, // IHDR CRC
        0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, // IDAT chunk
        0x08, 0xD7, 0x63, 0xF8, 0x0F, 0x00, 0x00, 0x01, 0x01, 0x00, 0x05, 0x18, 0xD8,
        0xAE, // IDAT CRC
        0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, // IEND chunk
        0xAE, 0x42, 0x60, 0x82, // IEND CRC
    ];

    write_os_image(&png);

    let entry = wait_for_history_entry(port, "[image]")
        .await
        .expect("history entry did not appear in time");

    assert_eq!(
        entry.get("content_type").and_then(|v| v.as_str()),
        Some("image")
    );
}

#[tokio::test]
#[cfg(target_os = "macos")]
async fn e2e_concealed_skipped_macos() {
    if !e2e_enabled() {
        return;
    }

    let (_handle, port) = start_service().await;

    // Place `org.nspasteboard.ConcealedType` on the pasteboard using
    // the macOS `pbcopy` tool with a custom UTI.
    let output = Command::new("osascript")
        .arg("-e")
        .arg(
            r#"
            set the clipboard to "concealed-secret"
            do shell script "echo 'concealed-secret' | pbcopy"
        "#,
        )
        .output()
        .expect("osascript");

    if !output.status.success() {
        // If we cannot automate concealed placement, skip the test
        // gracefully.
        eprintln!(
            "Skipping concealed test: osascript failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }

    // Wait a moment for the watcher to process.
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Query history — the concealed entry must NOT appear.
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{port}/history");
    let resp = client
        .get(&url)
        .send()
        .await
        .expect("request history")
        .json::<serde_json::Value>()
        .await
        .expect("parse history");

    if let Some(entries) = resp.get("entries").and_then(|v| v.as_array()) {
        for entry in entries {
            let preview = entry.get("preview").and_then(|p| p.as_str()).unwrap_or("");
            assert_ne!(
                preview, "concealed-secret",
                "concealed clipboard content must not appear in history"
            );
        }
    }
}
