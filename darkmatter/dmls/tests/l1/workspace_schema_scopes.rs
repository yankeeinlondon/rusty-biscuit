//! Level-1 sessions for the workspace scopes whose trigger registries DMLS
//! prepares without an open document: each workspace folder and each
//! repository the file-list walk finds inside one, every package and package
//! area its repository's catalog places inside it, and each monorepo
//! first-level directory with a `schemas` folder that is a package area only
//! by the catalog's first-directory fallback.
//!
//! The startup cases read the registry line's counts and the trigger-load
//! diagnostic published on an unopened envelope; the refresh cases show that
//! each refresh (watched manifest, watched or saved YAML, server rescan,
//! configuration change) prepares those scopes again from the current
//! catalog.
//!
//! Each startup case installs a process-wide capturing subscriber, which
//! nextest's process-per-test model allows.

use crate::common;

use std::path::Path;

use common::{LspFixture, LspWorkspace};
use dmls::log_capture::CapturedLogs;
use dmls::work_counters::work_counts;
use serde_json::{Value, json};

const BROKEN_TRIGGER: &str = "kind: schema-trigger\nmatch: {}\n";
const NAMED_SCHEMA: &str = "$schema:\n  model: string(required)\n";

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

/// A minimal repository Git discovery accepts, without running `git`.
fn init_repository(root: &Path) {
    std::fs::create_dir_all(root.join(".git/objects")).unwrap();
    std::fs::create_dir_all(root.join(".git/refs/heads")).unwrap();
    write(&root.join(".git/HEAD"), "ref: refs/heads/main\n");
    write(&root.join(".git/config"), "[core]\n\trepositoryformatversion = 0\n\tbare = false\n");
}

/// Declares `members` as Cargo packages of the repository at `repo`; Sniff
/// counts a declared member whose manifest exists.
fn declare_packages(repo: &Path, members: &[&str]) {
    let quoted: Vec<String> = members.iter().map(|member| format!("\"{member}\"")).collect();
    write(&repo.join("Cargo.toml"), &format!("[workspace]\nmembers = [{}]\n", quoted.join(", ")));
    for member in members {
        let name = member.rsplit('/').next().unwrap();
        write(
            &repo.join(member).join("Cargo.toml"),
            &format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"),
        );
        write(&repo.join(member).join("src/lib.rs"), "");
    }
}

/// The review fixture: package `area/pkg` with a broken trigger, and a valid
/// named schema in its package area. The tree has no schema folder and the
/// fixture home none either, so every count comes from the two scoped roots.
fn package_fixture(repo: &Path) {
    init_repository(repo);
    declare_packages(repo, &["area/pkg"]);
    write(&repo.join("area/schemas/area.yaml"), NAMED_SCHEMA);
    write(&repo.join("area/pkg/schemas/broken.yaml"), BROKEN_TRIGGER);
    write(&repo.join("area/pkg/docs/note.md"), "---\ntitle: Note\n---\n# Note\n");
}

fn uri(path: &Path) -> String {
    url::Url::from_file_path(path).unwrap().to_string()
}

fn folders(folders: &[&Path]) -> Vec<Value> {
    folders
        .iter()
        .map(|folder| json!({ "uri": url::Url::from_directory_path(folder).unwrap().as_str(), "name": "f" }))
        .collect()
}

/// A client that watches files.
fn watched_params(workspace_folders: &[&Path]) -> Value {
    json!({
        "processId": null,
        "clientInfo": { "name": "Scope Client", "version": "1.0" },
        "capabilities": {
            "general": { "positionEncodings": ["utf-16"] },
            "workspace": { "didChangeWatchedFiles": { "dynamicRegistration": true } }
        },
        "workspaceFolders": folders(workspace_folders)
    })
}

/// A client with no file watcher, so DMLS rescans on save.
fn rescan_params(folder: &Path) -> Value {
    let mut params = watched_params(&[folder]);
    params["clientInfo"] = json!({ "name": "Neovim", "version": "0.11.0" });
    params["capabilities"]["workspace"].as_object_mut().unwrap().remove("didChangeWatchedFiles");
    params
}

/// The one startup registry line, asserting its message and every
/// `key=value` in `fields`.
fn assert_registry_line(logs: &CapturedLogs, message: &str, fields: &[&str]) {
    let lines = logs.matching("schema registry");
    assert_eq!(lines.len(), 1, "one registry line: {:#?}", logs.lines());
    let line = &lines[0];
    assert!(line.contains(message), "{line}");
    for field in fields {
        assert!(line.contains(&format!(" {field}")), "`{field}` missing from: {line}");
    }
}

