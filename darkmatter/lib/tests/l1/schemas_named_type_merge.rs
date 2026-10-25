//! Public resolution evidence for reused named types: inherited presence,
//! bare local references, and use-site constraint merging.

use std::path::{Path, PathBuf};

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::schemas::resolve::{ResolvedSchema, expand_named_types, resolve_yaml_schema};
use darkmatter::markdown::schemas::{
    Constraint, DarkmatterSchemas, PropertyAtom, PropertyDef, SchemaConstraintConflictKind,
    SchemaError, SchemaPhase, SchemaSourceId, SchemaSourcePath, SimplifiedSchema,
    SuggestionLintReason, ValidationProblemCode, lint_suggestions,
    parse_standalone_schema_document,
};
use serde_json::{Value, json};
use serde_yaml_ng::Value as YamlValue;
use tempfile::TempDir;

/// A valid example artifact whose `invocation` identifies it.
fn example_artifact(invocation: &str) -> String {
    format!("kind: example\ninvocation: \"{invocation}\"\nreturns: \"x\"\ndescription: d\n")
}

fn write(dir: &Path, name: &str, body: &str) -> PathBuf {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create fixture directory");
    }
    std::fs::write(&path, body).expect("write fixture");
    path
}

fn canonical(path: &Path) -> PathBuf {
    biscuit_file::canonicalize_simplified(path).expect("fixture path canonicalizes")
}

fn path(segments: &[&str]) -> SchemaSourcePath {
    segments
        .iter()
        .fold(SchemaSourcePath::root(), |path, segment| path.property(*segment))
}

/// Resolves an inline `$schema` mapping (the YAML body of the mapping) from
/// `dir`, the way a document in `dir` would.
fn resolve_inline(dir: &Path, schema: &str) -> Result<ResolvedSchema, SchemaError> {
    let value: YamlValue = serde_yaml_ng::from_str(schema).expect("fixture YAML parses");
    resolve_yaml_schema(&value, dir, &FileResolutionContext::new(dir))
}

/// Resolves `$schema: <reference>` from a document in `dir`.
fn resolve_reference(dir: &Path, reference: &str) -> Result<ResolvedSchema, SchemaError> {
    resolve_yaml_schema(
        &YamlValue::String(reference.to_string()),
        dir,
        &FileResolutionContext::new(dir),
    )
}

fn properties(resolved: &ResolvedSchema) -> &serde_json::Map<String, Value> {
    resolved.json_schema["properties"]
        .as_object()
        .expect("resolved schema has properties")
}

fn required(resolved: &ResolvedSchema) -> Vec<String> {
    resolved.json_schema["required"]
        .as_array()
        .map(|names| {
            names
                .iter()
                .map(|name| name.as_str().expect("required names are strings").to_string())
                .collect()
        })
        .unwrap_or_default()
}

/// The resolved single atom of a top-level property.
fn atom<'a>(resolved: &'a ResolvedSchema, property: &str) -> &'a PropertyAtom {
    let Some(SimplifiedSchema::Single(shape)) = &resolved.simplified else {
        panic!("expected a single simplified shape");
    };
    match &shape.properties[property] {
        PropertyDef::Single(atom) => atom,
        PropertyDef::Union(_) => panic!("`{property}` resolved to a union"),
    }
}

fn effective_for_document(dir: &Path, schema_block: &str) -> darkmatter::markdown::schemas::EffectiveSchema {
    let source: Markdown = format!("---\n$schema:{schema_block}\n---\nbody\n").into();
    DarkmatterSchemas::new(FileResolutionContext::new(dir))
        .effective_for(&source)
        .expect("schema resolves")
        .expect("effective schema exists")
}

fn missing_required(report: &darkmatter::markdown::schemas::ValidationReport) -> Vec<String> {
    let mut names: Vec<String> = report
        .problems
        .iter()
        .filter(|problem| problem.code == ValidationProblemCode::MissingRequired)
        .filter_map(|problem| problem.property.clone())
        .collect();
    names.sort();
    names
}

// ── AC1: presence survives bare, `@this`, and cross-file references ───────

