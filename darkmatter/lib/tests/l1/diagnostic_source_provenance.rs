//! `source` of every shared diagnostic (`compose::report_diagnostics`): a
//! value keeps what its producer knew, a file alone when no position is
//! proven, and a line counted in a rewritten body is never published.

use std::path::{Path, PathBuf};
use std::time::Duration;

use biscuit_terminal::errors::SourceContext;
use darkmatter::markdown::compose::expression::{Diagnostic, DiagnosticSource, ExpressionError};
use darkmatter::markdown::compose::page_blocks::PageBlockError;
use darkmatter::markdown::compose::policy::BlockingPolicy;
use darkmatter::markdown::compose::report_diagnostics::{error_diagnostics, warning_diagnostic, with_document_fallback};
use darkmatter::markdown::compose::shell_blocks::types::SourceExcerpt;
use darkmatter::markdown::compose::shell_expansion::template::{RejectReason, TemplateGrammarError, TemplateTargetError};
use darkmatter::markdown::compose::{
    ComposeContext, ComposeFailure, ComposeOptions, ComposeReport, ComposeWarning, ShellBlockError, ShellCommandOrigin, ShellExpansionError,
    TransclusionError,
};
use darkmatter::markdown::reference::ReferenceError;
use darkmatter::markdown::{Markdown, MarkdownError, SourceRef};
use tempfile::TempDir;

/// A temporary directory under its canonical path, the spelling composition
/// names a transcluded child by.
fn directory() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    let root = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
    (dir, root)
}

fn write(root: &Path, name: &str, content: &str) -> PathBuf {
    let path = root.join(name);
    std::fs::write(&path, content).unwrap();
    path
}

type Composed = Result<(Markdown, ComposeReport), Box<ComposeFailure>>;

fn compose_with(root: &Path, path: &Path, options: impl FnOnce(ComposeOptions) -> ComposeOptions) -> Composed {
    let base = ComposeOptions::new_with_context(ComposeContext::capture_for_content(root, "")).with_source_file(path.to_path_buf());
    let request = crate::request_support::request_at(root, options(base));
    Markdown::try_from(path).unwrap().compose_with_warnings(&request)
}

/// The one record `content`'s composition fails with, and its file.
fn failed_record(content: &str) -> (PathBuf, Diagnostic) {
    let (_dir, root) = directory();
    write(&root, "x.md", "# X\n");
    let path = write(&root, "doc.md", content);
    let failure = compose_with(&root, &path, |options| options).err().unwrap_or_else(|| panic!("fails: {content}"));
    let records = error_diagnostics(&failure.error);
    let [record] = records.as_slice() else {
        panic!("one record: {records:#?}");
    };
    (path, record.clone())
}

/// The record `content` warns `code` with, and its file.
fn warned_record(content: &str, code: Option<&str>) -> (PathBuf, Diagnostic) {
    let (_dir, root) = directory();
    write(&root, "x.md", "# X\n");
    let path = write(&root, "doc.md", content);
    let (_, report) = compose_with(&root, &path, |options| options).unwrap_or_else(|failure| panic!("composes: {}", failure.error));
    let warning = report
        .warnings
        .iter()
        .find(|warning| warning.code.as_deref() == code)
        .unwrap_or_else(|| panic!("{code:?} in {:#?}", report.warnings));
    (path, warning_diagnostic(warning))
}

fn at(file: &Path, line: usize, column: usize) -> Option<DiagnosticSource> {
    Some(DiagnosticSource::at(Some(file.to_path_buf()), line, column))
}

fn at_line(file: &Path, line: usize) -> Option<DiagnosticSource> {
    Some(DiagnosticSource { line: Some(line), ..DiagnosticSource::file_only(Some(file.to_path_buf())) })
}

fn file_only(file: &Path) -> Option<DiagnosticSource> {
    Some(DiagnosticSource::file_only(Some(file.to_path_buf())))
}

/// A body two lines longer than it was authored: a line counted in the
/// current body differs from the file's.
const REWRITES_BODY: &str = "---\nmulti: \"a\\n\\nb\"\n---\n{{ multi }}\n\n";

