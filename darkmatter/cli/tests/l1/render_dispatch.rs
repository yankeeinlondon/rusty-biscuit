//! Render dispatch through the real binary: `md` and `md render` share one
//! preparation and handler, the CLI style layers (`--style` files and
//! `style…=VALUE` setters) reach the page over the document's own style, and
//! every style failure stops the run before anything is printed or opened.
//!
//! Every process runs with `MD_DRY_RUN` set, so `--show` never launches an
//! application: `fail` makes an attempted launch fail loudly instead.

use std::process::Output;

use crate::common::CliProcessFixture;
use serde_json::Value;

const DOCUMENT: &str = "# Title\n\nSome body text.\n";

fn fixture(name: &str, document: &str) -> CliProcessFixture {
    let fixture = CliProcessFixture::named(name);
    fixture.write_file("cwd/doc.md", document);
    fixture
}

fn command(fixture: &CliProcessFixture) -> assert_cmd::Command {
    fixture.command_builder().application_input("MD_DRY_RUN", "fail").build()
}

fn run(fixture: &CliProcessFixture, args: &[&str]) -> Output {
    command(fixture).args(args).output().expect("md runs")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// Stdout as the one JSON value a data output prints, with stderr empty.
fn one_value(output: &Output) -> Value {
    assert!(output.stderr.is_empty(), "a data output owns stderr: {}", stderr(output));
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| panic!("{error}: {}", stdout(output)))
}

/// The number of leading spaces on the line holding `needle`.
fn indent_of(text: &str, needle: &str) -> usize {
    let line = text.lines().find(|line| line.contains(needle)).unwrap_or_else(|| panic!("no {needle:?} in:\n{text}"));
    line.len() - line.trim_start_matches(' ').len()
}

fn records(value: &Value) -> &Vec<Value> {
    value["diagnostics"].as_array().unwrap_or_else(|| panic!("no diagnostics: {value:#}"))
}

/// `md` and `md render` with a document warning, a style file, and a setter
/// print the same bytes, stderr, and status for `target`.
fn assert_both_spellings_agree(target: &str) {
    let fixture = fixture(&format!("dispatch-parity-{target}"), "---\nstyle:\n  page:\n    bogus: 1\n---\n# Title\n\nSome body text.\n");
    fixture.write_file("cwd/style.yaml", "page:\n  margin-left: 2\n");
    let tail = ["doc.md", "--style", "style.yaml", "style.page.margin-top=1", "--output", target, "--width", "60"];
    let implicit = run(&fixture, &tail);
    let explicit = run(&fixture, &[&["render"][..], &tail[..]].concat());
    assert_eq!(implicit.status.code(), Some(0), "{target}: {}", stderr(&implicit));
    assert_eq!(implicit.status.code(), explicit.status.code(), "{target}");
    assert_eq!(stdout(&implicit), stdout(&explicit), "{target}");
    assert_eq!(stderr(&implicit), stderr(&explicit), "{target}");
}

/// One test per target, so the spawns spread across cores.
macro_rules! spelling_parity {
    ($($name:ident => $target:literal;)+) => {
        $(
            #[test]
            fn $name() {
                assert_both_spellings_agree($target);
            }
        )+

        #[test]
        fn spelling_parity_covers_every_target() {
            let mut covered = vec![$($target),+];
            let mut all: Vec<&str> = biscuit_terminal::output::OutputTarget::ALL.iter().map(|target| target.as_str()).collect();
            covered.sort_unstable();
            all.sort_unstable();
            assert_eq!(covered, all);
        }
    };
}

spelling_parity! {
    both_spellings_agree_for_terminal => "terminal";
    both_spellings_agree_for_plain => "plain";
    both_spellings_agree_for_html => "html";
    both_spellings_agree_for_markdown => "markdown";
    both_spellings_agree_for_markdown_plus => "markdown-plus";
    both_spellings_agree_for_json => "json";
    both_spellings_agree_for_json5 => "json5";
    both_spellings_agree_for_yaml => "yaml";
    both_spellings_agree_for_toml => "toml";
}