#[test]
fn required_and_generated_survive_every_reference_spelling() {
    let dir = TempDir::new().expect("tempdir");
    write(
        dir.path(),
        "types.yaml",
        "kind: schema\ntypes:\n  code: string(required; generated)\n",
    );
    write(
        dir.path(),
        "schema.yaml",
        "kind: schema\ntypes:\n  code: string(required; generated)\n  a: code\n  b: code@this\n  c: code@./types.yaml\n",
    );

    let resolved = resolve_reference(dir.path(), "./schema.yaml").expect("schema resolves");
    let props = properties(&resolved);
    for name in ["a", "b", "c"] {
        assert_eq!(
            props[name],
            json!({"type": "string", "x-darkmatter-generated": true}),
            "`{name}` must be the non-nullable generated fragment"
        );
        assert_eq!(
            atom(&resolved, name).constraints,
            vec![Constraint::Required, Constraint::Generated],
            "`{name}` must keep both presence constraints"
        );
    }

    let schema = effective_for_document(dir.path(), " ./schema.yaml");
    // Authored validation lets a generated property be absent.
    assert!(schema.validate(&json!({})).valid);
    // Runtime validation enforces the inherited `required`.
    let completion = schema
        .validate_for_phase(&json!({}), SchemaPhase::Completion)
        .expect("phase schema builds");
    assert_eq!(missing_required(&completion), ["a", "b", "c", "code"]);
    // A supplied invalid value fails in both phases.
    let invalid = json!({"code": "x", "a": [1], "b": "x", "c": "x"});
    assert!(!schema.validate(&invalid).valid);
    assert!(
        !schema
            .validate_for_phase(&invalid, SchemaPhase::Completion)
            .expect("phase schema builds")
            .valid
    );
}

// ── AC2: the spike case ───────────────────────────────────────────────────

#[test]
fn required_expression_type_is_required_at_top_level_and_inside_a_mapping_type() {
    let dir = TempDir::new().expect("tempdir");
    let resolved = resolve_inline(
        dir.path(),
        "req: expression(required)\nwhen: req\nblock:\n  when: req\nflow: '{ when: req }'\n",
    )
    .expect("schema resolves");
    assert!(required(&resolved).contains(&"when".to_string()));
    let props = properties(&resolved);
    for nested in ["block", "flow"] {
        // The enclosing object is optional, so its object arm is the second
        // `anyOf` entry; inside it, `when` is required.
        assert_eq!(
            props[nested]["anyOf"][1]["required"],
            json!(["when"]),
            "`{nested}.when` must inherit `required`"
        );
    }
}

// ── AC3: presence coverage ────────────────────────────────────────────────

#[test]
fn each_presence_combination_is_inherited_as_declared() {
    let dir = TempDir::new().expect("tempdir");
    let resolved = resolve_inline(
        dir.path(),
        "r: string(required)\ng: string(generated)\nrg: string(required; generated)\nu: string\n\
         use_r: r\nuse_g: g\nuse_rg: rg\nuse_u: u\n",
    )
    .expect("schema resolves");
    let props = properties(&resolved);
    assert_eq!(props["use_r"], json!({"type": "string"}));
    assert_eq!(
        props["use_g"],
        json!({"anyOf": [{"type": "null"}, {"type": "string"}], "x-darkmatter-generated": true})
    );
    assert_eq!(props["use_rg"], json!({"type": "string", "x-darkmatter-generated": true}));
    assert_eq!(props["use_u"], json!({"anyOf": [{"type": "null"}, {"type": "string"}]}));
    assert_eq!(required(&resolved), ["r", "use_r"]);
}

#[test]
fn multi_hop_chains_inherit_presence_locally_and_across_files() {
    let dir = TempDir::new().expect("tempdir");
    write(
        dir.path(),
        "types.yaml",
        "kind: schema\ntypes:\n  base: number(required)\n  mid: base\n",
    );
    let resolved = resolve_inline(
        dir.path(),
        "base: number(required)\nmid: base\nlocal: mid@this\nremote: mid@./types.yaml\n",
    )
    .expect("schema resolves");
    let props = properties(&resolved);
    assert_eq!(props["local"], json!({"type": "number"}));
    assert_eq!(props["remote"], json!({"type": "number"}));
    assert_eq!(required(&resolved), ["base", "mid", "local", "remote"]);
}

#[test]
fn a_required_inner_field_does_not_require_the_enclosing_property() {
    let dir = TempDir::new().expect("tempdir");
    let resolved = resolve_inline(
        dir.path(),
        "person: '{ name: string(required), email: email }'\nowner: person\n",
    )
    .expect("schema resolves");
    let owner = &properties(&resolved)["owner"];
    assert_eq!(owner["anyOf"][0], json!({"type": "null"}), "`owner` stays optional");
    assert_eq!(owner["anyOf"][1]["required"], json!(["name"]));
    assert!(required(&resolved).is_empty());
}

#[test]
fn an_array_of_a_required_type_is_a_required_property() {
    let dir = TempDir::new().expect("tempdir");
    let resolved = resolve_inline(
        dir.path(),
        "member: '{ name: string(required) }(required)'\npeople: member[]\n",
    )
    .expect("schema resolves");
    let people = &properties(&resolved)["people"];
    assert_eq!(people["type"], "array", "`people` is required, so not null-wrapped: {people}");
    assert_eq!(people["items"]["required"], json!(["name"]));
    assert_eq!(required(&resolved), ["member", "people"]);
}

