//! An inclusion composition leaves out because its target did not resolve
//! (an absent target under the missing-transclusion recovery, or any target
//! failure in a document that tolerates invalid references, including a URL
//! while remote execution is disabled) is reported as a warning that keeps
//! the failure's typed code and the inclusion's authored site: the body
//! directive's opener, or the `prologue`/`epilogue` value (each list entry
//! its own), in the file that wrote it. The expected record is the one the
//! same document fails with when nothing tolerates it, with warning
//! severity.

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::expression::{Diagnostic, Severity};
use darkmatter::markdown::compose::report_diagnostics::{error_diagnostics, warning_diagnostic};
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions, ComposeRequest};
use std::path::Path;
use tempfile::TempDir;

const MISSING_TRANSCLUSION: &str = "dm.reference.missing_transclusion";
const TRANSCLUSION_FAILURE: &str = "dm.transclusion.failure";
/// A well-formed loopback URL on the discard port: remote execution is
/// disabled, so nothing may connect to it.
const LOOPBACK: &str = "http://127.0.0.1:9/x.md";

/// What tolerates the failure.
#[derive(Debug, Clone, Copy)]
enum Tolerance {
    None,
    Missing,
    IgnoreInvalid,
}

/// A temporary repository holding `side.md` and the non-Markdown `side.txt`.
fn repository() -> TempDir {
    let repo = TempDir::new().unwrap();
    std::fs::create_dir_all(repo.path().join(".git")).unwrap();
    std::fs::write(repo.path().join("side.md"), "# Side\n").unwrap();
    std::fs::write(repo.path().join("side.txt"), "side\n").unwrap();
    repo
}

/// A request in the repository's frozen context under its own home, with
/// remote execution left disabled.
fn request(root: &Path, tolerance: Tolerance) -> ComposeRequest {
    let home = root.join(".home");
    std::fs::create_dir_all(&home).unwrap();
    let context = FileResolutionContext::from_snapshot(root, None, std::collections::HashMap::new())
        .with_repository_root(root);
    let options = ComposeOptions::new_with_context(ComposeContext::capture_for_content(root, ""))
        .with_config_snapshot(crate::request_support::shell_config(&home, &[]))
        .with_allow_missing_transclusions(matches!(tolerance, Tolerance::Missing))
        .with_ignore_invalid_references(matches!(tolerance, Tolerance::IgnoreInvalid).then_some(true));
    crate::request_support::request_in(options, context)
}

fn document(repo: &TempDir, name: &str, content: &str) -> Markdown {
    let path = repo.path().join(name);
    std::fs::write(&path, content).unwrap();
    Markdown::try_from(path.as_path()).unwrap()
}

/// A record as `(code, file name, line, column)`.
type Site = (String, Option<String>, Option<usize>, Option<usize>);

fn site(record: &Diagnostic) -> Site {
    let source = record.source.as_ref();
    let file = source
        .and_then(|source| source.file.as_ref())
        .and_then(|file| file.file_name())
        .map(|name| name.to_string_lossy().into_owned());
    (record.code.clone(), file, source.and_then(|source| source.line), source.and_then(|source| source.column))
}

/// The one record `md` fails with when nothing tolerates the failure.
fn hard_error(repo: &TempDir, md: &Markdown, shape: &str) -> Site {
    let error = md.compose_with(&request(repo.path(), Tolerance::None)).expect_err(shape);
    let records = error_diagnostics(&error);
    let [record] = records.as_slice() else { panic!("{shape}: {error}: {records:#?}") };
    site(record)
}

/// The one warning `md` composes with under `tolerance`.
fn tolerated(repo: &TempDir, md: &Markdown, tolerance: Tolerance, shape: &str) -> Site {
    let (_, report) = md
        .compose_with(&request(repo.path(), tolerance))
        .unwrap_or_else(|error| panic!("{shape}: {tolerance:?}: {error}"));
    let records: Vec<_> = report.warnings.iter().map(warning_diagnostic).collect();
    let [record] = records.as_slice() else { panic!("{shape}: {tolerance:?}: {records:#?}") };
    assert_eq!(record.severity, Severity::Warning, "{shape}: {record:#?}");
    site(record)
}

/// Asserts `content`, written as `doc.md`, fails with `code` at `line` and,
/// under `tolerance`, composes with one warning at the same site.
fn assert_same_site(repo: &TempDir, content: &str, tolerance: Tolerance, code: &str, line: usize, column: usize) {
    let md = document(repo, "doc.md", content);
    let expected = (code.to_string(), Some("doc.md".to_string()), Some(line), Some(column));
    assert_eq!(hard_error(repo, &md, content), expected, "hard error: {content}");
    assert_eq!(tolerated(repo, &md, tolerance, content), expected, "warning: {content}: {tolerance:?}");
}

/// A YAML header, then `directive` on file line 8.
fn plain(directive: &str) -> (String, usize) {
    (format!("---\ntitle: x\nother: y\n---\n\n# Doc\n\n{directive}\n"), 8)
}

/// The same header and an earlier false page block, then `directive` on
/// file line 11: composition removes the block before the directive runs.
fn after_removed_block(directive: &str) -> (String, usize) {
    (format!("---\ntitle: x\nother: y\n---\n\n::block when=\"false\"\nhidden\n::end-block\n# Doc\n\n{directive}\n"), 11)
}

