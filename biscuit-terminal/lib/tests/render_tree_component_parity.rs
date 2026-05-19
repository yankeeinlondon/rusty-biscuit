//! Parity gate between a component's bespoke renderer and the render-tree
//! routed renderer ("Flow B").
//!
//! A [`BlockQuote`] can reach the terminal two ways:
//!
//! - **Bespoke** — [`BlockQuote`]'s own [`TerminalRenderable`] impl
//!   (`quote.render(&term)`). This is the original, battle-tested renderer.
//! - **Tree-routed** — [`TreeComponent::new(quote)`], whose
//!   [`TerminalRenderable`] impl calls `quote.render_tree()` and then folds
//!   the projected [`RenderNode`] through `render_terminal_node`.
//!
//! This integration test renders a spread of [`BlockQuote`] cases both ways
//! with the *same* [`Terminal`] and asserts that the two paths preserve the
//! same **semantic content**.
//!
//! ## What parity means here
//!
//! The two paths are **different renderers** and their output is *not*
//! byte-identical. Asserting byte-equality would only prove the two
//! implementations are the same implementation. Every assertion below checks
//! a **semantic invariant**: visible quote text and attribution text must
//! survive on *both* paths. Pure formatting differences never fail the test.
//!
//! ## Accepted divergences
//!
//! The following differences between the bespoke and tree-routed paths are
//! expected and deliberately NOT asserted against:
//!
//! - **Border treatment.** Both paths now prefix every content line with a
//!   `│ ` left border: the bespoke renderer draws it directly, and the
//!   tree-routed path carries the component's declared `Style` (left border
//!   plus any colors) on the projected `NodeKind::BlockQuote`, which the
//!   terminal tree renderer lowers through `render_styled`. The blank-border
//!   separator before an attribution still differs between paths and is not
//!   asserted. The dedicated `render_tree_carries_blockquote_style` test below
//!   checks the styled SGR/glyph output on the tree path directly.
//! - **Attribution placement.** Bespoke renders the attribution as a final
//!   `│ — {attribution}` line inside the same quote. The tree projection
//!   emits the attribution as a *separate child paragraph* (`— {attribution}`)
//!   that the renderer joins with a blank line and re-borders. The visible
//!   text `— {attribution}` survives on both paths, but its layout differs.
//! - **`Prose` styling is flattened.** `BlockQuote::render_tree()` is a
//!   deliberately lossy projection: a `Prose` content component is rendered
//!   optimistically and its ANSI escapes are stripped to plain text (see
//!   `BlockQuote::plain_text`). The bespoke path keeps the `Prose` component
//!   live and renders its inline styling (bold/italic/color) as real SGR
//!   sequences. After ANSI-stripping, the *words* match; the styling does
//!   not, and is not asserted.
//! - **Wrapping and color.** Wrap columns, the left-block color, and the
//!   palette differ between paths and are formatting concerns, not semantics.
//!
//! A token present in one path's output but missing from the other fails the
//! test with a message naming the case, the token, and the path that dropped
//! it. Should a future change to `render_tree()`, `TreeComponent`, or the
//! terminal renderer drop a semantic token, the invariant assertions here
//! fail and name the offending token.

use biscuit_terminal::components::block_quote::BlockQuote;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::{RenderableTerminalContent, TerminalRenderable};
use biscuit_terminal::render_tree::TreeComponent;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::escape_codes::strip_escape_codes;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Builds the fixed [`Terminal`] used for every case.
///
/// A deterministic optimistic terminal keeps the comparison independent of
/// the host environment so the test behaves identically in CI and locally.
fn test_terminal() -> Terminal {
    Terminal::new_optimistic(80)
}

