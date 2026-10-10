//! `code_link()` end to end: the real migrated template lines compose to a link
//! whose text is inline code, and that link renders as one on every target,
//! through darkmatter's own Markdown parser and through `Prose`.

use std::fs;
use std::path::Path;

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::discovery::detection::ColorDepth as TermColorDepth;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::escape_codes::strip_escape_codes;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::ComposeOptions;
use darkmatter::markdown::output::{ColorDepth, HtmlOptions, HyperlinkMode, TerminalOptions};
use darkmatter::markdown::render_tree::{render_tree_markdown, render_tree_terminal};
use renderable::browser::BrowserRenderable;
use renderable::markdown::MarkdownRenderable;
use renderable::tree::{NodeKind, RenderNode, TreeRenderable};

/// Each migrated template, with the frontmatter value line that carries its
/// `code_link()` call. Two are frozen copies of the review prompt held by
/// other packages' tests; the live `prompts/` are never read here.
const TEMPLATES: [(&str, &str); 2] = [
    (
        "claudine/cli/tests/fixtures/nested_span_regression/review-spec-inline.md",
        include_str!(
            "../../../../claudine/cli/tests/fixtures/nested_span_regression/review-spec-inline.md"
        ),
    ),
    (
        "darkmatter/dmls/tests/fixtures/mapping_only_corpus/_reviews__review-spec-inline.md",
        include_str!(
            "../../../../darkmatter/dmls/tests/fixtures/mapping_only_corpus/_reviews__review-spec-inline.md"
        ),
    ),
];

/// The double-quoted YAML value on the one line of `template` that calls
/// `code_link()`.
fn code_link_value(name: &str, template: &str) -> String {
    let lines: Vec<&str> = template.lines().filter(|line| line.contains("code_link(")).collect();
    assert_eq!(lines.len(), 1, "{name}: expected one code_link() line, got {lines:?}");
    let (_, value) = lines[0].split_once(": ").expect("a `key: value` line");
    let value = value.trim();
    assert!(
        value.starts_with('"') && value.ends_with('"') && !value.contains('\\'),
        "{name}: expected a plain double-quoted value, got {value}"
    );
    let backticked_link = template.contains("`{{link(") || template.contains("`{{ link(");
    assert!(!backticked_link, "{name}: a backtick-wrapped link() remains");
    value[1..value.len() - 1].to_string()
}

/// Composes `body` from a document beside `plans/foo.md` whose frontmatter
/// binds both template variables to that file.
fn compose_line(dir: &Path, frontmatter: &str, body: &str) -> String {
    fs::create_dir_all(dir.join("plans")).unwrap();
    fs::write(dir.join("plans/foo.md"), "# Foo\n").unwrap();
    let doc = dir.join("doc.md");
    fs::write(&doc, format!("---\n{frontmatter}---\n{body}\n")).unwrap();

    let options = ComposeOptions::new().with_source_file(&doc);
    let md = Markdown::try_from(doc.as_path()).unwrap();
    let (composed, _) = md.compose_with(&crate::request_support::request(options)).unwrap();
    let content = composed.content().trim().to_string();
    assert!(!content.contains("{{"), "no raw span may survive: {content}");
    content
}

/// Every link in `nodes`, depth first.
fn links(nodes: &[RenderNode]) -> Vec<&RenderNode> {
    let mut found = Vec::new();
    for node in nodes {
        if matches!(node.kind, NodeKind::Link { .. }) {
            found.push(node);
        }
        found.extend(links(node.children()));
    }
    found
}

/// The URL and inline-code value of the one link in `nodes`, which must have
/// exactly one child and that child must be inline code.
fn code_link_node(nodes: &[RenderNode]) -> (String, String) {
    let found = links(nodes);
    assert_eq!(found.len(), 1, "expected one link, got {found:#?}");
    let NodeKind::Link { url, children, .. } = &found[0].kind else {
        unreachable!()
    };
    let [child] = children.as_slice() else {
        panic!("a code link has one child, got {children:#?}");
    };
    let NodeKind::InlineCode { value } = &child.kind else {
        panic!("the link text must be inline code, got {child:#?}");
    };
    (url.clone(), value.clone())
}

