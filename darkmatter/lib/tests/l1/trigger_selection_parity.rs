//! Selection parity for the dialect trigger fixtures under
//! `darkmatter/tests/fixtures/schema-triggers/schemas/`: each trigger file
//! selects exactly the documents it selected before the `schema-trigger`
//! migration (`2026-10-27-darkmatter-lifecycle-ownership`, criterion 18).
//!
//! The expected tables were recorded from the public scan at the revision
//! before the migration (when the files declared `kind: trigger-schema`) and
//! are not edited by it. Every document is judged through
//! [`DarkmatterSchemas::with_trigger_discovery`] and [`evaluate_registry`]
//! over a temporary repository holding the fixture files unchanged.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{RequestSnapshot, build_resolution_context};
use darkmatter::markdown::schemas::DarkmatterSchemas;
use darkmatter::markdown::schemas::triggers::evaluate_registry;
use serde_json::Value;

/// The fixture schema files, verbatim.
const FIXTURE_SCHEMAS: &[(&str, &str)] = &[
    ("claudine.trigger.yaml", include_str!("../../../tests/fixtures/schema-triggers/schemas/claudine.trigger.yaml")),
    ("claudine.yaml", include_str!("../../../tests/fixtures/schema-triggers/schemas/claudine.yaml")),
    (
        "inline-compose.trigger.yaml",
        include_str!("../../../tests/fixtures/schema-triggers/schemas/inline-compose.trigger.yaml"),
    ),
    ("inline-compose.yaml", include_str!("../../../tests/fixtures/schema-triggers/schemas/inline-compose.yaml")),
    ("sequence.trigger.yaml", include_str!("../../../tests/fixtures/schema-triggers/schemas/sequence.trigger.yaml")),
    ("sequence.yaml", include_str!("../../../tests/fixtures/schema-triggers/schemas/sequence.yaml")),
];

/// The fixture documents, verbatim.
const FIXTURE_DOCUMENTS: &[(&str, &str)] = &[
    ("claudine", include_str!("../../../tests/fixtures/schema-triggers/docs/claudine.md")),
    ("inline-compose", include_str!("../../../tests/fixtures/schema-triggers/docs/inline-compose.md")),
    ("plain", include_str!("../../../tests/fixtures/schema-triggers/docs/plain.md")),
    ("sequence", include_str!("../../../tests/fixtures/schema-triggers/docs/sequence.md")),
];

/// A temporary repository whose `schemas/` folder holds `schemas`, with an
/// empty fixture home so no `~/schemas` trigger is discovered.
struct Repository {
    _dir: tempfile::TempDir,
    docs: PathBuf,
    home: PathBuf,
}

impl Repository {
    fn new(schemas: &[(&str, &str)]) -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = dir.path().join("repo");
        let git = root.join(".git");
        std::fs::create_dir_all(git.join("objects")).expect("git objects");
        std::fs::create_dir_all(git.join("refs/heads")).expect("git refs");
        std::fs::write(git.join("HEAD"), "ref: refs/heads/main\n").expect("git HEAD");
        std::fs::write(git.join("config"), "[core]\nrepositoryformatversion = 0\nbare = false\n").expect("git config");
        std::fs::create_dir_all(root.join("schemas")).expect("schemas dir");
        for (name, text) in schemas {
            std::fs::write(root.join("schemas").join(name), text).expect("write schema");
        }
        let docs = root.join("docs");
        std::fs::create_dir_all(&docs).expect("docs dir");
        let home = dir.path().join("home");
        std::fs::create_dir_all(&home).expect("home dir");
        Self { _dir: dir, docs, home }
    }

    /// The trigger file names that select each of `documents`, keyed by label.
    fn selections(&self, documents: &[(String, String)]) -> BTreeMap<String, Vec<String>> {
        let snapshot = RequestSnapshot::new(&self.docs).with_home(Some(self.home.clone()));
        let context = build_resolution_context(&snapshot).expect("fixture context");
        let schemas = DarkmatterSchemas::new(context).with_trigger_discovery().expect("discovery succeeds");
        let registry = schemas.trigger_registry().expect("discovery installed a registry");
        documents
            .iter()
            .map(|(label, text)| {
                let path = self.docs.join(format!("{}.md", label.replace([' ', '=', '[', ']', ':'], "_")));
                std::fs::write(&path, text).expect("write document");
                let selected = evaluate_registry(registry, &frontmatter(&path), Some(&path))
                    .into_iter()
                    .filter(|evaluation| evaluation.matched)
                    .map(|evaluation| evaluation.trigger.source.file_name().expect("file trigger").to_string())
                    .collect();
                (label.clone(), selected)
            })
            .collect()
    }
}

