//! Embedded trigger bundles: a host's in-memory trigger schemas as the
//! last-resort trigger location after the five schema roots, read through
//! public results (the effective schema's verdicts, origins, and
//! dependencies, and the typed load errors).
//!
//! Every bundle here is synthetic text written in the test. The document
//! lives outside any repository, so its tree root (and the one disk schema
//! root these tests write to) is its own folder; the context is built by
//! `build_resolution_context` from a `RequestSnapshot` with an empty fixture
//! `HOME` and no environment.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use biscuit_file::{FileResolutionContext, canonicalize_simplified};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{RequestSnapshot, build_resolution_context};
use darkmatter::markdown::schemas::{
    DarkmatterSchemas, EmbeddedSchemaBundle, SchemaError, SchemaOriginKind, SchemaSourceId,
    TriggerSource, trace_registry,
};
use serde_json::json;

const BUNDLE: &str = "host-bundle";

/// The bundle's trigger: an inline payload importing a library that imports
/// another library, plus an `@this` reference to the payload's own `tag`.
const HOST_TRIGGER: &str = "kind: trigger-schema\n\
    match:\n  prompt: string(required)\n\
    $schema:\n  model: string\n  tag: string(not-empty)\n  title: tag@this\n  stack: stack@./lifecycle.yaml\n";

/// A library whose `stack` uses its own `verb` (bare, so `@this`), which is
/// imported from `action.yaml`.
const LIFECYCLE: &str = "kind: schema\ntypes:\n  verb: verb@./action.yaml\n  stack: \"verb[]\"\n";

const ACTION: &str = "kind: schema\ntypes:\n  verb: enum(say, stop)\n";

const HOST_BUNDLE: EmbeddedSchemaBundle = EmbeddedSchemaBundle::new(
    BUNDLE,
    &[("host.yaml", HOST_TRIGGER), ("lifecycle.yaml", LIFECYCLE), ("action.yaml", ACTION)],
);

/// A bundle whose files are test-built text.
fn bundle(name: &'static str, files: Vec<(&'static str, String)>) -> EmbeddedSchemaBundle {
    let files: Vec<(&'static str, &'static str)> =
        files.into_iter().map(|(file, text)| (file, &*Box::leak(text.into_boxed_str()))).collect();
    EmbeddedSchemaBundle::new(name, Box::leak(files.into_boxed_slice()))
}

fn write(path: &Path, contents: &str) {
    std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    std::fs::write(path, contents).expect("write fixture");
}

/// `{dir}/work` holds the document and its `schemas/` root; `{dir}/home` is
/// an empty fixture `HOME`.
struct Fixture {
    _dir: tempfile::TempDir,
    root: PathBuf,
    context: FileResolutionContext,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = canonicalize_simplified(dir.path()).expect("canonical temp dir");
        std::fs::create_dir_all(root.join("work")).expect("work dir");
        std::fs::create_dir_all(root.join("home")).expect("home dir");
        let snapshot = RequestSnapshot::new(root.join("work"))
            .with_home(Some(root.join("home")))
            .with_env(HashMap::new());
        let context = build_resolution_context(&snapshot).expect("fixture context");
        Self { _dir: dir, root, context }
    }

    fn disk_schema(&self, name: &str) -> PathBuf {
        self.root.join("work/schemas").join(name)
    }

    /// Writes `contents` as `name` in the document's `schemas/` root.
    fn write_disk_schema(&self, name: &str, contents: &str) -> PathBuf {
        let path = self.disk_schema(name);
        write(&path, contents);
        path
    }

    /// Trigger discovery with `bundles` appended after it.
    fn schemas(&self, bundles: &[EmbeddedSchemaBundle]) -> Result<DarkmatterSchemas, SchemaError> {
        let mut api = DarkmatterSchemas::new(self.context.clone()).with_trigger_discovery()?;
        for bundle in bundles {
            api = api.with_embedded_trigger_bundle(*bundle)?;
        }
        Ok(api)
    }
}

/// A document holding `frontmatter`.
fn document(frontmatter: &str) -> Markdown {
    Markdown::from(format!("---\n{frontmatter}---\nbody\n").as_str())
}

