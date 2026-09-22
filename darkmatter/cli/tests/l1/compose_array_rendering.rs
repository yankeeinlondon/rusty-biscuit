use crate::common;

use common::CliProcessFixture;
use predicates::prelude::*;

#[test]
fn compose_body_renders_bare_array_as_compact_json() {
    let fixture = CliProcessFixture::named("compose_body_renders_bare_array_as_compact_json");
    fixture
        .command()
        .args(["compose", "-"])
        .write_stdin("---\nitems:\n  - a\n  - b\n---\nItems: {{ items }}\n")
        .assert()
        .success()
        .stdout(predicate::str::contains(r#"Items: ["a","b"]"#));
}

/// Body paragraphs reflow soft line breaks, so the newline-joined form is
/// pinned through frontmatter, where the block scalar preserves it verbatim.
#[test]
fn compose_keeps_as_line_separated_newline_joined() {
    let fixture = CliProcessFixture::named("compose_keeps_as_line_separated_newline_joined");
    fixture
        .command()
        .args(["compose", "-", "--frontmatter"])
        .write_stdin(
            "---\nitems:\n  - a\n  - b\njoined: \"{{ as_line_separated(items) }}\"\nmixed: \"x {{ items }}\"\n---\nBody\n",
        )
        .assert()
        .success()
        .stdout(predicate::str::contains("joined: |-\n  a\n  b"))
        .stdout(predicate::str::contains(r#"mixed: x ["a","b"]"#));
}
