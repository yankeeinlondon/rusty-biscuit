//! DMLS applies Claudine's schemas through its host registration alone.
//!
//! A fixture repository holds package `claudine/lib` in package area
//! `claudine`, and registers Claudine's bundle (its trigger, library, and
//! namespace files and its lifecycle artifact, installed under the
//! repository's `.claudine/`, which is no schema root) in its
//! `.darkmatter.yaml`. A document in the repository that declares no
//! `$schema` is typed by the registered `composition.yaml`,
//! `inline-composition.yaml`, and `sequence-document.yaml` trigger payloads.
//! A document outside the repository is not: no registration applies there.
//! DMLS has no Claudine dependency, so nothing else could supply it.
//!
//! The same trigger payloads drive completion and hover: every editor request
//! below runs against a document with no `$schema`. The editor requests use
//! documents `composition.yaml` selects (`agent`): `inline-composition.yaml`'s
//! payload is a root union, and a registered root-union layer carries no
//! editor shape, so DMLS offers no completion or hover from it.

use crate::common;
use crate::host_fixtures::{Bundle, MANIFEST_NAME};

use std::path::{Path, PathBuf};

use biscuit_file::canonicalize_simplified;
use common::{LspFixture, LspWorkspace};
use darkmatter::markdown::compose::RequestSnapshot;
use simplified_schema::roots::SchemaRoots;
use dmls::context::RepositoryContexts;
use dmls::overlay::SchemaOutcome;
use dmls::{DmlsConfig, OverlayState};
use serde_json::{Value, json};

/// Claudine's registration members, copied byte for byte into the fixture;
/// `globals.lifecycle.json` is the bundle's lifecycle artifact.
const BUNDLE: [(&str, &str); 14] = [
    ("action.yaml", include_str!("../../../../claudine/registration/action.yaml")),
    ("composition.yaml", include_str!("../../../../claudine/registration/composition.yaml")),
    ("document.yaml", include_str!("../../../../claudine/registration/document.yaml")),
    ("inline-composition.yaml", include_str!("../../../../claudine/registration/inline-composition.yaml")),
    ("lifecycle-events.yaml", include_str!("../../../../claudine/registration/lifecycle-events.yaml")),
    ("linking.yaml", include_str!("../../../../claudine/registration/linking.yaml")),
    ("shared.yaml", include_str!("../../../../claudine/registration/shared.yaml")),
    ("content-policy.yaml", include_str!("../../../../claudine/registration/content-policy.yaml")),
    ("sequence.yaml", include_str!("../../../../claudine/registration/sequence.yaml")),
    ("sequence-document.yaml", include_str!("../../../../claudine/registration/sequence-document.yaml")),
    ("group.yaml", include_str!("../../../../claudine/registration/group.yaml")),
    ("loop.yaml", include_str!("../../../../claudine/registration/loop.yaml")),
    ("seq.yaml", include_str!("../../../../claudine/registration/seq.yaml")),
    ("globals.lifecycle.json", include_str!("../../../../claudine/lib/generated/globals.lifecycle.json")),
];

/// The bundle path of the lifecycle artifact among [`BUNDLE`]'s files.
const LIFECYCLE: &str = "schemas/globals.lifecycle.json";

/// The eleven linking keys `linking.yaml` types.
const LINKING_KEYS: [&str; 11] = [
    "name",
    "allowed-tools",
    "tools",
    "skills",
    "license",
    "compatibility",
    "metadata",
    "user-invocable",
    "disable-model-invocation",
    "argument-hint",
    "max_turns",
];

pub(crate) fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

pub(crate) struct Fixture {
    root: PathBuf,
}