/// `None` when no schema applies to `frontmatter`, else its validity.
fn verdict(api: &DarkmatterSchemas, frontmatter: &str) -> Option<bool> {
    let markdown = document(frontmatter);
    api.effective_for(&markdown).expect("effective schema")?;
    Some(api.validate(&markdown).expect("validation").valid)
}

fn embedded_load_error(error: &SchemaError) -> Option<(&str, &str, &SchemaError)> {
    match error {
        SchemaError::EmbeddedTriggerLoad { bundle, name, source } => Some((bundle, name, source)),
        _ => None,
    }
}

// ── Applying an embedded trigger ────────────────────────────────────────────

#[test]
fn an_embedded_trigger_applies_with_its_imports_resolved_inside_the_bundle() {
    let fixture = Fixture::new();
    let api = fixture.schemas(&[HOST_BUNDLE]).expect("bundle loads");

    assert_eq!(verdict(&api, "prompt: x\nstack: [say, stop]\ntitle: hi\nmodel: m\n"), Some(true));
    // `stack` is typed through lifecycle.yaml's `verb[]` and action.yaml's enum.
    assert_eq!(verdict(&api, "prompt: x\nstack: [jump]\n"), Some(false));
    // `title` is the payload's own `tag` through `@this`.
    assert_eq!(verdict(&api, "prompt: x\ntitle: \"\"\n"), Some(false));
    assert_eq!(verdict(&api, "prompt: x\nmodel: [1]\n"), Some(false));
    // A document the trigger does not match gets no schema.
    assert_eq!(verdict(&api, "stack: [jump]\n"), None);

    let registry = api.trigger_registry().expect("registry");
    let sources: Vec<String> = registry.triggers.iter().map(|trigger| trigger.source.to_string()).collect();
    assert_eq!(sources, ["embedded:host-bundle/host.yaml"], "libraries are not triggers");
}

#[test]
fn registering_the_bundle_before_discovery_gives_the_same_registry() {
    let fixture = Fixture::new();
    let early = DarkmatterSchemas::new(fixture.context.clone())
        .with_embedded_trigger_bundle(HOST_BUNDLE)
        .expect("bundle")
        .with_trigger_discovery()
        .expect("discovery");
    let late = fixture.schemas(&[HOST_BUNDLE]).expect("bundle");
    assert_eq!(early.trigger_registry(), late.trigger_registry());
    assert_eq!(verdict(&early, "prompt: x\nstack: [jump]\n"), Some(false));
}

#[test]
fn a_string_payload_names_a_bundle_document() {
    let fixture = Fixture::new();
    for reference in ["payload.yaml", "./payload.yaml"] {
        let host = bundle(
            "strings",
            vec![
                (
                    "host.yaml",
                    format!("kind: trigger-schema\nmatch:\n  prompt: string(required)\n$schema: {reference}\n"),
                ),
                ("payload.yaml", "$schema:\n  stack: stack@./lifecycle.yaml\n".into()),
                ("lifecycle.yaml", LIFECYCLE.into()),
                ("action.yaml", ACTION.into()),
            ],
        );
        let api = fixture.schemas(&[host]).expect(reference);
        assert_eq!(verdict(&api, "prompt: x\nstack: [say]\n"), Some(true), "{reference}");
        assert_eq!(verdict(&api, "prompt: x\nstack: [jump]\n"), Some(false), "{reference}");
    }
}

#[test]
fn a_payload_naming_a_trigger_is_rejected_inside_the_bundle() {
    let fixture = Fixture::new();
    let trigger = |payload: &str| {
        format!("kind: trigger-schema\nmatch:\n  prompt: string(required)\n$schema: {payload}\n")
    };
    for (payload, target) in [("./host.yaml", "cycle/host.yaml"), ("other.yaml", "cycle/other.yaml")] {
        let host = bundle(
            "cycle",
            vec![("host.yaml", trigger(payload)), ("other.yaml", trigger("./host.yaml"))],
        );
        let error = fixture.schemas(&[host]).err().expect(payload);
        let (_, name, source) = embedded_load_error(&error).unwrap_or_else(|| panic!("{error:?}"));
        assert_eq!(name, "host.yaml");
        match source {
            SchemaError::TriggerPayloadCycle { payload_path, .. } => {
                assert_eq!(payload_path, Path::new(target), "{payload}");
            }
            other => panic!("{payload}: {other:?}"),
        }
    }
}

