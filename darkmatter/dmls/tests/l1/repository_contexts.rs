//! DMLS resolves file references through one context per repository.
//!
//! Full LSP sessions over the in-memory fixture, against hand-built
//! repositories in a temporary workspace. Covers: one build per repository
//! with the repository root as request directory, `&`/`^`/`@` in every
//! feature, invalidation by watched manifests, rescans, and configuration
//! changes, the context-failure diagnostic, and untitled buffers.

use crate::common;

use std::path::{Path, PathBuf};

use common::{LspFixture, LspWorkspace};
use darkmatter::markdown::compose::RequestSnapshot;
use dmls::context::context_build_count;
use serde_json::{Value, json};

/// A client that watches files, with work-done progress so a test can wait
/// for the startup index, and resource operations for create-file actions.
fn watched_params(folders: &[&Path]) -> Value {
    let folders: Vec<Value> = folders
        .iter()
        .map(|folder| {
            json!({ "uri": url::Url::from_directory_path(folder).unwrap().as_str(), "name": "f" })
        })
        .collect();
    json!({
        "processId": null,
        "clientInfo": { "name": "Watched Client", "version": "1.0" },
        "capabilities": {
            "general": { "positionEncodings": ["utf-16"] },
            "window": { "workDoneProgress": true },
            "workspace": {
                "didChangeWatchedFiles": { "dynamicRegistration": true },
                "workspaceEdit": { "documentChanges": true, "resourceOperations": ["create"] }
            },
            "textDocument": { "codeAction": {} }
        },
        "workspaceFolders": folders
    })
}

/// A Neovim-on-Linux-shaped client: no file watcher, so DMLS rescans on save.
fn rescan_params(folder: &Path) -> Value {
    let mut params = watched_params(&[folder]);
    params["clientInfo"] = json!({ "name": "Neovim", "version": "0.11.0" });
    params["capabilities"]["workspace"]
        .as_object_mut()
        .unwrap()
        .remove("didChangeWatchedFiles");
    params
}

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

const GIT_CONFIG: &str = "[core]\n\trepositoryformatversion = 0\n\tbare = false\n";

/// A minimal repository Git discovery accepts, without running `git`.
fn init_repository(root: &Path) {
    std::fs::create_dir_all(root.join(".git/objects")).unwrap();
    std::fs::create_dir_all(root.join(".git/refs/heads")).unwrap();
    write(&root.join(".git/HEAD"), "ref: refs/heads/main\n");
    write(&root.join(".git/config"), GIT_CONFIG);
}

/// Declares `members` as the workspace's packages. Sniff recognizes a
/// package only under a workspace manifest, and counts a declared member
/// whose directory exists.
fn declare_workspace(repo: &Path, members: &[&str]) {
    let members: Vec<String> = members.iter().map(|member| format!("\"{member}\"")).collect();
    write(&repo.join("Cargo.toml"), &format!("[workspace]\nmembers = [{}]\n", members.join(", ")));
}

fn add_package(dir: &Path, name: &str) -> PathBuf {
    let manifest = dir.join("Cargo.toml");
    write(
        &manifest,
        &format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"),
    );
    write(&dir.join("src/lib.rs"), "");
    manifest
}

fn uri(path: &Path) -> String {
    url::Url::from_file_path(path).unwrap().to_string()
}

fn open(fixture: &LspFixture<'_>, uri: &str, text: &str) {
    fixture.notify(
        "textDocument/didOpen",
        json!({
            "textDocument": { "uri": uri, "languageId": "markdown", "version": 1, "text": text }
        }),
    );
}

fn notify_watched(fixture: &LspFixture<'_>, path: &Path, kind: u32) {
    fixture.notify(
        "workspace/didChangeWatchedFiles",
        json!({ "changes": [ { "uri": uri(path), "type": kind } ] }),
    );
}

/// The diagnostics most recently published for `uri` once every queued
/// notification has been handled.
fn current_diagnostics(fixture: &mut LspFixture<'_>, uri: &str) -> Vec<Value> {
    fixture.flush_server();
    let latest = fixture
        .take_buffered_diagnostics(uri)
        .unwrap_or_else(|| panic!("no diagnostics published for {uri}"));
    while fixture.take_buffered_diagnostics(uri).is_some() {}
    latest["diagnostics"].as_array().cloned().unwrap_or_default()
}