impl Fixture {
    /// A repository (a `.git` skeleton Git discovery accepts, without running
    /// `git`) that registers the bundle installed in its `.claudine/`, and an
    /// empty home.
    pub(crate) fn new(workspace: &Path) -> Self {
        let fixture = Self { root: canonicalize_simplified(workspace).unwrap() };
        let repo = fixture.repo();
        std::fs::create_dir_all(repo.join(".git/objects")).unwrap();
        std::fs::create_dir_all(repo.join(".git/refs/heads")).unwrap();
        write(&repo.join(".git/HEAD"), "ref: refs/heads/main\n");
        write(&repo.join(".git/config"), "[core]\n\trepositoryformatversion = 0\n\tbare = false\n");
        write(&repo.join("Cargo.toml"), "[workspace]\nmembers = [\"claudine/lib\"]\n");
        write(
            &repo.join("claudine/lib/Cargo.toml"),
            "[package]\nname = \"claudine\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        );
        write(&repo.join("claudine/lib/src/lib.rs"), "");
        let mut bundle = Bundle {
            host: "claudine".to_string(),
            version: "0.0.1".to_string(),
            files: Vec::new(),
            lifecycle: LIFECYCLE.to_string(),
            members: Default::default(),
        };
        for (name, text) in BUNDLE {
            let path = format!("schemas/{name}");
            bundle = if path == LIFECYCLE { bundle.with_member(&path, text) } else { bundle.with_file(&path, text) };
        }
        bundle.write_to(&repo.join(".claudine"));
        write(&repo.join(".darkmatter.yaml"), &format!("register:\n  - ./.claudine/{MANIFEST_NAME}\n"));
        std::fs::create_dir_all(fixture.home()).unwrap();
        fixture
    }

    pub(crate) fn repo(&self) -> PathBuf {
        self.root.join("repo")
    }

    /// The installed bundle's `schemas/` folder.
    fn bundle(&self) -> PathBuf {
        self.repo().join(".claudine/schemas")
    }

    /// Writes `text` to `outside/{name}`, beside the repository, where no
    /// registration applies, and returns its path.
    pub(crate) fn outside(&self, name: &str, text: &str) -> PathBuf {
        let path = self.root.join("outside").join(name);
        write(&path, text);
        path
    }

    fn home(&self) -> PathBuf {
        self.root.join("home")
    }

    /// The server's snapshot: fixture home, no environment.
    pub(crate) fn snapshot(&self) -> RequestSnapshot {
        RequestSnapshot::new(&self.root).with_home(Some(self.home()))
    }

    /// Writes `text` to `claudine/lib/tasks/{name}` and returns its path.
    pub(crate) fn document(&self, name: &str, text: &str) -> PathBuf {
        let path = self.repo().join("claudine/lib/tasks").join(name);
        write(&path, text);
        path
    }
}

pub(crate) fn initialize_params(folder: &Path) -> Value {
    json!({
        "processId": null,
        "clientInfo": { "name": "Neovim", "version": "0.11.0" },
        "capabilities": {
            "general": { "positionEncodings": ["utf-16"] },
            "textDocument": {}
        },
        "workspaceFolders": [
            { "uri": url::Url::from_directory_path(folder).unwrap().as_str(), "name": "repo" }
        ]
    })
}

pub(crate) fn uri_of(path: &Path) -> String {
    url::Url::from_file_path(path).unwrap().to_string()
}

pub(crate) fn open(session: &LspFixture<'_>, uri: &str, text: &str) {
    session.notify(
        "textDocument/didOpen",
        json!({
            "textDocument": { "uri": uri, "languageId": "markdown", "version": 1, "text": text }
        }),
    );
}

/// Schema diagnostics (`dm.schema.*`) as `(instance path, message)` pairs.
fn schema_diagnostics(diagnostics: &[Value]) -> Vec<(String, String)> {
    diagnostics
        .iter()
        .filter(|diagnostic| diagnostic["code"].as_str().is_some_and(|code| code.starts_with("dm.schema.")))
        .map(|diagnostic| {
            (
                diagnostic["data"]["instance_path"].as_str().unwrap_or_default().to_string(),
                diagnostic["message"].as_str().unwrap_or_default().to_string(),
            )
        })
        .collect()
}

/// Opens each `(path, text)` in one session and returns each document's
/// schema diagnostics, in order.
fn session_diagnostics(workspace: &LspWorkspace, fixture: &Fixture, documents: &[(&Path, &str)]) -> Vec<Vec<(String, String)>> {
    let mut session = LspFixture::start_with_snapshot(workspace, fixture.snapshot());
    session.initialize(initialize_params(&fixture.repo()));
    let found = documents
        .iter()
        .map(|(path, text)| {
            let uri = uri_of(path);
            open(&session, &uri, text);
            schema_diagnostics(&session.wait_for_diagnostics(&uri))
        })
        .collect();
    session.shutdown();
    found
}

/// A misspelled long-form parameter under `start` is reported at the action's
/// path with the offending key, from the trigger alone; the same text outside
/// the repository, where no registration applies, is not.
#[test]
fn a_misspelled_long_form_key_is_reported_under_start_without_a_schema_pointer() {
    let workspace = LspWorkspace::new();
    let fixture = Fixture::new(workspace.path());
    let text = "---\nstart: [{say: {txet: hi}}]\n---\n\nBody\n";
    let inside = fixture.document("typo.md", text);
    let outside = fixture.outside("typo.md", text);

    let found = session_diagnostics(&workspace, &fixture, &[(&inside, text), (&outside, text)]);
    let (inside_found, outside_found) = (&found[0], &found[1]);
    assert!(
        inside_found.iter().any(|(path, message)| path.starts_with("/start/0/say") && message.contains("txet")),
        "expected a diagnostic at /start/0/say naming txet: {inside_found:?}"
    );
    assert!(inside_found.iter().all(|(path, _)| path.starts_with("/start/")), "{inside_found:?}");
    assert!(outside_found.is_empty(), "no Claudine trigger applies outside the repository: {outside_found:?}");
}

/// A lifecycle document using every stack shape and all eleven linking keys.
const VALID_LIFECYCLE_DOCUMENT: &str = concat!(
        "---\n",
        "model: sonnet\n",
        "start:\n",
        "  - say: starting\n",
        "  - say: { text: hello, no_error: true }\n",
        "  - when: \"{{ true }}\"\n",
        "    then:\n",
        "      - info: branch\n",
        "    else:\n",
        "      warn: other\n",
        "  - stop\n",
        "success:\n",
        "  notify: done\n",
        "  when: \"{{ true }}\"\n",
        "failure:\n",
        "  - error: broke\n",
        "loop:\n",
        "  max: 3\n",
        "  until: \"{{ true }}\"\n",
        "  gate:\n",
        "    - info: gate\n",
        "name: linked\n",
        "allowed-tools: Read Grep\n",
        "tools: [Read, Grep]\n",
        "skills: [rust]\n",
        "license: MIT\n",
        "compatibility: { claude: \">=1\" }\n",
        "metadata: { owner: someone }\n",
        "user-invocable: true\n",
        "disable-model-invocation: false\n",
        "argument-hint: \"[msg]\"\n",
        "max_turns: 5\n",
        "---\n\nBody\n",
);

/// A sequence document with no `$schema` pointer is typed by the
/// `sequence-document.yaml` trigger: the container specification's example raises no
/// schema diagnostic, and an undeclared step key is step data.
#[test]
fn sequence_documents_are_typed_by_the_sequence_trigger() {
    let workspace = LspWorkspace::new();
    let fixture = Fixture::new(workspace.path());
    let kinds = include_str!("../../../../claudine/lib/tests/fixtures/sequence-schema/example-containers-kinds.md");
    let step_data = include_str!("../../../../claudine/lib/tests/fixtures/sequence-schema/step-data.md");
    let kinds_path = fixture.document("kinds.md", kinds);
    let step_data_path = fixture.document("step-data.md", step_data);

    let found = session_diagnostics(&workspace, &fixture, &[(&kinds_path, kinds), (&step_data_path, step_data)]);
    assert!(found[0].is_empty(), "kinds: {:?}", found[0]);
    assert!(found[1].is_empty(), "step data: {:?}", found[1]);
}

/// A group's removed `execution` key is reported at the group, naming the
/// key, and a sequence document's root `fail_fast` at its own path.
#[test]
fn removed_sequence_keys_are_reported_at_their_paths() {
    let workspace = LspWorkspace::new();
    let fixture = Fixture::new(workspace.path());
    let execution = include_str!("../../../../claudine/lib/tests/fixtures/sequence-schema/group-execution.md");
    let fail_fast = include_str!("../../../../claudine/lib/tests/fixtures/sequence-schema/root-fail-fast.md");
    let execution_path = fixture.document("execution.md", execution);
    let fail_fast_path = fixture.document("fail-fast.md", fail_fast);

    let found = session_diagnostics(&workspace, &fixture, &[(&execution_path, execution), (&fail_fast_path, fail_fast)]);
    assert!(
        found[0].iter().any(|(path, message)| path == "/sequence/0/group" && message.contains("execution")),
        "expected a diagnostic at /sequence/0/group naming execution: {:?}",
        found[0]
    );
    assert!(found[1].iter().any(|(path, _)| path == "/fail_fast"), "expected a diagnostic at /fail_fast: {:?}", found[1]);
}

/// A native number or boolean `sequence` is reported at `/sequence`, never
/// read as a reference; a list and written reference text raise nothing.
#[test]
fn a_scalar_sequence_source_is_reported_at_sequence() {
    let workspace = LspWorkspace::new();
    let fixture = Fixture::new(workspace.path());
    let texts = [
        "---\nsequence: 42\n---\n\nBody\n",
        "---\nsequence: true\n---\n\nBody\n",
        "---\nsequence: []\n---\n\nBody\n",
        "---\nsequence: ./steps.yaml\n---\n\nBody\n",
    ];
    let paths: Vec<PathBuf> = texts.iter().enumerate().map(|(index, text)| fixture.document(&format!("s{index}.md"), text)).collect();
    let documents: Vec<(&Path, &str)> = paths.iter().map(PathBuf::as_path).zip(texts).collect();

    let found = session_diagnostics(&workspace, &fixture, &documents);
    for (index, value) in ["42", "true"].into_iter().enumerate() {
        let paths: Vec<&str> = found[index].iter().map(|(path, _)| path.as_str()).collect();
        assert_eq!(paths, ["/sequence"], "`sequence: {value}`: {:?}", found[index]);
    }
    assert!(found[2].is_empty(), "list: {:?}", found[2]);
    assert!(found[3].is_empty(), "reference: {:?}", found[3]);
}

/// A native number or boolean inline `prompt` is reported at `/prompt`, as
/// Claudine's inline runtime refuses it; prompt text, quoted digits included,
/// raises nothing.
#[test]
fn a_scalar_inline_prompt_is_reported_at_prompt() {
    let workspace = LspWorkspace::new();
    let fixture = Fixture::new(workspace.path());
    let texts = [
        "---\nprompt: 42\n---\n\nBody\n",
        "---\nprompt: true\n---\n\nBody\n",
        "---\nprompt: hi\n---\n\nBody\n",
        "---\nprompt: \"42\"\n---\n\nBody\n",
    ];
    let paths: Vec<PathBuf> = texts.iter().enumerate().map(|(index, text)| fixture.document(&format!("p{index}.md"), text)).collect();
    let documents: Vec<(&Path, &str)> = paths.iter().map(PathBuf::as_path).zip(texts).collect();

    let found = session_diagnostics(&workspace, &fixture, &documents);
    for (index, value) in ["42", "true"].into_iter().enumerate() {
        let paths: Vec<&str> = found[index].iter().map(|(path, _)| path.as_str()).collect();
        assert_eq!(paths, ["/prompt"], "`prompt: {value}`: {:?}", found[index]);
    }
    assert!(found[2].is_empty(), "text: {:?}", found[2]);
    assert!(found[3].is_empty(), "quoted digits: {:?}", found[3]);
}

/// A valid lifecycle stack and all eleven linking keys raise no schema
/// diagnostic, and a wrongly typed linking key is rejected.
#[test]
fn lifecycle_stacks_and_linking_keys_are_typed_by_the_discovered_trigger() {
    let workspace = LspWorkspace::new();
    let fixture = Fixture::new(workspace.path());
    let valid = VALID_LIFECYCLE_DOCUMENT;
    let mistyped = "---\nmodel: sonnet\nmax_turns: many\n---\n\nBody\n";
    let valid_path = fixture.document("valid.md", valid);
    let mistyped_path = fixture.document("mistyped.md", mistyped);

    let found = session_diagnostics(&workspace, &fixture, &[(&valid_path, valid), (&mistyped_path, mistyped)]);
    assert!(found[0].is_empty(), "{:?}", found[0]);
    assert!(found[1].iter().any(|(path, _)| path == "/max_turns"), "linking keys are typed: {:?}", found[1]);
}

/// The payload that types a document in the repository is the registered
/// bundle: no schema root holds a Claudine file, the payload declares the
/// events and linking keys, and the installed files are the effective
/// schema's dependencies.
#[test]
fn the_effective_schema_in_the_repository_is_the_registered_bundle() {
    let workspace = LspWorkspace::new();
    let fixture = Fixture::new(workspace.path());
    let valid = VALID_LIFECYCLE_DOCUMENT;
    let valid_path = fixture.document("valid.md", valid);

    let contexts = RepositoryContexts::new(fixture.snapshot());
    let resolution = contexts.for_document(&valid_path);
    let roots = SchemaRoots::for_document(resolution.context().expect("dmls context")).expect("schema roots");
    for root in roots.searched() {
        assert!(
            !root.path.join("composition.yaml").exists(),
            "no {:?} schema root holds a Claudine file: {}",
            root.kind,
            root.path.display()
        );
    }

    let uri: lsp_types::Uri = uri_of(&valid_path).parse().unwrap();
    let overlay = OverlayState::default()
        .for_document(&uri, valid, &valid_path, &DmlsConfig::default(), &[fixture.repo()], &resolution)
        .expect("overlay");
    let SchemaOutcome::Ready(Some(bundle)) = &overlay.schema else {
        panic!("the trigger yields an effective schema");
    };
    let properties = bundle.effective.json_schema["properties"].as_object().expect("properties");
    for key in LINKING_KEYS.iter().chain(&["initialize", "start", "blocked", "success", "failure", "finalize", "loop"]) {
        assert!(properties.contains_key(*key), "{key} is declared by the payload");
    }
    let dependencies: Vec<PathBuf> =
        bundle.effective.dependencies().iter().map(|path| canonicalize_simplified(path).unwrap()).collect();
    for name in ["composition.yaml", "shared.yaml", "lifecycle-events.yaml", "action.yaml", "linking.yaml"] {
        assert!(dependencies.contains(&fixture.bundle().join(name)), "{name}: {dependencies:?}");
    }
    assert!(dependencies.iter().all(|path| path.starts_with(fixture.bundle())), "{dependencies:?}");
}

/// DMLS reads Claudine's schemas from its registration only: no dependency table of its
/// manifest names the `claudine` crate or a path into the Claudine area.
#[test]
fn dmls_has_no_claudine_dependency() {
    let manifest: toml::Table = toml::from_str(include_str!("../../Cargo.toml")).expect("Cargo.toml");
    let mut tables: Vec<(&str, &toml::Table)> = Vec::new();
    for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
        if let Some(table) = manifest.get(section).and_then(|value| value.as_table()) {
            tables.push((section, table));
        }
    }
    for target in manifest.get("target").and_then(|value| value.as_table()).into_iter().flat_map(|t| t.values()) {
        for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
            if let Some(table) = target.get(section).and_then(|value| value.as_table()) {
                tables.push((section, table));
            }
        }
    }
    assert!(tables.iter().any(|(_, table)| table.contains_key("darkmatter")), "the walk read real dependency tables");
    for (section, table) in tables {
        for (name, spec) in table {
            let package = spec.get("package").and_then(|value| value.as_str()).unwrap_or(name);
            let path = spec.get("path").and_then(|value| value.as_str()).unwrap_or_default();
            assert!(!package.starts_with("claudine"), "{section}.{name}");
            assert!(!path.contains("claudine"), "{section}.{name} = {path}");
        }
    }
}


