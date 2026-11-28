//! The two callable catalog envelopes, `kind: expression-schema` and
//! `kind: mutation-schema`: their strict shape admission against
//! `expression-schema.yaml` and `mutation-schema.yaml`, their syntax-only
//! kind checks, and their privacy (never a `$schema` target, a `Name@file`
//! import source, or an expandable types library).
//!
//! Boundary: the crate's public API in-process (the standalone parser, the
//! envelope admission, the YAML document loader and its kind checks, the
//! resolver and the bundle scope). Each matrix row edits one control fixture
//! once. Default outcome for an invalid shape: a `SchemaDocument` error
//! located at the offending field. Exceptions: an omitted `description`, an
//! authored empty `description`, and a quoted `"null"` helper are valid.

use std::path::Path;

use simplified_schema::envelope::{
    ENVELOPE_DEFINITIONS, EXPRESSION_SCHEMA_DEFINITION_PATH, MUTATION_SCHEMA_DEFINITION_PATH, admit_envelope_text,
};
use simplified_schema::yaml_documents::{LoadedYamlDocument, duplicate_key, shipped_kind_definition_paths};
use simplified_schema::{
    NotSchemaReason, SchemaError, SchemaSourcePath, SchemaSpanKind, SimplifiedSchema, StandaloneSchemaEnvelope,
    parse_standalone_schema_document,
};

/// The specification's expression catalog, with a private helper.
const EXPRESSION: &str = "kind: expression-schema
description: Text helpers.
categories:
    - Text -> Transform text.
types:
    kebab_case: |-
        function(
            parameters(x: string | null);
            returns(string | null);
            category(Text);
        ) -> Converts text to kebab-case.
    Label: string(min(1))
";

/// The specification's mutation catalog.
const MUTATION: &str = "kind: mutation-schema
categories:
    - Files -> Update a Markdown file.
types:
    set_frontmatter: |-
        function(
            parameters(file: string, prop: string, value: unknown | null);
            returns(unknown | null);
            category(Files);
            fallible;
        ) -> Sets a frontmatter property and returns its prior value, or null.
";

/// `EXPRESSION` with its line starting `from` replaced by `to` (`to` empty
/// removes the line).
fn edited(from: &str, to: &str) -> String {
    let mut found = false;
    let lines: Vec<String> = EXPRESSION
        .lines()
        .filter_map(|line| {
            if !found && line.starts_with(from) {
                found = true;
                return (!to.is_empty()).then(|| to.to_string());
            }
            Some(line.to_string())
        })
        .collect();
    assert!(found, "the control has a line starting `{from}`");
    format!("{}\n", lines.join("\n"))
}

/// Asserts `text` is refused with a message naming `field`, located at a
/// span whose text starts with `at`.
#[track_caller]
fn assert_refused(label: &str, text: &str, field: &str, at: &str) {
    match parse_standalone_schema_document(text, "catalog.yaml") {
        Err(SchemaError::SchemaDocument { message, span, .. }) => {
            assert!(message.contains(field), "{label}: expected `{field}` in {message}");
            let span = span.unwrap_or_else(|| panic!("{label}: the refusal is located: {message}"));
            assert!(text[span.clone()].starts_with(at), "{label}: located at {:?}, expected {at:?}: {message}", &text[span]);
        }
        other => panic!("{label}: expected a located refusal, got {other:?}"),
    }
}

// ── Recognition and the shipped definitions ───────────────────────────────

#[test]
fn both_catalog_kinds_are_recognized_private_and_admitted_strictly() {
    for (text, envelope, definition, category) in [
        (EXPRESSION, StandaloneSchemaEnvelope::ExpressionSchema, EXPRESSION_SCHEMA_DEFINITION_PATH, "Text"),
        (MUTATION, StandaloneSchemaEnvelope::MutationSchema, MUTATION_SCHEMA_DEFINITION_PATH, "Files"),
    ] {
        let document = parse_standalone_schema_document(text, "catalog.yaml")
            .unwrap_or_else(|error| panic!("{definition}: {error}"))
            .expect("a catalog is recognized");
        assert_eq!(document.envelope, envelope);
        assert!(envelope.is_callable_catalog());
        assert!(!document.exports_schema, "a catalog never exports `$schema`");
        let categories = document.categories.as_ref().expect("categories are read");
        assert_eq!(categories[0].name, category);
        let names: Vec<&String> = document.named_types().expect("types").properties.keys().collect();
        assert!(!names.is_empty());

        let admission = admit_envelope_text(text, "catalog.yaml").expect("YAML").expect("recognized");
        assert!(admission.is_admitted(), "{:?}", admission.problems);
        assert_eq!(admission.definition, definition);
    }
    assert_eq!(StandaloneSchemaEnvelope::ExpressionSchema.kind(), Some("expression-schema"));
    assert_eq!(StandaloneSchemaEnvelope::MutationSchema.kind(), Some("mutation-schema"));
    assert_eq!(StandaloneSchemaEnvelope::ExpressionSchema.label(), "`kind: expression-schema`");
    for envelope in [StandaloneSchemaEnvelope::ExpressionSchema, StandaloneSchemaEnvelope::MutationSchema] {
        assert_eq!(envelope.allowed_keys(), ["kind", "description", "categories", "types"]);
        assert!(!envelope.admits_top_level_key("$schema") && !envelope.admits_top_level_key("name"));
        assert!(envelope.admits_top_level_key("categories") && envelope.admits_top_level_key("types"));
    }
    assert!(!StandaloneSchemaEnvelope::Tagged.is_callable_catalog());
}

