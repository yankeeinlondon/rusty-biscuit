//! Shell Blocks — block-level shell command execution.
//!
//! This module provides `::shell-block` / `::end-block` directives that
//! execute multiple shell commands sequentially and render their combined
//! output.

pub mod body;
pub mod parser;
pub mod render;
pub mod types;

pub use types::ShellBlockError;

use super::ComposeOptions;
use super::shell_expansion::types::{ShellCommandOrigin, ShellDirective, ShellExpansionRuntime};
use super::shell_expansion::{
    apply_replacements_in_reverse, execute_prepared_directive, prepare_directive,
    resolve_policy_paths,
};
use super::ComposeReport;
use crate::markdown::MarkdownResult;
use std::path::PathBuf;
use types::{ShellBlockCommandResult, SourceExcerpt};

/// Extract source file path from compose options.
fn source_file_from_options(options: &ComposeOptions) -> Option<PathBuf> {
    match &options.source {
        super::ComposeSource::File(path) => Some(path.clone()),
        _ => None,
    }
}

/// Run the shell blocks stage on the given content.
///
/// Returns the transformed content and a compose report.
pub(crate) fn run_shell_blocks_stage(
    content: &str,
    options: &ComposeOptions,
    runtime: &mut ShellExpansionRuntime,
    ctx: &biscuit_terminal::errors::SourceContext,
    line_offset: usize,
) -> Result<(String, ComposeReport), ShellBlockError> {
    let source_file = source_file_from_options(options);

    // Scan for block pairs (both page and shell)
    let pairs = super::block_pairs::scan_block_pairs(content).map_err(|e| {
        // Extract line number from BlockPairError
        let line = match &e {
            super::block_pairs::BlockPairError::UnmatchedEnd { line } => *line,
            super::block_pairs::BlockPairError::UnterminatedBlock { line, .. } => *line,
            super::block_pairs::BlockPairError::TrailingContent { line, .. } => *line,
        };
        ShellBlockError::Parse {
            line,
            message: e.to_string(),
            excerpt: SourceExcerpt::default(),
            source_file: source_file.clone(),
        }
    })?;

    // Filter for shell blocks only
    let shell_pairs: Vec<_> = pairs
        .into_iter()
        .filter(|p| matches!(p.kind, super::block_pairs::BlockOpenKind::Shell))
        .collect();

    if shell_pairs.is_empty() {
        return Ok((content.to_string(), ComposeReport::new()));
    }

    // Resolve policy paths once
    let shell_opts = options.shell_options();
    let policy_paths =
        resolve_policy_paths(&shell_opts, &options.source).map_err(|e| ShellBlockError::Parse {
            line: 0,
            message: format!("Policy path resolution failed: {e}"),
            excerpt: SourceExcerpt::default(),
            source_file: source_file.clone(),
        })?;

    runtime
        .ensure_loaded(&policy_paths)
        .map_err(|e| ShellBlockError::Parse {
            line: 0,
            message: format!("Policy loading failed: {e}"),
            excerpt: SourceExcerpt::default(),
            source_file: source_file.clone(),
        })?;

    let mut replacements = Vec::new();
    let mut report = ComposeReport::new();

    for pair in shell_pairs {
        let region = parser::parse_shell_block_region(content, &pair)?;
        let body_text = &content[pair.body_span.clone()];
        let commands = body::split_logical_commands(body_text, pair.start_line + 1)?;

        if commands.is_empty() {
            // Empty block body: replace with empty string
            replacements.push((pair.span.clone(), String::new()));
            report.shell_blocks_applied += 1;
            continue;
        }

        // Prepare all commands before executing any
        let mut prepared = Vec::new();
        for command in &commands {
            let directive = ShellDirective {
                raw_command: command.raw_command.clone(),
                executable: command.executable.clone(),
                args: command.args.clone(),
                span: command.physical_span.clone(),
                // Opener indentation is applied once to the combined block output
                // at the splice boundary, not per command.
                indent: String::new(),
                origin: ShellCommandOrigin::ShellBlock {
                    start_line: pair.start_line + line_offset,
                    command_line: command.start_line + line_offset,
                },
                error_handling: region.options.clone(),
                timeout_override: region.timeout_override,
                no_cache: region.no_cache,
                pipeline: Some(command.pipeline.clone()),
                ctx: ctx.clone(),
            };

            match prepare_directive(&directive, options, &policy_paths, runtime) {
                Ok(p) => prepared.push((p, command.clone())),
                Err(e) => {
                    return Err(ShellBlockError::Command {
                        block_start_line: pair.start_line + line_offset,
                        command_line: command.start_line + line_offset,
                        partial_output: Box::new(Vec::new()),
                        excerpt: SourceExcerpt::from_text(
                            body_text,
                            command.start_line + line_offset,
                            pair.start_line + 1 + line_offset,
                            2,
                        ),
                        source: Box::new(e),
                        source_file: source_file.clone(),
                    });
                }
            }
        }

        // Execute sequentially
        let mut results = Vec::new();

        for (prep, command) in prepared {
            match execute_prepared_directive(&prep, options, runtime) {
                Ok(execution) => {
                    results.push(ShellBlockCommandResult {
                        output: execution.combined_output(),
                    });
                    report.warnings.extend(execution.warnings);
                }
                Err(e) => {
                    // Check if this is an ExecutionFailed that might have been
                    // handled by ErrorHandling. execute_prepared_directive already
                    // applies error handling, so if we get an error here it's
                    // unhandled.
                    let partial: Vec<String> = results.iter().map(|r| r.output.clone()).collect();
                    return Err(ShellBlockError::Command {
                        block_start_line: pair.start_line + line_offset,
                        command_line: command.start_line + line_offset,
                        partial_output: Box::new(partial),
                        excerpt: SourceExcerpt::from_text(
                            body_text,
                            command.start_line + line_offset,
                            pair.start_line + 1 + line_offset,
                            2,
                        ),
                        source: Box::new(e),
                        source_file: source_file.clone(),
                    });
                }
            }
        }

        // Re-indent the combined block output to the opener's column so
        // generated lines stay nested under the surrounding list or block
        // quote. `indent_text` returns an empty rendered block unchanged, so an
        // empty block never becomes an indentation-only string.
        let output = super::indent::indent_text(
            &render::render_block_output(&results),
            &region.indent,
            None,
        );
        replacements.push((pair.span.clone(), output));
        report.shell_blocks_applied += 1;
    }

    // Apply replacements in reverse span order
    let mut new_content = content.to_string();
    apply_replacements_in_reverse(&mut new_content, replacements);
    report.shell_approvals_used += runtime.take_recent_approval_count();

    Ok((new_content, report))
}

