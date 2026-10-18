//! Claudine's schema bundle under `claudine/schemas/` (`2026-10-21-claudine-schemas`,
//! criteria 1–5): the five files load through Darkmatter's loader, the action
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
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::expression::expression_function_descriptors;
use darkmatter::markdown::compose::{ComposeSource, RequestSnapshot, build_resolution_context};
use darkmatter::markdown::schemas::triggers::{PathSubject, matches, parse_trigger_envelope_from_str};
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
}

impl Discovered {
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
            let selected = trigger
                .envelope
                .match_arms
                .0
                .iter()
                .any(|arm| matches(arm, &frontmatter, &subject));
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
                advisories: Vec::new(),
            },
        };
        Ok(Judged {
            claudine,
            inline,
            properties,
            report,
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
fn generated_verbs() -> BTreeSet<String> {
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
        .flat_map(|category| property_names(&types[*category]))
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
        for verb in property_names(&types[*category]) {
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
            assert!(
                types[export]["properties"][verb.as_str()]["description"].is_string(),
                "`{export}.{verb}` is described"
            );
            let mut value = types[export]["properties"][verb.as_str()].clone();
            if let Some(object) = value.as_object_mut() {
                object.remove("description");
            }
            assert_eq!(value, json!({}), "`{export}.{verb}` is untyped");
        }
    }

    // The loop declares `max` once, as its iteration control, and every
    // other catalog verb as a loose key.
    let lifecycle = resolved_exports(&repo, "lifecycle.yaml", &["loop"]);
    let loop_keys = property_names(&lifecycle["loop"]);
    for verb in &catalogs {
        assert!(loop_keys.contains(verb), "`loop` lists `{verb}`");
    }
    let max_arms = arms_of(&lifecycle["loop"]["properties"]["max"]);
    assert_eq!(max_arms.len(), 2, "`loop.max` is a count or a template: {max_arms:?}");
    assert!(max_arms.iter().any(|arm| arm["type"] == "integer"), "`loop.max` is the iteration count");
    assert!(
        max_arms.iter().any(|arm| arm["type"] == "string" && arm["pattern"].is_string()),
        "`loop.max` may be a whole-value interpolation"
    );
    assert!(!loop_keys.contains("fail_fast"), "a loop has no `fail_fast`");
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
    let mut long_forms = 0;
    for category in CATEGORIES {
        for verb in property_names(&types[*category]) {
            let declared = &types[*category]["properties"][verb.as_str()];
            let arms: Vec<Value> = arms_of(declared).iter().map(without_descriptions).collect();
            // A long form is the one closed-object arm; `set` and the
            // generated verbs have none.
            let (long, short): (Vec<Value>, Vec<Value>) = arms.into_iter().partition(is_closed_object);
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
                        if parameter_arm["type"] == "object" {
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
        "bare-verb", "loose-body", "stack", "item-1", "conditional-1", "stack-2", "item-2", "conditional-2",
        "stack-3", "item-3", "conditional-3", "initialize-stack", "initialize-item-1", "initialize-conditional-1",
        "initialize-stack-2", "initialize-item-2", "initialize-conditional-2", "initialize-stack-3",
        "initialize-item-3", "initialize-conditional-3", "cancelled-conditional-5", "cancelled-item-6",
        "loop-action", "loop-action-entry", "loop",
    ];
    let lifecycle = resolved_exports(&repo, "lifecycle.yaml", &lifecycle_exports);
    assert_eq!(lifecycle.len(), lifecycle_exports.len());
    let linking_exports = [
        "name", "allowed-tools", "tools", "skills", "license", "compatibility", "metadata",
        "user-invocable", "disable-model-invocation", "argument-hint", "max_turns",
    ];
    assert_eq!(resolved_exports(&repo, "linking.yaml", &linking_exports).len(), 11);

    // Both triggers load, and each payload resolves: a document selecting
    // both gets every payload property.
    let discovered = repo.discover().expect("the shipped bundle loads");
    assert_eq!(discovered.schemas.trigger_registry().expect("registry").triggers.len(), 2);
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
/// `leaf` as its only action.
fn nested(depth: usize, leaf: &str) -> String {
    let mut body = format!("[{leaf}]");
    for level in (1..=depth).rev() {
        body = format!("[{{ when: \"ctx.level == {level}\", then: {body}, else: [{{ warn: \"no\" }}] }}]");
    }
    body
}

/// Judges stacks placed by `holder` (an event, or the loop's gate) at depths
/// 1, 2, 3, 5, and 6: a good leaf is valid at every depth, a bad leaf is
/// rejected only within the three typed levels (a leaf inside a level-3
/// conditional's body is not typed), and a level-3 conditional is itself
/// typed.
fn assert_typed_levels(bundle: &[BundleFile], claudine: &str, inline: &str, holder: &str, leaf_bad: &str) {
    let repo = SchemaRepo::new(bundle);
    let discovered = repo.discover().expect("the shipped bundle loads");
    let close = if holder.starts_with("loop") { " }" } else { "" };
    let judge = |name: &str, stack: String| {
        let path = repo.write_document(&format!("{name}.md"), &format!("---\n{holder}{stack}{close}\n---\n"));
        discovered.judge(&path, claudine, inline).expect("judged").report
    };
    for depth in [1, 2, 3, 5, 6] {
        let typed = depth < 3;
        let valid = judge("ok", nested(depth, "{ info: hi }"));
        assert!(valid.valid, "{holder}depth {depth}: {:?}", problems(&valid));
        let invalid = judge("bad", nested(depth, leaf_bad));
        assert_eq!(
            invalid.valid, !typed,
            "{holder}depth {depth}: a bad leaf is rejected only within the typed levels: {:?}",
            problems(&invalid)
        );
    }
    let mixed = judge("mixed", nested(2, "{ when: \"ctx.x\", then: [stop], info: hi }"));
    assert!(!mixed.valid, "{holder}a level-3 conditional is typed");
}

#[test]
fn event_stacks_are_typed_three_levels_deep_and_loose_below() {
    assert_typed_levels(
        &shipped_bundle!(),
        include_str!("../../../schemas/claudine.yaml"),
        include_str!("../../../schemas/inline-compose.yaml"),
        "start: ",
        "{ inf0: hi }",
    );
}

#[test]
fn initialize_stacks_are_typed_three_levels_deep_without_shell() {
    assert_typed_levels(
        &shipped_bundle!(),
        include_str!("../../../schemas/claudine.yaml"),
        include_str!("../../../schemas/inline-compose.yaml"),
        "initialize: ",
        "{ shell: ls }",
    );
}

#[test]
fn loop_gate_is_typed_three_levels_deep_and_loose_below() {
    assert_typed_levels(
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
    &["depth1", "depth3", "depth5", "depth6", "root-dictionary", "bundle-in-then", "loop-gate", "long-forms", "bare-stop"],
    &[
        "shell-in-initialize-list",
        "shell-in-initialize-then",
        "shell-in-initialize-root-dictionary",
        "shell-in-initialize-below-typed-levels",
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
        "generated-verb-arbitrary-value",
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

/// Depth 1, 3, and 5 are valid and depth 6 is the parser's to refuse; root
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
            ("root-dictionary", include_str!("../fixtures/claudine-schemas/root-dictionary.md"), CLAUDINE, Valid, Accepts),
            ("bundle-in-then", include_str!("../fixtures/claudine-schemas/bundle-in-then.md"), CLAUDINE, Valid, Accepts),
            ("loop-gate", include_str!("../fixtures/claudine-schemas/loop-gate.md"), CLAUDINE, Valid, Accepts),
            ("long-forms", include_str!("../fixtures/claudine-schemas/long-forms.md"), CLAUDINE, Valid, Accepts),
            ("bare-stop", include_str!("../fixtures/claudine-schemas/bare-stop.md"), CLAUDINE, Valid, Accepts),
        ],
    );
}

/// `shell` in `initialize` is a schema error at every typed site, the
/// parser's to refuse below them or inside `set`, and valid in `start`.
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
                "shell-in-initialize-below-typed-levels",
                include_str!("../fixtures/claudine-schemas/shell-in-initialize-below-typed-levels.md"),
                CLAUDINE,
                Valid,
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
/// bundles are schema errors naming the offending key; a generated verb's
/// value is left to the parser.
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
            (
                "generated-verb-arbitrary-value",
                include_str!("../fixtures/claudine-schemas/generated-verb-arbitrary-value.md"),
                CLAUDINE,
                Valid,
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
        for retired in ["fail_fast", "state", "previous", "next", "outputs", "sequence_id"] {
            assert!(!judged.properties.contains(retired), "retired catalog property {retired}");
        }
    }
}

/// Task guards and container loops are typed by shape only: a guard is
/// non-empty text, a container loop an object. The task and loop parsers
/// judge their contents.
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
/// shape in a `start` stack: what one accepts, the other accepts. An explicit
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
        ("[{break: {code: '2'}}]", false),
        ("[{break: {reason: [a]}}]", false),
        ("[{break: {exit: 2}}]", false),
        ("[{break: 5}]", false),
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
        let report = discovered
            .judge(
                &path,
                include_str!("../../../schemas/claudine.yaml"),
                include_str!("../../../schemas/inline-compose.yaml"),
            )
            .expect("judged")
            .report;
        assert_eq!(report.valid, *valid, "schema verdict for `start: {items}`: {:?}", problems(&report));
        let markdown = Markdown::try_from(path.as_path()).expect("document loads");
        let parsed = parse_lifecycle_config(&frontmatter_json(&markdown), &path);
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
            frontmatter: "$schema:\n  nested: task@../schemas/lifecycle.yaml\nnested:\n  name: child\n  sequence: []\n  cancelled: ITEMS",
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
            frontmatter: "$schema:\n  nested: group@../schemas/lifecycle.yaml\nnested:\n  tasks: [{ name: leg, shell: 'echo done' }]\n  cancelled: ITEMS",
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
            frontmatter: "$schema:\n  nested: group@../schemas/lifecycle.yaml\nnested:\n  tasks:\n    - name: leg\n      sequence: []\n      cancelled: ITEMS",
            pointer: "/nested/tasks/0/cancelled/0",
        },
        &FORBIDDEN_IN_CANCELLED[..1],
    );
}

