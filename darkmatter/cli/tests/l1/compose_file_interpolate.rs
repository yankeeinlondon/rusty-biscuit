//! `::file interpolate=false` through `md compose`: a caller skill published
//! with the publication arguments (`compose --frontmatter --deny all`, as
//! `just publish-skills` runs them), located errors for invalid values, and a
//! passive check that the shipped caller-skill sources parse with the option.

use crate::common;

use std::path::Path;

use biscuit_terminal::errors::SourceContext;
use common::CliProcessFixture;
use darkmatter::markdown::compose::directives_api::{DirectiveKind, parse_interpolate_value, scan_darkmatter_directives};
use darkmatter::markdown::compose::transclusion::parse_directives;
use serde_json::Value;

/// The arguments `just publish-skills` composes every skill file with.
const PUBLICATION_ARGS: [&str; 4] = ["compose", "--frontmatter", "--deny", "all"];

/// Writes the frozen publication fixture under `cwd/` and returns the
/// skill's `SKILL.md`. `skill` replaces the frozen `SKILL.md` text when given.
fn publication_fixture(fixture: &CliProcessFixture, examples: &str, skill: &str) -> std::path::PathBuf {
    fixture.write_file("cwd/topics/examples.md", examples);
    fixture.write_file("cwd/use-skills/use-example/SKILL.md", skill)
}

#[test]
fn a_published_skill_keeps_a_disabled_inclusions_examples_verbatim() {
    let fixture = CliProcessFixture::named("a_published_skill_keeps_a_disabled_inclusions_examples_verbatim");
    let skill = publication_fixture(
        &fixture,
        include_str!("../fixtures/file_interpolate_publication/topics/examples.md"),
        include_str!("../fixtures/file_interpolate_publication/use-skills/use-example/SKILL.md"),
    );
    let output = fixture.command().args(PUBLICATION_ARGS).arg(&skill).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "publication succeeds: {stderr}");
    assert!(stderr.trim().is_empty(), "no warning reaches a deny-all publication: {stderr}");

    // The skill's own prose and frontmatter still interpolate.
    assert!(stdout.starts_with("---\nname: use-example\n"), "frontmatter is published: {stdout}");
    assert!(stdout.contains("# use-example\n"), "the parent's prose interpolates: {stdout}");
    // The included examples survive byte for byte, braces and all.
    assert!(stdout.contains("Write {{ name }} to show the document's name."), "{stdout}");
    assert!(stdout.contains("An undeclared {{ undeclared }} is a warning"), "{stdout}");
    assert!(stdout.contains("Write {{{ x }}} to publish"), "{stdout}");
    assert!(!stdout.contains("::file"), "the directive is replaced: {stdout}");
}

#[test]
fn without_the_option_publication_still_fails_on_an_undeclared_name() {
    let fixture = CliProcessFixture::named("without_the_option_publication_still_fails_on_an_undeclared_name");
    let frozen = include_str!("../fixtures/file_interpolate_publication/use-skills/use-example/SKILL.md");
    let skill = publication_fixture(
        &fixture,
        include_str!("../fixtures/file_interpolate_publication/topics/examples.md"),
        &frozen.replace(" interpolate=false", ""),
    );
    let output = fixture.command().args(PUBLICATION_ARGS).arg(&skill).output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "the undeclared example blocks publication: {stderr}");
    assert!(stderr.contains("undeclared"), "the blocking warning names the example: {stderr}");
    assert!(output.stdout.is_empty(), "nothing is published");
}

/// Composes `top.md` → `mid.md` → `child.md`, where `mid.md`'s inclusion
/// carries `options`, and returns the process output for `--output json`
/// when `json` is set, with the fixture (which owns the files) and `mid.md`.
fn nested_compose(name: &str, options: &str, json: bool) -> (std::process::Output, CliProcessFixture, std::path::PathBuf) {
    let fixture = CliProcessFixture::named(name);
    fixture.write_file("cwd/child.md", "Child {{ x }}.\n");
    let mid = fixture.write_file("cwd/mid.md", &format!("Mid.\n\n::file ./child.md {options}\n"));
    let top = fixture.write_file("cwd/top.md", "---\nx: 1\n---\nTop.\n\n::file ./mid.md\n");
    let mut command = fixture.command();
    command.args(["compose", "--deny", "all"]).arg(&top);
    if json {
        command.args(["--output", "json"]);
    }
    (command.output().unwrap(), fixture, mid)
}

fn envelope(output: &std::process::Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!("a JSON envelope ({error}): {}", String::from_utf8_lossy(&output.stdout))
    })
}

