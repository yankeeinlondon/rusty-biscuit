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

/// A directive's `set` value selects the child's branch for the reference
/// check exactly as it does for composition: with only the selected branch's
/// target present, `md compose` succeeds and prints that branch.
#[test]
fn the_reference_check_follows_a_set_override_under_interpolate_false() {
    let fixture = CliProcessFixture::named("the_reference_check_follows_a_set_override_under_interpolate_false");
    fixture.write_file(
        "cwd/child.md",
        "---\nword: default\n---\n::block when=\"word == 'supplied'\"\n[Selected](./supplied.md)\n::end-block\n::block when=\"word == 'default'\"\n[Default](./default.md)\n::end-block\n",
    );
    fixture.write_file("cwd/supplied.md", "# Supplied\n");
    let root = fixture.write_file("cwd/root.md", "::file ./child.md interpolate=false set.word=\"supplied\"\n");

    let output = fixture.command().arg("compose").arg(&root).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "the check passes with only the selected target: {stderr}");
    assert!(stdout.contains("[Selected]("), "{stdout}");
    assert!(!stdout.contains("[Default]("), "{stdout}");
    assert!(!stderr.contains("default.md"), "no reference issue names the removed branch: {stderr}");
}

/// A value inherited from a parent included with `set` selects the
/// grandchild's branch for the reference check as it does for composition:
/// the selected link's missing target fails the run, whichever loader
/// (body `::file`, `prologue:`, `epilogue:`) includes the grandchild.
macro_rules! inherited_reference_checks {
    ($($name:ident: $parent:literal),+ $(,)?) => {$(
        #[test]
        fn $name() {
            let fixture = CliProcessFixture::named(stringify!($name));
            fixture.write_file("cwd/parent.md", $parent);
            fixture.write_file(
                "cwd/leaf.md",
                "::block when=\"word == 'supplied'\"\n[Selected](./missing.md)\n::end-block\n",
            );
            let root = fixture.write_file("cwd/root.md", "::file ./parent.md interpolate=false set.word=\"supplied\"\n");

            let output = fixture.command().arg("compose").arg(&root).args(["--output", "json"]).output().unwrap();
            assert_eq!(output.status.code(), Some(1), "{}", String::from_utf8_lossy(&output.stdout));
            let envelope = envelope(&output);
            assert_eq!(envelope["outcome"], "failure", "{envelope:#}");
            let diagnostics = envelope["diagnostics"].as_array().expect("diagnostics");
            assert!(
                diagnostics.iter().any(|diagnostic| diagnostic.to_string().contains("missing.md")),
                "the selected link's missing target is reported: {envelope:#}"
            );
        }
    )+};
}

inherited_reference_checks! {
    the_reference_check_follows_an_inherited_value_into_a_body_file_grandchild: "::file ./leaf.md\n",
    the_reference_check_follows_an_inherited_value_into_a_prologue_grandchild: "---\nprologue: ./leaf.md\n---\nParent.\n",
    the_reference_check_follows_an_inherited_value_into_an_epilogue_grandchild: "---\nepilogue: ./leaf.md\n---\nParent.\n",
}

/// A directive's one-off `replace=` map rewrites the child's link before the
/// reference check reads it: with only the replaced target present, the
/// normal `md compose` passes and emits the replaced link.
#[test]
fn the_reference_check_reads_a_link_after_the_directives_replacement() {
    let fixture = CliProcessFixture::named("the_reference_check_reads_a_link_after_the_directives_replacement");
    fixture.write_file("cwd/child.md", "[Link](./WORD.md)\n");
    fixture.write_file("cwd/one.md", "# One\n");
    let root = fixture.write_file("cwd/root.md", "::file ./child.md interpolate=false replace='{\"WORD\":\"one\"}'\n");

    let output = fixture.command().arg("compose").arg(&root).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "the check passes with only the replaced target: {stderr}");
    assert!(stdout.contains("one.md)"), "{stdout}");
    assert!(!stderr.contains("WORD.md"), "no reference issue names the unreplaced target: {stderr}");
}

/// `::toc-linking` links its target's raw headings, so `validate refs
/// --fragments` accepts the anchor `md compose` emitted even when the
/// target's own replacement rewrites that heading.
#[test]
fn fragment_validation_accepts_the_raw_anchor_a_toc_links() {
    let fixture = CliProcessFixture::named("fragment_validation_accepts_the_raw_anchor_a_toc_links");
    fixture.write_file("cwd/toc.md", "---\nreplace:\n  WORD: World\n---\n# Toc\n\n## Hello WORD\n");
    fixture.write_file("cwd/parent.md", "::toc-linking ./toc.md\n");
    let root = fixture.write_file("cwd/root.md", "::file ./parent.md interpolate=false\n");

    let composed = fixture.command().arg("compose").arg(&root).output().unwrap();
    let stdout = String::from_utf8_lossy(&composed.stdout);
    assert!(composed.status.success(), "{}", String::from_utf8_lossy(&composed.stderr));
    assert!(stdout.contains("[Hello WORD](./toc.md#hello-word)"), "composition links the raw heading: {stdout}");

    let validated =
        fixture.command().args(["validate", "refs"]).arg(&root).args(["--fragments", "--output", "json"]).output().unwrap();
    assert_eq!(validated.status.code(), Some(0), "the generated anchor is valid: {:#}", envelope(&validated));
}

