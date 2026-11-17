//! Null and empty envelope carriers judged the same way on every load path.
//!
//! A null optional field (`description`, `categories`, `types`, a tagged
//! document's `$schema`) is absent; an empty one (`""`, `[]`, `{}`) is a
//! written value without content and is rejected where the definition says
//! `not-empty`. Each cell runs through the direct parser in both preparation
//! modes (the process's loaded definitions and a generation's freshly
//! prepared ones), envelope admission in both modes, the content and file
//! factories, and whole-YAML loading against the shipped kind definitions;
//! every path must reach the same verdict.

use std::path::Path;
use std::sync::{Arc, OnceLock};

use simplified_schema::embedded::ShippedSources;
use simplified_schema::envelope::{admit_envelope_text, admit_envelope_text_with_mode};
use simplified_schema::shipped::{FreshLower, PreparationMode};
use simplified_schema::triggers::parse_trigger_envelope_from_str;
use simplified_schema::yaml_documents::{
    KindRegistry, LoadedYamlDocument, Selection, ShippedDefinition, prepare_shipped_definitions, select,
};
use simplified_schema::{
    Prepared, SchemaError, SimplifiedSchema, ValidationProblem, parse_standalone_schema_document,
    parse_standalone_schema_document_with_mode,
};

const TAGGED: &str = "kind: schema\ndescription: Reviews\ncategories: [Core]\n$schema:\n  title: string\ntypes:\n  Id: string\n";
const YAML_SCHEMA: &str =
    "kind: yaml-schema\ndescription: Reviews\ncategories: [Core]\n$schema:\n  kind: literal(review; required)\ntypes:\n  Id: string\n";
const INFORMAL: &str = "description: Reviews\ncategories: [Core]\n$schema:\n  title: string\ntypes:\n  Id: string\n";

const TAGGED_EXPORT: &str = "$schema:\n  title: string\n";
const YAML_SCHEMA_EXPORT: &str = "$schema:\n  kind: literal(review; required)\n";
const TYPES: &str = "types:\n  Id: string\n";

/// What every load path must conclude about one envelope.
#[derive(Debug, Clone, Copy)]
enum Verdict {
    /// Admitted, with a usable `$schema` export.
    Exports,
    /// Admitted as a `types`-only document.
    TypesOnly,
    /// Rejected at the authored token `at`, naming `pointer`.
    Rejected { pointer: &'static str, at: &'static str },
    /// No envelope claims the text: it is ordinary YAML.
    Unclaimed,
}

fn fresh() -> &'static FreshLower {
    static FRESH: OnceLock<FreshLower> = OnceLock::new();
    FRESH.get_or_init(|| FreshLower::prepare().expect("the compiled envelope definitions generate"))
}

fn shipped() -> &'static [ShippedDefinition] {
    static SHIPPED: OnceLock<Vec<ShippedDefinition>> = OnceLock::new();
    SHIPPED.get_or_init(|| prepare_shipped_definitions(ShippedSources::NONE))
}

/// `base` with `from` replaced once by `to`; panics when `from` is absent so
/// a stale fixture edit cannot pass silently.
#[track_caller]
fn edit(base: &str, from: &str, to: &str) -> String {
    assert!(base.contains(from), "`{from}` is not in {base:?}");
    base.replacen(from, to, 1)
}

