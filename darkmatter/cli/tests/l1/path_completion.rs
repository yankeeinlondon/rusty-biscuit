//! Tab completion through the real `md` process: `COMPLETE=<shell> md -- <words>`.
//!
//! These run the normal completion entry point with a fixture-owned document,
//! schema, and home, so they prove the wiring from the shell's words to the
//! candidates: the input found on the line, the schema resolved for it, and what
//! is printed. The candidates themselves are covered in the library's
//! `path_candidates` tests.

use crate::common;

use common::{CliProcessFixture, MockHttpResponse, mock_http_server};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use test_toolkit::argv_recorder::{ARGV_RECORDER_FILE, read_argv_records};
use test_toolkit::{Level, expect_level};

const DOCUMENT: &str = "---\ntitle: Hello\nauthor:\n  name: Ann\nitems: [a, b]\n---\nBody\n";

const SCHEMA: &str = "$schema:\n  lang: enum(rust, go; required)\n  mode: enum(\"two words\", \"$x\", \"a=b\", plain)\n  inc: file\n  sub:\n    key: string\n";

fn fixture(name: &str) -> CliProcessFixture {
    let fixture = CliProcessFixture::named(name);
    write_in_cwd(&fixture, "doc.md", DOCUMENT);
    write_in_cwd(&fixture, "s.yaml", SCHEMA);
    write_in_cwd(&fixture, "schemaed.md", "---\n$schema: ./s.yaml\nlang: rust\n---\n");
    fixture
}

fn write_in_cwd(fixture: &CliProcessFixture, relative: &str, content: &str) {
    fixture.write_file(fixture.cwd().join(relative), content);
}

/// Where the `argv-recorder` program writes if anything ever runs it; inside the
/// workspace, so `files_under` sees it appear.
fn recorder_file(fixture: &CliProcessFixture) -> PathBuf {
    fixture.tmp_dir().join("argv.bin")
}

/// What a Tab press prints, one candidate per line, with help removed. Asserts
/// that it succeeded and said nothing else.
fn complete_in(fixture: &CliProcessFixture, shell: &str, index: Option<usize>, words: &[&str]) -> Vec<String> {
    let mut command = fixture.command();
    command
        .env("COMPLETE", shell)
        .env(ARGV_RECORDER_FILE, recorder_file(fixture))
        .arg("--")
        .args(words)
        .write_stdin("---\nfrom_stdin: 1\n---\n");
    if let Some(index) = index {
        command.env("_CLAP_COMPLETE_INDEX", index.to_string());
    }
    let output = command.output().unwrap();
    assert!(output.status.success(), "{words:?}: {output:?}");
    assert!(output.stderr.is_empty(), "{words:?}: {}", String::from_utf8_lossy(&output.stderr));
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| line.split(['\t', ':']).next().unwrap_or_default().to_string())
        .collect()
}

fn fish(fixture: &CliProcessFixture, words: &[&str]) -> Vec<String> {
    complete_in(fixture, "fish", None, words)
}

/// Bash with the cursor on the last word, as its registered function reports it.
fn bash(fixture: &CliProcessFixture, words: &[&str]) -> Vec<String> {
    complete_in(fixture, "bash", Some(words.len() - 1), words)
}

fn holds(candidates: &[String], expected: &[&str]) {
    for candidate in expected {
        assert!(candidates.iter().any(|line| line == candidate), "`{candidate}` in {candidates:?}");
    }
}

fn lacks(candidates: &[String], unexpected: &[&str]) {
    for candidate in unexpected {
        assert!(!candidates.iter().any(|line| line == candidate), "`{candidate}` in {candidates:?}");
    }
}

fn paths_only(candidates: Vec<String>) -> Vec<String> {
    candidates.into_iter().filter(|candidate| !candidate.starts_with('-')).collect()
}

