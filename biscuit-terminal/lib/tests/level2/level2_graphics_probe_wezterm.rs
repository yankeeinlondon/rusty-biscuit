//! Level-2 real-terminal evidence for the live graphics probe.
//!
//! The `discovery_probe` example (`PROBE=graphics`) runs inside a real
//! WezTerm pane driven through `biscuit-test-harness`. WezTerm, not the test,
//! parses the Kitty graphics query and answers it, which the L1 sibling
//! `level1_graphics_probe.rs` cannot show: there the test writes the replies.
//!
//! The probe runs with the terminal's identity hidden (no `TERM_PROGRAM`, no
//! `WEZTERM_*`, `TERM=xterm-256color`), so the environment cannot decide the
//! route and the library has to ask. A `Verified` Kitty route then has exactly
//! one explanation: WezTerm answered the query with `OK`.
//!
//! Spawning, focus safety (background panes only), and the skip when WezTerm
//! is unreachable follow `level2_terminal_osc_wezterm.rs`.

use std::time::{Duration, Instant};

use biscuit_test_harness::wezterm::WezTermHarness;
use biscuit_test_harness::{TerminalHarness, strip_ansi};
use test_toolkit::{Backend, Level, require_level};

/// Variables that would let the library identify WezTerm (or a multiplexer)
/// without asking it.
const HIDDEN_IDENTITY: &[&str] = &[
    "TERM_PROGRAM",
    "TERM_PROGRAM_VERSION",
    "LC_TERMINAL",
    "LC_TERMINAL_VERSION",
    "WEZTERM_PANE",
    "WEZTERM_UNIX_SOCKET",
    "WEZTERM_EXECUTABLE",
    "WEZTERM_EXECUTABLE_DIR",
    "WEZTERM_CONFIG_DIR",
    "WEZTERM_CONFIG_FILE",
    "TMUX",
    "STY",
    "ZELLIJ",
    "BISCUIT_GRAPHICS",
    "BISCUIT_GRAPHICS_TRANSPORT",
    "BISCUIT_TERMINAL_PROBE",
];

/// The first Zellij release that renders Kitty graphics.
const ZELLIJ_KITTY_MIN: (u32, u32) = (0, 45);

fn discovery_probe_path() -> std::path::PathBuf {
    let exe = std::env::current_exe().expect("current_exe unavailable");
    let dir = exe
        .parent()
        .and_then(|deps| deps.parent())
        .expect("unexpected test exe layout");
    dir.join("examples").join("discovery_probe")
}

fn probe_binary() -> std::path::PathBuf {
    let probe = discovery_probe_path();
    assert!(
        probe.exists(),
        "discovery_probe example not found at {}. Run \
         `cargo build -p biscuit-terminal --example discovery_probe`.",
        probe.display()
    );
    probe
}

/// `key=value` for the last line containing `key` (the shared pane may
/// hold an earlier run's output above this one).
fn field(text: &str, key: &str) -> Option<String> {
    text.lines().rev().find_map(|line| {
        let idx = line.find(key)?;
        Some(line[idx + key.len()..].trim().to_string())
    })
}

