//! Markdown written by renderable's shared writer renders faithfully through
//! darkmatter.
//!
//! The writer separates two code spans that would touch with an empty HTML
//! comment (`` `a`<!-- -->`b` ``). A comment has no visible content, so the
//! darkmatter fold and the terminal and browser renderers behind
//! [`Markdown::as_terminal`] and [`Markdown::as_html`] must show only the two
//! code values: no visible or escaped comment, and no strict rejection the
//! same Markdown without the comment would not also get.
//!
//! The cases are renderable's neighboring-code matrix (sixteen contexts ×
//! eleven wrapper routes × thirteen boundaries × six value pairs), written
//! in both dialects, shared through `#[path]` so both suites read the same
//! fixtures.

#[path = "../../../../renderable/tests/support/adjacent_code_matrix.rs"]
mod matrix;

use biscuit_terminal::render_tree::{TerminalRenderOptions, render_terminal_document};
use biscuit_terminal::terminal::Terminal;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::output::{HtmlOptions, TerminalOptions};
use darkmatter::markdown::render_tree::fold_markdown_to_document;
use matrix::{Context, contexts, for_each_case, groups, opts};
use renderable::tree::{MarkdownDialect, RenderStrictness, SourceDescriptor, render_markdown_node};

/// The separator the shared writer puts between touching code spans.
const SEPARATOR: &str = "<!-- -->";

/// Whether the terminal render of `markdown` under `strictness` succeeds,
/// and its output.
fn terminal(markdown: &str, strictness: RenderStrictness) -> Result<String, String> {
    let (doc, _) = fold_markdown_to_document(
        SourceDescriptor::Virtual {
            name: "case.md".into(),
        },
        markdown,
    );
    let term = Terminal::new_optimistic(80);
    render_terminal_document(&doc, &TerminalRenderOptions::new(&term, strictness))
        .map(|rendered| rendered.output)
        .map_err(|error| error.to_string())
}

/// Checks one serialized case and returns its problems.
///
/// `html_backed` marks MarkdownPlus disclosure summaries and columns, which
/// the writer lowers to raw HTML elements (with no separator): darkmatter's
/// default policy escapes that HTML, so their code values are not read from
/// `<code>` elements.
fn check(markdown: &str, values: (&str, &str), html_backed: bool) -> Vec<String> {
    let mut problems = Vec::new();
    let md = Markdown::new(markdown);

    match md.as_terminal(TerminalOptions::default()) {
        Ok(output) if output.contains("<!--") => problems.push(format!("as_terminal shows a comment: {output:?}")),
        Ok(_) => {}
        Err(error) => problems.push(format!("as_terminal failed: {error}")),
    }
    match md.as_html(HtmlOptions::default()) {
        Ok(html) => {
            if html.contains("&lt;!--") {
                problems.push("as_html escapes a comment into visible text".into());
            }
            for value in [values.0, values.1] {
                if !html_backed && !value.is_empty() && !html.contains(&format!(">{value}</code>")) {
                    problems.push(format!("as_html lost code value {value:?}"));
                }
            }
        }
        Err(error) => problems.push(format!("as_html failed: {error}")),
    }

    for strictness in [RenderStrictness::Warn, RenderStrictness::Lossy] {
        match terminal(markdown, strictness) {
            Ok(output) if output.contains("<!--") => {
                problems.push(format!("{strictness:?} terminal shows a comment: {output:?}"));
            }
            Ok(_) => {}
            Err(error) => problems.push(format!("{strictness:?} terminal failed: {error}")),
        }
    }

    // The comment alone never makes Strict reject: the same Markdown with a
    // visible space in its place must get the same outcome.
    let strict = terminal(markdown, RenderStrictness::Strict);
    let control = terminal(&markdown.replace(SEPARATOR, " "), RenderStrictness::Strict);
    if strict.is_ok() != control.is_ok() {
        problems.push(format!("Strict terminal {strict:?}, without the comment {control:?}"));
    }
    problems
}