#[test]
fn compose_offers_schema_keys_document_keys_and_files() {
    let fixture = fixture("completion-compose-root");
    let candidates = fish(&fixture, &["md", "compose", "doc.md", ""]);
    holds(&candidates, &["title=", "style.", "style=", "author.", "author=", "items.", "items=", "doc.md", "schemaed.md"]);
    lacks(&candidates, &["s.yaml", "$schema="]);

    let position = |name: &str| candidates.iter().position(|candidate| candidate == name).unwrap();
    assert!(position("title=") < position("author."), "schema keys come before document keys: {candidates:?}");
    assert!(position("author.") < position("doc.md"), "keys come before files: {candidates:?}");
}

#[test]
fn compose_walks_the_schema_one_level_at_a_time() {
    let fixture = fixture("completion-compose-walk");
    holds(&fish(&fixture, &["md", "compose", "doc.md", "style."]), &["style.page.", "style.page=", "style.table."]);
    holds(&fish(&fixture, &["md", "compose", "doc.md", "style.page."]), &["style.page.alignment="]);
    assert_eq!(
        fish(&fixture, &["md", "compose", "doc.md", "style.page.alignment="]),
        ["style.page.alignment=left", "style.page.alignment=center", "style.page.alignment=right"]
    );
    assert_eq!(fish(&fixture, &["md", "compose", "doc.md", "style.page.alignment=ce"]), ["style.page.alignment=center"]);
}

#[test]
fn the_input_may_come_after_setters_and_flag_values_are_skipped() {
    let fixture = fixture("completion-compose-input");
    holds(&fish(&fixture, &["md", "compose", "x=1", "doc.md", "aut"]), &["author."]);
    holds(&fish(&fixture, &["md", "compose", "--output", "json", "doc.md", "aut"]), &["author."]);
    holds(&fish(&fixture, &["md", "compose", "--output=json", "doc.md", "aut"]), &["author."]);
    lacks(&fish(&fixture, &["md", "compose", "--state", "doc.md", "aut"]), &["author."]);
}

#[test]
fn a_word_that_is_a_path_with_equals_completes_files() {
    let fixture = fixture("completion-compose-path-word");
    write_in_cwd(&fixture, "foo=bar.md", "");
    assert_eq!(fish(&fixture, &["md", "compose", "./foo="]), ["foo=bar.md"]);
    assert_eq!(fish(&fixture, &["md", "compose", "=x"]), Vec::<String>::new());
}

#[test]
fn no_input_offers_only_the_baseline_and_files() {
    let fixture = fixture("completion-compose-none");
    let candidates = fish(&fixture, &["md", "compose", ""]);
    holds(&candidates, &["title=", "style.", "doc.md"]);
    lacks(&candidates, &["author.", "lang="]);
}

#[test]
fn get_set_and_rm_offer_their_own_keys() {
    let fixture = fixture("completion-path-commands");
    let get = paths_only(fish(&fixture, &["md", "get", "doc.md", ""]));
    assert_eq!(get[..5], ["title", "author.", "author", "items.", "items"], "present keys first: {get:?}");
    holds(&get, &["style."]);

    let rm = paths_only(fish(&fixture, &["md", "rm", "doc.md", ""]));
    assert_eq!(rm, ["title", "author.", "author", "items.", "items"], "only keys present");
    assert_eq!(fish(&fixture, &["md", "rm", "doc.md", "author."]), ["author.name"]);

    let set = paths_only(fish(&fixture, &["md", "set", "doc.md", ""]));
    holds(&set, &["title", "style.", "author.", "author"]);
    lacks(&set, &["title="]);
}

#[test]
fn set_offers_values_as_its_own_argument() {
    let fixture = fixture("completion-set-value");
    assert_eq!(paths_only(fish(&fixture, &["md", "set", "doc.md", "style.page.alignment", ""])), ["left", "center", "right"]);
    assert_eq!(fish(&fixture, &["md", "set", "doc.md", "style.page.alignment", "ce"]), ["center"]);
    assert_eq!(paths_only(fish(&fixture, &["md", "set", "doc.md", "a..b", ""])), Vec::<String>::new());
}

