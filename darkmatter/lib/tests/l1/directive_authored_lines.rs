//! A body directive's parse failure names the line it was written on in its
//! file, never a line of the body a reader prepared from it: the frontmatter
//! header shifts every body line, and a false page block before the directive
//! shortens the prepared body. Every reader (composition, both graph modes,
//! the local query, flattened references, validation) reports the authored
//! line in its message and in its diagnostic's `source`, and a body the
//! file no longer describes names the file without a line.

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::compose::report_diagnostics::{error_diagnostics, warning_diagnostic};
use darkmatter::markdown::compose::shell_expansion::types::{
    ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest,
};
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions, ComposeRequest, ShellExpansionError};
use darkmatter::markdown::reference::types::ReferenceGraphOptions;
use darkmatter::markdown::reference::validate::ReferenceValidationOptions;
use darkmatter::markdown::{Markdown, MarkdownError};
use std::path::Path;
use tempfile::TempDir;

/// A temporary repository holding the included `side.md` and a `docs/`
/// directory for `::file-links`.
fn repository() -> TempDir {
    let repo = TempDir::new().unwrap();
    std::fs::create_dir_all(repo.path().join(".git")).unwrap();
    std::fs::create_dir_all(repo.path().join("docs")).unwrap();
    std::fs::write(repo.path().join("side.md"), "# Side\n").unwrap();
    std::fs::write(repo.path().join("docs/a.md"), "# A\n").unwrap();
    repo
}

/// A request in the repository's frozen context, so nothing scans the real
/// working tree.
fn request(root: &Path) -> ComposeRequest {
    let context = FileResolutionContext::from_snapshot(root, None, std::collections::HashMap::new())
        .with_repository_root(root);
    crate::request_support::request_in(
        ComposeOptions::new_with_context(ComposeContext::capture_for_content(root, "")),
        context,
    )
}

/// `doc.md` written with `content` and loaded from its file.
fn document(repo: &TempDir, content: &str) -> Markdown {
    let path = repo.path().join("doc.md");
    std::fs::write(&path, content).unwrap();
    Markdown::try_from(path.as_path()).unwrap()
}

/// A two-key header, then `directive` on file line 7 (body line 3).
fn plain(directive: &str) -> (String, usize) {
    (format!("---\na: 1\nb: 2\n---\n\n# Doc\n{directive}\n"), 7)
}

/// The same header and a false page block before `directive` on file line
/// 10: the prepared body loses the block, the raw body keeps it.
fn after_removed_block(directive: &str) -> (String, usize) {
    (format!("---\na: 1\nb: 2\n---\n\n::block when=\"false\"\nhidden\n::end-block\n# Doc\n{directive}\n"), 10)
}

/// Every malformed body directive family, with its diagnostic code.
const MALFORMED: [(&str, &str); 7] = [
    ("::file", "dm.directive.parse_failure"),
    ("::code", "dm.directive.parse_failure"),
    ("::url", "dm.directive.parse_failure"),
    ("::toc-linking ./side.md level=h9", "dm.toc_linking.invalid_level"),
    ("::toc-linking ./side.md levels=x", "dm.directive.parse_failure"),
    ("::toc-linking ./side.md cleanup=bogus", "dm.toc_linking.invalid_cleanup"),
    ("::file-links --dir ./docs --depth two", "dm.file_links.invalid_depth"),
];

/// Each reader's failure for `md`.
fn failures(repo: &TempDir, md: &Markdown) -> Vec<(&'static str, Result<(), MarkdownError>)> {
    let request = request(repo.path());
    let graph = ReferenceGraphOptions::with_compose(&request);
    vec![
        ("compose", md.compose_with(&request).map(drop)),
        ("reference graph", md.reference_graph(graph.clone()).map(drop)),
        ("transclusion graph", md.transclusion_graph(graph.clone()).map(drop)),
        ("local query", md.transclusions_with_options(&request).map(drop)),
        ("flattened references", md.composed_references(graph.clone()).map(drop)),
        ("validation", md.validate_references(ReferenceValidationOptions::with_graph(graph)).map(drop)),
    ]
}

