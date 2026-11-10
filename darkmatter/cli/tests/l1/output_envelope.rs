//! The data contract of `md compose` and `md render`, through the real binary:
//! the envelope against its schema file, format parity, streams, width, the
//! lifecycle's output, exit statuses, `--show`, the render targets, and the
//! bytes the recipes that pipe `md compose` depend on.
//!
//! The opener is stubbed with `MD_DRY_RUN`, so nothing here launches an
//! application or takes focus.

use crate::common;

use common::CliProcessFixture;
use serde_json::{Value, json};
use simplified_schema::SimplifiedSchema;
use std::path::{Path, PathBuf};
use std::process::Output;
use test_toolkit::argv_recorder::ARGV_RECORDER_FILE;

const GOOD: &str = "---\ntitle: T\n---\n# Head\n\nbody text\n";
const BROKEN: &str = "# Head\n\nsee [gone](./nope.md)\n";
const LEGACY_TITLE: &str = "[Docs](https://example.com/a \"class='x'\")\n";
/// A paragraph with a soft break, wide enough that 20 columns reflows it.
const SOFT_BROKEN: &str = "one two three four five six seven\neight nine ten eleven twelve thirteen\n";
const LONG_PROSE: &str =
    "one two three four five six seven eight nine ten eleven twelve thirteen fourteen fifteen sixteen seventeen\n";
const DISCLOSURE: &str = "::disclosure Summary\n::details\nBody\n::end-disclosure\n";

const FRACTIONAL_WIDTH: &str = "dm.output.fractional_width";
const OPEN_FAILED: &str = "dm.output.open_failed";

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn run(fixture: &CliProcessFixture, args: &[&str]) -> Output {
    fixture.command().args(args).output().expect("md runs")
}

/// A fixture holding `content` as `doc.md`, in the launch directory.
fn with_document(name: &str, content: &str) -> CliProcessFixture {
    let fixture = CliProcessFixture::named(name);
    fixture.write_file("cwd/doc.md", content);
    fixture
}

/// A failing input for `command`: a broken link for compose, nothing to read
/// for render, which has no reference check.
fn failing_input(fixture: &CliProcessFixture, command: &str) -> &'static str {
    if command == "compose" {
        fixture.write_file("cwd/doc.md", BROKEN);
        "doc.md"
    } else {
        "missing.md"
    }
}

/// `text` in `format` as data, after proving it is exactly one value.
fn parse_data(format: &str, text: &str) -> Value {
    match format {
        "json" => {
            let mut stream = serde_json::Deserializer::from_str(text).into_iter::<Value>();
            let value = stream.next().expect("a value").unwrap_or_else(|error| panic!("{error}: {text}"));
            assert!(stream.next().is_none(), "one value only: {text}");
            value
        }
        "json5" => biscuit_file::Json5::from_str(text).unwrap_or_else(|error| panic!("{error}: {text}")).value().clone(),
        "yaml" => {
            assert!(!text.lines().skip(1).any(|line| line == "---"), "one document only: {text}");
            let parsed = biscuit_file::Yaml::from_str(text).unwrap_or_else(|error| panic!("{error}: {text}"));
            serde_json::to_value(parsed.value()).expect("YAML is data")
        }
        "toml" => {
            let parsed = biscuit_file::Toml::from_str(text).unwrap_or_else(|error| panic!("{error}: {text}"));
            serde_json::to_value(parsed.value()).expect("TOML is data")
        }
        other => panic!("not a data format: {other}"),
    }
}

/// `value` as TOML can hold it: every `null` object field removed.
fn without_nulls(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            Value::Object(map.iter().filter(|(_, field)| !field.is_null()).map(|(key, field)| (key.clone(), without_nulls(field))).collect())
        }
        Value::Array(items) => Value::Array(items.iter().map(without_nulls).collect()),
        other => other.clone(),
    }
}

fn validate_strictly(schema_text: &str, value: &Value) {
    let schema = SimplifiedSchema::try_from(schema_text).expect("the schema file parses");
    if let Err(failure) = schema.validate_strict(value) {
        panic!("{failure:?}\n{value:#}");
    }
}

fn codes(envelope: &Value) -> Vec<&str> {
    envelope["diagnostics"].as_array().expect("diagnostics").iter().map(|row| row["code"].as_str().expect("a code")).collect()
}

/// `document`, as `println!` put it on stdout, without that final newline.
fn printed(stdout: &str) -> &str {
    stdout.strip_suffix('\n').unwrap_or_else(|| panic!("no final newline: {stdout:?}"))
}

// ── 1. Conformance ─────────────────────────────────────────────────────────

enum Expect {
    Success,
    Failure,
}

/// Runs `args` with `--output json` beside `document` (when one is written),
/// validates the envelope strictly, and asserts its outcome.
fn conform(schema_text: &str, name: &str, document: Option<&str>, args: &[&str], expect: Expect) -> Value {
    let fixture = CliProcessFixture::named(name);
    if let Some(document) = document {
        fixture.write_file("cwd/doc.md", document);
    }
    let output = fixture.command().args(args).args(["--output", "json"]).output().expect("md runs");
    assert!(output.stderr.is_empty(), "{}", stderr_of(&output));
    let envelope = parse_data("json", &stdout_of(&output));
    validate_strictly(schema_text, &envelope);
    match expect {
        Expect::Success => {
            assert_eq!(output.status.code(), Some(0), "{envelope:#}");
            assert_eq!(envelope["outcome"], "success", "{envelope:#}");
            assert!(envelope["document"].is_string(), "{envelope:#}");
        }
        Expect::Failure => {
            assert_eq!(output.status.code(), Some(1), "{envelope:#}");
            assert_eq!(envelope["outcome"], "failure", "{envelope:#}");
            assert_eq!(envelope["document"], Value::Null, "{envelope:#}");
            for row in envelope["diagnostics"].as_array().expect("diagnostics") {
                if row["code"].as_str().expect("a code").starts_with("dm.reference.") {
                    assert!(row["source"]["line"].is_u64(), "{row:#}");
                    assert!(row["source"]["column"].is_u64(), "{row:#}");
                }
            }
        }
    }
    envelope
}

/// [`conform`], with exactly `codes` reported.
///
/// Each test below spells its schema file itself, because the CI index maps an
/// `include_str!` to the test whose name it can read; one inside a macro's
/// `fn $name` would select the whole binary for a schema edit.
fn assert_conforms(schema_text: &str, name: &str, document: Option<&str>, args: &[&str], expect: Expect, codes_expected: &[&str]) {
    let envelope = conform(schema_text, name, document, args, expect);
    assert_eq!(codes(&envelope), codes_expected, "{envelope:#}");
}