// ── Disk files of the same name win ─────────────────────────────────────────

#[test]
fn a_same_name_matching_disk_trigger_replaces_the_embedded_one() {
    let fixture = Fixture::new();
    let disk = fixture.write_disk_schema(
        "host.yaml",
        "kind: trigger-schema\nmatch:\n  prompt: string(required)\n$schema:\n  owner: string\n",
    );
    let api = fixture.schemas(&[HOST_BUNDLE]).expect("load");

    // The embedded `stack` type is gone; the disk payload's `owner` applies.
    assert_eq!(verdict(&api, "prompt: x\nstack: [jump]\n"), Some(true));
    assert_eq!(verdict(&api, "prompt: x\nowner: [1]\n"), Some(false));
    let effective = api.effective_for(&document("prompt: x\n")).unwrap().unwrap();
    let origin = &effective.origins["owner"];
    assert_eq!(origin.kind, SchemaOriginKind::Trigger);
    assert_eq!(origin.trigger_source(), Some(TriggerSource::File(disk)));
}

#[test]
fn a_same_name_disk_trigger_that_does_not_match_still_suppresses_the_embedded_one() {
    let fixture = Fixture::new();
    fixture.write_disk_schema(
        "host.yaml",
        "kind: trigger-schema\nmatch:\n  title: string(required)\n$schema:\n  owner: string\n",
    );
    let api = fixture.schemas(&[HOST_BUNDLE]).expect("load");
    assert_eq!(verdict(&api, "prompt: x\nstack: [jump]\n"), None);
}

#[test]
fn a_same_name_disk_file_that_is_not_a_trigger_suppresses_the_embedded_one() {
    let fixture = Fixture::new();
    fixture.write_disk_schema("host.yaml", "kind: schema\ntypes:\n  verb: string\n");
    let api = fixture.schemas(&[HOST_BUNDLE]).expect("load");
    assert_eq!(verdict(&api, "prompt: x\nstack: [jump]\n"), None);
    assert!(api.trigger_registry().unwrap().triggers.is_empty());
}

#[test]
fn a_same_name_malformed_disk_trigger_is_an_error_with_no_embedded_fallback() {
    let fixture = Fixture::new();
    let disk = fixture.write_disk_schema("host.yaml", "kind: trigger-schema\nmatch: 42\n$schema: {}\n");
    for early in [false, true] {
        let result = if early {
            DarkmatterSchemas::new(fixture.context.clone())
                .with_embedded_trigger_bundle(HOST_BUNDLE)
                .and_then(DarkmatterSchemas::with_trigger_discovery)
        } else {
            fixture.schemas(&[HOST_BUNDLE])
        };
        match result {
            Err(SchemaError::TriggerLoad { path, .. }) => assert_eq!(path, disk),
            Err(other) => panic!("registered early: {early}: {other:?}"),
            Ok(_) => panic!("registered early: {early}: the malformed disk trigger was replaced"),
        }
    }
}

#[test]
fn a_disk_trigger_never_imports_from_the_bundle() {
    let fixture = Fixture::new();
    let disk = fixture.write_disk_schema(
        "other.yaml",
        "kind: trigger-schema\nmatch:\n  title: string(required)\n$schema:\n  stack: stack@./lifecycle.yaml\n",
    );
    match fixture.schemas(&[HOST_BUNDLE]) {
        Err(SchemaError::TriggerLoad { path, .. }) => assert_eq!(path, disk),
        Err(other) => panic!("{other:?}"),
        Ok(_) => panic!("`lifecycle.yaml` resolved from the embedded bundle"),
    }
}

// ── Embedded import failures ────────────────────────────────────────────────

