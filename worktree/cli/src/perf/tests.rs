use std::time::Duration;

use biscuit_terminal::components::metrics_tree::{MetricNode, MetricShare, MetricValue};
use worktree::timing::{
    ChildrenKind, LaunchReport, Scope, Span, SpanList, Stage, Timings, WorkerReport, WorkerReportStatus, WorkerTimings,
};

use super::*;

fn ms(value: u64) -> Duration {
    Duration::from_millis(value)
}

fn list(kind: ChildrenKind, spans: impl IntoIterator<Item = Span>) -> SpanList {
    let mut list = match kind {
        ChildrenKind::Sequential => SpanList::sequential(),
        ChildrenKind::Concurrent => SpanList::concurrent(),
    };
    for span in spans {
        list.push(span);
    }
    list
}

/// A remote listing: a concurrent region whose graph history has sequential
/// steps with `git` counts.
fn remote_listing(total: Duration) -> Timings {
    let history = Span::new(Stage::GraphHistory, ms(40)).with_git_calls(9).with_children(list(
        ChildrenKind::Sequential,
        [
            Span::new(Stage::ShallowCheck, ms(5)).with_git_calls(1),
            Span::new(Stage::DefaultTips, ms(5)).with_git_calls(1),
            Span::new(Stage::LaneAssembly, ms(25)).with_git_calls(7),
        ],
    ));
    let region = Span::new(Stage::RemoteAndLocal, ms(60)).with_children(list(
        ChildrenKind::Concurrent,
        [Span::new(Stage::RefreshWorker, ms(60)), Span::new(Stage::LocalGather, ms(30)), history],
    ));
    let root = list(ChildrenKind::Sequential, [Span::new(Stage::Startup, ms(10)), region, Span::new(Stage::TableRender, ms(10))]);
    Timings::new(Scope::Command, total, root)
}

/// A share as whole tenths of a percent, so float rounding cannot matter;
/// `None` for no share.
fn permille(share: MetricShare) -> Option<u32> {
    match share {
        MetricShare::Of(fraction) => Some((fraction * 1000.0).round() as u32),
        MetricShare::Full => Some(1000),
        MetricShare::Unknown => None,
    }
}

/// `(label, share in permille)` of each child of `node`.
fn rows(node: &MetricNode) -> Vec<(String, Option<u32>)> {
    node.children.iter().map(|child| (child.label.clone(), permille(child.share))).collect()
}

fn child<'a>(node: &'a MetricNode, label: &str) -> &'a MetricNode {
    node.children.iter().find(|child| child.label == label).unwrap_or_else(|| panic!("no {label}: {node:#?}"))
}

#[test]
fn sequential_rows_carry_a_share_and_concurrent_rows_none() {
    let tree = report_tree(&remote_listing(ms(100)));

    assert_eq!(tree.share, MetricShare::Full);
    assert_eq!(
        rows(&tree),
        [
            ("startup".to_string(), Some(100)),
            ("refresh worker ‖ local reads".to_string(), Some(600)),
            ("table render".to_string(), Some(100)),
            (UNATTRIBUTED.to_string(), Some(200)),
        ]
    );
    let region = child(&tree, "refresh worker ‖ local reads");
    assert!(region.children.iter().all(|row| row.share == MetricShare::Unknown), "{region:#?}");
    assert!(region.children.iter().all(|row| row.label != UNATTRIBUTED), "a concurrent parent has no remainder row");
}

#[test]
fn a_sequential_parent_shows_its_steps_with_git_counts_and_its_own_remainder() {
    let tree = report_tree(&remote_listing(ms(100)));

    let history = child(child(&tree, "refresh worker ‖ local reads"), "graph history  [9 git]");
    assert_eq!(history.share, MetricShare::Unknown, "it overlapped its siblings");
    assert_eq!(
        rows(history),
        [
            ("shallow check  [1 git]".to_string(), Some(125)),
            ("default-branch tips  [1 git]".to_string(), Some(125)),
            ("assemble lanes and fork holders  [7 git]".to_string(), Some(625)),
            (UNATTRIBUTED.to_string(), Some(125)),
        ]
    );
    let launch = child(child(&tree, "refresh worker ‖ local reads"), "launch and wait for refresh");
    assert!(launch.children.is_empty(), "an unknown count adds no suffix: {launch:#?}");
}