/// One editor request against a document opened in an editor session.
#[derive(Clone, Copy)]
pub(crate) enum Ask {
    /// Completion labels just past the needle.
    Complete,
    /// Hover Markdown at the start of the needle (empty when there is none).
    Hover,
}

/// Opens each `(name, text, needle, ask)` under the Claudine package in one
/// session, answers it, closes it, and returns the answers in order.
/// Documents named with an `outside/` prefix are written outside the
/// repository, where no registration applies.
pub(crate) fn editor_answers(cases: &[(&str, &str, &str, Ask)]) -> Vec<Vec<String>> {
    let located: Vec<(&str, &str, usize, Ask)> = cases
        .iter()
        .map(|(name, text, needle, ask)| {
            let start = text.find(needle).unwrap_or_else(|| panic!("`{needle}` in {text:?}"));
            let offset = match ask {
                Ask::Complete => start + needle.len(),
                Ask::Hover => start,
            };
            (*name, *text, offset, *ask)
        })
        .collect();
    editor_answers_at(&located)
}

/// [`editor_answers`] with each request at an explicit byte offset, for a
/// document whose request position no unique needle names.
pub(crate) fn editor_answers_at(cases: &[(&str, &str, usize, Ask)]) -> Vec<Vec<String>> {
    let workspace = LspWorkspace::new();
    let fixture = Fixture::new(workspace.path());
    let mut session = LspFixture::start_with_snapshot(&workspace, fixture.snapshot());
    session.initialize(initialize_params(&fixture.repo()));
    let answers = cases
        .iter()
        .map(|&(name, text, offset, ask)| {
            let path = match name.strip_prefix("outside/") {
                Some(outside) => fixture.outside(outside, text),
                None => fixture.document(name, text),
            };
            let uri = uri_of(&path);
            open(&session, &uri, text);
            session.wait_for_diagnostics(&uri);
            let method = match ask {
                Ask::Complete => "textDocument/completion",
                Ask::Hover => "textDocument/hover",
            };
            let before = &text[..offset];
            let line = before.matches('\n').count();
            let character = offset - before.rfind('\n').map_or(0, |index| index + 1);
            let result = session
                .request(
                    method,
                    json!({ "textDocument": { "uri": uri }, "position": { "line": line, "character": character } }),
                )
                .result
                .unwrap_or(Value::Null);
            session.notify("textDocument/didClose", json!({ "textDocument": { "uri": uri } }));
            match ask {
                Ask::Complete => {
                    let items = result.as_array().or_else(|| result["items"].as_array()).cloned().unwrap_or_default();
                    items.iter().map(|item| item["label"].as_str().unwrap_or_default().to_string()).collect()
                }
                Ask::Hover => vec![result["contents"]["value"].as_str().unwrap_or_default().to_string()],
            }
        })
        .collect();
    session.shutdown();
    answers
}

