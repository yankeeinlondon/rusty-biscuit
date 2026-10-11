//! Re-export extraction tests: JavaScript and TypeScript.
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
    let mut line = format!(
        "{:?}/{:?} exp={} orig={} local={} src={} scope={} ty={} cond={} import=`{}` span=`{}` stmt=`{}`",
        entry.form,
        entry.evidence,
        dash(&entry.exported_name),
        dash(&entry.original_name),
        dash(&entry.local_name),
        dash(&entry.source),
        dash(&entry.scope),
        entry.type_only,
        entry.conditional,
        entry.import_span.as_ref().map_or("-", |span| snip.text(span)),
        snip.text(&entry.span),
        snip.text(&entry.statement_span),
    );
    if entry.restriction.is_some() || !entry.caveats.is_empty() || !entry.excluded_names.is_empty() {
        line.push_str(&format!(
            " UNEXPECTED restriction={:?} caveats={:?} excluded={:?}",
            entry.restriction, entry.caveats, entry.excluded_names
        ));
    }
    line
}

fn digests(snip: &Snippet) -> Vec<String> {
    snip.report().entries.iter().map(|entry| digest(snip, entry)).collect()
}

/// A digest line for an entry that is not type-only, conditional or scoped.
#[allow(clippy::too_many_arguments)]
fn want(
    kind: &str,
    exported: &str,
    original: &str,
    local: &str,
    source: &str,
    import: &str,
    span: &str,
    statement: &str,
) -> String {
    format!(
        "{kind} exp={exported} orig={original} local={local} src={source} scope=- ty=false cond=false import=`{import}` span=`{span}` stmt=`{statement}`"
    )
}

/// `want` for an ES-module forward that goes through no local binding.
fn direct(kind: &str, exported: &str, original: &str, source: &str, span: &str, statement: &str) -> String {
    want(kind, exported, original, "-", source, "-", span, statement)
}

fn typed(line: String) -> String {
    line.replacen("ty=false", "ty=true", 1)
}

fn conditional(line: String) -> String {
    line.replacen("cond=false", "cond=true", 1)
}

fn scoped(line: String, scope: &str) -> String {
    line.replacen("scope=-", &format!("scope={scope}"), 1)
}

fn limits(snip: &Snippet, report: &ReexportReport) -> Vec<(ReexportLimitReason, String)> {
    report.coverage.limits.iter().map(|limit| (limit.reason, snip.text(&limit.span).to_owned())).collect()
}

fn assert_complete(report: &ReexportReport) {
    assert_eq!(report.coverage.state, ReexportCoverageState::Complete);
    assert!(report.coverage.limits.is_empty());
}

fn assert_limited(snip: &Snippet, expected: &[(ReexportLimitReason, &str)]) {
    let report = snip.report();
    assert_eq!(report.coverage.state, ReexportCoverageState::Limited);
    let expected: Vec<_> = expected.iter().map(|(reason, text)| (*reason, (*text).to_owned())).collect();
    assert_eq!(limits(snip, &report), expected);
}

fn marked_imports(snip: &Snippet) -> Vec<(String, bool)> {
    snip.indexed().imports.iter().map(|import| (snip.text(&import.span).to_owned(), import.reexported)).collect()
}

// ---------------------------------------------------------------- JavaScript