#[test]
fn union_arms_hoist_inherited_required() {
    let dir = TempDir::new().expect("tempdir");
    let resolved = resolve_inline(
        dir.path(),
        "choice: [string(required), number]\nreq: string(required)\n\
         whole: choice\narm: [req, number]\n",
    )
    .expect("schema resolves");
    let props = properties(&resolved);
    for name in ["whole", "arm"] {
        assert_eq!(
            props[name],
            json!({"anyOf": [{"type": "string"}, {"type": "number"}]}),
            "`{name}` hoists `required` and drops the null arm"
        );
    }
    assert_eq!(required(&resolved), ["choice", "req", "whole", "arm"]);
}

// ── AC4: constraint merging ───────────────────────────────────────────────

#[test]
fn string_length_bounds_are_read_against_the_resolved_target_and_keep_the_stricter_value() {
    let dir = TempDir::new().expect("tempdir");
    let resolved = resolve_inline(
        dir.path(),
        "label: string(min(3); required)\ntitle: label(max(80))\nstricter: label(min(5))\n\
         looser: label(min(1))\nexact: label(max(3))\n",
    )
    .expect("schema resolves");
    let props = properties(&resolved);
    assert_eq!(props["title"], json!({"type": "string", "minLength": 3, "maxLength": 80}));
    assert_eq!(props["stricter"], json!({"type": "string", "minLength": 5}));
    assert_eq!(props["looser"], json!({"type": "string", "minLength": 3}));
    // Equal bounds are valid.
    assert_eq!(props["exact"], json!({"type": "string", "minLength": 3, "maxLength": 3}));
}

#[test]
fn a_fractional_length_bound_on_a_string_target_is_rejected() {
    let dir = TempDir::new().expect("tempdir");
    let err = resolve_inline(dir.path(), "label: string\ntitle: label(max(2.5))\n")
        .expect_err("a fractional length must fail");
    let SchemaError::Convert { property, message } = err else {
        panic!("expected a Convert error, got {err:?}");
    };
    assert_eq!(property, "title");
    assert!(message.contains("non-negative integer"), "{message}");
}

#[test]
fn number_bounds_keep_the_stricter_value() {
    let dir = TempDir::new().expect("tempdir");
    let resolved = resolve_inline(
        dir.path(),
        "count: number(min(1); max(100); required)\nsmall: count(max(10))\n\
         big: count(min(50); max(200))\n",
    )
    .expect("schema resolves");
    let props = properties(&resolved);
    assert_eq!(props["small"], json!({"type": "number", "minimum": 1, "maximum": 10}));
    assert_eq!(props["big"], json!({"type": "number", "minimum": 50, "maximum": 100}));
}

#[test]
fn item_count_bounds_merge_on_an_already_array_target_and_never_with_item_bounds() {
    let dir = TempDir::new().expect("tempdir");
    let resolved = resolve_inline(
        dir.path(),
        "tags: string[](min(1); max(10); required)\nfew: tags(max(3))\n\
         label: string(min(3))\nlabels: label[](min(1); required)\n\
         picks: tags(suggest(a, b))\n",
    )
    .expect("schema resolves");
    let props = properties(&resolved);
    assert_eq!(
        props["few"],
        json!({"type": "array", "items": {"type": "string"}, "minItems": 1, "maxItems": 3})
    );
    // Suggestion candidates describe items, so they stay on the items.
    assert_eq!(props["picks"]["items"]["x-darkmatter-suggest"], json!(["a", "b"]));
    assert_eq!(
        props["labels"],
        json!({"type": "array", "items": {"type": "string", "minLength": 3}, "minItems": 1})
    );
}

#[test]
fn key_bounds_from_constraints_blocks_and_postfixes_are_one_scope() {
    let dir = TempDir::new().expect("tempdir");
    let resolved = resolve_inline(
        dir.path(),
        "card:\n  $constraints:\n    min-keys: 1\n    max-keys: 5\n  a: string\n  b: string\n  c: string\n\
         tight: card(max-keys(2))\nfull: card(min-keys(3); max-keys(9))\n\
         postfixed: '{ a: string, b: string }(min-keys(1))'\nboth: postfixed(min-keys(2))\n",
    )
    .expect("schema resolves");
    let props = properties(&resolved);
    let object_arm = |name: &str| props[name]["anyOf"][1].clone();
    assert_eq!(object_arm("tight")["minProperties"], 1);
    assert_eq!(object_arm("tight")["maxProperties"], 2);
    assert_eq!(object_arm("full")["minProperties"], 3);
    assert_eq!(object_arm("full")["maxProperties"], 5);
    assert_eq!(object_arm("both")["minProperties"], 2);

    let err = resolve_inline(
        dir.path(),
        "card:\n  $constraints:\n    min-keys: 3\n  a: string\nbox: card(max-keys(2))\n",
    )
    .expect_err("a `$constraints` minimum above a postfix maximum must fail");
    let SchemaError::ConstraintConflict { kind, constraints, .. } = err else {
        panic!("expected ConstraintConflict, got {err:?}");
    };
    assert_eq!(kind, SchemaConstraintConflictKind::Bounds);
    let paths: Vec<_> = constraints.iter().map(|origin| origin.path.clone()).collect();
    assert_eq!(paths, [path(&["card", "$constraints", "min-keys"]), path(&["box"])]);
}

