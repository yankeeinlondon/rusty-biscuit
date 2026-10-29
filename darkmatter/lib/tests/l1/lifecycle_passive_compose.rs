//! The library compose API is passive about a document's lifecycle: only a
//! host that drives a run (`md compose` through `md_host::drive`) executes a
//! stack. Composing, and the `md` host's own parsing and shell audit, leave
//! the stacks as data.
//!
//! The document's stacks would leave a file each if any of them ran:
//! `initialize` and `finalize` through `ensure_file`, `composed` through a
//! whitelisted `touch`.

use std::path::{Path, PathBuf};

use darkmatter::lifecycle::md_host;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions};
use serde_json::{Map, Value};

const DOCUMENT: &str = "\
---
title: Passive
initialize:
  - ensure_file: [\"sentinel-a.md\", \"initialize ran\"]
composed:
  - shell: \"touch sentinel-b\"
  - stdout: LIFECYCLE-RAN
finalize:
  - ensure_file: [\"sentinel-c.md\", \"finalize ran\"]
---
# Passive body
";

const SENTINELS: [&str; 3] = ["sentinel-a.md", "sentinel-b", "sentinel-c.md"];

/// `DOCUMENT` in its own directory, with its `composed` command whitelisted
/// so nothing but passivity keeps it from running.
fn workspace() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("doc.md");
    std::fs::write(&path, DOCUMENT).unwrap();
    std::fs::write(dir.path().join(".darkmatter-shell-whitelist"), "exact touch sentinel-b\n").unwrap();
    (dir, path)
}

fn created(dir: &Path) -> Vec<&'static str> {
    SENTINELS.into_iter().filter(|name| dir.join(name).exists()).collect()
}

/// A context capturing nothing of the host, anchored in the fixture.
fn options(dir: &Path) -> ComposeOptions {
    ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir, ""))
}

#[test]
fn composing_a_document_with_lifecycle_stacks_runs_none_of_them() {
    let (dir, path) = workspace();
    let md = Markdown::try_from(path.as_path()).expect("document loads");

    let (composed, _report) = md
        .compose_with(&crate::request_support::request(options(dir.path()).with_source_file(path.clone())))
        .expect("the document composes");

    assert!(composed.content().contains("Passive body"), "{}", composed.content());
    assert!(!composed.content().lines().any(|line| line.trim() == "LIFECYCLE-RAN"), "{}", composed.content());
    assert_eq!(created(dir.path()), Vec::<&str>::new());
    // The stacks stay frontmatter data.
    assert!(composed.frontmatter().as_map().contains_key("composed"));
}

#[test]
fn the_md_hosts_parse_and_shell_audit_run_nothing() {
    let (dir, path) = workspace();
    let md = Markdown::try_from(path.as_path()).expect("document loads");
    let frontmatter = Value::Object(md.frontmatter().as_map().iter().map(|(k, v)| (k.clone(), v.clone())).collect::<Map<_, _>>());

    assert!(md_host::authors_lifecycle(&frontmatter));
    let config = md_host::parse_lifecycle(&frontmatter, &path, Some(md.frontmatter())).expect("the lifecycle parses");
    let sources = md_host::lifecycle_shell_sources(&config, &path);

    assert_eq!(sources.entries.iter().map(|entry| entry.normalized.as_str()).collect::<Vec<_>>(), ["touch sentinel-b"]);
    assert_eq!(created(dir.path()), Vec::<&str>::new());
}