#[test]
fn a_setter_and_a_style_file_move_the_page_and_the_later_layer_wins() {
    let fixture = fixture("dispatch-layers", "---\nstyle:\n  page:\n    margin-left: 3\n---\n# Title\n\nSome body text.\n");
    fixture.write_file("cwd/style.yaml", "page:\n  margin-left: 5\n");
    let plain = |extra: &[&str]| {
        let output = run(&fixture, &[&["doc.md", "--output", "plain", "--width", "60"][..], extra].concat());
        assert_eq!(output.status.code(), Some(0), "{extra:?}: {}", stderr(&output));
        indent_of(&stdout(&output), "Some body text.")
    };
    let document = plain(&[]);
    // Each layer adds its own left margin over the same document.
    assert_eq!(plain(&["--style", "style.yaml"]), document + 2, "a file outranks the document");
    assert_eq!(plain(&["style.page.margin-left=1"]), document - 2, "a setter outranks the document");
    assert_eq!(plain(&["--style", "style.yaml", "style.ml=1"]), document - 2, "a setter outranks a file");
    assert_eq!(plain(&["style.ml=1", "--style", "style.yaml"]), document - 2, "order on the line does not matter");
}

#[test]
fn a_bad_setter_fails_every_target_before_anything_is_printed_or_opened() {
    let fixture = fixture("dispatch-bad-setter", DOCUMENT);
    for target in ["terminal", "plain", "html", "markdown", "markdown-plus"] {
        let output = run(&fixture, &["doc.md", "style.page.margin-top=abc", "--output", target, "--show"]);
        assert_eq!(output.status.code(), Some(1), "{target}: {}", stderr(&output));
        assert!(output.stdout.is_empty(), "{target}: {}", stdout(&output));
        assert!(!stderr(&output).contains("no default application"), "{target} opened an artifact: {}", stderr(&output));
    }
    for target in ["json", "yaml"] {
        let output = run(&fixture, &["render", "doc.md", "style.page.margin-top=abc", "--output", target, "--show"]);
        assert_eq!(output.status.code(), Some(1), "{target}: {}", stdout(&output));
        assert!(output.stderr.is_empty(), "{target}: {}", stderr(&output));
    }
    let output = run(&fixture, &["doc.md", "style.page.margin-top=abc", "--output", "json"]);
    let value = one_value(&output);
    assert_eq!(value["outcome"], "failure");
    assert!(value["document"].is_null(), "{value:#}");
    let [record] = records(&value).as_slice() else { panic!("one record: {value:#}") };
    assert_eq!(record["code"], "dm.schema.type_mismatch", "{record:#}");
    // A setter is in no file: the document's path is never stamped on it.
    assert!(record["source"].is_null(), "{record:#}");
}

#[test]
fn a_style_file_failure_names_the_style_file_never_the_document() {
    let fixture = fixture("dispatch-file-failure", DOCUMENT);
    let missing = one_value(&run(&fixture, &["doc.md", "--style", "missing.yaml", "--output", "json"]));
    assert_eq!(missing["outcome"], "failure");
    let record = &records(&missing)[0];
    assert_eq!(record["code"], "dm.input.read_failure", "{record:#}");
    assert_eq!(record["failure"], "no-match", "{record:#}");
    assert!(record["source"].is_null(), "{record:#}");

    fixture.write_file("cwd/bad.yaml", "page:\n  margin-left: [\n");
    let malformed = one_value(&run(&fixture, &["doc.md", "--style", "bad.yaml", "--output", "json"]));
    let record = &records(&malformed)[0];
    assert_eq!(record["code"], "dm.data.malformed", "{record:#}");
    assert!(record["source"]["file"].as_str().is_some_and(|file| file.ends_with("bad.yaml")), "{record:#}");
}

