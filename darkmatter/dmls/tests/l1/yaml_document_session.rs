//! Level-1 sessions for whole YAML documents (`.yaml`, `.yml`) driven through
//! normal protocol messages: the kinds a document can select are prepared on
//! the registry worker, never the dispatch thread, and an open document is
//! refreshed when what selected or judged it changes: an unsaved edit of a
//! definition, a definition created or deleted on disk, or an edit of the
//! file its root `$schema` names. Each workspace folder's documents select
//! from that folder's own definitions. No client window is involved.

use crate::common;

use std::path::Path;

use common::{LspFixture, LspWorkspace};
use dmls::RegistryJobKind;
use darkmatter::work_counters::{self as library_work, Work};
use dmls::work_counters::{registry_scan_threads, work_counts, yaml_preparation_threads};
use serde_json::{Value, json};

/// A user kind's definition: `owner` is a string.
const ROSTER_DEFINITION: &str = "\
kind: yaml-schema
$schema:
    kind: literal(roster; required)
    roster_version: literal(1; required)
    owner: string
    $constraints: { closed: true }
";

/// A valid roster.
const ROSTER: &str = "kind: roster\nroster_version: 1\nowner: ops\n";

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

/// A minimal repository Git discovery accepts, without running `git`, so the
/// folder is the tree root and `schemas/` a schema root.
fn init_repository(root: &Path) {
    std::fs::create_dir_all(root.join(".git/objects")).unwrap();
    std::fs::create_dir_all(root.join(".git/refs/heads")).unwrap();
    write(&root.join(".git/HEAD"), "ref: refs/heads/main\n");
    write(&root.join(".git/config"), "[core]\n\trepositoryformatversion = 0\n\tbare = false\n");
}

fn uri(path: &Path) -> String {
    url::Url::from_file_path(path).unwrap().to_string()
}

/// A client that watches files, with `folders` as its workspace folders.
fn watched_params(folders: &[&Path]) -> Value {
    let folders: Vec<Value> = folders
        .iter()
        .map(|folder| json!({ "uri": url::Url::from_directory_path(folder).unwrap().as_str(), "name": "w" }))
        .collect();
    json!({
        "processId": null,
        "clientInfo": { "name": "Watched Client", "version": "1.0" },
        "capabilities": {
            "general": { "positionEncodings": ["utf-16"] },
            "workspace": { "didChangeWatchedFiles": { "dynamicRegistration": true } }
        },
        "workspaceFolders": folders
    })
}

fn open(fixture: &LspFixture<'_>, uri: &str, text: &str) {
    fixture.notify(
        "textDocument/didOpen",
        json!({ "textDocument": { "uri": uri, "languageId": "yaml", "version": 1, "text": text } }),
    );
}

fn edit(fixture: &LspFixture<'_>, uri: &str, version: i32, text: &str) {
    fixture.notify(
        "textDocument/didChange",
        json!({ "textDocument": { "uri": uri, "version": version }, "contentChanges": [{ "text": text }] }),
    );
}

/// Reports watched-file events: `(path, kind)` with 1 created, 2 changed,
/// 3 deleted.
fn changed(fixture: &LspFixture<'_>, events: &[(&Path, u8)]) {
    let changes: Vec<Value> =
        events.iter().map(|(path, kind)| json!({ "uri": uri(path), "type": kind })).collect();
    fixture.notify("workspace/didChangeWatchedFiles", json!({ "changes": changes }));
}

/// The diagnostics last published for `uri` once the server is idle; the
/// earlier publications still buffered are dropped.
fn latest_diagnostics(fixture: &mut LspFixture<'_>, uri: &str) -> Vec<Value> {
    fixture.flush_server();
    // The buffered take returns the latest publication first.
    let latest = fixture.wait_for_diagnostics_params(uri);
    while fixture.take_buffered_diagnostics(uri).is_some() {}
    latest["diagnostics"].as_array().cloned().unwrap_or_default()
}

/// The `dm.schema.*` codes among `diagnostics`.
fn schema_codes(diagnostics: &[Value]) -> Vec<String> {
    diagnostics
        .iter()
        .filter_map(|diagnostic| diagnostic["code"].as_str())
        .filter(|code| code.starts_with("dm.schema."))
        .map(str::to_string)
        .collect()
}

fn completion_labels(fixture: &mut LspFixture<'_>, uri: &str, line: u32) -> Vec<String> {
    let response = fixture.request(
        "textDocument/completion",
        json!({ "textDocument": { "uri": uri }, "position": { "line": line, "character": 0 } }),
    );
    assert!(response.error.is_none(), "{response:?}");
    let items = response.result.unwrap_or(Value::Null);
    let items = items.get("items").cloned().unwrap_or(items);
    items.as_array().cloned().unwrap_or_default().iter().filter_map(|item| item["label"].as_str().map(str::to_string)).collect()
}

fn hover(fixture: &mut LspFixture<'_>, uri: &str, line: u32, character: u32) -> String {
    let response = fixture.request(
        "textDocument/hover",
        json!({ "textDocument": { "uri": uri }, "position": { "line": line, "character": character } }),
    );
    assert!(response.error.is_none(), "{response:?}");
    response.result.map(|result| result["contents"]["value"].as_str().unwrap_or_default().to_string()).unwrap_or_default()
}