macro_rules! failure_sources {
    ($($name:ident: $content:expr, $code:expr, $expected:expr;)*) => {$(
        #[test]
        fn $name() {
            let content: String = $content.into();
            let (path, record) = failed_record(&content);
            assert_eq!(record.code, $code, "{record:#?}");
            let expected: fn(&Path) -> Option<DiagnosticSource> = $expected;
            assert_eq!(record.source, expected(&path), "{record:#?}");
        }
    )*};
}

failure_sources! {
    // R1-F2.1
    schema_preparation_names_its_file:
        "---\n$schema:\n  count: definitely_unknown_type\n---\nbody\n",
        "dm.schema.prepare", file_only;
    // R1-F2.2
    schema_validation_keeps_its_position:
        "---\ncount: hello\n$schema:\n  count: number(required)\n---\nbody\n",
        "dm.schema.type_mismatch", |file| at(file, 2, 8);
    // R1-F2.4
    a_block_condition_evaluation_keeps_its_authored_position:
        "---\ntitle: x\n---\n# T\n\n::block when=\"current.nosuch\"\nhi\n::end-block\n",
        "dm.expression.evaluation_failure", |file| at(file, 6, 1);
    a_block_condition_parse_keeps_its_authored_position:
        "---\ntitle: x\n---\n# T\n\n::block when=\"1 +\"\nhi\n::end-block\n",
        "dm.expression.parse_failure", |file| at(file, 6, 1);
    a_block_condition_past_a_rewrite_names_the_authored_line:
        format!("{REWRITES_BODY}::block when=\"current.nosuch\"\nhi\n::end-block\n"),
        "dm.expression.evaluation_failure", |file| at(file, 6, 1);
    a_nested_block_condition_names_its_own_opener:
        "::block when=\"true\"\n::block when=\"current.nosuch\"\nx\n::end-block\n::end-block\n",
        "dm.expression.evaluation_failure", |file| at(file, 2, 1);
    a_block_condition_beside_a_collection_keeps_its_position:
        "---\n$schema:\n  items: string[]\nitems: [a]\n---\n::block when=\"current.nosuch\"\nx\n::end-block\n\n::map items \"i => i\"\nb\n::end-map\n",
        "dm.expression.evaluation_failure", |file| at(file, 6, 1);
    // R1-F2.5
    a_transclusion_condition_evaluation_keeps_its_position:
        format!("{REWRITES_BODY}::file ./x.md when=\"current.nosuch\"\n"),
        "dm.expression.evaluation_failure", |file| at(file, 6, 1);
    // R1-F2.6
    a_transclusion_condition_parse_keeps_its_position:
        format!("{REWRITES_BODY}::file ./x.md when=\"1 +\"\n"),
        "dm.transclusion.failure", |file| at(file, 6, 1);
    // R1-F2.7
    an_omitted_transclusion_target_names_its_file:
        "---\na: 1\n---\n\n::file\n",
        "dm.directive.parse_failure", file_only;
    // R1-F2.9
    a_frontmatter_command_names_its_key_line:
        "---\ntitle: x\nx: \"$(echo hello)\"\n---\nbody\n",
        "dm.shell.failure", |file| at_line(file, 3);
    // R1-F2.27
    a_frontmatter_command_is_not_located_at_a_nested_key_of_its_name:
        "---\na:\n  x: 1\nx: \"$(echo hello)\"\n---\nbody\n",
        "dm.shell.failure", |file| at_line(file, 4);
    // R1-F2.10
    a_body_command_past_a_rewrite_publishes_no_prepared_line:
        format!("{REWRITES_BODY}::shell echo hi\n"),
        "dm.shell.failure", file_only;
    // R1-F2.11
    a_shell_block_command_publishes_no_prepared_line:
        format!("{REWRITES_BODY}::shell-block\necho hi\n::end-block\n"),
        "dm.shell.failure", file_only;
    // R1-F2.15
    a_body_interpolation_keeps_its_position:
        "# Doc\n\n{{ 1 / 0 }}\n",
        "dm.expression.evaluation_failure", |file| at(file, 3, 1);
    a_frontmatter_interpolation_keeps_its_position:
        "---\ntitle: x\nvalue: \"{{ 1 / 0 }}\"\n---\nbody\n",
        "dm.expression.evaluation_failure", |file| at(file, 3, 9);
    // R1-F2.24
    an_unmatched_end_block_names_its_file:
        "# T\n\n::end-block\n",
        "dm.directive.parse_failure", file_only;
    an_unterminated_block_names_its_file:
        "# T\n\n::block when=\"true\"\nhi\n",
        "dm.directive.parse_failure", file_only;
}

