//! Which schema a whole YAML document is validated against, and how, through
//! `DarkmatterSchemas::validate_yaml_document`: a claimed `kind` wins over a
//! root `$schema`, the fallback `$schema` is stripped from the validation
//! copy only, no baseline or trigger layer applies, validation is soft and
//! never writes back, and `{{ … }}` / `$(…)` strings are data.
//!
//! The field shapes of the two selection inputs are walked once each here:
//! `kind` (absent, null, number, list, mapping, empty, case) and the
//! fallback `$schema` (null, number, boolean, empty, every valid form, and
//! the element shapes of a root union).

use darkmatter::markdown::schemas::{DarkmatterSchemas, Route, YamlOutcome, YamlValidation};
use simplified_schema::{SchemaError, ValidationProblemCode};

use crate::yaml_kind_registry::{Fixture, ROSTER_DEFINITION, ROSTER_DOCUMENT, write};

fn validated(result: Result<YamlOutcome, SchemaError>) -> YamlValidation {
    match result {
        Ok(YamlOutcome::Validated(validation)) => validation,
        other => panic!("expected a validated document, got {other:?}"),
    }
}

fn with_roster() -> Fixture {
    let fixture = Fixture::new();
    write(&fixture.repo().join("schemas/roster.yaml"), ROSTER_DEFINITION);
    fixture
}

// ── Route precedence ───────────────────────────────────────────────────────

#[test]
fn a_claimed_kind_wins_and_its_root_schema_is_an_ordinary_property() {
    let fixture = with_roster();
    let document = fixture.package().join("roster.yaml");
    // The roster definition is closed and does not declare `$schema`, so a
    // `$schema` beside a claimed kind is an unknown key, never a contract
    // (it names a file that does not exist and is never resolved).
    let text = format!("$schema: ./absent.schema.yaml\n{ROSTER_DOCUMENT}");
    let validation = validated(fixture.validate(&document, &text));
    assert_eq!(validation.route, Route::Kind("roster".into()));
    assert_eq!(validation.report.problems.len(), 1, "{:?}", validation.report.problems);
    let problem = &validation.report.problems[0];
    assert_eq!(problem.code, ValidationProblemCode::UnknownKey);
    assert_eq!(problem.offending_property.as_deref(), Some("$schema"));
    assert_eq!(problem.line, Some(1));
}

#[test]
fn the_global_catalog_is_validated_as_a_schema_document_not_against_its_globals() {
    let fixture = Fixture::new();
    let schemas = DarkmatterSchemas::new(fixture.context(&fixture.package().join("x.yaml"), &[]));
    let catalog = include_str!("../../../schemas/darkmatter.yaml");
    let validation = validated(schemas.validate_yaml_document(catalog, std::path::Path::new("darkmatter/schemas/darkmatter.yaml")));
    assert_eq!(validation.route, Route::Kind("schema".into()));
    assert_eq!(validation.definition, "built-in simplified-schema/schemas/schema.yaml");
}

#[test]
fn a_trigger_is_validated_by_its_definition_never_against_its_payload() {
    let fixture = Fixture::new();
    // The payload requires a numeric `title`; the trigger has none.
    let text = "kind: schema-trigger\nmatch:\n    tag: string(required)\n$schema:\n    title: number(required)\n";
    let validation = validated(fixture.validate(&fixture.package().join("t.yaml"), text));
    assert_eq!(validation.route, Route::Kind("schema-trigger".into()));
    assert!(validation.report.valid, "{:?}", validation.report.problems);
}

#[test]
fn an_unclaimed_kind_falls_back_to_the_root_schema_or_to_no_schema() {
    let fixture = with_roster();
    let document = fixture.package().join("inv.yaml");
    assert!(matches!(fixture.validate(&document, "kind: inventory\nitems: 3\n"), Ok(YamlOutcome::NoSchema)));
    let text = "kind: inventory\n$schema:\n    kind: string\n    items: number(required)\nitems: three\n";
    let validation = validated(fixture.validate(&document, text));
    assert_eq!(validation.route, Route::RootSchema);
    assert_eq!(validation.report.problems[0].code, ValidationProblemCode::TypeMismatch);
}

#[test]
fn the_fallback_schema_is_stripped_from_the_validation_copy_only() {
    let fixture = Fixture::new();
    let document = fixture.package().join("doc.yaml");
    let text = "$schema:\n    name: string(required)\n    $constraints: { closed: true }\nname: api\n";
    let validation = validated(fixture.validate(&document, text));
    assert!(validation.report.valid, "a closed contract never sees `$schema`: {:?}", validation.report.problems);
    assert_eq!(std::fs::read_to_string(&document).unwrap(), text, "the document is untouched");

    // Positions still count the stripped line: `extra` is on line 5.
    let text = format!("{text}extra: 1\n");
    let validation = validated(fixture.validate(&document, &text));
    let problem = &validation.report.problems[0];
    assert_eq!((problem.code, problem.line), (ValidationProblemCode::UnknownKey, Some(5)));
}

