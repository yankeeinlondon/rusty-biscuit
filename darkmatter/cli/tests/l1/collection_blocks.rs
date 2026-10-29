//! `::map` and `::filter` through the real `md compose` process.
//!
//! A small suite: a representative success of each directive, a typed
//! failure with its authored location and iteration path, preapproval of a
//! command in a candidate the run skips, budget exhaustion, and template
//! shell arguments inside a map. The shape cross product is covered
//! in-process by the library suite, not here.
//!
//! Shell evidence is the argument vector the `darkmatter-argv-recorder`
//! fixture received, copied into the fixture `bin` as `argv-recorder`.

use crate::common::{CliProcessFixture, unwrapped};
use darkmatter::markdown::compose::shell_expansion::template::{TemplateShape, parse_authored};
use serde_json::Value;
use std::path::PathBuf;
use test_toolkit::argv_recorder::{ARGV_RECORDER_FILE, read_argv_records};

/// A fixture with the recorder on its `PATH` and a file it records into.
pub(crate) struct Recorder {
    pub(crate) fixture: CliProcessFixture,
    records: PathBuf,
}

impl Recorder {
    pub(crate) fn new(name: &str) -> Self {
        let fixture = CliProcessFixture::named(name);
        let program = fixture.bin_dir().join(format!("argv-recorder{}", std::env::consts::EXE_SUFFIX));
        std::fs::copy(biscuit_test_harness::bin_exe!("darkmatter-argv-recorder"), &program)
            .expect("the recorder fixture binary is built with `test-fixtures`");
        let records = fixture.tmp_dir().join("argv.bin");
        Self { fixture, records }
    }

    /// Writes `doc.md` under the fixture `cwd`.
    pub(crate) fn document(&self, content: &str) -> PathBuf {
        self.fixture.write_file("cwd/doc.md", content)
    }

    pub(crate) fn whitelist(&self, rules: &str) {
        self.fixture.write_file("cwd/.darkmatter-shell-whitelist", rules);
    }

    /// `md compose doc.md` with `args` after the document.
    pub(crate) fn compose(&self, args: &[&str]) -> std::process::Output {
        self.fixture
            .command()
            .env(ARGV_RECORDER_FILE, &self.records)
            .arg("compose")
            .arg(self.fixture.cwd().join("doc.md"))
            .args(args)
            .output()
            .unwrap()
    }

    /// Every argument vector the recorder received; empty when it never ran.
    pub(crate) fn runs(&self) -> Vec<Vec<String>> {
        match read_argv_records(&self.records) {
            Ok(runs) => runs,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(error) => panic!("unreadable recorder file: {error}"),
        }
    }
}

/// The `template` rule that approves `command`'s shape.
pub(crate) fn template_rule(command: &str) -> String {
    let template = parse_authored(command).expect("a valid template");
    format!("template {}\n", TemplateShape::from_template(&template).canonical_text())
}

