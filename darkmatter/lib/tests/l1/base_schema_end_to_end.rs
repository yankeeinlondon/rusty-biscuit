use std::collections::{BTreeSet, HashMap};

use darkmatter::markdown::{
    Markdown,
    compose::{RequestSnapshot, build_resolution_context},
    schemas::{
        DarkmatterSchemas, EmbeddedSchemaBundle, PropertyDef, SchemaShape, TypeExpr, darkmatter_base_json_schema,
        darkmatter_base_schema, parse_yaml_schema, resolve::resolve_yaml_schema, to_json_schema,
    },
};
use biscuit_test_harness::manifest_dir;
use serde_yaml_ng::Value as YamlValue;

fn base_api() -> DarkmatterSchemas {
    DarkmatterSchemas::new(crate::request_support::cwd_context())
        .with_baseline(darkmatter_base_schema())
        .expect("base schema must be baseline-compatible")
}

/// Claudine's complete schema bundle, as Claudine embeds it.
const CLAUDINE_BUNDLE: EmbeddedSchemaBundle = EmbeddedSchemaBundle::new(
    "claudine",
    &[
        ("action.yaml", include_str!("../../../../claudine/schemas/action.yaml")),
        ("claudine.yaml", include_str!("../../../../claudine/schemas/claudine.yaml")),
        ("document.yaml", include_str!("../../../../claudine/schemas/document.yaml")),
        ("inline-compose.yaml", include_str!("../../../../claudine/schemas/inline-compose.yaml")),
        ("lifecycle.yaml", include_str!("../../../../claudine/schemas/lifecycle.yaml")),
        ("linking.yaml", include_str!("../../../../claudine/schemas/linking.yaml")),
    ],
);

/// The Darkmatter baseline with trigger discovery and Claudine's bundle
/// registered, the way Claudine's launch verdict applies it to a document with
/// no `$schema`. Discovery runs in an empty folder with an empty home, so the
/// embedded bundle is the only trigger source; the folder lives as long as the
/// returned guard.
fn claudine_api() -> (tempfile::TempDir, DarkmatterSchemas) {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
    std::fs::create_dir_all(root.join("home")).expect("home dir");
    let snapshot = RequestSnapshot::new(&root)
        .with_home(Some(root.join("home")))
        .with_env(HashMap::new());
    let context = build_resolution_context(&snapshot).expect("fixture context");
    let api = DarkmatterSchemas::new(context)
        .with_baseline(darkmatter_base_schema())
        .expect("base schema must be baseline-compatible")
        .with_trigger_discovery()
        .expect("an empty folder has no disk triggers")
        .with_embedded_trigger_bundle(CLAUDINE_BUNDLE)
        .expect("Claudine's bundle must load");
    (dir, api)
}

fn validate_with_base(frontmatter: &str) -> darkmatter::markdown::schemas::ValidationReport {
    let md: Markdown = format!("---\n{frontmatter}---\nBody\n").as_str().into();
    base_api().validate(&md).expect("validation must run")
}

fn parse_schema_yaml(raw: &str) -> serde_json::Value {
    let yaml: YamlValue = serde_yaml_ng::from_str(raw).expect("schema YAML must parse");
    let schema = parse_yaml_schema(&yaml).expect("SimplifiedSchema must parse");
    to_json_schema(&schema).expect("SimplifiedSchema must convert")
}

fn collect_style_catalog_paths(
    shape: &SchemaShape,
    prefix: &str,
    catalog: &BTreeSet<&str>,
    found: &mut BTreeSet<String>,
) {
    for (name, def) in &shape.properties {
        let path = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{prefix}.{name}")
        };
        if catalog.contains(path.as_str()) {
            found.insert(path);
            continue;
        }
        let atoms = match def {
            PropertyDef::Single(atom) => std::slice::from_ref(atom),
            PropertyDef::Union(atoms) => atoms.as_slice(),
        };
        for atom in atoms {
            if let TypeExpr::InlineObject(nested) = &atom.ty {
                collect_style_catalog_paths(nested, &path, catalog, found);
            }
        }
    }
}

