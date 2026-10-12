//! Re-export extraction tests: rust.
//!
//! Every test goes through the public `TreeFile::reexports` / `query` paths.
//! An entry is compared whole, as one digest line, so a changed role fails
//! the test that names it.

use crate::reexports_support::{Snippet, snippet};
use tree_hugger::{ReexportCoverageState, ReexportEntry, ReexportLimitReason, ReexportReport};

fn dash(value: &Option<String>) -> &str {
    value.as_deref().unwrap_or("-")
}

/// Every field of `entry` on one line; text roles are the source they cover.
fn digest(snip: &Snippet, entry: &ReexportEntry) -> String {
    format!(
        "{:?}/{:?} exp={} orig={} local={} src={} scope={} res={} type_only={} cond={} caveats={:?} excl={:?} import={} span=`{}` stmt=`{}`",
        entry.form,
        entry.evidence,
        dash(&entry.exported_name),
        dash(&entry.original_name),
        dash(&entry.local_name),
        dash(&entry.source),
        dash(&entry.scope),
        dash(&entry.restriction),
        entry.type_only,
        entry.conditional,
        entry.caveats,
        entry.excluded_names,
        entry.import_span.as_ref().map_or("-", |span| snip.text(span)),
        snip.text(&entry.span),
        snip.text(&entry.statement_span),
    )
}

fn digests(snip: &Snippet) -> Vec<String> {
    snip.report().entries.iter().map(|entry| digest(snip, entry)).collect()
}

/// The digest of a `syntax` entry that has no local name, caveat, exclusion
/// or `type_only`.
#[allow(clippy::too_many_arguments)]
fn want(
    form: &str,
    names: (&str, &str),
    source: &str,
    scope: &str,
    restriction: &str,
    conditional: bool,
    import: &str,
    span: &str,
    statement: &str,
) -> String {
    format!(
        "{form}/Syntax exp={} orig={} local=- src={source} scope={scope} res={restriction} type_only=false cond={conditional} caveats=[] excl=[] import={import} span=`{span}` stmt=`{statement}`",
        names.0, names.1
    )
}

fn named(exported: &str, original: &str, source: &str, import: &str, span: &str, statement: &str) -> String {
    want("Named", (exported, original), source, "-", "-", false, import, span, statement)
}

fn wildcard(source: &str, import: &str, span: &str, statement: &str) -> String {
    want("Wildcard", ("-", "-"), source, "-", "-", false, import, span, statement)
}

fn limits(snip: &Snippet, report: &ReexportReport) -> Vec<String> {
    report
        .coverage
        .limits
        .iter()
        .map(|limit| format!("{:?}: {}", limit.reason, snip.text(&limit.span)))
        .collect()
}

fn assert_complete(report: &ReexportReport) {
    assert_eq!(report.coverage.state, ReexportCoverageState::Complete);
    assert!(report.coverage.limits.is_empty());
}

/// Names of the imports the planner marks as forwarded.
fn forwarded_imports(snip: &Snippet) -> Vec<String> {
    snip.indexed()
        .imports
        .iter()
        .filter(|import| import.reexported)
        .map(|import| snip.text(&import.span).to_owned())
        .collect()
}

#[test]
fn a_plain_pub_use_is_one_named_entry() {
    let source = include_str!("../fixtures/reexports/rust/pos-simple.rs");
    let snip = snippet("a.rs", source);
    let report = snip.report();
    assert_complete(&report);
    assert_eq!(digests(&snip), vec![named("B", "B", "a", "B", "a::B", "pub use a::B;")]);
    assert_eq!(forwarded_imports(&snip), ["B"]);
}

#[test]
fn an_alias_renames_the_exported_name() {
    let source = include_str!("../fixtures/reexports/rust/pos-alias.rs");
    let snip = snippet("a.rs", source);
    assert_complete(&snip.report());
    assert_eq!(digests(&snip), vec![named("C", "B", "a", "C", "a::B as C", "pub use a::B as C;")]);
    assert_eq!(forwarded_imports(&snip), ["C"]);
}

