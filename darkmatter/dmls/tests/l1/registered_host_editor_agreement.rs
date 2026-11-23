//! Every editor feature reads one host view per document, so diagnostics,
//! hover, completion, and lifecycle analysis agree on whether a registered
//! host applies: after an atomic replacement, a deletion, and the recovery
//! from each, at startup over a refused install, over duplicate
//! registrations, and over a configuration the contract rejects. A
//! registered host's required defaults are pending, never missing.
//!
//! Boundary: a hermetic stdio session (`LspFixture`) whose home is a folder
//! of its temporary repository, with the shared `relay` install from
//! Darkmatter's host fixtures. Nothing is read from the real home or
//! checkout.

use std::path::Path;

use serde_json::{Value, json};

use crate::common::{LspFixture, LspWorkspace};
use crate::host_fixtures::{Bundle, RELAY_NAMESPACE, RELAY_TRIGGER};
use crate::host_registration_watches::{CHANGED, CREATED, DELETED, registration_on, report, start, uri};
use crate::registered_host_views::{
    MOTIVATING, REGISTRATION, UNAVAILABLE, UNKNOWN_FUNCTION, UNKNOWN_HOST, completion_at, hover_at, install_relay,
    line_containing, repository, with_code, write,
};
use crate::union_item_editor::{Document, open_text};

/// Every file a relay install, or the configuration registering it, can be
/// warned on.
const WARNABLE: [&str; 7] = [
    "home/.darkmatter.yaml",
    "home/.relay/darkmatter-host.yaml",
    "home/.relay/schemas/relay.yaml",
    "home/.relay/schemas/seq.yaml",
    "home/.relay/schemas/globals.lifecycle.json",
    "home/.relay/schemas/functions.yaml",
    "home/.relay/schemas/functions.facts.json",
];

/// What each editor feature says about [`MOTIVATING`].
#[derive(Debug, PartialEq)]
struct Analysis {
    /// Completion offers the relay's `seq.name`.
    completes_seq: bool,
    /// Hover over `seq.name` shows a description from any relay catalog.
    hovers_seq: bool,
    /// `start:` is a lifecycle stack (its `err` read is unavailable there),
    /// not data.
    start_is_a_stack: bool,
    /// The relay's `acme_label` function is typed.
    knows_host_function: bool,
    unknown_host: usize,
    registration_on_document: usize,
}

const RELAY: Analysis = Analysis {
    completes_seq: true,
    hovers_seq: true,
    start_is_a_stack: true,
    knows_host_function: true,
    unknown_host: 0,
    registration_on_document: 0,
};

const MD: Analysis = Analysis {
    completes_seq: false,
    hovers_seq: false,
    start_is_a_stack: false,
    knows_host_function: false,
    unknown_host: 0,
    registration_on_document: 0,
};

fn analysis(fixture: &mut LspFixture<'_>, document: &Document, diagnostics: &[Value]) -> Analysis {
    let start_line = line_containing(&document.text, "in start");
    Analysis {
        completes_seq: completion_at(fixture, document, "{{ seq.na").contains(&"seq.name".to_string()),
        hovers_seq: hover_at(fixture, document, "name }}").contains("step's name"),
        start_is_a_stack: with_code(diagnostics, UNAVAILABLE)
            .iter()
            .any(|diagnostic| diagnostic["range"]["start"]["line"] == json!(start_line)),
        knows_host_function: with_code(diagnostics, UNKNOWN_FUNCTION).is_empty(),
        unknown_host: with_code(diagnostics, UNKNOWN_HOST).len(),
        registration_on_document: with_code(diagnostics, REGISTRATION).len(),
    }
}

/// The document's analysis after a reported change refreshed it.
fn refreshed(fixture: &mut LspFixture<'_>, document: &Document) -> Analysis {
    let diagnostics = fixture.take_latest_diagnostics_ending(&document.uri).expect("the document was refreshed");
    analysis(fixture, document, &diagnostics)
}

/// `(file, warning count)` for every [`WARNABLE`] file published since the
/// last call, in that order.
fn registration_warnings(fixture: &mut LspFixture<'_>) -> Vec<(&'static str, usize)> {
    WARNABLE
        .into_iter()
        .filter_map(|suffix| registration_on(fixture, suffix).map(|warnings| (suffix, warnings.len())))
        .filter(|(_, count)| *count > 0)
        .collect()
}

/// Replaces `path` the way an editor's atomic save does: a temporary file
/// renamed over it, reported as the client's watcher sees it.
fn replace_atomically(fixture: &mut LspFixture<'_>, document: &Document, path: &Path, text: &str) {
    let temporary = path.with_file_name(format!(".{}.tmp", path.file_name().unwrap().to_string_lossy()));
    std::fs::write(&temporary, text).unwrap();
    std::fs::rename(&temporary, path).unwrap();
    report(fixture, document, &[(&temporary, CREATED), (&temporary, DELETED), (path, CHANGED)]);
}