/// The two-file provenance fixture: conflicts point at the declarations that
/// introduced each value, including through a multi-hop reference.
fn write_provenance_types(dir: &Path) -> PathBuf {
    write(
        dir,
        "types.yaml",
        "kind: schema\ntypes:\n  label: \"number(min(10))\"\n  title_base: label@this\n  \
         req: expression(required)\n  card:\n    $constraints:\n      min-keys: 3\n    a: string\n",
    )
}

#[test]
fn a_multi_hop_bound_conflict_names_both_origins_with_the_defining_span() {
    let dir = TempDir::new().expect("tempdir");
    let types = write_provenance_types(dir.path());
    let types_text = std::fs::read_to_string(&types).expect("read fixture");
    let err = resolve_inline(dir.path(), "title: title_base(max(5))@./types.yaml\n")
        .expect_err("min(10) and max(5) contradict");
    let SchemaError::ConstraintConflict {
        name,
        kind,
        site,
        constraints,
    } = err
    else {
        panic!("expected ConstraintConflict, got {err:?}");
    };
    assert_eq!(name, "title_base");
    assert_eq!(kind, SchemaConstraintConflictKind::Bounds);
    assert_eq!(site.source, SchemaSourceId::InlineDocument);
    assert_eq!(site.path, path(&["title"]));
    assert_eq!(constraints.len(), 2);

    let inherited = &constraints[0];
    assert_eq!(inherited.source, SchemaSourceId::File(canonical(&types)));
    assert_eq!(inherited.path, path(&["label"]), "the declaration that introduced `min`");
    assert_eq!(inherited.keyword, "min");
    let span = inherited.span.clone().expect("a file origin carries its span");
    assert_eq!(&types_text[span], "min(10)");

    let use_site = &constraints[1];
    assert_eq!(use_site.source, SchemaSourceId::InlineDocument);
    assert_eq!(use_site.path, path(&["title"]));
    assert_eq!(use_site.keyword, "max");
    assert_eq!(use_site.span, None, "inline origins are projected by the consumer");
}

#[test]
fn a_constraints_block_conflict_points_at_the_block_entry() {
    let dir = TempDir::new().expect("tempdir");
    let types = write_provenance_types(dir.path());
    let types_text = std::fs::read_to_string(&types).expect("read fixture");
    let err = resolve_inline(
        dir.path(),
        "when: req@./types.yaml\nbox: card(max-keys(2))@./types.yaml\n",
    )
    .expect_err("min-keys 3 and max-keys 2 contradict");
    let SchemaError::ConstraintConflict {
        name, constraints, ..
    } = err
    else {
        panic!("expected ConstraintConflict, got {err:?}");
    };
    assert_eq!(name, "card");
    assert_eq!(constraints[0].path, path(&["card", "$constraints", "min-keys"]));
    assert_eq!(constraints[0].keyword, "min-keys");
    let span = constraints[0].span.clone().expect("file origin span");
    assert_eq!(&types_text[span], "min-keys: 3");
    assert_eq!(constraints[1].path, path(&["box"]));
    assert_eq!(constraints[1].keyword, "max-keys");
}

#[test]
fn a_conflict_inside_a_referenced_schema_keeps_the_referenced_wrapper() {
    let dir = TempDir::new().expect("tempdir");
    write_provenance_types(dir.path());
    let wrapper = write(
        dir.path(),
        "wrapper.yaml",
        "$schema:\n  title: title_base(max(5))@./types.yaml\n",
    );
    let err = resolve_reference(dir.path(), "./wrapper.yaml").expect_err("conflict");
    let SchemaError::ReferencedSchema { path: wrapped, source } = err else {
        panic!("expected ReferencedSchema, got {err:?}");
    };
    assert_eq!(wrapped, canonical(&wrapper));
    let SchemaError::ConstraintConflict { site, .. } = *source else {
        panic!("expected a wrapped ConstraintConflict, got {source:?}");
    };
    assert_eq!(site.source, SchemaSourceId::File(canonical(&wrapper)));
    assert_eq!(site.path, path(&["title"]));
}

