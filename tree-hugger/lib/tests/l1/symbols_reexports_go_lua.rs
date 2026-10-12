//! Re-export extraction tests: Go alias and value-copy forwarding, and Lua
//! returned-table forwarding.

use tree_hugger::{
    ReexportCaveat, ReexportCoverageState, ReexportEntry, ReexportEvidence, ReexportForm, ReexportLimitReason,
    ReexportReport,
};

use crate::reexports_support::{Snippet, snippet};

/// The facts one entry must state; roles it does not name must be empty.
struct Expect<'a> {
    form: ReexportForm,
    evidence: ReexportEvidence,
    exported: Option<&'a str>,
    original: Option<&'a str>,
    local: Option<&'a str>,
    source: Option<&'a str>,
    caveats: &'a [ReexportCaveat],
    conditional: bool,
    span: &'a str,
    statement: &'a str,
    /// Text of the import record the entry forwards.
    import: Option<&'a str>,
}

fn assert_entry(s: &Snippet, found: &ReexportEntry, expect: &Expect<'_>) {
    assert_eq!(found.form, expect.form, "form of {}", expect.span);
    assert_eq!(found.evidence, expect.evidence, "evidence of {}", expect.span);
    assert_eq!(found.exported_name.as_deref(), expect.exported, "exported of {}", expect.span);
    assert_eq!(found.original_name.as_deref(), expect.original, "original of {}", expect.span);
    assert_eq!(found.local_name.as_deref(), expect.local, "local of {}", expect.span);
    assert_eq!(found.source.as_deref(), expect.source, "source of {}", expect.span);
    assert_eq!(found.caveats, expect.caveats, "caveats of {}", expect.span);
    assert_eq!(found.conditional, expect.conditional, "conditional of {}", expect.span);
    assert_eq!(s.text(&found.span), expect.span);
    assert_eq!(s.text(&found.statement_span), expect.statement);
    assert_eq!(found.import_span.as_ref().map(|span| s.text(span)), expect.import, "import of {}", expect.span);
    assert!(found.excluded_names.is_empty());
    assert!(found.scope.is_none());
    assert!(found.restriction.is_none());
    assert!(!found.type_only);
}

fn assert_complete(report: &ReexportReport) {
    assert_eq!(report.coverage.state, ReexportCoverageState::Complete);
    assert!(report.coverage.limits.is_empty());
}

fn assert_negative(s: &Snippet) {
    let report = s.report();
    assert!(report.entries.is_empty(), "unexpected entries: {:?}", report.entries);
    assert_complete(&report);
}

fn assert_limited(report: &ReexportReport, reasons: &[ReexportLimitReason]) {
    assert_eq!(report.coverage.state, ReexportCoverageState::Limited);
    let found: Vec<_> = report.coverage.limits.iter().map(|limit| limit.reason).collect();
    assert_eq!(found, reasons);
}

/// Invariants every report keeps, whatever the input.
fn assert_report_invariants(name: &str, report: &ReexportReport) {
    let starts: Vec<_> = report.entries.iter().map(|e| (e.span.start_byte, e.span.end_byte)).collect();
    let mut sorted = starts.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(starts, sorted, "{name}: entries are source-ordered with unique spans");
    let keys: Vec<_> = report.coverage.limits.iter().map(|l| (l.span.start_byte, l.reason)).collect();
    let mut sorted_keys = keys.clone();
    sorted_keys.sort();
    assert_eq!(keys, sorted_keys, "{name}: limits are ordered");
    let limited = report.coverage.state == ReexportCoverageState::Limited;
    assert_eq!(limited, !report.coverage.limits.is_empty(), "{name}: limited iff limits");
    assert_ne!(report.coverage.state, ReexportCoverageState::NotApplicable, "{name}");
}

const INFERRED: &[ReexportCaveat] = &[ReexportCaveat::InferredPackageName];

// ---------------------------------------------------------------- Go

fn go_alias<'a>(exported: &'a str, original: &'a str, local: &'a str, source: &'a str, span: &'a str, statement: &'a str, import: &'a str) -> Expect<'a> {
    Expect {
        form: ReexportForm::Named,
        evidence: ReexportEvidence::Syntax,
        exported: Some(exported),
        original: Some(original),
        local: Some(local),
        source: Some(source),
        caveats: INFERRED,
        conditional: false,
        span,
        statement,
        import: Some(import),
    }
}

#[test]
fn go_alias_forwards_a_package_type() {
    let s = snippet("pos-alias.go", include_str!("../fixtures/reexports/go/pos-alias.go"));
    let report = s.report();
    assert_complete(&report);
    assert_eq!(report.entries.len(), 1);
    assert_entry(
        &s,
        &report.entries[0],
        &go_alias("X", "X", "pkg", "example.com/pkg", "X = pkg.X", "type X = pkg.X", "\"example.com/pkg\""),
    );
}

#[test]
fn go_alias_marks_the_forwarded_import_only() {
    let s = snippet(
        "marker.go",
        "package p\n\nimport (\n\t\"example.com/used\"\n\t\"example.com/unused\"\n)\n\ntype X = used.X\n",
    );
    let index = s.indexed();
    assert_eq!(index.imports.len(), 2);
    let marked: Vec<_> = index.imports.iter().map(|i| (i.source.as_deref().unwrap_or(""), i.reexported)).collect();
    assert_eq!(marked, [("example.com/used", true), ("example.com/unused", false)]);
    let report = index.reexports.expect("reexports computed");
    assert_eq!(report.entries.len(), 1);
    assert_eq!(report.entries[0].import_span.as_ref(), Some(&index.imports[0].span));
}

#[test]
fn go_alias_renames_the_exported_name() {
    let s = snippet("pos-alias-renamed.go", include_str!("../fixtures/reexports/go/pos-alias-renamed.go"));
    let report = s.report();
    assert_eq!(report.entries.len(), 1);
    assert_entry(
        &s,
        &report.entries[0],
        &go_alias("Y", "X", "pkg", "example.com/pkg", "Y = pkg.X", "type Y = pkg.X", "\"example.com/pkg\""),
    );
}

#[test]
fn go_alias_through_an_aliased_import_has_no_inferred_caveat() {
    let s = snippet("pos-alias-aliased-import.go", include_str!("../fixtures/reexports/go/pos-alias-aliased-import.go"));
    let report = s.report();
    assert_eq!(report.entries.len(), 1);
    let expect = Expect { caveats: &[], ..go_alias("X", "X", "q", "example.com/pkg", "X = q.X", "type X = q.X", "q") };
    assert_entry(&s, &report.entries[0], &expect);
    let index = s.indexed();
    assert!(index.imports.iter().any(|i| i.reexported));
}

#[test]
fn go_alias_matches_the_last_path_segment() {
    let s = snippet("pos-alias-nested-pkgpath.go", include_str!("../fixtures/reexports/go/pos-alias-nested-pkgpath.go"));
    let report = s.report();
    assert_eq!(report.entries.len(), 1);
    assert_entry(
        &s,
        &report.entries[0],
        &go_alias("X", "X", "sub", "example.com/pkg/sub", "X = sub.X", "type X = sub.X", "\"example.com/pkg/sub\""),
    );
}

#[test]
fn go_alias_grouped_gives_one_entry_per_alias() {
    let s = snippet("pos-alias-grouped.go", include_str!("../fixtures/reexports/go/pos-alias-grouped.go"));
    let report = s.report();
    assert_complete(&report);
    assert_eq!(report.entries.len(), 2);
    let statement = "type (\n\tA = pkg.A\n\tB = pkg.B\n)";
    assert_entry(&s, &report.entries[0], &go_alias("A", "A", "pkg", "example.com/pkg", "A = pkg.A", statement, "\"example.com/pkg\""));
    assert_entry(&s, &report.entries[1], &go_alias("B", "B", "pkg", "example.com/pkg", "B = pkg.B", statement, "\"example.com/pkg\""));
    assert_eq!(s.indexed().imports.iter().filter(|i| i.reexported).count(), 1);
}

#[test]
fn go_generic_alias_forwards_the_generic_type() {
    let s = snippet("pos-alias-generic.go", include_str!("../fixtures/reexports/go/pos-alias-generic.go"));
    let report = s.report();
    assert_eq!(report.entries.len(), 1);
    assert_entry(
        &s,
        &report.entries[0],
        &go_alias("G", "G", "pkg", "example.com/pkg", "G[T any] = pkg.G[T]", "type G[T any] = pkg.G[T]", "\"example.com/pkg\""),
    );
}

#[test]
fn go_pointer_alias_is_not_a_forward() {
    let s = snippet("pos-alias-pointer.go", include_str!("../fixtures/reexports/go/pos-alias-pointer.go"));
    assert_negative(&s);
}

#[test]
fn go_build_constraint_marks_every_entry_conditional() {
    let s = snippet("pos-alias-build-tag.go", include_str!("../fixtures/reexports/go/pos-alias-build-tag.go"));
    let report = s.report();
    assert_eq!(report.entries.len(), 1);
    let expect = Expect {
        conditional: true,
        ..go_alias("X", "X", "pkg", "example.com/pkg", "X = pkg.X", "type X = pkg.X", "\"example.com/pkg\"")
    };
    assert_entry(&s, &report.entries[0], &expect);

    let mixed = snippet(
        "mixed.go",
        "// +build linux\n\npackage p\n\nimport \"example.com/pkg\"\n\ntype X = pkg.X\nvar Y = pkg.Y\n",
    );
    let report = mixed.report();
    assert_eq!(report.entries.len(), 2);
    assert!(report.entries.iter().all(|e| e.conditional));
}

#[test]
fn go_constraint_text_after_the_package_clause_is_not_a_constraint() {
    let s = snippet(
        "late.go",
        "package p\n\n//go:build linux\n\nimport \"example.com/pkg\"\n\ntype X = pkg.X\n",
    );
    let report = s.report();
    assert_eq!(report.entries.len(), 1);
    assert!(!report.entries[0].conditional);
}

