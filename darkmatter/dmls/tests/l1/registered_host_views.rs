//! DMLS analyzes every Markdown document as exactly one host, selected once
//! per schema preparation from the registrations of its configuration and
//! the legacy host catalogs in its schema roots: completion, hover, lifecycle
//! stacks, host function typing, and diagnostics all read that one view.
//!
//! The `relay` host (shaped like Claudine: `composed` → `start`, `failed` →
//! `blocked`, `success`/`failure` of its own, a `seq` global, the
//! `acme_label` function) and the `acme` host come from the shared host
//! fixtures. Every session's home is a folder of its temporary workspace;
//! nothing is read from the real home or checkout.

use std::path::Path;
use std::sync::Arc;

use serde_json::{Value, json};

use crate::common::{LspFixture, LspWorkspace};
use crate::host_fixtures::{
    FUNCTIONS, GADGET_TRIGGER, GLOBALS, IDENTIFY, LIBRARY, MANIFEST, RELAY_GLOBALS, RELAY_TRIGGER, WIDGET_TRIGGER,
    facts_artifact, lifecycle_artifact_for, relay_files, relay_lifecycle_artifact,
};
use crate::union_item_editor::{Document, initialize_params, offset_of, open_text, request};

const AMBIGUOUS: &str = "dm.host.ambiguous";
pub(crate) const REGISTRATION: &str = "dm.host.registration";
pub(crate) const UNKNOWN_HOST: &str = "dm.lifecycle.unknown_host";
pub(crate) const UNAVAILABLE: &str = "dm.expression.unavailable_global";
pub(crate) const UNKNOWN_FUNCTION: &str = "dm.expression.unknown_function";

pub(crate) fn write(root: &Path, relative: &str, text: &str) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

/// A repository workspace whose home (`home/`) registers each install named
/// in `registered` (folders under `home/`).
pub(crate) fn repository(root: &Path, registered: &[&str]) {
    for dir in [".git/objects", ".git/refs/heads"] {
        std::fs::create_dir_all(root.join(dir)).unwrap();
    }
    std::fs::write(root.join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
    std::fs::create_dir_all(root.join("home")).unwrap();
    std::fs::create_dir_all(root.join("notes")).unwrap();
    if !registered.is_empty() {
        let entries: String = registered.iter().map(|folder| format!("  - ./{folder}/darkmatter-host.yaml\n")).collect();
        write(root, "home/.darkmatter.yaml", &format!("register:\n{entries}"));
    }
}

pub(crate) fn install_relay(root: &Path, folder: &str) {
    for (name, text) in relay_files() {
        write(root, &format!("home/{folder}/{name}"), &text);
    }
}

fn install_acme(root: &Path, folder: &str) {
    let files = [
        ("darkmatter-host.yaml", MANIFEST.to_string()),
        ("schemas/identify.yaml", IDENTIFY.to_string()),
        ("schemas/widget.yaml", WIDGET_TRIGGER.to_string()),
        ("schemas/gadget.yaml", GADGET_TRIGGER.to_string()),
        ("schemas/library.yaml", LIBRARY.to_string()),
        ("schemas/globals.yaml", GLOBALS.to_string()),
        ("schemas/globals.lifecycle.json", lifecycle_artifact_for("acme", GLOBALS)),
        ("schemas/functions.yaml", FUNCTIONS.to_string()),
        ("schemas/functions.facts.json", facts_artifact()),
    ];
    for (name, text) in files {
        write(root, &format!("home/{folder}/{name}"), &text);
    }
}

fn start(workspace: &LspWorkspace) -> LspFixture<'_> {
    let mut fixture = LspFixture::start(workspace);
    fixture.initialize(initialize_params(workspace.path()));
    fixture
}

pub(crate) fn with_code<'d>(diagnostics: &'d [Value], code: &str) -> Vec<&'d Value> {
    diagnostics.iter().filter(|diagnostic| diagnostic["code"] == json!(code)).collect()
}

pub(crate) fn line_containing(text: &str, needle: &str) -> u64 {
    text[..offset_of(text, needle, false)].matches('\n').count() as u64
}

pub(crate) fn labels(result: &Value) -> Vec<String> {
    let items = result.get("items").unwrap_or(result);
    items
        .as_array()
        .map(|items| items.iter().filter_map(|item| item["label"].as_str().map(str::to_string)).collect())
        .unwrap_or_default()
}

pub(crate) fn completion_at(fixture: &mut LspFixture<'_>, document: &Document, after: &str) -> Vec<String> {
    labels(&request(fixture, "textDocument/completion", document, offset_of(&document.text, after, true)))
}

pub(crate) fn hover_at(fixture: &mut LspFixture<'_>, document: &Document, needle: &str) -> String {
    let hover = request(fixture, "textDocument/hover", document, offset_of(&document.text, needle, false));
    hover["contents"]["value"].as_str().unwrap_or_default().to_string()
}