/// The cells: one control edit each.
fn cells() -> Vec<(&'static str, String, Verdict)> {
    use Verdict::*;
    let rejected = |pointer, at| Rejected { pointer, at };
    vec![
        // Tagged `kind: schema`.
        ("tagged control", TAGGED.to_string(), Exports),
        ("tagged description: null", edit(TAGGED, "description: Reviews", "description: null"), Exports),
        ("tagged categories: null", edit(TAGGED, "categories: [Core]", "categories: null"), Exports),
        ("tagged types: null", edit(TAGGED, TYPES, "types: null\n"), Exports),
        ("tagged $schema: null beside types", edit(TAGGED, TAGGED_EXPORT, "$schema: null\n"), TypesOnly),
        ("tagged bare $schema: beside types", edit(TAGGED, TAGGED_EXPORT, "$schema:\n"), TypesOnly),
        (
            "tagged $schema: null and types: null",
            edit(&edit(TAGGED, TAGGED_EXPORT, "$schema: null\n"), TYPES, "types: null\n"),
            rejected("`/$schema`", "null"),
        ),
        (
            "tagged $schema: null without types",
            edit(&edit(TAGGED, TAGGED_EXPORT, "$schema: null\n"), TYPES, ""),
            rejected("`/types`", "kind: schema"),
        ),
        (
            "tagged $schema: null beside types: {}",
            edit(&edit(TAGGED, TAGGED_EXPORT, "$schema: null\n"), TYPES, "types: {}\n"),
            rejected("`/types`", "{}"),
        ),
        ("tagged types: {} beside an export", edit(TAGGED, TYPES, "types: {}\n"), rejected("`/types`", "{}")),
        ("tagged types-only types: {}", edit(TAGGED, TAGGED_EXPORT, "").replacen(TYPES, "types: {}\n", 1), rejected("`/types`", "{}")),
        ("tagged types: [Id]", edit(TAGGED, TYPES, "types: [Id]\n"), rejected("`/types`", "[Id]")),
        ("tagged description: \"\"", edit(TAGGED, "description: Reviews", "description: \"\""), rejected("`/description`", "\"\"")),
        ("tagged categories: []", edit(TAGGED, "categories: [Core]", "categories: []"), rejected("`/categories`", "[]")),
        // Wrong carriers and members. Scalar members keep the soft text
        // conversion metadata has always had (see
        // `scalar_metadata_is_converted_to_text`); a null or a mapping member
        // is refused at itself.
        ("tagged categories: Core", edit(TAGGED, "categories: [Core]", "categories: Core"), rejected("`/categories`", "Core")),
        ("tagged categories: {Core: x}", edit(TAGGED, "categories: [Core]", "categories: {Core: x}"), rejected("`/categories`", "{Core: x}")),
        ("tagged categories: [Core, 5]", edit(TAGGED, "categories: [Core]", "categories: [Core, 5]"), Exports),
        ("tagged categories: [5, true]", edit(TAGGED, "categories: [Core]", "categories: [5, true]"), Exports),
        ("tagged categories: [Core, null]", edit(TAGGED, "categories: [Core]", "categories: [Core, null]"), rejected("`/categories/1`", "null")),
        ("tagged categories: [Core, {a: 1}]", edit(TAGGED, "categories: [Core]", "categories: [Core, {a: 1}]"), rejected("`/categories/1`", "{a: 1}")),
        ("tagged categories: [{a: 1}, null]", edit(TAGGED, "categories: [Core]", "categories: [{a: 1}, null]"), rejected("`/categories/0`", "{a: 1}")),
        ("tagged description: 5", edit(TAGGED, "description: Reviews", "description: 5"), Exports),
        ("tagged description: [a]", edit(TAGGED, "description: Reviews", "description: [a]"), rejected("`/description`", "[a]")),
        ("tagged types: Id", edit(TAGGED, TYPES, "types: Id\n"), rejected("`/types`", "Id")),
        // `$schema` is the required export of its arm, so an empty mapping is
        // an (open) export rather than an empty optional carrier.
        ("tagged $schema: {}", edit(TAGGED, TAGGED_EXPORT, "$schema: {}\n"), Exports),
        ("tagged $schema: []", edit(TAGGED, TAGGED_EXPORT, "$schema: []\n"), rejected("`/$schema`", "[]")),
        ("tagged $schema: 5", edit(TAGGED, TAGGED_EXPORT, "$schema: 5\n"), rejected("`/$schema`", "5")),
        (
            "tagged null-typed payload property",
            edit(TAGGED, "  title: string\n", "  title: string\n  n: null\n"),
            Exports,
        ),
        // `kind: yaml-schema` requires its export.
        ("yaml-schema control", YAML_SCHEMA.to_string(), Exports),
        ("yaml-schema types: null", edit(YAML_SCHEMA, TYPES, "types: null\n"), Exports),
        ("yaml-schema description: null", edit(YAML_SCHEMA, "description: Reviews", "description: null"), Exports),
        ("yaml-schema $schema: null", edit(YAML_SCHEMA, YAML_SCHEMA_EXPORT, "$schema: null\n"), rejected("`/$schema`", "null")),
        ("yaml-schema $schema: {}", edit(YAML_SCHEMA, YAML_SCHEMA_EXPORT, "$schema: {}\n"), rejected("`/$schema`", "{}")),
        ("yaml-schema types: {}", edit(YAML_SCHEMA, TYPES, "types: {}\n"), rejected("`/types`", "{}")),
        // The informal envelope is claimed only by a usable `$schema`.
        ("informal control", INFORMAL.to_string(), Exports),
        ("informal types: null", edit(INFORMAL, TYPES, "types: null\n"), Exports),
        ("informal $schema: null", edit(INFORMAL, TAGGED_EXPORT, "$schema: null\n"), Unclaimed),
        ("informal $schema: 5", edit(INFORMAL, TAGGED_EXPORT, "$schema: 5\n"), rejected("`/$schema`", "5")),
    ]
}

