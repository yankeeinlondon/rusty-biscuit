//! An interpolation literal (`{{{ expr }}}`) composes to `{{ expr }}` in every
//! file, whether or not that file also has a span to evaluate, both when it is
//! composed directly and when it is transcluded.

use crate::common;

use common::CliProcessFixture;

const LITERAL_ONLY: &str = "only a literal: {{{ expr }}}\n";

/// Compose `path` and return its stdout.
fn compose(fixture: &CliProcessFixture, path: &std::path::Path) -> String {
    let output = fixture
        .command()
        .arg("compose")
        .arg(path)
        .assert()
        .success()
        .get_output()
        .clone();
    String::from_utf8(output.stdout).expect("compose stdout is UTF-8")
}

#[test]
fn literal_only_file_converts_when_composed_directly() {
    let fixture = CliProcessFixture::named("literal_only_file_converts_when_composed_directly");
    let path = fixture.write_file("cwd/literal.md", LITERAL_ONLY);

    assert_eq!(
        compose(&fixture, &path).trim_end(),
        "only a literal: {{ expr }}"
    );
}

#[test]
fn literal_only_file_converts_when_transcluded() {
    let fixture = CliProcessFixture::named("literal_only_file_converts_when_transcluded");
    fixture.write_file("cwd/literal.md", LITERAL_ONLY);
    let parent = fixture.write_file("cwd/parent.md", "parent\n\n::file ./literal.md\n");

    assert_eq!(
        compose(&fixture, &parent).trim_end(),
        "parent\n\nonly a literal: {{ expr }}"
    );
}

#[test]
fn literal_only_frontmatter_value_converts() {
    let fixture = CliProcessFixture::named("literal_only_frontmatter_value_converts");
    let path = fixture.write_file(
        "cwd/literal.md",
        "---\nnote: \"a {{{ expr }}}\"\n---\nbody\n",
    );

    let output = fixture
        .command()
        .arg("compose")
        .arg("--frontmatter")
        .arg(&path)
        .assert()
        .success()
        .get_output()
        .clone();
    let stdout = String::from_utf8(output.stdout).expect("compose stdout is UTF-8");
    assert!(stdout.contains("note: a {{ expr }}\n"), "{stdout}");
    assert!(!stdout.contains("{{{"), "{stdout}");
}

/// Control: a file that also has a real span converts its literal too.
#[test]
fn literal_beside_a_real_span_converts() {
    let fixture = CliProcessFixture::named("literal_beside_a_real_span_converts");
    let path = fixture.write_file(
        "cwd/mixed.md",
        "---\nx: 1\n---\nreal {{ x }} and {{{ expr }}}\n",
    );

    assert_eq!(compose(&fixture, &path).trim_end(), "real 1 and {{ expr }}");
}