fn codes(diagnostics: &[Value]) -> Vec<String> {
    diagnostics
        .iter()
        .filter_map(|diagnostic| diagnostic["code"].as_str().map(str::to_string))
        .collect()
}

/// The canonical file each document-link target names.
fn link_targets(fixture: &mut LspFixture<'_>, uri: &str) -> Vec<PathBuf> {
    let response = fixture.request("textDocument/documentLink", json!({ "textDocument": { "uri": uri } }));
    let links = response.result.unwrap_or(Value::Null);
    links
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|link| link["target"].as_str())
        .map(|target| canonical(&url::Url::parse(target).unwrap().to_file_path().unwrap()))
        .collect()
}

fn definition(fixture: &mut LspFixture<'_>, uri: &str, line: u32, character: u32) -> Vec<PathBuf> {
    let response = fixture.request(
        "textDocument/definition",
        json!({ "textDocument": { "uri": uri }, "position": { "line": line, "character": character } }),
    );
    response
        .result
        .unwrap_or(Value::Null)
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|location| location["uri"].as_str())
        .map(|target| canonical(&url::Url::parse(target).unwrap().to_file_path().unwrap()))
        .collect()
}

/// One spelling per file, so `/var` and `/private/var` (macOS) or a verbatim
/// prefix (Windows) compare equal.
fn canonical(path: &Path) -> PathBuf {
    biscuit_file::canonicalize_simplified(path).unwrap_or_else(|_| path.to_path_buf())
}

/// The line and column of the first occurrence of `needle` in `text`.
fn position_of(text: &str, needle: &str) -> (u32, u32) {
    let offset = text.find(needle).unwrap_or_else(|| panic!("`{needle}` in document"));
    let line = text[..offset].matches('\n').count();
    let column = offset - text[..offset].rfind('\n').map_or(0, |newline| newline + 1);
    (line as u32, column as u32)
}

// ── Acceptance Criterion 8: one context per repository ──────────────────────

#[test]
fn documents_in_one_repository_share_one_context_built_at_its_root() {
    // The workspace folder is *above* the repository: the context's request
    // directory is still the repository root, so `&` anchors there.
    let workspace = LspWorkspace::new();
    let repo = workspace.path().join("repo");
    init_repository(&repo);
    write(&repo.join("root-only.md"), "# Root\n");
    let first = repo.join("docs/first.md");
    let second = repo.join("docs/deeper/second.md");
    let first_text = "# First\n\n::file &root-only.md\n";
    let second_text = "# Second\n\n::file &root-only.md\n";
    write(&first, first_text);
    write(&second, second_text);

    let before = context_build_count();
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(&[workspace.path()]));
    fixture.wait_for_startup_complete();
    open(&fixture, &uri(&first), first_text);
    open(&fixture, &uri(&second), second_text);

    let root_only = canonical(&repo.join("root-only.md"));
    assert_eq!(link_targets(&mut fixture, &uri(&first)), vec![root_only.clone()]);
    assert_eq!(link_targets(&mut fixture, &uri(&second)), vec![root_only]);
    assert_eq!(
        context_build_count() - before,
        1,
        "the startup index and both documents share the repository's one build"
    );
    fixture.shutdown();
}

#[test]
fn a_document_in_no_repository_resolves_against_its_own_folder() {
    let workspace = LspWorkspace::new();
    let folder = workspace.path().join("plain/notes");
    let document = folder.join("doc.md");
    let text = "# Doc\n\n::file beside.md\n\n::file &beside.md\n";
    write(&document, text);
    write(&folder.join("beside.md"), "# Beside\n");

    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(&[workspace.path()]));
    open(&fixture, &uri(&document), text);

    assert_eq!(link_targets(&mut fixture, &uri(&document)), vec![canonical(&folder.join("beside.md"))]);
    // `&` needs a repository: it is broken, and its class says why.
    let diagnostics = current_diagnostics(&mut fixture, &uri(&document));
    let broken: Vec<&Value> = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic["code"] == json!("dm.transclusion.broken_path"))
        .collect();
    assert_eq!(broken.len(), 1, "{diagnostics:#?}");
    assert_eq!(broken[0]["data"], json!({ "resolution_failure": "MissingContext" }));
    assert!(!codes(&diagnostics).contains(&"dm.context.build_failure".to_string()));
    fixture.shutdown();
}

