//! Expression callbacks through the public document composition boundary.

use darkmatter::markdown::compose::{ComposeContext, ComposeOptions, ComposeReport};
use darkmatter::markdown::Markdown;
use serde_json::json;

fn compose(source: &str) -> (Markdown, ComposeReport) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("doc.md");
    std::fs::write(&path, source).unwrap();
    let options = ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir.path(), ""))
        .with_source_file(path.clone());
    Markdown::try_from(path.as_path()).unwrap()
        .compose_with(&crate::request_support::request_at(dir.path(), options)).unwrap()
}

#[test]
fn whole_value_callbacks_keep_typed_arrays_and_feed_body_and_block_conditions() {
    for values in ["[1, 2, 3]", "['1', '2', '3']"] {
        let source = format!(r#"---
$schema:
  values: number[](required)
values: {values}
mapped: "{{{{ map(values, n -> n * 2) }}}}"
selected: "{{{{ filter(values, n => n > 1) }}}}"
---
{{{{ map(values, n => n * 2) }}}}
{{{{ filter(values, n -> n > 1) }}}}
::block when="filter(selected, n => n > 2)"
filter shown
::end-block
::block when="map(mapped, n => n + 1)"
map shown
::end-block
::block when="filter(values, n => false)"
hidden
::end-block
"#);
        let (document, report) = compose(&source);
        assert_eq!(document.frontmatter().as_map()["mapped"], json!([2, 4, 6]));
        let selected = if values.contains("\'") { json!(["2", "3"]) } else { json!([2, 3]) };
        assert_eq!(document.frontmatter().as_map()["selected"], selected);
        assert_eq!(document.content(), "[2,4,6] [2,3] filter shown map shown\n");
        assert!(report.warnings.is_empty(), "{:?}", report.warnings);
    }
}

#[test]
fn nested_capture_and_shadowing_restore_frontmatter_without_local_warnings() {
    let (document, report) = compose(r#"---
title: document
---
{{ map([1, 2], title => map(filter([2, 3], n => n > title), n -> {outer: local.title, value: n, doc: fm.title})) }}
{{ title }}
"#);
    assert_eq!(document.content(), "[[{\"outer\":1,\"value\":2,\"doc\":\"document\"},{\"outer\":1,\"value\":3,\"doc\":\"document\"}],[{\"outer\":2,\"value\":3,\"doc\":\"document\"}]] document");
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
}

#[test]
fn produced_expression_text_stays_data_across_frontmatter_and_body_passes() {
    let (document, report) = compose(r#"---
text: "{{ map([1], n => '{{ missing }}') }}"
filtered: "{{ filter(['{{ missing }}'], n => true) }}"
---
{{ text }}
{{ filtered }}
{{ map([1], n => '{{ missing }}') }}
{{ filter(['{{ missing }}'], n => true) }}
"#);
    assert_eq!(document.frontmatter().as_map()["text"], json!(["{{ missing }}"]));
    assert_eq!(document.frontmatter().as_map()["filtered"], json!(["{{ missing }}"]));
    assert_eq!(document.content(), "[\"{{ missing }}\"] [\"{{ missing }}\"] [\"{{ missing }}\"] [\"{{ missing }}\"]");
    assert!(report.warnings.is_empty(), "{:?}", report.warnings);
}

#[test]
fn absent_sources_are_nullable_and_only_unhandled_free_reads_warn() {
    let (document, report) = compose("{{ map(absent, n => n) }} {{ filter(absent, n => true) }} {{ map([1], n => missing) }}\n");
    assert_eq!(document.content(), "[] [] [null]\n");
    let warnings: Vec<_> = report.warnings.iter().filter(|warning| warning.code.as_deref() == Some("dm.expression.undeclared_property")).collect();
    assert_eq!(warnings.len(), 1, "{:?}", report.warnings);
    assert!(warnings[0].message.contains("missing"));
}
