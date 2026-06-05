use std::collections::HashMap;

use biscuit_terminal::prelude::TerminalRenderable;
use claudine::events::EnvironmentContext;
use claudine::provider::Provider;
use claudine::stream::stderr::Verbosity;
use claudine::stream::summary::StreamExecutionSummary;

use crate::commands::wrap::profile::WrapperProfile;
use crate::commands::wrap::{
    StreamSummaryContext, StructuredSummaryDetails, emit_stream_summary_with_context,
    format_summary_prose, format_verbose_summary_details_prose,
};

/// Emit the structured-stream summary trailer for a composition run.
///
/// Unifies the two previous paths:
/// - `defer_section_separator = false` (compose): routes through
///   [`emit_stream_summary_with_context`] so the section tracker inserts the
///   separator blank exactly once.
/// - `defer_section_separator = true` (inline-compose): prints the trailer
///   directly so the caller controls the blank-line spacing (e.g. around
///   closure validation output).
///
/// Both paths write the JSONL summary event.
#[allow(clippy::too_many_arguments)]
pub(crate) fn emit_composition_summary(
    summary: &StreamExecutionSummary,
    details: &StructuredSummaryDetails,
    profile: &dyn WrapperProfile,
    env_context: &EnvironmentContext,
    verbosity: Verbosity,
    verbose: bool,
    dispatch_context: &HashMap<String, serde_json::Value>,
    section_stream: Option<&crate::commands::wrap::section::SectionStream>,
    defer_section_separator: bool,
    agent_pid: Option<u32>,
) {
    if defer_section_separator {
        use biscuit_terminal::components::prose::Prose;

        if verbosity != Verbosity::Silent
            && let Some(markup) = format_summary_prose(summary)
        {
            let term = crate::log::terminal();
            let rendered = Prose::new(markup).render(&term);
            eprintln!("{rendered}");
        }
        if verbosity != Verbosity::Silent
            && verbose
            && let Some(markup) = format_verbose_summary_details_prose(summary, details)
        {
            let term = crate::log::terminal();
            let rendered = Prose::new(markup).render(&term);
            eprintln!("  {rendered}");
        }

        if let Some(protocol) = profile.stream_protocol() {
            let meta = claudine::stream::reporting::summary_to_event_meta_with_context(
                summary,
                protocol,
                env_context,
                Some(dispatch_context),
                agent_pid,
            );
            if let Err(e) = claudine::stream::reporting::write_summary_event(&meta) {
                tracing::warn!("Failed to write stream summary event: {e}");
            }
        }
    } else {
        emit_stream_summary_with_context(
            StreamSummaryContext {
                summary,
                profile,
                env_context,
                verbosity,
                verbose,
                details,
                section_stream,
                agent_pid,
            },
            dispatch_context,
        );
    }
}

/// Emit a minimal composition summary for legacy (non-structured) runs.
///
/// Builds a minimal [`StreamExecutionSummary`] (exit_code, is_error, provider)
/// and routes through [`emit_composition_summary`] with no section stream and
/// deferred section separators. This delivers the same stderr trailer and
/// JSONL event that structured runs emit, so legacy paths reach full parity
/// with structured runs.
pub(crate) fn emit_minimal_composition_summary(
    provider: Provider,
    exit_code: i32,
    profile: &dyn WrapperProfile,
    env_context: &EnvironmentContext,
    dispatch_context: &HashMap<String, serde_json::Value>,
    agent_pid: Option<u32>,
) {
    let summary = StreamExecutionSummary {
        provider,
        exit_code,
        is_error: exit_code != 0,
        ..Default::default()
    };
    let details = StructuredSummaryDetails::default();
    emit_composition_summary(
        &summary,
        &details,
        profile,
        env_context,
        Verbosity::Normal,
        false,
        dispatch_context,
        None,
        true,
        agent_pid,
    );
}