#[test]
fn the_catalog_definitions_ship_as_kind_definitions() {
    let paths = shipped_kind_definition_paths();
    for definition in [EXPRESSION_SCHEMA_DEFINITION_PATH, MUTATION_SCHEMA_DEFINITION_PATH] {
        assert!(paths.iter().any(|path| path == Path::new(definition)), "{definition} seeds every kind registry");
        assert!(ENVELOPE_DEFINITIONS.iter().any(|document| document.path == definition));
    }
}

/// `allowed_keys()` is written beside the definitions; every tagged
/// envelope's list is exactly its definition's keys (every arm's, for a root
/// union), so the two cannot drift.
#[test]
fn allowed_keys_match_each_shipped_definition() {
    for (envelope, definition) in [
        (StandaloneSchemaEnvelope::Tagged, simplified_schema::envelope::SCHEMA_DEFINITION_PATH),
        (StandaloneSchemaEnvelope::YamlSchema, simplified_schema::envelope::YAML_SCHEMA_DEFINITION_PATH),
        (StandaloneSchemaEnvelope::NamespaceSchema, simplified_schema::envelope::NAMESPACE_SCHEMA_DEFINITION_PATH),
        (StandaloneSchemaEnvelope::ExpressionSchema, EXPRESSION_SCHEMA_DEFINITION_PATH),
        (StandaloneSchemaEnvelope::MutationSchema, MUTATION_SCHEMA_DEFINITION_PATH),
    ] {
        let text = ENVELOPE_DEFINITIONS.iter().find(|document| document.path == definition).expect("shipped").text;
        let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(text).expect("definition YAML");
        let arms = match &value["$schema"] {
            serde_yaml_ng::Value::Sequence(arms) => arms.clone(),
            arm => vec![arm.clone()],
        };
        let mut declared: Vec<String> = arms
            .iter()
            .flat_map(|arm| arm.as_mapping().expect("an object arm").keys().filter_map(|key| key.as_str().map(str::to_string)))
            .filter(|key| key != "$constraints")
            .collect();
        declared.sort();
        declared.dedup();
        let mut allowed: Vec<String> = envelope.allowed_keys().iter().map(|key| (*key).to_string()).collect();
        allowed.sort();
        assert_eq!(allowed, declared, "{definition}");
        assert!(allowed.iter().all(|key| envelope.admits_top_level_key(key)), "{definition}");
    }
}

// ── The authored-shape matrix (expression catalog) ─────────────────────────

#[test]
fn description_may_be_omitted_or_empty_but_not_null_or_another_type() {
    let omitted = parse_standalone_schema_document(&edited("description:", ""), "catalog.yaml")
        .expect("an omitted description is valid")
        .expect("recognized");
    assert_eq!(omitted.description, None);
    let empty = parse_standalone_schema_document(&edited("description:", "description: \"\""), "catalog.yaml")
        .expect("an authored empty description is valid")
        .expect("recognized");
    assert_eq!(empty.description.as_deref(), Some(""));

    assert_refused("null", &edited("description:", "description: null"), "description", "null");
    assert_refused("bare null", &edited("description:", "description:"), "description", "");
    // Strict admission: a scalar of another type is never converted.
    assert_refused("number", &edited("description:", "description: 123"), "description", "123");
    assert_refused("list", &edited("description:", "description: [a]"), "description", "[a]");
}

#[test]
fn a_tagged_schema_still_converts_its_metadata_softly() {
    let document = parse_standalone_schema_document("kind: schema\ndescription: 123\ntypes:\n  Id: string\n", "lib.yaml")
        .expect("soft admission is kept for other envelopes")
        .expect("recognized");
    assert_eq!(document.description.as_deref(), Some("123"));
}

#[test]
fn categories_are_required_nonempty_strings() {
    let without = EXPRESSION.replace("categories:\n    - Text -> Transform text.\n", "");
    match parse_standalone_schema_document(&without, "catalog.yaml") {
        Err(SchemaError::SchemaDocument { message, .. }) => assert!(message.contains("categories"), "{message}"),
        other => panic!("absent categories: {other:?}"),
    }
    let with = |value: &str| EXPRESSION.replace("categories:\n    - Text -> Transform text.\n", &format!("categories: {value}\n"));
    assert_refused("null", &with("null"), "categories", "null");
    assert_refused("scalar", &with("Text"), "categories", "Text");
    assert_refused("empty", &with("[]"), "categories", "[]");
    assert_refused("one bad element", &with("[Text, 5]"), "categories", "5");
    assert_refused("every element bad", &with("[5]"), "categories", "5");
}

/// The category entry grammar, once: trimmed case-sensitive labels and an
/// optional nonblank description after `->`.
#[test]
fn category_entries_follow_the_shared_entry_grammar() {
    let with = |entries: &str| {
        EXPRESSION.replace("categories:\n    - Text -> Transform text.\n", &format!("categories:\n{entries}"))
    };
    let trimmed = parse_standalone_schema_document(&with("    - \"  Text  ->  Transform.  \"\n"), "catalog.yaml")
        .expect("surrounding blanks are trimmed")
        .expect("recognized");
    let category = &trimmed.categories.expect("read")[0];
    assert_eq!((category.name.as_str(), category.description.as_deref()), ("Text", Some("Transform.")));

    for (entries, reason, failing_text, earlier_text) in [
        ("    - \" -> Transform.\"\n", "blank", "\" -> Transform.\"", None),
        ("    - \"Text ->\"\n", "must be followed by a description", "\"Text ->\"", None),
        ("    - Text\n    - Text -> Again.\n", "duplicate category `Text`", "Text -> Again.", Some("Text")),
        ("    - text\n", "category `Text` is not declared", "function(", None),
    ] {
        let text = with(entries);
        let error = parse_standalone_schema_document(&text, "catalog.yaml").unwrap_err();
        let SchemaError::CallableCatalog { message, span, related, .. } = error else { panic!("{error:?}") };
        assert!(message.contains(reason), "{message}");
        assert_eq!(span.unwrap().start, text.find(failing_text).unwrap(), "{message}");
        assert_eq!(related.len(), usize::from(earlier_text.is_some()), "{message}");
        if let Some(earlier) = earlier_text {
            assert_eq!(&text[related[0].span.clone().unwrap()], earlier);
        }
    }
}

