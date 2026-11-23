//! DMLS watches every passive input of the host registrations (both
//! configuration files, present or absent, each manifest, member, import,
//! and JSON artifact, and the repository's `HEAD`) and follows a change to
//! any of them without a restart: a deleted, recreated, or atomically
//! replaced member, a `register` entry added or removed, a manifest that
//! appears where an entry pointed. Registration warnings are published on
//! the file they name, beside that file's own diagnostics, and a missing
//! user configuration is one session notice that creates nothing.
//!
//! Every session's home is a folder of its temporary workspace (the shared
//! relay install from Darkmatter's host fixtures); nothing is read from the
//! real home or checkout.

use std::path::Path;

use serde_json::{Value, json};

use crate::common::{LspFixture, LspWorkspace};
use crate::host_fixtures::{Bundle, Fixture, MANIFEST_NAME, WIDGET_TRIGGER};
use crate::registered_host_views::{
    MOTIVATING, REGISTRATION, completion_at, install_relay, labels, repository, root_catalog, with_code, write,
};
use crate::union_item_editor::{Document, offset_of, open_text, request};

pub(crate) const CREATED: u32 = 1;
pub(crate) const CHANGED: u32 = 2;
pub(crate) const DELETED: u32 = 3;

/// A client that watches files and roots watchers at folders.
fn watching_params(root: &Path) -> Value {
    json!({
        "processId": null,
        "clientInfo": { "name": "Watched Client", "version": "1.0" },
        "capabilities": {
            "general": { "positionEncodings": ["utf-16"] },
            "workspace": { "didChangeWatchedFiles": { "dynamicRegistration": true, "relativePatternSupport": true } }
        },
        "workspaceFolders": [{ "uri": url::Url::from_directory_path(root).unwrap().as_str(), "name": "scratch" }]
    })
}

pub(crate) fn start(workspace: &LspWorkspace) -> LspFixture<'_> {
    let mut fixture = LspFixture::start(workspace);
    fixture.initialize(watching_params(workspace.path()));
    fixture
}

pub(crate) fn uri(path: &Path) -> String {
    url::Url::from_file_path(path).unwrap().to_string()
}

/// Reports `changes` as one `didChangeWatchedFiles` notification, then waits
/// until the server handled it and finished the preparations it queued.
pub(crate) fn report(fixture: &mut LspFixture<'_>, document: &Document, changes: &[(&Path, u32)]) {
    let changes: Vec<Value> = changes.iter().map(|(path, kind)| json!({ "uri": uri(path), "type": kind })).collect();
    fixture.notify("workspace/didChangeWatchedFiles", json!({ "changes": changes }));
    settle(fixture, document);
}

fn settle(fixture: &mut LspFixture<'_>, document: &Document) {
    fixture.flush_server_with("textDocument/documentSymbol", json!({ "textDocument": { "uri": document.uri } }));
}

fn offers_seq(fixture: &mut LspFixture<'_>, document: &Document) -> bool {
    completion_at(fixture, document, "{{ seq.na").contains(&"seq.name".to_string())
}

/// The registration warnings last published on the file whose URI ends with
/// `suffix` since the last call; `None` when nothing was published for it.
pub(crate) fn registration_on(fixture: &mut LspFixture<'_>, suffix: &str) -> Option<Vec<Value>> {
    fixture
        .take_latest_diagnostics_ending(suffix)
        .map(|diagnostics| with_code(&diagnostics, REGISTRATION).into_iter().cloned().collect())
}

/// The `(base URI, pattern)` of every relative watcher the server
/// registered so far under `id`.
fn watchers(fixture: &LspFixture<'_>, id: &str) -> Vec<(String, String)> {
    fixture
        .buffered_server_requests()
        .iter()
        .filter(|request| request.method == "client/registerCapability")
        .flat_map(|request| request.params["registrations"].as_array().cloned().unwrap_or_default())
        .filter(|registration| registration["id"] == json!(id))
        .flat_map(|registration| registration["registerOptions"]["watchers"].as_array().cloned().unwrap_or_default())
        .filter_map(|watcher| {
            let pattern = &watcher["globPattern"];
            Some((pattern["baseUri"].as_str()?.trim_end_matches('/').to_string(), pattern["pattern"].as_str()?.to_string()))
        })
        .collect()
}

fn watched(watchers: &[(String, String)], base_suffix: &str, pattern: &str) -> bool {
    watchers.iter().any(|(base, watched)| base.ends_with(base_suffix) && watched == pattern)
}

