//! Opener-line grammar through [`parse_collection_regions`], run for both
//! kinds from one table.

use darkmatter::markdown::SourceRef;
use darkmatter::markdown::compose::collection_blocks::{
    CollectionError, CollectionKind, CollectionRegion, SortOrder, parse_collection_regions,
    scan_collection_regions_lenient,
};

use super::support::{KINDS, closer, opener, other, region, row_tests};

enum Expect {
    /// The header parses and renders as this summary.
    Parses(&'static str),
    /// Strict parsing fails with this code, pointing at this authored text.
    Rejects(&'static str, &'static str),
}
use Expect::{Parses, Rejects};

const MALFORMED: &str = "dm.collection.malformed";
const CALLBACK: &str = "dm.expression.invalid_callback";

/// `(row, header text after the keyword, outcome)`. Every row runs for both
/// kinds; a kind-specific difference is its own test further down.
const GRAMMAR: &[(&str, &str, Expect)] = &[
    // Defaults and sources.
    ("bare_source_has_every_default", "items", Parses("items")),
    ("dotted_dashed_source", "team.my-members", Parses("team.my-members")),
    ("local_prefixed_source", "local.my-items", Parses("local.my-items")),
    ("fm_prefixed_source", "fm.items", Parses("fm.items")),
    ("sort_defaults_to_no_explicit_order", "items sort=\"i => i\"", Parses("items|sort=i => i")),
    // Both arrows in every slot.
    ("positional_fat_arrow", "items \"i => i\"", Parses("items|cb=i => i")),
    ("positional_thin_arrow", "items \"i -> i\"", Parses("items|cb=i -> i")),
    ("filter_option_fat_arrow", "items filter=\"i => i\"", Parses("items|filter=i => i")),
    ("filter_option_thin_arrow", "items filter=\"i -> i\"", Parses("items|filter=i -> i")),
    ("sort_option_fat_arrow", "items sort=\"i => i\"", Parses("items|sort=i => i")),
    ("sort_option_thin_arrow", "items sort=\"i -> i\"", Parses("items|sort=i -> i")),
    ("while_option_fat_arrow", "items while=\"i => i\"", Parses("items|while=i => i")),
    ("while_option_thin_arrow", "items while=\"i -> i\"", Parses("items|while=i -> i")),
    ("until_option_fat_arrow", "items until=\"i => i\"", Parses("items|until=i => i")),
    ("until_option_thin_arrow", "items until=\"i -> i\"", Parses("items|until=i -> i")),
    ("single_quoted_values", "items 'i => i' when='on'", Parses("items|cb=i => i|when=on")),
    ("when_is_an_expression_not_a_callback", "items when=\"a -> b\"", Rejects(MALFORMED, "-")),
    // Strict option combinations and order.
    (
        "every_option_in_canonical_order",
        "items \"i => i\" when=\"on\" filter=\"i => i\" sort=\"i => i\" sort-order=\"desc\" until=\"i => i\"",
        Parses("items|cb=i => i|when=on|filter=i => i|sort=i => i|order=desc|until=i => i"),
    ),
    (
        "options_in_reverse_order",
        "items until=\"i => i\" sort-order=\"asc\" sort=\"i => i\" filter=\"i => i\" when=\"on\" \"i => i\"",
        Rejects(MALFORMED, "\"i => i\""),
    ),
    (
        "options_in_reverse_order_after_the_positional",
        "items \"i => i\" while=\"i => i\" sort-order=\"asc\" sort=\"i => i\"",
        Parses("items|cb=i => i|sort=i => i|order=asc|while=i => i"),
    ),
    ("positional_after_an_option", "items when=\"on\" \"i => i\"", Rejects(MALFORMED, "\"i => i\"")),
    ("second_positional", "items \"i => i\" \"j => j\"", Rejects(MALFORMED, "\"j => j\"")),
    ("duplicate_filter", "items filter=\"i => i\" filter=\"i => i\"", Rejects(MALFORMED, "filter")),
    ("duplicate_when", "items when=\"a\" when=\"b\"", Rejects(MALFORMED, "when")),
    ("unknown_option", "items bogus=\"x\"", Rejects(MALFORMED, "bogus")),
    ("fn_option_is_gone", "items fn=\"i => i\"", Rejects(MALFORMED, "fn")),
    ("while_with_until", "items while=\"i => i\" until=\"i => i\"", Rejects(MALFORMED, "until")),
    ("bare_flag", "items sort", Rejects(MALFORMED, "sort")),
    ("unquoted_value", "items when=on", Rejects(MALFORMED, "on")),
    ("empty_value", "items when=\"\"", Rejects(MALFORMED, "\"\"")),
    ("empty_positional", "items \"\"", Rejects(MALFORMED, "\"\"")),
    ("extra_bare_word", "items other", Rejects(MALFORMED, "other")),
    ("upper_case_option_name", "items SORT=\"i => i\"", Rejects(MALFORMED, "SORT")),
    ("upper_case_order", "items sort=\"i => i\" sort-order=\"DESC\"", Rejects(MALFORMED, "\"DESC\"")),
    ("order_without_sort", "items sort-order=\"asc\"", Rejects(MALFORMED, "sort-order")),
    ("missing_source", "", Rejects(MALFORMED, " ")),
    ("bracket_source", "items[0]", Rejects(MALFORMED, "items[0]")),
    ("call_source", "items()", Rejects(MALFORMED, "items()")),
    ("interpolated_source", "{{ items }}", Rejects(MALFORMED, "{{")),
    ("unterminated_quote", "items \"i => i", Rejects(MALFORMED, "\"i => i")),
    // Quoted arrows, nested calls and ternaries.
    ("arrow_inside_a_string_literal", "items \"i => i == '=> x'\"", Parses("items|cb=i => i == '=> x'")),
    ("thin_arrow_inside_a_string_literal", "items \"i => 'a -> b'\"", Parses("items|cb=i => 'a -> b'")),
    ("nested_calls", "items \"i => upper(trim(join(i, ',')))\"", Parses("items|cb=i => upper(trim(join(i, ',')))")),
    (
        "nested_ternaries",
        "items \"i => i > 1 ? 'a' : i > 0 ? 'b' : 'c'\"",
        Parses("items|cb=i => i > 1 ? 'a' : i > 0 ? 'b' : 'c'"),
    ),
    ("ternary_in_a_condition", "items when=\"a ? b : c\"", Parses("items|when=a ? b : c")),
    ("destructuring_pattern", "items \"{ a, b } => a\"", Parses("items|cb={ a, b } => a")),
    ("destructuring_alias_is_unsupported", "items \"{ a: x, b } => x\"", Rejects(CALLBACK, ":")),
    ("escaped_quote_in_a_callback", "items \"i => i == \\\"x\\\"\"", Parses("items|cb=i => i == \"x\"")),
    // Malformed patterns and callbacks.
    ("no_arrow", "items \"i i\"", Rejects(CALLBACK, "i")),
    ("empty_body", "items \"i => \"", Rejects(CALLBACK, "\"")),
    ("empty_pattern", "items \"=> i\"", Rejects(CALLBACK, "=")),
    ("two_arrows", "items \"i => i => i\"", Rejects(CALLBACK, "=")),
    ("dangling_operator", "items \"i => i +\"", Rejects(CALLBACK, "\"")),
    ("numeric_parameter", "items \"1 => i\"", Rejects(CALLBACK, "1")),
    ("unclosed_pattern", "items \"{ a => a\"", Rejects(CALLBACK, "=")),
    ("trailing_pattern_comma_only", "items \"{ , } => 1\"", Rejects(CALLBACK, ",")),
    ("paren_parameters", "items \"(a, b) => a\"", Rejects(CALLBACK, "(")),
    ("positional_syntax_error", "items \"i => i )\"", Rejects(CALLBACK, ")")),
    ("filter_option_without_an_arrow_is_a_callback_error", "items filter=\"i\"", Rejects(CALLBACK, "\"")),
];

fn summary(region: &CollectionRegion) -> String {
    let header = &region.header;
    let mut parts = vec![header.source.value.clone()];
    let slots = [
        ("cb", &header.callback),
        ("when", &header.when),
        ("filter", &header.filter),
        ("sort", &header.sort),
    ];
    for (name, value) in slots {
        if let Some(value) = value {
            parts.push(format!("{name}={}", value.text));
        }
    }
    if let Some(order) = &header.sort_order {
        parts.push(format!("order={}", if order.value == SortOrder::Asc { "asc" } else { "desc" }));
    }
    if let Some(value) = &header.while_predicate {
        parts.push(format!("while={}", value.text));
    }
    if let Some(value) = &header.until_predicate {
        parts.push(format!("until={}", value.text));
    }
    parts.join("|")
}

/// The 1-based line and authored text a located error points at.
fn located<'a>(source: &'a str, error: &CollectionError) -> (usize, &'a str) {
    match error.site().expect("a located error").source.as_ref() {
        SourceRef::OnDiskSpan { span, .. } => (span.line(), &source[span.range()]),
        other => panic!("expected an authored span in {error}, got {other:?}"),
    }
}