/// A single-file target whose text looks like a glob names that file
/// literally; its broken-path diagnostic carries biscuit-file's literal-glob
/// hint, while a plain miss and a failure other than a miss do not.
#[test]
fn a_literal_glob_miss_diagnostic_hints_at_glob_forms() {
    let workspace = LspWorkspace::new();
    let folder = workspace.path().join("plain/notes");
    let document = folder.join("doc.md");
    let text = "# Doc\n\n::file docs/*.md\n\n::file docs/missing.md\n\n::file &docs/*.md\n\n::file docs/a.md\n\n[glob](docs/*.md) [plain](docs/missing.md)\n";
    write(&document, text);
    write(&folder.join("docs/a.md"), "# A\n");

    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(&[workspace.path()]));
    open(&fixture, &uri(&document), text);

    let diagnostics = current_diagnostics(&mut fixture, &uri(&document));
    let message_of = |code: &str, needle: &str| -> String {
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic["code"] == json!(code))
            .filter_map(|diagnostic| diagnostic["message"].as_str())
            .find(|message| message.contains(needle))
            .unwrap_or_else(|| panic!("no {code} diagnostic names {needle}: {diagnostics:#?}"))
            .to_string()
    };
    let hinted = |message: &str| message.contains("::file-links");

    assert!(hinted(&message_of("dm.transclusion.broken_path", "`docs/*.md`")));
    assert!(!hinted(&message_of("dm.transclusion.broken_path", "docs/missing.md")));
    assert!(!hinted(&message_of("dm.transclusion.broken_path", "&docs/*.md")), "a missing-context failure");
    assert!(hinted(&message_of("dm.links.broken_path", "`docs/*.md`")));
    assert!(!hinted(&message_of("dm.links.broken_path", "docs/missing.md")));
    assert!(
        !diagnostics.iter().any(|diagnostic| diagnostic["message"].as_str().is_some_and(|m| m.contains("docs/a.md"))),
        "the control resolves: {diagnostics:#?}"
    );
    fixture.shutdown();
}

// ── Acceptance Criterion 9 (outside the matrix): every feature ──────────────

#[test]
fn repository_sigils_resolve_in_every_feature() {
    let workspace = LspWorkspace::new();
    let repo = workspace.path().join("repo");
    let magic = workspace.path().join("magic");
    init_repository(&repo);
    declare_workspace(&repo, &["pkg"]);
    let package = repo.join("pkg");
    add_package(&package, "pkg");
    write(&repo.join("root-only.md"), "# Root\n");
    write(&package.join("pkg-only.md"), "# Package\n");
    write(&magic.join("magic-only.md"), "# Magic\n");
    let document = package.join("docs/doc.md");
    let text = "---\n$schema:\n  root: file(eager)\n  scoped: file(eager)\n  extra: file(eager)\nroot: \"&root-only.md\"\nscoped: \"^pkg-only.md\"\nextra: \"@magic-only.md\"\n---\n\n# Doc\n\n::file &root-only.md\n\n::file ^pkg-only.md\n\n::file @magic-only.md\n\n[root](&root-only.md) [scoped](^pkg-only.md) [missing](&missing.md)\n";
    write(&document, text);

    // An extra `@` root reaches DMLS only through the startup snapshot.
    let snapshot = LspFixture::fixture_snapshot(&workspace)
        .with_magic_root(&magic, biscuit_file::PathPosition::End);
    let mut fixture = LspFixture::start_with_snapshot(&workspace, snapshot);
    fixture.initialize(watched_params(&[workspace.path()]));
    fixture.wait_for_startup_complete();
    let doc_uri = uri(&document);
    open(&fixture, &doc_uri, text);

    let root_only = canonical(&repo.join("root-only.md"));
    let pkg_only = canonical(&package.join("pkg-only.md"));
    let magic_only = canonical(&magic.join("magic-only.md"));

    // Schema validation: every eager file value resolves (an unresolved one
    // is `dm.schema.invalid_file_reference`); only `&missing.md`, a link, is
    // broken.
    let diagnostics = current_diagnostics(&mut fixture, &doc_uri);
    assert_eq!(codes(&diagnostics), vec!["dm.links.broken_path".to_string()], "{diagnostics:#?}");
    assert_eq!(diagnostics[0]["data"], json!({ "resolution_failure": "NoMatch" }));

    // Document links: three frontmatter values, three transclusions, and the
    // two resolved Markdown links (the link graph).
    let mut targets = link_targets(&mut fixture, &doc_uri);
    targets.sort();
    let mut expected = vec![
        root_only.clone(), pkg_only.clone(), magic_only.clone(),
        root_only.clone(), pkg_only.clone(), magic_only.clone(),
        root_only.clone(), pkg_only.clone(),
    ];
    expected.sort();
    assert_eq!(targets, expected);

    // Go-to-definition on each transclusion target and each Markdown link.
    for (needle, target) in [
        ("&root-only.md\n\n::file ^", &root_only),
        ("^pkg-only.md\n\n::file @", &pkg_only),
        ("@magic-only.md\n\n[root]", &magic_only),
        ("root](&root-only.md)", &root_only),
        ("scoped](^pkg-only.md)", &pkg_only),
    ] {
        let (line, column) = position_of(text, needle);
        assert_eq!(&definition(&mut fixture, &doc_uri, line, column + 1), &vec![target.clone()], "{needle}");
    }

    // Code action: the missing `&` target is created at the repository root,
    // where `md compose` would look first.
    let missing = diagnostics[0].clone();
    let response = fixture.request(
        "textDocument/codeAction",
        json!({
            "textDocument": { "uri": doc_uri },
            "range": missing["range"],
            "context": { "diagnostics": [missing] }
        }),
    );
    let actions = response.result.unwrap();
    let created: Vec<String> = actions
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|action| action["edit"]["documentChanges"].as_array().cloned().unwrap_or_default())
        .filter(|change| change["kind"] == json!("create"))
        .filter_map(|change| change["uri"].as_str().map(str::to_string))
        .collect();
    assert_eq!(created, vec![uri(&repo.join("missing.md"))]);
    fixture.shutdown();
}

