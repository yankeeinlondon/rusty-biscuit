//! Level 2 terminal-capture tests for `schematic-gen` CLI output.
//!
//! These tests run the `schematic-gen` binary inside a real `tmux`
//! session so that the `colored` crate detects an attached terminal and
//! emits SGR escape sequences. The pane is then captured with
//! `tmux capture-pane -e` (which preserves escape codes) and the raw
//! buffer is asserted against expected ANSI styling and content.
//!
//! The tests **self-skip** when `tmux` is not available on `PATH`,
//! keeping CI environments without a terminal multiplexer green.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread::sleep;
use std::time::{Duration, Instant};

use biscuit_test_harness::bin_exe;

/// Per-process tmux socket path. Lives under `std::env::temp_dir()` in a
/// directory created with mode 0700 so tmux does not refuse it as unsafe
/// (which happens when the shared `/tmp/tmux-$UID` directory is left
/// world-writable by some other process).
fn tmux_socket() -> &'static Path {
    static SOCKET: OnceLock<PathBuf> = OnceLock::new();
    SOCKET.get_or_init(|| {
        let dir = std::env::temp_dir().join(format!("schematic-gen-tmux-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create tmux socket dir");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))
                .expect("chmod 700 tmux socket dir");
        }
        dir.join("default")
    })
}

/// Build a `tmux` command pre-wired with `-S <socket>` so every call in
/// this test binary uses our private socket dir.
fn tmux() -> Command {
    let mut c = Command::new("tmux");
    c.arg("-S").arg(tmux_socket());
    c
}

