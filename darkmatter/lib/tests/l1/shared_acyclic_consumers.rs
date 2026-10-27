//! Every consumer of a shared acyclic named type sees what the inlined type gave.
//!
//! A structured type with two authored uses is stored once in `$defs` and
//! referenced at each use; with one use it is inlined, as every acyclic type
//! was before sharing. Most tests here build both forms of the same schema
//! (`a` typed once, and `a` beside a second use `b`) and require each
//! consumer's answer for `a` to be identical: lowering, launch and completion
//! presence, conversion, validation with positions, eager-file
//! normalization, deferred templates, completion, the call check, and
//! subtyping. Each also asserts that the two-use form really holds a
//! definition, so it cannot pass with both forms inlined.
//!
//! The remaining tests cover what has no inlined counterpart: merging layers
//! that share types (equal-body reuse, rekeying of different bodies together
//! with the references between definitions, deterministic bytes), dependency
//! invalidation of an imported shared type, embedded and reserved imports,
//! and a root-union arm's suggestion-only `match(...)`.

use std::collections::HashSet;
use std::path::PathBuf;

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::expression::{
    ArgumentCategory, DeclaredTypes, TypeScope, check_expression_calls, parse_spanned,
};
use darkmatter::markdown::compose::{ComposeSource, RequestSnapshot, build_resolution_context};
use darkmatter::markdown::schemas::completion::{PathStep, for_path};
use darkmatter::markdown::schemas::resolve::{ResolvedSchema, resolve_yaml_schema};
use darkmatter::markdown::schemas::validate::build_position_map;
use darkmatter::markdown::schemas::{
    DarkmatterSchemas, EffectiveSchema, EmbeddedSchemaBundle, SchemaPhase, SimplifiedSchema, ValidationOptions,
    ValidationProblemCode, ValidationReport, is_definition_subtype_with_definitions,
};
use serde_json::{Map, Value, json};
use tempfile::TempDir;

/// A temporary Git repository (`repo/`, with `schemas/` as its trigger root)
/// and a fixture home beside it. Every context is anchored here, never at the
/// process directory.
struct Repo {
    _dir: TempDir,
    root: PathBuf,
    home: PathBuf,
}

impl Repo {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let base = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
        let root = base.join("repo");
        let git = root.join(".git");
        std::fs::create_dir_all(git.join("objects")).expect("git objects");
        std::fs::create_dir_all(git.join("refs/heads")).expect("git refs");
        std::fs::write(git.join("HEAD"), "ref: refs/heads/main\n").expect("git HEAD");
        std::fs::write(git.join("config"), "[core]\nrepositoryformatversion = 0\nbare = false\n").expect("git config");
        std::fs::create_dir_all(root.join("docs")).expect("docs dir");
        let home = base.join("home");
        std::fs::create_dir_all(&home).expect("home dir");
        Self { _dir: dir, root, home }
    }

    fn write(&self, relative: &str, contents: &str) -> PathBuf {
        let path = self.root.join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        std::fs::write(&path, contents).expect("write fixture");
        path
    }

    /// Schemas for documents in `docs/`, with trigger discovery when asked.
    fn schemas(&self, triggers: bool) -> DarkmatterSchemas {
        let snapshot = RequestSnapshot::new(self.root.join("docs")).with_home(Some(self.home.clone()));
        let schemas = DarkmatterSchemas::new(build_resolution_context(&snapshot).expect("fixture context"));
        if triggers { schemas.with_trigger_discovery().expect("trigger discovery") } else { schemas }
    }

    /// Writes `text` as `relative` and opens it with its own path.
    fn document(&self, relative: &str, text: &str) -> Markdown {
        let path = self.write(relative, text);
        Markdown::try_from(path.as_path()).expect("open document").with_source(ComposeSource::File(path))
    }
}

fn effective(schemas: &DarkmatterSchemas, document: &Markdown) -> EffectiveSchema {
    schemas.effective_for(document).expect("schema resolves").expect("an effective schema")
}

fn has_definitions(effective: &EffectiveSchema) -> bool {
    effective.json_schema.get("$defs").and_then(Value::as_object).is_some_and(|defs| !defs.is_empty())
}

/// `schema` with every local `$ref` replaced by its definition body, the
/// reference's siblings merged in, and the root `$defs` removed: the
/// inlined schema the references stand for. Acyclic only.
fn dereferenced(schema: &Value) -> Value {
    fn walk(node: &Value, defs: &Map<String, Value>, depth: usize) -> Value {
        assert!(depth < 64, "a shared definition is acyclic");
        match node {
            Value::Object(map) => {
                let mut out = Map::new();
                if let Some(pointer) = map.get("$ref").and_then(Value::as_str) {
                    let key = pointer.strip_prefix("#/$defs/").expect("a local definition reference");
                    let key = key.replace("~1", "/").replace("~0", "~");
                    let body = defs.get(&key).unwrap_or_else(|| panic!("`{pointer}` resolves"));
                    let Value::Object(body) = walk(body, defs, depth + 1) else {
                        panic!("a definition body is an object schema");
                    };
                    out.extend(body);
                }
                for (key, value) in map {
                    if key != "$ref" && key != "$defs" {
                        let previous = out.insert(key.clone(), walk(value, defs, depth + 1));
                        assert!(previous.is_none(), "a use-site `{key}` beside a body that sets it");
                    }
                }
                Value::Object(out)
            }
            Value::Array(items) => Value::Array(items.iter().map(|item| walk(item, defs, depth + 1)).collect()),
            other => other.clone(),
        }
    }
    let defs = schema.get("$defs").and_then(Value::as_object).cloned().unwrap_or_default();
    walk(schema, &defs, 0)
}