#[test]
fn base_schema_file_parses_and_converts() {
    let schema = darkmatter_base_schema();
    let json = darkmatter_base_json_schema();

    match schema {
        darkmatter::markdown::schemas::SimplifiedSchema::Single(shape) => {
            assert!(shape.properties.contains_key("$schema"));
            assert!(shape.properties.contains_key("title"));
            assert!(shape.properties.contains_key("ctx"));
        }
        other => panic!("base schema must be a single object shape, got {other:?}"),
    }

    assert_eq!(json.get("type").and_then(|v| v.as_str()), Some("object"));
    assert!(
        json.get("properties")
            .and_then(|v| v.as_object())
            .is_some_and(|properties| properties.contains_key("ctx")),
        "base JSON Schema must expose ctx"
    );
}

#[test]
fn shipped_base_schema_exposes_the_nullable_generated_ctx_cwd_contract() {
    let darkmatter::markdown::schemas::SimplifiedSchema::Single(root) =
        darkmatter_base_schema()
    else {
        panic!("base schema must be a single object shape");
    };
    let PropertyDef::Single(ctx) = root.properties.get("ctx").expect("ctx property") else {
        panic!("ctx must be a single inline object");
    };
    let TypeExpr::InlineObject(ctx) = &ctx.ty else {
        panic!("ctx must expose its nested shape");
    };
    let PropertyDef::Single(cwd) = ctx.properties.get("cwd").expect("ctx.cwd property") else {
        panic!("ctx.cwd must be a single property atom");
    };

    assert!(matches!(
        cwd.ty,
        TypeExpr::Primitive(darkmatter::markdown::schemas::SimplifiedType::String)
    ));
    assert!(cwd.constraints.iter().any(|constraint| matches!(
        constraint,
        darkmatter::markdown::schemas::Constraint::Generated
    )));
    assert!(!cwd.constraints.iter().any(|constraint| matches!(
        constraint,
        darkmatter::markdown::schemas::Constraint::Required
    )));
}

#[test]
fn base_style_schema_matches_runtime_descriptor() {
    let darkmatter::markdown::schemas::SimplifiedSchema::Single(root) =
        darkmatter_base_schema()
    else {
        panic!("base schema must be a single object shape");
    };
    let style = root.properties.get("style").expect("style property");
    let PropertyDef::Single(style) = style else {
        panic!("style must be a single inline object");
    };
    let TypeExpr::InlineObject(style) = &style.ty else {
        panic!("style must expose its nested shape");
    };

    let expected: BTreeSet<&str> = darkmatter::style::descriptor::SCHEMA
        .iter()
        .map(|leaf| leaf.canonical)
        .collect();
    let mut actual = BTreeSet::new();
    collect_style_catalog_paths(style, "", &expected, &mut actual);

    assert_eq!(
        actual,
        expected.into_iter().map(str::to_string).collect(),
        "the authored base schema must stay in lockstep with style::descriptor::SCHEMA"
    );
}

#[test]
fn base_style_schema_validates_nested_values() {
    let valid = validate_with_base(
        "style:\n\
         \x20 block-quote:\n\
         \x20   alignment: center\n\
         \x20   border:\n\
         \x20     left: true\n\
         \x20     style: dotted\n\
         \x20   emphasis:\n\
         \x20     italic: true\n",
    );
    assert!(valid.valid, "valid nested style must pass: {:?}", valid.problems);

    let invalid = validate_with_base(
        "style:\n\
         \x20 block-quote:\n\
         \x20   alignment: middle\n",
    );
    assert!(!invalid.valid, "invalid nested style enum must fail");

    let invalid_meta = validate_with_base("style:\n  page:\n    meta: description\n");
    assert!(!invalid_meta.valid, "page metadata must be an object");
}