/// Asserts every reader refuses `content` naming `line` of `doc.md` in its
/// message and diagnostic.
fn assert_authored(repo: &TempDir, content: &str, line: usize, code: &str, shape: &str) {
    let md = document(repo, content);
    for (reader, result) in failures(repo, &md) {
        let error = result.expect_err(&format!("{shape}: {reader} must refuse"));
        let message = error.to_string();
        assert!(message.contains(&format!("line {line}")), "{shape}: {reader}: {message}");
        for body_line in [3, 4, 7].into_iter().filter(|body_line| *body_line != line) {
            assert!(!message.contains(&format!("line {body_line}")), "{shape}: {reader}: {message}");
        }
        let records = error_diagnostics(&error);
        let [record] = records.as_slice() else { panic!("{shape}: {reader}: {records:#?}") };
        assert_eq!(record.code, code, "{shape}: {reader}: {record:#?}");
        let source = record.source.as_ref().unwrap_or_else(|| panic!("{shape}: {reader}: {record:#?}"));
        assert!(source.file.as_ref().is_some_and(|file| file.ends_with("doc.md")), "{shape}: {reader}: {record:#?}");
        assert_eq!((source.line, source.column), (Some(line), Some(1)), "{shape}: {reader}: {record:#?}");
    }
}

/// R11-F2.1–.3: with a frontmatter header, each parser family names the file
/// line, not the body line.
#[test]
fn a_malformed_directive_after_frontmatter_names_its_file_line() {
    let repo = repository();
    for (directive, code) in MALFORMED {
        let (content, line) = plain(directive);
        assert_authored(&repo, &content, line, code, directive);
    }
}

/// R11-F2.4: a false page block before the directive leaves the prepared
/// body shorter than the file; the failure still names the directive's
/// opener, in the raw local query as in the prepared readers.
#[test]
fn a_removed_page_block_does_not_move_the_reported_line() {
    let repo = repository();
    for (directive, code) in MALFORMED {
        let (content, line) = after_removed_block(directive);
        assert_authored(&repo, &content, line, code, directive);
    }
}

/// R11-F2.4: a body the file no longer describes has no authored position,
/// so the failure names the file and no line, never the body's.
#[test]
fn an_unproven_directive_names_its_file_without_a_line() {
    let repo = repository();
    for (directive, code) in MALFORMED {
        let (content, _) = plain(directive);
        let mut md = document(&repo, &content);
        md.content_mut().insert_str(0, "Inserted.\n\n");
        let request = request(repo.path());
        let error = md.transclusions_with_options(&request).expect_err(directive);
        let message = error.to_string();
        assert!(!message.contains("line "), "{directive}: {message}");
        let records = error_diagnostics(&error);
        let [record] = records.as_slice() else { panic!("{directive}: {records:#?}") };
        assert_eq!(record.code, code, "{directive}: {record:#?}");
        let source = record.source.as_ref().unwrap_or_else(|| panic!("{directive}: {record:#?}"));
        assert!(source.file.as_ref().is_some_and(|file| file.ends_with("doc.md")), "{directive}: {record:#?}");
        assert_eq!(source.line, None, "{directive}: {record:#?}");
    }
}

/// Valid spellings of the same directives raise nothing.
#[test]
fn valid_spellings_report_nothing() {
    let repo = repository();
    for directive in [
        "::file ./side.md",
        "::code ./side.md",
        "::toc-linking ./side.md level=h2",
        "::file-links --dir ./docs --depth 1",
    ] {
        for (content, _) in [plain(directive), after_removed_block(directive)] {
            let md = document(&repo, &content);
            for (reader, result) in failures(&repo, &md) {
                result.unwrap_or_else(|error| panic!("{directive}: {reader}: {error}"));
            }
        }
    }
}

/// R11-F2.5: a frontmatter field's error keeps its key line, and a `when=`
/// failure keeps the authored opener it already reported, past both the
/// header and a removed block.
#[test]
fn frontmatter_and_condition_failures_keep_their_lines() {
    let repo = repository();
    let md = document(&repo, "---\na: 1\nprologue: null\n---\n\n# Doc\n");
    let error = md.compose_with(&request(repo.path())).expect_err("a null prologue is refused");
    assert_eq!(error_diagnostics(&error)[0].source.as_ref().and_then(|source| source.line), Some(3), "{error}");

    for (expression, code) in [("1 / 0", "dm.expression.evaluation_failure"), ("1 +", "dm.expression.parse_failure")] {
        let directive = format!("::file ./side.md when=\"{expression}\"");
        for (content, line) in [plain(&directive), after_removed_block(&directive)] {
            let md = document(&repo, &content);
            let error = md.compose_with(&request(repo.path())).expect_err(expression);
            let records = error_diagnostics(&error);
            assert_eq!(records[0].code, code, "{expression}: {records:#?}");
            assert_eq!(records[0].source.as_ref().and_then(|source| source.line), Some(line), "{expression}: {records:#?}");
        }
    }
}

