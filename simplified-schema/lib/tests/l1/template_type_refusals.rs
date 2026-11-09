//! Template-type resolution: what returned data may never do.
//!
//! The boundary is `PendingSchema::resolve` with a table-driven `SlotHost`:
//! a definition returned for `{{ t }}` may choose a type, but never cause
//! another expression to run (a nested or reachable active slot) and never
//! bypass the slot-pattern rules through a pattern it introduces, directly
//! or through its imports. Authored schema text keeps its own rules.
//!
//! Every test resolves a schema file, so the module needs the `file` feature.
#![cfg(feature = "file")]

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

use biscuit_file::FileResolutionContext;
use serde_json::{Value, json};
use simplified_schema::embedded::ShippedSources;
use simplified_schema::resolve::resolve_yaml_schema;
use simplified_schema::slots::{
    MAX_SLOT_PATTERN_BYTES, SchemaRead, SchemaReadOutcome, SlotAdmissionError, SlotFailure, SlotHost,
    SlotRequest, SlotResolutionError, SlotRunRequest, TemplateReads,
};
use simplified_schema::{Prepared, SchemaError, Slot, SlotRelated, SlotResultKind};

/// Allows every read a returned definition introduces.
struct AllowReads;

impl TemplateReads for AllowReads {
    fn admit(&self, _: &Slot, _: &SchemaRead<'_>) -> Result<(), SlotAdmissionError> {
        Ok(())
    }

    fn observe(&self, _: &Slot, _: &SchemaReadOutcome<'_>) {}
}

/// Answers each expression from a table and records what it evaluated.
struct TableHost {
    answers: Vec<(&'static str, Value)>,
    evaluated: RefCell<Vec<String>>,
}

impl TableHost {
    fn new(answers: Vec<(&'static str, Value)>) -> Self {
        Self { answers, evaluated: RefCell::default() }
    }

    fn t(value: Value) -> Self {
        Self::new(vec![("t", value)])
    }

    fn evaluated(&self, expression: &str) -> bool {
        self.evaluated.borrow().iter().any(|seen| seen == expression)
    }
}

impl SlotHost for TableHost {
    fn admit(&self, _: &Slot) -> Result<(), SlotAdmissionError> {
        Ok(())
    }

    fn resolve(&self, request: &SlotRequest<'_>) -> Result<Value, SlotFailure> {
        let expression = request.slot.expression.as_str();
        self.evaluated.borrow_mut().push(expression.to_string());
        let (_, value) = self
            .answers
            .iter()
            .find(|(key, _)| *key == expression)
            .unwrap_or_else(|| panic!("the host has no answer for `{expression}`"));
        Ok(value.clone())
    }

