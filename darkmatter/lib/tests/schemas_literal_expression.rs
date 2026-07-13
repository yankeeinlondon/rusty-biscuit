//! Phase-1 failing-test scaffold for the SimplifiedSchema `literal` and
//! `expression` types (feature `2026-07-12-literal-expression`).
//!
//! Every test here asserts behavior that does **not** exist yet, so each is
//! gated with `#[ignore = "pending Phase N: …"]`. That keeps the area
//! `just test` suite green while pinning the acceptance criteria as executable
//! intent. As each phase lands the corresponding type behavior, lift the
//! `#[ignore]` (or migrate the assertion into its permanent home listed in
//! `features/2026-07-12-literal-expression/phase1-test-matrix.md`) and confirm
//! it passes.
//!
//! To confirm a scaffold "fails for the intended missing behavior rather than a
//! harness error", run a single case with `--ignored`, e.g.:
//!
//! ```text
//! cargo nextest run -p darkmatter --test schemas_literal_expression \
//!     literal_bare_string_parses --run-ignored all
//! ```
//!
//! All entry points below are compile-safe against the **current** public API:
//! `literal` / `expression` currently lex as unknown type keywords, so the
//! grammar helpers return `Err`, validation of an inline `$schema` fails to
//! compile the schema, and the compose write-back never runs — exactly the
//! missing behavior these scaffolds document.

use darkmatter::markdown::Markdown;
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
#[ignore = "pending Phase 2: literal grammar (number typing)"]
fn literal_bare_number_typed() {
    assert!(atom("literal(2)").is_ok());
    // Number-typed const in the compiled schema (AC 1 / AC 3).
    let schema = json_schema("version: literal(2)\n");
    let constv = &schema["properties"]["version"]["const"];
    assert!(constv.is_number(), "literal(2) const must be a JSON number, got {constv}");
}