#[test]
fn an_unsaved_definition_edit_refreshes_an_open_document_until_the_definition_closes() {
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    init_repository(root);
    let definition = root.join("schemas/roster.yaml");
    write(&definition, ROSTER_DEFINITION);
    let document = root.join("team.yaml");
    write(&document, ROSTER);
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(&[root]));

    open(&fixture, &uri(&document), ROSTER);
    assert_eq!(schema_codes(&latest_diagnostics(&mut fixture, &uri(&document))), Vec::<String>::new());

    // The definition's buffer, edited and never saved, is what selects.
    open(&fixture, &uri(&definition), ROSTER_DEFINITION);
    edit(&fixture, &uri(&definition), 2, &ROSTER_DEFINITION.replace("owner: string", "owner: number"));
    let diagnostics = latest_diagnostics(&mut fixture, &uri(&document));
    assert_eq!(schema_codes(&diagnostics), vec!["dm.schema.type_mismatch".to_string()], "{diagnostics:#?}");
    assert_eq!(diagnostics[0]["range"]["start"], json!({ "line": 2, "character": 7 }), "{diagnostics:#?}");

    // Closing it unsaved restores the file on disk.
    fixture.notify("textDocument/didClose", json!({ "textDocument": { "uri": uri(&definition) } }));
    assert_eq!(schema_codes(&latest_diagnostics(&mut fixture, &uri(&document))), Vec::<String>::new());
    fixture.shutdown();
}

#[test]
fn a_definition_created_on_disk_starts_claiming_and_deleting_it_clears_the_results() {
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    init_repository(root);
    write(&root.join("schemas/notes.yaml"), "just: data\n");
    let document = root.join("team.yaml");
    let text = "kind: roster\nroster_version: 1\nowner: [a]\n";
    write(&document, text);
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(&[root]));

    // No definition claims `roster` yet: the document is not validated.
    open(&fixture, &uri(&document), text);
    assert_eq!(schema_codes(&latest_diagnostics(&mut fixture, &uri(&document))), Vec::<String>::new());
    assert!(!completion_labels(&mut fixture, &uri(&document), 3).contains(&"owner".to_string()));

    let definition = root.join("schemas/roster.yaml");
    write(&definition, ROSTER_DEFINITION);
    changed(&fixture, &[(&definition, 1)]);
    let diagnostics = latest_diagnostics(&mut fixture, &uri(&document));
    assert_eq!(schema_codes(&diagnostics), vec!["dm.schema.type_mismatch".to_string()], "{diagnostics:#?}");
    let body = hover(&mut fixture, &uri(&document), 1, 2);
    assert!(body.contains("roster_version"), "{body}");

    std::fs::remove_file(&definition).unwrap();
    changed(&fixture, &[(&definition, 3)]);
    assert_eq!(schema_codes(&latest_diagnostics(&mut fixture, &uri(&document))), Vec::<String>::new());
    assert!(hover(&mut fixture, &uri(&document), 1, 2).is_empty(), "no stale hover");
    fixture.shutdown();
}

#[test]
fn an_edited_root_schema_file_refreshes_the_document_that_names_it() {
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    init_repository(root);
    let contract = root.join("contract.yaml");
    write(&contract, "$schema:\n    port: number\n");
    let document = root.join("service.yaml");
    let text = "$schema: ./contract.yaml\nport: http\n";
    write(&document, text);
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(&[root]));

    open(&fixture, &uri(&document), text);
    assert_eq!(schema_codes(&latest_diagnostics(&mut fixture, &uri(&document))), vec!["dm.schema.type_mismatch".to_string()]);

    write(&contract, "$schema:\n    port: string\n");
    changed(&fixture, &[(&contract, 2)]);
    assert_eq!(schema_codes(&latest_diagnostics(&mut fixture, &uri(&document))), Vec::<String>::new());

    std::fs::remove_file(&contract).unwrap();
    changed(&fixture, &[(&contract, 3)]);
    let diagnostics = latest_diagnostics(&mut fixture, &uri(&document));
    assert_eq!(schema_codes(&diagnostics).len(), 1, "a deleted contract is a prepare error: {diagnostics:#?}");
    fixture.shutdown();
}

#[test]
fn each_workspace_folder_selects_from_its_own_definitions() {
    let workspace = LspWorkspace::new();
    let first = workspace.path().join("first");
    let second = workspace.path().join("second");
    for (folder, owner) in [(&first, "string"), (&second, "number")] {
        init_repository(folder);
        write(&folder.join("schemas/roster.yaml"), &ROSTER_DEFINITION.replace("owner: string", &format!("owner: {owner}")));
        write(&folder.join("team.yaml"), ROSTER);
    }
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(&[&first, &second]));

    open(&fixture, &uri(&first.join("team.yaml")), ROSTER);
    open(&fixture, &uri(&second.join("team.yaml")), ROSTER);
    assert_eq!(schema_codes(&latest_diagnostics(&mut fixture, &uri(&first.join("team.yaml")))), Vec::<String>::new());
    assert_eq!(
        schema_codes(&latest_diagnostics(&mut fixture, &uri(&second.join("team.yaml")))),
        vec!["dm.schema.type_mismatch".to_string()]
    );
    fixture.shutdown();
}

#[test]
fn a_yaml_documents_kinds_are_prepared_off_the_dispatch_thread() {
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    init_repository(root);
    write(&root.join("schemas/roster.yaml"), ROSTER_DEFINITION);
    let document = root.join("team.yaml");
    write(&document, ROSTER);
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(&[root]));

    open(&fixture, &uri(&document), ROSTER);
    let dispatch = fixture.server_thread_id();
    assert_ne!(fixture.wait_for_registry_prepared(RegistryJobKind::Document), dispatch);
    let threads = registry_scan_threads();
    assert!(!threads.contains(&dispatch), "a registry scan ran on the dispatch thread: {threads:?}");
    // The prepared kinds judge the document: an error proves the
    // definition was found by that preparation.
    edit(&fixture, &uri(&document), 2, &ROSTER.replace("roster_version: 1", "roster_version: 2"));
    assert_eq!(schema_codes(&latest_diagnostics(&mut fixture, &uri(&document))), vec!["dm.schema.constraint".to_string()]);
    fixture.shutdown();
}