pub(crate) fn stdout_of(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

pub(crate) fn stderr_of(output: &std::process::Output) -> String {
    unwrapped(&String::from_utf8_lossy(&output.stderr))
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

const STRINGS_HEADER: &str = "---\n$schema:\n  data: string[](required)\n";

/// Three strings under a declared `string[]` schema.
fn strings_document(data: &str, body: &str) -> String {
    format!("{STRINGS_HEADER}data: {data}\n---\n{body}")
}

/// A typed map whose callback fails on its first item (line 8).
const FAILING_MAP: &str = "---
$schema:
  data: \"{ name: string(required) }[](required)\"
data:
  - { name: a }
  - { name: b }
---
::map data \"p => p.name * 2\"
- {{p}}
::end-map
";

#[test]
fn a_map_composes_to_markdown() {
    let recorder = Recorder::new("collection_blocks_map");
    recorder.document(&strings_document("[foo, bar, baz]", "::map data \"p => p\"\n- {{p}}\n::end-map\n"));

    let output = recorder.compose(&[]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr_of(&output));
    assert_eq!(stdout_of(&output).trim_end(), "- foo\n- bar\n- baz");
    assert!(recorder.runs().is_empty());
}

#[test]
fn a_filter_composes_the_retained_original_items() {
    let recorder = Recorder::new("collection_blocks_filter");
    recorder.document(
        "---
$schema:
  tasks: \"{ title: string(required), done: boolean(required) }[](required)\"
tasks:
  - { title: Write, done: false }
  - { title: Ship, done: true }
  - { title: Rest, done: false }
---
::filter tasks \"task => !task.done\"
- {{task.title}}
::end-filter
",
    );

    let output = recorder.compose(&[]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr_of(&output));
    assert_eq!(stdout_of(&output).trim_end(), "- Write\n- Rest");
}

/// A typed callback failure names the directive's authored line and column
/// and the original position of the failing item, and writes no document.
#[test]
fn a_typed_failure_reports_location_and_iteration_path() {
    let recorder = Recorder::new("collection_blocks_typed_failure");
    let document = recorder.document(FAILING_MAP);

    let output = recorder.compose(&[]);
    let stderr = stderr_of(&output);

    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(output.stdout.is_empty(), "no successful document: {}", stdout_of(&output));
    assert!(stderr.contains("dm.collection.callback_failed"), "{stderr}");
    assert!(stderr.contains("doc.md"), "{stderr}");
    assert!(stderr.contains("Collection at line: 8, column: 12"), "{stderr}");
    assert!(stderr.contains("Iteration path: data[0]"), "{stderr}");
    assert!(stderr.contains("Multiplication requires numeric operands"), "the cause is kept: {stderr}");
    assert!(document.exists());
    assert!(recorder.runs().is_empty());
}

/// The report carries the terminal's code, file, line, column, and path.
#[test]
fn report_json_agrees_with_the_terminal_on_the_failure() {
    let recorder = Recorder::new("collection_blocks_report_parity");
    let document = recorder.document(FAILING_MAP);

    let terminal = stderr_of(&recorder.compose(&[]));
    let output = recorder.compose(&["--output", "report-json"]);

    let stdout = stdout_of(&output);
    let mut stream = serde_json::Deserializer::from_str(&stdout).into_iter::<Value>();
    let report = stream.next().expect("a report on stdout").expect("valid JSON");
    assert!(stream.next().is_none(), "exactly one object: {stdout}");
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(report["outcome"], "failure");
    assert_eq!(report["document"], Value::Null);
    let diagnostics = report["diagnostics"].as_array().expect("an array");
    assert_eq!(diagnostics.len(), 1, "{report:#}");
    let diagnostic = &diagnostics[0];
    let code = diagnostic["code"].as_str().unwrap();
    assert!(terminal.contains(code), "terminal and report share the code {code}: {terminal}");
    let source = &diagnostic["source"];
    assert_eq!(
        source["file"].as_str().map(PathBuf::from),
        Some(recorder.fixture.cwd().join("doc.md")),
        "the root is named as given"
    );
    assert!(terminal.contains(&format!("line: {}, column: {}", source["line"], source["column"])), "{terminal}");
    assert_eq!(source["line"], 8);
    assert_eq!(source["column"], 12);
    let message = diagnostic["message"].as_str().unwrap();
    assert!(message.ends_with("at data[0]"), "{message}");
    assert!(terminal.contains("Iteration path: data[0]"), "{terminal}");
    assert!(document.exists());
}

/// `md compose --output report-json` prints, for a failure inside a repeated
/// body, the provenance the library's record carries: the authored file,
/// line, span, and original item path. The library owns it; the CLI only
/// collects it.
macro_rules! report_parity {
    ($name:ident, $opener:literal, $closer:literal, $files:expr, $body:literal, $args:expr, $code:literal, $line:literal, $path:literal) => {
        #[test]
        fn $name() {
            let recorder = Recorder::new(stringify!($name));
            for (name, content) in $files {
                recorder.fixture.write_file(format!("cwd/{name}"), content);
            }
            let body = $body.replace("{OPEN}", $opener).replace("{CLOSE}", $closer);
            let document = recorder.document(&strings_document("[./ok.md, ./missing.md]", &body));
            let args: &[&str] = $args;

            let output = recorder.compose(&[&["--output", "report-json"], args].concat());
            let stdout = stdout_of(&output);
            let report: Value = serde_json::from_str(&stdout).unwrap_or_else(|error| panic!("{error}: {stdout}"));
            assert_eq!(output.status.code(), Some(1), "{report:#}");
            let [diagnostic] = report["diagnostics"].as_array().expect("an array").as_slice() else {
                panic!("one record: {report:#}")
            };
            assert_eq!(diagnostic["code"], $code, "{diagnostic:#}");
            assert_eq!(diagnostic["source"]["line"], $line, "{diagnostic:#}");
            assert_eq!(
                diagnostic["source"]["file"].as_str().map(PathBuf::from),
                Some(document.clone()),
                "{diagnostic:#}"
            );
            assert!(diagnostic["span"]["start"].is_u64(), "{diagnostic:#}");
            let path: Vec<String> = diagnostic["iteration_path"]
                .as_array()
                .expect("an array")
                .iter()
                .map(|frame| format!("{}[{}]", frame["collection"].as_str().unwrap(), frame["position"]))
                .collect();
            assert_eq!(path.join(" → "), $path, "{diagnostic:#}");
            assert!(diagnostic["message"].as_str().unwrap().ends_with(concat!(" at ", $path)), "{diagnostic:#}");

            // The terminal names the same item.
            let terminal = stderr_of(&recorder.compose(args));
            assert!(terminal.contains(concat!("Iteration path: ", $path)), "{terminal}");
        }
    };
}

const OK: [(&str, &str); 1] = [("ok.md", "---\ntitle: ./ok.md\n---\nok\n")];

report_parity!(
    report_locates_a_visit_exhaustion_in_a_map, "::map", "::end-map", OK,
    "{OPEN} data\nx\n{CLOSE}\n", &["--max-iteration-visits", "1"], "dm.iteration.visit_limit", 6, "data[1]"
);
report_parity!(
    report_locates_a_visit_exhaustion_in_a_filter, "::filter", "::end-filter", OK,
    "{OPEN} data\nx\n{CLOSE}\n", &["--max-iteration-visits", "1"], "dm.iteration.visit_limit", 6, "data[1]"
);
report_parity!(
    report_locates_a_repeated_interpolation_failure_in_a_map, "::map", "::end-map", OK,
    "{OPEN} data\n{{ frontmatter(i, 'title') }}\n{CLOSE}\n", &[], "dm.expression.evaluation_failure", 7, "data[1]"
);
report_parity!(
    report_locates_a_repeated_gate_failure_in_a_filter, "::filter", "::end-filter", OK,
    "{OPEN} data\n::block when=\"frontmatter(i, 'title')\"\nx\n::end-block\n{CLOSE}\n", &[],
    "dm.expression.evaluation_failure", 7, "data[1]"
);
report_parity!(
    report_locates_a_repeated_child_insertion_in_a_map, "::map", "::end-map",
    [("ok.md", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\n")],
    "{OPEN} data\n::file ./ok.md\n{CLOSE}\n", &["--max-expansion-bytes", "40"], "dm.iteration.expansion_limit", 7, "data[0]"
);

/// A failing command inside a repeated `::shell-block` is located at its own
/// line, not the block's opener: the second command, given a null argument,
/// is the one reported, and nothing runs because every command of a block is
/// prepared before the first one runs.
#[test]
fn report_locates_a_later_shell_block_command_in_each_kind() {
    for (opener, closer) in [("::map", "::end-map"), ("::filter", "::end-filter")] {
        let recorder = Recorder::new("report_locates_a_later_shell_block_command");
        recorder.whitelist(&format!("exact argv-recorder first\n{}", template_rule("argv-recorder {{ i }}")));
        let document_text = format!(
            "---\n$schema:\n  data: any[](required)\ndata: [null]\n---\n{opener} data\n::shell-block\nargv-recorder first\nargv-recorder {{{{ i }}}}\n::end-block\n{closer}\n"
        );
        let document = recorder.document(&document_text);

        let output = recorder.compose(&["--output", "report-json"]);
        let stdout = stdout_of(&output);
        let report: Value = serde_json::from_str(&stdout).unwrap_or_else(|error| panic!("{error}: {stdout}"));
        assert_eq!(output.status.code(), Some(1), "{opener}: {report:#}");
        let [diagnostic] = report["diagnostics"].as_array().expect("an array").as_slice() else {
            panic!("{opener}: one record: {report:#}")
        };
        assert_eq!(diagnostic["code"], "dm.shell.failure", "{opener}: {diagnostic:#}");
        assert_eq!(diagnostic["source"]["line"], 9, "{opener}: {diagnostic:#}");
        assert_eq!(diagnostic["source"]["column"], 1, "{opener}: {diagnostic:#}");
        let start = document_text.find("argv-recorder {{ i }}").unwrap();
        assert_eq!(diagnostic["span"]["start"], start, "{opener}: {diagnostic:#}");
        let end = diagnostic["span"]["end"].as_u64().expect("a command span") as usize;
        assert!(end > start && end <= start + "argv-recorder {{ i }}".len(), "{opener}: {diagnostic:#}");
        assert_eq!(diagnostic["source"]["file"].as_str().map(PathBuf::from), Some(document), "{opener}");
        assert_eq!(diagnostic["iteration_path"], serde_json::json!([{ "collection": "data", "position": 0 }]), "{opener}");
        assert!(recorder.runs().is_empty(), "{opener}: nothing ran: {:?}", recorder.runs());
    }
}

/// A command inside a candidate the run skips is still authorized at
/// preflight: without approval the command fails before anything runs.
macro_rules! skipped_candidate_is_denied {
    ($name:ident, $body:expr) => {
        #[test]
        fn $name() {
            let recorder = Recorder::new(stringify!($name));
            recorder.document(&strings_document("[a, b]", $body));

            let output = recorder.compose(&[]);
            let stderr = stderr_of(&output);

            assert_ne!(output.status.code(), Some(0), "{stderr}");
            assert!(output.stdout.is_empty(), "no document: {}", stdout_of(&output));
            assert!(stderr.contains("Approval required for 'argv-recorder"), "{stderr}");
            assert!(recorder.runs().is_empty(), "nothing ran: {:?}", recorder.runs());
        }
    };
}

skipped_candidate_is_denied!(
    a_command_in_a_false_map_gate_is_denied,
    "::map data when=\"false\"\n::shell argv-recorder {{ i }}\n::end-map\n"
);
skipped_candidate_is_denied!(
    a_command_in_a_filtered_out_item_is_denied,
    "::filter data \"n => false\"\n::shell argv-recorder {{ n }}\n::end-filter\n"
);

/// Exhausting either allowance fails the request with the limit's code and
/// the original position of the item that ran it out. Nothing runs, because
/// the bodies' commands follow evaluation.
macro_rules! budget_exhaustion {
    ($name:ident, $flag:literal, $limit:literal, $code:literal) => {
        #[test]
        fn $name() {
            let recorder = Recorder::new(stringify!($name));
            recorder.document(&strings_document("[a, b, c]", "::map data \"p => p\"\n::shell argv-recorder {{ p }}\n::end-map\n"));
            recorder.whitelist(&template_rule("argv-recorder {{ p }}"));

            let output = recorder.compose(&[$flag, $limit]);
            let stderr = stderr_of(&output);

            assert_eq!(output.status.code(), Some(1), "{stderr}");
            assert!(output.stdout.is_empty(), "no document: {}", stdout_of(&output));
            assert!(stderr.contains($code), "{stderr}");
            assert!(stderr.contains("Iteration path: data["), "{stderr}");
            assert!(recorder.runs().is_empty(), "nothing ran: {:?}", recorder.runs());
        }
    };
}

budget_exhaustion!(the_visit_limit_stops_the_request, "--max-iteration-visits", "2", "dm.iteration.visit_limit");
budget_exhaustion!(the_expansion_limit_stops_the_request, "--max-expansion-bytes", "1", "dm.iteration.expansion_limit");

const SHELL_BODY: &str = "::map data \"p => p\"\n::shell argv-recorder {{ p }}\n::end-map\n";

/// Values that look like shell syntax fill exactly one argument each, and
/// the same filled command is served from the request cache.
#[test]
fn map_template_arguments_stay_single_words_and_repeats_are_cached() {
    let recorder = Recorder::new("collection_blocks_shell_arguments");
    recorder.document(&strings_document(
        "[\"a b\", \"x; rm -rf ~\", \"> out.txt\", \"$(id) && `id`\", \"a b\"]",
        SHELL_BODY,
    ));
    recorder.whitelist(&template_rule("argv-recorder {{ p }}"));

    let output = recorder.compose(&[]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr_of(&output));
    assert_eq!(
        recorder.runs(),
        vec![
            strings(&["a b"]),
            strings(&["x; rm -rf ~"]),
            strings(&["> out.txt"]),
            strings(&["$(id) && `id`"]),
        ],
        "the repeated `a b` is a cache hit"
    );
    assert!(!recorder.fixture.cwd().join("out.txt").exists(), "no redirection happened");
}

#[test]
fn no_cache_launches_every_reached_occurrence() {
    let recorder = Recorder::new("collection_blocks_no_cache");
    recorder.document(&strings_document(
        "[same, same, same]",
        "::map data \"p => p\"\n::shell --no-cache argv-recorder {{ p }}\n::end-map\n",
    ));
    recorder.whitelist(&template_rule("argv-recorder {{ p }}"));

    let output = recorder.compose(&[]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr_of(&output));
    assert_eq!(recorder.runs(), vec![strings(&["same"]); 3]);
}

/// A null item is a missing argument: the command fails and never starts
/// with a placeholder value, and no later item runs.
#[test]
fn a_null_argument_fails_the_request_without_launching_it() {
    let recorder = Recorder::new("collection_blocks_null_argument");
    recorder.document(
        "---\n$schema:\n  data: \"(string | null)[](required)\"\ndata: [null]\n---\n::map data \"p => p\"\n::shell argv-recorder {{ p }}\n::end-map\n",
    );
    recorder.whitelist(&template_rule("argv-recorder {{ p }}"));

    let output = recorder.compose(&[]);
    let stderr = stderr_of(&output);

    assert_ne!(output.status.code(), Some(0), "{stderr}");
    assert!(output.stdout.is_empty(), "{}", stdout_of(&output));
    assert!(stderr.contains("missing"), "{stderr}");
    assert!(recorder.runs().is_empty());
}