fn go_value<'a>(exported: &'a str, original: &'a str, span: &'a str, statement: &'a str, caveats: &'a [ReexportCaveat]) -> Expect<'a> {
    Expect {
        form: ReexportForm::ValueCopy,
        evidence: ReexportEvidence::Declared,
        exported: Some(exported),
        original: Some(original),
        local: Some("pkg"),
        source: Some("example.com/pkg"),
        caveats,
        conditional: false,
        span,
        statement,
        import: Some("\"example.com/pkg\""),
    }
}

const VAR_CAVEATS: &[ReexportCaveat] = &[ReexportCaveat::MutableBinding, ReexportCaveat::InferredPackageName];

#[test]
fn go_var_copy_is_a_mutable_value_copy() {
    let s = snippet("pos-var.go", include_str!("../fixtures/reexports/go/pos-var.go"));
    let report = s.report();
    assert_complete(&report);
    assert_eq!(report.entries.len(), 1);
    assert_entry(&s, &report.entries[0], &go_value("X", "Y", "X = pkg.Y", "var X = pkg.Y", VAR_CAVEATS));
    assert!(s.indexed().imports.iter().all(|i| i.reexported), "declared value copies mark their import");
}

#[test]
fn go_typed_var_and_func_var_copies_are_reported() {
    let typed = snippet("pos-var-typed.go", include_str!("../fixtures/reexports/go/pos-var-typed.go"));
    let report = typed.report();
    assert_eq!(report.entries.len(), 1);
    assert_entry(&typed, &report.entries[0], &go_value("X", "Y", "X T = pkg.Y", "var X T = pkg.Y", VAR_CAVEATS));

    let func = snippet("pos-func-var.go", include_str!("../fixtures/reexports/go/pos-func-var.go"));
    let report = func.report();
    assert_eq!(report.entries.len(), 1);
    assert_entry(&func, &report.entries[0], &go_value("F", "Func", "F = pkg.Func", "var F = pkg.Func", VAR_CAVEATS));
}

#[test]
fn go_const_copy_has_no_mutable_caveat() {
    let s = snippet("pos-const.go", include_str!("../fixtures/reexports/go/pos-const.go"));
    let report = s.report();
    assert_eq!(report.entries.len(), 1);
    assert_entry(&s, &report.entries[0], &go_value("C", "K", "C = pkg.K", "const C = pkg.K", INFERRED));
}

#[test]
fn go_var_copy_ignores_a_local_shadow_inside_a_function() {
    let s = snippet("pos-var-after-localshadow.go", include_str!("../fixtures/reexports/go/pos-var-after-localshadow.go"));
    let report = s.report();
    assert_eq!(report.entries.len(), 1);
    assert_eq!(report.entries[0].exported_name.as_deref(), Some("X"));
}

#[test]
fn go_grouped_value_specs_and_positional_pairing() {
    let s = snippet(
        "group.go",
        "package p\n\nimport \"example.com/pkg\"\n\nvar (\n\tA = pkg.A\n\tb = pkg.B\n)\n\nconst X, Z = pkg.K, 1\n\nvar P, Q = pkg.P, pkg.Q\n",
    );
    let report = s.report();
    assert_complete(&report);
    let found: Vec<_> = report
        .entries
        .iter()
        .map(|e| (s.text(&e.span).to_owned(), e.exported_name.clone().unwrap(), e.original_name.clone().unwrap()))
        .collect();
    assert_eq!(
        found,
        [
            ("A = pkg.A".to_owned(), "A".to_owned(), "A".to_owned()),
            ("X".to_owned(), "X".to_owned(), "K".to_owned()),
            ("P".to_owned(), "P".to_owned(), "P".to_owned()),
            ("Q".to_owned(), "Q".to_owned(), "Q".to_owned()),
        ]
    );
    assert_eq!(s.text(&report.entries[0].statement_span), "var (\n\tA = pkg.A\n\tb = pkg.B\n)");
    assert_eq!(s.text(&report.entries[1].statement_span), "const X, Z = pkg.K, 1");
}

#[test]
fn go_reassigned_var_keeps_the_mutable_caveat_entry() {
    let s = snippet("neg-var-mutated.go", include_str!("../fixtures/reexports/go/neg-var-mutated.go"));
    let report = s.report();
    assert_complete(&report);
    assert_eq!(report.entries.len(), 1);
    assert_eq!(report.entries[0].caveats, VAR_CAVEATS);

    let paired = snippet("neg-var-mutable-nonconst.go", include_str!("../fixtures/reexports/go/neg-var-mutable-nonconst.go"));
    let report = paired.report();
    assert_eq!(report.entries.len(), 1, "only the imported value pairs with an exported name");
    assert_eq!(paired.text(&report.entries[0].span), "X");
}

#[test]
fn go_negative_samples_report_nothing() {
    let samples = [
        ("neg-alias-builtin.go", include_str!("../fixtures/reexports/go/neg-alias-builtin.go")),
        ("neg-alias-in-func.go", include_str!("../fixtures/reexports/go/neg-alias-in-func.go")),
        ("neg-alias-local.go", include_str!("../fixtures/reexports/go/neg-alias-local.go")),
        ("neg-alias-shadowed.go", include_str!("../fixtures/reexports/go/neg-alias-shadowed.go")),
        ("neg-alias-unexported.go", include_str!("../fixtures/reexports/go/neg-alias-unexported.go")),
        ("neg-defined-type-grouped.go", include_str!("../fixtures/reexports/go/neg-defined-type-grouped.go")),
        ("neg-defined-type.go", include_str!("../fixtures/reexports/go/neg-defined-type.go")),
        ("neg-embedded-struct.go", include_str!("../fixtures/reexports/go/neg-embedded-struct.go")),
        ("neg-func-wrapper.go", include_str!("../fixtures/reexports/go/neg-func-wrapper.go")),
        ("neg-iface-embed.go", include_str!("../fixtures/reexports/go/neg-iface-embed.go")),
        ("neg-import-aliased.go", include_str!("../fixtures/reexports/go/neg-import-aliased.go")),
        ("neg-import-blank.go", include_str!("../fixtures/reexports/go/neg-import-blank.go")),
        ("neg-import-dot.go", include_str!("../fixtures/reexports/go/neg-import-dot.go")),
        ("neg-import-grouped.go", include_str!("../fixtures/reexports/go/neg-import-grouped.go")),
        ("neg-method-expr.go", include_str!("../fixtures/reexports/go/neg-method-expr.go")),
        ("neg-var-call.go", include_str!("../fixtures/reexports/go/neg-var-call.go")),
        ("pos-import-plain.go", include_str!("../fixtures/reexports/go/pos-import-plain.go")),
        // A dot import leaves `type X = Y` unqualified: nothing anchors it.
        ("amb-dot-import-alias.go", include_str!("../fixtures/reexports/go/amb-dot-import-alias.go")),
    ];
    for (name, source) in samples {
        let s = snippet(name, source);
        let report = s.report();
        assert!(report.entries.is_empty(), "{name}: {:?}", report.entries);
        assert_complete(&report);
        assert!(s.indexed().imports.iter().all(|i| !i.reexported), "{name}: no import is forwarded");
    }
}

#[test]
fn go_qualifier_must_name_an_import_of_the_file() {
    let s = snippet("noimport.go", "package p\n\nimport \"example.com/other\"\n\ntype X = pkg.X\nvar Y = pkg.Y\n");
    assert_negative(&s);
}

#[test]
fn go_blank_and_dot_imports_never_bind_a_qualifier() {
    let s = snippet(
        "binding.go",
        "package p\n\nimport (\n\t_ \"example.com/pkg\"\n\t. \"example.com/dot\"\n)\n\ntype X = pkg.X\ntype Y = dot.Y\n",
    );
    assert_negative(&s);
}

#[test]
fn go_explicit_alias_replaces_the_path_segment() {
    let s = snippet("alias.go", "package p\n\nimport q \"example.com/pkg\"\n\ntype X = pkg.X\n");
    assert_negative(&s);
}

#[test]
fn go_same_exported_name_at_distinct_sites_stays_separate() {
    let s = snippet("dup.go", "package p\n\nimport \"example.com/pkg\"\n\ntype X = pkg.X\nvar X = pkg.X\n");
    let report = s.report();
    assert_eq!(report.entries.len(), 2);
    assert_eq!(report.entries[0].form, ReexportForm::Named);
    assert_eq!(report.entries[1].form, ReexportForm::ValueCopy);
    assert_ne!(report.entries[0].span, report.entries[1].span);
}

#[test]
fn go_entries_follow_source_order_across_declaration_kinds() {
    let s = snippet(
        "order.go",
        "package p\n\nimport \"example.com/pkg\"\n\nvar B = pkg.B\ntype A = pkg.A\nconst C = pkg.C\n",
    );
    let report = s.report();
    let names: Vec<_> = report.entries.iter().map(|e| e.exported_name.clone().unwrap()).collect();
    assert_eq!(names, ["B", "A", "C"]);
}

#[test]
fn go_query_carries_the_same_report() {
    let s = snippet("q.go", "package p\n\nimport \"example.com/pkg\"\n\ntype X = pkg.X\n");
    let direct = s.report();
    let via_query = s.indexed().reexports.expect("computed");
    assert_eq!(direct.entries.len(), via_query.entries.len());
    assert_eq!(direct.entries[0].span, via_query.entries[0].span);
}

#[test]
fn go_unrelated_parse_error_keeps_entries_and_stays_complete() {
    let s = snippet(
        "unrelated.go",
        "package p\n\nimport \"example.com/pkg\"\n\ntype X = pkg.X\n\nfunc f() { x := := }\n",
    );
    let report = s.report();
    assert_eq!(report.entries.len(), 1);
    assert_complete(&report);
}

