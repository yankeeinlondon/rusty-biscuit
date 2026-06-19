//! Body shell-expansion compose stage (condition-aware execution).

use super::super::super::Markdown;
use super::super::super::types::MarkdownResult;
use super::super::indent;
use super::super::perf;
use super::super::shell_expansion::{
    self, apply_replacements_in_reverse, execute_directive_detailed,
};
use super::super::{ComposeOptions, ComposeReport, ShellCommandSpan, redact_shell_command};
use tracing::debug;

/// Runs Stage 1 shell expansion directives.
pub(crate) fn run_stage(
    markdown: &mut Markdown,
    options: &ComposeOptions,
    runtime: &mut shell_expansion::types::PipelineRuntime,
    report: &mut ComposeReport,
    perf: &mut perf::PerfCollector,
) -> MarkdownResult<()> {
    let line_offset = markdown.frontmatter_line_count();
    let directives = shell_expansion::parse_directives(
        markdown.content(),
        markdown.full_source_context_for_errors(),
        line_offset,
    )?;
    debug!(
        directive_count = directives.len(),
        "compose: shell expansion directives found"
    );
    if directives.is_empty() {
        return Ok(());
    }

    let shell_opts = options.shell_options();
    let policy_paths = shell_expansion::resolve_policy_paths(&shell_opts, &options.source)?;
    runtime.shell.ensure_loaded(&policy_paths)?;

    let mut replacements = Vec::new();

    for directive in directives {
        let span_start = perf.is_enabled().then(std::time::Instant::now);
        let execution =
            execute_directive_detailed(&directive, options, &policy_paths, &mut runtime.shell)?;
        if let Some(start) = span_start {
            perf.record_shell_span(ShellCommandSpan {
                command_display: redact_shell_command(&directive.raw_command),
                command_hash: format!("{:016x}", biscuit_hash::xx_hash(&directive.raw_command)),
                elapsed: start.elapsed(),
            });
        }
        // Re-indent multi-line output to the directive's column so generated
        // lines stay nested under the surrounding list or block quote.
        let output = indent::indent_text(&execution.combined_output(), &directive.indent, None);
        replacements.push((directive.span.clone(), output));
        report.warnings.extend(execution.warnings);
        report.shell_expansions_applied += 1;
    }

    apply_replacements_in_reverse(markdown.content_mut(), replacements);
    report.shell_approvals_used += runtime.shell.take_recent_approval_count();
    Ok(())
}