#[test]
fn embedded_imports_that_miss_or_leave_the_bundle_are_errors() {
    let fixture = Fixture::new();
    let trigger = |payload: &str| {
        format!("kind: trigger-schema\nmatch:\n  prompt: string(required)\n$schema: {payload}\n")
    };
    for (cell, host, expected) in [
        ("missing import", trigger("\n  stack: stack@./absent.yaml"), "is not an embedded schema document"),
        ("missing payload", trigger("./absent.yaml"), "not a registered embedded schema document"),
        ("parent import", trigger("\n  stack: stack@../lifecycle.yaml"), "cannot leave bundle `escape`"),
        ("deep parent import", trigger("\n  stack: stack@../../x.yaml"), "cannot leave bundle `escape`"),
        ("parent payload", trigger("../payload.yaml"), "cannot leave bundle `escape`"),
        ("absolute import", trigger("\n  stack: stack@/lifecycle.yaml"), "relative path"),
    ] {
        let escape = bundle(
            "escape",
            vec![("host.yaml", host), ("lifecycle.yaml", LIFECYCLE.into()), ("action.yaml", ACTION.into())],
        );
        let error = fixture.schemas(&[escape]).err().unwrap_or_else(|| panic!("{cell}: loaded"));
        let (bundle_name, name, source) =
            embedded_load_error(&error).unwrap_or_else(|| panic!("{cell}: {error:?}"));
        assert_eq!((bundle_name, name), ("escape", "host.yaml"), "{cell}");
        assert!(source.to_string().contains(expected), "{cell}: {source}");
    }
}

#[test]
fn a_syntax_invalid_embedded_library_reports_its_yaml_error() {
    let fixture = Fixture::new();
    let broken = bundle(
        "broken",
        vec![("host.yaml", HOST_TRIGGER.into()), ("lifecycle.yaml", "kind: schema\ntypes:\n  stack: [\n".into())],
    );
    let error = fixture.schemas(&[broken]).err().expect("broken library");
    let (_, name, source) = embedded_load_error(&error).unwrap_or_else(|| panic!("{error:?}"));
    assert_eq!(name, "host.yaml");
    assert!(source.to_string().contains("not valid YAML"), "{source}");
}

#[test]
fn an_unusable_bundle_shape_is_rejected_when_registered() {
    let fixture = Fixture::new();
    for (cell, candidate) in [
        ("empty name", EmbeddedSchemaBundle::new("", &[("a.yaml", "")])),
        ("nested name", EmbeddedSchemaBundle::new("a/b", &[("a.yaml", "")])),
        ("dot-dot name", EmbeddedSchemaBundle::new("..", &[("a.yaml", "")])),
        ("nested file", EmbeddedSchemaBundle::new("b", &[("x/a.yaml", "")])),
        ("not yaml", EmbeddedSchemaBundle::new("b", &[("a.json", "")])),
        ("duplicate", EmbeddedSchemaBundle::new("b", &[("a.yaml", ""), ("a.yaml", "")])),
        ("case-folded", EmbeddedSchemaBundle::new("b", &[("A.yaml", ""), ("a.yaml", "")])),
    ] {
        let without_registry = DarkmatterSchemas::new(fixture.context.clone()).with_embedded_trigger_bundle(candidate);
        assert!(
            matches!(without_registry, Err(SchemaError::EmbeddedBundle { .. })),
            "{cell}: accepted"
        );
        assert!(matches!(fixture.schemas(&[candidate]), Err(SchemaError::EmbeddedBundle { .. })), "{cell}");
    }
}

// ── Origins, simultaneous triggers, and dependencies ────────────────────────

