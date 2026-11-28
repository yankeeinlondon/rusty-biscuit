//! A publish rewrites a dozen registration files, reported in several
//! batches. DMLS waits until a configuration's registration inputs have been
//! quiet for the settle window, then reloads them once, off the dispatch
//! thread; until that reload completes, open Markdown and YAML documents
//! keep the view they had. Ordinary edits and unrelated schema changes are
//! not held, and a client without folder watches reloads after a save.
//!
//! Every session runs on a [`ManualClock`] with the production window, so
//! the deadline is crossed by advancing the clock, never by sleeping.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use dmls::clock::ManualClock;
use dmls::overlay::hosts::REGISTRATION_SETTLE;
use dmls::work_counters::{host_loads, work_counts};
use serde_json::{Value, json};

use crate::common::{LspFixture, LspWorkspace};
use crate::host_registration_watches::{
    CHANGED, CREATED, DELETED, offers_seq, registration_on, report, settle, watchers, watching_params,
};
use crate::registered_host_views::{MOTIVATING, install_relay, repository, write};
use crate::registered_yaml_kinds::{ROSTER, TEAM, bundle, errors};
use crate::union_item_editor::{Document, open_text};

const ARTIFACT: &str = "home/.relay/schemas/globals.lifecycle.json";
const RELAY_MANIFEST: &str = "home/.relay/darkmatter-host.yaml";

/// The relay (Markdown) and acme with `kind: roster` (YAML) registered in
/// one home, with a session on a manual clock.
struct Session<'w> {
    fixture: LspFixture<'w>,
    clock: Arc<ManualClock>,
    markdown: Document,
}

impl<'w> Session<'w> {
    fn start(workspace: &'w LspWorkspace) -> Self {
        let root = workspace.path();
        repository(root, &[".relay", ".acme"]);
        install_relay(root, ".relay");
        bundle("acme", ROSTER).write_to(&root.join("home/.acme"));
        let clock = Arc::new(ManualClock::new());
        let mut fixture = LspFixture::start_with_registration_settle(workspace, clock.clone());
        fixture.initialize(watching_params(root));
        let (markdown, _) = open_text(&mut fixture, workspace, "notes/relay.md", MOTIVATING);
        let (_, diagnostics) = open_text(&mut fixture, workspace, "notes/team.yaml", TEAM);
        assert_eq!(errors(&diagnostics).len(), 1, "the registered kind judges `platforms: 3`: {diagnostics:#?}");
        assert!(offers_seq(&mut fixture, &markdown), "the relay is registered");
        settle(&mut fixture, &markdown);
        let mut session = Self { fixture, clock, markdown };
        session.drain();
        session
    }

    fn report(&mut self, changes: &[(&Path, u32)]) {
        report(&mut self.fixture, &self.markdown, changes);
    }

    fn advance(&mut self, millis: u64) {
        self.clock.advance(Duration::from_millis(millis));
        settle(&mut self.fixture, &self.markdown);
    }

    /// The YAML document's errors last published since the last call.
    fn yaml_errors(&mut self) -> Option<usize> {
        self.fixture.take_latest_diagnostics_ending("notes/team.yaml").map(|diagnostics| errors(&diagnostics).len())
    }

    fn relay_warnings(&mut self) -> Option<usize> {
        registration_on(&mut self.fixture, RELAY_MANIFEST).map(|warnings| warnings.len())
    }

    /// Forgets every publication so far.
    fn drain(&mut self) {
        self.yaml_errors();
        self.relay_warnings();
        self.fixture.take_latest_diagnostics_ending("notes/relay.md");
    }
}