#[test]
fn a_style_file_warning_is_reported_on_every_target_and_names_its_file() {
    let fixture = fixture("dispatch-file-warning", DOCUMENT);
    fixture.write_file("cwd/style.yaml", "page:\n  margin-left: 2\n  bogus: 1\n");
    let value = one_value(&run(&fixture, &["doc.md", "--style", "style.yaml", "--output", "json"]));
    assert_eq!(value["outcome"], "success");
    let [record] = records(&value).as_slice() else { panic!("one record: {value:#}") };
    assert_eq!(record["code"], "dm.style.unknown_key", "{record:#}");
    assert!(record["source"]["file"].as_str().is_some_and(|file| file.ends_with("style.yaml")), "{record:#}");
    assert_eq!(record["source"]["line"], 3, "{record:#}");

    // A source output builds no page, but a CLI layer's warning still shows.
    let markdown = run(&fixture, &["doc.md", "--style", "style.yaml", "--output", "markdown"]);
    assert_eq!(markdown.status.code(), Some(0), "{}", stderr(&markdown));
    assert!(stderr(&markdown).contains("bogus"), "{}", stderr(&markdown));
}

#[test]
fn a_formatted_source_output_shows_no_warning_of_the_document_style() {
    let fixture = fixture("dispatch-source-warning", "---\nstyle:\n  page:\n    bogus: 1\n---\n# Title\n");
    let markdown = run(&fixture, &["doc.md", "--output", "markdown"]);
    assert_eq!(markdown.status.code(), Some(0), "{}", stderr(&markdown));
    assert_eq!(stderr(&markdown), "");
    let plain = run(&fixture, &["doc.md", "--output", "plain"]);
    assert!(stderr(&plain).contains("unknown style key `style.page.bogus`"), "{}", stderr(&plain));
}

#[test]
fn a_source_output_carries_no_cli_style_and_ignores_a_broken_document_style() {
    let source = "---\ntitle: Notes\nstyle:\n  page:\n    margin-left: -4\n---\n# Title\n";
    let fixture = fixture("dispatch-source", source);
    let markdown = run(&fixture, &["doc.md", "style.page.margin-left=4", "--output", "markdown"]);
    assert_eq!(markdown.status.code(), Some(0), "{}", stderr(&markdown));
    assert_eq!(stdout(&markdown), source);
    let value = one_value(&run(&fixture, &["doc.md", "style.page.margin-left=4", "--output", "json"]));
    assert_eq!(value["outcome"], "success", "{value:#}");
    assert_eq!(value["document"].as_str().map(str::trim_end), Some(source.trim_end()), "{value:#}");
    // A page reads the document's style, so there the bad value fails.
    let plain = run(&fixture, &["doc.md", "--output", "plain"]);
    assert_eq!(plain.status.code(), Some(1), "{}", stdout(&plain));
}

#[test]
fn denying_style_blocks_an_inline_rule_style_and_a_warning_code_blocks_on_its_own() {
    let fixture = fixture("dispatch-deny", "# Doc\n\n--- { style: waves }\n");
    let allowed = one_value(&run(&fixture, &["doc.md", "--output", "json"]));
    assert_eq!(allowed["outcome"], "success", "{allowed:#}");
    for deny in ["style", "dm.style.deprecated"] {
        let denied = one_value(&run(&fixture, &["doc.md", "--deny", deny, "--output", "json"]));
        assert_eq!(denied["outcome"], "failure", "{deny}: {denied:#}");
        let record = &records(&denied)[0];
        assert_eq!(record["code"], "dm.style.deprecated", "{deny}: {record:#}");
        assert_eq!(record["severity"], "error", "{deny}: {record:#}");
    }
}

#[test]
fn render_reports_an_invalid_trigger_its_schema_selection_skipped() {
    let fixture = fixture("dispatch-trigger", DOCUMENT);
    fixture.write_file("cwd/.git/HEAD", "ref: refs/heads/main\n");
    fixture.write_file("cwd/.git/config", "[core]\n\trepositoryformatversion = 0\n");
    std::fs::create_dir_all(fixture.cwd().join(".git/objects")).unwrap();
    std::fs::create_dir_all(fixture.cwd().join(".git/refs")).unwrap();
    fixture.write_file("cwd/schemas/broken.trigger.yaml", "kind: schema-trigger\n");

    let value = one_value(&run(&fixture, &["doc.md", "--output", "json"]));
    assert_eq!(value["outcome"], "success", "{value:#}");
    assert!(records(&value).iter().any(|record| record["code"] == "dm.schema.invalid_trigger"), "{value:#}");
    let plain = run(&fixture, &["render", "doc.md", "--output", "plain"]);
    assert_eq!(plain.status.code(), Some(0), "{}", stderr(&plain));
    assert!(stderr(&plain).contains("broken.trigger.yaml"), "{}", stderr(&plain));
}