#[test]
fn schemes_intersect_and_an_empty_intersection_is_a_conflict() {
    let dir = TempDir::new().expect("tempdir");
    let resolved = resolve_inline(
        dir.path(),
        "link: url(scheme(http, https); required)\nsecure: link(scheme(https, ftp))\n",
    )
    .expect("schema resolves");
    assert_eq!(properties(&resolved)["secure"]["x-darkmatter-url-scheme"], json!(["https"]));

    let err = resolve_inline(dir.path(), "link: url(scheme(http, https))\nftp: link(scheme(ftp))\n")
        .expect_err("disjoint schemes contradict");
    let SchemaError::ConstraintConflict { kind, constraints, .. } = err else {
        panic!("expected ConstraintConflict, got {err:?}");
    };
    assert_eq!(kind, SchemaConstraintConflictKind::Scheme);
    assert_eq!(constraints[0].path, path(&["link"]));
    assert_eq!(constraints[1].path, path(&["ftp"]));
}

#[test]
fn defaults_must_be_equal_after_number_normalization() {
    let dir = TempDir::new().expect("tempdir");
    let resolved = resolve_inline(dir.path(), "n: number(default(3))\nsame: n(default(3.0))\n")
        .expect("3 and 3.0 are one default");
    assert_eq!(properties(&resolved)["same"]["default"], json!(3));
    assert_eq!(
        atom(&resolved, "same")
            .constraints
            .iter()
            .filter(|constraint| matches!(constraint, Constraint::Default(_)))
            .count(),
        1
    );

    let err = resolve_inline(dir.path(), "n: number(default(3))\ntext: 'n(default(\"3\"))'\n")
        .expect_err("a string and a number differ");
    assert!(
        matches!(
            err,
            SchemaError::ConstraintConflict {
                kind: SchemaConstraintConflictKind::Default,
                ..
            }
        ),
        "{err:?}"
    );
}

#[test]
fn programmatic_defaults_merge_when_their_lowered_numbers_are_equal() {
    for (inherited, supplied) in [
        (json!(9_223_372_036_854_775_808_u64), json!(9_223_372_036_854_775_808.0)),
        (json!(10_000_000_000_000_000_000_u64), json!(1e19)),
        (json!({"values": [9_223_372_036_854_775_808_u64]}), json!({"values": [9_223_372_036_854_775_808.0]})),
    ] {
        let dir = TempDir::new().expect("tempdir");
        let value: YamlValue = serde_yaml_ng::from_str("base: unknown\ncopy: base\n").unwrap();
        let mut schema = darkmatter::markdown::schemas::parse_yaml_schema(&value).unwrap();
        let SimplifiedSchema::Single(shape) = &mut schema else {
            panic!("single schema");
        };
        // Programmatic callers can supply floats directly; authored literals have
        // already passed numeric normalization in the grammar.
        for (name, default) in [("base", inherited.clone()), ("copy", supplied)] {
            let PropertyDef::Single(atom) = shape.properties.get_mut(name).unwrap() else {
                panic!("single property");
            };
            atom.constraints.push(Constraint::Default(default));
        }
        let expansion = expand_named_types(
            &schema,
            dir.path(),
            None,
            &[],
            &FileResolutionContext::new(dir.path()),
        )
        .expect("defaults lowering to the same number must merge");
        let lowered = darkmatter::markdown::schemas::to_json_schema(&expansion.schema).unwrap();
        assert_eq!(lowered["properties"]["copy"]["default"], inherited);
    }
}

#[test]
fn a_programmatic_float_at_the_signed_boundary_does_not_equal_the_smaller_integer() {
    let dir = TempDir::new().expect("tempdir");
    let value: YamlValue = serde_yaml_ng::from_str("base: unknown\ncopy: base\n").unwrap();
    let mut schema = darkmatter::markdown::schemas::parse_yaml_schema(&value).unwrap();
    let SimplifiedSchema::Single(shape) = &mut schema else {
        panic!("single schema");
    };
    for (name, default) in [("base", json!(i64::MAX)), ("copy", json!(9_223_372_036_854_775_808.0))] {
        let PropertyDef::Single(atom) = shape.properties.get_mut(name).unwrap() else {
            panic!("single property");
        };
        atom.constraints.push(Constraint::Default(default));
    }
    let error = expand_named_types(
        &schema,
        dir.path(),
        None,
        &[],
        &FileResolutionContext::new(dir.path()),
    )
    .expect_err("distinct defaults must not merge through a saturating cast");
    assert!(matches!(
        error,
        SchemaError::ConstraintConflict { kind: SchemaConstraintConflictKind::Default, .. }
    ), "{error:?}");
}