#[test]
fn an_anonymous_alias_is_written_as_an_underscore() {
    let snip = snippet("a.rs", "pub use a::B as _;\n");
    assert_eq!(digests(&snip), vec![named("_", "B", "a", "_", "a::B as _", "pub use a::B as _;")]);
}

#[test]
fn a_single_identifier_path_has_no_source() {
    let snip = snippet("a.rs", "pub use serde;\npub use serde as s;\n");
    assert_eq!(
        digests(&snip),
        vec![
            named("serde", "serde", "-", "serde", "serde", "pub use serde;"),
            named("s", "serde", "-", "s", "serde as s", "pub use serde as s;"),
        ]
    );
}

#[test]
fn a_group_yields_one_entry_per_leaf_sharing_the_statement() {
    let source = include_str!("../fixtures/reexports/rust/pos-grouped.rs");
    let snip = snippet("a.rs", source);
    let statement = "pub use a::{B, C as D, self, e::F};";
    assert_complete(&snip.report());
    assert_eq!(
        digests(&snip),
        vec![
            named("B", "B", "a", "B", "B", statement),
            named("D", "C", "a", "D", "C as D", statement),
            named("a", "a", "-", "a", "self", statement),
            named("F", "F", "a::e", "F", "e::F", statement),
        ]
    );
    // Import records are in source order: the group path `a` binds first.
    assert_eq!(forwarded_imports(&snip), ["a", "B", "D", "F"]);
}

#[test]
fn nested_groups_join_the_path_as_written() {
    let snip = snippet("a.rs", "pub use a::{b::{C, d::E as F}, g::{self, h}};\n");
    let statement = "pub use a::{b::{C, d::E as F}, g::{self, h}};";
    assert_eq!(
        digests(&snip),
        vec![
            named("C", "C", "a::b", "C", "C", statement),
            named("F", "E", "a::b::d", "F", "d::E as F", statement),
            named("g", "g", "a", "g", "self", statement),
            named("h", "h", "a::g", "h", "h", statement),
        ]
    );
}

#[test]
fn a_pathless_group_has_no_source() {
    let snip = snippet("a.rs", "pub use {a, b::c};\n");
    assert_eq!(
        digests(&snip),
        vec![
            named("a", "a", "-", "a", "a", "pub use {a, b::c};"),
            named("c", "c", "b", "c", "b::c", "pub use {a, b::c};"),
        ]
    );
}

#[test]
fn self_alone_and_renamed_self_forward_the_group_path() {
    let source = include_str!("../fixtures/reexports/rust/pos-self-only.rs");
    let snip = snippet("a.rs", source);
    assert_complete(&snip.report());
    assert_eq!(
        digests(&snip),
        vec![
            named("a", "a", "-", "a", "self", "pub use a::{self};"),
            named("z", "a", "-", "z", "self as z", "pub use a::{self as z};"),
        ]
    );
    let deeper = snippet("a.rs", "pub use a::b::{self};\n");
    assert_eq!(digests(&deeper), vec![named("b", "b", "a", "b", "self", "pub use a::b::{self};")]);
}

#[test]
fn globs_are_wildcards_with_the_written_source() {
    let source = include_str!("../fixtures/reexports/rust/pos-glob.rs");
    let snip = snippet("a.rs", source);
    assert_complete(&snip.report());
    assert_eq!(
        digests(&snip),
        vec![
            wildcard("a", "a", "a::*", "pub use a::*;"),
            wildcard("a::b", "b", "b::*", "pub use a::{b::*, C};"),
            named("C", "C", "a", "C", "C", "pub use a::{b::*, C};"),
        ]
    );
    assert_eq!(forwarded_imports(&snip), ["a", "b", "C"]);
}

#[test]
fn a_glob_inside_a_group_and_a_rooted_glob() {
    let snip = snippet("a.rs", "pub use a::{*};\npub use crate::*;\npub use *;\n");
    let report = snip.report();
    assert_complete(&report);
    assert_eq!(
        digests(&snip),
        vec![
            wildcard("a", "a", "*", "pub use a::{*};"),
            wildcard("crate", "crate", "crate::*", "pub use crate::*;"),
            wildcard("-", "-", "*", "pub use *;"),
        ]
    );
}