// ── No Markdown layers ─────────────────────────────────────────────────────

#[test]
fn no_baseline_or_trigger_layer_applies_to_a_yaml_document() {
    let fixture = Fixture::new();
    // A trigger in the tree root that would type `title` for any document
    // with a `title`, and the built-in baseline's string `title`.
    write(
        &fixture.repo().join("schemas/titled.yaml"),
        "kind: schema-trigger\nmatch:\n    title: unknown(required)\n$schema:\n    title: boolean(required)\n",
    );
    let document = fixture.package().join("doc.yaml");
    let text = "$schema:\n    title: number\n    content_policy: string\ntitle: 5\ncontent_policy: none\n";
    write(&document, text);
    let schemas = DarkmatterSchemas::new(fixture.context(&document, &[]))
        .with_darkmatter_baseline_json_schema()
        .expect("baseline")
        .with_trigger_discovery()
        .expect("triggers");
    let validation = validated(schemas.validate_yaml_document(text, &document));
    assert!(validation.report.valid, "{:?}", validation.report.problems);
    assert!(validation.report.trigger_findings.is_empty());
}

// ── Soft conversion and passive values ─────────────────────────────────────

#[test]
fn validation_is_soft_and_writes_nothing_back() {
    let fixture = Fixture::new();
    let document = fixture.package().join("doc.yaml");
    let schema = "$schema:\n    port: number(integer)\n";
    let text = format!("{schema}port: \"8080\"\n");
    assert!(validated(fixture.validate(&document, &text)).report.valid);
    assert_eq!(std::fs::read_to_string(&document).unwrap(), text);
    let validation = validated(fixture.validate(&document, &format!("{schema}port: \"2.5\"\n")));
    assert!(!validation.report.valid);
    assert_eq!(validation.report.problems[0].line, Some(3));
}

#[test]
fn template_and_shell_strings_are_data() {
    let fixture = Fixture::new();
    let document = fixture.package().join("doc.yaml");
    for value in ["\"{{ 1 }}\"", "\"$(echo 1)\""] {
        let numeric = format!("$schema:\n    count: number\ncount: {value}\n");
        let validation = validated(fixture.validate(&document, &numeric));
        assert!(!validation.report.valid, "{value} in a number property");
        assert!(validation.report.pending.is_empty() && validation.report.deferred.is_empty());
        let text = format!("$schema:\n    note: string\nnote: {value}\n");
        assert!(validated(fixture.validate(&document, &text)).report.valid, "{value} in a string property");
    }
    // Nested inside a list too: never deferred.
    let nested = "$schema:\n    counts: number[]\ncounts: [1, \"{{ fm.count }}\"]\n";
    assert!(!validated(fixture.validate(&document, nested)).report.valid);
}

// ── Field shapes: `kind` ───────────────────────────────────────────────────

#[test]
fn a_kind_that_is_not_a_string_claims_nothing() {
    let fixture = with_roster();
    // A schema root is no exception: its files are selected the same way.
    for document in [fixture.package().join("doc.yaml"), fixture.repo().join("schemas/doc.yaml")] {
        for kind in [
            "",
            "kind: null\n",
            "kind:\n",
            "kind: 3\n",
            "kind: true\n",
            "kind: [roster]\n",
            "kind: []\n",
            "kind: [roster, 1]\n",
            "kind: { name: roster }\n",
            "kind: {}\n",
            "kind: \"\"\n",
        ] {
            assert!(matches!(fixture.validate(&document, &format!("{kind}a: 1\n")), Ok(YamlOutcome::NoSchema)), "{kind:?}");
            let text = format!("{kind}$schema:\n    a: number(required)\na: x\n");
            let validation = validated(fixture.validate(&document, &text));
            assert_eq!(validation.route, Route::RootSchema, "{kind:?}");
            assert!(!validation.report.valid, "{kind:?}: the fallback schema applies");
            // An unresolvable fallback is its own failure, never a kind error.
            let result = fixture.validate(&document, &format!("{kind}$schema: ./missing.yaml\na: 1\n"));
            assert!(
                matches!(&result, Err(error) if !matches!(error, SchemaError::KindSelection { .. } | SchemaError::TriggerMatch { .. })),
                "{kind:?}: {result:?}"
            );
        }
    }
}