fn has_prepare_error(diagnostics: &[Value]) -> bool {
    diagnostics.iter().any(|diagnostic| diagnostic["code"] == json!("dm.schema.prepare"))
}

// ── Startup ─────────────────────────────────────────────────────────────────

#[test]
fn startup_classifies_a_broken_trigger_in_an_unopened_package() {
    let workspace = LspWorkspace::new();
    let repo = workspace.path();
    package_fixture(repo);
    let before = work_counts();
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(&[repo]));

    let broken = fixture.wait_for_diagnostics(&uri(&repo.join("area/pkg/schemas/broken.yaml")));
    assert!(has_prepare_error(&broken), "{broken:#?}");
    assert_eq!(work_counts().since(before).parses, 0, "startup parses no Markdown");
    fixture.shutdown();
}

/// The spelling before the rename is a broken trigger like any other: a
/// trigger-load diagnostic on its file naming `schema-trigger`, and one failed
/// scope, never a silently ignored file.
#[test]
fn startup_reports_a_renamed_trigger_kind_as_a_broken_trigger_in_its_scope_only() {
    let logs = CapturedLogs::global();
    let workspace = LspWorkspace::new();
    let repo = workspace.path();
    fallback_monorepo(repo);
    let renamed = repo.join("docs/schemas/renamed.yaml");
    write(&renamed, "kind: trigger-schema\nmatch:\n  title: string(required)\n");
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(&[repo]));

    let diagnostics = fixture.wait_for_diagnostics(&uri(&renamed));
    let prepare: Vec<&Value> =
        diagnostics.iter().filter(|diagnostic| diagnostic["code"] == json!("dm.schema.prepare")).collect();
    assert_eq!(prepare.len(), 1, "{diagnostics:#?}");
    let message = prepare[0]["message"].as_str().unwrap_or_default();
    assert!(message.contains("trigger-schema") && message.contains("schema-trigger"), "{message}");
    assert_registry_line(&logs, "schema registry initialized with failures", &[
        "failed_triggers=1",
        "failed_scopes=1",
    ]);
    fixture.shutdown();
}

#[test]
fn startup_counts_the_named_schema_of_an_unopened_package_area() {
    let logs = CapturedLogs::global();
    let workspace = LspWorkspace::new();
    let repo = workspace.path();
    package_fixture(repo);
    std::fs::remove_file(repo.join("area/pkg/schemas/broken.yaml")).unwrap();
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(&[repo]));

    // `area/pkg/schemas` is searched but empty; `area/schemas` holds one
    // named schema.
    assert_registry_line(&logs, "schema registry ready", &[
        "roots=2",
        "tree_schemas=0",
        "scoped_schemas=1",
        "trigger_schemas=0",
        "failed_triggers=0",
        "failed_scopes=0",
    ]);
    fixture.shutdown();
}

#[test]
fn startup_reports_an_unopened_package_failure_with_no_open_documents() {
    let logs = CapturedLogs::global();
    let workspace = LspWorkspace::new();
    let repo = workspace.path();
    package_fixture(repo);
    let before = work_counts();
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(&[repo]));

    assert_registry_line(&logs, "schema registry initialized with failures", &[
        "roots=2",
        "tree_schemas=0",
        "scoped_schemas=1",
        "trigger_schemas=0",
        "unclassified=0",
        "failed_triggers=1",
        "failed_scopes=1",
    ]);
    assert!(logs.matching("schema registry ready").is_empty(), "{:#?}", logs.lines());
    assert_eq!(work_counts().since(before).parses, 0, "startup parses no Markdown");
    fixture.shutdown();
}

#[test]
fn a_workspace_subfolder_keeps_the_repository_roots_above_it() {
    let logs = CapturedLogs::global();
    let workspace = LspWorkspace::new();
    let repo = workspace.path();
    package_fixture(repo);
    write(&repo.join("schemas/repo.yaml"), NAMED_SCHEMA);
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(&[&repo.join("area")]));

    // The folder is the package area; its package below it is prepared, and
    // the repository's tree root above it still applies.
    assert_registry_line(&logs, "schema registry initialized with failures", &[
        "roots=3",
        "tree_schemas=1",
        "scoped_schemas=1",
        "failed_triggers=1",
        "failed_scopes=1",
    ]);
    fixture.shutdown();
}

