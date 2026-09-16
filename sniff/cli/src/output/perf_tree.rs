//! Projection of a [`PerformanceReport`] into `biscuit-terminal` metric nodes.
//!
//! Sniff owns the report-to-tree projection: hierarchy from dotted stage names,
//! measured-versus-synthetic values, wall-clock shares, call counts, and the
//! single HOT selection. `biscuit-terminal` owns everything about how that tree
//! reaches a terminal — width math, connectors, unit alignment, glyph folding,
//! and color degradation.

use std::collections::BTreeMap;
use std::time::Duration;

use biscuit_terminal::components::metrics_tree::{
    MetricMarker, MetricNode, MetricShare, MetricValue, MetricsTree,
};
use sniff::PerformanceReport;
use sniff::performance::PerformanceStage;

/// Detection domains whose stages re-parent below an existing `detect.<domain>`
/// branch. This alias set is the only detection-specific mapping in the
/// projection; every other dotted name is parsed generically.
const DETECTION_DOMAINS: [&str; 4] = ["os", "hardware", "network", "filesystem"];

/// The request-total stage. It becomes the synthetic root rather than a child
/// of it, so it is never inserted into the tree.
const ROOT_STAGE: &str = "detect.total";

/// Why sibling durations can exceed their parent and why the tree is not
/// reconciled against wall-clock time.
pub(crate) const OVERLAP_NOTE: &str = "Concurrent, nested, and repeated stages may overlap; their durations do not sum to wall-clock time.";

/// Build the timing tree with its overlap note attached, ready to render.
pub(crate) fn timing_metrics_tree(report: &PerformanceReport) -> MetricsTree {
    MetricsTree::new(timing_tree(report)).with_notes(vec![OVERLAP_NOTE.to_string()])
}

/// Build the counter tree, or `None` when the report recorded no counters.
pub(crate) fn counter_metrics_tree(report: &PerformanceReport) -> Option<MetricsTree> {
    Some(MetricsTree::new(counter_tree(report)?))
}

/// Project a report's timing stages into a `Total`-rooted [`MetricNode`] tree.
///
/// Every non-root share is measured against the report wall clock, not against
/// the node's parent, and is deliberately not clamped: repeated and concurrent
/// stages legitimately accumulate more duration than the request took.
pub(crate) fn timing_tree(report: &PerformanceReport) -> MetricNode {
    let wall_clock = ms_to_duration(report.total_duration_ms);

    let mut root: PathNode<StageFacts> = PathNode::new("");
    let mut hot: Option<(Duration, &str, String)> = None;

    for (name, stage) in &report.stages {
        let Some(path) = stage_path(name, &report.stages) else {
            continue;
        };
        root.insert(
            &path,
            StageFacts {
                total_duration_ms: stage.total_duration_ms,
                calls: stage.calls,
                hot: false,
            },
        );

        let duration = ms_to_duration(stage.total_duration_ms);
        let wins = match hot.as_ref() {
            None => true,
            Some((best, best_name, _)) => {
                duration > *best || (duration == *best && name.as_str() < *best_name)
            }
        };
        if wins {
            hot = Some((duration, name.as_str(), path));
        }
    }

    if let Some((_, _, path)) = hot.as_ref()
        && let Some(node) = root.get_mut(path)
        && let Some(facts) = node.measured.as_mut()
    {
        facts.hot = true;
    }

    MetricNode::branch(
        "Total",
        MetricValue::Duration(wall_clock),
        MetricShare::Full,
        sorted_children(&root, wall_clock),
    )
    .emphasized()
}

/// The tree path a stage key occupies, or `None` when the key is not rendered.
///
/// `<domain>.<rest>` re-parents below `detect.<domain>` only when that stage was
/// actually measured. A focused report that never ran full detection keeps the
/// generic path; a synthetic `detect` node is never fabricated.
fn stage_path(name: &str, stages: &BTreeMap<String, PerformanceStage>) -> Option<String> {
    if name == ROOT_STAGE {
        return None;
    }
    match name.split_once('.') {
        Some((domain, rest))
            if !rest.is_empty()
                && DETECTION_DOMAINS.contains(&domain)
                && stages.contains_key(&format!("detect.{domain}")) =>
        {
            Some(format!("detect.{name}"))
        }
        _ => Some(name.to_string()),
    }
}

/// A measured timing stage, plus whether it won the HOT selection.
struct StageFacts {
    total_duration_ms: f64,
    calls: u64,
    hot: bool,
}

/// An intermediate dotted-name tree, built before projection into
/// [`MetricNode`]s.
///
/// A node is *measured* when a report key resolves exactly to it and *synthetic*
/// when it exists only to carry children. A measured node that later gains
/// children keeps its own payload — `filesystem.shared_walk` is both a measured
/// stage and the parent of `filesystem.shared_walk.docs`.
struct PathNode<M> {
    segment: String,
    measured: Option<M>,
    children: BTreeMap<String, PathNode<M>>,
}

impl<M> PathNode<M> {
    fn new(segment: impl Into<String>) -> Self {
        Self {
            segment: segment.into(),
            measured: None,
            children: BTreeMap::new(),
        }
    }

