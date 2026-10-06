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
    Timings::new(Scope::Command, total, root).unwrap()
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
    let tree = report_tree(&Timings::new(Scope::Command, ms(20), root).unwrap());

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
    let worker = WorkerTimings::new(ms(35), list(ChildrenKind::Sequential, [Span::new(Stage::WorkerSetup, ms(5)), halves])).unwrap();
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
    // The JSON's span tree, labeled, is the human tree less its remainder
    // rows, at every depth.
    #[derive(Debug, PartialEq)]
    struct Labeled(String, Vec<Labeled>);
    fn from_spans(spans: &[Span]) -> Vec<Labeled> {
        spans.iter().map(|span| Labeled(span_label(span), from_spans(span.children()))).collect()
    }
    fn from_rows(node: &MetricNode) -> Vec<Labeled> {
        node.children
            .iter()
            .filter(|row| row.label != UNATTRIBUTED && row.label != OVER_ATTRIBUTED)
            .map(|row| Labeled(row.label.clone(), from_rows(row)))
            .collect()
    }
    assert_eq!(from_rows(&report_tree(&timings)), from_spans(decoded.spans()));
    assert_eq!(decoded, timings);
}

/// One rendered report row: its depth below the section heading, label,
/// value, and share column, with ANSI codes, the quote border, and the tree
/// connectors removed.
#[derive(Debug, PartialEq)]
struct RenderedRow {
    depth: usize,
    label: String,
    value: String,
    share: String,
}

fn row(depth: usize, label: &str, value: &str) -> RenderedRow {
    RenderedRow { depth, label: label.to_string(), value: value.to_string(), share: "—".to_string() }
}

/// The rendered report's rows from the heading starting with `heading` up to
/// the next depth-0 row.
fn rendered_section(report: &str, heading: &str) -> Vec<RenderedRow> {
    let plain = biscuit_test_harness::strip_ansi(report);
    let rows = plain.lines().map(|line| {
        let line = line.strip_prefix("▌ ").unwrap_or(line);
        let body = line.trim_start_matches(['│', '├', '└', '─', ' ']);
        let depth = (line.chars().count() - body.chars().count()) / 3;
        let mut tokens: Vec<&str> = body.split_whitespace().collect();
        let share = tokens.pop().unwrap_or_default().to_string();
        let value = tokens.pop().unwrap_or_default().to_string();
        RenderedRow { depth, label: tokens.join(" "), value, share }
    });
    let mut section: Vec<RenderedRow> = rows.skip_while(|row| !row.label.starts_with(heading)).collect();
    let end = section.iter().skip(1).position(|row| row.depth == 0).map_or(section.len(), |at| at + 1);
    section.truncate(end);
    section
}

/// A worker report whose two halves each have one sequential child.
fn worker_with_halves(setup: Duration, pr: (Duration, Duration), head: (Duration, Duration)) -> WorkerTimings {
    let half = |stage, (elapsed, child): (Duration, Duration), child_stage| {
        Span::new(stage, elapsed).with_children(list(ChildrenKind::Sequential, [Span::new(child_stage, child)]))
    };
    let halves_elapsed = pr.0.max(head.0);
    let halves = Span::new(Stage::WorkerHalves, halves_elapsed).with_children(list(
        ChildrenKind::Concurrent,
        [half(Stage::PrRefresh, pr, Stage::PrRequest), half(Stage::HeadRefresh, head, Stage::HeadCheck)],
    ));
    WorkerTimings::new(setup + halves_elapsed, list(ChildrenKind::Sequential, [Span::new(Stage::WorkerSetup, setup), halves])).unwrap()
}

fn complete(launch_index: u32, worker: WorkerTimings) -> WorkerReport {
    WorkerReport { launch_index, attempt_id: format!("{launch_index:032x}"), report: LaunchReport::Complete(worker) }
}

#[test]
fn the_rendered_worker_section_reconciles_sequential_parents_without_any_percentage() {
    let reports = vec![
        // Each half's child covers half of it: a 20 ms gap under each.
        complete(0, worker_with_halves(ms(5), (ms(40), ms(20)), (ms(40), ms(20)))),
        // Each half's child outlasts it by 10 ms.
        complete(1, worker_with_halves(ms(5), (ms(10), ms(20)), (ms(10), ms(20)))),
        // A 0.5 ms gap under the PR half is hidden; the head half is exact.
        complete(
            2,
            worker_with_halves(ms(5), (Duration::from_micros(20_500), ms(20)), (Duration::from_micros(20_500), Duration::from_micros(20_500))),
        ),
    ];
    let timings = remote_listing(ms(100)).with_worker_reports(reports, WorkerReportStatus::Complete);

    let report = human_report_at(&timings, 120);

    let section = rendered_section(&report, WORKER_HEADING);
    let (pr, pr_request, head, head_check) =
        (Stage::PrRefresh.label(), Stage::PrRequest.label(), Stage::HeadRefresh.label(), Stage::HeadCheck.label());
    let (setup, halves) = (Stage::WorkerSetup.label(), Stage::WorkerHalves.label());
    assert_eq!(
        section,
        [
            row(0, &format!("{WORKER_HEADING}: complete"), "—"),
            row(1, "launch 0 (complete)", "45.0ms"),
            row(2, setup, "5.0ms"),
            row(2, halves, "40.0ms"),
            row(3, pr, "40.0ms"),
            row(4, pr_request, "20.0ms"),
            row(4, UNATTRIBUTED, "20.0ms"),
            row(3, head, "40.0ms"),
            row(4, head_check, "20.0ms"),
            row(4, UNATTRIBUTED, "20.0ms"),
            row(1, "launch 1 (complete)", "15.0ms"),
            row(2, setup, "5.0ms"),
            row(2, halves, "10.0ms"),
            row(3, pr, "10.0ms"),
            row(4, pr_request, "20.0ms"),
            row(4, OVER_ATTRIBUTED, "10.0ms"),
            row(3, head, "10.0ms"),
            row(4, head_check, "20.0ms"),
            row(4, OVER_ATTRIBUTED, "10.0ms"),
            row(1, "launch 2 (complete)", "25.5ms"),
            row(2, setup, "5.0ms"),
            row(2, halves, "20.5ms"),
            row(3, pr, "20.5ms"),
            row(4, pr_request, "20.0ms"),
            row(3, head, "20.5ms"),
            row(4, head_check, "20.5ms"),
        ],
        "{report}"
    );
}

