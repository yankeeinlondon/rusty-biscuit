//! Acceptance 12: the production in-memory full build (every listed file
//! parsed into one resident `WorkspaceGraph` for references, rename, file
//! rename, and workspace symbols) no longer exists. No production source
//! file names its API, its state, its log line, or its progress token, in
//! code or in a comment; the only graph over every listed file is the
//! test-only oracle.

use std::path::{Path, PathBuf};

/// The removed API and its traces.
const REMOVED: &[&str] = &[
    "from_indices",
    "begin_full_build",
    "finish_full_build",
    "abandon_full_build",
    "FullBuildStep",
    "unparsed_paths",
    ".is_full()",
    "index_is_full",
    "index_workspace",
    "collect_indices",
    "parse_full_index_round",
    "FullIndexRound",
    "FullIndexState",
    "bridge_to_index",
    "full_index",
    "full index",
    "full-index",
];

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn no_production_source_names_the_removed_full_build() {
    let src = biscuit_test_harness::manifest_dir!().join("src");
    let mut files = Vec::new();
    rust_files(&src, &mut files);
    assert!(files.len() > 20, "scanned {}: {files:?}", src.display());

    let mut found = Vec::new();
    for file in &files {
        let text = std::fs::read_to_string(file).unwrap();
        for (number, line) in text.lines().enumerate() {
            for needle in REMOVED.iter().filter(|needle| line.to_lowercase().contains(&needle.to_lowercase())) {
                found.push(format!("{}:{}: `{needle}` in {line:?}", file.strip_prefix(&src).unwrap().display(), number + 1));
            }
        }
    }
    assert!(found.is_empty(), "the removed full build is still named:\n{}", found.join("\n"));
}

#[test]
fn the_whole_workspace_graph_is_built_only_by_the_test_oracle() {
    let oracle = include_str!("../common/oracle.rs");
    assert!(oracle.contains("WorkspaceGraph::build_with_roots(&indices"), "the oracle builds the full graph");
}
