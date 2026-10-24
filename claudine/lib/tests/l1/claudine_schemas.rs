//! Claudine's schema bundle under `claudine/schemas/` (`2026-10-21-claudine-schemas`,
//! criteria 1–5): the bundle's files load through Darkmatter's loader, the action
//! types agree with the lifecycle parser's verb inventory and Darkmatter's
//! catalogs, the bundles copy the category short forms, the ordinary and
//! `initialize` lifecycle stacks are typed three conditional levels deep and
//! the `cancelled` stack at every depth the parser accepts, and the fixture
//! corpus under
//! `tests/fixtures/claudine-schemas/` gets its decided verdicts through the
//! same trigger-discovery path DMLS uses.
//!
//! Every bundle is copied into a temporary repository's `schemas/` folder and
//! discovered from a context built from a `RequestSnapshot` with a fixture
//! home, so neither the checkout nor the process environment is consulted.
//! Each test reads the repository files it needs with `include_str!`, so a
//! change to one of them schedules exactly the tests that read it.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use biscuit_file::FileResolutionContext;
use claudine::composition::lifecycle::actions::all_lifecycle_verbs;
use claudine::composition::parse_lifecycle_config;
use darkmatter::effects::effect_descriptors;
use darkmatter::lifecycle::schema_projection::LifecycleSchemaProjection;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::expression::expression_function_descriptors;
use darkmatter::markdown::compose::{ComposeSource, RequestSnapshot, build_resolution_context};
use darkmatter::markdown::schemas::triggers::{PathSubject, parse_trigger_envelope_from_str};
use darkmatter::markdown::schemas::validate::build_position_map;
use darkmatter::markdown::schemas::{DarkmatterSchemas, EmbeddedSchemaBundle, SchemaError, ValidationReport};
use serde_json::{Value, json};

/// One bundle file: its name in `schemas/` and its text.
pub(crate) type BundleFile = (&'static str, String);

/// A temporary repository whose `schemas/` folder holds a schema bundle, with
/// a fixture home beside it.
pub(crate) struct SchemaRepo {
    _dir: tempfile::TempDir,
    docs: PathBuf,
    home: PathBuf,
}

impl SchemaRepo {
    /// A repository holding `files` in `schemas/`. The `.git` skeleton makes
    /// the folder a repository without spawning Git, so its `schemas/` is the
    /// file-tree-root schema location of every document under it.
    pub(crate) fn new(files: &[BundleFile]) -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let repo = dir.path().join("repo");
        let git = repo.join(".git");
        std::fs::create_dir_all(git.join("objects")).expect("git objects");
        std::fs::create_dir_all(git.join("refs/heads")).expect("git refs");
        std::fs::write(git.join("HEAD"), "ref: refs/heads/main\n").expect("git HEAD");
        std::fs::write(git.join("config"), "[core]\nrepositoryformatversion = 0\nbare = false\n")
            .expect("git config");
        let schemas = repo.join("schemas");
        std::fs::create_dir_all(&schemas).expect("schemas dir");
        for (name, text) in files {
            std::fs::write(schemas.join(name), text).expect("write schema");
        }
        let docs = repo.join("docs");
        std::fs::create_dir_all(&docs).expect("docs dir");
        let home = dir.path().join("home");
        std::fs::create_dir_all(&home).expect("home dir");
        Self { _dir: dir, docs, home }
    }

    /// Writes a document into the repository's `docs/` folder.
    pub(crate) fn write_document(&self, name: &str, text: &str) -> PathBuf {
        let path = self.docs.join(name);
        std::fs::write(&path, text).expect("write document");
        path
    }

    /// The context of a document in `docs/`, built from a snapshot carrying
    /// the fixture home and an empty environment.
    pub(crate) fn context(&self) -> FileResolutionContext {
        let snapshot = RequestSnapshot::new(&self.docs).with_home(Some(self.home.clone()));
        build_resolution_context(&snapshot).expect("fixture context")
    }

    /// Trigger discovery over this repository's schema locations.
    pub(crate) fn discover(&self) -> Result<Discovered, SchemaError> {
        let context = self.context();
        let schemas = DarkmatterSchemas::new(context.clone()).with_trigger_discovery()?;
        Ok(Discovered { schemas, context })
    }

    /// Trigger discovery with `bundle` registered as the last-resort
    /// location, in the order Claudine's launch verdict registers it.
    pub(crate) fn discover_embedded(&self, bundle: EmbeddedSchemaBundle) -> Result<Discovered, SchemaError> {
        let context = self.context();
        let schemas = DarkmatterSchemas::new(context.clone())
            .with_embedded_trigger_bundle(bundle)?
            .with_trigger_discovery()?;
        Ok(Discovered { schemas, context })
    }
}

/// `files` as an embedded bundle named like Claudine's.
///
/// Darkmatter memoizes a loaded embedded trigger for the process by the
/// address of its `'static` text, so reusing an address skips resolving the
/// 2.5 MB `claudine.yaml` payload again. A trigger text is leaked once per
/// (trigger text, every library text) combination and reused only for that
/// same combination: a cell that edits a library always gets fresh trigger
/// addresses, and an unedited trigger beside an edited sibling trigger keeps
/// its memoized load.
pub(crate) fn embedded_bundle(files: &[BundleFile]) -> EmbeddedSchemaBundle {
    use claudine::composition::schema::bundle::CLAUDINE_TRIGGER_FILES;
    static TRIGGER_TEXTS: std::sync::Mutex<BTreeMap<(String, String), &'static str>> =
        std::sync::Mutex::new(BTreeMap::new());
    let libraries: String = files
        .iter()
        .filter(|(name, _)| !CLAUDINE_TRIGGER_FILES.contains(name))
        .map(|(name, text)| format!("{name}\0{text}\0"))
        .collect();
    let files: Vec<(&'static str, &'static str)> = files
        .iter()
        .map(|(name, text)| {
            let leak = || &*Box::leak(text.clone().into_boxed_str());
            if !CLAUDINE_TRIGGER_FILES.contains(name) {
                return (*name, leak());
            }
            let mut texts = TRIGGER_TEXTS.lock().expect("trigger texts");
            (*name, *texts.entry((format!("{name}\0{text}"), libraries.clone())).or_insert_with(leak))
        })
        .collect();
    EmbeddedSchemaBundle::new(
        claudine::composition::schema::bundle::CLAUDINE_SCHEMA_BUNDLE_NAME,
        Box::leak(files.into_boxed_slice()),
    )
}

/// A discovered bundle, reused for every document of one test.
pub(crate) struct Discovered {
    pub(crate) schemas: DarkmatterSchemas,
    context: FileResolutionContext,
}

/// What Darkmatter decided for one document.
pub(crate) struct Judged {
    /// Whether `claudine.yaml`'s trigger matched.
    pub(crate) claudine: bool,
    /// Whether `inline-compose.yaml`'s trigger matched.
    pub(crate) inline: bool,
    /// The effective schema's top-level properties.
    pub(crate) properties: BTreeSet<String>,
    /// The validation verdict.
    pub(crate) report: ValidationReport,
    /// The frontmatter as the validated document holds it: converted by the
    /// one conversion engine, as Claudine's preparation reads it.
    pub(crate) held: Value,
}

impl Discovered {
    /// Whether the discovered trigger whose file text is `trigger` selects
    /// the document at `path`, evaluated on the discovered registry.
    pub(crate) fn selects(&self, path: &Path, trigger: &str) -> bool {
        let markdown = Markdown::try_from(path).expect("document loads");
        let frontmatter = frontmatter_json(&markdown);
        let envelope = parse_trigger_envelope_from_str(trigger)
            .expect("the trigger parses")
            .expect("a trigger envelope");
        let subject = PathSubject::new(path, &self.context);
        self.schemas
            .trigger_registry()
            .expect("discovery installed a registry")
            .triggers
            .iter()
            .filter(|discovered| discovered.envelope == envelope)
            .any(|discovered| discovered.envelope.conditions.matches(&frontmatter, &subject))
    }

    /// Judges the document at `path`: which shipped triggers its frontmatter
    /// selects (evaluated on the discovered registry, identified by their
    /// envelopes), the effective schema, and the validation report.
    pub(crate) fn judge(&self, path: &Path, claudine_trigger: &str, inline_trigger: &str) -> Result<Judged, SchemaError> {
        let markdown = Markdown::try_from(path)
            .expect("document loads")
            .with_source(ComposeSource::File(path.to_path_buf()));
        let frontmatter = frontmatter_json(&markdown);
        // A matrix cell may have edited a file out of the trigger envelope;
        // such a file selects nothing.
        let claudine_envelope = parse_trigger_envelope_from_str(claudine_trigger).ok().flatten();
        let inline_envelope = parse_trigger_envelope_from_str(inline_trigger).ok().flatten();
        let subject = PathSubject::new(path, &self.context);
        let mut claudine = false;
        let mut inline = false;
        for trigger in &self.schemas.trigger_registry().expect("discovery installed a registry").triggers {
            let selected = trigger.envelope.conditions.matches(&frontmatter, &subject);
            if claudine_envelope.as_ref() == Some(&trigger.envelope) {
                claudine |= selected;
            } else if inline_envelope.as_ref() == Some(&trigger.envelope) {
                inline |= selected;
            }
        }
        let effective = self.schemas.effective_for(&markdown)?;
        let properties = effective
            .as_ref()
            .and_then(|effective| effective.json_schema["properties"].as_object())
            .map(|properties| properties.keys().cloned().collect())
            .unwrap_or_default();
        // The same judgment `DarkmatterSchemas::validate` makes, without
        // resolving the effective schema a second time.
        let mut data = frontmatter.clone();
        if let Some(map) = data.as_object_mut() {
            map.remove("$schema");
        }
        let report = match &effective {
            Some(effective) => effective.validate(&data),
            None => ValidationReport {
                valid: true,
                problems: Vec::new(),
                pending: Vec::new(),
                deferred: Vec::new(),
                advisories: Vec::new(),
            },
        };
        let held = match &effective {
            Some(effective) => effective.coerce_frontmatter_with_pending(&data, &Default::default()).value,
            None => data,
        };
        Ok(Judged {
            claudine,
            inline,
            properties,
            report,
            held,
        })
    }
}

pub(crate) fn frontmatter_json(markdown: &Markdown) -> Value {
    Value::Object(
        markdown
            .frontmatter()
            .as_map()
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect(),
    )
}

/// Every problem as `(instance path, message)`.
pub(crate) fn problems(report: &ValidationReport) -> Vec<(String, String)> {
    report
        .problems
        .iter()
        .map(|problem| (problem.path.clone(), problem.message.clone()))
        .collect()
}

/// The resolved JSON Schema of every named type in `exports`, imported from
/// `file` in a discovered bundle: each export becomes one property of an
/// inline document schema, so the result is exactly what a reference to it
/// lowers to.
fn resolved_exports(repo: &SchemaRepo, file: &str, exports: &[&str]) -> BTreeMap<String, Value> {
    let mut frontmatter = String::from("---\n$schema:\n");
    for export in exports {
        frontmatter.push_str(&format!("    t_{export}: \"{export}@../schemas/{file}\"\n"));
    }
    frontmatter.push_str("---\n");
    let path = repo.write_document(&format!("exports-{file}.md"), &frontmatter);
    let markdown = Markdown::try_from(path.as_path())
        .expect("document loads")
        .with_source(ComposeSource::File(path.clone()));
    let effective = DarkmatterSchemas::new(repo.context())
        .effective_for(&markdown)
        .unwrap_or_else(|error| panic!("the exports of {file} resolve: {error}"))
        .expect("an inline schema is effective");
    exports
        .iter()
        .map(|export| {
            let mut fragment = effective.json_schema["properties"][format!("t_{export}")].clone();
            assert!(!fragment.is_null(), "{file} exports `{export}`");
            strip_optional_null(&mut fragment);
            (export.to_string(), fragment)
        })
        .collect()
}

/// An optional property lowers to `anyOf: [null, …]`; this returns the
/// non-null part, so two references compare by their own shape.
fn strip_optional_null(fragment: &mut Value) {
    let Some(arms) = fragment.get("anyOf").and_then(Value::as_array).cloned() else {
        return;
    };
    let rest: Vec<Value> = arms.into_iter().filter(|arm| arm != &json!({ "type": "null" })).collect();
    *fragment = match rest.as_slice() {
        [single] => single.clone(),
        _ => json!({ "anyOf": rest }),
    };
}

/// The arms of a lowered property: its `anyOf` arms without the null arm, or
/// the fragment itself.
fn arms_of(fragment: &Value) -> Vec<Value> {
    match fragment.get("anyOf").and_then(Value::as_array) {
        Some(arms) => arms.iter().filter(|arm| *arm != &json!({ "type": "null" })).cloned().collect(),
        None => vec![fragment.clone()],
    }
}

/// Every `description` annotation in `fragment`.
fn collect_descriptions(fragment: &Value, found: &mut Vec<String>) {
    match fragment {
        Value::Object(map) => {
            for (key, value) in map {
                match (key.as_str(), value) {
                    ("description", Value::String(text)) => found.push(text.clone()),
                    _ => collect_descriptions(value, found),
                }
            }
        }
        Value::Array(items) => items.iter().for_each(|item| collect_descriptions(item, found)),
        _ => {}
    }
}

