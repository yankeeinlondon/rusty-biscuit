//! End-to-end `md compose` coverage for the schema-enhancements function
//! migration: calls whose arguments the coercion engine converts compose to
//! their converted results, and a call that cannot bind fails the command
//! with the binder's type error.

use crate::common;

use common::CliProcessFixture;

/// Numeric text converts for `min`, a number converts to text for `upper`,
/// `number` recovers through its fallback, and `keys` keeps the frontmatter
/// map's written key order.
#[test]
fn converted_calls_compose_to_their_results() {
    let fixture = CliProcessFixture::named("converted_calls_compose_to_their_results");
    let document = fixture.write_file(
        "cwd/doc.md",
        "---\nmeta:\n  b: 1\n  a: 2\n---\n{{ min(\"4\", 5) }}\n\n{{ upper(2026) }}\n\n{{ number(\"pear\", 7) }}\n\n{{ keys(meta) }}\n",
    );

    let output = fixture.command().arg("compose").arg(&document).output().unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "stderr: {stderr}");
    assert_eq!(String::from_utf8_lossy(&output.stdout), "4\n\n2026\n\n7\n\n[\"b\",\"a\"]\n");
}

/// `number("pear")` has no fallback, so the call does not bind: the command
/// fails with the binder's type error and writes no document.
#[test]
fn an_unbindable_call_fails_compose_with_the_type_error() {
    let fixture = CliProcessFixture::named("an_unbindable_call_fails_compose_with_the_type_error");
    let document = fixture.write_file("cwd/doc.md", "---\ntitle: Pear\n---\n{{ number(\"pear\") }}\n");

    let output = fixture.command().arg("compose").arg(&document).output().unwrap();

    let stderr = biscuit_test_harness::strip_ansi(&String::from_utf8_lossy(&output.stderr));
    assert_eq!(output.status.code(), Some(1), "stderr: {stderr}");
    assert!(output.stdout.is_empty(), "no document on stdout: {}", String::from_utf8_lossy(&output.stdout));
    for part in [
        "invalid arguments for number(x: numberlike, fallback?: number)",
        "x (argument 1): expected numberlike; received text \"pear\"",
        "conversion failed: not a number",
        "Expression at line: 4, column: 1",
    ] {
        assert!(stderr.contains(part), "{part:?} missing from:\n{stderr}");
    }
}
