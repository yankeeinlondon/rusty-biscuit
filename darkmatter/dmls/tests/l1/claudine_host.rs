//! A format 2 bundle of a Claudine-shaped host for editor tests and the A3
//! benchmark, registered explicitly: the frozen Claudine lifecycle artifact
//! (its events, renames, verbs, and effects) re-addressed to one namespace
//! file per global of the frozen Claudine host catalog (`seq`, `loop`,
//! `group`, with the catalog's members and descriptions), and one trigger.
//! Nothing is discovered from a schema root.
//!
//! Self-contained apart from the shared host fixtures (`super::host_fixtures`),
//! so the A3 benchmark includes it by `#[path]` beside them.

#![allow(dead_code)]

use std::path::Path;

use serde_json::Value;

use super::host_fixtures::{Bundle, catalog_hash_of};

/// The frozen Claudine host catalog: a `kind: schema` file whose `$schema`
/// declares each global.
const CATALOG: &str = include_str!("../../../lib/benches/fixtures/claudine-schemas/globals.yaml");

/// The frozen Claudine lifecycle artifact (written for `CATALOG` by an
/// earlier artifact format; [`artifact_for`] re-addresses it).
pub(crate) const ARTIFACT: &str = include_str!("../../../lib/benches/fixtures/pipeline-work/globals.lifecycle.json");

/// The frontmatter line [`TRIGGER`] matches.
pub(crate) const MARKER: &str = "run_as: claudine\n";

/// The editor tests' trigger: it matches a document with `run_as` and types
/// nothing else.
pub(crate) const TRIGGER: &str = "\
kind: schema-trigger
match:
    run_as: string(required)
$schema:
    run_as: \"string -> marks a document of the fixture host\"
";

/// The trigger of the A3 benchmark document, which it matches by a boolean
/// or string `yolo` (the faulted A3 document writes `yolo: maybe`) and
/// types nothing: the document's types come from Claudine's schema files in
/// its package area.
pub(crate) const A3_TRIGGER: &str = "\
kind: schema-trigger
match:
    group:
        - yolo: boolean(required)
        - yolo: string(required)
";

/// The bundle path of the lifecycle artifact.
const LIFECYCLE: &str = "schemas/globals.lifecycle.json";

/// The namespace file of global `name`: the member lines of its entry in
/// [`CATALOG`], under `$schema`.
pub(crate) fn namespace(name: &str) -> String {
    let mut text = format!("kind: namespace-schema\nnamespace: {name}\n$schema:\n");
    let mut inside = false;
    for line in CATALOG.lines().skip_while(|line| *line != "$schema:").skip(1) {
        if let Some(key) = line.strip_prefix("    ").filter(|rest| !rest.starts_with(' ')) {
            inside = key.trim_end_matches(':') == name;
            continue;
        }
        if inside {
            text.push_str(&line[4..]);
            text.push('\n');
        }
    }
    assert!(text.lines().count() > 3, "`{name}` is a global of the frozen catalog");
    text
}

/// The namespace files by bundle path, with `seq` as the `seq` file.
pub(crate) fn namespaces(seq: &str) -> Vec<(String, String)> {
    vec![
        ("schemas/group.yaml".to_string(), namespace("group")),
        ("schemas/loop.yaml".to_string(), namespace("loop")),
        ("schemas/seq.yaml".to_string(), seq.to_string()),
    ]
}

/// `artifact` in the current artifact format, addressed to `namespaces`.
pub(crate) fn artifact_for(artifact: &str, namespaces: &[(String, String)]) -> String {
    let pairs: Vec<(&str, &str)> = namespaces.iter().map(|(path, text)| (path.as_str(), text.as_str())).collect();
    let mut root: Value = serde_json::from_str(artifact).expect("the artifact is JSON");
    root["catalog_hash"] = Value::String(catalog_hash_of(&pairs));
    root["format_version"] = Value::from(2);
    serde_json::to_string_pretty(&root).expect("the artifact serializes")
}

/// The valid bundle with [`TRIGGER`].
pub(crate) fn bundle() -> Bundle {
    bundle_with(TRIGGER, &namespace("seq"))
}

/// The valid bundle with [`A3_TRIGGER`].
pub(crate) fn a3_bundle() -> Bundle {
    bundle_with(A3_TRIGGER, &namespace("seq"))
}

/// [`bundle`] with `seq` as the `seq` namespace file and the artifact
/// re-addressed to it: a valid republish of an edited namespace.
pub(crate) fn bundle_with_seq(seq: &str) -> Bundle {
    bundle_with(TRIGGER, seq)
}

/// The valid bundle with `trigger` as its one trigger and `seq` as the
/// `seq` namespace file.
pub(crate) fn bundle_with(trigger: &str, seq: &str) -> Bundle {
    let mut bundle = Bundle {
        host: "claudine".to_string(),
        version: "0.0.1".to_string(),
        files: Vec::new(),
        lifecycle: LIFECYCLE.to_string(),
        functions: Vec::new(),
        members: Default::default(),
    };
    bundle = bundle.with_file("schemas/claudine.yaml", trigger);
    let namespaces = namespaces(seq);
    for (path, text) in &namespaces {
        bundle = bundle.with_file(path, text);
    }
    bundle.with_member(LIFECYCLE, &artifact_for(ARTIFACT, &namespaces))
}

/// A repository workspace at `root` whose home (`root/home`) registers
/// `bundle` ([`register`]).
pub(crate) fn install(root: &Path, bundle: &Bundle) {
    for dir in [".git/objects", ".git/refs/heads", "home"] {
        std::fs::create_dir_all(root.join(dir)).unwrap();
    }
    std::fs::write(root.join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
    register(&root.join("home"), bundle);
}

/// Installs `bundle` under `home/.claudine/` and registers it in the user
/// configuration `home/.darkmatter.yaml`.
pub(crate) fn register(home: &Path, bundle: &Bundle) {
    bundle.write_to(&home.join(".claudine"));
    std::fs::write(home.join(".darkmatter.yaml"), "register:\n  - ./.claudine/darkmatter-host.yaml\n").unwrap();
}

/// `text` with [`MARKER`] as its first frontmatter line.
pub(crate) fn marked(text: &str) -> String {
    let body = text.strip_prefix("---\n").expect("the document opens with frontmatter");
    format!("---\n{MARKER}{body}")
}