#[test]
fn a_claimed_kind_routes_to_its_definition_and_validates_softly() {
    let fixture = with_roster();
    let document = fixture.package().join("doc.yaml");
    let validation = validated(fixture.validate(&document, ROSTER_DOCUMENT));
    assert_eq!(validation.route, Route::Kind("roster".into()));
    assert!(validation.definition.ends_with("roster.yaml"), "{}", validation.definition);
    assert!(validation.report.valid);
    // A typo the closed definition rejects, at the key.
    let typo = ROSTER_DOCUMENT.replace("platforms:", "platfroms:");
    let problems = validated(fixture.validate(&document, &typo)).report.problems;
    let unknown = problems.iter().find(|problem| problem.code == ValidationProblemCode::UnknownKey).expect("unknown key");
    assert_eq!((unknown.offending_property.as_deref(), unknown.line), (Some("platfroms"), Some(3)));
}

// ── Field shapes: the fallback `$schema` ───────────────────────────────────

#[test]
fn a_present_but_invalid_fallback_schema_is_a_schema_error_never_absence() {
    let fixture = Fixture::new();
    let document = fixture.package().join("doc.yaml");
    for schema in ["null", "", "3", "true", "\"\"", "[123]", "[\"./a.yaml\", 123]"] {
        let text = format!("$schema: {schema}\na: 1\n");
        let result = fixture.validate(&document, &text);
        assert!(result.is_err(), "`$schema: {schema}` must be a schema error, got {result:?}");
        assert!(!matches!(result, Err(SchemaError::SchemaDocument { .. })), "{schema}: not a load error: {result:?}");
    }
}

/// A document's root fields are data of the `$schema` it selects, even when
/// named like a schema document's metadata: under `{field}: unknown`, every
/// shape is valid, as are the shapes the contract declares.
#[test]
fn metadata_named_fields_are_data_of_the_selected_contract() {
    let fixture = Fixture::new();
    let folder = fixture.package().join("data");
    write(&folder.join("contract.yaml"), "$schema:\n    description: string\n");
    let document = folder.join("doc.yaml");
    for field in ["description", "categories", "types"] {
        for shape in ["null", "123", "[ops, 123]", "[123]", "\"\"", "[]", "{}", "ops"] {
            let text = format!("$schema:\n    {field}: unknown\n{field}: {shape}\n");
            let validation = validated(fixture.validate(&document, &text));
            assert!(validation.report.valid, "{text}: {:?}", validation.report.problems);
        }
    }
    for text in [
        "$schema:\n    description: string\ndescription: null\n",
        "$schema:\n    description: string\n    owner: string\ndescription: null\nowner: ops\n",
        "$schema:\n    categories: string[]\ncategories: [ops, ops]\n",
        "$schema:\n    types: string[]\ntypes: [ops]\n",
        "$schema:\n    types: unknown\ntypes: { mode: production }\n",
        "$schema: ./contract.yaml\ndescription: null\n",
        "$schema:\n    - { description: string, kind: literal(a) }\n    - { description: string, kind: literal(b) }\ndescription: null\n",
    ] {
        let validation = validated(fixture.validate(&document, text));
        assert!(validation.report.valid, "{text}: {:?}", validation.report.problems);
    }
}

#[test]
fn every_valid_fallback_schema_form_is_a_contract() {
    let fixture = Fixture::new();
    let folder = fixture.package().join("data");
    write(&folder.join("item.schema.yaml"), "$schema:\n    a: number(required)\n");
    write(
        &folder.join("item.schema.json"),
        r#"{"$schema": "https://json-schema.org/draft/2020-12/schema", "type": "object", "required": ["a"], "properties": {"a": {"type": "number"}}}"#,
    );
    let document = folder.join("doc.yaml");
    for schema in [
        "\n    a: number(required)",
        "\n    - a: number(required)\n    - b: string(required)",
        " ./item.schema.yaml",
        " ./item.schema.json",
    ] {
        let passing = format!("$schema:{schema}\na: 1\n");
        let validation = validated(fixture.validate(&document, &passing));
        assert!(validation.report.valid, "{schema}: {:?}", validation.report.problems);
        let failing = format!("$schema:{schema}\na: x\n");
        assert!(!validated(fixture.validate(&document, &failing)).report.valid, "{schema}");
    }
}

#[test]
fn a_fallback_schema_naming_a_trigger_file_is_refused() {
    let fixture = Fixture::new();
    let folder = fixture.package().join("data");
    write(&folder.join("trigger.yaml"), "kind: schema-trigger\nmatch:\n    a: string(required)\n");
    let result = fixture.validate(&folder.join("doc.yaml"), "$schema: ./trigger.yaml\na: x\n");
    assert!(matches!(result, Err(SchemaError::TriggerSchemaReferenced { .. })), "{result:?}");
}

// ── Definition fields wired to the `non-null` validator ────────────────────

