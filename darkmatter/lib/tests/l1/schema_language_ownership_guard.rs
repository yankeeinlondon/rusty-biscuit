//! One implementation of the schema language: `simplified-schema` owns the
//! SimplifiedSchema grammar and types, the expression lexer and parser, JSON
//! Schema lowering, subtyping, the numeric model, and the conversion engine.
//! Darkmatter, its CLI, and DMLS call them; none defines a second one.
//!
//! The check is by name. Each owned item must still be defined in the library
//! file listed for it (so a rename there cannot silently retire a row), and no
//! production source of the three Darkmatter crates (comments, literals, and
//! test-only code blanked by `source_scan`) may define an item of that name.
//! Thin delegating wrappers that share a library name (the expression
//! evaluator's `json_number`, collection blocks' `parse_callback`) are not
//! second implementations and are not listed.

use std::collections::BTreeMap;
use std::path::PathBuf;

#[allow(clippy::duplicate_mod)]
#[path = "../../../cli/tests/common/source_scan.rs"]
mod source_scan;

use source_scan::{ident_offsets, is_ident, line_at, production_sources, sanitize};

const DEFINING_KEYWORDS: &[&str] = &["fn", "struct", "enum", "trait", "type", "union"];

/// Offsets where `name` is defined (`fn name`, `struct name`, ...) in
/// sanitized source.
fn definitions(sanitized: &str, name: &str) -> Vec<usize> {
    ident_offsets(sanitized, name)
        .into_iter()
        .filter(|&offset| {
            let before = sanitized[..offset].trim_end();
            DEFINING_KEYWORDS.iter().any(|keyword| {
                before.strip_suffix(keyword).is_some_and(|rest| !rest.bytes().last().is_some_and(is_ident))
            })
        })
        .collect()
}

/// `"<file>:<line>: defines `<name>`"` for every owned name a sanitized
/// source defines.
fn second_implementations(file: &str, sanitized: &str, owned: &[&str]) -> Vec<String> {
    let mut violations: Vec<String> = owned
        .iter()
        .flat_map(|name| {
            definitions(sanitized, name)
                .into_iter()
                .map(move |offset| format!("{file}:{}: defines `{name}`", line_at(sanitized, offset)))
        })
        .collect();
    violations.sort();
    violations
}