#[test]
fn an_unattributed_remainder_under_a_millisecond_is_hidden() {
    let shown = report_tree(&remote_listing(ms(81)));
    let hidden = report_tree(&remote_listing(ms(80) + Duration::from_micros(999)));

    assert_eq!(shown.children.last().map(|row| row.label.as_str()), Some(UNATTRIBUTED));
    assert!(matches!(shown.children.last().map(|row| row.value), Some(MetricValue::Duration(d)) if d == ms(1)));
    assert!(hidden.children.iter().all(|row| row.label != UNATTRIBUTED), "{hidden:#?}");
}

#[test]
fn over_attribution_is_shown_at_every_sequential_level_however_small() {
    let inner = Span::new(Stage::Regather, ms(10)).with_children(list(
        ChildrenKind::Sequential,
        [Span::new(Stage::PrepareLocal, ms(6)), Span::new(Stage::LocalReads, Duration::from_micros(4_100))],
    ));
    let root = list(ChildrenKind::Sequential, [Span::new(Stage::Startup, Duration::from_micros(10_100)), inner]);
    let tree = report_tree(&Timings::new(Scope::Command, ms(20), root));

    let top = tree.children.last().expect("a remainder row");
    assert_eq!(top.label, OVER_ATTRIBUTED);
    assert!(matches!(top.value, MetricValue::Duration(d) if d == Duration::from_micros(100)));
    let regather = child(&tree, "regather");
    let nested = regather.children.last().expect("a remainder row");
    assert_eq!(nested.label, OVER_ATTRIBUTED);
    assert!(matches!(nested.value, MetricValue::Duration(d) if d == Duration::from_micros(100)));
}

#[test]
fn worker_reports_are_a_separate_section_without_shares() {
    let halves = Span::new(Stage::WorkerHalves, ms(30)).with_children(list(
        ChildrenKind::Concurrent,
        [Span::new(Stage::PrRefresh, ms(30)), Span::new(Stage::HeadRefresh, ms(20))],
    ));
    let worker = WorkerTimings::new(ms(35), list(ChildrenKind::Sequential, [Span::new(Stage::WorkerSetup, ms(5)), halves]));
    let reports = vec![
        WorkerReport { launch_index: 0, attempt_id: "0123456789abcdef0123456789abcdef".into(), report: LaunchReport::Missing },
        WorkerReport {
            launch_index: 1,
            attempt_id: "fedcba9876543210fedcba9876543210".into(),
            report: LaunchReport::Complete(worker),
        },
    ];
    let timings = remote_listing(ms(100)).with_worker_reports(reports, WorkerReportStatus::Partial);

    assert!(worker_tree(&remote_listing(ms(100))).is_none(), "no section without a followed worker");
    let section = worker_tree(&timings).expect("a worker section");
    assert!(section.label.ends_with(": partial"), "{}", section.label);
    assert_eq!(
        section.children.iter().map(|row| row.label.as_str()).collect::<Vec<_>>(),
        ["launch 0 (missing)", "launch 1 (complete)"]
    );
    fn no_shares(node: &MetricNode) -> bool {
        node.share == MetricShare::Unknown && node.children.iter().all(no_shares)
    }
    assert!(no_shares(&section), "{section:#?}");
    assert!(
        report_tree(&timings).children.iter().all(|row| !row.label.starts_with("launch")),
        "worker rows never join the foreground tree"
    );
}

#[test]
fn the_json_record_is_one_framed_line_that_decodes_to_the_same_document() {
    let timings = remote_listing(ms(100));

    let record = json_record(&timings);

    assert!(record.starts_with('\n') && record.ends_with('\n'), "{record:?}");
    let line = record.trim_matches('\n');
    assert!(!line.contains('\n') && !line.contains('\r') && !line.contains('\u{1b}'), "{record:?}");
    let document = line.strip_prefix(JSON_PREFIX).expect("framed");
    assert_eq!(Timings::from_json(document).expect("decodes"), timings);
}

#[test]
fn the_human_report_names_the_stages_and_both_renderers_describe_one_tree() {
    let timings = remote_listing(ms(100));

    let human = human_report(&timings);
    let decoded = Timings::from_json(json_record(&timings).trim().strip_prefix(JSON_PREFIX).expect("framed"))
        .expect("decodes");

    for label in ["Performance", "startup", "refresh worker ‖ local reads", "graph history  [9 git]", "table render"] {
        assert!(human.contains(label), "{label} missing from {human}");
    }
    let stages = |timings: &Timings| timings.spans().iter().map(Span::stage).collect::<Vec<_>>();
    assert_eq!(stages(&decoded), stages(&timings));
}
