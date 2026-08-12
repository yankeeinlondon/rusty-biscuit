//! Rendering of a built [`CommandPerfReport`] into the styled `--perf` tree.
//!
//! Projects the assembled [`PerfNode`] tree ([`super::tree`]) into a
//! biscuit-terminal [`MetricNode`] (BT-1) and writes the single stderr
//! artifact every `--perf` emit site shares.

use std::time::Duration;

use biscuit_terminal::components::metrics_tree::{
    MetricMarker, MetricNode, MetricShare, MetricValue,
};

use super::tree::build_perf_tree;
use super::{CommandPerfReport, Marker, NodeRole, PerfNode};

/// True when an `unattributed` remainder is small enough to omit (TR-3 render
/// rule): below `1ms` **and** below `1%` of wall-clock. A material remainder is
/// always kept — it honestly signals an uninstrumented window.
fn is_remainder_noise(total: Duration, wall: Duration) -> bool {
    if total >= Duration::from_millis(1) {
        return false;
    }
    if wall.is_zero() {
        return true;
    }
    total.as_secs_f64() / wall.as_secs_f64() * 100.0 < 1.0
}

/// Drop below-threshold `unattributed` remainders so they don't clutter the
/// tree (TR-3). Pruning happens before connectors are computed so the
/// `├─`/`└─` choice reflects the rows that actually render.
fn prune_unattributed_noise(node: &mut PerfNode, wall: Duration) {
    node.children
        .retain(|c| !(c.role == NodeRole::Unattributed && is_remainder_noise(c.total, wall)));
    for child in &mut node.children {
        prune_unattributed_noise(child, wall);
    }
}

/// Insert the dry-run `agent execution` placeholder just before the root's
/// `unattributed` remainder (P-5). Zero total → `0%` share; the row collector
/// renders it as the `—` leaf.
fn inject_dry_run_agent(tree: &mut PerfNode) {
    let pos = tree
        .children
        .iter()
        .position(|c| c.role == NodeRole::Unattributed)
        .unwrap_or(tree.children.len());
    tree.children.insert(
        pos,
        PerfNode::leaf("agent execution", Duration::ZERO, NodeRole::Structural),
    );
}

/// Largest non-`Unattributed` leaf total in the tree, if any (P-3 candidate).
fn max_leaf_total(node: &PerfNode) -> Option<Duration> {
    if node.children.is_empty() {
        return (node.role != NodeRole::Unattributed).then_some(node.total);
    }
    node.children.iter().filter_map(max_leaf_total).max()
}

/// Flag the first leaf matching `target` as `HOT`.
fn set_hot(node: &mut PerfNode, target: Duration) -> bool {
    if node.children.is_empty() {
        if node.role != NodeRole::Unattributed && node.total == target {
            node.marker = Some(Marker::Hot);
            return true;
        }
        return false;
    }
    node.children.iter_mut().any(|c| set_hot(c, target))
}

/// Mark the dominant leaf `HOT`, but only if it clears the materiality floor
/// (≥20% of wall-clock); otherwise there is no hot spot worth pointing at (P-3).
fn mark_dominant_leaf(tree: &mut PerfNode, wall: Duration) {
    if wall.is_zero() {
        return;
    }
    if let Some(max) = max_leaf_total(tree)
        && !max.is_zero()
        && max.as_secs_f64() / wall.as_secs_f64() >= 0.20
    {
        set_hot(tree, max);
    }
}