/// Whether `reported` (absolute, or relative to the run's directory, which
/// holds every fixture document) names `expected`.
fn same_file(reported: &str, expected: &Path) -> bool {
    let run_dir = expected.parent().expect("the run's directory");
    let reported = biscuit_file::canonicalize_simplified(&run_dir.join(reported)).unwrap_or_else(|error| panic!("reported `{reported}` under {run_dir:?}: {error}"));
    reported == biscuit_file::canonicalize_simplified(expected).expect("expected file exists")
}

#[test]
fn a_valid_interpolate_value_composes_the_nested_child_literally() {
    let (output, _fixture, _) = nested_compose("a_valid_interpolate_value_composes_the_nested_child_literally", "interpolate=false", false);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(stdout.contains("Top.") && stdout.contains("Mid."), "{stdout}");
    assert!(stdout.contains("Child {{ x }}."), "the child's prose is literal: {stdout}");
}

/// The invalid shapes: a quoted `"null"` (text, not the null value) and a
/// missing value. Each fails the run with the including file named, nothing
/// composed, and the terminal error pointing at the directive's line.
macro_rules! invalid_interpolate_values {
    ($($name:ident: $options:literal => $reason:literal),+ $(,)?) => {$(
        mod $name {
            use super::*;

            #[test]
            fn json_names_the_including_file_and_composes_nothing() {
                let (output, _fixture, mid) = nested_compose(concat!(stringify!($name), "_json"), $options, true);
                assert_eq!(output.status.code(), Some(1));
                let envelope = envelope(&output);
                assert_eq!(envelope["outcome"], "failure");
                assert!(envelope["document"].is_null(), "nothing composed: {envelope:#}");
                let [diagnostic] = envelope["diagnostics"].as_array().expect("diagnostics").as_slice() else {
                    panic!("one diagnostic: {envelope:#}");
                };
                assert_eq!(diagnostic["code"], "dm.directive.parse_failure");
                assert!(
                    same_file(diagnostic["source"]["file"].as_str().expect("a file"), &mid),
                    "the including file, not the root: {diagnostic:#}"
                );
                let message = diagnostic["message"].as_str().expect("message");
                assert!(message.contains("line 3") && message.contains($reason), "{message}");
            }

            #[test]
            fn the_terminal_error_points_at_the_directive_line() {
                let (output, _fixture, _) = nested_compose(concat!(stringify!($name), "_terminal"), $options, false);
                assert_eq!(output.status.code(), Some(1));
                assert!(output.stdout.is_empty(), "nothing composed");
                let stderr = String::from_utf8_lossy(&output.stderr);
                assert!(stderr.contains(concat!("3 │ ::file ./child.md ", $options)), "{stderr}");
                assert!(stderr.contains("Column 31"), "{stderr}");
                assert!(stderr.contains($reason), "{stderr}");
            }
        }
    )+};
}

invalid_interpolate_values! {
    quoted_null: "interpolate=\"null\"" => "not \"null\"",
    missing_value: "interpolate=" => "requires a value",
}

/// The shipped caller-skill sources, read passively: every `::file` /
/// `::url` directive the composition parser finds parses, and every
/// `interpolate` value on one is a value composition accepts.
#[test]
fn shipped_caller_skill_sources_parse_with_their_interpolate_options() {
    let use_skills = biscuit_test_harness::manifest_dir!().parent().expect("package area").join("docs/use-skills");
    let mut files = Vec::new();
    let mut pending = vec![use_skills.clone()];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir).expect("read use-skills") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|extension| extension == "md") {
                files.push(path);
            }
        }
    }
    assert!(files.len() >= 2, "both caller skills are scanned: {files:?}");

    for path in files {
        let text = std::fs::read_to_string(&path).expect("read skill source").replace("\r\n", "\n");
        let relative = path.strip_prefix(&use_skills).expect("under use-skills").to_path_buf();
        let ctx = SourceContext::new(path.clone(), relative, std::sync::Arc::from(text.as_str()));
        if let Err(error) = parse_directives(&text, ctx) {
            panic!("{} does not parse: {error}", path.display());
        }
        for directive in scan_darkmatter_directives(&text) {
            if !matches!(directive.kind, DirectiveKind::File | DirectiveKind::Url) {
                continue;
            }
            for option in directive.options.iter().filter(|option| option.key.value == "interpolate") {
                let value = option.value.as_ref().unwrap_or_else(|| panic!("{}: a value", path.display()));
                let was_quoted = text[value.span.clone()].starts_with(['\'', '"']);
                assert!(
                    parse_interpolate_value(&value.value, was_quoted).is_ok(),
                    "{}: `interpolate={}`",
                    path.display(),
                    value.value
                );
            }
        }
    }
}
