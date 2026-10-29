//! Source positions on public validation problems.
//!
//! Each case validates a complete Markdown document through
//! [`DarkmatterSchemas::validate`] and checks the reported line and UTF-8 byte
//! column, which count lines of the whole file (opening `---` included).

use biscuit_file::FileResolutionContext;
use serde_json::json;

use super::*;
use crate::markdown::Markdown;

fn api(baseline_yaml: &str) -> DarkmatterSchemas {
    let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(baseline_yaml).expect("baseline yaml");
    let baseline = super::simplified::parse_yaml_schema(&value).expect("baseline schema");
    DarkmatterSchemas::new(FileResolutionContext::new(std::env::temp_dir()))
        .with_baseline(baseline)
        .expect("baseline converts")
}

fn doc(text: &str) -> Markdown {
    text.to_string().into()
}

fn without_positions(problems: &[ValidationProblem]) -> Vec<ValidationProblem> {
    problems
        .iter()
        .cloned()
        .map(|mut problem| {
            problem.line = None;
            problem.column = None;
            problem
        })
        .collect()
}

/// Validates `md` with source positions, asserts the source-free report is
/// identical apart from coordinates, and returns the positioned report.
fn validate(api: &DarkmatterSchemas, md: &Markdown) -> ValidationReport {
    let report = api.validate(md).expect("validate");
    let effective = api.effective_for(md).expect("effective").expect("schema");
    let source_free = effective.validate(&frontmatter_as_json(md));
    assert!(source_free.problems.iter().all(|p| p.line.is_none() && p.column.is_none()));
    assert_eq!(report.valid, source_free.valid);
    // Every other field, and the problem order, must be unchanged.
    assert_eq!(
        format!("{:?}", without_positions(&report.problems)),
        format!("{:?}", source_free.problems)
    );
    report
}

fn problem<'a>(report: &'a ValidationReport, path: &str) -> &'a ValidationProblem {
    report
        .problems
        .iter()
        .find(|p| p.path == path)
        .unwrap_or_else(|| panic!("no problem at {path:?}: {:#?}", report.problems))
}

fn position(problem: &ValidationProblem) -> (Option<u32>, Option<u32>) {
    (problem.line, problem.column)
}

const START_SCHEMA: &str = "start: \"{ then: { say: string }[] }[]\"\n";
const ACCEPTANCE: &str = "---\nstart:\n  - then:\n      - say: hello\n      - sya: goodbye\n---\n";

#[test]
fn nested_unknown_key_reports_its_key_token() {
    for text in [ACCEPTANCE.to_string(), ACCEPTANCE.replace('\n', "\r\n")] {
        let report = validate(&api(START_SCHEMA), &doc(&text));
        assert_eq!(report.problems.len(), 1, "{:#?}", report.problems);
        let unknown = problem(&report, "/start/0/then/1");
        assert_eq!(unknown.code, ValidationProblemCode::UnknownKey);
        assert_eq!(unknown.offending_property.as_deref(), Some("sya"));
        assert_eq!(unknown.instance_path.as_pointer_string(), "/start/0/then/1");
        assert_eq!(position(unknown), (Some(5), Some(9)), "{text:?}");
    }
}

#[test]
fn a_wrong_list_item_reports_its_value_without_the_dash() {
    let report = validate(&api("tags: number[]\n"), &doc("---\ntags:\n  - 1\n  - oops\n---\n"));
    let wrong = problem(&report, "/tags/1");
    assert_eq!(wrong.code, ValidationProblemCode::TypeMismatch);
    assert_eq!(position(wrong), (Some(4), Some(5)));
}

#[test]
fn a_missing_field_reports_the_mapping_that_lacks_it() {
    let report = validate(
        &api("items: \"{ name: string, id: number(required) }[]\"\n"),
        &doc("---\nitems:\n  - name: a\n    id: 1\n  - name: b\n---\n"),
    );
    let missing = problem(&report, "/items/1");
    assert_eq!(missing.code, ValidationProblemCode::MissingRequired);
    assert_eq!(missing.property.as_deref(), Some("id"));
    assert_eq!(position(missing), (Some(5), Some(5)));
}