/// A problem as `(code, instance path, property)`, sorted.
fn outcome(report: &ValidationReport) -> Vec<(ValidationProblemCode, String, Option<String>)> {
    let mut found: Vec<_> =
        report.problems.iter().map(|problem| (problem.code, problem.path.clone(), problem.property.clone())).collect();
    found.sort();
    found
}

/// A problem as `(code, instance path, line, column, description)`, sorted.
type Located = (ValidationProblemCode, String, Option<u32>, Option<u32>, Option<String>);

fn located(report: &ValidationReport) -> Vec<Located> {
    let mut found: Vec<Located> = report
        .problems
        .iter()
        .map(|problem| {
            (problem.code, problem.path.clone(), problem.line, problem.column, problem.description.clone())
        })
        .collect();
    found.sort();
    found
}

/// An object type exercising each consumer: presence timed by `eager`, a
/// default, a bound, an eager file, a deferred place, an enum, and a list.
const TYPES: &str = "kind: schema
types:
  entry:
    name: \"string(required; eager) -> the entry name\"
    title: \"string(default(untitled)) -> the entry title\"
    size: \"number(min(0)) -> the entry size\"
    stamp: \"date(required) -> when the entry was made\"
    spec: \"file(eager) -> the entry's spec\"
    later: \"number(deferred) -> settled after rendering\"
    kind: \"enum(alpha, beta)\"
    tags: \"string[]\"
";

/// The two forms of one schema: `a` typed `use_a` alone (inlined), and `a`
/// beside a second use `b` of `entry` (shared). Both documents sit in
/// `docs/` beside `types.yaml`, so every relative value means the same.
struct Forms {
    repo: Repo,
    use_a: &'static str,
    schemas: DarkmatterSchemas,
    once: EffectiveSchema,
    twice: EffectiveSchema,
}

impl Forms {
    fn new(use_a: &'static str) -> Self {
        let repo = Repo::new();
        repo.write("docs/types.yaml", TYPES);
        repo.write("docs/notes/x.md", "# x\n");
        let schemas = repo.schemas(false);
        let once = effective(&schemas, &repo.document("docs/once.md", &format!("---\n{}---\n", Self::schema(use_a, false))));
        let twice = effective(&schemas, &repo.document("docs/twice.md", &format!("---\n{}---\n", Self::schema(use_a, true))));
        assert!(!has_definitions(&once), "one use inlines: {:#}", once.json_schema);
        assert!(has_definitions(&twice), "two uses share: {:#}", twice.json_schema);
        Self { repo, use_a, schemas, once, twice }
    }

    fn schema(use_a: &str, twice: bool) -> String {
        let second = if twice { "  b: entry@./types.yaml\n" } else { "" };
        format!("$schema:\n  a: {use_a}\n{second}")
    }

    /// A document of either form whose frontmatter, after `$schema`, is
    /// `body`.
    fn document(&self, twice: bool, name: &str, body: &str) -> Markdown {
        let yaml = format!("{}{body}", Self::schema(self.use_a, twice));
        self.repo.document(&format!("docs/{name}-{twice}.md"), &format!("---\n{yaml}---\nbody\n"))
    }
}

// ── Lowering ──────────────────────────────────────────────────────────────

#[test]
fn each_use_lowers_to_the_inlined_type_once_its_reference_is_followed() {
    for use_a in ["entry@./types.yaml", "entry(required)@./types.yaml", "entry[]@./types.yaml"] {
        let forms = Forms::new(use_a);
        let inlined = &forms.once.json_schema["properties"]["a"];
        let shared = dereferenced(&forms.twice.json_schema);
        assert_eq!(&shared["properties"]["a"], inlined, "{use_a}: defaults, eager files, deferral, descriptions");
        assert_eq!(shared["required"], forms.once.json_schema["required"], "{use_a}");
        for keyword in ["default", "x-darkmatter-deferred", "darkmatter-file", "the entry name"] {
            assert!(inlined.to_string().contains(keyword), "{use_a}: the fixture exercises `{keyword}`");
        }
    }
    let forms = Forms::new("entry@./types.yaml");
    let shared = dereferenced(&forms.twice.json_schema);
    assert_eq!(shared["properties"]["b"], shared["properties"]["a"], "both uses are the same type");
}

// ── Launch and completion presence ────────────────────────────────────────

