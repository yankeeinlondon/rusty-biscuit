use std::time::{Duration, Instant};

use biscuit_terminal::components::block_quote::BlockQuote;
use biscuit_terminal::components::metrics_tree::{MetricNode, MetricShare, MetricValue, MetricsTree};
use biscuit_terminal::components::renderable::TerminalRenderable as _;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::color::{Color, Tailwind};

/// The row shown in place of `unattributed` when the top-level rows add up to
/// more than the elapsed time. Top-level rows are recorded for sequential,
/// non-overlapping spans, so this row means a caller recorded overlapping
/// work at the top level instead of inside a group.
pub(crate) const OVER_ATTRIBUTED: &str = "over-attributed (overlapping top-level rows)";

/// Collects per-stage timings and renders a reconciling performance report.
///
/// Top-level rows are sequential spans of the process: they and
/// `unattributed` sum exactly to the total elapsed time. A group
/// ([`PerfCollector::record_group`]) is one top-level row whose duration is
/// its own measured elapsed time; its children ran concurrently inside that
/// span, so they are diagnostic only: shown with durations, never with a
/// share, and never added to the total.
pub(crate) struct PerfCollector {
    process_start: Instant,
    rows: Vec<Row>,
}

struct Row {
    name: &'static str,
    elapsed: Duration,
    children: Vec<(&'static str, Duration)>,
}

impl PerfCollector {
    pub(crate) fn new(process_start: Instant) -> Self {
        Self {
            process_start,
            rows: Vec::new(),
        }
    }

    /// Records one sequential top-level stage.
    pub(crate) fn record(&mut self, name: &'static str, elapsed: Duration) {
        self.record_group(name, elapsed, Vec::new());
    }