#[test]
fn repeated_patterns_and_match_groups_are_all_kept() {
    let dir = TempDir::new().expect("tempdir");
    let resolved = resolve_inline(
        dir.path(),
        "code: string(pattern(^a); not-empty)\nboth: code(pattern(z$); not-empty; pattern(^a))\n\
         doc: \"file(match('*.md'))\"\nguide: \"doc(match('guide*'))\"\n",
    )
    .expect("schema resolves");
    assert_eq!(
        atom(&resolved, "both").constraints,
        vec![
            Constraint::Pattern("^a".into()),
            Constraint::NotEmpty,
            Constraint::Pattern("z$".into()),
        ]
    );
    assert_eq!(
        atom(&resolved, "guide").constraints,
        vec![
            Constraint::Match(vec!["*.md".into()]),
            Constraint::Match(vec!["guide*".into()]),
        ]
    );
}

#[test]
fn inherited_examples_resolve_from_their_defining_file_and_come_first() {
    let dir = TempDir::new().expect("tempdir");
    write(dir.path(), "sub/ex.yaml", &example_artifact("inherited"));
    write(dir.path(), "local-ex.yaml", &example_artifact("use-site"));
    let types = write(
        dir.path(),
        "sub/types.yaml",
        "kind: schema\ntypes:\n  note: string(example(./ex.yaml))\n",
    );
    let resolved = resolve_inline(
        dir.path(),
        "plain: note@./sub/types.yaml\nmore: note(example(./local-ex.yaml))@./sub/types.yaml\n",
    )
    .expect("examples resolve from their defining files");
    let props = properties(&resolved);
    assert_eq!(props["plain"]["x-darkmatter-example"][0]["invocation"], "inherited");
    let more: Vec<_> = props["more"]["x-darkmatter-example"]
        .as_array()
        .expect("resolved examples")
        .iter()
        .map(|example| example["invocation"].clone())
        .collect();
    assert_eq!(more, [json!("inherited"), json!("use-site")]);
    assert_eq!(
        resolved.examples,
        [canonical(&dir.path().join("local-ex.yaml")), canonical(&dir.path().join("sub/ex.yaml"))]
    );
    assert_eq!(resolved.imports, [canonical(&types)]);
}

#[test]
fn duplicate_suggestions_through_different_union_arms_fail() {
    let dir = TempDir::new().expect("tempdir");
    let err = resolve_inline(
        dir.path(),
        "a: string(suggest(x))\nb: string(suggest(y))\np: [a, b]\n",
    )
    .expect_err("two suggestions on one property");
    let SchemaError::Grammar { property, message, .. } = err else {
        panic!("expected a Grammar error, got {err:?}");
    };
    assert_eq!(property, "p");
    assert!(message.contains("at most one `suggest`"), "{message}");
}

// ── AC5: bare local references ────────────────────────────────────────────

#[test]
fn bare_and_explicit_local_references_resolve_identically() {
    let dir = TempDir::new().expect("tempdir");
    let resolved = resolve_inline(
        dir.path(),
        "fwd_bare: later\nfwd_this: later@this\n\
         label: string(min(2)) -> inherited label\nchoice: [string, number]\n\
         scalar_bare: label\nscalar_this: label@this\n\
         array_bare: label[]\narray_this: label[]@this\n\
         constrained_bare: label(max(9); required)\nconstrained_this: label(max(9); required)@this\n\
         described_bare: label -> a label\ndescribed_this: label@this -> a label\n\
         union_bare: choice\nunion_this: choice@this\n\
         object_bare: '{ inner: label }'\nobject_this: '{ inner: label@this }'\n\
         later: number\n",
    )
    .expect("schema resolves");
    let props = properties(&resolved);
    for form in ["fwd", "scalar", "array", "constrained", "described", "union", "object"] {
        assert_eq!(
            props[&format!("{form}_bare")],
            props[&format!("{form}_this")],
            "`{form}` must resolve identically in both spellings"
        );
    }
    assert_eq!(props["scalar_bare"]["description"], "inherited label");
    assert_eq!(props["described_bare"]["description"], "a label");
    let required = required(&resolved);
    assert!(required.contains(&"constrained_bare".to_string()));
    assert!(required.contains(&"constrained_this".to_string()));
    assert!(resolved.imports.is_empty(), "local references add no dependency edge");
}

#[test]
fn a_bare_keyword_is_the_built_in_and_keyword_at_this_is_the_local_type() {
    let dir = TempDir::new().expect("tempdir");
    let resolved = resolve_inline(
        dir.path(),
        "string: number(min(1))\nbuiltin: string\nlocal: string@this\n",
    )
    .expect("schema resolves");
    let props = properties(&resolved);
    assert_eq!(props["builtin"], json!({"anyOf": [{"type": "null"}, {"type": "string"}]}));
    assert_eq!(
        props["local"],
        json!({"anyOf": [{"type": "null"}, {"type": "number", "minimum": 1}]})
    );
}