/// A publish reported in two batches 200 ms apart reloads once, 300 ms
/// after the second batch. Until then the Markdown document keeps its host
/// and the YAML document its contract, and no warning is published; after
/// it, both follow the new registrations and the refused relay's warning is
/// on its manifest. Restoring the member reloads once more and clears the
/// warning without reopening either document.
#[test]
fn a_burst_across_batches_reloads_once_after_quiet_and_a_repair_clears_the_warning() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    let mut session = Session::start(&workspace);
    let loads = host_loads();

    // Batch one: acme's roster now accepts numbers (members and manifest
    // republished on disk, the member reported first).
    bundle("acme", &ROSTER.replace("string[]", "number")).write_to(&root.join("home/.acme"));
    session.report(&[(&root.join("home/.acme/schemas/roster.yaml"), CHANGED)]);
    session.advance(200);
    // Batch two: acme's manifest, and the relay's artifact removed.
    let artifact = root.join(ARTIFACT);
    let saved = std::fs::read_to_string(&artifact).unwrap();
    std::fs::remove_file(&artifact).unwrap();
    session.report(&[(&root.join("home/.acme/darkmatter-host.yaml"), CHANGED), (&artifact, DELETED)]);
    session.advance(299);

    assert_eq!(host_loads(), loads, "nothing reloads inside the window");
    assert!(offers_seq(&mut session.fixture, &session.markdown), "the Markdown document keeps its host");
    assert!(session.yaml_errors().is_none_or(|errors| errors == 1), "the YAML document keeps its contract");
    assert!(session.relay_warnings().is_none_or(|warnings| warnings == 0), "no warning before the reload");

    session.advance(1);
    assert_eq!(host_loads() - loads, 1, "one load once quiet");
    assert!(!offers_seq(&mut session.fixture, &session.markdown), "the refused relay is dropped");
    assert_eq!(session.yaml_errors(), Some(0), "the republished roster judges the YAML document");
    assert_eq!(session.relay_warnings(), Some(1), "the refusal is published on the relay's manifest");

    std::fs::write(&artifact, saved).unwrap();
    session.report(&[(&artifact, CREATED)]);
    session.advance(REGISTRATION_SETTLE.as_millis() as u64);
    assert_eq!(host_loads() - loads, 2);
    assert!(offers_seq(&mut session.fixture, &session.markdown), "the repair restores the host");
    assert_eq!(session.relay_warnings(), Some(0), "and clears the warning");
    session.fixture.shutdown();
}

/// A reported change that leaves every registration's bytes as they were
/// reloads once, and the reload reanalyzes nothing: no frontmatter is
/// reassembled and no YAML contract prepared.
#[test]
fn an_unchanged_reload_changes_no_analysis() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    let mut session = Session::start(&workspace);
    let loads = host_loads();

    session.report(&[(&root.join(RELAY_MANIFEST), CHANGED), (&root.join("home/.acme/darkmatter-host.yaml"), CHANGED)]);
    // From after the report: the watch path's own refresh is not held.
    let before = work_counts();
    session.advance(REGISTRATION_SETTLE.as_millis() as u64);

    assert_eq!(host_loads() - loads, 1, "the reload ran");
    let work = work_counts().since(before);
    assert_eq!(work.root_assemblies, 0, "{work:#?}");
    assert_eq!(work.yaml_preparations, 0, "{work:#?}");
    assert!(offers_seq(&mut session.fixture, &session.markdown));
    session.fixture.shutdown();
}

/// A `register` entry moved to a missing folder is watched from that
/// folder's nearest existing ancestor; the install moved there is found
/// from the folder's own creation event alone.
#[test]
fn watch_coverage_follows_a_moved_install() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    let mut session = Session::start(&workspace);

    let config = root.join("home/.darkmatter.yaml");
    write(root, "home/.darkmatter.yaml", "register:\n  - ./moved/darkmatter-host.yaml\n  - ./.acme/darkmatter-host.yaml\n");
    session.report(&[(&config, CHANGED)]);
    session.advance(REGISTRATION_SETTLE.as_millis() as u64);
    assert!(!offers_seq(&mut session.fixture, &session.markdown), "nothing at the new location");
    let registered = watchers(&session.fixture, "dmls-watch-schema-inputs");
    assert!(
        registered.iter().any(|(base, pattern)| base.ends_with("/home") && pattern.starts_with("moved")),
        "the missing folder is watched from its parent: {registered:#?}"
    );

    std::fs::rename(root.join("home/.relay"), root.join("home/moved")).unwrap();
    session.report(&[(&root.join("home/moved"), CREATED)]);
    session.advance(REGISTRATION_SETTLE.as_millis() as u64);
    assert!(offers_seq(&mut session.fixture, &session.markdown), "the moved install registers the relay");
    let registered = watchers(&session.fixture, "dmls-watch-schema-inputs");
    assert!(
        registered.iter().any(|(base, pattern)| base.ends_with("/home/moved") && pattern == "schemas/*"),
        "the moved install's members are watched: {registered:#?}"
    );
    session.fixture.shutdown();
}