    fn template_reads(&self) -> Rc<dyn TemplateReads> {
        Rc::new(AllowReads)
    }
}

/// Writes `files` (relative path → text) into `dir`.
fn write_files(dir: &Path, files: &[(&str, &str)]) {
    for (name, text) in files {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create the fixture directory");
        }
        std::fs::write(path, text).expect("write the fixture file");
    }
}

/// Resolves `./schema.yaml` in `dir` through `host`. A schema with no slot
/// resolves without the host.
fn resolve(dir: &Path, host: &TableHost) -> Result<Value, SlotResolutionError> {
    let context = FileResolutionContext::new(dir);
    let reference: serde_yaml_ng::Value = serde_yaml_ng::from_str("./schema.yaml").expect("a YAML string");
    match resolve_yaml_schema(&reference, dir, &context, ShippedSources::NONE) {
        Ok(resolved) => Ok(resolved.json_schema),
        Err(SchemaError::UnresolvedSlots(unresolved)) => unresolved
            .pending
            .expect("a pending template")
            .resolve(host, &SlotRunRequest { context })
            .map(|resolved| resolved.json_schema),
        Err(other) => panic!("the schema itself should resolve or be a template: {other:?}"),
    }
}

/// Resolves a one-file schema whose `$schema` body is `body`.
fn resolve_schema(body: &str, host: &TableHost) -> Result<Value, SlotResolutionError> {
    let dir = tempfile::tempdir().expect("tempdir");
    write_files(dir.path(), &[("schema.yaml", &format!("$schema:\n{body}"))]);
    resolve(dir.path(), host)
}

/// The slot-result refusal of `outcome`, checked to belong to `{{ t }}` on
/// property `property`.
fn refusal(outcome: Result<Value, SlotResolutionError>, property: &str) -> (SlotResultKind, Vec<SlotRelated>) {
    let error = outcome.expect_err("the returned definition should be refused");
    let SlotResolutionError::Schema(schema_error) = error else {
        panic!("expected a schema error, got {error:?}");
    };
    match *schema_error {
        SchemaError::SlotResult { slot, kind, property: at, related } => {
            assert_eq!(slot.expression, "t");
            assert_eq!(at.as_deref(), Some(property));
            (kind, related)
        }
        other => panic!("expected SlotResult, got {other:?}"),
    }
}

fn pattern_reason(outcome: Result<Value, SlotResolutionError>, property: &str) -> String {
    match refusal(outcome, property) {
        (SlotResultKind::PatternRejected(reason), _) => reason,
        (other, _) => panic!("expected PatternRejected, got {other:?}"),
    }
}

/// The single related location of a `ReachableSlot` refusal.
fn reachable_file(outcome: Result<Value, SlotResolutionError>, property: &str) -> PathBuf {
    let (kind, related) = refusal(outcome, property);
    assert!(matches!(kind, SlotResultKind::ReachableSlot), "expected ReachableSlot, got {kind:?}");
    assert_eq!(related.len(), 1, "one related location: {related:?}");
    related[0].file.clone().expect("the related location names the slot's file")
}

fn same_file(actual: &Path, expected: &Path) -> bool {
    let canonical = |path: &Path| std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    canonical(actual) == canonical(expected)
}

/// The JSON Schema node of top-level property `name`, past the `null` arm an
/// optional property lowers with.
fn property<'a>(schema: &'a Value, name: &str) -> &'a Value {
    let node = &schema["properties"][name];
    node["anyOf"]
        .as_array()
        .and_then(|arms| arms.iter().find(|arm| arm["type"] != json!("null")))
        .unwrap_or(node)
}

// ── nested active slots ─────────────────────────────────────────────────

#[test]
fn a_returned_definition_holding_an_active_slot_is_a_nested_slot() {
    for returned in [json!("string(default({{ x }}))"), json!("{{ x }}"), json!("string | {{ x }}"), json!(["string", "{{ x }}"])] {
        let host = TableHost::new(vec![("t", returned.clone()), ("x", json!("number"))]);

        let (kind, _) = refusal(resolve_schema("  status: \"{{ t }}\"\n", &host), "status");

        assert!(matches!(kind, SlotResultKind::NestedSlot), "{returned}: expected NestedSlot, got {kind:?}");
        assert!(!host.evaluated("x"), "{returned}: the nested slot is never evaluated");
    }
}

#[test]
fn quoted_brace_text_in_a_returned_enum_member_or_default_stays_literal() {
    let host = TableHost::new(vec![
        ("t", json!("enum('{{ a }}', b)")),
        ("u", json!("string(default('{{ x }}'))")),
    ]);

    let schema = resolve_schema("  kind: \"{{ t }}\"\n  title: \"{{ u }}\"\n", &host).expect("literal text resolves");

    assert_eq!(schema["properties"]["title"]["default"], json!("{{ x }}"), "the default is the literal text: {schema}");
    assert!(!host.evaluated("a") && !host.evaluated("x"));
    let dir = tempfile::tempdir().expect("tempdir");
    let prepared =
        Prepared::new(Arc::new(schema.clone()), None, FileResolutionContext::new(dir.path())).expect("schema compiles");
    prepared.validate_strict(&json!({ "kind": "{{ a }}", "title": "{{ x }}" })).expect("the literal member is a value");
    assert!(prepared.validate_strict(&json!({ "kind": "c" })).is_err(), "the enum still closes: {schema}");
}

