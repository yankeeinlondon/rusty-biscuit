//! The labels and local tables of the kind envelopes: `name`,
//! `description`, and `types` on `kind: schema-trigger`, `name` on a
//! `types`-only `kind: schema` library, and the `kind: namespace-schema`
//! envelope, which is a catalog input only.
//!
//! Boundary: the crate's public API in-process (the envelope parsers, the
//! resolver, trigger discovery, and bundle preparation). The cases gated on
//! `feature = "file"` read real files in a temporary repository.

use simplified_schema::embedded::{EmbeddedSchemaBundle, ShippedSources};
use simplified_schema::envelope::{NAMESPACE_SCHEMA_DEFINITION_PATH, admit_envelope_text};
use simplified_schema::shipped::PreparationMode;
use simplified_schema::triggers::{TriggerEnvelope, parse_trigger_envelope_from_str, prepare_trigger_bundle};
use simplified_schema::{
    SchemaError, SimplifiedSchema, StandaloneSchemaEnvelope, parse_namespace_schema_document,
    parse_standalone_schema_document,
};

const TRIGGER: &str = "kind: schema-trigger\nmatch:\n  prompt: string(required)\n$schema:\n  title: string\n";
const LIBRARY: &str = "kind: schema\ntypes:\n  Id: string\n";
const NAMESPACE: &str = "kind: namespace-schema\nnamespace: loop\ndescription: The loop state.\n$schema:\n  count: \"number(integer; min(1)) -> The one-based iteration.\"\n  item: entry\ntypes:\n  entry: \"{ id: string }\"\n";

fn trigger(text: &str) -> Result<TriggerEnvelope, SchemaError> {
    parse_trigger_envelope_from_str(text).map(|envelope| envelope.expect("the envelope is claimed"))
}

/// `TRIGGER` with `line` added after its `kind`.
fn trigger_with(line: &str) -> String {
    TRIGGER.replacen("kind: schema-trigger\n", &format!("kind: schema-trigger\n{line}\n"), 1)
}

#[test]
fn trigger_labels_and_types_are_read_when_written() {
    let envelope = trigger(&format!(
        "{}description: Prompt documents.\ntypes:\n  headline: string(min(1))\n",
        trigger_with("name: prompt-documents")
    ))
    .expect("a labeled trigger parses");
    assert_eq!(envelope.name.as_deref(), Some("prompt-documents"));
    assert_eq!(envelope.description.as_deref(), Some("Prompt documents."));
    assert!(envelope.types.as_ref().is_some_and(serde_yaml_ng::Value::is_mapping));

    let unlabeled = trigger(TRIGGER).expect("the control parses");
    assert_eq!((unlabeled.name, unlabeled.description, unlabeled.types), (None, None, None));

    let empty = trigger(&trigger_with("types: {}")).expect("an empty `types` table is allowed");
    assert!(empty.types.is_some_and(|types| types.as_mapping().is_some_and(serde_yaml_ng::Mapping::is_empty)));
}

#[test]
fn trigger_label_field_shapes_are_refused_at_the_field() {
    for (line, field) in [
        ("name: null", "name"),
        ("name: ~", "name"),
        ("name: 5", "name"),
        ("name: [a]", "name"),
        ("name: \"\"", "name"),
        ("name: Prompt-Documents", "name"),
        ("name: prompt--documents", "name"),
        ("name: prompt-", "name"),
        ("name: 1prompt", "name"),
        ("description: null", "description"),
        ("description: [a]", "description"),
        ("types: null", "types"),
        ("types: [a]", "types"),
        ("types: ./types.yaml", "types"),
        ("types:\n  bad: \"string(\"", "types"),
    ] {
        let error = trigger(&trigger_with(line)).expect_err(line);
        let SchemaError::TriggerMatch { message, .. } = &error else { panic!("`{line}`: {error:?}") };
        assert!(message.starts_with(&format!("`{field}`:")), "`{line}`: {message}");
    }
    let error = trigger(&trigger_with("label: x")).expect_err("the envelope stays closed");
    assert!(error.to_string().contains("unknown keys: label"), "{error}");
}

