//! The generic `terminal`/`plain` fold (`render_terminal_output`): width
//! semantics, the plain profile, the non-clickable link and image rule, and
//! OSC 8 link metadata.

use std::cell::RefCell;
use std::rc::Rc;

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::render_tree::{
    TerminalRenderOptions, render_terminal_document, render_terminal_output,
};
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::block_constraint::visible_width;
use biscuit_terminal::utils::layout::WordWrap;
use renderable::color::{ColorDepth, ColorMode, TerminalCodeContext};
use renderable::output::link_metadata::{LinkMetadata, QueryParams};
use renderable::output::{OutputOptions, TerminalProfile, Width};
use renderable::tree::{
    CodeRenderer, Document, HeadingDepth, NodeAttrs, NodeKind, RenderNode, RenderStrictness,
    TreeRenderable,
};

const LONG_CODE: &str = "let value = compute(alpha, beta, gamma, delta, epsilon, zeta, eta, theta, iota);";
const LONG_URL: &str = "https://example.com/a/very-long/path/to/the-documentation/page.html";

fn styled_profile() -> TerminalProfile {
    TerminalProfile {
        color_depth: ColorDepth::TrueColor,
        color_mode: ColorMode::Dark,
        text_color: None,
        background_color: None,
        is_tty: true,
        hyperlinks: true,
        italics: true,
        underline: true,
        graphics: false,
        unicode: true,
    }
}

fn plain(width: Width) -> OutputOptions {
    OutputOptions::new(width, TerminalProfile::plain())
}

fn render(doc: &Document, options: &OutputOptions) -> String {
    render_terminal_output(doc, options, RenderStrictness::Warn).expect("render").output
}

fn doc(children: Vec<RenderNode>) -> Document {
    Document::new(RenderNode::root(children))
}

fn cell(text: &str) -> RenderNode {
    RenderNode::table_cell(vec![RenderNode::text(text)])
}

/// One of each element the plain rules name.
fn corpus() -> Document {
    doc(vec![
        RenderNode::heading(HeadingDepth::new(2).unwrap(), vec![RenderNode::text("Setup")]),
        RenderNode::paragraph(vec![
            RenderNode::text("first line"),
            RenderNode::soft_break(),
            RenderNode::text("second line with "),
            RenderNode::strong(vec![RenderNode::text("bold")]),
            RenderNode::text(" and "),
            RenderNode::emphasis(vec![RenderNode::text("italic")]),
            RenderNode::text(" and "),
            RenderNode::inline_code("just install"),
            RenderNode::text(" then some more words so that the paragraph is long enough to wrap at forty."),
        ]),
        RenderNode::code(Some("yaml".to_string()), None, format!("key: value\n# {LONG_CODE}")),
        RenderNode::table(
            Vec::new(),
            vec![
                RenderNode::table_row(vec![cell("Name"), cell("Type")]),
                RenderNode::table_row(vec![cell("id"), cell("ulid")]),
            ],
        ),
        RenderNode::block_quote(vec![RenderNode::paragraph(vec![RenderNode::text(
            "Quoted text keeps a visible marker.",
        )])]),
        RenderNode::thematic_break(),
        RenderNode::list(
            false,
            None,
            vec![
                RenderNode::list_item(None, vec![RenderNode::paragraph(vec![RenderNode::text("one")])]),
                RenderNode::list_item(None, vec![RenderNode::paragraph(vec![RenderNode::text("two")])]),
            ],
        ),
        RenderNode::paragraph(vec![
            RenderNode::text("See "),
            RenderNode::link("https://example.com/docs", None, vec![RenderNode::text("the docs")]),
            RenderNode::text(" and "),
            RenderNode::image("diagram.png", None, "a diagram"),
        ]),
        RenderNode::disclosure(
            vec![RenderNode::text("More details")],
            vec![RenderNode::paragraph(vec![RenderNode::text("Hidden body text.")])],
            None,
        ),
    ])
}

fn assert_plain_invariants(out: &str, context: &str) {
    assert!(!out.contains('\x1b'), "{context}: ESC byte in {out:?}");
    for line in out.lines() {
        assert_eq!(line, line.trim_end(), "{context}: trailing whitespace in {line:?}");
    }
}

