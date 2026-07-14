//! Acceptance suite for the SimplifiedSchema `literal` and `expression` types:
//! grammar parsing and round-trip serialization, JSON Schema conversion, and
//! read-only document validation across the two type keywords.

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::expression::{parse, parse_condition};
use darkmatter::markdown::schemas::{
    DarkmatterSchemas, ValidationReport, parse_yaml_schema, to_json_schema,
    simplified::{grammar::parse_type_expr, serialize_property_atom},
};
use serde_json::Value;

// ── Helpers ────────────────────────────────────────────────────────────────

/// Parse a single type-expression string as the value of property `prop`.
fn atom(input: &str) -> Result<darkmatter::markdown::schemas::PropertyAtom, String> {
    parse_type_expr("prop", input).map_err(|e| e.to_string())
}

/// Canonical serialization of a freshly parsed atom (round-trip left side).
fn serialize(input: &str) -> String {
    let parsed = parse_type_expr("prop", input).expect("type expression should parse");
    serialize_property_atom(&parsed)
}

/// Compile a `$schema`-body YAML mapping to its JSON Schema.
fn json_schema(schema_body: &str) -> Value {
    let yaml: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(schema_body).expect("schema YAML must parse");
    let schema = parse_yaml_schema(&yaml).expect("SimplifiedSchema must parse");
    to_json_schema(&schema).expect("SimplifiedSchema must convert to JSON Schema")
}

/// Validate an in-memory document (inline `$schema`) read-only (no coercion).
fn validate(doc: &str) -> ValidationReport {
    let md: Markdown = doc.into();
    DarkmatterSchemas::new()
        .validate(&md)
        .expect("schema must compile and validation must run")
}

/// The `const` a `literal`-typed **optional** property compiles to, reached
/// through the standard optional-nullable `anyOf` wrapper
/// (`[{"type":"null"}, {"const": …}]`).
fn optional_const<'a>(schema: &'a Value, prop: &str) -> &'a Value {
    &schema["properties"][prop]["anyOf"][1]["const"]
}

/// The item `const` a `literal(x)[]`-typed **optional** property compiles to
/// (`[{"type":"null"}, {"type":"array","items":{"const": …}}]`).
fn optional_array_item_const<'a>(schema: &'a Value, prop: &str) -> &'a Value {
    &schema["properties"][prop]["anyOf"][1]["items"]["const"]
}

/// Compose an in-memory document and read back one coerced frontmatter value.
fn compose_value(doc: &str, key: &str) -> Value {
    let md: Markdown = doc.into();
    let (composed, _) = md.compose().expect("compose must succeed");
    composed
        .frontmatter()
        .get::<Value>(key)
        .expect("frontmatter value must read")
        .expect("frontmatter key must be present")
}

// ══════════════════════════════════════════════════════════════════════════
// Feature A — `literal(value)` grammar & serialization (Phase 2)  [AC 1]
// ══════════════════════════════════════════════════════════════════════════

#[test]
fn literal_bare_string_parses_and_serializes() {
    assert!(atom("literal(spec)").is_ok());
    assert_eq!(serialize("literal(spec)"), "literal(spec)");
}

#[test]
fn literal_bare_number_typed() {
    assert!(atom("literal(2)").is_ok());
    // Number-typed const in the compiled schema (AC 1 / AC 3).
    let schema = json_schema("version: literal(2)\n");
    let constv = optional_const(&schema, "version");
    assert!(constv.is_number(), "literal(2) const must be a JSON number, got {constv}");
}

#[test]
fn literal_bare_boolean_typed() {
    assert!(atom("literal(false)").is_ok());
    let schema = json_schema("archived: literal(false)\n");
    assert_eq!(*optional_const(&schema, "archived"), Value::Bool(false));
}

#[test]
fn literal_quoted_protects_punctuation() {
    // Commas and semicolons inside quotes must not split the value or constraints.
    assert!(atom("literal('a, b')").is_ok());
    // Round-trips through canonical serialization.
    let once = serialize("literal('a, b')");
    assert_eq!(serialize(&once), once, "quoted literal must serialize stably");
}