fn styled_terminal() -> Terminal {
    Terminal::builder()
        .color_depth(TermColorDepth::TrueColor)
        .osc_link_support(true)
        .width(400)
        .build()
}

/// Asserts the composed `line` holds the code link `[`label`](url)` and that
/// both darkmatter and `Prose` render it as a link with inline-code text on
/// the tree, Markdown, HTML, and terminal targets.
fn assert_renders_code_link_everywhere(context: &str, line: &str, label: &str, url: &str) {
    let markdown_link = format!("[`{label}`]({url})");
    assert!(line.contains(&markdown_link), "{context}: {line}");

    // darkmatter's Markdown parser.
    let md = Markdown::new(line);
    let document = md.as_document().unwrap();
    assert_eq!(
        code_link_node(document.root.children()),
        (url.to_string(), label.to_string()),
        "{context}: darkmatter tree"
    );
    let markdown = render_tree_markdown(&md).unwrap().output;
    assert!(markdown.contains(&markdown_link), "{context}: darkmatter Markdown {markdown}");
    let html = md.as_html(HtmlOptions::default()).unwrap();
    let html_link = format!("<code>{label}</code></a>");
    assert!(html.contains(&html_link), "{context}: darkmatter HTML {html}");
    let mut options = TerminalOptions::default();
    options.color_depth = Some(ColorDepth::TrueColor);
    options.hyperlink_mode = HyperlinkMode::Always;
    options.max_width = Some(400);
    let terminal = render_tree_terminal(&md, &options).unwrap().output;
    assert!(terminal.contains(&format!("\x1b]8;;{url}")), "{context}: darkmatter OSC 8 {terminal:?}");
    let visible = strip_escape_codes(terminal.clone());
    assert!(visible.contains(label), "{context}: darkmatter terminal {visible:?}");
    assert!(!visible.contains(&format!("`{label}`")), "{context}: no fence when styled {visible:?}");

    // `Prose`, which claudine renders lifecycle messages through. Its tree,
    // Markdown, and HTML keep the authored destination; only its terminal
    // output resolves the path to a `file://` URL for OSC 8.
    let prose = Prose::new(line);
    let tree = prose.render_tree();
    assert_eq!(
        code_link_node(tree.children()),
        (url.to_string(), label.to_string()),
        "{context}: Prose tree"
    );
    let markdown = prose.render_markdown();
    assert!(markdown.contains(&markdown_link), "{context}: Prose Markdown {markdown}");
    let html = prose.render_html_fragment().render();
    assert!(html.contains(&html_link), "{context}: Prose HTML {html}");
    let terminal = prose.render(&styled_terminal());
    let url_tail = url.trim_start_matches("./");
    let prose_url = terminal
        .split("\x1b]8;;")
        .nth(1)
        .and_then(|rest| rest.split_once("\x1b\\"))
        .map(|(osc_url, _)| osc_url)
        .unwrap_or_default();
    if url.contains("://") {
        assert_eq!(prose_url, url, "{context}: Prose OSC 8 {terminal:?}");
    } else {
        assert!(
            prose_url.starts_with("file://") && prose_url.ends_with(url_tail),
            "{context}: Prose OSC 8 {terminal:?}"
        );
    }
    let visible = strip_escape_codes(terminal);
    assert!(visible.contains(label), "{context}: Prose terminal {visible:?}");
    assert!(!visible.contains(&format!("`{label}`")), "{context}: no fence when styled {visible:?}");
}