pub(crate) fn has(labels: &[String], label: &str) -> bool {
    labels.iter().any(|candidate| candidate == label)
}

/// Top-level, nested-parameter, lifecycle-item, and loop keys complete from
/// the trigger payloads; the same top-level text outside the repository,
/// where no registration applies, offers none of them.
#[test]
fn trigger_payload_keys_complete_without_a_schema() {
    let found = editor_answers(&[
        ("keys.md", "---\nprompt: hi\nmod:\n---\n", "mod", Ask::Complete),
        ("nested.md", "---\nprompt: hi\nstart:\n  - say:\n      te: null\n---\n", "      te", Ask::Complete),
        ("item.md", "---\nprompt: hi\nstart:\n  - sa\n---\n", "  - sa", Ask::Complete),
        ("loop.md", "---\nprompt: hi\nloop:\n  ma: null\n---\n", "  ma", Ask::Complete),
        ("outside/keys.md", "---\nprompt: hi\nmod:\n---\n", "mod", Ask::Complete),
    ]);
    let [top, nested, item, loop_keys, outside] = &found[..] else { panic!("{found:?}") };

    let mut top_sorted = top.clone();
    top_sorted.sort();
    assert_eq!(top_sorted, ["mode", "model"], "the payload keys starting `mod`");
    assert_eq!(nested, &["text"], "the long-form parameter of `say`");
    assert!(has(item, "say"), "{item:?}");
    assert!(item.iter().all(|label| label.starts_with("sa")), "only the verbs starting `sa`: {item:?}");
    assert!(has(loop_keys, "max"), "{loop_keys:?}");
    assert!(!has(loop_keys, "mode") && !has(loop_keys, "model"), "no top-level keys inside `loop`: {loop_keys:?}");
    assert!(!has(outside, "mode") && !has(outside, "model"), "no Claudine trigger applies outside the repository: {outside:?}");
}