/// A roster, an inline-fallback service, and a referenced-fallback service,
/// each with `owner` as `owner`, beside the definition and contract they
/// select.
fn contract_documents(root: &Path, owner: &str) -> Vec<(std::path::PathBuf, String)> {
    write(&root.join("schemas/roster.yaml"), ROSTER_DEFINITION);
    write(&root.join("contract.yaml"), "$schema:\n    owner: string\n");
    let documents = vec![
        (root.join("team.yaml"), format!("kind: roster\nroster_version: 1\nowner: {owner}\n")),
        (root.join("inline.yaml"), format!("$schema:\n    owner: string\nowner: {owner}\n")),
        (root.join("service.yaml"), format!("$schema: ./contract.yaml\nowner: {owner}\n")),
    ];
    for (path, text) in &documents {
        write(path, text);
    }
    documents
}

/// Selecting, resolving, and compiling a YAML document's contract runs on
/// the registry worker for a claimed kind, an inline `$schema`, and a
/// referenced one, and so does the import resolution of a `kind: schema`
/// document (its kind checks and its authoring state), on its first open and
/// after an edit. Each document's diagnostics prove its prepared contract
/// judged it.
#[test]
fn a_yaml_documents_contract_is_prepared_and_compiled_off_the_dispatch_thread() {
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    init_repository(root);
    let documents = contract_documents(root, "[ops]");
    write(&root.join("shared/types.yaml"), &type_library("string"));
    let library_user = root.join("owners.yaml");
    let library_text = "kind: schema\n$schema:\n    owner: Owner@./shared/types.yaml\n";
    write(&library_user, library_text);
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(&[root]));
    let before = library_work::snapshot();

    for (path, text) in &documents {
        open(&fixture, &uri(path), text);
        let diagnostics = latest_diagnostics(&mut fixture, &uri(path));
        assert_eq!(schema_codes(&diagnostics), vec!["dm.schema.type_mismatch".to_string()], "{}: {diagnostics:#?}", path.display());
    }
    open(&fixture, &uri(&library_user), library_text);
    assert_eq!(schema_codes(&latest_diagnostics(&mut fixture, &uri(&library_user))), Vec::<String>::new());
    // An edit of the schema document is judged again, on the worker.
    edit(&fixture, &uri(&library_user), 2, &library_text.replace("Owner@", "Team@"));
    let diagnostics = latest_diagnostics(&mut fixture, &uri(&library_user));
    assert!(
        diagnostics.iter().any(|diagnostic| diagnostic["message"].as_str().unwrap_or_default().contains("unknown named type `Team`")),
        "{diagnostics:#?}"
    );

    let prepared = library_work::snapshot().since(&before);
    assert!(prepared.get(Work::YamlContractPreparation) >= 4, "{:?}", prepared.nonzero());
    let dispatch = fixture.server_thread_id();
    for work in [Work::YamlContractPreparation, Work::YamlKindCheck, Work::StandaloneImportCheck] {
        let threads = library_work::threads(work);
        assert!(!threads.is_empty(), "{work:?} never ran");
        assert!(!threads.contains(&dispatch), "{work:?} ran on the dispatch thread: {threads:?}");
    }
    let threads = yaml_preparation_threads();
    assert!(!threads.is_empty() && !threads.contains(&dispatch), "{threads:?}");
    fixture.shutdown();
}

/// Editing only a document's data, with its `kind`, `$schema`, context, and
/// contract sources unchanged, judges the new data against the contract
/// prepared before: no file is read and nothing is prepared, resolved, or
/// compiled, for a claimed kind and both fallback routes, while the
/// diagnostics follow the data. Changing the declaration prepares again.
#[test]
fn a_data_only_edit_reuses_the_prepared_contract_and_reads_no_schema_file() {
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    init_repository(root);
    let documents = contract_documents(root, "ops");
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(&[root]));
    let mismatch = vec!["dm.schema.type_mismatch".to_string()];

    for (path, text) in &documents {
        open(&fixture, &uri(path), text);
        assert_eq!(schema_codes(&latest_diagnostics(&mut fixture, &uri(path))), Vec::<String>::new(), "{}", path.display());
        let (dmls_before, library_before) = (work_counts(), library_work::snapshot());

        edit(&fixture, &uri(path), 2, &text.replace("owner: ops", "owner: [ops]"));
        let diagnostics = latest_diagnostics(&mut fixture, &uri(path));
        assert_eq!(schema_codes(&diagnostics), mismatch, "{}: {diagnostics:#?}", path.display());
        edit(&fixture, &uri(path), 3, text);
        assert_eq!(schema_codes(&latest_diagnostics(&mut fixture, &uri(path))), Vec::<String>::new(), "{}", path.display());

        let dmls_work = work_counts().since(dmls_before);
        assert_eq!(
            (dmls_work.reads, dmls_work.yaml_preparations, dmls_work.registry_scans, dmls_work.schema_target_inspections),
            (0, 0, 0, 0),
            "{}: {dmls_work:?}",
            path.display()
        );
        let library = library_work::snapshot().since(&library_before);
        for work in [Work::YamlContractPreparation, Work::ValidatorCompile, Work::NamedTypeTableParse, Work::EffectivePreparation] {
            assert_eq!(library.get(work), 0, "{}: {work:?} in {:?}", path.display(), library.nonzero());
        }
    }

    // Control: a changed declaration is prepared again, and judges.
    let (path, _) = &documents[2];
    let before = work_counts();
    edit(&fixture, &uri(path), 4, "$schema:\n    owner: number\nowner: ops\n");
    assert_eq!(schema_codes(&latest_diagnostics(&mut fixture, &uri(path))), mismatch);
    assert!(work_counts().since(before).yaml_preparations >= 1, "a new declaration is prepared");
    fixture.shutdown();
}