#[test]
fn literal_missing_value_errors() {
    let err = atom("literal()").expect_err("literal() must be rejected");
    assert!(
        err.contains("literal requires a value"),
        "expected 'literal requires a value', got: {err}"
    );
}

#[test]
fn literal_multiple_values_point_at_enum() {
    let err = atom("literal(a, b)").expect_err("two literal values must be rejected");
    assert!(
        err.contains("enum"),
        "multiple-value error should direct the author to enum(...), got: {err}"
    );
}

#[test]
fn literal_bare_null_rejected() {
    let err = atom("literal(null)").expect_err("bare null literal must be rejected");
    assert!(
        err.to_lowercase().contains("quote") || err.to_lowercase().contains("drop"),
        "bare null error should suggest quoting or dropping the key, got: {err}"
    );
}

#[test]
fn literal_leading_zero_is_string_not_number() {
    // `007` fails the numberlike shape test (leading-zero) → typed as string.
    let schema = json_schema("code: literal(007)\n");
    assert_eq!(*optional_const(&schema, "code"), Value::String("007".into()));
}

#[test]
fn literal_array_suffix_allowed() {
    assert!(atom("literal(auto)[]").is_ok());
    let schema = json_schema("modes: literal(auto)[]\n");
    // `const` lives under `items` for the array form.
    assert!(
        optional_array_item_const(&schema, "modes").is_string(),
        "literal(x)[] must place const under items"
    );
}

#[test]
fn literal_parse_serialize_reparse_equivalent() {
    for input in ["literal(spec)", "literal(2)", "literal(false)", "literal('a, b')"] {
        let first = parse_type_expr("prop", input).expect("parse");
        let text = serialize_property_atom(&first);
        let second = parse_type_expr("prop", &text).expect("reparse");
        assert_eq!(first, second, "round-trip mismatch for {input}");
    }
}

// ══════════════════════════════════════════════════════════════════════════
// Feature A — literal validation & constraints (Phase 2/3)  [AC 2]
// ══════════════════════════════════════════════════════════════════════════

#[test]
fn literal_required_enforces_equality() {
    let ok = validate("---\n$schema:\n  kind: literal(spec; required)\nkind: spec\n---\nbody\n");
    assert!(ok.valid, "matching required literal must validate: {:?}", ok.problems);

    let bad = validate("---\n$schema:\n  kind: literal(spec; required)\nkind: other\n---\nbody\n");
    assert!(!bad.valid, "wrong literal value must fail");
}

#[test]
fn literal_optional_accepts_missing_and_null() {
    let missing = validate("---\n$schema:\n  kind: literal(spec)\n---\nbody\n");
    assert!(missing.valid, "optional literal accepts missing: {:?}", missing.problems);

    let null = validate("---\n$schema:\n  kind: literal(spec)\nkind: null\n---\nbody\n");
    assert!(null.valid, "optional literal accepts null: {:?}", null.problems);
}

#[test]
fn literal_default_mismatch_fails_schema_load() {
    // A matching default must load fine (fails today: `literal` is unknown).
    let matching: Markdown =
        "---\n$schema:\n  kind: literal(spec; default(spec))\n---\nbody\n".into();
    assert!(
        DarkmatterSchemas::new().validate(&matching).is_ok(),
        "default equal to the literal value must load"
    );

    // A default that violates its own const is a schema-load error.
    let mismatch: Markdown =
        "---\n$schema:\n  kind: literal(spec; default(other))\n---\nbody\n".into();
    assert!(
        DarkmatterSchemas::new().validate(&mismatch).is_err(),
        "default(other) against literal(spec) must fail to load"
    );
}

#[test]
fn literal_suggest_rejected() {
    let err = atom("literal(spec; suggest(a, b))").expect_err("suggest not allowed on literal");
    assert!(err.to_lowercase().contains("suggest"), "got: {err}");
}

// ══════════════════════════════════════════════════════════════════════════
// Feature A — literal coercion (Phase 4)  [AC 3]
// ══════════════════════════════════════════════════════════════════════════

