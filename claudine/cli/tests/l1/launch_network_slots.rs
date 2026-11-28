//! A real run resolves a `$schema` slot that reads a URL under the captured
//! `permissions.network`, as the dry run judges it (F3, L1): `allow` fetches
//! and launches, `deny` is the typed `dm.schema.slot_network_denied` refusal
//! with no request and no launch, and `ask` with no terminal to ask on is
//! `dm.schema.slot_approval_required`, again with no request and no launch.
//! A refusal is `composition.schema_slot_failed` carrying the slot's own
//! code, and every refusal Claudine surfaces names a remedy Claudine
//! supports, never `md`'s `--allow host=HOST`. The pre-launch check and the
//! composition share one slot session, so `ask` asks once per run.
//!
//! The provider is the compiled `claudine-fake-goose`, which appends one line
//! to `PROVIDER_MARKER` per launch; the URL is a loopback responder counting
//! requests (shared with `dry_run_network_approval`).

use std::fs;
use std::path::PathBuf;

use crate::common::{CliProcessFixture, strip_ansi};
use crate::dry_run_network_approval::Responder;

const MD_REMEDY: &str = "--allow host=";

struct Run {
    code: Option<i32>,
    stderr: String,
    launches: usize,
    /// The diagnostic the run rendered, when it failed.
    diagnostic: Option<serde_json::Value>,
}

fn fixture(setting: &str) -> (CliProcessFixture, PathBuf) {
    let fixture = CliProcessFixture::named("launch-network-slots");
    fs::write(fixture.home().join(".darkmatter.yaml"), format!("permissions:\n  network: {setting}\n")).unwrap();
    fs::copy(
        biscuit_test_harness::bin_exe!("claudine-fake-goose"),
        fixture.bin_dir().join(format!("goose{}", std::env::consts::EXE_SUFFIX)),
    )
    .expect("the fake provider is built with `test-fixtures`");
    let marker = fixture.cwd().join("provider-launched.txt");
    (fixture, marker)
}

/// `doc.md` whose `$schema` slot reads `/os.yaml` from `server`.
fn write_slot_document(fixture: &CliProcessFixture, server: &Responder, frontmatter: &str) {
    fs::write(
        fixture.cwd().join("schema.yaml"),
        format!("kind: schema\n$schema:\n    os: \"enum({{{{ load_data(\\\"{}\\\", \\\"string[]\\\") }}}})\"\n", server.url("/os.yaml")),
    )
    .unwrap();
    fs::write(fixture.cwd().join("doc.md"), format!("---\n$schema: ./schema.yaml\nos: macOS\n{frontmatter}---\nBody.\n")).unwrap();
}

fn run(fixture: &CliProcessFixture, marker: &PathBuf, args: &[&str]) -> Run {
    let snapshot = fixture.cwd().join("diagnostic.json");
    let output = fixture
        .command()
        .env("PROVIDER_MARKER", marker)
        .env("CLAUDINE_TEST_DIAGNOSTIC_SNAPSHOT", &snapshot)
        .args(args)
        .output()
        .unwrap();
    assert!(!fixture.audio_spool().exists());
    Run {
        code: output.status.code(),
        stderr: strip_ansi(&String::from_utf8_lossy(&output.stderr)),
        launches: fs::read_to_string(marker).map_or(0, |log| log.lines().count()),
        diagnostic: fs::read_to_string(&snapshot).ok().map(|text| serde_json::from_str(&text).unwrap()),
    }
}

fn compose_slot(setting: &str) -> (Responder, Run) {
    let server = Responder::start();
    let (fixture, marker) = fixture(setting);
    write_slot_document(&fixture, &server, "");
    let run = run(&fixture, &marker, &["compose", "--goose", "doc.md"]);
    (server, run)
}

fn assert_launched(server: &Responder, run: &Run) {
    assert_eq!(run.code, Some(0), "{}", run.stderr);
    assert_eq!(run.launches, 1, "{}", run.stderr);
    // The pre-launch check resolves the template once; the composition, whose
    // source-derived resolution context keys its own pin, may read it again.
    assert!(server.requests() >= 1, "{}", run.stderr);
    assert!(!run.stderr.contains("unresolved slot"), "{}", run.stderr);
}