    /// Insert `payload` at `path`, creating missing intermediates as synthetic
    /// nodes. A path with no non-empty segment inserts nothing, so a malformed
    /// key can never overwrite the root.
    fn insert(&mut self, path: &str, payload: M) {
        let mut node = self;
        let mut placed = false;
        for segment in path.split('.').filter(|s| !s.is_empty()) {
            node = node
                .children
                .entry(segment.to_string())
                .or_insert_with(|| PathNode::new(segment));
            placed = true;
        }
        if placed {
            node.measured = Some(payload);
        }
    }

    fn get_mut(&mut self, path: &str) -> Option<&mut PathNode<M>> {
        let mut node = self;
        for segment in path.split('.').filter(|s| !s.is_empty()) {
            node = node.children.get_mut(segment)?;
        }
        Some(node)
    }
}

/// Project one dotted-name node and its descendants.
fn timing_node(path: &PathNode<StageFacts>, wall_clock: Duration) -> MetricNode {
    let children = sorted_children(path, wall_clock);
    let duration = match &path.measured {
        Some(facts) => ms_to_duration(facts.total_duration_ms),
        None => children.iter().map(node_duration).sum(),
    };

    // Labels reach `MetricsTree` unescaped on purpose: the component measures
    // and pads them before handing them to Prose, so escaping here would steal
    // a visible column from every row's width accounting. `build_markup`
    // escapes the already-padded label instead.
    let mut node = MetricNode::branch(
        path.segment.clone(),
        MetricValue::Duration(duration),
        share_of(duration, wall_clock),
        children,
    );
    if let Some(facts) = &path.measured {
        if facts.calls > 1 {
            node = node.with_calls(usize::try_from(facts.calls).unwrap_or(usize::MAX));
        }
        if facts.hot {
            node = node.with_marker(MetricMarker::Highlight);
        }
    }
    node
}

fn sorted_children(parent: &PathNode<StageFacts>, wall_clock: Duration) -> Vec<MetricNode> {
    let mut children: Vec<MetricNode> = parent
        .children
        .values()
        .map(|child| timing_node(child, wall_clock))
        .collect();
    children.sort_by(|a, b| {
        node_duration(b)
            .cmp(&node_duration(a))
            .then_with(|| a.label.cmp(&b.label))
    });
    children
}

fn node_duration(node: &MetricNode) -> Duration {
    match node.value {
        MetricValue::Duration(duration) => duration,
        _ => Duration::ZERO,
    }
}

fn share_of(value: Duration, wall_clock: Duration) -> MetricShare {
    if wall_clock.is_zero() {
        MetricShare::Unknown
    } else {
        MetricShare::Of(value.as_secs_f64() / wall_clock.as_secs_f64())
    }
}

/// Convert a millisecond measurement into a [`Duration`].
///
/// [`Duration::from_secs_f64`] panics on NaN, infinite, and negative input, and
/// every figure in a `PerformanceReport` is an unvalidated `f64` that a caller
/// may have deserialized or hand-built. Anything that is not a finite, positive
/// measurement collapses to [`Duration::ZERO`] rather than aborting the CLI.
fn ms_to_duration(ms: f64) -> Duration {
    if ms.is_finite() && ms > 0.0 {
        Duration::from_secs_f64(ms / 1000.0)
    } else {
        Duration::ZERO
    }
}

/// Project a report's work counters into a `Counters`-rooted [`MetricNode`] tree.
///
/// Counter names are parsed generically: none of the timing tree's
/// detection-domain aliasing, root-stage suppression, or HOT selection applies
/// here. Heterogeneous work counters share no meaningful denominator, so every
/// node — the root included — carries [`MetricShare::Unknown`]. The component
/// still prints `100%` on the root, which is expected and not a defect: it
/// overrides the root's share unconditionally.
pub(crate) fn counter_tree(report: &PerformanceReport) -> Option<MetricNode> {
    if report.counters.is_empty() {
        return None;
    }

    let mut root: PathNode<u64> = PathNode::new("");
    for (name, count) in &report.counters {
        root.insert(name, *count);
    }

    let children = sorted_counter_children(&root);
    Some(
        MetricNode::branch(
            "Counters",
            MetricValue::Count(sum_counts(&children)),
            MetricShare::Unknown,
            children,
        )
        .emphasized(),
    )
}

/// Project one dotted-name counter node and its descendants.
fn counter_node(path: &PathNode<u64>) -> MetricNode {
    let children = sorted_counter_children(path);
    let count = match path.measured {
        Some(count) => count,
        None => sum_counts(&children),
    };

    MetricNode::branch(
        path.segment.clone(),
        MetricValue::Count(count),
        MetricShare::Unknown,
        children,
    )
}

fn sorted_counter_children(parent: &PathNode<u64>) -> Vec<MetricNode> {
    let mut children: Vec<MetricNode> = parent.children.values().map(counter_node).collect();
    children.sort_by(|a, b| {
        node_count(b)
            .cmp(&node_count(a))
            .then_with(|| a.label.cmp(&b.label))
    });
    children
}