#[test]
fn origins_name_the_embedded_or_disk_trigger_that_declared_each_property() {
    let fixture = Fixture::new();
    let disk = fixture.write_disk_schema(
        "disk.yaml",
        "kind: trigger-schema\nmatch:\n  prompt: string(required)\n$schema:\n  owner: string\n",
    );
    let api = fixture.schemas(&[HOST_BUNDLE]).expect("load");

    // Both winning triggers apply at once.
    assert_eq!(verdict(&api, "prompt: x\nowner: [1]\n"), Some(false));
    assert_eq!(verdict(&api, "prompt: x\nstack: [jump]\n"), Some(false));
    assert_eq!(verdict(&api, "prompt: x\nowner: o\nstack: [say]\n"), Some(true));

    let effective = api.effective_for(&document("prompt: x\n")).unwrap().unwrap();
    let model = &effective.origins["model"];
    assert_eq!(model.kind, SchemaOriginKind::Trigger);
    assert_eq!(model.source, Some(SchemaSourceId::Embedded("host-bundle/host.yaml".into())));
    assert_eq!(model.file(), None, "an embedded origin names no host file");
    assert_eq!(
        model.trigger_source(),
        Some(TriggerSource::Embedded { bundle: BUNDLE.into(), name: "host.yaml".into() })
    );
    let owner = &effective.origins["owner"];
    assert_eq!(owner.kind, SchemaOriginKind::Trigger);
    assert_eq!(owner.file(), Some(disk.as_path()));
    assert_eq!(owner.trigger_source(), Some(TriggerSource::File(disk.clone())));

    // Only the disk envelope is a dependency.
    assert_eq!(effective.dependencies(), [disk]);

    // An authored redeclaration takes the property over.
    let authored = api
        .effective_for(&document("$schema:\n  model: number\nprompt: x\n"))
        .unwrap()
        .unwrap();
    assert_eq!(authored.origins["model"].kind, SchemaOriginKind::Document);
    assert_eq!(authored.origins["model"].trigger_source(), None);
    assert_eq!(authored.origins["stack"].kind, SchemaOriginKind::Trigger);
}

#[test]
fn an_embedded_trigger_adds_no_dependencies() {
    let fixture = Fixture::new();
    let api = fixture.schemas(&[HOST_BUNDLE]).expect("load");
    let effective = api.effective_for(&document("prompt: x\n")).unwrap().unwrap();
    assert!(effective.dependencies().is_empty(), "{:?}", effective.dependencies());
}

// ── Reuse across verdicts ───────────────────────────────────────────────────

/// A bundle's texts never change in a process, so every registry that loads
/// it shares one resolved payload, and judging the same document again
/// shares one effective schema rather than rebuilding a host-sized one.
#[test]
fn every_registration_of_a_bundle_shares_its_resolved_payload() {
    let fixture = Fixture::new();
    let first = fixture.schemas(&[HOST_BUNDLE]).expect("first registry");
    let second = fixture.schemas(&[HOST_BUNDLE]).expect("second registry");
    let markdown = document("prompt: x\nmodel: m\n");
    let a = first.effective_for(&markdown).expect("effective").expect("trigger applies");
    let b = second.effective_for(&markdown).expect("effective").expect("trigger applies");
    assert!(std::sync::Arc::ptr_eq(&a.json_schema, &b.json_schema), "one payload serves both");
    assert!(!a.validate(&json!({"prompt": "x", "model": [1]})).valid);
    assert!(b.validate(&json!({"prompt": "x", "model": "m"})).valid);
}

/// Merges are shared only between equal inputs: the same authored `$schema`
/// over the same payload gives one merged schema, and a different authored
/// declaration gives its own, with its own verdicts.
#[test]
fn a_merge_is_shared_only_by_equal_authored_schemas() {
    let fixture = Fixture::new();
    let api = fixture.schemas(&[HOST_BUNDLE]).expect("registry");
    let effective = |frontmatter: &str| {
        api.effective_for(&document(frontmatter)).expect("effective").expect("schema applies")
    };
    let first = effective("$schema:\n  extra: string\nprompt: x\n");
    let again = effective("$schema:\n  extra: string\nprompt: y\n");
    let other = effective("$schema:\n  extra: number\nprompt: x\n");
    assert!(std::sync::Arc::ptr_eq(&first.json_schema, &again.json_schema));
    assert!(!std::sync::Arc::ptr_eq(&first.json_schema, &other.json_schema));
    assert!(first.validate(&json!({"prompt": "x", "extra": "e"})).valid);
    assert!(!first.validate(&json!({"prompt": "x", "extra": "e", "model": [1]})).valid, "the payload still applies");
    assert!(!other.validate(&json!({"prompt": "x", "extra": [1]})).valid);
    assert!(other.validate(&json!({"prompt": "x", "extra": 2})).valid);
}

