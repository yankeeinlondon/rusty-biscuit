//! DMLS analyzes every Markdown document as exactly one host, selected once
//! per schema preparation from the registrations of its configuration:
//! completion, hover, lifecycle stacks, host function typing, and diagnostics
//! all read that one view. A host catalog and lifecycle artifact in a schema
//! root supply no host.
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
use crate::host_fixtures::{Bundle, RELAY_TRIGGER, relay_files, relay_lifecycle_artifact};
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
    Bundle::acme().write_to(&root.join(format!("home/{folder}")));
}

/// A host global catalog declaring the relay's `seq`, as a host installed it
/// in a schema root before registration. It never supplies a host.
pub(crate) const ROOT_CATALOG: &str =
    "kind: schema\ndescription: the relay's globals\n$schema:\n    seq:\n        name: \"string -> The current step's name.\"\n";

/// Writes [`ROOT_CATALOG`] and the relay's lifecycle artifact beside it in
/// the tree root's `schemas/`.
pub(crate) fn root_catalog(root: &Path) {
    write(root, "schemas/globals.yaml", ROOT_CATALOG);
    write(root, "schemas/globals.lifecycle.json", &relay_lifecycle_artifact());
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

/// Registered triggers match as ordinary triggers: a `sequence` value the
/// relay's `match` type does not admit (it covers lists only) does not
/// identify the document, so no relay global applies.
#[test]
fn a_value_the_trigger_type_does_not_admit_does_not_identify() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    repository(root, &[".relay"]);
    install_relay(root, ".relay");
    let mut fixture = start(&workspace);
    let text = "---\nsequence: not a list\n---\n{{ seq.na }}\n";
    let (document, _) = open_text(&mut fixture, &workspace, "notes/wrong.md", text);
    let members = completion_at(&mut fixture, &document, "{{ seq.na");
    assert!(!members.contains(&"seq.name".to_string()), "{members:?}");
    fixture.shutdown();
}

/// A document with only a shared event next to a root catalog of the
/// relay's globals is an `md` document, registered or not: no trigger of
/// the registered relay matches it, and a root-local catalog supplies no
/// host.
#[test]
fn a_shared_events_only_document_beside_a_root_local_catalog_is_md() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let text = "---\ninitialize:\n  - info: hello\n---\n{{ seq.na }}\n";
    for registered in [&[".relay"][..], &[][..]] {
        let workspace = LspWorkspace::new();
        let root = workspace.path();
        repository(root, registered);
        install_relay(root, ".relay");
        root_catalog(root);
        let mut fixture = start(&workspace);
        let (document, diagnostics) = open_text(&mut fixture, &workspace, "shared.md", text);
        let members = completion_at(&mut fixture, &document, "{{ seq.na");
        assert!(!members.contains(&"seq.name".to_string()), "registered {registered:?}: {members:?}");
        assert!(with_code(&diagnostics, UNKNOWN_HOST).is_empty(), "{diagnostics:#?}");
        fixture.shutdown();
    }
}

/// A document two registered hosts identify is ambiguous: one warning, and
/// `md` analysis with no host's globals; a document neither identifies is
/// `md` too, whatever catalog its schema root holds.
#[test]
fn an_ambiguous_document_is_md_and_a_root_catalog_supplies_no_host() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    repository(root, &[".relay", ".acme"]);
    install_relay(root, ".relay");
    install_acme(root, ".acme");
    root_catalog(root);
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
    assert!(!members.contains(&"seq.name".to_string()), "the root catalog supplies no host: {members:?}");
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

/// Replacing a member with bytes its manifest does not address, then
/// reporting the change, drops the host view at once and reports the new
/// failure on the manifest that lists the member: the last-good host is
/// never served, and the document itself carries no registration warning.
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
    let refused = fixture.take_latest_diagnostics_ending("home/.relay/darkmatter-host.yaml").expect("published on the manifest");
    let refused = with_code(&refused, REGISTRATION);
    assert_eq!(refused.len(), 1, "{refused:#?}");
    let message = refused[0]["message"].as_str().unwrap();
    assert!(message.contains("globals.lifecycle.json") && message.contains("digest"), "{refused:#?}");
    let diagnostics = fixture.take_latest_diagnostics_ending(&document.uri).expect("the document was refreshed");
    assert!(with_code(&diagnostics, REGISTRATION).is_empty(), "not on the document: {diagnostics:#?}");
    assert!(with_code(&diagnostics, UNAVAILABLE).is_empty(), "no relay stacks remain: {diagnostics:#?}");
    assert!(!completion_at(&mut fixture, &document, "{{ seq.na").contains(&"seq.name".to_string()));
    fixture.shutdown();
}