#[test]
fn library_name_is_admitted_only_in_the_types_only_form() {
    let named = parse_standalone_schema_document(&LIBRARY.replace("kind: schema\n", "kind: schema\nname: shared-ids\n"), "lib.yaml")
        .expect("a named types-only library parses")
        .expect("claimed");
    assert_eq!(named.name.as_deref(), Some("shared-ids"));
    assert!(!named.exports_schema);
    let unnamed = parse_standalone_schema_document(LIBRARY, "lib.yaml").unwrap().unwrap();
    assert_eq!(unnamed.name, None);
    let null_export = "kind: schema\nname: shared-ids\n$schema: null\ntypes:\n  Id: string\n";
    assert!(parse_standalone_schema_document(null_export, "lib.yaml").unwrap().unwrap().name.is_some());

    for (text, label) in [
        ("kind: schema\nname: null\ntypes:\n  Id: string\n", "null"),
        ("kind: schema\nname: 5\ntypes:\n  Id: string\n", "number"),
        ("kind: schema\nname: [a]\ntypes:\n  Id: string\n", "sequence"),
        ("kind: schema\nname: \"\"\ntypes:\n  Id: string\n", "empty"),
        ("kind: schema\nname: Shared\ntypes:\n  Id: string\n", "bad pattern"),
        ("kind: schema\nname: shared-ids\n$schema:\n  id: Id\ntypes:\n  Id: string\n", "beside an export"),
        ("kind: schema\nname: shared-ids\n$schema:\n  id: string\n", "export only"),
    ] {
        let error = parse_standalone_schema_document(text, "lib.yaml").expect_err(label);
        assert!(error.to_string().contains("name"), "{label}: {error}");
    }
}

#[test]
fn namespace_schema_control_is_admitted_and_read() {
    let admission = admit_envelope_text(NAMESPACE, "loop.yaml").unwrap().expect("claimed");
    assert_eq!(admission.envelope, StandaloneSchemaEnvelope::NamespaceSchema);
    assert_eq!(admission.definition, NAMESPACE_SCHEMA_DEFINITION_PATH);
    assert!(admission.is_admitted(), "{:?}", admission.problems);

    let document = parse_namespace_schema_document(NAMESPACE, "loop.yaml").unwrap().expect("claimed");
    assert_eq!(document.namespace, "loop");
    assert_eq!(document.description.as_deref(), Some("The loop state."));
    assert_eq!(document.schema.properties.keys().collect::<Vec<_>>(), ["count", "item"]);
    assert_eq!(document.local_types.as_ref().map(|types| types.properties.keys().collect::<Vec<_>>()), Some(vec![&"entry".to_string()]));
    assert_eq!(document.path, std::path::Path::new("loop.yaml"));

    assert_eq!(parse_namespace_schema_document(LIBRARY, "lib.yaml").unwrap(), None, "another kind is not claimed");
}

#[test]
fn namespace_schema_envelope_shapes() {
    let empty = parse_namespace_schema_document("kind: namespace-schema\nnamespace: seq\n$schema: {}\n", "seq.yaml")
        .expect("`$schema: {}` declares no properties")
        .expect("claimed");
    assert!(empty.schema.properties.is_empty() && empty.local_types.is_none() && empty.description.is_none());
    let empty_types = "kind: namespace-schema\nnamespace: seq\n$schema: {}\ntypes: {}\n";
    assert!(parse_namespace_schema_document(empty_types, "seq.yaml").is_ok(), "`types: {{}}` is allowed");

    for (text, needle) in [
        ("kind: namespace-schema\n$schema: {}\n", "namespace"),
        ("kind: namespace-schema\nnamespace: seq\n", "$schema"),
        ("kind: namespace-schema\nnamespace: seq\n$schema: ./seq-value.yaml\n", "$schema"),
        ("kind: namespace-schema\nnamespace: seq\n$schema:\n  - a: string\n", "$schema"),
        ("kind: namespace-schema\nnamespace: seq\n$schema: {}\nname: seq\n", "name"),
        ("kind: namespace-schema\nnamespace: seq\n$schema: {}\ndescription: null\n", "description"),
        ("kind: namespace-schema\nnamespace: seq\n$schema: {}\ntypes: null\n", "types"),
        ("kind: namespace-schema\nnamespace: 5\n$schema: {}\n", "namespace"),
    ] {
        let error = parse_namespace_schema_document(text, "seq.yaml").expect_err(text);
        assert!(error.to_string().contains(needle), "{text}: {error}");
    }
}

