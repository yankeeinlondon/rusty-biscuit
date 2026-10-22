//! Selection parity for Claudine's shipped trigger files (`claudine.yaml`,
//! `inline-compose.yaml`, `sequence.yaml` under `claudine/schemas/`): each
//! selects exactly the documents it selected before the `schema-trigger`
//! migration (`2026-10-27-darkmatter-lifecycle-ownership`, criterion 18).
//!
//! The expected table was recorded from the public scan at the revision
//! before the migration (when the files declared `kind: trigger-schema` and
//! used `any:`) and is not edited by it. The bundle is the one Claudine ships,
//! registered as the last-resort location of a temporary repository whose
//! `schemas/` folder is empty, and each document is judged through
//! [`evaluate_registry`].

use std::collections::BTreeMap;

use claudine::composition::schema::bundle::claudine_schema_bundle;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{RequestSnapshot, build_resolution_context};
use darkmatter::markdown::schemas::DarkmatterSchemas;
use darkmatter::markdown::schemas::triggers::evaluate_registry;
use serde_json::Value;

/// Every top-level key Claudine's triggers read, plus one they do not.
const KEYS: &[&str] = &[
    "initialize",
    "start",
    "blocked",
    "success",
    "failure",
    "cancelled",
    "finalize",
    "loop",
    "prompt",
    "agent",
    "model",
    "sequence",
    "title",
];

/// One value of each YAML type, plus null.
const VALUES: &[&str] = &["text", "1", "true", "{ a: 1 }", "[1]", "null"];

/// Every key alone with every value, then `prompt` beside `sequence` with
/// every pair of values, then a document with no frontmatter keys.
fn documents() -> Vec<(String, String)> {
    let mut documents = Vec::new();
    for key in KEYS {
        for value in VALUES {
            documents.push((format!("{key}={value}"), format!("{key}: {value}\n")));
        }
    }
    for prompt in VALUES {
        for sequence in VALUES {
            documents.push((
                format!("prompt={prompt} sequence={sequence}"),
                format!("prompt: {prompt}\nsequence: {sequence}\n"),
            ));
        }
    }
    documents.push(("empty".to_string(), String::new()));
    documents
}

/// The Claudine trigger files that select each of `documents`, keyed by label.
fn selections(documents: &[(String, String)]) -> BTreeMap<String, Vec<String>> {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = dir.path().join("repo");
    let git = root.join(".git");
    std::fs::create_dir_all(git.join("objects")).expect("git objects");
    std::fs::create_dir_all(git.join("refs/heads")).expect("git refs");
    std::fs::write(git.join("HEAD"), "ref: refs/heads/main\n").expect("git HEAD");
    std::fs::write(git.join("config"), "[core]\nrepositoryformatversion = 0\nbare = false\n").expect("git config");
    std::fs::create_dir_all(root.join("schemas")).expect("schemas dir");
    let docs = root.join("docs");
    std::fs::create_dir_all(&docs).expect("docs dir");
    let home = dir.path().join("home");
    std::fs::create_dir_all(&home).expect("home dir");

    let snapshot = RequestSnapshot::new(&docs).with_home(Some(home));
    let context = build_resolution_context(&snapshot).expect("fixture context");
    let schemas = DarkmatterSchemas::new(context)
        .with_embedded_trigger_bundle(claudine_schema_bundle())
        .expect("bundle registers")
        .with_trigger_discovery()
        .expect("discovery succeeds");
    let registry = schemas.trigger_registry().expect("discovery installed a registry");
    documents
        .iter()
        .enumerate()
        .map(|(index, (label, frontmatter))| {
            let path = docs.join(format!("document-{index}.md"));
            std::fs::write(&path, format!("---\n{frontmatter}---\nBody.\n")).expect("write document");
            let markdown = Markdown::try_from(path.as_path()).expect("document loads");
            let frontmatter =
                Value::Object(markdown.frontmatter().as_map().iter().map(|(k, v)| (k.clone(), v.clone())).collect());
            let selected = evaluate_registry(registry, &frontmatter, Some(&path))
                .into_iter()
                .filter(|evaluation| evaluation.matched)
                .map(|evaluation| evaluation.trigger.source.file_name().expect("named trigger").to_string())
                .collect();
            (label.clone(), selected)
        })
        .collect()
}