/// A relay whose lifecycle artifact binds `typo_root`, a root no namespace
/// declares, on its own `success` event (the manifest re-addressed) is
/// refused at that binding before any view is derived: the document is an
/// `md` document, and the refusal is published on the artifact.
#[test]
fn a_lifecycle_binding_no_namespace_declares_refuses_the_host_at_the_binding() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    repository(root, &[".relay"]);
    let mut artifact: Value = serde_json::from_str(&relay_lifecycle_artifact()).unwrap();
    let success = artifact["events"].as_array_mut().unwrap().iter_mut().find(|entry| entry["name"] == "success").unwrap();
    let timing = success["globals"]["declarations"].as_array_mut().unwrap().iter_mut().find(|entry| entry["root"] == "timing").unwrap();
    timing["root"] = json!("typo_root");
    let mut edited = serde_json::to_string_pretty(&artifact).unwrap();
    edited.push('\n');
    let line = line_containing(&edited, "\"typo_root\"");
    let character = edited.lines().nth(line as usize).unwrap().find("\"typo_root\"").unwrap() as u64;
    let bundle = Bundle::relay().with_member("schemas/globals.lifecycle.json", &edited);
    bundle.write_to(&root.join("home/.relay"));
    let mut fixture = start(&workspace);
    let (document, diagnostics) = open_text(&mut fixture, &workspace, "notes/relay.md", MOTIVATING);

    assert!(with_code(&diagnostics, UNAVAILABLE).is_empty(), "no relay stacks: {diagnostics:#?}");
    assert!(!completion_at(&mut fixture, &document, "{{ seq.na").contains(&"seq.name".to_string()));
    fixture.flush_server_with("textDocument/documentSymbol", json!({ "textDocument": { "uri": document.uri } }));
    let refused = fixture.take_latest_diagnostics_ending("home/.relay/schemas/globals.lifecycle.json").expect("published on the artifact");
    let refused = with_code(&refused, REGISTRATION);
    assert_eq!(refused.len(), 1, "{refused:#?}");
    assert_eq!(refused[0]["range"]["start"], json!({ "line": line, "character": character }), "{refused:#?}");
    assert!(refused[0]["message"].as_str().unwrap().contains("binds `typo_root`"), "{refused:#?}");
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

    // A republish of one member: the manifest is re-addressed with it.
    Bundle::relay()
        .with_member("schemas/relay.yaml", &RELAY_TRIGGER.replace("the run's timeout", "the run's time limit"))
        .write_to(&root.join("home/.relay"));
    state.invalidate_host_inputs(&[trigger, root.join("home/.relay/darkmatter-host.yaml")]);
    let changed = build();
    assert_ne!(revision(&first), revision(&changed), "a changed member is a new revision");
    assert!(!Arc::ptr_eq(&first.analysis, &changed.analysis), "a new revision is a new snapshot");
    let bundle = |overlay: &DocumentOverlay| overlay.bundle().map(|bundle| bundle as *const _);
    assert_ne!(bundle(&first), bundle(&changed), "and a reassembled schema");
}

/// A closed document `$schema` holds over a registered host's root union:
/// the undeclared key is reported where it is written, and the declared
/// ones (the document's and the host arm's) are not.
#[test]
fn a_closed_document_schema_refuses_an_undeclared_key_over_a_registered_root_union() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    repository(root, &[".acme"]);
    let union = "kind: schema-trigger\nmatch:\n    team: string(required)\n$schema:\n    - prompt: string(required)\n      sections: null\n    - sections: object(required)\n      prompt: null\n";
    Bundle::acme().with_file("schemas/inline.yaml", union).write_to(&root.join("home/.acme"));
    let mut fixture = start(&workspace);
    let text = "---\n$schema:\n  $constraints: { closed: true }\n  team: string\n  prompt: string\nteam: core\nprompt: hi\nextra: hi\n---\nBody\n";
    let (_, diagnostics) = open_text(&mut fixture, &workspace, "notes/inline.md", text);

    let unknown = with_code(&diagnostics, "dm.schema.unknown_key");
    let [diagnostic] = unknown.as_slice() else { panic!("one unknown key: {diagnostics:#?}") };
    assert_eq!(diagnostic["range"]["start"]["line"], json!(line_containing(text, "extra: hi")), "{diagnostic:#?}");
    assert!(with_code(&diagnostics, REGISTRATION).is_empty(), "{diagnostics:#?}");
    fixture.shutdown();
}

