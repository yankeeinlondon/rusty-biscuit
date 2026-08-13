//! Body interpolation compose stage.

use super::super::super::Markdown;
use super::super::super::types::MarkdownResult;
use super::super::context::effective_state as state;
use super::super::interpolation;
use super::super::shell_expansion;
use super::super::{ComposeOptions, ComposeReport, EffectiveState};
use tracing::debug;

/// Runs the interpolation stage.
///
/// Finds `{{ expression }}` patterns in content and evaluates them against the
/// effective state. Inline code spans (single backticks) are always scanned.
/// Fenced and indented code blocks are skipped by default; set
/// `interpolate_code_blocks` (via options or frontmatter) to scan them too.
///
/// ## Returns
///
/// The number of interpolations applied.
pub(crate) fn run_stage(
    markdown: &mut Markdown,
    state: &EffectiveState,
    options: &ComposeOptions,
    runtime: &shell_expansion::types::PipelineRuntime,
    report: &mut ComposeReport,
) -> MarkdownResult<usize> {
    use interpolation::{Evaluator, ScanMode, interpolate_text};

    let scan_mode = ScanMode::Body {
        include_code_blocks: resolve_interpolate_code_blocks(markdown, options),
    };

    // Wrap the effective state with a resolution context so read-side
    // expression functions (`frontmatter`, `file_exists`, `markdown_title`, …)
    // resolve filesystem paths and — when remote reads are enabled — HTTP(S)
    // URL arguments through the run's remote-fetch runtime.
    let lookup = state::ResolvingLookup::new(
        state,
        options.expression_resolution_context(&runtime.remote_fetch),
    );
    let evaluator = Evaluator::new(&lookup).with_presentation_values(state.presentation_values());
    let result = interpolate_text(
        markdown.content(),
        &evaluator,
        scan_mode,
        options.fail_fast,
        "interpolation",
    )?;

    if result.replacements > 0 {
        *markdown.content_mut() = result.output;
    }
    report.warnings.extend(result.warnings);
    debug!(
        count = result.replacements,
        "compose: interpolations applied"
    );
    Ok(result.replacements)
}

/// Resolves whether interpolation should process fenced/indented code blocks.
///
/// Inline code spans are always interpolated; this only governs fenced and
/// indented code blocks.
///
/// Checks (in priority order):
/// 1. `ComposeOptions::interpolate_code_blocks`
/// 2. Frontmatter `interpolate_code_blocks` key
fn resolve_interpolate_code_blocks(markdown: &Markdown, options: &ComposeOptions) -> bool {
    if options.interpolate_code_blocks {
        return true;
    }

    if let Ok(Some(value)) = markdown.fm_get::<bool>("interpolate_code_blocks") {
        return value;
    }

    false
}