#[test]
fn null_metadata_and_payloads_fail_the_shipped_definitions_on_their_line() {
    let fixture = Fixture::new();
    let document = fixture.package().join("def.yaml");
    let base = "kind: yaml-schema\n$schema:\n    kind: literal(thing; required)\n";
    let control = validated(fixture.validate(&document, base));
    assert!(control.report.valid, "{:?}", control.report.problems);
    for field in ["description", "categories", "types"] {
        let validation = validated(fixture.validate(&document, &format!("{base}{field}: null\n")));
        let problem = &validation.report.problems[0];
        assert_eq!(problem.line, Some(4), "{field}: {problem:?}");
    }
    let validation = validated(fixture.validate(&document, "kind: yaml-schema\n$schema: null\n"));
    assert_eq!(validation.report.problems[0].line, Some(2), "{:?}", validation.report.problems);

    let trigger = "kind: schema-trigger\nmatch:\n    a: string(required)\n";
    assert!(validated(fixture.validate(&document, trigger)).report.valid);
    let validation = validated(fixture.validate(&document, &format!("{trigger}$schema: null\n")));
    assert_eq!(validation.report.problems[0].line, Some(4), "{:?}", validation.report.problems);
}

// ── Prepared contracts ─────────────────────────────────────────────────────

/// A contract prepared once judges a later text of the document with the
/// same declaration exactly as validating that text does, compiling
/// nothing (counted with `work-counters`), on the kind route and on a referenced fallback `$schema`; the
/// declaration key ignores data and follows `kind` and `$schema`.
#[test]
fn a_prepared_contract_judges_later_data_as_validation_does_without_compiling() {
    use darkmatter::markdown::schemas::LoadedYamlDocument;
    #[cfg(feature = "work-counters")]
    use darkmatter::work_counters::{Work, snapshot};

    let fixture = with_roster();
    write(&fixture.package().join("contract.yaml"), "$schema:\n    owner: string\n");
    let cases = [
        (fixture.package().join("roster.yaml"), ROSTER_DOCUMENT.to_string(), ("slack", "[slack]")),
        (fixture.package().join("service.yaml"), "$schema: ./contract.yaml\nowner: ops\n".to_string(), ("ops", "[ops]")),
    ];
    for (document, text, (data, edited_data)) in &cases {
        write(document, text);
        let schemas = DarkmatterSchemas::new(fixture.context(document, &[]));
        let loaded = LoadedYamlDocument::load(text, document).unwrap();
        let contract = schemas.prepare_yaml_contract(&loaded).unwrap().expect("a selected schema");
        assert!(!contract.checks_read_files());

        let edited_text = text.replace(data, edited_data);
        let edited = LoadedYamlDocument::load(&edited_text, document).unwrap();
        assert_eq!(edited.declaration_key(), loaded.declaration_key(), "data is not the declaration");
        #[cfg(feature = "work-counters")]
        let before = snapshot();
        let judged = contract.validate(&edited);
        #[cfg(feature = "work-counters")]
        {
            let work = snapshot().since(&before);
            assert_eq!(
                (work.get(Work::ValidatorCompile), work.get(Work::YamlContractPreparation)),
                (0, 0),
                "{:?}",
                work.nonzero()
            );
        }
        let direct = validated(schemas.validate_loaded_yaml_document(&edited));
        assert_eq!(judged.route, direct.route);
        assert_eq!(judged.definition, direct.definition);
        assert_eq!(format!("{:?}", judged.report), format!("{:?}", direct.report));
        assert_eq!(judged.report.problems[0].code, ValidationProblemCode::TypeMismatch, "{}", document.display());
    }

    let key = |text: &str| LoadedYamlDocument::load(text, &cases[1].0).unwrap().declaration_key();
    let base = key("$schema: ./contract.yaml\nowner: ops\n");
    assert_ne!(base, key("$schema: ./other.yaml\nowner: ops\n"), "a changed `$schema`");
    assert_ne!(base, key("kind: roster\n$schema: ./contract.yaml\nowner: ops\n"), "an added `kind`");
}

/// The kind checks of a `schema` document resolve its own imports, so its
/// contract reports that judging it reads files.
#[test]
fn a_schema_documents_contract_reads_files_to_judge_it() {
    use darkmatter::markdown::schemas::LoadedYamlDocument;

    let fixture = Fixture::new();
    let document = fixture.package().join("owners.yaml");
    let text = "kind: schema\n$schema:\n    owner: string\n";
    write(&document, text);
    let loaded = LoadedYamlDocument::load(text, &document).unwrap();
    assert!(loaded.kind_checks_read_files());
    let contract = DarkmatterSchemas::new(fixture.context(&document, &[])).prepare_yaml_contract(&loaded).unwrap().unwrap();
    assert!(contract.checks_read_files());
    assert!(contract.validate(&loaded).report.valid);
}