#[test]
fn launch_and_completion_presence_is_unchanged_at_each_use() {
    let scalar = Forms::new("entry@./types.yaml");
    let list = Forms::new("entry[]@./types.yaml");
    let cases = [
        (&scalar, json!({ "a": {} })),
        (&scalar, json!({ "a": { "name": "n" } })),
        (&scalar, json!({ "a": { "stamp": "2026-01-02" } })),
        (&scalar, json!({ "a": null })),
        (&list, json!({ "a": [{}] })),
        (&list, json!({ "a": [null] })),
        (&list, json!({ "a": null })),
    ];
    for (forms, value) in cases {
        for phase in [SchemaPhase::Launch, SchemaPhase::Completion] {
            let inlined = forms.once.validate_for_phase(&value, phase).expect("inlined phase schema");
            let shared = forms.twice.validate_for_phase(&value, phase).expect("shared phase schema");
            assert_eq!(outcome(&shared), outcome(&inlined), "{phase:?} {value}");
        }
        assert_eq!(outcome(&forms.twice.validate(&value)), outcome(&forms.once.validate(&value)), "{value}");
    }

    // The captured rules themselves: `name` is eager, so launch requires it;
    // `stamp` is required at completion only; an optional list's items are
    // not nullable.
    let missing = |report: &ValidationReport| -> Vec<String> {
        report.problems.iter().filter_map(|problem| problem.property.clone()).collect()
    };
    let empty = json!({ "a": {} });
    let launch = scalar.twice.validate_for_phase(&empty, SchemaPhase::Launch).expect("launch");
    assert_eq!(missing(&launch), ["name"], "{:#?}", launch.problems);
    let completion = scalar.twice.validate_for_phase(&empty, SchemaPhase::Completion).expect("completion");
    let mut names = missing(&completion);
    names.sort();
    assert_eq!(names, ["name", "stamp"], "{:#?}", completion.problems);
    assert!(scalar.twice.validate(&json!({ "a": null })).valid, "an optional use accepts null");
    let item = list.twice.validate(&json!({ "a": [null] }));
    assert_eq!(outcome(&item).iter().map(|(_, path, _)| path.as_str()).collect::<Vec<_>>(), ["/a/0"]);
}

// ── Conversion and located validation ─────────────────────────────────────

#[test]
fn conversion_reaches_each_use_as_it_reached_the_inlined_type() {
    let scalar = Forms::new("entry@./types.yaml");
    let list = Forms::new("entry[]@./types.yaml");
    let pending = HashSet::new();
    for (forms, value) in [
        (&scalar, json!({ "a": { "name": 7, "size": "3", "stamp": "2026-01-02", "later": "4", "tags": "solo" } })),
        (&scalar, json!({ "a": { "name": "n", "size": "big", "stamp": "2026-01-02" } })),
        (&list, json!({ "a": [{ "name": "n", "size": "5", "stamp": "2026-01-02" }, { "size": "6" }] })),
    ] {
        let inlined = forms.once.coerce_frontmatter_with_pending(&value, &pending);
        let shared = forms.twice.coerce_frontmatter_with_pending(&value, &pending);
        assert_eq!(shared.value, inlined.value, "{value}");
    }
    let converted = scalar.twice.coerce_frontmatter_with_pending(
        &json!({ "a": { "name": "n", "size": "3", "stamp": "2026-01-02" }, "b": { "name": "m", "size": "4", "stamp": "2026-01-02" } }),
        &pending,
    );
    assert_eq!(converted.value["a"]["size"], json!(3));
    assert_eq!(converted.value["b"]["size"], json!(4), "the second use converts too");
}

#[test]
fn invalid_values_are_located_at_each_use_as_the_inlined_type_located_them() {
    let scalar = Forms::new("entry@./types.yaml");
    let list = Forms::new("entry[]@./types.yaml");
    let cases = [
        (&scalar, "scalar-bad", "a:\n  name: n\n  size: -1\n  stamp: 2026-01-02\n  kind: gamma\n"),
        (&scalar, "scalar-missing", "a:\n  size: 1\n"),
        (&list, "list-items", "a:\n  - name: n\n    stamp: 2026-01-02\n    tags: [x, { y: 1 }]\n  - size: 2\n"),
    ];
    for (forms, name, body) in cases {
        let inlined_doc = forms.document(false, name, body);
        let shared_doc = forms.document(true, name, body);
        let inlined = forms.schemas.validate(&inlined_doc).expect("inlined resolves");
        let shared = forms.schemas.validate(&shared_doc).expect("shared resolves");
        assert!(!inlined.valid, "{name}: the fixture fails");
        assert!(inlined.problems.iter().all(|problem| problem.line.is_some()), "{name}: {:#?}", inlined.problems);
        assert!(has_definitions(&effective(&forms.schemas, &shared_doc)), "{name}");
        // The shared document has one more schema line before the values,
        // so its problems sit one line lower.
        let expected: Vec<Located> =
            located(&inlined).into_iter().map(|(code, path, line, column, description)| {
                (code, path, line.map(|line| line + 1), column, description)
            }).collect();
        assert_eq!(located(&shared), expected, "{name}");
    }
}

// ── Eager files, deferred places, and validation-only paths ───────────────

