//! Page-blocks compose stage (conditional content regions).

use super::super::super::Markdown;
use super::super::super::types::MarkdownResult;
use super::super::context::effective_state as state;
use super::super::body_origin::BodyProvenance;
use super::super::page_blocks;
use super::super::shell_expansion;
use super::super::context::report::CandidateLocus;
use super::super::{ComposeReport, ComposeWarning, EffectiveState};
use super::super::pipeline::{FinalVerdict, ReportWatch};
use super::super::schema_validation::PreparedSchemas;
use tracing::debug;

/// Runs page blocks (conditional content regions) and carries `body` past
/// the regions they remove. A marker line that holds data is not a marker.
pub(crate) fn run_stage(
    markdown: &mut Markdown,
    state: &EffectiveState,
    options: &crate::markdown::compose::ComposeRequest,
    runtime: &shell_expansion::types::PipelineRuntime,
    report: &mut ComposeReport,
    body: &mut BodyProvenance,
    schemas: &PreparedSchemas,
) -> MarkdownResult<()> {
    if super::collection_blocks::owns_page_blocks(markdown, options, body) {
        return Ok(());
    }
    debug!("compose: running page blocks");
    let mut watch = ReportWatch::new(report, options, FinalVerdict { markdown, state, schemas });
    let source = markdown.source_context_for_errors();
    body.data.ensure_describes(markdown.content())?;
    let regions = page_blocks::parser::parse_page_blocks_in(
        markdown.content(),
        Some(&body.data),
        source.clone(),
    )?;
    if regions.is_empty() {
        return Ok(());
    }

    for region in &regions {
        warn_unknown_options(region, report);
    }

    let lookup = state::ResolvingLookup::new(
        state,
        options.expression_resolution_context(&runtime.remote_fetch),
    );
    let recorded = report.unknown_root_candidates.len();
    // A blocking finding stops before the next condition runs: a condition
    // may compose nested content.
    let mut stopped = None;
    let mut stop = |report: &ComposeReport| match watch.as_mut().map(|watch| watch.check(report)) {
        Some(Err(error)) => {
            stopped = Some(error);
            true
        }
        _ => false,
    };
    let rendered = page_blocks::engine::render_page_blocks_with_edits(
        markdown.content(),
        &regions,
        &lookup,
        report,
        source,
        &mut stop,
    )?;
    let Some((rendered, edits)) = rendered else {
        return Err(stopped.expect("the stop recorded its error"));
    };
    report.unknown_root_candidates.relocate_from(recorded, |locus| match locus {
        CandidateLocus::BodyLine(line) => {
            super::super::unknown_identifiers::directive_locus(markdown, *line)
        }
        other => other.clone(),
    });
    body.advance(markdown.content(), &edits, &rendered)?;
    *markdown.content_mut() = rendered;
    Ok(())
}

/// Reports each unknown option of `region` and its nested blocks.
pub(super) fn warn_unknown_options(region: &page_blocks::PageBlockRegion, report: &mut ComposeReport) {
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