fn assert_refused(server: &Responder, run: &Run, code: &str) {
    assert_eq!(run.code, Some(1), "{}", run.stderr);
    assert_eq!(run.launches, 0, "{}", run.stderr);
    assert_eq!(server.requests(), 0, "{}", run.stderr);
    assert!(run.stderr.contains(code), "names `{code}`: {}", run.stderr);
    let diagnostic = run.diagnostic.as_ref().expect("a rendered diagnostic");
    assert_eq!(diagnostic["code"], "composition.schema_slot_failed", "{diagnostic:#}");
    assert_eq!(diagnostic["detail"]["slot_code"], code, "{diagnostic:#}");
    assert!(!run.stderr.contains("unresolved slot"), "{}", run.stderr);
    assert!(!run.stderr.contains(MD_REMEDY), "a Claudine remedy, not `md`'s: {}", run.stderr);
}

#[test]
fn compose_slot_allow_fetches_and_launches() {
    let (server, run) = compose_slot("allow");
    assert_launched(&server, &run);
}

#[test]
fn compose_slot_deny_is_the_typed_refusal() {
    let (server, run) = compose_slot("deny");
    assert_refused(&server, &run, "dm.schema.slot_network_denied");
}

#[test]
fn compose_slot_ask_without_a_terminal_needs_approval() {
    let (server, run) = compose_slot("ask");
    assert_refused(&server, &run, "dm.schema.slot_approval_required");
}

/// A positional setter is owned before the template resolves: a slot schema
/// leaves its names unestablished, as a templated `$schema` reference does.
#[test]
fn compose_slot_allow_with_a_setter_launches() {
    let server = Responder::start();
    let (fixture, marker) = fixture("allow");
    write_slot_document(&fixture, &server, "");
    let run = run(&fixture, &marker, &["compose", "--goose", "doc.md", "os=Linux"]);
    assert_launched(&server, &run);
}

#[test]
fn inline_compose_slot_allow_fetches_and_launches() {
    let server = Responder::start();
    let (fixture, marker) = fixture("allow");
    write_slot_document(&fixture, &server, "prompt: Say hi.\n");
    let run = run(&fixture, &marker, &["inline-compose", "--goose", "doc.md"]);
    // The stub writes nothing back, so the run then fails its completion
    // check; the launch is what this cell is about.
    assert_eq!(run.launches, 1, "{}", run.stderr);
    assert!(server.requests() >= 1, "{}", run.stderr);
    assert!(!run.stderr.contains("unresolved"), "{}", run.stderr);
    assert!(!run.stderr.contains("schema slot failed"), "{}", run.stderr);
}

#[test]
fn sequence_prompt_slot_allow_fetches_and_launches() {
    let server = Responder::start();
    let (fixture, marker) = fixture("allow");
    write_slot_document(&fixture, &server, "agent: goose\n");
    fs::write(fixture.cwd().join("seq.md"), "---\nsequence:\n  - name: ask\n    prompt: ./doc.md\n---\nBody.\n").unwrap();
    let run = run(&fixture, &marker, &["sequence", "seq.md"]);
    assert_launched(&server, &run);
}

/// A remote `::file` under `ask`: the real run refuses it, and its hint
/// names Claudine's remedy.
#[test]
fn transclusion_needing_approval_names_a_claudine_remedy() {
    let server = Responder::start();
    let (fixture, marker) = fixture("ask");
    fs::write(fixture.cwd().join("doc.md"), format!("---\nagent: goose\n---\nBefore.\n\n::file {}\n", server.url("/child.md"))).unwrap();
    let run = run(&fixture, &marker, &["compose", "doc.md"]);
    assert_eq!(run.code, Some(1), "{}", run.stderr);
    assert_eq!(run.launches, 0, "{}", run.stderr);
    assert_eq!(server.requests(), 0, "{}", run.stderr);
    assert!(run.stderr.contains("permissions.network"), "{}", run.stderr);
    assert!(!run.stderr.contains(MD_REMEDY), "a Claudine remedy, not `md`'s: {}", run.stderr);
}
