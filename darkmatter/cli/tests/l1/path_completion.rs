//! Tab completion through the real `md` process: `COMPLETE=<shell> md -- <words>`.
//!
//! These run the normal completion entry point with a fixture-owned document,
//! schema, and home, so they prove the wiring from the shell's words to the
//! candidates: the input found on the line, the schema resolved for it, and what
//! is printed. The candidates themselves are covered in the library's
//! `path_candidates` tests.

use crate::common;

use common::CliProcessFixture;

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

/// What a Tab press prints, one candidate per line, with help removed. Asserts
/// that it succeeded and said nothing else.
fn complete_in(fixture: &CliProcessFixture, shell: &str, index: Option<usize>, words: &[&str]) -> Vec<String> {
    let mut command = fixture.command();
    command.env("COMPLETE", shell).arg("--").args(words).write_stdin("---\nfrom_stdin: 1\n---\n");
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
