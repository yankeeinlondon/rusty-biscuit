//! The render style input-robustness matrix through the real binary, kept
//! sparse: each `--style` loader's control, one malformed file, and one
//! semantic failure, then the cross-cutting rules (stylesheet origins and
//! target reads, refused and denied styles, and no execution of style text).
//!
//! The in-process cells are in `render_style_matrix`. Every process runs in
//! its own fixture with `MD_DRY_RUN=fail`, so an attempted artifact launch
//! fails loudly instead of opening anything.

use std::process::Output;

use serde_json::{Value, json};

use crate::common::{CliProcessFixture, MockHttpResponse, mock_http_server};


fn process(name: &str) -> CliProcessFixture {
    let fixture = CliProcessFixture::named(name);
    fixture.write_file("cwd/doc.md", "# Title\n\nSome body text.\n");
    fixture
}

fn run(fixture: &CliProcessFixture, args: &[&str]) -> Output {
    fixture.command_builder().application_input("MD_DRY_RUN", "fail").build().args(args).output().expect("md runs")
}

fn run_stdin(fixture: &CliProcessFixture, args: &[&str], stdin: &str) -> Output {
    fixture
        .command_builder()
        .application_input("MD_DRY_RUN", "fail")
        .build()
        .args(args)
        .write_stdin(stdin)
        .output()
        .expect("md runs")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// The one envelope of a data output: exit `code`, stderr empty.
fn envelope(output: &Output, code: i32) -> Value {
    assert_eq!(output.status.code(), Some(code), "{}", stdout(output));
    assert!(output.stderr.is_empty(), "a data output owns stderr: {}", stderr(output));
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| panic!("one JSON value: {error}: {}", stdout(output)))
}

fn error_records(value: &Value) -> Vec<&Value> {
    let records = value["diagnostics"].as_array().unwrap_or_else(|| panic!("no diagnostics: {value:#}"));
    records.iter().filter(|record| record["severity"] == "error").collect()
}

/// A formatted failure: exit 1, nothing on stdout, nothing opened, and the
/// diagnostic on stderr naming `needle`.
fn assert_formatted_failure(output: &Output, needle: &str) {
    assert_eq!(output.status.code(), Some(1), "{}", stderr(output));
    assert!(output.stdout.is_empty(), "nothing is rendered: {}", stdout(output));
    assert!(!stderr(output).contains("no default application"), "an artifact was opened: {}", stderr(output));
    assert!(stderr(output).contains(needle), "{needle} in: {}", stderr(output));
}

/// The number of leading spaces on the line holding `needle`.
fn indent_of(text: &str, needle: &str) -> usize {
    let line = text.lines().find(|line| line.contains(needle)).unwrap_or_else(|| panic!("no {needle:?} in:\n{text}"));
    line.len() - line.trim_start_matches(' ').len()
}

/// Three wiring tests per loader: its control moves the page as the
/// equivalent setter does; a malformed file and a semantic failure each stop
/// the run with a record in the file (at `$line` for the semantic one), no
/// output, and no artifact.
macro_rules! loader_wiring {
    ($($module:ident: $file:literal, control = $control:literal, malformed = $malformed:literal, semantic = $semantic:literal, line = $line:literal;)+) => {
        $(
            mod $module {
                use super::*;

                #[test]
                fn control_reaches_the_page_like_its_setter() {
                    let fixture = process(concat!("style-matrix-", stringify!($module), "-control"));
                    fixture.write_file(concat!("cwd/", $file), $control);
                    let from_file = run(&fixture, &["doc.md", "--style", $file, "--output", "plain", "--width", "60"]);
                    let from_setter = run(&fixture, &["doc.md", "style.page.margin-left=7", "--output", "plain", "--width", "60"]);
                    assert_eq!(from_file.status.code(), Some(0), "{}", stderr(&from_file));
                    assert_eq!(stderr(&from_file), "");
                    assert_eq!(indent_of(&stdout(&from_file), "Some body text."), indent_of(&stdout(&from_setter), "Some body text."));
                    assert_eq!(stdout(&from_file), stdout(&from_setter));
                }

                #[test]
                fn malformed_stops_every_output() {
                    let fixture = process(concat!("style-matrix-", stringify!($module), "-malformed"));
                    fixture.write_file(concat!("cwd/", $file), $malformed);
                    let value = envelope(&run(&fixture, &["doc.md", "--style", $file, "--output", "json"]), 1);
                    assert_eq!(value["outcome"], "failure", "{value:#}");
                    let errors = error_records(&value);
    let [record] = errors.as_slice() else { panic!("one error: {value:#}") };
                    assert_eq!(record["code"], "dm.data.malformed", "{record:#}");
                    assert!(record["source"]["file"].as_str().is_some_and(|file| file.ends_with($file)), "{record:#}");
                    assert!(record["source"]["line"].is_number(), "{record:#}");
                    assert_formatted_failure(&run(&fixture, &["doc.md", "--style", $file, "--output", "html", "--show"]), $file);
                }

                #[test]
                fn semantic_failure_names_the_file_and_key_line() {
                    let fixture = process(concat!("style-matrix-", stringify!($module), "-semantic"));
                    fixture.write_file(concat!("cwd/", $file), $semantic);
                    let value = envelope(&run(&fixture, &["render", "doc.md", "--style", $file, "--output", "json"]), 1);
                    assert_eq!(value["outcome"], "failure", "{value:#}");
                    assert!(value["document"].is_null(), "{value:#}");
                    let errors = error_records(&value);
    let [record] = errors.as_slice() else { panic!("one error: {value:#}") };
                    assert_eq!(record["code"], "dm.schema.constraint", "{record:#}");
                    assert!(record["message"].as_str().is_some_and(|message| message.contains("`style.page.max-width`")), "{record:#}");
                    assert!(record["source"]["file"].as_str().is_some_and(|file| file.ends_with($file)), "{record:#}");
                    assert_eq!(record["source"]["line"], $line, "{record:#}");
                    assert_formatted_failure(&run(&fixture, &["doc.md", "--style", $file, "--output", "plain", "--show"]), $file);
                }
            }
        )+
    };
}

