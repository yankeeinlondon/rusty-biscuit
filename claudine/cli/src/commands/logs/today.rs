use biscuit_terminal::components::renderable::TerminalRenderable;
use claudine::render::MetricsReport;
use claudine::reporting::DailySummary;

use crate::log;

/// Render the daily summary through the [`MetricsReport`] component.
///
/// The component owns the terminal string-building (kept byte-identical with
/// the previous inline implementation) and carries the mandatory browser
/// target; the CLI keeps only sink wiring — which display terminal, and the
/// plain-mode-aware inline terminal for colored spans.
pub(super) fn render_daily_summary(summary: &DailySummary, error_hint: Option<&str>) {
    let term = crate::log::terminal();
    let report = MetricsReport::new(summary.clone())
        .with_error_window(error_hint.unwrap_or("today"))
        .with_inline_terminal(crate::log::optimistic_terminal(None));
    log::data(&report.render(&term));
}
