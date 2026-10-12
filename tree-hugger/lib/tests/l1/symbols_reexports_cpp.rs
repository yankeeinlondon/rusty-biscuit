//! Re-export extraction tests: cpp.
//!
//! `.cppm` is not a recognized extension, so module-unit samples are parsed
//! under a `.cpp` name; the grammar sees the same text.

use tree_hugger::{
    Capability, SymbolQuery,
    ReexportCaveat, ReexportCoverageState, ReexportEntry, ReexportEvidence, ReexportForm, ReexportLimitReason,
    ReexportReport,
};

use crate::reexports_support::{Snippet, snippet};

/// What an entry must say; every unlisted role is checked to be empty.
struct Expect<'a> {
    form: ReexportForm,
    name: Option<&'a str>,
    source: &'a str,
    scope: Option<&'a str>,
    caveats: &'a [ReexportCaveat],
    span: &'a str,
    statement: &'a str,
}

fn named<'a>(name: &'a str, source: &'a str, scope: Option<&'a str>, statement: &'a str) -> Expect<'a> {
    Expect {
        form: ReexportForm::Named,
        name: Some(name),
        source,
        scope,
        caveats: &[],
        span: statement.strip_prefix("using ").unwrap().strip_suffix(';').unwrap(),
        statement,
    }
}

fn directive<'a>(source: &'a str, scope: Option<&'a str>, statement: &'a str) -> Expect<'a> {
    Expect {
        form: ReexportForm::Wildcard,
        name: None,
        source,
        scope,
        caveats: &[ReexportCaveat::FallbackLookup],
        span: statement.strip_prefix("using namespace ").unwrap().strip_suffix(';').unwrap(),
        statement,
    }
}

fn assert_entry(s: &Snippet, entry: &ReexportEntry, expect: &Expect<'_>) {
    assert_eq!(entry.form, expect.form);
    assert_eq!(entry.evidence, ReexportEvidence::Syntax);
    assert_eq!(entry.exported_name.as_deref(), expect.name);
    assert_eq!(entry.original_name.as_deref(), expect.name);
    assert_eq!(entry.local_name, None);
    assert_eq!(entry.source.as_deref(), Some(expect.source));
    assert!(entry.excluded_names.is_empty());
    assert_eq!(entry.scope.as_deref(), expect.scope);
    assert_eq!(entry.restriction, None);
    assert!(!entry.type_only);
    assert!(!entry.conditional);
    assert_eq!(entry.caveats, expect.caveats);
    assert_eq!(entry.import_span, None);
    assert_eq!(s.text(&entry.span), expect.span);
    assert_eq!(s.text(&entry.statement_span), expect.statement);
}