/// Returns true when the `tmux` binary is on PATH and responds to `-V`.
fn tmux_available() -> bool {
    Command::new("tmux")
        .arg("-V")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Path to the `schematic-gen` binary under test.
fn schematic_gen_bin() -> PathBuf {
    bin_exe!("schematic-gen")
}

/// Returns a unique tmux session name so concurrent test runs do not
/// collide.
fn unique_session_name(prefix: &str) -> String {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    format!("{prefix}_{}_{n}", std::process::id())
}

/// Quote a string for a POSIX shell so that it survives interpretation
/// by the shell tmux spawns inside the session.
fn shell_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for ch in s.chars() {
        if ch == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    out
}

/// Spawn a fresh detached tmux session, send `cmd` to its shell with a
/// trailing `DONE-MARKER` print, then poll the captured pane until the
/// marker is observed (or the timeout elapses).
///
/// The pane history is set wide and tall enough to capture the entire
/// output of `schematic-gen generate --api all --dry-run` (~40k lines)
/// so callers can assert on lines that scroll off the visible area.
///
/// ## Returns
///
/// The full pane buffer including escape sequences.
fn run_in_tmux(session_prefix: &str, cmd: &str) -> Result<String, String> {
    let session = unique_session_name(session_prefix);
    // The marker is emitted by `printf` from the spawned shell. To
    // distinguish the printf'd occurrence from the literal copy of the
    // command line that tmux echoes back, the sentinel includes a
    // numeric suffix (`READY=42`) that only appears in the printf
    // output - the typed command line shows the variable name, not the
    // expanded value.
    let marker_token = "SCHEMATIC_GEN_CAPTURE_DONE_MARKER";
    let marker_value = "READY=4242";
    let printed_marker = format!("{marker_token}_{marker_value}");

    // Create a wide, tall detached session.
    let status = tmux()
        .args([
            "new-session",
            "-d",
            "-s",
            &session,
            "-x",
            "200",
            "-y",
            "50",
            "bash",
        ])
        .status()
        .map_err(|e| format!("tmux new-session failed: {e}"))?;
    if !status.success() {
        return Err("tmux new-session returned non-zero".to_string());
    }

    // Bump the scrollback buffer so large outputs are fully captured.
    let _ = tmux()
        .args(["set-option", "-t", &session, "history-limit", "100000"])
        .status();

    // Give bash a moment to start before sending keys.
    sleep(Duration::from_millis(200));

    // Compose the command to send. The shell expands `$READY_VAR` so
    // the marker that lands in the pane buffer differs from the bytes
    // that tmux echoes back when the user "types" the command.
    let full_cmd = format!(
        "READY_VAR={marker_value}; env -u NO_COLOR CLICOLOR_FORCE=1 TERM=xterm-256color {cmd}; printf '{marker_token}_%s\\n' \"$READY_VAR\""
    );

    let send_status = tmux()
        .args(["send-keys", "-t", &session, &full_cmd, "Enter"])
        .status()
        .map_err(|e| format!("tmux send-keys failed: {e}"))?;
    if !send_status.success() {
        kill_session(&session);
        return Err("tmux send-keys returned non-zero".to_string());
    }

    // Poll capture-pane until the printed marker (with expanded value)
    // appears or 30 seconds elapse.
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut last_capture = String::new();
    while Instant::now() < deadline {
        let out = tmux()
            .args(["capture-pane", "-t", &session, "-p", "-e", "-S", "-"])
            .output();
        match out {
            Ok(o) if o.status.success() => {
                last_capture = String::from_utf8_lossy(&o.stdout).into_owned();
                if last_capture.contains(&printed_marker) {
                    kill_session(&session);
                    return Ok(last_capture);
                }
            }
            Ok(o) => {
                kill_session(&session);
                return Err(format!(
                    "tmux capture-pane failed: {}",
                    String::from_utf8_lossy(&o.stderr)
                ));
            }
            Err(e) => {
                kill_session(&session);
                return Err(format!("tmux capture-pane spawn failed: {e}"));
            }
        }
        sleep(Duration::from_millis(200));
    }

    kill_session(&session);
    Err(format!(
        "timed out waiting for {printed_marker}; last capture buffer:\n{last_capture}"
    ))
}

fn kill_session(name: &str) {
    let _ = tmux()
        .args(["kill-session", "-t", name])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

/// Assert that `haystack` contains `needle`; on failure, print a
/// context-rich error including a sanitized snippet of the buffer.
fn assert_contains(haystack: &str, needle: &str, label: &str) {
    if !haystack.contains(needle) {
        let preview: String = haystack.chars().take(2_000).collect();
        panic!("{label}: expected output to contain {needle:?}\n--- buffer preview ---\n{preview}");
    }
}

fn assert_contains_any(haystack: &str, needles: &[&str], label: &str) {
    if needles.iter().any(|needle| haystack.contains(needle)) {
        return;
    }

    let preview: String = haystack.chars().take(2_000).collect();
    panic!(
        "{label}: expected output to contain one of {needles:?}\n--- buffer preview ---\n{preview}"
    );
}

fn assert_not_contains(haystack: &str, needle: &str, label: &str) {
    if haystack.contains(needle) {
        let preview: String = haystack.chars().take(2_000).collect();
        panic!(
            "{label}: expected output to NOT contain {needle:?}\n--- buffer preview ---\n{preview}"
        );
    }
}

/// `schematic-gen validate --api openai` should emit bold-green
/// `[PASS]` and `[OK]` markers when run inside a real terminal.
///
/// Skips when tmux is unavailable so CI hosts without it stay green.
#[test]
fn validate_emits_ansi_styled_ok_on_success() {
    if !tmux_available() {
        eprintln!("tmux not available on PATH - skipping terminal capture test");
        return;
    }

    let bin = schematic_gen_bin();
    let cmd = format!(
        "{} validate --api openai",
        shell_quote(&bin.to_string_lossy())
    );
    let buffer = run_in_tmux("schematic_gen_validate", &cmd)
        .expect("running schematic-gen validate inside tmux");

    // The `colored` crate may emit bold+color either as separate SGR
    // sequences or as one combined sequence depending on platform/version.
    // Verify both PASS and OK carry equivalent bold-green styling.
    assert_contains_any(
        &buffer,
        &[
            "\x1b[1m\x1b[32m  [PASS]\x1b[0m",
            "\x1b[1;32m  [PASS]\x1b[0m",
        ],
        "validate PASS marker",
    );
    assert_contains_any(
        &buffer,
        &["\x1b[1m\x1b[32m[OK]\x1b[0m", "\x1b[1;32m[OK]\x1b[0m"],
        "validate OK marker",
    );
    // The summary line should mention the API being validated.
    assert_contains(&buffer, "OpenAI", "validate summary mentions API name");
    assert_contains(
        &buffer,
        "All validation checks passed",
        "validate summary text",
    );
}

/// `schematic-gen generate --api all --dry-run` should announce
/// OpenAPI exports using the **module-grouped** filenames
/// (`ollama.json`, `emqx.json`) and never the legacy per-API filenames
/// (`ollamanative.json`, `emqxbasic.json`).
///
/// Captures from a real terminal so we additionally verify the
/// bold-green `[OK]` marker survives the styling pipeline. Skips when
/// tmux is unavailable.
#[test]
fn generate_dry_run_emits_module_grouped_filenames() {
    if !tmux_available() {
        eprintln!("tmux not available on PATH - skipping terminal capture test");
        return;
    }

    // Use a temp dir so paths are absolute and the shell `cd` does not
    // matter.
    let tmp = tempfile::tempdir().expect("tempdir");
    let out_dir = tmp.path().to_string_lossy().into_owned();

    let bin = schematic_gen_bin();
    let cmd = format!(
        "{} generate --api all --output {} --openapi-out {} --postman-out {} --dry-run",
        shell_quote(&bin.to_string_lossy()),
        shell_quote(&out_dir),
        shell_quote(&out_dir),
        shell_quote(&out_dir),
    );
    let buffer = run_in_tmux("schematic_gen_generate", &cmd)
        .expect("running schematic-gen generate inside tmux");

    // Module-grouped filenames must appear in the dry-run report.
    assert_contains(&buffer, "ollama.json", "ollama module artifact");
    assert_contains(&buffer, "emqx.json", "emqx module artifact");

    // Phase 2 regression guard: the legacy per-API filenames must not
    // come back through the user-visible CLI report.
    assert_not_contains(&buffer, "ollamanative.json", "no legacy ollama-native file");
    assert_not_contains(&buffer, "ollamaopenai.json", "no legacy ollama-openai file");
    assert_not_contains(&buffer, "emqxbasic.json", "no legacy emqx-basic file");
    assert_not_contains(&buffer, "emqxbearer.json", "no legacy emqx-bearer file");
    assert_not_contains(
        &buffer,
        "huggingfacehub.json",
        "no legacy huggingface-hub file",
    );
    assert_not_contains(
        &buffer,
        "samsungsmarttv.json",
        "no legacy samsung-smart-tv file",
    );

    // The `[OK]` markers next to "Would export OpenAPI spec" lines
    // must carry the bold-green SGR styling end-to-end through tmux.
    assert_contains_any(
        &buffer,
        &["\x1b[1m\x1b[32m[OK]\x1b[0m", "\x1b[1;32m[OK]\x1b[0m"],
        "OK marker keeps bold-green styling",
    );
}

/// Validating an unknown API should print a red `[ERROR]` marker and
/// list the available APIs. Verifies that error styling survives the
/// terminal pipeline. Skips when tmux is unavailable.
#[test]
fn validate_unknown_api_emits_red_error() {
    if !tmux_available() {
        eprintln!("tmux not available on PATH - skipping terminal capture test");
        return;
    }

    let bin = schematic_gen_bin();
    // `definitely-not-a-real-api` is not in the registry, so the CLI
    // should emit a red `[ERROR]` marker and the words "Unknown API".
    let cmd = format!(
        "{} validate --api definitely-not-a-real-api",
        shell_quote(&bin.to_string_lossy()),
    );
    let buffer = run_in_tmux("schematic_gen_validate_err", &cmd)
        .expect("running schematic-gen validate (error path) inside tmux");

    // Bold-red `[ERROR]` marker.
    assert_contains_any(
        &buffer,
        &["\x1b[1m\x1b[31m[ERROR]\x1b[0m", "\x1b[1;31m[ERROR]\x1b[0m"],
        "ERROR marker uses bold-red styling",
    );
    assert_contains(&buffer, "Unknown API", "error message mentions Unknown API");
}