#[test]
fn each_workspace_folder_prepares_only_the_packages_inside_it() {
    let logs = CapturedLogs::global();
    let workspace = LspWorkspace::new();
    let repo = workspace.path();
    init_repository(repo);
    declare_packages(repo, &["one/pkg", "two/pkg", "three/pkg"]);
    for area in ["one", "two", "three"] {
        write(&repo.join(area).join("pkg/schemas").join(format!("{area}.yaml")), NAMED_SCHEMA);
    }
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(&[&repo.join("one"), &repo.join("two")]));

    // `three/` holds no workspace document, so its package is not prepared.
    assert_registry_line(&logs, "schema registry ready", &[
        "roots=2",
        "scoped_schemas=2",
        "failed_scopes=0",
    ]);
    fixture.shutdown();
}

#[test]
fn nested_package_areas_each_prepare_their_roots() {
    let logs = CapturedLogs::global();
    let workspace = LspWorkspace::new();
    let repo = workspace.path();
    init_repository(repo);
    declare_packages(repo, &["outer/lib", "outer/inner/pkg"]);
    write(&repo.join("outer/schemas/outer.yaml"), NAMED_SCHEMA);
    write(&repo.join("outer/inner/schemas/inner.yaml"), NAMED_SCHEMA);
    write(&repo.join("outer/inner/pkg/schemas/pkg.yaml"), NAMED_SCHEMA);
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(&[repo]));

    // Area `outer/inner` belongs to `outer/inner/pkg`, area `outer` to
    // `outer/lib`; no document's roots include both areas, yet both count.
    assert_registry_line(&logs, "schema registry ready", &[
        "roots=3",
        "tree_schemas=0",
        "scoped_schemas=3",
        "failed_scopes=0",
    ]);
    fixture.shutdown();
}

// ── Refreshes ───────────────────────────────────────────────────────────────

/// A repository whose `area/pkg` holds a broken trigger but is not a package
/// yet, so startup does not search its schema folder.
fn undeclared_package(repo: &Path) -> String {
    init_repository(repo);
    write(&repo.join("Cargo.toml"), "[workspace]\nmembers = []\n");
    write(&repo.join("area/pkg/schemas/broken.yaml"), BROKEN_TRIGGER);
    uri(&repo.join("area/pkg/schemas/broken.yaml"))
}

#[test]
fn a_watched_manifest_change_prepares_a_new_package() {
    let workspace = LspWorkspace::new();
    let repo = workspace.path();
    let broken = undeclared_package(repo);
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(&[repo]));

    declare_packages(repo, &["area/pkg"]);
    fixture.notify(
        "workspace/didChangeWatchedFiles",
        json!({ "changes": [ { "uri": uri(&repo.join("Cargo.toml")), "type": 2 } ] }),
    );
    let diagnostics = fixture.wait_for_diagnostics(&broken);
    assert!(has_prepare_error(&diagnostics), "{diagnostics:#?}");
    fixture.shutdown();
}

#[test]
fn a_server_rescan_prepares_a_new_package() {
    let workspace = LspWorkspace::new();
    let repo = workspace.path();
    let broken = undeclared_package(repo);
    write(&repo.join("a.md"), "# A\n");
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(rescan_params(repo));

    declare_packages(repo, &["area/pkg"]);
    fixture.notify("textDocument/didSave", json!({ "textDocument": { "uri": uri(&repo.join("a.md")) } }));
    let diagnostics = fixture.wait_for_diagnostics(&broken);
    assert!(has_prepare_error(&diagnostics), "{diagnostics:#?}");
    fixture.shutdown();
}

#[test]
fn a_configuration_change_prepares_a_new_package() {
    let workspace = LspWorkspace::new();
    let repo = workspace.path();
    let broken = undeclared_package(repo);
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(&[repo]));

    // No manifest event: the configuration change drops every context and
    // queues a workspace pass.
    declare_packages(repo, &["area/pkg"]);
    fixture.notify(
        "workspace/didChangeConfiguration",
        json!({ "settings": { "dmls": { "formatting": { "markdown": { "unwrap": true } } } } }),
    );
    let diagnostics = fixture.wait_for_diagnostics(&broken);
    assert!(has_prepare_error(&diagnostics), "{diagnostics:#?}");
    fixture.shutdown();
}

