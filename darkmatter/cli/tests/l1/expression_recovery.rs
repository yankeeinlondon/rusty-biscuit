//! `md compose` with `try(expr, fallback)`: recovered causes leave no
//! diagnostic, invalid requests stay fatal, and an unrelated warning still
//! obeys `--deny`.

use crate::common;

use common::CliProcessFixture;
use serde_json::{Value, json};
use simplified_schema::SimplifiedSchema;
use std::path::Path;
use std::process::Output;
use test_toolkit::argv_recorder::ARGV_RECORDER_FILE;

/// The spec's example, with the reference check on (the default).
const EXAMPLE: &str = "{{ as_json(try(load_data(\"optional.yaml\", \"string[]\"), [])) }}\n";

/// A later shell command whose only effect is creating `marker` in the launch
/// directory. The native argv recorder stands in for `touch`, which Windows
/// lacks on the fixture's minimal `PATH`.
const SENTINEL: &str = "\n::shell argv-recorder marker\n";

fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// Installs the recorder on the fixture `PATH` and approves the sentinel.
fn with_sentinel(fixture: &CliProcessFixture) {
    std::fs::copy(
        biscuit_test_harness::bin_exe!("darkmatter-argv-recorder"),
        fixture.bin_dir().join(format!("argv-recorder{}", std::env::consts::EXE_SUFFIX)),
    )
    .expect("the recorder fixture binary is built with `test-fixtures`");
    fixture.write_file("cwd/.darkmatter-shell-whitelist", "exact argv-recorder marker\n");
}

fn compose(fixture: &CliProcessFixture, document: &Path, extra: &[&str]) -> Output {
    fixture
        .command()
        .env(ARGV_RECORDER_FILE, fixture.cwd().join("marker"))
        .arg("compose")
        .args(extra)
        .arg(document)
        .output()
        .unwrap()
}

/// The envelope on stdout: exactly one JSON object and nothing else.
fn envelope(output: &Output) -> Value {
    let text = stdout_of(output);
    let mut stream = serde_json::Deserializer::from_str(&text).into_iter::<Value>();
    let report = stream
        .next()
        .unwrap_or_else(|| panic!("no envelope; stderr: {}", stderr_of(output)))
        .unwrap_or_else(|error| panic!("stdout is not JSON ({error}): {text}"));
    assert!(stream.next().is_none(), "one object only: {text}");
    report
}

fn codes(report: &Value) -> Vec<&str> {
    report["diagnostics"].as_array().unwrap().iter().map(|diagnostic| diagnostic["code"].as_str().unwrap()).collect()
}

fn errors(report: &Value) -> Vec<&Value> {
    report["diagnostics"].as_array().unwrap().iter().filter(|diagnostic| diagnostic["severity"] == "error").collect()
}

fn read_names(report: &Value, name: &str) -> bool {
    report["read_set"].as_array().unwrap().iter().any(|entry| entry.as_str().is_some_and(|path| path.ends_with(name)))
}

// ── compose_recovers_optional_data ──────────────────────────────────────────

/// One `#[test]` per state of `optional.yaml` (`None` = absent). An
/// "unreadable" source needs a read that resolves and then fails with an I/O
/// cause; a spawned binary has no portable way to make one (permission bits
/// are not portable, and a directory resolves as a clean miss), so that case
/// is covered in-process by `expression_recovery_data::unreadable_source_recovers`.
macro_rules! recovers_optional_data {
    ($($case:ident: $data:expr => $stdout:expr;)*) => {
        mod compose_recovers_optional_data {
            use super::*;
            $(
                #[test]
                fn $case() {
                    let fixture = CliProcessFixture::named(concat!("er_recovers_", stringify!($case)));
                    if let Some(data) = $data {
                        fixture.write_file("cwd/optional.yaml", data);
                    }
                    let document = fixture.write_file("cwd/doc.md", EXAMPLE);

                    let output = compose(&fixture, &document, &[]);

                    assert_eq!(output.status.code(), Some(0), "{}", stderr_of(&output));
                    assert_eq!(stdout_of(&output), $stdout);
                    assert!(output.stderr.is_empty(), "{}", stderr_of(&output));
                }
            )*
        }
    };
}