#[test]
fn plain_corpus_follows_the_plain_rules_at_every_width() {
    for width in [Width::Columns(40), Width::Columns(80), Width::Unlimited] {
        let out = render(&corpus(), &plain(width));
        let context = format!("{width:?}");
        assert_plain_invariants(&out, &context);
        let lines: Vec<&str> = out.lines().collect();

        assert_eq!(lines[0], "Setup", "{context}: heading without `#`: {out}");
        assert!(!out.contains("**") && out.contains("bold") && out.contains("italic"), "{context}: {out}");
        assert!(out.contains("`just install`"), "{context}: inline code keeps backticks: {out}");
        assert!(lines.contains(&"```yaml"), "{context}: fence with language: {out}");
        assert!(lines.contains(&format!("# {LONG_CODE}").as_str()), "{context}: long code line whole: {out}");
        assert!(lines.iter().any(|l| l.starts_with('┌')), "{context}: box-drawn table: {out}");
        assert!(lines.contains(&"> Quoted text keeps a visible marker."), "{context}: {out}");
        assert!(lines.contains(&"---"), "{context}: plain rule: {out}");
        assert!(lines.contains(&"- one"), "{context}: list: {out}");
        assert!(out.contains("the docs") && !out.contains("https://example.com/docs"), "{context}: {out}");
        assert!(out.contains("IMAGE[a diagram]"), "{context}: {out}");
        let summary = lines.iter().position(|l| *l == "More details").expect("summary line");
        assert!(
            lines[summary + 1..].contains(&"Hidden body text."),
            "{context}: disclosure body beneath its summary, no bar: {out}"
        );
    }
}

#[test]
fn plain_prose_wraps_at_a_column_width() {
    let out = render(&corpus(), &plain(Width::Columns(40)));
    for line in out.lines().filter(|l| !l.starts_with("# ") && !l.contains("compute(")) {
        assert!(visible_width(line) <= 40, "{line:?} exceeds 40 columns");
    }
}

#[test]
fn unlimited_joins_soft_breaks_and_never_fills() {
    let out = render(&corpus(), &plain(Width::Unlimited));
    assert!(
        out.lines().any(|l| l.starts_with("first line second line with bold")),
        "soft break joined, paragraph unwrapped: {out}"
    );
    let longest_line = out.lines().map(visible_width).max().unwrap_or(0);
    assert!(longest_line < 200, "nothing pads to a width: {longest_line}");
    assert!(out.len() < 2_000, "output is about the size of its input: {}", out.len());
}

#[test]
fn source_keeps_authored_soft_breaks() {
    let out = render(&corpus(), &plain(Width::Source));
    assert!(out.lines().any(|l| l == "first line"), "{out}");
}

#[test]
fn styled_rule_is_three_glyphs_unlimited_and_full_width_at_columns() {
    let rule = doc(vec![RenderNode::thematic_break()]);
    let unlimited = render(&rule, &OutputOptions::new(Width::Unlimited, styled_profile()));
    assert_eq!(visible_width(unlimited.trim_end()), 3, "{unlimited:?}");
    let bounded = render(&rule, &OutputOptions::new(Width::Columns(30), styled_profile()));
    assert_eq!(visible_width(bounded.trim_end()), 30, "{bounded:?}");
    assert_eq!(render(&rule, &plain(Width::Columns(30))), "---");
}

#[test]
fn styled_code_is_cut_at_the_width_and_plain_code_is_whole() {
    let code = doc(vec![RenderNode::code(Some("rust".to_string()), None, LONG_CODE)]);
    let styled = render(&code, &OutputOptions::new(Width::Columns(20), styled_profile()));
    assert!(styled.lines().all(|l| visible_width(l) <= 20), "{styled:?}");
    assert!(!styled.contains(LONG_CODE), "{styled:?}");

    let whole = render(&code, &plain(Width::Columns(20)));
    assert!(whole.lines().any(|l| l == LONG_CODE), "{whole:?}");
    assert_eq!(whole.lines().next(), Some("```rust"));
}

/// A code hook that records the width it was handed.
#[derive(Default)]
struct RecordingCode(RefCell<Vec<(Width, renderable::color::ColorDepth)>>);

impl CodeRenderer for RecordingCode {
    fn render_terminal_code(
        &self,
        _lang: Option<&str>,
        value: &str,
        _meta: Option<&str>,
        _attrs: &NodeAttrs,
        context: TerminalCodeContext,
    ) -> Option<String> {
        self.0.borrow_mut().push((context.width(), context.color_depth()));
        Some(format!("\x1b[1m{value}\x1b[0m   "))
    }

    fn render_browser_code(
        &self,
        _lang: Option<&str>,
        _value: &str,
        _meta: Option<&str>,
        _attrs: &NodeAttrs,
    ) -> Option<renderable::browser::fragment::BrowserFragment<renderable::browser::fragment::Ready>>
    {
        None
    }
}