#[test]
fn paths_already_given_are_not_offered_again() {
    let fixture = fixture("completion-earlier-paths");
    lacks(&fish(&fixture, &["md", "get", "doc.md", "title", ""]), &["title"]);
    lacks(&fish(&fixture, &["md", "rm", "doc.md", "author", ""]), &["author"]);
    assert_eq!(fish(&fixture, &["md", "get", "doc.md", "author", "author."]), ["author.name"]);
    lacks(&fish(&fixture, &["md", "compose", "doc.md", "title=x", ""]), &["title="]);
    let after_child = fish(&fixture, &["md", "compose", "doc.md", "style.page.alignment=left", "style.page."]);
    lacks(&after_child, &["style.page.alignment="]);
    holds(&after_child, &["style.page.max-width="]);
    holds(&fish(&fixture, &["md", "compose", "doc.md", "style.page.alignment=left", "st"]), &["style."]);
}

#[test]
fn a_document_schema_supplies_keys_and_values() {
    let fixture = fixture("completion-document-schema");
    let keys = fish(&fixture, &["md", "compose", "schemaed.md", ""]);
    holds(&keys, &["lang=", "mode=", "inc=", "sub.", "title="]);
    assert_eq!(keys[0], "lang=", "a required key sorts first: {keys:?}");
    assert_eq!(fish(&fixture, &["md", "compose", "schemaed.md", "lang="]), ["lang=rust", "lang=go"]);
}

