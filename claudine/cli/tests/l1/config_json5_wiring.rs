//! The legacy `~/.claudine/config.json` (JSON5) migration entry point, through
//! the `claudine` binary: a file the conversion refuses stops the command
//! with `config.json` named on stderr, before any provider is launched, and
//! publishes nothing (no YAML, no backup; the source keeps its bytes). The
//! valid control is `config_yaml_wiring::a_legacy_json_file_is_converted_once_with_a_notice`;
//! the in-process matrix (`claudine::l1 config_json_conversion`) owns every
//! other cell.
//!
//! The provider is the compiled `claudine-fake-goose`, which appends a line
//! to `PROVIDER_MARKER` when launched, so these cells run on every OS.

use std::fs;
use std::path::PathBuf;

use crate::common;
use common::{CliProcessFixture, write};

const SOURCE: &str = "{\n  // mine\n  protect: false,\n  logging: \"yes\",\n}\n";

struct Refused {
    stderr: String,
    dir: PathBuf,
    marker: PathBuf,
    _fixture: CliProcessFixture,
}

/// Runs `claudine compose doc.md --goose` with `json5` as the user's legacy
/// file and no `config.yaml`; the command must fail.
fn refused(name: &str, json5: &str) -> Refused {
    let fixture = CliProcessFixture::named(name);
    let dir = fixture.home().join(".claudine");
    write(&dir.join("config.json"), json5);
    write(&fixture.cwd().join("doc.md"), "Say hello.\n");
    fs::copy(
        biscuit_test_harness::bin_exe!("claudine-fake-goose"),
        fixture.bin_dir().join(format!("goose{}", std::env::consts::EXE_SUFFIX)),
    )
    .expect("the fake provider is built with `test-fixtures`");
    let marker = fixture.cwd().join("provider-launched.txt");
    let output = fixture
        .command()
        .env("PROVIDER_MARKER", &marker)
        .args(["compose", "doc.md", "--goose", "--silent"])
        .output()
        .expect("claudine runs");
    let stderr = common::strip_ansi(&String::from_utf8_lossy(&output.stderr));
    assert!(!output.status.success(), "{stderr}");
    Refused { stderr, dir, marker, _fixture: fixture }
}

fn assert_nothing_published(run: &Refused, json5: &str) {
    assert!(!run.marker.exists(), "no provider was launched: {}", run.stderr);
    assert_eq!(fs::read_to_string(run.dir.join("config.json")).unwrap(), json5, "the source keeps its bytes");
    assert!(!run.dir.join("config.yaml").exists(), "no YAML was published: {}", run.stderr);
    assert!(!run.dir.join("config.json.bak").exists(), "no backup was written");
}

#[test]
fn a_malformed_legacy_file_stops_before_any_launch() {
    let json5 = "{ protect: false, } trailing\n";
    let run = refused("config-json5-malformed", json5);
    assert!(run.stderr.contains("config.json") && run.stderr.contains("JSON5"), "{}", run.stderr);
    assert_nothing_published(&run, json5);
}

#[test]
fn a_field_the_schema_refuses_stops_before_any_launch() {
    let run = refused("config-json5-field", SOURCE);
    assert!(run.stderr.contains("config.json") && run.stderr.contains("/logging"), "{}", run.stderr);
    assert_nothing_published(&run, SOURCE);
}