#[test]
fn a_code_hook_sees_the_unbounded_width_and_a_plain_render_stays_escape_free() {
    let hook = Rc::new(RecordingCode::default());
    let code = doc(vec![RenderNode::code(None, None, "x = 1")]);

    let mut styled = TerminalRenderOptions::from_output_options(
        &OutputOptions::new(Width::Unlimited, styled_profile()),
        RenderStrictness::Warn,
    )
    .with_code_renderer(hook.clone());
    render_terminal_document(&code, &styled).expect("styled");
    assert_eq!(hook.0.borrow()[0], (Width::Unlimited, ColorDepth::TrueColor));

    // A hook that ignores the colorless context is still normalized away.
    styled.pin_plain();
    let out = render_terminal_document(&code, &styled).expect("plain").output;
    assert_eq!(hook.0.borrow()[1].1, ColorDepth::None);
    assert_eq!(out, "x = 1");
}

#[test]
fn plain_stays_pinned_when_capabilities_are_set_afterwards() {
    let mut options = TerminalRenderOptions::from_output_options(
        &OutputOptions::new(Width::Columns(80), styled_profile()),
        RenderStrictness::Warn,
    );
    options.pin_plain();
    // A later caller turning capabilities back on cannot restore escapes.
    options.context.hyperlinks = true;
    options.context.terminal.osc_link_support = true;
    options.context.terminal.color_depth = biscuit_terminal::discovery::detection::ColorDepth::TrueColor;
    let out = render_terminal_document(&corpus(), &options).expect("render").output;
    assert_plain_invariants(&out, "pinned");
}

fn link_doc(text: &str, url: &str) -> Document {
    doc(vec![RenderNode::paragraph(vec![RenderNode::link(url, None, vec![RenderNode::text(text)])])])
}

#[test]
fn non_clickable_links_and_images_follow_the_verbosity_rule() {
    let quiet = plain(Width::Columns(80));
    let verbose = plain(Width::Columns(80)).with_verbosity(1);
    let url = "https://example.com/x";

    assert_eq!(render(&link_doc("text", url), &quiet), "text");
    assert_eq!(render(&link_doc("text", url), &verbose), format!("text <{url}>"));
    assert_eq!(render(&link_doc(url, url), &quiet), url);
    assert_eq!(render(&link_doc(url, url), &verbose), url);

    let image = doc(vec![RenderNode::paragraph(vec![RenderNode::image("img/cat.png", None, "a cat")])]);
    assert_eq!(render(&image, &quiet), "IMAGE[a cat]");
    assert_eq!(render(&image, &verbose), "image: a cat <img/cat.png>");

    // A styled terminal without OSC 8 follows the same rule.
    let mut no_osc = styled_profile();
    no_osc.hyperlinks = false;
    let styled = OutputOptions::new(Width::Columns(80), no_osc).with_verbosity(1);
    let out = render(&link_doc("text", url), &styled);
    assert!(!out.contains("\x1b]8;"), "{out:?}");
    assert!(out.ends_with(&format!("text <{url}>")), "{out:?}");
}

#[test]
fn a_printed_url_is_one_unbreakable_word() {
    let paragraph = doc(vec![RenderNode::paragraph(vec![
        RenderNode::text("See the docs "),
        RenderNode::link(LONG_URL, None, vec![RenderNode::text("here")]),
        RenderNode::text(" for details."),
    ])]);
    let out = render(&paragraph, &plain(Width::Columns(40)).with_verbosity(1));
    let bracketed = format!("<{LONG_URL}>");
    assert!(
        out.lines().any(|l| l.trim() == bracketed),
        "the URL overflows on its own line, whole: {out}"
    );
    assert!(!out.contains("-\n"), "never hyphenated: {out}");
    assert!(out.lines().next().unwrap().starts_with("See the docs here"), "{out}");
}

#[test]
fn a_printed_url_survives_outer_prose_wrapping() {
    let term = Terminal::builder().width(30).osc_link_support(false).build();
    let out = Prose::new(format!("Read [{LONG_URL}]({LONG_URL}) now"))
        .with_word_wrap(WordWrap::WrapProse(None, None))
        .render(&term);
    assert!(out.lines().any(|l| l.trim() == LONG_URL), "{out:?}");
    assert!(!out.contains('\u{2063}') && !out.contains('\u{2064}'), "markers removed: {out:?}");
}

