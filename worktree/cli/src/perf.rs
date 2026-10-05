//! Renders a [`Timings`] document for `wt list --perf`; the measuring is the
//! `worktree` library's ([`worktree::timing`]) and the listing command's.
//!
//! The human report is a metrics tree. A sequential child shows its share of
//! its parent; a concurrent child shows none, since it overlapped its
//! siblings. Each sequential parent gets a generated `unattributed` row (hidden
//! under [`UNATTRIBUTED_FLOOR`]) or an `over-attributed` row (always shown),
//! so every level visibly reconciles. Worker reports are a separate,
//! labeled diagnostic section with no shares, its heading included: they were
//! measured in another process and are not part of the foreground total. Their
//! sequential parents reconcile the same way, with share-less remainder rows.

use std::time::Duration;

use biscuit_terminal::components::block_quote::BlockQuote;
use biscuit_terminal::components::metrics_tree::{MetricNode, MetricShare, MetricValue, MetricsTree};
use biscuit_terminal::components::renderable::TerminalRenderable as _;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::color::{Color, Tailwind};
use worktree::timing::{ChildrenKind, LaunchReport, Span, SpanList, Stage, Timings, WorkerReportStatus};

/// The prefix of the one `--perf=json` line; consumers take the final
/// nonempty stderr line and deserialize what follows it.
pub(crate) const JSON_PREFIX: &str = "WT_PERF_JSON ";

/// A human `unattributed` row shorter than this is not shown.
pub(crate) const UNATTRIBUTED_FLOOR: Duration = Duration::from_millis(1);

pub(crate) const UNATTRIBUTED: &str = "unattributed";
pub(crate) const OVER_ATTRIBUTED: &str = "over-attributed";
const WORKER_HEADING: &str = "Refresh worker (diagnostic, measured in the worker)";

/// Runs `work` as `stage` into `steps`, or only runs it.
pub(crate) fn time<T>(steps: Option<&mut SpanList>, stage: Stage, work: impl FnOnce() -> T) -> T {
    match steps {
        Some(steps) => steps.time(stage, work),
        None => work(),
    }
}

/// The `--perf=json` record: a newline, then one framed line.
pub(crate) fn json_record(timings: &Timings) -> String {
    format!("\n{JSON_PREFIX}{}\n", timings.to_json())
}

/// The human report, sized to the terminal.
pub(crate) fn human_report(timings: &Timings) -> String {
    human_report_at(timings, Terminal::default().width())
}

fn human_report_at(timings: &Timings, term_width: u32) -> String {
    let inner_width = term_width.saturating_sub(2);
    let mut rendered = MetricsTree::new(report_tree(timings)).render_optimistic(Some(inner_width));
    if let Some(workers) = worker_tree(timings) {
        rendered.push('\n');
        rendered.push_str(&MetricsTree::new(workers).with_given_root_share().render_optimistic(Some(inner_width)));
    }
    let mut block = BlockQuote::from(rendered)
        .with_left_block_color(Color::Tailwind(Tailwind::Yellow400))
        .with_border("▌ ")
        .render_optimistic(Some(term_width));
    if !block.ends_with('\n') {
        block.push('\n');
    }
    block
}

/// The foreground tree: `Performance`, then each top-level span and the
/// root's remainder row.
pub(crate) fn report_tree(timings: &Timings) -> MetricNode {
    let total = timings.total();
    let mut rows: Vec<MetricNode> =
        timings.spans().iter().map(|span| span_node(span, Some(total), Shares::Shown)).collect();
    rows.extend(remainder_row(timings.unattributed(), timings.over_attributed(), total, Shares::Shown));
    MetricNode::branch("Performance", MetricValue::Duration(total), MetricShare::Full, rows).emphasized()
}

/// Whether a projected row shows its share of its sequential parent.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Shares {
    Shown,
    /// Worker rows: they are not part of the foreground total.
    Hidden,
}

