use std::path::PathBuf;

use biscuit_terminal::discovery::detection::ColorDepth;

use super::*;
use crate::generate::CheckOutcome;

/// A no-color, non-TTY terminal — the degradation target the UX tests and
/// piped consumers see.
fn plain() -> Terminal {
    let mut term = Terminal::new_optimistic(120);
    term.color_depth = ColorDepth::None;
    term.is_tty = false;
    term.osc_link_support = false;
    term
}

/// A full-color terminal — the styled path.
fn colored() -> Terminal {
    Terminal::new_optimistic(120)
}

#[test]
fn plain_provider_clean_is_bare_text() {
    let out = provider_check(&plain(), "claude", &CheckOutcome::Clean);
    assert_eq!(out, "claude: clean (inputs match the committed data.rs)\n");
}

#[test]
fn colored_provider_clean_carries_sgr_but_same_visible_text() {
    let out = provider_check(&colored(), "claude", &CheckOutcome::Clean);
    assert!(out.contains("\x1b["), "color mode must emit SGR: {out:?}");
    assert!(out.contains("clean"));
    // The status keyword is styled, never the surrounding prose.
    assert!(out.contains("claude: "));
}

#[test]
fn plain_artifact_missing_preserves_hint_text() {
    let outcome = CheckOutcome::MissingCommitted {
        path: PathBuf::from("/x/catalog.json"),
    };
    let out = artifact_check(&plain(), "catalog.json", "(inputs match the committed catalog)", &outcome);
    assert_eq!(
        out,
        "catalog.json missing at /x/catalog.json — run `claudine-gen generate`\n"
    );
}

#[test]
fn plain_drift_diff_lists_details_without_ansi() {
    let outcome = CheckOutcome::Drift {
        details: vec!["line 1: a vs b".to_string(), "line 2: c vs d".to_string()],
    };
    let out = provider_check(&plain(), "kimi", &outcome);
    assert!(!out.contains("\x1b["), "plain mode must not emit ANSI: {out:?}");
    assert!(out.starts_with("kimi: DRIFT — regenerate with `claudine-gen generate`:\n"));
    assert!(out.contains("line 1: a vs b"));
    assert!(out.contains("line 2: c vs d"));
}

/// A `<placeholder>` token in dynamic text renders literally (never
/// swallowed as Prose markup), in both plain and color modes.
#[test]
fn angle_placeholder_survives_escaping() {
    let outcome = CheckOutcome::MissingCommitted {
        path: PathBuf::from("/repo/<slug>/data.rs"),
    };
    let plain_out = artifact_check(&plain(), "x", "(clean)", &outcome);
    assert!(plain_out.contains("/repo/<slug>/data.rs"), "{plain_out}");
    let color_out = artifact_check(&colored(), "x", "(clean)", &outcome);
    assert!(
        color_out.contains("<slug>"),
        "placeholder must survive markup parsing: {color_out:?}"
    );
}

#[test]
fn roster_lines_match_both_shapes() {
    assert_eq!(
        roster(&plain(), &[]),
        "roster: every active entry has a wired Provider variant\n"
    );
    assert_eq!(
        roster(&plain(), &["kilo".to_string(), "pi".to_string()]),
        "roster: kilo, pi researched but not wired (no Provider variant) — informational\n"
    );
}

#[test]
fn wrote_line_keeps_trailing_space_marker() {
    // The `clean_area_generates_nothing` UX test keys off the literal
    // "wrote " substring, so the marker must survive plain rendering.
    let out = wrote(&plain(), &PathBuf::from("/x/data.rs"));
    assert!(out.contains("wrote "), "{out:?}");
}