#[test]
fn eager_files_are_normalized_at_each_use() {
    let scalar = Forms::new("entry@./types.yaml");
    let list = Forms::new("entry[]@./types.yaml");
    let pending = HashSet::new();
    for (forms, value) in [
        (&scalar, json!({ "a": { "name": "n", "stamp": "2026-01-02", "spec": "./notes/x.md" } })),
        (&list, json!({ "a": [{ "name": "n", "stamp": "2026-01-02", "spec": "./notes/x.md" }] })),
    ] {
        let inlined = forms.once.normalize_frontmatter(&value, &pending);
        let shared = forms.twice.normalize_frontmatter(&value, &pending);
        assert!(inlined.changed, "{value}: the fixture is normalized");
        assert_eq!((shared.value, shared.changed), (inlined.value, inlined.changed), "{value}");
    }
    let both = json!({ "a": { "spec": "./notes/x.md" }, "b": { "spec": "./notes/x.md" } });
    let normalized = scalar.twice.normalize_frontmatter(&both, &pending).value;
    assert_ne!(normalized["a"]["spec"], json!("./notes/x.md"));
    assert_eq!(normalized["b"]["spec"], normalized["a"]["spec"], "the second use is normalized too");

    let missing = json!({ "a": { "name": "n", "stamp": "2026-01-02", "spec": "./notes/none.md" } });
    let inlined = scalar.once.validate(&missing);
    assert!(!inlined.valid, "a missing eager file fails");
    assert_eq!(outcome(&scalar.twice.validate(&missing)), outcome(&inlined));
}

#[test]
fn deferred_templates_and_pending_values_are_judged_as_before() {
    let forms = Forms::new("entry@./types.yaml");
    let options = ValidationOptions::default();
    let positions = build_position_map("");
    for value in [
        json!({ "a": { "name": "n", "stamp": "2026-01-02", "later": "{{ fm.count }}" } }),
        json!({ "a": { "name": "n", "stamp": "2026-01-02", "later": "{{ fm.count }}", "size": "{{ fm.size }}" } }),
        json!({ "a": { "name": "n", "stamp": "2026-01-02", "size": "{{ 1 + }}" } }),
        json!({ "a": "{{ fm.whole }}" }),
    ] {
        let inlined = forms.once.validate_with_options(&value, &positions, &options);
        let shared = forms.twice.validate_with_options(&value, &positions, &options);
        let pending = |report: &ValidationReport| -> Vec<String> {
            report.pending.iter().map(|pending| pending.key.clone()).collect()
        };
        assert_eq!(
            (shared.valid, outcome(&shared), shared.deferred.clone(), pending(&shared)),
            (inlined.valid, outcome(&inlined), inlined.deferred.clone(), pending(&inlined)),
            "{value}"
        );
    }
    let deferred = forms.twice.validate_with_options(
        &json!({ "b": { "name": "n", "stamp": "2026-01-02", "later": "{{ fm.count }}" } }),
        &positions,
        &options,
    );
    assert!(deferred.valid, "{:#?}", deferred.problems);
    assert_eq!(deferred.deferred, ["/b/later"], "the second use defers its own place");
}

#[test]
fn validating_a_shared_use_executes_nothing() {
    let forms = Forms::new("entry@./types.yaml");
    let marker = forms.repo.root.join("docs/executed");
    let shell = format!("\"$(touch {})\"", marker.display());
    for twice in [false, true] {
        let body = format!("a: {shell}\nb:\n  later: \"{{{{ fm.n }}}}\"\n");
        let document = forms.document(twice, "shell", &body);
        let schema = effective(&forms.schemas, &document);
        let frontmatter = Value::Object(
            document.frontmatter().as_map().iter().map(|(key, value)| (key.clone(), value.clone())).collect(),
        );
        let report = schema.validate_with_options(&frontmatter, &build_position_map(""), &ValidationOptions::default());
        assert!(report.pending.iter().any(|pending| pending.key == "a"), "{twice}: {:#?}", report.pending);
        forms.schemas.validate(&document).expect("validates");
        for phase in [SchemaPhase::Launch, SchemaPhase::Completion] {
            schema.validate_for_phase(&frontmatter, phase).expect("phase validates");
        }
        assert!(!marker.exists(), "{twice}: validation ran the shell expression");
    }
}

// ── Editor lookup and the call check ──────────────────────────────────────

