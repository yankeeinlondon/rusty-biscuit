//! Authored positions and sections on reference-graph records: where each
//! reference sits in the file it was loaded from, whatever preparation did to
//! the body in between. The fixtures reconstruct the findings table in
//! `2026-11-04-composition-errors`'s authored-positions spike.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use biscuit_test_harness::manifest_dir;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::report_diagnostics::error_diagnostics;
use darkmatter::markdown::compose::shell_expansion::types::{
    ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest,
};
use darkmatter::markdown::compose::{ComposeOptions, ComposeSource, ShellExpansionError};
use darkmatter::markdown::reference::validate::ReferenceValidationOptions;
use darkmatter::markdown::reference::{
    AuthoredReferenceLocation, ReferenceGraph, ReferenceGraphNode, ReferenceGraphOptions, ReferenceKind,
    ReferenceRecord, ReferenceSyntax, extract_document_references,
};

fn graph_options(dir: &Path) -> ReferenceGraphOptions {
    ReferenceGraphOptions::with_compose(&crate::request_support::request_at(dir, ComposeOptions::new()))
}

fn graph_of(path: &Path) -> ReferenceGraph {
    let dir = path.parent().unwrap();
    Markdown::try_from(path).unwrap().reference_graph(graph_options(dir)).unwrap()
}

/// A graph over one document written to a fresh directory; the directory is
/// returned to keep it alive.
fn graph_over(text: &str) -> (tempfile::TempDir, PathBuf, ReferenceGraph) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("doc.md");
    std::fs::write(&path, text).unwrap();
    let graph = graph_of(&path);
    (dir, path, graph)
}

fn record<'a>(node: &'a ReferenceGraphNode, target: &str) -> &'a ReferenceRecord {
    node.local_references
        .records
        .iter()
        .find(|record| record.target.raw() == Some(target))
        .unwrap_or_else(|| panic!("no record of {target} in {:#?}", node.local_references.records))
}

fn authored<'a>(node: &'a ReferenceGraphNode, target: &str) -> &'a AuthoredReferenceLocation {
    record(node, target)
        .origin
        .authored
        .as_ref()
        .unwrap_or_else(|| panic!("{target} has no authored position"))
}

/// `(line, column, inserted)`.
fn at(node: &ReferenceGraphNode, target: &str) -> (usize, usize, bool) {
    let location = authored(node, target);
    (location.line, location.column, location.inserted)
}

fn section<'a>(node: &'a ReferenceGraphNode, target: &str) -> Option<&'a str> {
    record(node, target).origin.section.as_deref()
}

fn child_node(graph: &ReferenceGraph) -> &ReferenceGraphNode {
    graph
        .iter()
        .find(|node| matches!(&node.source, ComposeSource::File(path) if path.ends_with("child.md")))
        .expect("the child was visited")
}

const ROOT_TARGETS: [(&str, (usize, usize, bool), &str); 5] = [
    ("./missing-one.md", (9, 5, false), "intro-root"),
    ("./missing-two.md", (15, 9, false), "second"),
    ("./missing-three.md", (22, 13, false), "second"),
    ("./missing-five.md", (24, 8, false), "second"),
    ("#no-such-heading", (28, 1, false), "second"),
];

const CHILD_TARGETS: [(&str, (usize, usize, bool), &str); 4] = [
    ("./root.md#nope", (7, 9, false), "child-heading"),
    ("./missing-four.md", (7, 43, false), "child-heading"),
    ("./missing-ins.md", (9, 7, true), "child-heading"),
    ("#absent", (11, 1, false), "child-heading"),
];

fn assert_spike_table(graph: &ReferenceGraph) {
    let root = graph.root();
    for (target, position, slug) in ROOT_TARGETS {
        assert_eq!(at(root, target), position, "{target}");
        assert_eq!(section(root, target), Some(slug), "{target}");
    }
    assert_eq!(at(root, "./child.md"), (26, 1, false));
    assert_eq!(section(root, "./child.md"), Some("second"));
    let [insertion] = root.child_insertions.as_slice() else {
        panic!("one insertion expected: {:#?}", root.child_insertions);
    };
    assert_eq!(insertion.authored_directive_line, Some(26));

    let child = child_node(graph);
    for (target, position, slug) in CHILD_TARGETS {
        assert_eq!(at(child, target), position, "{target}");
        assert_eq!(section(child, target), Some(slug), "{target}");
    }
}