/// A document holding only its root `$schema` is judged as data against
/// that schema, as `md schema validate` judges it: deleting its last data
/// property leaves the required property missing rather than hiding the
/// judgment. Data keys stay completed and hovered from the selected schema,
/// and the declaration inside `$schema` keeps its schema-authoring completion
/// and hover.
#[test]
fn deleting_the_last_data_property_of_a_root_schema_document_reports_the_missing_property() {
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    let schema = "$schema:\n    owner: string(required) -> the owning team\n";
    let with_data = format!("{schema}owner: [ops]\n");
    let document = root.join("team.yaml");
    write(&document, &with_data);
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(&[root]));

    open(&fixture, &uri(&document), &with_data);
    let diagnostics = latest_diagnostics(&mut fixture, &uri(&document));
    assert_eq!(schema_codes(&diagnostics), vec!["dm.schema.type_mismatch".to_string()], "{diagnostics:#?}");
    let body = hover(&mut fixture, &uri(&document), 2, 2);
    assert!(body.contains("the owning team"), "a data key hovers its selected declaration: {body}");

    edit(&fixture, &uri(&document), 2, schema);
    let diagnostics = latest_diagnostics(&mut fixture, &uri(&document));
    assert_eq!(schema_codes(&diagnostics), vec!["dm.schema.missing_required".to_string()], "{diagnostics:#?}");
    assert_eq!(diagnostics[0]["severity"], json!(1), "{diagnostics:#?}");
    assert!(diagnostics[0]["message"].as_str().is_some_and(|message| message.contains("owner")), "{diagnostics:#?}");

    let labels = completion_labels(&mut fixture, &uri(&document), 2);
    assert!(labels.contains(&"owner".to_string()), "the selected schema's data keys are offered: {labels:?}");
    let response = fixture.request(
        "textDocument/completion",
        json!({ "textDocument": { "uri": uri(&document) }, "position": { "line": 1, "character": 11 } }),
    );
    let items = response.result.unwrap_or(Value::Null);
    let items = items.get("items").cloned().unwrap_or(items);
    assert!(
        items.as_array().is_some_and(|items| items.iter().any(|item| item["label"] == "string")),
        "the declaration keeps the type grammar: {items:?}"
    );
    let body = hover(&mut fixture, &uri(&document), 1, 6);
    assert!(body.contains("string"), "the declaration keeps its authoring hover: {body}");
    fixture.shutdown();
}

/// An invalid declaration inside a root-`$schema` document's `$schema` is
/// the reason its route cannot prepare; it is published once, at its token,
/// as in Markdown frontmatter.
#[test]
fn an_invalid_declaration_of_a_root_schema_document_is_published_once() {
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    let text = "$schema:\n    owner: string(bogus(1))\n";
    let document = root.join("team.yaml");
    write(&document, text);
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(&[root]));

    open(&fixture, &uri(&document), text);
    let diagnostics = latest_diagnostics(&mut fixture, &uri(&document));
    assert_eq!(schema_codes(&diagnostics), vec!["dm.schema.invalid_type_definition".to_string()], "{diagnostics:#?}");
    assert_eq!(diagnostics[0]["range"]["start"], json!({ "line": 1, "character": 18 }), "{diagnostics:#?}");

    // A buffer that does not load is one parse problem, the loader's.
    edit(&fixture, &uri(&document), 2, "$schema:\n    owner: [\n");
    let diagnostics = latest_diagnostics(&mut fixture, &uri(&document));
    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    fixture.shutdown();
}

/// A bare root `$schema` searches the schema roots only for a document inside
/// a workspace folder, for a YAML document exactly as for Markdown
/// frontmatter; outside every folder the bare name is a file next to the
/// document, and the schema's path still names it.
#[test]
fn a_bare_root_schema_searches_the_schema_roots_only_inside_a_workspace_folder() {
    let workspace = LspWorkspace::new();
    let folder = workspace.path().join("folder");
    let loose = workspace.path().join("loose");
    let contract = workspace.path().join("home/schemas/contract.yaml");
    write(&contract, "kind: schema\n$schema:\n    owner: string(required)\n");
    let bare = "$schema: contract.yaml\nowner: ops\n";
    let explicit = format!("$schema: {}\nowner: ops\n", contract.display());
    let documents = [
        (folder.join("service.yaml"), bare.to_string()),
        (loose.join("service.yaml"), bare.to_string()),
        (loose.join("service.md"), format!("---\n{bare}---\n")),
        (loose.join("explicit.yaml"), explicit),
    ];
    for (path, text) in &documents {
        write(path, text);
    }
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(&[&folder]));

    let codes: Vec<Vec<String>> = documents
        .iter()
        .map(|(path, text)| {
            let language = if path.extension().is_some_and(|extension| extension == "md") { "markdown" } else { "yaml" };
            fixture.notify(
                "textDocument/didOpen",
                json!({ "textDocument": { "uri": uri(path), "languageId": language, "version": 1, "text": text } }),
            );
            schema_codes(&latest_diagnostics(&mut fixture, &uri(path)))
        })
        .collect();
    assert_eq!(codes[0], Vec::<String>::new(), "inside the folder `~/schemas` is searched");
    let prepare = "dm.schema.prepare".to_string();
    assert_eq!(codes[1], vec![prepare.clone()], "outside every folder no root is searched");
    assert!(codes[2].contains(&prepare), "a Markdown document outside every folder resolves the same way: {codes:?}");
    assert_eq!(codes[3], Vec::<String>::new(), "an explicit path resolves anywhere");
    fixture.shutdown();
}

