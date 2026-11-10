//! `clean_document` rewriting legacy `key=value` link titles to JSON5: by
//! source span, with the attributes the old parser found, idempotently.

use std::convert::Infallible;

use darkmatter::markdown::Markdown;
use darkmatter::markdown::cleanup::rules::{FixedWidth, Leaf, MarkdownRuleOverrides};
use darkmatter::markdown::cleanup::{CleanOutcome, CleanRequest, CleanResult, clean_body, clean_document};
use darkmatter::markdown::cleanup::rules::ResolvedMarkdownRules;
use darkmatter::markdown::schemas::CleanSchemaConfig;
use darkmatter::render::{ImageRef, Link, LinkTarget};
use renderable::tree::NodeKind;

fn clean_with(source: &str, overrides: &MarkdownRuleOverrides) -> CleanResult {
    let request = CleanRequest { source, overrides, workspace: None, document_path: None };
    clean_document(&request, || Ok::<_, Infallible>(CleanSchemaConfig::new(crate::request_support::cwd_context())))
        .expect("clean runs")
}

fn clean(source: &str) -> CleanResult {
    clean_with(source, &MarkdownRuleOverrides::default())
}

/// The cleaned text, proved to be a fixed point.
fn migrated(source: &str) -> String {
    let first = clean(source);
    let second = clean(&first.text);
    assert_eq!(second.text, first.text, "a second clean is byte-identical");
    assert_eq!(second.outcome, CleanOutcome::Unchanged, "{}", first.text);
    first.text
}

#[test]
fn spec_example_migrates_to_the_spec_form() {
    let text = migrated("[Docs](https://example.com/docs \"class='docs-link' target='_blank' data-section='help'\")\n");
    assert_eq!(
        text,
        "[Docs](https://example.com/docs '{ class: \"docs-link\", target: \"_blank\", \"data-section\": \"help\" }')\n"
    );
    let md: Markdown = text.as_str().into();
    assert!(md.link_title_warnings().is_empty(), "a cleaned document has no legacy warning");
}

#[test]
fn a_migrated_link_has_the_attributes_the_old_parser_found() {
    let text = migrated(
        "[a](./a.md \"title='Tip' class=\\\"c1 c2\\\" style='color: red' TARGET=_top prompt='Why?' data-id=42, data-x='y z' unknown=1\")\n",
    );
    let line = text.trim_end();
    let link = Link::try_from(line).unwrap();
    assert_eq!(link.title(), Some("Tip"));
    assert_eq!(link.class(), Some("c1 c2"));
    assert_eq!(link.style_css().as_deref(), Some("color: red;"));
    assert_eq!(link.target(), Some(&LinkTarget::Top));
    assert_eq!(link.prompt(), Some("Why?"));
    assert_eq!(link.data().get("id").map(String::as_str), Some("42"));
    assert_eq!(link.data().get("x").map(String::as_str), Some("y z"));
    assert_eq!(link.data().len(), 2, "an unknown key stays dropped");
    assert_eq!(link.title_notice(), None);
}

/// The old parser read the title after Markdown decoded it, so the value here
/// is `it's &amp; (a)`; the JSON5 title has to read back to exactly that.
#[test]
fn values_needing_escapes_survive_both_syntaxes() {
    let text = migrated("[a](./a.md 'title=\"it&#39;s &amp;amp; (a)\" class=x')\n");
    let md: Markdown = text.as_str().into();
    let document = md.as_document().unwrap();
    let paragraph = &document.root.children()[0];
    let NodeKind::Link { title, .. } = &paragraph.children()[0].kind else { panic!("a link: {text}") };
    assert_eq!(title.as_deref(), Some("it's &amp; (a)"), "{text}");
    assert_eq!(paragraph.children()[0].attrs.classes, ["x"], "{text}");
}

