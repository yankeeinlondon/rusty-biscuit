//! Parity guard for top-level prose wrapping on the render-tree terminal path.
//!
//! Stage 1 of the decorated-layout tree cutover added top-level paragraph
//! word-wrapping to the shared tree terminal renderer (gated by
//! `TerminalRenderContext::wrap_prose`, which the darkmatter terminal entry
//! point turns on). This test pins that a long top-level paragraph wraps to the
//! pinned content width through the tree path and that the **visible** (ANSI-
//! stripped) wrapped layout is consistent between the direct
//! `render_tree_terminal` entry point and the public `Markdown::as_terminal`.

use darkmatter::markdown::Markdown;
use darkmatter::markdown::output::{ColorDepth, TerminalOptions};
use darkmatter::markdown::render_tree::render_tree_terminal;

/// Strips ANSI escape sequences so the visible column layout can be compared
/// across the two renderers (which emit different SGR byte streams).
fn strip_ansi(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            // CSI: ESC [ ... <final byte in @..~>
            if chars.peek() == Some(&'[') {
                chars.next();
                for d in chars.by_ref() {
                    if ('@'..='~').contains(&d) {
                        break;
                    }
                }
            }
            continue;
        }
        out.push(c);
    }
    out
}

#[test]
fn long_top_level_paragraph_wraps_to_content_width_through_tree() {
    let para = "word ".repeat(40);
    let md: Markdown = format!("{}\n", para.trim()).into();

    let mut opts = TerminalOptions::default();
    opts.max_width = Some(40);
    opts.color_depth = Some(ColorDepth::TrueColor);

    let tree = render_tree_terminal(&md, &opts).expect("tree render").output;
    let plain = strip_ansi(&tree);

    let lines: Vec<&str> = plain.lines().filter(|l| !l.trim().is_empty()).collect();
    assert!(
        lines.len() > 1,
        "long paragraph must wrap onto multiple lines through the tree; got:\n{plain}",
    );
    let max_len = lines
        .iter()
        .map(|l| l.chars().count())
        .max()
        .unwrap_or(0);
    assert!(
        max_len <= 40,
        "wrapped lines must be capped to the 40-col content width; got max={max_len}:\n{plain}",
    );
}

/// The public `Markdown::as_terminal` entry and the direct
/// `render_tree_terminal` call must agree on the two load-bearing properties:
/// every visible line stays within the content width, and no word is lost or
/// reordered (the concatenated words are equal).
///
/// Exact line-break columns are NOT asserted — both paths reuse the same shared
/// `wrap_lines`/`WrapProse` primitive (also driving lists and two-column
/// layout), so the comparison guards against the public entry point diverging
/// from the direct call rather than against any second renderer.
#[test]
fn tree_prose_wrap_matches_public_entry_within_width_and_lossless() {
    let para = "alpha beta gamma delta epsilon zeta eta theta iota kappa lambda mu nu xi omicron pi rho sigma tau upsilon phi chi psi omega";
    let md: Markdown = format!("{para}\n").into();

    let mut opts = TerminalOptions::default();
    opts.max_width = Some(40);
    opts.color_depth = Some(ColorDepth::TrueColor);

    let tree = render_tree_terminal(&md, &opts).expect("tree render").output;
    let public = md.as_terminal(opts).expect("public render");

    let tree_words: Vec<String> = strip_ansi(&tree).split_whitespace().map(String::from).collect();
    let public_words: Vec<String> =
        strip_ansi(&public).split_whitespace().map(String::from).collect();
    assert_eq!(
        tree_words, public_words,
        "direct and public entry points must preserve the same words in order",
    );

    let max_len = strip_ansi(&tree)
        .lines()
        .map(|l| l.trim_end().chars().count())
        .max()
        .unwrap_or(0);
    assert!(max_len <= 40, "every wrapped line must stay within 40 cols; got {max_len}");

    // Both renderers wrap (more than one visible line) for this 123-col input.
    let tree_visible = strip_ansi(&tree)
        .lines()
        .filter(|l| !l.trim().is_empty())
        .count();
    assert!(tree_visible > 1, "tree must wrap the long paragraph");
}

/// Block quotes rendered through the tree terminal path must carry the
/// darkmatter quarter-block bar (`▐`), not the shared `BlockQuote` component's
/// `│` border. This is the Stage 3 glyph-parity guard: the darkmatter terminal
/// entry point sets `blockquote_prefix = "▐   "`, which the decorated-layout L2
/// guards pin.
#[test]
fn tree_blockquote_uses_legacy_quarter_block_bar() {
    let md: Markdown =
        "> This is a fairly long quoted paragraph that should wrap onto a second visible line once the 40-col width forces it below the page width.\n".into();

    let mut opts = TerminalOptions::default();
    opts.max_width = Some(40);
    opts.color_depth = Some(ColorDepth::TrueColor);

    let tree = render_tree_terminal(&md, &opts).expect("tree render").output;
    let plain = strip_ansi(&tree);

    let bar_lines: Vec<&str> = plain.lines().filter(|l| l.contains('▐')).collect();
    assert!(
        bar_lines.len() >= 2,
        "blockquote must carry the ▐ bar and wrap; got:\n{plain}",
    );
    assert!(
        !plain.contains('│'),
        "tree blockquote must not use the component │ border on the darkmatter path:\n{plain}",
    );
    let max_len = bar_lines
        .iter()
        .map(|l| l.trim_end().chars().count())
        .max()
        .unwrap_or(0);
    assert!(max_len <= 40, "blockquote lines must stay within 40 cols; got {max_len}");
}