/// The `seq` namespace file is atomically replaced with bytes its manifest
/// does not address: neither the old nor the new namespace's facts are
/// served, by any feature, until the addressed bytes are restored.
#[test]
fn every_feature_drops_the_host_after_an_invalid_atomic_replacement_and_restores_it() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    repository(root, &[".relay"]);
    install_relay(root, ".relay");
    let mut fixture = start(&workspace);
    let (document, diagnostics) = open_text(&mut fixture, &workspace, "notes/relay.md", MOTIVATING);
    assert_eq!(analysis(&mut fixture, &document, &diagnostics), RELAY);

    let namespace = root.join("home/.relay/schemas/seq.yaml");
    let stale = RELAY_NAMESPACE.replace("The current step's name.", "The step's name, renamed.");
    replace_atomically(&mut fixture, &document, &namespace, &stale);
    assert_eq!(refreshed(&mut fixture, &document), MD, "no last-good or partial host facts");
    let warned = registration_warnings(&mut fixture);
    assert_eq!(warned.len(), 1, "one warning, on the refused install: {warned:?}");

    replace_atomically(&mut fixture, &document, &namespace, RELAY_NAMESPACE);
    assert_eq!(refreshed(&mut fixture, &document), RELAY);
    assert_eq!(registration_warnings(&mut fixture), [], "the warning is cleared");
    fixture.shutdown();
}

/// Deleting the manifest unregisters the host for every feature and warns
/// on the configuration entry naming it; recreating it restores the host.
#[test]
fn every_feature_drops_the_host_after_its_manifest_is_deleted_and_restores_it() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    repository(root, &[".relay"]);
    install_relay(root, ".relay");
    let mut fixture = start(&workspace);
    let (document, diagnostics) = open_text(&mut fixture, &workspace, "notes/relay.md", MOTIVATING);
    assert_eq!(analysis(&mut fixture, &document, &diagnostics), RELAY);

    let manifest = root.join("home/.relay/darkmatter-host.yaml");
    let saved = std::fs::read_to_string(&manifest).unwrap();
    std::fs::remove_file(&manifest).unwrap();
    report(&mut fixture, &document, &[(&manifest, DELETED)]);
    assert_eq!(refreshed(&mut fixture, &document), MD);
    assert_eq!(registration_warnings(&mut fixture), [("home/.darkmatter.yaml", 1)]);

    std::fs::write(&manifest, saved).unwrap();
    report(&mut fixture, &document, &[(&manifest, CREATED)]);
    assert_eq!(refreshed(&mut fixture, &document), RELAY);
    assert_eq!(registration_warnings(&mut fixture), [], "the warning is cleared");
    fixture.shutdown();
}

/// A relay install refused when the session starts: the document is an
/// `md` document for every feature, and one located warning names the file
/// that is wrong.
macro_rules! refused_install {
    ($($name:ident: $break_install:expr => $warned_on:literal at $needle:expr;)+) => {$(
        #[test]
        fn $name() {
            let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
            let workspace = LspWorkspace::new();
            let root = workspace.path();
            repository(root, &[".relay"]);
            install_relay(root, ".relay");
            let break_install: fn(&Path) = $break_install;
            break_install(root);
            let mut fixture = start(&workspace);
            let (document, diagnostics) = open_text(&mut fixture, &workspace, "notes/relay.md", MOTIVATING);
            assert_eq!(analysis(&mut fixture, &document, &diagnostics), MD, "{diagnostics:#?}");

            // One warning per refusal: a broken library is refused for
            // itself and for each importer, all at the same declaration.
            let warnings = registration_on(&mut fixture, $warned_on).unwrap_or_default();
            assert!(!warnings.is_empty(), "the file is warned");
            let declaring = std::fs::read_to_string(root.join($warned_on)).unwrap();
            let needle: Option<&str> = $needle;
            let line = needle.map_or(0, |needle| line_containing(&declaring, needle));
            for warning in &warnings {
                assert_eq!(warning["range"]["start"]["line"], json!(line), "{warnings:#?}");
            }
            assert_eq!(registration_warnings(&mut fixture), [], "no other file is warned");
            fixture.shutdown();
        }
    )+};
}