#[test]
fn missing_fields_in_empty_flow_mappings_report_the_opening_brace() {
    let api = api("m: '{ id: number(required) }'\nitems: '{ id: number(required) }[]'\n");
    for line_end in ["\n", "\r\n"] {
        let text = "---\nm: {}\nitems: [{id: 1}, {}]\n---\n".replace('\n', line_end);
        let report = validate(&api, &doc(&text));
        assert_eq!(report.problems.len(), 2, "{:#?}", report.problems);
        for (path, expected) in [("/m", (2, 4)), ("/items/1", (3, 18))] {
            let missing = problem(&report, path);
            assert_eq!(missing.code, ValidationProblemCode::MissingRequired);
            assert_eq!(missing.property.as_deref(), Some("id"));
            assert_eq!(position(missing), (Some(expected.0), Some(expected.1)));
        }
    }
}

#[test]
fn an_anchor_without_an_alias_keeps_precise_nested_positions() {
    let api = api("m: '{ value: number }'\n");
    let report = validate(&api, &doc("---\nm: &entry\n  value: oops\n---\n"));
    assert_eq!(report.problems.len(), 1, "{:#?}", report.problems);
    let wrong = problem(&report, "/m/value");
    assert_eq!(wrong.code, ValidationProblemCode::TypeMismatch);
    assert_eq!(position(wrong), (Some(3), Some(10)));
}

#[test]
fn flow_collections_distinguish_same_line_keys_and_values() {
    let md = doc("---\ntitle: x\nm: {a: 1, b: x}\ns: [1, two, 3]\n---\n");
    let unknown = validate(&api("m: \"{ a: number }\"\n"), &md);
    let key = problem(&unknown, "/m");
    assert_eq!(key.offending_property.as_deref(), Some("b"));
    assert_eq!(position(key), (Some(3), Some(11)));

    let wrong = validate(&api("m: \"{ a: number, b: number }\"\ns: number[]\n"), &md);
    assert_eq!(position(problem(&wrong, "/m/b")), (Some(3), Some(14)));
    assert_eq!(position(problem(&wrong, "/s/1")), (Some(4), Some(8)));
}

#[test]
fn a_root_failure_reports_the_root_mapping_start() {
    let report = validate(
        &api("title: string(required)\n"),
        &doc("---\n# comment\n\n# again\nother: x\n---\n"),
    );
    let missing = problem(&report, "");
    assert_eq!(missing.property.as_deref(), Some("title"));
    assert_eq!(position(missing), (Some(5), Some(1)));
}

#[test]
fn constraint_and_file_reference_failures_report_their_values() {
    let report = validate(
        &api("n: number(min(5))\nf: file(eager)\n"),
        &doc("---\nn:   1\nf:   no-such-file-for-positions.md\n---\n"),
    );
    let constraint = problem(&report, "/n");
    assert_eq!(constraint.code, ValidationProblemCode::ConstraintViolation);
    assert_eq!(position(constraint), (Some(2), Some(6)));
    let file = problem(&report, "/f");
    assert_eq!(file.code, ValidationProblemCode::InvalidFileReference);
    assert_eq!(position(file), (Some(3), Some(6)));
}

/// The validator's selected property (the first unexpected key) is located;
/// the problem is neither split nor reordered.
#[test]
fn a_multi_key_unknown_error_locates_its_selected_property() {
    let report = validate(
        &api("m: \"{ a: string }\"\n"),
        &doc("---\nm:\n  a: x\n  b: 1\n  c: 2\n---\n"),
    );
    assert_eq!(report.problems.len(), 1, "{:#?}", report.problems);
    let unknown = problem(&report, "/m");
    assert_eq!(unknown.offending_property.as_deref(), Some("b"));
    assert_eq!(position(unknown), (Some(4), Some(3)));
}

/// A property union keeps the problems it reported before; each is located at
/// the authored node its pointer names.
#[test]
fn a_property_union_keeps_its_problems_and_locates_them() {
    let schema = "payload:\n  - \"{ type: enum(foo; required), foo_id: string(required) }\"\n  - \"{ type: enum(bar; required), bar_count: number(required) }\"\n";
    let report = validate(&api(schema), &doc("---\npayload:\n  type: bar\n  extra: 1\n---\n"));
    assert!(!report.problems.is_empty());
    assert_eq!(position(problem(&report, "/payload/type")), (Some(3), Some(9)));
}

#[test]
fn block_scalar_content_reports_the_scalar_start() {
    let report = validate(&api("doc: number\n"), &doc("---\ndoc: |\n  not a number\n---\n"));
    assert_eq!(position(problem(&report, "/doc")), (Some(2), Some(6)));
}