// ── Acceptance Criterion 12: invalidation ───────────────────────────────────

/// A repository whose `pkg/` is not a package yet, and a document there
/// transcluding `^pkg-only.md`.
fn package_scope_fixture(workspace: &LspWorkspace) -> (PathBuf, PathBuf, &'static str) {
    let repo = workspace.path().join("repo");
    init_repository(&repo);
    let document = repo.join("pkg/doc.md");
    let text = "# Doc\n\n::file ^pkg-only.md\n";
    write(&document, text);
    write(&repo.join("pkg/pkg-only.md"), "# Package only\n");
    (repo, document, text)
}

fn transclusion_broken(diagnostics: &[Value]) -> bool {
    codes(diagnostics).contains(&"dm.transclusion.broken_path".to_string())
}

#[test]
fn a_watched_manifest_change_rebuilds_the_repository_context() {
    let workspace = LspWorkspace::new();
    let (repo, document, text) = package_scope_fixture(&workspace);
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(&[workspace.path()]));
    fixture.wait_for_startup_complete();
    let doc_uri = uri(&document);
    open(&fixture, &doc_uri, text);

    assert!(transclusion_broken(&current_diagnostics(&mut fixture, &doc_uri)));
    assert!(link_targets(&mut fixture, &doc_uri).is_empty());

    // Adding the package changes nothing until DMLS hears about it. The
    // notification names only the package's own manifest.
    declare_workspace(&repo, &["pkg"]);
    let manifest = add_package(&repo.join("pkg"), "pkg");
    assert!(link_targets(&mut fixture, &doc_uri).is_empty());

    let before = context_build_count();
    notify_watched(&fixture, &manifest, 1);
    let diagnostics = current_diagnostics(&mut fixture, &doc_uri);
    assert!(!transclusion_broken(&diagnostics), "{diagnostics:#?}");
    assert_eq!(link_targets(&mut fixture, &doc_uri), vec![canonical(&repo.join("pkg/pkg-only.md"))]);
    assert_eq!(context_build_count() - before, 1, "one rebuild for the repository");
    fixture.shutdown();
}