#[test]
fn go_damaged_forwarding_syntax_limits_coverage() {
    let s = snippet(
        "damaged.go",
        "package p\n\nimport \"example.com/pkg\"\n\ntype A = pkg.A\n\ntype B = = pkg.B\n\nvar C = pkg.C\n",
    );
    let report = s.report();
    assert_limited(&report, &[ReexportLimitReason::ParseError]);
    assert!(report.entries.iter().any(|e| e.exported_name.as_deref() == Some("A")));
    assert!(report.entries.iter().any(|e| e.exported_name.as_deref() == Some("C")));
    let limit = &report.coverage.limits[0];
    assert_eq!(s.text(&limit.span), "=", "the limit covers the damaged node");
}

#[test]
fn go_damaged_var_declaration_never_reads_as_a_complete_negative() {
    let s = snippet("damaged-var.go", "package p\n\nimport \"example.com/pkg\"\n\nvar X = = pkg.X\n");
    let report = s.report();
    assert_limited(&report, &[ReexportLimitReason::ParseError]);
}

// ---------------------------------------------------------------- Lua

fn lua_module<'a>(source: &'a str, local: Option<&'a str>, span: &'a str, statement: &'a str) -> Expect<'a> {
    Expect {
        form: ReexportForm::Module,
        evidence: ReexportEvidence::Declared,
        exported: None,
        original: None,
        local,
        source: Some(source),
        caveats: &[],
        conditional: false,
        span,
        statement,
        import: None,
    }
}

fn lua_named<'a>(exported: &'a str, original: &'a str, local: Option<&'a str>, source: &'a str, span: &'a str, statement: &'a str) -> Expect<'a> {
    Expect {
        form: ReexportForm::Named,
        evidence: ReexportEvidence::Declared,
        exported: Some(exported),
        original: Some(original),
        local,
        source: Some(source),
        caveats: &[],
        conditional: false,
        span,
        statement,
        import: None,
    }
}

#[test]
fn lua_return_require_forwards_the_whole_module() {
    let s = snippet("pos-return-require.lua", include_str!("../fixtures/reexports/lua/pos-return-require.lua"));
    let report = s.report();
    assert_complete(&report);
    assert_eq!(report.entries.len(), 1);
    assert_entry(&s, &report.entries[0], &lua_module("m", None, "require(\"m\")", "return require(\"m\")"));
    assert!(s.indexed().imports.is_empty(), "Lua has no import records");
}

#[test]
fn lua_return_require_call_styles_all_forward() {
    let nop = snippet("pos-return-require-nop.lua", include_str!("../fixtures/reexports/lua/pos-return-require-nop.lua"));
    let longstr = snippet("pos-return-require-longstr.lua", include_str!("../fixtures/reexports/lua/pos-return-require-longstr.lua"));
    let dotted = snippet("pos-return-require-dotted.lua", include_str!("../fixtures/reexports/lua/pos-return-require-dotted.lua"));
    let parenthesized = snippet("pos-return-parenthesized.lua", include_str!("../fixtures/reexports/lua/pos-return-parenthesized.lua"));

    let report = nop.report();
    assert_entry(&nop, &report.entries[0], &lua_module("m", None, "require 'm'", "return require 'm'"));
    let report = longstr.report();
    assert_entry(&longstr, &report.entries[0], &lua_module("m", None, "require [[m]]", "return require [[m]]"));
    let report = dotted.report();
    assert_entry(&dotted, &report.entries[0], &lua_module("pkg.sub", None, "require(\"pkg.sub\")", "return require(\"pkg.sub\")"));
    let report = parenthesized.report();
    assert_eq!(report.entries.len(), 1);
    assert_entry(&parenthesized, &report.entries[0], &lua_module("m", None, "require(\"m\")", "return (require(\"m\"))"));
}

#[test]
fn lua_return_of_a_required_local_forwards_the_module() {
    let s = snippet("pos-local-return.lua", include_str!("../fixtures/reexports/lua/pos-local-return.lua"));
    let report = s.report();
    assert_complete(&report);
    assert_eq!(report.entries.len(), 1);
    assert_entry(&s, &report.entries[0], &lua_module("m", Some("m"), "m", "return m"));

    let hybrid = snippet("pos-return-module-after-assign.lua", include_str!("../fixtures/reexports/lua/pos-return-module-after-assign.lua"));
    let report = hybrid.report();
    assert_complete(&report);
    assert_eq!(report.entries.len(), 1);
    assert_entry(&hybrid, &report.entries[0], &lua_module("m", Some("M"), "M", "return M"));
}

#[test]
fn lua_returned_constructor_names_imported_members() {
    let s = snippet("pos-table-ctor.lua", include_str!("../fixtures/reexports/lua/pos-table-ctor.lua"));
    let report = s.report();
    assert_complete(&report);
    assert_eq!(report.entries.len(), 1);
    assert_entry(&s, &report.entries[0], &lua_named("x", "x", Some("m"), "m", "x = m.x", "return { x = m.x }"));
}

#[test]
fn lua_constructor_field_can_rename_and_require_inline() {
    let s = snippet(
        "rename.lua",
        "local m = require(\"m\")\nreturn { renamed = m.orig, inline = require(\"n\").y }\n",
    );
    let report = s.report();
    assert_eq!(report.entries.len(), 2);
    let statement = "return { renamed = m.orig, inline = require(\"n\").y }";
    assert_entry(&s, &report.entries[0], &lua_named("renamed", "orig", Some("m"), "m", "renamed = m.orig", statement));
    assert_entry(&s, &report.entries[1], &lua_named("inline", "y", None, "n", "inline = require(\"n\").y", statement));
}

#[test]
fn lua_local_table_constructor_collects_fields() {
    let s = snippet("pos-table-ctor-localvar.lua", include_str!("../fixtures/reexports/lua/pos-table-ctor-localvar.lua"));
    let report = s.report();
    assert_complete(&report);
    assert_eq!(report.entries.len(), 2);
    let statement = "local M = { x = m.x, y = require(\"n\").y }";
    assert_entry(&s, &report.entries[0], &lua_named("x", "x", Some("m"), "m", "x = m.x", statement));
    assert_entry(&s, &report.entries[1], &lua_named("y", "y", None, "n", "y = require(\"n\").y", statement));
}

#[test]
fn lua_constructor_field_holding_a_module_is_a_namespace() {
    let s = snippet("pos-table-ctor-module.lua", include_str!("../fixtures/reexports/lua/pos-table-ctor-module.lua"));
    let report = s.report();
    assert_eq!(report.entries.len(), 1);
    let expect = Expect {
        form: ReexportForm::Namespace,
        exported: Some("m"),
        original: None,
        ..lua_module("m", Some("m"), "m = m", "return { m = m }")
    };
    assert_entry(&s, &report.entries[0], &expect);

    let multi = snippet("pos-multi-return.lua", include_str!("../fixtures/reexports/lua/pos-multi-return.lua"));
    let report = multi.report();
    assert_eq!(report.entries.len(), 2);
    assert_eq!(report.entries[0].exported_name.as_deref(), Some("a"));
    assert_eq!(report.entries[0].source.as_deref(), Some("a"));
    assert_eq!(report.entries[1].exported_name.as_deref(), Some("b"));
    assert_eq!(report.entries[1].form, ReexportForm::Namespace);
}

#[test]
fn lua_positional_constructor_field_has_no_name_and_is_not_reported() {
    let s = snippet("pos-table-ctor-positional.lua", include_str!("../fixtures/reexports/lua/pos-table-ctor-positional.lua"));
    assert_negative(&s);
}

#[test]
fn lua_assigned_fields_before_return_m_forward_members() {
    let direct = snippet("pos-table-field-require.lua", include_str!("../fixtures/reexports/lua/pos-table-field-require.lua"));
    let report = direct.report();
    assert_complete(&report);
    assert_eq!(report.entries.len(), 1);
    let line = "M.foo = require(\"m\").foo";
    assert_entry(&direct, &report.entries[0], &lua_named("foo", "foo", None, "m", line, line));

    let chain = snippet("pos-require-string-call-chain.lua", include_str!("../fixtures/reexports/lua/pos-require-string-call-chain.lua"));
    let report = chain.report();
    assert_eq!(report.entries.len(), 1);
    let line = "M.foo = require \"m\".foo";
    assert_entry(&chain, &report.entries[0], &lua_named("foo", "foo", None, "m", line, line));

    let local = snippet("pos-table-field-local.lua", include_str!("../fixtures/reexports/lua/pos-table-field-local.lua"));
    let report = local.report();
    let line = "M.foo = m.foo";
    assert_entry(&local, &report.entries[0], &lua_named("foo", "foo", Some("m"), "m", line, line));

    let index = snippet("pos-table-field-alias-index.lua", include_str!("../fixtures/reexports/lua/pos-table-field-alias-index.lua"));
    let report = index.report();
    let line = "M[\"foo\"] = m.foo";
    assert_entry(&index, &report.entries[0], &lua_named("foo", "foo", Some("m"), "m", line, line));
}

#[test]
fn lua_field_name_may_differ_from_the_member() {
    let s = snippet("rename.lua", "local m = require(\"m\")\nlocal M = {}\nM.bar = m.foo\nreturn M\n");
    let report = s.report();
    assert_eq!(report.entries.len(), 1);
    assert_entry(&s, &report.entries[0], &lua_named("bar", "foo", Some("m"), "m", "M.bar = m.foo", "M.bar = m.foo"));
}

#[test]
fn lua_the_last_write_to_a_field_decides() {
    let overridden = snippet(
        "override.lua",
        "local m = require(\"m\")\nlocal M = {}\nM.foo = m.foo\nfunction M.foo() end\nM.bar = 1\nreturn M\n",
    );
    assert_negative(&overridden);

    let replaced = snippet(
        "replace.lua",
        "local m = require(\"m\")\nlocal n = require(\"n\")\nlocal M = {}\nM.foo = m.foo\nM.foo = n.foo\nreturn M\n",
    );
    let report = replaced.report();
    assert_eq!(report.entries.len(), 1);
    assert_eq!(report.entries[0].source.as_deref(), Some("n"));
    assert_eq!(report.entries[0].local_name.as_deref(), Some("n"));
}