#[test]
fn claudine_catalog_resolves_from_its_bundle() {
    let md: Markdown = "---\nprompt: Explain.\n---\nBody\n".into();
    let (_dir, api) = claudine_api();
    api.effective_for(&md)
        .expect("Claudine's catalog must resolve its imports")
        .expect("a document with a prompt selects Claudine's catalog");

    let raw = include_str!("../../../../claudine/schemas/claudine.yaml");
    let yaml: YamlValue = serde_yaml_ng::from_str(raw).expect("schema YAML must parse");
    let properties = yaml
        .get("$schema")
        .and_then(YamlValue::as_mapping)
        .expect("schema must declare a property mapping");
    assert!(properties.contains_key("max_turns"));
    assert!(!properties.contains_key("maxTurns"));
    assert!(!properties.contains_key("output"));
    assert!(!properties.contains_key("stall_timeout"));
    assert!(
        properties
            .get("mode")
            .and_then(YamlValue::as_str)
            .is_some_and(|mode| mode.starts_with("enum(append, replace) ->")),
        "`mode` is the system-prompt delivery enum"
    );
    assert!(!raw.contains("boolish"));
}

#[test]
fn claudine_schema_accepts_typed_lifecycle_stack() {
    let md: Markdown = "---\nstart:\n  - info: Starting\n  - when: env.CI == 'true'\n    then:\n      - message: Build started\n      - shell:\n          command: cargo test\n          no_error: true\nsuccess:\n  info: Done\n  effect: tick\n---\nBody\n".into();
    let (_dir, api) = claudine_api();
    let report = api.validate(&md).expect("validation must run");

    assert!(
        report.valid,
        "valid lifecycle stack must pass: {:?}",
        report.problems
    );
}

#[test]
fn claudine_schema_rejects_malformed_lifecycle_stack() {
    let md: Markdown = "---\nstart: run-now\n---\nBody\n".into();
    let (_dir, api) = claudine_api();
    let report = api.validate(&md).expect("validation must run");

    assert!(!report.valid, "a lifecycle stack must be a list or a dictionary");
}

#[test]
fn claudine_schema_accepts_loop_controls_and_lifecycle_concerns_together() {
    let md: Markdown = "---\nloop:\n  while: step < 3\n  action:\n    op: increment\n    prop: step\n  info: Iteration complete\n  warn: Iteration ended\n---\nBody\n".into();
    let (_dir, api) = claudine_api();
    let report = api.validate(&md).expect("validation must run");

    assert!(
        report.valid,
        "loop controls and lifecycle concerns must compose: {:?}",
        report.problems
    );
}

#[test]
fn base_schema_validation_accepts_valid_examples_and_unknown_keys() {
    let report = validate_with_base(
        "title: Release Notes\n\
         description: Human-readable summary\n\
         tags: [release, darkmatter]\n\
         draft: false\n\
         custom_key: 42\n",
    );

    assert!(
        report.valid,
        "valid base frontmatter plus unknown keys must pass: {:?}",
        report.problems
    );
}