/// `part`'s share of a sequential parent of `parent` elapsed.
fn share_of(part: Duration, parent: Duration, shares: Shares) -> MetricShare {
    if shares == Shares::Hidden || parent.is_zero() {
        MetricShare::Unknown
    } else {
        MetricShare::Of(part.as_secs_f64() / parent.as_secs_f64())
    }
}

/// `span`'s row; `parent` is the elapsed time of a sequential parent, `None`
/// under a concurrent one.
fn span_node(span: &Span, parent: Option<Duration>, shares: Shares) -> MetricNode {
    let share = parent.map_or(MetricShare::Unknown, |parent| share_of(span.elapsed(), parent, shares));
    let own = (span.children_kind() == ChildrenKind::Sequential).then_some(span.elapsed());
    let mut children: Vec<MetricNode> =
        span.children().iter().map(|child| span_node(child, own, shares)).collect();
    if own.is_some() && !span.children().is_empty() {
        children.extend(remainder_row(span.unattributed(), span.over_attributed(), span.elapsed(), shares));
    }
    MetricNode::branch(span_label(span), MetricValue::Duration(span.elapsed()), share, children)
}

/// The stage's label, with its `git` count when it is known.
pub(crate) fn span_label(span: &Span) -> String {
    match span.git_calls() {
        Some(calls) => format!("{}  [{calls} git]", span.stage().label()),
        None => span.stage().label().to_string(),
    }
}

/// The generated row reconciling a sequential parent of `parent` elapsed.
fn remainder_row(
    unattributed: Duration,
    over_attributed: Duration,
    parent: Duration,
    shares: Shares,
) -> Option<MetricNode> {
    let share = |part: Duration| share_of(part, parent, shares);
    if !over_attributed.is_zero() {
        return Some(MetricNode::leaf(OVER_ATTRIBUTED, MetricValue::Duration(over_attributed), share(over_attributed)));
    }
    (unattributed >= UNATTRIBUTED_FLOOR)
        .then(|| MetricNode::leaf(UNATTRIBUTED, MetricValue::Duration(unattributed), share(unattributed)))
}

/// The worker section: one row per owned launch, reconciled like the
/// foreground but with no shares; `None` when no worker was followed.
pub(crate) fn worker_tree(timings: &Timings) -> Option<MetricNode> {
    let status = timings.worker_report_status()?;
    let launches = timings
        .worker_reports()
        .iter()
        .map(|report| {
            let label = format!("launch {} ({})", report.launch_index, status_text(report.status()));
            match &report.report {
                LaunchReport::Complete(worker) | LaunchReport::Partial(worker) => {
                    let total = worker.total();
                    let mut rows: Vec<MetricNode> =
                        worker.spans().iter().map(|span| span_node(span, Some(total), Shares::Hidden)).collect();
                    rows.extend(remainder_row(worker.unattributed(), worker.over_attributed(), total, Shares::Hidden));
                    MetricNode::branch(label, MetricValue::Duration(total), MetricShare::Unknown, rows)
                }
                LaunchReport::Missing | LaunchReport::Invalid => {
                    MetricNode::leaf(label, MetricValue::Placeholder, MetricShare::Unknown)
                }
            }
        })
        .collect();
    Some(MetricNode::branch(
        format!("{WORKER_HEADING}: {}", status_text(status)),
        MetricValue::Placeholder,
        MetricShare::Unknown,
        launches,
    ))
}

fn status_text(status: WorkerReportStatus) -> &'static str {
    match status {
        WorkerReportStatus::Complete => "complete",
        WorkerReportStatus::Partial => "partial",
        WorkerReportStatus::Missing => "missing",
        WorkerReportStatus::Invalid => "invalid",
        WorkerReportStatus::Adopted => "adopted",
        WorkerReportStatus::OriginChanged => "origin changed",
    }
}

#[cfg(test)]
mod tests;