/// R1-F2.4: `span` is the opener's first byte in the file, past a rewrite.
#[test]
fn a_block_condition_span_indexes_the_loaded_file() {
    let content = format!("{REWRITES_BODY}::block when=\"current.nosuch\"\nhi\n::end-block\n");
    let (_, record) = failed_record(&content);
    let opener = content.find("::block").unwrap();
    assert_eq!(record.span, Some(opener..opener + 1));
    assert!(record.message.contains("at line 6"), "the message names the file line: {}", record.message);
}

/// R1-F2.8: the directive that closes a cycle is in the last document before
/// the repeat, never the root.
#[test]
fn a_cycle_names_the_document_that_closes_it() {
    let (_dir, root) = directory();
    write(&root, "b.md", "# B\n\n::file ./c.md\n");
    let closer = write(&root, "c.md", "# C\n\n::file ./b.md\n");
    let path = write(&root, "root.md", "::file ./b.md\n");

    let failure = compose_with(&root, &path, |options| options).expect_err("a cycle");

    let records = error_diagnostics(&failure.error);
    assert_eq!(records.len(), 1, "{records:#?}");
    assert_eq!(records[0].code, "dm.transclusion.failure");
    assert_eq!(records[0].source, file_only(&closer));
}

#[test]
fn a_self_inclusion_names_the_child() {
    let (_dir, root) = directory();
    let child = write(&root, "child.md", "# C\n\n::file ./child.md\n");
    let path = write(&root, "root.md", "::file ./child.md\n");

    let failure = compose_with(&root, &path, |options| options).expect_err("a cycle");

    assert_eq!(error_diagnostics(&failure.error)[0].source, file_only(&child));
}

/// R1-F2.25: a typed cycle has no parent line to publish.
#[test]
fn a_cycle_chain_line_is_never_published() {
    let (a, b) = (Path::new("/work/a.md"), Path::new("/work/b.md"));
    let error: MarkdownError = TransclusionError::CycleDetected {
        chain: vec![(a.to_path_buf(), 9), (b.to_path_buf(), 9), (a.to_path_buf(), 9)],
    }
    .into();
    assert_eq!(error_diagnostics(&error)[0].source, file_only(b));
}

fn document_context() -> SourceContext {
    SourceContext::new(PathBuf::from("/work/doc.md"), PathBuf::from("/work/doc.md"), "---\nx: 1\n---\nbody\n")
}

fn context() -> Box<SourceContext> {
    Box::new(document_context())
}

fn expression_failure() -> Box<ExpressionError> {
    Box::new(ExpressionError::Other { function: "f".into(), message: "m".into() })
}

fn frontmatter() -> ShellCommandOrigin {
    ShellCommandOrigin::Frontmatter { key: "x".into(), line: Some(2) }
}

fn body() -> ShellCommandOrigin {
    ShellCommandOrigin::Body { line: 40 }
}

macro_rules! shell_variants {
    ($($name:ident: $code:expr, |$origin:ident| $error:expr;)*) => {$(
        mod $name {
            use super::*;

            fn error($origin: ShellCommandOrigin) -> ShellExpansionError {
                $error
            }

            /// A frontmatter origin's line is the key's authored line.
            #[test]
            fn at_a_frontmatter_key() {
                let records = error_diagnostics(&error(frontmatter()));
                assert_eq!(records[0].code, $code);
                assert_eq!(records[0].source, at_line(Path::new("/work/doc.md"), 2));
            }

            /// A body origin's line is prepared, so only the file is known.
            #[test]
            fn in_a_body() {
                let records = error_diagnostics(&error(body()));
                assert_eq!(records[0].source, file_only(Path::new("/work/doc.md")));
            }
        }
    )*};
}