fn assert_entries(s: &Snippet, report: &ReexportReport, expected: &[Expect<'_>]) {
    assert_eq!(report.entries.len(), expected.len(), "{:#?}", report.entries);
    for (entry, expect) in report.entries.iter().zip(expected) {
        assert_entry(s, entry, expect);
    }
}

fn assert_complete(report: &ReexportReport) {
    assert_eq!(report.coverage.state, ReexportCoverageState::Complete);
    assert!(report.coverage.limits.is_empty());
}

fn limit_reasons(report: &ReexportReport) -> Vec<ReexportLimitReason> {
    report.coverage.limits.iter().map(|limit| limit.reason).collect()
}

#[test]
fn using_declarations_in_a_namespace_are_named_forwards() {
    let s = snippet(
        "a.hpp",
        "namespace outer {\nusing ns::x;\nusing ns::detail::deep;\nusing ::global_fn;\n}\n",
    );
    let report = s.report();
    assert_complete(&report);
    assert_entries(
        &s,
        &report,
        &[
            named("x", "ns", Some("outer"), "using ns::x;"),
            named("deep", "ns::detail", Some("outer"), "using ns::detail::deep;"),
            named("global_fn", "::", Some("outer"), "using ::global_fn;"),
        ],
    );
}

#[test]
fn a_using_declaration_at_global_scope_has_no_scope() {
    let s = snippet("a.hpp", "using ns::x;\n");
    let report = s.report();
    assert_complete(&report);
    assert_entries(&s, &report, &[named("x", "ns", None, "using ns::x;")]);
}

#[test]
fn using_directives_are_wildcards_with_fallback_lookup() {
    let s = snippet(
        "a.hpp",
        "namespace outer {\nusing namespace ns;\nusing namespace ns::inner;\n}\nusing namespace std;\n",
    );
    let report = s.report();
    assert_complete(&report);
    assert_entries(
        &s,
        &report,
        &[
            directive("ns", Some("outer"), "using namespace ns;"),
            directive("ns::inner", Some("outer"), "using namespace ns::inner;"),
            directive("std", None, "using namespace std;"),
        ],
    );
}

#[test]
fn using_enum_forwards_every_enumerator_without_fallback() {
    let s = snippet("a.hpp", "namespace n {\nusing enum ns::Color;\n}\n");
    let report = s.report();
    assert_complete(&report);
    assert_eq!(report.entries.len(), 1);
    let entry = &report.entries[0];
    assert_eq!(entry.form, ReexportForm::Wildcard);
    assert_eq!(entry.exported_name, None);
    assert_eq!(entry.source.as_deref(), Some("ns::Color"));
    assert_eq!(entry.scope.as_deref(), Some("n"));
    assert!(entry.caveats.is_empty());
    assert_eq!(s.text(&entry.statement_span), "using enum ns::Color;");
}

#[test]
fn scope_is_the_qualified_enclosing_namespace() {
    let s = snippet(
        "a.hpp",
        "namespace a::b {\nnamespace c {\ninline namespace v1 { using ns::x; }\n}\nusing ns::y;\n}\nnamespace { using ns::z; }\n",
    );
    let report = s.report();
    assert_complete(&report);
    let scopes: Vec<_> = report.entries.iter().map(|e| e.scope.as_deref()).collect();
    assert_eq!(scopes, [Some("a::b::c::v1"), Some("a::b"), None]);
    let names: Vec<_> = report.entries.iter().map(|e| e.exported_name.as_deref().unwrap()).collect();
    assert_eq!(names, ["x", "y", "z"]);
}

#[test]
fn extern_c_blocks_stay_namespace_scope() {
    let s = snippet("a.hpp", "extern \"C\" {\nusing ns::f;\n}\nnamespace n { extern \"C\" { using ns::g; } }\n");
    let report = s.report();
    assert_complete(&report);
    assert_entries(
        &s,
        &report,
        &[named("f", "ns", None, "using ns::f;"), named("g", "ns", Some("n"), "using ns::g;")],
    );
}

#[test]
fn the_extern_c_include_sample_forwards_nothing() {
    let s = snippet("pos-extern-c-block.hpp", include_str!("../fixtures/reexports/cpp/pos-extern-c-block.hpp"));
    let report = s.report();
    assert_complete(&report);
    assert!(report.entries.is_empty());
}

#[test]
fn the_using_declaration_sample_reports_the_clean_forms_and_limits_the_damaged_ones() {
    let s = snippet(
        "pos-using-decl.hpp",
        include_str!("../fixtures/reexports/cpp/pos-using-decl.hpp"),
    );
    let report = s.report();
    assert_eq!(report.coverage.state, ReexportCoverageState::Limited);
    assert_eq!(limit_reasons(&report), [ReexportLimitReason::ParseError, ReexportLimitReason::ParseError]);
    let lines: Vec<_> = report.coverage.limits.iter().map(|l| l.span.start.line).collect();
    assert_eq!(lines, [5, 9]);
    // The multi-name statement and the template's `using typename` give no entry.
    let statements: Vec<_> = report.entries.iter().map(|e| s.text(&e.statement_span)).collect();
    assert_eq!(
        statements,
        ["using ns::x;", "using ns::T;", "using ::global_fn;", "using ns::detail::deep;", "using enum ns::Color;"]
    );
    assert!(report.entries.iter().all(|e| e.scope.as_deref() == Some("outer")));
}

#[test]
fn the_using_directive_sample_reports_directives_only() {
    let s = snippet(
        "pos-using-directive.hpp",
        include_str!("../fixtures/reexports/cpp/pos-using-directive.hpp"),
    );
    let report = s.report();
    assert_complete(&report);
    assert_entries(
        &s,
        &report,
        &[
            directive("ns", Some("outer"), "using namespace ns;"),
            directive("ns::inner", Some("outer"), "using namespace ns::inner;"),
            directive("std", None, "using namespace std;"),
        ],
    );
}

#[test]
fn type_aliases_are_not_forwards() {
    let s = snippet("pos-alias.hpp", include_str!("../fixtures/reexports/cpp/pos-alias.hpp"));
    let report = s.report();
    assert_complete(&report);
    assert!(report.entries.is_empty());
}

#[test]
fn class_member_using_and_function_body_using_are_not_forwards() {
    let s = snippet(
        "a.hpp",
        "struct D : B {\n using B::x;\n using B::B;\n using Base = B;\n};\nvoid f() { using ns::g; using namespace ns; }\ntemplate <class T> struct W : T { using T::v; };\nnamespace n { struct E : B { using B::y; }; }\n",
    );
    let report = s.report();
    assert_complete(&report);
    assert!(report.entries.is_empty(), "{:#?}", report.entries);
}

#[test]
fn the_class_member_sample_gives_no_entries() {
    let s = snippet(
        "pos-class-using.hpp",
        include_str!("../fixtures/reexports/cpp/pos-class-using.hpp"),
    );
    let report = s.report();
    assert!(report.entries.is_empty());
    // `using typename` is damaged syntax even inside a class.
    assert_eq!(limit_reasons(&report), [ReexportLimitReason::ParseError]);
}

#[test]
fn includes_and_plain_code_are_complete_negatives() {
    for (name, text) in [
        ("pos-include.hpp", include_str!("../fixtures/reexports/cpp/pos-include.hpp")),
        ("neg-plain.cpp", include_str!("../fixtures/reexports/cpp/neg-plain.cpp")),
    ] {
        let s = snippet(name, text);
        let report = s.report();
        assert_complete(&report);
        assert!(report.entries.is_empty(), "{name}");
    }
}

#[test]
fn the_grouped_using_sample_is_a_parse_error_not_a_complete_negative() {
    let s = snippet(
        "neg-using-grouped-invalid.hpp",
        include_str!("../fixtures/reexports/cpp/neg-using-grouped-invalid.hpp"),
    );
    let report = s.report();
    assert!(report.entries.is_empty());
    assert_eq!(report.coverage.state, ReexportCoverageState::Limited);
    assert_eq!(limit_reasons(&report), [ReexportLimitReason::ParseError]);
    assert_eq!(report.coverage.limits[0].span.start.line, 2);
}

#[test]
fn a_multi_name_using_is_limited_and_reports_no_entry_for_itself() {
    let s = snippet("a.hpp", "namespace n {\nusing ns::a, ns::b;\nusing ns::c;\n}\n");
    let report = s.report();
    assert_eq!(limit_reasons(&report), [ReexportLimitReason::ParseError]);
    assert_entries(&s, &report, &[named("c", "ns", Some("n"), "using ns::c;")]);
}

#[test]
fn using_typename_is_limited() {
    let s = snippet("a.hpp", "template <class B> struct W : B { using typename B::U; };\nusing ns::k;\n");
    let report = s.report();
    assert_eq!(limit_reasons(&report), [ReexportLimitReason::ParseError]);
    assert_entries(&s, &report, &[named("k", "ns", None, "using ns::k;")]);
}

#[test]
fn an_unrelated_parse_error_keeps_entries_and_stays_complete() {
    let s = snippet("a.hpp", "using ns::a;\nint broken = ;\nvoid g( {\nusing ns::b;\n");
    let report = s.report();
    // The damage must not reach a `using`; if the recovery swallowed one the
    // limit is the correct outcome instead.
    if report.coverage.limits.is_empty() {
        assert_eq!(report.entries.len(), 2);
    } else {
        assert_eq!(limit_reasons(&report), [ReexportLimitReason::ParseError]);
    }
    let clean = snippet("b.hpp", "using ns::a;\nint broken = ;\nusing ns::b;\n");
    let report = clean.report();
    assert_complete(&report);
    assert_entries(
        &clean,
        &report,
        &[named("a", "ns", None, "using ns::a;"), named("b", "ns", None, "using ns::b;")],
    );
}

#[test]
fn damage_directly_after_using_is_limited() {
    let s = snippet("a.hpp", "using ns::a;\nusing ;\n");
    let report = s.report();
    assert_eq!(report.coverage.state, ReexportCoverageState::Limited);
    assert_eq!(limit_reasons(&report), [ReexportLimitReason::ParseError]);
    assert_entries(&s, &report, &[named("a", "ns", None, "using ns::a;")]);
}

#[test]
fn module_samples_are_limited_over_the_whole_file() {
    for name in ["pos-export-block.cppm", "pos-export-import.cppm", "pos-module-decls.cppm", "pos-impl-unit.cpp"] {
        let text = match name {
            "pos-export-block.cppm" => include_str!("../fixtures/reexports/cpp/pos-export-block.cppm"),
            "pos-export-import.cppm" => include_str!("../fixtures/reexports/cpp/pos-export-import.cppm"),
            "pos-module-decls.cppm" => include_str!("../fixtures/reexports/cpp/pos-module-decls.cppm"),
            _ => include_str!("../fixtures/reexports/cpp/pos-impl-unit.cpp"),
        };
        let s = snippet(&name.replace(".cppm", ".cpp"), text);
        let report = s.report();
        assert_eq!(report.coverage.state, ReexportCoverageState::Limited, "{name}");
        let gap = &report.coverage.limits[0];
        assert_eq!(gap.reason, ReexportLimitReason::GrammarGap, "{name}");
        assert_eq!(gap.span.start_byte, 0, "{name}");
        assert_eq!(gap.span.end_byte as usize, text.len(), "{name}");
        assert!(report.entries.is_empty(), "{name}: {:#?}", report.entries);
    }
}

#[test]
fn module_syntax_is_found_when_misparsed_as_a_declaration_or_hidden_in_a_conditional() {
    for text in [
        "module m;\n",
        "import other;\n",
        "export module m;\n",
        "module;\nusing ns::x;\n",
        "#if X\nexport import other;\n#endif\n",
    ] {
        let s = snippet("a.cpp", text);
        let report = s.report();
        assert!(limit_reasons(&report).contains(&ReexportLimitReason::GrammarGap), "{text}");
    }
    let s = snippet("a.cpp", "namespace module_ns { using ns::x; }\nint import_count = 0;\n");
    assert_complete(&s.report());
}

#[test]
fn preprocessor_conditionals_report_entries_in_source_order_without_a_condition() {
    let s = snippet("a.hpp", "#ifdef A\nusing ns::a;\n#else\nusing ns::b;\n#endif\nusing ns::c;\n");
    let report = s.report();
    assert_complete(&report);
    assert_entries(
        &s,
        &report,
        &[
            named("a", "ns", None, "using ns::a;"),
            named("b", "ns", None, "using ns::b;"),
            named("c", "ns", None, "using ns::c;"),
        ],
    );
}

#[test]
fn repeated_forwards_keep_one_entry_per_site() {
    let s = snippet("a.hpp", "using ns::a;\nusing ns::a;\n");
    let report = s.report();
    assert_eq!(report.entries.len(), 2);
    assert_ne!(report.entries[0].span, report.entries[1].span);
    assert!(report.entries[0].span.start_byte < report.entries[1].span.start_byte);
}

#[test]
fn no_import_is_marked_reexported() {
    let s = snippet("a.hpp", "#include <vector>\n#include \"other.hpp\"\nnamespace n { using ns::x; }\nusing namespace std;\n");
    let index = s
        .file
        .query(&SymbolQuery::new().kinds([]).capabilities([Capability::Declarations, Capability::Reexports]))
        .expect("query")
        .index;
    let report = index.reexports.expect("report requested");
    assert_eq!(report.entries.len(), 2);
    assert!(report.entries.iter().all(|e| e.import_span.is_none()));
    // C++ records no import for `#include`, so there is nothing to mark.
    assert!(index.imports.iter().all(|import| !import.reexported));
}

#[test]
fn the_query_and_direct_paths_agree() {
    let s = snippet("a.hpp", "namespace n { using ns::x; }\nusing namespace std;\n");
    assert_eq!(s.indexed().reexports.expect("report requested"), s.report());
}

#[test]
fn every_cpp_audit_sample_extracts_with_ordered_unique_output() {
    let samples = [
        ("neg-plain.cpp", include_str!("../fixtures/reexports/cpp/neg-plain.cpp")),
        ("neg-using-grouped-invalid.hpp", include_str!("../fixtures/reexports/cpp/neg-using-grouped-invalid.hpp")),
        ("pos-alias.hpp", include_str!("../fixtures/reexports/cpp/pos-alias.hpp")),
        ("pos-class-using.hpp", include_str!("../fixtures/reexports/cpp/pos-class-using.hpp")),
        ("pos-export-block.cpp", include_str!("../fixtures/reexports/cpp/pos-export-block.cppm")),
        ("pos-export-import.cpp", include_str!("../fixtures/reexports/cpp/pos-export-import.cppm")),
        ("pos-extern-c-block.hpp", include_str!("../fixtures/reexports/cpp/pos-extern-c-block.hpp")),
        ("pos-impl-unit.cpp", include_str!("../fixtures/reexports/cpp/pos-impl-unit.cpp")),
        ("pos-include.hpp", include_str!("../fixtures/reexports/cpp/pos-include.hpp")),
        ("pos-module-decls.cpp", include_str!("../fixtures/reexports/cpp/pos-module-decls.cppm")),
        ("pos-using-decl.hpp", include_str!("../fixtures/reexports/cpp/pos-using-decl.hpp")),
        ("pos-using-directive.hpp", include_str!("../fixtures/reexports/cpp/pos-using-directive.hpp")),
    ];
    for (name, text) in samples {
        let report = snippet(name, text).report();
        let starts: Vec<_> = report.entries.iter().map(|e| e.span.start_byte).collect();
        assert!(starts.windows(2).all(|w| w[0] < w[1]), "{name}: entries unordered or duplicate");
        let keys: Vec<_> = report.coverage.limits.iter().map(|l| (l.span.start_byte, l.reason)).collect();
        assert!(keys.windows(2).all(|w| w[0] <= w[1]), "{name}: limits unordered");
        let limited = report.coverage.state == ReexportCoverageState::Limited;
        assert_eq!(limited, !report.coverage.limits.is_empty(), "{name}");
    }
}