/// The configuration's folder, the manifest's, each member folder holding
/// YAML and JSON members, and the repository's `HEAD` and (still unborn)
/// branch ref are watched once the registrations are prepared.
#[test]
fn every_registration_input_is_watched_once_prepared() {
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    repository(root, &[".relay"]);
    install_relay(root, ".relay");
    let mut fixture = start(&workspace);
    let (document, _) = open_text(&mut fixture, &workspace, "notes/relay.md", MOTIVATING);
    settle(&mut fixture, &document);

    let folders = watchers(&fixture, "dmls-watch-schema-inputs");
    for (base, pattern) in [("", "home/*"), ("/home", ".relay/*"), ("/home/.relay", "schemas/*")] {
        assert!(watched(&folders, base, pattern), "{base} {pattern}: {folders:#?}");
    }
    let paths = watchers(&fixture, "dmls-schema-slot-watchers");
    assert!(watched(&paths, "/.git", "HEAD"), "{paths:#?}");
    assert!(watched(&paths, "/.git/refs/heads", "main"), "{paths:#?}");
    fixture.shutdown();
}

/// Deleting a member drops the host and reports the failure on the
/// manifest naming it; recreating it restores the host and clears the
/// warning, without a restart.
#[test]
fn a_deleted_then_recreated_member_updates_the_document() {
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    repository(root, &[".relay"]);
    install_relay(root, ".relay");
    let mut fixture = start(&workspace);
    let (document, _) = open_text(&mut fixture, &workspace, "notes/relay.md", MOTIVATING);
    assert!(offers_seq(&mut fixture, &document));

    let artifact = root.join("home/.relay/schemas/globals.lifecycle.json");
    let saved = std::fs::read_to_string(&artifact).unwrap();
    std::fs::remove_file(&artifact).unwrap();
    report(&mut fixture, &document, &[(&artifact, DELETED)]);
    assert!(!offers_seq(&mut fixture, &document), "the host is dropped");
    let published: Vec<Value> = ["home/.relay/darkmatter-host.yaml", "home/.relay/schemas/globals.lifecycle.json"]
        .into_iter()
        .filter_map(|suffix| registration_on(&mut fixture, suffix))
        .flatten()
        .collect();
    assert_eq!(published.len(), 1, "one warning, on the file it names: {published:#?}");

    std::fs::write(&artifact, saved).unwrap();
    report(&mut fixture, &document, &[(&artifact, CREATED)]);
    assert!(offers_seq(&mut fixture, &document), "the recreated member restores the host");
    let cleared: Vec<Value> = ["home/.relay/darkmatter-host.yaml", "home/.relay/schemas/globals.lifecycle.json"]
        .into_iter()
        .filter_map(|suffix| registration_on(&mut fixture, suffix))
        .flatten()
        .collect();
    assert!(cleared.is_empty(), "{cleared:#?}");
    fixture.shutdown();
}

/// An editor's atomic save (write a temporary file, rename it over the
/// member) is reported as the temporary file's creation and deletion and
/// the member's change: an invalid replacement drops the host, a valid one
/// restores it.
#[test]
fn an_atomic_replacement_of_a_member_is_followed() {
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    repository(root, &[".relay"]);
    install_relay(root, ".relay");
    let mut fixture = start(&workspace);
    let (document, _) = open_text(&mut fixture, &workspace, "notes/relay.md", MOTIVATING);
    assert!(offers_seq(&mut fixture, &document));

    let member = root.join("home/.relay/schemas/globals.lifecycle.json");
    let temporary = root.join("home/.relay/schemas/.globals.lifecycle.json.tmp");
    let valid = std::fs::read_to_string(&member).unwrap();
    for (text, offered) in [("{ \"not\": \"an artifact\" }\n", false), (valid.as_str(), true)] {
        std::fs::write(&temporary, text).unwrap();
        std::fs::rename(&temporary, &member).unwrap();
        report(&mut fixture, &document, &[(&temporary, CREATED), (&temporary, DELETED), (&member, CHANGED)]);
        assert_eq!(offers_seq(&mut fixture, &document), offered, "after replacing with {text:?}");
    }
    fixture.shutdown();
}

/// A `register` entry added to the user configuration registers the host;
/// removing it again unregisters it.
#[test]
fn adding_and_removing_a_register_entry_follows_the_configuration() {
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    repository(root, &[]);
    install_relay(root, ".relay");
    let mut fixture = start(&workspace);
    let (document, _) = open_text(&mut fixture, &workspace, "notes/relay.md", MOTIVATING);
    assert!(!offers_seq(&mut fixture, &document));

    let config = root.join("home/.darkmatter.yaml");
    write(root, "home/.darkmatter.yaml", "register:\n  - ./.relay/darkmatter-host.yaml\n");
    report(&mut fixture, &document, &[(&config, CREATED)]);
    assert!(offers_seq(&mut fixture, &document), "the new entry registers the relay");

    write(root, "home/.darkmatter.yaml", "register: []\n");
    report(&mut fixture, &document, &[(&config, CHANGED)]);
    assert!(!offers_seq(&mut fixture, &document), "the removed entry unregisters it");
    fixture.shutdown();
}