/// Integration with Markdown::run_shell_blocks_stage.
///
/// This is called from the compose pipeline. It runs the shell blocks stage
/// and updates the report.
pub(crate) fn run_shell_blocks_stage_for_markdown(
    content: &mut String,
    options: &ComposeOptions,
    runtime: &mut ShellExpansionRuntime,
    report: &mut ComposeReport,
    ctx: &biscuit_terminal::errors::SourceContext,
    line_offset: usize,
) -> MarkdownResult<()> {
    let (new_content, stage_report) = run_shell_blocks_stage(content, options, runtime, ctx, line_offset)?;
    *content = new_content;
    report.shell_blocks_applied += stage_report.shell_blocks_applied;
    report.shell_approvals_used += stage_report.shell_approvals_used;
    report.warnings.extend(stage_report.warnings);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::compose::shell_expansion::types::{
        ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest, ShellExpansionOptions,
        ShellTimeoutBehavior,
    };
    use std::sync::Arc;
    use tempfile::TempDir;

    fn test_ctx() -> biscuit_terminal::errors::SourceContext {
        biscuit_terminal::errors::SourceContext::new(
            std::path::PathBuf::from("/test"),
            std::path::PathBuf::from("test"),
            String::new(),
        )
    }

    struct AllowAllHandler;

    impl ShellApprovalHandler for AllowAllHandler {
        fn approve(
            &self,
            _request: ShellApprovalRequest,
        ) -> Result<ShellApprovalDecision, crate::markdown::compose::ShellExpansionError> {
            Ok(ShellApprovalDecision::AllowOnce)
        }
    }

    struct DenyAllHandler;

    impl ShellApprovalHandler for DenyAllHandler {
        fn approve(
            &self,
            _request: ShellApprovalRequest,
        ) -> Result<ShellApprovalDecision, crate::markdown::compose::ShellExpansionError> {
            Ok(ShellApprovalDecision::Deny)
        }
    }

    /// Handler that denies commands containing "deny-me" in the raw command.
    struct DenyMatchingHandler {
        pattern: String,
    }

    impl ShellApprovalHandler for DenyMatchingHandler {
        fn approve(
            &self,
            request: ShellApprovalRequest,
        ) -> Result<ShellApprovalDecision, crate::markdown::compose::ShellExpansionError> {
            if request.raw_command.contains(&self.pattern) {
                Ok(ShellApprovalDecision::Deny)
            } else {
                Ok(ShellApprovalDecision::AllowOnce)
            }
        }
    }

    fn test_options_with_handler(
        handler: Arc<dyn ShellApprovalHandler>,
    ) -> (ComposeOptions, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let options = ComposeOptions::new()
            .with_shell_approval_handler(handler)
            .with_shell_policy_root(temp_dir.path())
            .with_shell_working_directory(std::env::current_dir().unwrap());
        (options, temp_dir)
    }

    #[test]
    fn no_shell_blocks() {
        let content = "# Hello\n\nNo blocks here.\n";
        let options = ComposeOptions::new();
        let mut runtime = ShellExpansionRuntime::new();
        let (result, report) =
            run_shell_blocks_stage(content, &options, &mut runtime, &test_ctx(), 0).unwrap();
        assert_eq!(result, content);
        assert_eq!(report.shell_blocks_applied, 0);
    }

    /// T7 — `::shell-block no_cache=true` runs every command fresh: two
    /// `uuidgen` lines in one block yield distinct values.
    #[test]
    fn shell_block_no_cache_executes_fresh() {
        if which::which("uuidgen").is_err() {
            return;
        }
        let content = "::shell-block no_cache=true\nuuidgen\nuuidgen\n::end-block\n";
        let (options, _temp) = test_options_with_handler(Arc::new(AllowAllHandler));
        let mut runtime = ShellExpansionRuntime::new();
        let (result, _report) =
            run_shell_blocks_stage(content, &options, &mut runtime, &test_ctx(), 0).unwrap();
        let ids: Vec<&str> = result.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
        assert_eq!(ids.len(), 2, "expected two output lines: {result:?}");
        assert_ne!(ids[0], ids[1], "no_cache=true must yield distinct values");
    }

    /// Default caching collapses identical commands across a shell block.
    #[test]
    fn shell_block_caches_by_default() {
        if which::which("uuidgen").is_err() {
            return;
        }
        let content = "::shell-block\nuuidgen\nuuidgen\n::end-block\n";
        let (options, _temp) = test_options_with_handler(Arc::new(AllowAllHandler));
        let mut runtime = ShellExpansionRuntime::new();
        let (result, _report) =
            run_shell_blocks_stage(content, &options, &mut runtime, &test_ctx(), 0).unwrap();
        let ids: Vec<&str> = result.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
        assert_eq!(ids.len(), 2, "expected two output lines: {result:?}");
        assert_eq!(ids[0], ids[1], "cached command must share one value");
    }

    #[test]
    fn empty_shell_block() {
        let content = "::shell-block\n::end-block\n";
        let options = ComposeOptions::new();
        let mut runtime = ShellExpansionRuntime::new();
        let (result, report) =
            run_shell_blocks_stage(content, &options, &mut runtime, &test_ctx(), 0).unwrap();
        assert_eq!(result, "");
        assert_eq!(report.shell_blocks_applied, 1);
    }

    #[test]
    fn single_command_block() {
        let content = "::shell-block\necho hello\n::end-block\n";
        let (options, _temp) = test_options_with_handler(Arc::new(AllowAllHandler));
        let mut runtime = ShellExpansionRuntime::new();
        let (result, report) =
            run_shell_blocks_stage(content, &options, &mut runtime, &test_ctx(), 0).unwrap();
        assert_eq!(result.trim(), "hello");
        assert_eq!(report.shell_blocks_applied, 1);
    }

    #[test]
    fn multiple_commands() {
        let content = "::shell-block\necho hello\necho world\n::end-block\n";
        let (options, _temp) = test_options_with_handler(Arc::new(AllowAllHandler));
        let mut runtime = ShellExpansionRuntime::new();
        let (result, report) =
            run_shell_blocks_stage(content, &options, &mut runtime, &test_ctx(), 0).unwrap();
        // Each command's output is concatenated verbatim; the line break between
        // them is each `echo`'s own trailing newline, not an inserted blank line.
        assert_eq!(result, "hello\nworld\n");
        assert_eq!(report.shell_blocks_applied, 1);
    }

    #[test]
    fn pipeline_command_block() {
        let content = "::shell-block\necho first && echo second\n::end-block\n";
        let (options, _temp) = test_options_with_handler(Arc::new(AllowAllHandler));
        let mut runtime = ShellExpansionRuntime::new();
        let (result, report) =
            run_shell_blocks_stage(content, &options, &mut runtime, &test_ctx(), 0).unwrap();
        assert_eq!(result, "first\n\nsecond\n");
        assert_eq!(report.shell_blocks_applied, 1);
    }

    #[test]
    fn pipeline_command_block_supports_stdout_null_redirect() {
        let content = "::shell-block\necho hidden > /dev/null && echo visible\n::end-block\n";
        let (options, _temp) = test_options_with_handler(Arc::new(AllowAllHandler));
        let mut runtime = ShellExpansionRuntime::new();
        let (result, report) =
            run_shell_blocks_stage(content, &options, &mut runtime, &test_ctx(), 0).unwrap();
        assert_eq!(result, "visible\n");
        assert_eq!(report.shell_blocks_applied, 1);
    }

    #[test]
    fn denial_prevents_execution() {
        let content = "::shell-block\necho hello\necho world\n::end-block\n";
        let (options, _temp) = test_options_with_handler(Arc::new(DenyAllHandler));
        let mut runtime = ShellExpansionRuntime::new();
        let result = run_shell_blocks_stage(content, &options, &mut runtime, &test_ctx(), 0);
        assert!(result.is_err());
    }

    #[test]
    fn denial_of_second_command_prevents_first_execution() {
        // All commands are prepared before any execute. If command 2 is denied
        // during preparation, command 1 must never execute.
        let content = "::shell-block\necho cmd1\necho deny-me\n::end-block\n";
        let (options, _temp) = test_options_with_handler(Arc::new(DenyMatchingHandler {
            pattern: "deny-me".to_string(),
        }));
        let mut runtime = ShellExpansionRuntime::new();
        let err = run_shell_blocks_stage(content, &options, &mut runtime, &test_ctx(), 0).unwrap_err();
        // The error should be about the second command being denied
        let msg = err.to_string();
        assert!(
            msg.contains("denied") || msg.contains("Approval"),
            "Expected denial error, got: {msg}"
        );
        // Command 1 should NOT have executed — no "cmd1" in output
        // (The function returns an error, so there's no output at all)
    }

    #[test]
    fn multiple_sibling_blocks() {
        let content = "::shell-block\necho a\n::end-block\n\n::shell-block\necho b\n::end-block\n";
        let (options, _temp) = test_options_with_handler(Arc::new(AllowAllHandler));
        let mut runtime = ShellExpansionRuntime::new();
        let (result, report) =
            run_shell_blocks_stage(content, &options, &mut runtime, &test_ctx(), 0).unwrap();
        assert_eq!(report.shell_blocks_applied, 2);
        assert!(result.contains("a"));
        assert!(result.contains("b"));
    }

    #[test]
    fn when_error_per_command() {
        let content =
            "::shell-block when_error=\"fallback\"\necho hello\nfalse\necho world\n::end-block\n";
        let (options, _temp) = test_options_with_handler(Arc::new(AllowAllHandler));
        let mut runtime = ShellExpansionRuntime::new();
        let (result, report) =
            run_shell_blocks_stage(content, &options, &mut runtime, &test_ctx(), 0).unwrap();
        assert_eq!(report.shell_blocks_applied, 1);
        // "hello" succeeds, "false" fails and is replaced with "fallback", "echo world" runs
        assert!(result.contains("hello"));
        assert!(result.contains("fallback"));
        assert!(result.contains("world"));
    }

    #[test]
    fn unhandled_failure_preserves_partial_output() {
        let content = "::shell-block\necho hello\nfalse\n::end-block\n";
        let (options, _temp) = test_options_with_handler(Arc::new(AllowAllHandler));
        let mut runtime = ShellExpansionRuntime::new();
        let err = run_shell_blocks_stage(content, &options, &mut runtime, &test_ctx(), 0).unwrap_err();
        match err {
            ShellBlockError::Command { partial_output, .. } => {
                assert_eq!(partial_output.len(), 1);
                assert_eq!(partial_output[0].trim(), "hello");
            }
            other => panic!("Expected Command error, got: {other:?}"),
        }
    }

    #[test]
    fn timeout_override() {
        let content = "::shell-block timeout=1\nsleep 5\n::end-block\n";
        let (options, _temp) = test_options_with_handler(Arc::new(AllowAllHandler));
        let mut runtime = ShellExpansionRuntime::new();
        let err = run_shell_blocks_stage(content, &options, &mut runtime, &test_ctx(), 0).unwrap_err();
        assert!(
            err.to_string().contains("timed out"),
            "Expected timeout error: {err}"
        );
    }

    #[test]
    fn pipeline_timeout_fallback_emits_warning() {
        // One timeout budget covers every `&&` segment, so `echo after` has to
        // spawn and exit inside it as well. A tight budget makes the second
        // segment time out too under parallel-suite load — the sleep must
        // overshoot the timeout by a wide margin, and the timeout must leave
        // room for a slow process spawn. The sleeping child is killed at the
        // timeout, so its length costs no wall clock.
        let content = "::shell-block\nsleep 10 && echo after\n::end-block\n";
        let temp_dir = TempDir::new().unwrap();
        let options = ComposeOptions::new().with_shell(ShellExpansionOptions {
            timeout: std::time::Duration::from_secs(2),
            timeout_behavior: ShellTimeoutBehavior::EmptyString,
            policy_root: Some(temp_dir.path().to_path_buf()),
            approval_handler: Some(Arc::new(AllowAllHandler)),
            ..Default::default()
        });
        let mut runtime = ShellExpansionRuntime::new();
        let (result, report) =
            run_shell_blocks_stage(content, &options, &mut runtime, &test_ctx(), 0).unwrap();
        assert!(result.contains("after"));
        assert_eq!(report.warnings.len(), 1);
        assert!(report.warnings[0].message.contains("timed out"));
    }

    // ── Phase 4 edge-case tests ───────────────────────────────────────

    #[test]
    fn unterminated_shell_block() {
        let content = "::shell-block\necho hello\n";
        let options = ComposeOptions::new();
        let mut runtime = ShellExpansionRuntime::new();
        let err = run_shell_blocks_stage(content, &options, &mut runtime, &test_ctx(), 0).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("Unterminated") || msg.contains("parse error"),
            "Expected unterminated error: {msg}"
        );
    }

    #[test]
    fn unmatched_end_block() {
        let content = "::end-block\n";
        let options = ComposeOptions::new();
        let mut runtime = ShellExpansionRuntime::new();
        let err = run_shell_blocks_stage(content, &options, &mut runtime, &test_ctx(), 0).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("Unmatched") || msg.contains("parse error"),
            "Expected unmatched error: {msg}"
        );
    }

    #[test]
    fn nested_page_block_with_shell_block() {
        // Page block evaluates to true, inner shell block should execute
        let content = "::block\n::shell-block\necho nested\n::end-block\n::end-block\n";
        let (options, _temp) = test_options_with_handler(Arc::new(AllowAllHandler));
        let mut runtime = ShellExpansionRuntime::new();
        let (result, report) =
            run_shell_blocks_stage(content, &options, &mut runtime, &test_ctx(), 0).unwrap();
        assert_eq!(report.shell_blocks_applied, 1);
        assert!(
            result.contains("nested"),
            "Expected nested output: {result}"
        );
    }

    #[test]
    fn all_commands_empty_output() {
        let content = "::shell-block\necho -n\necho -n\n::end-block\n";
        let (options, _temp) = test_options_with_handler(Arc::new(AllowAllHandler));
        let mut runtime = ShellExpansionRuntime::new();
        let (result, report) =
            run_shell_blocks_stage(content, &options, &mut runtime, &test_ctx(), 0).unwrap();
        assert_eq!(report.shell_blocks_applied, 1);
        assert_eq!(result, "", "Expected empty output for all-empty commands");
    }

    #[test]
    fn mixed_empty_and_non_empty_outputs() {
        let content = "::shell-block\necho -n\necho hello\necho -n\necho world\n::end-block\n";
        let (options, _temp) = test_options_with_handler(Arc::new(AllowAllHandler));
        let mut runtime = ShellExpansionRuntime::new();
        let (result, report) =
            run_shell_blocks_stage(content, &options, &mut runtime, &test_ctx(), 0).unwrap();
        assert_eq!(report.shell_blocks_applied, 1);
        // `echo -n` contributes an empty string, so it simply concatenates to
        // nothing; the surviving outputs keep their own trailing newlines.
        assert_eq!(result, "hello\nworld\n", "Expected mixed output: {result}");
    }

    #[test]
    fn continuation_with_escaped_backslash() {
        // echo \\ should produce a literal backslash, not continue
        // Then echo hello is a separate command
        let content = "::shell-block\necho \\\\\necho hello\n::end-block\n";
        let (options, _temp) = test_options_with_handler(Arc::new(AllowAllHandler));
        let mut runtime = ShellExpansionRuntime::new();
        let (result, report) =
            run_shell_blocks_stage(content, &options, &mut runtime, &test_ctx(), 0).unwrap();
        assert_eq!(report.shell_blocks_applied, 1);
        assert!(result.contains("\\"), "Expected backslash output: {result}");
        assert!(result.contains("hello"), "Expected hello output: {result}");
    }

    #[test]
    fn source_file_in_error() {
        let content = "::shell-block\nfalse\n::end-block\n";
        let (options, _temp) = test_options_with_handler(Arc::new(AllowAllHandler));
        let options = options.with_source_file("/tmp/test.md");
        let mut runtime = ShellExpansionRuntime::new();
        let err = run_shell_blocks_stage(content, &options, &mut runtime, &test_ctx(), 0).unwrap_err();
        match err {
            ShellBlockError::Command { source_file, .. } => {
                assert_eq!(source_file, Some(std::path::PathBuf::from("/tmp/test.md")));
            }
            other => panic!("Expected Command error, got: {other:?}"),
        }
    }

    // ── Phase 4 indentation tests ─────────────────────────────────────

    /// Composes a shell-block through the stage with an allow-once handler,
    /// returning the rewritten content.
    fn run_block_stage(content: &str) -> (String, ComposeReport) {
        let (options, _temp) = test_options_with_handler(Arc::new(AllowAllHandler));
        let mut runtime = ShellExpansionRuntime::new();
        run_shell_blocks_stage(content, &options, &mut runtime, &test_ctx(), 0).unwrap()
    }

    #[test]
    fn indented_block_output_under_list_item_is_reindented() {
        let content =
            "- intro\n\n    ::shell-block\n    echo hello\n    echo world\n    ::end-block\n\n- next\n";
        let (result, report) = run_block_stage(content);
        assert_eq!(report.shell_blocks_applied, 1);
        // Every output line keeps the opener's 4-space indent so the lines stay
        // nested under the list item rather than becoming siblings of the outer
        // list. The two outputs are adjacent (each `echo`'s own trailing newline
        // separates them) — no blank line is inserted between commands.
        assert!(result.contains("    hello\n    world\n"), "got: {result:?}");
        assert!(!result.contains("\nhello\n"), "got: {result:?}");
    }

    #[test]
    fn tab_indented_block_output_is_reindented() {
        let content =
            "- intro\n\n\t::shell-block\n\techo hello\n\techo world\n\t::end-block\n\n- next\n";
        let (result, report) = run_block_stage(content);
        assert_eq!(report.shell_blocks_applied, 1);
        assert!(result.contains("\thello\n\tworld\n"), "got: {result:?}");
    }

    #[test]
    fn root_level_block_output_has_no_indent() {
        let content = "intro\n\n::shell-block\necho hello\necho world\n::end-block\n\nnext\n";
        let (result, report) = run_block_stage(content);
        assert_eq!(report.shell_blocks_applied, 1);
        assert!(result.contains("hello\nworld\n"), "got: {result:?}");
        assert!(!result.contains("    hello"), "got: {result:?}");
    }

    #[test]
    fn block_output_preserves_trailing_whitespace_and_line_endings() {
        // Spec requirement 5: the only transformation a shell block applies to
        // captured output is container indentation. Trailing spaces and the
        // command's exact newline shape must survive byte-for-byte — the old
        // `trim()` contract stripped both.
        let content = "intro\n\n::shell-block\nprintf 'a  \\n\\nb\\n'\n::end-block\n\nnext\n";
        let (result, report) = run_block_stage(content);
        assert_eq!(report.shell_blocks_applied, 1);
        // `a  ` keeps its two trailing spaces; the interior blank line and the
        // final newline are preserved exactly as emitted.
        assert!(result.contains("a  \n\nb\n"), "got: {result:?}");
    }

    #[test]
    fn indented_block_trailing_newline_does_not_become_indent_only_line() {
        // `render_block_output` preserves the command's trailing newline
        // verbatim, so after re-indenting the block ends in a bare newline —
        // never an indentation-only `"    "` line. Mirrors the `::shell`
        // trailing-newline guarantee.
        let content = "- intro\n\n    ::shell-block\n    printf 'one\\n'\n    ::end-block\n\n- next\n";
        let (result, report) = run_block_stage(content);
        assert_eq!(report.shell_blocks_applied, 1);
        assert!(result.contains("    one\n\n- next"), "got: {result:?}");
        assert!(!result.contains("    one\n    "), "got: {result:?}");
    }

    #[test]
    fn empty_indented_block_does_not_become_indentation_only() {
        let content = "- intro\n\n    ::shell-block\n    ::end-block\n\n- next\n";
        let (result, report) = run_block_stage(content);
        assert_eq!(report.shell_blocks_applied, 1);
        // An empty rendered block stays empty; it must never collapse to a line
        // holding only the captured indent.
        assert_eq!(result, "- intro\n\n\n- next\n");
    }

    /// Composes a shell-block through the stage, then renders the rewritten
    /// content to HTML so the splice can be validated against the CommonMark
    /// block structure rather than raw substrings.
    fn run_block_stage_to_html(content: &str) -> String {
        let (rewritten, _report) = run_block_stage(content);
        let md: crate::markdown::Markdown = rewritten.as_str().into();
        md.as_html(crate::markdown::output::HtmlOptions::default())
            .unwrap()
    }

    #[test]
    fn indented_block_output_is_nested_under_list_item_in_commonmark() {
        // Re-indented block output is a continuation of the first list item, so
        // the document stays a single list with two items — the generated lines
        // are children of the parent <li>, not siblings of the outer list.
        let content =
            "- intro\n\n    ::shell-block\n    echo hello\n    echo world\n    ::end-block\n\n- next\n";
        let html = run_block_stage_to_html(content);
        assert_eq!(html.matches("<ul>").count(), 1, "got: {html}");
        assert_eq!(html.matches("<li>").count(), 2, "got: {html}");
    }

    #[test]
    fn root_level_block_output_is_a_sibling_block_in_commonmark() {
        // The column-1 baseline is intentionally a top-level sibling: the spliced
        // output splits the surrounding list into two separate <ul> blocks.
        let content =
            "- intro\n\n::shell-block\necho hello\necho world\n::end-block\n\n- next\n";
        let html = run_block_stage_to_html(content);
        assert_eq!(html.matches("<ul>").count(), 2, "got: {html}");
    }

    #[test]
    fn blockquote_marked_shell_block_output_is_re_quoted() {
        // A `>`-led shell block is recognized: the opener, body, and closer
        // markers are stripped for execution, and the rendered output is
        // re-quoted with the opener's `> ` prefix on every line so it stays
        // inside the block quote rather than escaping it.
        let content = "> ::shell-block\n> echo hello\n> echo world\n> ::end-block\n";
        let (result, report) = run_block_stage(content);
        assert_eq!(report.shell_blocks_applied, 1);
        assert!(result.contains("> hello\n> world\n"), "got: {result:?}");
        assert!(!result.contains("\nhello\n"), "got: {result:?}");
    }

    #[test]
    fn blockquote_shell_block_output_is_nested_in_blockquote_in_commonmark() {
        // The re-quoted output keeps the whole region inside a single
        // <blockquote> rather than breaking out into a sibling block.
        let content = "> ::shell-block\n> echo hello\n> echo world\n> ::end-block\n";
        let html = run_block_stage_to_html(content);
        assert_eq!(html.matches("<blockquote>").count(), 1, "got: {html}");
        assert!(html.contains("hello"), "got: {html}");
        assert!(html.contains("world"), "got: {html}");
    }

    #[test]
    fn nested_blockquote_marked_shell_block_output_is_re_quoted() {
        // A `> > `-led shell block replays the nested block-quote markers on
        // every emitted line so the output stays inside the nested quote rather
        // than escaping it.
        let content = "> > ::shell-block\n> > echo hello\n> > echo world\n> > ::end-block\n";
        let (result, report) = run_block_stage(content);
        assert_eq!(report.shell_blocks_applied, 1);
        assert!(
            result.contains("> > hello\n> > world\n"),
            "got: {result:?}"
        );
        assert!(!result.contains("\nhello\n"), "got: {result:?}");
    }

    #[test]
    fn nested_blockquote_shell_block_output_stays_nested_in_commonmark() {
        // The re-quoted output keeps the whole region inside the two-level
        // block quote: CommonMark renders nested <blockquote> wrappers rather
        // than the output breaking out into a sibling block.
        let content = "> > ::shell-block\n> > echo hello\n> > echo world\n> > ::end-block\n";
        let html = run_block_stage_to_html(content);
        assert_eq!(html.matches("<blockquote>").count(), 2, "got: {html}");
        assert!(html.contains("hello"), "got: {html}");
        assert!(html.contains("world"), "got: {html}");
    }

    #[test]
    fn prepare_failure_applies_line_offset() {
        let content = "::shell-block\necho hello\n::end-block\n";
        let (options, _temp) = test_options_with_handler(Arc::new(DenyAllHandler));
        let mut runtime = ShellExpansionRuntime::new();
        let err = run_shell_blocks_stage(content, &options, &mut runtime, &test_ctx(), 5)
            .unwrap_err();

        match err {
            ShellBlockError::Command {
                block_start_line,
                command_line,
                source,
                ..
            } => {
                assert_eq!(
                    block_start_line, 6,
                    "block_start_line should include line_offset"
                );
                assert_eq!(command_line, 7, "command_line should include line_offset");
                if let crate::markdown::compose::ShellExpansionError::Denied { origin, .. } =
                    source.as_ref()
                {
                    assert_eq!(
                        origin.line_number(),
                        7,
                        "inner origin should include line_offset"
                    );
                } else {
                    panic!("expected Denied error, got: {source:?}");
                }
            }
            other => panic!("expected Command error, got: {other:?}"),
        }
    }

    #[test]
    fn execute_failure_applies_line_offset() {
        if which::which("rustc").is_err() {
            return;
        }
        let content = "::shell-block\nrustc --edition=invalid\n::end-block\n";
        let (options, _temp) = test_options_with_handler(Arc::new(AllowAllHandler));
        let mut runtime = ShellExpansionRuntime::new();
        let err = run_shell_blocks_stage(content, &options, &mut runtime, &test_ctx(), 5)
            .unwrap_err();

        match err {
            ShellBlockError::Command {
                block_start_line,
                command_line,
                source,
                ..
            } => {
                assert_eq!(
                    block_start_line, 6,
                    "block_start_line should include line_offset"
                );
                assert_eq!(command_line, 7, "command_line should include line_offset");
                if let crate::markdown::compose::ShellExpansionError::ExecutionFailed {
                    origin,
                    ..
                } = source.as_ref()
                {
                    assert_eq!(
                        origin.line_number(),
                        7,
                        "inner origin should include line_offset"
                    );
                } else {
                    panic!("expected ExecutionFailed error, got: {source:?}");
                }
            }
            other => panic!("expected Command error, got: {other:?}"),
        }
    }
}