/// Serializes every case in `contexts` in `dialect` and checks the result
/// through darkmatter. Returns the case count, how many carried a separator,
/// and the failures.
fn run(contexts: &[Context], dialect: MarkdownDialect) -> (usize, usize, Vec<String>) {
    let (mut cases, mut separated) = (0, 0);
    let mut failures = Vec::new();
    for_each_case(contexts, |case| {
        cases += 1;
        let label = format!(
            "{:?} / {} / {} / {} / {dialect:?}",
            case.context, case.route, case.boundary.name, case.pair
        );
        let markdown = match render_markdown_node(&case.node, &opts(dialect, RenderStrictness::Warn)) {
            Ok(rendered) => rendered.output,
            Err(error) => {
                failures.push(format!("{label}: writer failed: {error}"));
                return;
            }
        };
        if markdown.contains(SEPARATOR) {
            separated += 1;
        }
        let html_backed = dialect == MarkdownDialect::MarkdownPlus && case.context.html_in_markdown_plus();
        for problem in check(&markdown, case.values, html_backed) {
            failures.push(format!("{label}: {problem}\n{markdown}"));
        }
    });
    (cases, separated, failures)
}

/// Checks one dialect over every `parts`-th context of group `index`, starting
/// at `part`. Contexts are independent, so the slices together cover each
/// group exactly while running as parallel tests.
fn assert_slice(index: usize, dialect: MarkdownDialect, part: usize, parts: usize) {
    let slice: Vec<Context> = groups()[index].iter().copied().skip(part).step_by(parts).collect();
    let (cases, separated, failures) = run(&slice, dialect);
    assert!(cases > 0, "group {index} part {part}/{parts}: no cases");
    // MarkdownPlus lowers some contexts to raw HTML with no separator, so only
    // the plain dialect must always show one.
    assert!(
        dialect != MarkdownDialect::Markdown || separated > 0,
        "group {index} part {part}/{parts}: no case carried a separator"
    );
    assert!(
        failures.is_empty(),
        "{} failures in {cases} cases ({separated} with a separator); first four:\n{}",
        failures.len(),
        failures.iter().take(4).cloned().collect::<Vec<_>>().join("\n---\n")
    );
}

