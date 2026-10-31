//! `md schema validate` over YAML document arguments: a `.yaml`/`.yml` file is
//! validated against the shipped definition its top-level `kind` claims, never
//! read as Markdown. One `md` run per outcome; the kind validation matrix
//! itself is covered in-process by the library's `kind_documents` tests.

use crate::common;

use common::CliProcessFixture;
use serde_json::Value;
use std::path::PathBuf;

const VALID_SCHEMA: &str = "kind: schema\ndescription: A post.\n$schema:\n  title: string(required)\n";

/// The NDJSON records of one `md schema validate --format json` run, keyed by
/// file name, and the exit code.
fn validate_json(process: &CliProcessFixture, files: &[PathBuf]) -> (Option<i32>, Vec<(String, Value)>) {
    let output = process
        .command()
        .args(["schema", "validate", "--format", "json"])
        .args(files)
        .output()
        .expect("md runs");
    let records = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| {
            let record: Value = serde_json::from_str(line).unwrap_or_else(|_| panic!("NDJSON record: {line}"));
            let file = record["file"].as_str().expect("file").replace('\\', "/");
            (file.rsplit('/').next().expect("file name").to_string(), record)
        })
        .collect();
    (output.status.code(), records)
}

/// Pretty output of one run and its exit code, with ANSI styling removed.
fn validate_pretty(process: &CliProcessFixture, files: &[PathBuf]) -> (Option<i32>, String) {
    let output = process
        .command()
        .args(["schema", "validate"])
        .args(files)
        .output()
        .expect("md runs");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    (output.status.code(), strip_ansi(&stdout))
}

fn strip_ansi(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(character) = chars.next() {
        if character == '\u{1b}' {
            match chars.peek() {
                // CSI: up to the final byte.
                Some('[') => {
                    chars.next();
                    for next in chars.by_ref() {
                        if next.is_ascii_alphabetic() {
                            break;
                        }
                    }
                }
                // OSC 8 hyperlink: up to ST (`ESC \`) or BEL.
                Some(']') => {
                    while let Some(next) = chars.next() {
                        if next == '\u{7}' || (next == '\u{1b}' && chars.next_if_eq(&'\\').is_some()) {
                            break;
                        }
                    }
                }
                _ => {}
            }
        } else {
            out.push(character);
        }
    }
    out
}

/// Control: a valid `kind: schema` file and both real shipped definitions are
/// claimed by their kinds and valid, and validation leaves them unchanged.
#[test]
fn valid_kind_documents_and_shipped_definitions_are_claimed_and_valid() {
    let process = CliProcessFixture::new();
    let files = vec![
        process.write_file("post.yaml", VALID_SCHEMA),
        process.write_file("schema.yaml", include_str!("../../../schemas/schema.yaml")),
        process.write_file("schema-trigger.yaml", include_str!("../../../schemas/schema-trigger.yaml")),
    ];
    let before: Vec<Vec<u8>> = files.iter().map(|file| std::fs::read(file).expect("readable")).collect();

    let (code, stdout) = validate_pretty(&process, &files);

    assert_eq!(code, Some(0), "{stdout}");
    for name in ["post.yaml", "schema.yaml", "schema-trigger.yaml"] {
        assert!(
            stdout.lines().any(|line| line.contains(name) && line.contains("has a schema definition and is valid.")),
            "{name} must be claimed and valid:\n{stdout}"
        );
    }
    let after: Vec<Vec<u8>> = files.iter().map(|file| std::fs::read(file).expect("readable")).collect();
    assert_eq!(before, after, "validation is passive");
}

/// A structurally invalid `kind: schema` file fails with the offending value's
/// line and column in the rendered output.
#[test]
fn invalid_kind_schema_reports_the_located_problem() {
    let process = CliProcessFixture::new();
    let file = process.write_file("post.yaml", &VALID_SCHEMA.replace("description: A post.", "description: 42"));

    let (code, stdout) = validate_pretty(&process, &[file]);

    assert_eq!(code, Some(1), "{stdout}");
    assert!(stdout.contains("failed schema validation"), "{stdout}");
    assert!(stdout.contains("description"), "{stdout}");
    assert!(stdout.contains("(at line 2, column 14)"), "{stdout}");
}