/// `fragment` without its `description` annotations, so two declarations
/// compare by their value types alone.
fn without_descriptions(fragment: &Value) -> Value {
    match fragment {
        Value::Object(map) => Value::Object(
            map.iter()
                .filter(|(key, _)| key.as_str() != "description")
                .map(|(key, value)| (key.clone(), without_descriptions(value)))
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(without_descriptions).collect()),
        other => other.clone(),
    }
}

/// The one non-null arm of an optional property.
fn present(fragment: &Value) -> Value {
    let arms = arms_of(fragment);
    assert_eq!(arms.len(), 1, "a single-type property: {fragment}");
    arms[0].clone()
}

fn property_names(object: &Value) -> BTreeSet<String> {
    object["properties"]
        .as_object()
        .map(|properties| properties.keys().cloned().collect())
        .unwrap_or_default()
}

/// The keys beside a single action that `uncounted-keys` leaves out of a
/// category's one-verb count.
const SIBLINGS: &[&str] = &["when", "no_error"];

/// A category's verbs: its keys without the sibling keys.
fn verb_names(object: &Value) -> BTreeSet<String> {
    property_names(object).into_iter().filter(|key| !SIBLINGS.contains(&key.as_str())).collect()
}

fn is_closed_object(fragment: &Value) -> bool {
    fragment["type"] == "object" && fragment["additionalProperties"] == json!(false)
}

/// The hand-typed verbs the parser knows: every verb of
/// `all_lifecycle_verbs()` that is neither a Darkmatter side effect nor an
/// expression function, plus `set`, which the parser handles itself.
fn parser_hand_verbs() -> BTreeSet<String> {
    let generated = generated_verbs();
    let mut verbs: BTreeSet<String> = all_lifecycle_verbs()
        .into_iter()
        .filter(|verb| !generated.contains(*verb))
        .map(str::to_string)
        .collect();
    verbs.insert("set".to_string());
    verbs
}

/// Darkmatter's side-effect and expression-function names, without `set`,
/// which is typed once as the hand verb.
pub(crate) fn generated_verbs() -> BTreeSet<String> {
    let name_of = |signature: &str| signature.split('(').next().expect("a name").trim().to_string();
    effect_descriptors()
        .iter()
        .map(|descriptor| name_of(descriptor.signature))
        .chain(expression_function_descriptors().iter().map(|descriptor| name_of(descriptor.signature)))
        .filter(|name| name != "set")
        .collect()
}

/// The verbs the parser accepts as a bare list item (`- stop`), found by
/// parsing each known verb that way. `initialize` is the one event that also
/// places `skip`.
fn parser_bare_verbs() -> BTreeSet<String> {
    all_lifecycle_verbs()
        .into_iter()
        .filter(|verb| {
            parse_lifecycle_config(&json!({ "initialize": [verb] }), Path::new("bare.md")).is_ok()
        })
        .map(str::to_string)
        .collect()
}

const ACTION_EXPORTS: &[&str] = &["communicate", "flow", "shell", "expression", "bundle", "initialize-bundle"];
const CATEGORIES: &[&str] = &["communicate", "flow", "shell", "expression"];

macro_rules! shipped_bundle {
    () => {
        [
            ("action.yaml", include_str!("../../../schemas/action.yaml").to_string()),
            ("lifecycle.yaml", include_str!("../../../schemas/lifecycle.yaml").to_string()),
            ("linking.yaml", include_str!("../../../schemas/linking.yaml").to_string()),
            ("claudine.yaml", include_str!("../../../schemas/claudine.yaml").to_string()),
            ("inline-compose.yaml", include_str!("../../../schemas/inline-compose.yaml").to_string()),
            ("sequence-types.yaml", include_str!("../../../schemas/sequence-types.yaml").to_string()),
            ("sequence.yaml", include_str!("../../../schemas/sequence.yaml").to_string()),
        ]
    };
}
pub(crate) use shipped_bundle;

// ── AC2: inventories ───────────────────────────────────────────────────────

#[test]
fn hand_verbs_equal_the_parser_inventory() {
    let bundle = [("action.yaml", include_str!("../../../schemas/action.yaml").to_string())];
    let repo = SchemaRepo::new(&bundle);
    let types = resolved_exports(&repo, "action.yaml", CATEGORIES);
    let generated = generated_verbs();
    let hand: BTreeSet<String> = CATEGORIES
        .iter()
        .flat_map(|category| verb_names(&types[*category]))
        .filter(|verb| !generated.contains(verb))
        .collect();
    let parser = parser_hand_verbs();

    let unparsed: BTreeSet<&String> = hand.difference(&parser).collect();
    assert!(unparsed.is_empty(), "action types the parser does not accept: {unparsed:?}");
    let missing: BTreeSet<&String> = parser.difference(&hand).collect();
    assert!(missing.is_empty(), "parser verbs missing from the action types: {missing:?}");

    // Every verb sits in exactly one category.
    let mut seen = BTreeMap::new();
    for category in CATEGORIES {
        for verb in verb_names(&types[*category]) {
            if let Some(previous) = seen.insert(verb.clone(), *category) {
                panic!("`{verb}` is in both `{previous}` and `{category}`");
            }
        }
    }

    // `defer` is the one verb the parser accepts and the runtime refuses, and
    // its description says so where an author sees it; `break` and `prep`
    // run, and their descriptions say what happens when they cannot.
    let descriptions_of = |verb: &str| {
        let mut descriptions = Vec::new();
        collect_descriptions(&types["flow"]["properties"][verb], &mut descriptions);
        descriptions
    };
    let defer = descriptions_of("defer");
    assert!(defer.iter().any(|description| description.contains("planned")), "`defer` is described as planned: {defer:?}");
    for (verb, says) in [("break", "a break outside every loop fails its event"), ("prep", "a target that fails fails this event")] {
        let descriptions = descriptions_of(verb);
        assert!(
            descriptions.iter().any(|description| description.contains(says))
                && !descriptions.iter().any(|description| description.contains("planned") || description.contains("running it fails")),
            "`{verb}` is described as running: {descriptions:?}"
        );
    }
}

#[test]
fn generated_verb_keys_equal_the_darkmatter_catalogs() {
    let bundle = [
        ("action.yaml", include_str!("../../../schemas/action.yaml").to_string()),
        ("lifecycle.yaml", include_str!("../../../schemas/lifecycle.yaml").to_string()),
    ];
    let repo = SchemaRepo::new(&bundle);
    let types = resolved_exports(&repo, "action.yaml", ACTION_EXPORTS);
    let catalogs = generated_verbs();
    assert!(catalogs.len() >= 120, "the catalogs shrank to {}", catalogs.len());
    let hand = parser_hand_verbs();
    for (export, expected_generated) in [("expression", &catalogs), ("bundle", &catalogs), ("initialize-bundle", &catalogs)] {
        let generated: BTreeSet<String> = property_names(&types[export])
            .into_iter()
            .filter(|verb| !hand.contains(verb) && verb != "when" && verb != "no_error")
            .collect();
        assert_eq!(&generated, expected_generated, "`{export}` lists every catalog verb once");
        for verb in &generated {
            let value = without_descriptions(&types[export]["properties"][verb.as_str()]);
            assert_ne!(value, json!({}), "`{export}.{verb}` is typed");
        }
    }

    // Each generated key names the projected per-verb types by reference: a
    // category takes the short and (when the verb has one) the long form, a
    // bundle and the loop's loose bundle the short form only, and the loop's
    // single-action arm the category's forms.
    let projection = LifecycleSchemaProjection::shipped().expect("the shipped catalogs project");
    let projected: BTreeSet<String> = projection.verbs.iter().map(|(_, verb)| verb.name.clone()).collect();
    assert_eq!(projected, catalogs, "the projection covers the catalog verbs");
    let authored = |file: &str| -> biscuit_file::serde_yaml_ng::Value {
        let text = match file {
            "action.yaml" => include_str!("../../../schemas/action.yaml"),
            _ => include_str!("../../../schemas/lifecycle.yaml"),
        };
        biscuit_file::serde_yaml_ng::from_str(text).expect("the file parses")
    };
    let action = authored("action.yaml");
    let lifecycle = authored("lifecycle.yaml");
    let reference = |name: String| biscuit_file::serde_yaml_ng::Value::String(format!("{name}@darkmatter:lifecycle"));
    for (_, verb) in &projection.verbs {
        let short = reference(verb.short_type_name());
        let category = match verb.long_type_name() {
            Some(long) => biscuit_file::serde_yaml_ng::Value::Sequence(vec![short.clone(), reference(long)]),
            None => short.clone(),
        };
        assert_eq!(action["types"]["expression"][verb.name.as_str()], category, "`expression.{}`", verb.name);
        for bundle in ["bundle", "initialize-bundle", "cancelled-bundle"] {
            assert_eq!(action["types"][bundle][verb.name.as_str()], short, "`{bundle}.{}`", verb.name);
        }
        if verb.name != "max" {
            assert_eq!(lifecycle["types"]["loop-bundle"][verb.name.as_str()], short, "`loop-bundle.{}`", verb.name);
            assert_eq!(lifecycle["types"]["loop-single"][verb.name.as_str()], category, "`loop-single.{}`", verb.name);
        }
    }

    // The loop declares `max` once, as its iteration control, and every
    // other catalog verb as a loose key, in both arms.
    let lifecycle = resolved_exports(&repo, "lifecycle.yaml", &["loop-bundle", "loop-single"]);
    let loop_keys = property_names(&lifecycle["loop-bundle"]);
    let single_keys = property_names(&lifecycle["loop-single"]);
    for verb in &catalogs {
        assert!(loop_keys.contains(verb) && single_keys.contains(verb), "both loop arms list `{verb}`");
    }
    assert_eq!(single_keys, &loop_keys - &BTreeSet::from(["gate".to_string()]), "the arms differ only by `gate`");
    for arm in ["loop-bundle", "loop-single"] {
        let max_arms = arms_of(&lifecycle[arm]["properties"]["max"]);
        assert_eq!(max_arms.len(), 2, "`{arm}.max` is a count or a template: {max_arms:?}");
        assert!(max_arms.iter().any(|arm| arm["type"] == "integer"), "`{arm}.max` is the iteration count");
        assert!(
            max_arms.iter().any(|arm| arm["type"] == "string" && arm["pattern"].is_string()),
            "`{arm}.max` may be a whole-value interpolation"
        );
        assert!(!property_names(&lifecycle[arm]).contains("fail_fast"), "a loop has no `fail_fast`");
    }
    assert!(!single_keys.contains("gate"), "a `gate` never sits beside a loose verb");
}

#[test]
fn bare_verb_enum_equals_what_the_parser_accepts_as_a_bare_item() {
    let bundle = [
        ("action.yaml", include_str!("../../../schemas/action.yaml").to_string()),
        ("lifecycle.yaml", include_str!("../../../schemas/lifecycle.yaml").to_string()),
    ];
    let repo = SchemaRepo::new(&bundle);
    let types = resolved_exports(&repo, "lifecycle.yaml", &["bare-verb"]);
    let declared: BTreeSet<String> = types["bare-verb"]["enum"]
        .as_array()
        .expect("bare-verb is an enum")
        .iter()
        .map(|member| member.as_str().expect("string member").to_string())
        .collect();
    let parser = parser_bare_verbs();
    assert!(
        ["stop", "skip", "error", "retry"].iter().all(|verb| parser.contains(*verb)),
        "the derivation found the zero-argument directives: {parser:?}"
    );
    assert_eq!(declared, parser);
}

// ── AC4: bundles copy short forms; closure ─────────────────────────────────

#[test]
fn bundle_keys_are_the_union_of_their_categories_plus_when_and_no_error() {
    let bundle = [("action.yaml", include_str!("../../../schemas/action.yaml").to_string())];
    let repo = SchemaRepo::new(&bundle);
    let types = resolved_exports(&repo, "action.yaml", ACTION_EXPORTS);
    for (export, categories) in [
        ("bundle", &["communicate", "flow", "shell", "expression"][..]),
        ("initialize-bundle", &["communicate", "flow", "expression"][..]),
    ] {
        let mut expected: BTreeSet<String> =
            categories.iter().flat_map(|category| property_names(&types[*category])).collect();
        expected.insert("when".to_string());
        expected.insert("no_error".to_string());
        assert_eq!(property_names(&types[export]), expected, "`{export}`'s keys");
        assert_eq!(present(&types[export]["properties"]["when"])["format"], "darkmatter-expression");
        assert_eq!(present(&types[export]["properties"]["no_error"])["type"], "boolean");
    }
    // A category is a single-action dictionary: one verb, with `when` and
    // `no_error` beside it and left out of the one-key count.
    for category in CATEGORIES {
        let object = &types[*category];
        assert!(SIBLINGS.iter().all(|key| property_names(object).contains(*key)), "`{category}` takes `when` and `no_error`");
        assert_eq!(present(&object["properties"]["when"])["format"], "darkmatter-expression");
        assert_eq!(present(&object["properties"]["no_error"])["type"], "boolean");
        assert!(object.get("maxProperties").is_none(), "`{category}` counts its verb apart from its siblings: {object}");
        assert_eq!(object["allOf"].as_array().map(Vec::len), Some(4), "`{category}` bounds each sibling combination");
    }
    assert!(
        !property_names(&types["initialize-bundle"]).contains("shell"),
        "the initialize bundle has no shell verb"
    );
}