/// Polls the pane's scrollback until `marker` appears, returning the
/// ANSI-stripped text.
fn wait_for(harness: &mut WezTermHarness, marker: &str, within: Duration) -> String {
    let deadline = Instant::now() + within;
    loop {
        let text = strip_ansi(
            &harness
                .capture_scrollback(200)
                .expect("capture_scrollback")
                .raw,
        );
        if text.contains(marker) {
            return text;
        }
        if Instant::now() > deadline {
            panic!("marker {marker:?} not seen within {within:?}; pane text:\n{text}");
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

/// Shared WezTerm pane, settled so it answers queries.
fn shared_pane() -> Option<WezTermHarness> {
    let harness = WezTermHarness::shared_or_spawn().ok()?;
    harness.settle();
    Some(harness)
}

/// With its identity hidden, WezTerm answers the real Kitty graphics query,
/// so the route is a verified Kitty route, and no reply bytes are left on the
/// pane after the probe finishes.
#[test]
#[serial_test::serial(level2_terminal)]
fn level2_wezterm_answers_the_kitty_graphics_query() {
    require_level!(Level::L2, WezTermHarness::available(), Backend::WezTerm);
    let Some(mut harness) = shared_pane() else {
        eprintln!("skipping: requires WezTerm (shared pane unavailable)");
        return;
    };

    let unset: String = HIDDEN_IDENTITY
        .iter()
        .map(|name| format!("-u {name} "))
        .collect();
    let command = format!(
        "env {unset}TERM=xterm-256color PROBE=graphics NO_COLOR=1 {}",
        probe_binary().display()
    );
    harness.send_command_with_env("clear", &[]).expect("clear");
    harness
        .send_command_with_env(&command, &[])
        .expect("send probe command");
    wait_for(&mut harness, "graphics_done", Duration::from_secs(20));
    // A reply that arrived after the exchange stopped reading would be echoed
    // onto the pane once the probe restores the terminal's modes; give it
    // time to land before the final capture.
    harness.settle();
    let text = wait_for(&mut harness, "graphics_done", Duration::from_secs(5));

    let format = field(&text, "graphics_format[0]=");
    let confidence = field(&text, "graphics_confidence[0]=");
    // The reason line wraps at the pane's width, so only its start is
    // reliable; the Verified confidence is the evidence.
    let reason = field(&text, "graphics_reason[0]=").unwrap_or_default();
    assert_eq!(
        (format.as_deref(), confidence.as_deref()),
        (Some("Kitty"), Some("Verified")),
        "reason: {reason}; pane text:\n{text}"
    );

    let run_start = text
        .rfind("graphics_format[0]=")
        .expect("format line located above");
    let after = &text[run_start..];
    assert!(
        !after.contains("Gi=") && !after.contains("[?6"),
        "reply bytes leaked onto the pane after the exchange:\n{after}"
    );
}

/// `zellij --version` when it is at least 0.45.
fn zellij_with_kitty() -> Option<String> {
    let out = std::process::Command::new("zellij")
        .arg("--version")
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let version = text.split_whitespace().last()?;
    let mut parts = version.split('.').map(|p| p.parse::<u32>().ok());
    let major = parts.next()??;
    let minor = parts.next()??;
    ((major, minor) >= ZELLIJ_KITTY_MIN).then_some(text)
}

/// Inside Zellij (0.45 or later) in a WezTerm pane, the probe either verifies
/// a route from Zellij's answer or reports a reasoned `Unknown`; it never
/// claims a route nobody confirmed.
///
/// The session is started from a layout that runs the probe in its only
/// pane, and killed by name afterwards.
#[test]
#[serial_test::serial(level2_terminal)]
fn level2_zellij_probe_is_verified_or_reasoned() {
    require_level!(Level::L2, WezTermHarness::available(), Backend::WezTerm);
    require_level!(Level::L2, zellij_with_kitty().is_some(), "zellij >= 0.45");
    let Some(mut harness) = shared_pane() else {
        eprintln!("skipping: requires WezTerm (shared pane unavailable)");
        return;
    };

    let session = format!("biscuit-graphics-probe-{}", std::process::id());
    let dir = tempfile::tempdir().expect("tempdir");
    // Zellij 0.45.1 rejects every session name ("must be less than 0
    // characters") when its socket directory sits under macOS's long
    // `$TMPDIR`, so the session gets a short socket directory of its own.
    let sockets = tempfile::Builder::new()
        .prefix("bz")
        .tempdir_in("/tmp")
        .expect("socket dir");
    let socket_dir = sockets.path().display().to_string();
    let layout = dir.path().join("probe.kdl");
    std::fs::write(
        &layout,
        format!(
            "layout {{\n    pane command=\"sh\" {{\n        args \"-c\" \"PROBE=graphics NO_COLOR=1 {}; sleep 30\"\n    }}\n}}\n",
            probe_binary().display()
        ),
    )
    .expect("write layout");
    // A hermetic config: no tips or release-notes popup over the probe's
    // output, whatever the host's own Zellij config says.
    let config = dir.path().join("config.kdl");
    std::fs::write(&config, "show_startup_tips false\nshow_release_notes false\n")
        .expect("write config");

    harness.send_command_with_env("clear", &[]).expect("clear");
    harness
        .send_command_with_env(
            &format!(
                "zellij --config {} --session {session} --new-session-with-layout {}",
                config.display(),
                layout.display()
            ),
            &[("ZELLIJ_SOCKET_DIR", socket_dir.as_str())],
        )
        .expect("start zellij");
    let text = wait_for(&mut harness, "graphics_done", Duration::from_secs(20));
    let _ = std::process::Command::new("zellij")
        .env("ZELLIJ_SOCKET_DIR", &socket_dir)
        .args(["kill-session", &session])
        .output();
    let _ = std::process::Command::new("zellij")
        .env("ZELLIJ_SOCKET_DIR", &socket_dir)
        .args(["delete-session", "--force", &session])
        .output();

    let confidence = field(&text, "graphics_confidence[0]=");
    let reason = field(&text, "graphics_reason[0]=").unwrap_or_default();
    match confidence.as_deref() {
        Some("Verified") => assert!(reason.contains("Zellij"), "reason: {reason}"),
        Some("Unknown") => assert!(
            reason.contains("Zellij") && reason.contains(';'),
            "an Unknown route must say why: {reason}"
        ),
        other => panic!("expected Verified or Unknown, got {other:?}; pane text:\n{text}"),
    }
}