/// A registration warning sits on a line whose earlier text is ASCII prose,
/// `café`, or `🧪 café`. The library locates it by line and character
/// column; each client receives that point counted in the encoding it
/// negotiated (UTF-8 bytes or UTF-16 code units), on the unopened member and
/// on the same member once it is open.
/// A refused trigger that declares a valid `name` is published on its file
/// with its label beside its manifest field.
#[test]
fn a_registration_warning_names_a_named_trigger_by_its_label() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    repository(root, &[".acme"]);
    let trigger = format!("name: gadget-schema\n{}", crate::host_fixtures::GADGET_TRIGGER.replacen("string(required)", "string(requird)", 1));
    Bundle::acme().with_member("schemas/gadget.yaml", &trigger).write_to(&root.join("home/.acme"));
    let mut fixture = start(&workspace);
    let (document, _) = open_text(&mut fixture, &workspace, "notes/notes.md", "---\ntitle: Notes\n---\n# Notes\n");
    fixture.flush_server_with("textDocument/documentSymbol", json!({ "textDocument": { "uri": document.uri } }));
    let published = fixture.take_latest_diagnostics_ending("home/.acme/schemas/gadget.yaml").expect("published on the member");
    let refused = with_code(&published, REGISTRATION);
    assert_eq!(refused.len(), 1, "{refused:#?}");
    let message = refused[0]["message"].as_str().unwrap_or_default();
    assert!(message.contains("is not loaded at `files[1]` (`gadget-schema (schemas/gadget.yaml)`): "), "{refused:#?}");
    assert_eq!(refused[0]["range"]["start"]["line"], json!(line_containing(&trigger, "string(requird)")), "the range is kept: {refused:#?}");
    fixture.shutdown();
}

#[test]
fn a_registration_warning_is_published_in_each_clients_position_encoding() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    for prose in ["plain prose", "café", "🧪 café"] {
        let types = format!("types: {{label: 'string -> {prose}', review-entry: 'string(example(./options.yaml))'}}\n");
        let trigger = format!("{RELAY_TRIGGER}{types}");
        let line = line_containing(&trigger, "review-entry");
        let prefix = &types[..types.find("'string(example(").unwrap()];
        for (encoding, character) in [("utf-8", prefix.len()), ("utf-16", prefix.encode_utf16().count())] {
            let case = format!("{prose} / {encoding}");
            let workspace = LspWorkspace::new();
            let root = workspace.path();
            repository(root, &[".relay"]);
            Bundle::relay().with_member("schemas/relay.yaml", &trigger).write_to(&root.join("home/.relay"));
            let mut fixture = LspFixture::start(&workspace);
            let mut params = initialize_params(root);
            params["capabilities"]["general"]["positionEncodings"] = json!([encoding]);
            let initialized = fixture.initialize(params);
            assert_eq!(initialized["capabilities"]["positionEncoding"], json!(encoding), "{case}");
            let expected = json!({ "line": line, "character": character });

            let (document, _) = open_text(&mut fixture, &workspace, "notes/relay.md", MOTIVATING);
            fixture.flush_server_with("textDocument/documentSymbol", json!({ "textDocument": { "uri": document.uri } }));
            let closed = fixture.take_latest_diagnostics_ending("home/.relay/schemas/relay.yaml").expect("published on the member");
            let closed = with_code(&closed, REGISTRATION);
            assert_eq!(closed.len(), 1, "{case}: {closed:#?}");
            assert_eq!(closed[0]["range"]["start"], expected, "{case}: unopened: {closed:#?}");

            let member = url::Url::from_file_path(root.join("home/.relay/schemas/relay.yaml")).unwrap().to_string();
            fixture.notify(
                "textDocument/didOpen",
                json!({ "textDocument": { "uri": member, "languageId": "yaml", "version": 1, "text": trigger } }),
            );
            let opened = fixture.wait_for_diagnostics(&member);
            let opened = with_code(&opened, REGISTRATION);
            assert_eq!(opened.len(), 1, "{case}: {opened:#?}");
            assert_eq!(opened[0]["range"]["start"], expected, "{case}: opened: {opened:#?}");
            fixture.shutdown();
        }
    }
}