/// Project a [`PerfNode`] into a biscuit-terminal [`MetricNode`] (BT-1).
///
/// The claudine tree decides *content* — which value, which share, which row is
/// hot, and the dry-run placeholder/annotation — while the shared component owns
/// connectors, column alignment, and the highlight glyph. `wall` is the report
/// total used for the share-of-wall-clock column (P-2); the root reads `100%`.
fn to_metric_node(node: &PerfNode, wall: Duration, is_root: bool, dry_run: bool) -> MetricNode {
    // The injected dry-run placeholder is the only `agent execution` leaf in a
    // dry-run report (the real agent node is omitted), so this match is exact.
    let is_dry_agent = dry_run && node.children.is_empty() && node.label == "agent execution";

    let value = if is_dry_agent {
        MetricValue::Placeholder
    } else {
        MetricValue::Duration(node.total)
    };
    let share = if is_root {
        MetricShare::Full
    } else if is_dry_agent || wall.is_zero() {
        MetricShare::Unknown
    } else {
        MetricShare::Of(node.total.as_secs_f64() / wall.as_secs_f64())
    };

    let children = node
        .children
        .iter()
        .map(|c| to_metric_node(c, wall, false, dry_run))
        .collect();

    let mut metric = MetricNode::branch(node.label.clone(), value, share, children);
    metric.calls = node.calls;
    metric.emphasize = is_root;
    if node.marker == Some(Marker::Hot) {
        metric.marker = Some(MetricMarker::Highlight);
    }
    if is_dry_agent {
        metric.note = Some("(dry run)".to_string());
    }
    metric
}

/// Render a built [`CommandPerfReport`] to stderr.
///
/// The single stderr write for every `--perf` emit site. Routing all
/// sites through here keeps the report a human-facing, stderr-only
/// artifact (G-8) and prevents the headline-sampling logic from drifting
/// back apart across the wrapper, composition, and sequence paths (TM-1).
pub(crate) fn emit_report(report: &CommandPerfReport) {
    eprint!("{}", render_perf_report(report));
}

/// Render a [`CommandPerfReport`] to a styled string suitable for stderr.
///
/// The report is one reconciling tree (TM-2): a `Performance` root over
/// `Structural` buckets that sum back to wall-clock. The renderer walks it
/// generically — box-drawing connectors from depth (P-1), a unit-aligned
/// duration column (P-4), a share-of-wall-clock percent column (P-2), and a
/// single `HOT` marker on the dominant leaf (P-3) — so the imbalance that
/// matters reads as the report's headline finding rather than noise.
pub(crate) fn render_perf_report(report: &CommandPerfReport) -> String {
    use biscuit_terminal::components::block_quote::BlockQuote;
    use biscuit_terminal::components::metrics_tree::MetricsTree;
    use biscuit_terminal::components::renderable::TerminalRenderable as _;
    use biscuit_terminal::terminal::Terminal;
    use biscuit_terminal::utils::color::{Color, Tailwind};

    let mut tree = build_perf_tree(report, report.placement);
    let wall = tree.total;
    // Dry run: the agent was skipped, so the tree carries no agent node; the
    // note flags it. We render a `—` placeholder leaf in its place (P-5).
    //
    // A sequence carries no top-level agent node even when agents ran (execution
    // lives under the `steps` node), so the injected `—` leaf and the note
    // suppression do not apply — the dry-run note renders as a trailing note and
    // the per-step subtrees already show the absence of agent children.
    let dry_run = report.sequence_steps.is_empty()
        && report.agent.is_none()
        && report.notes.iter().any(|n| n.contains("dry run"));

    prune_unattributed_noise(&mut tree, wall);
    if dry_run {
        inject_dry_run_agent(&mut tree);
    }
    mark_dominant_leaf(&mut tree, wall);

    // Notes other than the dry-run note, which the `—` leaf already conveys.
    let notes: Vec<String> = report
        .notes
        .iter()
        .filter(|n| !(dry_run && n.contains("dry run")))
        .cloned()
        .collect();

    // The shared component (BT-1) owns connectors, the unit-aligned value
    // column (P-4), the share column (P-2), and the single HOT marker (P-3);
    // claudine only projects its tree and chooses the highlighted row.
    let metrics = MetricsTree::new(to_metric_node(&tree, wall, true, dry_run)).with_notes(notes);

    // Render at the real terminal width minus the `▌ ` border so MetricsTree
    // caps its label column to exactly what survives inside the BlockQuote;
    // otherwise a long `::shell` label inflates the shared column and every row
    // wraps onto two lines (the `render_optimistic(None)` 80-col assumption hid
    // this until a verbose command appeared). The border is 2 columns wide.
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