#[test]
fn earlier_setters_and_overrides_choose_the_schema_the_keys_come_from() {
    let fixture = fixture("completion-overlay");
    write_in_cwd(&fixture, "plain.md", "---\ntitle: x\n---\n");
    write_in_cwd(&fixture, "union.yaml", "$schema:\n  - { kind: 'literal(a)', only_a: string }\n  - { kind: 'literal(b)', only_b: string }\n");
    write_in_cwd(&fixture, "tagged.md", "---\n$schema: ./union.yaml\n---\n");

    let without = fish(&fixture, &["md", "compose", "tagged.md", ""]);
    holds(&without, &["only_a=", "only_b="]);
    let narrowed = fish(&fixture, &["md", "compose", "tagged.md", "kind=b", ""]);
    holds(&narrowed, &["only_b="]);
    lacks(&narrowed, &["only_a="]);

    // The setter's position on the line does not matter, only that it is earlier.
    let cursor_first = complete_in(&fixture, "elvish", Some(3), &["md", "compose", "tagged.md", "", "kind=b"]);
    holds(&cursor_first, &["only_a="]);

    holds(
        &fish(&fixture, &["md", "compose", "--set", r#"{"$schema":"./s.yaml"}"#, "plain.md", ""]),
        &["lang=", "mode="],
    );
    lacks(&fish(&fixture, &["md", "compose", "plain.md", ""]), &["lang="]);
    holds(&fish(&fixture, &["md", "compose", "--state", "{extra:{deep:1}}", "plain.md", "extra."]), &["extra.deep="]);
}

#[test]
fn an_earlier_setter_selects_a_trigger_schema() {
    let fixture = fixture("completion-trigger");
    assert!(fixture.initialize_repository());
    write_in_cwd(
        &fixture,
        "schemas/lang.trigger.yaml",
        "kind: schema-trigger\nmatch:\n  lang: enum(rust; required)\n$schema:\n  edition: enum(2021, 2024)\n",
    );
    lacks(&fish(&fixture, &["md", "compose", "doc.md", "ed"]), &["edition="]);
    assert_eq!(fish(&fixture, &["md", "compose", "doc.md", "lang=rust", "ed"]), ["edition="]);
    assert_eq!(fish(&fixture, &["md", "compose", "doc.md", "lang=rust", "edition=2"]), ["edition=2021", "edition=2024"]);
}

#[test]
fn invalid_earlier_setters_are_ignored_one_at_a_time() {
    let fixture = fixture("completion-invalid-setters");
    let line = |current: &'static str| {
        ["md", "compose", "doc.md", "title={x:1,x:2}", "items.9=z", "author.name.deep=1", "draft=true", current]
    };
    // A refused setter does not count as given; the valid one beside it does.
    let root = fish(&fixture, &line(""));
    holds(&root, &["title=", "style."]);
    lacks(&root, &["draft="]);
    // A refused child of a scalar leaves that scalar in place.
    assert_eq!(fish(&fixture, &line("author.")), ["author.name="]);
    assert_eq!(fish(&fixture, &line("items.")), ["items.0=", "items.1="]);
}

#[test]
fn words_after_the_cursor_are_ignored() {
    let fixture = fixture("completion-after-cursor");
    // The cursor is on the empty word; the document and setter come after it.
    let words = ["md", "compose", "", "doc.md", "title=x"];
    let candidates = complete_in(&fixture, "zsh", Some(2), &words);
    lacks(&candidates, &["author."]);
    holds(&candidates, &["title="]);
}

#[test]
fn a_file_value_offers_ordinary_files() {
    let fixture = fixture("completion-file-values");
    write_in_cwd(&fixture, "notes.txt", "");
    write_in_cwd(&fixture, "sub/inner.json", "");

    holds(&fish(&fixture, &["md", "compose", "schemaed.md", "inc="]), &["inc=notes.txt", "inc=doc.md", "inc=sub/"]);
    assert_eq!(fish(&fixture, &["md", "compose", "schemaed.md", "inc=sub/"]), ["inc=sub/inner.json"]);
    assert_eq!(bash(&fixture, &["md", "compose", "schemaed.md", "inc", "=", "not"]), ["notes.txt"]);
    assert!(fish(&fixture, &["md", "set", "schemaed.md", "inc", ""]).contains(&"notes.txt".to_string()));
    lacks(&fish(&fixture, &["md", "compose", "schemaed.md", "lang="]), &["lang=notes.txt"]);
}

#[test]
fn a_broken_document_or_schema_leaves_the_baseline_without_a_word() {
    let fixture = fixture("completion-broken");
    write_in_cwd(&fixture, "broken.md", "---\ntitle: [unclosed\n---\n");
    write_in_cwd(&fixture, "missing-schema.md", "---\n$schema: ./nowhere.yaml\nmine: 1\n---\n");
    write_in_cwd(&fixture, "bad.yaml", "$schema:\n  lang: enum(two words)\n");
    write_in_cwd(&fixture, "bad-schema.md", "---\n$schema: ./bad.yaml\nmine: 1\n---\n");
    write_in_cwd(&fixture, "remote-schema.md", "---\n$schema: https://127.0.0.1:9/never.yaml\nmine: 1\n---\n");

    let broken = fish(&fixture, &["md", "compose", "broken.md", ""]);
    holds(&broken, &["title=", "style."]);

    for document in ["missing-schema.md", "bad-schema.md", "remote-schema.md"] {
        let candidates = fish(&fixture, &["md", "compose", document, ""]);
        holds(&candidates, &["title=", "style.", "mine="]);
        lacks(&candidates, &["lang="]);
    }

    holds(&fish(&fixture, &["md", "compose", "absent.md", ""]), &["title=", "style."]);
    holds(&fish(&fixture, &["md", "--magic-root", "absent-dir", "compose", "doc.md", ""]), &["author."]);
    holds(&fish(&fixture, &["md", "compose", "--set", "{not json", "doc.md", ""]), &["author."]);
}

#[test]
fn stdin_is_never_read() {
    let fixture = fixture("completion-stdin");
    assert_eq!(paths_only(fish(&fixture, &["md", "rm", "-", ""])), Vec::<String>::new());
    lacks(&fish(&fixture, &["md", "get", "-", ""]), &["from_stdin"]);
    lacks(&fish(&fixture, &["md", "compose", "-", ""]), &["from_stdin="]);
}

#[test]
fn a_magic_root_on_the_line_finds_the_document() {
    let fixture = fixture("completion-magic-root");
    write_in_cwd(&fixture, "roots/found.md", "---\nfrom_root: 1\n---\n");
    holds(&fish(&fixture, &["md", "--magic-root", "roots", "get", "@found.md", ""]), &["from_root"]);
    lacks(&fish(&fixture, &["md", "get", "@found.md", ""]), &["from_root"]);
}

#[test]
fn bash_gets_only_the_last_piece_in_every_word_shape() {
    let fixture = fixture("completion-bash-shapes");
    let values = ["left", "center", "right"];
    // Bash 4 and later: the cursor right after `=`, on `=` and on a further empty word.
    assert_eq!(bash(&fixture, &["md", "compose", "doc.md", "style.page.alignment", "="]), values);
    assert_eq!(bash(&fixture, &["md", "compose", "doc.md", "style.page.alignment", "=", ""]), values);
    assert_eq!(bash(&fixture, &["md", "compose", "doc.md", "style.page.alignment", "=", "ce"]), ["center"]);
    // Bash 3.2 keeps the word whole, but still replaces only what follows `=`.
    assert_eq!(bash(&fixture, &["md", "compose", "doc.md", "style.page.alignment="]), values);
    assert_eq!(bash(&fixture, &["md", "compose", "doc.md", "style.page.alignment=ce"]), ["center"]);
    // Setters earlier on the line rejoin too, so one still counts as given.
    let later = bash(&fixture, &["md", "compose", "doc.md", "title", "=", "x", ""]);
    lacks(&later, &["title="]);
    holds(&later, &["style."]);
}

#[test]
fn bash_escapes_a_value_so_it_arrives_as_one_word() {
    let fixture = fixture("completion-bash-escape");
    assert_eq!(bash(&fixture, &["md", "compose", "schemaed.md", "mode", "=", "two"]), [r"two\ words"]);
    assert_eq!(bash(&fixture, &["md", "compose", "schemaed.md", "mode", "=", "$"]), [r"\$x"]);
    assert_eq!(bash(&fixture, &["md", "compose", "schemaed.md", "mode", "=", "a", "=", ""]), ["b"]);
    // The escape a person types is removed before the path is read.
    assert_eq!(bash(&fixture, &["md", "compose", "schemaed.md", r"mode=two\ w"]), [r"two\ words"]);
}

#[test]
fn the_shells_that_pass_a_whole_word_get_a_whole_word() {
    let fixture = fixture("completion-whole-word");
    let word = ["md", "compose", "doc.md", "style.page.alignment=ce"];
    assert_eq!(complete_in(&fixture, "zsh", Some(3), &word), ["style.page.alignment=center"]);
    assert_eq!(complete_in(&fixture, "elvish", Some(3), &word), ["style.page.alignment=center"]);
    assert_eq!(complete_in(&fixture, "powershell", None, &word), ["style.page.alignment=center"]);
}

#[test]
fn a_lone_equals_word_is_only_rejoined_for_bash() {
    let fixture = fixture("completion-lone-equals");
    let words = ["md", "set", "doc.md", "style.page.alignment", "="];
    assert_eq!(fish(&fixture, &words), Vec::<String>::new());
}

#[test]
fn registration_scripts_print_for_every_shell() {
    let fixture = CliProcessFixture::named("completion-registration");
    for (shell, marker) in [
        ("bash", "_MD_COMPLETE_QUOTE"),
        ("zsh", "continues -S ''"),
        ("fish", "complete --keep-order"),
        ("elvish", "arg-completer"),
        ("powershell", "Register-ArgumentCompleter"),
    ] {
        let output = fixture.command().env("COMPLETE", shell).output().unwrap();
        assert!(output.status.success(), "{shell}: {output:?}");
        let script = String::from_utf8_lossy(&output.stdout);
        assert!(script.contains(marker), "{shell}: {script}");
    }
}

/// Every file under `directory` with its bytes, so a test can show a Tab changed nothing.
fn files_under(directory: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut files = BTreeMap::new();
    let mut pending = vec![directory.to_path_buf()];
    while let Some(next) = pending.pop() {
        for entry in std::fs::read_dir(next).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else {
                let bytes = std::fs::read(&path).unwrap();
                files.insert(path, bytes);
            }
        }
    }
    files
}