// R1-F2.10: every variant `ShellExpansionError::context` matches.
shell_variants! {
    shell_parse_directive: "dm.directive.parse_failure", |origin| ShellExpansionError::ParseDirective {
        ctx: context(), origin, message: "m".into(),
    };
    shell_expression_evaluation: "dm.shell.failure", |origin| ShellExpansionError::ExpressionEvaluation {
        ctx: context(), origin, message: "m".into(), cause: expression_failure(),
    };
    shell_command_not_found: "dm.shell.failure", |origin| ShellExpansionError::CommandNotFound {
        ctx: context(), command: "c".into(), origin,
    };
    shell_blacklisted: "dm.shell.failure", |origin| ShellExpansionError::Blacklisted {
        ctx: context(), command: "c".into(), reason: "r".into(), origin,
    };
    shell_approval_required: "dm.shell.failure", |origin| ShellExpansionError::ApprovalRequired {
        ctx: context(), command: "c".into(), whitelist_path: Box::default(), blacklist_path: Box::default(), origin,
        suggested_rules: Box::default(),
    };
    shell_denied: "dm.shell.failure", |origin| ShellExpansionError::Denied {
        ctx: context(), command: "c".into(), origin,
    };
    shell_not_pre_approved: "dm.shell.failure", |origin| ShellExpansionError::NotPreApproved {
        ctx: context(), command: "c".into(), origin, source_desc: String::new(),
    };
    shell_missing_value: "dm.shell.failure", |origin| ShellExpansionError::MissingValue {
        ctx: context(), command: "c".into(), expression: "e".into(), origin,
    };
    shell_template_grammar: "dm.shell.failure", |origin| ShellExpansionError::TemplateGrammar {
        ctx: context(), command: "c".into(), reason: TemplateGrammarError::Pipe, origin,
    };
    shell_template_value_rejected: "dm.shell.failure", |origin| ShellExpansionError::TemplateValueRejected {
        ctx: context(), command: "c".into(), placeholder: "p".into(), declared_type: "t".into(),
        reason: RejectReason::NotScalar, origin,
    };
    shell_template_target: "dm.shell.failure", |origin| ShellExpansionError::TemplateTarget {
        ctx: context(), command: "c".into(),
        reason: Box::new(TemplateTargetError::NotFound { program: "p".into() }), origin,
    };
    shell_dynamic_command_shape: "dm.shell.failure", |origin| ShellExpansionError::DynamicCommandShape {
        ctx: context(), command: "c".into(), key: "k".into(), origin,
    };
    shell_unevaluated_dependency_shape: "dm.shell.failure", |origin| ShellExpansionError::UnevaluatedDependencyShape {
        ctx: context(), command: "c".into(), dependency: "d".into(), origin,
    };
    shell_timeout: "dm.shell.failure", |origin| ShellExpansionError::Timeout {
        ctx: context(), command: "c".into(), timeout: Duration::from_secs(1), origin,
    };
    shell_execution_failed: "dm.shell.failure", |origin| ShellExpansionError::ExecutionFailed {
        ctx: context(), command: "c".into(), code: 1, stdout: String::new(), stderr: String::new(), origin,
    };
}

/// R1-F2.10: a policy file's failure is about that file.
#[test]
fn a_policy_failure_names_the_policy_file() {
    let error = ShellExpansionError::PolicyIo {
        path: PathBuf::from("/home/me/whitelist"),
        source: std::io::Error::other("denied"),
    };
    assert_eq!(error_diagnostics(&error)[0].source, file_only(Path::new("/home/me/whitelist")));
}

/// R1-F2.28: a frontmatter value's own context is named after its key; the key
/// is not a file.
#[test]
fn a_context_named_after_a_key_is_no_file() {
    let error = ShellExpansionError::ParseDirective {
        ctx: Box::new(SourceContext::new(PathBuf::from("x"), PathBuf::from("x"), "$(echo)")),
        origin: ShellCommandOrigin::Frontmatter { key: "x".into(), line: None },
        message: "m".into(),
    };
    assert_eq!(error_diagnostics(&error)[0].source, None);
}

fn excerpt() -> SourceExcerpt {
    SourceExcerpt { lines: vec![(40, "text".to_string())], highlight_line: 40 }
}

/// R1-F2.11: each variant's file, never its prepared line.
#[test]
fn a_shell_block_failure_names_its_file() {
    let file = PathBuf::from("/work/block.md");
    for error in [
        ShellBlockError::Parse { line: 40, message: "m".into(), excerpt: excerpt(), source_file: Some(file.clone()) },
        ShellBlockError::Unterminated { line: 40, opening_text: "o".into(), excerpt: excerpt(), source_file: Some(file.clone()) },
    ] {
        let error: MarkdownError = error.into();
        assert_eq!(error_diagnostics(&error)[0].source, file_only(&file), "{error}");
    }
}