/// The motivating document: identified by `sequence:` outside any package
/// area, with `seq` read in a body interpolation, `err` read in the relay's
/// `start` (renamed `composed`, where it is unavailable) and `blocked`
/// (renamed `failed`, where it is available) stacks, and host function calls.
pub(crate) const MOTIVATING: &str = "---\nsequence:\n  - name: one\n    prompt: hi\nstart:\n  - info: \"{{ err.message }} in start\"\nblocked:\n  - info: \"{{ err.message }} in blocked\"\n---\n{{ seq.na }} {{ seq.name }} {{ acme_label(\"x\") }}\n";

#[test]
fn a_registered_hosts_document_gets_its_globals_stacks_and_functions() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    repository(root, &[".relay"]);
    install_relay(root, ".relay");
    let mut fixture = start(&workspace);
    let (document, diagnostics) = open_text(&mut fixture, &workspace, "notes/relay.md", MOTIVATING);

    let members = completion_at(&mut fixture, &document, "{{ seq.na");
    assert!(members.contains(&"seq.name".to_string()), "{members:?}");
    let hover = hover_at(&mut fixture, &document, "name }}");
    assert!(hover.contains("The current step's name."), "{hover}");

    let unavailable = with_code(&diagnostics, UNAVAILABLE);
    assert_eq!(
        unavailable.iter().map(|diagnostic| diagnostic["range"]["start"]["line"].as_u64().unwrap()).collect::<Vec<_>>(),
        [line_containing(MOTIVATING, "in start")],
        "`start` is the relay's renamed `composed` stack and `blocked` its `failed`: {diagnostics:#?}"
    );
    assert!(with_code(&diagnostics, UNKNOWN_FUNCTION).is_empty(), "{diagnostics:#?}");
    for code in [AMBIGUOUS, REGISTRATION, UNKNOWN_HOST] {
        assert!(with_code(&diagnostics, code).is_empty(), "{code}: {diagnostics:#?}");
    }
    fixture.shutdown();
}

/// Without the registration the same document is an `md` document: no
/// `seq`, `start` is data, and the host function is unknown.
#[test]
fn without_a_registration_the_motivating_document_is_an_md_document() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    repository(root, &[]);
    install_relay(root, ".relay");
    let mut fixture = start(&workspace);
    let (document, diagnostics) = open_text(&mut fixture, &workspace, "notes/relay.md", MOTIVATING);

    let members = completion_at(&mut fixture, &document, "{{ seq.na");
    assert!(!members.contains(&"seq.name".to_string()), "{members:?}");
    assert!(with_code(&diagnostics, UNAVAILABLE).is_empty(), "{diagnostics:#?}");
    assert_eq!(with_code(&diagnostics, UNKNOWN_FUNCTION).len(), 1, "{diagnostics:#?}");
    fixture.shutdown();
}

/// A wrong-typed identifying value still identifies the document: the
/// relay's schema judges it, and its globals apply.
#[test]
fn a_wrong_typed_identifying_value_still_identifies() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    repository(root, &[".relay"]);
    install_relay(root, ".relay");
    let mut fixture = start(&workspace);
    let text = "---\nsequence: not a list\n---\n{{ seq.na }}\n";
    let (document, _) = open_text(&mut fixture, &workspace, "notes/wrong.md", text);
    let members = completion_at(&mut fixture, &document, "{{ seq.na");
    assert!(members.contains(&"seq.name".to_string()), "{members:?}");
    fixture.shutdown();
}

/// A document with only a shared event next to the registered relay's own
/// root catalog is an `md` document: the registered host is never a legacy
/// host. Unregistered, that catalog is the legacy fallback.
#[test]
fn a_shared_events_only_document_beside_an_installed_hosts_catalog_is_md() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let text = "---\ninitialize:\n  - info: hello\n---\n{{ seq.na }}\n";
    for (registered, offers_seq) in [(&[".relay"][..], false), (&[][..], true)] {
        let workspace = LspWorkspace::new();
        let root = workspace.path();
        repository(root, registered);
        install_relay(root, ".relay");
        write(root, "schemas/globals.yaml", RELAY_GLOBALS);
        write(root, "schemas/globals.lifecycle.json", &relay_lifecycle_artifact());
        let mut fixture = start(&workspace);
        let (document, diagnostics) = open_text(&mut fixture, &workspace, "shared.md", text);
        let members = completion_at(&mut fixture, &document, "{{ seq.na");
        assert_eq!(members.contains(&"seq.name".to_string()), offers_seq, "registered {registered:?}: {members:?}");
        assert!(with_code(&diagnostics, UNKNOWN_HOST).is_empty(), "{diagnostics:#?}");
        fixture.shutdown();
    }
}