#[test]
fn a_rescan_detected_manifest_change_rebuilds_the_repository_context() {
    let workspace = LspWorkspace::new();
    let (repo, document, text) = package_scope_fixture(&workspace);
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(rescan_params(workspace.path()));
    fixture.wait_for_startup_complete();
    let doc_uri = uri(&document);
    open(&fixture, &doc_uri, text);
    assert!(transclusion_broken(&current_diagnostics(&mut fixture, &doc_uri)));

    // No notification: the save-driven rescan finds the new manifests.
    declare_workspace(&repo, &["pkg"]);
    add_package(&repo.join("pkg"), "pkg");
    fixture.notify("textDocument/didSave", json!({ "textDocument": { "uri": doc_uri } }));
    let diagnostics = current_diagnostics(&mut fixture, &doc_uri);
    assert!(!transclusion_broken(&diagnostics), "{diagnostics:#?}");
    assert_eq!(link_targets(&mut fixture, &doc_uri), vec![canonical(&repo.join("pkg/pkg-only.md"))]);
    fixture.shutdown();
}

#[test]
fn a_configuration_change_drops_every_context() {
    let workspace = LspWorkspace::new();
    let first_repo = workspace.path().join("first");
    let second_repo = workspace.path().join("second");
    init_repository(&first_repo);
    init_repository(&second_repo);
    let first = first_repo.join("a.md");
    let second = second_repo.join("b.md");
    write(&first, "# A\n");
    write(&second, "# B\n");
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(&[workspace.path()]));
    fixture.wait_for_startup_complete();
    open(&fixture, &uri(&first), "# A\n");
    open(&fixture, &uri(&second), "# B\n");
    fixture.flush_server();

    let before = context_build_count();
    fixture.notify("workspace/didChangeConfiguration", json!({ "settings": {} }));
    current_diagnostics(&mut fixture, &uri(&first));
    current_diagnostics(&mut fixture, &uri(&second));
    assert_eq!(context_build_count() - before, 2, "both repositories rebuild");
    fixture.shutdown();
}

#[test]
fn home_comes_from_the_startup_snapshot() {
    // The test process's own HOME is never consulted: `~` names the
    // snapshot's home, which exists only inside the fixture.
    let workspace = LspWorkspace::new();
    let home = workspace.path().join("fixture-home");
    write(&home.join("notes.md"), "# Notes\n");
    let repo = workspace.path().join("repo");
    init_repository(&repo);
    let document = repo.join("doc.md");
    let text = "# Doc\n\n::file ~/notes.md\n";
    write(&document, text);
    assert_ne!(biscuit_file::home_dir(), Some(home.clone()));

    let snapshot = RequestSnapshot::new(workspace.path()).with_home(Some(home.clone()));
    let mut fixture = LspFixture::start_with_snapshot(&workspace, snapshot);
    fixture.initialize(watched_params(&[workspace.path()]));
    open(&fixture, &uri(&document), text);
    assert_eq!(link_targets(&mut fixture, &uri(&document)), vec![canonical(&home.join("notes.md"))]);
    fixture.shutdown();
}

// ── Acceptance Criterion 13: the context-failure diagnostic ─────────────────