/// An empty lifecycle item offers the action keys and the bare verbs, and an
/// empty enum or boolean value offers exactly its candidates.
#[test]
fn trigger_payload_items_and_values_complete_without_a_schema() {
    let found = editor_answers(&[
        ("union.md", "---\nprompt: hi\nstart:\n  - \n---\n", "  - ", Ask::Complete),
        ("mode.md", "---\nprompt: hi\nmode: \n---\n", "mode: ", Ask::Complete),
        ("interactive.md", "---\nprompt: hi\ninteractive: \n---\n", "interactive: ", Ask::Complete),
    ]);
    let [union, mode, interactive] = &found[..] else { panic!("{found:?}") };

    for label in ["say", "notify", "shell", "set", "when", "stop", "skip", "retry"] {
        assert!(has(union, label), "`{label}` among the item's alternatives: {union:?}");
    }
    assert!(!has(union, "prompt") && !has(union, "mode"), "no top-level keys inside an item: {union:?}");
    assert_eq!(mode, &["append", "replace"]);
    assert_eq!(interactive, &["true", "false"]);
}

/// Hover describes trigger-declared top-level, lifecycle-parameter, loop, and
/// linking properties.
#[test]
fn trigger_payload_properties_hover_without_a_schema() {
    let found = editor_answers(&[
        ("mode.md", "---\nprompt: hi\nmode: append\n---\n", "mode", Ask::Hover),
        ("text.md", "---\nprompt: hi\nstart:\n  - say:\n      text: hi\n---\n", "text", Ask::Hover),
        ("max.md", "---\nprompt: hi\nloop:\n  max: 3\n---\n", "max", Ask::Hover),
        ("turns.md", "---\nprompt: hi\nmax_turns: 3\n---\n", "max_turns", Ask::Hover),
    ]);
    let hovers: Vec<&str> = found.iter().map(|answer| answer[0].as_str()).collect();
    let [mode, text, max, turns] = hovers[..] else { panic!("{hovers:?}") };

    for expected in ["**`mode`**", "Type: **enum**", "_append_, _replace_", "appends to or replaces the provider's own"] {
        assert!(mode.contains(expected), "`{expected}` in {mode:?}");
    }
    // The single-verb item matches the communication category, named by its verbs.
    for expected in ["**`text`**", "Type: **string**", "Required", "matches alternative", "`say`, `speak`, `effect`, `message`, `notify`"] {
        assert!(text.contains(expected), "`{expected}` in {text:?}");
    }
    assert!(!text.contains("`shell`"), "only the communication alternative is named: {text:?}");
    for expected in ["**`max`**", "Type: **number**", "the most iterations to run"] {
        assert!(max.contains(expected), "`{expected}` in {max:?}");
    }
    for expected in ["**`max_turns`**", "Type: **number**", "the most agent turns the resource may take"] {
        assert!(turns.contains(expected), "`{expected}` in {turns:?}");
    }
}