#[test]
fn spike_fixture_positions_and_sections_are_exact() {
    let root = manifest_dir!().join("tests/fixtures/composition_errors/root.md");
    let child = manifest_dir!().join("tests/fixtures/composition_errors/child.md");
    assert!(child.is_file());

    assert_spike_table(&graph_of(&root));
}

/// `line`, `span`, and `directive_line` stay in the prepared body: the
/// file-tree model matches them in that frame.
#[test]
fn prepared_coordinates_are_unchanged() {
    let root = manifest_dir!().join("tests/fixtures/composition_errors/root.md");
    let child = manifest_dir!().join("tests/fixtures/composition_errors/child.md");
    assert!(child.is_file());
    let graph = graph_of(&root);

    let node = graph.root();
    let line = |target| record(node, target).origin.line;
    assert_eq!(
        (line("./missing-one.md"), line("./missing-two.md"), line("./missing-three.md")),
        (3, 11, 14)
    );
    assert_eq!((line("./missing-five.md"), line("#no-such-heading")), (16, 20));
    assert_eq!(line("./child.md"), 18);
    assert_eq!(node.child_insertions[0].directive_line, 18);
    assert_eq!(record(child_node(&graph), "./missing-ins.md").origin.line, 5);
}

/// A validation issue carries its record's origin, fragment issues included.
#[test]
fn validation_issues_carry_the_authored_position() {
    let root = manifest_dir!().join("tests/fixtures/composition_errors/root.md");
    let child = manifest_dir!().join("tests/fixtures/composition_errors/child.md");
    assert!(child.is_file());
    let dir = root.parent().unwrap();
    let options = ReferenceValidationOptions {
        validate_fragments: true,
        ..ReferenceValidationOptions::with_graph(graph_options(dir))
    };

    let markdown = Markdown::try_from(root.as_path()).unwrap();
    let report = markdown.validate_references(options).unwrap();

    let position = |display: &str| {
        let issue = report
            .issues
            .iter()
            .find(|issue| issue.reference_display == display)
            .unwrap_or_else(|| panic!("no issue for {display}: {:#?}", report.issues));
        let expected_trail: &[Option<usize>] = match display {
            "./missing-ins.md" | "#absent" => &[Some(26)],
            _ => &[],
        };
        let trail: Vec<Option<usize>> = issue.included_via.iter().map(|step| step.line).collect();
        assert_eq!(trail, expected_trail, "{display}");
        let location = issue.origin.authored.as_ref().expect("located");
        (location.line, location.column, location.inserted, issue.origin.section.clone().unwrap())
    };
    assert_eq!(position("#no-such-heading"), (28, 1, false, "second".to_string()));
    assert_eq!(position("./missing-two.md"), (15, 9, false, "second".to_string()));
    assert_eq!(position("./missing-ins.md"), (9, 7, true, "child-heading".to_string()));
    assert_eq!(position("#absent"), (11, 1, false, "child-heading".to_string()));
}

/// Writes the two fixtures through `encode` into a fresh directory.
fn encoded_fixtures(root: &Path, child: &Path, encode: impl Fn(&str) -> String) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("root.md");
    std::fs::write(&path, encode(&std::fs::read_to_string(root).unwrap())).unwrap();
    std::fs::write(dir.path().join("child.md"), encode(&std::fs::read_to_string(child).unwrap())).unwrap();
    (dir, path)
}

#[test]
fn crlf_line_endings_report_the_same_positions() {
    let root = manifest_dir!().join("tests/fixtures/composition_errors/root.md");
    let child = manifest_dir!().join("tests/fixtures/composition_errors/child.md");
    let (_dir, path) = encoded_fixtures(&root, &child, |text| text.replace('\n', "\r\n"));
    assert_spike_table(&graph_of(&path));
}

#[test]
fn a_byte_order_mark_reports_the_same_positions() {
    let root = manifest_dir!().join("tests/fixtures/composition_errors/root.md");
    let child = manifest_dir!().join("tests/fixtures/composition_errors/child.md");
    let (_dir, path) = encoded_fixtures(&root, &child, |text| format!("\u{FEFF}{text}"));
    assert_spike_table(&graph_of(&path));
}