#[test]
fn a_shell_block_command_falls_back_to_its_command_file() {
    let command = |source_file: Option<PathBuf>| -> MarkdownError {
        ShellBlockError::Command {
            block_start_line: 40,
            command_line: 41,
            partial_output: Box::default(),
            excerpt: excerpt(),
            source: Box::new(ShellExpansionError::Denied {
                ctx: context(),
                command: "c".into(),
                origin: ShellCommandOrigin::ShellBlock { start_line: 40, command_line: 41 },
            }),
            source_file,
        }
        .into()
    };
    assert_eq!(error_diagnostics(&command(None))[0].source, file_only(Path::new("/work/doc.md")));
    let own = PathBuf::from("/work/block.md");
    assert_eq!(error_diagnostics(&command(Some(own.clone())))[0].source, file_only(&own));
}

/// R1-F2.3: a problem with no line keeps the enclosing path.
#[test]
fn a_schema_problem_with_no_line_keeps_its_file() {
    let (_dir, root) = directory();
    let path = write(&root, "doc.md", "---\ncount: hello\n$schema:\n  count: number(required)\n---\nbody\n");
    let failure = compose_with(&root, &path, |options| options).expect_err("invalid");
    let MarkdownError::SchemaValidationFailed { path: failed, mut problems, summary, description, .. } = failure.error else {
        panic!("a schema failure");
    };
    problems[0].line = None;
    problems[0].column = None;
    let error = MarkdownError::SchemaValidationFailed { path: failed, problems, summary, description, source: None };

    assert_eq!(error_diagnostics(&error)[0].source, file_only(&path));
}

/// R1-F2.30: input with no file is named by a placeholder, which is not a file.
#[test]
fn a_placeholder_path_is_no_file() {
    for placeholder in ["<stdin>", "unknown"] {
        let error = MarkdownError::SchemaValidationFailed {
            path: PathBuf::from(placeholder),
            problems: Vec::new(),
            summary: "s".into(),
            description: None,
            source: None,
        };
        assert_eq!(error_diagnostics(&error)[0].source, None, "{placeholder}");
    }
}

/// R1-F2.19: a parser error with no position keeps the file.
#[test]
fn a_frontmatter_error_with_no_position_names_its_file() {
    let error = MarkdownError::FrontmatterParse {
        ctx: document_context(),
        source: <biscuit_file::YamlParseError as serde::de::Error>::custom("no position"),
    };
    let records = error_diagnostics(&error);
    assert_eq!(records[0].code, "dm.frontmatter.yaml_parse");
    assert_eq!(records[0].source, file_only(Path::new("/work/doc.md")));
}

/// R1-F2.20: a file with no proven span keeps the file; neither other locus is
/// a file.
#[test]
fn an_unproven_interpolation_keeps_its_file() {
    let error = |source: SourceRef| MarkdownError::Interpolation {
        key: None,
        expression: "e".into(),
        source: Box::new(source),
        cause: expression_failure(),
    };

    let on_disk = error_diagnostics(&error(SourceRef::OnDisk(document_context())));
    assert_eq!(on_disk[0].source, file_only(Path::new("/work/doc.md")));
    assert_eq!(on_disk[0].span, None);

    let supplied = error(SourceRef::Supplied { supplier: "a flag".into() });
    assert_eq!(error_diagnostics(&supplied)[0].source, None);
    let effective = error(SourceRef::Effective { rendered: "e".into(), origin_key: None });
    assert_eq!(error_diagnostics(&effective)[0].source, None);
}

/// R1-F2.24: the parser's line counts the current body, so the file alone.
#[test]
fn a_page_block_directive_failure_publishes_no_body_line() {
    let error: MarkdownError = PageBlockError::ParseDirective { ctx: context(), line: 40, message: "m".into() }.into();
    let records = error_diagnostics(&error);
    assert_eq!(records[0].code, "dm.directive.parse_failure");
    assert_eq!(records[0].source, file_only(Path::new("/work/doc.md")));
}

