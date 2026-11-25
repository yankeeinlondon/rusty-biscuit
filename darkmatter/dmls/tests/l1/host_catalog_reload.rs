//! A registered host's expression catalog reaches the editor only through
//! its format 3 bundle: the catalog is a `files` member and its facts
//! companion a digested sibling. DMLS follows the companion like any other
//! member, without reopening a document: a missing, removed, or stale
//! companion refuses the host (its function completions and call
//! diagnostics disappear) and a republish or restore brings them back. A
//! publish reported in several batches reloads once per settled revision,
//! whatever the number of open documents, and an edit to a document
//! reloads nothing.
//!
//! Only the selected host's functions are visible: an ordinary document, a
//! document another host identifies, and a refused registration expose
//! none, and neither does a catalog the manifest does not list, wherever it
//! lies. A name two catalogs of one host claim refuses the host with the
//! earlier declaration as related information.
//!
//! Every session's home is a folder of its temporary workspace (the shared
//! `relay` install from Darkmatter's host fixtures); nothing is read from
//! the real home or checkout.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use darkmatter::markdown::compose::expression::{HostFunctionCatalog, HostFunctions, HostHandler};
use dmls::clock::ManualClock;
use dmls::overlay::hosts::{HostRegistrations, REGISTRATION_SETTLE};
use dmls::work_counters::host_loads;
use serde_json::{Value, json};
use simplified_schema::coercion::ValueKind;

use crate::common::{LspFixture, LspWorkspace};
use crate::host_fixtures::{Bundle, FUNCTIONS, MANIFEST_NAME};
use crate::host_registration_watches::{CHANGED, CREATED, DELETED, registration_on, report, settle, start, watching_params};
use crate::registered_host_views::{REGISTRATION, UNKNOWN_FUNCTION, completion_at, line_containing, repository, with_code, write};
use crate::union_item_editor::{Document, open_text};

const INCOMPATIBLE: &str = "dm.expression.incompatible_call";
const RELAY_MANIFEST: &str = "home/.relay/darkmatter-host.yaml";
const COMPANION: &str = "home/.relay/schemas/functions.facts.json";

/// Identified by the relay (`sequence:`): a partial host function name to
/// complete, and a call whose argument the host's `name: string` refuses.
const CALLS: &str = "---\nsequence:\n  - name: one\n    prompt: hi\n---\n{{ acme_lab }} {{ acme_label(42) }}\n";

/// What the editor exposes of the relay's `acme_label` in a document.
#[derive(Debug, PartialEq)]
struct Exposure {
    /// Completion after `{{ acme_lab` offers `acme_label`.
    completes: bool,
    /// The call's argument is checked against the host's binding.
    incompatible: usize,
    /// The call names an unknown function.
    unknown: usize,
}

const SELECTED: Exposure = Exposure { completes: true, incompatible: 1, unknown: 0 };
const NONE: Exposure = Exposure { completes: false, incompatible: 0, unknown: 1 };

fn exposure(fixture: &mut LspFixture<'_>, document: &Document, diagnostics: &[Value]) -> Exposure {
    Exposure {
        completes: completion_at(fixture, document, "{{ acme_lab").contains(&"acme_label".to_string()),
        incompatible: with_code(diagnostics, INCOMPATIBLE).len(),
        unknown: with_code(diagnostics, UNKNOWN_FUNCTION).len(),
    }
}

/// The diagnostics last published for `document`, keeping `previous` when
/// nothing was published since.
fn latest(fixture: &mut LspFixture<'_>, document: &Document, previous: &mut Vec<Value>) -> Vec<Value> {
    if let Some(published) = fixture.take_latest_diagnostics_ending(&document.uri) {
        *previous = published;
    }
    previous.clone()
}

/// The facts companion a host registering each `(canonical, aliases)` of
/// `catalog` writes. Every function refuses a number for its `name`, so a
/// call with a number argument is incompatible only under the host's
/// binding.
fn facts_for(catalog: &str, functions: &[(&str, &[&str])]) -> String {
    let catalog = HostFunctionCatalog::load(catalog, "functions.yaml").expect("the catalog loads");
    let mut builder = HostFunctions::builder(catalog);
    for (canonical, aliases) in functions {
        let handler: HostHandler = Arc::new(|_| Ok(json!("label")));
        builder = builder.function(
            *canonical,
            aliases,
            |registration| registration.reject_sources("name", &[ValueKind::Number]),
            handler,
        );
    }
    builder.build().expect("the host registers").facts_artifact()
}

