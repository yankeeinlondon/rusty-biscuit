//! Collection-blocks compose stage (`::map` / `::filter` regions).

use super::super::super::Markdown;
use super::super::super::types::{AuthoredSpan, MarkdownError, MarkdownResult, SourceRef};
use super::super::body_origin::BodyProvenance;
use super::super::collection_blocks::engine::{Execution, Halt, structure};
use super::super::collection_blocks::CollectionError;
use super::super::collection_blocks::parser::parse_collection_regions_in;
use super::super::context::effective_state as state;
use super::super::context::report::CandidateLocus;
use super::super::page_blocks;
use super::super::pipeline::{FinalVerdict, ReportWatch};
use super::super::schema_validation::PreparedSchemas;
use super::super::shell_expansion;
use super::super::{ComposeOperation, ComposeReport, EffectiveState};
use tracing::debug;

/// Whether `markdown`'s body holds an active collection region, in which case
/// this stage owns the page blocks too: a gate inside a repeated body must
/// see the iteration it is repeated for.
pub(crate) fn owns_page_blocks(
    markdown: &Markdown,
    options: &crate::markdown::compose::ComposeRequest,
    body: &BodyProvenance,
) -> bool {
    let content = markdown.content();
    options.is_enabled(ComposeOperation::CollectionBlocks)
        && (content.contains("::map") || content.contains("::filter"))
        && parse_collection_regions_in(content, Some(&body.data), &options.enabled_operations, None)
            .map_or(true, |regions| !regions.is_empty())
}

/// Expands every active `::map` / `::filter` region, together with the page
/// blocks around and inside them, and carries `body` past the expansion.
///
/// Callbacks run here, in document order, before any body interpolation or
/// shell stage; the repeated bodies leave this stage as authored text with a
/// recorded lexical frame per copy.
pub(crate) fn run_stage(
    markdown: &mut Markdown,
    state: &EffectiveState,
    options: &crate::markdown::compose::ComposeRequest,
    runtime: &shell_expansion::types::PipelineRuntime,
    report: &mut ComposeReport,
    body: &mut BodyProvenance,
    schemas: &PreparedSchemas,
) -> MarkdownResult<()> {
    body.data.ensure_describes(markdown.content())?;
    let content = markdown.content();
    if !content.contains("::map") && !content.contains("::filter") {
        return Ok(());
    }
    debug!("compose: running collection blocks");
    let collections = parse_collection_regions_in(
        content,
        Some(&body.data),
        &options.enabled_operations,
        Some(std::path::Path::new("document")),
    )
    .map_err(|error| rejection(markdown, body, error))?;
    if collections.is_empty() {
        return Ok(());
    }
    let source = markdown.source_context_for_errors();
    let pages = if options.is_enabled(ComposeOperation::PageBlocks) {
        page_blocks::parser::parse_page_blocks_in(content, Some(&body.data), source.clone())?
    } else {
        Vec::new()
    };
    for region in &pages {
        super::page_blocks::warn_unknown_options(region, report);
    }

    let lookup = state::ResolvingLookup::new(
        state,
        options.expression_resolution_context(&runtime.remote_fetch),
    );
    let recorded = report.unknown_root_candidates.len();
    let mut stopped = None;
    let expansion = {
        let mut watch = ReportWatch::new(report, options, FinalVerdict { markdown, state, schemas });
        let mut stop = |report: &ComposeReport| match watch.as_mut().map(|watch| watch.check(report)) {
            Some(Err(error)) => {
                stopped = Some(error);
                true
            }
            _ => false,
        };
        let nodes = structure(&pages, &collections);
        Execution::new(content, &lookup, &runtime.iteration, source, report, &mut stop).expand(&nodes)
    };
    let expansion = match expansion {
        Ok(expansion) => expansion,
        Err(Halt::Blocked) => return Err(stopped.expect("the stop recorded its error")),
        Err(Halt::Failed(error)) => return Err(anchor(markdown, body, *error)),
    };
    report.unknown_root_candidates.relocate_from(recorded, |locus| match locus {
        CandidateLocus::BodyLine(line) => super::super::unknown_identifiers::directive_locus(markdown, *line),
        other => other.clone(),
    });
    body.expand(markdown.content(), &expansion.splices, &expansion.output, expansion.scopes)?;
    *markdown.content_mut() = expansion.output;
    Ok(())
}

fn rejection(markdown: &Markdown, body: &BodyProvenance, error: CollectionError) -> MarkdownError {
    anchor(markdown, body, error.into())
}

/// A collection failure located in the loaded file the way body
/// interpolation locates its own: the engine located it in the body text.
fn anchor(markdown: &Markdown, body: &BodyProvenance, error: MarkdownError) -> MarkdownError {
    let MarkdownError::Collection(inner) = error else {
        return error;
    };
    let span = match inner.site().map(|site| site.source.as_ref()) {
        Some(SourceRef::OnDiskSpan { span, .. }) => Some(span.range()),
        _ => None,
    };
    let origin = body.origin.as_ref().filter(|origin| origin.describes(markdown.content()));
    let located = match (span, markdown.loaded_source_context_for_errors(), origin) {
        (Some(span), Some(context), Some(origin)) => origin
            .project(&span)
            .and_then(|range| AuthoredSpan::locate(&context.content, range))
            .map(|authored| SourceRef::OnDiskSpan { context, span: authored }),
        _ => None,
    };
    match located {
        Some(source) => (*inner).with_source(source).into(),
        None => MarkdownError::Collection(inner),
    }
}