/// One fragment included twice with different values: the good copy first
/// and the copy whose selected link is missing second. `md compose`, `md
/// validate refs`, and `md graph --validate` each judge the second copy's
/// own link, fail, and name it; the followed graph shows each copy's link,
/// and labels only the missing one.
#[test]
fn every_reference_gate_judges_each_copy_of_a_repeated_inclusion() {
    let fixture = CliProcessFixture::named("every_reference_gate_judges_each_copy_of_a_repeated_inclusion");
    fixture.write_file("cwd/present.md", "# Present\n");
    fixture.write_file(
        "cwd/leaf.md",
        "::block when=\"word == 'a'\"\n[Good](./present.md)\n::end-block\n::block when=\"word == 'b'\"\n[Bad](./missing.md)\n::end-block\n",
    );
    let root = fixture.write_file(
        "cwd/root.md",
        "::file ./leaf.md interpolate=false set.word=\"a\"\n\n::file ./leaf.md interpolate=false set.word=\"b\"\n",
    );

    let gates: [&[&str]; 3] = [&["compose"], &["validate", "refs"], &["graph", "--follow", "--validate"]];
    for gate in gates {
        let output = fixture.command().args(gate).arg(&root).args(["--output", "json"]).output().unwrap();
        let envelope = envelope(&output);
        assert_eq!(output.status.code(), Some(1), "{gate:?}: {envelope:#}");
        let missing: Vec<&Value> = envelope["diagnostics"]
            .as_array()
            .expect("diagnostics")
            .iter()
            .filter(|diagnostic| diagnostic.to_string().contains("missing.md"))
            .collect();
        assert_eq!(missing.len(), 1, "{gate:?} names the missing link once: {envelope:#}");
        assert!(!envelope["diagnostics"].to_string().contains("present.md"), "{gate:?}: {envelope:#}");
    }

    let graph = fixture.command().args(["graph", "--follow", "--validate"]).arg(&root).output().unwrap();
    let text = String::from_utf8_lossy(&graph.stdout);
    assert_eq!(graph.status.code(), Some(1), "{text}");
    let rows: Vec<&str> = text.lines().filter(|line| line.contains("present.md") || line.contains("missing.md")).collect();
    assert!(rows.iter().any(|row| row.contains("present.md") && !row.contains("[missing]")), "{text}");
    assert!(rows.iter().any(|row| row.contains("missing.md") && row.contains("[missing]")), "{text}");
    assert!(!rows.iter().any(|row| row.contains("present.md") && row.contains("[missing]")), "{text}");
}

/// A holder whose `::toc-linking` target follows a supplied value, included
/// twice with different values: `validate refs --fragments` checks each
/// copy's anchor against its own target's raw headings.
#[test]
fn fragment_validation_checks_each_copy_of_a_repeated_toc_holder() {
    let fixture = CliProcessFixture::named("fragment_validation_checks_each_copy_of_a_repeated_toc_holder");
    for target in ["a", "b"] {
        fixture.write_file(format!("cwd/{target}.md"), "---\nreplace:\n  WORD: World\n---\n# Toc\n\n## Hello WORD\n");
    }
    fixture.write_file("cwd/holder.md", "::toc-linking ./{{ word }}.md\n");
    let root = fixture.write_file("cwd/root.md", "::file ./holder.md set.word=\"a\"\n\n::file ./holder.md set.word=\"b\"\n");

    let composed = fixture.command().arg("compose").arg(&root).output().unwrap();
    let stdout = String::from_utf8_lossy(&composed.stdout);
    assert!(composed.status.success(), "{}", String::from_utf8_lossy(&composed.stderr));
    assert!(stdout.contains("a.md#hello-word)") && stdout.contains("b.md#hello-word)"), "{stdout}");

    let validated =
        fixture.command().args(["validate", "refs"]).arg(&root).args(["--fragments", "--output", "json"]).output().unwrap();
    assert_eq!(validated.status.code(), Some(0), "both generated anchors are valid: {:#}", envelope(&validated));
}

/// A `::map` body whose inclusion reads its item, held by a fragment included
/// with `interpolate=false`.
const MAPPED_PARENT: &str =
    "---\n$schema:\n  words: string[](required)\nwords: [one, two]\n---\n::map words \"item => item\"\nBODY\n::end-map\n";