/// The shared `relay` bundle with [`facts_for`]'s companion for `acme_label`
/// under `aliases`.
fn relay(aliases: &[&str]) -> Bundle {
    Bundle::relay().with_member("schemas/functions.facts.json", &facts_for(FUNCTIONS, &[("acme_label", aliases)]))
}

/// [`FUNCTIONS`] with a second function, `acme_tag`.
fn with_tag() -> String {
    format!(
        "{FUNCTIONS}    acme_tag: |-\n        function(\n            parameters(name: string);\n            returns(string);\n            category(Acme);\n        ) -> The acme tag of a name.\n"
    )
}

/// A repository workspace registering `registered` (folders under `home/`)
/// in strict expression mode, where a call's arguments are checked.
fn strict_repository(root: &Path, registered: &[&str]) {
    repository(root, registered);
    write(root, ".dmls.toml", "[expressions]\nstrict = true\n");
}

fn warnings_on(fixture: &mut LspFixture<'_>, suffix: &str) -> Vec<Value> {
    registration_on(fixture, suffix).unwrap_or_default()
}

/// A companion missing when the session starts refuses the relay at the
/// catalog's `files` entry, naming the expected sibling; nothing of the
/// host is exposed. Publishing the bundle loads the host into the open
/// document and clears the warning.
#[test]
fn a_missing_companion_at_startup_refuses_the_host_until_it_is_published() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    strict_repository(root, &[".relay"]);
    let bundle = relay(&[]);
    bundle.write_to(&root.join("home/.relay"));
    let companion = root.join(COMPANION);
    std::fs::remove_file(&companion).unwrap();
    let mut fixture = start(&workspace);
    let (document, diagnostics) = open_text(&mut fixture, &workspace, "notes/calls.md", CALLS);
    assert_eq!(exposure(&mut fixture, &document, &diagnostics), NONE, "{diagnostics:#?}");

    settle(&mut fixture, &document);
    let warnings = warnings_on(&mut fixture, RELAY_MANIFEST);
    let [warning] = warnings.as_slice() else { panic!("one warning on the manifest: {warnings:#?}") };
    let manifest = std::fs::read_to_string(root.join(RELAY_MANIFEST)).unwrap();
    assert_eq!(warning["range"]["start"]["line"], json!(line_containing(&manifest, "- ./schemas/functions.yaml")), "{warning:#?}");
    assert!(warning["message"].as_str().unwrap().contains("functions.facts.json"), "{warning:#?}");

    bundle.write_to(&root.join("home/.relay"));
    report(&mut fixture, &document, &[(&companion, CREATED), (&root.join(RELAY_MANIFEST), CHANGED)]);
    let mut previous = diagnostics;
    let diagnostics = latest(&mut fixture, &document, &mut previous);
    assert_eq!(exposure(&mut fixture, &document, &diagnostics), SELECTED, "{diagnostics:#?}");
    assert!(warnings_on(&mut fixture, RELAY_MANIFEST).is_empty(), "the warning is cleared");
    fixture.shutdown();
}

/// Removing a valid companion withdraws the host's completions and call
/// diagnostics; restoring it brings them back. Bytes the manifest does not
/// address refuse the host the same way, and the addressed bytes recover
/// it.
#[test]
fn a_removed_or_stale_companion_withdraws_the_host_and_its_restoration_recovers_it() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    strict_repository(root, &[".relay"]);
    relay(&[]).write_to(&root.join("home/.relay"));
    let companion = root.join(COMPANION);
    let valid = std::fs::read_to_string(&companion).unwrap();
    let mut fixture = start(&workspace);
    let (document, mut previous) = open_text(&mut fixture, &workspace, "notes/calls.md", CALLS);
    let diagnostics = previous.clone();
    assert_eq!(exposure(&mut fixture, &document, &diagnostics), SELECTED, "{diagnostics:#?}");

    std::fs::remove_file(&companion).unwrap();
    report(&mut fixture, &document, &[(&companion, DELETED)]);
    let diagnostics = latest(&mut fixture, &document, &mut previous);
    assert_eq!(exposure(&mut fixture, &document, &diagnostics), NONE, "removed: {diagnostics:#?}");
    assert_eq!(warnings_on(&mut fixture, RELAY_MANIFEST).len(), 1, "the refusal is on the manifest");

    std::fs::write(&companion, &valid).unwrap();
    report(&mut fixture, &document, &[(&companion, CREATED)]);
    let diagnostics = latest(&mut fixture, &document, &mut previous);
    assert_eq!(exposure(&mut fixture, &document, &diagnostics), SELECTED, "restored: {diagnostics:#?}");
    assert!(warnings_on(&mut fixture, RELAY_MANIFEST).is_empty(), "the warning is cleared");

    let stale = valid.replacen('{', "{ ", 1);
    assert_ne!(stale, valid);
    std::fs::write(&companion, &stale).unwrap();
    report(&mut fixture, &document, &[(&companion, CHANGED)]);
    let diagnostics = latest(&mut fixture, &document, &mut previous);
    assert_eq!(exposure(&mut fixture, &document, &diagnostics), NONE, "stale: {diagnostics:#?}");
    let warnings = warnings_on(&mut fixture, RELAY_MANIFEST);
    assert!(
        warnings.len() == 1 && warnings[0]["message"].as_str().unwrap().contains("digest"),
        "the stale bytes are refused at their digest: {warnings:#?}"
    );

    std::fs::write(&companion, &valid).unwrap();
    report(&mut fixture, &document, &[(&companion, CHANGED)]);
    let diagnostics = latest(&mut fixture, &document, &mut previous);
    assert_eq!(exposure(&mut fixture, &document, &diagnostics), SELECTED, "restored bytes: {diagnostics:#?}");
    fixture.shutdown();
}