#[test]
fn completion_offers_the_inlined_choices_at_each_use() {
    let scalar = Forms::new("entry@./types.yaml");
    let list = Forms::new("entry[]@./types.yaml");
    let fm = json!({ "a": {} });
    let items = json!({ "a": [{}] });
    let cases: [(&Forms, &Value, Vec<PathStep<'_>>); 4] = [
        (&scalar, &fm, vec![PathStep::Key("a")]),
        (&scalar, &fm, vec![PathStep::Key("a"), PathStep::Key("kind")]),
        (&list, &items, vec![PathStep::Key("a"), PathStep::Index(0)]),
        (&list, &items, vec![PathStep::Key("a"), PathStep::Index(0), PathStep::Key("kind")]),
    ];
    for (forms, frontmatter, path) in cases {
        let inlined = for_path(&forms.once, frontmatter, &path).unwrap_or_else(|| panic!("{path:?}: inlined"));
        let shared = for_path(&forms.twice, frontmatter, &path).unwrap_or_else(|| panic!("{path:?}: shared"));
        assert_eq!(format!("{shared:?}"), format!("{inlined:?}"), "{path:?}");
        assert!(!inlined.keys.is_empty() || !inlined.values.is_empty(), "{path:?}: something is offered");
    }
    let b = for_path(&scalar.twice, &json!({ "b": {} }), &[PathStep::Key("b")]).expect("the second use completes");
    let a = for_path(&scalar.twice, &fm, &[PathStep::Key("a")]).expect("the first use completes");
    assert_eq!(format!("{b:?}"), format!("{a:?}"));
}

#[test]
fn the_call_check_sees_through_each_use() {
    let forms = Forms::new("entry(required)@./types.yaml");
    let category = |schema: &EffectiveSchema, expression: &str| -> ArgumentCategory {
        let declared = DeclaredTypes::for_schema(Some(schema));
        let expr = parse_spanned(expression).expect("expression parses");
        let checks = check_expression_calls(&expr, &TypeScope::unguarded(&declared));
        checks.first().unwrap_or_else(|| panic!("{expression}: a call")).category()
    };
    for (expression, expected) in [
        ("keys(a)", ArgumentCategory::Compatible),
        ("min(a, 5)", ArgumentCategory::Incompatible),
        ("length(a.tags)", ArgumentCategory::NullableMatch),
        ("min(a.size, 5)", ArgumentCategory::NullableMatch),
    ] {
        assert_eq!(category(&forms.once, expression), expected, "inlined {expression}");
        assert_eq!(category(&forms.twice, expression), expected, "shared {expression}");
    }
}

/// Subtyping and structural identity open a shared reference: a list of a
/// shared type, a shared type inside an object, two shared types with one
/// body, and a list of a shared union each relate to their inlined
/// counterpart exactly as the single-use form did.
#[test]
fn subtyping_compares_a_shared_type_as_its_inlined_copy() {
    let dir = TempDir::new().expect("tempdir");
    std::fs::write(
        dir.path().join("types.yaml"),
        "kind: schema\ntypes:\n  entry: \"{ n: number }\"\n  item: \"{ n: number }\"\n  choice: \"{ n: number } | string\"\n",
    )
    .expect("write types");
    let context = biscuit_file::FileResolutionContext::new(dir.path());
    let resolve = |yaml: &str| {
        let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(yaml).expect("fixture YAML");
        resolve_yaml_schema(&value, dir.path(), &context).expect("fixture resolves")
    };
    let relations = |resolved: &ResolvedSchema, left: &str, right: &str| {
        let Some(SimplifiedSchema::Single(shape)) = &resolved.simplified else {
            panic!("a single shape");
        };
        let (left, right) = (&shape.properties[left], &shape.properties[right]);
        let defs = &resolved.definitions;
        (
            is_definition_subtype_with_definitions(left, right, defs),
            is_definition_subtype_with_definitions(right, left, defs),
        )
    };
    for (case, base, more, left, right) in [
        ("a list", "a: entry[]@./types.yaml\nc: \"{ n: number }[]\"\n", "z: entry@./types.yaml\n", "a", "c"),
        (
            "nested in an object",
            "w: \"{ x: entry@./types.yaml }\"\nd: \"{ x: { n: number } }\"\n",
            "z: entry@./types.yaml\n",
            "w",
            "d",
        ),
        (
            "two types, one body",
            "e: entry[]@./types.yaml\ni: item[]@./types.yaml\n",
            "y: entry@./types.yaml\nz: item@./types.yaml\n",
            "e",
            "i",
        ),
        ("a list of a union", "u: choice[]@./types.yaml\nv: \"({ n: number } | string)[]\"\n", "z: choice@./types.yaml\n", "u", "v"),
    ] {
        let once = resolve(base);
        let twice = resolve(&format!("{base}{more}"));
        assert!(once.definitions.is_empty() && !twice.definitions.is_empty(), "{case}");
        assert_eq!(relations(&once, left, right), (true, true), "{case}: the fixture relates them");
        assert_eq!(relations(&twice, left, right), relations(&once, left, right), "{case}");
    }
}

// ── Root-union `match(...)` ───────────────────────────────────────────────

/// In a root-union arm a `file(match(...))` reached through an inlined
/// acyclic type only suggests, while one reached through a stored
/// definition is enforced by the arm's copy. A type holding a glob is
/// therefore never stored, so two uses keep the inlined verdict; the same
/// shape without the glob is stored, so the exclusion is the glob's alone.
#[test]
fn a_root_union_arm_keeps_a_suggestion_only_match_through_a_repeated_type() {
    let repo = Repo::new();
    let schemas = repo.schemas(false);
    repo.write("a/notes/x.md", "# x\n");
    let file = "../a/notes/x.md";
    let glob = "file(match('./notes/*.md'))";
    for (shape, ty, value) in [
        ("object", "{ path: GLOB }", json!({ "path": file })),
        ("nested list", "{ inner: { path: GLOB }[] }", json!({ "inner": [{ "path": file }] })),
        ("dictionary", "{ <pattern::^n>: GLOB }", json!({ "n1": file })),
        ("tuple", "tuple([GLOB, number])", json!([file, 1])),
        ("union with a file list", "GLOB[] | { n: number }", json!([file])),
    ] {
        for (matched, ty) in [(true, ty.replace("GLOB", glob)), (false, ty.replace("GLOB", "file"))] {
            let schema = format!("$schema:\n  - entry: \"{ty}\"\n    spec: entry\n    other: entry\n  - spec: boolean\n");
            let document = repo.document(&format!("b/{}-{matched}.md", shape.replace(' ', "-")), &format!("---\n{schema}spec: {value}\n---\n"));
            let effective = effective(&schemas, &document);
            let lowered = effective.json_schema.to_string();
            assert_eq!(has_definitions(&effective), !matched, "{shape}, glob {matched}: {lowered}");
            assert!(!lowered.contains("x-darkmatter-match"), "{shape}: an acyclic glob only suggests: {lowered}");
            let report = schemas.validate(&document).expect("validates");
            assert!(report.valid, "{shape}, glob {matched}: {:#?}", report.problems);
            for phase in [SchemaPhase::Launch, SchemaPhase::Completion] {
                let verdict = effective.validate_for_phase(&json!({ "spec": value }), phase).expect("phase schema");
                assert!(verdict.valid, "{shape}, {phase:?}: {:#?}", verdict.problems);
            }
        }
    }
}

// ── Merges ────────────────────────────────────────────────────────────────

/// A trigger matching documents with a `prompt`, whose payload is `payload`.
fn prompt_trigger(payload: &str) -> String {
    format!("kind: schema-trigger\nmatch:\n  prompt: string(required)\n$schema:{payload}")
}

/// The `$defs` key a property's value (or its optional wrapper's typed arm)
/// references.
fn reference_key(property: &Value) -> String {
    let pointer = property
        .get("$ref")
        .or_else(|| property["anyOf"].as_array()?.iter().find_map(|arm| arm.get("$ref")))
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("a reference: {property:#}"));
    pointer.strip_prefix("#/$defs/").expect("local").to_string()
}