/// R1-F2.25: every transclusion variant holding a context names its file.
#[test]
fn a_transclusion_failure_with_a_context_names_its_file() {
    for error in [
        TransclusionError::ParseDirective { ctx: context(), line: 40, message: "m".into(), caret_col: None },
        TransclusionError::InvalidFrontmatterAssignment { ctx: context(), line: 40, raw: "r".into(), reason: "m".into() },
        TransclusionError::InvalidReassignedFrontmatterProperty { ctx: context(), line: 40, name: "n".into() },
        TransclusionError::ConditionParse { ctx: context(), expr: "e".into(), line: 40, message: "m".into(), authored: None },
        TransclusionError::ConditionEval { ctx: context(), expr: "e".into(), line: 40, authored: None, cause: expression_failure() },
    ] {
        let error: MarkdownError = error.into();
        assert_eq!(error_diagnostics(&error)[0].source, file_only(Path::new("/work/doc.md")), "{error}");
    }
}

/// R1-F2.18: the directive branch keeps its file; a resolution or I/O failure
/// holds none, and the composed document is its fallback.
#[test]
fn a_reference_failure_keeps_what_it_holds() {
    let directive: MarkdownError = ReferenceError::ParseDirective {
        ctx: context(),
        line: 40,
        message: "m".into(),
        directive_text: "::file".into(),
        caret_col: None,
    }
    .into();
    assert_eq!(error_diagnostics(&directive)[0].source, file_only(Path::new("/work/doc.md")));

    for error in [
        ReferenceError::MissingSourceContext { reference: "r".into(), line: 40 },
        ReferenceError::Validation("v".into()),
        ReferenceError::Io(std::io::Error::other("gone")),
    ] {
        let error: MarkdownError = error.into();
        let mut records = error_diagnostics(&error);
        assert_eq!(records[0].source, None, "{error}");
        with_document_fallback(&mut records, Some(Path::new("/work/root.md")));
        assert_eq!(records[0].source, file_only(Path::new("/work/root.md")), "{error}");
    }
}

/// R1-F2.21: these hold no file; a caller that knows the document supplies it.
#[test]
fn a_failure_holding_no_file_takes_the_composed_document() {
    let document = Path::new("/work/root.md");
    for error in [
        MarkdownError::ParameterBinding { property: "p".into(), provided: "v".into(), reason: "r".into() },
        MarkdownError::CallerFileClassificationChanged { property: "p".into() },
        MarkdownError::Transform("t".into()),
        TransclusionError::MaxDepthExceeded { max_depth: 1 }.into(),
    ] {
        let mut records = error_diagnostics(&error);
        assert_eq!(records[0].source, None, "{error}");
        with_document_fallback(&mut records, Some(document));
        assert_eq!(records[0].source, file_only(document), "{error}");
        assert_eq!((records[0].source.as_ref().unwrap().line, records[0].span.as_ref()), (None, None), "{error}");
    }
}

/// The fallback never replaces what a record knows, and a document with no
/// name leaves a null source.
#[test]
fn the_document_fallback_fills_only_an_empty_source() {
    let (path, located) = failed_record("# Doc\n\n{{ 1 / 0 }}\n");
    let mut records = vec![located];
    with_document_fallback(&mut records, Some(Path::new("/elsewhere.md")));
    assert_eq!(records[0].source, at(&path, 3, 1));

    let mut records = error_diagnostics(&MarkdownError::Transform("t".into()));
    with_document_fallback(&mut records, None);
    assert_eq!(records[0].source, None);
}

macro_rules! warning_sources {
    ($($name:ident: $content:expr, $code:expr, $expected:expr;)*) => {$(
        #[test]
        fn $name() {
            let content: String = $content.into();
            let (path, record) = warned_record(&content, $code);
            let expected: fn(&Path) -> Option<DiagnosticSource> = $expected;
            assert_eq!(record.source, expected(&path), "{record:#?}");
        }
    )*};
}