/// A republish adding `acme_tag`, reported in three batches 100 ms apart
/// (catalog, companion, manifest), reloads once 300 ms after the last batch
/// for both open documents; until then both keep the installed revision. An
/// edit to a document afterwards reloads nothing.
#[test]
fn a_batched_publish_reloads_once_for_every_open_document_and_an_edit_reloads_nothing() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    strict_repository(root, &[".relay"]);
    let install = root.join("home/.relay");
    relay(&[]).write_to(&install);
    let clock = Arc::new(ManualClock::new());
    let mut fixture = LspFixture::start_with_registration_settle(&workspace, clock.clone());
    fixture.initialize(watching_params(root));
    let probe_text = format!("{CALLS}{{{{ acme_t }}}}\n");
    let (mut first, _) = open_text(&mut fixture, &workspace, "notes/first.md", &probe_text);
    let (second, _) = open_text(&mut fixture, &workspace, "notes/second.md", &probe_text);
    let advance = |fixture: &mut LspFixture<'_>, document: &Document, millis: u64| {
        clock.advance(Duration::from_millis(millis));
        settle(fixture, document);
    };
    let loads = host_loads();
    let offers_tag = |fixture: &mut LspFixture<'_>, document: &Document| {
        completion_at(fixture, document, "{{ acme_t").contains(&"acme_tag".to_string())
    };
    assert!(!offers_tag(&mut fixture, &first) && !offers_tag(&mut fixture, &second), "no `acme_tag` yet");

    let catalog = with_tag();
    let republished = Bundle::relay()
        .with_member("schemas/functions.yaml", &catalog)
        .with_member("schemas/functions.facts.json", &facts_for(&catalog, &[("acme_label", &[]), ("acme_tag", &[])]));
    write(root, "home/.relay/schemas/functions.yaml", &catalog);
    report(&mut fixture, &first, &[(&install.join("schemas/functions.yaml"), CHANGED)]);
    advance(&mut fixture, &first, 100);
    write(root, COMPANION, &republished.members["schemas/functions.facts.json"]);
    report(&mut fixture, &first, &[(&root.join(COMPANION), CHANGED)]);
    advance(&mut fixture, &first, 100);
    republished.write_to(&install);
    report(&mut fixture, &first, &[(&install.join(MANIFEST_NAME), CHANGED)]);
    advance(&mut fixture, &first, 299);
    assert_eq!(host_loads(), loads, "nothing reloads inside the window");
    assert!(!offers_tag(&mut fixture, &first) && !offers_tag(&mut fixture, &second), "the installed revision is served");

    advance(&mut fixture, &first, 1);
    assert!(offers_tag(&mut fixture, &first) && offers_tag(&mut fixture, &second), "both documents follow the new revision");
    assert_eq!(host_loads() - loads, 1, "one load for the settled revision, whatever the number of documents");

    first.text = format!("{probe_text}More text.\n");
    fixture.notify(
        "textDocument/didChange",
        json!({ "textDocument": { "uri": first.uri, "version": 2 }, "contentChanges": [{ "text": first.text }] }),
    );
    fixture.wait_for_diagnostics(&first.uri);
    advance(&mut fixture, &first, REGISTRATION_SETTLE.as_millis() as u64);
    assert_eq!(host_loads() - loads, 1, "a document edit reloads nothing");
    assert!(offers_tag(&mut fixture, &first), "and keeps the host");
    fixture.shutdown();
}