#[test]
fn the_rendered_foreground_keeps_its_shares_and_remainders_beside_a_worker_section() {
    let reports = vec![complete(0, worker_with_halves(ms(5), (ms(40), ms(20)), (ms(40), ms(20))))];
    let timings = remote_listing(ms(100)).with_worker_reports(reports, WorkerReportStatus::Complete);

    let report = human_report_at(&timings, 120);

    let foreground = rendered_section(&report, "Performance");
    let share = |depth: usize, label: &str| {
        let found = foreground.iter().find(|row| row.depth == depth && row.label == label);
        found.unwrap_or_else(|| panic!("no {label} at depth {depth} in {report}")).share.clone()
    };
    assert_eq!(share(0, "Performance"), "100%");
    assert_eq!(share(1, "startup"), "10%");
    assert_eq!(share(1, UNATTRIBUTED), "20%", "the root's remainder");
    assert_eq!(share(1, "refresh worker ‖ local reads"), "60%");
    assert_eq!(share(2, "graph history [9 git]"), "—", "a concurrent child");
    assert_eq!(share(3, UNATTRIBUTED), "13%", "a nested sequential remainder");
    assert!(foreground.iter().all(|row| !row.label.starts_with(WORKER_HEADING)), "{report}");
}

#[test]
fn the_rendered_foreground_shows_any_over_attribution_and_hides_a_sub_millisecond_gap() {
    let inner = Span::new(Stage::Regather, ms(10)).with_children(list(
        ChildrenKind::Sequential,
        [Span::new(Stage::PrepareLocal, ms(6)), Span::new(Stage::LocalReads, Duration::from_micros(4_100))],
    ));
    let root = list(ChildrenKind::Sequential, [Span::new(Stage::Startup, Duration::from_micros(10_100)), inner]);
    let over = human_report_at(&Timings::new(Scope::Command, ms(20), root).unwrap(), 120);
    let hidden = human_report_at(&remote_listing(ms(80) + Duration::from_micros(999)), 120);

    let rows: Vec<(usize, String, String)> = rendered_section(&over, "Performance")
        .into_iter()
        .filter(|row| row.label == OVER_ATTRIBUTED)
        .map(|row| (row.depth, row.value, row.share))
        .collect();
    assert_eq!(
        rows,
        [(2, "100.0µs".to_string(), "1%".to_string()), (1, "100.0µs".to_string(), "<1%".to_string())],
        "{over}"
    );
    let top: Vec<RenderedRow> = rendered_section(&hidden, "Performance").into_iter().filter(|row| row.depth == 1).collect();
    assert!(top.iter().all(|row| row.label != UNATTRIBUTED), "{hidden}");
    assert_eq!(top.last().map(|row| row.label.as_str()), Some("table render"), "{hidden}");
}

#[test]
fn a_rendered_worker_heading_shows_no_percentage_whatever_reached_the_wait() {
    let partial = vec![
        complete(0, worker_with_halves(ms(5), (ms(20), ms(20)), (ms(20), ms(20)))),
        WorkerReport { launch_index: 1, attempt_id: format!("{:032x}", 1), report: LaunchReport::Missing },
    ];
    let cases = [
        (vec![], WorkerReportStatus::Missing, vec![row(0, &format!("{WORKER_HEADING}: missing"), "—")]),
        (
            partial,
            WorkerReportStatus::Partial,
            vec![
                row(0, &format!("{WORKER_HEADING}: partial"), "—"),
                row(1, "launch 0 (complete)", "25.0ms"),
                row(2, Stage::WorkerSetup.label(), "5.0ms"),
                row(2, Stage::WorkerHalves.label(), "20.0ms"),
                row(3, Stage::PrRefresh.label(), "20.0ms"),
                row(4, Stage::PrRequest.label(), "20.0ms"),
                row(3, Stage::HeadRefresh.label(), "20.0ms"),
                row(4, Stage::HeadCheck.label(), "20.0ms"),
                row(1, "launch 1 (missing)", "—"),
            ],
        ),
    ];
    for (reports, status, expected) in cases {
        let report = human_report_at(&remote_listing(ms(100)).with_worker_reports(reports, status), 80);

        assert_eq!(rendered_section(&report, WORKER_HEADING), expected, "{report}");
        let heading = report.lines().find(|line| line.contains(WORKER_HEADING)).expect("a heading");
        assert!(!heading.contains('%'), "{heading:?}");
    }
}