#[test]
fn js_named_reexport_is_one_syntax_entry() {
    let source = include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/js/pos-named.js");
    let snip = snippet("a.js", source);
    assert_eq!(
        digests(&snip),
        [direct("Named/Syntax", "a", "a", "m", "a", r#"export { a } from "m";"#)]
    );
    assert_complete(&snip.report());
}

#[test]
fn js_renamed_specifiers_each_get_an_entry_in_order() {
    let source = include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/js/pos-renamed.js");
    let snip = snippet("a.js", source);
    let statement = r#"export { a as b, c, d as e } from "m";"#;
    assert_eq!(
        digests(&snip),
        [
            direct("Named/Syntax", "b", "a", "m", "a as b", statement),
            direct("Named/Syntax", "c", "c", "m", "c", statement),
            direct("Named/Syntax", "e", "d", "m", "d as e", statement),
        ]
    );
}

#[test]
fn js_default_is_read_from_the_anonymous_token_in_either_position() {
    let as_default = include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/js/pos-default-as-default.js");
    let bare = include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/js/pos-default-bare.js");
    let as_name = include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/js/pos-default-as-name.js");
    assert_eq!(
        digests(&snippet("a.js", as_default)),
        [direct("Named/Syntax", "default", "x", "m", "x as default", r#"export { x as default } from "m";"#)]
    );
    assert_eq!(
        digests(&snippet("a.js", bare)),
        [direct("Named/Syntax", "default", "default", "m", "default", r#"export { default } from "m";"#)]
    );
    assert_eq!(
        digests(&snippet("a.js", as_name)),
        [direct("Named/Syntax", "x", "default", "m", "default as x", r#"export { default as x } from "m";"#)]
    );
}

#[test]
fn js_string_literal_names_are_kept_without_their_quotes() {
    let source = include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/js/pos-string-name.js");
    let snip = snippet("a.js", source);
    assert_eq!(
        digests(&snip),
        [direct("Named/Syntax", "c d", "a b", "m", r#""a b" as "c d""#, r#"export { "a b" as "c d" } from "m";"#)]
    );
}

#[test]
fn js_wildcard_and_namespace_stay_distinct() {
    let wildcard = include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/js/pos-wildcard.js");
    let namespace = include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/js/pos-namespace.js");
    let default = include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/js/pos-namespace-default.js");
    assert_eq!(
        digests(&snippet("a.js", wildcard)),
        [direct("Wildcard/Syntax", "-", "-", "m", "*", r#"export * from "m";"#)]
    );
    assert_eq!(
        digests(&snippet("a.js", namespace)),
        [direct("Namespace/Syntax", "ns", "-", "m", "* as ns", r#"export * as ns from "m";"#)]
    );
    assert_eq!(
        digests(&snippet("a.js", default)),
        [direct("Namespace/Syntax", "default", "-", "m", "* as default", r#"export * as default from "m";"#)]
    );
}

#[test]
fn js_import_then_export_keeps_the_local_binding_and_marks_the_import() {
    let source = include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/js/pos-import-export.js");
    let snip = snippet("a.js", source);
    assert_eq!(
        digests(&snip),
        [want("Named/Syntax", "b", "a", "local", "m", "local", "local as b", "export { local as b };")]
    );
    assert_eq!(marked_imports(&snip), [("local".to_owned(), true)]);
    assert_eq!(snip.indexed().reexports, Some(snip.report()));
}

#[test]
fn js_default_and_namespace_imports_forward_through_export_clause_and_default() {
    let source = include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/js/pos-import-default-export.js");
    let snip = snippet("a.js", source);
    assert_eq!(
        digests(&snip),
        [
            want("Named/Syntax", "d", "default", "d", "m", "d", "d", "export { d, ns };"),
            want("Namespace/Syntax", "ns", "-", "ns", "n", "ns", "ns", "export { d, ns };"),
            want("Named/Syntax", "default", "default", "d", "m", "d", "d", "export default d;"),
        ]
    );
    assert_eq!(marked_imports(&snip), [("d".to_owned(), true), ("ns".to_owned(), true)]);
}

#[test]
fn js_export_default_of_a_named_import_names_the_original() {
    let source = include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/js/pos-import-then-export-default-id.js");
    let snip = snippet("a.js", source);
    assert_eq!(
        digests(&snip),
        [want("Named/Syntax", "default", "a", "a", "m", "a", "a", "export default a;")]
    );
    assert_eq!(marked_imports(&snip), [("a".to_owned(), true)]);
}

#[test]
fn js_local_exports_are_not_forwards() {
    let source = include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/js/neg-local-export.js");
    let snip = snippet("a.js", source);
    assert!(digests(&snip).is_empty());
    assert_complete(&snip.report());
    let imported_but_unexported = snippet("a.js", "import { a } from \"m\";\nexport const b = a;\nexport default function () {}\nexport default 1 + a;\n");
    assert!(digests(&imported_but_unexported).is_empty());
    assert_eq!(marked_imports(&imported_but_unexported), [("a".to_owned(), false)]);
}

#[test]
fn js_import_that_is_not_exported_is_not_marked() {
    let snip = snippet("a.js", "import { a, b } from \"m\";\nexport { b };\n");
    assert_eq!(marked_imports(&snip), [("a".to_owned(), false), ("b".to_owned(), true)]);
}

#[test]
fn js_block_scoped_declaration_shadows_the_import_with_a_limit() {
    let source = include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/js/amb-import-shadow-block.js");
    let snip = snippet("a.js", source);
    assert!(digests(&snip).is_empty());
    assert_limited(&snip, &[(ReexportLimitReason::ShadowedGlobal, "a")]);
    assert_eq!(marked_imports(&snip), [("a".to_owned(), false)]);
}

#[test]
fn js_same_scope_declaration_shadows_the_import() {
    let snip = snippet("a.js", "import { a } from \"m\";\nvar a = 1;\nexport { a };\n");
    assert!(digests(&snip).is_empty());
    assert_limited(&snip, &[(ReexportLimitReason::ShadowedGlobal, "a")]);
}

#[test]
fn js_function_parameter_does_not_shadow_the_module_binding() {
    let source = include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/js/amb-import-rebound.js");
    let snip = snippet("a.js", source);
    assert_eq!(
        digests(&snip),
        [want("Named/Syntax", "a", "a", "a", "m", "a", "a", "export { a };")]
    );
}

#[test]
fn js_mutable_copy_of_an_import_that_is_reassigned_is_limited_not_forwarded() {
    let source = include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/js/amb-import-rebound.js");
    let snip = snippet("a.js", source);
    assert_limited(&snip, &[(ReexportLimitReason::ReboundBinding, "b")]);
    let const_copy = snippet("a.js", "import { a } from \"m\";\nconst b = a;\nexport { b };\n");
    assert!(digests(&const_copy).is_empty());
    assert_complete(&const_copy.report());
}

#[test]
fn js_reassigned_import_binding_is_limited_and_a_shadowed_assignment_is_not() {
    let rebound = snippet("a.js", "import { a } from \"m\";\na = 2;\nexport { a };\n");
    assert!(digests(&rebound).is_empty());
    assert_limited(&rebound, &[(ReexportLimitReason::ReboundBinding, "a")]);
    let unrelated = snippet("a.js", "import { a } from \"m\";\nfunction f(a) { a = 2; }\nexport { a };\n");
    assert_eq!(
        digests(&unrelated),
        [want("Named/Syntax", "a", "a", "a", "m", "a", "a", "export { a };")]
    );
    assert_complete(&unrelated.report());
}

#[test]
fn js_import_attributes_limit_with_parse_error_and_forward_nothing_from_the_statement() {
    let source = include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/js/pos-attributes.js");
    let snip = snippet("a.js", source);
    assert!(digests(&snip).is_empty());
    assert_limited(&snip, &[(ReexportLimitReason::ParseError, r#"export { a } from "m" with {"#)]);
}

#[test]
fn js_damaged_export_keeps_earlier_valid_entries_but_is_limited() {
    let snip = snippet("a.js", "export { z } from \"k\";\nexport { a } from ;\n");
    assert_eq!(digests(&snip), [direct("Named/Syntax", "z", "z", "k", "z", r#"export { z } from "k";"#)]);
    assert_eq!(snip.report().coverage.state, ReexportCoverageState::Limited);
    assert_eq!(snip.report().coverage.limits[0].reason, ReexportLimitReason::ParseError);
}

#[test]
fn js_unrelated_parse_error_keeps_entries_and_stays_complete() {
    let snip = snippet("a.js", "export { a } from \"m\";\nconst x = ;\nfunction f( {\n");
    assert_eq!(digests(&snip), [direct("Named/Syntax", "a", "a", "m", "a", r#"export { a } from "m";"#)]);
    assert_complete(&snip.report());
}

#[test]
fn js_duplicate_forwards_stay_distinct_sites_in_source_order() {
    let snip = snippet("a.js", "export { a, a } from \"m\";\nexport * from \"m\";\nexport * from \"m\";\n");
    let report = snip.report();
    assert_eq!(report.entries.len(), 4);
    let starts: Vec<_> = report.entries.iter().map(|entry| entry.span.start_byte).collect();
    let mut sorted = starts.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(starts, sorted);
}

#[test]
fn js_extensions_select_the_javascript_extractor() {
    for name in ["a.js", "a.mjs", "a.cjs", "a.jsx"] {
        let snip = snippet(name, "export { a } from \"m\";\nexport const el = 1;\n");
        assert_eq!(
            digests(&snip),
            [direct("Named/Syntax", "a", "a", "m", "a", r#"export { a } from "m";"#)],
            "{name}"
        );
    }
}

#[test]
fn jsx_syntax_does_not_disturb_the_extraction() {
    let snip = snippet("a.jsx", "export { a } from \"m\";\nexport const el = <div className=\"x\">hi</div>;\n");
    assert_eq!(digests(&snip), [direct("Named/Syntax", "a", "a", "m", "a", r#"export { a } from "m";"#)]);
    assert_complete(&snip.report());
}

// ---------------------------------------------------------------- CommonJS

#[test]
fn cjs_module_exports_of_require_is_a_declared_module_forward() {
    let source = include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/js/pos-cjs-module-exports.js");
    let snip = snippet("a.cjs", source);
    assert_eq!(
        digests(&snip),
        [direct("Module/Declared", "-", "-", "m", r#"module.exports = require("m")"#, r#"module.exports = require("m");"#)]
    );
    assert_complete(&snip.report());
    assert_eq!(snip.indexed().reexports, Some(snip.report()));
}

#[test]
fn cjs_property_assignments_are_named_or_namespace_forwards() {
    let source = include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/js/pos-cjs-exports-prop.js");
    let snip = snippet("a.js", source);
    assert_eq!(
        digests(&snip),
        [
            direct("Named/Declared", "x", "y", "m", r#"exports.x = require("m").y"#, r#"exports.x = require("m").y;"#),
            direct("Namespace/Declared", "z", "-", "m", r#"module.exports.z = require("m")"#, r#"module.exports.z = require("m");"#),
            direct("Named/Declared", "w", "v", "m", r#"module.exports.w = require("m").v"#, r#"module.exports.w = require("m").v;"#),
        ]
    );
    assert_complete(&snip.report());
}

#[test]
fn cjs_destructured_and_object_literal_forms_resolve_through_require_bindings() {
    let source = include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/js/pos-cjs-destructure.js");
    let snip = snippet("a.js", source);
    assert_eq!(
        digests(&snip),
        [
            want("Named/Declared", "a", "a", "a", "m", "-", "a", "module.exports = { a };"),
            want("Namespace/Declared", "b", "-", "b", "n", "-", "b", "module.exports = { b, c: b.c };"),
            want("Named/Declared", "c", "c", "b", "n", "-", "c: b.c", "module.exports = { b, c: b.c };"),
            want("Named/Declared", "d", "a", "a", "m", "-", "exports.d = a", "exports.d = a;"),
        ]
    );
    assert_complete(&snip.report());
    assert!(marked_imports(&snip).is_empty());
}

#[test]
fn cjs_renamed_destructuring_names_the_original_member() {
    let snip = snippet("a.js", "const { a: z } = require(\"m\");\nexports.q = z;\n");
    assert_eq!(
        digests(&snip),
        [want("Named/Declared", "q", "a", "z", "m", "-", "exports.q = z", "exports.q = z;")]
    );
}

#[test]
fn cjs_forwards_of_an_es_import_link_the_import_record() {
    let snip = snippet("a.js", "import d from \"m\";\nexports.x = d;\n");
    assert_eq!(
        digests(&snip),
        [want("Named/Declared", "x", "default", "d", "m", "d", "exports.x = d", "exports.x = d;")]
    );
    assert_eq!(marked_imports(&snip), [("d".to_owned(), true)]);
}

#[test]
fn cjs_local_objects_and_the_bare_require_function_are_not_forwards() {
    let source = include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/js/neg-cjs-local.js");
    let snip = snippet("a.js", source);
    assert!(digests(&snip).is_empty());
    assert_complete(&snip.report());
}

#[test]
fn cjs_conditional_dynamic_and_merge_forms_are_limited_and_the_guarded_one_is_conditional() {
    let source = include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/js/amb-cjs-conditional.js");
    let snip = snippet("a.js", source);
    assert_eq!(
        digests(&snip),
        [conditional(direct(
            "Module/Declared",
            "-",
            "-",
            "m",
            r#"module.exports = require("m")"#,
            r#"module.exports = require("m");"#
        ))]
    );
    assert_limited(
        &snip,
        &[
            (ReexportLimitReason::DynamicForwarding, r#"module.exports = cond ? require("a") : require("b")"#),
            (ReexportLimitReason::DynamicForwarding, "module.exports = require(name)"),
            (ReexportLimitReason::DynamicForwarding, r#"module.exports = require("m") || {}"#),
            (ReexportLimitReason::DynamicForwarding, r#"module.exports[key] = require("m")"#),
            (ReexportLimitReason::DynamicForwarding, r#"Object.assign(module.exports, require("m"))"#),
            (ReexportLimitReason::DynamicForwarding, r#"...require("m")"#),
        ],
    );
}

#[test]
fn cjs_shadowed_require_module_and_exports_are_limited_not_forwarded() {
    let source = include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/js/amb-cjs-shadowed.js");
    let snip = snippet("a.js", source);
    assert!(digests(&snip).is_empty());
    assert_limited(
        &snip,
        &[
            (ReexportLimitReason::ShadowedGlobal, r#"module.exports = require("m")"#),
            (ReexportLimitReason::ShadowedGlobal, r#"module.exports = require("m")"#),
            (ReexportLimitReason::ShadowedGlobal, r#"exports.x = require("m").y"#),
        ],
    );
}

#[test]
fn cjs_module_scope_require_declaration_shadows_the_global() {
    let snip = snippet("a.js", "const require = makeRequire();\nmodule.exports = require(\"m\");\n");
    assert!(digests(&snip).is_empty());
    assert_limited(&snip, &[(ReexportLimitReason::ShadowedGlobal, r#"module.exports = require("m")"#)]);
}

#[test]
fn cjs_assignment_inside_a_function_may_never_run_and_is_dynamic() {
    let snip = snippet("a.js", "function setup() {\n  module.exports = require(\"m\");\n}\n");
    assert!(digests(&snip).is_empty());
    assert_limited(&snip, &[(ReexportLimitReason::DynamicForwarding, r#"module.exports = require("m")"#)]);
}

#[test]
fn cjs_reassigned_binding_is_limited() {
    let snip = snippet("a.js", "let b = require(\"n\");\nb = null;\nexports.x = b;\n");
    assert!(digests(&snip).is_empty());
    assert_limited(&snip, &[(ReexportLimitReason::ReboundBinding, "exports.x = b")]);
}

#[test]
fn cjs_try_and_ternary_guarded_assignments_are_conditional() {
    let snip = snippet("a.js", "try { exports.x = require(\"m\"); } catch (e) {}\n");
    assert_eq!(
        digests(&snip),
        [conditional(direct("Namespace/Declared", "x", "-", "m", r#"exports.x = require("m")"#, r#"exports.x = require("m");"#))]
    );
}

#[test]
fn cjs_and_es_entries_interleave_in_source_order() {
    let snip = snippet("a.js", "module.exports.a = require(\"a\");\nexport { b } from \"b\";\nexports.c = require(\"c\");\n");
    let sources: Vec<_> = snip.report().entries.iter().map(|entry| entry.source.clone().unwrap_or_default()).collect();
    assert_eq!(sources, ["a", "b", "c"]);
}

#[test]
fn js_mixed_destructured_binding_with_computed_key_is_dynamic() {
    let snip = snippet("a.js", "const m = require(\"m\");\nmodule.exports = { [key]: m, ...m };\n");
    assert!(digests(&snip).is_empty());
    assert_limited(
        &snip,
        &[(ReexportLimitReason::DynamicForwarding, "[key]: m"), (ReexportLimitReason::DynamicForwarding, "...m")],
    );
}

// ---------------------------------------------------------------- TypeScript

#[test]
fn ts_shares_the_javascript_forms() {
    let named = include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/ts/pos-named.ts");
    let renamed = include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/ts/pos-renamed.ts");
    let wildcard = include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/ts/pos-wildcard.ts");
    let namespace = include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/ts/pos-namespace.ts");
    assert_eq!(digests(&snippet("a.ts", named)), [direct("Named/Syntax", "a", "a", "m", "a", r#"export { a } from "m";"#)]);
    assert_eq!(
        digests(&snippet("a.ts", renamed)),
        [direct("Named/Syntax", "b", "a", "m", "a as b", r#"export { a as b } from "m";"#)]
    );
    assert_eq!(digests(&snippet("a.ts", wildcard)), [direct("Wildcard/Syntax", "-", "-", "m", "*", r#"export * from "m";"#)]);
    assert_eq!(
        digests(&snippet("a.ts", namespace)),
        [direct("Namespace/Syntax", "ns", "-", "m", "* as ns", r#"export * as ns from "m";"#)]
    );
}

#[test]
fn ts_default_is_an_identifier_named_default() {
    let source = include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/ts/pos-default-forms.ts");
    let snip = snippet("a.ts", source);
    assert_eq!(
        digests(&snip),
        [
            direct("Named/Syntax", "default", "x", "m", "x as default", r#"export { x as default } from "m";"#),
            direct("Named/Syntax", "default", "default", "m", "default", r#"export { default } from "m";"#),
            direct("Named/Syntax", "y", "default", "m", "default as y", r#"export { default as y } from "m";"#),
        ]
    );
}

#[test]
fn ts_import_then_export_links_the_import() {
    let source = include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/ts/pos-import-export.ts");
    let snip = snippet("a.ts", source);
    assert_eq!(
        digests(&snip),
        [want("Named/Syntax", "b", "a", "local", "m", "local", "local as b", "export { local as b };")]
    );
    assert_eq!(marked_imports(&snip), [("local".to_owned(), true)]);
}

#[test]
fn ts_statement_type_qualifier_makes_every_specifier_type_only() {
    let source = include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/ts/pos-type-statement.ts");
    let snip = snippet("a.ts", source);
    let statement = r#"export type { T, U as V } from "m";"#;
    assert_eq!(
        digests(&snip),
        [
            typed(direct("Named/Syntax", "T", "T", "m", "T", statement)),
            typed(direct("Named/Syntax", "V", "U", "m", "U as V", statement)),
        ]
    );
}

#[test]
fn ts_specifier_type_qualifier_is_per_specifier() {
    let source = include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/ts/pos-type-specifier.ts");
    let snip = snippet("a.ts", source);
    let statement = r#"export { type T, U, type W as X } from "m";"#;
    assert_eq!(
        digests(&snip),
        [
            typed(direct("Named/Syntax", "T", "T", "m", "type T", statement)),
            direct("Named/Syntax", "U", "U", "m", "U", statement),
            typed(direct("Named/Syntax", "X", "W", "m", "type W as X", statement)),
        ]
    );
}

#[test]
fn ts_type_only_imports_make_their_forwards_type_only() {
    let source = include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/ts/pos-type-local-clause.ts");
    let snip = snippet("a.ts", source);
    assert_eq!(
        digests(&snip),
        [
            typed(want("Named/Syntax", "T", "T", "T", "m", "T", "T", "export type { T };")),
            typed(want("Named/Syntax", "U", "U", "U", "n", "U", "type U", "export { type U };")),
        ]
    );
    assert_eq!(marked_imports(&snip), [("T".to_owned(), true), ("U".to_owned(), true)]);
}

#[test]
fn ts_type_export_of_a_local_type_is_not_a_forward() {
    let source = include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/ts/neg-local-export.ts");
    let snip = snippet("a.ts", source);
    assert!(digests(&snip).is_empty());
    assert_complete(&snip.report());
}

#[test]
fn ts_export_type_star_forms_are_grammar_gaps() {
    let wildcard = include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/ts/pos-type-wildcard.ts");
    let namespace = include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/ts/pos-type-namespace.ts");
    for source in [wildcard, namespace] {
        let snip = snippet("a.ts", source);
        assert!(digests(&snip).is_empty());
        assert_limited(&snip, &[(ReexportLimitReason::GrammarGap, "type")]);
    }
}

#[test]
fn ts_export_equals_of_an_import_is_a_declared_module_forward() {
    let require_import = include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/ts/pos-export-equals-import.ts");
    let default_import = include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/ts/pos-export-equals-import2.ts");
    let snip = snippet("a.ts", require_import);
    assert_eq!(
        digests(&snip),
        [want("Module/Declared", "-", "-", "x", "m", "x", "x", "export = x;")]
    );
    assert_eq!(marked_imports(&snip), [("x".to_owned(), true)]);
    let snip = snippet("a.ts", default_import);
    assert_eq!(
        digests(&snip),
        [want("Module/Declared", "-", "-", "x", "m", "x", "x", "export = x;")]
    );
    assert_eq!(marked_imports(&snip), [("x".to_owned(), true)]);
}

#[test]
fn ts_export_equals_require_has_no_local_binding() {
    let source = include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/ts/pos-export-equals-require.ts");
    let snip = snippet("a.ts", source);
    assert_eq!(
        digests(&snip),
        [direct("Module/Declared", "-", "-", "m", r#"require("m")"#, r#"export = require("m");"#)]
    );
    let shadowed = snippet("a.ts", "function f(require: any) { }\nconst require = 1;\nexport = require(\"m\");\n");
    assert!(digests(&shadowed).is_empty());
    assert_eq!(shadowed.report().coverage.limits[0].reason, ReexportLimitReason::ShadowedGlobal);
}

#[test]
fn ts_export_equals_of_a_local_is_not_a_forward() {
    let source = include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/ts/neg-export-equals-local.ts");
    let snip = snippet("a.ts", source);
    assert!(digests(&snip).is_empty());
    assert_complete(&snip.report());
}

#[test]
fn ts_export_equals_of_a_member_or_object_is_limited() {
    let source = include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/ts/amb-export-equals-call.ts");
    let snip = snippet("a.ts", source);
    assert!(digests(&snip).is_empty());
    assert_limited(
        &snip,
        &[(ReexportLimitReason::DynamicForwarding, "x.y"), (ReexportLimitReason::DynamicForwarding, "{ x }")],
    );
}

#[test]
fn ts_export_as_namespace_is_not_a_forward_but_the_following_export_equals_is() {
    let source = include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/ts/pos-export-as-namespace.ts");
    let snip = snippet("a.ts", source);
    assert_eq!(
        digests(&snip),
        [want("Module/Declared", "-", "-", "m", "m", "m", "m", "export = m;")]
    );
    let alone = snippet("a.ts", "import * as m from \"m\";\nexport as namespace MyLib;\n");
    assert!(digests(&alone).is_empty());
    assert_complete(&alone.report());
}

#[test]
fn ts_export_import_alias_reports_only_when_the_head_is_an_import() {
    let imported = snippet("a.ts", "import * as B from \"m\";\nexport import A = B.C;\nexport import E = B;\n");
    assert_eq!(
        digests(&imported),
        [
            want("Named/Syntax", "A", "C", "B", "m", "B", "import A = B.C;", "export import A = B.C;"),
            want("Namespace/Syntax", "E", "-", "B", "m", "B", "import E = B;", "export import E = B;"),
        ]
    );
    // The alias bindings `A` and `E` are import records too; the entries
    // forward `B`, the binding they go through.
    assert_eq!(
        marked_imports(&imported),
        [("B".to_owned(), true), ("A".to_owned(), false), ("E".to_owned(), false)]
    );
    let named = snippet("a.ts", "import { X } from \"m\";\nexport import A = X.Y.Z;\n");
    assert_eq!(
        digests(&named),
        [want("Named/Syntax", "A", "X.Y.Z", "X", "m", "X", "import A = X.Y.Z;", "export import A = X.Y.Z;")]
    );
}

#[test]
fn ts_export_import_of_a_local_namespace_is_not_a_forward() {
    let source = include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/ts/neg-ns-alias-local.ts");
    let snip = snippet("a.ts", source);
    assert!(digests(&snip).is_empty());
    assert_complete(&snip.report());
}

#[test]
fn ts_export_import_require_is_a_grammar_gap_and_other_lines_survive() {
    let source = include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/ts/pos-export-import-alias.ts");
    let snip = snippet("a.ts", source);
    assert!(digests(&snip).is_empty());
    assert_limited(&snip, &[(ReexportLimitReason::GrammarGap, "")]);
    let mixed = snippet("a.ts", "export { a } from \"m\";\nexport import D = require(\"m\");\n");
    assert_eq!(digests(&mixed), [direct("Named/Syntax", "a", "a", "m", "a", r#"export { a } from "m";"#)]);
    assert_eq!(mixed.report().coverage.state, ReexportCoverageState::Limited);
}

#[test]
fn ts_namespace_and_ambient_module_bodies_carry_their_scope() {
    let source = include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/ts/pos-ns-export-import.ts");
    let snip = snippet("a.ts", source);
    assert_eq!(
        digests(&snip),
        [
            scoped(direct("Named/Syntax", "x", "x", "m", "x", r#"export { x } from "m";"#), "N"),
            scoped(direct("Wildcard/Syntax", "-", "-", "m", "*", r#"export * from "m";"#), "mod"),
            scoped(direct("Named/Syntax", "a", "a", "n", "a", r#"export { a } from "n";"#), "mod"),
        ]
    );
    assert_complete(&snip.report());
}

#[test]
fn ts_declaration_files_report_their_forwards_and_not_local_exports() {
    let source = include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/ts/pos-dts-reexport.d.ts");
    let snip = snippet("a.d.ts", source);
    assert_eq!(
        digests(&snip),
        [
            direct("Wildcard/Syntax", "-", "-", "./a", "*", r#"export * from "./a";"#),
            direct("Named/Syntax", "B", "B", "./b", "B", r#"export { B } from "./b";"#),
        ]
    );
}

#[test]
fn ts_decorated_exports_are_not_forwards() {
    let source = include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/ts/pos-decorator-export.ts");
    let snip = snippet("a.ts", source);
    assert!(digests(&snip).is_empty());
    assert_complete(&snip.report());
}

#[test]
fn ts_unrelated_parse_error_stays_complete_and_a_body_error_in_an_export_is_unrelated() {
    let snip = snippet("a.ts", "export { a } from \"m\";\nlet x: = ;\nexport function f() { let y = ; }\n");
    assert_eq!(digests(&snip), [direct("Named/Syntax", "a", "a", "m", "a", r#"export { a } from "m";"#)]);
    assert_complete(&snip.report());
}

#[test]
fn ts_broken_forward_is_limited_with_parse_error() {
    let snip = snippet("a.ts", "export { a } from ;\n");
    assert!(digests(&snip).is_empty());
    assert_eq!(snip.report().coverage.state, ReexportCoverageState::Limited);
    assert!(snip.report().coverage.limits.iter().all(|limit| limit.reason == ReexportLimitReason::ParseError));
}

#[test]
fn ts_commonjs_forms_work_in_typescript_files() {
    let snip = snippet("a.cts", "module.exports = require(\"m\");\n");
    assert_eq!(
        digests(&snip),
        [direct("Module/Declared", "-", "-", "m", r#"module.exports = require("m")"#, r#"module.exports = require("m");"#)]
    );
}

#[test]
fn tsx_files_parse_with_jsx_and_report_forwards() {
    let snip = snippet("a.tsx", "export { a } from \"m\";\nexport const el = <div>{1}</div>;\n");
    assert_eq!(digests(&snip), [direct("Named/Syntax", "a", "a", "m", "a", r#"export { a } from "m";"#)]);
    assert_complete(&snip.report());
}

#[test]
fn mts_files_use_the_typescript_extractor() {
    let snip = snippet("a.mts", "export type { T } from \"m\";\n");
    assert_eq!(digests(&snip), [typed(direct("Named/Syntax", "T", "T", "m", "T", r#"export type { T } from "m";"#))]);
}

// ---------------------------------------------------------------- corpus

fn assert_report_invariants(snip: &Snippet, name: &str) {
    let report = snip.report();
    let starts: Vec<_> = report.entries.iter().map(|entry| entry.span.start_byte).collect();
    assert!(starts.windows(2).all(|pair| pair[0] < pair[1]), "{name}: entries are source-ordered with unique spans");
    let keys: Vec<_> = report.coverage.limits.iter().map(|limit| (limit.span.start_byte, limit.reason)).collect();
    assert!(keys.windows(2).all(|pair| pair[0] <= pair[1]), "{name}: limits are ordered");
    assert_eq!(
        report.coverage.state == ReexportCoverageState::Limited,
        !report.coverage.limits.is_empty(),
        "{name}: limited iff limits"
    );
    assert_ne!(report.coverage.state, ReexportCoverageState::NotApplicable, "{name}");
    assert_eq!(snip.indexed().reexports, Some(report), "{name}: query agrees with the direct API");
}

#[test]
fn js_corpus_satisfies_the_report_invariants() {
    let files: [(&str, &str); 22] = [
        ("amb-cjs-conditional.js", include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/js/amb-cjs-conditional.js")),
        ("amb-cjs-shadowed.js", include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/js/amb-cjs-shadowed.js")),
        ("amb-import-rebound.js", include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/js/amb-import-rebound.js")),
        ("amb-import-shadow-block.js", include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/js/amb-import-shadow-block.js")),
        ("neg-cjs-local.js", include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/js/neg-cjs-local.js")),
        ("neg-local-export.js", include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/js/neg-local-export.js")),
        ("pos-attributes.js", include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/js/pos-attributes.js")),
        ("pos-cjs-destructure.js", include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/js/pos-cjs-destructure.js")),
        ("pos-cjs-exports-prop.js", include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/js/pos-cjs-exports-prop.js")),
        ("pos-cjs-module-exports.js", include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/js/pos-cjs-module-exports.js")),
        ("pos-default-as-default.js", include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/js/pos-default-as-default.js")),
        ("pos-default-as-name.js", include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/js/pos-default-as-name.js")),
        ("pos-default-bare.js", include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/js/pos-default-bare.js")),
        ("pos-import-default-export.js", include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/js/pos-import-default-export.js")),
        ("pos-import-export.js", include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/js/pos-import-export.js")),
        ("pos-import-then-export-default-id.js", include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/js/pos-import-then-export-default-id.js")),
        ("pos-named.js", include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/js/pos-named.js")),
        ("pos-namespace-default.js", include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/js/pos-namespace-default.js")),
        ("pos-namespace.js", include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/js/pos-namespace.js")),
        ("pos-renamed.js", include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/js/pos-renamed.js")),
        ("pos-string-name.js", include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/js/pos-string-name.js")),
        ("pos-wildcard.js", include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/js/pos-wildcard.js")),
    ];
    for (name, source) in files {
        assert_report_invariants(&snippet(name, source), name);
    }
}

#[test]
fn ts_corpus_satisfies_the_report_invariants() {
    let files: [(&str, &str); 23] = [
        ("amb-export-equals-call.ts", include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/ts/amb-export-equals-call.ts")),
        ("neg-export-equals-local.ts", include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/ts/neg-export-equals-local.ts")),
        ("neg-local-export.ts", include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/ts/neg-local-export.ts")),
        ("neg-ns-alias-local.ts", include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/ts/neg-ns-alias-local.ts")),
        ("pos-decorator-export.ts", include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/ts/pos-decorator-export.ts")),
        ("pos-default-forms.ts", include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/ts/pos-default-forms.ts")),
        ("pos-dts-reexport.d.ts", include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/ts/pos-dts-reexport.d.ts")),
        ("pos-export-as-namespace.ts", include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/ts/pos-export-as-namespace.ts")),
        ("pos-export-equals-import.ts", include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/ts/pos-export-equals-import.ts")),
        ("pos-export-equals-import2.ts", include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/ts/pos-export-equals-import2.ts")),
        ("pos-export-equals-require.ts", include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/ts/pos-export-equals-require.ts")),
        ("pos-export-import-alias.ts", include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/ts/pos-export-import-alias.ts")),
        ("pos-import-export.ts", include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/ts/pos-import-export.ts")),
        ("pos-named.ts", include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/ts/pos-named.ts")),
        ("pos-namespace.ts", include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/ts/pos-namespace.ts")),
        ("pos-ns-export-import.ts", include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/ts/pos-ns-export-import.ts")),
        ("pos-renamed.ts", include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/ts/pos-renamed.ts")),
        ("pos-type-local-clause.ts", include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/ts/pos-type-local-clause.ts")),
        ("pos-type-namespace.ts", include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/ts/pos-type-namespace.ts")),
        ("pos-type-specifier.ts", include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/ts/pos-type-specifier.ts")),
        ("pos-type-statement.ts", include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/ts/pos-type-statement.ts")),
        ("pos-type-wildcard.ts", include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/ts/pos-type-wildcard.ts")),
        ("pos-wildcard.ts", include_str!("../../../features/2026-10-11-reexports-all-languages/audit/samples/ts/pos-wildcard.ts")),
    ];
    for (name, source) in files {
        assert_report_invariants(&snippet(name, source), name);
    }
}