// The composed document keeps the source's final newline and `md` ends its
// output with one more.
recovers_optional_data! {
    valid: Some("- a\n- b\n") => "[\"a\",\"b\"]\n\n";
    missing: None::<&str> => "[]\n\n";
    malformed: Some("a: [\n") => "[]\n\n";
    mismatched: Some("a: 1\n") => "[]\n\n";
}

// ── compose_keeps_invalid_requests_fatal ────────────────────────────────────

/// The fallback reads `fallback.yaml`, so its absence from the read set shows
/// the fallback never ran; `control` is the same shape recovering a miss, which
/// runs both the fallback and the sentinel.
macro_rules! keeps_invalid_requests_fatal {
    ($($case:ident: files $files:expr, call $call:expr, code $code:expr, detail $detail:expr;)*) => {
        mod compose_keeps_invalid_requests_fatal {
            use super::*;

            #[test]
            fn control() {
                let fixture = fatal_fixture("er_fatal_control", &[]);
                let document = fatal_document(&fixture, "", "load_data(\"optional.yaml\", \"string[]\")");

                let output = compose(&fixture, &document, &["--output", "json"]);

                assert_eq!(output.status.code(), Some(0), "{}", stdout_of(&output));
                let report = envelope(&output);
                assert_eq!(report["outcome"], "success", "{report:#}");
                assert!(report["document"].as_str().unwrap().starts_with("[\"fallback\"]"), "{report:#}");
                assert!(read_names(&report, "fallback.yaml"), "{report:#}");
                assert!(fixture.cwd().join("marker").exists(), "the control runs its sentinel");
            }

            $(
                #[test]
                fn $case() {
                    let fixture = fatal_fixture(concat!("er_fatal_", stringify!($case)), $files);
                    let (frontmatter, call): (&str, &str) = $call;
                    let document = fatal_document(&fixture, frontmatter, call);

                    let output = compose(&fixture, &document, &["--output", "json"]);

                    assert_eq!(output.status.code(), Some(1), "{}", stdout_of(&output));
                    let report = envelope(&output);
                    assert_eq!(report["outcome"], "failure", "{report:#}");
                    assert_eq!(report["document"], Value::Null, "{report:#}");
                    let errors = errors(&report);
                    assert_eq!(errors.len(), 1, "{report:#}");
                    assert_eq!(errors[0]["code"], $code, "{report:#}");
                    let detail: fn(&Value) -> bool = $detail;
                    assert!(detail(errors[0]), "{report:#}");
                    assert!(!read_names(&report, "fallback.yaml"), "the fallback ran: {report:#}");
                    assert!(!fixture.cwd().join("marker").exists(), "the later shell ran");
                }
            )*
        }
    };
}

fn fatal_fixture(name: &str, files: &[(&str, &str)]) -> CliProcessFixture {
    let fixture = CliProcessFixture::named(name);
    with_sentinel(&fixture);
    fixture.write_file("cwd/fallback.yaml", "- fallback\n");
    for (file, content) in files {
        fixture.write_file(format!("cwd/{file}"), content);
    }
    fixture
}

fn fatal_document(fixture: &CliProcessFixture, frontmatter: &str, call: &str) -> std::path::PathBuf {
    let content = format!("{frontmatter}{{{{ as_json(try({call}, load_data(\"fallback.yaml\", \"string[]\"))) }}}}\n{SENTINEL}");
    fixture.write_file("cwd/doc.md", &content)
}