/// An authored `$schema` that redeclares a trigger property wins that property
/// in completion and hover; the trigger's other properties are still offered.
#[test]
fn an_authored_redeclaration_wins_over_the_trigger_per_property() {
    let schema = "$schema:\n  mode: 'enum(fast, slow) -> the authored mode'\n";
    let found = editor_answers(&[
        ("values.md", &format!("---\n{schema}prompt: hi\nmode: \n---\n"), "\nmode: ", Ask::Complete),
        ("hover.md", &format!("---\n{schema}prompt: hi\nmode: fast\n---\n"), "mode: fast", Ask::Hover),
        ("other.md", &format!("---\n{schema}prompt: hi\ninteractive: \n---\n"), "interactive: ", Ask::Complete),
        ("keys.md", &format!("---\n{schema}prompt: hi\nmod:\n---\n"), "\nmod", Ask::Complete),
    ]);
    let [values, hover, other, keys] = &found[..] else { panic!("{found:?}") };

    assert_eq!(values, &["fast", "slow"], "the authored enum, not the trigger's");
    let hover = &hover[0];
    assert!(hover.contains("the authored mode") && hover.contains("_fast_, _slow_"), "{hover:?}");
    assert!(!hover.contains("append"), "the trigger's definition is replaced: {hover:?}");
    assert_eq!(other, &["true", "false"], "a trigger property the author did not redeclare");
    assert!(has(keys, "model"), "trigger keys are still offered: {keys:?}");
}