/// A document two registered hosts identify is ambiguous: one warning, and
/// `md` analysis with no host's globals and no legacy fallback, while a
/// document neither identifies still gets the unregistered root catalog.
#[test]
fn an_ambiguous_document_is_md_and_never_falls_back_to_legacy() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    repository(root, &[".relay", ".acme"]);
    install_relay(root, ".relay");
    install_acme(root, ".acme");
    write(root, "schemas/globals.yaml", RELAY_GLOBALS);
    write(root, "schemas/globals.lifecycle.json", &relay_lifecycle_artifact().replace("\"relay\"", "\"legacy\""));
    let mut fixture = start(&workspace);

    let text = "---\nsequence:\n  - name: one\n    prompt: hi\ngadget: name\n---\n{{ seq.na }} {{ ru }}\n";
    let (document, diagnostics) = open_text(&mut fixture, &workspace, "both.md", text);
    let ambiguous = with_code(&diagnostics, AMBIGUOUS);
    assert_eq!(ambiguous.len(), 1, "{diagnostics:#?}");
    let message = ambiguous[0]["message"].as_str().unwrap();
    assert!(message.contains("`relay`") && message.contains("`acme`"), "{message}");
    assert!(!completion_at(&mut fixture, &document, "{{ seq.na").contains(&"seq.name".to_string()));
    assert!(!completion_at(&mut fixture, &document, "{{ ru").contains(&"run".to_string()));

    let neither = "---\ntitle: plain\n---\n{{ seq.na }}\n";
    let (document, diagnostics) = open_text(&mut fixture, &workspace, "neither.md", neither);
    assert!(with_code(&diagnostics, AMBIGUOUS).is_empty(), "{diagnostics:#?}");
    let members = completion_at(&mut fixture, &document, "{{ seq.na");
    assert!(members.contains(&"seq.name".to_string()), "the unregistered legacy catalog: {members:?}");
    fixture.shutdown();
}

/// Two manifests claiming one host with different content: each claim is a
/// registration warning on a document the host would identify, and no host
/// applies.
#[test]
fn conflicting_claims_of_one_host_apply_no_host() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    repository(root, &[".relay", ".relay-next"]);
    install_relay(root, ".relay");
    install_relay(root, ".relay-next");
    let manifest = std::fs::read_to_string(root.join("home/.relay-next/darkmatter-host.yaml")).unwrap();
    write(root, "home/.relay-next/darkmatter-host.yaml", &manifest.replace("version: 0.3.0", "version: 0.4.0"));
    let mut fixture = start(&workspace);
    let (document, diagnostics) = open_text(&mut fixture, &workspace, "notes/relay.md", MOTIVATING);
    let conflicts = with_code(&diagnostics, REGISTRATION);
    assert_eq!(conflicts.len(), 2, "one per claim: {diagnostics:#?}");
    assert!(conflicts.iter().all(|diagnostic| diagnostic["severity"] == json!(2)), "{conflicts:#?}");
    assert!(!completion_at(&mut fixture, &document, "{{ seq.na").contains(&"seq.name".to_string()));
    assert!(with_code(&diagnostics, UNAVAILABLE).is_empty(), "`start` is data: {diagnostics:#?}");
    fixture.shutdown();
}

/// A host-only event of the registered relay, in a document the relay does
/// not identify, warns at the key and names the relay; unrelated keys and
/// `md`'s own events stay quiet.
#[test]
fn a_known_host_only_event_in_an_unidentified_document_names_its_host() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    repository(root, &[".relay"]);
    install_relay(root, ".relay");
    let mut fixture = start(&workspace);
    let text = "---\ntitle: notes\nmy_data: 1\nsuccess:\n  - info: done\nfinalize:\n  - info: bye\n---\nBody\n";
    let (_, diagnostics) = open_text(&mut fixture, &workspace, "notes/plain.md", text);
    let unknown = with_code(&diagnostics, UNKNOWN_HOST);
    assert_eq!(unknown.len(), 1, "{diagnostics:#?}");
    assert_eq!(unknown[0]["range"]["start"]["line"].as_u64().unwrap(), line_containing(text, "success"));
    assert_eq!(unknown[0]["severity"], json!(2));
    let message = unknown[0]["message"].as_str().unwrap();
    assert!(message.contains("`success`") && message.contains("`relay`"), "{message}");
    fixture.shutdown();

    // Without the registration nothing knows `success`.
    let workspace = LspWorkspace::new();
    repository(workspace.path(), &[]);
    let mut fixture = start(&workspace);
    let (_, diagnostics) = open_text(&mut fixture, &workspace, "notes/plain.md", text);
    assert!(with_code(&diagnostics, UNKNOWN_HOST).is_empty(), "{diagnostics:#?}");
    fixture.shutdown();
}

