//! Host function scope through `md compose`: a registered host's expression
//! catalog is visible only while `md` composes a document that host
//! identifies. Its functions are analysis-only in `md` (the companion's facts
//! type every call, and no handler exists), so a call that does not bind is
//! rejected by the passive call check, and a call that binds fails when
//! evaluated because it needs the host runtime. The same call in an ordinary
//! Markdown document, or in a document another registered host identifies,
//! names an unknown function.
//!
//! The `acme` host's catalog declares `acme_double(value: number)`; its
//! registration excludes a string source for `value` before any conversion,
//! so `acme_double('2')` is rejected although `'2'` converts to a number. The
//! `relay` host is registered beside it with its own catalog.

use std::sync::Arc;

use crate::common;
use crate::host_fixtures::{Bundle, MANIFEST_NAME, companion_of};

use common::CliProcessFixture;
use darkmatter::markdown::compose::expression::{HostFunctionCatalog, HostFunctions, HostHandler};
use serde_json::{Value, json};
use simplified_schema::SimplifiedSchema;
use simplified_schema::coercion::ValueKind;
use std::process::Output;

const CATALOG: &str = "schemas/functions.yaml";

const DOUBLE_CATALOG: &str = "\
kind: expression-schema
categories:
    - Acme -> Ask the acme host.
types:
    acme_double: |-
        function(
            parameters(value: number);
            returns(number);
            category(Acme);
        ) -> Twice the value.
";

/// The companion the acme host's registration gives [`DOUBLE_CATALOG`]: a
/// string source for `value` is refused before conversion.
fn double_facts() -> String {
    let catalog = HostFunctionCatalog::load(DOUBLE_CATALOG, "functions.yaml").expect("the catalog loads");
    let handler: HostHandler = Arc::new(|call| Ok(json!(call.args().number(0).unwrap_or_default() * 2.0)));
    HostFunctions::builder(catalog)
        .function("acme_double", &[], |registration| registration.reject_sources("value", &[ValueKind::Text]), handler)
        .build()
        .expect("the host registers")
        .facts_artifact()
}

fn acme() -> Bundle {
    Bundle::acme().with_member(CATALOG, DOUBLE_CATALOG).with_member(&companion_of(CATALOG), &double_facts())
}

/// A fixture whose home registers `acme` (under `home/.acme/`) and `relay`
/// (under `home/.relay/`).
fn registered(name: &str, acme: &Bundle) -> CliProcessFixture {
    let fixture = CliProcessFixture::named(name);
    for (folder, bundle) in [(".acme", acme), (".relay", &Bundle::relay())] {
        for (file, text) in bundle.files_with_manifest() {
            fixture.write_file(format!("home/{folder}/{file}"), &text);
        }
    }
    fixture.write_file(
        "home/.darkmatter.yaml",
        &format!("register:\n  - ./.acme/{MANIFEST_NAME}\n  - ./.relay/{MANIFEST_NAME}\n"),
    );
    fixture
}

/// `md compose <file> --output json`: the one envelope, judged against its
/// schema, with stderr empty.
fn compose(fixture: &CliProcessFixture, file: &str) -> (Option<i32>, Value) {
    let output: Output = fixture.command().args(["compose", file, "--output", "json"]).output().expect("md runs");
    assert!(output.stderr.is_empty(), "a data output owns stderr too: {}", String::from_utf8_lossy(&output.stderr));
    let value: Value = serde_json::from_slice(&output.stdout).expect("one JSON value");
    let schema = SimplifiedSchema::try_from(include_str!("../../schemas/output/compose.yaml")).expect("the schema parses");
    if let Err(failure) = schema.validate_strict(&value) {
        panic!("{failure:?}\n{value:#}");
    }
    (output.status.code(), value)
}

fn records<'v>(value: &'v Value, code: &str) -> Vec<&'v Value> {
    value["diagnostics"].as_array().expect("diagnostics").iter().filter(|record| record["code"] == code).collect()
}

/// `(line, column, message)` of every record with `code`.
fn located(value: &Value, code: &str) -> Vec<(Option<u64>, Option<u64>, String)> {
    records(value, code)
        .iter()
        .map(|record| {
            let source = &record["source"];
            (source["line"].as_u64(), source["column"].as_u64(), record["message"].as_str().unwrap_or_default().to_string())
        })
        .collect()
}

/// An acme document: the `widget` trigger identifies it.
fn acme_document(body: &str) -> String {
    format!("---\nwidget:\n  size: 1\n---\n# Widget\n\n{body}\n")
}

const RUNTIME: &str = "host function acme_double() requires the host runtime";
const INCOMPATIBLE: &str = "dm.expression.incompatible_call";
const UNKNOWN: &str = "dm.expression.unknown_function";

