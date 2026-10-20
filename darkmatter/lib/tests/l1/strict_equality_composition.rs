//! Strict typed `==`/`!=` reaches every composition surface: frontmatter and
//! body interpolation, `::block when=` selection, and a guarded transclusion,
//! each composed from a real document in its own temp dir.

use std::path::{Path, PathBuf};

use darkmatter::markdown::compose::{ComposeContext, ComposeOptions};
use darkmatter::markdown::{Markdown, MarkdownError};
use tempfile::TempDir;

fn write(dir: &Path, name: &str, content: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, content).unwrap();
    path
}

/// Composes `content` as `doc.md` beside the given extra files.
fn compose_with_files(content: &str, files: &[(&str, &str)]) -> Result<String, MarkdownError> {
    let dir = TempDir::new().unwrap();
    for (name, body) in files {
        write(dir.path(), name, body);
    }
    let path = write(dir.path(), "doc.md", content);
    let options = ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir.path(), ""))
        .with_source_file(path.clone());
    Markdown::try_from(path.as_path())
        .expect("document loads")
        .compose_with(&crate::request_support::request(options))
        .map(|(composed, _)| composed.content().trim_end().to_string())
}

fn compose(content: &str) -> String {
    compose_with_files(content, &[]).unwrap_or_else(|error| panic!("{content}: {error}"))
}

/// Each row: frontmatter, interpolated expression, expected render.
#[test]
fn interpolation_compares_typed_values() {
    let rows = [
        // A frontmatter value interpolated before the body reads it.
        ("count: 5\nlabel: \"{{ count == '5' ? 'text' : 'typed' }}\"", "label", "typed"),
        ("count: 5\nlabel: \"{{ count == 5 ? 'match' : 'typed' }}\"", "label", "match"),
        ("label: \"{{ missing == null ? 'absent' : 'present' }}\"", "label", "absent"),
        ("flag: true", "flag == 'true' ? 'a' : 'b'", "b"),
        ("flag: \"true\"", "flag == 'true' ? 'a' : 'b'", "a"),
        ("items: []", "items == '[]' ? 'a' : 'b'", "b"),
        ("items: []", "items == [] ? 'a' : 'b'", "a"),
    ];
    for (frontmatter, expression, expected) in rows {
        let content = compose(&format!("---\n{frontmatter}\n---\n[{{{{ {expression} }}}}]\n"));
        assert_eq!(content, format!("[{expected}]"), "{frontmatter} / {expression}");
    }
}

/// Each row: frontmatter, block condition, whether the block is kept. A
/// missing `note` is `null`, which equals only `null`.
#[test]
fn block_conditions_select_by_strict_equality() {
    let rows = [
        ("", "note != ''", true),
        ("", "note != null", false),
        ("note: \"\"", "note != null", true),
        ("note: \"\"", "note != ''", false),
    ];
    for (frontmatter, condition, kept) in rows {
        let content = compose(&format!(
            "---\n{frontmatter}\n---\n::block when=\"{condition}\"\nINSIDE\n::end-block\nafter\n"
        ));
        assert_eq!(content.contains("INSIDE"), kept, "{frontmatter:?} / {condition}: {content}");
        assert!(content.contains("after"), "{condition}: {content}");
    }
}

#[test]
fn a_null_guard_transcludes_only_a_present_part() {
    let body = "::block when=\"part != null\"\n::file {{part}}\n::end-block\nafter\n";
    let files = [("part.md", "PART BODY\n")];

    let present = compose_with_files(&format!("---\npart: part.md\n---\n{body}"), &files).unwrap();
    assert!(present.contains("PART BODY"), "{present}");

    let absent = compose_with_files(&format!("---\ntitle: t\n---\n{body}"), &files).unwrap();
    assert!(!absent.contains("PART BODY"), "{absent}");
    assert!(absent.contains("after"), "{absent}");
}

/// A comparison does not swallow an operand failure, and a ternary branch
/// that is not selected is never evaluated.
#[test]
fn an_operand_failure_survives_the_comparison_and_a_skipped_branch_never_runs() {
    let error = compose_with_files("---\nflag: true\n---\n{{ (flag + 1) == 5 }}\n", &[])
        .expect_err("the operand fails");
    assert!(error.to_string().contains("flag + 1"), "{error}");

    assert_eq!(compose("---\nflag: true\n---\n[{{ false ? flag + 1 : 'ok' }}]\n"), "[ok]");
}

/// Schema coercion runs before the body is composed, so a quoted value
/// declared `number` compares as a number. This is the shape of the
/// shipped `phase == total_phases` blocks: `total_phases` is read from a
/// plan whose value may be quoted, and `phase` may arrive as CLI text.
#[test]
fn a_number_declared_value_compares_as_a_number_after_coercion() {
    let schema = "$schema:\n    phase: number\n    total_phases: number\n";
    let body = "::block when=\"phase == total_phases\"\nFINAL\n::end-block\n[{{ is_number(phase) }}]\n";
    let rows = [
        ("phase: \"3\"\ntotal_phases: 3", true),
        ("phase: \"2\"\ntotal_phases: 3", false),
        ("phase: 3\ntotal_phases: \"{{ frontmatter('plan.md', 'total_phases') }}\"", true),
    ];
    let files = [("plan.md", "---\ntotal_phases: \"3\"\n---\n")];
    for (frontmatter, final_phase) in rows {
        let content = compose_with_files(&format!("---\n{schema}{frontmatter}\n---\n{body}"), &files)
            .unwrap_or_else(|error| panic!("{frontmatter}: {error}"));
        assert_eq!(content.contains("FINAL"), final_phase, "{frontmatter}: {content}");
        assert!(content.ends_with("[true]"), "{frontmatter}: {content}");
    }

    // Undeclared, the quoted value stays text and no longer equals a number.
    let undeclared = compose(&format!("---\nphase: \"3\"\ntotal_phases: 3\n---\n{body}"));
    assert!(!undeclared.contains("FINAL"), "{undeclared}");
    assert!(undeclared.ends_with("[false]"), "{undeclared}");
}