#[test]
fn namespace_schema_is_not_a_schema_object() {
    let error = SimplifiedSchema::try_from(NAMESPACE).expect_err("a namespace is a catalog input");
    assert!(error.to_string().contains("namespace-schema"), "{error}");
}

#[test]
fn embedded_bundle_preparation_refuses_a_duplicate_name() {
    static FILES: &[(&str, &str)] = &[
        ("a.yaml", "kind: schema-trigger\nname: twin\nmatch:\n  a: string(required)\n"),
        ("b.yaml", "kind: schema-trigger\nname: twin\nmatch:\n  b: string(required)\n"),
    ];
    let error = prepare_trigger_bundle(EmbeddedSchemaBundle::new("twins", FILES), PreparationMode::Runtime, ShippedSources::NONE)
        .expect_err("a bundle with one name twice does not prepare");
    assert!(error.to_string().contains("`twin`"), "{error}");
}

#[test]
fn embedded_bundle_record_carries_labels() {
    static FILES: &[(&str, &str)] = &[(
        "labeled.yaml",
        "kind: schema-trigger\nname: labeled\ndescription: A labeled trigger.\nmatch:\n  a: string(required)\n$schema:\n  a: id\ntypes:\n  id: string(min(1))\n",
    )];
    let record = prepare_trigger_bundle(EmbeddedSchemaBundle::new("labels", FILES), PreparationMode::Runtime, ShippedSources::NONE)
        .expect("a labeled bundle with local types prepares");
    let document = &record.documents[0];
    assert_eq!(document.label.as_deref(), Some("labeled"));
    assert_eq!(document.description.as_deref(), Some("A labeled trigger."));
    let simplified_schema::triggers::TriggerOutcomeRecord::Payload(payload) = &document.outcome else {
        panic!("{:?}", document.outcome)
    };
    assert!(payload.json_schema.to_string().contains("\"minLength\":1"), "{}", payload.json_schema);
}

#[cfg(feature = "file")]
mod files {
    use std::collections::HashMap;
    use std::fs;
    use std::path::{Path, PathBuf};

    use biscuit_file::{FileResolutionContext, canonicalize_simplified};
    use serde_json::json;
    use simplified_schema::embedded::{EmbeddedSchemaBundle, ShippedSources};
    use simplified_schema::resolve::resolve_schema_with_roots;
    use simplified_schema::triggers::{TriggerRegistry, TriggerSource, scan, scan_with_embedded};
    use simplified_schema::{NotSchemaReason, SchemaError, SchemaSourceId, SchemaSourcePath};

    use super::NAMESPACE;

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

    /// A temporary repository with a `pkg` package; documents live in
    /// `pkg/`, so `pkg/schemas/` and `schemas/` are two schema roots.
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
            // Joined per segment so the path displays with native separators,
            // as discovery reports it on Windows.
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