/// A `::toc-linking` filter that fails only when composition renders it
/// (tolerated as a warning), and a `::file-links` target that is not a
/// directory, name the directive's file line too, past a removed block.
#[test]
fn a_rendering_or_listing_failure_names_its_file_line() {
    let repo = repository();
    for (directive, code) in [
        ("::toc-linking ./side.md filter=\"[\"", "dm.toc_linking.invalid_filter"),
        ("::file-links --dir ./side.md", "dm.file_links.not_a_directory"),
    ] {
        for (content, line) in [plain(directive), after_removed_block(directive)] {
            let md = document(&repo, &content);
            // Composition tolerates a rendering failure as a warning and
            // fails on a listing failure; either way it names the line.
            let record = match md.compose_with(&request(repo.path())) {
                Ok((_, report)) => {
                    let warning = report
                        .warnings
                        .iter()
                        .find(|warning| warning.code.as_deref() == Some(code))
                        .unwrap_or_else(|| panic!("{directive}: {:#?}", report.warnings));
                    warning_diagnostic(warning)
                }
                Err(error) => error_diagnostics(&error).remove(0),
            };
            assert!(record.message.contains(&format!("line {line}")), "{directive}: {record:#?}");
            assert_eq!(record.code, code, "{directive}: {record:#?}");
            assert_eq!(record.source.as_ref().and_then(|source| source.line), Some(line), "{directive}: {record:#?}");
        }
    }
}

/// A `::shell` line that does not parse is named by its file line even when
/// interpolation inserted lines before it, in its message and in its
/// diagnostic's `source`.
#[test]
fn a_malformed_shell_line_names_its_file_line() {
    let repo = repository();
    let md = document(&repo, "---\nx: \"a\\nb\\nc\\nd\"\n---\n\n{{ x }}\n::shell echo \"unterminated\n");
    let error = md.compose_with(&request(repo.path())).expect_err("an unterminated quote is refused");
    let message = error.to_string();
    assert!(message.contains("line 6"), "{message}");
    let records = error_diagnostics(&error);
    assert_eq!(records[0].code, "dm.directive.parse_failure", "{records:#?}");
    let source = records[0].source.as_ref();
    assert_eq!(source.map(|source| (source.line, source.column)), Some((Some(6), Some(1))), "{records:#?}");
}

/// How a test's approval handler answers every command.
struct Answer(ShellApprovalDecision);

impl ShellApprovalHandler for Answer {
    fn approve(&self, _request: ShellApprovalRequest) -> Result<ShellApprovalDecision, ShellExpansionError> {
        Ok(self.0.clone())
    }
}

/// [`request`] whose approval handler answers `decision`.
fn answering(root: &Path, decision: ShellApprovalDecision) -> ComposeRequest {
    let context = FileResolutionContext::from_snapshot(root, None, std::collections::HashMap::new())
        .with_repository_root(root);
    let options = ComposeOptions::new_with_context(ComposeContext::capture_for_content(root, ""))
        .with_shell_approval_handler(std::sync::Arc::new(Answer(decision)));
    crate::request_support::request_in(options, context)
}

/// A program no host has, so running it fails the same way everywhere.
const MISSING_PROGRAM: &str = "darkmatter-r11f2-no-such-program";

/// Asserts `error` is one `code` record naming `line` of `doc.md`, in its
/// message (and no line the body counts) and in its `source`.
#[track_caller]
fn assert_shell_line(error: &MarkdownError, code: &str, line: usize, shape: &str) {
    let message = error.to_string();
    assert!(message.contains(&format!("line {line}")), "{shape}: {message}");
    for body_line in [3, 4, 7].into_iter().filter(|body_line| *body_line != line && *body_line != line - 1) {
        assert!(!message.contains(&format!("line {body_line}")), "{shape}: {message}");
    }
    let records = error_diagnostics(error);
    let [record] = records.as_slice() else { panic!("{shape}: {records:#?}") };
    assert_eq!(record.code, code, "{shape}: {record:#?}");
    let source = record.source.as_ref().unwrap_or_else(|| panic!("{shape}: {record:#?}"));
    assert!(source.file.as_ref().is_some_and(|file| file.ends_with("doc.md")), "{shape}: {record:#?}");
    assert_eq!((source.line, source.column), (Some(line), Some(1)), "{shape}: {record:#?}");
}