#[test]
fn a_call_that_cannot_bind_is_rejected_and_one_that_binds_needs_the_host_runtime() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let fixture = registered("host-scope-calls", &acme());
    fixture.write_file("cwd/wrong.md", &acme_document("{{ acme_double(true) }}"));
    fixture.write_file("cwd/excluded.md", &acme_document("{{ acme_double('2') }}"));
    fixture.write_file("cwd/valid.md", &acme_document("{{ acme_double(2) }}"));

    let (wrong_code, wrong) = compose(&fixture, "wrong.md");
    let (excluded_code, excluded) = compose(&fixture, "excluded.md");
    let (valid_code, valid) = compose(&fixture, "valid.md");

    assert_eq!(wrong_code, Some(1), "{wrong:#}");
    let rejected = located(&wrong, INCOMPATIBLE);
    assert_eq!(rejected.len(), 1, "{wrong:#}");
    assert_eq!((rejected[0].0, rejected[0].1), (Some(7), Some(16)), "at the argument: {wrong:#}");
    assert!(rejected[0].2.contains("expected number"), "{wrong:#}");
    // `'2'` converts to a number; the companion refuses a text source first.
    assert_eq!(excluded_code, Some(1), "{excluded:#}");
    let rejected = located(&excluded, INCOMPATIBLE);
    assert_eq!(rejected.len(), 1, "the companion's source exclusion is honored: {excluded:#}");
    assert_eq!((rejected[0].0, rejected[0].1), (Some(7), Some(16)), "{excluded:#}");
    assert!(!excluded.to_string().contains(RUNTIME), "a rejected call is never evaluated: {excluded:#}");
    assert_eq!(valid_code, Some(1), "{valid:#}");
    assert!(valid["document"].is_null(), "nothing is written: {valid:#}");
    assert!(records(&valid, INCOMPATIBLE).is_empty(), "the call binds: {valid:#}");
    let failures = located(&valid, "dm.expression.evaluation_failure");
    assert_eq!(failures.len(), 1, "{valid:#}");
    assert_eq!((failures[0].0, failures[0].1), (Some(7), Some(1)), "{valid:#}");
    assert!(failures[0].2.contains(RUNTIME), "{valid:#}");
}

#[test]
fn every_transcluded_child_of_a_host_document_inherits_its_functions() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let fixture = registered("host-scope-children", &acme());
    // Several children, so the transclusion phase resolves them on pool
    // workers rather than inline.
    fixture.write_file("cwd/parent.md", &acme_document("::file ./one.md\n\n::file ./two.md\n\n::file ./three.md"));
    for child in ["one", "two", "three"] {
        fixture.write_file(format!("cwd/{child}.md"), &format!("{child} {{{{ acme_double(2) }}}}\n"));
    }

    let (_, value) = compose(&fixture, "parent.md");

    let text = value["diagnostics"].to_string();
    assert_eq!(text.matches(RUNTIME).count(), 3, "each child sees the host function: {value:#}");
    assert!(!text.contains("Unknown function"), "{value:#}");
}

#[test]
fn the_call_is_unknown_in_an_ordinary_document_and_in_another_hosts_document() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let fixture = registered("host-scope-unselected", &acme());
    fixture.write_file("cwd/notes.md", "---\ntitle: Notes\n---\n# Notes\n\n{{ acme_double(2) }}\n");
    fixture.write_file("cwd/relay.md", "---\nsequence: []\n---\n# Relay\n\n{{ acme_double(2) }}\n");

    let (notes_code, notes) = compose(&fixture, "notes.md");
    let (relay_code, relay) = compose(&fixture, "relay.md");

    for (code, value, host) in [(notes_code, &notes, None), (relay_code, &relay, Some("relay"))] {
        assert_eq!(code, Some(1), "{value:#}");
        let unknown = located(value, UNKNOWN);
        assert_eq!(unknown.len(), 1, "{value:#}");
        assert_eq!((unknown[0].0, unknown[0].1), (Some(6), Some(1)), "{value:#}");
        assert!(!value.to_string().contains(RUNTIME), "no acme scope leaks: {value:#}");
        let notices = records(value, "dm.host.foreign_document");
        match host {
            None => assert!(notices.is_empty(), "no host selected: {value:#}"),
            Some(host) => {
                assert_eq!(notices.len(), 1, "{value:#}");
                assert!(notices[0]["message"].as_str().unwrap().contains(&format!("`{host}` document")), "{value:#}");
            }
        }
    }
}

#[test]
fn a_catalog_without_its_companion_refuses_the_host_at_its_files_entry() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let fixture = registered("host-scope-companion", &acme());
    std::fs::remove_file(fixture.home().join(".acme").join(companion_of(CATALOG))).expect("the companion was written");
    fixture.write_file("cwd/widget.md", &acme_document("Body."));

    let (code, value) = compose(&fixture, "widget.md");

    assert_eq!(code, Some(0), "no host types the widget: {value:#}");
    assert_eq!(value["document"], "# Widget\n\nBody.", "{value:#}");
    assert!(records(&value, "dm.host.foreign_document").is_empty(), "no host applies: {value:#}");
    assert_eq!(value["lifecycle_output"], json!([]), "nothing ran: {value:#}");
    let refusals = records(&value, "dm.host.registration");
    assert_eq!(refusals.len(), 1, "{value:#}");
    assert_eq!(refusals[0]["severity"], "warning", "{value:#}");
    let source = &refusals[0]["source"];
    assert!(
        std::path::Path::new(source["file"].as_str().expect("a file")).ends_with(format!(".acme/{MANIFEST_NAME}")),
        "located at the manifest: {value:#}"
    );
    // `files[4]` (`- ./schemas/functions.yaml`) is line 10; its value starts
    // at column 7.
    assert_eq!((source["line"].as_u64(), source["column"].as_u64()), (Some(10), Some(7)), "{value:#}");
    let message = refusals[0]["message"].as_str().unwrap();
    assert!(message.contains("functions.facts.json"), "names the companion: {message}");
}