loader_wiring! {
    json_loader: "style.json",
        control = "{\"page\": {\"margin-left\": 7}}",
        malformed = "{\"page\": {\"margin-left\": 7}} x",
        semantic = "{\n  \"page\": {\n    \"max-width\": \"abc\"\n  }\n}\n",
        line = 3;
    json5_loader: "style.json5",
        control = "// left\n{ page: { 'margin-left': 7, }, }\n",
        malformed = "{ page: { 'margin-left': 7 } } x",
        semantic = "{\n  page: {\n    'max-width': 'abc',\n  },\n}\n",
        line = 3;
    yaml_loader: "style.yaml",
        control = "page:\n  margin-left: 7\n",
        malformed = "page:\n  margin-left: [\n",
        semantic = "page:\n  max-width: abc\n",
        line = 2;
    toml_loader: "style.toml",
        control = "[page]\nmargin-left = 7\n",
        malformed = "[page]\nmargin-left = 7 x\n",
        semantic = "[page]\nmax-width = \"abc\"\n",
        line = 2;
}

#[test]
fn css_from_an_earlier_file_survives_a_later_unrelated_file() {
    let fixture = process("style-matrix-css-earlier");
    fixture.write_file("cwd/one/style.yaml", "page:\n  stylesheet: ./theme.css\n");
    fixture.write_file("cwd/one/theme.css", ".from-one-dir { color: red; }\n");
    fixture.write_file("cwd/two/style.yaml", "margin: 1\n");
    fixture.write_file("cwd/two/theme.css", ".from-two-dir { color: blue; }\n");
    fixture.write_file("cwd/theme.css", ".from-launch-dir { color: blue; }\n");
    let output = run(&fixture, &["doc.md", "--style", "one/style.yaml", "--style", "two/style.yaml", "--output", "html"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let html = stdout(&output);
    assert!(html.contains(".from-one-dir"), "{html}");
    assert!(!html.contains(".from-two-dir") && !html.contains(".from-launch-dir"), "{html}");
}

#[test]
fn stdin_markdown_takes_local_style_files() {
    let fixture = CliProcessFixture::named("style-matrix-stdin");
    fixture.write_file("cwd/styles/style.yaml", "page:\n  margin-left: 7\n  stylesheet: ./theme.css\n");
    fixture.write_file("cwd/styles/theme.css", ".beside-the-style-file { color: red; }\n");
    let markdown = "# From stdin\n\nSome body text.\n";
    let styled = run_stdin(&fixture, &["--style", "styles/style.yaml", "--output", "plain", "--width", "60"], markdown);
    let setter = run_stdin(&fixture, &["style.page.margin-left=7", "--output", "plain", "--width", "60"], markdown);
    assert_eq!(styled.status.code(), Some(0), "{}", stderr(&styled));
    assert_eq!(stdout(&styled), stdout(&setter));
    let html = run_stdin(&fixture, &["render", "-", "--style", "styles/style.yaml", "--output", "html"], markdown);
    assert_eq!(html.status.code(), Some(0), "{}", stderr(&html));
    assert!(stdout(&html).contains(".beside-the-style-file"), "{}", stdout(&html));
}

/// A stylesheet resolves where its winning field was written: beside the
/// document, in the launch directory for a setter, beside a style file.
/// Each run has a decoy of the same name in every other place.
#[test]
fn document_launch_and_file_stylesheets_resolve_in_their_own_contexts() {
    let fixture = CliProcessFixture::named("style-matrix-css-origins");
    fixture.write_file("cwd/docs/doc.md", "---\nstyle:\n  page:\n    stylesheet: ./theme.css\n---\n# Doc\n");
    fixture.write_file("cwd/docs/theme.css", ".document-dir { color: red; }\n");
    fixture.write_file("cwd/theme.css", ".launch-dir { color: red; }\n");
    fixture.write_file("cwd/styles/style.yaml", "page:\n  stylesheet: ./theme.css\n");
    fixture.write_file("cwd/styles/theme.css", ".style-file-dir { color: red; }\n");
    let html = |args: &[&str]| {
        let output = run(&fixture, &[&["docs/doc.md", "--output", "html"][..], args].concat());
        assert_eq!(output.status.code(), Some(0), "{args:?}: {}", stderr(&output));
        let html = stdout(&output);
        [".document-dir", ".launch-dir", ".style-file-dir"].into_iter().filter(|marker| html.contains(marker)).collect::<Vec<_>>()
    };
    assert_eq!(html(&[]), [".document-dir"]);
    assert_eq!(html(&["style.page.stylesheet=./theme.css"]), [".launch-dir"]);
    assert_eq!(html(&["--style", "styles/style.yaml"]), [".style-file-dir"]);
}

/// A missing stylesheet is the sentinel: a target that read it would fail.
#[test]
fn source_data_and_terminal_targets_never_read_css() {
    let fixture = process("style-matrix-no-css-read");
    for target in ["terminal", "plain", "markdown", "json"] {
        let output = run(&fixture, &["doc.md", "style.page.stylesheet=./missing.css", "--output", target]);
        assert_eq!(output.status.code(), Some(0), "{target}: {} {}", stdout(&output), stderr(&output));
        assert!(!stderr(&output).contains("missing.css"), "{target}: {}", stderr(&output));
    }
}

#[test]
fn html_and_markdown_plus_read_local_css() {
    let fixture = process("style-matrix-css-read");
    fixture.write_file("cwd/theme.css", ".read-for-the-page { color: red; }\n");
    for target in ["html", "markdown-plus"] {
        let output = run(&fixture, &["doc.md", "style.page.stylesheet=./theme.css", "--output", target]);
        assert_eq!(output.status.code(), Some(0), "{target}: {}", stderr(&output));
        // HTML inlines the sheet; MarkdownPlus prints Markdown, and the
        // missing sheet below proves it reads the file all the same.
        if target == "html" {
            assert!(stdout(&output).contains(".read-for-the-page"), "{target}: {}", stdout(&output));
        }
        let missing = run(&fixture, &["doc.md", "style.page.stylesheet=./missing.css", "--output", target, "--show"]);
        assert_formatted_failure(&missing, "missing.css");
    }
}

#[test]
fn a_remote_stylesheet_is_emitted_as_a_link() {
    let fixture = process("style-matrix-css-link");
    let output = run(&fixture, &["doc.md", "style.page.stylesheet=https://example.com/theme.css", "--output", "html"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let html = stdout(&output);
    assert!(html.contains("<link") && html.contains("https://example.com/theme.css"), "{html}");
}

/// A malformed style stops the run before its stylesheet (missing) is read:
/// the only error is the malformed file's.
#[test]
fn a_malformed_style_reads_no_css_and_opens_nothing() {
    let fixture = process("style-matrix-malformed-no-css");
    fixture.write_file("cwd/bad.yaml", "page: [\n");
    let args = ["doc.md", "--style", "bad.yaml", "style.page.stylesheet=./missing.css"];
    let value = envelope(&run(&fixture, &[&args[..], &["--output", "json"]].concat()), 1);
    let codes: Vec<&Value> = error_records(&value).iter().map(|record| &record["code"]).collect();
    assert_eq!(codes, [&json!("dm.data.malformed")], "{value:#}");
    let html = run(&fixture, &[&args[..], &["--output", "html", "--show"]].concat());
    assert_formatted_failure(&html, "bad.yaml");
    assert!(!stderr(&html).contains("missing.css"), "{}", stderr(&html));
}

/// A remote `--style` reference is refused before any request.
#[test]
fn a_remote_style_file_makes_no_request() {
    let server = mock_http_server(vec![MockHttpResponse { status: 200, body: "margin: 2\n", cache_control: None }]);
    let fixture = process("style-matrix-remote-style");
    let url = server.url("/style.yaml");
    let value = envelope(&run(&fixture, &["doc.md", "--style", &url, "--output", "json"]), 1);
    let errors = error_records(&value);
    let [record] = errors.as_slice() else { panic!("one error: {value:#}") };
    assert_eq!(record["code"], "dm.input.read_failure", "{record:#}");
    assert_eq!(record["failure"], "unsupported-remote", "{record:#}");
    assert_formatted_failure(&run(&fixture, &["doc.md", "--style", &url, "--output", "html", "--show"]), "style.yaml");
    assert_eq!(server.request_count(), 0, "{:?}", server.requests());
}

#[test]
fn denying_style_blocks_a_style_file_unknown_key() {
    let fixture = process("style-matrix-deny-style");
    fixture.write_file("cwd/style.yaml", "margin: 2\nbogus: 1\n");
    let allowed = envelope(&run(&fixture, &["doc.md", "--style", "style.yaml", "--output", "json"]), 0);
    assert_eq!(allowed["diagnostics"][0]["severity"], "warning", "{allowed:#}");
    let denied = envelope(&run(&fixture, &["doc.md", "--style", "style.yaml", "--deny", "style", "--output", "json"]), 1);
    let errors = error_records(&denied);
    let [record] = errors.as_slice() else { panic!("one error: {denied:#}") };
    assert_eq!(record["code"], "dm.style.unknown_key", "{record:#}");
    assert!(record["source"]["file"].as_str().is_some_and(|file| file.ends_with("style.yaml")), "{record:#}");
    assert_eq!(record["source"]["line"], 2, "{record:#}");
    assert_formatted_failure(&run(&fixture, &["doc.md", "--style", "style.yaml", "--deny", "style", "--output", "html", "--show"]), "bogus");
}

/// Expression and shell text in an admitted style value is a plain string:
/// nothing runs, whether the run succeeds or fails.
#[test]
fn no_expression_or_shell_runs_from_style_input() {
    let fixture = CliProcessFixture::named("style-matrix-no-exec");
    fixture.write_file("cwd/doc.md", "---\n$schema:\n  style:\n    foo: string\n    \"<string>\": unknown\n---\n# Title\n");
    let sentinel = |name: &str| biscuit_file::to_portable_string(&fixture.cwd().join(name));
    let (file_shell, file_expression, setter_shell, setter_expression) =
        (sentinel("file-shell"), sentinel("file-expression"), sentinel("setter-shell"), sentinel("setter-expression"));
    fixture.write_file(
        "cwd/style.yaml",
        &format!("foo: \"$(touch {file_shell})\"\nbar: '{{{{ shell(\"touch {file_expression}\") }}}}'\nmargin: 1\n"),
    );
    let setters = [format!("style.foo=$(touch {setter_shell})"), format!("style.bar={{{{ shell(\"touch {setter_expression}\") }}}}")];
    let mut args = vec!["doc.md", "--style", "style.yaml", setters[0].as_str(), setters[1].as_str()];
    for target in ["plain", "html", "json"] {
        let output = run(&fixture, &[&args[..], &["--output", target]].concat());
        assert_eq!(output.status.code(), Some(0), "{target}: {} {}", stdout(&output), stderr(&output));
    }
    // The same text in a rendering field fails as an invalid value.
    args.push("style.page.color={{ shell(\"touch never\") }}");
    let failed = run(&fixture, &[&args[..], &["--output", "plain"]].concat());
    assert_eq!(failed.status.code(), Some(1), "{}", stderr(&failed));
    for name in ["file-shell", "file-expression", "setter-shell", "setter-expression", "never"] {
        assert!(!fixture.cwd().join(name).exists(), "{name} was created: a style value ran");
    }
}

#[test]
fn the_output_width_still_bounds_a_valid_cap() {
    let fixture = CliProcessFixture::named("style-matrix-cap");
    fixture.write_file("cwd/doc.md", &format!("# Title\n\n{}\n", "word ".repeat(60)));
    let output = run(&fixture, &["doc.md", "style.max-width=200", "--width", "40", "--output", "plain"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.lines().filter(|line| line.contains("word")).count() > 1, "{text}");
    assert!(text.lines().all(|line| line.chars().count() <= 40), "{text}");
}