#[test]
fn lua_return_of_a_member_is_not_reported() {
    let field = snippet("pos-return-field.lua", include_str!("../fixtures/reexports/lua/pos-return-field.lua"));
    assert_negative(&field);
    let local = snippet("pos-return-local-field.lua", include_str!("../fixtures/reexports/lua/pos-return-local-field.lua"));
    assert_negative(&local);
}

#[test]
fn lua_local_function_member_has_no_require_provenance() {
    let s = snippet("pos-local-func-field.lua", include_str!("../fixtures/reexports/lua/pos-local-func-field.lua"));
    assert_negative(&s);
}

#[test]
fn lua_negative_samples_report_nothing_and_stay_complete() {
    let samples = [
        ("neg-call-result.lua", include_str!("../fixtures/reexports/lua/neg-call-result.lua")),
        ("neg-global-assign.lua", include_str!("../fixtures/reexports/lua/neg-global-assign.lua")),
        ("neg-method-wrap.lua", include_str!("../fixtures/reexports/lua/neg-method-wrap.lua")),
        ("neg-no-return.lua", include_str!("../fixtures/reexports/lua/neg-no-return.lua")),
        ("neg-require-concat.lua", include_str!("../fixtures/reexports/lua/neg-require-concat.lua")),
        ("neg-require-dynamic.lua", include_str!("../fixtures/reexports/lua/neg-require-dynamic.lua")),
        ("neg-require-unreturned.lua", include_str!("../fixtures/reexports/lua/neg-require-unreturned.lua")),
        ("neg-return-conditional.lua", include_str!("../fixtures/reexports/lua/neg-return-conditional.lua")),
        ("neg-return-in-function.lua", include_str!("../fixtures/reexports/lua/neg-return-in-function.lua")),
        ("neg-return-ternary.lua", include_str!("../fixtures/reexports/lua/neg-return-ternary.lua")),
    ];
    for (name, source) in samples {
        let s = snippet(name, source);
        let report = s.report();
        assert!(report.entries.is_empty(), "{name}: {:?}", report.entries);
        assert_complete(&report);
    }
}

#[test]
fn lua_rebound_bindings_limit_coverage() {
    let table = snippet("neg-rebound-table.lua", include_str!("../fixtures/reexports/lua/neg-rebound-table.lua"));
    let report = table.report();
    assert!(report.entries.is_empty());
    assert_limited(&report, &[ReexportLimitReason::ReboundBinding]);
    assert_eq!(table.text(&report.coverage.limits[0].span), "M = {}");

    let module = snippet("neg-rebound-module.lua", include_str!("../fixtures/reexports/lua/neg-rebound-module.lua"));
    let report = module.report();
    assert!(report.entries.is_empty());
    assert_limited(&report, &[ReexportLimitReason::ReboundBinding]);
    assert_eq!(module.text(&report.coverage.limits[0].span), "m = other");
}

#[test]
fn lua_rebinding_an_unrelated_name_does_not_limit() {
    let s = snippet("unrelated.lua", "local m = require(\"m\")\nlocal count = 0\ncount = count + 1\nreturn m\n");
    let report = s.report();
    assert_complete(&report);
    assert_eq!(report.entries.len(), 1);
}

/// A function whose own `local m` is written: a separate binding from the
/// chunk-level `m`.
const UNRELATED_LOCAL_WRITE: &str = "local function unrelated() local m; m = {} end\n";

/// `sample` with [`UNRELATED_LOCAL_WRITE`] inserted after its first line.
fn with_unrelated_local_write(sample: &str) -> String {
    let (first, rest) = sample.split_once('\n').expect("multi-line sample");
    format!("{first}\n{UNRELATED_LOCAL_WRITE}{rest}")
}

/// The report is complete, and the planner path reports the same.
fn assert_complete_on_both_paths(s: &Snippet) -> ReexportReport {
    let report = s.report();
    assert_complete(&report);
    assert_eq!(s.indexed().reexports.as_ref(), Some(&report), "query path agrees");
    report
}

#[test]
fn lua_write_to_a_function_local_keeps_the_returned_module() {
    let source = with_unrelated_local_write(include_str!("../fixtures/reexports/lua/pos-local-return.lua"));
    let s = snippet("pos-local-return.lua", &source);
    let report = assert_complete_on_both_paths(&s);
    assert_eq!(report.entries.len(), 1);
    assert_entry(&s, &report.entries[0], &lua_module("m", Some("m"), "m", "return m"));
}

#[test]
fn lua_write_to_a_function_local_keeps_the_returned_constructor() {
    let source = with_unrelated_local_write(include_str!("../fixtures/reexports/lua/pos-table-ctor.lua"));
    let s = snippet("pos-table-ctor.lua", &source);
    let report = assert_complete_on_both_paths(&s);
    assert_eq!(report.entries.len(), 1);
    assert_entry(&s, &report.entries[0], &lua_named("x", "x", Some("m"), "m", "x = m.x", "return { x = m.x }"));
}

#[test]
fn lua_write_to_a_function_local_keeps_the_assembled_table() {
    let source = with_unrelated_local_write(include_str!("../fixtures/reexports/lua/pos-table-field-local.lua"));
    let s = snippet("pos-table-field-local.lua", &source);
    let report = assert_complete_on_both_paths(&s);
    assert_eq!(report.entries.len(), 1);
    let line = "M.foo = m.foo";
    assert_entry(&s, &report.entries[0], &lua_named("foo", "foo", Some("m"), "m", line, line));
}

#[test]
fn lua_writes_to_nearer_bindings_of_the_same_name_do_not_limit() {
    for (name, write) in [
        ("parameter", "local function f(m) m = {} end\n"),
        ("numeric-for", "for m = 1, 2 do m = 3 end\n"),
        ("generic-for", "for _, m in ipairs({}) do m = 1 end\n"),
        ("do-block", "do local m; m = {} end\n"),
        ("nested-local-function", "local function f() local function m() end; m = 1 end\n"),
        ("repeat-until", "repeat local m = 1; m = 2 until m\n"),
        ("anonymous-function", "local f = function(...) local m = 1; m = 2 end\n"),
    ] {
        let s = snippet("nearer.lua", &format!("local m = require(\"m\")\n{write}return m\n"));
        let report = s.report();
        assert_complete(&report);
        assert_eq!(report.entries.len(), 1, "{name}");
        assert_eq!(report.entries[0].source.as_deref(), Some("m"), "{name}");
    }
}

#[test]
fn lua_function_body_write_to_the_chunk_local_still_limits() {
    for (name, write, limit_text) in [
        ("assignment", "local function f() m = {} end\n", "m = {}"),
        ("before-a-later-local", "local function f() m = {}; local m end\n", "m = {}"),
        ("function-statement", "function m() end\n", "function m() end"),
    ] {
        let s = snippet("rebound.lua", &format!("local m = require(\"m\")\n{write}return m\n"));
        let report = s.report();
        assert!(report.entries.is_empty(), "{name}: {:?}", report.entries);
        assert_limited(&report, &[ReexportLimitReason::ReboundBinding]);
        assert_eq!(s.text(&report.coverage.limits[0].span), limit_text, "{name}");
    }
}

#[test]
fn lua_later_chunk_declaration_hides_the_required_local() {
    for (name, declaration) in [("local-function", "local function m() end\n"), ("multi-name", "local m, n\n")] {
        let s = snippet("hidden.lua", &format!("local m = require(\"m\")\n{declaration}return m\n"));
        let report = s.report();
        assert!(report.entries.is_empty(), "{name}: {:?}", report.entries);
        assert_complete(&report);
    }
}

#[test]
fn lua_shadowed_name_at_a_dynamic_use_site_is_not_the_import() {
    for (name, block) in [
        ("metatable", "do local m = {}; setmetatable(U, { __index = m }) end\n"),
        ("copy-loop", "do local m = {}; for k, v in pairs(m) do U[k] = v end end\n"),
        ("loop-variable", "for _, m in ipairs({}) do setmetatable(U, { __index = m }) end\n"),
        ("nested-require", "do local require = function() return {} end; setmetatable(U, { __index = require(\"m\") }) end\n"),
    ] {
        // `U` is not returned, so only the dynamic-forwarding check sees it.
        let s = snippet("dynamic.lua", &format!("local m = require(\"m\")\nlocal U = {{}}\n{block}return {{}}\n"));
        let report = s.report();
        assert!(report.entries.is_empty(), "{name}: {:?}", report.entries);
        assert_complete(&report);
    }

    let unshadowed = snippet(
        "dynamic.lua",
        "local m = require(\"m\")\nlocal M = {}\ndo setmetatable(M, { __index = m }) end\nreturn M\n",
    );
    assert_limited(&unshadowed.report(), &[ReexportLimitReason::DynamicForwarding]);
}

#[test]
fn lua_require_shadowed_only_inside_a_function_stays_the_loader() {
    let nested = snippet(
        "nested.lua",
        "local function f() local require = print; require = print end\nreturn require(\"m\")\n",
    );
    let report = nested.report();
    assert_complete(&report);
    assert_eq!(report.entries.len(), 1);
    assert_entry(&nested, &report.entries[0], &lua_module("m", None, "require(\"m\")", "return require(\"m\")"));

    let global = snippet("global.lua", "local function f() require = print end\nreturn require(\"m\")\n");
    let report = global.report();
    assert!(report.entries.is_empty());
    assert_limited(&report, &[ReexportLimitReason::ShadowedGlobal]);
    assert_eq!(global.text(&report.coverage.limits[0].span), "require = print");
}