#[test]
fn crlf_and_a_byte_order_mark_report_the_same_positions_together() {
    let root = manifest_dir!().join("tests/fixtures/composition_errors/root.md");
    let child = manifest_dir!().join("tests/fixtures/composition_errors/child.md");
    let (_dir, path) = encoded_fixtures(&root, &child, |text| format!("\u{FEFF}{}", text.replace('\n', "\r\n")));
    assert_spike_table(&graph_of(&path));
}

/// Without frontmatter a byte-order mark sits on the reference's own line.
#[test]
fn a_byte_order_mark_does_not_count_as_a_column() {
    let (_dir, _path, graph) = graph_over("\u{FEFF}[a](./a.md) then [b](./b.md)\n");
    assert_eq!(at(graph.root(), "./a.md"), (1, 1, false));
    assert_eq!(at(graph.root(), "./b.md"), (1, 18, false));
}

/// Without frontmatter a CRLF body keeps its carriage returns.
#[test]
fn crlf_without_frontmatter_reports_exact_positions() {
    let text = "# Top\r\n\r\n[a](./a.md)\r\n\r\n<a href=\"./b.md\">b</a>\r\n<img src=\"./c.png\">\r\n";
    let (_dir, _path, graph) = graph_over(text);
    assert_eq!(at(graph.root(), "./a.md"), (3, 1, false));
    assert_eq!(at(graph.root(), "./b.md"), (5, 1, false));
    assert_eq!(at(graph.root(), "./c.png"), (6, 1, false));
    assert_eq!(section(graph.root(), "./c.png"), Some("top"));
}

fn span_text<'a>(text: &'a str, location: &AuthoredReferenceLocation) -> &'a str {
    &text[location.span.clone()]
}

/// The span is the whole reference exactly as authored, on either line ending,
/// and a directive's span crosses its line terminator's carriage return.
#[test]
fn an_authored_span_is_the_whole_reference_under_either_line_ending() {
    let lf = "---\nk: 1\n---\nSee [a](./a.md) now\n\n::file ./a.md\n";
    for text in [lf.to_string(), lf.replace('\n', "\r\n")] {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.md"), "body\n").unwrap();
        let path = dir.path().join("doc.md");
        std::fs::write(&path, &text).unwrap();
        let graph = graph_of(&path);
        let link = graph
            .root()
            .local_references
            .hyperlinks()
            .into_iter()
            .next()
            .and_then(|record| record.origin.authored.clone())
            .unwrap();
        assert_eq!(span_text(&text, &link), "[a](./a.md)");
        let directive = graph
            .root()
            .local_references
            .transclusions()
            .into_iter()
            .next()
            .and_then(|record| record.origin.authored.clone())
            .unwrap();
        assert!(span_text(&text, &directive).starts_with("::file ./a.md"), "{:?}", span_text(&text, &directive));
    }
}

/// An indented directive starts at its `::`, not at its indentation.
#[test]
fn an_indented_directive_is_located_at_its_opener() {
    let text = "---\nk: 1\n---\n  ::code ./x.rs\n";
    let (_dir, _path, graph) = graph_over(text);
    let location = authored(graph.root(), "./x.rs");
    assert_eq!((location.line, location.column), (4, 3));
    assert!(span_text(text, location).starts_with("::code"), "{location:?}");
}

/// `::toc-linking` and `::file-links` records, and the links a TOC synthesizes,
/// locate at their directive, past the frontmatter and earlier edits.
#[test]
fn toc_and_file_links_directives_are_located() {
    let text = "---\nmulti: \"a\\n\\nb\"\n---\n# Top\n\n{{ multi }}\n\n::toc-linking ./toc.md\n\n::file-links ./*.md\n\n::toc-linking ./absent.md\n";
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("toc.md"), "# Title\n\n## Toc Head\n").unwrap();
    let path = dir.path().join("doc.md");
    std::fs::write(&path, text).unwrap();

    let graph = graph_of(&path);

    let root = graph.root();
    assert_eq!(at(root, "./toc.md"), (8, 1, false));
    assert_eq!(at(root, "./toc.md#toc-head"), (8, 1, false));
    assert_eq!(at(root, "./*.md"), (10, 1, false));
    assert_eq!(at(root, "./absent.md"), (12, 1, false));
    assert_eq!(section(root, "./*.md"), Some("top"));
    let lines: Vec<_> = root
        .child_insertions
        .iter()
        .map(|insertion| (insertion.directive_line, insertion.authored_directive_line))
        .collect();
    assert_eq!(lines, vec![(7, Some(8)), (11, Some(12))]);
}