/// Replacing a member with one that no longer loads, then reporting the
/// change, drops the host view at once and reports the new failure on the
/// member it names: the last-good host is never served, and the document
/// itself carries no registration warning.
#[test]
fn an_invalid_replacement_clears_the_host_view_and_reports_the_failure() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    repository(root, &[".relay"]);
    install_relay(root, ".relay");
    let mut fixture = start(&workspace);
    let (document, diagnostics) = open_text(&mut fixture, &workspace, "notes/relay.md", MOTIVATING);
    assert!(with_code(&diagnostics, REGISTRATION).is_empty(), "{diagnostics:#?}");
    assert!(completion_at(&mut fixture, &document, "{{ seq.na").contains(&"seq.name".to_string()));

    let artifact = root.join("home/.relay/schemas/globals.lifecycle.json");
    std::fs::write(&artifact, "{ \"not\": \"an artifact\" }\n").unwrap();
    fixture.notify(
        "workspace/didChangeWatchedFiles",
        json!({ "changes": [{ "uri": url::Url::from_file_path(&artifact).unwrap().as_str(), "type": 2 }] }),
    );
    fixture.flush_server_with("textDocument/documentSymbol", json!({ "textDocument": { "uri": document.uri } }));
    let refused = fixture.take_latest_diagnostics_ending("home/.relay/schemas/globals.lifecycle.json").expect("published on the artifact");
    let refused = with_code(&refused, REGISTRATION);
    assert_eq!(refused.len(), 1, "{refused:#?}");
    assert!(refused[0]["message"].as_str().unwrap().contains("lifecycle"), "{refused:#?}");
    let diagnostics = fixture.take_latest_diagnostics_ending(&document.uri).expect("the document was refreshed");
    assert!(with_code(&diagnostics, REGISTRATION).is_empty(), "not on the document: {diagnostics:#?}");
    assert!(with_code(&diagnostics, UNAVAILABLE).is_empty(), "no relay stacks remain: {diagnostics:#?}");
    assert!(!completion_at(&mut fixture, &document, "{{ seq.na").contains(&"seq.name".to_string()));
    fixture.shutdown();
}

/// In-process: the document's analysis snapshot and schema are keyed on the
/// registration's revision. An invalidation of an unchanged file keeps both;
/// a changed member installs a new revision and replaces both.
#[test]
fn the_analysis_snapshot_follows_the_registration_revision() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    use darkmatter::config::registration::HostSelection;
    use dmls::config::DmlsConfig;
    use dmls::context::DocumentResolution;
    use dmls::overlay::{DocumentOverlay, OverlayState};

    let dir = tempfile::tempdir().unwrap();
    let root = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
    repository(&root, &[".relay"]);
    install_relay(&root, ".relay");
    let path = root.join("notes/relay.md");
    write(&root, "notes/relay.md", MOTIVATING);
    let context = biscuit_file::FileResolutionContext::from_snapshot(&root, Some(root.join("home")), Default::default())
        .with_repository_root(&root)
        .for_source(&path);
    let resolution = DocumentResolution::from_context(context);
    let uri: lsp_types::Uri = url::Url::from_file_path(&path).unwrap().as_str().parse().unwrap();
    let state = OverlayState::default();
    let roots = [root.clone()];
    let build = || -> DocumentOverlay {
        state.for_document(&uri, MOTIVATING, &path, &DmlsConfig::default(), &roots, &resolution).expect("frontmatter")
    };
    let revision = |overlay: &DocumentOverlay| match overlay.host.as_deref().map(|view| view.selection()) {
        Some(HostSelection::Registered(profile)) => profile.revision().to_string(),
        other => panic!("{other:?}"),
    };

    let first = build();
    let trigger = root.join("home/.relay/schemas/relay.yaml");
    state.invalidate_host_inputs(std::slice::from_ref(&trigger));
    let unchanged = build();
    assert!(Arc::ptr_eq(&first.analysis, &unchanged.analysis), "an unchanged file keeps the snapshot");
    assert_eq!(revision(&first), revision(&unchanged));

    write(&root, "home/.relay/schemas/relay.yaml", &RELAY_TRIGGER.replace("the run's timeout", "the run's time limit"));
    state.invalidate_host_inputs(&[trigger]);
    let changed = build();
    assert_ne!(revision(&first), revision(&changed), "a changed member is a new revision");
    assert!(!Arc::ptr_eq(&first.analysis, &changed.analysis), "a new revision is a new snapshot");
    let bundle = |overlay: &DocumentOverlay| overlay.bundle().map(|bundle| bundle as *const _);
    assert_ne!(bundle(&first), bundle(&changed), "and a reassembled schema");
}