keeps_invalid_requests_fatal! {
    unsupported_extension:
        files &[("optional.xyz", "- a\n")],
        call ("", "load_data(\"optional.xyz\", \"string[]\")"),
        code "dm.data.unsupported_format",
        detail |error| error["message"].as_str().unwrap().contains("no supported data format");
    // A frontmatter value the passive check cannot judge, so the options are
    // refused only at execution.
    computed_invalid_options:
        files &[("rows.csv", "a,b\n1,2\n")],
        call (
            "---\n$schema:\n  delim: string(required)\ndelim: \";;\"\n---\n",
            "load_data(\"rows.csv\", \"{a: string, b: string}[]\", {delimiter: delim})",
        ),
        code "dm.data.invalid_options",
        detail |error| error["message"].as_str().unwrap().contains("option `delimiter` is invalid");
    // The data file is absent, but no file it could be would accept a
    // delimiter, so the request fails rather than selecting the fallback.
    computed_invalid_options_for_missing_source:
        files &[],
        call (
            "---\n$schema:\n  delim: string(required)\ndelim: \";\"\n---\n",
            "load_data(\"optional.yaml\", \"string[]\", {delimiter: delim})",
        ),
        code "dm.data.invalid_options",
        detail |error| error["message"].as_str().unwrap().contains("applies only to CSV and TSV data");
    // The data file is present, so the only miss is the contract's.
    missing_contract_import:
        files &[("optional.yaml", "- a\n")],
        call ("", "load_data(\"optional.yaml\", \"Names@./types.yaml\")"),
        code "dm.expression.evaluation_failure",
        detail |error| error["reference"] == "./types.yaml";
    invalid_contract_import:
        files &[("optional.yaml", "- a\n"), ("types.yaml", "kind: schema\ntypes:\n  Names: definitely_unknown_type\n")],
        call ("", "load_data(\"optional.yaml\", \"Names@./types.yaml\")"),
        code "dm.expression.evaluation_failure",
        detail |error| error["message"].as_str().unwrap().contains("the contract `Names@./types.yaml` is invalid");
}

// ── Reporting ───────────────────────────────────────────────────────────────

/// Formatted output keeps stderr empty, and the envelope carries no diagnostic
/// for the absorbed miss while still conforming to its schema. The unguarded
/// control shows the miss would otherwise be reported.
#[test]
fn recovered_cause_emits_no_diagnostic() {
    let schema = SimplifiedSchema::try_from(include_str!("../../schemas/output/compose.yaml")).expect("the schema file parses");
    let fixture = CliProcessFixture::named("er_recovered_no_diagnostic");
    let document = fixture.write_file("cwd/doc.md", EXAMPLE);

    let formatted = compose(&fixture, &document, &[]);
    assert_eq!(formatted.status.code(), Some(0), "{}", stderr_of(&formatted));
    assert_eq!(stdout_of(&formatted), "[]\n\n");
    assert!(formatted.stderr.is_empty(), "{}", stderr_of(&formatted));

    let data = compose(&fixture, &document, &["--output", "json"]);
    assert_eq!(data.status.code(), Some(0), "{}", stderr_of(&data));
    assert!(data.stderr.is_empty(), "{}", stderr_of(&data));
    let report = envelope(&data);
    schema.validate_strict(&report).unwrap_or_else(|failure| panic!("{failure:?}\n{report:#}"));
    assert_eq!(report["outcome"], "success", "{report:#}");
    assert_eq!(report["diagnostics"], json!([]), "{report:#}");

    let unguarded = fixture.write_file("cwd/unguarded.md", "{{ as_json(load_data(\"optional.yaml\", \"string[]\")) }}\n");
    let control = compose(&fixture, &unguarded, &["--output", "json"]);
    assert_eq!(control.status.code(), Some(1), "{}", stdout_of(&control));
    let control = envelope(&control);
    schema.validate_strict(&control).unwrap_or_else(|failure| panic!("{failure:?}\n{control:#}"));
    assert_eq!(errors(&control).len(), 1, "{control:#}");
    assert_eq!(errors(&control)[0]["reference"], "optional.yaml", "{control:#}");
}

/// A recursive reference whose filename comes from the environment is judged
/// by the name it expands to: missing, it recovers like `%optional.yaml`
/// rather than failing as an extensionless request; present, it loads.
#[test]
fn recursive_env_reference_recovers_only_a_miss() {
    for (case, data, document) in [("missing", None, "[]"), ("present", Some("- loaded\n"), "[\"loaded\"]")] {
        let fixture = CliProcessFixture::named(&format!("er_recursive_env_{case}"));
        if let Some(data) = data {
            fixture.write_file("cwd/optional.yaml", data);
        }
        let doc = fixture.write_file("cwd/doc.md", "{{ as_json(try(load_data(\"%{{DATA_FILE}}\", \"string[]\"), [])) }}\n");

        let output = fixture
            .command()
            .env("DATA_FILE", "optional.yaml")
            .arg("compose")
            .arg(&doc)
            .args(["--output", "json"])
            .output()
            .unwrap();

        assert_eq!(output.status.code(), Some(0), "{case}: {}", stderr_of(&output));
        let report = envelope(&output);
        assert_eq!(report["diagnostics"], json!([]), "{case}: {report:#}");
        assert_eq!(report["document"].as_str().map(str::trim_end), Some(document), "{case}: {report:#}");
    }
}