/// A payload reads the bundle's libraries, so a reused trigger text with an
/// edited library is resolved again: the edit changes the verdict, and a
/// broken library is reported rather than masked by the earlier success.
#[test]
fn an_edited_library_under_the_same_trigger_text_is_resolved_again() {
    let fixture = Fixture::new();
    let shared_trigger: &'static str = Box::leak(
        "kind: trigger-schema\nmatch:\n  prompt: string(required)\n$schema:\n  verb: verb@./action.yaml\n"
            .to_string()
            .into_boxed_str(),
    );
    let with_action = |action: &str| {
        let files: Vec<(&'static str, &'static str)> =
            vec![("host.yaml", shared_trigger), ("action.yaml", Box::leak(action.to_string().into_boxed_str()))];
        EmbeddedSchemaBundle::new("shared-trigger", Box::leak(files.into_boxed_slice()))
    };
    let say_only = fixture.schemas(&[with_action("kind: schema\ntypes:\n  verb: enum(say)\n")]).expect("say");
    let stop_only = fixture.schemas(&[with_action("kind: schema\ntypes:\n  verb: enum(stop)\n")]).expect("stop");
    assert_eq!(verdict(&say_only, "prompt: x\nverb: say\n"), Some(true));
    assert_eq!(verdict(&stop_only, "prompt: x\nverb: say\n"), Some(false), "the edited library applies");
    let broken = fixture.schemas(&[with_action("kind: schema\ntypes: [\n")]).err().expect("broken library");
    assert!(embedded_load_error(&broken).is_some(), "{broken:?}");
}

/// Only a successful load is reused: a bundle that fails fails every time.
#[test]
fn a_failing_bundle_fails_on_every_registration() {
    let fixture = Fixture::new();
    let broken = bundle(
        "broken-twice",
        vec![(
            "host.yaml",
            "kind: trigger-schema\nmatch:\n  prompt: string(required)\n$schema:\n  stack: stack@./absent.yaml\n".into(),
        )],
    );
    for attempt in ["first", "second"] {
        let error = fixture.schemas(&[broken]).err().unwrap_or_else(|| panic!("{attempt}: loaded"));
        assert!(embedded_load_error(&error).is_some(), "{attempt}: {error:?}");
    }
}

#[test]
fn the_trigger_trace_lists_an_embedded_trigger_distinctly() {
    let fixture = Fixture::new();
    let api = fixture.schemas(&[HOST_BUNDLE]).expect("load");
    let registry = api.trigger_registry().expect("registry");
    let trace = trace_registry(registry, &json!({ "prompt": "x" }), None);
    assert_eq!(trace.triggers.len(), 1);
    assert_eq!(trace.triggers[0].source.to_string(), "embedded:host-bundle/host.yaml");
    assert!(trace.triggers[0].matched);
    assert_eq!(serde_json::to_value(&trace.triggers[0]).unwrap()["source"], json!("embedded:host-bundle/host.yaml"));
}

// ── Trigger-envelope input matrix over the embedded origin ──────────────────

/// What one matrix cell must do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Outcome {
    /// The trigger loads, applies to matching documents only, and enforces
    /// `model: string`.
    Applies,
    /// Not a trigger: the file contributes nothing.
    Unclassified,
    /// `$schema: {}`: the trigger loads and contributes no constraints.
    EmptyPayload,
    /// A visible error naming the embedded trigger; nothing loads.
    Error,
}

