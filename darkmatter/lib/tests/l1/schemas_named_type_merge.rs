//! Public resolution evidence for reused named types: inherited presence,
//! bare local references, and use-site constraint merging.

use std::path::{Path, PathBuf};

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::schemas::resolve::{ResolvedSchema, expand_named_types, resolve_yaml_schema};
use darkmatter::markdown::schemas::{
    Constraint, DarkmatterSchemas, PropertyAtom, PropertyDef, SchemaConstraintConflictKind,
    SchemaConstraintParticipant,
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
    // Each type has one use, so each use inlines and merges into one scope;
    // a type used twice would be stored once and referenced.
    let resolved = resolve_inline(
        dir.path(),
        "card:\n  $constraints:\n    min-keys: 1\n    max-keys: 5\n  a: string\n  b: string\n  c: string\n\
         wide:\n  $constraints:\n    min-keys: 1\n    max-keys: 5\n  a: string\n  b: string\n  c: string\n\
         tight: card(max-keys(2))\nfull: wide(min-keys(3); max-keys(9))\n\
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
    let paths: Vec<_> = constraints.iter().map(|participant| participant.origin.path.clone()).collect();
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
    assert_eq!(inherited.origin.source, SchemaSourceId::File(canonical(&types)));
    assert_eq!(inherited.origin.path, path(&["label"]), "the declaration that introduced `min`");
    assert_eq!(inherited.origin.keyword, "min");
    let span = inherited.origin.span.clone().expect("a file origin carries its span");
    assert_eq!(&types_text[span], "min(10)");
    assert_eq!(inherited.constraint, Constraint::Min(10.0));

    let use_site = &constraints[1];
    assert_eq!(use_site.origin.source, SchemaSourceId::InlineDocument);
    assert_eq!(use_site.origin.path, path(&["title"]));
    assert_eq!(use_site.origin.keyword, "max");
    assert_eq!(use_site.origin.span, None, "inline origins are projected by the consumer");
    assert_eq!(use_site.constraint, Constraint::Max(5.0));
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
    assert_eq!(constraints[0].origin.path, path(&["card", "$constraints", "min-keys"]));
    assert_eq!(constraints[0].origin.keyword, "min-keys");
    let span = constraints[0].origin.span.clone().expect("file origin span");
    assert_eq!(&types_text[span], "min-keys: 3");
    assert_eq!(constraints[1].origin.path, path(&["box"]));
    assert_eq!(constraints[1].origin.keyword, "max-keys");
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
    assert_eq!(constraints[0].origin.path, path(&["link"]));
    assert_eq!(constraints[1].origin.path, path(&["ftp"]));
    assert_eq!(constraints[0].constraint, Constraint::Scheme(vec!["http".into(), "https".into()]));
    assert_eq!(constraints[1].constraint, Constraint::Scheme(vec!["ftp".into()]));

    // After a successful narrowing, the inherited participant is the
    // effective list before the failing intersection, at the origin that
    // introduced the group.
    let err = resolve_inline(
        dir.path(),
        "link: url(scheme(http, https))
narrow: link(scheme(https, ftp); scheme(ftp))
",
    )
    .expect_err("https and then ftp leave nothing");
    let SchemaError::ConstraintConflict { kind, constraints, .. } = err else {
        panic!("expected ConstraintConflict, got {err:?}");
    };
    assert_eq!(kind, SchemaConstraintConflictKind::Scheme);
    assert_eq!(constraints[0].origin.path, path(&["link"]));
    assert_eq!(constraints[0].constraint, Constraint::Scheme(vec!["https".into()]));
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

// ── Conflict participants carry their value ─────────────────────────────────

/// The two-import fixture: each `*_mid` type in `near.yaml` re-exports a type
/// from `far.yaml`, so a document reaching it through `near.yaml` inherits
/// the far file's constraint across two imports.
struct TwoImports {
    dir: TempDir,
    far: PathBuf,
    far_text: String,
    near: PathBuf,
}

impl TwoImports {
    fn new() -> Self {
        let dir = TempDir::new().expect("tempdir");
        let far = write(
            dir.path(),
            "far.yaml",
            "kind: schema\ntypes:\n  \
             count: \"number(min(10))\"\n  \
             label: \"string(min(10))\"\n  \
             tags: \"string[](min(10))\"\n  \
             card: \"{ a: string }(min-keys(10))\"\n  \
             amount: \"number(default(3))\"\n  \
             word: \"string(default('null'))\"\n  \
             slug: alphanumeric\n  \
             handle: \"alphanumeric(allow-dashes)\"\n  \
             link: \"url(scheme(http, https))\"\n",
        );
        let near = write(
            dir.path(),
            "near.yaml",
            "kind: schema\ntypes:\n  \
             count_mid: count@./far.yaml\n  \
             label_mid: label@./far.yaml\n  \
             tags_mid: tags@./far.yaml\n  \
             card_mid: card@./far.yaml\n  \
             amount_mid: amount@./far.yaml\n  \
             word_mid: word@./far.yaml\n  \
             slug_mid: slug@./far.yaml\n  \
             handle_mid: handle@./far.yaml\n  \
             link_mid: link@./far.yaml\n  \
             secure: \"link(scheme(https, ftp))@./far.yaml\"\n  \
             stricter: \"count(min(12))@./far.yaml\"\n  \
             equal: \"count(min(10))@./far.yaml\"\n  \
             looser: \"count(min(1))@./far.yaml\"\n",
        );
        let far_text = std::fs::read_to_string(&far).expect("read fixture");
        Self { dir, far, far_text, near }
    }

    fn far_source(&self) -> SchemaSourceId {
        SchemaSourceId::File(canonical(&self.far))
    }

    fn far_display(&self) -> String {
        self.far_source().to_string()
    }

    /// Resolves `x: <reference>` from an inline `$schema` and returns the
    /// conflict it must raise, with its `Display` text.
    fn conflict(&self, reference: &str) -> (String, SchemaConstraintConflictKind, Vec<SchemaConstraintParticipant>) {
        let err = resolve_inline(self.dir.path(), &format!("x: \"{reference}\"\n"))
            .expect_err("the reference must conflict");
        let display = err.to_string();
        let SchemaError::ConstraintConflict { kind, site, constraints, .. } = err else {
            panic!("expected ConstraintConflict for {reference}, got {err:?}");
        };
        assert_eq!(site.source, SchemaSourceId::InlineDocument);
        assert_eq!(site.path, path(&["x"]));
        (display, kind, constraints)
    }

    /// Asserts a participant's value, source, path, keyword, occurrence, and
    /// (for a far-file origin) the authored text at its span.
    fn assert_far(&self, participant: &SchemaConstraintParticipant, value: &Constraint, at: &str, keyword: &str, authored: Option<&str>) {
        assert_eq!(&participant.constraint, value);
        assert_eq!(participant.origin.source, self.far_source());
        assert_eq!(participant.origin.path, path(&[at]));
        assert_eq!(participant.origin.keyword, keyword);
        assert_eq!(participant.origin.occurrence, 0);
        match authored {
            Some(text) => {
                let span = participant.origin.span.clone().expect("a far-file origin is located");
                assert_eq!(&self.far_text[span], text);
            }
            None => assert_eq!(participant.origin.span, None),
        }
    }
}

fn assert_inline(participant: &SchemaConstraintParticipant, value: &Constraint, keyword: &str, occurrence: usize) {
    assert_eq!(&participant.constraint, value);
    assert_eq!(participant.origin.source, SchemaSourceId::InlineDocument);
    assert_eq!(participant.origin.path, path(&["x"]));
    assert_eq!(participant.origin.keyword, keyword);
    assert_eq!(participant.origin.occurrence, occurrence);
    assert_eq!(participant.origin.span, None, "inline origins are projected by the consumer");
}

#[test]
fn bound_conflicts_through_two_imports_carry_each_surviving_bound() {
    let fixture = TwoImports::new();
    for (reference, name, lower, upper, lower_text, upper_text, lower_keyword, upper_keyword) in [
        ("count_mid(max(5))@./near.yaml", "count_mid", Constraint::Min(10.0), Constraint::Max(5.0), "min(10)", "max(5)", "min", "max"),
        ("label_mid(max(5))@./near.yaml", "label_mid", Constraint::MinLen(10), Constraint::MaxLen(5), "min(10)", "max(5)", "min", "max"),
        ("tags_mid(max(5))@./near.yaml", "tags_mid", Constraint::MinItems(10), Constraint::MaxItems(5), "min(10)", "max(5)", "min", "max"),
        ("card_mid(max-keys(5))@./near.yaml", "card_mid", Constraint::MinKeys(10), Constraint::MaxKeys(5), "min-keys(10)", "max-keys(5)", "min-keys", "max-keys"),
    ] {
        let (display, kind, constraints) = fixture.conflict(reference);
        assert_eq!(kind, SchemaConstraintConflictKind::Bounds, "{reference}");
        assert_eq!(constraints.len(), 2, "{reference}");
        let at = name.trim_end_matches("_mid");
        fixture.assert_far(&constraints[0], &lower, at, lower_keyword, Some(lower_text));
        assert_inline(&constraints[1], &upper, upper_keyword, 0);
        assert_eq!(
            display,
            format!(
                "x: contradictory bounds on named type `{name}`: `{lower_text}` in {} and `{upper_text}` in \
                 the inline `$schema`",
                fixture.far_display()
            )
        );
    }
}

#[test]
fn a_stricter_bound_replaces_the_looser_one_and_an_equal_bound_keeps_the_first_origin() {
    let fixture = TwoImports::new();
    let near = SchemaSourceId::File(canonical(&fixture.near));

    // `near.yaml`'s stricter `min(12)` replaced the far `min(10)` and took
    // its origin.
    let (display, _, constraints) = fixture.conflict("stricter(max(5))@./near.yaml");
    assert_eq!(constraints[0].constraint, Constraint::Min(12.0));
    assert_eq!(constraints[0].origin.source, near);
    assert_eq!(constraints[0].origin.path, path(&["stricter"]));
    assert_eq!(constraints[0].origin.keyword, "min");
    assert!(display.contains(&format!("`min(12)` in {near}")), "{display}");

    // An equal or looser bound leaves the far declaration in place.
    for reference in ["equal(max(5))@./near.yaml", "looser(max(5))@./near.yaml"] {
        let (display, _, constraints) = fixture.conflict(reference);
        fixture.assert_far(&constraints[0], &Constraint::Min(10.0), "count", "min", Some("min(10)"));
        assert!(display.contains(&format!("`min(10)` in {}", fixture.far_display())), "{display}");
    }

    // A use site repeating its own bound keeps the stricter occurrence.
    let (_, _, constraints) = fixture.conflict("count_mid(max(8); max(5))@./near.yaml");
    assert_inline(&constraints[1], &Constraint::Max(5.0), "max", 1);
}

#[test]
fn scheme_conflicts_through_two_imports_carry_the_effective_inherited_list() {
    let fixture = TwoImports::new();
    let near = SchemaSourceId::File(canonical(&fixture.near));

    // An immediate empty intersection.
    let (display, kind, constraints) = fixture.conflict("link_mid(scheme(ftp))@./near.yaml");
    assert_eq!(kind, SchemaConstraintConflictKind::Scheme);
    fixture.assert_far(
        &constraints[0],
        &Constraint::Scheme(vec!["http".into(), "https".into()]),
        "link",
        "scheme",
        Some("scheme(http, https)"),
    );
    assert_inline(&constraints[1], &Constraint::Scheme(vec!["ftp".into()]), "scheme", 0);
    assert_eq!(
        display,
        format!(
            "x: contradictory schemes on named type `link_mid`: `scheme(http,https)` in {} and \
             `scheme(ftp)` in the inline `$schema`",
            fixture.far_display()
        )
    );

    // After `near.yaml` narrowed the list to `https`, the inherited
    // participant is that effective list at the far origin that introduced
    // the group.
    let (display, _, constraints) = fixture.conflict("secure(scheme(ftp))@./near.yaml");
    fixture.assert_far(
        &constraints[0],
        &Constraint::Scheme(vec!["https".into()]),
        "link",
        "scheme",
        Some("scheme(http, https)"),
    );
    assert_ne!(constraints[0].origin.source, near);
    assert!(display.contains("`scheme(https)` in"), "{display}");
}

#[test]
fn default_conflicts_through_two_imports_keep_scalar_identity() {
    let fixture = TwoImports::new();

    let (display, kind, constraints) = fixture.conflict("amount_mid(default('3'))@./near.yaml");
    assert_eq!(kind, SchemaConstraintConflictKind::Default);
    fixture.assert_far(&constraints[0], &Constraint::Default(json!(3)), "amount", "default", Some("default(3)"));
    assert_inline(&constraints[1], &Constraint::Default(json!("3")), "default", 0);
    assert_eq!(
        display,
        format!(
            "x: contradictory defaults on named type `amount_mid`: `default(3)` in {} and \
             `default('3')` in the inline `$schema`",
            fixture.far_display()
        )
    );

    let (display, _, constraints) = fixture.conflict("word_mid(default(null))@./near.yaml");
    fixture.assert_far(&constraints[0], &Constraint::Default(json!("null")), "word", "default", Some("default('null')"));
    assert_inline(&constraints[1], &Constraint::Default(Value::Null), "default", 0);
    assert!(display.contains("`default('null')` in") && display.contains("`default(null)` in the inline"), "{display}");

    // Normalized numeric equality is still one default.
    resolve_inline(fixture.dir.path(), "x: amount_mid(default(3.0))@./near.yaml\n")
        .expect("3 and 3.0 are one default");
}

#[test]
fn item_and_container_defaults_reconcile_with_their_values() {
    let fixture = TwoImports::new();

    let (display, kind, constraints) = fixture.conflict("amount_mid[](default('3'))@./near.yaml");
    assert_eq!(kind, SchemaConstraintConflictKind::Default);
    fixture.assert_far(&constraints[0], &Constraint::Default(json!(3)), "amount", "default", Some("default(3)"));
    assert_inline(&constraints[1], &Constraint::Default(json!("3")), "default", 0);
    assert!(display.contains("`default(3)` in") && display.contains("`default('3')` in the inline"), "{display}");

    resolve_inline(fixture.dir.path(), "x: amount_mid[](default(3.0))@./near.yaml\n")
        .expect("an equal container default reconciles");
}

#[test]
fn option_conflicts_through_two_imports_carry_both_canonical_sets() {
    let fixture = TwoImports::new();

    // A bare `alphanumeric` type takes part as the empty set. With no option
    // token to locate, its synthetic origin is the declaration of the named
    // type the use site referenced (`near.yaml`'s `slug_mid`).
    let (display, kind, constraints) = fixture.conflict("slug_mid(allow-dashes)@./near.yaml");
    assert_eq!(kind, SchemaConstraintConflictKind::Options);
    let near = SchemaSourceId::File(canonical(&fixture.near));
    assert_eq!(constraints[0].constraint, Constraint::Options(vec![]));
    assert_eq!(constraints[0].origin.source, near);
    assert_eq!(constraints[0].origin.path, path(&["slug_mid"]));
    assert_eq!(constraints[0].origin.keyword, "alphanumeric");
    assert_eq!(constraints[0].origin.occurrence, 0);
    assert_eq!(constraints[0].origin.span, None, "bare `alphanumeric` has no option token");
    assert_inline(
        &constraints[1],
        &Constraint::Options(vec![darkmatter::markdown::schemas::AlphanumericOption::AllowDashes]),
        "allow-dashes",
        0,
    );
    assert_eq!(
        display,
        format!(
            "x: contradictory `alphanumeric` options on named type `slug_mid`: `alphanumeric` in {near} and \
             `alphanumeric(allow-dashes)` in the inline `$schema`"
        )
    );

    // Two different nonempty sets.
    let (display, _, constraints) = fixture.conflict("handle_mid(allow-periods, allow-dashes)@./near.yaml");
    fixture.assert_far(
        &constraints[0],
        &Constraint::Options(vec![darkmatter::markdown::schemas::AlphanumericOption::AllowDashes]),
        "handle",
        "allow-dashes",
        Some("allow-dashes"),
    );
    assert!(
        display.contains("`alphanumeric(allow-dashes)` in")
            && display.contains("`alphanumeric(allow-dashes,allow-periods)` in the inline"),
        "{display}"
    );
}