#[test]
fn a_trigger_and_a_document_sharing_one_imported_type_reuse_its_body() {
    let repo = Repo::new();
    repo.write("schemas/types.yaml", "kind: schema\ntypes:\n  entry: \"{ n: number(required) }\"\n");
    repo.write("schemas/payload.trigger.yaml", &prompt_trigger("\n  t1: entry@./types.yaml\n  t2: entry@./types.yaml\n"));
    let schemas = repo.schemas(true);
    let document = repo.document(
        "docs/doc.md",
        "---\nprompt: x\n$schema:\n  d1: entry@../schemas/types.yaml\n  d2: entry@../schemas/types.yaml\n---\n",
    );
    let effective = effective(&schemas, &document);
    let schema = &effective.json_schema;
    let defs = schema["$defs"].as_object().expect("$defs");
    assert_eq!(defs.len(), 1, "one body for one type: {:?}", defs.keys().collect::<Vec<_>>());
    let properties = &schema["properties"];
    let keys: Vec<String> = ["d1", "d2", "t1", "t2"].iter().map(|name| reference_key(&properties[name])).collect();
    assert!(keys.iter().all(|key| key == &keys[0]), "{keys:?}");

    for property in ["d1", "d2", "t1", "t2"] {
        let bad = json!({ "prompt": "x", property: { "n": "one" } });
        let report = effective.validate(&bad);
        assert_eq!(outcome(&report).iter().map(|(_, path, _)| path.clone()).collect::<Vec<_>>(), [format!("/{property}/n")]);
    }
    assert!(effective.validate(&json!({ "prompt": "x", "d1": { "n": 1 }, "t2": { "n": 2 } })).valid);
}

/// A document and a trigger payload each declare `entry` and a `pair` whose
/// two sides reference it, and each use `pair` twice. The bodies differ, so
/// the payload's are rekeyed together with the reference between them.
#[test]
fn different_bodies_are_rekeyed_with_the_references_between_definitions() {
    let repo = Repo::new();
    let payload = "\n  entry: \"{ count: number(required) }\"\n  pair: \"{ left: entry(required), right: entry }\"\n  p1: pair\n  p2: pair\n";
    repo.write("schemas/payload.trigger.yaml", &prompt_trigger(payload));
    let schemas = repo.schemas(true);
    let text = "---\nprompt: x\n$schema:\n  entry: \"{ label: string(required) }\"\n  pair: \"{ left: entry(required), right: entry }\"\n  d1: pair\n  d2: pair\n---\n";
    let first = effective(&schemas, &repo.document("docs/doc.md", text));
    let schema = &first.json_schema;
    let defs = schema["$defs"].as_object().expect("$defs");
    assert_eq!(defs.len(), 4, "two layers, two bodies each: {:?}", defs.keys().collect::<Vec<_>>());

    let document_pair = reference_key(&schema["properties"]["d1"]);
    let payload_pair = reference_key(&schema["properties"]["p1"]);
    assert_ne!(document_pair, payload_pair);
    assert_eq!(reference_key(&schema["properties"]["d2"]), document_pair);
    assert_eq!(reference_key(&schema["properties"]["p2"]), payload_pair);
    let side = |pair: &str| reference_key(&defs[pair]["properties"]["left"]);
    let (document_entry, payload_entry) = (side(&document_pair), side(&payload_pair));
    assert_ne!(document_entry, payload_entry, "the payload's pair references the payload's entry");
    assert!(defs[&document_entry].to_string().contains("label"), "{:#}", defs[&document_entry]);
    assert!(defs[&payload_entry].to_string().contains("count"), "{:#}", defs[&payload_entry]);

    let ok = json!({ "prompt": "x", "d1": { "left": { "label": "l" } }, "p2": { "left": { "count": 1 }, "right": { "count": 2 } } });
    assert!(first.validate(&ok).valid, "{:#?}", first.validate(&ok).problems);
    for (property, value, path) in [
        ("d2", json!({ "left": { "count": 1 } }), "/d2/left"),
        ("p1", json!({ "left": { "label": "l" } }), "/p1/left"),
        ("p2", json!({ "left": { "count": 1 }, "right": { "count": "two" } }), "/p2/right/count"),
    ] {
        let report = first.validate(&json!({ "prompt": "x", property: value }));
        assert!(outcome(&report).iter().any(|(_, at, _)| at == path), "{property}: {:#?}", report.problems);
    }
    for phase in [SchemaPhase::Launch, SchemaPhase::Completion] {
        let report = first.validate_for_phase(&json!({ "prompt": "x", "p1": { "left": { "count": "x" } } }), phase);
        let report = report.expect("phase schema");
        assert!(outcome(&report).iter().any(|(_, at, _)| at == "/p1/left/count"), "{phase:?}: {:#?}", report.problems);
    }

    // Resolving again, from a fresh schema set, gives the same bytes.
    let again = effective(&repo.schemas(true), &repo.document("docs/again.md", text));
    assert_eq!(again.json_schema.to_string(), first.json_schema.to_string());
}