#[test]
fn lua_shadowed_require_limits_and_derives_nothing() {
    let s = snippet("neg-shadowed-require.lua", include_str!("../fixtures/reexports/lua/neg-shadowed-require.lua"));
    let report = s.report();
    assert!(report.entries.is_empty());
    assert_limited(&report, &[ReexportLimitReason::ShadowedGlobal]);
    assert!(s.text(&report.coverage.limits[0].span).starts_with("local require"));

    let global = snippet("global.lua", "require = function() end\nreturn require(\"m\")\n");
    let report = global.report();
    assert!(report.entries.is_empty());
    assert_limited(&report, &[ReexportLimitReason::ShadowedGlobal]);

    let unused = snippet("unused.lua", "local require = function() end\nreturn {}\n");
    assert_negative(&unused);
}

#[test]
fn lua_legacy_module_limits_with_dynamic_forwarding() {
    for (name, source) in [
        ("legacy-module.lua", include_str!("../fixtures/reexports/lua/legacy-module.lua")),
        ("legacy-module-name.lua", include_str!("../fixtures/reexports/lua/legacy-module-name.lua")),
    ] {
        let s = snippet(name, source);
        let report = s.report();
        assert!(report.entries.is_empty(), "{name}");
        assert_limited(&report, &[ReexportLimitReason::DynamicForwarding]);
        assert!(s.text(&report.coverage.limits[0].span).starts_with("module("), "{name}");
    }
}

#[test]
fn lua_dynamic_forwarding_samples_are_limited() {
    let copy = snippet("amb-copy-loop.lua", include_str!("../fixtures/reexports/lua/amb-copy-loop.lua"));
    let report = copy.report();
    assert!(report.entries.is_empty());
    // The loop forwards dynamically, and its `M[k]` write may replace any field.
    assert_limited(&report, &[ReexportLimitReason::DynamicForwarding, ReexportLimitReason::ReboundBinding]);
    assert!(copy.text(&report.coverage.limits[0].span).starts_with("for k, v in pairs(m)"));
    assert_eq!(copy.text(&report.coverage.limits[1].span), "M[k] = v");

    let meta = snippet("amb-setmetatable-index.lua", include_str!("../fixtures/reexports/lua/amb-setmetatable-index.lua"));
    let report = meta.report();
    assert!(report.entries.is_empty());
    assert_limited(&report, &[ReexportLimitReason::DynamicForwarding]);
    assert!(meta.text(&report.coverage.limits[0].span).starts_with("setmetatable("));

    let pcall = snippet("amb-pcall-require.lua", include_str!("../fixtures/reexports/lua/amb-pcall-require.lua"));
    let report = pcall.report();
    assert!(report.entries.is_empty());
    assert_limited(&report, &[ReexportLimitReason::DynamicForwarding]);
    assert_eq!(pcall.text(&report.coverage.limits[0].span), "pcall(require, \"m\")");

    let extra = snippet("amb-extra-return-values.lua", include_str!("../fixtures/reexports/lua/amb-extra-return-values.lua"));
    let report = extra.report();
    assert!(report.entries.is_empty());
    assert_limited(&report, &[ReexportLimitReason::DynamicForwarding]);
}

#[test]
fn lua_ordinary_metatables_and_unreturned_pcall_do_not_limit() {
    let class = snippet(
        "class.lua",
        "local Class = {}\nClass.__index = Class\nreturn setmetatable({}, { __index = Class })\n",
    );
    assert_negative(&class);

    let optional = snippet(
        "optional.lua",
        "local ok, extra = pcall(require, \"extra\")\nlocal M = {}\nreturn M\n",
    );
    assert_negative(&optional);
}

#[test]
fn lua_unrelated_parse_error_keeps_entries_and_stays_complete() {
    let s = snippet(
        "unrelated.lua",
        "local m = require(\"m\")\nfunction broken( ) local = end\nreturn m\n",
    );
    let report = s.report();
    assert_eq!(report.entries.len(), 1);
    assert_complete(&report);
}

#[test]
fn lua_damaged_return_limits_coverage() {
    let s = snippet("damaged.lua", "local m = require(\"m\")\nreturn { x = m.x, = }\n");
    let report = s.report();
    assert_limited(&report, &[ReexportLimitReason::ParseError]);
}

// ------------------------------------------------------------- corpus

#[test]
fn go_corpus_runs_every_sample_without_breaking_report_invariants() {
    let samples = [
        ("amb-dot-import-alias.go", include_str!("../fixtures/reexports/go/amb-dot-import-alias.go")),
        ("neg-alias-builtin.go", include_str!("../fixtures/reexports/go/neg-alias-builtin.go")),
        ("neg-alias-in-func.go", include_str!("../fixtures/reexports/go/neg-alias-in-func.go")),
        ("neg-alias-local.go", include_str!("../fixtures/reexports/go/neg-alias-local.go")),
        ("neg-alias-shadowed.go", include_str!("../fixtures/reexports/go/neg-alias-shadowed.go")),
        ("neg-alias-unexported.go", include_str!("../fixtures/reexports/go/neg-alias-unexported.go")),
        ("neg-defined-type-grouped.go", include_str!("../fixtures/reexports/go/neg-defined-type-grouped.go")),
        ("neg-defined-type.go", include_str!("../fixtures/reexports/go/neg-defined-type.go")),
        ("neg-embedded-struct.go", include_str!("../fixtures/reexports/go/neg-embedded-struct.go")),
        ("neg-func-wrapper.go", include_str!("../fixtures/reexports/go/neg-func-wrapper.go")),
        ("neg-iface-embed.go", include_str!("../fixtures/reexports/go/neg-iface-embed.go")),
        ("neg-import-aliased.go", include_str!("../fixtures/reexports/go/neg-import-aliased.go")),
        ("neg-import-blank.go", include_str!("../fixtures/reexports/go/neg-import-blank.go")),
        ("neg-import-dot.go", include_str!("../fixtures/reexports/go/neg-import-dot.go")),
        ("neg-import-grouped.go", include_str!("../fixtures/reexports/go/neg-import-grouped.go")),
        ("neg-method-expr.go", include_str!("../fixtures/reexports/go/neg-method-expr.go")),
        ("neg-var-call.go", include_str!("../fixtures/reexports/go/neg-var-call.go")),
        ("neg-var-mutable-nonconst.go", include_str!("../fixtures/reexports/go/neg-var-mutable-nonconst.go")),
        ("neg-var-mutated.go", include_str!("../fixtures/reexports/go/neg-var-mutated.go")),
        ("pos-alias-aliased-import.go", include_str!("../fixtures/reexports/go/pos-alias-aliased-import.go")),
        ("pos-alias-build-tag.go", include_str!("../fixtures/reexports/go/pos-alias-build-tag.go")),
        ("pos-alias-generic.go", include_str!("../fixtures/reexports/go/pos-alias-generic.go")),
        ("pos-alias-grouped.go", include_str!("../fixtures/reexports/go/pos-alias-grouped.go")),
        ("pos-alias-nested-pkgpath.go", include_str!("../fixtures/reexports/go/pos-alias-nested-pkgpath.go")),
        ("pos-alias-pointer.go", include_str!("../fixtures/reexports/go/pos-alias-pointer.go")),
        ("pos-alias-renamed.go", include_str!("../fixtures/reexports/go/pos-alias-renamed.go")),
        ("pos-alias.go", include_str!("../fixtures/reexports/go/pos-alias.go")),
        ("pos-const.go", include_str!("../fixtures/reexports/go/pos-const.go")),
        ("pos-func-var.go", include_str!("../fixtures/reexports/go/pos-func-var.go")),
        ("pos-import-plain.go", include_str!("../fixtures/reexports/go/pos-import-plain.go")),
        ("pos-var-after-localshadow.go", include_str!("../fixtures/reexports/go/pos-var-after-localshadow.go")),
        ("pos-var-typed.go", include_str!("../fixtures/reexports/go/pos-var-typed.go")),
        ("pos-var.go", include_str!("../fixtures/reexports/go/pos-var.go")),
    ];
    for (name, source) in samples {
        let s = snippet(name, source);
        let report = s.report();
        assert_report_invariants(name, &report);
        let index = s.indexed();
        let marked = index.imports.iter().filter(|i| i.reexported).count();
        let linked = report.entries.iter().filter(|e| e.import_span.is_some()).count();
        assert!(marked <= linked, "{name}: a marked import has an entry");
        for entry in &report.entries {
            assert!(entry.import_span.as_ref().is_none_or(|span| index.imports.iter().any(|i| &i.span == span)), "{name}: import_span names a record");
        }
    }
}