refused_install! {
    a_stale_lifecycle_artifact_refuses_the_host:
        |root| Bundle::relay()
            .with_member("schemas/seq.yaml", &RELAY_NAMESPACE.replace("current step", "present step"))
            .write_to(&root.join("home/.relay"))
        => "home/.relay/schemas/globals.lifecycle.json" at Some("\"catalog_hash\"");
    a_facts_artifact_for_another_catalog_refuses_the_host:
        |root| {
            let bundle = Bundle::relay();
            let text = bundle.members["schemas/functions.yaml"].replace("The acme label", "The relay label");
            bundle.with_member("schemas/functions.yaml", &text).write_to(&root.join("home/.relay"));
        }
        => "home/.relay/schemas/functions.facts.json" at Some("\"catalog_hash\"");
    an_error_in_an_imported_library_is_warned_in_the_library_at_the_declaration:
        |root| Bundle::relay()
            .with_file("schemas/lib.yaml", "kind: schema\ntypes:\n    span: \"numbr -> a duration\"\n")
            .with_member(
                "schemas/relay.yaml",
                &RELAY_TRIGGER.replace("\"string -> the run's timeout, such as 30m\"", "\"span@./lib.yaml -> the run's timeout\""),
            )
            .write_to(&root.join("home/.relay"))
        => "home/.relay/schemas/lib.yaml" at Some("numbr");
    a_failing_grouped_match_condition_is_warned_at_the_condition_not_the_payload:
        |root| Bundle::relay()
            .with_member("schemas/relay.yaml", &RELAY_TRIGGER.replace("start: \"unknown[](required)\"", "start: \"unknown[](requird)\""))
            .write_to(&root.join("home/.relay"))
        => "home/.relay/schemas/relay.yaml" at Some("unknown[](requird)");
    an_unlisted_example_repeated_in_prose_is_warned_at_the_reference:
        |root| {
            let trigger = RELAY_TRIGGER.replacen(
                "kind: schema-trigger\n",
                "kind: schema-trigger\ndescription: Settings may come from ./options.yaml.\n",
                1,
            );
            let trigger = format!("{trigger}types:\n    review-note: string(example(./options.yaml))\n");
            Bundle::relay().with_member("schemas/relay.yaml", &trigger).write_to(&root.join("home/.relay"));
            std::fs::write(root.join("home/.relay/schemas/options.yaml"), "kind: schema\ntypes:\n    Options: string\n").unwrap();
        }
        => "home/.relay/schemas/relay.yaml" at Some("example(./options.yaml)");
    an_unlisted_import_after_a_sibling_description_is_warned_at_the_reference:
        |root| {
            let trigger = format!(
                "{RELAY_TRIGGER}types:\n    review-setting: 'string -> Read options from ./options.yaml.'\n    review-entry: Options@./options.yaml\n"
            );
            Bundle::relay().with_member("schemas/relay.yaml", &trigger).write_to(&root.join("home/.relay"));
            std::fs::write(root.join("home/.relay/schemas/options.yaml"), "kind: schema\ntypes:\n    Options: string\n").unwrap();
        }
        => "home/.relay/schemas/relay.yaml" at Some("review-entry: Options@");
    a_manifest_naming_a_missing_member_refuses_the_host:
        |root| std::fs::remove_file(root.join("home/.relay/schemas/functions.facts.json")).unwrap()
        => "home/.relay/darkmatter-host.yaml" at Some("facts:");
    a_register_entry_naming_a_missing_manifest_applies_no_host:
        |root| std::fs::remove_file(root.join("home/.relay/darkmatter-host.yaml")).unwrap()
        => "home/.darkmatter.yaml" at Some("./.relay/darkmatter-host.yaml");
}

/// One manifest registered more than once, or two installs with the same
/// content: one host, every feature applies it, and nothing is warned.
macro_rules! duplicate_registration {
    ($($name:ident: $register:expr;)+) => {$(
        #[test]
        fn $name() {
            let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
            let workspace = LspWorkspace::new();
            let root = workspace.path();
            repository(root, &[]);
            install_relay(root, ".relay");
            let register: fn(&Path) = $register;
            register(root);
            let mut fixture = start(&workspace);
            let (document, diagnostics) = open_text(&mut fixture, &workspace, "notes/relay.md", MOTIVATING);
            assert_eq!(analysis(&mut fixture, &document, &diagnostics), RELAY, "{diagnostics:#?}");
            assert_eq!(registration_warnings(&mut fixture), []);
            fixture.shutdown();
        }
    )+};
}

duplicate_registration! {
    the_user_and_repository_files_registering_one_manifest_load_it_once: |root| {
        write(root, "home/.darkmatter.yaml", "register:\n  - ./.relay/darkmatter-host.yaml\n");
        write(root, ".darkmatter.yaml", "register:\n  - ./home/.relay/darkmatter-host.yaml\n");
    };
    one_manifest_spelled_two_ways_loads_once: |root| {
        let entries = "register:\n  - ./.relay/darkmatter-host.yaml\n  - ./.relay/schemas/../darkmatter-host.yaml\n";
        write(root, "home/.darkmatter.yaml", entries);
    };
    two_identical_installs_agree_on_one_host: |root| {
        install_relay(root, ".relay-copy");
        let entries = "register:\n  - ./.relay/darkmatter-host.yaml\n  - ./.relay-copy/darkmatter-host.yaml\n";
        write(root, "home/.darkmatter.yaml", entries);
    };
}