/// Collapses all whitespace runs to single spaces for tolerant comparison.
///
/// The two paths legitimately differ in wrapping and border whitespace; the
/// semantic question is only whether a token's words are present.
fn normalize(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Renders `quote` both ways and asserts every `tokens` entry survives on
/// both paths.
///
/// The bespoke path is `quote.render(&term)`; the tree-routed path is
/// `TreeComponent::new(quote.clone()).render(&term)`. Both use the *same*
/// fixed [`Terminal`]. ANSI/OSC escapes are stripped before comparison so
/// only visible text is checked.
///
/// ## Panics
///
/// Panics naming the `case`, the token, and the offending path if a token is
/// present in one path's output but missing from the other — a genuine
/// semantic parity gap.
fn assert_component_parity(case: &str, quote: BlockQuote, tokens: &[&str]) {
    let term = test_terminal();

    let bespoke = strip_escape_codes(quote.render(&term));
    let tree_routed = strip_escape_codes(TreeComponent::new(quote.clone()).render(&term));

    let bespoke_norm = normalize(&bespoke);
    let tree_norm = normalize(&tree_routed);

    for token in tokens {
        let needle = normalize(token);
        let in_bespoke = bespoke_norm.contains(&needle);
        let in_tree = tree_norm.contains(&needle);

        assert!(
            in_bespoke || in_tree,
            "[{case}] token {token:?} is absent from BOTH paths — fixture or extraction bug\n\
             bespoke: {bespoke:?}\ntree-routed: {tree_routed:?}",
        );
        assert!(
            in_bespoke,
            "[{case}] semantic parity gap: token {token:?} present in the TREE-ROUTED \
             output but MISSING from the BESPOKE renderer\n\
             bespoke: {bespoke:?}",
        );
        assert!(
            in_tree,
            "[{case}] semantic parity gap: token {token:?} present in the BESPOKE \
             output but MISSING from the TREE-ROUTED path\n\
             tree-routed: {tree_routed:?}",
        );
    }
}

// ---------------------------------------------------------------------------
// Per-case parity tests
// ---------------------------------------------------------------------------

/// A plain single-line quote built with `BlockQuote::from(&str)`.
#[test]
fn render_tree_component_parity_plain() {
    let quote = BlockQuote::from("The only way to do great work is to love what you do.");
    assert_component_parity(
        "plain",
        quote,
        &["The only way to do great work is to love what you do."],
    );
}

/// A quote with an attribution built with `BlockQuote::new(content, Some(_))`.
///
/// The attribution text `— Shakespeare` must survive on both paths even
/// though the bespoke path renders it as a final bordered line and the tree
/// path renders it as a separate child paragraph.
#[test]
fn render_tree_component_parity_with_attribution() {
    let quote = BlockQuote::new(
        RenderableTerminalContent::from("To be, or not to be, that is the question."),
        Some("Shakespeare"),
    );
    assert_component_parity(
        "with_attribution",
        quote,
        &[
            "To be, or not to be, that is the question.",
            "— Shakespeare",
        ],
    );
}

/// A multi-line quote: every line's words must survive on both paths.
///
/// The bespoke path keeps the explicit line breaks as separate bordered
/// lines; the tree projection collapses the content into a single paragraph
/// of `Text` that the renderer re-wraps. Word presence is the invariant.
#[test]
fn render_tree_component_parity_multiline() {
    let quote = BlockQuote::from("First line of the quote\nSecond line of the quote\nThird line");
    assert_component_parity(
        "multiline",
        quote,
        &[
            "First line of the quote",
            "Second line of the quote",
            "Third line",
        ],
    );
}

/// A quote built from a `Prose` component (rich inline content).
///
/// `render_tree()` flattens the `Prose` styling to plain text; the bespoke
/// path keeps it live. After ANSI-stripping the *words* match on both paths,
/// which is what is asserted — the styling divergence is accepted.
#[test]
fn render_tree_component_parity_from_prose() {
    let prose = Prose::new("This is <b>bold</b> and <i>italic</i> emphasis text.");
    let quote = BlockQuote::from(prose);
    assert_component_parity(
        "from_prose",
        quote,
        &["This is", "bold", "and", "italic", "emphasis text."],
    );
}

/// A quote built from a `Prose` component that also carries an attribution.
///
/// Exercises both accepted divergences at once: flattened `Prose` styling and
/// attribution rendered as a separate child paragraph.
#[test]
fn render_tree_component_parity_prose_with_attribution() {
    let prose = Prose::new("<red>error</red>: something went <b>wrong</b>");
    let quote = BlockQuote::new(
        RenderableTerminalContent::Component(std::rc::Rc::new(prose)),
        Some("Anonymous"),
    );
    assert_component_parity(
        "prose_with_attribution",
        quote,
        &["error", "something went", "wrong", "— Anonymous"],
    );
}

/// A styled [`BlockQuote`]'s declared [`Style`] survives the render-tree
/// projection: the tree-routed path carries the left border and the text and
/// border colors onto the projected node, and the terminal tree renderer
/// lowers them to real SGR and box-drawing output.
///
/// This is the Level 1 byte/SGR guard the bespoke vs. tree semantic-token
/// comparison above intentionally does not cover.
#[test]
fn render_tree_carries_blockquote_style() {
    use biscuit_terminal::utils::color::{Color, Tailwind};

    let term = test_terminal();
    let quote = BlockQuote::from("Styled quote text")
        .with_text_color(Color::Tailwind(Tailwind::Red500))
        .with_left_block_color(Color::Tailwind(Tailwind::Blue500));

    let tree_routed = TreeComponent::new(quote).render(&term);

    // The left border glyph is drawn from the declared `Style.border`.
    assert!(
        tree_routed.contains('│'),
        "tree-routed styled block quote dropped its left border: {tree_routed:?}"
    );
    // The declared text and border colors lower to truecolor SGR escapes.
    assert!(
        tree_routed.contains("\x1b[38;2;"),
        "tree-routed styled block quote dropped its declared colors: {tree_routed:?}"
    );
    assert!(
        strip_escape_codes(&tree_routed).contains("Styled quote text"),
        "tree-routed styled block quote dropped its text: {tree_routed:?}"
    );
}

/// Sanity check: every case renders through both paths without panicking.
#[test]
fn render_tree_component_parity_all_cases_render() {
    let term = test_terminal();
    let cases: Vec<BlockQuote> = vec![
        BlockQuote::from("plain quote"),
        BlockQuote::new(
            RenderableTerminalContent::from("attributed quote"),
            Some("Author"),
        ),
        BlockQuote::from("line one\nline two"),
        BlockQuote::from(Prose::new("rich <b>content</b>")),
    ];
    for quote in cases {
        let _ = quote.render(&term);
        let _ = TreeComponent::new(quote).render(&term);
    }
}