#[test]
fn leading_path_roots_stay_in_the_source() {
    let source = include_str!("../fixtures/reexports/rust/pos-use-leading.rs");
    let snip = snippet("a.rs", source);
    assert_eq!(
        digests(&snip),
        vec![
            named("B", "B", "crate::a", "B", "crate::a::B", "pub use crate::a::B;"),
            named("B2", "B2", "self::a", "B2", "self::a::B2", "pub use self::a::B2;"),
            named("C", "C", "super", "C", "super::C", "pub use super::C;"),
        ]
    );
}

#[test]
fn a_rooted_external_path_keeps_its_leading_colons() {
    let snip = snippet("a.rs", "pub use ::serde_json::Value;\npub use ::foo;\n");
    assert_eq!(
        digests(&snip),
        vec![
            named("Value", "Value", "::serde_json", "Value", "::serde_json::Value", "pub use ::serde_json::Value;"),
            named("foo", "foo", "-", "foo", "::foo", "pub use ::foo;"),
        ]
    );
}

#[test]
fn restricted_visibility_keeps_its_written_text() {
    let source = include_str!("../fixtures/reexports/rust/pos-restricted.rs");
    let snip = snippet("a.rs", source);
    assert_complete(&snip.report());
    let at = |restriction: &str, name: &str, statement: &str| {
        want("Named", (name, name), "a", "-", restriction, false, name, &format!("a::{name}"), statement)
    };
    assert_eq!(
        digests(&snip),
        vec![
            at("crate", "B", "pub(crate) use a::B;"),
            at("super", "C", "pub(super) use a::C;"),
            at("in crate::x", "E", "pub(in crate::x) use a::E;"),
        ]
    );
}

#[test]
fn pub_self_is_private_and_forwards_nothing() {
    let snip = snippet("a.rs", "pub(self) use a::D;\npub(self) extern crate x;\n");
    let report = snip.report();
    assert_complete(&report);
    assert!(report.entries.is_empty());
    assert!(forwarded_imports(&snip).is_empty());
}

#[test]
fn restricted_whitespace_is_normalized() {
    let snip = snippet("a.rs", "pub( in  crate :: x ) use a::E;\n");
    let report = snip.report();
    assert_eq!(report.entries[0].restriction.as_deref(), Some("in crate::x"));
}

#[test]
fn private_uses_and_pub_items_are_negatives() {
    let source = include_str!("../fixtures/reexports/rust/neg-private-use.rs");
    let snip = snippet("a.rs", source);
    let report = snip.report();
    assert_complete(&report);
    assert!(report.entries.is_empty());
    assert!(forwarded_imports(&snip).is_empty());

    let items = include_str!("../fixtures/reexports/rust/neg-pub-local.rs");
    let snip = snippet("a.rs", items);
    let report = snip.report();
    assert_complete(&report);
    assert!(report.entries.is_empty());
}

#[test]
fn pub_extern_crate_is_a_namespace_entry() {
    let source = include_str!("../fixtures/reexports/rust/pos-extern-crate.rs");
    let snip = snippet("a.rs", source);
    assert_complete(&snip.report());
    let entries = digests(&snip);
    assert_eq!(
        &entries[..2],
        [
            want("Namespace", ("serde", "-"), "serde", "-", "-", false, "serde", "serde", "pub extern crate serde;"),
            want("Namespace", ("bar", "-"), "foo", "-", "-", false, "bar", "foo as bar", "pub extern crate foo as bar;"),
        ]
    );
    // The `pub use` lines of the same sample are ordinary named entries.
    assert_eq!(entries.len(), 5);
    assert_eq!(forwarded_imports(&snip), ["serde", "bar", "Serialize", "Value", "HashMap"]);
}