#[test]
fn lua_corpus_runs_every_sample_without_breaking_report_invariants() {
    let samples = [
        ("amb-copy-loop.lua", include_str!("../fixtures/reexports/lua/amb-copy-loop.lua")),
        ("amb-extra-return-values.lua", include_str!("../fixtures/reexports/lua/amb-extra-return-values.lua")),
        ("amb-pcall-require.lua", include_str!("../fixtures/reexports/lua/amb-pcall-require.lua")),
        ("amb-setmetatable-index.lua", include_str!("../fixtures/reexports/lua/amb-setmetatable-index.lua")),
        ("legacy-module-name.lua", include_str!("../fixtures/reexports/lua/legacy-module-name.lua")),
        ("legacy-module.lua", include_str!("../fixtures/reexports/lua/legacy-module.lua")),
        ("neg-call-result.lua", include_str!("../fixtures/reexports/lua/neg-call-result.lua")),
        ("neg-computed-key.lua", include_str!("../fixtures/reexports/lua/neg-computed-key.lua")),
        ("neg-field-in-function.lua", include_str!("../fixtures/reexports/lua/neg-field-in-function.lua")),
        ("neg-global-assign.lua", include_str!("../fixtures/reexports/lua/neg-global-assign.lua")),
        ("neg-method-wrap.lua", include_str!("../fixtures/reexports/lua/neg-method-wrap.lua")),
        ("neg-no-return.lua", include_str!("../fixtures/reexports/lua/neg-no-return.lua")),
        ("neg-rebound-table.lua", include_str!("../fixtures/reexports/lua/neg-rebound-table.lua")),
        ("neg-rebound-module.lua", include_str!("../fixtures/reexports/lua/neg-rebound-module.lua")),
        ("neg-require-concat.lua", include_str!("../fixtures/reexports/lua/neg-require-concat.lua")),
        ("neg-require-dynamic.lua", include_str!("../fixtures/reexports/lua/neg-require-dynamic.lua")),
        ("neg-require-unreturned.lua", include_str!("../fixtures/reexports/lua/neg-require-unreturned.lua")),
        ("neg-return-conditional.lua", include_str!("../fixtures/reexports/lua/neg-return-conditional.lua")),
        ("neg-return-in-function.lua", include_str!("../fixtures/reexports/lua/neg-return-in-function.lua")),
        ("neg-return-ternary.lua", include_str!("../fixtures/reexports/lua/neg-return-ternary.lua")),
        ("neg-shadowed-require.lua", include_str!("../fixtures/reexports/lua/neg-shadowed-require.lua")),
        ("pos-local-func-field.lua", include_str!("../fixtures/reexports/lua/pos-local-func-field.lua")),
        ("pos-local-return.lua", include_str!("../fixtures/reexports/lua/pos-local-return.lua")),
        ("pos-multi-return.lua", include_str!("../fixtures/reexports/lua/pos-multi-return.lua")),
        ("pos-require-string-call-chain.lua", include_str!("../fixtures/reexports/lua/pos-require-string-call-chain.lua")),
        ("pos-return-field.lua", include_str!("../fixtures/reexports/lua/pos-return-field.lua")),
        ("pos-return-local-field.lua", include_str!("../fixtures/reexports/lua/pos-return-local-field.lua")),
        ("pos-return-module-after-assign.lua", include_str!("../fixtures/reexports/lua/pos-return-module-after-assign.lua")),
        ("pos-return-parenthesized.lua", include_str!("../fixtures/reexports/lua/pos-return-parenthesized.lua")),
        ("pos-return-require-dotted.lua", include_str!("../fixtures/reexports/lua/pos-return-require-dotted.lua")),
        ("pos-return-require-longstr.lua", include_str!("../fixtures/reexports/lua/pos-return-require-longstr.lua")),
        ("pos-return-require-nop.lua", include_str!("../fixtures/reexports/lua/pos-return-require-nop.lua")),
        ("pos-return-require.lua", include_str!("../fixtures/reexports/lua/pos-return-require.lua")),
        ("pos-table-ctor-localvar.lua", include_str!("../fixtures/reexports/lua/pos-table-ctor-localvar.lua")),
        ("pos-table-ctor-module.lua", include_str!("../fixtures/reexports/lua/pos-table-ctor-module.lua")),
        ("pos-table-ctor-positional.lua", include_str!("../fixtures/reexports/lua/pos-table-ctor-positional.lua")),
        ("pos-table-ctor.lua", include_str!("../fixtures/reexports/lua/pos-table-ctor.lua")),
        ("pos-table-field-alias-index.lua", include_str!("../fixtures/reexports/lua/pos-table-field-alias-index.lua")),
        ("pos-table-field-local.lua", include_str!("../fixtures/reexports/lua/pos-table-field-local.lua")),
        ("pos-table-field-require.lua", include_str!("../fixtures/reexports/lua/pos-table-field-require.lua")),
    ];
    for (name, source) in samples {
        let s = snippet(name, source);
        let report = s.report();
        assert_report_invariants(name, &report);
        assert!(report.entries.iter().all(|e| e.import_span.is_none()), "{name}: Lua has no import records");
        if name.starts_with("amb-") || name.starts_with("legacy-") {
            assert_eq!(report.coverage.state, ReexportCoverageState::Limited, "{name}");
        }
    }
}

// ------------------------------------------- Lua: uses of the returned table

/// `pos-table-field-local.lua` with `inserted` placed before its final
/// `return M`.
fn field_local_with(inserted: &str) -> Snippet {
    let sample = include_str!("../fixtures/reexports/lua/pos-table-field-local.lua");
    let (body, last) = sample.trim_end().rsplit_once('\n').expect("multi-line sample");
    assert_eq!(last, "return M");
    snippet("pos-table-field-local.lua", &format!("{body}\n{inserted}\n{last}\n"))
}

/// The report is limited by exactly `reason` at `site`, and the planner path
/// reports the same.
fn assert_limited_on_both_paths(s: &Snippet, reason: ReexportLimitReason, site: &str) -> ReexportReport {
    let report = s.report();
    assert_limited(&report, &[reason]);
    assert_eq!(s.text(&report.coverage.limits[0].span), site);
    assert_eq!(s.indexed().reexports.as_ref(), Some(&report), "query path agrees");
    report
}

#[test]
fn lua_conditional_field_write_withholds_the_field() {
    for inserted in [
        "if enabled then M.foo = 1 end",
        "do M.foo = 1 end",
        "while enabled do M.foo = 1 end",
        "for i = 1, 2 do M.foo = 1 end",
        "repeat M.foo = 1 until enabled",
        "if enabled then M[\"foo\"] = 1 end",
    ] {
        let s = field_local_with(inserted);
        let site = inserted.split_once(" M").map(|(_, rest)| format!("M{rest}")).expect("write");
        let site = site.split(" end").next().and_then(|t| t.split(" until").next()).expect("site");
        let report = assert_limited_on_both_paths(&s, ReexportLimitReason::ReboundBinding, site);
        assert!(report.entries.is_empty(), "{inserted}: {:?}", report.entries);
    }
}

#[test]
fn lua_conditional_write_keeps_the_fields_it_does_not_name() {
    let s = field_local_with("M.bar = m.bar\nif enabled then M.foo = 1 end");
    let report = assert_limited_on_both_paths(&s, ReexportLimitReason::ReboundBinding, "M.foo = 1");
    assert_eq!(report.entries.len(), 1);
    let line = "M.bar = m.bar";
    assert_entry(&s, &report.entries[0], &lua_named("bar", "bar", Some("m"), "m", line, line));
}

#[test]
fn lua_field_write_in_a_function_body_withholds_the_field() {
    for (inserted, site) in [
        ("local function replace() M.foo = 1 end\nreplace()", "M.foo = 1"),
        ("local function replace() M.foo = 1 end", "M.foo = 1"),
        ("local replace = function() M.foo = 1 end", "M.foo = 1"),
        ("function M.reset() M.foo = 1 end", "M.foo = 1"),
        ("do function M.foo() end end", "function M.foo() end"),
    ] {
        let s = field_local_with(inserted);
        let report = assert_limited_on_both_paths(&s, ReexportLimitReason::ReboundBinding, site);
        assert!(report.entries.is_empty(), "{inserted}: {:?}", report.entries);
    }
}

#[test]
fn lua_returned_table_passed_to_a_call_is_limited() {
    for (inserted, site) in [
        ("configure(M)", "configure(M)"),
        ("f(x, M)", "f(x, M)"),
        ("obj:f(M)", "obj:f(M)"),
        ("M:init()", "M:init()"),
        ("rawset(M, \"x\", 1)", "rawset(M, \"x\", 1)"),
        ("table.insert(M, 1)", "table.insert(M, 1)"),
        ("setmetatable(M, {})", "setmetatable(M, {})"),
        ("configure((M))", "configure((M))"),
        ("if enabled then configure(M) end", "configure(M)"),
    ] {
        let s = field_local_with(inserted);
        let report = assert_limited_on_both_paths(&s, ReexportLimitReason::DynamicForwarding, site);
        assert!(report.entries.is_empty(), "{inserted}: {:?}", report.entries);
    }
}

#[test]
fn lua_constructor_initialized_table_passed_to_a_call_is_limited() {
    let control = snippet("ctor.lua", "local m = require(\"m\")\nlocal M = {read = m.read}\nreturn M\n");
    let report = assert_complete_on_both_paths(&control);
    assert_eq!(report.entries.len(), 1);
    assert_entry(&control, &report.entries[0], &lua_named("read", "read", Some("m"), "m", "read = m.read", "local M = {read = m.read}"));

    let escaped = snippet("ctor.lua", "local m = require(\"m\")\nlocal M = {read = m.read}\nconfigure(M)\nreturn M\n");
    let report = assert_limited_on_both_paths(&escaped, ReexportLimitReason::DynamicForwarding, "configure(M)");
    assert!(report.entries.is_empty(), "{:?}", report.entries);
}

#[test]
fn lua_unknown_key_write_withholds_every_field() {
    for (inserted, site) in [("M[key] = 1", "M[key] = 1"), ("if enabled then M[key] = 1 end", "M[key] = 1")] {
        let s = field_local_with(&format!("M.bar = m.bar\n{inserted}"));
        let report = assert_limited_on_both_paths(&s, ReexportLimitReason::ReboundBinding, site);
        assert!(report.entries.is_empty(), "{inserted}: {:?}", report.entries);
    }
    let sample = snippet("neg-computed-key.lua", include_str!("../fixtures/reexports/lua/neg-computed-key.lua"));
    assert_limited_on_both_paths(&sample, ReexportLimitReason::ReboundBinding, "M[key] = m.x");
    let in_function = snippet("neg-field-in-function.lua", include_str!("../fixtures/reexports/lua/neg-field-in-function.lua"));
    let report = assert_limited_on_both_paths(&in_function, ReexportLimitReason::ReboundBinding, "M.foo = require(\"m\").foo");
    assert!(report.entries.is_empty());
}

#[test]
fn lua_returned_table_stored_elsewhere_is_limited() {
    for (inserted, site) in [
        ("local alias = M", "M"),
        ("other.t = M", "M"),
        ("local list = {M}", "M"),
        ("local holder = {t = M}", "M"),
        ("local fallback = M or {}", "M or {}"),
        ("local function get() return M end", "M"),
        ("local keyed = {[M] = true}", "M"),
    ] {
        let s = field_local_with(inserted);
        let report = assert_limited_on_both_paths(&s, ReexportLimitReason::DynamicForwarding, site);
        assert!(report.entries.is_empty(), "{inserted}: {:?}", report.entries);
    }
}