/// The direct parser's verdict, with the rejection's message and token.
fn parsed(result: Result<Option<simplified_schema::StandaloneSchemaDocument>, SchemaError>, text: &str) -> String {
    match result {
        Ok(None) => "unclaimed".into(),
        Ok(Some(document)) => format!("admitted exports={}", document.exports_schema),
        Err(SchemaError::SchemaDocument { message, span, .. }) => {
            format!("rejected {message} at {:?}", span.map(|span| &text[span]))
        }
        Err(other) => format!("other error {other}"),
    }
}

#[test]
fn every_direct_load_path_reaches_the_cell_s_verdict() {
    for (label, text, verdict) in cells() {
        let runtime = parsed(parse_standalone_schema_document(&text, "subject.yaml"), &text);
        let uncached = parsed(
            parse_standalone_schema_document_with_mode(&text, "subject.yaml", PreparationMode::Uncached(fresh())),
            &text,
        );
        assert_eq!(runtime, uncached, "{label}: the two preparation modes disagree");

        let admissions = [
            admit_envelope_text(&text, "subject.yaml").expect("YAML"),
            admit_envelope_text_with_mode(&text, "subject.yaml", PreparationMode::Uncached(fresh())).expect("YAML"),
        ];
        let factory = SimplifiedSchema::try_from(text.as_str());
        match verdict {
            Verdict::Exports | Verdict::TypesOnly => {
                let exports = matches!(verdict, Verdict::Exports);
                assert_eq!(runtime, format!("admitted exports={exports}"), "{label}");
                for admission in &admissions {
                    let admission = admission.as_ref().expect("recognized");
                    assert!(admission.is_admitted(), "{label}: {:#?}", admission.problems);
                }
                factory.unwrap_or_else(|error| panic!("{label}: {error}"));
            }
            Verdict::Rejected { pointer, at } => {
                assert!(runtime.starts_with("rejected "), "{label}: {runtime}");
                assert!(runtime.contains("does not satisfy its definition"), "{label}: {runtime}");
                assert!(runtime.contains(pointer), "{label}: expected `{pointer}` in {runtime}");
                assert!(runtime.ends_with(&format!("at Some({at:?})")), "{label}: expected the token {at:?}: {runtime}");
                for admission in &admissions {
                    let admission = admission.as_ref().expect("recognized");
                    assert!(!admission.is_admitted(), "{label}");
                    assert!(admission.problems.iter().all(|problem| problem.line.is_some()), "{label}: unlocated");
                }
                let message = factory.err().unwrap_or_else(|| panic!("{label}: the factory loaded it")).to_string();
                assert!(message.contains(pointer), "{label}: {message}");
            }
            Verdict::Unclaimed => {
                assert_eq!(runtime, "unclaimed", "{label}");
                assert!(admissions.iter().all(|admission| admission.is_none()), "{label}");
                let message = factory.err().unwrap_or_else(|| panic!("{label}: the factory loaded it")).to_string();
                assert!(message.contains("holds no schema"), "{label}: {message}");
            }
        }
    }
}

#[cfg(feature = "file")]
#[test]
fn the_file_factory_reaches_the_cell_s_verdict() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = biscuit_file::canonicalize_simplified(tmp.path()).unwrap();
    for (index, (label, text, verdict)) in cells().into_iter().enumerate() {
        let path = dir.join(format!("cell-{index}.yaml"));
        std::fs::write(&path, &text).unwrap();
        // A bound context: the factory captures nothing per cell.
        let source = simplified_schema::SchemaSource::from(path.as_path())
            .bound(biscuit_file::FileResolutionContext::new(&dir));
        let loaded = SimplifiedSchema::try_from_yaml(source);
        match verdict {
            Verdict::Exports | Verdict::TypesOnly => {
                loaded.unwrap_or_else(|error| panic!("{label}: {error}"));
            }
            Verdict::Rejected { pointer, .. } => {
                let message = loaded.err().unwrap_or_else(|| panic!("{label}: the factory loaded it")).to_string();
                assert!(message.contains(pointer), "{label}: {message}");
            }
            Verdict::Unclaimed => assert!(loaded.is_err(), "{label}: ordinary YAML is not a schema"),
        }
    }
}