#[test]
fn literal_number_coerces_string_document_value() {
    // `version: '2'` (string) against literal(2) coerces to number 2.
    let value = compose_value(
        "---\n$schema:\n  version: literal(2)\nversion: '2'\n---\nbody\n",
        "version",
    );
    assert_eq!(value, Value::from(2), "string '2' must coerce to number 2");
}

#[test]
fn literal_string_value_unchanged() {
    let value = compose_value(
        "---\n$schema:\n  kind: literal(spec)\nkind: spec\n---\nbody\n",
        "kind",
    );
    assert_eq!(value, Value::String("spec".into()));
}

// ══════════════════════════════════════════════════════════════════════════
// Feature A — property-level unions mixing literal (Phase 4)  [AC 4]
// ══════════════════════════════════════════════════════════════════════════

#[test]
fn literal_property_union_mixes_with_atom() {
    // width: [literal(auto), number(min(1))]
    let keyword = validate(
        "---\n$schema:\n  width:\n    - literal(auto)\n    - number\nwidth: auto\n---\nbody\n",
    );
    assert!(keyword.valid, "literal arm must accept 'auto': {:?}", keyword.problems);

    let numeric = validate(
        "---\n$schema:\n  width:\n    - literal(auto)\n    - number\nwidth: 5\n---\nbody\n",
    );
    assert!(numeric.valid, "number arm must accept 5: {:?}", numeric.problems);
}

// ══════════════════════════════════════════════════════════════════════════
// Feature A — discriminated-union arm narrowing (Phase 4)  [AC 10]
// ══════════════════════════════════════════════════════════════════════════

#[test]
fn union_narrows_on_matched_literal_discriminant() {
    // Two inline-object arms tagged by a literal `kind`. A document tagged
    // `created` but missing `path` must report only the `created` arm's
    // missing-required problem, not the full anyOf noise.
    let doc = "---\n$schema:\n  event:\n    - '{ kind: literal(created), path: file(required) }'\n    - '{ kind: literal(deleted), reason: string }'\nevent:\n  kind: created\n---\nbody\n";
    let report = validate(doc);
    assert!(!report.valid, "missing required `path` should fail");
    // Narrowed reporting: a single, arm-scoped problem (not merged anyOf).
    assert_eq!(
        report.problems.len(),
        1,
        "narrowed diagnostics should report exactly the matched arm's problem, got {:?}",
        report.problems
    );
}

#[test]
fn union_discriminant_is_type_sensitive() {
    // Typed `2` must NOT select an arm tagged with the string `'2'`.
    let doc = "---\n$schema:\n  v:\n    - '{ tag: literal(2), a: string(required) }'\n    - '{ tag: literal(other), b: string(required) }'\nv:\n  tag: '2'\n---\nbody\n";
    let report = validate(doc);
    // String '2' does not match literal(2) (number) → no narrowing to arm 0.
    assert!(!report.valid, "string '2' should not satisfy number literal(2) arm");
}

#[test]
fn union_matched_arm_reports_only_its_unknown_key() {
    // `event` tagged `deleted` carries an unknown key. Narrowing reports only
    // the matched (deleted) arm's unknown-key problem — never the created arm's
    // `kind` const mismatch.
    let doc = "---\n$schema:\n  event:\n    - '{ kind: literal(created), path: string }'\n    - '{ kind: literal(deleted), reason: string }'\nevent:\n  kind: deleted\n  bogus: 1\n---\nbody\n";
    let report = validate(doc);
    assert!(!report.valid, "unknown key `bogus` should fail");
    assert_eq!(
        report.problems.len(),
        1,
        "only the matched arm's unknown-key problem should surface, got {:?}",
        report.problems
    );
    assert_eq!(
        report.problems[0].offending_property.as_deref(),
        Some("bogus"),
        "the surviving problem should name the undeclared key, got {:?}",
        report.problems
    );
}