#[test]
fn a_baseline_and_a_document_sharing_same_named_types_keep_their_own_bodies() {
    let repo = Repo::new();
    let baseline = repo.write(
        "baseline.yaml",
        "$schema:\n  entry: \"{ count: number(required) }\"\n  base1: entry\n  base2: entry\n",
    );
    let snapshot = RequestSnapshot::new(repo.root.join("docs")).with_home(Some(repo.home.clone()));
    let build = || {
        DarkmatterSchemas::new(build_resolution_context(&snapshot).expect("context"))
            .with_baseline_from_file(&baseline)
            .expect("a baseline sharing a type is merge-compatible")
    };
    let text = "---\n$schema:\n  entry: \"{ label: string(required) }\"\n  d1: entry\n  d2: entry\n---\n";
    let first = effective(&build(), &repo.document("docs/doc.md", text));
    let defs = first.json_schema["$defs"].as_object().expect("$defs");
    assert_eq!(defs.keys().collect::<Vec<_>>(), ["entry", "entry-2"], "the baseline's body is rekeyed");
    let properties = &first.json_schema["properties"];
    for (property, key) in [("d1", "entry"), ("d2", "entry"), ("base1", "entry-2"), ("base2", "entry-2")] {
        assert_eq!(reference_key(&properties[property]), key, "{property}");
    }

    let ok = json!({ "d1": { "label": "l" }, "base2": { "count": 1 } });
    assert!(first.validate(&ok).valid, "{:#?}", first.validate(&ok).problems);
    for (property, value) in [
        ("d1", json!({ "count": 1 })),
        ("d2", json!({ "count": 1 })),
        ("base1", json!({ "label": "l" })),
        ("base2", json!({ "label": "l" })),
    ] {
        let report = first.validate(&json!({ property: value }));
        assert!(
            outcome(&report).iter().any(|(code, at, _)| *code == ValidationProblemCode::MissingRequired && at == &format!("/{property}")),
            "{property} keeps its own layer's body: {:#?}",
            report.problems
        );
    }
    let again = effective(&build(), &repo.document("docs/again.md", text));
    assert_eq!(again.json_schema.to_string(), first.json_schema.to_string(), "deterministic bytes");
}

#[test]
fn root_union_arms_sharing_same_named_types_keep_their_own_bodies() {
    let repo = Repo::new();
    repo.write("docs/a.yaml", "$schema:\n  kind: literal(a)\n  entry: \"{ label: string(required) }\"\n  x: entry\n  y: entry\n");
    repo.write("docs/b.yaml", "$schema:\n  kind: literal(b)\n  entry: \"{ count: number(required) }\"\n  x: entry\n  y: entry\n");
    let text = "---\n$schema: [./a.yaml, ./b.yaml]\n---\n";
    let first = effective(&repo.schemas(false), &repo.document("docs/doc.md", text));
    let schema = &first.json_schema;
    let defs = schema["$defs"].as_object().expect("union root $defs");
    assert_eq!(defs.len(), 2, "{:?}", defs.keys().collect::<Vec<_>>());
    let arms = schema["anyOf"].as_array().expect("arms");
    assert!(arms.iter().all(|arm| arm.get("$defs").is_none()), "definitions live at the union root");
    let (a_key, b_key) = (reference_key(&arms[0]["properties"]["x"]), reference_key(&arms[1]["properties"]["y"]));
    assert_ne!(a_key, b_key);
    assert!(defs[&a_key].to_string().contains("label") && defs[&b_key].to_string().contains("count"));

    assert!(first.validate(&json!({ "kind": "a", "x": { "label": "l" }, "y": { "label": "m" } })).valid);
    assert!(first.validate(&json!({ "kind": "b", "x": { "count": 1 }, "y": { "count": 2 } })).valid);
    assert!(!first.validate(&json!({ "kind": "a", "y": { "count": 1 } })).valid, "arm a keeps its own entry");
    assert!(!first.validate(&json!({ "kind": "b", "x": { "label": "l" } })).valid, "arm b keeps its own entry");
    for phase in [SchemaPhase::Launch, SchemaPhase::Completion] {
        let report = first.validate_for_phase(&json!({ "kind": "b", "y": { "count": "two" } }), phase).expect("phase");
        assert!(!report.valid, "{phase:?}");
    }
    let again = effective(&repo.schemas(false), &repo.document("docs/again.md", text));
    assert_eq!(again.json_schema.to_string(), first.json_schema.to_string(), "deterministic bytes");
}