/// Judges `text` as a whole YAML document the way a host does: load, select
/// by `kind`, validate softly against the claiming shipped definition, then
/// run the kind's checks on the accepted copy.
fn whole_yaml_problems(text: &str, dir: &Path) -> Vec<ValidationProblem> {
    let document = LoadedYamlDocument::load(text, &dir.join("subject.yaml")).expect("admissible YAML");
    let registry = KindRegistry::shipped_from(shipped());
    let context = biscuit_file::FileResolutionContext::new(dir);
    let Selection::Kind { kind, definition } = select(&registry, &document.loaded().value).expect("selected") else {
        panic!("{text:?} is not selected by its kind");
    };
    let prepared =
        Prepared::new(Arc::new(definition.resolved().json_schema.clone()), None, context.clone()).expect("prepared");
    match prepared.validate_soft(document.json()) {
        Ok(accepted) => document.kind_checks(&kind, &accepted.value, &[], &context, ShippedSources::NONE),
        Err(failure) => failure.into_problems(),
    }
}

#[test]
fn whole_yaml_loading_agrees_with_direct_loading() {
    let tmp = tempfile::tempdir().unwrap();
    for (label, text, verdict) in cells() {
        // An informal envelope is a document with a root `$schema`, not one
        // selected by its `kind`.
        if !text.starts_with("kind:") {
            continue;
        }
        let problems = whole_yaml_problems(&text, tmp.path());
        match verdict {
            Verdict::Exports | Verdict::TypesOnly => assert!(problems.is_empty(), "{label}: {problems:#?}"),
            Verdict::Rejected { .. } => {
                let direct = parse_standalone_schema_document(&text, "subject.yaml").unwrap_err().to_string();
                assert!(
                    problems.iter().any(|problem| direct.contains(&problem.message)),
                    "{label}: the direct load's problem ({direct}) is not among {problems:#?}"
                );
            }
            Verdict::Unclaimed => unreachable!("{label}: a tagged cell is always claimed"),
        }
    }
}

#[test]
fn a_tagged_null_export_is_the_types_only_schema_through_the_factory() {
    let null_export = edit(TAGGED, TAGGED_EXPORT, "$schema: null\n");
    let types_only = edit(TAGGED, TAGGED_EXPORT, "");
    let with_null = SimplifiedSchema::try_from(null_export.as_str()).unwrap();
    let without = SimplifiedSchema::try_from(types_only.as_str()).unwrap();
    assert_eq!(with_null.to_json_schema().unwrap(), without.to_json_schema().unwrap());
    assert!(with_null.named_type("Id").is_some());
    assert!(with_null.property("title").is_none(), "a null export declares no document property");
}

/// A scalar `description` or category member is read as its text, the
/// conversion every other string metadata value gets.
#[test]
fn scalar_metadata_is_converted_to_text() {
    let text = edit(&edit(TAGGED, "description: Reviews", "description: 5"), "categories: [Core]", "categories: [Core, 5, true]");
    let document = parse_standalone_schema_document(&text, "subject.yaml").unwrap().expect("claimed");
    assert_eq!(document.description.as_deref(), Some("5"));
    let names: Vec<&str> = document.categories.iter().flatten().map(|category| category.name.as_str()).collect();
    assert_eq!(names, ["Core", "5", "true"]);
}

// ── Load-level controls ───────────────────────────────────────────────────

/// Null handling leaves YAML admission alone: a duplicate key and an
/// inadmissible number still fail the load on every path, before any
/// envelope or definition is consulted.
#[test]
fn duplicate_keys_and_inadmissible_numbers_still_fail_the_load() {
    let duplicate = format!("{TAGGED}types:\n  Other: string\n");
    let number = edit(TAGGED, "description: Reviews", "description: .inf");
    for (label, text, expected) in [("duplicate", &duplicate, "duplicate entry"), ("number", &number, "not finite")] {
        let direct = parse_standalone_schema_document(text, "subject.yaml").unwrap_err().to_string();
        assert!(direct.contains(expected), "{label}: {direct}");
        let admitted = admit_envelope_text(text, "subject.yaml").unwrap_err().to_string();
        assert!(admitted.contains(expected), "{label}: {admitted}");
        let factory = SimplifiedSchema::try_from(text.as_str()).expect_err("refused").to_string();
        assert!(factory.contains(expected), "{label}: {factory}");
        match LoadedYamlDocument::load(text, Path::new("subject.yaml")) {
            Err(SchemaError::SchemaDocument { message, span, .. }) => {
                assert!(message.contains(expected), "{label}: {message}");
                let span = span.unwrap_or_else(|| panic!("{label}: the whole-YAML load locates its error"));
                let line = text[..span.start].matches('\n').count() + 1;
                // The parser places a duplicate key at the mapping holding
                // it (here the root), a number at the number.
                let expected_line = if label == "duplicate" { 1 } else { 2 };
                assert_eq!(line, expected_line, "{label}: {message} at {span:?}");
            }
            other => panic!("{label}: expected a located load error, got {other:?}"),
        }
    }
}