/// An assignment changes values, not where their paths were authored.
#[test]
fn replaced_and_new_paths_use_the_original_source() {
    let api = api("tags: number[]\nm: \"{ a: number, b: number }\"\nextra: number\n");
    let mut md = doc("---\ntags:\n  - 1\n  - 2\nm:\n  a: 1\n---\n");
    let map = md.frontmatter_mut().as_map_mut();
    map.insert("tags".into(), json!([1, "replaced", "new"]));
    map.insert("m".into(), json!({ "a": 1, "b": "new" }));
    map.insert("extra".into(), json!("new"));
    let report = validate(&api, &md);
    assert_eq!(position(problem(&report, "/tags/1")), (Some(4), Some(5)));
    assert_eq!(position(problem(&report, "/tags/2")), (Some(3), Some(3)));
    assert_eq!(position(problem(&report, "/m/b")), (Some(6), Some(3)));
    assert_eq!(position(problem(&report, "/extra")), (None, None));
}

#[test]
fn unsupported_yaml_keeps_validity_and_top_level_positions() {
    let api = api("base: \"{ x: number }\"\nother: \"{ x: number }\"\nn: number\n");
    let valid = doc("---\nbase: &b\n  x: 1\nother: *b\n---\n");
    assert!(validate(&api, &valid).valid);

    let invalid = doc("---\nbase: &b\n  x: 1\nother: *b\nn:   oops\n---\n");
    let report = validate(&api, &invalid);
    assert_eq!(report.problems.len(), 1, "{:#?}", report.problems);
    assert_eq!(position(problem(&report, "/n")), (Some(5), Some(1)));
}

/// A constructed document is located in its serialized view.
#[test]
fn a_constructed_document_uses_its_serialized_view() {
    let mut md = Markdown::new("body");
    let map = md.frontmatter_mut().as_map_mut();
    map.insert("title".into(), json!("x"));
    map.insert("m".into(), json!({ "a": 1, "b": 2 }));
    let report = validate(&api("m: \"{ a: number }\"\n"), &md);
    let unknown = problem(&report, "/m");
    assert_eq!(position(unknown), (Some(4), Some(3)));
}

/// Loaded text whose frontmatter failed to parse is never projected.
#[test]
fn unparsed_frontmatter_reports_no_positions() {
    let md = doc("---\ntitle: [unclosed\n---\nbody\n");
    assert!(md.frontmatter().raw_source().is_none());
    let report = validate(&api("title: string(required)\n"), &md);
    assert_eq!(position(problem(&report, "")), (None, None));
}


/// Quoted and escaped keys are matched by their decoded text, the pointer
/// escapes `/` and `~`, and a `"0"` key walks a mapping where `0` under a
/// sequence walks an index. A key token's start includes its opening quote.
#[test]
fn quoted_keys_are_matched_decoded_and_escaped_in_the_pointer() {
    let api = api(concat!(
        "\"0\": \"{ x: number }\"\n",
        "\"a/b\": number\n",
        "\"a~b\": \"{ x: number }\"\n",
        "s: \"{ x: number }[]\"\n",
        "escA: number\n",
    ));
    let md = doc(concat!(
        "---\n",
        "\"0\":\n",
        "  x: oops\n",
        "\"a/b\": oops\n",
        "\"a~b\":\n",
        "  x: 1\n",
        "  'y': 2\n",
        "s:\n",
        "  - x: oops\n",
        "\"esc\\u0041\": oops\n",
        "---\n",
    ));
    let report = validate(&api, &md);
    assert_eq!(report.problems.len(), 5, "{:#?}", report.problems);
    assert_eq!(position(problem(&report, "/0/x")), (Some(3), Some(6)));
    assert_eq!(position(problem(&report, "/a~1b")), (Some(4), Some(8)));
    let unknown = problem(&report, "/a~0b");
    assert_eq!(unknown.offending_property.as_deref(), Some("y"));
    assert_eq!(position(unknown), (Some(7), Some(3)));
    assert_eq!(position(problem(&report, "/s/0/x")), (Some(9), Some(8)));
    assert_eq!(position(problem(&report, "/escA")), (Some(10), Some(14)));
}

