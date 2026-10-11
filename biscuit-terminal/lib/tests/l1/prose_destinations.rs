//! Link destinations through `Prose` and `InlineProse`: the render tree,
//! Markdown, and HTML keep the authored destination, and only the terminal
//! target resolves a file destination to a `file://` URL for OSC 8.
//!
//! Every assertion reads a public result: the tree, a rendered string, or the
//! OSC 8 destination in terminal output.

use std::path::{Path, PathBuf};

use biscuit_terminal::components::inline_content::InlineContent;
use biscuit_terminal::components::prose::{InlineProse, Prose};
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::status::StatusState;
use biscuit_terminal::components::status_block::StatusBlock;
use biscuit_terminal::components::table::{Table, TableCellContent, TableColumn};
use biscuit_terminal::discovery::detection::ColorDepth;
use biscuit_terminal::render_tree::{TerminalRenderOptions, render_terminal_node};
use biscuit_terminal::terminal::Terminal;
use renderable::browser::BrowserRenderable;
use renderable::markdown::MarkdownRenderable;
use renderable::tree::{
    MarkdownRenderOptions, NodeKind, RenderNode, RenderStrictness, TreeRenderable, render_markdown_node,
};

/// A relative destination that names no file in the package directory or the
/// repository root, so it resolves to where it would be.
const MISSING: &str = "prose-destination-missing-fixture.md";

/// Destinations every non-terminal target must keep exactly as authored.
const DESTINATIONS: [&str; 7] = [
    "plan.md",
    "./plan.md",
    "../plan.md",
    "docs/plan.md",
    "/abs/plan.md",
    "https://x.io/plan.md",
    "mailto:a@b.io",
];

