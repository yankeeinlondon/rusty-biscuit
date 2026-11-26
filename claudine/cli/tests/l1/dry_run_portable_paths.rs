//! Path spelling in the dry run's two outputs (`--dry-run` and
//! `--dry-run --json`): every path the human report shows under the launch
//! directory is relative and written with `/`, the JSON envelope's path
//! values use `biscuit_file::to_forward_slash_string`, an `@` reference's
//! searched locations keep the resolver's tier order, and a CRLF copy of the
//! sequence reports the same lines and columns as the LF original.
//!
//! The fixture is `tests/fixtures/check/fleet-run2/` with its missing prompt
//! moved one directory down (`./parts/_frontmatter.md`), a near-miss file
//! beside it, and a third task whose prompt is an `@` reference. The
//! Windows-only spellings (drive letters, authored `\`, UNC bases) are
//! covered in-process by Darkmatter's `findings_portable_paths` module, which
//! owns the rendering these outputs delegate to.

use std::fs;
use std::path::Path;

use serde_json::Value;

use crate::common::{CliProcessFixture, strip_ansi};

const FLEET: &str = "renderers/_fleet.yaml";

/// The two tasks of `fleet-run2`, the second's prompt nested one directory
/// down, and a third task naming an `@` prompt that exists nowhere.
fn nested_fleet() -> String {
    let original = fs::read_to_string(biscuit_test_harness::manifest_dir!().join("tests/fixtures/check/fleet-run2/renderers/_fleet.yaml")).unwrap();
    // A Windows checkout may carry CRLF; the CRLF case is built below.
    let mut text = original.replace("\r\n", "\n").replace("prompt: ./_frontmatter.md", "prompt: ./parts/_frontmatter.md");
    text.push_str("    - name: magic\n      description: a prompt found by search\n      prompt: \"@portable-magic.md\"\n");
    text
}

fn fixture(test: &str, fleet: &str) -> CliProcessFixture {
    let fixture = CliProcessFixture::named(test);
    let renderers = fixture.cwd().join("renderers");
    fs::create_dir_all(renderers.join("parts")).unwrap();
    fs::copy(
        biscuit_test_harness::manifest_dir!().join("tests/fixtures/check/fleet-run2/renderers/_find.md"),
        renderers.join("_find.md"),
    )
    .unwrap();
    fs::write(renderers.join("parts/_frontmater.md"), "near miss\n").unwrap();
    fs::write(fixture.cwd().join(FLEET), fleet).unwrap();
    fixture
}

fn run(fixture: &CliProcessFixture, json: bool) -> (Option<i32>, String, String) {
    let mut command = fixture.command();
    command.env("NO_COLOR", "1").args(["sequence", FLEET, "--dry-run"]);
    if json {
        command.arg("--json");
    }
    let output = command.output().unwrap();
    (
        output.status.code(),
        strip_ansi(&String::from_utf8_lossy(&output.stdout)),
        strip_ansi(&String::from_utf8_lossy(&output.stderr)),
    )
}

/// `text` with each run of whitespace as one space, so a wrapped row reads as
/// one line.
fn prose(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn missing_documents(envelope: &Value) -> Vec<&Value> {
    envelope["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|diagnostic| diagnostic["code"] == "dm.reference.missing_document")
        .collect()
}

/// Every string in `value` that holds a `\`, by JSON pointer.
fn backslashed(value: &Value, pointer: String, found: &mut Vec<String>) {
    match value {
        Value::String(text) if text.contains('\\') => found.push(format!("{pointer} = {text}")),
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                backslashed(item, format!("{pointer}/{index}"), found);
            }
        }
        Value::Object(map) => {
            for (key, item) in map {
                backslashed(item, format!("{pointer}/{key}"), found);
            }
        }
        _ => {}
    }
}

