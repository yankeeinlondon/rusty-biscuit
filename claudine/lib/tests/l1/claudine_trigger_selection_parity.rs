//! Selection parity for Claudine's shipped trigger files (`composition.yaml`,
//! `inline-composition.yaml`, `sequence-document.yaml` under
//! `claudine/registration/`): each selects exactly the documents it selected
//! before the `schema-trigger` migration
//! (`2026-10-27-darkmatter-lifecycle-ownership`, criterion 18).
//!
//! The expected table was recorded from the public scan at the revision
//! before the migration (when the files declared `kind: trigger-schema` and
//! used `any:`) and is not edited by it, with three later, deliberate changes.
//! `initialize`, `cancelled`, and `finalize` are events Claudine shares with
//! Darkmatter's `md` host, so they no longer select `composition.yaml` on their
//! own and their rows select nothing. A non-null `sequence` identifies a
//! Claudine document, so it now selects `composition.yaml` beside
//! `sequence-document.yaml`. A document `inline-composition.yaml` selects is no
//! longer also selected by `composition.yaml`, so exactly one of the two applies.
//! The bundle is the registration compiled into Claudine, loaded as its
//! registered host for a temporary repository whose `schemas/` folder is
//! empty, and each document is judged by registered host selection
//! ([`DarkmatterSchemas::registered_selection_for`]), which matches every
//! trigger of the host.

use std::collections::BTreeMap;

use claudine::composition::schema::bundle::with_claudine_registration;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{RequestSnapshot, build_resolution_context};
use darkmatter::markdown::schemas::DarkmatterSchemas;

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
    let schemas = with_claudine_registration(DarkmatterSchemas::new(context))
        .with_trigger_discovery()
        .expect("discovery succeeds");
    documents
        .iter()
        .enumerate()
        .map(|(index, (label, frontmatter))| {
            let path = docs.join(format!("document-{index}.md"));
            std::fs::write(&path, format!("---\n{frontmatter}---\nBody.\n")).expect("write document");
            let markdown = Markdown::try_from(path.as_path()).expect("document loads");
            let selected = schemas
                .registered_selection_for(&markdown)
                .matched_triggers()
                .map(|trigger| trigger.bundle_path.rsplit('/').next().expect("a file name").to_string())
                .collect();
            (label.clone(), selected)
        })
        .collect()
}