    /// Records a top-level group: `elapsed` is the group's own measured span
    /// (not a sum or maximum of `children`), and each child is a concurrent
    /// task measured inside it.
    pub(crate) fn record_group(
        &mut self,
        name: &'static str,
        elapsed: Duration,
        children: Vec<(&'static str, Duration)>,
    ) {
        self.rows.push(Row {
            name,
            elapsed,
            children,
        });
    }

    pub(crate) fn emit(&self) {
        eprint!("{}", self.render());
    }

    /// Every recorded name with its duration, a group followed by its
    /// children.
    #[cfg(test)]
    pub(crate) fn recorded_stages(&self) -> Vec<(&'static str, Duration)> {
        self.rows
            .iter()
            .flat_map(|row| std::iter::once((row.name, row.elapsed)).chain(row.children.iter().copied()))
            .collect()
    }

    fn render(&self) -> String {
        let tree = self.build_perf_tree();
        let wall = tree.total;
        let metrics = MetricsTree::new(to_metric_node(&tree, wall, 0));

        let term_width = Terminal::default().width();
        let inner_width = term_width.saturating_sub(2);
        let rendered = metrics.render_optimistic(Some(inner_width));

        let mut block = BlockQuote::from(rendered)
            .with_left_block_color(Color::Tailwind(Tailwind::Yellow400))
            .with_border("▌ ")
            .render_optimistic(Some(term_width));
        if !block.ends_with('\n') {
            block.push('\n');
        }
        block
    }

    pub(crate) fn build_perf_tree(&self) -> PerfNode {
        reconcile(self.process_start.elapsed(), &self.rows)
    }
}

/// The report for `rows` over `total` elapsed. The last top-level row is
/// `unattributed`, or [`OVER_ATTRIBUTED`] with the excess when the rows
/// exceed `total`: excess is surfaced, never clipped.
fn reconcile(total: Duration, rows: &[Row]) -> PerfNode {
    let mut children: Vec<PerfNode> = rows
        .iter()
        .map(|row| {
            let nested = row.children.iter().map(|(name, elapsed)| PerfNode::leaf(*name, *elapsed)).collect();
            PerfNode::branch(row.name, row.elapsed, nested)
        })
        .collect();

    let attributed: Duration = rows.iter().map(|row| row.elapsed).sum();
    children.push(match total.checked_sub(attributed) {
        Some(rest) => PerfNode::leaf("unattributed", rest),
        None => PerfNode::leaf(OVER_ATTRIBUTED, attributed - total),
    });

    PerfNode::branch("Performance", total, children)
}

/// Records a stage timing into an optional collector, letting callers avoid
/// branching on `Option<PerfCollector>` at every stage.
pub(crate) fn record(
    collector: &mut Option<PerfCollector>,
    name: &'static str,
    elapsed: Duration,
) {
    if let Some(c) = collector {
        c.record(name, elapsed);
    }
}

/// [`PerfCollector::record_group`] into an optional collector.
pub(crate) fn record_group(
    collector: &mut Option<PerfCollector>,
    name: &'static str,
    elapsed: Duration,
    children: Vec<(&'static str, Duration)>,
) {
    if let Some(c) = collector {
        c.record_group(name, elapsed, children);
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PerfNode {
    pub label: String,
    pub total: Duration,
    pub children: Vec<PerfNode>,
}

impl PerfNode {
    fn leaf(label: impl Into<String>, total: Duration) -> Self {
        Self {
            label: label.into(),
            total,
            children: Vec::new(),
        }
    }

    fn branch(label: impl Into<String>, total: Duration, children: Vec<PerfNode>) -> Self {
        Self {
            label: label.into(),
            total,
            children,
        }
    }
}

/// Only top-level rows carry a share of `wall`; a group's children overlap
/// each other, so a share would suggest they add up. The metrics tree has no
/// blank share, so they show its not-applicable dash.
fn to_metric_node(node: &PerfNode, wall: Duration, depth: usize) -> MetricNode {
    let value = MetricValue::Duration(node.total);
    let share = match depth {
        0 => MetricShare::Full,
        1 if !wall.is_zero() => MetricShare::Of(node.total.as_secs_f64() / wall.as_secs_f64()),
        _ => MetricShare::Unknown,
    };

    let children = node
        .children
        .iter()
        .map(|c| to_metric_node(c, wall, depth + 1))
        .collect();

    let mut metric = MetricNode::branch(node.label.clone(), value, share, children);
    metric.emphasize = depth == 0;
    metric
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    fn ms(value: u64) -> Duration {
        Duration::from_millis(value)
    }

    fn row(name: &'static str, elapsed: Duration, children: Vec<(&'static str, Duration)>) -> Row {
        Row { name, elapsed, children }
    }

    fn top_level_sum(tree: &PerfNode) -> Duration {
        tree.children.iter().map(|c| c.total).sum()
    }

    #[test]
    fn build_perf_tree_reconciles_children_to_root_total() {
        let start = Instant::now();
        thread::sleep(Duration::from_millis(20));
        let mut collector = PerfCollector::new(start);
        collector.record("stage-a", Duration::from_millis(5));
        collector.record("stage-b", Duration::from_millis(3));

        let tree = collector.build_perf_tree();

        assert_eq!(tree.label, "Performance");
        assert!(!tree.total.is_zero());
        assert_eq!(tree.total, top_level_sum(&tree), "children plus unattributed must equal the root total");
    }

    #[test]
    fn overlapping_group_children_are_excluded_from_the_top_level_sum() {
        let rows = [
            row("pre-dispatch", ms(10), vec![]),
            row(
                "remote wait ‖ local gather",
                ms(300),
                vec![("remote wait", ms(300)), ("list gather", ms(150)), ("graph gather", ms(175))],
            ),
            row("table render", ms(5), vec![]),
        ];

        let tree = reconcile(ms(330), &rows);

        let labels: Vec<_> = tree.children.iter().map(|c| (c.label.as_str(), c.total)).collect();
        assert_eq!(
            labels,
            [("pre-dispatch", ms(10)), ("remote wait ‖ local gather", ms(300)), ("table render", ms(5)), ("unattributed", ms(15))]
        );
        assert_eq!(top_level_sum(&tree), tree.total, "top-level rows reconcile exactly");
        let group = &tree.children[1];
        let nested: Vec<_> = group.children.iter().map(|c| (c.label.as_str(), c.total)).collect();
        assert_eq!(nested, [("remote wait", ms(300)), ("list gather", ms(150)), ("graph gather", ms(175))]);
        let nested_sum: Duration = group.children.iter().map(|c| c.total).sum();
        assert!(nested_sum > group.total, "the children overlap, so adding them would double count");
    }

    #[test]
    fn groups_keep_their_measured_span_and_sit_beside_sequential_rows() {
        let mut collector = PerfCollector::new(Instant::now());
        collector.record("pr gather", ms(2));
        collector.record_group("local gather", ms(40), vec![("list gather", ms(40)), ("verbose gather", ms(12))]);
        collector.record_group("regather", ms(25), vec![("list regather", ms(20)), ("verbose regather", ms(9))]);
        collector.record("checkout status refresh", ms(3));

        let tree = reconcile(ms(100), &collector.rows);

        let shape: Vec<_> = tree
            .children
            .iter()
            .map(|c| (c.label.as_str(), c.total, c.children.iter().map(|n| n.label.as_str()).collect::<Vec<_>>()))
            .collect();
        assert_eq!(
            shape,
            [
                ("pr gather", ms(2), vec![]),
                ("local gather", ms(40), vec!["list gather", "verbose gather"]),
                ("regather", ms(25), vec!["list regather", "verbose regather"]),
                ("checkout status refresh", ms(3), vec![]),
                ("unattributed", ms(30), vec![]),
            ]
        );
        assert_eq!(
            collector.recorded_stages(),
            [
                ("pr gather", ms(2)),
                ("local gather", ms(40)),
                ("list gather", ms(40)),
                ("verbose gather", ms(12)),
                ("regather", ms(25)),
                ("list regather", ms(20)),
                ("verbose regather", ms(9)),
                ("checkout status refresh", ms(3)),
            ],
            "recorded_stages flattens a group into its name then its children"
        );
    }

    #[test]
    fn rows_beyond_the_elapsed_time_are_surfaced_not_clipped() {
        let rows = [row("list gather", ms(30), vec![]), row("graph gather", ms(25), vec![])];

        let tree = reconcile(ms(40), &rows);

        let last = tree.children.last().expect("a reconciling row");
        assert_eq!((last.label.as_str(), last.total), (OVER_ATTRIBUTED, ms(15)));
        assert!(tree.children.iter().all(|c| c.label != "unattributed"), "no zero `unattributed` hides the excess");
    }

    #[test]
    fn only_top_level_rows_carry_a_share() {
        let rows = [row("local gather", ms(50), vec![("list gather", ms(50)), ("graph gather", ms(30))])];

        let metric = to_metric_node(&reconcile(ms(100), &rows), ms(100), 0);

        assert_eq!(metric.share, MetricShare::Full);
        let group = &metric.children[0];
        assert_eq!(group.share, MetricShare::Of(0.5));
        assert!(group.children.iter().all(|child| child.share == MetricShare::Unknown));
        assert_eq!(metric.children[1].share, MetricShare::Of(0.5), "unattributed");
    }

    #[test]
    fn record_appends_stages_in_order() {
        let mut collector = PerfCollector::new(Instant::now());
        collector.record("first", Duration::from_nanos(100));
        collector.record("second", Duration::from_nanos(200));

        let tree = collector.build_perf_tree();
        let labels: Vec<&str> = tree.children.iter().map(|c| c.label.as_str()).collect();
        assert_eq!(labels, vec!["first", "second", "unattributed"]);
    }

    #[test]
    fn empty_collector_tree_has_only_unattributed() {
        let start = Instant::now();
        thread::sleep(Duration::from_millis(1));
        let collector = PerfCollector::new(start);

        let tree = collector.build_perf_tree();
        assert_eq!(tree.children.len(), 1);
        assert_eq!(tree.children[0].label, "unattributed");
        assert!(!tree.children[0].total.is_zero());
    }
}