#[test]
fn an_unknown_bare_name_is_a_typed_error_naming_the_type_and_schema() {
    let dir = TempDir::new().expect("tempdir");
    let err = resolve_inline(dir.path(), "title: nope\n").expect_err("unknown name");
    let SchemaError::UnknownNamedType {
        name,
        namespace,
        site,
    } = err
    else {
        panic!("expected UnknownNamedType, got {err:?}");
    };
    assert_eq!(name, "nope");
    assert_eq!(namespace, SchemaSourceId::InlineDocument);
    assert_eq!(site.source, SchemaSourceId::InlineDocument);
    assert_eq!(site.path, path(&["title"]));

    // The same reference in a nested inline object of a referenced schema
    // file names that file and keeps the referenced-schema wrapper.
    let schema = write(dir.path(), "schema.yaml", "$schema:\n  outer:\n    title: nope\n");
    let err = resolve_reference(dir.path(), "./schema.yaml").expect_err("unknown name");
    let SchemaError::ReferencedSchema { path: wrapped, source } = err else {
        panic!("expected ReferencedSchema, got {err:?}");
    };
    assert_eq!(wrapped, canonical(&schema));
    let SchemaError::UnknownNamedType {
        name,
        namespace,
        site,
    } = *source
    else {
        panic!("expected a wrapped UnknownNamedType, got {source:?}");
    };
    assert_eq!(name, "nope");
    assert_eq!(namespace, SchemaSourceId::File(canonical(&schema)));
    assert_eq!(site.path, path(&["outer", "title"]));
    assert!(
        SchemaError::UnknownNamedType {
            name,
            namespace,
            site
        }
        .to_string()
        .contains("schema.yaml"),
        "the message names the schema file"
    );
}

#[test]
fn an_unknown_name_in_a_root_union_arm_is_addressed_through_the_arm() {
    let dir = TempDir::new().expect("tempdir");
    let value: YamlValue =
        serde_yaml_ng::from_str("- title: string\n- body: nope\n").expect("fixture YAML parses");
    let err = resolve_yaml_schema(&value, dir.path(), &FileResolutionContext::new(dir.path()))
        .expect_err("unknown name");
    let SchemaError::UnknownNamedType { site, .. } = err else {
        panic!("expected UnknownNamedType, got {err:?}");
    };
    assert_eq!(site.path, SchemaSourcePath::root().union_arm(1).property("body"));
}

#[test]
fn an_unknown_name_inside_an_imported_definition_names_its_defining_file() {
    let dir = TempDir::new().expect("tempdir");
    let types = write(
        dir.path(),
        "types.yaml",
        "kind: schema\ntypes:\n  wrapper:\n    arms: [string, missing]\n",
    );
    let err = resolve_inline(dir.path(), "w: wrapper@./types.yaml\n").expect_err("unknown name");
    let SchemaError::UnknownNamedType {
        namespace, site, ..
    } = err
    else {
        panic!("expected UnknownNamedType, got {err:?}");
    };
    assert_eq!(namespace, SchemaSourceId::File(canonical(&types)));
    assert_eq!(site.source, SchemaSourceId::File(canonical(&types)));
    assert_eq!(site.path, path(&["wrapper", "arms"]).union_arm(1));
}

#[test]
fn a_bare_name_inside_an_imported_definition_resolves_in_its_defining_file() {
    let dir = TempDir::new().expect("tempdir");
    let types = write(
        dir.path(),
        "types.yaml",
        "kind: schema\ntypes:\n  postcode: string(min(5); required)\n  address: '{ postcode: postcode }'\n",
    );
    let resolved = resolve_inline(
        dir.path(),
        "postcode: number\naddr: address@./types.yaml\n",
    )
    .expect("schema resolves");
    assert_eq!(
        properties(&resolved)["addr"]["anyOf"][1]["properties"]["postcode"],
        json!({"type": "string", "minLength": 5})
    );
    assert_eq!(resolved.imports, [canonical(&types)], "only the imported file is an edge");
}

#[test]
fn a_schema_file_with_only_bare_references_expands() {
    let dir = TempDir::new().expect("tempdir");
    write(
        dir.path(),
        "schema.yaml",
        "kind: schema\ntypes:\n  base: string(required)\n  title: base\n",
    );
    let resolved = resolve_reference(dir.path(), "./schema.yaml").expect("schema resolves");
    assert_eq!(properties(&resolved)["title"], json!({"type": "string"}));
    assert_eq!(required(&resolved), ["base", "title"]);
    assert!(resolved.imports.is_empty());
}