const COMMANDS_BODY: &str = "mine: 1\nfrom_command: $(argv-recorder frontmatter)\nshown: \"{{ title }}\"\ntitle: T\n---\n::shell argv-recorder body\n";

/// A document with a frontmatter command, an interpolated value, and a body command,
/// all approved, so only completion's own restraint stops them. The command is the
/// recorder, a native program, because it exists on every platform and its evidence
/// is exactly what ran.
fn commands_fixture(name: &str, schema_line: &str) -> CliProcessFixture {
    let fixture = CliProcessFixture::named(name);
    let program = fixture.bin_dir().join(format!("argv-recorder{}", std::env::consts::EXE_SUFFIX));
    std::fs::copy(biscuit_test_harness::bin_exe!("darkmatter-argv-recorder"), program).unwrap();
    write_in_cwd(&fixture, "doc.md", &format!("---\n{schema_line}\n{COMMANDS_BODY}"));
    write_in_cwd(&fixture, ".darkmatter-shell-whitelist", "prefix argv-recorder\n");
    fixture
}

/// `md compose`'s, `get`'s, `set`'s, and `rm`'s root positions, with and without a
/// host permission that would let an ordinary composition fetch.
fn assert_tabs_leave_everything_alone(fixture: &CliProcessFixture) {
    let before = files_under(fixture.workspace_path());
    let plain = fish(fixture, &["md", "compose", "doc.md", ""]);
    holds(&plain, &["mine=", "from_command=", "shown=", "style."]);
    let allowed = fish(fixture, &["md", "compose", "--allow", "host=127.0.0.1", "doc.md", ""]);
    assert_eq!(allowed, plain, "a host permission on the line changes nothing");
    holds(&paths_only(fish(fixture, &["md", "get", "doc.md", ""])), &["mine", "from_command"]);
    holds(&paths_only(fish(fixture, &["md", "set", "doc.md", ""])), &["mine", "from_command"]);
    holds(&paths_only(fish(fixture, &["md", "rm", "doc.md", ""])), &["mine", "from_command"]);
    bash(fixture, &["md", "compose", "doc.md", "mine", "=", ""]);
    assert_eq!(files_under(fixture.workspace_path()), before, "a Tab ran a command, or wrote a cache entry or an edit");
}