fn frontmatter(path: &Path) -> Value {
    let markdown = Markdown::try_from(path).expect("document loads");
    Value::Object(markdown.frontmatter().as_map().iter().map(|(key, value)| (key.clone(), value.clone())).collect())
}

/// Every combination of the three keys the dialect triggers read, each
/// absent or set to a value of several YAML types.
fn synthetic_documents() -> Vec<(String, String)> {
    let kinds = [None, Some("claudine"), Some("other"), Some("1")];
    let prompts = [None, Some("Explain."), Some("1"), Some("null")];
    let steps = [None, Some("[]"), Some("text"), Some("null")];
    let mut documents = Vec::new();
    for kind in kinds {
        for prompt in prompts {
            for step in steps {
                let mut label = Vec::new();
                let mut frontmatter = String::from("title: synthetic\n");
                for (key, value) in [("kind", kind), ("prompt", prompt), ("steps", step)] {
                    if let Some(value) = value {
                        label.push(format!("{key}={value}"));
                        frontmatter.push_str(&format!("{key}: {value}\n"));
                    }
                }
                let label = if label.is_empty() { "none".to_string() } else { label.join(" ") };
                documents.push((label, format!("---\n{frontmatter}---\nBody.\n")));
            }
        }
    }
    documents
}

/// `(label, selecting trigger files)` recorded before the migration.
const RECORDED_SYNTHETIC: &[(&str, &[&str])] = &[
    ("kind=1", &[]),
    ("kind=1 prompt=1", &[]),
    ("kind=1 prompt=1 steps=[]", &["sequence.trigger.yaml"]),
    ("kind=1 prompt=1 steps=null", &[]),
    ("kind=1 prompt=1 steps=text", &[]),
    ("kind=1 prompt=Explain.", &["inline-compose.trigger.yaml"]),
    ("kind=1 prompt=Explain. steps=[]", &[]),
    ("kind=1 prompt=Explain. steps=null", &["inline-compose.trigger.yaml"]),
    ("kind=1 prompt=Explain. steps=text", &["inline-compose.trigger.yaml"]),
    ("kind=1 prompt=null", &[]),
    ("kind=1 prompt=null steps=[]", &["sequence.trigger.yaml"]),
    ("kind=1 prompt=null steps=null", &[]),
    ("kind=1 prompt=null steps=text", &[]),
    ("kind=1 steps=[]", &["sequence.trigger.yaml"]),
    ("kind=1 steps=null", &[]),
    ("kind=1 steps=text", &[]),
    ("kind=claudine", &["claudine.trigger.yaml"]),
    ("kind=claudine prompt=1", &["claudine.trigger.yaml"]),
    ("kind=claudine prompt=1 steps=[]", &[]),
    ("kind=claudine prompt=1 steps=null", &["claudine.trigger.yaml"]),
    ("kind=claudine prompt=1 steps=text", &["claudine.trigger.yaml"]),
    ("kind=claudine prompt=Explain.", &[]),
    ("kind=claudine prompt=Explain. steps=[]", &[]),
    ("kind=claudine prompt=Explain. steps=null", &[]),
    ("kind=claudine prompt=Explain. steps=text", &[]),
    ("kind=claudine prompt=null", &["claudine.trigger.yaml"]),
    ("kind=claudine prompt=null steps=[]", &[]),
    ("kind=claudine prompt=null steps=null", &["claudine.trigger.yaml"]),
    ("kind=claudine prompt=null steps=text", &["claudine.trigger.yaml"]),
    ("kind=claudine steps=[]", &[]),
    ("kind=claudine steps=null", &["claudine.trigger.yaml"]),
    ("kind=claudine steps=text", &["claudine.trigger.yaml"]),
    ("kind=other", &[]),
    ("kind=other prompt=1", &[]),
    ("kind=other prompt=1 steps=[]", &["sequence.trigger.yaml"]),
    ("kind=other prompt=1 steps=null", &[]),
    ("kind=other prompt=1 steps=text", &[]),
    ("kind=other prompt=Explain.", &["inline-compose.trigger.yaml"]),
    ("kind=other prompt=Explain. steps=[]", &[]),
    ("kind=other prompt=Explain. steps=null", &["inline-compose.trigger.yaml"]),
    ("kind=other prompt=Explain. steps=text", &["inline-compose.trigger.yaml"]),
    ("kind=other prompt=null", &[]),
    ("kind=other prompt=null steps=[]", &["sequence.trigger.yaml"]),
    ("kind=other prompt=null steps=null", &[]),
    ("kind=other prompt=null steps=text", &[]),
    ("kind=other steps=[]", &["sequence.trigger.yaml"]),
    ("kind=other steps=null", &[]),
    ("kind=other steps=text", &[]),
    ("none", &[]),
    ("prompt=1", &[]),
    ("prompt=1 steps=[]", &["sequence.trigger.yaml"]),
    ("prompt=1 steps=null", &[]),
    ("prompt=1 steps=text", &[]),
    ("prompt=Explain.", &["inline-compose.trigger.yaml"]),
    ("prompt=Explain. steps=[]", &[]),
    ("prompt=Explain. steps=null", &["inline-compose.trigger.yaml"]),
    ("prompt=Explain. steps=text", &["inline-compose.trigger.yaml"]),
    ("prompt=null", &[]),
    ("prompt=null steps=[]", &["sequence.trigger.yaml"]),
    ("prompt=null steps=null", &[]),
    ("prompt=null steps=text", &[]),
    ("steps=[]", &["sequence.trigger.yaml"]),
    ("steps=null", &[]),
    ("steps=text", &[]),
];

