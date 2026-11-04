//! The schemas the repository ships, judged by the library alone: Darkmatter's
//! global catalog and partials, Content Policy's editor schema, Claudine's
//! embedded bundle (its trigger payloads and match arms included), and
//! representative consumer sidecars. Every document is supplied as text, never
//! discovered: the library parses each one passively (`TryFrom<&str>`, no I/O,
//! references kept as written), and resolves its imports among the supplied
//! documents with the embedded-source resolver, the path Darkmatter's own
//! shipped schemas and Claudine's bundle take at run time.
//!
//! Each `include_str!` stays inside the test that reads it, so CI's
//! test-input index runs exactly that test when a shipped schema changes.

use serde_json::json;
use serde_yaml_ng::{Mapping, Value as YamlValue};
use simplified_schema::embedded::{EmbeddedDocument, ShippedSources};
use simplified_schema::resolve::{
    Sharing, prepare_embedded_catalog_declaration, prepare_embedded_inline_schema, prepare_embedded_schema,
    prepare_embedded_schema_in,
};
use simplified_schema::{Prepared, SimplifiedSchema, ValidationFailure, parse_standalone_schema_document, parse_yaml_schema};

const DARKMATTER_CATALOG: &str = "darkmatter/schemas/darkmatter.yaml";

fn yaml(text: &str) -> YamlValue {
    serde_yaml_ng::from_str(text).expect("shipped schemas are YAML")
}

/// `{ key: value }` as a YAML mapping.
fn single(key: &str, value: YamlValue) -> YamlValue {
    let mut mapping = Mapping::new();
    mapping.insert(YamlValue::String(key.to_string()), value);
    YamlValue::Mapping(mapping)
}