/// The `replace=` value of each copy reads its item, and both targets exist:
/// `md compose`, `md validate refs`, and `md graph --follow --validate` all
/// accept the document.
#[test]
fn every_reference_gate_reads_each_mapped_copys_item() {
    let fixture = CliProcessFixture::named("every_reference_gate_reads_each_mapped_copys_item");
    fixture.write_file("cwd/one.md", "# One\n");
    fixture.write_file("cwd/two.md", "# Two\n");
    fixture.write_file("cwd/leaf.md", "[Link](./WORD.md)\n");
    fixture.write_file("cwd/parent.md", &MAPPED_PARENT.replace("BODY", "::file ./leaf.md replace='{\"WORD\":\"{{ item }}\"}'"));
    let root = fixture.write_file("cwd/root.md", "::file ./parent.md interpolate=false\n");

    let composed = fixture.command().arg("compose").arg(&root).output().unwrap();
    let stdout = String::from_utf8_lossy(&composed.stdout);
    assert!(composed.status.success(), "{}", String::from_utf8_lossy(&composed.stderr));
    assert!(stdout.contains("one.md)") && stdout.contains("two.md)"), "{stdout}");

    let gates: [&[&str]; 3] = [&["compose"], &["validate", "refs"], &["graph", "--follow", "--validate"]];
    for gate in gates {
        let output = fixture.command().args(gate).arg(&root).args(["--output", "json"]).output().unwrap();
        assert_eq!(output.status.code(), Some(0), "{gate:?}: {:#}", envelope(&output));
    }
}

/// Control: the copy for `two` selects a missing link, and every reference
/// gate fails naming it once.
#[test]
fn every_reference_gate_names_a_mapped_copys_missing_link() {
    let fixture = CliProcessFixture::named("every_reference_gate_names_a_mapped_copys_missing_link");
    fixture.write_file("cwd/present.md", "# Present\n");
    fixture.write_file(
        "cwd/leaf.md",
        "::block when=\"word == 'one'\"\n[Good](./present.md)\n::end-block\n::block when=\"word == 'two'\"\n[Bad](./missing.md)\n::end-block\n",
    );
    fixture.write_file("cwd/parent.md", &MAPPED_PARENT.replace("BODY", "::file ./leaf.md set.word=\"{{ item }}\""));
    let root = fixture.write_file("cwd/root.md", "::file ./parent.md interpolate=false\n");

    let gates: [&[&str]; 3] = [&["compose"], &["validate", "refs"], &["graph", "--follow", "--validate"]];
    for gate in gates {
        let output = fixture.command().args(gate).arg(&root).args(["--output", "json"]).output().unwrap();
        let envelope = envelope(&output);
        assert_eq!(output.status.code(), Some(1), "{gate:?}: {envelope:#}");
        let diagnostics = envelope["diagnostics"].as_array().expect("diagnostics");
        let missing = diagnostics.iter().filter(|diagnostic| diagnostic.to_string().contains("missing.md")).count();
        assert_eq!(missing, 1, "{gate:?} names the missing link once: {envelope:#}");
        assert!(!envelope["diagnostics"].to_string().contains("present.md"), "{gate:?}: {envelope:#}");
    }
}

/// Copies a `::map` repeats from one directive line share their reference
/// ids and inclusion trail. `md graph --follow --validate` labels only the
/// copy whose transcluded target is missing, and fails for that error alone.
#[test]
fn the_followed_graph_labels_only_the_missing_copy_of_a_same_line_inclusion() {
    let fixture = CliProcessFixture::named("the_followed_graph_labels_only_the_missing_copy_of_a_same_line_inclusion");
    fixture.write_file("cwd/present.md", "# Present\n");
    fixture.write_file("cwd/leaf.md", "::file ./WORD.md\n");
    fixture.write_file(
        "cwd/parent.md",
        "---\n$schema:\n  words: string[](required)\nwords: [present, missing]\n---\n::map words \"word => word\"\n::file ./leaf.md replace='{\"WORD\":\"{{ word }}\"}'\n::end-map\n",
    );
    let root = fixture.write_file("cwd/root.md", "::file ./parent.md interpolate=false\n");

    let graph =
        fixture.command().args(["graph", "--follow", "--validate", "--output", "plain"]).arg(&root).output().unwrap();
    let text = String::from_utf8_lossy(&graph.stdout);
    assert_eq!(graph.status.code(), Some(1), "{text}");
    let rows: Vec<&str> = text.lines().filter(|line| line.contains("./present.md") || line.contains("./missing.md")).collect();
    assert!(rows.iter().any(|row| row.contains("./missing.md") && row.contains("[missing]")), "{text}");
    assert!(rows.iter().any(|row| row.contains("./present.md")), "{text}");
    assert!(!rows.iter().any(|row| row.contains("./present.md") && row.contains("[missing]")), "{text}");
}