/// An entry naming a manifest that does not exist is reported on the
/// configuration file; installing the host where it pointed registers it
/// and clears the warning.
#[test]
fn a_previously_missing_manifest_registers_when_it_appears() {
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    repository(root, &[".relay"]);
    let mut fixture = start(&workspace);
    let (document, diagnostics) = open_text(&mut fixture, &workspace, "notes/relay.md", MOTIVATING);
    settle(&mut fixture, &document);
    assert!(with_code(&diagnostics, REGISTRATION).is_empty(), "never on the document: {diagnostics:#?}");
    let missing = registration_on(&mut fixture, "home/.darkmatter.yaml").expect("published on the configuration");
    assert_eq!(missing.len(), 1, "{missing:#?}");
    assert!(missing[0]["range"]["start"]["line"].as_u64().is_some());

    install_relay(root, ".relay");
    let manifest = root.join("home/.relay/darkmatter-host.yaml");
    report(&mut fixture, &document, &[(&manifest, CREATED)]);
    assert!(offers_seq(&mut fixture, &document), "the manifest appeared");
    assert_eq!(registration_on(&mut fixture, "home/.darkmatter.yaml"), Some(Vec::new()), "the warning is cleared");
    fixture.shutdown();
}

/// An open configuration file keeps its contract diagnostics and gets the
/// registration warning that names it, both in one publication.
#[test]
fn an_open_configuration_merges_its_contract_and_registration_diagnostics() {
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    repository(root, &[".relay"]);
    let mut fixture = start(&workspace);
    let (document, _) = open_text(&mut fixture, &workspace, "notes/relay.md", MOTIVATING);
    settle(&mut fixture, &document);

    // The unsaved buffer adds a value the contract rejects; the saved file
    // is what the registration load read.
    let config = root.join("home/.darkmatter.yaml");
    let text = format!("{}permissions:\n  network: sometimes\n", std::fs::read_to_string(&config).unwrap());
    fixture.notify(
        "textDocument/didOpen",
        json!({ "textDocument": { "uri": uri(&config), "languageId": "yaml", "version": 1, "text": text } }),
    );
    settle(&mut fixture, &document);
    let diagnostics = fixture.take_latest_diagnostics_ending("home/.darkmatter.yaml").expect("published");
    let codes: Vec<&str> = diagnostics.iter().filter_map(|diagnostic| diagnostic["code"].as_str()).collect();
    assert!(codes.contains(&"dm.schema.constraint"), "{diagnostics:#?}");
    assert!(codes.contains(&REGISTRATION), "{diagnostics:#?}");
    fixture.shutdown();
}

/// A document without frontmatter is analyzed under the same selection: no
/// registered trigger matches it, and a catalog with its lifecycle artifact
/// in the schema root beside it supplies no host, registered or not.
#[test]
fn a_document_without_frontmatter_gets_no_host_from_a_root_catalog() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    for registered in [&[".relay"][..], &[][..]] {
        let workspace = LspWorkspace::new();
        let root = workspace.path();
        repository(root, registered);
        install_relay(root, ".relay");
        root_catalog(root);
        let mut fixture = start(&workspace);
        let (document, _) = open_text(&mut fixture, &workspace, "plain.md", "{{ seq.na }}\n");
        let members = labels(&request(&mut fixture, "textDocument/completion", &document, offset_of(&document.text, "{{ seq.na", true)));
        assert!(!members.contains(&"seq.name".to_string()), "registered {registered:?}: {members:?}");
        fixture.shutdown();
    }
}

/// In-process: whether the `acme` host is registered for a document its
/// `widget` trigger matches, and every file the registration load depends
/// on.
fn acme_state(state: &dmls::overlay::OverlayState, fixture: &Fixture) -> (bool, std::collections::BTreeSet<std::path::PathBuf>) {
    use darkmatter::config::registration::HostSelection;
    use dmls::config::DmlsConfig;
    use dmls::context::DocumentResolution;

    let path = fixture.path("work/doc.md");
    let text = "---\nwidget:\n  size: 1\n---\n";
    fixture.write("work/doc.md", text);
    let resolution = DocumentResolution::from_context(fixture.context().for_source(&path));
    let uri: lsp_types::Uri = url::Url::from_file_path(&path).unwrap().as_str().parse().unwrap();
    let roots = [fixture.path("work")];
    let overlay = state.for_document(&uri, text, &path, &DmlsConfig::default(), &roots, &resolution).expect("frontmatter");
    let registered = matches!(overlay.host.as_deref().map(|view| view.selection()), Some(HostSelection::Registered(_)));
    (registered, state.host_dependencies())
}