/// The spike's `twice.md`: each insertion path reads the authored line of its
/// directive, not the prepared one every node of the path shares.
#[test]
fn a_nested_insertion_has_its_authored_directive_line() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("leaf.md"), "# Leaf\n").unwrap();
    std::fs::write(dir.path().join("mid.md"), "---\nk: 1\n---\n# Mid\n\n::file ./leaf.md\n").unwrap();
    let twice = dir.path().join("twice.md");
    std::fs::write(&twice, "---\nk: 1\n---\n\n::file ./mid.md\n\n::file ./mid.md\n").unwrap();

    let graph = graph_of(&twice);

    let root = graph.root();
    let lines: Vec<_> = root.child_insertions.iter().map(|insertion| insertion.authored_directive_line).collect();
    assert_eq!(lines, vec![Some(5), Some(7)]);
    let mid = graph
        .iter()
        .find(|node| matches!(&node.source, ComposeSource::File(path) if path.ends_with("mid.md")))
        .unwrap();
    assert_eq!(mid.child_insertions[0].directive_line, 3);
    assert_eq!(mid.child_insertions[0].authored_directive_line, Some(6));
}

/// A link whose label holds an interpolation starts with an authored `[`.
#[test]
fn a_mixed_label_stays_authored_with_a_first_character_span() {
    let text = "---\ntitle: Guide\n---\n\nSee [{{ title }} docs](./x.md)\n";
    let (_dir, _path, graph) = graph_over(text);
    let location = authored(graph.root(), "./x.md");
    assert_eq!((location.line, location.column, location.inserted), (5, 5, false));
    assert_eq!(span_text(text, location), "[");
}

/// A multibyte character before the reference counts as one column.
#[test]
fn an_image_after_a_multibyte_character_has_a_character_column() {
    let text = "é ![pic](./p.png)\n";
    let (_dir, _path, graph) = graph_over(text);
    let location = authored(graph.root(), "./p.png");
    assert_eq!((location.line, location.column), (1, 3));
    assert_eq!(span_text(text, location), "![pic](./p.png)");
}

/// An inserted reference names its `{{ … }}`, and an authored one beside it on
/// the same line still names itself.
#[test]
fn same_line_producer_and_authored_link_stay_distinct() {
    let text = "---\nlink: \"[i](./i.md)\"\n---\nA {{ link }} then [a](./a.md)\n";
    let (_dir, _path, graph) = graph_over(text);
    let inserted = authored(graph.root(), "./i.md");
    assert_eq!((inserted.line, inserted.column, inserted.inserted), (4, 3, true));
    assert_eq!(span_text(text, inserted), "{{ link }}");
    assert_eq!(at(graph.root(), "./a.md"), (4, 19, false));
}

/// A replacement at either edge of the body has only one authored neighbor.
#[test]
fn producers_at_both_body_edges_are_located() {
    let text = "---\nfirst: \"[f](./f.md)\"\nlast: \"[l](./l.md)\"\n---\n{{ first }} middle {{ last }}";
    let (_dir, _path, graph) = graph_over(text);
    assert_eq!(span_text(text, authored(graph.root(), "./f.md")), "{{ first }}");
    assert_eq!(span_text(text, authored(graph.root(), "./l.md")), "{{ last }}");
    assert_eq!(at(graph.root(), "./f.md"), (5, 1, true));
    assert_eq!(at(graph.root(), "./l.md"), (5, 20, true));
}

/// Adjacent replacements share a gap, so only recorded producers separate them.
#[test]
fn adjacent_producers_name_their_own_expression() {
    let text = "---\na: \"[a](./a.md)\"\nb: \"[b](./b.md)\"\n---\nx {{ a }}{{ b }} y\n";
    let (_dir, _path, graph) = graph_over(text);
    assert_eq!(span_text(text, authored(graph.root(), "./a.md")), "{{ a }}");
    assert_eq!(span_text(text, authored(graph.root(), "./b.md")), "{{ b }}");
    assert_eq!(at(graph.root(), "./a.md"), (5, 3, true));
    assert_eq!(at(graph.root(), "./b.md"), (5, 10, true));
}