/// The two authored link syntaxes, as `(name, source)` for a destination.
fn link_sources(destination: &str) -> [(&'static str, String); 2] {
    [
        ("markdown link", format!("[the plan]({destination})")),
        ("explicit anchor", format!(r#"<a href="{destination}">the plan</a>"#)),
    ]
}

fn osc8_term() -> Terminal {
    Terminal::builder()
        .color_depth(ColorDepth::None)
        .osc_link_support(true)
        .width(200)
        .build()
}

fn fallback_term() -> Terminal {
    Terminal::builder()
        .color_depth(ColorDepth::None)
        .osc_link_support(false)
        .width(200)
        .build()
}

/// Every OSC 8 destination opened in `output`, in order.
fn osc8_destinations(output: &str) -> Vec<String> {
    output
        .split("\x1b]8;;")
        .skip(1)
        .filter_map(|rest| rest.split_once("\x1b\\").map(|(url, _)| url.to_string()))
        .filter(|url| !url.is_empty())
        .collect()
}

/// The single OSC 8 destination in `output`.
fn only_destination(context: &str, output: &str) -> String {
    let destinations = osc8_destinations(output);
    assert_eq!(destinations.len(), 1, "{context}: {output:?}");
    destinations[0].clone()
}

/// The filesystem path a `file://` URL names.
fn file_url_path(context: &str, url: &str) -> PathBuf {
    assert!(url.starts_with("file://"), "{context}: {url}");
    url::Url::parse(url)
        .unwrap_or_else(|e| panic!("{context}: {url}: {e}"))
        .to_file_path()
        .unwrap_or_else(|()| panic!("{context}: {url} is not a file path"))
}

/// Compares two paths that both exist, through symlinks such as macOS's
/// `/var` → `/private/var`.
fn assert_same_file(context: &str, actual: &Path, expected: &Path) {
    assert_eq!(
        biscuit_file::canonicalize_simplified(actual).unwrap(),
        biscuit_file::canonicalize_simplified(expected).unwrap(),
        "{context}"
    );
}

fn cwd() -> PathBuf {
    std::env::current_dir().unwrap()
}

fn link_urls(node: &RenderNode) -> Vec<String> {
    let mut out = Vec::new();
    if let NodeKind::Link { url, .. } = &node.kind {
        out.push(url.clone());
    }
    for child in node.children() {
        out.extend(link_urls(child));
    }
    out
}

// ── portable targets keep the authored destination (AC 1) ───────────────

#[test]
fn inline_prose_acceptance_example_keeps_the_relative_destination() {
    let label = InlineProse::new("Run `md hash` on [the plan](plan.md)");
    assert_eq!(label.render_markdown(), "Run `md hash` on [the plan](plan.md)");
    assert_eq!(
        label.render_html_fragment().render(),
        r#"Run <code>md hash</code> on <a href="plan.md">the plan</a>"#
    );
}

#[test]
fn both_components_keep_every_authored_destination_off_the_terminal() {
    for destination in DESTINATIONS {
        for (syntax, source) in link_sources(destination) {
            let context = format!("{syntax} {destination}");
            let markdown_link = format!("[the plan]({destination})");
            let html_link = format!(r#"<a href="{destination}">the plan</a>"#);

            let inline = InlineProse::new(&source);
            assert_eq!(link_urls(&inline.render_tree()), [destination], "InlineProse tree, {context}");
            assert_eq!(inline.render_markdown(), markdown_link, "InlineProse Markdown, {context}");
            assert_eq!(inline.render_html_fragment().render(), html_link, "InlineProse HTML, {context}");

            let prose = Prose::new(&source);
            assert_eq!(link_urls(&prose.render_tree()), [destination], "Prose tree, {context}");
            assert_eq!(prose.render_markdown().trim_end(), markdown_link, "Prose Markdown, {context}");
            assert_eq!(
                prose.render_html_fragment().render(),
                format!("<p>{html_link}</p>"),
                "Prose HTML, {context}"
            );
        }
    }
}

// ── the terminal resolves file destinations for OSC 8 ────────────────────

/// Terminal output of `source` through both components, labeled.
fn terminal_outputs(source: &str, terminal: &Terminal) -> [(&'static str, String); 2] {
    [
        ("InlineProse", InlineProse::new(source).render(terminal)),
        ("Prose", Prose::new(source).render(terminal)),
    ]
}

#[test]
fn terminal_links_an_existing_relative_file_by_its_file_url() {
    // The package manifest exists in the working directory of every test run.
    for destination in ["Cargo.toml", "./Cargo.toml"] {
        for (syntax, source) in link_sources(destination) {
            for (component, output) in terminal_outputs(&source, &osc8_term()) {
                let context = format!("{component} {syntax} {destination}");
                let url = only_destination(&context, &output);
                assert_same_file(&context, &file_url_path(&context, &url), &cwd().join("Cargo.toml"));
            }
        }
    }
}

#[test]
fn terminal_links_a_missing_relative_file_where_it_would_be() {
    for destination in [MISSING.to_string(), format!("./{MISSING}"), format!("sub/{MISSING}")] {
        for (syntax, source) in link_sources(&destination) {
            for (component, output) in terminal_outputs(&source, &osc8_term()) {
                let context = format!("{component} {syntax} {destination}");
                let url = only_destination(&context, &output);
                assert_eq!(
                    file_url_path(&context, &url),
                    std::path::absolute(cwd().join(&destination)).unwrap(),
                    "{context}"
                );
            }
        }
    }
}

#[test]
fn terminal_keeps_urls_with_a_scheme_unchanged() {
    for destination in ["https://x.io/plan.md", "mailto:a@b.io", "file:///abs/plan.md"] {
        for (syntax, source) in link_sources(destination) {
            for (component, output) in terminal_outputs(&source, &osc8_term()) {
                let context = format!("{component} {syntax} {destination}");
                assert_eq!(only_destination(&context, &output), destination, "{context}");
            }
        }
    }
}

#[test]
fn terminal_fallback_without_osc8_shows_the_resolved_destination() {
    for (component, output) in terminal_outputs(&format!("[x]({MISSING})"), &fallback_term()) {
        let expected = url::Url::from_file_path(std::path::absolute(MISSING).unwrap()).unwrap();
        assert_eq!(output.trim_end(), format!("[x]({expected})"), "{component}");
    }
}

// ── embedded Prose resolves too; foreign links do not ────────────────────

#[test]
fn embedded_prose_links_resolve_only_on_the_terminal() {
    let source = format!("see [the plan]({MISSING})");
    let expected = std::path::absolute(cwd().join(MISSING)).unwrap();

    let inline = InlineContent::from("status: ").with(InlineProse::new(&source));
    let table = Table::new()
        .with_columns(vec![TableColumn::new("Col")])
        .with_data(vec![vec![TableCellContent::from(InlineProse::new(&source))]]);
    let status = StatusBlock::new(StatusState::Info).body(Prose::new(&source));

    let markdown = |tree: RenderNode| {
        render_markdown_node(&tree, &MarkdownRenderOptions::default()).unwrap().output
    };
    // `InlineContent` is terminal-only; it has no portable output.
    let cases: [(&str, String, Option<String>); 3] = [
        ("InlineContent", inline.render(&osc8_term()), None),
        ("Table", table.render(&osc8_term()), Some(markdown(table.render_tree()))),
        ("StatusBlock", status.render(&osc8_term()), Some(markdown(status.render_tree()))),
    ];
    for (container, terminal, markdown) in cases {
        let url = only_destination(container, &terminal);
        assert_eq!(file_url_path(container, &url), expected, "{container}");
        if let Some(markdown) = markdown {
            assert!(
                markdown.contains(&format!("[the plan]({MISSING})")),
                "{container} Markdown keeps the authored destination: {markdown:?}"
            );
            assert!(!markdown.contains("file://"), "{container}: {markdown:?}");
        }
    }
}

#[test]
fn links_built_outside_prose_keep_their_destination_on_the_terminal() {
    // A darkmatter document resolves its own links against its own base
    // directory; the terminal renderer must not re-resolve them.
    let tree = RenderNode::root(vec![RenderNode::paragraph(vec![RenderNode::link(
        "./plans/foo.md",
        None,
        vec![RenderNode::text("foo")],
    )])]);
    let options = TerminalRenderOptions::new(&osc8_term(), RenderStrictness::Warn);
    let output = render_terminal_node(&tree, &options).unwrap().output;
    assert_eq!(osc8_destinations(&output), ["./plans/foo.md"], "{output:?}");
}
