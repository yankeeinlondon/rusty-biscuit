//! The research [`Loader`] resolves through the file-resolution context it is
//! given, and that context, built for a research root, binds the reference
//! forms the research tree uses wherever the root sits: a repository's top
//! level, outside any repository, or below a repository's top level.
//!
//! Every workspace is written into a temporary directory with a minimal
//! contract schema, so nothing here reads the repository or the test
//! process's environment, home, or current directory.
#![cfg(feature = "research")]

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use darkmatter::markdown::compose::{RequestSnapshot, build_resolution_context};
use messenger::research::model::PlatformDocument;
use messenger::research::{Loaded, Loader, Rule, Workspace};

const PLATFORMS: &str = "messenger/docs/research/platforms";

/// Writes a minimal document contract (with one imported type, so schema
/// imports resolve through the context too) at its fixed layout under `root`.
fn write_contract(root: &Path) -> PathBuf {
    let platforms = root.join(PLATFORMS);
    fs::create_dir_all(&platforms).expect("mkdir");
    fs::write(platforms.join("_schema.yaml"), "$schema:\n    schema_version: literal(1; required)\n    note: \"Note@./_types.yaml\"\n")
        .expect("schema");
    fs::write(platforms.join("_types.yaml"), "$schema:\n    Note: string\n").expect("types");
    platforms
}

/// Writes a document at `platforms/<name>` that binds `$schema: <reference>`.
fn write_document(platforms: &Path, name: &str, reference: &str) -> PathBuf {
    let path = platforms.join(name);
    fs::write(&path, format!("---\n$schema: \"{reference}\"\nschema_version: 1\nnote: probe\n---\n")).expect("document");
    path
}

fn loader(snapshot: &RequestSnapshot) -> Loader {
    let context = build_resolution_context(snapshot).expect("research root context builds");
    Loader::new(Workspace::new(snapshot.request_dir()).expect("absolute root"), context)
}

fn rules(loaded: &Loaded<PlatformDocument>) -> Vec<Rule> {
    loaded.diagnostics.iter().map(|diagnostic| diagnostic.rule).collect()
}

/// Loads `document` and asserts its `$schema` bound the shipped contract and
/// Darkmatter resolved that schema (and its import) through the same context.
fn assert_binds(loader: &Loader, document: &Path) {
    let loaded = loader.load_document(document).expect("schema resolves");
    assert!(!rules(&loaded).contains(&Rule::SchemaBinding), "{}: {:?}", document.display(), loaded.diagnostics);
}

/// Makes `dir` a Git work tree's top level, ignoring host Git configuration.
fn git_init(dir: &Path) {
    let status = Command::new("git")
        .args(["init", "-q"])
        .current_dir(dir)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", dir.join("no-global-gitconfig"))
        .status()
        .expect("run git init");
    assert!(status.success(), "git init {}", dir.display());
}

/// Both relative spellings the research tree uses: beside the schema, and
/// climbing to the research root and back down.
fn assert_relative_forms_bind(root: &Path) {
    let platforms = write_contract(root);
    let beside = write_document(&platforms, "beside.md", "./_schema.yaml");
    let climbing = write_document(&platforms, "climbing.md", &format!("../../../../{PLATFORMS}/_schema.yaml"));
    let loader = loader(&RequestSnapshot::new(root));
    assert_binds(&loader, &beside);
    assert_binds(&loader, &climbing);
}

#[test]
fn the_loader_binds_a_variable_schema_reference_from_the_context_it_is_given() {
    const VARIABLE: &str = "MESSENGER_RESEARCH_CONTEXT_PROBE_SCHEMAS";
    assert!(std::env::var_os(VARIABLE).is_none(), "the test process must not carry {VARIABLE}");
    let dir = tempfile::tempdir().expect("tempdir");
    let platforms = write_contract(dir.path());
    let document = write_document(&platforms, "variable.md", &format!("{{{{{VARIABLE}}}}}/_schema.yaml"));

    let given = RequestSnapshot::new(dir.path())
        .with_env(HashMap::from([(VARIABLE.to_string(), platforms.to_string_lossy().into_owned())]));
    assert_binds(&loader(&given), &document);

    let without = loader(&RequestSnapshot::new(dir.path())).load_document(&document).expect("load");
    assert_eq!(rules(&without), vec![Rule::SchemaBinding], "{:?}", without.diagnostics);
}

#[test]
fn a_research_root_outside_any_repository_binds_its_relative_schema_references() {
    let dir = tempfile::tempdir().expect("tempdir");
    let context = build_resolution_context(&RequestSnapshot::new(dir.path())).expect("context");
    assert_eq!(context.repository_root(), None, "a temporary directory is in no repository");
    assert_relative_forms_bind(dir.path());
}

#[test]
fn a_research_root_at_a_repository_top_level_binds_its_relative_schema_references() {
    let dir = tempfile::tempdir().expect("tempdir");
    git_init(dir.path());
    let context = build_resolution_context(&RequestSnapshot::new(dir.path())).expect("context");
    let found = context.repository_root().map(|root| fs::canonicalize(root).expect("canonical repository root"));
    assert_eq!(found, Some(fs::canonicalize(dir.path()).expect("canonical root")));
    assert_relative_forms_bind(dir.path());
}

#[test]
fn a_research_root_below_a_repository_top_level_binds_its_relative_schema_references() {
    let dir = tempfile::tempdir().expect("tempdir");
    git_init(dir.path());
    let root = dir.path().join("nested");
    fs::create_dir(&root).expect("mkdir");
    let context = build_resolution_context(&RequestSnapshot::new(&root)).expect("context");
    let found = context.repository_root().map(|top| fs::canonicalize(top).expect("canonical repository root"));
    assert_eq!(found, Some(fs::canonicalize(dir.path()).expect("canonical top level")), "the repository is the enclosing one");
    assert_relative_forms_bind(&root);
}