/// R11-F2: a `::shell` line's parse failure names its file line in its
/// diagnostic's `source` too, after a frontmatter header (the control) and
/// past a false page block the prepared body lost.
#[test]
fn a_shell_parse_failure_places_its_source_at_the_file_line() {
    let repo = repository();
    for (content, line) in [plain("::shell echo \"unterminated"), after_removed_block("::shell echo \"unterminated")] {
        let md = document(&repo, &content);
        let error = md.compose_with(&request(repo.path())).expect_err("an unterminated quote is refused");
        assert_shell_line(&error, "dm.directive.parse_failure", line, &content);
    }
}

/// R11-F2: a `::shell` command that fails after it was read (a program not
/// found, a refused approval) names the directive's file line, not the line
/// the prepared body counts past the header.
#[test]
fn a_failing_shell_command_names_its_file_line() {
    let repo = repository();
    let directive = format!("::shell {MISSING_PROGRAM}");
    for decision in [ShellApprovalDecision::AllowOnce, ShellApprovalDecision::Deny] {
        for (content, line) in [plain(&directive), after_removed_block(&directive)] {
            let md = document(&repo, &content);
            let error = md.compose_with(&answering(repo.path(), decision.clone())).expect_err(&content);
            assert_shell_line(&error, "dm.shell.failure", line, &format!("{decision:?}: {content}"));
        }
    }
}

/// R11-F2: a `::shell-block` command's failure names the command's file line
/// and the block's opener, past a removed page block too.
#[test]
fn a_failing_shell_block_command_names_its_file_lines() {
    let repo = repository();
    let block = format!("::shell-block\n{MISSING_PROGRAM}\n::end-block");
    for (content, opener) in [plain(&block), after_removed_block(&block)] {
        let md = document(&repo, &content);
        let error = md.compose_with(&answering(repo.path(), ShellApprovalDecision::AllowOnce)).expect_err(&content);
        let message = error.to_string();
        assert!(message.contains(&format!("starting at line {opener}, command at line {}", opener + 1)), "{message}");
        assert_shell_line(&error, "dm.shell.failure", opener + 1, &content);
    }
}

/// A body the file no longer describes proves no line: the shell failure's
/// diagnostic names the file alone.
#[test]
fn an_unproven_shell_failure_names_its_file_without_a_line() {
    let repo = repository();
    let (content, _) = plain(&format!("::shell {MISSING_PROGRAM}"));
    let mut md = document(&repo, &content);
    md.content_mut().insert_str(0, "Inserted.\n\n");
    let error = md.compose_with(&answering(repo.path(), ShellApprovalDecision::AllowOnce)).expect_err("fails");
    let records = error_diagnostics(&error);
    let source = records[0].source.as_ref().unwrap_or_else(|| panic!("{records:#?}"));
    assert!(source.file.as_ref().is_some_and(|file| file.ends_with("doc.md")), "{records:#?}");
    assert_eq!(source.line, None, "{records:#?}");
}

