//! Level-2 evidence for graphics through a real tmux.
//!
//! The `discovery_probe` example (`PROBE=graphics`, `PROBE_GRAPHICS_IMAGE`)
//! runs inside a detached pane on the harness's private tmux server, so the
//! library reads the live facts: `tmux -V`, the pane's `allow-passthrough`
//! (set by the test before the probe starts), and the attached clients (none:
//! the session is detached, so the outer terminal is unidentified). The L1
//! sibling `graphics_tmux_matrix.rs` covers every route row with fixed facts.
//!
//! `pipe-pane` records the raw bytes the probe writes, which is how the test
//! sees the passthrough envelopes: tmux consumes them, so they never reach
//! the pane's grid. No window is opened and nothing takes focus.

use std::path::PathBuf;
use std::process::Stdio;
use std::time::{Duration, Instant};

use biscuit_test_harness::strip_ansi;
use biscuit_test_harness::tmux::{TmuxHarness, kill_session_by_name, tmux_command};
use test_toolkit::{Backend, Level, require_level};

/// Variables that would decide the route without the tmux facts.
const UNSET: &[&str] = &[
    "BISCUIT_GRAPHICS",
    "BISCUIT_GRAPHICS_TRANSPORT",
    "BISCUIT_TERMINAL_PROBE",
    "ZELLIJ",
    "SSH_CONNECTION",
    "SSH_CLIENT",
    "SSH_TTY",
];

fn probe_binary() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe unavailable");
    let probe = exe
        .parent()
        .and_then(|deps| deps.parent())
        .expect("unexpected test exe layout")
        .join("examples")
        .join("discovery_probe");
    assert!(
        probe.exists(),
        "discovery_probe example not found at {}. Run \
         `cargo build -p biscuit-terminal --example discovery_probe`.",
        probe.display()
    );
    probe
}