/// While a reload waits for quiet, an edit to an open document and a change
/// to a schema file another document names are analyzed at once, under the
/// registrations still installed.
#[test]
fn a_normal_edit_and_an_unrelated_schema_change_are_not_held() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    write(root, "notes/plain.yaml", "kind: schema\n$schema:\n    title: string\n");
    let mut session = Session::start(&workspace);
    let (plain, diagnostics) = open_text(&mut session.fixture, &workspace, "notes/plain.md", "---\n$schema: ./plain.yaml\ntitle: one\n---\n");
    assert!(errors(&diagnostics).is_empty(), "{diagnostics:#?}");
    let artifact = root.join(ARTIFACT);
    std::fs::remove_file(&artifact).unwrap();
    session.report(&[(&artifact, DELETED)]);
    let loads = host_loads();

    write(root, "notes/plain.yaml", "kind: schema\n$schema:\n    title: number\n");
    session.report(&[(&root.join("notes/plain.yaml"), CHANGED)]);
    let diagnostics = session.fixture.take_latest_diagnostics_ending("notes/plain.md").expect("republished at once");
    assert_eq!(errors(&diagnostics).len(), 1, "the edited schema judges `title` at once: {diagnostics:#?}");

    let edited = MOTIVATING.replace("name: one", "name: two");
    session.fixture.notify(
        "textDocument/didChange",
        json!({ "textDocument": { "uri": session.markdown.uri, "version": 2 }, "contentChanges": [{ "text": edited }] }),
    );
    let published = session.fixture.wait_for_diagnostics(&session.markdown.uri);
    assert!(published.iter().all(|diagnostic: &Value| diagnostic["code"] != json!("dm.lifecycle.unknown_host")), "{published:#?}");
    session.markdown.text = edited;
    assert!(offers_seq(&mut session.fixture, &session.markdown), "the edit is analyzed under the installed host");
    assert_eq!(host_loads(), loads, "neither change reloaded the registrations");
    drop(plain);

    session.advance(REGISTRATION_SETTLE.as_millis() as u64);
    assert!(!offers_seq(&mut session.fixture, &session.markdown), "the held reload ran once quiet");
    session.fixture.shutdown();
}

/// A client that cannot root watchers at folders learns of a publish only
/// when it saves: the save schedules the same reload, after the window.
#[test]
fn a_save_fallback_client_reloads_after_the_window() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    repository(root, &[".relay"]);
    install_relay(root, ".relay");
    let clock = Arc::new(ManualClock::new());
    let mut fixture = LspFixture::start_with_registration_settle(&workspace, clock.clone());
    let mut params = watching_params(root);
    params["capabilities"]["workspace"]["didChangeWatchedFiles"]["relativePatternSupport"] = json!(false);
    fixture.initialize(params);
    let (document, _) = open_text(&mut fixture, &workspace, "notes/relay.md", MOTIVATING);
    assert!(offers_seq(&mut fixture, &document));

    std::fs::remove_file(root.join(ARTIFACT)).unwrap();
    fixture.notify("textDocument/didSave", json!({ "textDocument": { "uri": document.uri } }));
    settle(&mut fixture, &document);
    assert!(offers_seq(&mut fixture, &document), "the save only schedules the reload");
    clock.advance(REGISTRATION_SETTLE);
    settle(&mut fixture, &document);
    assert!(!offers_seq(&mut fixture, &document), "the reload ran after the window");
    fixture.shutdown();
}