#[test]
fn union_duplicate_tags_are_not_narrowed() {
    // Two arms share the same literal `kind` value: the tag is ambiguous, so
    // the general `anyOf` reporting stands (no narrowing to either arm).
    let doc = "---\n$schema:\n  event:\n    - '{ kind: literal(same), a: string(required) }'\n    - '{ kind: literal(same), b: string(required) }'\nevent:\n  kind: same\n---\nbody\n";
    let report = validate(doc);
    assert!(!report.valid, "neither arm is satisfied (missing a / b)");
    // Ambiguous → NOT narrowed: the general merged `anyOf` diagnostic stands
    // (byte-for-byte the pre-literal behavior), rather than a single arm's keys.
    assert!(
        report
            .problems
            .iter()
            .any(|p| p.message.contains("anyOf")),
        "duplicate tags must retain the merged anyOf diagnostic, got {:?}",
        report.problems
    );
}

#[test]
fn union_conflicting_multi_key_discriminants_are_not_narrowed() {
    // `kind` selects arm 0, `mode` selects arm 1: the keys disagree, so no arm
    // is selected and the merged `anyOf` reporting stands.
    let doc = "---\n$schema:\n  cfg:\n    - '{ kind: literal(a), mode: literal(x), one: string(required) }'\n    - '{ kind: literal(b), mode: literal(y), two: string(required) }'\ncfg:\n  kind: a\n  mode: y\n---\nbody\n";
    let report = validate(doc);
    assert!(!report.valid, "conflicting discriminants cannot satisfy any arm");
}

#[test]
fn union_sparse_known_key_plus_unknown_key_are_not_narrowed() {
    // Sparse arms: `kind` tags arms 0/1, `mode` tags arms 2/3. An instance
    // authors `kind: a` (which alone selects arm 0) AND `mode: z` (a qualifying
    // discriminant matching no arm). Because arm 0 does not declare `mode`, a
    // naive selector would wrongly narrow to arm 0 and report only its lone
    // missing-required `one`. The ambiguous second discriminant must defeat
    // narrowing, leaving the general union reporting in place.
    let doc = concat!(
        "---\n$schema:\n  cfg:\n",
        "    - '{ kind: literal(a), one: string(required) }'\n",
        "    - '{ kind: literal(b), two: string(required) }'\n",
        "    - '{ mode: literal(x), three: string(required) }'\n",
        "    - '{ mode: literal(y), four: string(required) }'\n",
        "cfg:\n  kind: a\n  mode: z\n---\nbody\n",
    );
    let report = validate(doc);
    assert!(!report.valid, "no arm is satisfied");
    // Anti-narrowing evidence. Had the selector narrowed to arm 0, the report
    // would be exactly one problem: its missing-required `one`. Instead the
    // general (un-narrowed) union reporting drills the unselected arms' deeper
    // discriminant `const` mismatches (`/cfg/kind` against literal(b),
    // `/cfg/mode` against literal(x)/literal(y)) — multiple problems, none of
    // them arm 0's `one`.
    let narrowed_to_arm0 =
        report.problems.len() == 1 && report.problems[0].property.as_deref() == Some("one");
    assert!(!narrowed_to_arm0, "must not narrow to arm 0, got {:?}", report.problems);
    assert!(
        report
            .problems
            .iter()
            .any(|p| p.path == "/cfg/kind" || p.path == "/cfg/mode"),
        "an unknown second discriminant must defeat narrowing and surface the \
         unselected arms' deeper discriminant mismatches, got {:?}",
        report.problems
    );
}

#[test]
fn root_union_narrows_to_discriminant_over_closest_arm() {
    // Root-level inline-object union tagged by literal `kind`. A `created`
    // document missing two required keys has MORE problems on the created arm
    // than on the deleted arm — so the closest-by-count report would wrongly
    // pick `deleted`. Discriminant narrowing selects `created` and reports its
    // two missing keys instead.
    let doc = "---\n$schema:\n  - kind: literal(created)\n    a: string(required)\n    b: string(required)\n  - kind: literal(deleted)\nkind: created\n---\nbody\n";
    let report = validate(doc);
    assert!(!report.valid, "missing required keys should fail");
    let missing: std::collections::BTreeSet<&str> = report
        .problems
        .iter()
        .filter_map(|p| p.property.as_deref())
        .collect();
    assert_eq!(
        missing,
        ["a", "b"].into_iter().collect(),
        "narrowed to the created arm's two missing keys, got {:?}",
        report.problems
    );
}