/// A roster definition whose `owner` is the named type `Owner` that
/// `reference` imports.
fn importing_definition(reference: &str) -> String {
    ROSTER_DEFINITION.replace("owner: string", &format!("owner: {reference}"))
}

/// A normal `kind: schema` type library declaring `Owner` as `owner_type`.
fn type_library(owner_type: &str) -> String {
    format!("kind: schema\ntypes:\n    Owner: {owner_type}\n")
}

/// One unsaved-source probe: `dependent` is valid while `source` holds
/// `saved`; an unsaved edit to `edited` makes `owner` a type mismatch (and
/// the hover of `owner` names `number`), closing the buffer unsaved restores
/// the saved result, and saving `edited` with a watched change gives the
/// same mismatch the buffer gave.
fn assert_unsaved_source_refreshes(
    fixture: &mut LspFixture<'_>,
    dependent: &Path,
    dependent_text: &str,
    owner_line: u32,
    source: &Path,
    saved: &str,
    edited: &str,
) {
    let mismatch = vec!["dm.schema.type_mismatch".to_string()];
    open(fixture, &uri(dependent), dependent_text);
    assert_eq!(schema_codes(&latest_diagnostics(fixture, &uri(dependent))), Vec::<String>::new(), "saved control");
    let saved_hover = hover(fixture, &uri(dependent), owner_line, 2);
    assert!(saved_hover.contains("string") && !saved_hover.contains("number"), "{saved_hover}");

    open(fixture, &uri(source), saved);
    edit(fixture, &uri(source), 2, edited);
    let diagnostics = latest_diagnostics(fixture, &uri(dependent));
    assert_eq!(schema_codes(&diagnostics), mismatch, "the unsaved source judges the dependent: {diagnostics:#?}");
    let unsaved_hover = hover(fixture, &uri(dependent), owner_line, 2);
    assert!(unsaved_hover.contains("number"), "hover reads the unsaved contract: {unsaved_hover}");

    fixture.notify("textDocument/didClose", json!({ "textDocument": { "uri": uri(source) } }));
    assert_eq!(schema_codes(&latest_diagnostics(fixture, &uri(dependent))), Vec::<String>::new(), "closed unsaved");
    let restored_hover = hover(fixture, &uri(dependent), owner_line, 2);
    assert!(!restored_hover.contains("number"), "{restored_hover}");

    write(source, edited);
    changed(fixture, &[(source, 2)]);
    assert_eq!(schema_codes(&latest_diagnostics(fixture, &uri(dependent))), mismatch, "saved edit, same result");
}

#[test]
fn an_unsaved_edit_of_a_type_library_in_a_schema_root_refreshes_the_definitions_dependents() {
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    init_repository(root);
    write(&root.join("schemas/roster.yaml"), &importing_definition("Owner@types.yaml"));
    let library = root.join("schemas/types.yaml");
    write(&library, &type_library("string"));
    let document = root.join("team.yaml");
    write(&document, ROSTER);
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(&[root]));

    assert_unsaved_source_refreshes(
        &mut fixture,
        &document,
        ROSTER,
        2,
        &library,
        &type_library("string"),
        &type_library("number"),
    );
    fixture.shutdown();
}

#[test]
fn an_unsaved_edit_of_a_type_library_outside_the_schema_roots_refreshes_the_definitions_dependents() {
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    init_repository(root);
    write(&root.join("schemas/roster.yaml"), &importing_definition("Owner@../shared/types.yaml"));
    let library = root.join("shared/types.yaml");
    write(&library, &type_library("string"));
    let document = root.join("team.yaml");
    write(&document, ROSTER);
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(&[root]));

    assert_unsaved_source_refreshes(
        &mut fixture,
        &document,
        ROSTER,
        2,
        &library,
        &type_library("string"),
        &type_library("number"),
    );
    fixture.shutdown();
}

#[test]
fn an_unsaved_edit_of_an_explicit_root_schema_target_refreshes_the_document_that_names_it() {
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    init_repository(root);
    let contract = root.join("contract.yaml");
    let saved = "$schema:\n    owner: string\n";
    write(&contract, saved);
    let document = root.join("service.yaml");
    let text = "$schema: ./contract.yaml\nowner: ops\n";
    write(&document, text);
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(&[root]));

    assert_unsaved_source_refreshes(&mut fixture, &document, text, 1, &contract, saved, &saved.replace("string", "number"));
    fixture.shutdown();
}

#[test]
fn an_unsaved_new_definition_in_a_schema_root_claims_its_kind_until_it_closes() {
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    init_repository(root);
    write(&root.join("schemas/notes.yaml"), "just: data\n");
    let document = root.join("team.yaml");
    write(&document, ROSTER);
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(&[root]));

    // No definition claims `roster` yet: the document is not validated.
    open(&fixture, &uri(&document), ROSTER);
    assert_eq!(schema_codes(&latest_diagnostics(&mut fixture, &uri(&document))), Vec::<String>::new());
    assert!(hover(&mut fixture, &uri(&document), 1, 2).is_empty(), "saved control: unclaimed");

    // A definition that exists only in the editor claims the kind.
    let definition = root.join("schemas/roster.yaml");
    assert!(!definition.exists());
    open(&fixture, &uri(&definition), &ROSTER_DEFINITION.replace("owner: string", "owner: number"));
    let diagnostics = latest_diagnostics(&mut fixture, &uri(&document));
    assert_eq!(schema_codes(&diagnostics), vec!["dm.schema.type_mismatch".to_string()], "{diagnostics:#?}");
    let body = hover(&mut fixture, &uri(&document), 1, 2);
    assert!(body.contains("roster_version"), "{body}");

    // Closing it unsaved removes the claim.
    fixture.notify("textDocument/didClose", json!({ "textDocument": { "uri": uri(&definition) } }));
    assert_eq!(schema_codes(&latest_diagnostics(&mut fixture, &uri(&document))), Vec::<String>::new());
    assert!(hover(&mut fixture, &uri(&document), 1, 2).is_empty(), "no stale claim");
    fixture.shutdown();
}