#[test]
fn a_tab_runs_no_command_and_writes_nothing_while_composing_the_same_document_does() {
    let fixture = commands_fixture("completion-sentinel", "description: D");
    assert_tabs_leave_everything_alone(&fixture);

    // The control: the document really does run these when it is composed.
    let output = fixture.command().env(ARGV_RECORDER_FILE, recorder_file(&fixture)).args(["compose", "doc.md"]).output().unwrap();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(read_argv_records(&recorder_file(&fixture)).unwrap(), [["frontmatter"], ["body"]]);
}

#[test]
fn a_remote_schema_is_never_requested_and_local_keys_remain() {
    let server = mock_http_server(vec![MockHttpResponse { status: 200, body: "$schema:\n  remote_key: string\n", cache_control: None }]);
    let url = server.url("/remote.yaml");
    let fixture = commands_fixture("completion-remote-schema", &format!("$schema: {url}"));
    assert_tabs_leave_everything_alone(&fixture);
    lacks(&fish(&fixture, &["md", "compose", "doc.md", ""]), &["remote_key="]);
    assert_eq!(server.requests(), Vec::<String>::new());

    // The control: this host and server do answer an allowed composition.
    let output = fixture
        .command()
        .args(["compose", "-", "--allow", "host=127.0.0.1"])
        .write_stdin(format!("::file {url}\n"))
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(server.request_count(), 1);
}