#[test]
fn a_cycle_through_bare_and_explicit_references_reports_the_chain() {
    let dir = TempDir::new().expect("tempdir");
    let err = resolve_inline(dir.path(), "a: b\nb: a@this\n").expect_err("cycle");
    let SchemaError::NonDescendingCycle { cycles } = &err else {
        panic!("expected NonDescendingCycle, got {err:?}");
    };
    let references: Vec<(String, &str)> = cycles[0]
        .iter()
        .map(|reference| (reference.site.path.to_string(), reference.target.as_str()))
        .collect();
    assert_eq!(references, [("b".to_string(), "a"), ("a".to_string(), "b")], "{err:?}");
}

// ── AC10: retained restrictions and unchanged output ──────────────────────

#[test]
fn already_array_suffix_remains_rejected() {
    let dir = TempDir::new().expect("tempdir");
    let schema = "list: string[]\np: list[]\n";
    let err = resolve_inline(dir.path(), schema).expect_err(schema);
    assert!(matches!(err, SchemaError::Convert { .. }), "{schema}: {err:?}");
}

#[test]
fn a_reference_without_presence_or_repeats_lowers_like_its_inlined_definition() {
    let dir = TempDir::new().expect("tempdir");
    let referenced = resolve_inline(
        dir.path(),
        "base: string(min(2); pattern(^a))\nx: base(max(9)) -> described\nlist: base[](min(1))\n",
    )
    .expect("schema resolves");
    let inlined = resolve_inline(
        dir.path(),
        "base: string(min(2); pattern(^a))\nx: string(min(2); pattern(^a); max(9)) -> described\n\
         list: string(min(2); pattern(^a))[](min(1))\n",
    )
    .expect("schema resolves");
    assert_eq!(referenced.json_schema, inlined.json_schema);
}

#[test]
fn suggestions_on_a_reference_are_linted_against_the_resolved_target() {
    let dir = TempDir::new().expect("tempdir");
    let schema = write(
        dir.path(),
        "schema.yaml",
        "kind: schema\ntypes:\n  label: string(max(3))\n  pick: 'label(suggest(ab, toolong))'\n",
    );
    // Passive parsing cannot judge a candidate before the target is known, so
    // it neither fails nor reports a problem.
    let document = parse_standalone_schema_document(
        &std::fs::read_to_string(&schema).expect("read fixture"),
        &schema,
    )
    .expect("a suggestion on a reference parses passively")
    .expect("a recognized envelope");
    assert!(document.suggestion_lints.is_empty());

    let resolved = resolve_reference(dir.path(), "./schema.yaml").expect("schema resolves");
    let problems = lint_suggestions(resolved.simplified.as_ref().expect("simplified schema"))
        .expect("lint runs");
    let flagged: Vec<_> = problems.iter().map(|problem| problem.decoded.as_str()).collect();
    assert_eq!(flagged, ["toolong"], "judged against the inherited `max(3)`");
    assert_eq!(problems[0].reason, SuggestionLintReason::Length);
}

// ── Checkpoint: merged reuse survives lowering and actual validation ──────

#[test]
fn reused_patterns_and_match_groups_stay_conjunctive_in_validation() {
    let dir = TempDir::new().expect("tempdir");
    let root = canonical(dir.path());
    for file in ["guide-intro.md", "notes.md", "guide.txt"] {
        write(&root, file, "x\n");
    }
    write(
        &root,
        "types.yaml",
        "kind: schema\ntypes:\n  code: string(pattern(^a))\n  doc: \"file(match('*.md'))\"\n",
    );

    let schema = effective_for_document(&root, "\n  both: code(pattern(z$))@./types.yaml\n");
    assert!(schema.validate(&json!({"both": "az"})).valid, "satisfies both patterns");
    for value in ["ab", "bz"] {
        assert!(
            !schema.validate(&json!({"both": value})).valid,
            "`{value}` satisfies only one pattern"
        );
    }

    // A root union cannot use `@this`, so the arm reuses `doc` across files.
    let union = "\n  - doc: \"doc(match('guide*'))@./types.yaml\"\n  - doc: file\n    other: string(required)\n";
    let validate = |value: &str| {
        let text = format!("---\n$schema:{union}doc: {value}\n---\nBody\n");
        let path = root.join("doc.md");
        std::fs::write(&path, &text).expect("write document");
        let markdown = Markdown::from(text.as_str())
            .with_source(darkmatter::markdown::compose::ComposeSource::File(path));
        DarkmatterSchemas::new(FileResolutionContext::new(&root))
            .validate(&markdown)
            .expect("schema resolves")
            .valid
    };
    assert!(validate("guide-intro.md"), "both groups admit it");
    assert!(!validate("notes.md"), "only the inherited group admits it");
    assert!(!validate("guide.txt"), "only the use-site group admits it");
}