#[test]
fn osc8_link_metadata_rides_in_the_params_with_a_bare_uri() {
    let mut link = RenderNode::link(
        "https://example.com/doc.md#setup",
        None,
        vec![RenderNode::text("doc")],
    );
    link.attrs.set_link_metadata(LinkMetadata {
        query: QueryParams::from_iter([("zeta", "z"), ("line", "14"), ("bad", "a:b")]),
    });
    let out = render(
        &doc(vec![RenderNode::paragraph(vec![link])]),
        &OutputOptions::new(Width::Columns(80), styled_profile()),
    );
    assert!(
        out.contains("\x1b]8;line=14:heading=setup:zeta=z;https://example.com/doc.md\x1b\\doc\x1b]8;;\x1b\\"),
        "{out:?}"
    );

    // Without metadata the destination keeps its fragment.
    let out = render(
        &link_doc("doc", "https://example.com/doc.md#setup"),
        &OutputOptions::new(Width::Columns(80), styled_profile()),
    );
    assert!(out.contains("\x1b]8;;https://example.com/doc.md#setup\x1b\\"), "{out:?}");
}

#[test]
fn an_unlimited_table_uses_natural_column_widths() {
    let long = "a cell whose text is much longer than eighty columns so a bounded table would have to wrap it";
    let table = doc(vec![RenderNode::table(
        Vec::new(),
        vec![
            RenderNode::table_row(vec![cell("Key"), cell("Value")]),
            RenderNode::table_row(vec![cell("k"), cell(long)]),
        ],
    )]);
    let out = render(&table, &plain(Width::Unlimited));
    let widths: Vec<u32> = out.lines().map(visible_width).collect();
    assert!(widths.windows(2).all(|w| w[0] == w[1]), "rectangular: {out}");
    assert!(out.lines().any(|l| l.contains(long)), "cell not wrapped: {out}");
    assert_eq!(out.lines().count(), 5, "one line per row plus borders: {out}");

    let bounded = render(&table, &plain(Width::Columns(40)));
    assert!(bounded.lines().all(|l| visible_width(l) <= 40), "{bounded}");
}

fn prose_link(source: &str) -> RenderNode {
    let tree = Prose::new(source).render_tree();
    fn find(node: &RenderNode) -> Option<RenderNode> {
        if matches!(node.kind, NodeKind::Link { .. }) {
            return Some(node.clone());
        }
        node.children().iter().find_map(find)
    }
    find(&tree).expect("a link node")
}

#[test]
fn a_prose_json5_title_becomes_osc8_params() {
    let link = prose_link(r#"[doc](https://example.com/doc.md "{ qp: { line: 14, column: 2 } }")"#);
    let NodeKind::Link { url, title, .. } = &link.kind else { unreachable!() };
    assert_eq!(url, "https://example.com/doc.md");
    assert_eq!(title, &None);
    assert_eq!(link.attrs.link_metadata().unwrap().query.len(), 2);

    let term = Terminal::builder().width(80).osc_link_support(true).build();
    let out = Prose::new(r#"[doc](https://example.com/doc.md "{ qp: { line: 14, column: 2 } }")"#).render(&term);
    assert!(out.contains("\x1b]8;line=14:column=2;https://example.com/doc.md\x1b\\"), "{out:?}");
}

#[test]
fn a_prose_plain_title_stays_a_title() {
    let link = prose_link(r#"[doc](https://example.com/doc.md "Read me")"#);
    let NodeKind::Link { url, title, .. } = &link.kind else { unreachable!() };
    assert_eq!(url, "https://example.com/doc.md");
    assert_eq!(title.as_deref(), Some("Read me"));
    assert!(link.attrs.link_metadata().is_none());

    let term = Terminal::builder().width(80).osc_link_support(false).build();
    let out = Prose::new(r#"[doc](https://example.com/doc.md "Read me")"#).render(&term);
    assert_eq!(out.trim_end(), "doc", "no `[text](url)` fallback: {out:?}");
}

/// A value with only a document, so `render_output` takes the generic fold.
struct GenericFold(Document);

impl renderable::CommandOutput for GenericFold {
    type Data = ();

    fn as_prose(&self) -> Document {
        self.0.clone()
    }

    fn as_data(&self) {}
}

#[test]
fn the_generic_terminal_fold_reports_what_it_could_not_render() {
    use biscuit_terminal::output::{OutputTarget, render_output};

    let lossy = GenericFold(doc(vec![RenderNode::html("<div>raw</div>", true)]));
    for target in [OutputTarget::Plain, OutputTarget::Terminal] {
        let rendered = render_output(&lossy, target, &plain(Width::Columns(40))).expect("render");
        assert_eq!(rendered.diagnostics.len(), 1, "{target}: {rendered:?}");
        assert!(rendered.diagnostics[0].message.contains("raw HTML"), "{rendered:?}");
    }
    let clean = GenericFold(doc(vec![RenderNode::paragraph(vec![RenderNode::text("fine")])]));
    assert!(render_output(&clean, OutputTarget::Plain, &plain(Width::Columns(40))).expect("render").diagnostics.is_empty());
}