#[test]
fn a_remote_import_inside_a_local_schema_is_never_requested() {
    let server = mock_http_server(vec![MockHttpResponse { status: 200, body: "kind: schema\ntypes:\n  name: string\n", cache_control: None }]);
    let fixture = commands_fixture("completion-remote-import", "$schema: ./s.yaml");
    write_in_cwd(&fixture, "s.yaml", &format!("$schema:\n  imported: name@{}\n", server.url("/types.yaml")));
    assert_tabs_leave_everything_alone(&fixture);
    lacks(&fish(&fixture, &["md", "compose", "doc.md", ""]), &["imported="]);
    assert_eq!(server.requests(), Vec::<String>::new());
}

#[test]
fn an_interpolated_tag_is_not_evaluated_to_narrow_a_union() {
    let fixture = CliProcessFixture::named("completion-unevaluated-tag");
    write_in_cwd(&fixture, "union.yaml", "$schema:\n  - { kind: 'literal(a)', only_a: string }\n  - { kind: 'literal(b)', only_b: string }\n");
    write_in_cwd(&fixture, "written.md", "---\n$schema: ./union.yaml\nkind: a\n---\n");
    write_in_cwd(&fixture, "interpolated.md", "---\n$schema: ./union.yaml\nkind: \"{{ 'a' }}\"\n---\n");

    let written = fish(&fixture, &["md", "compose", "written.md", ""]);
    holds(&written, &["only_a="]);
    lacks(&written, &["only_b="]);
    holds(&fish(&fixture, &["md", "compose", "interpolated.md", ""]), &["only_a=", "only_b="]);
}

/// Any diagnostic printed into the reply would be an extra candidate, so a broken
/// document must answer exactly as one with nothing wrong answers.
fn assert_failures_add_no_text(shell: &str) {
    let fixture = CliProcessFixture::named("completion-no-diagnostics");
    write_in_cwd(&fixture, "clean.md", "---\nmine: 1\n---\n");
    write_in_cwd(&fixture, "bad.yaml", "$schema:\n  lang: enum(two words)\n");
    write_in_cwd(&fixture, "missing-schema.md", "---\n$schema: ./nowhere.yaml\nmine: 1\n---\n");
    write_in_cwd(&fixture, "bad-schema.md", "---\n$schema: ./bad.yaml\nmine: 1\n---\n");
    write_in_cwd(&fixture, "remote-schema.md", "---\n$schema: https://127.0.0.1:9/never.yaml\nmine: 1\n---\n");
    write_in_cwd(&fixture, "broken.md", "---\ntitle: [unclosed\n---\n");

    let reply = |document: &str| complete_in(&fixture, shell, Some(3), &["md", "get", document, ""]);
    let clean = reply("clean.md");
    holds(&clean, &["mine", "style."]);
    for document in ["missing-schema.md", "bad-schema.md", "remote-schema.md"] {
        assert_eq!(reply(document), clean, "{shell}, {document}");
    }
    assert_eq!(reply("broken.md"), reply("absent.md"), "{shell}");
}

#[test]
fn a_failure_adds_no_text_in_bash() {
    assert_failures_add_no_text("bash");
}

#[test]
fn a_failure_adds_no_text_in_zsh() {
    assert_failures_add_no_text("zsh");
}

#[test]
fn a_failure_adds_no_text_in_fish() {
    assert_failures_add_no_text("fish");
}

#[test]
fn a_failure_adds_no_text_in_elvish() {
    assert_failures_add_no_text("elvish");
}

#[test]
fn a_failure_adds_no_text_in_powershell() {
    assert_failures_add_no_text("powershell");
}

/// Bash 3.2 has no `compopt`, so its registered function sets this and every
/// finished word carries its own space.
#[test]
fn bash_3_finished_words_carry_their_space_and_continuing_ones_do_not() {
    let fixture = fixture("completion-bash-3-space");
    let reply = |word: &str| {
        let output = fixture
            .command()
            .env("COMPLETE", "bash")
            .env("_CLAP_COMPLETE_SPACE", "true")
            .env("_CLAP_COMPLETE_INDEX", "3")
            .args(["--", "md", "compose", "doc.md", word])
            .output()
            .unwrap();
        String::from_utf8_lossy(&output.stdout).into_owned()
    };
    assert_eq!(reply("style.page.alignment=ce"), "center ");
    assert_eq!(reply("style.page.al"), "style.page.alignment=");
    assert!(reply("style.pa").lines().all(|line| !line.ends_with(' ')), "{}", reply("style.pa"));
}