/// A `replace:` value names the replaced key.
#[test]
fn a_replaced_key_is_the_producer_of_its_value() {
    let text = "---\nreplace: {KEY: \"[r](./r.md)\"}\n---\nSee KEY now\n";
    let (_dir, _path, graph) = graph_over(text);
    let location = authored(graph.root(), "./r.md");
    assert_eq!((location.line, location.column, location.inserted), (4, 5, true));
    assert_eq!(span_text(text, location), "KEY");
}

/// A `replace:` value is scanned by interpolation, so that edit lands inside
/// produced text and inherits the original producer: the key.
#[test]
fn an_edit_inside_produced_text_inherits_the_original_producer() {
    let text = "---\nlink: \"[c](./c.md)\"\nreplace: {KEY: \"{{ link }}\"}\n---\nSee KEY now\n";
    let (_dir, _path, graph) = graph_over(text);
    let location = authored(graph.root(), "./c.md");
    assert_eq!((location.line, location.column, location.inserted), (5, 5, true));
    assert_eq!(span_text(text, location), "KEY");
}

/// Every reference a multi-line value wrote names the same expression.
#[test]
fn every_line_a_producer_wrote_names_it() {
    let text = "---\nmulti: \"[a](./a.md)\\n\\n[b](./b.md)\"\n---\n\n{{ multi }}\n\n[c](./c.md)\n";
    let (_dir, _path, graph) = graph_over(text);
    assert_eq!(at(graph.root(), "./a.md"), (5, 1, true));
    assert_eq!(at(graph.root(), "./b.md"), (5, 1, true));
    assert_eq!(at(graph.root(), "./c.md"), (7, 1, false));
}

#[test]
fn css_records_are_rebased_onto_the_body_and_include_their_opener() {
    let text = concat!(
        "---\nk: 1\n---\n",
        "Intro text.\n\n",
        "<style>@import url(\"./a.css\"); @font-face { src: url(\"./f.woff2\"); }</style>\n",
    );
    let (_dir, path, graph) = graph_over(text);
    let node = graph.root();
    let body = Markdown::try_from(path.as_path()).unwrap();

    let import = record(node, "./a.css");
    assert_eq!(import.kind, ReferenceKind::CssImport);
    assert_eq!(&body.content()[import.origin.span.clone()], "@import url(\"./a.css\")");
    let location = authored(node, "./a.css");
    assert_eq!((location.line, location.column, location.inserted), (6, 8, false));
    assert_eq!(span_text(text, location), "@import url(\"./a.css\")");

    let font = record(node, "./f.woff2");
    assert_eq!(font.kind, ReferenceKind::FontImport);
    assert_eq!(&body.content()[font.origin.span.clone()], "url(\"./f.woff2\")");
    assert_eq!(span_text(text, authored(node, "./f.woff2")), "url(\"./f.woff2\")");
}

#[test]
fn an_unquoted_css_url_spans_its_whole_token() {
    let content = "<style>@font-face { src: url(./f.woff2); }</style>\n";
    let references = extract_document_references(content);
    let font = references
        .records
        .iter()
        .find(|record| record.origin.syntax == ReferenceSyntax::CssFontFaceSrc)
        .unwrap();
    assert_eq!(&content[font.origin.span.clone()], "url(./f.woff2)");
}

/// An entity-bearing block's decoded CSS has no offsets of its own, so its
/// records are spanned by the tag rather than by a guess.
#[test]
fn a_decoded_style_block_spans_its_records_by_the_tag() {
    let content = "<style>/* &amp; */ @import \"./a.css\";</style>\n";
    let references = extract_document_references(content);
    let import = references
        .records
        .iter()
        .find(|record| record.origin.syntax == ReferenceSyntax::CssAtImport)
        .unwrap();
    assert_eq!(&content[import.origin.span.clone()], content.trim_end());
}

/// A `::block` removed before the style block still projects: the map, not an
/// offset of the frontmatter, carries the record.
#[test]
fn css_positions_follow_preparation_edits() {
    let text = "---\ndraft: false\ntitle: T\n---\n::block when=\"draft\"\nGone\n::end-block\n\n{{ title }}\n\n<style>@import \"./a.css\";</style>\n";
    let (_dir, _path, graph) = graph_over(text);
    let location = authored(graph.root(), "./a.css");
    assert_eq!(span_text(text, location), "@import \"./a.css\"");
    assert_eq!((location.line, location.column, location.inserted), (11, 8, false));
}