// ══════════════════════════════════════════════════════════════════════════
// Feature B — `expression` grammar & serialization (Phase 2)  [AC 5]
// ══════════════════════════════════════════════════════════════════════════

#[test]
fn expression_bare_parses_and_serializes() {
    assert!(atom("expression").is_ok());
    assert_eq!(serialize("expression"), "expression");
}

#[test]
fn expression_with_required_constraint() {
    assert!(atom("expression(required)").is_ok());
}

#[test]
fn expression_parameterized_rejected_in_v1() {
    // Bare `expression` must first be a known keyword (fails today), and the
    // parameterized form must be rejected with a *specific* reserved-form
    // message rather than the generic "unknown type" lexer error.
    assert!(atom("expression").is_ok(), "bare expression must be a known type");
    let err = atom("expression(condition)").expect_err("expression(condition) reserved in v1");
    assert!(
        !err.contains("unknown type"),
        "parameterized expression must get a specific rejection, not the unknown-type error: {err}"
    );
}

// ══════════════════════════════════════════════════════════════════════════
// Feature B — expression conversion & validation (Phase 3)  [AC 5]
// ══════════════════════════════════════════════════════════════════════════

#[test]
fn expression_compiles_to_string_with_format() {
    let schema = json_schema("when: expression\n");
    // Optional property → nullable wrapper; the typed string+format arm follows.
    let typed = &schema["properties"]["when"]["anyOf"][1];
    assert_eq!(typed["type"], "string");
    assert_eq!(typed["format"], "darkmatter-expression");
}

#[test]
fn expression_accepts_either_dialect() {
    // Condition-dialect `&&` must be accepted by bare `expression` (Q2).
    let report = validate(
        "---\n$schema:\n  when: expression\nwhen: 'is_agent() && os == \"macos\"'\n---\nbody\n",
    );
    assert!(report.valid, "either-dialect expression must validate: {:?}", report.problems);
}

#[test]
fn expression_rejects_unparseable_with_format_problem() {
    // A required (non-nullable) expression so the format-specific problem
    // surfaces directly rather than being folded into a nullable `anyOf`.
    let report = validate("---\n$schema:\n  when: 'expression(required)'\nwhen: '(('\n---\nbody\n");
    assert!(!report.valid, "unparseable expression must fail");
    let has_format = report
        .problems
        .iter()
        .any(|p| format!("{p:?}").contains("darkmatter-expression"));
    assert!(has_format, "problem should carry the format name, got {:?}", report.problems);
}

#[test]
fn expression_unknown_identifier_ok() {
    // Identifier resolution is a compose-time concern; schema validation checks
    // parseability only.
    let report = validate("---\n$schema:\n  when: expression\nwhen: some_unknown_thing\n---\nbody\n");
    assert!(report.valid, "unknown identifier must not be a schema error: {:?}", report.problems);
}

// ══════════════════════════════════════════════════════════════════════════
// Feature B — expression coercion & type mismatches (Phase 4)  [AC 6]
// ══════════════════════════════════════════════════════════════════════════

#[test]
fn expression_native_boolean_coerces_to_string() {
    let value = compose_value(
        "---\n$schema:\n  when: expression\nwhen: true\n---\nbody\n",
        "when",
    );
    assert_eq!(value, Value::String("true".into()));
}

#[test]
fn expression_native_number_coerces_to_string() {
    let value = compose_value(
        "---\n$schema:\n  retries: expression\nretries: 3\n---\nbody\n",
        "retries",
    );
    assert_eq!(value, Value::String("3".into()));
}

#[test]
fn expression_mapping_is_type_mismatch() {
    let report = validate("---\n$schema:\n  when: expression\nwhen:\n  a: 1\n---\nbody\n");
    assert!(!report.valid, "mapping against expression must be a type mismatch");
}

#[test]
fn expression_sequence_is_type_mismatch() {
    let report = validate("---\n$schema:\n  when: expression\nwhen:\n  - a\n  - b\n---\nbody\n");
    assert!(!report.valid, "sequence against expression must be a type mismatch");
}