#[test]
fn code_and_unrelated_text_stay_intact() {
    let source = "\
Prose with class='x' and `[a](./a.md \"class='y'\")` inline.

```text
[b](./b.md \"class='z'\")
```

    [c](./c.md \"class='w'\")

::shell echo '[d](./d.md \"class=v\")'

[e](./e.md \"class='k'\") then [f](./f.md \"Plain\").
";
    let text = migrated(source);
    let expected = source.replace("[e](./e.md \"class='k'\")", "[e](./e.md '{ class: \"k\" }')");
    assert_eq!(text, expected);
}

#[test]
fn an_image_title_keeps_its_own_syntax() {
    let source = "![i](./i.png \"class='hero' style='width: 20%'\")\n\n[![t](./t.png \"class='thumb'\")](./full.png \"target='_blank'\")\n";
    let text = migrated(source);
    assert_eq!(
        text,
        "![i](./i.png \"class='hero' style='width: 20%'\")\n\n[![t](./t.png \"class='thumb'\")](./full.png '{ target: \"_blank\" }')\n"
    );
    let image = ImageRef::try_from("![i](./i.png \"class='hero' style='width: 20%'\")").unwrap();
    assert_eq!(image.class(), Some("hero"));
}

#[test]
fn a_reference_definition_migrates_unless_an_image_also_uses_it() {
    let text = migrated("[a][one] and [b][two] and ![c][two]\n\n[one]: ./a.md \"class='x'\"\n[two]: ./b.md \"class='y'\"\n");
    assert_eq!(
        text,
        "[a][one] and [b][two] and ![c][two]\n\n[one]: ./a.md '{ class: \"x\" }'\n[two]: ./b.md \"class='y'\"\n"
    );
}

#[test]
fn a_table_cell_title_keeps_its_row() {
    let text = migrated("| h |\n|---|\n| [a](./a.md \"title='x\\|y'\") |\n");
    assert!(text.contains(r#"[a](./a.md '{ title: "x\|y" }')"#), "{text}");
}

#[test]
fn a_prose_title_with_no_supported_key_keeps_its_text() {
    let text = migrated("[a](./a.md \"Run x=1 first\")\n");
    assert_eq!(text, "[a](./a.md '{ title: \"Run x=1 first\" }')\n");
    assert_eq!(Link::try_from(text.trim_end()).unwrap().title(), Some("Run x=1 first"));
}

#[test]
fn a_title_holding_an_expression_is_left_for_composition() {
    let source = "[a](./a.md \"class='{{ kind }}'\")\n";
    assert_eq!(migrated(source), source);
}

#[test]
fn every_title_delimiter_migrates() {
    assert_eq!(migrated("[a](./a.md 'class=\"x\"')\n"), "[a](./a.md '{ class: \"x\" }')\n");
    assert_eq!(migrated("[a](./a.md (class='x'))\n"), "[a](./a.md '{ class: \"x\" }')\n");
}

#[test]
fn line_endings_bom_and_frontmatter_are_kept() {
    let source = "\u{feff}# T\r\n\r\n[a](./a.md \"class='x'\")\r\n";
    assert_eq!(migrated(source), "\u{feff}# T\r\n\r\n[a](./a.md '{ class: \"x\" }')\r\n");

    let source = "---\ntitle: \"a=b\"\n---\n\n[a](./a.md \"class='x'\")\n";
    assert_eq!(migrated(source), "---\ntitle: \"a=b\"\n---\n\n[a](./a.md '{ class: \"x\" }')\n");
}

#[test]
fn a_fixed_width_clean_of_a_migrated_title_is_a_fixed_point() {
    let overrides = MarkdownRuleOverrides {
        fixed_width: Leaf::Value(FixedWidth::Columns(30)),
        ..MarkdownRuleOverrides::default()
    };
    let source = "Words before the link [a](./a.md \"class='docs-link' target='_blank' data-section='help'\") and words after it.\n";
    let first = clean_with(source, &overrides);
    assert_eq!(first.outcome, CleanOutcome::Changed, "{:?}", first.diagnostics);
    assert!(first.text.contains("docs-link\""), "{}", first.text);
    let second = clean_with(&first.text, &overrides);
    assert_eq!(second.text, first.text);
}

/// Compose's cleanup and `Markdown::cleanup` share `clean_body`: rewriting a
/// title there would silently apply attributes the warning says are not.
#[test]
fn only_the_clean_transaction_migrates() {
    let source = "[a](./a.md \"class='x'\")\n";
    assert_eq!(clean_body(source, &ResolvedMarkdownRules::defaults()).text, source);
}

#[test]
fn a_decline_after_a_migration_reports_the_source_position() {
    let title = "[a](./a.md \"class='x' target='_blank' data-a='b'\")\n\n";
    let source = format!("{title}::block when=\"true\"\n");
    let cleaned = clean(&source);
    assert_eq!(cleaned.outcome, CleanOutcome::Declined);
    assert_eq!(cleaned.text, source, "the migration is discarded with the rest");
    let fallback = cleaned.diagnostics.last().unwrap().message.clone();
    assert!(fallback.contains("line 3"), "{fallback}");
}