#[test]
fn lua_reads_of_the_returned_table_keep_its_fields() {
    for inserted in [
        "print(M.foo)",
        "local x = M.foo",
        "if M.foo then end",
        "if M then end",
        "local n = #M",
        "local same = M == other",
        "M.foo.bar = 1",
        "local t = {M = 1}",
        "local function f(M) M.foo = 1 end",
        "do local M = {}; configure(M) end",
        "M.foo = m.foo",
    ] {
        let s = field_local_with(inserted);
        let report = assert_complete_on_both_paths(&s);
        assert_eq!(report.entries.len(), 1, "{inserted}");
        assert_eq!(report.entries[0].exported_name.as_deref(), Some("foo"), "{inserted}");
        assert_eq!(report.entries[0].source.as_deref(), Some("m"), "{inserted}");
    }
}

#[test]
fn lua_definite_chunk_level_overwrite_stays_complete() {
    for inserted in ["M.foo = 1", "M[\"foo\"] = 1"] {
        let s = field_local_with(inserted);
        let report = assert_complete_on_both_paths(&s);
        assert!(report.entries.is_empty(), "{inserted}: {:?}", report.entries);
    }
}

#[test]
fn lua_reassigned_table_and_import_metatable_keep_their_reasons() {
    let rebound = field_local_with("M = {}");
    let report = assert_limited_on_both_paths(&rebound, ReexportLimitReason::ReboundBinding, "M = {}");
    assert!(report.entries.is_empty());

    let meta = field_local_with("setmetatable(M, {__index = m})");
    let report = assert_limited_on_both_paths(&meta, ReexportLimitReason::DynamicForwarding, "setmetatable(M, {__index = m})");
    assert!(report.entries.is_empty());

    let direct = snippet("direct.lua", "local m = require(\"m\")\nreturn {read = m.read}\n");
    let report = assert_complete_on_both_paths(&direct);
    assert_entry(&direct, &report.entries[0], &lua_named("read", "read", Some("m"), "m", "read = m.read", "return {read = m.read}"));
}

// ------------------------------------------------------- Lua: table keys

/// `sample` with its one occurrence of `from` replaced by `to`.
fn edited(sample: &str, from: &str, to: &str) -> String {
    assert_eq!(sample.matches(from).count(), 1, "`{from}` occurs once in the sample");
    sample.replacen(from, to, 1)
}

#[test]
fn lua_string_literal_constructor_key_names_the_field() {
    let sample = include_str!("../fixtures/reexports/lua/pos-table-ctor.lua");
    for key in ["[\"x\"]", "['x']", "[ [[x]] ]", "[ [==[x]==] ]"] {
        let field = format!("{key} = m.x");
        let s = snippet("pos-table-ctor.lua", &edited(sample, "x = m.x", &field));
        let report = assert_complete_on_both_paths(&s);
        assert_eq!(report.entries.len(), 1, "{key}");
        assert_entry(&s, &report.entries[0], &lua_named("x", "x", Some("m"), "m", &field, &format!("return {{ {field} }}")));
    }
}

#[test]
fn lua_string_literal_key_in_a_local_table_constructor_names_the_field() {
    let s = snippet("ctor.lua", "local m = require(\"m\")\nlocal M = {[\"read\"] = m.read}\nreturn M\n");
    let report = assert_complete_on_both_paths(&s);
    assert_eq!(report.entries.len(), 1);
    assert_entry(&s, &report.entries[0], &lua_named("read", "read", Some("m"), "m", "[\"read\"] = m.read", "local M = {[\"read\"] = m.read}"));
}

#[test]
fn lua_computed_constructor_key_withholds_every_field() {
    let sample = include_str!("../fixtures/reexports/lua/pos-table-ctor.lua");
    let s = snippet("pos-table-ctor.lua", &edited(sample, "x = m.x", "[key] = m.x"));
    let report = assert_limited_on_both_paths(&s, ReexportLimitReason::ReboundBinding, "[key] = m.x");
    assert!(report.entries.is_empty(), "{:?}", report.entries);

    for (source, site) in [
        ("local m = require(\"m\")\nlocal M = {[key] = m.read}\nreturn M\n", "[key] = m.read"),
        ("local m = require(\"m\")\nreturn {read = m.read, [key] = 1}\n", "[key] = 1"),
        ("local m = require(\"m\")\nlocal M = {read = m.read, [k .. \"x\"] = 1}\nreturn M\n", "[k .. \"x\"] = 1"),
        ("local m = require(\"m\")\nlocal M = {[key] = 1}\nM.read = m.read\nreturn M\n", "[key] = 1"),
    ] {
        let s = snippet("computed.lua", source);
        let report = assert_limited_on_both_paths(&s, ReexportLimitReason::ReboundBinding, site);
        assert!(report.entries.is_empty(), "{source}: {:?}", report.entries);
    }
}

#[test]
fn lua_computed_key_in_an_unreturned_table_does_not_limit() {
    let s = snippet("unreturned.lua", "local m = require(\"m\")\nlocal lookup = {[key] = 1}\nreturn {read = m.read}\n");
    let report = assert_complete_on_both_paths(&s);
    assert_eq!(report.entries.len(), 1);
    assert_eq!(report.entries[0].exported_name.as_deref(), Some("read"));
}

#[test]
fn lua_equal_constructor_keys_share_one_field() {
    // Lua leaves constructor assignment order undefined for a repeated key, so
    // a repeat involving an import-derived value states no field at all.
    let sample = include_str!("../fixtures/reexports/lua/pos-table-ctor.lua");
    for (fields, site) in [
        ("x = m.x, [\"x\"] = 1", "[\"x\"] = 1"),
        ("[\"x\"] = m.x, x = 1", "x = 1"),
        ("x = m.x, ['x'] = 1", "['x'] = 1"),
        ("x = 1, [\"x\"] = m.x", "[\"x\"] = m.x"),
    ] {
        let s = snippet("pos-table-ctor.lua", &edited(sample, "x = m.x", fields));
        let report = assert_limited_on_both_paths(&s, ReexportLimitReason::ReboundBinding, site);
        assert!(report.entries.is_empty(), "{fields}: {:?}", report.entries);
    }

    // A repeat of a key whose values are all local leaves the other fields.
    let s = snippet("pos-table-ctor.lua", &edited(sample, "x = m.x", "y = 1, [\"y\"] = 2, x = m.x"));
    let report = assert_complete_on_both_paths(&s);
    assert_eq!(report.entries.len(), 1);
    assert_eq!(report.entries[0].exported_name.as_deref(), Some("x"));
    assert_eq!(report.entries[0].original_name.as_deref(), Some("x"));
}

#[test]
fn lua_number_and_boolean_keys_name_no_field() {
    for field in ["[1] = m.read", "[1.5] = m.read", "[true] = m.read"] {
        let s = snippet("keys.lua", &format!("local m = require(\"m\")\nreturn {{{field}}}\n"));
        let report = assert_complete_on_both_paths(&s);
        assert!(report.entries.is_empty(), "{field}: {:?}", report.entries);
    }

    // A number key cannot replace a string-named field, in a constructor or
    // an assignment, before or after it.
    for source in [
        "local m = require(\"m\")\nreturn {read = m.read, [1] = 1}\n",
        "local m = require(\"m\")\nlocal M = {[1] = 1, read = m.read}\nreturn M\n",
        "local m = require(\"m\")\nlocal M = {read = m.read}\nM[1] = 1\nreturn M\n",
        "local m = require(\"m\")\nlocal M = {read = m.read}\nif c then M[1] = 1 end\nreturn M\n",
    ] {
        let s = snippet("keys.lua", source);
        let report = assert_complete_on_both_paths(&s);
        assert_eq!(report.entries.len(), 1, "{source}");
        assert_eq!(report.entries[0].exported_name.as_deref(), Some("read"), "{source}");
        assert_eq!(report.entries[0].original_name.as_deref(), Some("read"), "{source}");
    }
}

#[test]
fn lua_string_keys_this_scan_does_not_decode_are_unknown() {
    for (field, site) in [
        ("[\"re\\x61d\"] = m.read", "[\"re\\x61d\"] = m.read"),
        ("[\"a\\\"b\"] = m.read", "[\"a\\\"b\"] = m.read"),
        ("[ [[\nread]] ] = m.read", "[ [[\nread]] ] = m.read"),
        ("[\"\"] = m.read", "[\"\"] = m.read"),
    ] {
        let s = snippet("escaped.lua", &format!("local m = require(\"m\")\nreturn {{{field}}}\n"));
        let report = assert_limited_on_both_paths(&s, ReexportLimitReason::ReboundBinding, site);
        assert!(report.entries.is_empty(), "{field}: {:?}", report.entries);
    }

    let assigned = field_local_with("M[\"f\\111o\"] = 1");
    let report = assert_limited_on_both_paths(&assigned, ReexportLimitReason::ReboundBinding, "M[\"f\\111o\"] = 1");
    assert!(report.entries.is_empty(), "{:?}", report.entries);
}

#[test]
fn lua_assignment_keys_share_one_field_however_spelled() {
    for (source, expect_forward) in [
        ("M.read = m.read\nM[\"read\"] = 1", false),
        ("M[\"read\"] = m.read\nM.read = 1", false),
        ("M['read'] = m.read\nfunction M:read() end", false),
        ("M.read = 1\nM[ [[read]] ] = m.read", true),
    ] {
        let s = snippet("assign.lua", &format!("local m = require(\"m\")\nlocal M = {{}}\n{source}\nreturn M\n"));
        let report = assert_complete_on_both_paths(&s);
        let names: Vec<_> = report.entries.iter().map(|e| e.exported_name.as_deref()).collect();
        let expected: Vec<Option<&str>> = if expect_forward { vec![Some("read")] } else { vec![] };
        assert_eq!(names, expected, "{source}");
    }
}