#[test]
fn a_failed_context_is_one_diagnostic_and_no_resolution_until_repaired() {
    let workspace = LspWorkspace::new();
    let repo = workspace.path().join("repo");
    init_repository(&repo);
    // A syntactically corrupt config makes repository discovery fail.
    write(&repo.join(".git/config"), "[core\nthis is not = = valid\n");
    write(&repo.join("docs/beside.md"), "# Beside\n");
    let document = repo.join("docs/doc.md");
    let text = "---\n$schema:\n  include: file(eager)\n  missing: file(eager)\ninclude: ./beside.md\nmissing: ./nope.md\n---\n\n# Doc\n\n::file ./beside.md\n\n::file ./absent.md\n\n[beside](beside.md)\n";
    write(&document, text);

    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(&[workspace.path()]));
    fixture.wait_for_startup_complete();
    let doc_uri = uri(&document);
    open(&fixture, &doc_uri, text);

    let diagnostics = current_diagnostics(&mut fixture, &doc_uri);
    assert_eq!(diagnostics.len(), 1, "exactly the failure, no reference diagnostics: {diagnostics:#?}");
    let failure = &diagnostics[0];
    assert_eq!(failure["code"], json!("dm.context.build_failure"));
    assert_eq!(failure["source"], json!("darkmatter.context"));
    assert_eq!(failure["severity"], json!(1));
    assert_eq!(failure["range"]["start"], json!({ "line": 0, "character": 0 }));
    assert_eq!(failure["data"], json!({ "resolution_failure": "MissingContext" }));
    let message = failure["message"].as_str().unwrap();
    assert!(message.contains("MissingContext"), "{message}");
    assert!(message.contains("repository discovery failed"), "{message}");
    assert!(message.contains(&repo.join("docs").display().to_string()), "{message}");

    assert!(link_targets(&mut fixture, &doc_uri).is_empty());
    let (line, column) = position_of(text, "./beside.md\n\n::file ./absent");
    assert!(definition(&mut fixture, &doc_uri, line, column + 1).is_empty());

    // Repair the config and report it: the failure clears, references return.
    write(&repo.join(".git/config"), GIT_CONFIG);
    notify_watched(&fixture, &repo.join(".git/config"), 2);
    let diagnostics = current_diagnostics(&mut fixture, &doc_uri);
    let codes = codes(&diagnostics);
    assert!(!codes.contains(&"dm.context.build_failure".to_string()), "{diagnostics:#?}");
    assert!(codes.contains(&"dm.transclusion.broken_path".to_string()), "{diagnostics:#?}");
    assert!(codes.contains(&"dm.schema.invalid_file_reference".to_string()), "{diagnostics:#?}");
    // Frontmatter values (`missing` navigates to where it would be created),
    // the existing transclusion, and the Markdown link.
    let beside = canonical(&repo.join("docs/beside.md"));
    let mut targets = link_targets(&mut fixture, &doc_uri);
    targets.sort();
    let mut expected =
        vec![beside.clone(), beside.clone(), beside.clone(), canonical(&repo.join("docs/nope.md"))];
    expected.sort();
    assert_eq!(targets, expected);
    assert_eq!(definition(&mut fixture, &doc_uri, line, column + 1), vec![beside]);
    fixture.shutdown();
}

// ── Acceptance Criterion 14: untitled buffers ───────────────────────────────

const UNTITLED: &str = "untitled:Untitled-1";
const UNTITLED_TEXT: &str = "# Scratch\n\n::file &root-only.md\n";

#[test]
fn an_untitled_buffer_uses_the_single_repository_across_the_workspace_folders() {
    let workspace = LspWorkspace::new();
    let repo = workspace.path().join("repo");
    init_repository(&repo);
    write(&repo.join("root-only.md"), "# Root\n");
    std::fs::create_dir_all(repo.join("docs")).unwrap();

    // Two folders, one repository.
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(&[&repo.join("docs"), &repo]));
    open(&fixture, UNTITLED, UNTITLED_TEXT);
    assert_eq!(link_targets(&mut fixture, UNTITLED), vec![canonical(&repo.join("root-only.md"))]);
    let diagnostics = current_diagnostics(&mut fixture, UNTITLED);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    fixture.shutdown();
}

fn assert_untitled_fails(folders: &[&Path], workspace: &LspWorkspace) {
    let mut fixture = LspFixture::start(workspace);
    fixture.initialize(watched_params(folders));
    open(&fixture, UNTITLED, UNTITLED_TEXT);
    assert!(link_targets(&mut fixture, UNTITLED).is_empty());
    let diagnostics = current_diagnostics(&mut fixture, UNTITLED);
    assert_eq!(codes(&diagnostics), vec!["dm.context.build_failure".to_string()], "{diagnostics:#?}");
    assert_eq!(diagnostics[0]["data"], json!({ "resolution_failure": "MissingContext" }));
    fixture.shutdown();
}

#[test]
fn an_untitled_buffer_fails_with_folders_in_two_repositories() {
    let workspace = LspWorkspace::new();
    let first = workspace.path().join("first");
    let second = workspace.path().join("second");
    init_repository(&first);
    init_repository(&second);
    write(&first.join("root-only.md"), "# Root\n");
    assert_untitled_fails(&[&first, &second], &workspace);
}

#[test]
fn an_untitled_buffer_fails_with_folders_in_no_repository() {
    let workspace = LspWorkspace::new();
    let plain = workspace.path().join("plain");
    write(&plain.join("root-only.md"), "# Root\n");
    assert_untitled_fails(&[&plain], &workspace);
}