#[test]
fn bundle_values_are_the_category_short_forms() {
    let bundle = [("action.yaml", include_str!("../../../schemas/action.yaml").to_string())];
    let repo = SchemaRepo::new(&bundle);
    let types = resolved_exports(&repo, "action.yaml", ACTION_EXPORTS);
    let generated = generated_verbs();
    let mut long_forms = 0;
    for category in CATEGORIES {
        for verb in verb_names(&types[*category]) {
            let declared = &types[*category]["properties"][verb.as_str()];
            let arms: Vec<Value> = arms_of(declared).iter().map(without_descriptions).collect();
            // A long form is a closed-object arm. A hand verb has at most one
            // and `set` none; a generated verb has one per overload and
            // optional-parameter subset, whose shapes
            // `generated_verb_keys_equal_the_darkmatter_catalogs` pins.
            let (long, short): (Vec<Value>, Vec<Value>) = arms.into_iter().partition(is_closed_object);
            if generated.contains(&verb) {
                for export in ["bundle", "initialize-bundle"] {
                    let copied: Vec<Value> =
                        arms_of(&types[export]["properties"][verb.as_str()]).iter().map(without_descriptions).collect();
                    assert_eq!(copied, short, "`{export}.{verb}` is `{category}.{verb}`'s short form");
                }
                continue;
            }
            assert!(long.len() <= 1, "`{verb}` has at most one long form: {declared}");
            if let Some(long) = long.first() {
                long_forms += 1;
                assert_eq!(
                    present(&long["properties"]["no_error"])["type"],
                    "boolean",
                    "`{verb}`'s long form carries `no_error: boolean`: {long}"
                );
            }
            for export in ["bundle", "initialize-bundle"] {
                if export == "initialize-bundle" && *category == "shell" {
                    continue;
                }
                let copied: Vec<Value> =
                    arms_of(&types[export]["properties"][verb.as_str()]).iter().map(without_descriptions).collect();
                assert_eq!(copied, short, "`{export}.{verb}` is `{category}.{verb}`'s short form");
            }
        }
    }
    assert_eq!(long_forms, 20, "every hand verb but `set` has a long form");
    assert!(
        arms_of(&types["expression"]["properties"]["set"]).iter().all(|arm| !is_closed_object(arm)),
        "`set` has no long form"
    );
}

#[test]
fn action_objects_are_closed_and_data_maps_stay_open() {
    let bundle = [
        ("action.yaml", include_str!("../../../schemas/action.yaml").to_string()),
        ("linking.yaml", include_str!("../../../schemas/linking.yaml").to_string()),
    ];
    let repo = SchemaRepo::new(&bundle);
    let types = resolved_exports(&repo, "action.yaml", ACTION_EXPORTS);
    // A generated verb's object-typed parameter (`merge_frontmatter`'s `obj`,
    // `append_line`'s structured `text`) is data, typed by its catalog.
    let generated = generated_verbs();
    for export in ACTION_EXPORTS {
        let fragment = &types[*export];
        assert!(is_closed_object(fragment), "`{export}` is a closed object: {fragment}");
        assert!(fragment.get("patternProperties").is_none(), "`{export}` has no pattern keys");
        for (verb, declared) in fragment["properties"].as_object().expect("properties") {
            for arm in arms_of(declared) {
                if arm["type"] != "object" {
                    continue;
                }
                if verb == "set" {
                    assert!(arm.get("additionalProperties").is_none() && arm.get("properties").is_none(), "`{export}.set` is an open mapping: {arm}");
                    continue;
                }
                assert!(is_closed_object(&arm), "`{export}.{verb}`'s long form is closed: {arm}");
                for (parameter, value) in arm["properties"].as_object().expect("parameters") {
                    for parameter_arm in arms_of(value) {
                        if parameter_arm["type"] == "object" && !generated.contains(verb) {
                            assert_eq!(parameter, "with", "`{export}.{verb}.{parameter}` is a scalar parameter");
                            assert!(parameter_arm.get("additionalProperties").is_none(), "`with` stays open");
                        }
                    }
                }
            }
        }
    }
    let linking = resolved_exports(&repo, "linking.yaml", &["metadata"]);
    assert_eq!(linking["metadata"]["type"], "object");
    assert!(linking["metadata"].get("additionalProperties").is_none(), "linking metadata stays open");

    // No catch-all or pattern key is authored anywhere in the action types.
    let text = include_str!("../../../schemas/action.yaml");
    for key in ["<string>", "<pattern::", "<starting::", "<ending::"] {
        assert!(!text.contains(key), "action.yaml declares no `{key}` key");
    }
}

#[test]
fn required_long_form_fields_survive_every_reference() {
    let repo = SchemaRepo::new(&shipped_bundle!());
    let discovered = repo.discover().expect("the shipped bundle loads");
    let claudine = include_str!("../../../schemas/claudine.yaml");
    let inline = include_str!("../../../schemas/inline-compose.yaml");
    for (event, item, missing) in [
        ("start", "{ shell: { on_error: oops } }", "command"),
        ("start", "{ proxy: { with: { a: 1 } } }", "target"),
        ("failure", "{ resume: { max_attempts: 2 } }", "message"),
        ("success", "{ say: { no_error: true } }", "text"),
        ("initialize", "{ info: {} }", "text"),
    ] {
        let path = repo.write_document("required.md", &format!("---\n{event}:\n    - {item}\n---\n"));
        let judged = discovered.judge(&path, claudine, inline).expect("judged");
        assert!(!judged.report.valid, "`{event}: [{item}]` lacks `{missing}`");
        assert!(
            judged.report.problems.iter().all(|problem| problem.path.starts_with(&format!("/{event}/0"))),
            "{:?}",
            problems(&judged.report)
        );
    }
}

// ── AC1 / AC3: loading and typed levels ────────────────────────────────────

#[test]
fn every_file_loads_and_every_export_resolves() {
    let repo = SchemaRepo::new(&shipped_bundle!());
    let action = resolved_exports(&repo, "action.yaml", &["interpolation", "count", "communicate", "flow", "shell", "expression", "bundle", "initialize-bundle"]);
    assert_eq!(action.len(), 8);
    let lifecycle_exports = [
        "bare-verb", "stack", "item", "conditional", "initialize-stack", "initialize-item", "initialize-conditional",
        "cancelled-stack", "cancelled-item", "cancelled-conditional", "loop-action", "loop-action-entry", "loop",
    ];
    let lifecycle = resolved_exports(&repo, "lifecycle.yaml", &lifecycle_exports);
    assert_eq!(lifecycle.len(), lifecycle_exports.len());
    let linking_exports = [
        "name", "allowed-tools", "tools", "skills", "license", "compatibility", "metadata",
        "user-invocable", "disable-model-invocation", "argument-hint", "max_turns",
    ];
    assert_eq!(resolved_exports(&repo, "linking.yaml", &linking_exports).len(), 11);
    let sequence_exports = ["steps", "step", "task", "group"];
    assert_eq!(resolved_exports(&repo, "sequence-types.yaml", &sequence_exports).len(), sequence_exports.len());

    // The three triggers load, and each payload resolves: a document
    // selecting claudine.yaml and inline-compose.yaml gets every payload
    // property of both.
    let discovered = repo.discover().expect("the shipped bundle loads");
    assert_eq!(discovered.schemas.trigger_registry().expect("registry").triggers.len(), 3);
    let path = repo.write_document("both.md", "---\nprompt: hi\n---\n");
    let judged = discovered
        .judge(
            &path,
            include_str!("../../../schemas/claudine.yaml"),
            include_str!("../../../schemas/inline-compose.yaml"),
        )
        .expect("both payloads resolve");
    assert!(judged.claudine && judged.inline);
    for property in ["initialize", "start", "loop", "agent", "hash", "last_updated", "max_turns", "verbosity"] {
        assert!(judged.properties.contains(property), "the effective schema declares `{property}`");
    }
    assert!(judged.report.valid, "{:?}", problems(&judged.report));
}

/// A stack whose innermost conditional is `depth` levels deep and holds
/// `leaf` as its only `then` action.
fn nested(depth: usize, leaf: &str) -> String {
    let mut body = format!("[{leaf}]");
    for level in (1..=depth).rev() {
        body = format!("[{{ when: \"ctx.level == {level}\", then: {body}, else: [{{ warn: \"no\" }}] }}]");
    }
    body
}

/// A stack whose innermost conditional is `depth` levels deep and holds
/// `leaf` as its only `else` action: an unselected branch when the guards
/// are false.
fn nested_else(depth: usize, leaf: &str) -> String {
    let mut body = format!("[{leaf}]");
    for level in (1..=depth).rev() {
        let (then, otherwise) = if level == depth {
            ("[{ warn: \"no\" }]".to_string(), body)
        } else {
            (body, "[{ warn: \"no\" }]".to_string())
        };
        body = format!("[{{ when: \"ctx.level == {level}\", then: {then}, else: {otherwise} }}]");
    }
    body
}

/// Judges stacks placed by `holder` (an event, or the loop's gate) at depths
/// 1, 3, 5, 6, and 8: the recursive family types every level, so a good leaf
/// is valid and a bad leaf or a malformed long form is rejected at every
/// depth, in `then` and in `else`. The schema encodes no depth limit, so depths past the parser's
/// five levels are judged by type alone.
fn assert_typed_at_every_depth(bundle: &[BundleFile], claudine: &str, inline: &str, holder: &str, leaf_bad: &str) {
    let repo = SchemaRepo::new(bundle);
    let discovered = repo.discover().expect("the shipped bundle loads");
    let close = if holder.starts_with("loop") { " }" } else { "" };
    let judge = |name: &str, stack: String| {
        let path = repo.write_document(&format!("{name}.md"), &format!("---\n{holder}{stack}{close}\n---\n"));
        discovered.judge(&path, claudine, inline).expect("judged").report
    };
    for depth in [1, 3, 5, 6, 8] {
        for (branch, build) in [("then", nested as fn(usize, &str) -> String), ("else", nested_else)] {
            let valid = judge("ok", build(depth, "{ info: hi }"));
            assert!(valid.valid, "{holder}depth {depth} {branch}: {:?}", problems(&valid));
            let invalid = judge("bad", build(depth, leaf_bad));
            assert!(!invalid.valid, "{holder}depth {depth} {branch}: a bad leaf is rejected");
            let long = judge("long", build(depth, "{ warn: { txet: careful } }"));
            assert!(!long.valid, "{holder}depth {depth} {branch}: a malformed long form is rejected");
        }
    }
    for depth in [1, 4] {
        let mixed = judge("mixed", nested(depth, "{ when: \"ctx.x\", then: [stop], info: hi }"));
        assert!(!mixed.valid, "{holder}a conditional beside a sibling verb is rejected below level {depth}");
        let orphan = judge("orphan", nested(depth, "{ when: \"ctx.x\", else: [stop] }"));
        assert!(!orphan.valid, "{holder}an `else` without `then` is rejected below level {depth}");
        let sibling_no_error = judge("no-error", nested(depth, "{ when: \"ctx.x\", then: [stop], no_error: true }"));
        assert!(!sibling_no_error.valid, "{holder}a conditional's sibling `no_error` is rejected below level {depth}");
        let bundle = judge("bundle", nested(depth, "{ when: \"ctx.x\", info: hi, warn: careful }"));
        assert!(bundle.valid, "{holder}a guarded bundle below level {depth}: {:?}", problems(&bundle));
        let bare = judge("bare", nested(depth, "stop"));
        assert!(bare.valid, "{holder}a bare verb below level {depth}: {:?}", problems(&bare));
    }
}

#[test]
fn event_stacks_are_typed_at_every_depth() {
    assert_typed_at_every_depth(
        &shipped_bundle!(),
        include_str!("../../../schemas/claudine.yaml"),
        include_str!("../../../schemas/inline-compose.yaml"),
        "start: ",
        "{ inf0: hi }",
    );
}

#[test]
fn initialize_stacks_exclude_shell_at_every_depth() {
    assert_typed_at_every_depth(
        &shipped_bundle!(),
        include_str!("../../../schemas/claudine.yaml"),
        include_str!("../../../schemas/inline-compose.yaml"),
        "initialize: ",
        "{ shell: ls }",
    );
}

#[test]
fn loop_gate_is_typed_at_every_depth() {
    assert_typed_at_every_depth(
        &shipped_bundle!(),
        include_str!("../../../schemas/claudine.yaml"),
        include_str!("../../../schemas/inline-compose.yaml"),
        "loop: { while: \"x < 3\", gate: ",
        "{ inf0: hi }",
    );
}

// ── AC5: the fixture corpus ────────────────────────────────────────────────