#[test]
fn migrated_templates_render_an_inline_code_link_on_every_target() {
    for (name, template) in TEMPLATES {
        let value = code_link_value(name, template);
        let dir = tempfile::tempdir().unwrap();
        let line = compose_line(
            dir.path(),
            "plan: plans/foo.md\nspec: plans/foo.md\n",
            &value,
        );

        let start = line.find("[`plans/foo.md`](").unwrap_or_else(|| panic!("{name}: {line}"));
        let rest = &line[start + "[`plans/foo.md`](".len()..];
        let url = &rest[..rest.find(')').unwrap()];
        // Compose normalizes the destination relative to the document, as it
        // does for `link()`.
        assert!(url.ends_with("plans/foo.md"), "{name}: destination {url}");

        assert_renders_code_link_everywhere(name, &line, "plans/foo.md", url);
    }
}

/// `link()`'s destination for the same file is the `code_link()` destination,
/// and wrapping `{{link(x)}}` in backticks no longer produces a link at all.
#[test]
fn code_link_matches_link_and_a_backticked_link_is_literal_code() {
    let dir = tempfile::tempdir().unwrap();
    let line = compose_line(
        dir.path(),
        "plan: plans/foo.md\n",
        "A {{link(plan)}} B {{code_link(plan)}} C `{{link(plan)}}`",
    );

    let link_start = line.find("A [plans/foo.md](").expect("link() output") + 2;
    let link_end = link_start + line[link_start..].find(')').unwrap() + 1;
    let plain = &line[link_start..link_end];
    let destination = &plain["[plans/foo.md](".len()..plain.len() - 1];
    assert!(line.contains(&format!("B [`plans/foo.md`]({destination}) C")), "{line}");

    let tail = &line[line.find(" C ").unwrap() + 3..];
    let tree = Prose::new(tail).render_tree();
    assert!(links(tree.children()).is_empty(), "a code span is opaque: {tail}");
}

/// A backslash in the text is literal in both parsers (AC 22).
#[test]
fn code_link_backslash_text_is_literal_in_prose_and_darkmatter() {
    let dir = tempfile::tempdir().unwrap();
    let line = compose_line(
        dir.path(),
        "label: 'a\\_b'\n",
        "{{code_link(\"https://example.com/x\", label)}}",
    );
    assert_eq!(line, "[`a\\_b`](https://example.com/x)");
    assert_renders_code_link_everywhere("a\\_b", &line, "a\\_b", "https://example.com/x");
}

/// Brackets stay unescaped and a backtick widens the fence, end to end.
#[test]
fn code_link_brackets_and_backticks_survive_compose_and_render() {
    let dir = tempfile::tempdir().unwrap();
    let line = compose_line(
        dir.path(),
        "bracket: 'a]b'\ntick: 'a`b'\n",
        "{{code_link(\"https://example.com/x\", bracket)}} and {{code_link(\"https://example.com/y\", tick)}}",
    );
    assert_eq!(
        line,
        "[`a]b`](https://example.com/x) and [``a`b``](https://example.com/y)"
    );

    let first = "[`a]b`](https://example.com/x)";
    assert_renders_code_link_everywhere("a]b", first, "a]b", "https://example.com/x");

    let second = Prose::new("[``a`b``](https://example.com/y)").render_tree();
    assert_eq!(
        code_link_node(second.children()),
        ("https://example.com/y".to_string(), "a`b".to_string())
    );
    let document = Markdown::new("[``a`b``](https://example.com/y)").as_document().unwrap();
    assert_eq!(
        code_link_node(document.root.children()),
        ("https://example.com/y".to_string(), "a`b".to_string())
    );
}

/// An empty text composes to an ordinary empty link with no inline code.
#[test]
fn code_link_empty_text_is_an_empty_link() {
    let dir = tempfile::tempdir().unwrap();
    let line = compose_line(
        dir.path(),
        "empty: ''\n",
        "x {{code_link(\"https://example.com/x\", empty)}} y",
    );
    assert_eq!(line, "x [](https://example.com/x) y");
}