#[test]
fn types_are_a_required_nonempty_mapping_without_null_entries() {
    let without = EXPRESSION.split("types:\n").next().expect("control").to_string();
    match parse_standalone_schema_document(&without, "catalog.yaml") {
        Err(SchemaError::SchemaDocument { message, .. }) => assert!(message.contains("types"), "{message}"),
        other => panic!("absent types: {other:?}"),
    }
    let with = |value: &str| format!("{without}types: {value}\n");
    assert_refused("null", &with("null"), "types", "null");
    assert_refused("list", &with("[a]"), "types", "[a]");
    assert_refused("empty", &with("{}"), "types", "{}");
    assert_refused("scalar", &with("string"), "types", "string");

    // A bare null entry declares nothing; the quoted spelling is the null type.
    assert_refused("null entry", &edited("    Label:", "    Label:"), "types.Label", "");
    let quoted = parse_standalone_schema_document(&edited("    Label:", "    Label: \"null\""), "catalog.yaml")
        .expect("a quoted null helper is a type")
        .expect("recognized");
    assert!(quoted.named_types().expect("types").properties.contains_key("Label"));

    // One grammar failure among valid entries is located inside its string.
    let text = edited("    Label:", "    Label: string(min(1)");
    match parse_standalone_schema_document(&text, "catalog.yaml") {
        Err(SchemaError::SchemaDocument { span: Some(span), .. }) => {
            let entry = text.find("string(min(1)").expect("edited");
            assert!(span.start >= entry && span.end <= text.len(), "located inside the rejected entry: {span:?}");
        }
        other => panic!("bad entry: {other:?}"),
    }
}

#[test]
fn type_carrier_spellings_and_union_arms_are_parsed_by_the_shared_grammar() {
    let sequence = edited(
        "    Label:",
        "    Label:\n        - string\n        - \"null\"",
    );
    parse_standalone_schema_document(&sequence, "catalog.yaml").expect("a sequence union").expect("recognized");
    let mapping = edited("    Label:", "    Label:\n        id: string(required)");
    parse_standalone_schema_document(&mapping, "catalog.yaml").expect("a helper mapping").expect("recognized");
    let mixed = edited("    Label:", "    Label:\n        - string\n        - string(min(1)");
    assert!(parse_standalone_schema_document(&mixed, "catalog.yaml").is_err(), "one invalid union arm");
    let invalid = edited("    Label:", "    Label:\n        - string(min(1)\n        - number(max(2)");
    assert!(parse_standalone_schema_document(&invalid, "catalog.yaml").is_err(), "every union arm invalid");
}

#[test]
fn unknown_keys_are_refused_including_schema_and_when() {
    for (key, line) in [("$schema", "$schema: {}"), ("when", "when: \"{{ true }}\""), ("name", "name: text-helpers")] {
        let text = format!("{EXPRESSION}{line}\n");
        match parse_standalone_schema_document(&text, "catalog.yaml") {
            Err(SchemaError::SchemaDocument { message, span, .. }) => {
                assert!(message.contains(key), "{key}: {message}");
                assert!(span.is_some(), "{key}: located: {message}");
            }
            other => panic!("{key}: {other:?}"),
        }
    }
}

#[test]
fn inline_examples_are_passive_data() {
    let text = edited(
        "    Label:",
        "    Label: string(example(\"{{ ctx.secret }}\"))",
    );
    parse_standalone_schema_document(&text, "catalog.yaml").expect("an inline example is data").expect("recognized");
}