/// The schema verdict a fixture must get.
enum Verdict {
    Valid,
    /// Every listed `(instance path, text in the message)` is reported, and
    /// every problem lies under the first listed path's event.
    Invalid(&'static [(&'static str, &'static str)]),
}

/// What Claudine's lifecycle parser must decide for a fixture's stacks.
enum Parser {
    Accepts,
    /// Rejects, with this text in the error.
    Rejects(&'static str),
    /// The row's subject is not a lifecycle stack.
    NotJudged,
}

use Parser::{Accepts, NotJudged, Rejects};
use Verdict::{Invalid, Valid};

/// `(claudine.yaml matches, inline-compose.yaml matches)`.
const CLAUDINE: (bool, bool) = (true, false);
const BOTH: (bool, bool) = (true, true);
const NEITHER: (bool, bool) = (false, false);

/// One AC5 row: the fixture's name and text, the triggers it must select,
/// and its schema and parser verdicts.
type Case = (&'static str, &'static str, (bool, bool), Verdict, Parser);

/// The AC5 rows, split by subject so each test stays small; together they
/// name every file in `tests/fixtures/claudine-schemas/`.
pub(crate) const AC5_GROUPS: &[&[&str]] = &[
    &[
        "depth1",
        "depth3",
        "depth5",
        "depth6",
        "depth6-wrong-verb",
        "depth5-guarded-bundle",
        "depth4-completion",
        "root-dictionary",
        "bundle-in-then",
        "loop-gate",
        "long-forms",
        "bare-stop",
    ],
    &[
        "shell-in-initialize-list",
        "shell-in-initialize-then",
        "shell-in-initialize-root-dictionary",
        "shell-in-initialize-depth3-then",
        "shell-expansion-in-initialize-set",
        "shell-in-start",
        "message-shell-bundle-with-when",
    ],
    &[
        "when-with-then-and-sibling",
        "misspelled-parameter",
        "misspelled-verb",
        "misspelled-verb-in-then",
        "long-form-in-bundle",
        "long-form-in-root-dictionary",
    ],
    &[
        "start-number",
        "agent-empty-list",
        "loop-number",
        "empty-frontmatter",
        "unrelated-ticket",
        "null-identifiers",
        "controls-beside-ticket",
        "prompt-with-malformed-sequence",
        "linked-prompt",
    ],
    &[
        "generated-verb-typed-value",
        "generated-verb-wrong-type",
        "generated-verb-convertible-text",
        "generated-verb-deferred",
        "generated-verb-deferred-beside-unknown-parameter",
        "generated-verb-deferred-beside-extra-argument",
        "generated-verb-deferred-beside-bad-sibling",
        "generated-verb-malformed-template",
        "generated-verb-malformed-string-template",
        "handwritten-action-malformed-template",
        "optional-nonnullable-null",
        "ensure-file-overloads",
        "variadic-and-or",
        "generated-verb-numeric-recovery",
        "generated-verb-excluded-source",
    ],
    &[
        "single-action-root-dictionary",
        "single-action-branch-dictionaries",
        "single-action-loop-loose",
        "long-forms-in-loop-bundle",
        "conditional-root-dictionary",
    ],
];

/// Judges every row through trigger discovery, the effective schema, and
/// validation, then through Claudine's lifecycle parser. Selection is
/// asserted on its own, before the verdict.
fn assert_ac5(group: usize, bundle: &[BundleFile], claudine: &str, inline: &str, cases: &[Case]) {
    assert_eq!(
        cases.iter().map(|case| case.0).collect::<Vec<_>>(),
        AC5_GROUPS[group],
        "the rows are the group's fixtures"
    );
    let repo = SchemaRepo::new(bundle);
    let discovered = repo.discover().expect("the shipped bundle loads");
    for (name, text, (claudine_selected, inline_selected), verdict, parser) in cases {
        let path = repo.write_document(&format!("{name}.md"), text);
        let judged = discovered.judge(&path, claudine, inline).expect("judged");

        assert_eq!(
            (judged.claudine, judged.inline),
            (*claudine_selected, *inline_selected),
            "{name}: trigger selection"
        );
        assert_eq!(
            judged.properties.contains("initialize"),
            *claudine_selected,
            "{name}: Claudine's payload is effective exactly when its trigger matches"
        );

        let found = problems(&judged.report);
        match verdict {
            Valid => assert!(judged.report.valid, "{name}: {found:?}"),
            Invalid(expected) => {
                assert!(!judged.report.valid, "{name} is invalid");
                for (pointer, needle) in *expected {
                    assert!(
                        found.iter().any(|(path, message)| path == pointer && message.contains(needle)),
                        "{name}: expected a problem at `{pointer}` naming `{needle}`, got {found:?}"
                    );
                }
                let event = expected[0].0.split('/').nth(1).expect("an event");
                assert!(
                    found.iter().all(|(path, _)| path.split('/').nth(1) == Some(event)),
                    "{name}: every problem is under `/{event}`: {found:?}"
                );
            }
        }

        let markdown = Markdown::try_from(path.as_path()).expect("document loads");
        let parsed = parse_lifecycle_config(&frontmatter_json(&markdown), &path);
        match parser {
            Accepts => assert!(parsed.is_ok(), "{name}: the parser accepts it: {parsed:?}"),
            Rejects(needle) => {
                let error = parsed.expect_err(&format!("{name}: the parser rejects it"));
                let rendered = format!("{error} {error:?}");
                assert!(rendered.contains(needle), "{name}: `{needle}` in {rendered}");
            }
            NotJudged => {}
        }
    }
}

#[test]
fn ac5_groups_name_every_fixture_once() {
    let fixtures: BTreeSet<String> =
        std::fs::read_dir(biscuit_test_harness::manifest_dir!().join("tests/fixtures/claudine-schemas"))
            .expect("fixture directory")
            .map(|entry| entry.expect("entry").path().file_stem().expect("stem").to_string_lossy().into_owned())
            .collect();
    let named: Vec<String> = AC5_GROUPS.iter().flat_map(|group| group.iter().map(|name| name.to_string())).collect();
    assert_eq!(named.len(), fixtures.len(), "no fixture is named twice");
    assert_eq!(named.into_iter().collect::<BTreeSet<_>>(), fixtures);
}

/// Depth 1, 3, and 5 are valid and depth 6 is the parser's to refuse (the
/// schema encodes no depth limit but types a misspelled verb at any depth); root
/// dictionaries, bundles as bodies, the loop gate, long forms, and a bare
/// `- stop` are valid.
#[test]
fn ac5_depths_and_stack_shapes() {
    assert_ac5(
        0,
        &shipped_bundle!(),
        include_str!("../../../schemas/claudine.yaml"),
        include_str!("../../../schemas/inline-compose.yaml"),
        &[
            ("depth1", include_str!("../fixtures/claudine-schemas/depth1.md"), CLAUDINE, Valid, Accepts),
            ("depth3", include_str!("../fixtures/claudine-schemas/depth3.md"), CLAUDINE, Valid, Accepts),
            ("depth5", include_str!("../fixtures/claudine-schemas/depth5.md"), CLAUDINE, Valid, Accepts),
            (
                "depth6",
                include_str!("../fixtures/claudine-schemas/depth6.md"),
                CLAUDINE,
                Valid,
                Rejects("conditionals nest at most 5 levels"),
            ),
            (
                // The schema reports the misspelled verb; the parser reports
                // the sixth level or the verb, whichever it meets first, and
                // both lie on this path.
                "depth6-wrong-verb",
                include_str!("../fixtures/claudine-schemas/depth6-wrong-verb.md"),
                CLAUDINE,
                Invalid(&[("/success/0/then/0/then/0/then/0/then/0/then/0/then/0", "sya")]),
                Rejects("success[0].then[0].then[0].then[0].then[0]"),
            ),
            (
                // A guarded bundle inside a level-5 conditional is level 6.
                "depth5-guarded-bundle",
                include_str!("../fixtures/claudine-schemas/depth5-guarded-bundle.md"),
                CLAUDINE,
                Valid,
                Rejects("conditionals nest at most 5 levels"),
            ),
            (
                // Four conditionals in `initialize`, `start`, `cancelled`, and
                // the loop gate, with generated long forms in the leaves: the
                // DMLS completion/hover document.
                "depth4-completion",
                include_str!("../fixtures/claudine-schemas/depth4-completion.md"),
                CLAUDINE,
                Valid,
                Accepts,
            ),
            ("root-dictionary", include_str!("../fixtures/claudine-schemas/root-dictionary.md"), CLAUDINE, Valid, Accepts),
            ("bundle-in-then", include_str!("../fixtures/claudine-schemas/bundle-in-then.md"), CLAUDINE, Valid, Accepts),
            ("loop-gate", include_str!("../fixtures/claudine-schemas/loop-gate.md"), CLAUDINE, Valid, Accepts),
            ("long-forms", include_str!("../fixtures/claudine-schemas/long-forms.md"), CLAUDINE, Valid, Accepts),
            ("bare-stop", include_str!("../fixtures/claudine-schemas/bare-stop.md"), CLAUDINE, Valid, Accepts),
        ],
    );
}

/// `depth6-wrong-verb`'s problem carries the source position of the
/// misspelled verb's item, which the CLI and DMLS report.
#[test]
fn depth6_wrong_verb_is_positioned_at_the_verb() {
    let text = include_str!("../fixtures/claudine-schemas/depth6-wrong-verb.md");
    let repo = SchemaRepo::new(&shipped_bundle!());
    let discovered = repo.discover().expect("the shipped bundle loads");
    let path = repo.write_document("depth6-wrong-verb.md", text);
    let markdown = Markdown::try_from(path.as_path()).expect("document loads").with_source(ComposeSource::File(path.clone()));
    let effective = discovered.schemas.effective_for(&markdown).expect("resolves").expect("a trigger applies");
    let yaml = text.split("---\n").nth(1).expect("the frontmatter");
    let report = effective.validate_with_positions(&frontmatter_json(&markdown), &build_position_map(yaml));
    let line = u32::try_from(yaml.lines().position(|line| line.contains("sya:")).expect("the verb's line") + 1)
        .expect("a small line");
    let problem = report
        .problems
        .iter()
        .find(|problem| problem.path == "/success/0/then/0/then/0/then/0/then/0/then/0/then/0")
        .unwrap_or_else(|| panic!("a problem at the sixth-level item: {:?}", problems(&report)));
    assert_eq!(problem.line, Some(line), "{problem:?}");
    assert!(problem.column.is_some(), "{problem:?}");
}

/// `shell` in `initialize` is a schema error at every depth and the parser's
/// alone to refuse inside `set`; it is valid in `start`.
#[test]
fn ac5_shell_placement() {
    assert_ac5(
        1,
        &shipped_bundle!(),
        include_str!("../../../schemas/claudine.yaml"),
        include_str!("../../../schemas/inline-compose.yaml"),
        &[
            (
                "shell-in-initialize-list",
                include_str!("../fixtures/claudine-schemas/shell-in-initialize-list.md"),
                CLAUDINE,
                Invalid(&[("/initialize/1", "\"shell\"")]),
                Rejects("shell"),
            ),
            (
                "shell-in-initialize-then",
                include_str!("../fixtures/claudine-schemas/shell-in-initialize-then.md"),
                CLAUDINE,
                Invalid(&[("/initialize/0/then/0", "\"shell\"")]),
                Rejects("shell"),
            ),
            (
                "shell-in-initialize-root-dictionary",
                include_str!("../fixtures/claudine-schemas/shell-in-initialize-root-dictionary.md"),
                CLAUDINE,
                Invalid(&[("/initialize", "\"shell\"")]),
                Rejects("shell"),
            ),
            (
                "shell-in-initialize-depth3-then",
                include_str!("../fixtures/claudine-schemas/shell-in-initialize-depth3-then.md"),
                CLAUDINE,
                Invalid(&[("/initialize/0/then/0/then/0/then/0", "\"shell\"")]),
                Rejects("initialize[0].then[0].then[0].then[0].shell"),
            ),
            (
                "shell-expansion-in-initialize-set",
                include_str!("../fixtures/claudine-schemas/shell-expansion-in-initialize-set.md"),
                CLAUDINE,
                Valid,
                Rejects("initialize[0].set"),
            ),
            ("shell-in-start", include_str!("../fixtures/claudine-schemas/shell-in-start.md"), CLAUDINE, Valid, Accepts),
            (
                "message-shell-bundle-with-when",
                include_str!("../fixtures/claudine-schemas/message-shell-bundle-with-when.md"),
                CLAUDINE,
                Valid,
                Accepts,
            ),
        ],
    );
}

/// Mixed conditionals, misspelled verbs and parameters, and long forms in
/// bundles are schema errors naming the offending key.
#[test]
fn ac5_malformed_actions() {
    assert_ac5(
        2,
        &shipped_bundle!(),
        include_str!("../../../schemas/claudine.yaml"),
        include_str!("../../../schemas/inline-compose.yaml"),
        &[
            (
                "when-with-then-and-sibling",
                include_str!("../fixtures/claudine-schemas/when-with-then-and-sibling.md"),
                CLAUDINE,
                Invalid(&[("/start/0", "\"warn\"")]),
                Rejects("warn"),
            ),
            (
                "misspelled-parameter",
                include_str!("../fixtures/claudine-schemas/misspelled-parameter.md"),
                CLAUDINE,
                Invalid(&[("/start/0/say", "txet")]),
                Rejects("start[0].say"),
            ),
            (
                "misspelled-verb",
                include_str!("../fixtures/claudine-schemas/misspelled-verb.md"),
                CLAUDINE,
                Invalid(&[("/start/0", "inf0")]),
                Rejects("inf0"),
            ),
            (
                "misspelled-verb-in-then",
                include_str!("../fixtures/claudine-schemas/misspelled-verb-in-then.md"),
                CLAUDINE,
                Invalid(&[("/start/0/then/0", "inf0")]),
                Rejects("inf0"),
            ),
            (
                "long-form-in-bundle",
                include_str!("../fixtures/claudine-schemas/long-form-in-bundle.md"),
                CLAUDINE,
                Invalid(&[("/success/0/say", "text")]),
                Rejects("long form"),
            ),
            (
                "long-form-in-root-dictionary",
                include_str!("../fixtures/claudine-schemas/long-form-in-root-dictionary.md"),
                CLAUDINE,
                Invalid(&[("/success/say", "text")]),
                Rejects("long form"),
            ),
        ],
    );
}

/// A single-action dictionary takes its verb's long form at a stack root, in
/// either branch, beside `when` in a list item, and as a loop's one loose
/// verb, as the parser does; several loose verbs beside a long form, and a
/// conditional written as a stack's root dictionary, are refused by both.
#[test]
fn ac5_single_action_dictionaries() {
    assert_ac5(
        5,
        &shipped_bundle!(),
        include_str!("../../../schemas/claudine.yaml"),
        include_str!("../../../schemas/inline-compose.yaml"),
        &[
            (
                "single-action-root-dictionary",
                include_str!("../fixtures/claudine-schemas/single-action-root-dictionary.md"),
                CLAUDINE,
                Valid,
                Accepts,
            ),
            (
                "single-action-branch-dictionaries",
                include_str!("../fixtures/claudine-schemas/single-action-branch-dictionaries.md"),
                CLAUDINE,
                Valid,
                Accepts,
            ),
            (
                "single-action-loop-loose",
                include_str!("../fixtures/claudine-schemas/single-action-loop-loose.md"),
                CLAUDINE,
                Valid,
                Accepts,
            ),
            (
                "long-forms-in-loop-bundle",
                include_str!("../fixtures/claudine-schemas/long-forms-in-loop-bundle.md"),
                CLAUDINE,
                Invalid(&[("/loop/ensure_file", "file")]),
                Rejects("long form"),
            ),
            (
                "conditional-root-dictionary",
                include_str!("../fixtures/claudine-schemas/conditional-root-dictionary.md"),
                CLAUDINE,
                Invalid(&[("/start", "anyOf")]),
                Rejects("never carries `when`"),
            ),
        ],
    );
}

/// A generated verb's value is typed by its catalog signature: convertible
/// text and deferred templates pass, and a deferred operand never hides a
/// mistake beside it (an unknown parameter, an extra argument, an invalid
/// literal sibling). The verb's runtime registration decides its literal
/// operands where the types alone cannot (a numeric recovery fallback, a
/// refused source type). The parser's own operand rules are judged
/// separately.
#[test]
fn ac5_generated_operands() {
    assert_ac5(
        4,
        &shipped_bundle!(),
        include_str!("../../../schemas/claudine.yaml"),
        include_str!("../../../schemas/inline-compose.yaml"),
        &[
            (
                "generated-verb-typed-value",
                include_str!("../fixtures/claudine-schemas/generated-verb-typed-value.md"),
                CLAUDINE,
                Valid,
                Accepts,
            ),
            (
                "generated-verb-wrong-type",
                include_str!("../fixtures/claudine-schemas/generated-verb-wrong-type.md"),
                CLAUDINE,
                Invalid(&[("/start/0/pr_list", "three")]),
                Accepts,
            ),
            (
                "generated-verb-convertible-text",
                include_str!("../fixtures/claudine-schemas/generated-verb-convertible-text.md"),
                CLAUDINE,
                Valid,
                Accepts,
            ),
            (
                "generated-verb-deferred",
                include_str!("../fixtures/claudine-schemas/generated-verb-deferred.md"),
                CLAUDINE,
                Valid,
                Accepts,
            ),
            (
                "generated-verb-deferred-beside-unknown-parameter",
                include_str!("../fixtures/claudine-schemas/generated-verb-deferred-beside-unknown-parameter.md"),
                CLAUDINE,
                Invalid(&[("/start/0/pr_list", "limit")]),
                Rejects("limit"),
            ),
            (
                "generated-verb-deferred-beside-extra-argument",
                include_str!("../fixtures/claudine-schemas/generated-verb-deferred-beside-extra-argument.md"),
                CLAUDINE,
                Invalid(&[("/start/0/min/2", "does not allow 3")]),
                Rejects("min"),
            ),
            (
                "generated-verb-deferred-beside-bad-sibling",
                include_str!("../fixtures/claudine-schemas/generated-verb-deferred-beside-bad-sibling.md"),
                CLAUDINE,
                Invalid(&[("/start/0/min/0", "three")]),
                Accepts,
            ),
            (
                "generated-verb-malformed-template",
                include_str!("../fixtures/claudine-schemas/generated-verb-malformed-template.md"),
                CLAUDINE,
                Invalid(&[("/start/0/pr_list", "{{ 1 + }}")]),
                Rejects("pr_list"),
            ),
            (
                "generated-verb-malformed-string-template",
                include_str!("../fixtures/claudine-schemas/generated-verb-malformed-string-template.md"),
                CLAUDINE,
                Invalid(&[
                    ("/start/0/ensure_file", "malformed template `{{ 1 + }}`"),
                    ("/start/1/trim/text", "malformed template `hello {{ 1 + }}`"),
                    ("/start/2/ensure_file/1", "malformed template `out/{{ 1 + }}.md`"),
                ]),
                Rejects("ensure_file"),
            ),
            (
                // The same syntax problem wherever a hand-written verb's
                // value is read as a template; the `$( … )` `set` value is
                // a command, not one.
                "handwritten-action-malformed-template",
                include_str!("../fixtures/claudine-schemas/handwritten-action-malformed-template.md"),
                CLAUDINE,
                Invalid(&[
                    ("/start/0/say", "malformed template `{{ 1 + }}`"),
                    ("/start/1/info/text", "malformed template `hello {{ 1 + }}`"),
                    ("/start/2/set/round", "malformed template `{{ fm.round + }}`"),
                    ("/start/3/shell", "malformed template `echo {{ 2 * }}`"),
                    ("/start/4/then/0/error/reason", "malformed template `failed {{ 3 - }}`"),
                    ("/start/4/else/0/proxy/with/note", "malformed template `next {{ 4 / }}`"),
                ]),
                Rejects("say"),
            ),
            (
                "optional-nonnullable-null",
                include_str!("../fixtures/claudine-schemas/optional-nonnullable-null.md"),
                CLAUDINE,
                Invalid(&[("/start/0/ping/1", "null"), ("/start/1/ping/timeout", "null")]),
                Rejects("{{ null }}"),
            ),
            (
                "ensure-file-overloads",
                include_str!("../fixtures/claudine-schemas/ensure-file-overloads.md"),
                CLAUDINE,
                Valid,
                Accepts,
            ),
            (
                "variadic-and-or",
                include_str!("../fixtures/claudine-schemas/variadic-and-or.md"),
                CLAUDINE,
                Valid,
                Accepts,
            ),
            (
                // The runtime registration decides: a fallback recovers a
                // subject the projected type rejects.
                "generated-verb-numeric-recovery",
                include_str!("../fixtures/claudine-schemas/generated-verb-numeric-recovery.md"),
                CLAUDINE,
                Valid,
                Accepts,
            ),
            (
                // `length` refuses a boolean its projected `string` converts,
                // at every depth.
                "generated-verb-excluded-source",
                include_str!("../fixtures/claudine-schemas/generated-verb-excluded-source.md"),
                CLAUDINE,
                Invalid(&[
                    ("/start/0/length/0", "does not accept a boolean"),
                    ("/start/1/then/0/length/val", "does not accept a boolean"),
                ]),
                Accepts,
            ),
        ],
    );
}

/// Malformed identifying values select `claudine.yaml`, which rejects them;
/// missing, null, and unrelated values select nothing; a malformed
/// non-null `sequence` defeats the inline trigger; application data and
/// every linking key are kept.
#[test]
fn ac5_trigger_selection() {
    assert_ac5(
        3,
        &shipped_bundle!(),
        include_str!("../../../schemas/claudine.yaml"),
        include_str!("../../../schemas/inline-compose.yaml"),
        &[
            (
                "start-number",
                include_str!("../fixtures/claudine-schemas/start-number.md"),
                CLAUDINE,
                Invalid(&[("/start", "42")]),
                Rejects("start"),
            ),
            (
                "agent-empty-list",
                include_str!("../fixtures/claudine-schemas/agent-empty-list.md"),
                CLAUDINE,
                Invalid(&[("/agent", "[]")]),
                NotJudged,
            ),
            (
                "loop-number",
                include_str!("../fixtures/claudine-schemas/loop-number.md"),
                CLAUDINE,
                Invalid(&[("/loop", "42")]),
                Rejects("loop"),
            ),
            ("empty-frontmatter", include_str!("../fixtures/claudine-schemas/empty-frontmatter.md"), NEITHER, Valid, NotJudged),
            ("unrelated-ticket", include_str!("../fixtures/claudine-schemas/unrelated-ticket.md"), NEITHER, Valid, NotJudged),
            ("null-identifiers", include_str!("../fixtures/claudine-schemas/null-identifiers.md"), NEITHER, Valid, Accepts),
            ("controls-beside-ticket", include_str!("../fixtures/claudine-schemas/controls-beside-ticket.md"), BOTH, Valid, Accepts),
            (
                "prompt-with-malformed-sequence",
                include_str!("../fixtures/claudine-schemas/prompt-with-malformed-sequence.md"),
                CLAUDINE,
                Invalid(&[("/sequence", "tasks")]),
                NotJudged,
            ),
            ("linked-prompt", include_str!("../fixtures/claudine-schemas/linked-prompt.md"), BOTH, Valid, NotJudged),
        ],
    );
}

/// A linked prompt's eleven linking keys are all declared, so DMLS raises
/// no unknown-key warning, and the document root stays open for
/// application data.
#[test]
fn linked_prompt_declares_every_linking_key_and_keeps_the_root_open() {
    let repo = SchemaRepo::new(&shipped_bundle!());
    let discovered = repo.discover().expect("the shipped bundle loads");
    let path = repo.write_document("linked.md", include_str!("../fixtures/claudine-schemas/linked-prompt.md"));
    let judged = discovered
        .judge(
            &path,
            include_str!("../../../schemas/claudine.yaml"),
            include_str!("../../../schemas/inline-compose.yaml"),
        )
        .expect("judged");
    for key in [
        "name", "allowed-tools", "tools", "skills", "license", "compatibility", "metadata",
        "user-invocable", "disable-model-invocation", "argument-hint", "max_turns",
    ] {
        assert!(judged.properties.contains(key), "linking key `{key}` is declared");
    }
    let path = repo.write_document(
        "ticket.md",
        include_str!("../fixtures/claudine-schemas/controls-beside-ticket.md"),
    );
    let markdown = Markdown::try_from(path.as_path())
        .expect("document loads")
        .with_source(ComposeSource::File(path.clone()));
    let effective = discovered.schemas.effective_for(&markdown).expect("effective").expect("schema");
    assert_ne!(effective.json_schema["additionalProperties"], json!(false), "the document root stays open");
    assert!(!effective.json_schema["properties"].as_object().expect("properties").contains_key("ticket"));
    assert!(effective.validate(&frontmatter_json(&markdown)).valid, "`ticket: 123` is accepted beside the controls");
}

#[test]
fn shared_payload_properties_have_identical_types() {
    let repo = SchemaRepo::new(&shipped_bundle!());
    let discovered = repo.discover().expect("the shipped bundle loads");
    let lowered = |name: &str, frontmatter: &str| {
        let path = repo.write_document(name, frontmatter);
        let markdown = Markdown::try_from(path.as_path())
            .expect("document loads")
            .with_source(ComposeSource::File(path.clone()));
        let effective = discovered.schemas.effective_for(&markdown).expect("effective").expect("schema");
        ["prompt", "hash", "last_updated"]
            .iter()
            .map(|key| (key.to_string(), effective.json_schema["properties"][*key].clone()))
            .collect::<BTreeMap<_, _>>()
    };
    // Only `claudine.yaml` matches the first document. `prompt` selects both
    // triggers, and `inline-compose.yaml` is applied after `claudine.yaml`,
    // so the second shows the inline declarations.
    let claudine_only = lowered("claudine-only.md", "---\nstart: [stop]\n---\n");
    let inline_last = lowered("inline.md", "---\nprompt: hi\n---\n");
    assert!(claudine_only.values().all(|fragment| !fragment.is_null()));
    assert_eq!(claudine_only, inline_last);

    let inline = include_str!("../../../schemas/inline-compose.yaml");
    let claudine = include_str!("../../../schemas/claudine.yaml");
    let payload_lines = |text: &'static str| -> BTreeSet<&'static str> {
        text.split("\n$schema:\n").nth(1).expect("a payload").lines().collect()
    };
    let shared: BTreeSet<&str> = payload_lines(inline);
    assert!(
        shared.is_subset(&payload_lines(claudine)),
        "every inline-compose declaration is spelled exactly as in claudine.yaml"
    );
}

#[test]
fn linking_types_accept_every_reader_form() {
    let repo = SchemaRepo::new(&shipped_bundle!());
    let discovered = repo.discover().expect("the shipped bundle loads");
    let claudine = include_str!("../../../schemas/claudine.yaml");
    let inline = include_str!("../../../schemas/inline-compose.yaml");
    for (frontmatter, valid) in [
        ("allowed-tools: Read, Grep", true),
        ("allowed-tools: [Read, Grep]", true),
        ("tools: [Read]", true),
        ("tools: { Read: true }", true),
        ("skills: review", true),
        ("compatibility: { claude: \">=2\" }", true),
        ("argument-hint: \"<file>\"", true),
        ("argument-hint: [msg]", true),
        ("metadata: { anything: [1, 2], nested: { ok: true } }", true),
        ("max_turns: 4", true),
        ("user-invocable: [1]", false),
        ("max_turns: [4]", false),
        ("name: { first: x }", false),
    ] {
        let path = repo.write_document("linking.md", &format!("---\nprompt: hi\n{frontmatter}\n---\n"));
        let judged = discovered.judge(&path, claudine, inline).expect("judged");
        assert_eq!(judged.report.valid, valid, "`{frontmatter}`: {:?}", problems(&judged.report));
    }
}

#[test]
fn sequence_container_schema_contract() {
    let files = shipped_bundle!();
    let repo = SchemaRepo::new(&files);
    let discovered = repo.discover().expect("shipped schema corpus loads");
    let trigger = include_str!("../../../schemas/claudine.yaml");
    let inline = include_str!("../../../schemas/inline-compose.yaml");
    for (name, yaml, valid) in [
        ("empty", "cancelled: []", true),
        ("communication", "cancelled: [{stderr: cancelled}]", true),
        ("shell", "cancelled: [{shell: 'echo cancelled'}]", true),
        ("stop", "cancelled: [stop]", true),
        ("set", "cancelled: [{set: {finished: true}}]", true),
        ("conditional", "cancelled: [{when: true, then: [{stderr: cancelled}], else: [stop]}]", true),
        ("retry", "cancelled: [retry]", false),
        ("bundle-retry", "cancelled: {retry: null}", false),
        ("number", "cancelled: 7", false),
        ("task-fields", "cancelled: []\nsequence: [{name: review, prompt: './step.md', operation: review, flow: audit, topic: parsing, with: {x: 1}}]", true),
        ("nested", "cancelled: []\nsequence: [{name: child, sequence: [], cancelled: []}, {group: {tasks: [{name: leg, shell: 'echo done'}], fail_fast: false, cancelled: []}}]", true),
    ] {
        let path = repo.write_document(&format!("container-{name}.md"), &format!("---\n{yaml}\n---\n"));
        let judged = discovered.judge(&path, trigger, inline).unwrap();
        assert!(judged.claudine, "{name}: selects shipped trigger");
        assert_eq!(judged.report.valid, valid, "{name}: {:?}", judged.report);
        for retired in ["state", "previous", "next", "outputs", "sequence_id"] {
            assert!(!judged.properties.contains(retired), "retired catalog property {retired}");
        }
        // Root `fail_fast` is declared only by sequence.yaml, as `null`, so
        // a value is rejected by name (see `sequence_schema.rs`).
        assert_eq!(judged.properties.contains("fail_fast"), yaml.contains("sequence:"), "{name}: fail_fast");
    }
}

/// A task guard is typed by shape only: non-empty text. A container's loop
/// (on a group object, or beside an inline nested sequence) uses the shared
/// `loop` type at every depth. The task and loop parsers judge the rest.
#[test]
fn task_when_and_container_loop_schema_shapes() {
    let files = shipped_bundle!();
    let repo = SchemaRepo::new(&files);
    let discovered = repo.discover().expect("shipped schema corpus loads");
    let trigger = include_str!("../../../schemas/claudine.yaml");
    let inline = include_str!("../../../schemas/inline-compose.yaml");
    for (name, yaml, valid) in [
        ("task-when", "cancelled: []\nsequence: [{name: a, shell: x, when: 'ready && n > 1'}]", true),
        // A scalar such as `when: 5` passes as text here; the task parser
        // rejects every non-string guard.
        ("task-when-list", "cancelled: []\nsequence: [{name: a, shell: x, when: [ready]}]", false),
        ("task-when-empty", "cancelled: []\nsequence: [{name: a, shell: x, when: ''}]", false),
        ("leg-when-map", "cancelled: []\nsequence: [{group: {tasks: [{name: leg, shell: x, when: {a: 1}}]}}]", false),
        ("group-loop", "cancelled: []\nsequence: [{group: {tasks: [{name: leg, shell: x}], loop: {until: 'true'}}}]", true),
        ("group-loop-scalar", "cancelled: []\nsequence: [{group: {tasks: [{name: leg, shell: x}], loop: 5}}]", false),
        ("nested-loop-list", "cancelled: []\nsequence: [{name: child, sequence: [], loop: [until]}]", false),
        ("group-loop-max-zero", "cancelled: []\nsequence: [{group: {tasks: [{name: leg, shell: x}], loop: {until: 'true', max: 0}}}]", false),
        ("nested-loop-unknown-key", "cancelled: []\nsequence: [{name: child, sequence: [], loop: {until: 'true', maxx: 2}}]", false),
        ("deeper-loop-max-zero", "cancelled: []\nsequence: [{name: a, sequence: [{name: b, sequence: [], loop: {until: 'true', max: 0}}]}]", false),
        ("deeper-group-loop-gate", "cancelled: []\nsequence: [{name: a, sequence: [{group: {tasks: [{name: leg, shell: x}], loop: {until: 'true', gate: [{inf0: hi}]}}}]}]", false),
        ("root-loop", "loop: {until: 'true'}\nsequence: [{name: a, shell: x, when: ready}]", true),
    ] {
        let path = repo.write_document(&format!("shape-{name}.md"), &format!("---\n{yaml}\n---\n"));
        let judged = discovered.judge(&path, trigger, inline).unwrap();
        assert!(judged.claudine, "{name}: selects shipped trigger");
        assert_eq!(judged.report.valid, valid, "{name}: {:?}", judged.report);
    }
}

// ── Forbidden cancellation actions at every container boundary ─────────────

/// One well-formed item per flow directive that section 4 excludes from
/// `cancelled`. Each is accepted by a `start` stack, so a rejection under
/// `cancelled` is the restricted action set, not a malformed item.
const FORBIDDEN_IN_CANCELLED: [(&str, &str); 8] = [
    ("retry", "{ retry: 2 }"),
    ("resume", "{ resume: continue }"),
    ("proxy", "{ proxy: '@prompts/next.md' }"),
    ("prep", "{ prep: '@prompts/next.md' }"),
    ("skip", "{ skip: null }"),
    ("error", "{ error: stopped }"),
    ("break", "{ break: done }"),
    ("defer", "{ defer: 5m }"),
];

/// One cancellation site: the frontmatter, with `ITEMS` standing for the
/// stack's items, and the instance path of its first item.
struct CancelledSite {
    name: &'static str,
    frontmatter: &'static str,
    pointer: &'static str,
}

/// Judges `site` against the discovered `bundle`: each `forbidden` item is
/// rejected with a problem at the site's pointer, and `[stop]` and `[]` are
/// valid.
fn assert_cancelled_restricted_at(
    bundle: &[BundleFile],
    claudine: &str,
    inline: &str,
    site: CancelledSite,
    forbidden: &[(&str, &str)],
) {
    let repo = SchemaRepo::new(bundle);
    let discovered = repo.discover().expect("the shipped bundle loads");
    let judge = |name: &str, items: &str| {
        let text = format!("---\n{}\n---\n", site.frontmatter.replace("ITEMS", items));
        let path = repo.write_document(&format!("{name}.md"), &text);
        discovered.judge(&path, claudine, inline).expect("judged").report
    };
    let CancelledSite { name: site_name, pointer, .. } = site;
    for (verb, item) in forbidden {
        let report = judge(&format!("cancelled-{verb}"), &format!("[{item}]"));
        let found = problems(&report);
        assert!(!report.valid, "{site_name}: `{verb}` is rejected in `cancelled`");
        assert!(
            found.iter().any(|(path, _)| path == pointer),
            "{site_name}: `{verb}` is reported at `{pointer}`: {found:?}"
        );
    }
    for (control, items) in [("stop", "[stop]"), ("empty", "[]")] {
        let report = judge(&format!("control-{control}"), items);
        assert!(report.valid, "{site_name}: `cancelled: {items}` is valid: {:?}", problems(&report));
    }
}

#[test]
fn forbidden_cancelled_items_are_well_formed_start_actions() {
    let repo = SchemaRepo::new(&shipped_bundle!());
    let discovered = repo.discover().expect("the shipped bundle loads");
    for (verb, item) in FORBIDDEN_IN_CANCELLED {
        let path = repo.write_document(&format!("start-{verb}.md"), &format!("---\nstart: [{item}]\n---\n"));
        let report = discovered
            .judge(
                &path,
                include_str!("../../../schemas/claudine.yaml"),
                include_str!("../../../schemas/inline-compose.yaml"),
            )
            .expect("judged")
            .report;
        assert!(report.valid, "`{verb}` is a valid `start` action: {:?}", problems(&report));
    }
}

/// The schema and the lifecycle parser agree on every `break` and `prep`
/// shape in a `start` stack: what one accepts, the other accepts. The parser
/// reads the frontmatter the validated document holds, after the conversion
/// engine (`code: '2'` is `2` to it), exactly as Claudine's preparation
/// does. An explicit
/// `null` on an optional key reads as absent to the schema, so `prep: null`
/// is the parser's alone to reject (its unit tests cover it).
#[test]
fn break_and_prep_shapes_agree_between_schema_and_parser() {
    const SHAPES: &[(&str, bool)] = &[
        ("[break]", true),
        ("[{break: null}]", true),
        ("[{break: []}]", true),
        ("[{break: done}]", true),
        ("[{break: ''}]", true),
        ("[{break: {reason: done, code: 0}}]", true),
        ("[{break: {code: 255, no_error: true}}]", true),
        ("[{break: {code: 256}}]", false),
        ("[{break: {code: -1}}]", false),
        ("[{break: {code: 1.5}}]", false),
        ("[{break: {code: '2'}}]", true),
        ("[{break: {code: 'two'}}]", false),
        ("[{break: {reason: [a]}}]", false),
        ("[{break: {exit: 2}}]", false),
        ("[{break: 5}]", true),
        ("[{prep: '@a.md'}, {info: after}]", true),
        ("[{prep: {target: '@a.md', with: {}}}]", true),
        ("[{prep: {target: '@a.md', with: {stage: all}, no_error: true}}]", true),
        ("[prep]", false),
        ("[{prep: ''}]", false),
        ("[{prep: {with: {}}}]", false),
        ("[{prep: {target: ''}}]", false),
        ("[{prep: {target: '@a.md', with: [a]}}]", false),
        ("[{prep: {target: '@a.md', args: 1}}]", false),
    ];
    let repo = SchemaRepo::new(&shipped_bundle!());
    let discovered = repo.discover().expect("the shipped bundle loads");
    for (index, (items, valid)) in SHAPES.iter().enumerate() {
        let path = repo.write_document(&format!("directive-{index}.md"), &format!("---\nstart: {items}\n---\n"));
        let judged = discovered
            .judge(
                &path,
                include_str!("../../../schemas/claudine.yaml"),
                include_str!("../../../schemas/inline-compose.yaml"),
            )
            .expect("judged");
        let report = &judged.report;
        assert_eq!(report.valid, *valid, "schema verdict for `start: {items}`: {:?}", problems(report));
        let parsed = parse_lifecycle_config(&judged.held, &path);
        assert_eq!(parsed.is_ok(), *valid, "parser verdict for `start: {items}`: {parsed:?}");
    }
}

#[test]
fn document_cancelled_rejects_forbidden_actions() {
    assert_cancelled_restricted_at(
        &shipped_bundle!(),
        include_str!("../../../schemas/claudine.yaml"),
        include_str!("../../../schemas/inline-compose.yaml"),
        CancelledSite {
            name: "document",
            frontmatter: "cancelled: ITEMS",
            pointer: "/cancelled/0",
        },
        &FORBIDDEN_IN_CANCELLED,
    );
}

#[test]
fn task_cancelled_rejects_forbidden_actions() {
    assert_cancelled_restricted_at(
        &shipped_bundle!(),
        include_str!("../../../schemas/claudine.yaml"),
        include_str!("../../../schemas/inline-compose.yaml"),
        CancelledSite {
            name: "task",
            frontmatter: "agent: claude\nsequence:\n  - name: child\n    sequence: []\n    cancelled: ITEMS",
            pointer: "/sequence/0/cancelled/0",
        },
        &FORBIDDEN_IN_CANCELLED,
    );
}

#[test]
fn group_cancelled_rejects_forbidden_actions() {
    assert_cancelled_restricted_at(
        &shipped_bundle!(),
        include_str!("../../../schemas/claudine.yaml"),
        include_str!("../../../schemas/inline-compose.yaml"),
        CancelledSite {
            name: "group",
            frontmatter: "agent: claude\nsequence:\n  - group:\n      tasks: [{ name: leg, shell: 'echo done' }]\n      cancelled: ITEMS",
            pointer: "/sequence/0/group/cancelled/0",
        },
        &FORBIDDEN_IN_CANCELLED,
    );
}

#[test]
fn nested_sequence_child_cancelled_rejects_forbidden_actions() {
    assert_cancelled_restricted_at(
        &shipped_bundle!(),
        include_str!("../../../schemas/claudine.yaml"),
        include_str!("../../../schemas/inline-compose.yaml"),
        CancelledSite {
            name: "nested sequence child",
            frontmatter: "agent: claude\nsequence:\n  - name: outer\n    sequence:\n      - name: inner\n        sequence: []\n        cancelled: ITEMS",
            pointer: "/sequence/0/sequence/0/cancelled/0",
        },
        &FORBIDDEN_IN_CANCELLED,
    );
}

#[test]
fn group_leg_sequence_cancelled_rejects_forbidden_actions() {
    assert_cancelled_restricted_at(
        &shipped_bundle!(),
        include_str!("../../../schemas/claudine.yaml"),
        include_str!("../../../schemas/inline-compose.yaml"),
        CancelledSite {
            name: "group leg",
            frontmatter: "agent: claude\nsequence:\n  - group:\n      tasks:\n        - name: leg\n          sequence: []\n          cancelled: ITEMS",
            pointer: "/sequence/0/group/tasks/0/cancelled/0",
        },
        &FORBIDDEN_IN_CANCELLED,
    );
}

/// A task or group imported as an authored parameter type gets the same
/// restriction as one reached through a document's `sequence:`. Each document
/// compiles its own `$schema`, so one verb stands for the eight already judged
/// through the trigger at the same named types.
#[test]
fn imported_task_cancelled_rejects_forbidden_actions() {
    assert_cancelled_restricted_at(
        &shipped_bundle!(),
        include_str!("../../../schemas/claudine.yaml"),
        include_str!("../../../schemas/inline-compose.yaml"),
        CancelledSite {
            name: "imported task",
            frontmatter: "$schema:\n  nested: task@../schemas/sequence-types.yaml\nnested:\n  name: child\n  sequence: []\n  cancelled: ITEMS",
            pointer: "/nested/cancelled/0",
        },
        &FORBIDDEN_IN_CANCELLED[..1],
    );
}

#[test]
fn imported_group_cancelled_rejects_forbidden_actions() {
    assert_cancelled_restricted_at(
        &shipped_bundle!(),
        include_str!("../../../schemas/claudine.yaml"),
        include_str!("../../../schemas/inline-compose.yaml"),
        CancelledSite {
            name: "imported group",
            frontmatter: "$schema:\n  nested: group@../schemas/sequence-types.yaml\nnested:\n  tasks: [{ name: leg, shell: 'echo done' }]\n  cancelled: ITEMS",
            pointer: "/nested/cancelled/0",
        },
        &FORBIDDEN_IN_CANCELLED[..1],
    );
}

#[test]
fn imported_group_leg_cancelled_rejects_forbidden_actions() {
    assert_cancelled_restricted_at(
        &shipped_bundle!(),
        include_str!("../../../schemas/claudine.yaml"),
        include_str!("../../../schemas/inline-compose.yaml"),
        CancelledSite {
            name: "imported group leg",
            frontmatter: "$schema:\n  nested: group@../schemas/sequence-types.yaml\nnested:\n  tasks:\n    - name: leg\n      sequence: []\n      cancelled: ITEMS",
            pointer: "/nested/tasks/0/cancelled/0",
        },
        &FORBIDDEN_IN_CANCELLED[..1],
    );
}

// ── Cancellation stacks at every conditional depth ─────────────────────────

/// The deepest conditional level Claudine's lifecycle parser accepts
/// (`MAX_CONDITIONAL_DEPTH` in `darkmatter/lib/src/lifecycle/parse.rs`).
const PARSER_MAX_CONDITIONAL_DEPTH: usize = 5;

/// A cancellation stack's site: the frontmatter, with `ITEMS` standing for
/// the stack, and the instance path of the stack itself.
struct StackSite {
    name: &'static str,
    frontmatter: &'static str,
    stack: &'static str,
}

const DOCUMENT_CANCELLED: StackSite = StackSite {
    name: "document cancelled",
    frontmatter: "cancelled: ITEMS",
    stack: "/cancelled",
};
const TASK_CANCELLED: StackSite = StackSite {
    name: "task cancelled",
    frontmatter: "agent: claude\nsequence:\n  - name: child\n    sequence: []\n    cancelled: ITEMS",
    stack: "/sequence/0/cancelled",
};
const GROUP_CANCELLED: StackSite = StackSite {
    name: "group cancelled",
    frontmatter: "agent: claude\nsequence:\n  - group:\n      tasks: [{ name: leg, shell: 'echo done' }]\n      cancelled: ITEMS",
    stack: "/sequence/0/group/cancelled",
};
const TASK_2_CANCELLED: StackSite = StackSite {
    name: "nested task cancelled",
    frontmatter: "agent: claude\nsequence:\n  - name: outer\n    sequence:\n      - name: inner\n        sequence: []\n        cancelled: ITEMS",
    stack: "/sequence/0/sequence/0/cancelled",
};
const GROUP_2_CANCELLED: StackSite = StackSite {
    name: "nested group cancelled",
    frontmatter: "agent: claude\nsequence:\n  - group:\n      tasks:\n        - name: leg\n          group:\n            tasks: [{ name: inner, shell: 'echo done' }]\n            cancelled: ITEMS",
    stack: "/sequence/0/group/tasks/0/group/cancelled",
};

/// The items placed in `branch` (`then` or `else`) of a conditional `level`
/// levels deep, with each enclosing conditional descending through `then`.
/// Returns the stack and the instance path of `leaf` below the stack.
fn in_conditional(level: usize, branch: &str, leaf: &str) -> (String, String) {
    let mut stack = if branch == "then" {
        format!("[{{ when: \"ctx.l{level}\", then: [{leaf}] }}]")
    } else {
        format!("[{{ when: \"ctx.l{level}\", then: [{{ info: hi }}], else: [{leaf}] }}]")
    };
    for outer in (1..level).rev() {
        stack = format!("[{{ when: \"ctx.l{outer}\", then: {stack} }}]");
    }
    (stack, format!("{}/0/{branch}/0", "/0/then".repeat(level - 1)))
}

/// Judges `site` with each `forbidden` item in the `then` and the `else` of a
/// conditional `level` levels deep: each is rejected at its exact instance
/// path, and `stop` in the same places, `[stop]`, and `[]` are valid.
fn assert_cancelled_restricted_in_conditionals(site: &StackSite, level: usize, forbidden: &[(&str, &str)]) {
    let repo = SchemaRepo::new(&shipped_bundle!());
    let discovered = repo.discover().expect("the shipped bundle loads");
    let judge = |name: &str, items: &str| {
        let text = format!("---\n{}\n---\n", site.frontmatter.replace("ITEMS", items));
        let path = repo.write_document(&format!("{name}.md"), &text);
        discovered
            .judge(
                &path,
                include_str!("../../../schemas/claudine.yaml"),
                include_str!("../../../schemas/inline-compose.yaml"),
            )
            .expect("judged")
            .report
    };
    let StackSite { name: site_name, stack: base, .. } = site;
    for branch in ["then", "else"] {
        for (verb, item) in forbidden {
            let (items, below) = in_conditional(level, branch, item);
            let pointer = format!("{base}{below}");
            let found = problems(&judge(&format!("{verb}-{branch}"), &items));
            assert!(
                found.iter().any(|(path, _)| *path == pointer),
                "{site_name}: `{verb}` in a level-{level} `{branch}` is rejected at `{pointer}`: {found:?}"
            );
        }
        let (items, _) = in_conditional(level, branch, "stop");
        let report = judge(&format!("stop-{branch}"), &items);
        assert!(report.valid, "{site_name}: `stop` in a level-{level} `{branch}`: {:?}", problems(&report));
    }
    for (control, items) in [("stop", "[stop]"), ("empty", "[]")] {
        let report = judge(&format!("control-{control}"), items);
        assert!(report.valid, "{site_name}: `{items}` is valid: {:?}", problems(&report));
    }
}

/// One test per site and conditional level, each judging both branches.
macro_rules! cancelled_conditional_tests {
    ($($test:ident => ($site:expr, $level:expr, $forbidden:expr);)*) => {
        $(
            #[test]
            fn $test() {
                assert_cancelled_restricted_in_conditionals(&$site, $level, $forbidden);
            }
        )*
    };
}

cancelled_conditional_tests! {
    document_cancelled_rejects_forbidden_actions_in_level_1_conditionals =>
        (DOCUMENT_CANCELLED, 1, &FORBIDDEN_IN_CANCELLED);
    document_cancelled_rejects_forbidden_actions_in_level_2_conditionals =>
        (DOCUMENT_CANCELLED, 2, &FORBIDDEN_IN_CANCELLED);
    document_cancelled_rejects_forbidden_actions_in_level_3_conditionals =>
        (DOCUMENT_CANCELLED, 3, &FORBIDDEN_IN_CANCELLED);
    document_cancelled_rejects_forbidden_actions_in_level_4_conditionals =>
        (DOCUMENT_CANCELLED, 4, &FORBIDDEN_IN_CANCELLED);
    document_cancelled_rejects_forbidden_actions_in_level_5_conditionals =>
        (DOCUMENT_CANCELLED, PARSER_MAX_CONDITIONAL_DEPTH, &FORBIDDEN_IN_CANCELLED);
    task_cancelled_rejects_forbidden_actions_in_the_deepest_conditionals =>
        (TASK_CANCELLED, PARSER_MAX_CONDITIONAL_DEPTH, &FORBIDDEN_IN_CANCELLED);
    group_cancelled_rejects_forbidden_actions_in_the_deepest_conditionals =>
        (GROUP_CANCELLED, PARSER_MAX_CONDITIONAL_DEPTH, &FORBIDDEN_IN_CANCELLED);
    nested_task_cancelled_rejects_forbidden_actions_in_the_deepest_conditionals =>
        (TASK_2_CANCELLED, PARSER_MAX_CONDITIONAL_DEPTH, &FORBIDDEN_IN_CANCELLED);
    nested_group_cancelled_rejects_forbidden_actions_in_the_deepest_conditionals =>
        (GROUP_2_CANCELLED, PARSER_MAX_CONDITIONAL_DEPTH, &FORBIDDEN_IN_CANCELLED);
}

/// The schema encodes no conditional depth limit: a conditional one level
/// deeper than the parser accepts is schema-valid on a document and on a
/// nested container, while a forbidden action there is still a schema error,
/// and the lifecycle parser alone refuses the sixth level.
#[test]
fn cancelled_conditional_depth_is_the_parsers_alone_to_limit() {
    let repo = SchemaRepo::new(&shipped_bundle!());
    let discovered = repo.discover().expect("the shipped bundle loads");
    let level = PARSER_MAX_CONDITIONAL_DEPTH + 1;
    let judge = |site: &StackSite, items: &str| {
        let text = format!("---\n{}\n---\n", site.frontmatter.replace("ITEMS", items));
        let path = repo.write_document("too-deep.md", &text);
        discovered
            .judge(
                &path,
                include_str!("../../../schemas/claudine.yaml"),
                include_str!("../../../schemas/inline-compose.yaml"),
            )
            .expect("judged")
            .report
    };
    for site in [DOCUMENT_CANCELLED, TASK_2_CANCELLED] {
        let (items, _) = in_conditional(level, "then", "stop");
        let report = judge(&site, &items);
        assert!(report.valid, "{}: a level-{level} conditional is schema-valid: {:?}", site.name, problems(&report));
        let (items, below) = in_conditional(level, "else", "{ retry: 2 }");
        let pointer = format!("{}{below}", site.stack);
        let found = problems(&judge(&site, &items));
        assert!(
            found.iter().any(|(path, _)| *path == pointer),
            "{}: `retry` in a level-{level} `else` is rejected at `{pointer}`: {found:?}",
            site.name
        );
    }
    let path = repo.write_document("too-deep.md", &format!("---\ncancelled: {}\n---\n", in_conditional(level, "then", "stop").0));
    let markdown = Markdown::try_from(path.as_path()).expect("document loads");
    let error = parse_lifecycle_config(&frontmatter_json(&markdown), &path).expect_err("the parser refuses a sixth level");
    assert!(format!("{error}").contains("conditionals nest at most 5 levels"), "{error}");
}

/// A whole cancellation stack written as one dictionary, and a dictionary
/// `then`/`else` body at the deepest level, reject the same eight verbs.
#[test]
fn cancelled_dictionaries_reject_forbidden_actions() {
    let repo = SchemaRepo::new(&shipped_bundle!());
    let discovered = repo.discover().expect("the shipped bundle loads");
    let judge = |name: &str, frontmatter: &str| {
        let path = repo.write_document(&format!("{name}.md"), &format!("---\n{frontmatter}\n---\n"));
        discovered
            .judge(
                &path,
                include_str!("../../../schemas/claudine.yaml"),
                include_str!("../../../schemas/inline-compose.yaml"),
            )
            .expect("judged")
            .report
    };
    let deepest = PARSER_MAX_CONDITIONAL_DEPTH;
    for (verb, item) in FORBIDDEN_IN_CANCELLED {
        let found = problems(&judge(&format!("root-{verb}"), &format!("cancelled: {item}")));
        assert!(found.iter().any(|(path, _)| path == "/cancelled"), "root dictionary `{verb}`: {found:?}");
        for branch in ["then", "else"] {
            let (items, below) = in_conditional(deepest, branch, "stop");
            let items = items.replace("[stop]", item);
            let pointer = format!("/cancelled{}", below.trim_end_matches("/0"));
            let found = problems(&judge(&format!("body-{verb}-{branch}"), &format!("cancelled: {items}")));
            assert!(
                found.iter().any(|(path, _)| *path == pointer),
                "a level-{deepest} `{branch}` dictionary with `{verb}` is rejected at `{pointer}`: {found:?}"
            );
        }
    }
    let report = judge("root-control", "cancelled: { warn: cancelled, stop: null }");
    assert!(report.valid, "a dictionary of permitted verbs: {:?}", problems(&report));
}

/// Containers recurse, so a third-level container's `cancelled` stack is
/// typed like every other: a forbidden action there is a schema error at its
/// path.
#[test]
fn grandchild_cancelled_is_typed() {
    let repo = SchemaRepo::new(&shipped_bundle!());
    let discovered = repo.discover().expect("the shipped bundle loads");
    let path = repo.write_document(
        "grandchild.md",
        "---\nagent: claude\nsequence:\n  - name: a\n    sequence:\n      - name: b\n        sequence:\n          - name: c\n            sequence: []\n            cancelled: [{ retry: 2 }]\n---\n",
    );
    let report = discovered
        .judge(
            &path,
            include_str!("../../../schemas/claudine.yaml"),
            include_str!("../../../schemas/inline-compose.yaml"),
        )
        .expect("judged")
        .report;
    let found = problems(&report);
    assert!(
        found.iter().any(|(path, _)| path == "/sequence/0/sequence/0/sequence/0/cancelled/0"),
        "a grandchild's forbidden `cancelled` action is rejected at its path: {found:?}"
    );
}

// ── Single-action dictionaries (review 1, R1-F2) ───────────────────────────

/// The schema's and the lifecycle parser's verdicts on one frontmatter, with
/// the schema's problems.
struct SingleActionJudge {
    repo: SchemaRepo,
    discovered: Discovered,
}

impl SingleActionJudge {
    fn new() -> Self {
        let repo = SchemaRepo::new(&shipped_bundle!());
        let discovered = repo.discover().expect("the shipped bundle loads");
        Self { repo, discovered }
    }