/// A refused registration is watched where its repair will appear: the
/// manifest, every member the manifest lists, and the path of a member that
/// is missing. Restoring the missing member, or republishing a corrupted one
/// with its manifest re-addressed, loads the host without a restart.
#[test]
fn a_refused_registration_watches_its_manifest_and_members_and_loads_once_repaired() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let fixture = Fixture::new();
    fixture.write("home/.darkmatter.yaml", &format!("register:\n  - ../host/{MANIFEST_NAME}\n"));
    let state = dmls::overlay::OverlayState::default();
    let (registered, _) = acme_state(&state, &fixture);
    assert!(registered, "the valid install loads");

    // A missing member: the facts artifact.
    let facts = fixture.path("host/schemas/functions.facts.json");
    let saved = std::fs::read(&facts).unwrap();
    std::fs::remove_file(&facts).unwrap();
    state.invalidate_host_inputs(std::slice::from_ref(&facts));
    let (registered, watched) = acme_state(&state, &fixture);
    assert!(!registered, "the missing member refuses the host");
    assert!(!state.host_registration_diagnostics().is_empty());
    assert!(watched.contains(&fixture.manifest()), "the manifest: {watched:#?}");
    for listed in Bundle::acme().listed() {
        assert!(watched.contains(&fixture.path(&format!("host/{listed}"))), "listed member {listed}: {watched:#?}");
    }
    assert!(watched.contains(&facts), "the missing member's path: {watched:#?}");
    let folders = state.host_watch_folders();
    assert!(folders.iter().any(|folder| folder.folder == fixture.path("host/schemas")), "{folders:#?}");

    std::fs::write(&facts, saved).unwrap();
    state.invalidate_host_inputs(std::slice::from_ref(&facts));
    let (registered, _) = acme_state(&state, &fixture);
    assert!(registered, "the restored member loads the host");
    assert!(state.host_registration_diagnostics().is_empty(), "{:#?}", state.host_registration_diagnostics());

    // A member whose bytes the manifest does not address, then the same
    // bytes republished with the manifest re-addressed.
    let widget = fixture.path("host/schemas/widget.yaml");
    let edited = WIDGET_TRIGGER.replace("widget: widget@./library.yaml", "widget: widget@./library.yaml\n    note: string");
    assert_ne!(edited, WIDGET_TRIGGER);
    fixture.corrupt_member("host", "schemas/widget.yaml", &edited);
    state.invalidate_host_inputs(std::slice::from_ref(&widget));
    let (registered, watched) = acme_state(&state, &fixture);
    assert!(!registered, "the unaddressed bytes refuse the host");
    assert!(watched.contains(&widget) && watched.contains(&fixture.manifest()), "{watched:#?}");

    fixture.write_member("host", "schemas/widget.yaml", &edited);
    state.invalidate_host_inputs(&[fixture.manifest()]);
    let (registered, _) = acme_state(&state, &fixture);
    assert!(registered, "the re-addressed republish loads the host");
}

/// A missing user configuration is one warning per session naming the file
/// and `md init`, however many documents load it; nothing is created. A
/// session whose home has a configuration shows none.
#[test]
fn a_missing_user_configuration_is_one_session_notice() {
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    repository(root, &[]);
    let mut fixture = start(&workspace);
    let (first, _) = open_text(&mut fixture, &workspace, "notes/one.md", "---\ntitle: one\n---\n");
    open_text(&mut fixture, &workspace, "two.md", "Body\n");
    settle(&mut fixture, &first);
    let notices: Vec<Value> = fixture
        .take_show_messages()
        .into_iter()
        .filter(|message| message["message"].as_str().is_some_and(|text| text.contains("dm.config.missing")))
        .collect();
    assert_eq!(notices.len(), 1, "{notices:#?}");
    assert_eq!(notices[0]["type"], json!(2), "a warning");
    let message = notices[0]["message"].as_str().unwrap();
    assert!(message.contains(".darkmatter.yaml") && message.contains("md init"), "{message}");
    assert!(!root.join("home/.darkmatter.yaml").exists(), "nothing is created");
    fixture.shutdown();

    let workspace = LspWorkspace::new();
    repository(workspace.path(), &[]);
    write(workspace.path(), "home/.darkmatter.yaml", "{}\n");
    let mut fixture = start(&workspace);
    let (document, _) = open_text(&mut fixture, &workspace, "notes/one.md", "---\ntitle: one\n---\n");
    settle(&mut fixture, &document);
    assert!(fixture.take_show_messages().is_empty(), "the user file exists");
    fixture.shutdown();
}