#[test]
fn restricted_extern_crate_keeps_its_restriction_and_private_is_ignored() {
    let snip = snippet("a.rs", "extern crate x;\npub(crate) extern crate y;\n");
    assert_eq!(
        digests(&snip),
        vec![want("Namespace", ("y", "-"), "y", "-", "crate", false, "y", "y", "pub(crate) extern crate y;")]
    );
}

#[test]
fn cfg_and_cfg_attr_attributes_make_an_entry_conditional() {
    let source = include_str!("../fixtures/reexports/rust/pos-cfg.rs");
    let snip = snippet("a.rs", source);
    assert_complete(&snip.report());
    let conditional = |name: &str, statement: &str| {
        want("Named", (name, name), "a", "-", "-", true, name, &format!("a::{name}"), statement)
    };
    assert_eq!(
        digests(&snip),
        vec![conditional("B", "pub use a::B;"), conditional("C", "pub use a::C;")]
    );
}

#[test]
fn other_attributes_comments_between_and_unattached_cfg_are_not_conditions() {
    let snip = snippet(
        "a.rs",
        "#[allow(unused)]\npub use a::A;\n#[cfg(x)]\n// note\n#[doc = \"d\"]\npub use a::B;\n#[cfg(x)]\nfn f() {}\npub use a::C;\n",
    );
    let flags: Vec<(String, bool)> = snip
        .report()
        .entries
        .iter()
        .map(|entry| (entry.exported_name.clone().unwrap_or_default(), entry.conditional))
        .collect();
    assert_eq!(flags, [("A".into(), false), ("B".into(), true), ("C".into(), false)]);
}

#[test]
fn a_cfg_on_a_module_or_the_file_conditions_what_it_holds() {
    let snip = snippet("a.rs", "#[cfg(unix)]\nmod u {\n    pub use a::A;\n}\nmod w {\n    #![cfg(windows)]\n    pub use a::B;\n}\nmod p {\n    pub use a::C;\n}\n");
    let flags: Vec<(String, bool)> = snip
        .report()
        .entries
        .iter()
        .map(|entry| (entry.scope.clone().unwrap_or_default(), entry.conditional))
        .collect();
    assert_eq!(flags, [("u".into(), true), ("w".into(), true), ("p".into(), false)]);

    let whole = snippet("a.rs", "#![cfg(unix)]\npub use a::A;\n");
    assert!(whole.report().entries[0].conditional);
}

#[test]
fn nested_modules_qualify_the_scope() {
    let source = include_str!("../fixtures/reexports/rust/pos-nested-mod.rs");
    let snip = snippet("a.rs", source);
    assert_complete(&snip.report());
    assert_eq!(
        digests(&snip),
        vec![
            want("Named", ("B", "B"), "crate::a", "outer::inner", "-", false, "B", "crate::a::B", "pub use crate::a::B;"),
            want("Named", ("C", "C"), "super", "outer", "crate", false, "C", "super::C", "pub(crate) use super::C;"),
        ]
    );
}

#[test]
fn a_restricted_forward_in_a_module_matches_the_documented_example() {
    let snip = snippet("a.rs", "mod a { pub(crate) use std::io::Read as R; }\n");
    let report = snip.report();
    let found = &report.entries[0];
    assert_eq!(found.exported_name.as_deref(), Some("R"));
    assert_eq!(found.original_name.as_deref(), Some("Read"));
    assert_eq!(found.source.as_deref(), Some("std::io"));
    assert_eq!(found.scope.as_deref(), Some("a"));
    assert_eq!(found.restriction.as_deref(), Some("crate"));
    assert!(found.import_span.is_some());
    assert!(found.caveats.is_empty() && found.excluded_names.is_empty() && !found.type_only);
    assert_eq!(snip.text(&found.statement_span), "pub(crate) use std::io::Read as R;");
}