/// An unchanged configuration notification queues no workspace pass, but it
/// still drops every context: an open document whose context rebuilds
/// differently (its package was declared with no manifest event) is
/// prepared again, and only it.
#[test]
fn an_unchanged_configuration_prepares_an_open_document_whose_context_changed() {
    let workspace = LspWorkspace::new();
    let repo = workspace.path();
    let broken = undeclared_package(repo);
    let note = repo.join("area/pkg/docs/note.md");
    let text = "---\ntitle: Note\n---\n# Note\n";
    write(&note, text);
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(&[repo]));
    fixture.notify(
        "textDocument/didOpen",
        json!({ "textDocument": { "uri": uri(&note), "languageId": "markdown", "version": 1, "text": text } }),
    );
    fixture.flush_server();
    assert!(fixture.take_buffered_diagnostics(&broken).is_none(), "the undeclared package is not a scope yet");

    declare_packages(repo, &["area/pkg"]);
    fixture.notify("workspace/didChangeConfiguration", json!({ "settings": {} }));
    let diagnostics = fixture.wait_for_diagnostics(&broken);
    assert!(has_prepare_error(&diagnostics), "{diagnostics:#?}");
    assert_eq!(fixture.registry_jobs_until_idle(), vec![dmls::RegistryJobKind::Document], "no workspace pass");
    fixture.shutdown();
}

#[test]
fn a_watched_fix_clears_a_trigger_failure_in_an_unopened_package() {
    let workspace = LspWorkspace::new();
    let repo = workspace.path();
    package_fixture(repo);
    let path = repo.join("area/pkg/schemas/broken.yaml");
    write(&repo.join("area/pkg/schemas/payload.yaml"), NAMED_SCHEMA);
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(&[repo]));
    assert!(has_prepare_error(&fixture.wait_for_diagnostics(&uri(&path))));

    write(&path, "kind: schema-trigger\nmatch:\n  title: string(required)\n$schema: payload.yaml\n");
    fixture.notify(
        "workspace/didChangeWatchedFiles",
        json!({ "changes": [ { "uri": uri(&path), "type": 2 } ] }),
    );
    let diagnostics = fixture.wait_for_diagnostics(&uri(&path));
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    fixture.shutdown();
}

#[test]
fn a_saved_fix_clears_a_trigger_failure_in_an_unopened_package() {
    let workspace = LspWorkspace::new();
    let repo = workspace.path();
    package_fixture(repo);
    let path = repo.join("area/pkg/schemas/broken.yaml");
    write(&repo.join("area/pkg/schemas/payload.yaml"), NAMED_SCHEMA);
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(&[repo]));
    assert!(has_prepare_error(&fixture.wait_for_diagnostics(&uri(&path))));

    write(&path, "kind: schema-trigger\nmatch:\n  title: string(required)\n$schema: payload.yaml\n");
    fixture.notify("textDocument/didSave", json!({ "textDocument": { "uri": uri(&path) } }));
    let diagnostics = fixture.wait_for_diagnostics(&uri(&path));
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    fixture.shutdown();
}

// ── Scopes outside the package catalog ──────────────────────────────────────

/// A monorepo (two Cargo packages under `libs/`) whose `docs/` directory no
/// package or package area claims, so `docs/` is a package area only by the
/// catalog's first-directory fallback.
fn fallback_monorepo(repo: &Path) {
    init_repository(repo);
    declare_packages(repo, &["libs/one", "libs/two"]);
    write(&repo.join("docs/guide.md"), "---\ntitle: Guide\n---\n# Guide\n");
}

#[test]
fn startup_prepares_a_fallback_package_area_with_no_catalog_entry() {
    let logs = CapturedLogs::global();
    let workspace = LspWorkspace::new();
    let repo = workspace.path();
    fallback_monorepo(repo);
    write(&repo.join("docs/schemas/broken.yaml"), BROKEN_TRIGGER);
    let before = work_counts();
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(&[repo]));

    assert_registry_line(&logs, "schema registry initialized with failures", &[
        "roots=1",
        "failed_triggers=1",
        "failed_scopes=1",
    ]);
    assert_eq!(work_counts().since(before).parses, 0, "startup parses no Markdown");
    fixture.shutdown();
}