/// `try` is not an absence guard: an undeclared read beside a recovered call
/// still warns, and `--deny all` stops the run before the later shell.
#[test]
fn unrelated_warning_blocks_under_deny() {
    let fixture = CliProcessFixture::named("er_unrelated_warning_deny");
    with_sentinel(&fixture);
    let document = fixture.write_file("cwd/doc.md", &format!("{EXAMPLE}\n{{{{ undeclared }}}}\n{SENTINEL}"));
    let marker = fixture.cwd().join("marker");

    let denied = compose(&fixture, &document, &["--output", "json", "--deny", "all"]);
    assert_eq!(denied.status.code(), Some(1), "{}", stdout_of(&denied));
    let report = envelope(&denied);
    assert_eq!(report["outcome"], "failure", "{report:#}");
    assert_eq!(codes(&report), ["dm.expression.undeclared_property"], "{report:#}");
    assert!(!marker.exists(), "the shell after the blocking warning ran");

    let allowed = compose(&fixture, &document, &["--output", "json"]);
    assert_eq!(allowed.status.code(), Some(0), "{}", stdout_of(&allowed));
    let report = envelope(&allowed);
    assert_eq!(codes(&report), ["dm.expression.undeclared_property"], "{report:#}");
    assert!(marker.exists(), "the control runs its shell");
}

// ── fallback_failure_reports_its_own_location ───────────────────────────────

/// A runtime data error is located at the start of the interpolation that
/// raised it (exactly as the same call unguarded would be), so the second
/// `{{` on line 10 locates the fallback's failure, not the recovered call on
/// line 8. The protected miss leaves no trace.
macro_rules! fallback_failure_location {
    ($($case:ident: $expression:expr, path $path:expr;)*) => {
        mod fallback_failure_reports_its_own_location {
            use super::*;
            $(
                #[test]
                fn $case() {
                    let fixture = CliProcessFixture::named(concat!("er_fallback_location_", stringify!($case)));
                    fixture.write_file("cwd/bad.xyz", "- a\n");
                    let content = format!(
                        "---\n$schema:\n  names: string[]\nnames: [\"missing.yaml\"]\n---\n# Lead\n\nok {EXAMPLE}\nLead {{{{ {} }}}}\n",
                        $expression
                    );
                    let document = fixture.write_file("cwd/doc.md", &content);

                    let output = compose(&fixture, &document, &["--output", "json"]);

                    assert_eq!(output.status.code(), Some(1), "{}", stdout_of(&output));
                    let report = envelope(&output);
                    assert_eq!(codes(&report), ["dm.data.unsupported_format"], "{report:#}");
                    let diagnostic = &report["diagnostics"][0];
                    assert_eq!(diagnostic["source"]["line"], 10, "{report:#}");
                    assert_eq!(diagnostic["source"]["column"], 6, "{report:#}");
                    assert_eq!(diagnostic["iteration_path"], $path, "{report:#}");
                    assert!(diagnostic["message"].as_str().unwrap().contains("bad.xyz"), "{report:#}");
                    assert_eq!(diagnostic.get("reference"), None, "the protected miss leaked: {report:#}");
                }
            )*
        }
    };
}

fallback_failure_location! {
    direct:
        "as_json(try(load_data(\"missing.yaml\", \"string[]\"), load_data(\"bad.xyz\", \"string[]\")))",
        path json!([]);
    inside_map:
        "as_json(map(names, name => try(load_data(name, \"string[]\"), load_data(\"bad.xyz\", \"string[]\"))))",
        path json!([{ "collection": "map", "position": 0 }]);
}