#[test]
fn conforms_compose_success() {
    assert_conforms(
        include_str!("../../schemas/output/compose.yaml"),
        "conforms_compose_success",
        Some(GOOD),
        &["compose", "doc.md"],
        Expect::Success,
        &[],
    );
}

#[test]
fn conforms_render_success() {
    assert_conforms(
        include_str!("../../schemas/output/render.yaml"),
        "conforms_render_success",
        Some(GOOD),
        &["render", "doc.md"],
        Expect::Success,
        &[],
    );
}

#[test]
fn conforms_compose_broken_link() {
    assert_conforms(
        include_str!("../../schemas/output/compose.yaml"),
        "conforms_compose_broken_link",
        Some(BROKEN),
        &["compose", "doc.md"],
        Expect::Failure,
        &["dm.reference.missing_hyperlink"],
    );
}

#[test]
fn conforms_compose_unreadable_input() {
    assert_conforms(
        include_str!("../../schemas/output/compose.yaml"),
        "conforms_compose_unreadable_input",
        None,
        &["compose", "missing.md"],
        Expect::Failure,
        &["dm.input.read_failure"],
    );
}

#[test]
fn conforms_render_unreadable_input() {
    assert_conforms(
        include_str!("../../schemas/output/render.yaml"),
        "conforms_render_unreadable_input",
        None,
        &["render", "missing.md"],
        Expect::Failure,
        &["dm.input.read_failure"],
    );
}

#[test]
fn conforms_compose_denied_warning() {
    assert_conforms(
        include_str!("../../schemas/output/compose.yaml"),
        "conforms_compose_denied_warning",
        Some(LEGACY_TITLE),
        &["compose", "doc.md", "--deny", "all"],
        Expect::Failure,
        &["dm.link.legacy_structured_title"],
    );
}

#[test]
fn conforms_render_denied_warning() {
    assert_conforms(
        include_str!("../../schemas/output/render.yaml"),
        "conforms_render_denied_warning",
        Some(LEGACY_TITLE),
        &["render", "doc.md", "--deny", "all"],
        Expect::Failure,
        &["dm.link.legacy_structured_title"],
    );
}

#[test]
fn conforms_compose_legacy_title_warning() {
    assert_conforms(
        include_str!("../../schemas/output/compose.yaml"),
        "conforms_compose_legacy_title_warning",
        Some(LEGACY_TITLE),
        &["compose", "doc.md"],
        Expect::Success,
        &["dm.link.legacy_structured_title"],
    );
}

#[test]
fn conforms_render_legacy_title_warning() {
    assert_conforms(
        include_str!("../../schemas/output/render.yaml"),
        "conforms_render_legacy_title_warning",
        Some(LEGACY_TITLE),
        &["render", "doc.md"],
        Expect::Success,
        &["dm.link.legacy_structured_title"],
    );
}

#[test]
fn conforms_compose_fractional_width() {
    assert_conforms(
        include_str!("../../schemas/output/compose.yaml"),
        "conforms_compose_fractional_width",
        Some(GOOD),
        &["compose", "doc.md", "--width", "12.9"],
        Expect::Success,
        &["dm.output.fractional_width"],
    );
}

#[test]
fn conforms_render_fractional_width() {
    assert_conforms(
        include_str!("../../schemas/output/render.yaml"),
        "conforms_render_fractional_width",
        Some(GOOD),
        &["render", "doc.md", "--width", "12.9"],
        Expect::Success,
        &["dm.output.fractional_width"],
    );
}

#[test]
fn conforms_with_a_null_file_for_stdin_and_a_failure_location() {
    let fixture = CliProcessFixture::named("conforms_stdin");
    let output = fixture.command().args(["compose", "-", "--output", "json"]).write_stdin("# Piped\n").output().expect("md runs");
    let envelope = parse_data("json", &stdout_of(&output));
    validate_strictly(include_str!("../../schemas/output/compose.yaml"), &envelope);
    assert_eq!(envelope["file"], Value::Null, "{envelope:#}");
    assert_eq!(envelope["summary"], Value::Null, "stdin runs no reference check: {envelope:#}");

    let envelope = conform(include_str!("../../schemas/output/compose.yaml"), "conforms_location", Some(BROKEN), &["compose", "doc.md"], Expect::Failure);
    let source = &envelope["diagnostics"][0]["source"];
    assert_eq!((&source["line"], &source["column"]), (&json!(3), &json!(5)), "{envelope:#}");
    assert_eq!(envelope["summary"], json!({ "references": 1, "documents": 1, "ok": 0, "broken": 1 }));
}

/// The conformance tests prove something only if the schema can refuse.
#[test]
fn the_schema_refuses_a_payload_outside_the_contract() {
    let schema = SimplifiedSchema::try_from(include_str!("../../schemas/output/compose.yaml")).expect("the schema file parses");
    let valid = json!({ "outcome": "success", "document": "x", "diagnostics": [], "lifecycle_output": [], "read_set": [] });
    assert!(schema.validate_strict(&valid).is_ok());
    let located = |data_location: Value| {
        json!({ "outcome": "failure", "diagnostics": [{ "code": "x", "severity": "error", "message": "m", "iteration_path": [], "data_location": data_location }], "lifecycle_output": [], "read_set": [] })
    };
    assert!(schema.validate_strict(&located(json!({ "file": "/d.json", "pointer": "/0", "line": 1, "column": 2 }))).is_ok());
    assert!(schema.validate_strict(&located(json!({ "file": "/d.csv", "byte_offset": 7 }))).is_ok());
    for invalid in [
        json!({ "outcome": "maybe", "diagnostics": [], "lifecycle_output": [], "read_set": [] }),
        json!({ "outcome": "success", "lifecycle_output": [], "read_set": [] }),
        json!({ "outcome": "success", "diagnostics": [], "lifecycle_output": [{ "stage": "composed" }], "read_set": [] }),
        json!({ "outcome": "success", "diagnostics": [{ "code": "x", "severity": "warn", "message": "m", "iteration_path": [] }], "lifecycle_output": [], "read_set": [] }),
        json!({ "outcome": "success", "diagnostics": [], "lifecycle_output": [] }),
        json!({ "outcome": "success", "diagnostics": [], "lifecycle_output": [], "read_set": [1] }),
        located(json!({ "pointer": "/0" })),
        located(json!({ "file": "/d.json", "offset": 3 })),
    ] {
        assert!(schema.validate_strict(&invalid).is_err(), "{invalid:#}");
    }
}

// ── 2. Format parity ───────────────────────────────────────────────────────