fn tmux(args: &[&str]) -> String {
    let out = tmux_command()
        .args(args)
        .stdin(Stdio::null())
        .output()
        .expect("run tmux");
    assert!(
        out.status.success(),
        "tmux {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// A detached session whose only pane waits for Enter, then runs the probe
/// with `allow-passthrough` set to `passthrough`. Killed on drop.
struct ProbeSession {
    name: String,
    raw: PathBuf,
    _dir: tempfile::TempDir,
}

impl ProbeSession {
    fn run(passthrough: &str) -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        let image = dir.path().join("dot.png");
        image::RgbImage::from_pixel(4, 4, image::Rgb([200, 30, 30]))
            .save(&image)
            .expect("write image");
        let raw = dir.path().join("raw.bin");
        let name = format!("biscuit_test_{}_graphics_{passthrough}", std::process::id());

        let unset: String = UNSET.iter().map(|name| format!("-u {name} ")).collect();
        let command = format!(
            "read _; env {unset}PROBE=graphics NO_COLOR=1 PROBE_GRAPHICS_IMAGE='{}' '{}'; sleep 60",
            image.display(),
            probe_binary().display()
        );
        tmux(&[
            "new-session", "-d", "-s", &name, "-x", "250", "-y", "50", "sh", "-c", &command,
        ]);
        let session = Self {
            name,
            raw,
            _dir: dir,
        };
        tmux(&["set-option", "-p", "-t", &session.name, "allow-passthrough", passthrough]);
        tmux(&[
            "pipe-pane",
            "-o",
            "-t",
            &session.name,
            &format!("cat >> '{}'", session.raw.display()),
        ]);
        tmux(&["send-keys", "-t", &session.name, "Enter"]);
        session
    }

    /// The pane text once the probe printed `graphics_done`.
    fn finished_text(&self) -> String {
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            let text = strip_ansi(&tmux(&["capture-pane", "-p", "-t", &self.name]));
            if text.contains("graphics_done") {
                return text;
            }
            assert!(Instant::now() < deadline, "probe did not finish; pane:\n{text}");
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    /// Everything the probe wrote, as tmux received it.
    fn raw_output(&self) -> Vec<u8> {
        // `pipe-pane` copies asynchronously; wait for the end marker.
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let bytes = std::fs::read(&self.raw).unwrap_or_default();
            if find(&bytes, b"graphics_done").is_some() || Instant::now() > deadline {
                return bytes;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

impl Drop for ProbeSession {
    fn drop(&mut self) {
        kill_session_by_name(&self.name);
    }
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn field<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    text.lines()
        .find_map(|line| line.trim_start().strip_prefix(key))
        .map(str::trim)
}

fn tmux_version_at_least_3_3() -> bool {
    let out = tmux_command().arg("-V").output();
    let Ok(out) = out else { return false };
    let text = String::from_utf8_lossy(&out.stdout);
    let mut parts = text
        .split(|c: char| !c.is_ascii_digit())
        .filter(|part| !part.is_empty())
        .map(|part| part.parse::<u32>().unwrap_or(0));
    (parts.next().unwrap_or(0), parts.next().unwrap_or(0)) >= (3, 3)
}

/// `allow-passthrough on`, no client attached: Kitty through tmux,
/// unverified, and every Kitty chunk reaches tmux in its own envelope with
/// its ESCs doubled, none of it left on the pane.
#[test]
fn level2_tmux_passthrough_on_wraps_kitty_chunks() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    require_level!(Level::L2, tmux_version_at_least_3_3(), "tmux >= 3.3");
    let session = ProbeSession::run("on");
    let text = session.finished_text();

    assert_eq!(field(&text, "graphics_format[0]="), Some("Kitty"), "pane:\n{text}");
    assert_eq!(field(&text, "graphics_transport[0]="), Some("Tmux"), "pane:\n{text}");
    assert_eq!(field(&text, "graphics_confidence[0]="), Some("Unknown"), "pane:\n{text}");
    let reason = field(&text, "graphics_reason[0]=").unwrap_or_default();
    assert!(reason.contains("allow-passthrough on"), "reason: {reason}");
    assert_eq!(field(&text, "graphics_image[0]="), Some("pixels"), "pane:\n{text}");
    let after = &text[text.find("graphics_image[0]=").unwrap_or(0)..];
    assert!(
        !after.contains("_G") && !after.contains("tmux;"),
        "graphics bytes reached the pane's grid:\n{after}"
    );

    let raw = session.raw_output();
    let envelope = find(&raw, b"\x1bPtmux;\x1b\x1b_Gq=2,i=").unwrap_or_else(|| {
        panic!("no wrapped Kitty upload in the raw output: {:?}", String::from_utf8_lossy(&raw))
    });
    let upload = &raw[envelope..];
    let end = find(upload, b"\x1b\x1b\\\x1b\\").expect("the chunk's ST is doubled, then the envelope's");
    let chunk = String::from_utf8_lossy(&upload[..end]);
    assert!(chunk.contains(",f=100,a=T,t=d,c="), "{chunk}");
    assert!(
        find(&raw, b"\x1b_G").is_none_or(|at| at > 0 && raw[at - 1] == 0x1b),
        "every Kitty sequence is inside an envelope"
    );
}

/// `allow-passthrough off`: no pixel route, the reason names the setting,
/// and the image prints as alt text with no graphics bytes at all.
#[test]
fn level2_tmux_passthrough_off_falls_back_and_names_the_setting() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    require_level!(Level::L2, tmux_version_at_least_3_3(), "tmux >= 3.3");
    let session = ProbeSession::run("off");
    let text = session.finished_text();

    assert_eq!(field(&text, "graphics_format[0]="), Some("None"), "pane:\n{text}");
    assert_eq!(field(&text, "graphics_reason_kind[0]="), Some("BlockedTransport"), "pane:\n{text}");
    assert_eq!(
        field(&text, "graphics_fix[0]="),
        Some("Some(\"set -g allow-passthrough on\")"),
        "pane:\n{text}"
    );
    assert_eq!(
        field(&text, "graphics_image[0]="),
        Some("fallback:[Image: dot.png]"),
        "pane:\n{text}"
    );
    let raw = session.raw_output();
    assert!(find(&raw, b"\x1b_G").is_none(), "no Kitty bytes are written");
    assert!(find(&raw, b"tmux;").is_none(), "no passthrough envelope is written");
}