#[test]
fn darkmatter_defines_no_second_schema_language_implementation() {
    // Every owned item, the library file that defines it, and that file's
    // text. The literals stay here so CI's test-input index ties each file to
    // this test.
    let owners: &[(&str, &str, &[&str])] = &[
        (
            "simplified/mod.rs",
            include_str!("../../../../simplified-schema/lib/src/simplified/mod.rs"),
            &["parse_yaml_schema", "parse_property_definition", "parse_schema_declaration"],
        ),
        (
            "simplified/grammar.rs",
            include_str!("../../../../simplified-schema/lib/src/simplified/grammar.rs"),
            &["parse_type_expr", "parse_type_definition"],
        ),
        (
            "simplified/standalone.rs",
            include_str!("../../../../simplified-schema/lib/src/simplified/standalone.rs"),
            &["parse_standalone_schema_document"],
        ),
        (
            "simplified/types.rs",
            include_str!("../../../../simplified-schema/lib/src/simplified/types.rs"),
            &["SchemaRoot", "SchemaShape", "PropertyDef", "PropertyAtom", "TypeExpr"],
        ),
        (
            "simplified/serialize.rs",
            include_str!("../../../../simplified-schema/lib/src/simplified/serialize.rs"),
            &["serialize_property_def", "serialize_property_atom"],
        ),
        (
            "simplified/subtype.rs",
            include_str!("../../../../simplified-schema/lib/src/simplified/subtype.rs"),
            &["is_subtype", "is_subtype_with_definitions", "is_definition_subtype", "is_definition_subtype_with_definitions"],
        ),
        (
            "simplified/convert.rs",
            include_str!("../../../../simplified-schema/lib/src/simplified/convert.rs"),
            &["to_json_schema", "to_json_schema_with_definitions"],
        ),
        (
            "json_shape.rs",
            include_str!("../../../../simplified-schema/lib/src/json_shape.rs"),
            &["shape_from_json_schema"],
        ),
        (
            "expression/lexer.rs",
            include_str!("../../../../simplified-schema/lib/src/expression/lexer.rs"),
            &["Lexer", "LexerError", "lex_spanned"],
        ),
        (
            "expression/parser.rs",
            include_str!("../../../../simplified-schema/lib/src/expression/parser.rs"),
            &["ParseError", "parse_spanned", "parse_callback_spanned"],
        ),
        (
            "expression/ast.rs",
            include_str!("../../../../simplified-schema/lib/src/expression/ast.rs"),
            &["Expr", "SpannedExpr"],
        ),
        (
            "numeric.rs",
            include_str!("../../../../simplified-schema/lib/src/numeric.rs"),
            &[
                "NumberTextError",
                "NumericAdmissionError",
                "admit_number_spelling",
                "admit_number_text",
                "admit_f64",
                "admit_json_number",
                "admit_integer",
                "format_number",
            ],
        ),
        (
            "coercion/mod.rs",
            include_str!("../../../../simplified-schema/lib/src/coercion/mod.rs"),
            &["Bound", "BindOptions", "bind_with", "bind_with_marked", "check_exact", "accepts_exactly"],
        ),
        (
            "coercion/error.rs",
            include_str!("../../../../simplified-schema/lib/src/coercion/error.rs"),
            &["ConversionError"],
        ),
        (
            "simplified/definitions.rs",
            include_str!("../../../../simplified-schema/lib/src/simplified/definitions.rs"),
            &["DefinitionTable"],
        ),
    ];

    let mut stale = Vec::new();
    for (file, text, names) in owners {
        let sanitized = String::from_utf8(sanitize(text)).expect("sanitizing keeps UTF-8");
        for name in *names {
            if definitions(&sanitized, name).is_empty() {
                stale.push(format!("simplified-schema/lib/src/{file} no longer defines `{name}`"));
            }
        }
    }
    assert!(stale.is_empty(), "update the owned-item table:\n{}", stale.join("\n"));

    let owned: Vec<&str> = owners.iter().flat_map(|(_, _, names)| names.iter().copied()).collect();
    let crates: BTreeMap<&str, PathBuf> = [
        ("darkmatter/lib/src", biscuit_test_harness::manifest_dir!().join("src")),
        ("darkmatter/cli/src", biscuit_test_harness::manifest_dir!().join("../cli/src")),
        ("darkmatter/dmls/src", biscuit_test_harness::manifest_dir!().join("../dmls/src")),
    ]
    .into_iter()
    .collect();
    let mut scanned = 0;
    let mut violations = Vec::new();
    for (label, src) in &crates {
        for (file, text) in production_sources(src) {
            scanned += 1;
            violations.extend(second_implementations(&format!("{label}/{file}"), &text, &owned));
        }
    }
    assert!(scanned > 100, "the scan reached the three crates' sources ({scanned} files)");
    assert!(
        violations.is_empty(),
        "the schema language has one implementation, in simplified-schema; call it instead:\n{}",
        violations.join("\n")
    );
}

#[test]
fn a_planted_second_implementation_is_detected() {
    let source = r#"
pub fn to_json_schema(root: &SchemaRoot) -> Value { todo!() }
pub(crate) struct Lexer<'a> { input: &'a str }
enum SchemaRoot { Single }
// fn is_subtype(a: u8) -> bool
const TEXT: &str = "fn format_number(value: f64)";
fn caller() { let _ = simplified_schema::simplified::to_json_schema(&root); let my_fn = is_subtype; }
fn to_json_schema_cached() {}
"#;
    let sanitized = String::from_utf8(sanitize(source)).expect("sanitizing keeps UTF-8");

    assert_eq!(
        second_implementations(
            "planted.rs",
            &sanitized,
            &["to_json_schema", "Lexer", "SchemaRoot", "is_subtype", "format_number"]
        ),
        ["planted.rs:2: defines `to_json_schema`", "planted.rs:3: defines `Lexer`", "planted.rs:4: defines `SchemaRoot`"]
    );
}