#[test]
fn an_unsaved_file_at_a_missing_root_schema_target_is_selected_until_it_closes() {
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    init_repository(root);
    let contract = root.join("contract.yaml");
    let document = root.join("service.yaml");
    let text = "$schema: ./contract.yaml\nowner: ops\n";
    write(&document, text);
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(&[root]));

    open(&fixture, &uri(&document), text);
    let prepare = vec!["dm.schema.prepare".to_string()];
    assert_eq!(schema_codes(&latest_diagnostics(&mut fixture, &uri(&document))), prepare, "saved control: missing");

    open(&fixture, &uri(&contract), "$schema:\n    owner: number\n");
    let diagnostics = latest_diagnostics(&mut fixture, &uri(&document));
    assert_eq!(schema_codes(&diagnostics), vec!["dm.schema.type_mismatch".to_string()], "{diagnostics:#?}");

    fixture.notify("textDocument/didClose", json!({ "textDocument": { "uri": uri(&contract) } }));
    assert_eq!(schema_codes(&latest_diagnostics(&mut fixture, &uri(&document))), prepare, "closed unsaved");
    fixture.shutdown();
}

#[test]
fn an_unsaved_file_at_a_missing_import_of_a_definition_is_read_until_it_closes() {
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    init_repository(root);
    write(&root.join("schemas/roster.yaml"), &importing_definition("Owner@../shared/types.yaml"));
    let document = root.join("team.yaml");
    write(&document, ROSTER);
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(&[root]));

    open(&fixture, &uri(&document), ROSTER);
    let missing = schema_codes(&latest_diagnostics(&mut fixture, &uri(&document)));
    assert!(!missing.is_empty(), "saved control: the import is missing");

    let library = root.join("shared/types.yaml");
    open(&fixture, &uri(&library), &type_library("number"));
    let diagnostics = latest_diagnostics(&mut fixture, &uri(&document));
    assert_eq!(schema_codes(&diagnostics), vec!["dm.schema.type_mismatch".to_string()], "{diagnostics:#?}");

    fixture.notify("textDocument/didClose", json!({ "textDocument": { "uri": uri(&library) } }));
    assert_eq!(schema_codes(&latest_diagnostics(&mut fixture, &uri(&document))), missing, "closed unsaved");
    fixture.shutdown();
}

/// The `schema` kind's semantic checks resolve a `kind: schema` document's
/// own `Name@file` imports, so an unsaved edit of the imported type library
/// that drops the named type breaks the document before the library is
/// saved, and closing the library unsaved restores it.
#[test]
fn an_unsaved_edit_of_an_imported_type_library_refreshes_a_schema_documents_kind_checks() {
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    init_repository(root);
    let library = root.join("shared/types.yaml");
    write(&library, &type_library("string"));
    let document = root.join("contract.yaml");
    let text = "kind: schema\n$schema:\n    owner: Owner@./shared/types.yaml\n";
    write(&document, text);
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(&[root]));

    open(&fixture, &uri(&document), text);
    assert_eq!(schema_codes(&latest_diagnostics(&mut fixture, &uri(&document))), Vec::<String>::new(), "saved control");

    open(&fixture, &uri(&library), &type_library("string"));
    edit(&fixture, &uri(&library), 2, "kind: schema\ntypes:\n    Team: string\n");
    let diagnostics = latest_diagnostics(&mut fixture, &uri(&document));
    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0]["severity"], json!(1), "{diagnostics:#?}");
    assert_eq!(diagnostics[0]["range"]["start"]["line"], json!(2), "ranged at the import: {diagnostics:#?}");
    let message = diagnostics[0]["message"].as_str().unwrap_or_default();
    assert!(message.contains("unknown named type `Owner`"), "{diagnostics:#?}");

    fixture.notify("textDocument/didClose", json!({ "textDocument": { "uri": uri(&library) } }));
    assert_eq!(schema_codes(&latest_diagnostics(&mut fixture, &uri(&document))), Vec::<String>::new(), "closed unsaved");
    fixture.shutdown();
}