fn doc_path() -> &'static std::path::Path {
    std::path::Path::new("doc.md")
}

fn run_row(name: &str) {
    let (_, header, expect) = GRAMMAR.iter().find(|(row, ..)| *row == name).unwrap_or_else(|| panic!("no row {name}"));
    for kind in KINDS {
        let source = region(kind, header, "body\n");
        match (expect, parse_collection_regions(&source, Some(doc_path()))) {
            (Parses(want), Ok(regions)) => {
                assert_eq!(regions.len(), 1, "{name} {kind:?}");
                assert_eq!(regions[0].kind, kind, "{name}");
                assert_eq!(summary(&regions[0]), *want, "{name} {kind:?}: {source:?}");
            }
            (Rejects(code, text), Err(error)) => {
                assert_eq!(error.code(), *code, "{name} {kind:?}: {error}");
                assert_eq!(located(&source, &error), (1, *text), "{name} {kind:?}");
            }
            (Parses(_), Err(error)) => panic!("{name} {kind:?} should parse: {error}"),
            (Rejects(..), Ok(_)) => panic!("{name} {kind:?} should be rejected: {source:?}"),
        }
    }
}

row_tests! { ROW_TESTS, run_row;
    bare_source_has_every_default, dotted_dashed_source, local_prefixed_source, fm_prefixed_source,
    sort_defaults_to_no_explicit_order, positional_fat_arrow, positional_thin_arrow, filter_option_fat_arrow,
    filter_option_thin_arrow, sort_option_fat_arrow, sort_option_thin_arrow, while_option_fat_arrow,
    while_option_thin_arrow, until_option_fat_arrow, until_option_thin_arrow, single_quoted_values,
    when_is_an_expression_not_a_callback, every_option_in_canonical_order, options_in_reverse_order,
    options_in_reverse_order_after_the_positional, positional_after_an_option, second_positional,
    duplicate_filter, duplicate_when, unknown_option, fn_option_is_gone, while_with_until, bare_flag,
    unquoted_value, empty_value, empty_positional, extra_bare_word, upper_case_option_name, upper_case_order,
    order_without_sort, missing_source, bracket_source, call_source, interpolated_source, unterminated_quote,
    arrow_inside_a_string_literal, thin_arrow_inside_a_string_literal, nested_calls, nested_ternaries,
    ternary_in_a_condition, destructuring_pattern, destructuring_alias_is_unsupported, escaped_quote_in_a_callback,
    no_arrow, empty_body, empty_pattern, two_arrows, dangling_operator, numeric_parameter, unclosed_pattern,
    trailing_pattern_comma_only, paren_parameters, positional_syntax_error,
    filter_option_without_an_arrow_is_a_callback_error,
}