fn assert_parity(name: &str, command: &str, failing: bool, format: &str) {
    let fixture = with_document(name, GOOD);
    let input = if failing { failing_input(&fixture, command) } else { "doc.md" };
    let json_run = run(&fixture, &[command, input, "--output", "json"]);
    let run_in_format = run(&fixture, &[command, input, "--output", format]);
    assert_eq!(run_in_format.status.code(), json_run.status.code());
    assert_eq!(run_in_format.status.code(), Some(i32::from(failing)));

    let baseline = parse_data("json", &stdout_of(&json_run));
    let expected = if format == "toml" { without_nulls(&baseline) } else { baseline };
    assert_eq!(parse_data(format, &stdout_of(&run_in_format)), expected);
}

macro_rules! parity {
    ($($name:ident: $command:literal, $failing:literal, $format:literal;)*) => {$(
        #[test]
        fn $name() {
            assert_parity(stringify!($name), $command, $failing, $format);
        }
    )*};
}

parity! {
    parity_compose_success_json5: "compose", false, "json5";
    parity_compose_success_yaml: "compose", false, "yaml";
    parity_compose_success_toml: "compose", false, "toml";
    parity_compose_failure_json5: "compose", true, "json5";
    parity_compose_failure_yaml: "compose", true, "yaml";
    parity_compose_failure_toml: "compose", true, "toml";
    parity_render_success_json5: "render", false, "json5";
    parity_render_success_yaml: "render", false, "yaml";
    parity_render_success_toml: "render", false, "toml";
    parity_render_failure_json5: "render", true, "json5";
    parity_render_failure_yaml: "render", true, "yaml";
    parity_render_failure_toml: "render", true, "toml";
}

// ── 3. Streams ─────────────────────────────────────────────────────────────

fn assert_streams(command: &str, format: &str, flags: &[&str], failing: bool) {
    let fixture = with_document("streams", GOOD);
    let input = if failing { failing_input(&fixture, command) } else { "doc.md" };
    let output = fixture.command().args([command, input, "--output", format]).args(flags).output().expect("md runs");

    assert!(output.stderr.is_empty(), "stderr: {}", stderr_of(&output));
    assert_eq!(output.status.code(), Some(i32::from(failing)));
    let envelope = parse_data(format, &stdout_of(&output));
    assert_eq!(envelope["outcome"], if failing { "failure" } else { "success" }, "{envelope:#}");

    if flags.contains(&"--perf") {
        let stages = envelope["perf"]["stages"].as_array().unwrap_or_else(|| panic!("perf stages: {envelope:#}"));
        assert!(!stages.is_empty(), "{envelope:#}");
        for stage in stages {
            assert!(stage["name"].is_string(), "{stage:#}");
            assert!(stage["milliseconds"].is_number(), "{stage:#}");
        }
    } else {
        assert!(envelope.get("perf").is_none(), "{envelope:#}");
    }
}

macro_rules! stream_formats {
    ($command:literal, $flags:tt, $($format:ident),*) => {$(
        mod $format {
            #[test]
            fn success() {
                super::super::assert_streams($command, stringify!($format), &$flags, false);
            }

            #[test]
            fn failure() {
                super::super::assert_streams($command, stringify!($format), &$flags, true);
            }
        }
    )*};
}

macro_rules! streams {
    ($($name:ident: $command:literal, $flags:tt;)*) => {$(
        mod $name {
            stream_formats!($command, $flags, json, json5, yaml, toml);
        }
    )*};
}

streams! {
    streams_compose: "compose", [];
    streams_compose_verbose: "compose", ["-v"];
    streams_compose_very_verbose: "compose", ["-vv"];
    streams_compose_perf: "compose", ["--perf"];
    streams_compose_debug: "compose", ["--debug", "2"];
    streams_render: "render", [];
    streams_render_verbose: "render", ["-v"];
    streams_render_very_verbose: "render", ["-vv"];
    streams_render_debug: "render", ["--debug", "2"];
}

#[test]
fn a_perf_payload_validates_against_the_schema() {
    let envelope = conform(include_str!("../../schemas/output/compose.yaml"), "perf_conforms", Some(GOOD), &["compose", "doc.md", "--perf"], Expect::Success);
    assert!(!envelope["perf"]["stages"].as_array().expect("stages").is_empty(), "{envelope:#}");
}

// ── 4. Width in data ───────────────────────────────────────────────────────

fn document_of(output: &Output) -> String {
    let envelope = parse_data("json", &stdout_of(output));
    envelope["document"].as_str().unwrap_or_else(|| panic!("a document: {envelope:#}")).to_string()
}

fn assert_wrapped_at(text: &str, columns: usize) {
    assert!(text.trim_end().lines().count() > 1, "reflowed: {text:?}");
    assert!(text.lines().all(|line| line.chars().count() <= columns), "at {columns}: {text:?}");
}

#[test]
fn compose_document_without_a_width_is_the_default_stdout() {
    let fixture = with_document("width_default", SOFT_BROKEN);
    let markdown = stdout_of(&run(&fixture, &["compose", "doc.md"]));
    let data = run(&fixture, &["compose", "doc.md", "--output", "json"]);
    assert_eq!(document_of(&data).as_bytes(), printed(&markdown).as_bytes());
}

#[test]
fn compose_document_at_a_width_is_the_markdown_at_that_width() {
    let fixture = with_document("width_twenty", SOFT_BROKEN);
    let markdown = stdout_of(&run(&fixture, &["compose", "doc.md", "--width", "20"]));
    let document = document_of(&run(&fixture, &["compose", "doc.md", "--output", "json", "--width", "20"]));
    assert_eq!(document, printed(&markdown));
    assert_wrapped_at(&document, 20);
}

macro_rules! width_source_keeps_authored_newlines {
    ($($name:ident: $frontmatter:literal;)*) => {$(
        #[test]
        fn $name() {
            let fixture = with_document(stringify!($name), &format!("{}{SOFT_BROKEN}", $frontmatter));
            let markdown = stdout_of(&run(&fixture, &["compose", "doc.md", "--width", "source"]));
            assert_eq!(markdown.trim_end_matches('\n'), SOFT_BROKEN.trim_end_matches('\n'));
            let document = document_of(&run(&fixture, &["compose", "doc.md", "--output", "json", "--width", "source"]));
            assert_eq!(document, printed(&markdown));
        }
    )*};
}

width_source_keeps_authored_newlines! {
    width_source_without_frontmatter: "";
    width_source_with_fixed_width_frontmatter: "---\nfixed-width: 20\n---\n";
    width_source_with_unwrap_frontmatter: "---\nunwrap: true\n---\n";
}

#[test]
fn render_document_at_a_width_is_the_markdown_output_at_that_width() {
    let fixture = with_document("width_render", &format!("---\ntitle: T\n---\n{SOFT_BROKEN}"));
    let markdown = stdout_of(&run(&fixture, &["render", "doc.md", "--output", "markdown", "--width", "20"]));
    let document = document_of(&run(&fixture, &["render", "doc.md", "--output", "json", "--width", "20"]));
    assert_eq!(document, printed(&markdown));
    assert!(document.starts_with("---\ntitle: T\n---\n"), "{document}");
}