/// Saturating so a hand-built report carrying extreme counts reports a ceiling
/// rather than aborting the CLI on an arithmetic overflow.
fn sum_counts(nodes: &[MetricNode]) -> u64 {
    nodes
        .iter()
        .map(node_count)
        .fold(0u64, |acc, count| acc.saturating_add(count))
}

fn node_count(node: &MetricNode) -> u64 {
    match node.value {
        MetricValue::Count(count) => count,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use biscuit_terminal::components::renderable::TerminalRenderable;
    use biscuit_terminal::prelude::strip_escape_codes;
    use biscuit_terminal::terminal::Terminal;

    /// `(stage name, total ms, calls)`.
    type StageSpec<'a> = (&'a str, f64, u64);

    fn report(total_ms: f64, stages: &[StageSpec<'_>]) -> PerformanceReport {
        PerformanceReport {
            total_duration_ms: total_ms,
            stages: stages
                .iter()
                .map(|(name, ms, calls)| {
                    (
                        (*name).to_string(),
                        PerformanceStage {
                            calls: *calls,
                            total_duration_ms: *ms,
                            max_duration_ms: *ms,
                            last_duration_ms: *ms,
                        },
                    )
                })
                .collect(),
            counters: BTreeMap::new(),
        }
    }

    fn counter_report(counters: &[(&str, u64)]) -> PerformanceReport {
        PerformanceReport {
            total_duration_ms: 1000.0,
            stages: BTreeMap::new(),
            counters: counters
                .iter()
                .map(|(name, value)| ((*name).to_string(), *value))
                .collect(),
        }
    }

    fn labels(node: &MetricNode) -> Vec<&str> {
        node.children.iter().map(|c| c.label.as_str()).collect()
    }

    fn child<'a>(node: &'a MetricNode, label: &str) -> &'a MetricNode {
        node.children
            .iter()
            .find(|c| c.label == label)
            .unwrap_or_else(|| {
                panic!(
                    "no child {label:?} under {:?}; children are {:?}",
                    node.label,
                    labels(node)
                )
            })
    }

    /// Navigate a dotted display path from `root`, e.g. `detect.filesystem.docs`.
    fn at<'a>(root: &'a MetricNode, path: &str) -> &'a MetricNode {
        path.split('.').fold(root, child)
    }

    fn walk<'a>(node: &'a MetricNode, out: &mut Vec<&'a MetricNode>) {
        out.push(node);
        for c in &node.children {
            walk(c, out);
        }
    }

    fn flatten(root: &MetricNode) -> Vec<&MetricNode> {
        let mut out = Vec::new();
        walk(root, &mut out);
        out
    }

    fn hot_labels(root: &MetricNode) -> Vec<&str> {
        flatten(root)
            .into_iter()
            .filter(|n| n.marker == Some(MetricMarker::Highlight))
            .map(|n| n.label.as_str())
            .collect()
    }

    /// The live counter key set captured in `baseline-before.txt` (S-2), with
    /// its real values.
    fn baseline_counters() -> Vec<(&'static str, u64)> {
        vec![
            ("filesystem.docs.documents_parsed", 8572),
            ("filesystem.file_inventory.classified_binary_signature", 1),
            (
                "filesystem.file_inventory.classified_embedded_language_hint",
                14,
            ),
            ("filesystem.file_inventory.classified_exact_filename", 299),
            ("filesystem.file_inventory.classified_extension", 8080),
            ("filesystem.file_inventory.classified_fallback", 1604),
            ("filesystem.file_inventory.classified_shebang", 2),
            ("filesystem.file_inventory.entries_over_cap", 776),
            ("filesystem.file_inventory.files_accepted", 10000),
            ("filesystem.file_inventory.files_classified", 10000),
            (
                "filesystem.file_inventory.files_classified_by_content",
                1551,
            ),
            ("filesystem.io.bytes_read", 63028365),
            ("filesystem.io.canonicalizations", 882),
            ("filesystem.io.file_opens", 7612),
            ("filesystem.io.metadata_probes", 6544),
            ("filesystem.io.read_dirs", 189),
            ("filesystem.repo.lockfile_parses", 2),
            ("filesystem.repo.manifest_parses", 84),
            ("filesystem.repo.package_enrichments", 80),
            ("filesystem.repo.root_config_probes", 1),
            ("filesystem.walk.entries_visited", 13059),
            ("filesystem.walk.walks_started", 1),
            ("git.blob_loads", 27),
            ("git.commit_visits", 21865),
            ("git.file_diffs", 1),
            ("git.ref_walks", 1),
            ("git.repository_discoveries", 1),
            ("git.repository_opens", 1),
            ("git.status_walks", 2),
            ("network.wan_ip.cache_misses", 1),
            ("network.wan_ip.endpoint_attempts", 1),
            ("os.path.command_misses", 2),
            ("os.path.directories_scanned", 106),
            ("process.spawns", 1),
        ]
    }

    /// The live stage key set captured in `baseline-before.txt` (S-2), with its
    /// real totals and call counts.
    fn baseline_stages() -> Vec<StageSpec<'static>> {
        vec![
            ("detect.total", 722.70, 1),
            ("detect.filesystem", 721.29, 1),
            ("detect.hardware", 277.60, 1),
            ("detect.network", 89.07, 1),
            ("detect.os", 7.59, 1),
            ("filesystem.docs", 22.28, 1),
            ("filesystem.file_inventory.classify.binary_signature", 0.22, 1),
            (
                "filesystem.file_inventory.classify.embedded_language_hint",
                2.34,
                14,
            ),
            ("filesystem.file_inventory.classify.exact_filename", 0.55, 299),
            ("filesystem.file_inventory.classify.extension", 8.61, 8080),
            ("filesystem.file_inventory.classify.fallback", 478.56, 1604),
            ("filesystem.file_inventory.classify.framework", 2.36, 15),
            ("filesystem.file_inventory.classify.hyperpolyglot", 86.10, 1548),
            ("filesystem.file_inventory.classify.shebang", 0.12, 2),
            ("filesystem.file_inventory.walk.entry", 527.95, 10000),
            ("filesystem.formatting", 0.20, 1),
            ("filesystem.git", 676.65, 1),
            ("filesystem.inventory", 3.96, 1),
            ("filesystem.repo", 378.34, 1),
            ("filesystem.shared_walk", 241.48, 1),
            ("filesystem.shared_walk.docs", 1483.78, 4286),
            ("hardware.audio", 275.67, 1),
            ("hardware.core", 1.84, 1),
            ("hardware.gpu", 2.50, 1),
            ("hardware.storage", 27.55, 1),
            ("network.local_interfaces", 5.63, 1),
            ("network.wan_ip", 88.46, 1),
            ("os.command_exists_in_path.fink", 0.13, 1),
            ("os.command_exists_in_path.port", 1.99, 1),
            ("os.identity", 0.05, 1),
            ("os.locale", 0.69, 1),
            ("os.package_managers", 3.76, 1),
            ("os.time", 1.83, 1),
        ]
    }

    // ---- ms_to_duration (R-10) --------------------------------------------

    #[test]
    fn ms_to_duration_collapses_non_finite_and_negative_input() {
        assert_eq!(ms_to_duration(f64::NAN), Duration::ZERO);
        assert_eq!(ms_to_duration(f64::INFINITY), Duration::ZERO);
        assert_eq!(ms_to_duration(f64::NEG_INFINITY), Duration::ZERO);
        assert_eq!(ms_to_duration(-1.0), Duration::ZERO);
        assert_eq!(ms_to_duration(0.0), Duration::ZERO);
        assert_eq!(ms_to_duration(1234.56), Duration::from_secs_f64(1.23456));
    }

    #[test]
    fn a_malformed_report_projects_without_panicking() {
        let tree = timing_tree(&report(
            f64::NAN,
            &[
                ("os.identity", f64::NAN, 1),
                ("os.locale", -5.0, 0),
                ("hardware.gpu", f64::INFINITY, u64::MAX),
                ("hardware.core", 4.0, 2),
            ],
        ));

        assert_eq!(node_duration(&tree), Duration::ZERO);
        for node in flatten(&tree) {
            if let MetricValue::Duration(d) = node.value {
                assert!(d.as_secs_f64().is_finite());
            }
            assert!(!matches!(node.share, MetricShare::Of(f) if !f.is_finite()));
        }
        // A zero wall clock makes every non-root share Unknown, so the ordinary
        // `Of` guard cannot mask a NaN here.
        assert_eq!(at(&tree, "hardware.core").share, MetricShare::Unknown);
    }

    // ---- hierarchy (R2.1, R-3, R-9) ---------------------------------------

    #[test]
    fn detect_total_is_the_root_and_never_a_child() {
        let tree = timing_tree(&report(
            100.0,
            &[("detect.total", 100.0, 1), ("os.identity", 10.0, 1)],
        ));

        assert_eq!(tree.label, "Total");
        assert!(tree.emphasize);
        assert_eq!(tree.share, MetricShare::Full);
        assert_eq!(node_duration(&tree), Duration::from_millis(100));
        assert_eq!(labels(&tree), vec!["os"]);
        assert!(flatten(&tree).iter().all(|n| n.label != "total"));
    }

    #[test]
    fn a_domain_stage_reparents_below_an_existing_detect_branch() {
        let tree = timing_tree(&report(
            1000.0,
            &[
                ("detect.total", 1000.0, 1),
                ("detect.filesystem", 900.0, 1),
                ("filesystem.shared_walk", 400.0, 1),
                ("filesystem.shared_walk.docs", 300.0, 12),
            ],
        ));

        assert_eq!(labels(&tree), vec!["detect"]);
        let docs = at(&tree, "detect.filesystem.shared_walk.docs");
        assert_eq!(node_duration(docs), Duration::from_millis(300));
        // R-9: the label is the last dotted segment, not the full path.
        assert_eq!(docs.label, "docs");
    }

    #[test]
    fn a_domain_stage_stays_generic_when_its_detect_branch_is_absent() {
        let tree = timing_tree(&report(
            1000.0,
            &[
                ("filesystem.shared_walk", 400.0, 1),
                ("filesystem.shared_walk.docs", 300.0, 12),
            ],
        ));

        // No synthetic `detect` node is fabricated (R-3).
        assert_eq!(labels(&tree), vec!["filesystem"]);
        assert_eq!(
            node_duration(at(&tree, "filesystem.shared_walk.docs")),
            Duration::from_millis(300)
        );
    }

    #[test]
    fn only_the_four_detection_domains_reparent() {
        let tree = timing_tree(&report(
            1000.0,
            &[
                ("detect.total", 1000.0, 1),
                ("detect.filesystem", 900.0, 1),
                ("detect.os", 50.0, 1),
                // `repo` is not a detection domain even though `detect.*` exists.
                ("repo.scan.manifests", 20.0, 3),
            ],
        ));

        let mut top = labels(&tree);
        top.sort_unstable();
        assert_eq!(top, vec!["detect", "repo"]);
        assert_eq!(
            node_duration(at(&tree, "repo.scan.manifests")),
            Duration::from_millis(20)
        );
    }

    #[test]
    fn a_single_segment_stage_is_a_direct_child_of_the_root() {
        let tree = timing_tree(&report(100.0, &[("startup", 25.0, 1)]));

        assert_eq!(labels(&tree), vec!["startup"]);
        assert_eq!(
            node_duration(at(&tree, "startup")),
            Duration::from_millis(25)
        );
    }

    // ---- values, shares, ordering (R2.1.6, R2.2, R-2) ----------------------

    #[test]
    fn a_measured_parent_keeps_its_own_duration_and_a_synthetic_parent_sums_children() {
        let tree = timing_tree(&report(
            1000.0,
            &[
                ("detect.total", 1000.0, 1),
                ("detect.filesystem", 900.0, 1),
                // Measured parent that also has a child (R-2).
                ("filesystem.shared_walk", 200.0, 1),
                ("filesystem.shared_walk.docs", 700.0, 30),
                // `file_inventory` and `classify` are purely synthetic.
                ("filesystem.file_inventory.walk", 100.0, 1),
                ("filesystem.file_inventory.walk.entry", 500.0, 900),
                ("filesystem.file_inventory.classify.extension", 40.0, 800),
            ],
        ));

        let filesystem = at(&tree, "detect.filesystem");
        assert_eq!(node_duration(filesystem), Duration::from_millis(900));

        let shared_walk = at(&tree, "detect.filesystem.shared_walk");
        assert_eq!(node_duration(shared_walk), Duration::from_millis(200));

        // Synthetic: `walk` (100, measured) + `classify` (40, synthetic sum of
        // its own immediate children) — the 500 ms `entry` leaf is NOT rolled up.
        let inventory = at(&tree, "detect.filesystem.file_inventory");
        assert!(inventory.calls.is_none());
        assert_eq!(node_duration(inventory), Duration::from_millis(140));
        assert_eq!(
            node_duration(at(&tree, "detect.filesystem.file_inventory.classify")),
            Duration::from_millis(40)
        );
    }

    #[test]
    fn shares_are_of_wall_clock_and_are_not_clamped() {
        let tree = timing_tree(&report(
            1000.0,
            &[
                ("detect.total", 1000.0, 1),
                ("detect.filesystem", 500.0, 1),
                // A repeated leaf whose accumulated total exceeds wall clock.
                ("filesystem.shared_walk", 2500.0, 4286),
            ],
        ));

        let MetricShare::Of(domain) = at(&tree, "detect.filesystem").share else {
            panic!("expected a measured share on detect.filesystem");
        };
        assert!((domain - 0.5).abs() < 1e-9);

        let MetricShare::Of(leaf) = at(&tree, "detect.filesystem.shared_walk").share else {
            panic!("expected a measured share on shared_walk");
        };
        // Share of wall clock (2.5), not share of the 500 ms parent.
        assert!((leaf - 2.5).abs() < 1e-9, "unclamped share was {leaf}");
    }

    #[test]
    fn a_zero_duration_report_yields_unknown_shares_and_no_non_finite_values() {
        let tree = timing_tree(&report(
            0.0,
            &[
                ("detect.total", 0.0, 1),
                ("detect.os", 0.0, 1),
                ("os.identity", 0.0, 1),
                ("os.locale", 5.0, 1),
            ],
        ));

        assert_eq!(tree.share, MetricShare::Full);
        for node in flatten(&tree).into_iter().skip(1) {
            assert_eq!(
                node.share,
                MetricShare::Unknown,
                "{} should have an unknown share",
                node.label
            );
        }
        assert!(
            flatten(&tree)
                .iter()
                .all(|n| !matches!(n.share, MetricShare::Of(f) if !f.is_finite()))
        );
    }

    #[test]
    fn siblings_sort_by_duration_descending_then_label_ascending() {
        let tree = timing_tree(&report(
            1000.0,
            &[
                ("alpha", 10.0, 1),
                ("beta", 90.0, 1),
                ("gamma", 50.0, 1),
                // Equal to `gamma`: ties fall back to the display label.
                ("delta", 50.0, 1),
            ],
        ));

        assert_eq!(labels(&tree), vec!["beta", "delta", "gamma", "alpha"]);
    }

    // ---- calls (R2.1.7) ----------------------------------------------------

    #[test]
    fn calls_surface_above_one_and_stay_absent_at_one() {
        let tree = timing_tree(&report(
            1000.0,
            &[("os.identity", 10.0, 1), ("os.locale", 20.0, 4286)],
        ));

        assert_eq!(at(&tree, "os.identity").calls, None);
        assert_eq!(at(&tree, "os.locale").calls, Some(4286));
        // Synthetic intermediates never carry a call count.
        assert_eq!(at(&tree, "os").calls, None);
    }

    // ---- HOT selection (R2.3, R-4) -----------------------------------------

    #[test]
    fn the_largest_measured_non_root_stage_is_the_sole_hot_node() {
        let tree = timing_tree(&report(
            722.70,
            &[
                ("detect.total", 722.70, 1),
                ("detect.filesystem", 721.29, 1),
                ("filesystem.shared_walk", 241.48, 1),
                // Exceeds wall clock and is a depth-4 leaf, as observed live.
                ("filesystem.shared_walk.docs", 1483.78, 4286),
            ],
        ));

        assert_eq!(hot_labels(&tree), vec!["docs"]);
        assert_eq!(
            at(&tree, "detect.filesystem.shared_walk.docs").marker,
            Some(MetricMarker::Highlight)
        );
    }

    #[test]
    fn synthetic_intermediates_are_never_hot() {
        let tree = timing_tree(&report(
            1000.0,
            &[
                ("filesystem.file_inventory.walk.entry", 500.0, 900),
                ("filesystem.file_inventory.classify.extension", 40.0, 800),
            ],
        ));

        // The synthetic `file_inventory` sums to 540 ms — more than any measured
        // node — yet HOT stays on the largest measured stage.
        assert_eq!(hot_labels(&tree), vec!["entry"]);
    }

    #[test]
    fn an_equal_duration_tie_breaks_on_the_full_dotted_stage_name() {
        let tree = timing_tree(&report(
            1000.0,
            &[("zeta.aaa", 50.0, 1), ("alpha.zzz", 50.0, 1)],
        ));

        // Breaking on the display segment would pick `aaa`; the rule is the full
        // dotted name, so `alpha.zzz` wins.
        assert_eq!(hot_labels(&tree), vec!["zzz"]);
    }

    #[test]
    fn an_empty_stage_map_yields_a_bare_root_with_no_marker() {
        let tree = timing_tree(&report(12.5, &[]));

        assert_eq!(tree.label, "Total");
        assert!(tree.children.is_empty());
        assert_eq!(node_duration(&tree), Duration::from_secs_f64(0.0125));
        assert!(hot_labels(&tree).is_empty());
    }

    #[test]
    fn a_total_only_report_yields_a_bare_root_with_no_marker() {
        let tree = timing_tree(&report(12.5, &[("detect.total", 12.5, 1)]));

        assert!(tree.children.is_empty());
        assert!(hot_labels(&tree).is_empty());
    }

    // ---- the live baseline key set (S-2) -----------------------------------

    #[test]
    fn the_baseline_key_set_projects_to_the_expected_shape() {
        let tree = timing_tree(&report(722.70, &baseline_stages()));

        assert_eq!(labels(&tree), vec!["detect"]);
        let detect = at(&tree, "detect");
        assert_eq!(
            labels(detect),
            vec!["filesystem", "hardware", "network", "os"]
        );

        // Depth 5 from the root: Total → detect → filesystem → file_inventory →
        // classify → extension.
        let extension = at(&tree, "detect.filesystem.file_inventory.classify.extension");
        assert_eq!(extension.calls, Some(8080));

        // Exactly one HOT row, on the live winner.
        assert_eq!(hot_labels(&tree), vec!["docs"]);

        // Every rendered value and share is finite.
        for node in flatten(&tree) {
            assert!(!matches!(node.share, MetricShare::Of(f) if !f.is_finite()));
        }
    }

    #[test]
    fn at_most_one_hot_node_exists_for_every_projected_report() {
        for stages in [
            baseline_stages(),
            vec![("detect.total", 1.0, 1)],
            vec![("a", 1.0, 1), ("b", 1.0, 1), ("c", 1.0, 1)],
            vec![],
        ] {
            let tree = timing_tree(&report(100.0, &stages));
            assert!(
                hot_labels(&tree).len() <= 1,
                "more than one HOT row for {stages:?}"
            );
        }
    }

    // ---- the overlap note (R2.2) -------------------------------------------

    #[test]
    fn the_overlap_note_reaches_the_rendered_tree() {
        let tree = timing_metrics_tree(&report(722.70, &baseline_stages()));
        let terminal = Terminal::builder().width(200).build();

        let rendered = tree.render(&terminal);

        assert!(
            rendered.contains(OVERLAP_NOTE),
            "overlap note missing from:\n{rendered}"
        );
        assert_eq!(
            OVERLAP_NOTE,
            "Concurrent, nested, and repeated stages may overlap; their durations do not sum to wall-clock time."
        );
    }

    // ---- glyph fallback (R4, R-5, R-6) -------------------------------------

    #[test]
    fn connectors_and_the_hot_marker_follow_the_terminal_unicode_capability() {
        let tree = timing_metrics_tree(&report(722.70, &baseline_stages()));

        let ascii = strip_escape_codes(
            tree.render(&Terminal::builder().supports_unicode(false).width(80).build()),
        );
        assert!(ascii.contains("+- "), "ASCII connectors missing:\n{ascii}");
        assert!(ascii.contains("# HOT"), "ASCII marker missing:\n{ascii}");
        assert!(ascii.contains("x4286"), "ASCII call count missing:\n{ascii}");
        assert!(!ascii.contains('├'), "Unicode connector leaked:\n{ascii}");
        assert!(!ascii.contains('└'), "Unicode connector leaked:\n{ascii}");
        assert!(!ascii.contains('▇'), "Unicode marker leaked:\n{ascii}");

        let unicode = strip_escape_codes(
            tree.render(&Terminal::builder().supports_unicode(true).width(80).build()),
        );
        assert!(unicode.contains("├─ "), "tee connector missing:\n{unicode}");
        assert!(unicode.contains("└─ "), "elbow connector missing:\n{unicode}");
        assert!(unicode.contains("▇ HOT"), "marker missing:\n{unicode}");
        assert!(unicode.contains("×4286"), "call count missing:\n{unicode}");
    }

    #[test]
    fn the_production_key_set_renders_clean_on_narrow_terminals() {
        // Sniff's own counter names (`classified_embedded_language_hint` and
        // friends) are long and underscore-heavy, so a narrow terminal cuts
        // them mid-identifier. Before `MetricsTree::build_markup` escaped the
        // padded label, the stranded underscore opened a Prose italic span that
        // ate visible columns and misaligned the value column on widths 27
        // through 48. The sweep covers that band plus the narrowest supported
        // width (20) and the first clean width (49), which pin its two edges;
        // wider terminals only repeat the clean case.
        let counters = counter_metrics_tree(&counter_report(&baseline_counters()))
            .expect("counters present");
        let timings = timing_metrics_tree(&report(722.70, &baseline_stages()));

        let mut saw_underscore_cut = false;
        for unicode in [true, false] {
            for width in std::iter::once(20u32).chain(27..=49) {
                let terminal = Terminal::builder()
                    .supports_unicode(unicode)
                    .width(width)
                    .build();

                let rendered = counters.render(&terminal);
                assert!(
                    !rendered.contains("\u{1b}[3m"),
                    "counter label leaked italics (unicode={unicode}, width={width}):\n{rendered}"
                );
                let rows: Vec<String> = strip_escape_codes(rendered)
                    .lines()
                    .filter(|l| !l.trim().is_empty())
                    .map(str::to_string)
                    .collect();
                // Counter rows carry no marker, call count, or note, so the
                // padded label / count / share grid makes every row exactly as
                // wide as every other. A swallowed underscore shortens only the
                // rows that carry one.
                let widths: Vec<usize> = rows.iter().map(|l| l.chars().count()).collect();
                assert!(
                    widths.iter().all(|w| *w == widths[0]),
                    "counter rows lost columns to markup (unicode={unicode}, width={width}): {widths:?}"
                );
                let cut = if unicode { "_…" } else { "_..." };
                saw_underscore_cut |= rows.iter().any(|l| l.contains(cut));

                // The timing tree does carry decorations, so assert only that
                // its rows stay markup-clean; the trailing overlap note is
                // legitimately italic and is excluded.
                let timing = timings.render(&terminal);
                let body = timing.split("\n\n").next().unwrap_or_default();
                assert!(
                    !body.contains("\u{1b}[3m"),
                    "timing label leaked italics (unicode={unicode}, width={width}):\n{timing}"
                );
            }
        }
        assert!(
            saw_underscore_cut,
            "the sweep never truncated a label immediately after an underscore, \
             so it does not exercise the regression"
        );
    }

    // ---- counters (R3, R-1) ------------------------------------------------

    #[test]
    fn an_empty_counter_map_produces_no_tree() {
        let stages_only = report(100.0, &[("os.identity", 10.0, 1)]);

        assert!(counter_tree(&stages_only).is_none());
        assert!(counter_metrics_tree(&stages_only).is_none());
    }

    #[test]
    fn nested_counter_names_nest_generically() {
        let tree = counter_tree(&counter_report(&[
            ("filesystem.io.bytes_read", 63028365),
            ("network.wan_ip.cache_hits", 4),
            ("process.spawns", 1),
        ]))
        .expect("counters present");

        assert_eq!(tree.label, "Counters");
        assert!(tree.emphasize);
        assert_eq!(labels(&tree), vec!["filesystem", "network", "process"]);
        assert_eq!(
            node_count(at(&tree, "filesystem.io.bytes_read")),
            63028365
        );
        assert_eq!(node_count(at(&tree, "network.wan_ip.cache_hits")), 4);
        // A single-segment counter is a direct child of the root.
        assert_eq!(node_count(at(&tree, "process.spawns")), 1);
    }

    #[test]
    fn counter_names_never_reparent_and_are_never_suppressed() {
        let mut both = report(
            1000.0,
            &[("detect.total", 1000.0, 1), ("detect.filesystem", 900.0, 1)],
        );
        both.counters = [
            ("filesystem.io.bytes_read".to_string(), 10u64),
            ("detect.total".to_string(), 3u64),
        ]
        .into_iter()
        .collect();

        let counters = counter_tree(&both).expect("counters present");

        // `detect.filesystem` is measured, so the timing tree re-parents
        // `filesystem.*` below it — and suppresses `detect.total` outright.
        // Neither rule is a counter rule.
        let mut top = labels(&counters);
        top.sort_unstable();
        assert_eq!(top, vec!["detect", "filesystem"]);
        assert_eq!(node_count(at(&counters, "detect.total")), 3);
        assert_eq!(node_count(at(&counters, "filesystem.io.bytes_read")), 10);

        // The timing tree is unaffected by the counters sharing its namespace.
        let timing = timing_tree(&both);
        assert_eq!(labels(&timing), vec!["detect"]);
        assert!(flatten(&timing).iter().all(|n| n.label != "bytes_read"));
    }

    #[test]
    fn counter_siblings_sort_by_count_descending_then_label_ascending() {
        let tree = counter_tree(&counter_report(&[
            ("io.alpha", 10),
            ("io.beta", 90),
            ("io.gamma", 50),
            // Equal to `gamma`: ties fall back to the display label.
            ("io.delta", 50),
        ]))
        .expect("counters present");

        assert_eq!(
            labels(at(&tree, "io")),
            vec!["beta", "delta", "gamma", "alpha"]
        );
    }

    #[test]
    fn a_measured_counter_parent_keeps_its_count_and_a_synthetic_parent_sums_children() {
        let tree = counter_tree(&counter_report(&[
            // Measured and also a parent.
            ("filesystem.io", 7),
            ("filesystem.io.bytes_read", 100),
            ("filesystem.repo.manifest_parses", 4),
            ("filesystem.repo.lockfile_parses", 2),
        ]))
        .expect("counters present");

        assert_eq!(node_count(at(&tree, "filesystem.io")), 7);
        assert_eq!(node_count(at(&tree, "filesystem.repo")), 6);
        // Synthetic `filesystem` sums its immediate children (7 + 6), not the
        // whole subtree — the 100-count leaf is not rolled up.
        assert_eq!(node_count(at(&tree, "filesystem")), 13);
        assert_eq!(node_count(&tree), 13);
    }

    #[test]
    fn extreme_counter_values_saturate_rather_than_overflowing() {
        let tree = counter_tree(&counter_report(&[("a.one", u64::MAX), ("b.two", u64::MAX)]))
            .expect("counters present");

        assert_eq!(node_count(&tree), u64::MAX);
    }

    #[test]
    fn counter_nodes_carry_counts_and_never_a_marker_or_call_count() {
        let tree =
            counter_tree(&counter_report(&baseline_counters())).expect("counters present");

        for node in flatten(&tree) {
            assert!(
                matches!(node.value, MetricValue::Count(_)),
                "{} is not a count",
                node.label
            );
            assert_eq!(node.marker, None, "{} carries a marker", node.label);
            assert_eq!(node.calls, None, "{} carries a call count", node.label);
        }
    }

    #[test]
    fn every_counter_share_is_unknown_and_only_the_component_renders_the_root_full() {
        let tree = counter_tree(&counter_report(&[
            ("filesystem.io.bytes_read", 10),
            ("git.blob_loads", 4),
        ]))
        .expect("counters present");

        for node in flatten(&tree) {
            assert_eq!(
                node.share,
                MetricShare::Unknown,
                "{} should have an unknown share",
                node.label
            );
        }

        // R-1: the projection sets `Unknown` on the root as R3 specifies, but
        // `collect_rows` overrides the root's share unconditionally
        // (`metrics_tree.rs:459-463`), so the rendered root row reads `100%`.
        // That is the accepted component behavior, not a defect to file.
        let rendered = strip_escape_codes(
            MetricsTree::new(tree).render(&Terminal::builder().width(200).build()),
        );
        let mut lines = rendered.lines();
        let root = lines.next().expect("a root row");
        assert!(
            root.contains("Counters") && root.contains("100%"),
            "unexpected root row: {root}"
        );
        assert!(
            lines.all(|line| !line.contains('%')),
            "a non-root counter row rendered a share:\n{rendered}"
        );
    }

    #[test]
    fn the_baseline_counter_key_set_projects_to_the_expected_shape() {
        let counters = baseline_counters();
        let tree = counter_tree(&counter_report(&counters)).expect("counters present");

        let total: u64 = counters.iter().map(|(_, value)| *value).sum();
        assert_eq!(node_count(&tree), total);
        assert_eq!(
            labels(&tree),
            vec!["filesystem", "git", "os", "network", "process"]
        );

        // Depth 4 from the root, and the stage/counter near-collision the two
        // separate trees exist to keep apart.
        assert_eq!(
            node_count(at(
                &tree,
                "filesystem.file_inventory.classified_extension"
            )),
            8080
        );

        let rendered = strip_escape_codes(
            MetricsTree::new(tree).render(&Terminal::builder().width(200).build()),
        );
        // Counts render with no unit suffix (`metrics_tree.rs:60`).
        assert!(
            rendered.contains("63028365"),
            "raw count missing from:\n{rendered}"
        );
    }
}