macro_rules! slices {
    ($($name:ident: $group:expr, $dialect:expr, $part:expr, $parts:expr;)*) => {
        $(#[test] fn $name() { assert_slice($group, $dialect, $part, $parts); })*
    };
}

slices! {
    neighbors_in_paragraphs_compositions_and_roots_markdown_0: 0, MarkdownDialect::Markdown, 0, 4;
    neighbors_in_paragraphs_compositions_and_roots_markdown_1: 0, MarkdownDialect::Markdown, 1, 4;
    neighbors_in_paragraphs_compositions_and_roots_markdown_2: 0, MarkdownDialect::Markdown, 2, 4;
    neighbors_in_paragraphs_compositions_and_roots_markdown_3: 0, MarkdownDialect::Markdown, 3, 4;
    neighbors_in_paragraphs_compositions_and_roots_plus_0: 0, MarkdownDialect::MarkdownPlus, 0, 4;
    neighbors_in_paragraphs_compositions_and_roots_plus_1: 0, MarkdownDialect::MarkdownPlus, 1, 4;
    neighbors_in_paragraphs_compositions_and_roots_plus_2: 0, MarkdownDialect::MarkdownPlus, 2, 4;
    neighbors_in_paragraphs_compositions_and_roots_plus_3: 0, MarkdownDialect::MarkdownPlus, 3, 4;
    neighbors_in_headings_and_section_headings_markdown_0: 1, MarkdownDialect::Markdown, 0, 2;
    neighbors_in_headings_and_section_headings_markdown_1: 1, MarkdownDialect::Markdown, 1, 2;
    neighbors_in_headings_and_section_headings_plus_0: 1, MarkdownDialect::MarkdownPlus, 0, 2;
    neighbors_in_headings_and_section_headings_plus_1: 1, MarkdownDialect::MarkdownPlus, 1, 2;
    neighbors_in_table_cells_markdown_0: 2, MarkdownDialect::Markdown, 0, 4;
    neighbors_in_table_cells_markdown_1: 2, MarkdownDialect::Markdown, 1, 4;
    neighbors_in_table_cells_markdown_2: 2, MarkdownDialect::Markdown, 2, 4;
    neighbors_in_table_cells_markdown_3: 2, MarkdownDialect::Markdown, 3, 4;
    neighbors_in_table_cells_plus_0: 2, MarkdownDialect::MarkdownPlus, 0, 4;
    neighbors_in_table_cells_plus_1: 2, MarkdownDialect::MarkdownPlus, 1, 4;
    neighbors_in_table_cells_plus_2: 2, MarkdownDialect::MarkdownPlus, 2, 4;
    neighbors_in_table_cells_plus_3: 2, MarkdownDialect::MarkdownPlus, 3, 4;
    neighbors_in_quotes_lists_and_footnotes_markdown_0: 3, MarkdownDialect::Markdown, 0, 3;
    neighbors_in_quotes_lists_and_footnotes_markdown_1: 3, MarkdownDialect::Markdown, 1, 3;
    neighbors_in_quotes_lists_and_footnotes_markdown_2: 3, MarkdownDialect::Markdown, 2, 3;
    neighbors_in_quotes_lists_and_footnotes_plus_0: 3, MarkdownDialect::MarkdownPlus, 0, 3;
    neighbors_in_quotes_lists_and_footnotes_plus_1: 3, MarkdownDialect::MarkdownPlus, 1, 3;
    neighbors_in_quotes_lists_and_footnotes_plus_2: 3, MarkdownDialect::MarkdownPlus, 2, 3;
    neighbors_in_disclosures_and_columns_markdown_0: 4, MarkdownDialect::Markdown, 0, 3;
    neighbors_in_disclosures_and_columns_markdown_1: 4, MarkdownDialect::Markdown, 1, 3;
    neighbors_in_disclosures_and_columns_markdown_2: 4, MarkdownDialect::Markdown, 2, 3;
    neighbors_in_disclosures_and_columns_plus_0: 4, MarkdownDialect::MarkdownPlus, 0, 3;
    neighbors_in_disclosures_and_columns_plus_1: 4, MarkdownDialect::MarkdownPlus, 1, 3;
    neighbors_in_disclosures_and_columns_plus_2: 4, MarkdownDialect::MarkdownPlus, 2, 3;
}

#[test]
fn groups_cover_every_context() {
    let grouped: Vec<Context> = groups().into_iter().flatten().collect();
    assert_eq!(grouped, contexts());
    // CI's test-input index does not count a `#[path]` include; this names the
    // shared fixture so a change to it schedules this module.
    let _ = include_str!("../../../../renderable/tests/support/adjacent_code_matrix.rs");
}

/// The review's reproduction, exactly: the comment is gone from both public
/// outputs, and the HTML body holds the two code elements side by side.
#[test]
fn separated_code_spans_render_as_two_values() {
    let md = Markdown::new("`a`<!-- -->`b`");
    let terminal = md.as_terminal(TerminalOptions::default()).expect("terminal");
    assert!(!terminal.contains("<!--"), "{terminal:?}");
    let html = md.as_html(HtmlOptions::default()).expect("html");
    assert!(html.contains("<p><code>a</code><code>b</code></p>"), "{html}");
}

/// Comment-shaped content that is not a comment node stays visible: a code
/// span, fenced code, escaped text, and a real tag's quoted attribute (raw
/// HTML that is more than a comment keeps the escape policy).
#[test]
fn comment_shaped_content_stays_visible() {
    let cases = [
        ("`<!-- -->`", "&lt;!-- --&gt;</code>", "<!-- -->"),
        ("```\n<!-- kept -->\n```", "&lt;!-- kept --&gt;", "<!-- kept -->"),
        ("\\<!-- x --> y", "&lt;!-- x --&gt; y", "<!-- x --> y"),
        (
            "a <span title=\"<!-- -->\">b</span>",
            "&lt;span title=\"&lt;!-- --&gt;\"&gt;",
            "<span title=\"<!-- -->\">",
        ),
    ];
    for (markdown, html_fragment, terminal_fragment) in cases {
        let md = Markdown::new(markdown);
        let html = md.as_html(HtmlOptions::default()).expect("html");
        assert!(html.contains(html_fragment), "{markdown:?}: {html}");
        let terminal = md.as_terminal(TerminalOptions::default()).expect("terminal");
        assert!(terminal.contains(terminal_fragment), "{markdown:?}: {terminal:?}");
    }
}

/// Folding and re-serializing keeps the separator, so the round trip is
/// stable and still reads as two code values.
#[test]
fn reserialization_keeps_the_separator() {
    let md = Markdown::new("`a`<!-- -->`b`");
    let first = darkmatter::markdown::render_tree::render_tree_markdown(&md).expect("markdown");
    assert_eq!(first.output.trim_end(), "`a`<!-- -->`b`");
    assert!(first.render_diagnostics.is_empty(), "{:?}", first.render_diagnostics);
}