#[test]
fn frontmatter_values_locate_their_authored_position() {
    let text = concat!(
        "---\n",
        "title: T\n",
        "prologue: ./pro.md\n",
        "epilogue:\n",
        "  - ./epi-one.md\n",
        "  - \"./epi-two.md\"\n",
        "---\n",
        "# Body\n",
    );
    let dir = tempfile::tempdir().unwrap();
    for name in ["pro.md", "epi-one.md", "epi-two.md"] {
        std::fs::write(dir.path().join(name), "text\n").unwrap();
    }
    for text in [text.to_string(), text.replace('\n', "\r\n")] {
        let path = dir.path().join("doc.md");
        std::fs::write(&path, &text).unwrap();
        let graph = graph_of(&path);
        let root = graph.root();

        let prologue = authored(root, "./pro.md");
        assert_eq!((prologue.line, prologue.column, prologue.inserted), (3, 11, false));
        assert_eq!(span_text(&text, prologue), "./pro.md");
        assert_eq!(at(root, "./epi-one.md"), (5, 5, false));
        let second = authored(root, "./epi-two.md");
        assert_eq!((second.line, second.column), (6, 5));
        assert_eq!(span_text(&text, second), "\"./epi-two.md\"");

        // The graph's prepared frame keeps its sentinels; no authored field does.
        assert_eq!(record(root, "./pro.md").origin.line, 0);
        assert_eq!(record(root, "./epi-one.md").origin.line, usize::MAX);
        assert_eq!(record(root, "./pro.md").origin.section, None);

        let lines: Vec<_> = root
            .child_insertions
            .iter()
            .map(|insertion| (insertion.directive_line, insertion.authored_directive_line))
            .collect();
        assert_eq!(lines, vec![(0, Some(3)), (usize::MAX, Some(5)), (usize::MAX, Some(6))]);
    }
}

/// A flow list's items have no exact leaf span, but the property's own line is
/// proven, so the value's key stands in.
#[test]
fn a_flow_list_value_falls_back_to_its_key() {
    let text = "---\ntitle: T\nprologue: [./a.md, ./b.md]\n---\nBody\n";
    let (_dir, _path, graph) = graph_over(text);
    assert_eq!(at(graph.root(), "./a.md"), (3, 1, false));
    assert_eq!(at(graph.root(), "./b.md"), (3, 1, false));
}

/// A literal prologue's links have no body position; they name the value.
#[test]
fn a_literal_prologue_locates_its_links_at_the_value() {
    let text = "---\nprologue: |\n  See [p](./p.md)\n  more\n---\nBody [b](./b.md)\n";
    let (_dir, _path, graph) = graph_over(text);
    let root = graph.root();
    let link = record(root, "./p.md");
    assert_eq!(link.origin.authored.as_ref().map(|location| location.line), Some(2));
    assert_eq!(link.origin.section, None);
    assert_eq!(at(root, "./b.md"), (6, 6, false));
}

/// `Markdown::new` does not parse frontmatter, so a prepared body starting
/// with `---` and holding another `---` cannot be misread as a block that
/// would shift every heading offset.
#[test]
fn a_leading_rule_is_not_read_as_frontmatter() {
    let path = manifest_dir!().join("tests/fixtures/composition_errors/leading-rule.md");
    let graph = graph_of(&path);
    assert_eq!(at(graph.root(), "./missing.md"), (8, 1, false));
    assert_eq!(section(graph.root(), "./missing.md"), Some("after-the-rule"));
    assert_eq!(record(graph.root(), "./missing.md").origin.line, 5);
}