/// An expression naming an unset property the trigger payload declares is
/// not undeclared, as compose judges it; an undeclared name still is.
#[test]
fn a_trigger_declared_property_is_not_an_undeclared_expression_root() {
    let workspace = LspWorkspace::new();
    let fixture = Fixture::new(workspace.path());
    let text = "---\nprompt: hi\n---\n\n{{ max_turns }} and {{ nowhere_declared }}\n";
    let path = fixture.document("roots.md", text);
    let mut session = LspFixture::start_with_snapshot(&workspace, fixture.snapshot());
    session.initialize(initialize_params(&fixture.repo()));
    let uri = uri_of(&path);
    open(&session, &uri, text);
    let diagnostics = session.wait_for_diagnostics(&uri);
    session.shutdown();

    let undeclared: Vec<&str> = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic["code"] == "dm.expression.undeclared_property")
        .map(|diagnostic| diagnostic["message"].as_str().unwrap_or_default())
        .collect();
    assert_eq!(undeclared.len(), 1, "{undeclared:?}");
    assert!(undeclared[0].contains("nowhere_declared"), "{undeclared:?}");
}

/// Criterion 13, diagnostics: under Darkmatter's baseline (outside the
/// repository that registers Claudine) `composed` is typed with the md profile and `start` is not;
/// a verb only another host provides passes, and a misspelled long-form key of
/// an md verb does not. Under Claudine's trigger `start` is typed and
/// `composed` is an error; the hover that explains it names `start`
/// ([`renamed_events_hover_with_the_claudine_name`]).
#[test]
fn md_and_claudine_type_their_own_event_names() {
    let workspace = LspWorkspace::new();
    let fixture = Fixture::new(workspace.path());
    let outside = |name: &str, text: &str| (fixture.outside(name, text), text.to_string());
    let md_typo = outside("md-typo.md", "---\ncomposed: [{info: {txet: hi}}]\n---\n\nBody\n");
    let md_say = outside("md-say.md", "---\ncomposed: [{say: hi}, {info: hi}]\n---\n\nBody\n");
    let md_start = outside("md-start.md", "---\nstart: [{info: {txet: hi}}]\n---\n\nBody\n");
    let claudine_text = "---\nprompt: hi\ncomposed: [{info: hi}]\nstart: [{info: {txet: hi}}]\n---\n\nBody\n";
    let claudine = fixture.document("renamed.md", claudine_text);

    let found = session_diagnostics(
        &workspace,
        &fixture,
        &[
            (&md_typo.0, &md_typo.1),
            (&md_say.0, &md_say.1),
            (&md_start.0, &md_start.1),
            (&claudine, claudine_text),
        ],
    );
    let [typo, say, start, renamed] = &found[..] else { panic!("{found:?}") };
    assert!(
        typo.iter().any(|(path, message)| path.starts_with("/composed/0") && message.contains("txet")),
        "a misspelled `info` key under `composed` is a schema error: {typo:?}"
    );
    assert!(say.is_empty(), "`say` is untyped under the baseline and `info: hi` is valid: {say:?}");
    assert!(start.is_empty(), "the baseline does not type `start`: {start:?}");
    assert!(
        renamed.iter().any(|(path, _)| path == "/composed"),
        "`composed` is an error under Claudine's schema: {renamed:?}"
    );
    assert!(
        renamed.iter().any(|(path, message)| path.starts_with("/start/0") && message.contains("txet")),
        "`start` is typed under Claudine's schema: {renamed:?}"
    );
}