warning_sources! {
    // R1-F2.12
    an_undeclared_body_read_names_its_line:
        "---\na: 1\n---\n\n{{ nosuch }}\n",
        Some(ComposeWarning::UNDECLARED_PROPERTY_CODE), |file| at_line(file, 5);
    an_undeclared_frontmatter_read_names_its_key_line:
        "---\na: \"{{ nosuch }}\"\n---\nb\n",
        Some(ComposeWarning::UNDECLARED_PROPERTY_CODE), |file| at_line(file, 2);
    // R1-F2.13
    a_block_condition_warning_names_the_authored_directive:
        format!("{REWRITES_BODY}::block when=\"ctx.toady.x == 1\"\nhi\n::end-block\n"),
        Some(ComposeWarning::UNKNOWN_CONTEXT_VARIABLE_CODE), |file| at(file, 6, 1);
    a_transclusion_condition_warning_names_the_authored_directive:
        format!("{REWRITES_BODY}::file ./x.md when=\"ctx.toady.x == 1\"\n"),
        Some(ComposeWarning::UNKNOWN_CONTEXT_VARIABLE_CODE), |file| at(file, 6, 1);
    a_nested_block_condition_warning_names_its_own_directive:
        "::block when=\"true\"\n::block when=\"ctx.toady.x == 1\"\nx\n::end-block\n::end-block\n",
        Some(ComposeWarning::UNKNOWN_CONTEXT_VARIABLE_CODE), |file| at(file, 2, 1);
    a_condition_warning_beside_a_collection_names_the_directive:
        "---\n$schema:\n  items: string[]\nitems: [a]\n---\n::block when=\"ctx.toady.x == 1\"\nx\n::end-block\n\n::map items \"i => i\"\nb\n::end-map\n",
        Some(ComposeWarning::UNKNOWN_CONTEXT_VARIABLE_CODE), |file| at(file, 6, 1);
    // R1-F2.31
    a_body_context_warning_names_its_document:
        "# Doc\n\n{{ ctx.toady }}\n",
        Some(ComposeWarning::UNKNOWN_CONTEXT_VARIABLE_CODE), file_only;
    an_uncoded_warning_counted_in_the_body_names_only_its_document:
        format!("{REWRITES_BODY}::block when=\"true\" bogus=\"1\"\nhi\n::end-block\n"),
        None, file_only;
}

/// R1-F2.13: the warning's `span` indexes the loaded file too.
#[test]
fn a_condition_warning_span_indexes_the_loaded_file() {
    let content = format!("{REWRITES_BODY}::block when=\"ctx.toady.x == 1\"\nhi\n::end-block\n");
    let (_, record) = warned_record(&content, Some(ComposeWarning::UNKNOWN_CONTEXT_VARIABLE_CODE));
    let opener = content.find("::block").unwrap();
    assert_eq!(record.span, Some(opener..opener + 1));
}

/// R1-F2.26: a merged child warning names the child, not the root.
#[test]
fn a_child_warning_names_the_child() {
    let (_dir, root) = directory();
    let child = write(&root, "child.md", "# C\n\n::block when=\"ctx.toady.x == 1\"\nhi\n::end-block\n");
    let path = write(&root, "root.md", "# R\n\n::file ./child.md\n");

    let (_, report) = compose_with(&root, &path, |options| options).unwrap_or_else(|failure| panic!("{}", failure.error));

    let warning = report
        .warnings
        .iter()
        .find(|warning| warning.code.as_deref() == Some(ComposeWarning::UNKNOWN_CONTEXT_VARIABLE_CODE))
        .expect("the child's warning");
    assert_eq!(warning_diagnostic(warning).source, at(&child, 3, 1));
}

/// A warning from outside a pipeline keeps the null of a value with no file.
#[test]
fn a_warning_with_no_document_has_no_source() {
    assert_eq!(warning_diagnostic(&ComposeWarning::new("stage", "message")).source, None);
    let mut report = ComposeReport::new();
    report.add_warning(ComposeWarning::new("stage", "message").at_line(40));
    assert_eq!(warning_diagnostic(&report.warnings[0]).source, None, "a body line is not an authored one");
}

/// R1-F2.29: inside a repeated copy the record names the expression site the
/// copy proved, as the failure outside a copy names the directive.
#[test]
fn a_repeated_gate_keeps_its_expression_site() {
    let content = "---\n$schema:\n  items: string[]\nitems: [a]\n---\n::map items \"i => i\"\n::block when=\"current.nosuch\"\nx\n::end-block\n::end-map\n";
    let (path, record) = failed_record(content);
    let expression = content.find("current.nosuch").unwrap();
    assert_eq!(record.source, at(&path, 7, 15), "{record:#?}");
    assert_eq!(record.span, Some(expression..expression + "current.nosuch".len()));
}

