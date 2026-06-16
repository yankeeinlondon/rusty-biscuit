use std::time::{Duration, Instant};

use biscuit_terminal::components::block_quote::BlockQuote;
use biscuit_terminal::components::metrics_tree::{MetricNode, MetricShare, MetricValue, MetricsTree};
use biscuit_terminal::components::renderable::TerminalRenderable as _;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::color::{Color, Tailwind};

/// Collects per-stage timings and renders a reconciling performance report.
pub(crate) struct PerfCollector {
    process_start: Instant,
    stages: Vec<(&'static str, Duration)>,
}

impl PerfCollector {
    pub(crate) fn new(process_start: Instant) -> Self {
        Self {
            process_start,
            stages: Vec::new(),
        }
    }

    pub(crate) fn record(&mut self, name: &'static str, elapsed: Duration) {
        self.stages.push((name, elapsed));
    }

    pub(crate) fn emit(&self) {
        eprint!("{}", self.render());
    }

    fn render(&self) -> String {
        let tree = self.build_perf_tree();
        let wall = tree.total;
        let metrics = MetricsTree::new(to_metric_node(&tree, wall, true));

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

    fn build_perf_tree(&self) -> PerfNode {
        let total = self.process_start.elapsed();
        let mut children: Vec<PerfNode> = self
            .stages
            .iter()
            .map(|(name, elapsed)| PerfNode::leaf(*name, *elapsed))
            .collect();

        let attributed: Duration = self.stages.iter().map(|(_, elapsed)| *elapsed).sum();
        let unattributed = total.saturating_sub(attributed);
        children.push(PerfNode::leaf("unattributed", unattributed));

        PerfNode::branch("Performance", total, children)
    }
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

fn to_metric_node(node: &PerfNode, wall: Duration, is_root: bool) -> MetricNode {
    let value = MetricValue::Duration(node.total);
    let share = if is_root {
        MetricShare::Full
    } else if wall.is_zero() {
        MetricShare::Unknown
    } else {
        MetricShare::Of(node.total.as_secs_f64() / wall.as_secs_f64())
    };

    let children = node
        .children
        .iter()
        .map(|c| to_metric_node(c, wall, false))
        .collect();

    let mut metric = MetricNode::branch(node.label.clone(), value, share, children);
    metric.emphasize = is_root;
    metric
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn build_perf_tree_reconciles_children_to_root_total() {
        let start = Instant::now();
        thread::sleep(Duration::from_millis(20));
        let mut collector = PerfCollector::new(start);
        collector.record("stage-a", Duration::from_millis(5));
        collector.record("stage-b", Duration::from_millis(3));

        let tree = collector.build_perf_tree();
        let children_total: Duration = tree.children.iter().map(|c| c.total).sum();

        assert_eq!(tree.label, "Performance");
        assert!(!tree.total.is_zero());
        assert_eq!(
            tree.total, children_total,
            "children plus unattributed must equal the root total"
        );
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