/// `(label, selecting trigger files)` recorded before the migration.
const RECORDED: &[(&str, &[&str])] = &[
    ("agent=1", &["composition.yaml"]),
    ("agent=[1]", &["composition.yaml"]),
    ("agent=null", &[]),
    ("agent=text", &["composition.yaml"]),
    ("agent=true", &["composition.yaml"]),
    ("agent={ a: 1 }", &["composition.yaml"]),
    ("blocked=1", &["composition.yaml"]),
    ("blocked=[1]", &["composition.yaml"]),
    ("blocked=null", &[]),
    ("blocked=text", &["composition.yaml"]),
    ("blocked=true", &["composition.yaml"]),
    ("blocked={ a: 1 }", &["composition.yaml"]),
    ("cancelled=1", &[]),
    ("cancelled=[1]", &[]),
    ("cancelled=null", &[]),
    ("cancelled=text", &[]),
    ("cancelled=true", &[]),
    ("cancelled={ a: 1 }", &[]),
    ("empty", &[]),
    ("failure=1", &["composition.yaml"]),
    ("failure=[1]", &["composition.yaml"]),
    ("failure=null", &[]),
    ("failure=text", &["composition.yaml"]),
    ("failure=true", &["composition.yaml"]),
    ("failure={ a: 1 }", &["composition.yaml"]),
    ("finalize=1", &[]),
    ("finalize=[1]", &[]),
    ("finalize=null", &[]),
    ("finalize=text", &[]),
    ("finalize=true", &[]),
    ("finalize={ a: 1 }", &[]),
    ("initialize=1", &[]),
    ("initialize=[1]", &[]),
    ("initialize=null", &[]),
    ("initialize=text", &[]),
    ("initialize=true", &[]),
    ("initialize={ a: 1 }", &[]),
    ("loop=1", &["composition.yaml"]),
    ("loop=[1]", &["composition.yaml"]),
    ("loop=null", &[]),
    ("loop=text", &["composition.yaml"]),
    ("loop=true", &["composition.yaml"]),
    ("loop={ a: 1 }", &["composition.yaml"]),
    ("model=1", &["composition.yaml"]),
    ("model=[1]", &["composition.yaml"]),
    ("model=null", &[]),
    ("model=text", &["composition.yaml"]),
    ("model=true", &["composition.yaml"]),
    ("model={ a: 1 }", &["composition.yaml"]),
    ("prompt=1", &["inline-composition.yaml"]),
    ("prompt=1 sequence=1", &["composition.yaml", "sequence-document.yaml"]),
    ("prompt=1 sequence=[1]", &["composition.yaml", "sequence-document.yaml"]),
    ("prompt=1 sequence=null", &["inline-composition.yaml"]),
    ("prompt=1 sequence=text", &["composition.yaml", "sequence-document.yaml"]),
    ("prompt=1 sequence=true", &["composition.yaml", "sequence-document.yaml"]),
    ("prompt=1 sequence={ a: 1 }", &["composition.yaml", "sequence-document.yaml"]),
    ("prompt=[1]", &["inline-composition.yaml"]),
    ("prompt=[1] sequence=1", &["composition.yaml", "sequence-document.yaml"]),
    ("prompt=[1] sequence=[1]", &["composition.yaml", "sequence-document.yaml"]),
    ("prompt=[1] sequence=null", &["inline-composition.yaml"]),
    ("prompt=[1] sequence=text", &["composition.yaml", "sequence-document.yaml"]),
    ("prompt=[1] sequence=true", &["composition.yaml", "sequence-document.yaml"]),
    ("prompt=[1] sequence={ a: 1 }", &["composition.yaml", "sequence-document.yaml"]),
    ("prompt=null", &[]),
    ("prompt=null sequence=1", &["composition.yaml", "sequence-document.yaml"]),
    ("prompt=null sequence=[1]", &["composition.yaml", "sequence-document.yaml"]),
    ("prompt=null sequence=null", &[]),
    ("prompt=null sequence=text", &["composition.yaml", "sequence-document.yaml"]),
    ("prompt=null sequence=true", &["composition.yaml", "sequence-document.yaml"]),
    ("prompt=null sequence={ a: 1 }", &["composition.yaml", "sequence-document.yaml"]),
    ("prompt=text", &["inline-composition.yaml"]),
    ("prompt=text sequence=1", &["composition.yaml", "sequence-document.yaml"]),
    ("prompt=text sequence=[1]", &["composition.yaml", "sequence-document.yaml"]),
    ("prompt=text sequence=null", &["inline-composition.yaml"]),
    ("prompt=text sequence=text", &["composition.yaml", "sequence-document.yaml"]),
    ("prompt=text sequence=true", &["composition.yaml", "sequence-document.yaml"]),
    ("prompt=text sequence={ a: 1 }", &["composition.yaml", "sequence-document.yaml"]),
    ("prompt=true", &["inline-composition.yaml"]),
    ("prompt=true sequence=1", &["composition.yaml", "sequence-document.yaml"]),
    ("prompt=true sequence=[1]", &["composition.yaml", "sequence-document.yaml"]),
    ("prompt=true sequence=null", &["inline-composition.yaml"]),
    ("prompt=true sequence=text", &["composition.yaml", "sequence-document.yaml"]),
    ("prompt=true sequence=true", &["composition.yaml", "sequence-document.yaml"]),
    ("prompt=true sequence={ a: 1 }", &["composition.yaml", "sequence-document.yaml"]),
    ("prompt={ a: 1 }", &["inline-composition.yaml"]),
    ("prompt={ a: 1 } sequence=1", &["composition.yaml", "sequence-document.yaml"]),
    ("prompt={ a: 1 } sequence=[1]", &["composition.yaml", "sequence-document.yaml"]),
    ("prompt={ a: 1 } sequence=null", &["inline-composition.yaml"]),
    ("prompt={ a: 1 } sequence=text", &["composition.yaml", "sequence-document.yaml"]),
    ("prompt={ a: 1 } sequence=true", &["composition.yaml", "sequence-document.yaml"]),
    ("prompt={ a: 1 } sequence={ a: 1 }", &["composition.yaml", "sequence-document.yaml"]),
    ("sequence=1", &["composition.yaml", "sequence-document.yaml"]),
    ("sequence=[1]", &["composition.yaml", "sequence-document.yaml"]),
    ("sequence=null", &[]),
    ("sequence=text", &["composition.yaml", "sequence-document.yaml"]),
    ("sequence=true", &["composition.yaml", "sequence-document.yaml"]),
    ("sequence={ a: 1 }", &["composition.yaml", "sequence-document.yaml"]),
    ("start=1", &["composition.yaml"]),
    ("start=[1]", &["composition.yaml"]),
    ("start=null", &[]),
    ("start=text", &["composition.yaml"]),
    ("start=true", &["composition.yaml"]),
    ("start={ a: 1 }", &["composition.yaml"]),
    ("success=1", &["composition.yaml"]),
    ("success=[1]", &["composition.yaml"]),
    ("success=null", &[]),
    ("success=text", &["composition.yaml"]),
    ("success=true", &["composition.yaml"]),
    ("success={ a: 1 }", &["composition.yaml"]),
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