/// An open schema document's own authoring state (its named-type imports,
/// their hover, and its envelope's resolution) reads an imported type
/// library's unsaved text, for a `kind: schema` document and a root-`$schema`
/// contract alike, and closing the library unsaved restores the saved text.
#[test]
fn an_unsaved_edit_of_an_imported_type_library_refreshes_an_open_schema_documents_authoring_state() {
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    init_repository(root);
    let library = root.join("shared/types.yaml");
    write(&library, &type_library("string"));
    let documents = [
        (root.join("contract.yaml"), "kind: schema\n$schema:\n    owner: Owner@./shared/types.yaml\n", 2),
        (root.join("informal.yaml"), "$schema:\n    owner: Owner@./shared/types.yaml\n", 1),
    ];
    for (path, text, _) in &documents {
        write(path, text);
    }
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(&[root]));
    let declares = |fixture: &mut LspFixture<'_>, path: &Path, line: u32| {
        let body = hover(fixture, &uri(path), line, 16);
        assert!(body.contains("**`Owner`**"), "{body}");
        body.split("Declares: ").nth(1).and_then(|rest| rest.lines().next()).unwrap_or_default().to_string()
    };

    for (path, text, line) in &documents {
        open(&fixture, &uri(path), text);
        assert_eq!(schema_codes(&latest_diagnostics(&mut fixture, &uri(path))), Vec::<String>::new(), "saved control");
        assert_eq!(declares(&mut fixture, path, *line), "**string**");
    }

    open(&fixture, &uri(&library), &type_library("string"));
    edit(&fixture, &uri(&library), 2, &type_library("number"));
    for (path, _, line) in &documents {
        assert_eq!(declares(&mut fixture, path, *line), "**number**", "{}", path.display());
    }

    edit(&fixture, &uri(&library), 3, "kind: schema\ntypes:\n    Team: string\n");
    // Each locates the broken import at its reference: the envelope of the
    // schema document, the route of the root-`$schema` contract.
    for (path, _, line) in &documents {
        let diagnostics = latest_diagnostics(&mut fixture, &uri(path));
        assert_eq!(schema_codes(&diagnostics), vec!["dm.schema.invalid_type_definition".to_string()], "{diagnostics:#?}");
        assert_eq!(diagnostics[0]["range"]["start"], json!({ "line": line, "character": 11 }), "{diagnostics:#?}");
        let message = diagnostics[0]["message"].as_str().unwrap_or_default();
        assert!(message.contains("unknown named type `Owner`"), "{diagnostics:#?}");
    }

    fixture.notify("textDocument/didClose", json!({ "textDocument": { "uri": uri(&library) } }));
    for (path, _, line) in &documents {
        assert_eq!(schema_codes(&latest_diagnostics(&mut fixture, &uri(path))), Vec::<String>::new(), "closed unsaved");
        assert_eq!(declares(&mut fixture, path, *line), "**string**");
    }
    fixture.shutdown();
}

/// A Draft 2020-12 JSON Schema contract that `owner` is a value of `owner_type`.
fn json_contract(owner_type: &str) -> String {
    format!(
        "{{\"$schema\": \"https://json-schema.org/draft/2020-12/schema\", \"type\": \"object\", \
         \"properties\": {{\"owner\": {{\"type\": \"{owner_type}\"}}}}}}\n"
    )
}

#[test]
fn an_unsaved_edit_of_a_json_schema_target_refreshes_the_document_that_names_it() {
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    init_repository(root);
    let contract = root.join("contract.json");
    write(&contract, &json_contract("string"));
    let document = root.join("service.yaml");
    let text = "$schema: ./contract.json\nowner: ops\n";
    write(&document, text);
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(&[root]));

    // Raw JSON Schema has no SimplifiedSchema hover, so only the judgment is
    // compared (the YAML targets' probe also compares hover).
    let mismatch = vec!["dm.schema.type_mismatch".to_string()];
    open(&fixture, &uri(&document), text);
    assert_eq!(schema_codes(&latest_diagnostics(&mut fixture, &uri(&document))), Vec::<String>::new(), "saved control");

    open(&fixture, &uri(&contract), &json_contract("string"));
    edit(&fixture, &uri(&contract), 2, &json_contract("number"));
    let diagnostics = latest_diagnostics(&mut fixture, &uri(&document));
    assert_eq!(schema_codes(&diagnostics), mismatch, "the unsaved contract judges the document: {diagnostics:#?}");

    fixture.notify("textDocument/didClose", json!({ "textDocument": { "uri": uri(&contract) } }));
    assert_eq!(schema_codes(&latest_diagnostics(&mut fixture, &uri(&document))), Vec::<String>::new(), "closed unsaved");

    write(&contract, &json_contract("number"));
    changed(&fixture, &[(&contract, 2)]);
    assert_eq!(schema_codes(&latest_diagnostics(&mut fixture, &uri(&document))), mismatch, "saved edit, same result");
    fixture.shutdown();
}

/// A schema root that does not exist yet is a searched missing location: a
/// definition opened in the editor where that root would be claims its kind
/// before the folder or the file is saved, and closing it unsaved removes
/// the claim.
#[test]
fn an_unsaved_definition_in_a_schema_root_folder_that_does_not_exist_claims_its_kind_until_it_closes() {
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    init_repository(root);
    let document = root.join("team.yaml");
    write(&document, ROSTER);
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(&[root]));

    open(&fixture, &uri(&document), ROSTER);
    assert_eq!(schema_codes(&latest_diagnostics(&mut fixture, &uri(&document))), Vec::<String>::new());
    assert!(hover(&mut fixture, &uri(&document), 1, 2).is_empty(), "saved control: unclaimed");

    let definition = root.join("schemas/roster.yaml");
    assert!(!definition.parent().unwrap().exists(), "the schema root is absent");
    open(&fixture, &uri(&definition), &ROSTER_DEFINITION.replace("owner: string", "owner: number"));
    let diagnostics = latest_diagnostics(&mut fixture, &uri(&document));
    assert_eq!(schema_codes(&diagnostics), vec!["dm.schema.type_mismatch".to_string()], "{diagnostics:#?}");
    let body = hover(&mut fixture, &uri(&document), 1, 2);
    assert!(body.contains("roster_version"), "{body}");

    fixture.notify("textDocument/didClose", json!({ "textDocument": { "uri": uri(&definition) } }));
    assert_eq!(schema_codes(&latest_diagnostics(&mut fixture, &uri(&document))), Vec::<String>::new());
    assert!(hover(&mut fixture, &uri(&document), 1, 2).is_empty(), "no stale claim");
    fixture.shutdown();
}