// ── Imports: dependency invalidation, embedded, and reserved ──────────────

#[test]
fn editing_and_deleting_an_imported_shared_type_is_observed() {
    let repo = Repo::new();
    let schemas = repo.schemas(false);
    repo.write("docs/types.yaml", "kind: schema\ntypes:\n  entry: \"{ n: string }\"\n");
    let document = repo.document("docs/doc.md", "---\n$schema:\n  a: entry@./types.yaml\n  b: entry@./types.yaml\nb:\n  n: one\n---\n");
    let first = effective(&schemas, &document);
    assert!(has_definitions(&first));
    assert!(first.dependencies().iter().any(|path| path.ends_with("types.yaml")), "{:?}", first.dependencies());
    assert!(schemas.validate(&document).expect("resolves").valid);

    repo.write("docs/types.yaml", "kind: schema\ntypes:\n  entry: \"{ n: number }\"\n");
    let report = schemas.validate(&document).expect("resolves");
    assert_eq!(outcome(&report).iter().map(|(_, path, _)| path.as_str()).collect::<Vec<_>>(), ["/b/n"]);

    std::fs::remove_file(repo.root.join("docs/types.yaml")).expect("remove the import");
    assert!(schemas.validate(&document).is_err(), "a deleted import fails resolution");
}

#[test]
fn an_embedded_payload_shares_a_library_type_at_each_use() {
    const HOST: &str = "kind: schema-trigger\nmatch:\n  prompt: string(required)\n$schema:\n  e1: entry@./lib.yaml\n  e2: \"entry[]@./lib.yaml\"\n";
    const LIBRARY: &str = "kind: schema\ntypes:\n  entry: \"{ id: number(required), tag: string }\"\n";
    let repo = Repo::new();
    let snapshot = RequestSnapshot::new(repo.root.join("docs")).with_home(Some(repo.home.clone()));
    let schemas = DarkmatterSchemas::new(build_resolution_context(&snapshot).expect("context"))
        .with_trigger_discovery()
        .expect("discovery")
        .with_embedded_trigger_bundle(EmbeddedSchemaBundle::new("shared-bundle", &[("host.yaml", HOST), ("lib.yaml", LIBRARY)]))
        .expect("an embedded payload sharing a type is merge-compatible");
    let effective = effective(&schemas, &repo.document("docs/doc.md", "---\nprompt: x\n---\n"));
    assert!(has_definitions(&effective), "{:#}", effective.json_schema);

    assert!(effective.validate(&json!({ "prompt": "x", "e1": { "id": 1 }, "e2": [{ "id": 2 }] })).valid);
    let report = effective.validate(&json!({ "prompt": "x", "e1": { "id": "one" }, "e2": [{ "id": 2 }, { "tag": "t" }] }));
    assert_eq!(
        outcome(&report).iter().map(|(code, path, _)| (*code, path.as_str())).collect::<Vec<_>>(),
        [(ValidationProblemCode::MissingRequired, "/e2/1"), (ValidationProblemCode::TypeMismatch, "/e1/id")],
        "{:#?}",
        report.problems
    );
}

#[test]
fn a_reserved_partial_type_is_shared_at_each_use() {
    let repo = Repo::new();
    let schemas = repo.schemas(false);
    let twice = repo.document(
        "docs/twice.md",
        "---\n$schema:\n  first: timing@darkmatter:timing\n  second: timing@darkmatter:timing\n---\n",
    );
    let once = repo.document("docs/once.md", "---\n$schema:\n  first: timing@darkmatter:timing\n---\n");
    let (twice, once) = (effective(&schemas, &twice), effective(&schemas, &once));
    assert!(has_definitions(&twice) && !has_definitions(&once));
    assert_eq!(dereferenced(&twice.json_schema)["properties"]["first"], once.json_schema["properties"]["first"]);

    for property in ["first", "second"] {
        let report = twice.validate(&json!({ property: { "total_ms": -1, "step_ms": 1.5 } }));
        let mut paths: Vec<String> = outcome(&report).into_iter().map(|(_, path, _)| path).collect();
        paths.sort();
        assert_eq!(paths, [format!("/{property}/step_ms"), format!("/{property}/total_ms")], "{:#?}", report.problems);
    }
}