/// `(code, line, column)` of every located diagnostic, in report order.
fn positions(envelope: &Value) -> Vec<(String, u64, u64)> {
    envelope["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|diagnostic| diagnostic["source"]["line"].is_u64())
        .map(|diagnostic| {
            (
                diagnostic["code"].as_str().unwrap().to_string(),
                diagnostic["source"]["line"].as_u64().unwrap(),
                diagnostic["source"]["column"].as_u64().unwrap(),
            )
        })
        .collect()
}

/// The `/` spellings of `root`: as written and canonical.
fn spellings(root: &Path) -> Vec<String> {
    let mut spellings = vec![format!("{}/", biscuit_file::to_forward_slash_string(root))];
    if let Ok(canonical) = biscuit_file::canonicalize_simplified(root) {
        spellings.push(format!("{}/", biscuit_file::to_forward_slash_string(&canonical)));
    }
    spellings
}

fn same_directory(reported: &str, expected: &Path) -> bool {
    let parent = Path::new(reported).parent().expect("a reported file has a parent");
    biscuit_file::canonicalize_simplified(parent).ok() == biscuit_file::canonicalize_simplified(expected).ok()
}

#[test]
fn the_human_report_shows_nested_relative_paths_with_forward_slashes() {
    let fixture = fixture("dry-run-portable-human", &nested_fleet());
    let (status, _, stderr) = run(&fixture, false);
    assert_eq!(status, Some(2), "stderr:\n{stderr}");
    let report = prose(&stderr);
    for expected in [
        "Dry run of renderers/_fleet.yaml",
        "resolved to renderers/parts/_frontmatter.md (relative to renderers/_fleet.yaml)",
        "at renderers/_fleet.yaml:12:15",
        "hint: create renderers/parts/_frontmatter.md before a real run",
    ] {
        assert!(report.contains(expected), "missing `{expected}` in:\n{stderr}");
    }
    // A suggestion is written absolute, so a long temporary path may wrap
    // inside it.
    let unwrapped: String = stderr.split_whitespace().collect();
    assert!(unwrapped.contains("didyoumean:") && unwrapped.contains("/cwd/renderers/parts/_frontmater.md"), "{stderr}");
    assert!(!stderr.contains('\\'), "a human path is never written with `\\`:\n{stderr}");
}

#[test]
fn json_paths_are_portable_magic_roots_keep_their_order_and_crlf_keeps_positions() {
    let lf = nested_fleet();
    let fixture = fixture("dry-run-portable-json", &lf);
    let (status, stdout, stderr) = run(&fixture, true);
    assert_eq!(status, Some(2), "stderr:\n{stderr}");
    let envelope: Value = serde_json::from_str(&stdout).unwrap();

    let mut found = Vec::new();
    backslashed(&envelope, String::new(), &mut found);
    assert!(found.is_empty(), "JSON paths are written with `/`: {found:#?}\n{envelope:#}");
    let file = envelope["file"].as_str().unwrap();
    assert!(file.ends_with("/renderers/_fleet.yaml") && same_directory(file, &fixture.cwd().join("renderers")), "{file}");

    let missing = missing_documents(&envelope);
    assert_eq!(missing.len(), 2, "{envelope:#}");
    let nested = missing[0];
    let resolved = nested["resolved"].as_str().unwrap();
    assert!(resolved.ends_with("/renderers/parts/_frontmatter.md"), "{nested:#}");
    assert!(same_directory(resolved, &fixture.cwd().join("renderers/parts")), "{resolved}");
    let suggestions = nested["suggestions"].as_array().expect("the near miss is suggested");
    assert!(suggestions[0].as_str().unwrap().ends_with("/renderers/parts/_frontmater.md"), "{nested:#}");

    // `@` searches every local root before any home root.
    let magic = missing[1];
    assert_eq!(magic["reference"], "@portable-magic.md", "{magic:#}");
    let searched: Vec<&str> = magic["searched"].as_array().expect("an `@` miss lists its search").iter().map(|path| path.as_str().unwrap()).collect();
    assert!(searched.iter().all(|path| path.ends_with("/portable-magic.md")), "{searched:#?}");
    let under = |root: &Path, path: &str| spellings(root).iter().any(|prefix| path.starts_with(prefix.as_str()));
    let tiers: Vec<&str> = searched
        .iter()
        .map(|path| match (under(fixture.cwd(), path), under(fixture.home(), path)) {
            (true, false) => "local",
            (false, true) => "home",
            _ => panic!("{path} is under neither the launch directory nor home"),
        })
        .collect();
    let first_home = tiers.iter().position(|tier| *tier == "home").expect("home is searched");
    assert!(first_home > 0, "the launch directory is searched first: {searched:#?}");
    assert!(tiers[first_home..].iter().all(|tier| *tier == "home"), "a local root after a home root: {searched:#?}");

    // The same sequence with CRLF line endings reports the same positions.
    fs::write(fixture.cwd().join(FLEET), lf.replace('\n', "\r\n")).unwrap();
    let (crlf_status, crlf_stdout, crlf_stderr) = run(&fixture, true);
    assert_eq!(crlf_status, Some(2), "stderr:\n{crlf_stderr}");
    let crlf: Value = serde_json::from_str(&crlf_stdout).unwrap();
    let located = positions(&envelope);
    assert!(located.contains(&("dm.reference.missing_document".to_string(), 12, 15)), "{located:?}");
    assert_eq!(positions(&crlf), located, "CRLF:\n{crlf:#}");
}