#[test]
fn a_misspelled_setter_suggests_the_style_spelling() {
    let fixture = fixture("dispatch-setter-hint", DOCUMENT);
    let output = run(&fixture, &["stlye.page.margin-left=4", "--output", "plain"]);
    assert_eq!(output.status.code(), Some(1), "{}", stdout(&output));
    assert!(stderr(&output).contains("style.page.margin-left=4"), "{}", stderr(&output));
    let value = one_value(&run(&fixture, &["render", "stlye.page.margin-left=4", "--output", "json"]));
    assert!(records(&value)[0]["message"].as_str().unwrap().contains("style.page.margin-left=4"), "{value:#}");
}

#[test]
fn redirected_stdin_renders_in_both_spellings_even_when_empty() {
    let fixture = CliProcessFixture::named("dispatch-stdin");
    for input in ["", "# From stdin\n"] {
        let render = |args: &[&str]| {
            command(&fixture).args(args).write_stdin(input).output().expect("md runs")
        };
        let implicit = render(&["--output", "plain", "style.page.margin-left=2"]);
        let explicit = render(&["render", "--output", "plain", "style.page.margin-left=2"]);
        let dash = render(&["render", "-", "--output", "plain", "style.page.margin-left=2"]);
        assert_eq!(implicit.status.code(), Some(0), "{input:?}: {}", stderr(&implicit));
        assert_eq!(stdout(&implicit), stdout(&explicit), "{input:?}");
        assert_eq!(stdout(&implicit), stdout(&dash), "{input:?}");
    }
}

#[cfg(unix)]
#[test]
fn a_non_unicode_input_is_an_invalid_reference_not_a_lossy_file() {
    use std::os::unix::ffi::OsStrExt;
    let fixture = CliProcessFixture::named("dispatch-non-unicode");
    // The lossy spelling of the argument exists; it must not be opened.
    fixture.write_file("cwd/doc\u{FFFD}.md", "# Wrong file\n");
    let output = fixture
        .command()
        .arg(std::ffi::OsStr::from_bytes(b"doc\xFF.md"))
        .args(["--output", "json"])
        .output()
        .expect("md runs");
    assert_eq!(output.status.code(), Some(1));
    let value = one_value(&output);
    let record = &records(&value)[0];
    assert_eq!(record["code"], "dm.input.read_failure", "{record:#}");
    assert_eq!(record["failure"], "invalid-reference", "{record:#}");
}

/// No input with a terminal on stdin, decided in process from the captured
/// stream state: stdin is never read.
mod no_input {
    use clap::Parser as _;
    use darkmatter::markdown::compose::RequestSnapshot;
    use darkmatter_cli::Cli;
    use darkmatter_cli::args::{Command, RenderOptions};
    use darkmatter_cli::commands::{ReportedExit, dispatch_render};
    use darkmatter_cli::output::OutputEnvironment;
    use darkmatter_cli::request::MdRequest;

    fn exit_of(args: &[&str]) -> i32 {
        let cli = Cli::try_parse_from(args).expect("parses");
        let options: RenderOptions = match cli.command_or_render() {
            Command::Render(options) => options,
            other => panic!("expected render, got {other:?}"),
        };
        let dir = tempfile::tempdir().expect("temp dir");
        let request = MdRequest::new(RequestSnapshot::new(dir.path()).with_home(Some(dir.path().to_path_buf())));
        let terminal = OutputEnvironment { stdin_is_terminal: true, ..OutputEnvironment::default() };
        let error = dispatch_render(&options, &cli, &request, &terminal).expect_err("nothing to render");
        error.downcast_ref::<ReportedExit>().unwrap_or_else(|| panic!("a reported exit: {error:#}")).code()
    }

    #[test]
    fn both_spellings_fail_with_status_one_for_every_family() {
        for args in [
            &["md"][..],
            &["md", "render"],
            &["md", "--output", "plain"],
            &["md", "render", "--output", "json"],
            &["md", "style.page.margin-left=2", "--output", "yaml"],
        ] {
            assert_eq!(exit_of(args), 1, "{args:?}");
        }
    }
}
