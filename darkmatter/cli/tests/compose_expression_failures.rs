//! End-to-end `md compose` coverage for Requirement 2 of the
//! dasherized-identifiers spec
//! (`darkmatter/features/2026-09-15-dasherized-identifiers`): a body or
//! mixed-text frontmatter expression that cannot be parsed or evaluated fails
//! the command with one rendered error naming its source line, and nothing
//! reaches stdout.

mod common;

use common::CliProcessFixture;

struct Failure {
    stdout: String,
    stderr: String,
}

/// Runs `md compose` on `content` and requires exit 1 with an empty stdout
/// and exactly one rendered error.
#[track_caller]
fn compose_failure(name: &str, content: &str) -> Failure {
    let fixture = CliProcessFixture::named(name);
    let document = fixture.write_file("cwd/doc.md", content);

    let output = fixture.command().arg("compose").arg(&document).output().unwrap();

    let failure = Failure {
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    };
    assert_eq!(output.status.code(), Some(1), "stderr: {}", failure.stderr);
    assert!(failure.stdout.is_empty(), "no partial document on stdout: {}", failure.stdout);
    assert_eq!(failure.stderr.matches("MarkdownError:").count(), 1, "{}", failure.stderr);
    // The rendered path wraps at the block width, so assert the locus heading
    // rather than a file name a long temp directory could split.
    assert!(failure.stderr.contains("Defined in:"), "the error names the file: {}", failure.stderr);
    failure
}

#[test]
fn compose_fails_on_a_body_parse_failure() {
    let failure = compose_failure(
        "compose_fails_on_a_body_parse_failure",
        "---\ntitle: Hello\n---\n# {{ title }}\n\nHello {{ 1 + }} world.\n",
    );

    assert!(failure.stderr.contains("Expression at line: 6, column: 7"), "{}", failure.stderr);
    assert!(failure.stderr.contains("> 6 │ Hello {{ 1 + }} world."), "{}", failure.stderr);
    assert!(!failure.stdout.contains("{{ 1 + }}"));
}

#[test]
fn compose_fails_on_a_body_evaluation_failure() {
    let failure = compose_failure(
        "compose_fails_on_a_body_evaluation_failure",
        "---\nflag: true\n---\nResult: {{ flag + 1 }}\n",
    );

    assert!(failure.stderr.contains("Addition requires numeric operands"), "{}", failure.stderr);
    assert!(failure.stderr.contains("Expression at line: 4"), "{}", failure.stderr);
}

#[test]
fn compose_fails_on_a_mixed_frontmatter_failure() {
    let failure = compose_failure(
        "compose_fails_on_a_mixed_frontmatter_failure",
        "---\ntitle: Hello\nnote: \"x {{ min(1) }} y\"\n---\nbody {{ note }}\n",
    );

    assert!(failure.stderr.contains("min() requires 2 arguments"), "{}", failure.stderr);
    assert!(failure.stderr.contains("Expression at line: 3, column: 10"), "{}", failure.stderr);
    assert!(failure.stderr.contains("3 │ note: \"x {{ min(1) }} y\""), "{}", failure.stderr);
    assert!(!failure.stderr.contains("title:"), "only the failing key is excerpted: {}", failure.stderr);
}

/// A failure inside a literal block scalar names the expression's own line
/// and column, not just the key.
#[test]
fn compose_fails_on_a_block_scalar_frontmatter_failure_with_its_locus() {
    let failure = compose_failure(
        "compose_fails_on_a_block_scalar_frontmatter_failure_with_its_locus",
        "---\ntitle: Hello\nnote: |\n  first\n  x {{ min(1) }} y\n---\nbody {{ note }}\n",
    );

    assert!(failure.stderr.contains("min() requires 2 arguments"), "{}", failure.stderr);
    assert!(failure.stderr.contains("Expression at line: 5, column: 5"), "{}", failure.stderr);
}

/// A tag is valid YAML before a string value; the locus is the expression
/// behind it, not the key or the tag.
#[test]
fn compose_fails_on_a_tagged_frontmatter_failure_with_its_locus() {
    let failure = compose_failure(
        "compose_fails_on_a_tagged_frontmatter_failure_with_its_locus",
        "---\ntitle: !!str \"prefix {{ upper( }}\"\n---\nbody {{ title }}\n",
    );

    assert!(failure.stderr.contains("Expression at line: 2, column: 22"), "{}", failure.stderr);
    assert!(!failure.stdout.contains("{{ upper( }}"));
}

/// An expression authored once behind an anchor and reused through an alias
/// fails once, at the line and column where it is authored.
#[test]
fn compose_fails_once_on_an_aliased_frontmatter_failure_at_its_anchor() {
    let failure = compose_failure(
        "compose_fails_once_on_an_aliased_frontmatter_failure_at_its_anchor",
        "---\ndefaults:\n  greeting: &shared \"café — {{ min(1) }}\"\ntitle: *shared\n---\nbody {{ title }}\n",
    );

    assert!(failure.stderr.contains("min() requires 2 arguments"), "{}", failure.stderr);
    assert!(failure.stderr.contains("Expression at line: 3, column: 29"), "{}", failure.stderr);
    assert_eq!(failure.stderr.matches("Expression at line:").count(), 1, "{}", failure.stderr);
}

/// When the alias's definition cannot be proven (the anchor's name also
/// appears in a comment), the locus is the alias token, not the bare key. The
/// anchor sits on a mapping key, which composition never interpolates, so the
/// alias is the value that fails.
#[test]
fn compose_fails_on_an_unprovable_alias_at_the_alias_token() {
    let failure = compose_failure(
        "compose_fails_on_an_unprovable_alias_at_the_alias_token",
        "---\n? &shared \"x {{ min(1) }}\"\n: held # &shared\ncafé: *shared\n---\nbody\n",
    );

    assert!(failure.stderr.contains("min() requires 2 arguments"), "{}", failure.stderr);
    assert!(failure.stderr.contains("Expression at line: 4, column: 7"), "{}", failure.stderr);
    assert_eq!(failure.stderr.matches("Expression at line:").count(), 1, "{}", failure.stderr);
}

/// A successful replacement beside the failure does not leak onto stdout.
#[test]
fn compose_emits_no_partially_rewritten_document() {
    let failure = compose_failure(
        "compose_emits_no_partially_rewritten_document",
        "---\ntitle: Hello\n---\n{{ title }}\n\n{{ length(1, 2) }}\n",
    );

    assert!(!failure.stdout.contains("Hello"));
    assert!(failure.stderr.contains("Expression at line: 6"), "{}", failure.stderr);
}