// ══════════════════════════════════════════════════════════════════════════
// Feature B — pending-value deferral (Phase 3/4)  [AC 5]
// ══════════════════════════════════════════════════════════════════════════

#[test]
fn expression_pending_shell_value_deferred() {
    // A value still holding `$(...)` follows existing pending-value deferral,
    // not an eager format failure.
    let md: Markdown = "---\n$schema:\n  when: expression\nwhen: $(echo true)\n---\nbody\n".into();
    let report = DarkmatterSchemas::new()
        .validate(&md)
        .expect("schema must compile");
    // Read-only validate: the pending `$(...)` value is a plain string that
    // still parses; it must not be reported as a malformed expression here.
    let malformed = report
        .problems
        .iter()
        .any(|p| format!("{p:?}").contains("darkmatter-expression"));
    assert!(!malformed, "pending $() value must not eager-fail the format check");
}

// ══════════════════════════════════════════════════════════════════════════
// Feature B — condition-mode parse superset (Phase 3)  [AC 5]
// ══════════════════════════════════════════════════════════════════════════

#[test]
fn value_dialect_is_condition_parse_superset() {
    // The `expression` format is implemented as a single condition-mode parse.
    // For "either dialect" to be sound, every value-dialect-valid string must
    // also condition-parse (the condition dialect adds `&&` and re-lowers `||`,
    // so it is a parse superset). This regression pins that property.
    const CORPUS: &[&str] = &[
        "name",
        "ctx.os",
        "env.HOME",
        "upper(title)",
        "author || \"anon\"",
        "is_agent()",
        "file_exists(\"README.md\")",
        "length(tags)",
    ];
    let mut value_dialect_valid = 0usize;
    for &expr in CORPUS {
        if parse(expr).is_ok() {
            value_dialect_valid += 1;
            assert!(
                parse_condition(expr).is_ok(),
                "value-dialect-valid `{expr}` must also parse in condition mode",
            );
        }
    }
    assert!(
        value_dialect_valid >= 4,
        "corpus should exercise several value-dialect expressions, got {value_dialect_valid}",
    );
}

// ══════════════════════════════════════════════════════════════════════════
// Feature B — expression validation is parse-only (Phase 3)  [AC 5]
// ══════════════════════════════════════════════════════════════════════════

#[test]
fn expression_validation_never_evaluates() {
    // Schema validation checks parseability only — it must never evaluate the
    // expression. A value whose evaluation would touch the filesystem, shell, or
    // environment still validates purely because it *parses*; nothing runs.
    let report = validate(
        "---\n$schema:\n  when: expression\nwhen: 'shell(\"rm -rf /\") || env(\"SECRET\")'\n---\nbody\n",
    );
    assert!(
        report.valid,
        "a parseable expression must validate without evaluation: {:?}",
        report.problems,
    );

    // A reference to a file that does not exist is not an existence check —
    // `expression` never resolves file references (that is `file`'s job).
    let missing = validate(
        "---\n$schema:\n  when: expression\nwhen: 'file_exists(\"/no/such/path-xyz.md\")'\n---\nbody\n",
    );
    assert!(
        missing.valid,
        "expression validation must not resolve file references: {:?}",
        missing.problems,
    );
}

// ══════════════════════════════════════════════════════════════════════════
// Feature A — large-integer literal precision (regression)
// ══════════════════════════════════════════════════════════════════════════

/// Boundary integers past f64's 2^53 exact range: 2^53+1, `i64::MAX`,
/// `i64::MAX + 1` (a u64), and `u64::MAX`. Each pairs the literal spelling with
/// an off-by-one neighbor for the negative validation case.
const LARGE_INT_CASES: &[(&str, &str)] = &[
    ("9007199254740993", "9007199254740992"),
    ("9223372036854775807", "9223372036854775806"),
    ("9223372036854775808", "9223372036854775807"),
    ("18446744073709551615", "18446744073709551614"),
];