/// `(fixture document, selecting trigger files)` recorded before the migration.
const RECORDED_FIXTURES: &[(&str, &[&str])] = &[
    ("claudine", &["claudine.trigger.yaml"]),
    ("inline-compose", &["inline-compose.trigger.yaml"]),
    ("plain", &[]),
    ("sequence", &["sequence.trigger.yaml"]),
];

fn recorded(table: &[(&str, &[&str])]) -> BTreeMap<String, Vec<String>> {
    table
        .iter()
        .map(|(label, selected)| (label.to_string(), selected.iter().map(|name| name.to_string()).collect()))
        .collect()
}

fn render(table: &BTreeMap<String, Vec<String>>) -> String {
    table.iter().map(|(label, selected)| format!("    ({label:?}, &{selected:?}),\n")).collect()
}

#[test]
fn dialect_fixture_triggers_select_the_recorded_fixture_documents() {
    let repository = Repository::new(FIXTURE_SCHEMAS);
    let documents: Vec<(String, String)> =
        FIXTURE_DOCUMENTS.iter().map(|(name, text)| (name.to_string(), text.to_string())).collect();
    let actual = repository.selections(&documents);
    assert_eq!(actual, recorded(RECORDED_FIXTURES), "actual table:\n{}", render(&actual));
}

#[test]
fn dialect_fixture_triggers_select_the_recorded_synthetic_documents() {
    let repository = Repository::new(FIXTURE_SCHEMAS);
    let actual = repository.selections(&synthetic_documents());
    assert_eq!(actual, recorded(RECORDED_SYNTHETIC), "actual table:\n{}", render(&actual));
}