#[test]
fn base_schema_validation_rejects_invalid_known_property_values() {
    let report = validate_with_base("title: 42\ndraft: maybe\n");

    assert!(
        !report.valid,
        "invalid known property values must fail validation"
    );
    let diagnostics = report
        .problems
        .iter()
        .map(|problem| {
            format!(
                "{} {}",
                problem.path,
                problem.property.as_deref().unwrap_or(&problem.message)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        diagnostics.contains("title") || diagnostics.contains("draft"),
        "diagnostics should identify known invalid properties: {diagnostics}"
    );
}

#[test]
fn document_schema_definitions_override_baseline_properties() {
    let md: Markdown = "---\n$schema:\n  title: number\ntitle: 42\n---\nBody\n".into();
    let report = base_api().validate(&md).expect("validation must run");

    assert!(
        report.valid,
        "document schema should override baseline title type: {:?}",
        report.problems
    );
}

#[test]
fn schema_document_transcludes_same_file_as_library_source() {
    let doc_path = manifest_dir!()
        .join("../docs/schemas/darkmatter-schema.md")
        .canonicalize()
        .expect("schema docs path must exist");
    let schema_path = manifest_dir!()
        .join("../schemas/partials/fm.yaml")
        .canonicalize()
        .expect("schema YAML path must exist");
    let doc = std::fs::read_to_string(&doc_path).expect("schema docs must be readable");

    assert!(
        doc.contains("::code ../../schemas/partials/fm.yaml"),
        "schema docs must transclude the catalog's `fm` partial"
    );
    let transclusion_path = doc_path
        .parent()
        .expect("schema docs must have a parent")
        .join("../../schemas/partials/fm.yaml")
        .canonicalize()
        .expect("transclusion target must exist");
    assert_eq!(transclusion_path, schema_path);

    // The authored partial imports Content Policy's editor schema; the disk
    // resolver reads it from the checkout, the library from its embedded copy.
    let policy_path = biscuit_file::canonicalize_simplified(
        &manifest_dir!().join("../../content-policy/schemas/content-policy.yaml"),
    )
    .expect("policy schema path must exist");
    assert_eq!(
        include_str!("../../../../content-policy/schemas/content-policy.yaml"),
        content_policy::EDITOR_SCHEMA
    );
    // The baseline is the catalog's `fm` declaration, resolved from the
    // catalog's directory as the catalog declares it.
    let catalog_path = biscuit_file::canonicalize_simplified(&manifest_dir!().join("../schemas/darkmatter.yaml"))
        .expect("catalog path");
    let catalog_dir = catalog_path.parent().expect("the catalog has a parent");
    let declaration: YamlValue = serde_yaml_ng::from_str("fm: fm@./partials/fm.yaml").expect("declaration");
    let from_disk =
        resolve_yaml_schema(&declaration, catalog_dir, &crate::request_support::context_at(catalog_dir))
            .expect("the authored baseline resolves from disk");
    assert!(from_disk.imports.contains(&policy_path), "{:?}", from_disk.imports);
    // A catalog declaration stores no shared definitions, so the baseline it
    // yields is a definition-free shape; compare through the catalog loader.
    let catalog = darkmatter::markdown::schemas::catalog::GlobalCatalog::from_file(
        &catalog_path,
        darkmatter::markdown::schemas::catalog::DARKMATTER_GLOBALS,
        &crate::request_support::context_at(catalog_dir),
    )
    .expect("the authored catalog loads from disk");
    let from_disk = catalog.baseline().expect("the catalog declares `fm`");
    assert_eq!(from_disk, darkmatter_base_schema());
    assert_eq!(
        darkmatter::markdown::schemas::to_json_schema(&from_disk).expect("lowers"),
        darkmatter_base_json_schema()
    );
}

#[test]
fn nested_mapping_object_matches_inline_object_literal_json_schema() {
    let mapping = parse_schema_yaml("item:\n  foo: string\n  bar: number\n");
    let inline = parse_schema_yaml("item: \"{ foo: string, bar: number }\"\n");

    assert_eq!(mapping, inline);
}

#[test]
fn sequence_union_arms_accept_nested_mapping_object_shapes() {
    let md: Markdown = "---\n$schema:\n  choice:\n    - string\n    - kind: string(required)\n      value: number\nchoice:\n  kind: score\n  value: 10\n---\nBody\n".into();
    let report = DarkmatterSchemas::new(crate::request_support::cwd_context())
        .validate(&md)
        .expect("validation must run");

    assert!(
        report.valid,
        "mapping object union arm must validate: {:?}",
        report.problems
    );
}

#[test]
fn base_schema_ctx_is_darkmatter_owned_generated_context() {
    let report = validate_with_base("title: Context Example\n");
    assert!(
        report.valid,
        "authored frontmatter may omit ctx: {:?}",
        report.problems
    );

    let generated_ctx = validate_with_base("ctx:\n  today: 2026-07-04\n");
    assert!(
        generated_ctx.valid,
        "known generated ctx keys must pass when present: {:?}",
        generated_ctx.problems
    );

    let unknown_ctx = validate_with_base("ctx:\n  project_slug: biscuit\n");
    assert!(
        !unknown_ctx.valid,
        "custom user ctx keys are not part of the base schema"
    );

    let json = darkmatter_base_json_schema();
    let ctx = &json["properties"]["ctx"];
    let ctx_object = ctx
        .get("anyOf")
        .and_then(|any_of| any_of.as_array())
        .and_then(|arms| arms.iter().find(|arm| arm["type"].as_str() == Some("object")))
        .unwrap_or(ctx);
    assert_eq!(ctx_object["type"].as_str(), Some("object"));
    assert_eq!(ctx_object["additionalProperties"].as_bool(), Some(false));
    assert!(
        ctx_object["properties"]["today"]["x-darkmatter-generated"].as_bool() == Some(true),
        "base ctx must expose generated metadata for known runtime keys: {ctx_object:?}"
    );
}

/// The baseline's `style.markdown` declarations agree with the runtime rule
/// validator (`markdown::cleanup::rules`) on every legal and illegal shape,
/// so DMLS validation and cleanup never disagree about a setting. Both bind
/// a value through the one conversion engine first, so `"80"` is `80` and
/// `5` is `"5"` to each of them.
#[test]
fn base_style_markdown_schema_agrees_with_the_rule_validator() {
    let control = "style:\n  markdown:\n    frontmatter: false\n    table-layout: false\n    \
                   fixed-width: 80\n    unwrap: true\n    indentation: 2\n    ordered-lists: false\n    \
                   list-padding: false\n    list-item-padding: one\n    list-interior-padding: all\n    \
                   default-language: sh\n";
    let report = validate_with_base(control);
    assert!(report.valid, "control rules must pass: {:?}", report.problems);

    let cases: &[(&str, &str, bool)] = &[
        ("fixed-width", "true", true),
        ("fixed-width", "false", true),
        ("fixed-width", "1", true),
        ("fixed-width", "1000", true),
        ("fixed-width", "0", false),
        ("fixed-width", "1001", false),
        ("fixed-width", "80.5", false),
        ("fixed-width", "\"80\"", true),
        ("fixed-width", "[80]", false),
        ("indentation", "preserve", true),
        ("indentation", "4", true),
        ("indentation", "8", true),
        ("indentation", "3", false),
        ("indentation", "\"4\"", true),
        ("indentation", "\"\"", false),
        ("unwrap", "\"true\"", true),
        ("unwrap", "[true]", false),
        ("list-item-padding", "none", true),
        ("list-item-padding", "two", false),
        ("list-interior-padding", "except-nested-lists", true),
        ("list-interior-padding", "some", false),
        ("default-language", "\"\"", true),
        ("default-language", "plain", true),
        ("default-language", "[sh]", false),
        ("default-language", "{}", false),
        ("default-language", "5", true),
        ("list-item-padding", "1", false),
        ("list-padding", "\"false\"", true),
        ("fixed-width", "\"true\"", true),
        ("bogus-rule", "true", false),
    ];
    let mut disagreements = Vec::new();
    for (rule, value, expected) in cases {
        let yaml = format!("style:\n  markdown:\n    {rule}: {value}\n");
        let schema_valid = validate_with_base(&yaml).valid;
        let parsed: serde_json::Value = serde_yaml_ng::from_str::<YamlValue>(value)
            .map(|yaml| serde_json::to_value(yaml).unwrap())
            .unwrap();
        let runtime_valid = darkmatter::markdown::cleanup::rules::validate_rule(rule, &parsed).is_ok();
        if schema_valid != *expected || runtime_valid != *expected {
            disagreements.push(format!(
                "{rule}: {value} expected {expected}, schema {schema_valid}, runtime {runtime_valid}"
            ));
        }
    }
    assert!(disagreements.is_empty(), "{}", disagreements.join("; "));
}

/// The baseline documents each rule's runtime default, and that documented
/// default is what `ResolvedMarkdownRules::defaults()` actually uses.
#[test]
fn base_style_markdown_defaults_match_the_runtime_defaults() {
    use darkmatter::markdown::cleanup::rules::{
        FixedWidth, Indentation, ListInteriorPadding, ListItemPadding, ResolvedMarkdownRules,
        RuleSource,
    };

    fn first_default(value: &serde_json::Value) -> Option<serde_json::Value> {
        match value {
            serde_json::Value::Object(map) => map
                .get("default")
                .cloned()
                .or_else(|| map.values().find_map(first_default)),
            serde_json::Value::Array(items) => items.iter().find_map(first_default),
            _ => None,
        }
    }

    let schema = darkmatter_base_json_schema();
    let markdown = &schema["properties"]["style"];
    let markdown = std::iter::successors(Some(markdown), |node| match node {
        serde_json::Value::Object(map) => map
            .get("anyOf")
            .and_then(|arms| arms.as_array())
            .and_then(|arms| arms.iter().find(|arm| arm.get("properties").is_some())),
        _ => None,
    })
    .find(|node| node.get("properties").is_some())
    .expect("style object")["properties"]["markdown"]
        .clone();
    assert!(!markdown.is_null(), "style.markdown must be declared");

    let defaults = ResolvedMarkdownRules::defaults();
    let runtime = [
        ("frontmatter", serde_json::json!(defaults.frontmatter.value)),
        ("table-layout", serde_json::json!(defaults.table_layout.value)),
        (
            "fixed-width",
            serde_json::json!(defaults.fixed_width.value != FixedWidth::Off),
        ),
        ("unwrap", serde_json::json!(defaults.unwrap.value)),
        (
            "indentation",
            serde_json::json!(defaults.indentation.value.spaces().expect("numeric default")),
        ),
        ("ordered-lists", serde_json::json!(defaults.ordered_lists.value)),
        ("list-padding", serde_json::json!(defaults.list_padding.value)),
        (
            "list-item-padding",
            serde_json::json!(match defaults.list_item_padding.value {
                ListItemPadding::Preserve => "preserve",
                ListItemPadding::None => "none",
                ListItemPadding::One => "one",
            }),
        ),
        (
            "list-interior-padding",
            serde_json::json!(match defaults.list_interior_padding.value {
                ListInteriorPadding::Preserve => "preserve",
                ListInteriorPadding::None => "none",
                ListInteriorPadding::ExceptNestedLists => "except-nested-lists",
                ListInteriorPadding::All => "all",
            }),
        ),
        (
            "default-language",
            serde_json::json!(defaults.default_language.value.as_str()),
        ),
    ];
    assert_eq!(defaults.indentation.value, Indentation::Four);
    for (rule, expected) in runtime {
        let property = markdown
            .pointer(&format!("/properties/{rule}"))
            .or_else(|| {
                std::iter::successors(Some(&markdown), |node| {
                    node.get("anyOf")
                        .and_then(|arms| arms.as_array())
                        .and_then(|arms| arms.iter().find(|arm| arm.get("properties").is_some()))
                })
                .find_map(|node| node.pointer(&format!("/properties/{rule}")))
            })
            .unwrap_or_else(|| panic!("baseline must declare style.markdown.{rule}"));
        assert_eq!(
            first_default(property),
            Some(expected),
            "style.markdown.{rule}: the baseline default must equal the runtime default"
        );
    }
    let sources = [
        defaults.frontmatter.source,
        defaults.indentation.source,
        defaults.default_language.source,
    ];
    assert!(sources.iter().all(|source| *source == RuleSource::Default));
}