/// Every document parses as content, with no file context and no I/O, and
/// the standalone loader recognizes it as a schema document.
fn assert_parses_passively(documents: &[EmbeddedDocument<'_>]) {
    for document in documents {
        SimplifiedSchema::try_from(document.text)
            .unwrap_or_else(|error| panic!("{} does not parse as content: {error}", document.path));
        let standalone = parse_standalone_schema_document(document.text, document.path)
            .unwrap_or_else(|error| panic!("{}: {error}", document.path));
        assert!(standalone.is_some(), "{} is not recognized as a schema document", document.path);
    }
}

/// The names a `kind: schema` document declares under `types`.
fn type_names(text: &str) -> Vec<String> {
    yaml(text)
        .get("types")
        .and_then(YamlValue::as_mapping)
        .map(|types| types.keys().filter_map(YamlValue::as_str).map(str::to_string).collect())
        .unwrap_or_default()
}

/// No file context: every file check is unavailable rather than probed.
fn prepared(json_schema: serde_json::Value) -> Prepared {
    let context = biscuit_file::FileResolutionContext::from_snapshot("", None, Default::default());
    Prepared::new(std::sync::Arc::new(json_schema), None, context).expect("the lowered schema compiles")
}

#[test]
fn darkmatter_catalog_and_partials_resolve_from_supplied_sources() {
    const DOCUMENTS: &[EmbeddedDocument<'static>] = &[
        EmbeddedDocument { path: DARKMATTER_CATALOG, text: include_str!("../../../../darkmatter/schemas/darkmatter.yaml") },
        EmbeddedDocument {
            path: "darkmatter/schemas/partials/fm.yaml",
            text: include_str!("../../../../darkmatter/schemas/partials/fm.yaml"),
        },
        EmbeddedDocument {
            path: "darkmatter/schemas/partials/style.yaml",
            text: include_str!("../../../../darkmatter/schemas/partials/style.yaml"),
        },
        EmbeddedDocument {
            path: "darkmatter/schemas/partials/context-variables.yaml",
            text: include_str!("../../../../darkmatter/schemas/partials/context-variables.yaml"),
        },
        EmbeddedDocument {
            path: "darkmatter/schemas/partials/error.yaml",
            text: include_str!("../../../../darkmatter/schemas/partials/error.yaml"),
        },
        EmbeddedDocument {
            path: "darkmatter/schemas/partials/timing.yaml",
            text: include_str!("../../../../darkmatter/schemas/partials/timing.yaml"),
        },
        EmbeddedDocument {
            path: "darkmatter/schemas/partials/lifecycle.yaml",
            text: include_str!("../../../../darkmatter/schemas/partials/lifecycle.yaml"),
        },
        EmbeddedDocument {
            path: "darkmatter/schemas/partials/md-lifecycle.yaml",
            text: include_str!("../../../../darkmatter/schemas/partials/md-lifecycle.yaml"),
        },
        EmbeddedDocument {
            path: "darkmatter/schemas/partials/functions.yaml",
            text: include_str!("../../../../darkmatter/schemas/partials/functions.yaml"),
        },
        EmbeddedDocument {
            path: "darkmatter/schemas/partials/mutations.yaml",
            text: include_str!("../../../../darkmatter/schemas/partials/mutations.yaml"),
        },
        EmbeddedDocument {
            path: "content-policy/schemas/content-policy.yaml",
            text: include_str!("../../../../content-policy/schemas/content-policy.yaml"),
        },
    ];
    const SHIPPED: ShippedSources = ShippedSources::new("darkmatter:", "darkmatter/schemas/partials", DOCUMENTS);
    assert_parses_passively(&DOCUMENTS[1..]);
    // The global catalog declares globals for a host to register through its
    // own loader. Its envelope is an ordinary tagged schema document with a
    // `description` and an exported `$schema`, so the standalone parser
    // accepts it rather than refusing it by structure.
    let catalog_document = parse_standalone_schema_document(DOCUMENTS[0].text, DARKMATTER_CATALOG)
        .expect("the catalog envelope parses")
        .expect("the catalog is a recognized envelope");
    assert!(catalog_document.exports_schema);
    assert_eq!(catalog_document.description.as_deref(), Some("the shapes of every Darkmatter global"));

    // Every global the catalog declares resolves on its own, as Darkmatter's
    // catalog loader resolves them.
    let catalog = yaml(DOCUMENTS[0].text);
    let declarations = catalog.get("$schema").and_then(YamlValue::as_mapping).expect("the catalog's `$schema`");
    assert!(declarations.len() >= 6, "the catalog declares {} globals", declarations.len());
    let mut baseline = None;
    for (name, value) in declarations {
        let name = name.as_str().expect("a global's name");
        let (resolved, reads) = prepare_embedded_catalog_declaration(
            &single(name, value.clone()),
            DARKMATTER_CATALOG,
            DOCUMENTS,
            Sharing::Automatic,
            SHIPPED,
        )
        .unwrap_or_else(|error| panic!("catalog global `{name}`: {error}"));
        assert!(!reads.is_empty(), "`{name}` read its partial");
        if name == "fm" {
            baseline = Some(resolved.json_schema);
        }
    }

    // The resolved `fm` declaration (the built-in baseline) judges data. Its
    // shared types live under the declaration's root `$defs`, so the whole
    // declaration is prepared and the page sits under `fm`.
    let baseline = prepared(baseline.expect("the catalog declares `fm`"));
    let page = json!({ "fm": { "title": "A page", "tags": ["a", "b"], "draft": true } });
    assert!(baseline.validate_strict(&page).is_ok(), "{:?}", baseline.validate_strict(&page).err());
    let wrong = json!({ "fm": { "tags": 5 } });
    assert!(matches!(baseline.validate_strict(&wrong), Err(ValidationFailure::InvalidData { .. })));

    // Every type of every partial resolves through the reserved namespace,
    // which names the shipped documents and never a file.
    let (mut resolved_types, mut function_types) = (0, 0);
    for document in &DOCUMENTS[1..DOCUMENTS.len() - 1] {
        let stem = document.path.rsplit('/').next().and_then(|file| file.strip_suffix(".yaml")).expect("a partial stem");
        let names = type_names(document.text);
        assert!(!names.is_empty(), "{} declares no types", document.path);
        for name in names {
            let probe = single("probe", YamlValue::String(format!("{name}@darkmatter:{stem}")));
            match prepare_embedded_inline_schema(&probe, "host/probe.yaml", &[], "", SHIPPED) {
                Ok(_) => resolved_types += 1,
                // A function type resolves but describes a callable, so it
                // keeps its located lowering error rather than typing a value.
                Err(error) if error.to_string().contains("`function` type is description-only") => function_types += 1,
                Err(error) => panic!("`{name}@darkmatter:{stem}`: {error}"),
            }
        }
    }
    assert!(resolved_types > 20, "only {resolved_types} partial types resolved");
    assert!(function_types > 0, "the function catalogs declare function types");
}

#[test]
fn claudine_bundle_and_its_triggers_resolve_from_supplied_sources() {
    const BUNDLE: &str = "claudine";
    const DOCUMENTS: &[EmbeddedDocument<'static>] = &[
        EmbeddedDocument { path: "claudine/action.yaml", text: include_str!("../../../../claudine/schemas/action.yaml") },
        EmbeddedDocument { path: "claudine/claudine.yaml", text: include_str!("../../../../claudine/schemas/claudine.yaml") },
        EmbeddedDocument { path: "claudine/document.yaml", text: include_str!("../../../../claudine/schemas/document.yaml") },
        EmbeddedDocument {
            path: "claudine/inline-compose.yaml",
            text: include_str!("../../../../claudine/schemas/inline-compose.yaml"),
        },
        EmbeddedDocument { path: "claudine/lifecycle.yaml", text: include_str!("../../../../claudine/schemas/lifecycle.yaml") },
        EmbeddedDocument { path: "claudine/linking.yaml", text: include_str!("../../../../claudine/schemas/linking.yaml") },
        EmbeddedDocument {
            path: "claudine/sequence-types.yaml",
            text: include_str!("../../../../claudine/schemas/sequence-types.yaml"),
        },
        EmbeddedDocument { path: "claudine/sequence.yaml", text: include_str!("../../../../claudine/schemas/sequence.yaml") },
    ];
    // Claudine's documents name Darkmatter's shipped partials through the
    // reserved namespace, so the host's shipped sources come too.
    const DARKMATTER: &[EmbeddedDocument<'static>] = &[
        EmbeddedDocument {
            path: "darkmatter/schemas/partials/lifecycle.yaml",
            text: include_str!("../../../../darkmatter/schemas/partials/lifecycle.yaml"),
        },
        EmbeddedDocument {
            path: "darkmatter/schemas/partials/md-lifecycle.yaml",
            text: include_str!("../../../../darkmatter/schemas/partials/md-lifecycle.yaml"),
        },
        EmbeddedDocument {
            path: "darkmatter/schemas/partials/functions.yaml",
            text: include_str!("../../../../darkmatter/schemas/partials/functions.yaml"),
        },
        EmbeddedDocument {
            path: "darkmatter/schemas/partials/context-variables.yaml",
            text: include_str!("../../../../darkmatter/schemas/partials/context-variables.yaml"),
        },
    ];
    const SHIPPED: ShippedSources = ShippedSources::new("darkmatter:", "darkmatter/schemas/partials", DARKMATTER);

    let mut triggers = 0;
    for document in DOCUMENTS {
        let root = yaml(document.text);
        if root.get("kind").and_then(YamlValue::as_str) == Some("schema-trigger") {
            triggers += 1;
            // The envelope (`kind`, `match`) is Darkmatter's; the library
            // judges its parts: each match arm and the payload.
            let arms = root["match"].get("group").and_then(YamlValue::as_sequence).cloned().unwrap_or_else(|| vec![root["match"].clone()]);
            for arm in &arms {
                parse_yaml_schema(arm).unwrap_or_else(|error| panic!("{} match arm {arm:?}: {error}", document.path));
            }
            match &root["$schema"] {
                YamlValue::String(reference) => {
                    let target = format!("{BUNDLE}/{}", reference.trim().trim_start_matches("./"));
                    prepare_embedded_schema_in(&target, DOCUMENTS, BUNDLE, SHIPPED)
                        .unwrap_or_else(|error| panic!("{} payload {target}: {error}", document.path));
                }
                payload => {
                    prepare_embedded_inline_schema(payload, document.path, DOCUMENTS, BUNDLE, SHIPPED)
                        .unwrap_or_else(|error| panic!("{} payload: {error}", document.path));
                }
            }
        } else {
            SimplifiedSchema::try_from(document.text).unwrap_or_else(|error| panic!("{}: {error}", document.path));
            // One property per declared type, resolved together: the types
            // share most of their graph, which one resolution expands once.
            let file = document.path.rsplit('/').next().expect("a file name");
            let mut probe = Mapping::new();
            for name in type_names(document.text) {
                probe.insert(YamlValue::String(name.clone()), YamlValue::String(format!("{name}@./{file}")));
            }
            assert!(!probe.is_empty(), "{} declares no types", document.path);
            prepare_embedded_inline_schema(&YamlValue::Mapping(probe), "claudine/probe.yaml", DOCUMENTS, BUNDLE, SHIPPED)
                .unwrap_or_else(|error| panic!("the types of {}: {error}", document.path));
        }
    }
    assert_eq!(triggers, 3, "claudine.yaml, inline-compose.yaml, and sequence.yaml are triggers");
}

#[test]
fn consumer_sidecars_resolve_from_supplied_sources() {
    const DOCUMENTS: &[EmbeddedDocument<'static>] = &[
        EmbeddedDocument {
            path: "claudine/docs/research/model-config/_schema.yaml",
            text: include_str!("../../../../claudine/docs/research/model-config/_schema.yaml"),
        },
        EmbeddedDocument {
            path: "claudine/docs/research/agent-cli/_schema.yaml",
            text: include_str!("../../../../claudine/docs/research/agent-cli/_schema.yaml"),
        },
        EmbeddedDocument {
            path: "claudine/docs/research/agent-cli/_types.yaml",
            text: include_str!("../../../../claudine/docs/research/agent-cli/_types.yaml"),
        },
        EmbeddedDocument {
            path: "claudine/docs/research/_types.yaml",
            text: include_str!("../../../../claudine/docs/research/_types.yaml"),
        },
        EmbeddedDocument {
            path: "messenger/docs/research/platforms/_schema.yaml",
            text: include_str!("../../../../messenger/docs/research/platforms/_schema.yaml"),
        },
        EmbeddedDocument {
            path: "messenger/docs/research/platforms/_types.yaml",
            text: include_str!("../../../../messenger/docs/research/platforms/_types.yaml"),
        },
        EmbeddedDocument {
            path: "messenger/docs/research/implementation/_schema.yaml",
            text: include_str!("../../../../messenger/docs/research/implementation/_schema.yaml"),
        },
    ];
    assert_parses_passively(DOCUMENTS);

    for path in [
        "claudine/docs/research/model-config/_schema.yaml",
        "claudine/docs/research/agent-cli/_schema.yaml",
        "messenger/docs/research/platforms/_schema.yaml",
        "messenger/docs/research/implementation/_schema.yaml",
    ] {
        let resolved =
            prepare_embedded_schema(path, DOCUMENTS, ShippedSources::NONE).unwrap_or_else(|error| panic!("{path}: {error}"));
        prepared(resolved.json_schema);
    }
}

/// The library's own envelope definitions pass the compiled definition set:
/// each is a `yaml-schema` admitted against `yaml-schema.yaml`, the
/// `yaml-schema` definition by itself.
#[test]
fn the_library_envelope_definitions_pass_the_compiled_definitions() {
    use simplified_schema::envelope::{YAML_SCHEMA_DEFINITION_PATH, admit_envelope_text};
    for (path, text) in [
        ("simplified-schema/schemas/schema.yaml", include_str!("../../../schemas/schema.yaml")),
        ("simplified-schema/schemas/yaml-schema.yaml", include_str!("../../../schemas/yaml-schema.yaml")),
    ] {
        let admission = admit_envelope_text(text, path)
            .unwrap_or_else(|error| panic!("{path}: {error}"))
            .unwrap_or_else(|| panic!("{path} is a recognized envelope"));
        assert_eq!(admission.definition, YAML_SCHEMA_DEFINITION_PATH, "{path}");
        assert!(admission.is_admitted(), "{path}: {:#?}", admission.problems);
        let document = parse_standalone_schema_document(text, path).unwrap().unwrap();
        assert!(document.exports_schema, "{path}");
    }
}

/// Envelope structure has one authority, the compiled definitions: the
/// standalone parser keeps no hand-written structural rejection. The
/// behavior is pinned by `envelope_admission`; this guards the source
/// against the deleted checks coming back beside the definitions.
#[test]
fn the_standalone_parser_carries_no_hand_written_structural_checks() {
    let source = include_str!("../../src/simplified/standalone.rs");
    for deleted in [
        "tagged schema documents support only",
        "needs a `$schema` or `types` property",
        "`description` must be a string",
        "`categories` must be a list",
        "`types` must be a mapping",
        "tagged schema document `types` must be a mapping",
    ] {
        assert!(!source.contains(deleted), "the deleted structural check `{deleted}` is back in standalone.rs");
    }
    // Recognition reads the key set only to decide whether an untagged
    // mapping is the informal envelope; it never rejects a tagged one.
    let recognition = &source[source.find("pub(crate) fn recognize_standalone").expect("recognition")..];
    let recognition = &recognition[..recognition.find("\n}\n").expect("end of recognition")];
    assert!(!recognition.contains("return Err("), "recognition must not reject an envelope:\n{recognition}");
}
