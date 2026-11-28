//! Path spelling in `md compose`'s report of a missing `::file`, through the
//! binary: the human report names the nested document relative to the launch
//! directory with `/`, and the JSON envelope's paths are written with `/`.
//! The JSON run reads a CRLF copy of the document and reports the same line
//! and column as the human run's LF original. The rendering itself, and the
//! Windows-only spellings, are covered in-process by the library's
//! `findings_portable_paths` module.

use crate::common;

use common::CliProcessFixture;
use serde_json::Value;

const GUIDE: &str = "---\ntitle: Guide\n---\n\n# Guide\n\nCafé intro.\n\n::file ./parts/missing.md\n";

fn compose(fixture: &CliProcessFixture, args: &[&str]) -> (Option<i32>, String, String) {
    let output = fixture
        .command_builder()
        .plain_terminal(80, 40)
        .build()
        .arg("compose")
        .args(args)
        .arg("docs/guide.md")
        .output()
        .unwrap();
    (
        output.status.code(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
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

#[test]
fn a_missing_nested_transclusion_is_reported_with_slash_paths_and_crlf_keeps_its_position() {
    let fixture = CliProcessFixture::named("compose-portable-paths");
    fixture.write_file("cwd/docs/guide.md", GUIDE);
    let (status, stdout, stderr) = compose(&fixture, &[]);
    assert_eq!(status, Some(1), "{stderr}");
    assert_eq!(stdout, "");
    let report = stderr.replace("\r\n", "\n");
    // The document heading is the launch-relative path; its rows name the
    // file under it.
    assert!(report.contains("\ndocs/guide.md\n"), "{report}");
    assert!(report.contains("`./parts/missing.md`  guide.md:9:1"), "{report}");
    assert!(!report.contains('\\'), "a human path is never written with `\\`:\n{report}");

    fixture.write_file("cwd/docs/guide.md", &GUIDE.replace('\n', "\r\n"));
    let (status, stdout, stderr) = compose(&fixture, &["--output", "json"]);
    assert_eq!(status, Some(1), "{stdout}\n{stderr}");
    let envelope: Value = serde_json::from_str(&stdout).unwrap();
    let mut found = Vec::new();
    backslashed(&envelope, String::new(), &mut found);
    assert!(found.is_empty(), "JSON paths are written with `/`: {found:#?}\n{envelope:#}");
    let file = envelope["file"].as_str().unwrap();
    // `file` echoes the document argument as given.
    assert_eq!(file, "docs/guide.md", "{envelope:#}");
    let missing = &envelope["diagnostics"][0];
    assert_eq!(missing["code"], "dm.reference.missing_transclusion", "{envelope:#}");
    assert_eq!(missing["reference"], "./parts/missing.md");
    assert!(missing["resolved"].as_str().is_some_and(|path| path.ends_with("/cwd/docs/parts/missing.md")), "{envelope:#}");
    assert_eq!((&missing["source"]["line"], &missing["source"]["column"]), (&Value::from(9), &Value::from(1)), "{envelope:#}");
}
