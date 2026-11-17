//! Loader wiring through `md compose`, one spawn per cell: for Markdown
//! frontmatter, a raw JSON Schema named by `$schema`, and the user
//! configuration, a malformed input and a field-invalid input stop the run
//! with a located error before any lifecycle effect runs, leave the source as
//! written, and publish no document. A valid control proves the same marker
//! effect does run. The loaders' input matrices are in-process
//! (`frontmatter_extraction.rs`, the schema robustness matrix,
//! `config_load.rs`); `md schema validate` of a whole YAML document is
//! `schema_validate_yaml.rs`.

use crate::common;

use common::CliProcessFixture;
use std::path::{Path, PathBuf};

/// A `composed` step whose effect is a file beside the document. Not
/// `initialize`: that event runs before the body is read and before schema
/// validation, and keeps its effects (`docs/topics/lifecycle/index.md`).
const MARKER_STEP: &str = "composed:\n  - ensure_file: [\"marker.md\", \"x\"]\n";
const BODY: &str = "published body\n";

struct Ran {
    status: Option<i32>,
    stdout: String,
    stderr: String,
}

fn compose(fixture: &CliProcessFixture, document: &Path) -> Ran {
    let output = fixture.command().arg("compose").arg(document).output().expect("md runs");
    Ran {
        status: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: common::unwrapped(&String::from_utf8_lossy(&output.stderr)),
    }
}

/// `doc.md` with `frontmatter` and the marker step, and the marker's path.
fn document(fixture: &CliProcessFixture, frontmatter: &str) -> (PathBuf, String, PathBuf) {
    let text = format!("---\n{frontmatter}{MARKER_STEP}---\n{BODY}");
    let doc = fixture.write_file("cwd/doc.md", &text);
    (doc, text, fixture.cwd().join("marker.md"))
}

/// The run stopped with status 1, ran no effect, published nothing, and left
/// the source as written; stderr holds every `needles` entry.
fn assert_stopped(ran: &Ran, doc: &Path, text: &str, marker: &Path, needles: &[&str]) {
    assert_eq!(ran.status, Some(1), "{}", ran.stderr);
    for needle in needles {
        assert!(ran.stderr.contains(needle), "`{needle}` on stderr: {}", ran.stderr);
    }
    assert!(!marker.exists(), "no lifecycle effect ran: {}", ran.stderr);
    assert!(!ran.stdout.contains("published body"), "no document is published: {}", ran.stdout);
    assert_eq!(std::fs::read_to_string(doc).expect("read source"), text, "the source is unchanged");
}

// ── Markdown frontmatter ───────────────────────────────────────────────────

#[test]
fn malformed_frontmatter_stops_before_any_effect_located_in_its_file() {
    let fixture = CliProcessFixture::named("loader-frontmatter-malformed");
    let (doc, text, marker) = document(&fixture, "title: T\nitems: [a, b\n");
    let ran = compose(&fixture, &doc);
    assert_stopped(&ran, &doc, &text, &marker, &["doc.md", "line"]);
}

#[test]
fn a_frontmatter_field_its_schema_refuses_stops_before_any_effect() {
    let fixture = CliProcessFixture::named("loader-frontmatter-field");
    let (doc, text, marker) = document(&fixture, "$schema:\n  count: number(required)\ncount: many\n");
    let ran = compose(&fixture, &doc);
    assert_stopped(&ran, &doc, &text, &marker, &["doc.md", "count", "at 4:8"]);
}

// ── Raw JSON Schema ────────────────────────────────────────────────────────

const RAW_SCHEMA: &str = r#"{"$schema":"https://json-schema.org/draft/2020-12/schema","type":"object","properties":{"count":{"type":"number"}}}"#;

#[test]
fn a_raw_json_schema_the_document_satisfies_runs_its_effects_and_publishes() {
    let fixture = CliProcessFixture::named("loader-json-control");
    fixture.write_file("cwd/schema.json", RAW_SCHEMA);
    let (doc, text, marker) = document(&fixture, "$schema: ./schema.json\ncount: 3\n");
    let ran = compose(&fixture, &doc);
    assert_eq!(ran.status, Some(0), "{}", ran.stderr);
    assert!(marker.exists(), "the marker effect ran: {}", ran.stderr);
    assert!(ran.stdout.contains("published body"), "{}", ran.stdout);
    assert_eq!(std::fs::read_to_string(&doc).expect("read source"), text, "the source is unchanged");
}

#[test]
fn a_raw_json_schema_with_a_duplicate_member_stops_before_any_effect() {
    let fixture = CliProcessFixture::named("loader-json-malformed");
    fixture.write_file(
        "cwd/schema.json",
        r#"{"$schema":"https://json-schema.org/draft/2020-12/schema","type":"object","type":"array"}"#,
    );
    let (doc, text, marker) = document(&fixture, "$schema: ./schema.json\ncount: 3\n");
    let ran = compose(&fixture, &doc);
    assert_stopped(&ran, &doc, &text, &marker, &["schema.json", "duplicate key \"type\""]);
}

#[test]
fn a_field_a_raw_json_schema_refuses_stops_before_any_effect() {
    let fixture = CliProcessFixture::named("loader-json-field");
    fixture.write_file("cwd/schema.json", RAW_SCHEMA);
    let (doc, text, marker) = document(&fixture, "$schema: ./schema.json\ncount: [1]\n");
    let ran = compose(&fixture, &doc);
    assert_stopped(&ran, &doc, &text, &marker, &["doc.md", "count", "at 3:8"]);
}

// ── User configuration ─────────────────────────────────────────────────────

/// A configuration that is not YAML stops a composing run at its file and
/// line. A configuration field the closed schema refuses is
/// `config_notices.rs`'s `a_user_file_that_does_not_load_stops_a_command_that_reads_it`.
#[test]
fn a_malformed_user_configuration_stops_before_any_effect_located_in_its_file() {
    let fixture = CliProcessFixture::named("loader-config-malformed");
    let config = fixture.home().join(".darkmatter.yaml");
    std::fs::write(&config, "register: [a\n").expect("write configuration");
    let (doc, text, marker) = document(&fixture, "title: T\n");
    let ran = compose(&fixture, &doc);
    let located = format!("{}:", config.display());
    assert_stopped(&ran, &doc, &text, &marker, &[located.as_str()]);
    assert_eq!(std::fs::read_to_string(&config).expect("read configuration"), "register: [a\n");
}