/// Observes one cell through the public load and validation results.
fn observe(fixture: &Fixture, text: &str) -> Result<Outcome, String> {
    let candidate = bundle("matrix", vec![("host.yaml", text.to_string())]);
    let api = match fixture.schemas(&[candidate]) {
        Ok(api) => api,
        Err(error) => {
            return match embedded_load_error(&error) {
                Some(("matrix", "host.yaml", _)) => Ok(Outcome::Error),
                _ => Err(format!("unexpected error {error:?}")),
            };
        }
    };
    let triggers = api.trigger_registry().map_or(0, |registry| registry.triggers.len());
    let violating = verdict(&api, "prompt: x\nmodel: [1]\n");
    let conforming = verdict(&api, "prompt: x\nmodel: m\n");
    let unmatched = verdict(&api, "model: [1]\n");
    match (triggers, violating, conforming, unmatched) {
        (0, None, None, None) => Ok(Outcome::Unclassified),
        (1, Some(false), Some(true), None) => Ok(Outcome::Applies),
        (1, Some(true), Some(true), None) => Ok(Outcome::EmptyPayload),
        other => Err(format!("inconsistent observation (triggers, violating, conforming, unmatched) = {other:?}")),
    }
}

/// Walks `cells` (name, source text, required outcome) and reports every
/// cell whose observed outcome differs.
fn walk(cells: &[(&str, String, Outcome)]) {
    let fixture = Fixture::new();
    let failures: Vec<String> = cells
        .iter()
        .filter_map(|(cell, text, expected)| match observe(&fixture, text) {
            Ok(observed) if observed == *expected => None,
            Ok(observed) => Some(format!("{cell}: expected {expected:?}, observed {observed:?}\n{text}")),
            Err(reason) => Some(format!("{cell}: {reason}\n{text}")),
        })
        .collect();
    assert!(failures.is_empty(), "{} cell(s) failed:\n{}", failures.len(), failures.join("\n---\n"));
}

#[test]
fn the_trigger_envelope_matrix_holds_for_block_yaml() {
    use Outcome::{Applies, EmptyPayload, Error, Unclassified};
    let kind = "kind: trigger-schema\n";
    let match_ = "match:\n  prompt: string(required)\n";
    let schema = "$schema:\n  model: string\n";
    let doc = |kind: &str, match_: &str, schema: &str| format!("{kind}{match_}{schema}");
    let cells = vec![
        ("control", doc(kind, match_, schema), Applies),
        ("kind absent", doc("", match_, schema), Unclassified),
        ("kind null", doc("kind: null\n", match_, schema), Unclassified),
        ("kind wrong type", doc("kind: 42\n", match_, schema), Unclassified),
        ("kind one element wrong", doc("kind: [trigger-schema, 42]\n", match_, schema), Unclassified),
        ("kind every element wrong", doc("kind: [1, 2]\n", match_, schema), Unclassified),
        ("kind empty", doc("kind: \"\"\n", match_, schema), Unclassified),
        ("kind duplicate", doc("kind: trigger-schema\nkind: trigger-schema\n", match_, schema), Error),
        ("kind trailing", doc("kind: trigger-schema\n}\n", match_, schema), Error),
        ("match absent", doc(kind, "", schema), Error),
        ("match null", doc(kind, "match: null\n", schema), Error),
        ("match wrong type", doc(kind, "match: 42\n", schema), Error),
        ("match one element wrong", doc(kind, "match:\n  - prompt: string(required)\n  - 42\n", schema), Error),
        ("match every element wrong", doc(kind, "match:\n  - 42\n  - true\n", schema), Error),
        ("match empty mapping", doc(kind, "match: {}\n", schema), Error),
        ("match empty sequence", doc(kind, "match: []\n", schema), Error),
        (
            "match duplicate",
            doc(kind, "match:\n  prompt: string(required)\n  prompt: string(required)\n", schema),
            Error,
        ),
        ("match trailing", doc(kind, "match:\n  prompt: string(required)\n}\n", schema), Error),
        ("$schema absent", doc(kind, match_, ""), Error),
        ("$schema null", doc(kind, match_, "$schema: null\n"), Error),
        ("$schema wrong type", doc(kind, match_, "$schema: 42\n"), Error),
        ("$schema one element wrong", doc(kind, match_, "$schema:\n  model: string\n  bad: 42\n"), Error),
        ("$schema every element wrong", doc(kind, match_, "$schema:\n  a: 42\n  b: true\n"), Error),
        ("$schema empty mapping", doc(kind, match_, "$schema: {}\n"), EmptyPayload),
        ("$schema empty string", doc(kind, match_, "$schema: \"\"\n"), Error),
        ("$schema empty sequence", doc(kind, match_, "$schema: []\n"), Error),
        ("$schema duplicate", doc(kind, match_, "$schema:\n  model: string\n  model: number\n"), Error),
        ("$schema trailing", doc(kind, match_, "$schema:\n  model: string\n}\n"), Error),
        ("non-string top-level key", format!("{}42: extra\n", doc(kind, match_, schema)), Error),
    ];
    walk(&cells);
}