#[test]
fn a_section_uses_the_final_slug_of_the_innermost_prior_heading() {
    let text = concat!(
        "---\ntitle: Root\ndraft: false\n---\n",
        "[above](./above.md)\n\n",
        "# Setup\n\n[one](./one.md)\n\n",
        "## Setup\n\n[two](./two.md)\n\n",
        "## Intro {{ title }}\n\n[three](./three.md)\n\n",
        "::block when=\"draft\"\n## Removed\n::end-block\n\n",
        "[four](./four.md)\n\n",
        "# [inside](./inside.md)\n",
    );
    let (_dir, _path, graph) = graph_over(text);
    let root = graph.root();
    assert_eq!(section(root, "./above.md"), None);
    assert_eq!(section(root, "./one.md"), Some("setup"));
    assert_eq!(section(root, "./two.md"), Some("setup-1"));
    assert_eq!(section(root, "./three.md"), Some("intro-root"));
    assert_eq!(section(root, "./four.md"), Some("intro-root"));
    assert_eq!(section(root, "./inside.md"), Some("inside"));
}

/// Fragment validation keeps checking the TOC's slugs (lowercased, duplicates
/// not numbered) beside the numbered sections.
#[test]
fn fragment_lookup_keeps_the_table_of_contents_slugs() {
    let text = "# Setup\n\n## Setup\n\n[fine](#setup)\n\n[numbered](#setup-1)\n";
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("doc.md");
    std::fs::write(&path, text).unwrap();
    let options = ReferenceValidationOptions {
        validate_fragments: true,
        ..ReferenceValidationOptions::with_graph(graph_options(dir.path()))
    };

    let report = Markdown::try_from(path.as_path()).unwrap().validate_references(options).unwrap();

    let broken: Vec<_> = report.issues.iter().map(|issue| issue.reference_display.as_str()).collect();
    assert_eq!(broken, vec!["#setup-1"]);
}

/// A document built in memory has no file to name: no position, but a section.
#[test]
fn an_in_memory_document_has_a_section_and_no_authored_position() {
    let text = "---\ntitle: Root\n---\n# Intro {{ title }}\n\n[a](./a.md)\n\n::file ./missing.md\n";
    for markdown in [
        Markdown::from(text),
        Markdown::new("# Intro Root\n\n[a](./a.md)\n\n::file ./missing.md\n"),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let graph = markdown.reference_graph(graph_options(dir.path())).unwrap();
        let root = graph.root();
        assert_eq!(record(root, "./a.md").origin.authored, None);
        assert_eq!(section(root, "./a.md"), Some("intro-root"));
        assert_eq!(record(root, "./a.md").origin.line, 3);
        assert_eq!(record(root, "./missing.md").origin.authored, None);
    }
}

/// A file identity alone proves no range: a document built from parts and then
/// given a path has no loaded text to locate in.
#[test]
fn a_file_identity_without_loaded_text_has_no_authored_position() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("doc.md");
    std::fs::write(&path, "# Top\n\n[a](./a.md)\n").unwrap();
    let markdown = Markdown::new("# Top\n\n[a](./a.md)\n").with_source(ComposeSource::File(path));

    let graph = markdown.reference_graph(graph_options(dir.path())).unwrap();

    assert_eq!(record(graph.root(), "./a.md").origin.authored, None);
    assert_eq!(section(graph.root(), "./a.md"), Some("top"));
}

/// A remote document's identity is its URL; it has no file coordinates.
#[test]
fn a_remote_document_keeps_its_url_and_has_no_authored_position() {
    let dir = tempfile::tempdir().unwrap();
    let url = url::Url::parse("https://example.com/doc.md").unwrap();
    let markdown =
        Markdown::from("---\nk: 1\n---\n# Top\n\n[a](./a.md)\n").with_source(ComposeSource::Url(url.clone()));

    let graph = markdown.reference_graph(graph_options(dir.path())).unwrap();

    let origin = &record(graph.root(), "./a.md").origin;
    assert_eq!(origin.source, ComposeSource::Url(url));
    assert_eq!(origin.authored, None);
    assert_eq!(origin.section.as_deref(), Some("top"));
}

fn compose_error(path: &Path) -> darkmatter::markdown::MarkdownError {
    let dir = path.parent().unwrap();
    let request = crate::request_support::request_at(dir, ComposeOptions::new().with_source_file(path));
    Markdown::try_from(path).unwrap().compose_with(&request).map(|_| ()).unwrap_err()
}