/// `key` written as `value`, starting on line 3.
fn frontmatter(key: &str, value: &str) -> String {
    format!("---\ntitle: x\n{key}: {value}\n---\n\n# Doc\n")
}

/// R14-F2.1: a missing `::file` or `::code`, bare and quoted, after a header
/// and after a removed block, warns at its opener under the recovery and
/// under ignore-invalid.
#[test]
fn a_skipped_missing_body_target_warns_at_its_directive() {
    let repo = repository();
    for kind in ["file", "code"] {
        for target in ["./gone.md", "\"./gone.md\""] {
            let directive = format!("::{kind} {target}");
            for (content, line) in [plain(&directive), after_removed_block(&directive)] {
                for tolerance in [Tolerance::Missing, Tolerance::IgnoreInvalid] {
                    assert_same_site(&repo, &content, tolerance, MISSING_TRANSCLUSION, line, 1);
                }
            }
        }
    }
}

/// R14-F2.1: a `::file` of a text file, and a `::url` of a local path, left
/// out under ignore-invalid keep their typed failure code at the opener.
#[test]
fn a_skipped_invalid_body_target_keeps_its_code_and_directive() {
    let repo = repository();
    for directive in ["::file ./side.txt", "::file \"./side.txt\"", "::url ./side.md"] {
        for (content, line) in [plain(directive), after_removed_block(directive)] {
            assert_same_site(&repo, &content, Tolerance::IgnoreInvalid, TRANSCLUSION_FAILURE, line, 1);
        }
    }
}

/// R14-F2.2: a missing `prologue`/`epilogue` scalar or list entry warns at
/// that value, and a text file left out under ignore-invalid keeps its code
/// there; the valid sibling entry is still included.
#[test]
fn a_skipped_frontmatter_target_warns_at_its_value() {
    let repo = repository();
    for key in ["prologue", "epilogue"] {
        let column = key.len() + 3;
        for (target, code, tolerances) in [
            ("./gone.md", MISSING_TRANSCLUSION, &[Tolerance::Missing, Tolerance::IgnoreInvalid][..]),
            ("./side.txt", TRANSCLUSION_FAILURE, &[Tolerance::IgnoreInvalid][..]),
        ] {
            for tolerance in tolerances {
                assert_same_site(&repo, &frontmatter(key, &format!("\"{target}\"")), *tolerance, code, 3, column);
                let list = frontmatter(key, &format!("\n  - ./side.md\n  - \"{target}\""));
                assert_same_site(&repo, &list, *tolerance, code, 5, 5);
                let md = document(&repo, "doc.md", &list);
                let (composed, _) = md.compose_with(&request(repo.path(), *tolerance)).unwrap();
                let text = composed.content();
                assert!(text.contains("# Side"), "{key}: {target}: the valid sibling is kept: {text}");
            }
        }
    }
}

/// R14-F2.3: a well-formed URL in `::url`, `::file`, `::code`, `prologue`,
/// or `epilogue` with remote execution disabled is left out under
/// ignore-invalid with the hard error's code at its site.
#[test]
fn a_skipped_remote_target_warns_at_its_site() {
    let repo = repository();
    for kind in ["url", "file", "code"] {
        let directive = format!("::{kind} {LOOPBACK}");
        for (content, line) in [plain(&directive), after_removed_block(&directive)] {
            assert_same_site(&repo, &content, Tolerance::IgnoreInvalid, TRANSCLUSION_FAILURE, line, 1);
        }
    }
    for key in ["prologue", "epilogue"] {
        let content = frontmatter(key, &format!("\"{LOOPBACK}\""));
        assert_same_site(&repo, &content, Tolerance::IgnoreInvalid, TRANSCLUSION_FAILURE, 3, key.len() + 3);
    }
}

/// R14-F2.4 (library): a nested child's skipped missing target names the
/// child's file and its line, not the including document.
#[test]
fn a_nested_skipped_target_warns_in_the_child() {
    let repo = repository();
    document(&repo, "nested.md", "---\na: b\n---\n\n# Nested\n\n::code ./gone.rs\n");
    let md = document(&repo, "doc.md", "# Root\n\n::file ./nested.md\n");
    let expected = (MISSING_TRANSCLUSION.to_string(), Some("nested.md".to_string()), Some(7), Some(1));
    assert_eq!(tolerated(&repo, &md, Tolerance::Missing, "nested"), expected);
}

/// Controls: an admissible target leaves no warning under either tolerance.
#[test]
fn a_valid_target_leaves_no_skip_warning() {
    let repo = repository();
    for content in [
        plain("::file ./side.md").0,
        after_removed_block("::code \"./side.md\"").0,
        frontmatter("prologue", "\"./side.md\""),
        frontmatter("epilogue", "\n  - ./side.md"),
    ] {
        let md = document(&repo, "doc.md", &content);
        for tolerance in [Tolerance::None, Tolerance::Missing, Tolerance::IgnoreInvalid] {
            let (_, report) = md.compose_with(&request(repo.path(), tolerance)).unwrap();
            assert!(report.warnings.is_empty(), "{content}: {tolerance:?}: {:#?}", report.warnings);
        }
    }
}