#[test]
fn every_generated_grammar_test_has_a_table_row_and_back() {
    let table: Vec<_> = GRAMMAR.iter().map(|(name, ..)| *name).collect();
    assert_eq!(ROW_TESTS, table.as_slice());
}

// ── closers, containers, line endings ────────────────────────────────────

/// `(row, document for a kind, error line, text the error points at)`.
type Structural = (&'static str, fn(CollectionKind) -> String, usize, fn(CollectionKind) -> String);

fn structural_rows() -> Vec<Structural> {
    vec![
        ("unterminated", |k| format!("text\n{} items\nbody\n", opener(k)), 2, |k| format!("{} items", opener(k))),
        ("stray_closer", |k| format!("a\n{}\n", closer(k)), 2, closer),
        (
            "crossed_closer",
            |k| format!("{} a\n{}\n{}\n", opener(k), closer(other(k)), closer(k)),
            2,
            |k| closer(other(k)),
        ),
        (
            "closer_with_trailing_text",
            |k| format!("{} a\n{} extra\n", opener(k), closer(k)),
            2,
            |k| format!("{} extra", closer(k)),
        ),
        (
            "interleaved_with_the_other_kind",
            |k| format!("{} a\n{} b\n{}\n{}\n", opener(k), opener(other(k)), closer(k), closer(other(k))),
            3,
            closer,
        ),
        (
            "inside_a_shell_block",
            |k| format!("::shell-block\n{} a\n{}\n::end-block\n", opener(k), closer(k)),
            2,
            |k| format!("{} a", opener(k)),
        ),
        (
            "closed_by_a_page_block_closer",
            |k| format!("{} a\n::end-block\n", opener(k)),
            2,
            |_| "::end-block".to_string(),
        ),
    ]
}

#[test]
fn every_invalid_closer_and_container_is_located_for_both_kinds() {
    for (name, build, line, text) in structural_rows() {
        for kind in KINDS {
            let source = build(kind);
            let error = parse_collection_regions(&source, Some(doc_path()))
                .expect_err(&format!("{name} {kind:?} should fail"));
            assert_eq!(error.code(), MALFORMED, "{name} {kind:?}: {error}");
            assert_eq!(located(&source, &error), (line, text(kind).as_str()), "{name} {kind:?}");
        }
    }
}

#[test]
fn a_collection_nested_in_a_page_block_or_the_other_kind_pairs_with_its_own_closer() {
    for kind in KINDS {
        let inner = region(other(kind), "b", "x\n");
        let source = format!("::block when=\"true\"\n{} a\n{inner}{}\n::end-block\n", opener(kind), closer(kind));
        let regions = parse_collection_regions(&source, None).unwrap();
        assert_eq!(regions.len(), 1, "{kind:?}");
        assert_eq!(regions[0].children.len(), 1, "{kind:?}");
        assert_eq!(regions[0].children[0].kind, other(kind));
    }
}

#[test]
fn line_endings_and_a_missing_final_newline_do_not_change_the_region() {
    for kind in KINDS {
        let lf = region(kind, "items \"i => i\" sort=\"i => i\"", "- {{ i }}\n");
        let crlf = lf.replace('\n', "\r\n");
        let bare = lf.trim_end_matches('\n').to_string();
        let bare_crlf = crlf.trim_end_matches("\r\n").to_string();
        for (label, source) in [("lf", &lf), ("crlf", &crlf), ("no final newline", &bare), ("crlf no final newline", &bare_crlf)] {
            let regions = parse_collection_regions(source, None).unwrap_or_else(|error| panic!("{label}: {error}"));
            assert_eq!(regions.len(), 1, "{label}");
            assert_eq!(summary(&regions[0]), "items|cb=i => i|sort=i => i", "{label}");
            assert_eq!((regions[0].start_line, regions[0].end_line), (1, 3), "{label}");
            let body = &source[regions[0].body_span.clone()];
            assert!(body.starts_with("- {{ i }}"), "{label}: {body:?}");
            assert!(!body.contains(&closer(kind)), "{label}: {body:?}");
        }
    }
}

#[test]
fn escaped_and_multibyte_callbacks_keep_authored_offsets_for_every_slot_and_kind() {
    for kind in KINDS {
        for slot in ["", "filter=", "sort=", "while=", "until="] {
            let header = format!("items {slot}\"é => é.name == \\\"ü\\\"\"");
            let source = region(kind, &header, "x\n");
            let regions = parse_collection_regions(&source, None).unwrap();
            let h = &regions[0].header;
            let callback = [&h.callback, &h.filter, &h.sort, &h.while_predicate, &h.until_predicate]
                .into_iter()
                .flatten()
                .next()
                .unwrap();
            let decoded = callback.text.find("\"ü\"").unwrap();
            let authored = callback.authored_range(decoded..decoded + "\"ü\"".len()).unwrap();
            assert_eq!(&source[authored], "\\\"ü\\\"", "{kind:?} {slot}");
        }
    }
}

#[test]
fn an_error_in_an_escaped_callback_points_at_the_authored_text() {
    for kind in KINDS {
        let source = region(kind, "items \"é => é == \\\"ü\\\" )\"", "x\n");
        let error = parse_collection_regions(&source, Some(doc_path())).unwrap_err();
        assert_eq!(error.code(), CALLBACK, "{error}");
        assert_eq!(located(&source, &error), (1, ")"), "{kind:?}");
    }
}

#[test]
fn directive_shaped_text_in_code_fences_is_not_a_region() {
    for kind in KINDS {
        let fenced = format!("```md\n{} items\n```\n", opener(kind));
        assert!(parse_collection_regions(&fenced, None).unwrap().is_empty(), "{kind:?}");
        let tilde = format!("~~~\n{} items\n~~~\n", opener(kind));
        assert!(parse_collection_regions(&tilde, None).unwrap().is_empty(), "{kind:?}");
    }
}

#[test]
fn the_lenient_scan_reports_both_kinds_the_same_way() {
    for kind in KINDS {
        let source = format!("{} items bogus=\"x\"\nbody\n{}\n", opener(kind), closer(kind));
        let scan = scan_collection_regions_lenient(&source);
        assert_eq!(scan.headers.len(), 1, "{kind:?}");
        assert_eq!(scan.headers[0].problems.len(), 1, "{kind:?}");
        assert!(scan.regions.is_empty(), "{kind:?}");

        let unpaired = format!("{} items\nbody\n", opener(kind));
        let scan = scan_collection_regions_lenient(&unpaired);
        assert_eq!(scan.headers.len(), 1);
        assert!(scan.structural.is_some() && scan.regions.is_empty(), "{kind:?}");
    }
}

#[test]
fn a_closer_outside_the_openers_container_cannot_close_it() {
    for kind in KINDS {
        for (name, source) in [
            ("quoted opener, plain closer", format!("> {} a\n> body\n{}\n", opener(kind), closer(kind))),
            ("plain opener, quoted closer", format!("{} a\nbody\n> {}\n", opener(kind), closer(kind))),
            ("deeper quote opener", format!("> > {} a\n> > body\n> {}\n", opener(kind), closer(kind))),
        ] {
            let error = parse_collection_regions(&source, Some(doc_path())).expect_err(&format!("{name} {kind:?}"));
            assert_eq!(error.code(), MALFORMED, "{name} {kind:?}: {error}");
        }
    }
}

#[test]
fn a_closer_accepts_trailing_whitespace_and_a_quoted_conditional_stays_literal() {
    for kind in KINDS {
        let spaced = format!("{} a\nbody\n{} \t \n", opener(kind), closer(kind));
        assert_eq!(parse_collection_regions(&spaced, None).unwrap().len(), 1, "{kind:?}");
        let quoted_conditional = format!("> ::block when=\"x\"\n> {} a\n> body\n> {}\n> ::end-block\n", opener(kind), closer(kind));
        let regions = parse_collection_regions(&quoted_conditional, None).unwrap();
        assert_eq!(regions.len(), 1, "{kind:?}: the quoted collection is active inside a quoted conditional example");
    }
}