#[test]
fn uses_inside_function_bodies_and_blocks_are_not_reported() {
    for sample in [
        include_str!("../fixtures/reexports/rust/neg-in-fn.rs"),
        include_str!("../fixtures/reexports/rust/pos-in-fn-pub.rs"),
    ] {
        let snip = snippet("a.rs", sample);
        let report = snip.report();
        assert_complete(&report);
        assert!(report.entries.is_empty());
        assert!(forwarded_imports(&snip).is_empty());
    }
    let nested = snippet(
        "a.rs",
        "fn f() {\n    { pub use a::B; }\n    mod m { pub use a::C; }\n}\nimpl X { fn g() { pub use a::D; } }\npub use a::E;\n",
    );
    let names: Vec<_> = nested.report().entries.iter().map(|e| e.exported_name.clone().unwrap()).collect();
    assert_eq!(names, ["E"]);
}

#[test]
fn a_macro_rules_body_with_pub_then_use_limits_coverage() {
    let source = include_str!("../fixtures/reexports/rust/pos-macro.rs");
    let snip = snippet("a.rs", source);
    let report = snip.report();
    assert_eq!(report.coverage.state, ReexportCoverageState::Limited);
    assert_eq!(
        limits(&snip, &report),
        ["UnexpandedMacro: macro_rules! m { () => { pub use a::B; } }"]
    );
    assert!(report.entries.is_empty());
}

#[test]
fn an_item_position_macro_with_pub_then_use_limits_coverage() {
    let snip = snippet("a.rs", "cfg_if! { if #[cfg(x)] { pub use a::B; } }\nmod m { glue! { pub(crate) use b::C; } }\npub use d::E;\n");
    let report = snip.report();
    assert_eq!(
        limits(&snip, &report),
        [
            "UnexpandedMacro: cfg_if! { if #[cfg(x)] { pub use a::B; } }",
            "UnexpandedMacro: glue! { pub(crate) use b::C; }",
        ]
    );
    assert!(report.coverage.limits.iter().all(|l| l.reason == ReexportLimitReason::UnexpandedMacro));
    assert_eq!(report.entries.len(), 1);
}

#[test]
fn macros_without_a_pub_then_use_and_macros_in_bodies_do_not_limit() {
    let snip = snippet(
        "a.rs",
        "macro_rules! a { () => { use x::Y; pub struct S; } }\nmacro_rules! b { () => { use x::Y; pub fn f() {} } }\nm!();\nfn f() { glue! { pub use a::B; } macro_rules! c { () => { pub use a::C; } } }\n",
    );
    assert_complete(&snip.report());
}

#[test]
fn a_use_before_pub_in_a_macro_does_not_limit() {
    let snip = snippet("a.rs", "macro_rules! m { () => { use a::B; pub struct S; } }\n");
    assert_complete(&snip.report());
}

#[test]
fn an_unrelated_parse_error_keeps_entries_and_stays_complete() {
    let snip = snippet("a.rs", "pub struct S { x: ;\npub use a::B;\nfn f() { let = ; }\npub use c::D;\n");
    let report = snip.report();
    assert_complete(&report);
    assert_eq!(report.entries.len(), 2);
}

#[test]
fn damaged_forwarding_syntax_is_limited_and_never_a_complete_negative() {
    for broken in [
        "pub use a::{B, ;\npub use c::D;\n",
        "pub use a::B\npub use c::D;\n",
        "pub use ;\n",
        "pub use a::{B,, C};\n",
        "pub extern crate ;\n",
        "pub use\n",
        "#[cfg(x)]\npub use a::{B C};\n",
    ] {
        let snip = snippet("a.rs", broken);
        let report = snip.report();
        assert_eq!(report.coverage.state, ReexportCoverageState::Limited, "{broken:?}");
        assert!(
            report.coverage.limits.iter().all(|l| l.reason == ReexportLimitReason::ParseError),
            "{broken:?}"
        );
        assert!(
            report.entries.iter().all(|e| e.exported_name.as_deref() != Some("")),
            "an invented name was reported for {broken:?}"
        );
    }
}

#[test]
fn a_damaged_missing_name_reports_no_entry() {
    let snip = snippet("a.rs", "pub use ;\n");
    assert!(snip.report().entries.is_empty());
}