#[test]
fn the_mutation_catalog_is_wired_to_the_same_validators() {
    parse_standalone_schema_document(MUTATION, "actions.yaml").expect("the control").expect("recognized");
    let text = MUTATION.replace("kind: mutation-schema\n", "kind: mutation-schema\ndescription: 7\n");
    match parse_standalone_schema_document(&text, "actions.yaml") {
        Err(SchemaError::SchemaDocument { message, .. }) => {
            assert!(message.contains(MUTATION_SCHEMA_DEFINITION_PATH), "{message}");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn duplicate_overloads_locate_both_arms_in_each_catalog_carrier() {
    let first = "function(parameters(x: string); returns(string); category(Text))";
    let second = |ty: &str| format!("function(parameters(y: {ty}); returns(string); category(Text))");
    for kind in ["expression-schema", "mutation-schema"] {
        for spelling in ["sequence", "pipe", "group", "grouped-sequence"] {
            let text = |ty: &str| {
                let second = second(ty);
                let arms = match spelling {
                    "sequence" => format!("\n        - '{first}'\n        - '{second}'"),
                    "pipe" => format!(" >-\n        {first} |\n        {second}"),
                    "group" => format!(" >-\n        ({first} |\n        {second})"),
                    "grouped-sequence" => format!("\n        - '({first})'\n        - '({second})'"),
                    _ => unreachable!(),
                };
                format!("kind: {kind}\ncategories: [Text]\ntypes:\n    Label: string\n    pad:{arms}\n")
            };
            let control = parse_standalone_schema_document(&text("number"), "catalog.yaml").unwrap();
            assert!(control.is_some(), "{kind}, {spelling}");
            for ty in ["string", "Label@this"] {
                let text = text(ty);
                let error = parse_standalone_schema_document(&text, "catalog.yaml").unwrap_err();
                let SchemaError::CallableCatalog { message, span, related, .. } = error else { panic!("{error:?}") };
                assert!(message.contains("same binding contract"), "{message}");
                let span = span.unwrap();
                let second = second(ty);
                let prefix = if spelling == "grouped-sequence" { "(" } else { "" };
                assert!(text[span].starts_with(&format!("{prefix}{second}")), "{kind}, {spelling}, {ty}");
                assert_eq!(related.len(), 1, "{message}");
                assert!(text[related[0].span.clone().unwrap()].starts_with(&format!("{prefix}{first}")), "{kind}, {spelling}, {ty}");
            }
        }
    }
}

// ── The YAML loader and the syntax-only kind checks ────────────────────────

#[test]
fn the_kind_checks_are_syntax_only_and_located() {
    let context = biscuit_file::FileResolutionContext::new(std::env::temp_dir()).without_home_dir();
    let check = |kind: &str, text: &str| {
        let document = LoadedYamlDocument::load(text, Path::new("catalog.yaml")).expect("loads");
        document.kind_checks(kind, document.json(), &[], &context, simplified_schema::embedded::ShippedSources::NONE)
    };
    assert!(check("expression-schema", EXPRESSION).is_empty());
    assert!(check("mutation-schema", MUTATION).is_empty());

    // `é` precedes the failure, so a byte offset is not a column.
    let text = edited("    Label:", "    Label: string -> Libellé\n    Bad: string(min(1)");
    let problems = check("expression-schema", &text);
    assert_eq!(problems.len(), 1, "{problems:?}");
    let line = text.lines().position(|line| line.contains("Bad: string(min(1)")).expect("edited") + 1;
    assert_eq!(problems[0].line, u32::try_from(line).ok(), "{problems:?}");

    // A host's soft definition may have converted the metadata; the check
    // still refuses what was written.
    let numbered = edited("description:", "description: 5");
    let problems = check("expression-schema", &numbered);
    assert!(problems.iter().any(|problem| problem.path == "/description"), "{problems:?}");
    let entry = EXPRESSION.replace("    - Text -> Transform text.\n", "    - Text -> Transform text.\n    - 7\n");
    let problems = check("expression-schema", &entry);
    assert!(problems.iter().any(|problem| problem.message.contains("categories[1]")), "{problems:?}");

    // An import is not resolved: the catalog's types are never expanded.
    let importing = edited("    Label:", "    Label: Owner@./absent.yaml");
    assert!(check("expression-schema", &importing).is_empty(), "nothing is read");
    assert!(!LoadedYamlDocument::load(&importing, Path::new("catalog.yaml")).expect("loads").kind_checks_read_files());
}

#[test]
fn a_duplicate_key_is_refused_at_the_repeat_and_names_the_first() {
    let text = "kind: expression-schema\ndescription: Résumé helpers.\ncategories: [Text]\ntypes:\n    Label: string\n    Label: number\n";
    let error = LoadedYamlDocument::load(text, Path::new("catalog.yaml")).expect_err("duplicate key");
    let SchemaError::SchemaDocument { message, span: Some(span), .. } = error else { panic!("located: {error:?}") };
    assert!(message.contains("duplicate entry") && message.contains("first written at line 5 column 5"), "{message}");
    assert!(text[span.start..].starts_with("Label: number"), "at the repeated key");

    let marks = duplicate_key(text).expect("the repeat is found");
    assert!(text[marks.first..].starts_with("Label: string"));
    assert!(text[marks.repeated..].starts_with("Label: number"));
    assert_eq!(marks.repeated, span.start, "the loader and the marks agree");

    // The parser places the failure at the mapping's start; the message names
    // only the first spelling's line, never that start.
    let later = "kind: expression-schema\ncategories: [Text]\ntypes:\n    A: string\n    Label: string\n    B: string\n    Label: number\n";
    let SchemaError::SchemaDocument { message, .. } =
        LoadedYamlDocument::load(later, Path::new("catalog.yaml")).expect_err("duplicate key")
    else {
        panic!("a document error");
    };
    assert!(message.ends_with("; the key is first written at line 5 column 5"), "{message}");
    assert!(!message.contains("line 4"), "the mapping start is not reported: {message}");

    // A byte-order mark shifts both offsets into the caller's text.
    let marked = format!("\u{feff}{text}");
    let marks = duplicate_key(&marked).expect("found after a byte-order mark");
    assert!(marked[marks.first..].starts_with("Label: string"));
    assert!(duplicate_key(EXPRESSION).is_none());
}

#[test]
fn the_loader_refuses_trailing_documents_and_accepts_comments_and_crlf() {
    let second = format!("{EXPRESSION}---\nkind: expression-schema\n");
    assert!(LoadedYamlDocument::load(&second, Path::new("catalog.yaml")).is_err(), "a second document");
    let garbage = format!("{EXPRESSION}]\n");
    assert!(LoadedYamlDocument::load(&garbage, Path::new("catalog.yaml")).is_err(), "trailing garbage");
    let commented = format!("# helpers\n{}", EXPRESSION.replace("\n", "\r\n"));
    parse_standalone_schema_document(&commented, "catalog.yaml").expect("comments and CRLF").expect("recognized");
    LoadedYamlDocument::load(&commented, Path::new("catalog.yaml")).expect("comments and CRLF load");
}

#[test]
fn inline_comments_preserve_catalog_declarations_in_each_source_presentation() {
    for (control, commented) in [
        ("    Label: string", "    Label: string # plain"),
        ("    Label:\n        id: string", "    Label: # nested mapping\n        id: string # leaf"),
        ("    Label: &label\n        id: string", "    Label: &label # anchored mapping\n        id: string"),
        ("    Label:\n        - string\n        - number", "    Label:\n        - string # sequence arm\n        - # value below\n            number"),
        ("    Label: string | number", "    Label: string | number # scalar union"),
        ("    Label: string |\n        number", "    Label: string |\n        number # continued scalar"),
        ("    Label: [string, number]", "    Label: [string # flow arm\n        , number] # sequence"),
        ("    Label: {id: string, detail: {value: number}}", "    Label: {id: # flow value\n        string, # before key\n        detail: {value: number} # nested flow mapping\n    }"),
        ("    ? Label\n    : string", "    ? Label # explicit key\n    : string # explicit value"),
        ("    Label: |-\n        string", "    Label: |- # block header\n        string"),
        ("    Label: 'string -> A # tag'", "    Label: 'string -> A # tag' # quoted"),
        ("    Label: \"string -> A # tag\"", "    Label: \"string -> A # tag\" # quoted"),
        ("    Label: string -> A#tag", "    Label: string -> A#tag # plain with hash content"),
        ("    Label: string -> A\u{a0}#tag", "    Label: string -> A\u{a0}#tag # non-separating space"),
    ] {
        let control = edited("    Label:", control);
        let text = edited("    Label:", commented);
        let plain = parse_standalone_schema_document(&control, "catalog.yaml").expect(&control).expect("catalog");
        let with_comment = parse_standalone_schema_document(&text, "catalog.yaml").expect(&text).expect("catalog");
        assert_eq!(with_comment.declaration, plain.declaration, "{text}");
        let key = &with_comment
            .named_type_source_map()
            .spans(&SchemaSourcePath::root().property("Label"), SchemaSpanKind::MappingKey)[0];
        assert_eq!(&text[key.clone()], "Label");
    }
}

#[test]
fn inline_comments_preserve_callable_spans_with_line_end_and_bom_variants() {
    let callable = "function(parameters(void); returns(string); category(Files); fallible)";
    let base = format!("kind: mutation-schema\ncategories:\n    - Files -> Update a file.\ntypes:\n    change_label: {callable}\n    Label: string(min(1))\n");
    let plain = parse_standalone_schema_document(&base, "catalog.yaml").unwrap().unwrap();
    let commented = base
        .replace(callable, &format!("{callable} # Run the action."))
        .replace("Update a file.", "Update a file. # Category")
        .replace("Label: string(min(1))", "Label: string(min(1)) # Keep a label.");
    let document = format!("--- # catalog\n{commented}... # end\n");
    for (mark, line_end) in [("", "\n"), ("", "\r\n"), ("\u{feff}", "\n"), ("\u{feff}", "\r\n")] {
        let text = format!("{mark}{}", document.replace('\n', line_end));
        let catalog = parse_standalone_schema_document(&text, "catalog.yaml").expect(&text).expect("catalog");
        assert_eq!(catalog.declaration, plain.declaration, "{text}");
        let category = &catalog.categories.as_ref().unwrap()[0];
        assert_eq!((category.name.as_str(), category.description.as_deref()), ("Files", Some("Update a file.")));
        let map = catalog.named_type_source_map();
        let callable_span = &map.spans(&SchemaSourcePath::root().property("change_label"), SchemaSpanKind::Definition)[0];
        assert_eq!(&text[callable_span.clone()], callable);
        let helper = SchemaSourcePath::root().property("Label");
        let helper_span = &map.spans(&helper, SchemaSpanKind::Definition)[0];
        assert_eq!(&text[helper_span.clone()], "string(min(1))");
        let constraint = map.constraint_span(&text, &helper, "min", 0).unwrap();
        assert_eq!(&text[constraint], "min(1)");
    }
}

#[test]
fn inline_comments_preserve_the_ordinary_schema_export_and_its_type_requirements() {
    let base = "kind: schema\ntypes:\n    Label: string(required)\n$schema:\n    label: Label@this\n";
    let commented = base.replace("string(required)", "string(required) # An authored helper");
    for text in [base, &commented] {
        let document = parse_standalone_schema_document(text, "schema.yaml").unwrap().unwrap();
        assert!(document.exports_schema);
        let schema = SimplifiedSchema::try_from(text).unwrap();
        assert!(schema.validate_strict(&serde_json::json!({"label": "ready"})).is_ok());
        assert!(schema.validate_strict(&serde_json::json!({"label": 3})).is_err());
    }
}

// ── Privacy: never a document schema or an import source ───────────────────

#[test]
fn the_schema_factories_refuse_a_catalog() {
    for text in [EXPRESSION, MUTATION] {
        let error = SimplifiedSchema::try_from(text).expect_err("a catalog is not a document schema");
        assert!(error.to_string().contains("callable catalog"), "{error}");
    }
}

#[test]
fn the_callable_catalog_reason_is_a_role_refusal_with_a_hint() {
    let reason = NotSchemaReason::CallableCatalog;
    assert!(reason.is_role());
    assert!(reason.hint(Path::new("catalog.yaml")).is_some_and(|hint| hint.contains("kind: schema")));
    let text = reason.to_string();
    assert!(text.contains("callable signatures") && text.contains("named-type import"), "{text}");
}

#[cfg(feature = "file")]
mod file {
    use std::collections::{BTreeMap, HashMap};
    use std::fs;
    use std::path::PathBuf;
    use std::sync::Arc;

    use biscuit_file::{FileResolutionContext, canonicalize_simplified};
    use serde_json::json;
    use simplified_schema::embedded::ShippedSources;
    use simplified_schema::resolve::bundle::{self, BundleSources};
    use simplified_schema::resolve::{
        check_standalone_named_types, parse_referenced_schema_document, resolve_schema_with_roots,
        resolve_yaml_schema,
    };
    use simplified_schema::{NotSchemaReason, SchemaError, parse_standalone_schema_document};

    use super::{EXPRESSION, MUTATION};

    const LIBRARY: &str = "kind: schema\ntypes:\n  Owner: string\n";

    /// The first `NotASchemaDocument` reason in `error`'s source chain.
    fn not_schema(error: &SchemaError) -> Option<NotSchemaReason> {
        let mut cause: &dyn std::error::Error = error;
        loop {
            if let Some(SchemaError::NotASchemaDocument { reason, .. }) = cause.downcast_ref::<SchemaError>() {
                return Some(*reason);
            }
            cause = cause.source()?;
        }
    }

    struct Repo {
        _tmp: tempfile::TempDir,
        root: PathBuf,
    }

    impl Repo {
        fn new() -> Self {
            let tmp = tempfile::tempdir().unwrap();
            let root = canonicalize_simplified(tmp.path()).unwrap();
            fs::create_dir_all(root.join(".git")).unwrap();
            Self { _tmp: tmp, root }
        }

        fn write(&self, relative: &str, text: &str) -> PathBuf {
            let path = relative.split('/').fold(self.root.clone(), |path, segment| path.join(segment));
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, text).unwrap();
            path
        }

        fn context(&self) -> FileResolutionContext {
            FileResolutionContext::from_snapshot(self.root.join("pkg"), None, HashMap::new())
                .with_repository_root(&self.root)
                .with_package_root(self.root.join("pkg"))
        }
    }

    #[test]
    fn a_catalog_is_refused_as_a_schema_target_and_an_import_source_well_formed_or_not() {
        let repo = Repo::new();
        repo.write("pkg/library.yaml", LIBRARY);
        repo.write("pkg/text.yaml", EXPRESSION);
        repo.write("pkg/actions.yaml", MUTATION);
        repo.write("pkg/broken.yaml", "kind: expression-schema\ntypes: 5\nwhen: x\n");
        let (context, dir) = (repo.context(), repo.root.join("pkg"));

        // The control: an ordinary types library still serves imports.
        resolve_schema_with_roots(&json!({ "owner": "Owner@./library.yaml" }), &dir, &[], &context, ShippedSources::NONE)
            .expect("a types library is importable");

        for file in ["text.yaml", "actions.yaml", "broken.yaml"] {
            let target = format!("./{file}");
            let error = resolve_schema_with_roots(&json!(target), &dir, &[], &context, ShippedSources::NONE)
                .expect_err("a catalog is not a `$schema` target");
            assert_eq!(not_schema(&error), Some(NotSchemaReason::CallableCatalog), "{file}: {error}");

            let import = json!({ "label": format!("Label@./{file}") });
            let error = resolve_schema_with_roots(&import, &dir, &[], &context, ShippedSources::NONE)
                .expect_err("a catalog's types are private");
            assert_eq!(not_schema(&error), Some(NotSchemaReason::CallableCatalog), "{file}: {error}");
        }
        assert!(error_text_names_the_reason(&repo, &context));
    }

    fn error_text_names_the_reason(repo: &Repo, context: &FileResolutionContext) -> bool {
        let error = resolve_schema_with_roots(&json!("./text.yaml"), &repo.root.join("pkg"), &[], context, ShippedSources::NONE)
            .expect_err("refused");
        let mut text = error.to_string();
        let mut cause: Option<&dyn std::error::Error> = std::error::Error::source(&error);
        while let Some(next) = cause {
            text.push_str(&next.to_string());
            cause = next.source();
        }
        text.contains("callable catalog")
    }

    #[test]
    fn the_reference_reader_refuses_a_malformed_catalog_for_its_kind() {
        let path = std::path::Path::new("broken.yaml");
        for text in ["kind: expression-schema\ntypes: 5\n", "kind: mutation-schema\n$schema: {}\n", EXPRESSION] {
            let error = parse_referenced_schema_document(text, path).expect_err("refused for its kind");
            assert!(
                matches!(error, SchemaError::NotASchemaDocument { reason: NotSchemaReason::CallableCatalog, .. }),
                "{error:?}"
            );
        }
        assert!(parse_referenced_schema_document(LIBRARY, path).expect("control").is_some());
    }

    #[test]
    fn a_catalog_is_never_expanded_as_a_types_library() {
        let repo = Repo::new();
        let path = repo.write("pkg/text.yaml", EXPRESSION);
        let document = parse_standalone_schema_document(EXPRESSION, &path).expect("valid").expect("recognized");
        let error = check_standalone_named_types(&document, &[], &repo.context(), ShippedSources::NONE)
            .expect_err("never expanded");
        assert!(matches!(error, SchemaError::NotASchemaDocument { reason: NotSchemaReason::CallableCatalog, .. }), "{error:?}");
    }

    #[test]
    fn a_bundle_member_catalog_cannot_be_imported() {
        let members: BTreeMap<String, Arc<[u8]>> = [("schemas/types.yaml", LIBRARY), ("schemas/text.yaml", EXPRESSION)]
            .iter()
            .map(|(path, text)| ((*path).to_string(), Arc::from(text.as_bytes())))
            .collect();
        let sources = BundleSources::embedded("host", members).expect("valid bundle");
        let dir = tempfile::tempdir().expect("tempdir");
        let context = FileResolutionContext::new(dir.path());
        let base = sources.path_of("schemas/types.yaml").expect("listed").parent().expect("parent").to_path_buf();
        let yaml = |text: &str| -> serde_yaml_ng::Value { serde_yaml_ng::from_str(text).expect("YAML") };

        let (control, _) =
            bundle::with(&sources, || resolve_yaml_schema(&yaml("owner: Owner@./types.yaml\n"), &base, &context, ShippedSources::NONE));
        control.expect("a listed types library is importable");
        let (refused, _) =
            bundle::with(&sources, || resolve_yaml_schema(&yaml("label: Label@./text.yaml\n"), &base, &context, ShippedSources::NONE));
        let error = refused.expect_err("a listed catalog is private");
        assert_eq!(not_schema(&error), Some(NotSchemaReason::CallableCatalog), "{error}");
    }
}

#[test]
fn decorated_and_aliased_sequence_arms_preserve_their_contracts() {
    let number = "function(parameters(x: number, y: number); returns(string); category(Text))";
    let string = "function(parameters(x: string); returns(string); category(Text))";
    for kind in ["expression-schema", "mutation-schema"] {
        let document = |body: &str| {
            format!("kind: {kind}\ncategories: [Text]\ntypes:\n    secondary: &number '{number}'\n    pad:{body}\n")
        };
        let control = document(&format!(" ['{number}', '{string}']"));
        let control = parse_standalone_schema_document(&control, "catalog.yaml").unwrap().unwrap();
        for (body, authored) in [
            (format!("\n        - &arm '{number}' # Keep the number arm.\n        - '{string}'"), format!("&arm '{number}'")),
            (format!("\n        - &arm |-\n            {number}\n        - '{string}'"), format!("&arm |-\n            {number}")),
            (format!("\n        - !!str &arm '{number}'\n        - '{string}'"), format!("!!str &arm '{number}'")),
            (format!("\n        - &arm # Keep the number arm.\n          '{number}'\n        - '{string}'"), format!("&arm # Keep the number arm.\n          '{number}'")),
            (format!("\n        - *number\n        - '{string}'"), "*number".into()),
            (format!(" [&arm !!str '{number}', '{string}']"), format!("&arm !!str '{number}'")),
            (format!(" [!!str # Type of the first arm.\n        &arm '{number}', '{string}']"), format!("!!str # Type of the first arm.\n        &arm '{number}'")),
            (format!(" [*number, '{string}']"), "*number".into()),
        ] {
            let text = document(&body);
            let parsed = parse_standalone_schema_document(&text, "catalog.yaml").unwrap_or_else(|error| panic!("{body}: {error:?}")).unwrap();
            assert_eq!(parsed.declaration, control.declaration, "{body}");
            let path = SchemaSourcePath::root().property("pad").union_arm(0);
            let map = parsed.named_type_source_map();
            let [arm] = map.spans(&path, SchemaSpanKind::UnionArm) else { panic!("{body}") };
            assert_eq!(&text[arm.clone()], authored);
        }
    }
}

#[test]
fn decorated_and_aliased_duplicate_arms_keep_their_distinct_source_ranges() {
    let signature = "function(parameters(x: string); returns(string); category(Text))";
    let resolved = "function(parameters(x: Label@this); returns(string); category(Text))";
    for kind in ["expression-schema", "mutation-schema"] {
        for (body, primary, related) in [
            (format!("\n        - &arm '{signature}'\n        - '{signature}'"), signature, signature),
            (format!("\n        - &arm '{signature}'\n        - *arm"), "*arm", signature),
            (format!("\n        - *first\n        - '{signature}'"), signature, "*first"),
            (format!(" [&arm '{signature}', *arm]"), "*arm", signature),
            (format!(" [*first, '{resolved}']"), resolved, "*first"),
        ] {
            let text = format!("kind: {kind}\ncategories: [Text]\ntypes:\n    Label: &label string\n    Alias: *label\n    secondary: &first '{signature}'\n    pad:{body}\n");
            let error = parse_standalone_schema_document(&text, "catalog.yaml").unwrap_err();
            let SchemaError::CallableCatalog { message, span, related: notes, .. } = error else { panic!("{error:?}") };
            assert!(message.contains("same binding contract"), "{message}");
            let span = span.unwrap();
            let [note] = notes.as_slice() else { panic!("{notes:?}") };
            let first = note.span.clone().unwrap();
            assert!(first.start >= text.find("    pad:").unwrap() && first.end < span.start, "{text}");
            assert_eq!(&text[span], primary, "{text}");
            assert_eq!(&text[first], related, "{text}");
        }
    }
}

#[test]
fn ordinary_exports_accept_aliased_and_decorated_sequence_values() {
    let control = "kind: schema\ntypes:\n    Label: &label string\n    Shape: &shape {detail: string}\n    Result: [string, number]\n    Nested: [string, {detail: string}]\n$schema:\n    label: Result@this\n    nested: Nested@this\n";
    let reference = SimplifiedSchema::try_from(control).unwrap();
    for (from, to) in [
        ("[string, number]", "[*label, &num number]"),
        ("[string, number]", "\n        - !!str string\n        - number"),
        ("[string, {detail: string}]", "[string, *shape]"),
        ("[string, {detail: string}]", "[string, {detail: !!str &detail 'string'}]"),
        ("[string, {detail: string}]", "\n        - string\n        - &nested\n            detail: string"),
    ] {
        let text = control.replace(from, to);
        let document = parse_standalone_schema_document(&text, "schema.yaml").unwrap_or_else(|error| panic!("{text}: {error:?}")).unwrap();
        assert!(document.exports_schema, "{text}");
        let schema = SimplifiedSchema::try_from(text.as_str()).unwrap();
        for (value, accepted) in [
            (serde_json::json!({"label": 3, "nested": {"detail": "ready"}}), true),
            (serde_json::json!({"label": "ready", "nested": "ready"}), true),
            (serde_json::json!({"label": true, "nested": "ready"}), false),
            (serde_json::json!({"label": 3, "nested": {"detail": 3}}), false),
        ] {
            assert_eq!(reference.validate_strict(&value).is_ok(), accepted, "{value}");
            assert_eq!(schema.validate_strict(&value).is_ok(), accepted, "{text}\n{value}");
        }
    }
}

#[test]
fn whole_value_aliases_and_a_later_aliased_narrowing_arm_keep_their_verdicts() {
    let first = "function(parameters(x: string); returns(boolean); narrows(x: string; true-iff); category(Text))";
    let second = "function(parameters(x: number, y: number); returns(boolean); narrows(x: string; true-iff); category(Text))";
    for original in [
        format!("'{first}'"),
        format!("'{first} | {second}'"),
        format!("[ '{first}', '{second}' ]"),
    ] {
        let text = format!("kind: expression-schema\ncategories: [Text]\ntypes:\n    first: &first {original}\n    second: *first\n");
        let control = text.replace("*first", &original);
        let control = parse_standalone_schema_document(&control, "catalog.yaml").unwrap().unwrap();
        let actual = parse_standalone_schema_document(&text, "catalog.yaml").unwrap().unwrap();
        assert_eq!(actual.declaration, control.declaration, "{text}");
        let path = SchemaSourcePath::root().property("second");
        let [atom] = actual.named_type_source_map().spans(&path, SchemaSpanKind::Atom) else { panic!("{text}") };
        assert_eq!(&text[atom.clone()], "*first");
    }

    let text = format!("kind: expression-schema\ndescription: &second '{second}'\ncategories: [Text]\ntypes:\n    predicate: ['{first}', *second]\n");
    assert!(parse_standalone_schema_document(&text, "catalog.yaml").is_ok());
    let unguarded_first = first.replace(" narrows(x: string; true-iff);", "");
    let unguarded_second = second.replace(" narrows(x: string; true-iff);", "");
    // The declaring clause is primary when the other overload omits it.
    for (conflict, primary, earlier) in [
        (text.replace(first, &unguarded_first), "*second", unguarded_first.as_str()),
        (text.replace(second, &unguarded_second), "narrows(x: string; true-iff)", "*second"),
    ] {
        let error = parse_standalone_schema_document(&conflict, "catalog.yaml").unwrap_err();
        let SchemaError::CallableCatalog { message, span, related, .. } = error else { panic!("{error:?}") };
        assert!(message.contains("narrows"), "{message}");
        assert_eq!(&conflict[span.unwrap()], primary);
        let [note] = related.as_slice() else { panic!("{related:?}") };
        assert_eq!(&conflict[note.span.clone().unwrap()], earlier);
    }
}

#[test]
fn ordinary_root_arms_and_constraint_mappings_keep_aliased_values() {
    let control = "kind: schema\n$schema:\n  - first: string(required)\n    $constraints: &rules {closed: true}\n  - &shape\n    second: number(required)\n    $constraints: {closed: true}\n  - {second: 'number(required)', $constraints: {closed: true}}\n";
    let actual = control
        .replace("    $constraints: {closed: true}", "    $constraints: *rules")
        .replace("  - {second: 'number(required)', $constraints: {closed: true}}", "  - *shape");
    let expected = parse_standalone_schema_document(control, "schema.yaml").unwrap().unwrap();
    let parsed = parse_standalone_schema_document(&actual, "schema.yaml").unwrap().unwrap();
    assert!(parsed.exports_schema);
    assert_eq!(parsed.declaration, expected.declaration);

    let second = SchemaSourcePath::root().union_arm(1).property("$constraints");
    let [constraints] = parsed.source_map.spans(&second, SchemaSpanKind::Definition) else { panic!("{actual}") };
    assert_eq!(&actual[constraints.clone()], "*rules");
    let third = SchemaSourcePath::root().union_arm(2);
    let [arm] = parsed.source_map.spans(&third, SchemaSpanKind::UnionArm) else { panic!("{actual}") };
    assert_eq!(&actual[arm.clone()], "*shape");
    assert!(parsed.source_map.spans(&third.property("second"), SchemaSpanKind::MappingKey).is_empty());

    let schema = SimplifiedSchema::try_from(actual.as_str()).unwrap();
    assert!(schema.validate_strict(&serde_json::json!({"first": "ready"})).is_ok());
    assert!(schema.validate_strict(&serde_json::json!({"second": 2})).is_ok());
    assert!(schema.validate_strict(&serde_json::json!({"second": 2, "extra": true})).is_err());
}

#[test]
fn a_yaml_alias_used_as_an_extends_operand_marks_the_use() {
    let source = "kind: expression-schema\ndescription: &parent Parent@this\ncategories: [Text]\ntypes:\n    Parent: {text: string}\n    Child:\n        $extends: [*parent]\n        count: number\n    read_label: 'function(parameters(x: Child@this); returns(string); category(Text))'\n";
    let with_alias = parse_standalone_schema_document(source, "catalog.yaml").unwrap().unwrap();
    let written = source.replace("*parent", "Parent@this");
    let control = parse_standalone_schema_document(&written, "catalog.yaml").unwrap().unwrap();
    assert_eq!(with_alias.declaration, control.declaration);
    let path = SchemaSourcePath::root().property("Child").property("$extends").union_arm(0);
    let map = with_alias.named_type_source_map();
    let [operand] = map.spans(&path, SchemaSpanKind::CompositionOperand) else { panic!("{source}") };
    assert_eq!(&source[operand.clone()], "*parent");
    assert!(map.spans(&path, SchemaSpanKind::ImportName).is_empty());
}