macro_rules! fractional_width {
    ($($name:ident: $command:literal, $formatted:literal;)*) => {$(
        #[test]
        fn $name() {
            let fixture = with_document(stringify!($name), SOFT_BROKEN);
            let formatted = run(&fixture, &[$command, "doc.md", "--output", $formatted, "--width", "12.9"]);
            assert_eq!(formatted.status.code(), Some(0), "{}", stderr_of(&formatted));
            assert!(stderr_of(&formatted).contains("using 12 for 12.9"), "{}", stderr_of(&formatted));
            assert_wrapped_at(&stdout_of(&formatted), 12);

            let data = run(&fixture, &[$command, "doc.md", "--output", "json", "--width", "12.9"]);
            assert!(data.stderr.is_empty(), "{}", stderr_of(&data));
            assert_eq!(data.status.code(), Some(0));
            let envelope = parse_data("json", &stdout_of(&data));
            assert_eq!(codes(&envelope), [FRACTIONAL_WIDTH], "{envelope:#}");
            assert_wrapped_at(envelope["document"].as_str().expect("a document"), 12);
        }
    )*};
}

fractional_width! {
    fractional_width_on_compose: "compose", "markdown";
    fractional_width_on_render: "render", "plain";
}

fn run_with_output_columns(fixture: &CliProcessFixture, columns: &str, args: &[&str]) -> Output {
    fixture.command_builder().application_input("OUTPUT_COLUMNS", columns).build().args(args).output().expect("md runs")
}