    fn verdicts(&self, frontmatter: &str) -> (bool, Vec<(String, String)>, Result<(), String>) {
        let path = self.repo.write_document("single.md", &format!("---\n{frontmatter}\n---\n"));
        let judged = self
            .discovered
            .judge(&path, include_str!("../../../schemas/claudine.yaml"), include_str!("../../../schemas/inline-compose.yaml"))
            .expect("judged");
        let markdown = Markdown::try_from(path.as_path()).expect("document loads");
        let parsed = parse_lifecycle_config(&frontmatter_json(&markdown), &path).map(|_| ()).map_err(|error| format!("{error} {error:?}"));
        (judged.report.valid, problems(&judged.report), parsed)
    }

    /// Both the schema and the parser accept `frontmatter`.
    fn assert_both_accept(&self, frontmatter: &str) {
        let (valid, found, parsed) = self.verdicts(frontmatter);
        assert!(valid, "the schema accepts `{frontmatter}`: {found:?}");
        assert!(parsed.is_ok(), "the parser accepts `{frontmatter}`: {parsed:?}");
    }

    /// Both reject `frontmatter`: the schema at or below `pointer`, the
    /// parser naming `needle`.
    fn assert_both_reject(&self, frontmatter: &str, pointer: &str, needle: &str) {
        let (valid, found, parsed) = self.verdicts(frontmatter);
        assert!(!valid, "the schema rejects `{frontmatter}`");
        assert!(
            found.iter().any(|(path, _)| path == pointer || path.starts_with(&format!("{pointer}/"))),
            "`{frontmatter}` is rejected at `{pointer}`: {found:?}"
        );
        let error = parsed.expect_err(&format!("the parser rejects `{frontmatter}`"));
        assert!(error.contains(needle), "`{frontmatter}`: `{needle}` in {error}");
    }
}

/// A single-action dictionary `body` as the root of `event`'s stack, as the
/// `then` body of a guarded item, and as its `else` body, with the instance
/// pointer of the dictionary in each.
fn single_action_positions(event: &str, body: &str) -> [(String, String); 3] {
    [
        (format!("{event}: {body}"), format!("/{event}")),
        (format!("{event}: [{{ when: \"fm.ready\", then: {body} }}]"), format!("/{event}/0/then")),
        (format!("{event}: [{{ when: \"fm.ready\", then: [{{ info: hi }}], else: {body} }}]"), format!("/{event}/0/else")),
    ]
}

/// Every event family accepts a single-action dictionary in long form at its
/// root and in both branches, as the lifecycle parser does (R1-F2.1–R1-F2.3),
/// and still refuses there what the family refuses as a list item.
#[test]
fn single_action_dictionaries_take_long_forms_at_every_stack_position() {
    let judge = SingleActionJudge::new();
    let ordinary = [
        "{ ensure_file: { file: notes.md } }",
        "{ say: { text: done } }",
        "{ shell: { command: \"git status\" } }",
        "{ error: { reason: failed } }",
    ];
    for event in ["start", "blocked", "success", "failure", "finalize"] {
        for body in ordinary {
            for (frontmatter, _) in single_action_positions(event, body) {
                judge.assert_both_accept(&frontmatter);
            }
        }
    }
    for body in ["{ info: { text: preparing } }", "{ ensure_file: { file: notes.md } }", "{ skip: { no_error: true } }"] {
        for (frontmatter, _) in single_action_positions("initialize", body) {
            judge.assert_both_accept(&frontmatter);
        }
    }
    for body in ["{ stop: { no_error: true } }", "{ shell: { command: \"git stash\" } }", "{ warn: { text: cancelled } }"] {
        for (frontmatter, _) in single_action_positions("cancelled", body) {
            judge.assert_both_accept(&frontmatter);
        }
    }

    // Event restrictions hold in the new form: no shell in `initialize`, no
    // recovery directive in `cancelled`.
    for (frontmatter, pointer) in single_action_positions("initialize", "{ shell: { command: ls } }") {
        judge.assert_both_reject(&frontmatter, &pointer, "shell");
    }
    for (frontmatter, pointer) in single_action_positions("cancelled", "{ retry: { max_attempts: 2 } }") {
        judge.assert_both_reject(&frontmatter, &pointer, "retry");
    }
}

/// Short dictionaries, one-element lists holding a long form, and bundles of
/// short forms keep passing (R1-F2.6); several verbs beside a long form keep
/// failing (R1-F2.7).
#[test]
fn single_action_controls_keep_their_verdicts() {
    let judge = SingleActionJudge::new();
    for event in ["initialize", "start", "blocked", "success", "failure", "cancelled", "finalize"] {
        judge.assert_both_accept(&format!("{event}: {{ info: hi }}"));
        judge.assert_both_accept(&format!("{event}: [{{ info: {{ text: hi }} }}]"));
        judge.assert_both_accept(&format!("{event}: {{ info: hi, warn: careful }}"));
    }
    judge.assert_both_accept("loop: { until: \"{{ true }}\", info: hi, warn: careful }");
    judge.assert_both_accept("loop: { until: \"{{ true }}\", gate: [{ ensure_file: { file: review.txt } }] }");

    for (frontmatter, pointer) in [
        ("start: { ensure_file: { file: a.md }, info: hi }", "/start"),
        ("start: { ensure_file: { file: a.md }, ensure_dir: { dir: logs } }", "/start"),
        ("start: { ensure_file: { file: a.md }, info: hi, no_error: true }", "/start"),
        ("start: [{ when: \"fm.ready\", ensure_file: { file: a.md }, info: hi }]", "/start/0"),
        ("start: [{ when: \"fm.ready\", then: { ensure_file: { file: a.md }, info: hi } }]", "/start/0/then"),
        ("loop: { until: \"{{ true }}\", ensure_file: { file: a.md }, info: hi }", "/loop"),
        ("loop: { until: \"{{ true }}\", ensure_file: { file: a.md }, ensure_dir: { dir: logs } }", "/loop"),
    ] {
        judge.assert_both_reject(frontmatter, pointer, "long form");
    }
}

/// `no_error` beside a single long form, and `when` beside one in a list
/// item, are the parser's single-action dictionaries too.
#[test]
fn single_action_dictionaries_take_their_sibling_keys() {
    let judge = SingleActionJudge::new();
    for frontmatter in [
        "start: { ensure_file: { file: notes.md }, no_error: true }",
        "success: [{ when: \"fm.ready\", then: { say: { text: done }, no_error: true } }]",
        "failure: [{ when: \"fm.ready\", ensure_file: { file: failure.md } }]",
        "failure: [{ when: \"fm.ready\", info: { text: failed }, no_error: true }]",
        "finalize: [{ no_error: true, shell: { command: \"git status\" } }]",
        "initialize: [{ when: \"fm.ready\", info: { text: ready } }]",
        "cancelled: [{ when: \"fm.ready\", stop: { no_error: true } }]",
    ] {
        judge.assert_both_accept(frontmatter);
    }
}

/// A loop's single loose verb takes its long form (R1-F2.4), and its gate is
/// an ordinary stack whose root and branch dictionaries take long forms
/// (R1-F2.5).
#[test]
fn loops_take_a_single_loose_long_form_and_gate_dictionaries() {
    let judge = SingleActionJudge::new();
    for frontmatter in [
        "loop: { until: \"{{ true }}\", ensure_file: { file: review.txt } }",
        "loop: { until: \"{{ true }}\", ensure_file: { file: review.txt }, no_error: true }",
        "loop: { while: \"iteration < 3\", action: increment(iteration), max: 3, on_rate_limit: pause, say: { text: next } }",
        "loop: { while: \"iteration < 3\", gate: { ensure_file: { file: pass.txt } } }",
        "loop: { while: \"iteration < 3\", gate: [{ when: \"iteration == 3\", then: { warn: { text: cap } }, else: { ensure_file: { file: pass.txt } } }] }",
    ] {
        judge.assert_both_accept(frontmatter);
    }
    judge.assert_both_reject(
        "loop: { until: \"{{ true }}\", gate: [stop], ensure_file: { file: review.txt } }",
        "/loop/ensure_file",
        "gate",
    );
}

/// What the schema leaves to the parser at a dictionary: `when` beside the
/// actions of a stack root or body, which a bundle admitted before
/// single-action dictionaries were typed and a category admits now;
/// `no_error` written both beside and inside a long form; and a guarded item
/// holding no action, which the bundle type admits.
#[test]
fn single_action_placement_rules_stay_the_parsers() {
    let judge = SingleActionJudge::new();
    for (frontmatter, needle) in [
        ("start: { when: \"fm.ready\", info: hi }", "never carries `when`"),
        ("start: { when: \"fm.ready\", ensure_file: { file: a.md } }", "never carries `when`"),
        ("start: { ensure_file: { file: a.md, no_error: true }, no_error: true }", "both beside"),
        ("start: [{ when: \"fm.ready\", no_error: true }]", "needs `then` or sibling actions"),
    ] {
        let (valid, found, parsed) = judge.verdicts(frontmatter);
        assert!(valid, "the schema leaves `{frontmatter}` to the parser: {found:?}");
        let error = parsed.expect_err(frontmatter);
        assert!(error.contains(needle), "`{frontmatter}`: `{needle}` in {error}");
    }
}


/// A conditional is a list item only: in every family, a stack root or a
/// `then`/`else` body written as a dictionary holding `then` is refused by
/// the schema as by the parser, whose message names the misplaced `when`.
#[test]
fn a_conditional_is_never_a_stack_dictionary() {
    let judge = SingleActionJudge::new();
    for event in ["initialize", "start", "finalize", "cancelled"] {
        for (frontmatter, pointer) in single_action_positions(event, "{ when: \"fm.ready\", then: [stop] }") {
            judge.assert_both_reject(&frontmatter, &pointer, "when");
        }
    }
    judge.assert_both_reject("loop: { until: \"{{ true }}\", gate: { when: \"fm.ready\", then: [stop] } }", "/loop", "when");
}

/// A single-action dictionary's long form is bound through the verb's
/// runtime registration like a list item's: `number`'s numeric recovery binds
/// a subject its types reject, and `length` refuses a boolean its types
/// accept, at the root, in a branch, and as a loop's loose verb. The parser
/// does not type-check literal operands, so it accepts all of them.
#[test]
fn single_action_long_forms_bind_through_the_registration() {
    let judge = SingleActionJudge::new();
    for frontmatter in [
        "start: { number: { x: pear, fallback: 7 } }",
        "success: [{ when: \"fm.ready\", then: { number: { x: pear, fallback: 7 } } }]",
        "loop: { until: \"{{ true }}\", number: { x: pear, fallback: 7 } }",
    ] {
        judge.assert_both_accept(frontmatter);
    }
    for (frontmatter, pointer) in [
        ("start: { length: { val: true } }", "/start/length/val"),
        ("success: [{ when: \"fm.ready\", then: [{ info: hi }], else: { length: { val: true } } }]", "/success/0/else/length/val"),
        ("loop: { until: \"{{ true }}\", length: { val: true } }", "/loop/length/val"),
    ] {
        let (valid, found, parsed) = judge.verdicts(frontmatter);
        assert!(!valid, "the registration refuses `{frontmatter}`");
        assert!(found.iter().any(|(path, _)| path == pointer), "`{frontmatter}` at `{pointer}`: {found:?}");
        assert!(parsed.is_ok(), "the parser accepts `{frontmatter}`: {parsed:?}");
    }
}
