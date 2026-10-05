//! Whether a filled-in value is re-read as a template
//! (`2026-09-27-agent-text-is-data`).
//!
//! Text an expression, a file read, or a literal escape produced is data: it
//! is never scanned again, in the document or in a transcluded child. The
//! `--set` tests pin ruling N1: command-line setters remain templates, and a
//! failure in one names the override rather than the document (R5).

use crate::common;

use common::CliProcessFixture;
use darkmatter::testing::strip_ansi_codes;
use predicates::prelude::*;

#[test]
fn escaped_expression_in_frontmatter_stays_literal_in_body() {
    let fixture =
        CliProcessFixture::named("escaped_expression_in_frontmatter_stays_literal_in_body");
    let md_path = fixture.write_file(
        "cwd/esc.md",
        "---\narea: claudine\nnote: \"fixed {{{ area }}}\"\n---\nBody: {{ note }}\n",
    );

    fixture
        .command()
        .arg("compose")
        .arg(&md_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Body: fixed {{ area }}"));
}

#[test]
fn escaped_non_expression_in_frontmatter_stays_literal_in_body() {
    let fixture =
        CliProcessFixture::named("escaped_non_expression_in_frontmatter_stays_literal_in_body");
    let md_path = fixture.write_file(
        "cwd/esc.md",
        "---\narea: claudine\nnote: \"fixed {{{…}}}\"\n---\nBody: {{ note }}\n",
    );

    fixture
        .command()
        .arg("compose")
        .arg(&md_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Body: fixed {{…}}"));
}

#[test]
fn set_value_with_malformed_template_still_fails() {
    let fixture = CliProcessFixture::named("set_value_with_malformed_template_still_fails");
    let md_path = fixture.write_file("cwd/repro.md", "---\ntitle: t\n---\nLast: {{ note }}\n");

    let output = fixture
        .command()
        .arg("compose")
        .arg(&md_path)
        .args(["--set", r#"{"note":"see {{…}} siblings"}"#])
        .output()
        .expect("md compose should run");

    assert!(!output.status.success(), "expected a failure exit status");
    // The supplier's code span is styled (dim) even without color support.
    let stderr = strip_ansi_codes(&String::from_utf8_lossy(&output.stderr));
    assert!(stderr.contains("interpolation failed"), "stderr: {stderr}");
    assert!(stderr.contains("note"), "stderr: {stderr}");
    assert!(stderr.contains("command-line override (--set)"), "stderr: {stderr}");
    assert!(!stderr.contains("Defined in:"), "stderr: {stderr}");
}

/// R5 control: a malformed template the document authors keeps its
/// document location.
#[test]
fn authored_malformed_template_still_names_the_document() {
    let fixture = CliProcessFixture::named("authored_malformed_template_still_names_the_document");
    let md_path = fixture.write_file(
        "cwd/authored.md",
        "---\nnote: \"see {{…}} siblings\"\n---\nLast: {{ note }}\n",
    );

    fixture
        .command()
        .arg("compose")
        .arg(&md_path)
        .assert()
        .failure()
        .stderr(predicate::str::contains("interpolation failed"))
        .stderr(predicate::str::contains("Defined in:"))
        .stderr(predicate::str::contains("authored.md"))
        .stderr(predicate::str::contains("command-line override").not());
}

/// N14: a link inside inserted text is content, so a missing target warns
/// instead of failing the run; the same link authored in the body still
/// fails reference validation.
#[test]
fn missing_link_inside_inserted_text_warns() {
    let fixture = fixture_with_data_file("missing_link_inside_inserted_text_warns");
    fixture.write_file("cwd/links.md", "---\nlink: \"[l](./nope.md)\"\n---\n");
    let data_path = fixture.write_file(
        "cwd/data-link.md",
        "---\ntitle: t\n---\nsee {{ frontmatter('links.md', 'link') }}\n",
    );
    let authored_path =
        fixture.write_file("cwd/authored-link.md", "---\ntitle: t\n---\nsee [l](./nope.md)\n");

    fixture
        .command()
        .arg("compose")
        .arg(&data_path)
        .assert()
        .success()
        .stderr(predicate::str::contains("Missing local target: ./nope.md"))
        .stderr(predicate::str::contains("inside inserted text"));

    fixture.command().arg("compose").arg(&authored_path).assert().code(2);
}

#[test]
fn set_value_template_still_fills_in() {
    let fixture = CliProcessFixture::named("set_value_template_still_fills_in");
    let md_path = fixture.write_file("cwd/x.md", "---\ntitle: t\n---\nX: {{ x }}\n");

    fixture
        .command()
        .arg("compose")
        .arg(&md_path)
        .args(["--set", r#"{"x":"{{ title }}"}"#])
        .assert()
        .success()
        .stdout(predicate::str::contains("X: t"));
}

#[test]
fn set_value_shell_command_requires_approval_when_non_interactive() {
    let fixture =
        CliProcessFixture::named("set_value_shell_command_requires_approval_when_non_interactive");
    let md_path = fixture.write_file("cwd/repro.md", "---\ntitle: t\n---\nLast: {{ note }}\n");

    fixture
        .command()
        .arg("compose")
        .arg(&md_path)
        .args(["--set", r#"{"note":"$(echo INJECTED)"}"#])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Approval required for 'echo INJECTED'.",
        ))
        .stdout(predicate::str::contains("INJECTED").not());
}

/// A fixture whose `data.md` frontmatter holds non-authored values for
/// `frontmatter('data.md', …)` reads.
fn fixture_with_data_file(name: &str) -> CliProcessFixture {
    let fixture = CliProcessFixture::named(name);
    fixture.write_file(
        "cwd/data.md",
        "---\npayload: \"ok\\n::file ./secret.md\"\nbraces: \"see {{…}}\"\nfence: \"```\"\n---\n",
    );
    fixture.write_file("cwd/secret.md", "TOP-SECRET-CONTENTS\n");
    fixture
}

/// B1: a directive line inside read data is text, not a transclusion.
#[test]
fn directive_in_interpolated_file_data_is_not_executed() {
    let fixture = fixture_with_data_file("directive_in_interpolated_file_data_is_not_executed");
    let md_path = fixture.write_file(
        "cwd/b1.md",
        "---\ntitle: t\n---\nBefore\n\n{{ frontmatter('data.md', 'payload') }}\n\nAfter\n",
    );

    fixture
        .command()
        .arg("compose")
        .arg(&md_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("::file ./secret.md"))
        .stdout(predicate::str::contains("TOP-SECRET-CONTENTS").not());
}

/// B1: a code fence inside read data must not hide a later authored directive.
#[test]
fn fence_in_interpolated_file_data_does_not_hide_authored_directive() {
    let fixture =
        fixture_with_data_file("fence_in_interpolated_file_data_does_not_hide_authored_directive");
    let md_path = fixture.write_file(
        "cwd/fence.md",
        "---\ntitle: t\n---\n{{ frontmatter('data.md', 'fence') }}\n\n::file ./secret.md\n",
    );

    fixture
        .command()
        .arg("compose")
        .arg(&md_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("TOP-SECRET-CONTENTS"));
}

/// B4: inherited parent state is data in the child.
#[test]
fn transcluded_child_does_not_reevaluate_inherited_escape() {
    let fixture =
        CliProcessFixture::named("transcluded_child_does_not_reevaluate_inherited_escape");
    fixture.write_file("cwd/child.md", "Child: {{ note }}\n");
    let md_path = fixture.write_file(
        "cwd/parent.md",
        "---\narea: x\nnote: \"fixed {{{ area }}}\"\n---\n::file ./child.md\n",
    );

    fixture
        .command()
        .arg("compose")
        .arg(&md_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Child: fixed {{ area }}"));
}

/// B4: a parent value read from another file reaches the child verbatim.
#[test]
fn transcluded_child_prints_inherited_file_data_verbatim() {
    let fixture = fixture_with_data_file("transcluded_child_prints_inherited_file_data_verbatim");
    fixture.write_file("cwd/child.md", "Child: {{ braces }}\n");
    let md_path = fixture.write_file(
        "cwd/parent.md",
        "---\nbraces: \"{{ frontmatter('data.md', 'braces') }}\"\n---\n::file ./child.md\n",
    );

    fixture
        .command()
        .arg("compose")
        .arg(&md_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Child: see {{…}}"))
        .stdout(predicate::str::contains("Could not transclude").not());
}

/// B5: a `set.NAME=` value produced by a literal escape is data in the child.
#[test]
fn directive_set_value_from_escape_is_not_reevaluated_in_child() {
    let fixture =
        CliProcessFixture::named("directive_set_value_from_escape_is_not_reevaluated_in_child");
    fixture.write_file(
        "cwd/child.md",
        "---\nactor: default\n---\nActor: {{ actor }}\n",
    );
    let md_path = fixture.write_file(
        "cwd/parent.md",
        "---\narea: x\n---\n::file ./child.md set.actor=\"see {{{…}}}\"\n",
    );

    fixture
        .command()
        .arg("compose")
        .arg(&md_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Actor: see {{…}}"))
        .stdout(predicate::str::contains("Could not transclude").not());
}

/// B5: a `set.NAME=` value interpolated from file data reaches the child
/// verbatim.
#[test]
fn directive_set_value_from_file_data_reaches_child_verbatim() {
    let fixture =
        fixture_with_data_file("directive_set_value_from_file_data_reaches_child_verbatim");
    fixture.write_file(
        "cwd/child.md",
        "---\nactor: default\n---\nActor: {{ actor }}\n",
    );
    let md_path = fixture.write_file(
        "cwd/parent.md",
        "---\nx: \"{{ frontmatter('data.md', 'braces') }}\"\n---\n::file ./child.md set.actor=\"{{ x }}\"\n",
    );

    fixture
        .command()
        .arg("compose")
        .arg(&md_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("Actor: see {{…}}"));
}

/// R2 end to end: tokens holding a template, a command, and a token
/// look-alike print verbatim. Stdin is not a terminal, so a shell approval
/// request would fail the run.
#[test]
fn literal_tokens_compose_to_their_exact_text() {
    use darkmatter::markdown::literal_token::{encode, encode_yaml_scalar};

    let fixture = CliProcessFixture::named("literal_tokens_compose_to_their_exact_text");
    let md_path = fixture.write_file(
        "cwd/tokens.md",
        &format!(
            "---\narea: claudine\nsummary: {}\ncmd: {}\nlookalike: {}\n---\n\
             S={{{{ summary }}}} C={{{{ cmd }}}} L={{{{ lookalike }}}} A={{{{ area }}}}\n",
            encode_yaml_scalar("fixed {{ area }} parsing"),
            encode_yaml_scalar("$(echo X)"),
            encode_yaml_scalar(&encode("x")),
        ),
    );

    fixture
        .command()
        .arg("compose")
        .arg(&md_path)
        .assert()
        .success()
        .stdout(predicate::str::contains(format!(
            "S=fixed {{{{ area }}}} parsing C=$(echo X) L={} A=claudine",
            encode("x")
        )));
}

/// R2 end to end: a malformed token fails with its location, and is never
/// read as an expression.
#[test]
fn malformed_literal_token_fails_with_its_location() {
    let fixture = CliProcessFixture::named("malformed_literal_token_fails_with_its_location");
    let md_path = fixture.write_file(
        "cwd/bad.md",
        "---\ntitle: t\nnote: \"{{!data:v2:YQ}}\"\n---\nx\n",
    );

    let output = fixture
        .command()
        .arg("compose")
        .arg(&md_path)
        .output()
        .expect("md compose should run");

    assert!(!output.status.success(), "expected a failure exit status");
    // The reason's code span is styled (dim) even without color support.
    let stderr = strip_ansi_codes(&String::from_utf8_lossy(&output.stderr));
    assert!(stderr.contains("malformed literal token"), "stderr: {stderr}");
    assert!(stderr.contains("unsupported literal token version v2"), "stderr: {stderr}");
    assert!(stderr.contains("line: 3, column: 8"), "stderr: {stderr}");
    assert!(!stderr.contains("parse error"), "stderr: {stderr}");
}