// ── slots reachable through imports ─────────────────────────────────────

#[test]
fn a_returned_import_of_a_slot_bearing_type_is_a_reachable_slot() {
    for types in ["kind: schema\ntypes:\n  T: \"{{ x }}\"\n", "kind: schema\ntypes:\n  T:\n    a: \"{{ x }}\"\n"] {
        let dir = tempfile::tempdir().expect("tempdir");
        write_files(dir.path(), &[("schema.yaml", "$schema:\n  status: \"{{ t }}\"\n"), ("types.yaml", types)]);
        let host = TableHost::new(vec![("t", json!("T@./types.yaml")), ("x", json!("number"))]);

        let file = reachable_file(resolve(dir.path(), &host), "status");

        assert!(same_file(&file, &dir.path().join("types.yaml")), "{types}: related {file:?}");
        assert!(!host.evaluated("x"), "{types}: the imported slot is never evaluated");
    }
}

#[test]
fn a_slot_reached_transitively_names_the_file_that_holds_it() {
    let dir = tempfile::tempdir().expect("tempdir");
    write_files(
        dir.path(),
        &[
            ("schema.yaml", "$schema:\n  status: \"{{ t }}\"\n"),
            ("a.yaml", "kind: schema\ntypes:\n  A:\n    inner: B@./b.yaml\n"),
            ("b.yaml", "kind: schema\ntypes:\n  B: \"{{ x }}\"\n"),
        ],
    );
    let host = TableHost::new(vec![("t", json!("A@./a.yaml")), ("x", json!("number"))]);

    let file = reachable_file(resolve(dir.path(), &host), "status");

    assert!(same_file(&file, &dir.path().join("b.yaml")), "related {file:?}");
    assert!(!host.evaluated("x"));
}

#[test]
fn an_unused_slot_elsewhere_in_an_imported_file_does_not_disqualify_it() {
    let dir = tempfile::tempdir().expect("tempdir");
    write_files(
        dir.path(),
        &[
            ("schema.yaml", "$schema:\n  status: \"{{ t }}\"\n"),
            ("types.yaml", "kind: schema\ntypes:\n  Used: string\n  Unused: \"{{ x }}\"\n"),
        ],
    );
    let host = TableHost::new(vec![("t", json!("Used@./types.yaml")), ("x", json!("number"))]);

    let schema = resolve(dir.path(), &host).expect("the used type holds no slot");

    assert!(schema.to_string().contains("\"string\""), "{schema}");
    assert!(!host.evaluated("x"));
}

#[test]
fn a_statically_authored_import_of_a_slot_bearing_type_resolves_as_a_template() {
    let dir = tempfile::tempdir().expect("tempdir");
    write_files(
        dir.path(),
        &[("schema.yaml", "$schema:\n  s: T@./types.yaml\n"), ("types.yaml", "kind: schema\ntypes:\n  T: \"{{ x }}\"\n")],
    );
    let host = TableHost::new(vec![("x", json!("number"))]);

    let schema = resolve(dir.path(), &host).expect("an authored import is an ordinary template");

    assert!(host.evaluated("x"), "the host evaluates the authored slot");
    assert!(schema.to_string().contains("\"number\""), "{schema}");
}

// ── patterns a returned definition introduces ───────────────────────────

/// A returned `string(pattern('…'))` around `pattern`, resolved on `code`.
/// A backslash escapes the next byte in a quoted argument, so the pattern's own
/// backslashes are doubled to reach the regex unchanged.
fn resolve_returned_pattern(pattern: &str) -> Result<Value, SlotResolutionError> {
    let quoted = pattern.replace('\\', "\\\\");
    resolve_schema("  code: \"{{ t }}\"\n", &TableHost::t(json!(format!("string(pattern('{quoted}'))"))))
}