/// Malformed `::shell-block`s with the file line each failure is reported
/// at, given the block's opener line: a command that does not tokenize (its
/// own line) and an invalid option (the opener).
fn malformed_blocks() -> [(&'static str, usize); 2] {
    [("::shell-block\necho \"unterminated\n::end-block", 1), ("::shell-block timeout=\"x\"\necho hi\n::end-block", 0)]
}

/// Asserts `error` is one `code` record at `line`, column 1, of `doc.md`,
/// whose message names `line` and no unknown line `0`.
#[track_caller]
fn assert_block_line(error: &MarkdownError, code: &str, line: usize, shape: &str) {
    let message = error.to_string();
    assert!(message.contains(&format!("line {line}")), "{shape}: {message}");
    assert!(!message.contains("line 0"), "{shape}: {message}");
    let records = error_diagnostics(error);
    let [record] = records.as_slice() else { panic!("{shape}: {records:#?}") };
    assert_eq!(record.code, code, "{shape}: {record:#?}");
    let source = record.source.as_ref().unwrap_or_else(|| panic!("{shape}: {record:#?}"));
    assert!(source.file.as_ref().is_some_and(|file| file.ends_with("doc.md")), "{shape}: {record:#?}");
    assert_eq!((source.line, source.column), (Some(line), Some(1)), "{shape}: {record:#?}");
}

/// R11-F2: a `::shell-block` that does not parse names its file line in its
/// message and its diagnostic's `source`, after a frontmatter header (the
/// control) and past a false page block the prepared body lost. Before the
/// fix the message counted the body (`line 4`/`line 3`) and the tokenizer's
/// own `line 0`, and the `source` named the file alone.
#[test]
fn a_malformed_shell_block_names_its_file_line() {
    let repo = repository();
    for (block, past_opener) in malformed_blocks() {
        for (content, opener) in [plain(block), after_removed_block(block)] {
            let md = document(&repo, &content);
            let error = md.compose_with(&answering(repo.path(), ShellApprovalDecision::AllowOnce)).expect_err(&content);
            let line = opener + past_opener;
            for body_line in [3, 4] {
                assert!(!error.to_string().contains(&format!("line {body_line}:")), "{content}: {error}");
            }
            assert_block_line(&error, "dm.shell.failure", line, &content);
        }
    }
}

/// R11-F2: a `::shell-block` with no `::end-block` is refused by the block
/// scanner at its opener's file line, not the body line the scanner counted
/// (`6` past the removed block, `3` without it).
#[test]
fn an_unterminated_shell_block_names_its_opener_line() {
    let repo = repository();
    for (content, opener) in [plain("::shell-block\necho hi"), after_removed_block("::shell-block\necho hi")] {
        let md = document(&repo, &content);
        let error = md.compose_with(&answering(repo.path(), ShellApprovalDecision::AllowOnce)).expect_err(&content);
        assert!(error.to_string().contains(&format!("starting at line {opener}")), "{content}: {error}");
        assert_block_line(&error, "dm.directive.parse_failure", opener, &content);
    }
}

/// A request whose shell policy is an empty user configuration under
/// `home`, so pre-flight reads no host configuration.
fn preflight_request(root: &Path, home: &Path) -> ComposeRequest {
    let context = FileResolutionContext::from_snapshot(root, None, std::collections::HashMap::new())
        .with_repository_root(root);
    let options = ComposeOptions::new_with_context(ComposeContext::capture_for_content(root, ""))
        .with_config_snapshot(crate::request_support::shell_config(home, &[]));
    crate::request_support::request_in(options, context)
}

/// R11-F2, R10-F2: the shell-approval pre-flight refuses a malformed block
/// with the stage's own typed failure (`dm.shell.failure`, as composition
/// reports it), never a `dm.compose.failure` string, and at its file line
/// (the block is the walked document's own, so its `source` line is
/// published too). A valid block is collected for approval (the control).
#[test]
fn preflight_refuses_a_malformed_shell_block_with_its_code_and_file_line() {
    let repo = repository();
    let home = TempDir::new().unwrap();
    let request = preflight_request(repo.path(), home.path());
    let handler = || Some(std::sync::Arc::new(Answer(ShellApprovalDecision::AllowOnce)) as std::sync::Arc<dyn ShellApprovalHandler>);
    for (block, past_opener) in malformed_blocks().into_iter().take(1) {
        for (content, opener) in [plain(block), after_removed_block(block)] {
            let md = document(&repo, &content);
            let error: MarkdownError = md.compose_preflight_approvals(&request, handler()).expect_err(&content).into();
            assert_block_line(&error, "dm.shell.failure", opener + past_opener, &content);
        }
    }
    let (content, _) = after_removed_block("::shell-block\necho hi\n::end-block");
    let approvals = document(&repo, &content).compose_preflight_approvals(&request, handler()).expect("a valid block");
    assert!(approvals.pre_approved_commands.contains_exact("echo hi"), "{:?}", approvals.pre_approved_commands);
}

/// An unterminated `::block`'s excerpt numbers its gutter in the file's
/// frame: with a frontmatter header above it, the opener on file line 4 is
/// marked `> 4`, never the body's `> 1`.
#[test]
fn an_unterminated_block_excerpt_numbers_file_lines() {
    let repo = repository();
    let md = document(&repo, "---\ntitle: x\n---\n::block when=\"title\"\nbody\n");
    let error = md.compose_with(&request(repo.path())).expect_err("an unterminated block is refused");
    let block = darkmatter::markdown::errors::as_block_error(&error).expect("a block error");
    let rendered = biscuit_terminal::utils::escape_codes::strip_escape_codes(block.report_block_error_optimistic(Some(80)));
    assert!(rendered.contains("> 4 │ ::block when=\"title\""), "{rendered}");
    assert!(rendered.contains("  3 │ ---"), "{rendered}");
    assert!(!rendered.contains("> 1 │"), "{rendered}");
}
