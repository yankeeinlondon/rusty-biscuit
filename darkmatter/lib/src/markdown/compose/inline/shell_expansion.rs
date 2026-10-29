//! Body shell-expansion compose stage (condition-aware execution).

use super::super::super::Markdown;
use super::super::super::types::MarkdownResult;
use super::super::indent;
use super::super::perf;
use super::super::body_origin::{BodyProvenance, EditOrigin, apply_replacements_with_edits};
use super::super::context::effective_state::ResolvingLookup;
use super::super::expression::ExpressionFinder;
use super::super::page_blocks::BodyGuards;
use super::super::schema_validation::PreparedSchemas;
use super::super::shell_expansion::site::{SiteFrame, SiteScope, TemplateSite, authored_context, prepare_site};
use super::super::shell_expansion::template::LookupEvaluator;
use super::super::shell_expansion::types::{ShellCommandOrigin, ShellDirective, ShellExpansionError};
use super::super::body_origin::DataRanges;
use super::super::shell_expansion::{self, ExecutionMode, execute_prepared_directive, parser};
use super::super::{ComposeReport, EffectiveState, ShellCommandSpan, redact_shell_command};
use tracing::debug;

/// Runs Stage 1 shell expansion directives.
///
/// Only authored directives run (see [`shell_expansion::parser::parse_directives_in`]);
/// their output is data.
///
/// A directive whose authored text holds a placeholder is also read as a
/// template: its authored command is recovered through `body.authored`, typed
/// against the document's schema narrowed by the authored `::block when=`
/// guards around it, and its holes are read from `state`. It
/// runs in template mode when that is what authorizes it (see
/// [`prepare_site`]), including when the value interpolated into its current
/// text holds quotes or operators, so the text no longer parses as a command.
#[allow(clippy::too_many_arguments)]
pub(crate) fn run_stage(
    markdown: &mut Markdown,
    state: &EffectiveState,
    schemas: &PreparedSchemas,
    options: &crate::markdown::compose::ComposeRequest,
    runtime: &mut shell_expansion::types::PipelineRuntime,
    report: &mut ComposeReport,
    perf: &mut perf::PerfCollector,
    body: &mut BodyProvenance,
) -> MarkdownResult<()> {
    let line_offset = markdown.frontmatter_line_count();
    body.data.ensure_describes(markdown.content())?;
    let lines = parser::scan_directive_lines(
        markdown.content(),
        Some(&body.data),
        markdown.full_source_context_for_errors(),
        line_offset,
    );
    debug!(
        directive_count = lines.len(),
        "compose: shell expansion directives found"
    );
    if lines.is_empty() {
        return Ok(());
    }

    // The authored command of each line that holds a placeholder, with its
    // authored byte offset.
    let authored: Vec<Option<(String, usize, usize)>> = lines
        .iter()
        .map(|line| {
            let authored = body.authored.as_ref()?;
            let range = authored.project(markdown.content(), &line.keyword)?;
            let (command, file_line) = parser::authored_directive_at(authored.text(), range.start, line_offset)?;
            (!ExpressionFinder::scan_plain(&command).expressions.is_empty()).then_some((command, file_line, range.start))
        })
        .collect();
    // The page-block stage already removed every false block, so a site's
    // enclosing `::block when=` guards held; they are read from the authored
    // body, as discovery read them.
    let body_guards = body
        .authored
        .as_ref()
        .filter(|_| authored.iter().any(Option::is_some))
        .map(|authored| BodyGuards::scan_body(authored.text(), line_offset))
        .unwrap_or_default();
    let lookup = ResolvingLookup::new(state, options.expression_resolution_context(&runtime.remote_fetch));
    let evaluator = LookupEvaluator::new(&lookup);
    let scope = authored
        .iter()
        .any(Option::is_some)
        .then(|| SiteScope::new(options, schemas.effective_for(markdown)));
    let authored_ctx = authored_context(markdown, body.authored.as_ref().map(|authored| authored.text()));

    // Every directive is read before any runs, so one that cannot run fails
    // before the first command does.
    #[allow(clippy::type_complexity)]
    let mut directives: Vec<(
        Result<ShellDirective, ShellExpansionError>,
        Option<TemplateSite<'_>>,
        std::ops::Range<usize>,
        std::ops::Range<usize>,
        String,
    )> = Vec::with_capacity(lines.len());
    for (line, authored) in lines.into_iter().zip(authored) {
        let site = match (authored, &scope) {
            (Some((command, file_line, authored_offset)), Some(scope)) => {
                let frame = match &line.parsed {
                    Ok(directive) => Some(SiteFrame::of(directive)),
                    Err(_) => parser::authored_directive_options(&command, file_line, &authored_ctx)?.map(|options| {
                        SiteFrame {
                            span: line.span.clone(),
                            indent: line.indent.clone(),
                            error_handling: options.error_handling,
                            timeout_override: options.timeout_override,
                            no_cache: options.no_cache,
                        }
                    }),
                };
                match frame {
                    Some(frame) => scope.site(
                        &command,
                        parser::authored_template(&command),
                        ShellCommandOrigin::Body { line: file_line },
                        authored_ctx.clone(),
                        &evaluator,
                        frame,
                        &body_guards.enclosing(authored_offset),
                    )?,
                    None => None,
                }
            }
            _ => None,
        };
        // A line break a value inserted stays inside the directive in
        // template mode, where the value is one argument.
        let extent = site
            .as_ref()
            .map_or_else(|| line.span.clone(), |_| directive_extent(markdown.content(), &body.data, &line));
        let parsed = match (line.parsed, &site) {
            (Err(error), None) => return Err(error.into()),
            (parsed, _) => parsed,
        };
        directives.push((parsed, site, line.span, extent, line.indent));
    }

    let shell_opts = options.shell_options();
    let policy_paths = shell_expansion::resolve_policy_paths(&shell_opts, &options.source, options.resolution_context())?;
    runtime.shell.ensure_loaded(&policy_paths)?;

    let mut replacements = Vec::new();

    for (parsed, site, span, extent, line_indent) in directives {
        let span_start = perf.is_enabled().then(std::time::Instant::now);
        let prepared = prepare_site(parsed, site.as_ref(), options, &policy_paths, &mut runtime.shell)?;
        let execution = execute_prepared_directive(&prepared, options, &runtime.shell)?;
        if let Some(start) = span_start {
            let raw_command = &prepared.effective.raw_command;
            perf.record_shell_span(ShellCommandSpan {
                command_display: redact_shell_command(raw_command),
                command_hash: format!("{:016x}", biscuit_hash::xx_hash(raw_command)),
                elapsed: start.elapsed(),
            });
        }
        // Re-indent multi-line output to the directive's column so generated
        // lines stay nested under the surrounding list or block quote.
        let output = indent::indent_text(&execution.combined_output(), &line_indent, None);
        let span = match prepared.mode {
            ExecutionMode::Template => extent,
            ExecutionMode::ResolvedBytes => span,
        };
        replacements.push((span, output));
        report.warnings.extend(execution.warnings);
        report.shell_expansions_applied += 1;
        // A blocking warning this command reported stops the next one.
        super::super::pipeline::stop_if_blocked(report, options, None)?;
    }

    let before = markdown.content().to_string();
    let edits = apply_replacements_with_edits(markdown.content_mut(), replacements, EditOrigin::Data);
    body.advance(&before, &edits, markdown.content())?;
    report.shell_approvals_used += runtime.shell.take_recent_approval_count();
    Ok(())
}

/// `line`'s extent in `content`: through the first authored line break after
/// its keyword, so a line break a value inserted belongs to the directive.
fn directive_extent(content: &str, data: &DataRanges, line: &parser::DirectiveLine) -> std::ops::Range<usize> {
    let mut from = line.keyword.end;
    while let Some(offset) = content[from..].find('\n') {
        let at = from + offset;
        if !data.intersects(&(at..at + 1)) {
            return line.span.start..at + 1;
        }
        from = at + 1;
    }
    line.span.start..content.len()
}