#[test]
fn a_returned_pattern_at_the_byte_cap_is_accepted_and_one_past_it_refused() {
    let at_cap = "a".repeat(MAX_SLOT_PATTERN_BYTES);
    let schema = resolve_returned_pattern(&at_cap).expect("the cap is inclusive");
    assert_eq!(property(&schema, "code")["pattern"], json!(at_cap));

    let reason = pattern_reason(resolve_returned_pattern(&"a".repeat(MAX_SLOT_PATTERN_BYTES + 1)), "code");
    assert!(reason.contains("4096"), "{reason}");
}

#[test]
fn a_returned_pattern_with_lookaround_is_refused() {
    for pattern in ["(?=a)", "(?!a)", "(?<=a)b", "(?<!a)b"] {
        let reason = pattern_reason(resolve_returned_pattern(pattern), "code");
        assert!(reason.contains("lookaround"), "{pattern}: {reason}");
    }
}

#[test]
fn a_returned_pattern_with_a_backreference_is_refused() {
    for pattern in [r"(a)\1", r"(?<x>a)\k<x>"] {
        let reason = pattern_reason(resolve_returned_pattern(pattern), "code");
        assert!(reason.contains("backreference"), "{pattern}: {reason}");
    }
}

#[test]
fn a_returned_pattern_beyond_the_regex_size_limit_is_refused() {
    pattern_reason(resolve_returned_pattern(r"(?:\w{100}){100}"), "code");
}

#[test]
fn a_lookaround_pattern_reached_through_a_returned_import_is_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    let types = "kind: schema\ntypes:\n  Code: string(pattern('(?=a)'))\n";
    write_files(dir.path(), &[("schema.yaml", "$schema:\n  code: \"{{ t }}\"\n"), ("types.yaml", types)]);
    // The same file imported statically is an ordinary schema.
    write_files(dir.path(), &[("static.yaml", "$schema:\n  code: Code@./types.yaml\n")]);
    let static_ref: serde_yaml_ng::Value = serde_yaml_ng::from_str("./static.yaml").expect("a YAML string");
    resolve_yaml_schema(&static_ref, dir.path(), &FileResolutionContext::new(dir.path()), ShippedSources::NONE)
        .expect("an authored lookaround import resolves");

    let reason = pattern_reason(resolve(dir.path(), &TableHost::t(json!("Code@./types.yaml"))), "code");

    assert!(reason.contains("lookaround"), "{reason}");
}

#[test]
fn a_lookaround_pattern_key_in_a_returned_inline_object_is_refused() {
    let host = TableHost::t(json!("{ <pattern::(?=a)>: number }"));

    let reason = pattern_reason(resolve_schema("  dict: \"{{ t }}\"\n", &host), "dict");

    assert!(reason.contains("lookaround"), "{reason}");
}

// ── authored patterns keep their rules ──────────────────────────────────

#[test]
fn an_authored_lookaround_pattern_resolves_as_before() {
    let host = TableHost::new(Vec::new());

    let schema = resolve_schema("  code: string(pattern('(?=a)'))\n", &host).expect("an authored pattern resolves");

    assert_eq!(property(&schema, "code")["pattern"], json!("(?=a)"));
    assert!(host.evaluated.borrow().is_empty());
}

#[test]
fn a_lookaround_pattern_in_the_use_site_suffix_is_exempt() {
    let host = TableHost::t(json!("string"));

    let schema = resolve_schema("  code: \"{{ t }}(pattern('(?=a)'))\"\n", &host).expect("the suffix is authored text");

    assert_eq!(property(&schema, "code")["pattern"], json!("(?=a)"), "{schema}");
}