#[test]
fn output_columns_sets_the_width_when_no_flag_does() {
    let fixture = with_document("output_columns", LONG_PROSE);
    let output = run_with_output_columns(&fixture, "30", &["render", "doc.md", "--output", "plain"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr_of(&output));
    assert_wrapped_at(&stdout_of(&output), 30);
    let unset = stdout_of(&run(&fixture, &["render", "doc.md", "--output", "plain"]));
    assert_eq!(unset.trim_end().lines().count(), 1, "a pipe with neither is unlimited: {unset}");
}

#[test]
fn a_width_flag_beats_output_columns() {
    let fixture = with_document("width_beats_output_columns", LONG_PROSE);
    let with_both = run_with_output_columns(&fixture, "30", &["render", "doc.md", "--output", "plain", "--width", "20"]);
    assert_eq!(stdout_of(&with_both), stdout_of(&run(&fixture, &["render", "doc.md", "--output", "plain", "--width", "20"])));
    assert_wrapped_at(&stdout_of(&with_both), 20);
}

#[test]
fn an_invalid_output_columns_is_a_usage_error_only_when_needed() {
    let fixture = with_document("invalid_output_columns", LONG_PROSE);
    let needed = run_with_output_columns(&fixture, "wide", &["render", "doc.md", "--output", "plain"]);
    assert_eq!(needed.status.code(), Some(2), "{}", stderr_of(&needed));
    assert!(needed.stdout.is_empty(), "{}", stdout_of(&needed));
    assert!(stderr_of(&needed).contains("OUTPUT_COLUMNS"), "{}", stderr_of(&needed));

    let flagged = run_with_output_columns(&fixture, "wide", &["render", "doc.md", "--output", "plain", "--width", "20"]);
    assert_eq!(flagged.status.code(), Some(0), "{}", stderr_of(&flagged));
    assert_wrapped_at(&stdout_of(&flagged), 20);

    // Compose's `source` default reads no width at all.
    let source_default = run_with_output_columns(&fixture, "wide", &["compose", "doc.md"]);
    assert_eq!(source_default.status.code(), Some(0), "{}", stderr_of(&source_default));
}

// ── 5. Lifecycle output ────────────────────────────────────────────────────

const LIFECYCLE: &str = "---\ncomposed:\n  - stdout: hi\n  - info: note\n  - shell: \"argv-recorder marker\"\n---\nthe-document\n";

/// A fixture with `doc.md` holding [`LIFECYCLE`], its command approved and
/// native, and the path the command records to.
fn lifecycle_fixture(name: &str) -> (CliProcessFixture, PathBuf) {
    let fixture = with_document(name, LIFECYCLE);
    std::fs::copy(
        biscuit_test_harness::bin_exe!("darkmatter-argv-recorder"),
        fixture.bin_dir().join(format!("argv-recorder{}", std::env::consts::EXE_SUFFIX)),
    )
    .expect("the recorder fixture binary is built with `test-fixtures`");
    fixture.write_file("cwd/.darkmatter-shell-whitelist", "exact argv-recorder marker\n");
    let marker = fixture.cwd().join("marker");
    (fixture, marker)
}

fn run_lifecycle(fixture: &CliProcessFixture, marker: &Path, extra: &[&str]) -> Output {
    fixture
        .command()
        .env(ARGV_RECORDER_FILE, marker)
        .env("ARGV_RECORDER_STDOUT", "shell-out\n")
        .env("ARGV_RECORDER_STDERR", "shell-err\n")
        .args(["compose", "doc.md"])
        .args(extra)
        .output()
        .expect("md runs")
}

fn assert_lifecycle_output(name: &str, format: &str) {
    let (fixture, marker) = lifecycle_fixture(name);
    let output = run_lifecycle(&fixture, &marker, &["--output", format]);
    assert!(output.stderr.is_empty(), "{}", stderr_of(&output));
    assert_eq!(output.status.code(), Some(0));
    let envelope = parse_data(format, &stdout_of(&output));
    assert_eq!(envelope["document"], "the-document", "{envelope:#}");
    assert_eq!(
        envelope["lifecycle_output"],
        json!([
            { "stage": "composed", "step": "stdout", "text": "hi" },
            { "stage": "composed", "step": "info", "text": "note" },
            { "stage": "composed", "step": "argv-recorder marker", "text": "shell-out\nshell-err\n" },
        ]),
        "{envelope:#}"
    );
}

macro_rules! lifecycle_output {
    ($($name:ident: $format:literal;)*) => {$(
        #[test]
        fn $name() {
            assert_lifecycle_output(stringify!($name), $format);
        }
    )*};
}

lifecycle_output! {
    lifecycle_output_in_json: "json";
    lifecycle_output_in_json5: "json5";
    lifecycle_output_in_yaml: "yaml";
    lifecycle_output_in_toml: "toml";
}

#[test]
fn lifecycle_output_in_markdown_is_on_stderr_and_stdout_is_the_document() {
    let (fixture, marker) = lifecycle_fixture("lifecycle_markdown");
    let output = run_lifecycle(&fixture, &marker, &[]);
    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    assert_eq!(stdout_of(&output), "the-document\n");
    let mut remaining = stderr.as_str();
    for line in ["hi", "note", "shell-out", "shell-err"] {
        let found = remaining.find(line).unwrap_or_else(|| panic!("{line} in run order: {stderr}"));
        remaining = &remaining[found + line.len()..];
    }
}

#[test]
fn a_lifecycle_failure_is_one_payload() {
    let fixture = with_document(
        "lifecycle_failure",
        "---\ncomposed:\n  - stdout: hi\n  - error: the gate refused\n---\nthe-document\n",
    );
    let output = run(&fixture, &["compose", "doc.md", "--output", "json"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty(), "{}", stderr_of(&output));
    let envelope = parse_data("json", &stdout_of(&output));
    validate_strictly(include_str!("../../schemas/output/compose.yaml"), &envelope);
    assert_eq!(envelope["outcome"], "failure", "{envelope:#}");
    assert_eq!(envelope["document"], Value::Null, "{envelope:#}");
    assert_eq!(codes(&envelope), ["dm.lifecycle.failure"], "{envelope:#}");
    assert_eq!(envelope["lifecycle_output"][0]["text"], "hi", "{envelope:#}");
}

// ── 6. Exit status ─────────────────────────────────────────────────────────

macro_rules! broken_reference_exits_1 {
    ($($name:ident: $frontmatter:literal;)*) => {$(
        mod $name {
            use super::*;

            fn document() -> String {
                format!("{}{BROKEN}", $frontmatter)
            }

            #[test]
            fn formatted() {
                let fixture = with_document(stringify!($name), &document());
                let output = run(&fixture, &["compose", "doc.md"]);
                assert_eq!(output.status.code(), Some(1), "{}", stderr_of(&output));
                assert!(output.stdout.is_empty(), "{}", stdout_of(&output));
                assert!(stderr_of(&output).contains("Broken references in doc.md"), "{}", stderr_of(&output));
            }

            #[test]
            fn data() {
                let fixture = with_document(stringify!($name), &document());
                let output = run(&fixture, &["compose", "doc.md", "--output", "json"]);
                assert_eq!(output.status.code(), Some(1), "{}", stderr_of(&output));
                assert!(output.stderr.is_empty(), "{}", stderr_of(&output));
                let envelope = parse_data("json", &stdout_of(&output));
                assert_eq!(envelope["outcome"], "failure", "{envelope:#}");
                assert_eq!(codes(&envelope), ["dm.reference.missing_hyperlink"], "{envelope:#}");
            }
        }
    )*};
}

broken_reference_exits_1! {
    broken_reference_without_a_lifecycle: "";
    broken_reference_with_a_lifecycle: "---\ninitialize:\n  - ensure_file: [\"made\", \"x\"]\n---\n";
}

macro_rules! usage_errors {
    ($($name:ident: [$($arg:literal),*];)*) => {$(
        #[test]
        fn $name() {
            let fixture = with_document(stringify!($name), GOOD);
            let output = run(&fixture, &[$($arg),*]);
            assert_eq!(output.status.code(), Some(2), "{}", stderr_of(&output));
            assert!(output.stdout.is_empty(), "{}", stdout_of(&output));
            assert!(!output.stderr.is_empty());
        }
    )*};
}

usage_errors! {
    usage_unknown_flag: ["compose", "doc.md", "--no-such-flag"];
    usage_misspelled_subcommand: ["comopse", "doc.md"];
    usage_compose_html: ["compose", "doc.md", "--output", "html"];
    usage_compose_terminal: ["compose", "doc.md", "--output", "terminal"];
    usage_compose_plain: ["compose", "doc.md", "--output", "plain"];
    usage_compose_markdown_plus: ["compose", "doc.md", "--output", "markdown-plus"];
    usage_compose_browser: ["compose", "doc.md", "--output", "browser"];
    usage_compose_auto: ["compose", "doc.md", "--output", "auto"];
    usage_compose_ast: ["compose", "doc.md", "--output", "ast"];
    usage_compose_text: ["compose", "doc.md", "--output", "text"];
    usage_compose_report_json: ["compose", "doc.md", "--output", "report-json"];
    usage_render_auto: ["render", "doc.md", "--output", "auto"];
    usage_render_ast: ["render", "doc.md", "--output", "ast"];
    usage_render_text: ["render", "doc.md", "--output", "text"];
    usage_render_report_json: ["render", "doc.md", "--output", "report-json"];
    usage_shell_with_a_data_output: ["compose", "doc.md", "--shell", "--output", "json"];
    usage_compose_width_zero: ["compose", "doc.md", "--width", "0"];
    usage_compose_width_word: ["compose", "doc.md", "--width", "abc"];
    usage_render_width_zero: ["render", "doc.md", "--width", "0"];
    usage_render_width_word: ["render", "doc.md", "--width", "abc"];
}

// ── 7. `--show` ────────────────────────────────────────────────────────────

/// The files `--show` wrote: the fixture pins the temp directory for the child
/// through `TMPDIR`, `TEMP`, and `TMP`.
fn artifacts(fixture: &CliProcessFixture) -> Vec<PathBuf> {
    std::fs::read_dir(fixture.tmp_dir())
        .expect("the temp directory")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| path.file_name().is_some_and(|name| name.to_string_lossy().starts_with("md-output-")))
        .collect()
}

fn run_show(fixture: &CliProcessFixture, dry_run: &str, args: &[&str]) -> Output {
    fixture.command_builder().application_input("MD_DRY_RUN", dry_run).build().args(args).args(["--show"]).output().expect("md runs")
}

fn assert_show_writes_the_value(name: &str, command: &str, format: &str) {
    let fixture = with_document(name, GOOD);
    let output = run_show(&fixture, "1", &[command, "doc.md", "--output", format]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr_of(&output));
    assert!(output.stderr.is_empty(), "{}", stderr_of(&output));

    let files = artifacts(&fixture);
    assert_eq!(files.len(), 1, "{files:?}");
    assert_eq!(files[0].extension().and_then(|extension| extension.to_str()), Some(format));
    let content = std::fs::read_to_string(&files[0]).expect("the artifact");
    let stdout = stdout_of(&output);
    assert_eq!(content.trim_end_matches('\n'), stdout.trim_end_matches('\n'));
    assert_eq!(parse_data(format, &stdout)["outcome"], "success");
}

fn assert_show_failure_is_one_payload(name: &str, command: &str, format: &str) {
    let fixture = with_document(name, GOOD);
    let output = run_show(&fixture, "fail", &[command, "doc.md", "--output", format]);
    assert_eq!(output.status.code(), Some(1), "{}", stderr_of(&output));
    assert!(output.stderr.is_empty(), "{}", stderr_of(&output));
    let envelope = parse_data(format, &stdout_of(&output));
    assert_eq!(envelope["outcome"], "failure", "{envelope:#}");
    assert_eq!(envelope["document"], Value::Null, "absent in TOML, which indexes to null: {envelope:#}");
    assert_eq!(codes(&envelope), [OPEN_FAILED], "{envelope:#}");
    let files = artifacts(&fixture);
    assert_eq!(files.len(), 1, "{files:?}");
    let attempted = files[0].file_name().expect("a name").to_string_lossy().into_owned();
    let message = envelope["diagnostics"][0]["message"].as_str().expect("a message");
    assert!(message.contains(&attempted), "{attempted} in {message}");
}

macro_rules! show {
    ($($name:ident: $command:literal, $format:literal;)*) => {$(
        mod $name {
            use super::*;

            #[test]
            fn writes_the_value() {
                assert_show_writes_the_value(concat!(stringify!($name), "_ok"), $command, $format);
            }

            #[test]
            fn reports_an_open_failure() {
                assert_show_failure_is_one_payload(concat!(stringify!($name), "_fail"), $command, $format);
            }
        }
    )*};
}

show! {
    show_compose_json: "compose", "json";
    show_compose_json5: "compose", "json5";
    show_compose_yaml: "compose", "yaml";
    show_compose_toml: "compose", "toml";
    show_render_json: "render", "json";
    show_render_json5: "render", "json5";
    show_render_yaml: "render", "yaml";
    show_render_toml: "render", "toml";
}

#[test]
fn render_html_show_failure_names_the_path_on_stderr() {
    let fixture = with_document("show_render_html", GOOD);
    let output = run_show(&fixture, "fail", &["render", "doc.md", "--output", "html"]);
    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    let files = artifacts(&fixture);
    assert_eq!(files.len(), 1, "{files:?}");
    assert_eq!(files[0].extension().and_then(|extension| extension.to_str()), Some("html"));
    let attempted = files[0].file_name().expect("a name").to_string_lossy().into_owned();
    assert!(common::unwrapped(&stderr).contains(&attempted), "{attempted} in {stderr}");
}

// ── 8. Render targets ──────────────────────────────────────────────────────

macro_rules! terminal_is_plain_on_a_pipe {
    ($($name:ident: [$($arg:literal),*];)*) => {$(
        #[test]
        fn $name() {
            let fixture = with_document(stringify!($name), &format!("# Head\n\n{LONG_PROSE}\n- **a** `b`\n"));
            // A capable, colored environment: only the pipe turns styling off.
            let terminal = fixture
                .command_builder()
                .rendering_input("FORCE_COLOR", "1")
                .rendering_input("COLORTERM", "truecolor")
                .rendering_input("TERM", "xterm-256color")
                .build()
                .args(["render", "doc.md", "--output", "terminal", $($arg),*])
                .output()
                .expect("md runs");
            let plain = run(&fixture, &["render", "doc.md", "--output", "plain", $($arg),*]);
            assert_eq!(terminal.status.code(), Some(0), "{}", stderr_of(&terminal));
            assert_eq!(terminal.stdout, plain.stdout);
            assert!(!terminal.stdout.contains(&0x1b), "{}", stdout_of(&terminal));
            let default = run(&fixture, &["doc.md", $($arg),*]);
            assert_eq!(default.stdout, plain.stdout, "the default target");
        }
    )*};
}

terminal_is_plain_on_a_pipe! {
    terminal_is_plain_without_a_width: [];
    terminal_is_plain_at_a_width: ["--width", "40"];
}

#[test]
fn a_disclosure_stays_a_directive_in_markdown_and_is_details_in_markdown_plus() {
    let fixture = with_document("disclosure", DISCLOSURE);
    let markdown = stdout_of(&run(&fixture, &["render", "doc.md", "--output", "markdown"]));
    assert!(markdown.contains("::disclosure Summary"), "{markdown}");
    assert!(!markdown.contains("<details>"), "{markdown}");
    let plus = stdout_of(&run(&fixture, &["render", "doc.md", "--output", "markdown-plus"]));
    assert!(plus.contains("<details>"), "{plus}");
    assert!(!plus.contains("::disclosure"), "{plus}");
}

#[test]
fn render_markdown_is_the_source_with_its_frontmatter() {
    let fixture = with_document("render_markdown", GOOD);
    let markdown = stdout_of(&run(&fixture, &["render", "doc.md", "--output", "markdown"]));
    assert_eq!(markdown.trim_end_matches('\n'), GOOD.trim_end_matches('\n'));
}

#[test]
fn render_html_is_a_page() {
    let fixture = with_document("render_html", GOOD);
    let html = stdout_of(&run(&fixture, &["render", "doc.md", "--output", "html"]));
    assert!(html.contains("<!DOCTYPE"), "{html}");
}

/// `style.max-width` caps the content inside the width `--width` makes
/// available, in both the bare `md` form and `md render`; given before
/// `render` it is a root render argument, which is a usage error.
#[test]
fn max_width_caps_prose_inside_the_width() {
    let fixture = with_document("max_width", LONG_PROSE);
    for args in [
        &["doc.md", "style.max-width=30", "--width", "60", "--output", "plain"][..],
        &["render", "doc.md", "style.max-width=30", "--width", "60", "--output", "plain"],
    ] {
        let output = run(&fixture, args);
        assert_eq!(output.status.code(), Some(0), "{args:?}: {}", stderr_of(&output));
        assert_wrapped_at(&stdout_of(&output), 30);
    }
    let output = run(&fixture, &["style.max-width=30", "render", "doc.md", "--width", "60", "--output", "plain"]);
    assert_eq!(output.status.code(), Some(2), "{}", stderr_of(&output));
}

#[test]
fn the_render_envelope_keeps_the_frontmatter_in_its_document() {
    let fixture = with_document("render_envelope_frontmatter", GOOD);
    let document = document_of(&run(&fixture, &["render", "doc.md", "--output", "json"]));
    assert_eq!(document.trim_end_matches('\n'), GOOD.trim_end_matches('\n'));
}

const UNKNOWN_STYLE: &str = "---\nstyle:\n  page:\n    bogus: 1\n---\n# Styled\n";

#[test]
fn a_style_warning_goes_to_stderr_in_a_formatted_output() {
    let fixture = with_document("style_warning_formatted", UNKNOWN_STYLE);
    for target in ["plain", "html"] {
        let output = run(&fixture, &["doc.md", "--output", target]);
        assert_eq!(output.status.code(), Some(0), "{target}: {}", stderr_of(&output));
        assert!(stderr_of(&output).contains("unknown style key `style.page.bogus`"), "{target}: {}", stderr_of(&output));
        assert!(!stdout_of(&output).contains("unknown style key"), "{target}");
    }
}

#[test]
fn a_style_warning_is_a_diagnostic_with_an_empty_stderr_in_a_data_output() {
    let fixture = with_document("style_warning_data", UNKNOWN_STYLE);
    let output = run(&fixture, &["doc.md", "--output", "json"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr_of(&output));
    assert_eq!(stderr_of(&output), "");
    let envelope: serde_json::Value = serde_json::from_str(&stdout_of(&output)).expect("one value");
    let codes: Vec<&str> = envelope["diagnostics"].as_array().unwrap().iter().map(|d| d["code"].as_str().unwrap()).collect();
    assert_eq!(codes, ["dm.style.unknown_key"]);
}

#[test]
fn a_denied_style_warning_fails_a_data_output_with_one_payload() {
    let fixture = with_document("style_warning_denied", UNKNOWN_STYLE);
    let output = run(&fixture, &["doc.md", "--output", "json", "--deny", "style"]);
    assert_eq!(output.status.code(), Some(1), "{}", stderr_of(&output));
    assert_eq!(stderr_of(&output), "");
    let envelope: serde_json::Value = serde_json::from_str(&stdout_of(&output)).expect("one value");
    assert_eq!(envelope["outcome"], "failure");
    assert!(envelope["document"].is_null());
}

// ── 9. The recipes' bytes ──────────────────────────────────────────────────

const RECIPE: &str = "---\ntitle: Recipe\nmode: fast\n---\n# {{ title }}\n\nMode is {{ mode }}.\n\n::file ./part.md\n\n```sh\necho   hi\n```\n";
const RECIPE_PART: &str = "Included **text**\nwith a soft break.\n";

fn recipe_fixture(name: &str) -> CliProcessFixture {
    let fixture = with_document(name, RECIPE);
    fixture.write_file("cwd/part.md", RECIPE_PART);
    fixture
}

/// The homelab recipes run `md compose X --frontmatter`.
#[test]
fn the_frontmatter_recipe_keeps_its_bytes() {
    let fixture = recipe_fixture("recipe_frontmatter");
    let output = run(&fixture, &["compose", "doc.md", "--frontmatter"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr_of(&output));
    assert_eq!(
        stdout_of(&output),
        "---\ntitle: Recipe\nmode: fast\n---\n# Recipe\n\nMode is fast.\n\nIncluded **text** with a soft break.\n\n\n```sh\necho   hi\n```\n"
    );
    assert_eq!(output.stdout, run(&fixture, &["compose", "doc.md", "--frontmatter", "--output", "markdown"]).stdout);
}

/// The claudine recipes run `md compose X`.
#[test]
fn the_plain_recipe_keeps_its_bytes() {
    let fixture = recipe_fixture("recipe_plain");
    let output = run(&fixture, &["compose", "doc.md"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr_of(&output));
    assert_eq!(stdout_of(&output), "# Recipe\n\nMode is fast.\n\nIncluded **text** with a soft break.\n\n\n```sh\necho   hi\n```\n");
    assert_eq!(output.stdout, run(&fixture, &["compose", "doc.md", "--output", "markdown"]).stdout);
}

// ── 10. Formatted diagnostics ──────────────────────────────────────────────

/// A report line is never wrapped off a terminal, whatever `--width` says.
#[test]
fn the_problem_report_is_plain_and_unwrapped_on_a_pipe() {
    let reference = "./a-long-reference-name-well-past-any-narrow-column-limit-at-all.md";
    let fixture = with_document("report_unwrapped", &format!("[gone]({reference})\n"));
    for args in [&["compose", "doc.md"][..], &["compose", "doc.md", "--width", "20"]] {
        let output = run(&fixture, args);
        let stderr = stderr_of(&output);
        assert_eq!(output.status.code(), Some(1), "{stderr}");
        assert!(output.stdout.is_empty(), "{}", stdout_of(&output));
        assert!(!output.stderr.contains(&0x1b), "{stderr:?}");
        let line = stderr.lines().find(|line| line.contains(reference)).unwrap_or_else(|| panic!("whole on one line: {stderr}"));
        assert!(line.contains("doc.md:1:1"), "{line}");
    }
}

/// A terminal on stderr sets stderr's width alone; the deliverable's width
/// comes from `--width`.
#[test]
fn stderr_has_its_own_width_on_a_terminal() {
    use biscuit_terminal::output::{OutputTarget, WidthRequest};
    use darkmatter_cli::output::{OutputEnvironment, resolve_output};
    use renderable::Width;

    let environment = OutputEnvironment { stderr_is_terminal: true, stderr_columns: Some(33), ..OutputEnvironment::default() };
    let stderr = environment.stderr_options(&environment.stderr_terminal());
    assert_eq!(stderr.width, Width::Columns(33));

    let resolved = resolve_output(OutputTarget::Plain, Some(WidthRequest::Columns(90)), WidthRequest::Auto, 0, &environment)
        .expect("a valid width");
    assert_eq!(resolved.options.width, Width::Columns(90));
    assert_eq!(environment.stderr_options(&environment.stderr_terminal()).width, Width::Columns(33));
}

// ── Read set and data location ─────────────────────────────────────────────

/// How the read set names a file that exists: its canonical path, with `/`.
fn read_entry(path: &Path) -> String {
    biscuit_file::to_portable_string(&biscuit_file::canonicalize_simplified(path).expect("the file exists"))
}

/// Runs `args` with `--output json` and returns the one value, after requiring
/// an empty stderr and the exit status `status`.
#[track_caller]
fn json_run(fixture: &CliProcessFixture, args: &[&str], status: i32) -> Value {
    let output = fixture.command().args(args).args(["--output", "json"]).output().expect("md runs");
    assert!(output.stderr.is_empty(), "{}", stderr_of(&output));
    let envelope = parse_data("json", &stdout_of(&output));
    assert_eq!(output.status.code(), Some(status), "{envelope:#}");
    envelope
}

#[test]
fn compose_reports_every_expression_read_sorted() {
    let fixture = CliProcessFixture::named("read_set_success");
    let child = fixture.write_file("cwd/b.md", "---\ntitle: Child\n---\n");
    fixture.write_file("cwd/doc.md", "{{ frontmatter(\"b.md\").title }} {{ file_exists(\"c.md\") }} {{ file_exists(\"b.md\") }}\n");

    let envelope = json_run(&fixture, &["compose", "doc.md"], 0);
    validate_strictly(include_str!("../../schemas/output/compose.yaml"), &envelope);
    assert_eq!(envelope["document"].as_str().map(str::trim_end), Some("Child false true"), "{envelope:#}");
    // `c.md` does not exist: the read set names the path `file_exists` probed.
    let probed = format!("{}/c.md", read_entry(fixture.cwd()));
    assert_eq!(envelope["read_set"], json!([read_entry(&child), probed]), "{envelope:#}");
}

#[test]
fn compose_without_reads_reports_an_empty_read_set() {
    let envelope = conform(include_str!("../../schemas/output/compose.yaml"), "read_set_empty", Some(GOOD), &["compose", "doc.md"], Expect::Success);
    assert_eq!(envelope["read_set"], json!([]), "{envelope:#}");
}

/// A read-side function's missing file is an expression failure that also
/// carries the reference fields a broken link has.
#[test]
fn a_failed_frontmatter_read_reports_its_reference_details() {
    let envelope = conform(
        include_str!("../../schemas/output/compose.yaml"),
        "read_set_missing_frontmatter",
        Some("Value: {{ frontmatter(\"missing.md\").title }}\n"),
        &["compose", "doc.md"],
        Expect::Failure,
    );
    assert!(envelope["read_set"].is_array(), "{envelope:#}");
    let record = &envelope["diagnostics"][0];
    assert_eq!(record["code"], "dm.expression.evaluation_failure", "{envelope:#}");
    assert_eq!(record["construct"], "frontmatter()", "{record:#}");
    assert_eq!(record["reference"], "missing.md", "{record:#}");
    assert!(record["reason"].is_string(), "{record:#}");
    assert!(record["failure"].is_string(), "{record:#}");
}

/// The reference check reads the data before composition starts, so its
/// failure has no reference totals and still names the file it read.
#[test]
fn a_malformed_data_file_failing_the_reference_check_is_in_the_read_set() {
    let fixture = CliProcessFixture::named("read_set_reference_check");
    let data = fixture.write_file("cwd/bad.json", "{\"a\": 1,\n");
    fixture.write_file("cwd/doc.md", "{{ load_data(\"bad.json\", \"string[]\") }}\n");

    let envelope = json_run(&fixture, &["compose", "doc.md"], 1);
    validate_strictly(include_str!("../../schemas/output/compose.yaml"), &envelope);
    assert_eq!(envelope["outcome"], "failure", "{envelope:#}");
    assert_eq!(envelope["summary"], Value::Null, "the check stopped: {envelope:#}");
    assert_eq!(envelope["read_set"], json!([read_entry(&data)]), "{envelope:#}");
}

/// `[1, "x"]` against `number[]`: one record, at `/1`, line 1, column 5.
fn contract_mismatch(name: &str) -> (CliProcessFixture, PathBuf) {
    let fixture = CliProcessFixture::named(name);
    let data = fixture.write_file("cwd/ports.json", "[1, \"x\"]\n");
    fixture.write_file("cwd/doc.md", "Ports: {{ load_data(\"ports.json\", \"number[]\") }}\n");
    (fixture, data)
}

#[track_caller]
fn assert_mismatch_record(envelope: &Value, data: &Path) {
    let record = envelope["diagnostics"]
        .as_array()
        .expect("diagnostics")
        .iter()
        .find(|record| record["code"] == "dm.data.contract_mismatch")
        .unwrap_or_else(|| panic!("no contract mismatch: {envelope:#}"));
    assert_eq!(
        record["data_location"],
        json!({ "file": read_entry(data), "pointer": "/1", "line": 1, "column": 5 }),
        "{record:#}"
    );
    assert_eq!((&record["expected_type"], &record["actual_type"]), (&json!("number"), &json!("string")), "{record:#}");
}

#[test]
fn conforms_compose_contract_mismatch_with_its_data_location() {
    let (fixture, data) = contract_mismatch("data_location_compose");
    let envelope = json_run(&fixture, &["compose", "doc.md"], 1);
    validate_strictly(include_str!("../../schemas/output/compose.yaml"), &envelope);
    assert_mismatch_record(&envelope, &data);
    assert_eq!(envelope["read_set"], json!([read_entry(&data)]), "{envelope:#}");
}

/// Graph preparation evaluates the document's expressions.
#[test]
fn conforms_graph_contract_mismatch_with_its_data_location() {
    let (fixture, data) = contract_mismatch("data_location_graph");
    let envelope = json_run(&fixture, &["graph", "doc.md"], 1);
    validate_strictly(include_str!("../../schemas/output/graph.yaml"), &envelope);
    assert_mismatch_record(&envelope, &data);
}

#[test]
fn conforms_validate_refs_contract_mismatch_with_its_data_location() {
    let (fixture, data) = contract_mismatch("data_location_validate_refs");
    let envelope = json_run(&fixture, &["validate", "refs", "doc.md"], 1);
    validate_strictly(include_str!("../../schemas/output/validate-refs.yaml"), &envelope);
    assert_mismatch_record(&envelope, &data);
}

/// `md render` evaluates no expression, so a `load_data` call is text and the
/// read set is empty.
#[test]
fn conforms_render_with_an_empty_read_set_and_no_evaluation() {
    let (fixture, _) = contract_mismatch("read_set_render");
    let envelope = json_run(&fixture, &["render", "doc.md"], 0);
    validate_strictly(include_str!("../../schemas/output/render.yaml"), &envelope);
    assert_eq!(envelope["read_set"], json!([]), "{envelope:#}");
    assert!(envelope["document"].as_str().expect("a document").contains("load_data("), "{envelope:#}");
}

/// The stderr report of a missing `frontmatter()` file, exactly as it was
/// before diagnostics carried reference details. Only the header icon, which
/// the environment's icon support selects, is left out of the comparison.
#[test]
fn a_missing_frontmatter_file_reports_the_same_stderr_text() {
    let fixture = with_document("read_set_stderr_unchanged", "---\ntitle: A\n---\nValue: {{ frontmatter(\"missing.md\").title }}\n");
    let output = run(&fixture, &["compose", "doc.md"]);
    assert_eq!(output.status.code(), Some(1), "{}", stderr_of(&output));
    assert!(output.stdout.is_empty(), "{}", stdout_of(&output));
    let document = biscuit_file::canonicalize_simplified(&fixture.cwd().join("doc.md")).expect("the document exists");
    let expected = format!(
        "MarkdownError: invalid file path\n\
         ┃ \n\
         ┃ A document expression references the file missing.md, which could not be resolved.\n\
         ┃ \n\
         ┃ Defined in:\n\
         ┃ \n\
         ┃ {}\n\
         ┃ \n\
         ┃ Expression at line: 4, column: 8\n\
         ┃ \n\
         ┃ ```markdown\n\
         ┃   2 │ title: A\n\
         ┃   3 │ ---\n\
         ┃ > 4 │ Value: {{{{ frontmatter(\"missing.md\").title }}}}\n\
         ┃ ```\n\
         ┃ \n\
         ┃ Confirm the path is correct, or guard an optional reference with `file_exists(...)` so a missing file is not treated as an error.\n",
        document.display()
    );
    let stderr = stderr_of(&output);
    let (_icon, report) = stderr.split_once(' ').unwrap_or_else(|| panic!("an icon, then the report: {stderr}"));
    assert_eq!(report, expected);
}