/// A configuration whose `register` value the contract rejects shows that
/// error on the open configuration and loads no host from it, so the
/// document is analyzed without the relay; fixing it registers the relay.
#[test]
fn a_rejected_register_value_loads_no_host_until_it_is_fixed() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    repository(root, &[]);
    install_relay(root, ".relay");
    let config = root.join("home/.darkmatter.yaml");
    write(root, "home/.darkmatter.yaml", "register: 5\n");
    let mut fixture = start(&workspace);
    let (document, diagnostics) = open_text(&mut fixture, &workspace, "notes/relay.md", MOTIVATING);
    assert_eq!(analysis(&mut fixture, &document, &diagnostics), MD, "{diagnostics:#?}");

    fixture.notify(
        "textDocument/didOpen",
        json!({ "textDocument": {
            "uri": uri(&config),
            "languageId": "yaml", "version": 1, "text": "register: 5\n"
        } }),
    );
    fixture.flush_server_with("textDocument/documentSymbol", json!({ "textDocument": { "uri": document.uri } }));
    let published = fixture.take_latest_diagnostics_ending("home/.darkmatter.yaml").expect("published");
    let errors: Vec<&Value> = published.iter().filter(|diagnostic| diagnostic["severity"] == json!(1)).collect();
    assert_eq!(errors.len(), 1, "the contract error: {published:#?}");
    assert_eq!(errors[0]["range"]["start"]["line"], json!(0), "{errors:#?}");

    write(root, "home/.darkmatter.yaml", "register:\n  - ./.relay/darkmatter-host.yaml\n");
    report(&mut fixture, &document, &[(&config, CHANGED)]);
    assert_eq!(refreshed(&mut fixture, &document), RELAY);
    fixture.shutdown();
}

/// A required property the registered host's trigger defaults is pending:
/// strict schema checking reports the required property without a default
/// and never the defaulted one, and hover shows the default. An open
/// configuration file that omits its defaulted settings is not in error
/// either.
#[test]
fn a_registered_hosts_required_defaults_are_pending_not_missing() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let workspace = LspWorkspace::new();
    let root = workspace.path();
    repository(root, &[".relay"]);
    install_relay(root, ".relay");
    write(root, ".dmls.toml", "[expressions]\nstrict = true\n[schema]\nstrict = true\n");
    let trigger = RELAY_TRIGGER
        .replace(
            "timeout: \"string -> the run's timeout",
            "budget: \"number(required) -> the run's budget\"\n    timeout: \"string(required; default(30m)) -> the run's timeout",
        );
    assert_ne!(trigger, RELAY_TRIGGER);
    Bundle::relay().with_member("schemas/relay.yaml", &trigger).write_to(&root.join("home/.relay"));
    let mut fixture = start(&workspace);

    let text = "---\nsequence:\n  - name: one\n    prompt: hi\n---\n{{ timeout }} {{ seq.name }}\n";
    let (document, diagnostics) = open_text(&mut fixture, &workspace, "notes/relay.md", text);
    let missing: Vec<&str> = with_code(&diagnostics, "dm.schema.missing_required")
        .iter()
        .map(|diagnostic| diagnostic["message"].as_str().unwrap())
        .collect();
    assert_eq!(missing, ["\"budget\" is a required property"], "the default answers `timeout`: {diagnostics:#?}");
    let hover = hover_at(&mut fixture, &document, "imeout }}");
    assert!(hover.contains("Default: _\"30m\"_"), "{hover}");
    assert!(hover_at(&mut fixture, &document, "name }}").contains("The current step's name."));

    let config = root.join("home/.darkmatter.yaml");
    let config_text = format!("{}permissions:\n  network: deny\n", std::fs::read_to_string(&config).unwrap());
    fixture.notify(
        "textDocument/didOpen",
        json!({ "textDocument": {
            "uri": uri(&config),
            "languageId": "yaml", "version": 1, "text": config_text
        } }),
    );
    fixture.flush_server_with("textDocument/documentSymbol", json!({ "textDocument": { "uri": document.uri } }));
    let published = fixture.take_latest_diagnostics_ending("home/.darkmatter.yaml").unwrap_or_default();
    assert_eq!(published, Vec::<Value>::new(), "omitted defaulted settings are not errors");
    fixture.shutdown();
}
