use super::support::*;

#[test]
#[serial(level2_terminal)]
fn level2_tree_heading_text_survives_real_terminal() {
    let body = "# Level Two Marker\n\nSecond line of body content.\n";
    let Some((frame, _dir)) = run_in_pane(body, "heading") else {
        return;
    };

    // The heading text must appear in the rendered pane. ANSI styling is
    // applied by the renderer; the harness's `plain` field has it stripped
    // for matching.
    assert!(
        frame.plain.contains("Level Two Marker"),
        "heading text missing from real-terminal capture. plain:\n{}",
        frame.plain
    );
    assert!(
        frame.plain.contains("Second line of body content."),
        "paragraph text missing from real-terminal capture. plain:\n{}",
        frame.plain
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_tree_inline_styles_render_with_ansi_in_real_terminal() {
    let body = "Mixing *emphasis* and **strong** in real terminal output.\n";
    let Some((frame, _dir)) = run_in_pane(body, "inline_styles") else {
        return;
    };

    // The styled words must survive the real-terminal round-trip even
    // though SGR runs surround them. The harness strips ANSI for the
    // `plain` view.
    assert!(
        frame.plain.contains("emphasis"),
        "italic word missing. plain:\n{}",
        frame.plain
    );
    assert!(
        frame.plain.contains("strong"),
        "strong word missing. plain:\n{}",
        frame.plain
    );

    // The raw frame should contain at least one SGR sequence — otherwise
    // we've degraded to a no-color rendering even though the fixture is
    // styled.
    assert!(
        frame.raw.contains("\u{1b}["),
        "expected SGR styling in the raw capture but found none. raw:\n{}",
        frame.raw
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_tree_mark_renders_reverse_video_in_real_terminal() {
    let body = "Plain text with ==marked phrase== inside.\n";
    let Some((frame, _dir)) = run_in_pane_spanned(body, "mark_spanned") else {
        return;
    };

    assert!(
        frame.plain.contains("marked phrase"),
        "mark text missing from real-terminal capture. plain:\n{}",
        frame.plain
    );
    assert!(
        !frame.plain.contains("=="),
        "raw `==` delimiters must not leak through; plain:\n{}",
        frame.plain
    );
    // The reverse-video SGR open code is `ESC [ 7 m`. Accept either the
    // standalone form, a leading-position combined run (`ESC [ 7 ; …`), or
    // a trailing-position combined run (`ESC [ … ; 7 m`) — biscuit-terminal's
    // Prose emitter often collapses a reset and the layer into one sequence
    // like `\x1b[0;7m`.
    let has_reverse = frame.raw.contains("\u{1b}[7m")
        || frame.raw.contains("\u{1b}[7;")
        || frame.raw.contains(";7m")
        || frame.raw.contains(";7;");
    assert!(
        has_reverse,
        "expected reverse-video SGR for mark span; raw:\n{}",
        frame.raw
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_tree_dim_renders_dim_sgr_in_real_terminal() {
    let body = "Plain text with \u{2304}dimmed phrase\u{2304} inside.\n";
    let Some((frame, _dir)) = run_in_pane_spanned(body, "dim_spanned") else {
        return;
    };

    assert!(
        frame.plain.contains("dimmed phrase"),
        "dim text missing from real-terminal capture. plain:\n{}",
        frame.plain
    );
    assert!(
        !frame.plain.contains('\u{2304}'),
        "raw `⌄` delimiters must not leak through; plain:\n{}",
        frame.plain
    );
    // Dim SGR is `ESC [ 2 m`; accept any position within a combined run.
    let has_dim = frame.raw.contains("\u{1b}[2m")
        || frame.raw.contains("\u{1b}[2;")
        || frame.raw.contains(";2m")
        || frame.raw.contains(";2;");
    assert!(
        has_dim,
        "expected dim SGR (\\x1b[2m) for dim span; raw:\n{}",
        frame.raw
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_tree_hr_attributes_render_styled_rule_in_real_terminal() {
    let body = "Lead paragraph.\n\n--- { style: waves }\n\nTrailing paragraph.\n";
    // Render at the text tier: an image-capable terminal would emit the rule
    // as an embedded image the text-capture harness cannot see, so the glyph
    // form is forced to make the styled rule observable.
    let Some((frame, _dir)) = run_in_pane_spanned_text_tier(body, "hr_attributes_spanned") else {
        return;
    };

    assert!(
        frame.plain.contains("Lead paragraph"),
        "lead paragraph missing; plain:\n{}",
        frame.plain
    );
    assert!(
        frame.plain.contains("Trailing paragraph"),
        "trailing paragraph missing; plain:\n{}",
        frame.plain
    );
    assert!(
        !frame.plain.contains("style: waves"),
        "raw HR markdown source leaked through; plain:\n{}",
        frame.plain
    );

    // Isolate the rule line and require it to be a contiguous run of the
    // waves glyph — `≋` (U+224B) in Unicode-capable terminals, `~` in ASCII
    // fallback. A plain (dashed) rule renders `─` (U+2500) or `-`, so a line
    // made entirely of waves glyphs proves the renderer honored `style: waves`
    // (normalized to the `kind` hint) instead of falling back to the default.
    //
    // A bare `frame.plain.contains('~')` was a false-positive risk: a stray
    // `~` anywhere in the pane (a shell prompt, a `~/path` cwd) satisfied it
    // with no styled rule present. Matching a whole line of glyphs excludes
    // those, and the length floor excludes a lone prompt tilde.
    let is_waves_glyph = |c: char| c == '\u{224B}' || c == '~';
    let waves_rule_line = frame.plain.lines().find(|line| {
        let trimmed = line.trim();
        trimmed.chars().count() >= 10 && trimmed.chars().all(is_waves_glyph)
    });
    assert!(
        waves_rule_line.is_some(),
        "expected a styled waves rule line (a run of `≋` or `~`); plain:\n{}",
        frame.plain
    );
}