// ── Cancellation stacks at every conditional depth ─────────────────────────

/// The deepest conditional level Claudine's lifecycle parser accepts
/// (`MAX_CONDITIONAL_DEPTH` in `composition/lifecycle/parse.rs`).
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

/// The cancellation stack ends where the parser does: a conditional one level
/// deeper than it accepts is a schema error at that conditional, on a document
/// and on a nested container, and the lifecycle parser refuses the document's
/// stack.
#[test]
fn cancelled_rejects_a_conditional_deeper_than_the_parser_accepts() {
    let repo = SchemaRepo::new(&shipped_bundle!());
    let discovered = repo.discover().expect("the shipped bundle loads");
    let level = PARSER_MAX_CONDITIONAL_DEPTH + 1;
    for site in [DOCUMENT_CANCELLED, TASK_2_CANCELLED] {
        let (items, _) = in_conditional(level, "then", "stop");
        let text = format!("---\n{}\n---\n", site.frontmatter.replace("ITEMS", &items));
        let path = repo.write_document("too-deep.md", &text);
        let report = discovered
            .judge(
                &path,
                include_str!("../../../schemas/claudine.yaml"),
                include_str!("../../../schemas/inline-compose.yaml"),
            )
            .expect("judged")
            .report;
        let pointer = format!("{}{}", site.stack, "/0/then".repeat(level - 1) + "/0");
        let found = problems(&report);
        assert!(
            found.iter().any(|(path, _)| *path == pointer),
            "{}: a level-{level} conditional is rejected at `{pointer}`: {found:?}",
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

/// Pins the documented container bound: SimplifiedSchema inlines named types
/// and cannot recurse, so containers are typed two levels deep (a task or
/// group, and its direct children). A third-level container's list is any
/// list, so its `cancelled` stack is checked by the task parser before
/// anything runs, not by the schema. This bound is the agreed contract
/// (sequence-containers requirement 5, ruling R1-HR1); deepening it is a
/// deliberate change.
#[test]
fn grandchild_cancelled_is_left_to_the_task_parser() {
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
    assert!(report.valid, "below the typed levels: {:?}", problems(&report));
}