/// A `kind: schema-trigger` file without `match` fails as a missing property;
/// the upper-case `.YML` extension still selects the YAML route (Markdown
/// reading would find no frontmatter and call it valid).
#[test]
fn trigger_missing_match_fails_as_missing() {
    let process = CliProcessFixture::new();
    let file = process.write_file("trigger.YML", "kind: schema-trigger\n$schema:\n  title: string\n");

    let (code, records) = validate_json(&process, &[file]);

    assert_eq!(code, Some(1), "{records:?}");
    let [(name, record)] = records.as_slice() else {
        panic!("one record expected: {records:?}");
    };
    assert_eq!(name, "trigger.YML");
    assert_eq!(record["valid"], Value::Bool(false));
    let problems = record["problems"].as_array().expect("problems");
    assert!(
        problems
            .iter()
            .any(|problem| problem["property"] == "match" && problem["kind"] == "missing" && problem["line"] == 1),
        "{record}"
    );
}

/// A YAML document no shipped kind claims (another `kind`, no `kind`, or a
/// non-mapping root) is valid by default.
#[test]
fn unclaimed_yaml_documents_are_valid_by_default() {
    let process = CliProcessFixture::new();
    let files = vec![
        process.write_file("config.yaml", "kind: config\nport: 8080\n"),
        process.write_file("plain.yml", "port: 8080\n"),
        process.write_file("list.yaml", "- a\n- b\n"),
    ];

    let (code, stdout) = validate_pretty(&process, &files);

    assert_eq!(code, Some(0), "{stdout}");
    for name in ["config.yaml", "plain.yml", "list.yaml"] {
        assert!(
            stdout
                .lines()
                .any(|line| line.contains(name) && line.contains("doesn't have a schema definition so is valid by default.")),
            "{name} must be unclaimed:\n{stdout}"
        );
    }
}

/// A YAML syntax error and a duplicate key are load failures: exit 3, a
/// `yaml_parse` record, and no problems.
#[test]
fn unloadable_yaml_exits_three() {
    let process = CliProcessFixture::new();
    let files = vec![
        process.write_file("syntax.yaml", "kind: schema\n$schema: [unclosed\n"),
        process.write_file("duplicate.yaml", "kind: schema\nkind: schema\n$schema:\n  title: string\n"),
    ];

    let (code, records) = validate_json(&process, &files);

    assert_eq!(code, Some(3), "{records:?}");
    assert_eq!(records.len(), 2, "{records:?}");
    for (name, record) in &records {
        assert_eq!(record["error"], "yaml_parse", "{name}: {record}");
        assert_eq!(record["valid"], Value::Bool(false), "{name}: {record}");
        assert_eq!(record["problems"], Value::Array(Vec::new()), "{name}: {record}");
        assert!(record["message"].as_str().is_some_and(|message| message.starts_with("not valid YAML: ")), "{record}");
    }
    let duplicate = &records.iter().find(|(name, _)| name == "duplicate.yaml").expect("duplicate record").1;
    assert!(duplicate["message"].as_str().is_some_and(|message| message.contains("duplicate")), "{duplicate}");
}

/// An assignment alongside a YAML argument is a usage error raised before any
/// file, including the Markdown one listed first, is validated.
#[test]
fn assignment_with_a_yaml_argument_is_a_usage_error() {
    let process = CliProcessFixture::new();
    let markdown = process.write_file("post.md", "---\ntitle: Hello\n---\nBody\n");
    let yaml = process.write_file("post.yaml", VALID_SCHEMA);

    let output = process
        .command()
        .args(["schema", "validate"])
        .arg(&markdown)
        .arg(&yaml)
        .arg("title=Other")
        .output()
        .expect("md runs");

    assert_eq!(output.status.code(), Some(64), "{output:?}");
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    assert!(
        stderr.contains("assignments apply to Markdown frontmatter, not to YAML document"),
        "{stderr}"
    );
    assert!(stderr.contains("post.yaml"), "{stderr}");
    assert!(output.stdout.is_empty(), "no file is validated: {}", String::from_utf8_lossy(&output.stdout));
}
