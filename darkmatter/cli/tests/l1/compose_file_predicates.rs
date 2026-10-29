//! End-to-end `md compose` coverage for the file predicates: `is_file_type`
//! answers from the reference's syntax alone, its guard lets an `unknown`
//! value reach `frontmatter`'s file parameter (as does `file_exists`, whose
//! catalog declaration narrows to `file`), and a `file_exists` call whose
//! literal argument is malformed composes to `false` with one always-false
//! warning on stderr naming the file and line.

use crate::common;

use common::CliProcessFixture;

const DOCUMENT: &str = "---
$schema:
  source: unknown
source: ./target.md
---
[{{ is_file_type(source) }} {{ is_file_type('https://') }} {{ is_file_type(42) }}]

[{{ is_file_type(source) ? frontmatter(source, 'title') : 'none' }}]

[{{ file_exists(source) ? frontmatter(source, 'title') : 'none' }}]

[{{ file_exists('https://') }} {{ fileexists('target.md') }}]
";

#[test]
fn compose_answers_file_predicates_and_warns_for_an_always_false_call() {
    let fixture = CliProcessFixture::named("compose_file_predicates");
    fixture.write_file("cwd/target.md", "---\ntitle: Target\n---\n");
    let document = fixture.write_file("cwd/doc.md", DOCUMENT);

    let output = fixture.command().arg("compose").arg(&document).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(output.status.code(), Some(0), "stderr: {stderr}");
    assert_eq!(stdout.trim_end(), "[true false false]\n\n[Target]\n\n[Target]\n\n[false true]", "stderr: {stderr}");
    assert_eq!(stderr.matches("always false").count(), 1, "{stderr}");
    assert!(stderr.contains("doc.md:12"), "the call's file and line: {stderr}");
    assert!(!stdout.contains("always false"), "warnings stay on stderr: {stdout}");
}