#[test]
fn entries_are_in_source_order_with_unique_spans() {
    let snip = snippet(
        "a.rs",
        "mod m { pub use z::Z; }\npub use a::{c, b};\npub use a::{c, b};\npub extern crate q;\npub use a::*;\n",
    );
    let report = snip.report();
    let starts: Vec<u32> = report.entries.iter().map(|e| e.span.start_byte).collect();
    assert!(starts.windows(2).all(|pair| pair[0] < pair[1]), "{starts:?}");
    // The identical statements are two sites, not one.
    assert_eq!(report.entries.len(), 7);
}

#[test]
fn only_forwarded_imports_carry_the_reexported_marker() {
    let snip = snippet("a.rs", "use a::P;\npub use b::{Q, R};\npub(self) use c::S;\nuse d::*;\n");
    assert_eq!(forwarded_imports(&snip), ["Q", "R"]);
    let index = snip.indexed();
    let reexports = index.reexports.expect("reexports computed");
    for entry in &reexports.entries {
        let import_span = entry.import_span.as_ref().expect("linked");
        assert!(index.imports.iter().any(|import| &import.span == import_span));
    }
}

#[test]
fn query_and_direct_reports_agree() {
    let snip = snippet("a.rs", "pub use a::{B, c::*};\n#[cfg(x)]\npub(crate) extern crate z;\n");
    assert_eq!(snip.indexed().reexports.expect("reexports"), snip.report());
}

#[test]
fn every_audit_sample_extracts_in_order_without_panicking() {
    let samples: [(&str, &str); 15] = [
        ("neg-in-fn", include_str!("../fixtures/reexports/rust/neg-in-fn.rs")),
        ("neg-private-use", include_str!("../fixtures/reexports/rust/neg-private-use.rs")),
        ("neg-pub-local", include_str!("../fixtures/reexports/rust/neg-pub-local.rs")),
        ("pos-alias", include_str!("../fixtures/reexports/rust/pos-alias.rs")),
        ("pos-cfg", include_str!("../fixtures/reexports/rust/pos-cfg.rs")),
        ("pos-extern-crate", include_str!("../fixtures/reexports/rust/pos-extern-crate.rs")),
        ("pos-glob", include_str!("../fixtures/reexports/rust/pos-glob.rs")),
        ("pos-grouped", include_str!("../fixtures/reexports/rust/pos-grouped.rs")),
        ("pos-in-fn-pub", include_str!("../fixtures/reexports/rust/pos-in-fn-pub.rs")),
        ("pos-macro", include_str!("../fixtures/reexports/rust/pos-macro.rs")),
        ("pos-nested-mod", include_str!("../fixtures/reexports/rust/pos-nested-mod.rs")),
        ("pos-restricted", include_str!("../fixtures/reexports/rust/pos-restricted.rs")),
        ("pos-self-only", include_str!("../fixtures/reexports/rust/pos-self-only.rs")),
        ("pos-simple", include_str!("../fixtures/reexports/rust/pos-simple.rs")),
        ("pos-use-leading", include_str!("../fixtures/reexports/rust/pos-use-leading.rs")),
    ];
    for (name, source) in samples {
        let snip = snippet("a.rs", source);
        let report = snip.report();
        let starts: Vec<u32> = report.entries.iter().map(|e| e.span.start_byte).collect();
        assert!(starts.windows(2).all(|pair| pair[0] < pair[1]), "{name}: {starts:?}");
        let keys: Vec<_> = report.coverage.limits.iter().map(|l| (l.span.start_byte, l.reason)).collect();
        assert!(keys.windows(2).all(|pair| pair[0] <= pair[1]), "{name}");
        assert_eq!(
            report.coverage.state == ReexportCoverageState::Limited,
            !report.coverage.limits.is_empty(),
            "{name}"
        );
        // Every sample parses cleanly, so only the macro sample is limited.
        assert_eq!(!report.coverage.limits.is_empty(), name == "pos-macro", "{name}");
    }
}