/// `(label, selecting trigger files)` recorded before the migration.
const RECORDED: &[(&str, &[&str])] = &[
    ("agent=1", &["claudine.yaml"]),
    ("agent=[1]", &["claudine.yaml"]),
    ("agent=null", &[]),
    ("agent=text", &["claudine.yaml"]),
    ("agent=true", &["claudine.yaml"]),
    ("agent={ a: 1 }", &["claudine.yaml"]),
    ("blocked=1", &["claudine.yaml"]),
    ("blocked=[1]", &["claudine.yaml"]),
    ("blocked=null", &[]),
    ("blocked=text", &["claudine.yaml"]),
    ("blocked=true", &["claudine.yaml"]),
    ("blocked={ a: 1 }", &["claudine.yaml"]),
    ("cancelled=1", &["claudine.yaml"]),
    ("cancelled=[1]", &["claudine.yaml"]),
    ("cancelled=null", &[]),
    ("cancelled=text", &["claudine.yaml"]),
    ("cancelled=true", &["claudine.yaml"]),
    ("cancelled={ a: 1 }", &["claudine.yaml"]),
    ("empty", &[]),
    ("failure=1", &["claudine.yaml"]),
    ("failure=[1]", &["claudine.yaml"]),
    ("failure=null", &[]),
    ("failure=text", &["claudine.yaml"]),
    ("failure=true", &["claudine.yaml"]),
    ("failure={ a: 1 }", &["claudine.yaml"]),
    ("finalize=1", &["claudine.yaml"]),
    ("finalize=[1]", &["claudine.yaml"]),
    ("finalize=null", &[]),
    ("finalize=text", &["claudine.yaml"]),
    ("finalize=true", &["claudine.yaml"]),
    ("finalize={ a: 1 }", &["claudine.yaml"]),
    ("initialize=1", &["claudine.yaml"]),
    ("initialize=[1]", &["claudine.yaml"]),
    ("initialize=null", &[]),
    ("initialize=text", &["claudine.yaml"]),
    ("initialize=true", &["claudine.yaml"]),
    ("initialize={ a: 1 }", &["claudine.yaml"]),
    ("loop=1", &["claudine.yaml"]),
    ("loop=[1]", &["claudine.yaml"]),
    ("loop=null", &[]),
    ("loop=text", &["claudine.yaml"]),
    ("loop=true", &["claudine.yaml"]),
    ("loop={ a: 1 }", &["claudine.yaml"]),
    ("model=1", &["claudine.yaml"]),
    ("model=[1]", &["claudine.yaml"]),
    ("model=null", &[]),
    ("model=text", &["claudine.yaml"]),
    ("model=true", &["claudine.yaml"]),
    ("model={ a: 1 }", &["claudine.yaml"]),
    ("prompt=1", &["claudine.yaml", "inline-compose.yaml"]),
    ("prompt=1 sequence=1", &["claudine.yaml", "sequence.yaml"]),
    ("prompt=1 sequence=[1]", &["claudine.yaml", "sequence.yaml"]),
    ("prompt=1 sequence=null", &["claudine.yaml", "inline-compose.yaml"]),
    ("prompt=1 sequence=text", &["claudine.yaml", "sequence.yaml"]),
    ("prompt=1 sequence=true", &["claudine.yaml", "sequence.yaml"]),
    ("prompt=1 sequence={ a: 1 }", &["claudine.yaml", "sequence.yaml"]),
    ("prompt=[1]", &["claudine.yaml", "inline-compose.yaml"]),
    ("prompt=[1] sequence=1", &["claudine.yaml", "sequence.yaml"]),
    ("prompt=[1] sequence=[1]", &["claudine.yaml", "sequence.yaml"]),
    ("prompt=[1] sequence=null", &["claudine.yaml", "inline-compose.yaml"]),
    ("prompt=[1] sequence=text", &["claudine.yaml", "sequence.yaml"]),
    ("prompt=[1] sequence=true", &["claudine.yaml", "sequence.yaml"]),
    ("prompt=[1] sequence={ a: 1 }", &["claudine.yaml", "sequence.yaml"]),
    ("prompt=null", &[]),
    ("prompt=null sequence=1", &["sequence.yaml"]),
    ("prompt=null sequence=[1]", &["sequence.yaml"]),
    ("prompt=null sequence=null", &[]),
    ("prompt=null sequence=text", &["sequence.yaml"]),
    ("prompt=null sequence=true", &["sequence.yaml"]),
    ("prompt=null sequence={ a: 1 }", &["sequence.yaml"]),
    ("prompt=text", &["claudine.yaml", "inline-compose.yaml"]),
    ("prompt=text sequence=1", &["claudine.yaml", "sequence.yaml"]),
    ("prompt=text sequence=[1]", &["claudine.yaml", "sequence.yaml"]),
    ("prompt=text sequence=null", &["claudine.yaml", "inline-compose.yaml"]),
    ("prompt=text sequence=text", &["claudine.yaml", "sequence.yaml"]),
    ("prompt=text sequence=true", &["claudine.yaml", "sequence.yaml"]),
    ("prompt=text sequence={ a: 1 }", &["claudine.yaml", "sequence.yaml"]),
    ("prompt=true", &["claudine.yaml", "inline-compose.yaml"]),
    ("prompt=true sequence=1", &["claudine.yaml", "sequence.yaml"]),
    ("prompt=true sequence=[1]", &["claudine.yaml", "sequence.yaml"]),
    ("prompt=true sequence=null", &["claudine.yaml", "inline-compose.yaml"]),
    ("prompt=true sequence=text", &["claudine.yaml", "sequence.yaml"]),
    ("prompt=true sequence=true", &["claudine.yaml", "sequence.yaml"]),
    ("prompt=true sequence={ a: 1 }", &["claudine.yaml", "sequence.yaml"]),
    ("prompt={ a: 1 }", &["claudine.yaml", "inline-compose.yaml"]),
    ("prompt={ a: 1 } sequence=1", &["claudine.yaml", "sequence.yaml"]),
    ("prompt={ a: 1 } sequence=[1]", &["claudine.yaml", "sequence.yaml"]),
    ("prompt={ a: 1 } sequence=null", &["claudine.yaml", "inline-compose.yaml"]),
    ("prompt={ a: 1 } sequence=text", &["claudine.yaml", "sequence.yaml"]),
    ("prompt={ a: 1 } sequence=true", &["claudine.yaml", "sequence.yaml"]),
    ("prompt={ a: 1 } sequence={ a: 1 }", &["claudine.yaml", "sequence.yaml"]),
    ("sequence=1", &["sequence.yaml"]),
    ("sequence=[1]", &["sequence.yaml"]),
    ("sequence=null", &[]),
    ("sequence=text", &["sequence.yaml"]),
    ("sequence=true", &["sequence.yaml"]),
    ("sequence={ a: 1 }", &["sequence.yaml"]),
    ("start=1", &["claudine.yaml"]),
    ("start=[1]", &["claudine.yaml"]),
    ("start=null", &[]),
    ("start=text", &["claudine.yaml"]),
    ("start=true", &["claudine.yaml"]),
    ("start={ a: 1 }", &["claudine.yaml"]),
    ("success=1", &["claudine.yaml"]),
    ("success=[1]", &["claudine.yaml"]),
    ("success=null", &[]),
    ("success=text", &["claudine.yaml"]),
    ("success=true", &["claudine.yaml"]),
    ("success={ a: 1 }", &["claudine.yaml"]),
    ("title=1", &[]),
    ("title=[1]", &[]),
    ("title=null", &[]),
    ("title=text", &[]),
    ("title=true", &[]),
    ("title={ a: 1 }", &[]),
];

#[test]
fn shipped_claudine_triggers_select_the_recorded_documents() {
    let actual = selections(&documents());
    let expected: BTreeMap<String, Vec<String>> = RECORDED
        .iter()
        .map(|(label, selected)| (label.to_string(), selected.iter().map(|name| name.to_string()).collect()))
        .collect();
    let rendered: String =
        actual.iter().map(|(label, selected)| format!("    ({label:?}, &{selected:?}),\n")).collect();
    assert_eq!(actual, expected, "actual table:\n{rendered}");
}