/// The script `md` prints for `shell`, for the shell itself to load.
fn registration(fixture: &CliProcessFixture, shell: &str) -> String {
    let output = fixture.command().env("COMPLETE", shell).output().unwrap();
    assert!(output.status.success(), "{shell}: {output:?}");
    String::from_utf8(output.stdout).unwrap()
}

/// Whether `program` starts at all. Only a few shells are guaranteed per platform;
/// the tests for the others report themselves unproven when it does not.
fn shell_starts(program: &str) -> bool {
    Command::new(program).arg("--version").output().is_ok_and(|output| output.status.success())
}

/// Runs `script` in `program` and returns what it printed after asserting that
/// loading it succeeded and said nothing on stderr. `flags` must keep the shell from
/// reading or writing a profile, since this is not an `md` process and has no fixture.
fn run_shell(program: &str, flags: &[&str], script: &str) -> String {
    let output = Command::new(program).args(flags).arg(script).output().unwrap();
    assert!(output.status.success(), "{program}: {output:?}");
    assert!(output.stderr.is_empty(), "{program}: {}", String::from_utf8_lossy(&output.stderr));
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[cfg(unix)]
fn assert_bash_registers(program: &str) {
    let fixture = CliProcessFixture::named("completion-script-bash");
    let script = registration(&fixture, "bash");
    let printed = run_shell(program, &["--noprofile", "--norc", "-c"], &format!("{script}\ncomplete -p md"));
    assert!(printed.contains("_clap_complete_md"), "{printed}");
}

#[cfg(unix)]
#[test]
fn the_bash_script_loads() {
    expect_level!(Level::L1, shell_starts("bash"), "bash");
    assert_bash_registers("bash");
}

/// The one bash 3.2 that exists on every Mac, whichever bash `PATH` finds first.
#[cfg(target_os = "macos")]
#[test]
fn the_bash_script_loads_in_the_systems_bash_3() {
    expect_level!(Level::L1, shell_starts("/bin/bash"), "/bin/bash");
    assert_bash_registers("/bin/bash");
}

#[cfg(target_os = "macos")]
#[test]
fn the_zsh_script_loads() {
    expect_level!(Level::L1, shell_starts("zsh"), "zsh");
    let fixture = CliProcessFixture::named("completion-script-zsh");
    let script = registration(&fixture, "zsh");
    let printed = run_shell("zsh", &["-f", "-c"], &format!("autoload -Uz compinit && compinit -u -D\n{script}\nprint -r -- $_comps[md]"));
    assert!(printed.contains("_clap_dynamic_completer_md"), "{printed}");
}

/// Fish is optional on every platform: where it is absent this proves nothing and
/// says so, and the implementation log records which hosts ran it.
#[test]
fn the_fish_script_loads_where_fish_is_installed() {
    if !shell_starts("fish") {
        eprintln!("unproven: fish is not installed on this host");
        return;
    }
    let fixture = CliProcessFixture::named("completion-script-fish");
    let script = registration(&fixture, "fish");
    let printed = run_shell("fish", &["--no-config", "-c"], &format!("{script}\ncomplete -c md"));
    assert!(printed.contains("complete"), "{printed}");
}

#[cfg(windows)]
#[test]
fn the_powershell_script_loads() {
    // Windows PowerShell has no `--version` flag.
    let starts = Command::new("powershell").args(["-NoProfile", "-NonInteractive", "-Command", "exit 0"]).output();
    expect_level!(Level::L1, starts.is_ok_and(|output| output.status.success()), "powershell");
    let fixture = CliProcessFixture::named("completion-script-powershell");
    let script = registration(&fixture, "powershell");
    run_shell("powershell", &["-NoProfile", "-NonInteractive", "-Command"], &script);
}
