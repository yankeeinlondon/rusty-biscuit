//! CLI terminal-detection evidence for Findings 3 & 21 (2026-07-12
//! performance review follow-up).
//!
//! Finding 3: `md compose` must detect the terminal **once** per invocation
//! (the `term_cell` `OnceCell` in `commands/compose.rs`), shared across the
//! verbose summary, `-vv` perf metrics, and warnings-footer render branches —
//! not one fresh `Terminal::default()` per branch. We prove this by *counting
//! the emitted detection events* (the `biscuit_terminal::terminal` "Terminal
//! detected" debug span), not by inferring from equal rendered output.
//!
//! Finding 21: for fully redirected (non-TTY) output, macOS appearance
//! discovery must not fork the `defaults read -g AppleInterfaceStyle`
//! subprocess. We prove this by putting a sentinel-writing `defaults` shim
//! first on the child's PATH and asserting it is never executed.
//!
//! Both cases are **piped / redirected** and spawn an ordinary child process,
//! so they are L1 (no PTY). The interactive (PTY) OSC evidence lives in
//! biscuit-terminal's `level2_terminal_osc_cache.rs`.

mod common;

use common::md_cmd;

/// A document that composes successfully but emits one compose **warning**
/// (`{{ 1 + }}` fails to parse) so the warnings-footer render branch also runs.
/// Combined with `-vv --perf`, this exercises the verbose, perf, and warning
/// branches that each call `term_cell.get_or_init`.
const DOC_WITH_WARNING: &str = "# Title\n\nHello {{ 1 + }} world.\n";

fn count_terminal_detections(stderr: &[u8]) -> usize {
    String::from_utf8_lossy(stderr)
        .lines()
        .filter(|line| line.contains("Terminal detected"))
        .count()
}

/// One `md compose -vv --perf` invocation performs exactly one terminal
/// detection even though three human-render branches each request the shared
/// terminal. (Finding 3.)
#[test]
fn compose_verbose_perf_performs_single_terminal_detection() {
    let doc = common::md_file(DOC_WITH_WARNING);

    let output = md_cmd()
        .args(["compose"])
        .arg(doc.path())
        .args(["-vv", "--perf"])
        // Surface the `biscuit_terminal::terminal` detection span so we can
        // count constructions. `--debug` is ignored when RUST_LOG is set.
        .env("RUST_LOG", "biscuit_terminal=debug")
        .output()
        .expect("failed to run md compose");

    assert!(
        output.status.success(),
        "compose should succeed; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // The warning branch must actually have run (so all three get_or_init call
    // sites were exercised, not just verbose+perf).
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("failed to parse"),
        "expected the compose warning branch to render; stderr: {stderr}"
    );

    let detections = count_terminal_detections(&output.stderr);
    assert_eq!(
        detections, 1,
        "expected exactly one terminal detection across verbose+perf+warning \
         branches, saw {detections}; stderr: {stderr}"
    );
}

/// On macOS, fully redirected (non-TTY) output must not fork the
/// `defaults read -g AppleInterfaceStyle` appearance probe. A sentinel-writing
/// `defaults` shim placed first on the child's PATH proves the subprocess is
/// never spawned in the redirected path. (Finding 21.)
///
/// The shim would fire if the `is_tty()` guard in
/// `biscuit-terminal ::discovery::detection::color::detect_color_mode` were
/// removed, so this is a real regression guard, not a tautology. `DARK_MODE`
/// is intentionally left unset so nothing short-circuits before the guarded
/// branch. PATH is set only on the child command; the test process env is not
/// mutated, so no serialization is required.
#[cfg(target_os = "macos")]
#[test]
fn compose_redirected_does_not_spawn_appearance_defaults() {
    use std::os::unix::fs::PermissionsExt;

    let shim_dir = tempfile::tempdir().expect("shim tempdir");
    let sentinel = shim_dir.path().join("defaults-was-invoked");
    let shim = shim_dir.path().join("defaults");

    // A `defaults` stand-in that records any invocation, then behaves like a
    // no-match (exit 1) so behavior is unchanged if it *were* (wrongly) called.
    std::fs::write(
        &shim,
        format!(
            "#!/bin/sh\n/usr/bin/touch {:?}\nexit 1\n",
            sentinel.display()
        ),
    )
    .expect("write shim");
    std::fs::set_permissions(&shim, std::fs::Permissions::from_mode(0o755))
        .expect("chmod shim");

    let doc = common::md_file(DOC_WITH_WARNING);

    let existing_path = std::env::var("PATH").unwrap_or_default();
    let child_path = format!("{}:{}", shim_dir.path().display(), existing_path);

    let output = md_cmd()
        .args(["compose"])
        .arg(doc.path())
        .args(["-vv", "--perf"])
        .env("PATH", child_path)
        .env_remove("DARK_MODE")
        .output()
        .expect("failed to run md compose");

    assert!(
        output.status.success(),
        "compose should succeed; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !sentinel.exists(),
        "redirected `md compose` forked the macOS `defaults` appearance probe \
         (sentinel {:?} was created) — the non-TTY guard regressed",
        sentinel.display()
    );
}

/// Non-macOS platforms never fork the macOS `defaults` appearance probe; record
/// a clean unsupported/skip disposition so the suite is honest cross-platform.
#[cfg(not(target_os = "macos"))]
#[test]
fn compose_redirected_appearance_defaults_probe_is_macos_only() {
    eprintln!(
        "compose_redirected_does_not_spawn_appearance_defaults: skipped — the \
         `defaults read -g AppleInterfaceStyle` appearance probe is macOS-only"
    );
}