#[test]
fn the_trigger_envelope_matrix_holds_for_json_syntax() {
    use Outcome::{Applies, EmptyPayload, Error, Unclassified};
    let kind = r#""kind": "trigger-schema""#;
    let match_ = r#""match": {"prompt": "string(required)"}"#;
    let schema = r#""$schema": {"model": "string"}"#;
    let doc = |fields: &[&str]| {
        let fields: Vec<&str> = fields.iter().copied().filter(|field| !field.is_empty()).collect();
        format!("{{{}}}\n", fields.join(", "))
    };
    let cells = vec![
        ("control", doc(&[kind, match_, schema]), Applies),
        ("kind absent", doc(&["", match_, schema]), Unclassified),
        ("kind null", doc(&[r#""kind": null"#, match_, schema]), Unclassified),
        ("kind wrong type", doc(&[r#""kind": 42"#, match_, schema]), Unclassified),
        ("kind one element wrong", doc(&[r#""kind": ["trigger-schema", 42]"#, match_, schema]), Unclassified),
        ("kind every element wrong", doc(&[r#""kind": [1, 2]"#, match_, schema]), Unclassified),
        ("kind empty", doc(&[r#""kind": """#, match_, schema]), Unclassified),
        ("kind duplicate", doc(&[kind, kind, match_, schema]), Error),
        ("kind trailing", doc(&[r#""kind": "trigger-schema"]"#, match_, schema]), Error),
        ("match absent", doc(&[kind, "", schema]), Error),
        ("match null", doc(&[kind, r#""match": null"#, schema]), Error),
        ("match wrong type", doc(&[kind, r#""match": 42"#, schema]), Error),
        ("match one element wrong", doc(&[kind, r#""match": [{"prompt": "string(required)"}, 42]"#, schema]), Error),
        ("match every element wrong", doc(&[kind, r#""match": [42, true]"#, schema]), Error),
        ("match empty mapping", doc(&[kind, r#""match": {}"#, schema]), Error),
        ("match empty sequence", doc(&[kind, r#""match": []"#, schema]), Error),
        (
            "match duplicate",
            doc(&[kind, r#""match": {"prompt": "string(required)", "prompt": "string(required)"}"#, schema]),
            Error,
        ),
        ("match trailing", doc(&[kind, r#""match": {"prompt": "string(required)"}}"#, schema]), Error),
        ("$schema absent", doc(&[kind, match_, ""]), Error),
        ("$schema null", doc(&[kind, match_, r#""$schema": null"#]), Error),
        ("$schema wrong type", doc(&[kind, match_, r#""$schema": 42"#]), Error),
        ("$schema one element wrong", doc(&[kind, match_, r#""$schema": {"model": "string", "bad": 42}"#]), Error),
        ("$schema every element wrong", doc(&[kind, match_, r#""$schema": {"a": 42, "b": true}"#]), Error),
        ("$schema empty mapping", doc(&[kind, match_, r#""$schema": {}"#]), EmptyPayload),
        ("$schema empty string", doc(&[kind, match_, r#""$schema": """#]), Error),
        ("$schema empty sequence", doc(&[kind, match_, r#""$schema": []"#]), Error),
        (
            "$schema duplicate",
            doc(&[kind, match_, r#""$schema": {"model": "string", "model": "number"}"#]),
            Error,
        ),
        ("$schema trailing", doc(&[kind, match_, r#""$schema": {"model": "string"}]"#]), Error),
        ("non-string top-level key", doc(&[kind, match_, schema, r#"42: "extra""#]), Error),
    ];
    walk(&cells);
}