#[test]
fn a_watched_schema_creation_prepares_a_fallback_package_area() {
    let workspace = LspWorkspace::new();
    let repo = workspace.path();
    fallback_monorepo(repo);
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(&[repo]));

    let broken = repo.join("docs/schemas/broken.yaml");
    write(&broken, BROKEN_TRIGGER);
    fixture.notify(
        "workspace/didChangeWatchedFiles",
        json!({ "changes": [ { "uri": uri(&broken), "type": 1 } ] }),
    );
    let diagnostics = fixture.wait_for_diagnostics(&uri(&broken));
    assert!(has_prepare_error(&diagnostics), "{diagnostics:#?}");
    fixture.shutdown();
}

/// The trigger-registry preparations startup runs for `workspace`, with no
/// open document.
fn startup_registry_scans(workspace: &LspWorkspace) -> usize {
    let before = work_counts();
    let mut fixture = LspFixture::start(workspace);
    fixture.initialize(watched_params(&[workspace.path()]));
    fixture.wait_for_registry_jobs();
    let scans = work_counts().since(before).registry_scans;
    fixture.shutdown();
    scans
}

#[test]
fn first_level_directories_without_a_schema_root_add_no_scopes() {
    let plain = LspWorkspace::new();
    fallback_monorepo(plain.path());
    let baseline = startup_registry_scans(&plain);

    let crowded = LspWorkspace::new();
    fallback_monorepo(crowded.path());
    for directory in ["notes", "tools", ".claude"] {
        write(&crowded.path().join(directory).join("readme.md"), "# Readme\n");
    }
    assert_eq!(startup_registry_scans(&crowded), baseline, "directories with no `schemas` folder");

    let schemas = LspWorkspace::new();
    fallback_monorepo(schemas.path());
    write(&schemas.path().join("docs/schemas/docs.yaml"), NAMED_SCHEMA);
    assert_eq!(startup_registry_scans(&schemas), baseline + 1, "`docs/` with a `schemas` folder");
}

/// A workspace folder that is not a repository, holding the repository
/// `one/` with a named schema in its tree root.
fn folder_of_repositories(folder: &Path) {
    init_repository(&folder.join("one"));
    write(&folder.join("one/schemas/one.yaml"), NAMED_SCHEMA);
    write(&folder.join("one/readme.md"), "# One\n");
}

#[test]
fn startup_prepares_repositories_nested_in_a_non_repository_folder() {
    let logs = CapturedLogs::global();
    let workspace = LspWorkspace::new();
    let folder = workspace.path();
    folder_of_repositories(folder);
    init_repository(&folder.join("two"));
    write(&folder.join("two/schemas/two.yaml"), NAMED_SCHEMA);
    let before = work_counts();
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(&[folder]));

    assert_registry_line(&logs, "schema registry ready", &[
        "roots=2",
        "tree_schemas=2",
        "scoped_schemas=0",
        "failed_scopes=0",
    ]);
    assert_eq!(work_counts().since(before).parses, 0, "startup parses no Markdown");
    fixture.shutdown();
}

/// Adds the repository `two/` with a broken trigger to `folder`, returning
/// the trigger's URI.
fn add_broken_repository(folder: &Path) -> String {
    init_repository(&folder.join("two"));
    write(&folder.join("two/schemas/broken.yaml"), BROKEN_TRIGGER);
    uri(&folder.join("two/schemas/broken.yaml"))
}

#[test]
fn a_server_rescan_prepares_a_new_nested_repository() {
    let workspace = LspWorkspace::new();
    let folder = workspace.path();
    folder_of_repositories(folder);
    write(&folder.join("a.md"), "# A\n");
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(rescan_params(folder));

    let broken = add_broken_repository(folder);
    fixture.notify("textDocument/didSave", json!({ "textDocument": { "uri": uri(&folder.join("a.md")) } }));
    let diagnostics = fixture.wait_for_diagnostics(&broken);
    assert!(has_prepare_error(&diagnostics), "{diagnostics:#?}");
    fixture.shutdown();
}

#[test]
fn a_watched_git_config_creation_prepares_a_new_nested_repository() {
    let workspace = LspWorkspace::new();
    let folder = workspace.path();
    folder_of_repositories(folder);
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(&[folder]));

    let broken = add_broken_repository(folder);
    fixture.notify(
        "workspace/didChangeWatchedFiles",
        json!({ "changes": [ { "uri": uri(&folder.join("two/.git/config")), "type": 1 } ] }),
    );
    let diagnostics = fixture.wait_for_diagnostics(&broken);
    assert!(has_prepare_error(&diagnostics), "{diagnostics:#?}");
    fixture.shutdown();
}