/// Input built in memory has no authored position and no file.
#[test]
fn a_document_built_in_memory_stays_null() {
    let (_dir, root) = directory();
    let request = crate::request_support::request_at(&root, ComposeOptions::new_with_context(ComposeContext::capture_for_content(&root, "")));

    let failed = Markdown::from("# T\n\n::block when=\"current.nosuch\"\nhi\n::end-block\n").compose_with_warnings(&request);
    assert_eq!(error_diagnostics(&failed.expect_err("fails").error)[0].source, None);

    let (_, report) = Markdown::from("# T\n\n::block when=\"ctx.toady.x == 1\"\nhi\n::end-block\n")
        .compose_with_warnings(&request)
        .unwrap_or_else(|failure| panic!("{}", failure.error));
    assert_eq!(warning_diagnostic(&report.warnings[0]).source, None);
}

/// R1-F2.23: a policy stop reports the earlier warnings it was handed and the
/// blocking one, each located.
#[test]
fn a_policy_stop_keeps_every_warning_located() {
    let (_dir, root) = directory();
    let path = write(&root, "doc.md", "::block when=\"true\" bogus=\"1\"\nhi\n::end-block\n\n::block when=\"ctx.toady.x == 1\"\nhi\n::end-block\n");

    let failure = compose_with(&root, &path, |options| {
        options.with_blocking_policy(BlockingPolicy::default().deny_code(ComposeWarning::UNKNOWN_CONTEXT_VARIABLE_CODE))
    })
    .expect_err("the warning blocks");

    let records = error_diagnostics(&failure.error);
    let sources: Vec<_> = records.iter().map(|record| record.source.clone()).collect();
    assert_eq!(sources, [file_only(&path), at(&path, 5, 1)], "{records:#?}");
}

/// A stage's warnings not yet added to the report keep their document at a stop.
#[test]
fn a_policy_stop_inside_a_stage_names_its_document() {
    let (_dir, root) = directory();
    let path = write(&root, "doc.md", "# Doc\n\n{{ ctx.toady }}\n");

    let failure = compose_with(&root, &path, |options| {
        options.with_blocking_policy(BlockingPolicy::default().deny_code(ComposeWarning::UNKNOWN_CONTEXT_VARIABLE_CODE))
    })
    .expect_err("the warning blocks");

    let records = error_diagnostics(&failure.error);
    assert_eq!(records.len(), 1, "{records:#?}");
    assert_eq!(records[0].source, file_only(&path));
}

/// R1-F2.23: warnings a failed document reported keep their sources.
#[test]
fn a_failure_keeps_located_earlier_warnings() {
    let (_dir, root) = directory();
    let path = write(&root, "doc.md", "::block when=\"ctx.toady.x == 1\"\nhi\n::end-block\n\n{{ 1 / 0 }}\n");

    let failure = compose_with(&root, &path, |options| options).expect_err("the document fails");

    let warning = failure
        .warnings
        .iter()
        .find(|warning| warning.code.as_deref() == Some(ComposeWarning::UNKNOWN_CONTEXT_VARIABLE_CODE))
        .expect("the earlier warning");
    assert_eq!(warning_diagnostic(warning).source, at(&path, 1, 1));
    assert_eq!(error_diagnostics(&failure.error)[0].source, at(&path, 5, 1));
}

/// R1-F2.23: a child's blocking warning is fatal and keeps naming the child.
#[test]
fn a_child_policy_stop_names_the_child() {
    let (_dir, root) = directory();
    let child = write(&root, "child.md", "# C\n\n::block when=\"ctx.toady.x == 1\"\nhi\n::end-block\n");
    let path = write(&root, "root.md", "::file ./child.md\n");

    let failure = compose_with(&root, &path, |options| {
        options.with_blocking_policy(BlockingPolicy::default().deny_code(ComposeWarning::UNKNOWN_CONTEXT_VARIABLE_CODE))
    })
    .expect_err("the warning blocks");

    let records = error_diagnostics(&failure.error);
    assert_eq!(records.len(), 1, "{records:#?}");
    assert_eq!(records[0].source, at(&child, 3, 1));
}