#[test]
fn lua_assignment_replaces_a_string_literal_constructor_field() {
    for later in ["M.read = 1", "function M.read() end", "function M:read() end"] {
        let s = snippet("ctor.lua", &format!("local m = require(\"m\")\nlocal M = {{[\"read\"] = m.read}}\n{later}\nreturn M\n"));
        let report = assert_complete_on_both_paths(&s);
        assert!(report.entries.is_empty(), "{later}: {:?}", report.entries);
    }
}

#[test]
fn lua_bracketed_member_read_uses_the_same_key_rules() {
    let s = snippet("member.lua", "local m = require(\"m\")\nreturn {a = m['x'], b = m[ [[y]] ], c = m[\"\\122\"], d = m[1]}\n");
    let report = assert_complete_on_both_paths(&s);
    let found: Vec<_> =
        report.entries.iter().map(|e| (e.exported_name.as_deref(), e.original_name.as_deref())).collect();
    assert_eq!(found, vec![(Some("a"), Some("x")), (Some("b"), Some("y"))]);
}

#[test]
fn lua_nested_constructor_keys_are_not_an_interface() {
    let s = snippet("nested.lua", "local m = require(\"m\")\nreturn {sub = {[\"read\"] = m.read, [key] = 1}}\n");
    assert_negative(&s);
}

#[test]
fn lua_metatable_index_key_is_recognized_however_spelled() {
    for (inserted, site) in [
        ("setmetatable(M, {[\"__index\"] = m})", "setmetatable(M, {[\"__index\"] = m})"),
        ("setmetatable(M, {[key] = m})", "setmetatable(M, {[key] = m})"),
    ] {
        let s = field_local_with(inserted);
        let report = assert_limited_on_both_paths(&s, ReexportLimitReason::DynamicForwarding, site);
        assert!(report.entries.is_empty(), "{inserted}: {:?}", report.entries);
    }
}

// ------------------------------------------- Lua: parenthesized spellings

/// The exported names of a report, in order.
fn exported_names(report: &ReexportReport) -> Vec<&str> {
    report.entries.iter().filter_map(|e| e.exported_name.as_deref()).collect()
}

/// A chunk-level write through `(M)` is a write to `M`: it replaces the field
/// as definitely as `M.foo = 1` does, so the report stays complete without
/// `foo` and keeps `bar`. (Lua needs the `;` before a line opening with `(`.)
#[test]
fn lua_parenthesized_chunk_level_write_replaces_the_field() {
    for target in ["(M).foo", "(M)[\"foo\"]", "((M)).foo", "M.foo"] {
        let assembled = field_local_with(&format!("M.bar = m.bar;\n{target} = 1;"));
        let report = assert_complete_on_both_paths(&assembled);
        assert_eq!(exported_names(&report), ["bar"], "assembled, {target}");

        let constructed = snippet(
            "ctor.lua",
            &format!("local m = require(\"m\")\nlocal M = {{foo = m.foo, bar = m.bar}};\n{target} = 1;\nreturn M\n"),
        );
        let report = assert_complete_on_both_paths(&constructed);
        assert_eq!(exported_names(&report), ["bar"], "constructor, {target}");
    }
}

#[test]
fn lua_parenthesized_chunk_level_write_of_an_import_forwards_the_field() {
    let s = snippet("paren.lua", "local m = require(\"m\")\nlocal M = {};\n(M).foo = (m).foo;\nreturn (M)\n");
    let report = assert_complete_on_both_paths(&s);
    assert_eq!(report.entries.len(), 1);
    let line = "(M).foo = (m).foo";
    assert_entry(&s, &report.entries[0], &lua_named("foo", "foo", Some("m"), "m", line, line));
}

#[test]
fn lua_parenthesized_nested_write_still_withholds_the_field() {
    for inserted in ["if enabled then (M).foo = 1 end", "local function reset() (M)[\"foo\"] = 1 end"] {
        let s = field_local_with(&format!("M.bar = m.bar;\n{inserted}"));
        let site = if inserted.contains('[') { "(M)[\"foo\"] = 1" } else { "(M).foo = 1" };
        let report = assert_limited_on_both_paths(&s, ReexportLimitReason::ReboundBinding, site);
        assert_eq!(exported_names(&report), ["bar"], "{inserted}");
    }
}

/// `M or t` and `t and M` may or may not evaluate to `M`, so a write through
/// one is not folded: the field it names is withheld.
#[test]
fn lua_write_through_an_and_or_expression_withholds_the_field() {
    for target in ["(M or {}).foo", "(other or M).foo", "(M and other)[\"foo\"]"] {
        let s = field_local_with(&format!("M.bar = m.bar;\n{target} = 1;"));
        let report = assert_limited_on_both_paths(&s, ReexportLimitReason::ReboundBinding, &format!("{target} = 1"));
        assert_eq!(exported_names(&report), ["bar"], "{target}");
    }
}

#[test]
fn lua_parenthesized_requires_and_constructors_forward() {
    for source in [
        "local m = (require)(\"m\")\nreturn {foo = m.foo}\n",
        "local m = require((\"m\"))\nreturn {foo = m.foo}\n",
        "local m = require(\"m\")\nlocal M = ({foo = m.foo})\nreturn M\n",
        "local m = require(\"m\")\nlocal M = {}\nM.foo = (m)[\"foo\"]\nreturn M\n",
    ] {
        let s = snippet("paren.lua", source);
        let report = assert_complete_on_both_paths(&s);
        assert_eq!(exported_names(&report), ["foo"], "{source}");
        assert_eq!(report.entries[0].source.as_deref(), Some("m"), "{source}");
    }
}

#[test]
fn lua_parenthesized_dynamic_forms_still_limit() {
    for (source, site) in [
        ("local m = require(\"m\")\nreturn setmetatable({}, ({__index = m}))\n", "setmetatable({}, ({__index = m}))"),
        ("local m = require(\"m\")\nreturn (setmetatable)({}, {__index = m})\n", "(setmetatable)({}, {__index = m})"),
        ("local ok, m = pcall((require), \"m\")\nreturn m\n", "pcall((require), \"m\")"),
        ("local ok, m = (pcall)(require, \"m\")\nreturn m\n", "(pcall)(require, \"m\")"),
        ("(module)(\"x\")\nreturn {}\n", "(module)(\"x\")"),
    ] {
        let s = snippet("paren.lua", source);
        let report = assert_limited_on_both_paths(&s, ReexportLimitReason::DynamicForwarding, site);
        assert!(report.entries.is_empty(), "{source}: {:?}", report.entries);
    }
}

/// Lua leaves the order of one statement's assignments undefined, so a field
/// written twice in it holds either value.
#[test]
fn lua_field_written_twice_in_one_statement_is_withheld() {
    for targets in ["M.foo, M.foo", "M.foo, (M)[\"foo\"]"] {
        let s = field_local_with(&format!("M.bar = m.bar\n{targets} = m.foo, 1"));
        let site = format!("{targets} = m.foo, 1");
        let report = assert_limited_on_both_paths(&s, ReexportLimitReason::ReboundBinding, &site);
        assert_eq!(exported_names(&report), ["bar"], "{targets}");
    }

    // A later definite write decides the field; plain values and a table
    // that is not returned leave nothing to withhold.
    let settled = field_local_with("M.foo, M.foo = m.foo, 1\nM.foo = m.foo");
    assert_eq!(exported_names(&assert_complete_on_both_paths(&settled)), ["foo"]);
    let plain = field_local_with("M.foo, M.foo = 1, 2");
    assert!(assert_complete_on_both_paths(&plain).entries.is_empty());
    let unreturned =
        snippet("other.lua", "local m = require(\"m\")\nlocal T = {}\nT.foo, T.foo = m.foo, 1\nreturn {bar = m.bar}\n");
    assert_eq!(exported_names(&assert_complete_on_both_paths(&unreturned)), ["bar"]);
}

/// The parenthesized spellings report the same through the persistent cache:
/// cold, hit, and narrowed from a complete-index entry.
#[test]
fn lua_parenthesized_spellings_survive_the_cache() {
    use tree_hugger::cache::{CacheLookup, PersistentCache, SymbolCache};
    use tree_hugger::{Capability, SourceFile, SymbolQuery};

    let reexports = SymbolQuery::new().capabilities([Capability::Reexports]);
    for (name, source) in [
        ("assembled.lua", "local m = require(\"m\")\nlocal M = {}\nM.foo = m.foo\nM.bar = m.bar;\n(M).foo = 1;\nreturn M\n"),
        ("ctor.lua", "local m = require(\"m\")\nlocal M = {foo = m.foo, bar = m.bar};\n(M)[\"foo\"] = 1;\nreturn M\n"),
        ("or.lua", "local m = require(\"m\")\nlocal M = {foo = m.foo, bar = m.bar};\n(M or {}).foo = 1;\nreturn M\n"),
    ] {
        let dir = tempfile::TempDir::new().expect("temp");
        let path = dir.path().join(name);
        std::fs::write(&path, source).expect("write");
        let uncached = SourceFile::read(&path).unwrap().query(&reexports).unwrap().index.reexports;
        assert_eq!(uncached.as_ref().map(exported_names), Some(vec!["bar"]), "{name}");

        let cache = SymbolCache::new(PersistentCache::open(dir.path().join("cache"), true));
        let cold = cache.query(SourceFile::read(&path).unwrap(), &reexports).unwrap();
        let hit = cache.query(SourceFile::read(&path).unwrap(), &reexports).unwrap();
        assert!(matches!(hit.lookup, CacheLookup::Hit), "{name}: {:?}", hit.lookup);
        let wide = cache.query(SourceFile::read(&path).unwrap(), &SymbolQuery::complete_index()).unwrap();
        let narrowed = cache.query(SourceFile::read(&path).unwrap(), &reexports).unwrap();
        for (label, outcome) in [("cold", &cold), ("hit", &hit), ("wide", &wide), ("narrowed", &narrowed)] {
            assert_eq!(outcome.result.index.reexports, uncached, "{name} {label}");
        }
    }
}