/// The spike's prepared line 18 is authored line 26.
#[test]
fn a_transclusion_when_failure_names_the_authored_line_and_file() {
    let path = manifest_dir!().join("tests/fixtures/composition_errors/when-error.md");

    let error = compose_error(&path);

    let message = error.to_string();
    assert!(message.contains("at line 26"), "{message}");
    assert!(!message.contains("at line 18"), "{message}");
    let diagnostics = error_diagnostics(&error);
    let source = diagnostics.iter().find_map(|diagnostic| diagnostic.source.as_ref()).expect("located");
    assert_eq!(source.line, Some(26));
    assert!(source.file.as_ref().is_some_and(|file| file.ends_with("when-error.md")), "{source:?}");
}

/// The same failure in CRLF and in a transcluded child names that file's line.
#[test]
fn a_transclusion_when_failure_is_authored_in_crlf_and_in_a_child() {
    let dir = tempfile::tempdir().unwrap();
    let parent = dir.path().join("parent.md");
    std::fs::write(&parent, "# Parent\n\n::file ./inner.md\n").unwrap();
    let inner = "---\nmulti: \"a\\n\\nb\"\n---\n{{ multi }}\n\n::file ./x.md when=\"length(1, 2, 3)\"\n".replace('\n', "\r\n");
    std::fs::write(dir.path().join("inner.md"), inner).unwrap();

    let error = compose_error(&parent);

    let diagnostics = error_diagnostics(&error);
    let source = diagnostics.iter().find_map(|diagnostic| diagnostic.source.as_ref()).expect("located");
    assert_eq!(source.line, Some(6), "{diagnostics:#?}");
    assert!(source.file.as_ref().is_some_and(|file| file.ends_with("inner.md")), "{source:?}");
}

/// A failure no passive call check rejects reaches the condition itself.
#[test]
fn a_transclusion_when_evaluation_failure_is_located_at_the_directive() {
    let text = "---\nmulti: \"a\\n\\nb\"\n---\n{{ multi }}\n\n  ::file ./x.md when=\"current.nosuch\"\n";
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("doc.md");
    std::fs::write(&path, text).unwrap();

    let error = compose_error(&path);

    assert!(error.to_string().contains("at line 6"), "{error}");
    let diagnostics = error_diagnostics(&error);
    let [diagnostic] = diagnostics.as_slice() else {
        panic!("one diagnostic expected: {diagnostics:#?}");
    };
    let source = diagnostic.source.as_ref().expect("located");
    assert_eq!((source.line, source.column), (Some(6), Some(3)));
    assert_eq!(&text[diagnostic.span.clone().expect("spanned")], ":");
}

/// A document with no file keeps the body frame the error excerpts, and claims
/// no authored source.
#[test]
fn an_in_memory_when_failure_has_no_authored_source() {
    let dir = tempfile::tempdir().unwrap();
    let request = crate::request_support::request_at(dir.path(), ComposeOptions::new());
    let markdown = Markdown::new("# Top\n\n::file ./x.md when=\"current.nosuch\"\n");

    let error = markdown.compose_with(&request).map(|_| ()).unwrap_err();

    assert!(error.to_string().contains("line 3"), "{error}");
    assert!(error_diagnostics(&error).iter().all(|diagnostic| diagnostic.source.is_none()));
}

/// Approves every command the shell stage asks about.
struct AllowAll;

impl ShellApprovalHandler for AllowAll {
    fn approve(&self, _: ShellApprovalRequest) -> Result<ShellApprovalDecision, ShellExpansionError> {
        Ok(ShellApprovalDecision::AllowOnce)
    }
}

/// Shell output names the directive that wrote it, and its line terminator
/// stays authored so the next line's position is exact.
#[test]
fn shell_output_is_located_at_its_directive() {
    let text = "---\nk: 1\n---\n# Top\n\n::shell echo [s](./s.md)\n[a](./a.md)\n";
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("doc.md");
    std::fs::write(&path, text).unwrap();
    let options = ComposeOptions::new()
        .with_shell_policy_root(dir.path())
        .with_shell_working_directory(dir.path())
        .with_shell_approval_handler(Arc::new(AllowAll));
    let options = ReferenceGraphOptions::with_compose(&crate::request_support::request_at(dir.path(), options));

    let graph = Markdown::try_from(path.as_path()).unwrap().reference_graph(options).unwrap();

    let location = authored(graph.root(), "./s.md");
    assert_eq!((location.line, location.column, location.inserted), (6, 1, true));
    assert!(span_text(text, location).starts_with("::shell echo"), "{location:?}");
    assert_eq!(at(graph.root(), "./a.md"), (7, 1, false));
}