/// With the relay registered, an ordinary document and a document the
/// function-less `acme` host identifies see no `acme_label`: only the
/// selected host's functions are entered.
#[test]
fn an_unselected_registration_exposes_no_host_function() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    strict_repository(root, &[".relay", ".acme"]);
    relay(&[]).write_to(&root.join("home/.relay"));
    let mut acme = Bundle::acme().without_file("schemas/functions.yaml");
    acme.members.remove("schemas/functions.facts.json");
    acme.write_to(&root.join("home/.acme"));
    let mut fixture = start(&workspace);

    let (relay, diagnostics) = open_text(&mut fixture, &workspace, "notes/relay.md", CALLS);
    assert_eq!(exposure(&mut fixture, &relay, &diagnostics), SELECTED, "{diagnostics:#?}");
    for (name, frontmatter) in [("plain", "title: plain"), ("gadget", "gadget: name")] {
        let text = CALLS.replace("sequence:\n  - name: one\n    prompt: hi", frontmatter);
        let (document, diagnostics) = open_text(&mut fixture, &workspace, &format!("notes/{name}.md"), &text);
        assert_eq!(exposure(&mut fixture, &document, &diagnostics), NONE, "{name}: {diagnostics:#?}");
    }
    fixture.shutdown();
}

/// A selected host's function is found by its canonical name or an alias
/// without regard to case, in call checks and in completion, which offers
/// the canonical name.
#[test]
fn a_selected_hosts_function_is_found_case_insensitively_by_name_or_alias() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    strict_repository(root, &[".relay"]);
    relay(&["acmeLabel"]).write_to(&root.join("home/.relay"));
    let mut fixture = start(&workspace);
    let text = "---\nsequence:\n  - name: one\n    prompt: hi\n---\n{{ ACME_LABEL(42) }} {{ AcmeLabel(42) }} {{ ACME_LA }} {{ acmel }}\n";
    let (document, diagnostics) = open_text(&mut fixture, &workspace, "notes/cases.md", text);
    assert!(with_code(&diagnostics, UNKNOWN_FUNCTION).is_empty(), "{diagnostics:#?}");
    assert_eq!(with_code(&diagnostics, INCOMPATIBLE).len(), 2, "both spellings bind the host's signature: {diagnostics:#?}");
    for partial in ["{{ ACME_LA", "{{ acmel"] {
        let offered = completion_at(&mut fixture, &document, partial);
        assert!(offered.contains(&"acme_label".to_string()), "{partial}: {offered:?}");
    }
    fixture.shutdown();
}

#[test]
fn a_refused_callable_name_is_never_offered_by_completion() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    strict_repository(root, &[".relay"]);
    let facts = facts_for(FUNCTIONS, &[("acme_label", &["acme.alias"])]);
    let refused = facts.replace("\"acme.alias\"", "\"acme.alias\", \"acme label\"");
    assert_ne!(facts, refused);
    relay(&[]).with_member("schemas/functions.facts.json", &refused).write_to(&root.join("home/.relay"));
    let mut fixture = start(&workspace);
    let text = format!("{CALLS}{{{{ acme.ali }}}}");
    let (document, diagnostics) = open_text(&mut fixture, &workspace, "notes/calls.md", &text);
    assert_eq!(exposure(&mut fixture, &document, &diagnostics), NONE, "{diagnostics:#?}");
    assert!(!completion_at(&mut fixture, &document, "{{ acme.ali").contains(&"acme_label".to_string()));

    settle(&mut fixture, &document);
    let warnings = warnings_on(&mut fixture, COMPANION);
    let [warning] = warnings.as_slice() else { panic!("one warning on the companion: {warnings:#?}") };
    assert!(warning["message"].as_str().unwrap().contains("cannot be used as an expression call name"), "{warning:#?}");
    fixture.shutdown();
}