/// Criterion 13, completion: an md stack item under the baseline offers the
/// md verbs (`info`) and never a verb only another host provides (`say`);
/// `start` offers neither, since the baseline does not type it.
#[test]
fn baseline_stack_items_complete_md_verbs_only() {
    let found = editor_answers(&[
        ("outside/composed.md", "---\ncomposed:\n  - \n---\n", "  - ", Ask::Complete),
        ("outside/start.md", "---\nstart:\n  - \n---\n", "  - ", Ask::Complete),
    ]);
    let [composed, start] = &found[..] else { panic!("{found:?}") };
    for label in ["info", "stdout", "shell", "set", "stop"] {
        assert!(has(composed, label), "`{label}` among `composed`'s alternatives: {composed:?}");
    }
    for label in ["say", "retry", "notify", "proxy"] {
        assert!(!has(composed, label), "`{label}` is not offered under the baseline: {composed:?}");
    }
    assert!(!has(start, "info") && !has(start, "say"), "`start` is untyped under the baseline: {start:?}");
}

/// Criterion 13, guidance: under Claudine's trigger, the declaration that
/// rejects `composed` (and `failed`) names the Claudine event to write
/// instead, and hover shows it.
#[test]
fn renamed_events_hover_with_the_claudine_name() {
    let found = editor_answers(&[
        ("composed.md", "---\nprompt: hi\ncomposed: [{info: hi}]\n---\n", "composed", Ask::Hover),
        ("failed.md", "---\nprompt: hi\nfailed: [{info: hi}]\n---\n", "failed", Ask::Hover),
    ]);
    let [composed, failed] = &found[..] else { panic!("{found:?}") };
    assert!(composed[0].contains("`start`"), "{composed:?}");
    assert!(failed[0].contains("`blocked`"), "{failed:?}");
}

/// An inline document (Claudine's root-union payload, `inline-composition.yaml`)
/// keeps the declarations the agent-only control (`composition.yaml`, an
/// object payload) gets: a key every arm shares hovers with its declaration
/// in either branch, both branch keys complete, and a written branch key
/// leaves that branch's declarations, so its hover is never the other arm's
/// `null`.
#[test]
fn a_registered_root_union_keeps_its_editor_declarations() {
    let found = editor_answers(&[
        ("control.md", "---\nagent: claude\n---\n", "agent", Ask::Hover),
        ("prompt.md", "---\nprompt: hi\nagent: claude\n---\n", "agent", Ask::Hover),
        ("sections.md", "---\nsections:\n  Intro: hi\nagent: claude\n---\n", "agent", Ask::Hover),
        ("prompt-key.md", "---\nprompt: hi\nagent: claude\n---\n", "prompt", Ask::Hover),
        ("sections-key.md", "---\nsections:\n  Intro: hi\n---\n", "sections", Ask::Hover),
        ("branch-keys.md", "---\nprompt: hi\nsec:\npro:\n---\n", "sec", Ask::Complete),
        ("other-branch.md", "---\nsections:\n  Intro: hi\npro:\n---\n", "pro", Ask::Complete),
        ("shared-keys.md", "---\nsections:\n  Intro: hi\nage:\n---\n", "age", Ask::Complete),
        ("shared-value.md", "---\nsections:\n  Intro: hi\nmode: \n---\n", "mode: ", Ask::Complete),
    ]);
    let [control, prompt, sections, prompt_key, sections_key, branch, other, shared, value] = &found[..] else {
        panic!("{found:?}")
    };

    assert!(control[0].contains("the agent (provider) to run"), "agent-only control: {control:?}");
    assert_eq!(prompt, control, "the shared declaration, as the object payload gives it");
    assert_eq!(sections, control, "the shared declaration in the `sections` branch");
    assert!(prompt_key[0].contains("the prompt sent to the agent"), "{prompt_key:?}");
    assert!(!prompt_key[0].contains("has no `prompt`"), "the written branch, not the other arm's null: {prompt_key:?}");
    assert!(sections_key[0].contains("each key is a heading"), "{sections_key:?}");
    assert!(!sections_key[0].contains("has no `sections`"), "{sections_key:?}");
    assert!(has(branch, "sections"), "{branch:?}");
    assert!(has(other, "prompt"), "{other:?}");
    assert!(has(shared, "agent"), "{shared:?}");
    assert_eq!(value, &["append", "replace"]);
}
