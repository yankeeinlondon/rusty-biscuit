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
//! subprocess. We prove this by putting a `defaults` shim first on the child's
//! PATH and asserting that shim is never invoked *with the appearance-probe
//! argv*. `defaults` has other legitimate consumers on the same code path, so
//! the shim discriminates on arguments rather than on being run at all.
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
/// `defaults read -g AppleInterfaceStyle` appearance probe. A `defaults` shim
/// placed first on the child's PATH records a sentinel when — and only when —
/// that specific probe runs. (Finding 21.)
///
/// The shim must discriminate on argv: `defaults` has other, unrelated
/// consumers in the same `Terminal::default()` construction. Notably
/// `discovery::fonts::iterm2` reads `com.googlecode.iterm2 New Bookmarks` for
/// font name/size whenever `TERM_PROGRAM` says iTerm2, and those reads are
/// deliberately not TTY-gated (`fonts::nerd` consumes `font_name()` to pick
/// glyphs even in redirected output). A shim keyed on *any* `defaults`
/// invocation therefore fires on an iTerm2 host and reports an appearance-guard
/// regression that did not happen — the failure would track the developer's
/// `TERM_PROGRAM` rather than the code under test.
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
    let sentinel = shim_dir.path().join("appearance-probe-was-invoked");
    let shim = shim_dir.path().join("defaults");

    // Every invocation exits 1 (a `defaults` no-match) so unrelated callers see
    // unchanged behavior whether or not they matched the sentinel.
    std::fs::write(
        &shim,
        format!(
            "#!/bin/sh\n\
             if [ \"$1\" = read ] && [ \"$2\" = -g ] && [ \"$3\" = AppleInterfaceStyle ]; then\n\
             \x20 /usr/bin/touch {:?}\n\
             fi\n\
             exit 1\n",
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
        "redirected `md compose` ran `defaults read -g AppleInterfaceStyle` \
         (sentinel {:?} was created) — the non-TTY guard on the macOS \
         appearance probe regressed. Only that argv trips this sentinel; other \
         `defaults` consumers (e.g. the iTerm2 font reads) are ignored.",
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