/// A second catalog claiming `acme_label` refuses the relay at its own
/// declaration, with the earlier catalog's declaration as related
/// information; the host exposes nothing.
#[test]
fn a_cross_catalog_name_collision_points_at_the_earlier_declaration() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    strict_repository(root, &[".relay"]);
    let more = FUNCTIONS.replace("The acme label of a name.", "Another label of a name.");
    relay(&[])
        .with_file("schemas/more.yaml", &more)
        .with_member("schemas/more.facts.json", &facts_for(&more, &[("acme_label", &[])]))
        .write_to(&root.join("home/.relay"));
    let mut fixture = start(&workspace);
    let (document, diagnostics) = open_text(&mut fixture, &workspace, "notes/calls.md", CALLS);
    assert_eq!(exposure(&mut fixture, &document, &diagnostics), NONE, "{diagnostics:#?}");

    settle(&mut fixture, &document);
    let warnings = warnings_on(&mut fixture, "home/.relay/schemas/more.yaml");
    let [warning] = warnings.as_slice() else { panic!("one warning on the later catalog: {warnings:#?}") };
    assert_eq!(warning["range"]["start"]["line"], json!(line_containing(&more, "acme_label:")), "{warning:#?}");
    let related = warning["relatedInformation"].as_array().unwrap_or_else(|| panic!("related information: {warning:#?}"));
    let [earlier] = related.as_slice() else { panic!("one related location: {related:#?}") };
    assert!(earlier["location"]["uri"].as_str().unwrap().ends_with("home/.relay/schemas/functions.yaml"), "{earlier:#?}");
    assert_eq!(earlier["location"]["range"]["start"]["line"], json!(line_containing(FUNCTIONS, "acme_label:")), "{earlier:#?}");
    assert!(!earlier["message"].as_str().unwrap_or_default().is_empty(), "{earlier:#?}");
    fixture.shutdown();
}

/// An expression catalog and its companion present beside the bundle's
/// members and in the tree's schema root, but not listed by the manifest,
/// expose nothing: the relay loads without functions.
#[test]
fn a_catalog_the_manifest_does_not_list_exposes_nothing() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    strict_repository(root, &[".relay"]);
    let mut bundle = relay(&[]).without_file("schemas/functions.yaml");
    bundle.members.remove("schemas/functions.facts.json");
    bundle.write_to(&root.join("home/.relay"));
    let facts = facts_for(FUNCTIONS, &[("acme_label", &[])]);
    for folder in ["home/.relay/schemas", "schemas"] {
        write(root, &format!("{folder}/functions.yaml"), FUNCTIONS);
        write(root, &format!("{folder}/functions.facts.json"), &facts);
    }
    let mut fixture = start(&workspace);
    let (document, diagnostics) = open_text(&mut fixture, &workspace, "notes/calls.md", CALLS);
    assert_eq!(exposure(&mut fixture, &document, &diagnostics), NONE, "{diagnostics:#?}");
    assert!(with_code(&diagnostics, REGISTRATION).is_empty(), "the relay itself loads: {diagnostics:#?}");
    settle(&mut fixture, &document);
    assert!(warnings_on(&mut fixture, RELAY_MANIFEST).is_empty(), "nothing is refused");
    fixture.shutdown();
}

/// In-process: the loader's watch inputs include the facts companion both
/// when the profile is published and when a missing companion refuses the
/// registration, so either change schedules a reload.
#[test]
fn the_companion_is_a_watched_input_of_a_published_and_a_refused_load() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let fixture = crate::host_fixtures::Fixture::new();
    fixture.write("home/.darkmatter.yaml", "register:\n  - ../host/darkmatter-host.yaml\n");
    let context = fixture.context();
    let companion: PathBuf = fixture.path("host/schemas/functions.facts.json");
    let mut registrations = HostRegistrations::default();
    let watched = |registrations: &HostRegistrations| -> BTreeSet<PathBuf> {
        registrations.dependencies().into_iter().map(|path| biscuit_file::canonicalize_simplified(&path).unwrap_or(path)).collect()
    };
    let normalized = |path: &Path| biscuit_file::canonicalize_simplified(path).unwrap_or_else(|_| path.to_path_buf());

    registrations.prepare(&context, Instant::now());
    assert!(registrations.diagnostics().is_empty(), "{:#?}", registrations.diagnostics());
    assert!(watched(&registrations).contains(&normalized(&companion)), "published: {:#?}", watched(&registrations));

    std::fs::remove_file(&companion).unwrap();
    registrations.invalidate_paths(std::slice::from_ref(&companion), Instant::now());
    registrations.prepare(&context, Instant::now());
    assert_eq!(registrations.diagnostics().len(), 1, "{:#?}", registrations.diagnostics());
    let refused = watched(&registrations);
    assert!(
        refused.iter().any(|path| path.ends_with("host/schemas/functions.facts.json")),
        "refused: {refused:#?}"
    );
}
