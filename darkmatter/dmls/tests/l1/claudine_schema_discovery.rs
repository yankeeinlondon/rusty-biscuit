//! DMLS applies Claudine's schemas by trigger discovery alone.
//!
//! A fixture repository holds package `claudine/lib` in package area
//! `claudine`, with a complete copy of Claudine's five schema files in the
//! area's `schemas/` folder. A document under the package that declares no
//! `$schema` is typed by the `claudine.yaml` trigger's payload, read from disk
//! through the document's schema roots. DMLS has no Claudine dependency, so
//! nothing else could supply it.

use crate::common;

use std::path::{Path, PathBuf};

use biscuit_file::canonicalize_simplified;
use common::{LspFixture, LspWorkspace};
use darkmatter::markdown::compose::RequestSnapshot;
use darkmatter::markdown::schemas::{SchemaRootKind, SchemaRoots};
use dmls::context::RepositoryContexts;
use dmls::overlay::SchemaOutcome;
use dmls::{DmlsConfig, OverlayState};
use serde_json::{Value, json};

/// Claudine's complete schema bundle, copied byte for byte into the fixture.
const BUNDLE: [(&str, &str); 5] = [
    ("action.yaml", include_str!("../../../../claudine/schemas/action.yaml")),
    ("claudine.yaml", include_str!("../../../../claudine/schemas/claudine.yaml")),
    ("inline-compose.yaml", include_str!("../../../../claudine/schemas/inline-compose.yaml")),
    ("lifecycle.yaml", include_str!("../../../../claudine/schemas/lifecycle.yaml")),
    ("linking.yaml", include_str!("../../../../claudine/schemas/linking.yaml")),
];

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

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    /// A repository (a `.git` skeleton Git discovery accepts, without running
    /// `git`) whose `claudine/schemas/` holds the bundle, and an empty home.
    fn new(workspace: &Path) -> Self {
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
        for (name, text) in BUNDLE {
            write(&fixture.bundle().join(name), text);
        }
        std::fs::create_dir_all(fixture.home()).unwrap();
        fixture
    }

    fn repo(&self) -> PathBuf {
        self.root.join("repo")
    }

    fn bundle(&self) -> PathBuf {
        self.repo().join("claudine/schemas")
    }

    fn home(&self) -> PathBuf {
        self.root.join("home")
    }

    /// The server's snapshot: fixture home, no environment.
    fn snapshot(&self) -> RequestSnapshot {
        RequestSnapshot::new(&self.root).with_home(Some(self.home()))
    }

    /// Writes `text` to `claudine/lib/tasks/{name}` and returns its path.
    fn document(&self, name: &str, text: &str) -> PathBuf {
        let path = self.repo().join("claudine/lib/tasks").join(name);
        write(&path, text);
        path
    }
}

fn initialize_params(folder: &Path) -> Value {
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

fn uri_of(path: &Path) -> String {
    url::Url::from_file_path(path).unwrap().to_string()
}

fn open(session: &LspFixture<'_>, uri: &str, text: &str) {
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
/// the Claudine area, where no root holds the bundle, is not.
#[test]
fn a_misspelled_long_form_key_is_reported_under_start_without_a_schema_pointer() {
    let workspace = LspWorkspace::new();
    let fixture = Fixture::new(workspace.path());
    let text = "---\nstart: [{say: {txet: hi}}]\n---\n\nBody\n";
    let inside = fixture.document("typo.md", text);
    let outside = fixture.repo().join("docs/typo.md");
    write(&outside, text);

    let found = session_diagnostics(&workspace, &fixture, &[(&inside, text), (&outside, text)]);
    let (inside_found, outside_found) = (&found[0], &found[1]);
    assert!(
        inside_found.iter().any(|(path, message)| path.starts_with("/start/0/say") && message.contains("txet")),
        "expected a diagnostic at /start/0/say naming txet: {inside_found:?}"
    );
    assert!(inside_found.iter().all(|(path, _)| path.starts_with("/start/")), "{inside_found:?}");
    assert!(outside_found.is_empty(), "no Claudine trigger applies outside the area: {outside_found:?}");
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

/// The payload that types a document under the Claudine area is the copied
/// bundle: it comes from the package-area root, declares the events and
/// linking keys, and its files are the effective schema's dependencies.
#[test]
fn the_effective_schema_under_the_claudine_area_is_the_disk_bundle() {
    let workspace = LspWorkspace::new();
    let fixture = Fixture::new(workspace.path());
    let valid = VALID_LIFECYCLE_DOCUMENT;
    let valid_path = fixture.document("valid.md", valid);

    let contexts = RepositoryContexts::new(fixture.snapshot());
    let resolution = contexts.for_document(&valid_path);
    let roots = SchemaRoots::for_document(resolution.context().expect("dmls context")).expect("schema roots");
    let area_root = roots
        .searched()
        .iter()
        .find(|root| root.kind == SchemaRootKind::PackageArea)
        .map(|root| canonicalize_simplified(&root.path).unwrap());
    assert_eq!(area_root, Some(fixture.bundle()));

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
    for name in ["claudine.yaml", "lifecycle.yaml", "action.yaml", "linking.yaml"] {
        assert!(dependencies.contains(&fixture.bundle().join(name)), "{name}: {dependencies:?}");
    }
    assert!(dependencies.iter().all(|path| path.starts_with(fixture.bundle())), "{dependencies:?}");
}

/// DMLS reads Claudine's schemas from disk only: no dependency table of its
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
