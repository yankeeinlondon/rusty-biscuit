//! End-to-end `md compose` coverage for Requirement 5 of the
//! dasherized-identifiers spec
//! (`darkmatter/features/2026-09-15-dasherized-identifiers`): each issue is
//! reported on stderr exactly once per source document.

mod common;

use common::CliProcessFixture;

fn warning_count(stderr: &[u8], needle: &str) -> usize {
    String::from_utf8_lossy(stderr).matches(needle).count()
}

/// One document referencing an unknown `ctx.*` group in frontmatter and ten
/// times in its body, transcluding two children that each reference it too
/// (one of them twice): one warning for the root and one per child.
#[test]
fn compose_reports_an_unknown_root_once_per_source_document() {
    let fixture = CliProcessFixture::named("compose_reports_an_unknown_root_once_per_source_document");
    fixture.write_file("cwd/a.md", "A {{ ctx.toady }} {{ ctx.toady }}\n");
    fixture.write_file("cwd/b.md", "B {{ ctx.toady }}\n");
    let body = "- {{ ctx.toady }}\n".repeat(10);
    let root = fixture.write_file(
        "cwd/root.md",
        &format!(
            "---\nlabel: \"x {{{{ ctx.toady }}}}\"\n---\n{body}\n::file ./a.md\n\n::file ./b.md\n\n::file ./a.md\n"
        ),
    );

    let output = fixture.command().arg("compose").arg(&root).output().unwrap();

    assert!(output.status.success(), "warnings do not fail composition: {output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout.lines().filter(|line| *line == "A").count(), 2, "{stdout}");
    assert_eq!(stdout.lines().filter(|line| *line == "B").count(), 1, "{stdout}");
    assert_eq!(
        warning_count(&output.stderr, "unknown context variable 'ctx.toady'"),
        3,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// The original rescan fixture through the binary: one successful replacement
/// beside one bad expression is one fatal error, not one per pass, with its
/// source line and nothing on stdout.
#[test]
fn compose_reports_a_bad_expression_beside_a_replacement_once() {
    let fixture =
        CliProcessFixture::named("compose_reports_a_bad_expression_beside_a_replacement_once");
    let document =
        fixture.write_file("cwd/doc.md", "---\ntitle: Hello\n---\n{{ title }} {{ > invalid }}\n");

    let output = fixture.command().arg("compose").arg(&document).output().unwrap();

    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(output.stdout.is_empty(), "{}", String::from_utf8_lossy(&output.stdout));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(warning_count(&output.stderr, "MarkdownError:"), 1, "{stderr}");
    assert_eq!(warning_count(&output.stderr, "`> invalid`"), 1, "{stderr}");
    assert!(stderr.contains("Expression at line: 4"), "{stderr}");
}