#[test]
fn literal_large_integer_round_trips_through_serialization() {
    for (lit, _) in LARGE_INT_CASES {
        let input = format!("literal({lit})");
        let first = parse_type_expr("prop", &input).expect("parse");
        let text = serialize_property_atom(&first);
        let second = parse_type_expr("prop", &text).expect("reparse");
        assert_eq!(first, second, "round-trip mismatch for {input}");
    }
}

#[test]
fn literal_large_integer_const_is_exact() {
    for (lit, _) in LARGE_INT_CASES {
        let expected: Value = serde_json::from_str(lit).expect("literal is a JSON number");
        let schema = json_schema(&format!("version: literal({lit})\n"));
        assert_eq!(*optional_const(&schema, "version"), expected, "const for literal({lit})");
    }
}

#[test]
fn literal_large_integer_validates_exact_and_rejects_neighbor() {
    for (lit, neighbor) in LARGE_INT_CASES {
        let ok = validate(&format!(
            "---\n$schema:\n  version: literal({lit}; required)\nversion: {lit}\n---\nbody\n"
        ));
        assert!(ok.valid, "literal({lit}) must accept {lit}: {:?}", ok.problems);

        let bad = validate(&format!(
            "---\n$schema:\n  version: literal({lit}; required)\nversion: {neighbor}\n---\nbody\n"
        ));
        assert!(!bad.valid, "literal({lit}) must reject neighbor {neighbor}");
    }
}

#[test]
fn literal_large_integer_coerces_string_document_value() {
    for (lit, _) in LARGE_INT_CASES {
        let expected: Value = serde_json::from_str(lit).expect("literal is a JSON number");
        let value = compose_value(
            &format!("---\n$schema:\n  version: literal({lit})\nversion: '{lit}'\n---\nbody\n"),
            "version",
        );
        assert_eq!(value, expected, "string '{lit}' must coerce to the exact integer");
    }
}

#[test]
fn literal_large_integer_default_equality_is_exact() {
    for (lit, neighbor) in LARGE_INT_CASES {
        let matching: Markdown =
            format!("---\n$schema:\n  version: literal({lit}; default({lit}))\n---\nbody\n").into();
        assert!(
            DarkmatterSchemas::new().validate(&matching).is_ok(),
            "an equal large-integer default must load for literal({lit})"
        );

        let mismatch: Markdown =
            format!("---\n$schema:\n  version: literal({lit}; default({neighbor}))\n---\nbody\n")
                .into();
        assert!(
            DarkmatterSchemas::new().validate(&mismatch).is_err(),
            "an unequal large-integer default must fail to load for literal({lit})"
        );
    }
}

#[test]
fn trigger_matcher_large_integer_literal_is_exact() {
    // Exercise the PUBLIC trigger matcher path (`matches`) — the same
    // `literal(...)` discriminant equality used to layer content-triggered
    // schemas. A required literal gate must accept the exact large integer and
    // reject its f64-colliding off-by-one neighbor.
    use darkmatter::markdown::schemas::triggers::matches;
    use darkmatter::markdown::schemas::{Constraint, MatchExpr, PropertyAtom, SimplifiedType, TypeExpr};

    fn version_frontmatter(value: &Value) -> Value {
        let mut map = serde_json::Map::new();
        map.insert("version".to_string(), value.clone());
        Value::Object(map)
    }

    for (lit, neighbor) in LARGE_INT_CASES {
        let expected: Value = serde_json::from_str(lit).expect("literal is a JSON integer");
        let neighbor_value: Value =
            serde_json::from_str(neighbor).expect("neighbor is a JSON integer");

        let expr = MatchExpr::Property {
            name: "version".to_string(),
            atom: PropertyAtom {
                ty: TypeExpr::Primitive(SimplifiedType::Literal),
                is_array: false,
                constraints: vec![
                    Constraint::LiteralValue(expected.clone()),
                    Constraint::Required,
                ],
                array_constraints: vec![],
                description: None,
            },
        };

        assert!(
            matches(&expr, &version_frontmatter(&expected), ""),
            "trigger literal({lit}) must match {lit}"
        );
        assert!(
            !matches(&expr, &version_frontmatter(&neighbor_value), ""),
            "trigger literal({lit}) must reject f64-colliding neighbor {neighbor}"
        );
    }
}