/// A document with no claimed `kind` is data of its root `$schema`, even when
/// its fields are named like a schema document's metadata (`description`,
/// `categories`, `types`): over the normal protocol, every value the selected
/// contract accepts is clean (inline, referenced, and root-union contracts),
/// a value it rejects is its one problem, and the declaration inside
/// `$schema` keeps its authoring hover.
#[test]
fn metadata_named_fields_of_a_root_schema_document_are_judged_by_its_contract_alone() {
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    write(&root.join("contract.yaml"), "$schema:\n    description: string\n");
    let document = root.join("data.yaml");
    let first = "$schema:\n    description: string\ndescription: null\n";
    write(&document, first);
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(&[root]));

    open(&fixture, &uri(&document), first);
    assert_eq!(latest_diagnostics(&mut fixture, &uri(&document)), Vec::<Value>::new(), "{first}");
    let body = hover(&mut fixture, &uri(&document), 1, 6);
    assert!(body.contains("string"), "the declaration keeps its authoring hover: {body}");

    let clean = [
        "$schema:\n    description: string\n    owner: string\ndescription: null\nowner: ops\n",
        "$schema:\n    categories: string[]\ncategories: [ops, ops]\n",
        "$schema:\n    types: string[]\ntypes: [ops]\n",
        "$schema:\n    types: unknown\ntypes: { mode: production }\n",
        "$schema: ./contract.yaml\ndescription: null\n",
        "$schema:\n    - { description: string, kind: literal(a) }\n    - { description: string, kind: literal(b) }\ndescription: null\n",
    ];
    for (version, text) in (2..).zip(clean) {
        edit(&fixture, &uri(&document), version, text);
        assert_eq!(latest_diagnostics(&mut fixture, &uri(&document)), Vec::<Value>::new(), "{text}");
    }

    edit(&fixture, &uri(&document), 20, "$schema:\n    categories: number[]\ncategories: [ops]\n");
    let diagnostics = latest_diagnostics(&mut fixture, &uri(&document));
    assert_eq!(schema_codes(&diagnostics), vec!["dm.schema.type_mismatch".to_string()], "{diagnostics:#?}");
    fixture.shutdown();
}

/// A file in a schema root whose `kind` is being typed claims nothing: the
/// unfinished `kind:` publishes nothing, a missing root `$schema` is its one
/// reference error, and a valid one leaves the file clean.
#[test]
fn an_unfinished_kind_in_a_schema_root_publishes_only_whole_document_results() {
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    init_repository(root);
    let document = root.join("schemas/data.yaml");
    write(&document, "owner: ops\n");
    let mut fixture = LspFixture::start(&workspace);
    fixture.initialize(watched_params(&[root]));

    open(&fixture, &uri(&document), "owner: ops\n");
    assert_eq!(latest_diagnostics(&mut fixture, &uri(&document)), Vec::<Value>::new(), "control");

    edit(&fixture, &uri(&document), 2, "kind:\nowner: ops\n");
    assert_eq!(latest_diagnostics(&mut fixture, &uri(&document)), Vec::<Value>::new(), "`kind:`");

    edit(&fixture, &uri(&document), 3, "kind:\n$schema: ./missing.yaml\nowner: ops\n");
    let diagnostics = latest_diagnostics(&mut fixture, &uri(&document));
    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0]["code"], "dm.schema.prepare", "{diagnostics:#?}");
    assert_eq!(diagnostics[0]["range"]["start"], json!({ "line": 1, "character": 9 }), "{diagnostics:#?}");

    edit(&fixture, &uri(&document), 4, "kind:\n$schema:\n    owner: string\nowner: ops\n");
    assert_eq!(latest_diagnostics(&mut fixture, &uri(&document)), Vec::<Value>::new(), "valid fallback");
    fixture.shutdown();
}

/// A checkout copy of the shipped `schema` definition with its root closure
/// removed, selected over the normal protocol: a key it admits is clean, as
/// it is through `md schema validate`.
#[test]
fn an_opened_checkout_schema_definition_admits_an_added_key_in_the_editor() {
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    init_repository(root);
    let shipped = include_str!("../../../../simplified-schema/schemas/schema.yaml");
    let opened: String =
        shipped.lines().filter(|line| line.trim() != "$constraints: { closed: true }").map(|line| format!("{line}\n")).collect();
    assert_ne!(opened, shipped, "the definition had a root closure to remove");
    let copy = root.join("simplified-schema/schemas/schema.yaml");
    write(&copy, &opened);
    let document = root.join("docs/contract.yaml");
    let control = "kind: schema\n$schema:\n    title: string\n";
    write(&document, control);
    let snapshot = LspFixture::fixture_snapshot(&workspace).with_env(std::collections::HashMap::from([(
        "SCHEMAS_DIR".to_string(),
        copy.parent().unwrap().to_string_lossy().into_owned(),
    )]));
    let mut fixture = LspFixture::start_with_snapshot(&workspace, snapshot);
    fixture.initialize(watched_params(&[root]));

    open(&fixture, &uri(&document), control);
    assert_eq!(latest_diagnostics(&mut fixture, &uri(&document)), Vec::<Value>::new(), "control");
    edit(&fixture, &uri(&document), 2, &format!("{control}note: hi\n"));
    assert_eq!(latest_diagnostics(&mut fixture, &uri(&document)), Vec::<Value>::new(), "added `note`");
    // The selected copy's checks still run: an undeclared category is an
    // error.
    edit(&fixture, &uri(&document), 3, "kind: schema\ncategories: [text]\ntypes:\n    title: string(category(media))\nnote: hi\n");
    let diagnostics = latest_diagnostics(&mut fixture, &uri(&document));
    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert!(diagnostics[0]["message"].as_str().is_some_and(|message| message.contains("media")), "{diagnostics:#?}");
    fixture.shutdown();
}