/// A rejected carrier keeps its schema-source span: the token is located in
/// the authored text whichever preparation mode admitted it.
#[test]
fn a_rejected_carrier_is_located_identically_in_both_modes() {
    let text = edit(TAGGED, TYPES, "types: {}\n");
    let spans = [
        parse_standalone_schema_document(&text, "subject.yaml"),
        parse_standalone_schema_document_with_mode(&text, "subject.yaml", PreparationMode::Uncached(fresh())),
    ]
    .map(|result| match result {
        Err(SchemaError::SchemaDocument { span: Some(span), .. }) => span,
        other => panic!("expected a located rejection, got {other:?}"),
    });
    assert_eq!(spans[0], spans[1]);
    assert_eq!(&text[spans[0].clone()], "{}");
    assert_eq!(text[..spans[0].start].matches('\n').count() + 1, 6, "`types: {{}}` is on line 6");
}

// ── Trigger envelopes ─────────────────────────────────────────────────────

const TRIGGER: &str = "kind: schema-trigger\nmatch:\n  title: string(required)\n$schema:\n  summary: string\n";
const TRIGGER_PAYLOAD: &str = "$schema:\n  summary: string\n";

#[test]
fn a_null_trigger_payload_is_matcher_only_on_both_paths() {
    let tmp = tempfile::tempdir().unwrap();
    let omitted = parse_trigger_envelope_from_str(&edit(TRIGGER, TRIGGER_PAYLOAD, "")).unwrap().unwrap();
    for spelling in ["$schema: null\n", "$schema:\n", "$schema: ~\n"] {
        let text = edit(TRIGGER, TRIGGER_PAYLOAD, spelling);
        let envelope = parse_trigger_envelope_from_str(&text).unwrap().expect("claimed");
        assert_eq!(envelope, omitted, "{spelling:?}");
        assert!(envelope.payload.is_none(), "{spelling:?}");
        let problems = whole_yaml_problems(&text, tmp.path());
        assert!(problems.is_empty(), "{spelling:?}: {problems:#?}");
    }
    // A null-typed property inside a payload is a declaration, not an absent payload.
    let text = edit(TRIGGER, "  summary: string\n", "  summary: string\n  retired: null\n");
    let envelope = parse_trigger_envelope_from_str(&text).unwrap().unwrap();
    let payload = envelope.payload.expect("a payload");
    assert!(payload.get("retired").is_some_and(serde_yaml_ng::Value::is_null), "{payload:?}");
    assert!(whole_yaml_problems(&text, tmp.path()).is_empty());
}

#[test]
fn an_empty_or_malformed_trigger_payload_is_rejected_on_both_paths() {
    let tmp = tempfile::tempdir().unwrap();
    for spelling in ["$schema: {}\n", "$schema: []\n", "$schema: ''\n", "$schema: '  '\n", "$schema: 5\n"] {
        let text = edit(TRIGGER, TRIGGER_PAYLOAD, spelling);
        match parse_trigger_envelope_from_str(&text) {
            Err(SchemaError::TriggerMatch { message, .. }) => assert!(message.starts_with("`$schema`:"), "{spelling:?}: {message}"),
            other => panic!("{spelling:?}: expected a `$schema` error, got {other:?}"),
        }
        let problems = whole_yaml_problems(&text, tmp.path());
        assert!(
            problems.iter().any(|problem| problem.instance_path.first() == Some("$schema")),
            "{spelling:?}: {problems:#?}"
        );
    }
}

#[test]
fn match_conditions_still_forbid_defaults_imports_and_generated_values() {
    let tmp = tempfile::tempdir().unwrap();
    for (condition, constraint) in [
        ("string(required; default(x))", "default"),
        ("string(required; generated)", "generated"),
        ("Id@./types.yaml", "imported type"),
    ] {
        let text = edit(TRIGGER, "string(required)", condition);
        match parse_trigger_envelope_from_str(&text) {
            Err(SchemaError::TriggerForbiddenConstraint { property, constraint: found }) => {
                assert_eq!(property, "title", "{condition}");
                assert!(found.starts_with(constraint), "{condition}: {found}");
            }
            other => panic!("{condition}: expected a forbidden constraint, got {other:?}"),
        }
        let problems = whole_yaml_problems(&text, tmp.path());
        assert!(
            problems.iter().any(|problem| problem.message.contains("forbidden constraint")),
            "{condition}: {problems:#?}"
        );
    }
}
