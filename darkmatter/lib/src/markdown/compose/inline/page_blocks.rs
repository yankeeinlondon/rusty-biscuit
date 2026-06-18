//! Page-blocks compose stage (conditional content regions).

use super::super::super::Markdown;
use super::super::super::types::MarkdownResult;
use super::super::context::effective_state as state;
use super::super::page_blocks;
use super::super::shell_expansion;
use super::super::{ComposeOptions, ComposeReport, ComposeWarning, EffectiveState};
use tracing::debug;

/// Runs page blocks (conditional content regions).
pub(crate) fn run_stage(
    markdown: &mut Markdown,
    state: &EffectiveState,
    options: &ComposeOptions,
    runtime: &shell_expansion::types::PipelineRuntime,
    report: &mut ComposeReport,
) -> MarkdownResult<()> {
    debug!("compose: running page blocks");
    let source = markdown.source_context_for_errors();
    let regions = page_blocks::parser::parse_page_blocks(markdown.content(), source.clone())?;
    if regions.is_empty() {
        return Ok(());
    }

    // Warn for unknown options
    fn warn_unknown_options(region: &page_blocks::PageBlockRegion, report: &mut ComposeReport) {
        for unknown in &region.options.unknown_options {
            report.add_warning(
                ComposeWarning::new(
                    "page_blocks",
                    format!("Unknown page block option: '{}'", unknown),
                )
                .at_line(region.start_line),
            );
        }
        for child in &region.children {
            warn_unknown_options(child, report);
        }
    }
    for region in &regions {
        warn_unknown_options(region, report);
    }

    let lookup = state::ResolvingLookup::new(
        state,
        options.expression_resolution_context(&runtime.remote_fetch),
    );
    let rendered =
        page_blocks::engine::render_page_blocks(markdown.content(), &regions, &lookup, report, source)?;
    *markdown.content_mut() = rendered;
    Ok(())
}
