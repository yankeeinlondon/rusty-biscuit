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

use super::shell_expansion::types::{
    ShellCommandOrigin, ShellDirective, ShellExpansionRuntime,
};
use super::shell_expansion::{apply_replacements_in_reverse, execute_prepared_directive, prepare_directive, resolve_policy_paths};
use super::types::ComposeReport;
use super::ComposeOptions;
use crate::markdown::MarkdownResult;
use types::{ShellBlockCommandResult, SourceExcerpt};

/// Run the shell blocks stage on the given content.
///
/// Returns the transformed content and a compose report.
pub(crate) fn run_shell_blocks_stage(
    content: &str,
    options: &ComposeOptions,
    runtime: &mut ShellExpansionRuntime,
) -> Result<(String, ComposeReport), ShellBlockError> {
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
    let policy_paths = resolve_policy_paths(&shell_opts,
        &options.source
    ).map_err(|e| ShellBlockError::Parse {
        line: 0,
        message: format!("Policy path resolution failed: {e}"),
        excerpt: SourceExcerpt::default(),
    })?;

    runtime.ensure_loaded(&policy_paths).map_err(|e| {
        ShellBlockError::Parse {
            line: 0,
            message: format!("Policy loading failed: {e}"),
            excerpt: SourceExcerpt::default(),
        }
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
                origin: ShellCommandOrigin::Body { line: command.start_line },
                error_handling: region.options.clone(),
                timeout_override: region.timeout_override,
            };

            match prepare_directive(&directive,
                options,
                &policy_paths,
                runtime
            ) {
                Ok(p) => prepared.push((p, command.clone())),
                Err(e) => {
                    return Err(ShellBlockError::Command {
                        block_start_line: pair.start_line,
                        command_line: command.start_line,
                        partial_output: Vec::new(),
                        excerpt: SourceExcerpt::from_text(
                            body_text,
                            command.start_line,
                            pair.start_line + 1,
                            2,
                        ),
                        source: e,
                    });
                }
            }
        }

        // Execute sequentially
        let mut results = Vec::new();

        for (prep, command) in prepared {
            match execute_prepared_directive(&prep,
                options
            ) {
                Ok(execution) => {
                    results.push(ShellBlockCommandResult {
                        output: execution.combined_output(),
                        command: command.clone(),
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
                        block_start_line: pair.start_line,
                        command_line: command.start_line,
                        partial_output: partial,
                        excerpt: SourceExcerpt::from_text(
                            body_text,
                            command.start_line,
                            pair.start_line + 1,
                            2,
                        ),
                        source: e,
                    });
                }
            }
        }

        let output = render::render_block_output(&results);
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
) -> MarkdownResult<()> {
    let (new_content, stage_report) = run_shell_blocks_stage(content, options, runtime)?;
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
        ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest,
    };
    use std::sync::Arc;
    use tempfile::TempDir;

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

    fn test_options_with_handler(handler: Arc<dyn ShellApprovalHandler>) -> (ComposeOptions, TempDir) {
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
        let (result, report) = run_shell_blocks_stage(content, &options, &mut runtime).unwrap();
        assert_eq!(result, content);
        assert_eq!(report.shell_blocks_applied, 0);
    }

    #[test]
    fn empty_shell_block() {
        let content = "::shell-block\n::end-block\n";
        let options = ComposeOptions::new();
        let mut runtime = ShellExpansionRuntime::new();
        let (result, report) = run_shell_blocks_stage(content, &options, &mut runtime).unwrap();
        assert_eq!(result, "");
        assert_eq!(report.shell_blocks_applied, 1);
    }

    #[test]
    fn single_command_block() {
        let content = "::shell-block\necho hello\n::end-block\n";
        let (options, _temp) = test_options_with_handler(Arc::new(AllowAllHandler));
        let mut runtime = ShellExpansionRuntime::new();
        let (result, report) = run_shell_blocks_stage(content, &options, &mut runtime).unwrap();
        assert_eq!(result.trim(), "hello");
        assert_eq!(report.shell_blocks_applied, 1);
    }

    #[test]
    fn multiple_commands() {
        let content = "::shell-block\necho hello\necho world\n::end-block\n";
        let (options, _temp) = test_options_with_handler(Arc::new(AllowAllHandler));
        let mut runtime = ShellExpansionRuntime::new();
        let (result, report) = run_shell_blocks_stage(content, &options, &mut runtime).unwrap();
        assert_eq!(result, "hello\n\nworld\n");
        assert_eq!(report.shell_blocks_applied, 1);
    }

    #[test]
    fn denial_prevents_execution() {
        let content = "::shell-block\necho hello\necho world\n::end-block\n";
        let (options, _temp) = test_options_with_handler(Arc::new(DenyAllHandler));
        let mut runtime = ShellExpansionRuntime::new();
        let result = run_shell_blocks_stage(content, &options, &mut runtime);
        assert!(result.is_err());
    }

    #[test]
    fn multiple_sibling_blocks() {
        let content = "::shell-block\necho a\n::end-block\n\n::shell-block\necho b\n::end-block\n";
        let (options, _temp) = test_options_with_handler(Arc::new(AllowAllHandler));
        let mut runtime = ShellExpansionRuntime::new();
        let (result, report) = run_shell_blocks_stage(content, &options, &mut runtime).unwrap();
        assert_eq!(report.shell_blocks_applied, 2);
        assert!(result.contains("a"));
        assert!(result.contains("b"));
    }

    #[test]
    fn when_error_per_command() {
        let content = "::shell-block when_error=\"fallback\"\necho hello\nfalse\necho world\n::end-block\n";
        let (options, _temp) = test_options_with_handler(Arc::new(AllowAllHandler));
        let mut runtime = ShellExpansionRuntime::new();
        let (result, report) = run_shell_blocks_stage(content, &options, &mut runtime).unwrap();
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
        let err = run_shell_blocks_stage(content, &options, &mut runtime).unwrap_err();
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
        let err = run_shell_blocks_stage(content, &options, &mut runtime).unwrap_err();
        assert!(err.to_string().contains("timed out"), "Expected timeout error: {err}");
    }
}