        fn scan(&self) -> TriggerRegistry {
            scan(&self.context(), ShippedSources::NONE).expect("discovery completes")
        }
    }

    fn file_names(registry: &TriggerRegistry) -> Vec<String> {
        registry.triggers.iter().filter_map(|trigger| trigger.source.file_name().map(str::to_string)).collect()
    }

    fn trigger_text(name: &str, property: &str) -> String {
        format!("kind: schema-trigger\nname: {name}\nmatch:\n  {property}: string(required)\n$schema:\n  {property}: string\n")
    }

    #[test]
    fn a_namespace_file_is_refused_as_a_schema_target_and_an_import_source() {
        let repo = Repo::new();
        let namespace = repo.write("pkg/loop.yaml", NAMESPACE);
        repo.write("pkg/consumer.yaml", "$schema:\n  item: entry@./loop.yaml\n");
        let context = repo.context();
        let dir = repo.root.join("pkg");

        let error = resolve_schema_with_roots(&json!("./loop.yaml"), &dir, &[], &context, ShippedSources::NONE)
            .expect_err("a namespace file is not a `$schema` target");
        assert_eq!(not_schema(&error), Some(NotSchemaReason::NamespaceCatalog), "{error}");
        assert!(error.to_string().contains("namespace-schema"), "{error}");

        let error = resolve_schema_with_roots(&json!({ "item": "entry@./loop.yaml" }), &dir, &[], &context, ShippedSources::NONE)
            .expect_err("a namespace file's types are private");
        assert_eq!(not_schema(&error), Some(NotSchemaReason::NamespaceCatalog), "{error}");

        let malformed = repo.write("pkg/broken.yaml", "kind: namespace-schema\n$schema: {}\n");
        let error = resolve_schema_with_roots(&json!("./broken.yaml"), &dir, &[], &context, ShippedSources::NONE)
            .expect_err("a malformed namespace file is refused for its kind");
        assert_eq!(not_schema(&error), Some(NotSchemaReason::NamespaceCatalog), "{error}");
        assert!(namespace.exists() && malformed.exists());
    }

    #[test]
    fn a_trigger_payload_names_its_own_types_and_no_other_file_can() {
        let repo = Repo::new();
        repo.write(
            "schemas/titled.yaml",
            "kind: schema-trigger\nmatch:\n  title: string(required)\n$schema:\n  title: headline\ntypes:\n  headline: string(min(3))\n",
        );
        repo.write("pkg/doc.md", "# A document\n");
        let registry = repo.scan();
        assert!(registry.findings().is_empty(), "{:?}", registry.findings());
        assert_eq!(file_names(&registry), ["titled.yaml"]);
        let payload = &registry.payloads()[0];
        assert!(payload.json_schema.to_string().contains("\"minLength\":3"), "{}", payload.json_schema);

        let error = resolve_schema_with_roots(
            &json!({ "title": "headline@../schemas/titled.yaml" }),
            &repo.root.join("pkg"),
            &[],
            &repo.context(),
            ShippedSources::NONE,
        )
        .expect_err("a trigger's types are not importable");
        assert_eq!(not_schema(&error), Some(NotSchemaReason::TriggerEnvelope), "{error:?}");
        assert_eq!(error.reference_site().map(|site| site.path.to_string()).as_deref(), Some("title"), "refused at the import");
    }

    #[test]
    fn a_duplicate_name_in_one_root_marks_and_excludes_both_files() {
        let repo = Repo::new();
        let first = repo.write("schemas/first.yaml", &trigger_text("twin", "alpha"));
        let library = repo.write("schemas/library.yaml", "kind: schema\nname: twin\ntypes:\n  Id: string\n");
        repo.write("schemas/other.yaml", &trigger_text("single", "gamma"));
        let registry = repo.scan();

        assert_eq!(file_names(&registry), ["other.yaml"], "both participants are excluded");
        let findings = registry.findings();
        let reported: Vec<_> = findings.iter().map(|finding| finding.source().clone()).collect();
        assert_eq!(reported, [TriggerSource::File(first.clone()), TriggerSource::File(library.clone())]);
        for finding in findings {
            assert_eq!(finding.name(), Some("twin"));
            assert_eq!(finding.position(), Some((2, 7)), "located at the `name` value");
            let SchemaError::TriggerMatch { span: Some(span), message } = finding.cause() else {
                panic!("{:?}", finding.cause())
            };
            let path = finding.source().path().unwrap();
            assert_eq!(&fs::read_to_string(path).unwrap()[span.clone()], "twin");
            let other = if path == first { &library } else { &first };
            assert!(message.contains(&other.display().to_string()), "{message}");
            assert!(finding.message().contains(&format!("twin ({})", path.display())), "{}", finding.message());
        }
    }

    #[test]
    fn the_same_name_in_two_roots_is_not_a_duplicate() {
        let repo = Repo::new();
        repo.write("pkg/schemas/near.yaml", &trigger_text("shared", "alpha"));
        repo.write("schemas/far.yaml", &trigger_text("shared", "beta"));
        let registry = repo.scan();
        assert!(registry.findings().is_empty(), "{:?}", registry.findings());
        assert_eq!(file_names(&registry), ["near.yaml", "far.yaml"]);
        assert!(registry.triggers.iter().all(|trigger| trigger.label().starts_with("shared (")));
    }

    #[test]
    fn a_shadowed_file_still_counts_toward_its_roots_names() {
        let repo = Repo::new();
        repo.write("pkg/schemas/same.yaml", &trigger_text("winner", "alpha"));
        repo.write("schemas/same.yaml", &trigger_text("twin", "beta"));
        let sibling = repo.write("schemas/sibling.yaml", &trigger_text("twin", "gamma"));
        let registry = repo.scan();
        assert_eq!(file_names(&registry), ["same.yaml"], "the near file wins; the far root's twins are both excluded");
        assert!(registry.findings().iter().any(|finding| finding.source() == &TriggerSource::File(sibling.clone())));
        assert_eq!(registry.findings().len(), 2, "{:?}", registry.findings());
    }

    #[test]
    fn a_duplicate_name_in_one_bundle_marks_and_excludes_both_documents() {
        static FILES: &[(&str, &str)] = &[
            ("a.yaml", "kind: schema-trigger\nname: twin\nmatch:\n  a: string(required)\n"),
            ("b.yaml", "kind: schema-trigger\nname: twin\nmatch:\n  b: string(required)\n"),
            ("c.yaml", "kind: schema-trigger\nname: single\nmatch:\n  c: string(required)\n"),
        ];
        let repo = Repo::new();
        let registry = scan_with_embedded(&repo.context(), &[EmbeddedSchemaBundle::new("twins", FILES)], ShippedSources::NONE)
            .expect("discovery completes");
        assert_eq!(file_names(&registry), ["c.yaml"]);
        let reported: Vec<_> = registry.findings().iter().filter_map(|finding| finding.source().file_name()).collect();
        assert_eq!(reported, ["a.yaml", "b.yaml"]);
    }

    #[test]
    fn an_extended_library_type_keeps_its_declaring_file() {
        let repo = Repo::new();
        let library = repo.write(
            "pkg/lib/common.yaml",
            "kind: schema\nname: common-types\ntypes:\n  base:\n    count: number(default(3))\n    label: string\n",
        );
        repo.write(
            "schemas/composed.yaml",
            "kind: schema-trigger\nname: composed\nmatch:\n  title: string(required)\n$schema:\n  $extends: base@../pkg/lib/common.yaml\n  title: string\n",
        );
        let registry = repo.scan();
        assert!(registry.findings().is_empty(), "{:?}", registry.findings());
        let payload = &registry.payloads()[0];
        let properties = payload.json_schema["properties"].as_object().unwrap();
        assert_eq!(properties.keys().collect::<Vec<_>>(), ["count", "label", "title"]);
        let declared = payload.default_files.get(&vec!["count".to_string()]).expect("the inherited default has a file");
        assert_eq!(canonicalize_simplified(declared).unwrap(), library);
        assert!(payload.dependencies.iter().any(|path| canonicalize_simplified(path).unwrap() == library));

        // A failure inside an inherited member names its declaration in the
        // library, not the trigger.
        repo.write(
            "pkg/lib/common.yaml",
            "kind: schema\ntypes:\n  base:\n    count: number\n    f: 'function(parameters(void); returns(string))'\n",
        );
        let registry = repo.scan();
        let [finding] = registry.findings() else { panic!("{:?}", registry.findings()) };
        let mut cause: &SchemaError = finding.cause();
        if let SchemaError::TriggerLoad { source, .. } = cause {
            cause = source;
        }
        let SchemaError::ComposedMember { site, .. } = cause else { panic!("{cause:?}") };
        let SchemaSourceId::File(declaring) = &site.source else { panic!("{site:?}") };
        assert_eq!(canonicalize_simplified(declaring).unwrap(), library);
        assert_eq!(site.path, SchemaSourcePath::root().property("base").property("f"), "a types-only payload is its table");
        assert!(Path::new(&library).exists());
    }
}
