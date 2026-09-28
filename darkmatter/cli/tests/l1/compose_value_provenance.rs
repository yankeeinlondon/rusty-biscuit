//! Whether a filled-in value is re-read as a template
//! (`2026-09-27-agent-text-is-data`).
//!
//! The ignored tests reproduce the defect and assert the fixed behavior. The
//! others pin ruling N1: command-line setters remain templates.

use crate::common;

use common::CliProcessFixture;
use predicates::prelude::*;

#[test]
#[ignore = "red until phase 2"]
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
#[ignore = "red until phase 2"]
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

    fixture
        .command()
        .arg("compose")
        .arg(&md_path)
        .args(["--set", r#"{"note":"see {{…}} siblings"}"#])
        .assert()
        .failure()
        .stderr(predicate::str::contains("interpolation failed"))
        .stderr(predicate::str::contains("note"));
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
#[ignore = "red until phase 2"]
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
#[ignore = "red until phase 2"]
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
#[ignore = "red until phase 2"]
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
#[ignore = "red until phase 2"]
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
#[ignore = "red until phase 2"]
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
#[ignore = "red until phase 2"]
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