#[test]
#[ignore = "pending Phase 2: literal grammar (boolean typing)"]
fn literal_bare_boolean_typed() {
    assert!(atom("literal(false)").is_ok());
    let schema = json_schema("archived: literal(false)\n");
    assert_eq!(schema["properties"]["archived"]["const"], Value::Bool(false));
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
#[ignore = "pending Phase 2: literal grammar (numberlike boundary)"]
fn literal_leading_zero_is_string_not_number() {
    // `007` fails the numberlike shape test (leading-zero) → typed as string.
    let schema = json_schema("code: literal(007)\n");
    assert_eq!(schema["properties"]["code"]["const"], Value::String("007".into()));
}

#[test]
#[ignore = "pending Phase 2 / Q5: literal array suffix"]
fn literal_array_suffix_allowed() {
    assert!(atom("literal(auto)[]").is_ok());
    let schema = json_schema("modes: literal(auto)[]\n");
    // `const` lives under `items` for the array form.
    assert!(
        schema["properties"]["modes"]["items"]["const"].is_string(),
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
#[ignore = "pending Phase 3: literal required equality"]
fn literal_required_enforces_equality() {
    let ok = validate("---\n$schema:\n  kind: literal(spec; required)\nkind: spec\n---\nbody\n");
    assert!(ok.valid, "matching required literal must validate: {:?}", ok.problems);

    let bad = validate("---\n$schema:\n  kind: literal(spec; required)\nkind: other\n---\nbody\n");
    assert!(!bad.valid, "wrong literal value must fail");
}

#[test]
#[ignore = "pending Phase 3: literal optional nullability"]
fn literal_optional_accepts_missing_and_null() {
    let missing = validate("---\n$schema:\n  kind: literal(spec)\n---\nbody\n");
    assert!(missing.valid, "optional literal accepts missing: {:?}", missing.problems);

    let null = validate("---\n$schema:\n  kind: literal(spec)\nkind: null\n---\nbody\n");
    assert!(null.valid, "optional literal accepts null: {:?}", null.problems);
}

#[test]
#[ignore = "pending Phase 3: literal default must equal value"]
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
#[ignore = "pending Phase 3: literal rejects suggest"]
fn literal_suggest_rejected() {
    let err = atom("literal(spec; suggest(a, b))").expect_err("suggest not allowed on literal");
    assert!(err.to_lowercase().contains("suggest"), "got: {err}");
}

// ══════════════════════════════════════════════════════════════════════════
// Feature A — literal coercion (Phase 4)  [AC 3]
// ══════════════════════════════════════════════════════════════════════════

#[test]
#[ignore = "pending Phase 4: literal typed coercion write-back"]
fn literal_number_coerces_string_document_value() {
    // `version: '2'` (string) against literal(2) coerces to number 2.
    let value = compose_value(
        "---\n$schema:\n  version: literal(2)\nversion: '2'\n---\nbody\n",
        "version",
    );
    assert_eq!(value, Value::from(2), "string '2' must coerce to number 2");
}

#[test]
#[ignore = "pending Phase 4: string literal never coerces"]
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
#[ignore = "pending Phase 4: literal in property union"]
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
#[ignore = "pending Phase 4: discriminated-union narrowing"]
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
#[ignore = "pending Phase 4: type-sensitive discriminant"]
fn union_discriminant_is_type_sensitive() {
    // Typed `2` must NOT select an arm tagged with the string `'2'`.
    let doc = "---\n$schema:\n  v:\n    - '{ tag: literal(2), a: string(required) }'\n    - '{ tag: literal(other), b: string(required) }'\nv:\n  tag: '2'\n---\nbody\n";
    let report = validate(doc);
    // String '2' does not match literal(2) (number) → no narrowing to arm 0.
    assert!(!report.valid, "string '2' should not satisfy number literal(2) arm");
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
#[ignore = "pending Phase 3: expression → string + format"]
fn expression_compiles_to_string_with_format() {
    let schema = json_schema("when: expression\n");
    assert_eq!(schema["properties"]["when"]["type"], "string");
    assert_eq!(schema["properties"]["when"]["format"], "darkmatter-expression");
}

#[test]
#[ignore = "pending Phase 3: expression either-dialect superset"]
fn expression_accepts_either_dialect() {
    // Condition-dialect `&&` must be accepted by bare `expression` (Q2).
    let report = validate(
        "---\n$schema:\n  when: expression\nwhen: 'is_agent() && os == \"macos\"'\n---\nbody\n",
    );
    assert!(report.valid, "either-dialect expression must validate: {:?}", report.problems);
}

#[test]
#[ignore = "pending Phase 3: expression rejects unparseable"]
fn expression_rejects_unparseable_with_format_problem() {
    let report = validate("---\n$schema:\n  when: expression\nwhen: '(('\n---\nbody\n");
    assert!(!report.valid, "unparseable expression must fail");
    let has_format = report
        .problems
        .iter()
        .any(|p| format!("{p:?}").contains("darkmatter-expression"));
    assert!(has_format, "problem should carry the format name, got {:?}", report.problems);
}

#[test]
#[ignore = "pending Phase 3: unknown identifier is not a schema error"]
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
#[ignore = "pending Phase 4: expression native boolean coercion"]
fn expression_native_boolean_coerces_to_string() {
    let value = compose_value(
        "---\n$schema:\n  when: expression\nwhen: true\n---\nbody\n",
        "when",
    );
    assert_eq!(value, Value::String("true".into()));
}

#[test]
#[ignore = "pending Phase 4: expression native number coercion"]
fn expression_native_number_coerces_to_string() {
    let value = compose_value(
        "---\n$schema:\n  retries: expression\nretries: 3\n---\nbody\n",
        "retries",
    );
    assert_eq!(value, Value::String("3".into()));
}

#[test]
#[ignore = "pending Phase 4: expression mapping is a type mismatch"]
fn expression_mapping_is_type_mismatch() {
    let report = validate("---\n$schema:\n  when: expression\nwhen:\n  a: 1\n---\nbody\n");
    assert!(!report.valid, "mapping against expression must be a type mismatch");
}

#[test]
#[ignore = "pending Phase 4: expression sequence is a type mismatch"]
fn expression_sequence_is_type_mismatch() {
    let report = validate("---\n$schema:\n  when: expression\nwhen:\n  - a\n  - b\n---\nbody\n");
    assert!(!report.valid, "sequence against expression must be a type mismatch");
}

// ══════════════════════════════════════════════════════════════════════════
// Feature B — pending-value deferral (Phase 3/4)  [AC 5]
// ══════════════════════════════════════════════════════════════════════════

#[test]
#[ignore = "pending Phase 3/4: expression pending shell value deferred"]
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