/// The empty key's pointer is `/`; it is looked up as a decoded key, not as
/// the root.
#[test]
fn an_empty_key_is_a_property_not_the_root() {
    let api = api("m: \"{ a: number }\"\n");
    let report = validate(&api, &doc("---\nm:\n  a: 1\n  \"\": 2\n---\n"));
    assert_eq!(report.problems.len(), 1, "{:#?}", report.problems);
    let unknown = problem(&report, "/m");
    assert_eq!(unknown.offending_property.as_deref(), Some(""));
    assert_eq!(position(unknown), (Some(4), Some(3)));
}

/// Literal, folded, and quoted scalars report where the authored scalar
/// starts, including for a pointer below the scalar once its decoded content
/// stands in as structured data. No span inside the scalar is invented.
#[test]
fn scalar_content_and_its_descendants_report_the_authored_scalar_start() {
    let api = api(concat!(
        "lit: \"{ a: number }\"\n",
        "fold: \"{ a: number }\"\n",
        "cfg: \"{ a: number }\"\n",
        "j: json\n",
        "y: yaml\n",
    ));
    let source = concat!(
        "---\n",
        "lit: |\n",
        "  a: x\n",
        "fold: >\n",
        "  a: x\n",
        "cfg: '{\"a\": \"x\"}'\n",
        "j:   '{\"a\": }'\n",
        "y: |\n",
        "  a: [unclosed\n",
        "---\n",
    );

    // Authored as strings: each problem is the whole scalar's.
    let report = validate(&api, &doc(source));
    assert_eq!(position(problem(&report, "/lit")), (Some(2), Some(6)));
    assert_eq!(position(problem(&report, "/fold")), (Some(4), Some(7)));
    assert_eq!(position(problem(&report, "/cfg")), (Some(6), Some(6)));
    assert_eq!(position(problem(&report, "/j")), (Some(7), Some(6)));
    assert_eq!(position(problem(&report, "/y")), (Some(8), Some(4)));

    // Decoded into objects in memory: descendants keep the scalar start.
    let mut md = doc(source);
    let map = md.frontmatter_mut().as_map_mut();
    for key in ["lit", "fold", "cfg"] {
        map.insert(key.into(), json!({ "a": "x" }));
    }
    let report = validate(&api, &md);
    assert_eq!(position(problem(&report, "/lit/a")), (Some(2), Some(6)));
    assert_eq!(position(problem(&report, "/fold/a")), (Some(4), Some(7)));
    assert_eq!(position(problem(&report, "/cfg/a")), (Some(6), Some(6)));
}

/// A byte-order mark, CRLF line ends, leading comments, and blank lines move
/// byte offsets but not the reported lines: the opening delimiter is line 1
/// and is counted once.
#[test]
fn a_bom_crlf_document_with_leading_comments_counts_lines_once() {
    let api = api("m: \"{ a: number }\"\ntitle: string(required)\n");
    let text = "\u{feff}---\r\n# comment\r\n\r\n# again\r\nm:\r\n  a: x\r\n---\r\nbody\r\n";
    let md = doc(text);
    assert!(md.frontmatter().raw_source().is_some(), "BOM document parses its frontmatter");
    let report = validate(&api, &md);
    assert_eq!(report.problems.len(), 2, "{:#?}", report.problems);
    assert_eq!(position(problem(&report, "")), (Some(5), Some(1)));
    assert_eq!(position(problem(&report, "/m/a")), (Some(6), Some(6)));
}

/// A root union keeps its arm selection, count, and order; a nested failure
/// in the selected arm is located at the authored value.
#[test]
fn a_root_union_locates_a_nested_failure_in_its_selected_arm() {
    let api = DarkmatterSchemas::new(FileResolutionContext::new(std::env::temp_dir()));
    let md = doc(concat!(
        "---\n",
        "$schema:\n",
        "  - kind: literal(a)\n",
        "    m: \"{ x: number }\"\n",
        "  - kind: literal(b)\n",
        "    n: number(required)\n",
        "kind: a\n",
        "m:\n",
        "  x: oops\n",
        "---\n",
    ));
    let report = validate(&api, &md);
    assert_eq!(report.problems.len(), 1, "{:#?}", report.problems);
    let nested = problem(&report, "/m/x");
    assert_eq!(nested.arm_index, Some(0));
    assert_eq!(position(nested), (Some(9), Some(6)));
}
