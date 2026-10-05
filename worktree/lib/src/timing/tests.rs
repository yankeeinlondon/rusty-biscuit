use std::collections::HashSet;
use std::time::Duration;

use serde_json::{Value, json};

use super::*;

fn us(value: u64) -> Duration {
    Duration::from_micros(value)
}

fn leaf(stage: Stage, elapsed: u64) -> Span {
    Span::new(stage, us(elapsed))
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

const ATTEMPT_0: &str = "0123456789abcdef0123456789abcdef";
const ATTEMPT_1: &str = "fedcba9876543210fedcba9876543210";

fn worker_timings() -> WorkerTimings {
    let pr_refresh = leaf(Stage::PrRefresh, 300).with_children(list(ChildrenKind::Sequential, [leaf(Stage::PrRequest, 250)]));
    let head_refresh = leaf(Stage::HeadRefresh, 100).with_children(list(ChildrenKind::Sequential, [leaf(Stage::HeadCheck, 80)]));
    let halves = leaf(Stage::WorkerHalves, 350).with_children(list(ChildrenKind::Concurrent, [pr_refresh, head_refresh]));
    WorkerTimings::new(us(380), list(ChildrenKind::Sequential, [leaf(Stage::WorkerSetup, 20), halves]))
}

/// A command document shaped like a remote listing with a forced retry: the
/// first launch published no timings, the second did.
fn fixture() -> Timings {
    let refresh = leaf(Stage::RefreshWorker, 400)
        .with_children(list(ChildrenKind::Sequential, [leaf(Stage::WorkerLaunch, 5), leaf(Stage::WorkerWait, 390)]));
    let local = leaf(Stage::LocalGather, 300).with_children(list(
        ChildrenKind::Concurrent,
        [leaf(Stage::WorktreeStatus, 200), leaf(Stage::BranchComparisons, 250)],
    ));
    let graph = leaf(Stage::GraphHistory, 450).with_git_calls(12).with_children(list(
        ChildrenKind::Sequential,
        [
            leaf(Stage::ShallowCheck, 10).with_git_calls(1),
            leaf(Stage::DefaultTips, 100).with_git_calls(3),
            leaf(Stage::LaneAssembly, 300).with_git_calls(8),
        ],
    ));
    let remote = leaf(Stage::RemoteAndLocal, 500).with_children(list(ChildrenKind::Concurrent, [refresh, local, graph]));
    let regather = leaf(Stage::Regather, 100).with_children(list(ChildrenKind::Concurrent, [leaf(Stage::BranchComparisons, 80)]));
    let reports = vec![
        WorkerReport { launch_index: 0, attempt_id: ATTEMPT_0.into(), report: LaunchReport::Missing },
        WorkerReport { launch_index: 1, attempt_id: ATTEMPT_1.into(), report: LaunchReport::Complete(worker_timings()) },
    ];
    Timings::new(
        Scope::Command,
        us(1000),
        list(ChildrenKind::Sequential, [leaf(Stage::Startup, 10), remote, regather, leaf(Stage::WriteOutput, 50)]),
    )
    .with_worker_reports(reports, WorkerReportStatus::Partial)
}

fn document() -> Value {
    serde_json::from_str(&fixture().to_json()).expect("the fixture serializes to JSON")
}

#[test]
fn every_stage_id_round_trips_and_ids_and_labels_are_unique() {
    let mut ids = HashSet::new();
    let mut labels = HashSet::new();
    for &stage in Stage::ALL {
        assert!(ids.insert(stage.id()), "{} repeats", stage.id());
        assert!(labels.insert(stage.label()), "{} repeats", stage.label());
        assert_eq!(Stage::from_id(stage.id()), Some(stage));
        assert_eq!(stage.id().parse::<Stage>(), Ok(stage));
        assert_eq!(serde_json::to_value(stage).unwrap(), json!(stage.id()));
        assert_eq!(serde_json::from_value::<Stage>(json!(stage.id())).unwrap(), stage);
        assert!(stage.id().bytes().all(|b| b.is_ascii_lowercase() || b == b'_'), "{} is not snake_case", stage.id());
    }
    assert_eq!(Stage::ALL.len(), 41, "the closed stage list");
    assert_eq!("graph gather".parse::<Stage>(), Err(UnknownStage("graph gather".into())), "a label is not an id");
    assert!(serde_json::from_value::<Stage>(json!("")).is_err());
}

#[test]
fn sequential_spans_and_unattributed_reconcile_exactly_to_the_total() {
    let timings = Timings::new(Scope::Library, us(100), list(ChildrenKind::Sequential, [leaf(Stage::ReadWorktrees, 30), leaf(Stage::Commit, 45)]));

    assert_eq!((timings.unattributed(), timings.over_attributed()), (us(25), us(0)));
    let doc: Value = serde_json::from_str(&timings.to_json()).unwrap();
    let spans: u64 = doc["spans"].as_array().unwrap().iter().map(|span| span["elapsed_us"].as_u64().unwrap()).sum();
    assert_eq!(spans + doc["unattributed_us"].as_u64().unwrap() - doc["over_attributed_us"].as_u64().unwrap(), doc["total_us"].as_u64().unwrap());
}

#[test]
fn over_attribution_is_surfaced_at_every_sequential_level() {
    let nested = leaf(Stage::GraphHistory, 50)
        .with_children(list(ChildrenKind::Sequential, [leaf(Stage::DefaultTips, 40), leaf(Stage::LaneAssembly, 30)]));
    let timings = Timings::new(Scope::Library, us(60), list(ChildrenKind::Sequential, [nested, leaf(Stage::Commit, 20)]));

    assert_eq!((timings.unattributed(), timings.over_attributed()), (us(0), us(10)), "70 of 60 at the root");
    let graph = timings.span(&[Stage::GraphHistory]).unwrap();
    assert_eq!((graph.unattributed(), graph.over_attributed()), (us(0), us(20)), "70 of 50 inside the graph");

    let decoded = Timings::from_json(&timings.to_json()).expect("over-attribution is a valid document");
    assert_eq!(decoded, timings);
    let doc: Value = serde_json::from_str(&timings.to_json()).unwrap();
    assert_eq!(doc["over_attributed_us"], json!(10));
    assert_eq!(doc["spans"][0]["over_attributed_us"], json!(20));
    assert_eq!(doc["spans"][0]["unattributed_us"], json!(0));
}

#[test]
fn concurrent_children_never_enter_a_sum() {
    let group = leaf(Stage::LocalGather, 40)
        .with_children(list(ChildrenKind::Concurrent, [leaf(Stage::WorktreeStatus, 40), leaf(Stage::BranchComparisons, 35)]));
    let timings = Timings::new(Scope::Library, us(100), list(ChildrenKind::Sequential, [group]));

    let group = timings.span(&[Stage::LocalGather]).unwrap();
    assert_eq!((group.unattributed(), group.over_attributed()), (us(0), us(0)), "75 of 40 is overlap, not over-attribution");
    assert_eq!(timings.unattributed(), us(60), "the root counts the group's own 40 only");
}

#[test]
fn a_sequential_parent_reconciles_its_own_remainder_and_a_leaf_has_none() {
    let graph = leaf(Stage::GraphHistory, 450)
        .with_children(list(ChildrenKind::Sequential, [leaf(Stage::ShallowCheck, 10), leaf(Stage::LaneAssembly, 300)]));

    assert_eq!((graph.unattributed(), graph.over_attributed()), (us(140), us(0)));
    let shallow = graph.find(&[Stage::ShallowCheck]).unwrap();
    assert_eq!((shallow.unattributed(), shallow.over_attributed()), (us(0), us(0)), "a leaf is not a parent");
}

#[test]
fn a_stage_under_two_parents_is_found_by_its_path() {
    let timings = fixture();

    let first = timings.span(&[Stage::RemoteAndLocal, Stage::LocalGather, Stage::BranchComparisons]).unwrap();
    let again = timings.span(&[Stage::Regather, Stage::BranchComparisons]).unwrap();
    assert_eq!((first.elapsed(), again.elapsed()), (us(250), us(80)));
    assert!(timings.span(&[Stage::BranchComparisons]).is_none(), "not at the top level");
    assert!(timings.span(&[]).is_none());
    assert!(timings.span(&[Stage::FastForward]).is_none(), "a step that did not run");
}

#[test]
fn a_repeated_stage_accumulates_into_one_sibling() {
    let mut siblings = SpanList::sequential();
    siblings.push(leaf(Stage::DefaultTips, 10).with_git_calls(2));
    siblings.push(leaf(Stage::LaneAssembly, 5).with_git_calls(1));
    siblings.push(leaf(Stage::DefaultTips, 15).with_git_calls(3));
    siblings.push(leaf(Stage::LaneAssembly, 5));

    let spans = siblings.spans();
    assert_eq!(spans.iter().map(Span::stage).collect::<Vec<_>>(), [Stage::DefaultTips, Stage::LaneAssembly]);
    assert_eq!((spans[0].elapsed(), spans[0].git_calls()), (us(25), Some(5)));
    assert_eq!(spans[1].git_calls(), None, "one uncounted invocation makes the step's count unknown");
}

#[test]
fn timing_a_parent_records_its_children_inside_it() {
    let mut root = SpanList::sequential();
    let answer = root.time_parent(Stage::LocalGather, ChildrenKind::Concurrent, |children| {
        children.push(leaf(Stage::WorktreeStatus, 1));
        children.time(Stage::BranchComparisons, || 42)
    });

    assert_eq!(answer, 42);
    let group = &root.spans()[0];
    assert_eq!((group.stage(), group.children_kind()), (Stage::LocalGather, ChildrenKind::Concurrent));
    assert_eq!(group.children().iter().map(Span::stage).collect::<Vec<_>>(), [Stage::WorktreeStatus, Stage::BranchComparisons]);
}

#[test]
fn a_document_round_trips_through_json() {
    let timings = fixture();

    let text = timings.to_json();
    assert!(!text.contains('\n'), "compact: one line");
    let decoded = Timings::from_json(&text).expect("a written document reads back");
    assert_eq!(decoded, timings);
    assert_eq!(decoded.to_json(), text);
    assert_eq!(decoded.worker_report_status(), Some(WorkerReportStatus::Partial));
    assert_eq!(decoded.worker_reports()[1].timings(), Some(&worker_timings()));
    assert_eq!(decoded.span(&[Stage::RemoteAndLocal, Stage::GraphHistory]).unwrap().git_calls(), Some(12));

    let worker = worker_timings();
    assert_eq!(WorkerTimings::from_json(&worker.to_json()), Ok(worker));
}

#[test]
fn durations_become_whole_microseconds_before_remainders_are_computed() {
    let nanos = Duration::from_nanos;
    // Each child truncates to 1 µs, so 1.5 + 1.5 of 3 µs leaves 1 µs, not 0.
    let timings = Timings::new(
        Scope::Library,
        nanos(3_000),
        list(ChildrenKind::Sequential, [Span::new(Stage::ReadWorktrees, nanos(1_500)), Span::new(Stage::Commit, nanos(1_500))]),
    );

    assert_eq!((timings.unattributed(), timings.over_attributed()), (us(1), us(0)));
    let doc: Value = serde_json::from_str(&timings.to_json()).unwrap();
    assert_eq!(doc["spans"][0]["elapsed_us"], json!(1));
    assert_eq!(doc["unattributed_us"], json!(1));
    let decoded = Timings::from_json(&timings.to_json()).expect("the rounded document reconciles");
    assert_eq!(decoded.span(&[Stage::Commit]).unwrap().elapsed(), us(1));

    let sub_micro = Timings::new(Scope::Library, nanos(999), list(ChildrenKind::Sequential, [Span::new(Stage::Commit, nanos(999))]));
    assert_eq!((sub_micro.unattributed(), sub_micro.over_attributed()), (us(0), us(0)));
    assert!(Timings::from_json(&sub_micro.to_json()).is_ok());
}

#[test]
fn an_unsupported_version_is_rejected_at_either_level() {
    let mut doc = document();
    doc["format_version"] = json!(2);
    assert_eq!(Timings::from_json(&doc.to_string()), Err(TimingsError::UnsupportedVersion(2)));

    let mut doc = document();
    doc["worker_reports"][1]["report"]["format_version"] = json!(2);
    assert_eq!(Timings::from_json(&doc.to_string()), Err(TimingsError::UnsupportedVersion(2)));

    let mut worker: Value = serde_json::from_str(&worker_timings().to_json()).unwrap();
    worker["format_version"] = json!(0);
    assert_eq!(WorkerTimings::from_json(&worker.to_string()), Err(TimingsError::UnsupportedVersion(0)));
}

#[test]
fn an_unknown_stage_id_is_rejected() {
    let mut doc = document();
    doc["spans"][1]["children"][2]["children"][0]["stage"] = json!("graph_gather");

    assert_eq!(Timings::from_json(&doc.to_string()), Err(TimingsError::UnknownStage(UnknownStage("graph_gather".into()))));
}

#[test]
fn a_repeated_sibling_stage_is_rejected() {
    let mut doc = document();
    let children = doc["spans"][1]["children"][1]["children"].as_array_mut().unwrap();
    children[1] = children[0].clone();

    assert_eq!(
        Timings::from_json(&doc.to_string()),
        Err(TimingsError::DuplicateSibling(vec![Stage::RemoteAndLocal, Stage::LocalGather, Stage::WorktreeStatus]))
    );
}

#[test]
fn inconsistent_reconciliation_is_rejected_where_it_occurs() {
    let path = |doc: &Value| Timings::from_json(&doc.to_string());
    let graph = vec![Stage::RemoteAndLocal, Stage::GraphHistory];

    let mut doc = document();
    doc["unattributed_us"] = json!(339);
    assert_eq!(path(&doc), Err(TimingsError::InconsistentReconciliation(vec![])), "root");

    let mut doc = document();
    doc["spans"][1]["children"][2]["unattributed_us"] = json!(0);
    doc["spans"][1]["children"][2]["over_attributed_us"] = json!(0);
    assert_eq!(path(&doc), Err(TimingsError::InconsistentReconciliation(graph.clone())), "a sequential parent");

    let mut doc = document();
    doc["spans"][1]["children"][2]["children_kind"] = json!("concurrent");
    assert_eq!(path(&doc), Err(TimingsError::InconsistentReconciliation(graph)), "a concurrent parent claims a remainder");

    let mut doc = document();
    doc["spans"][1]["over_attributed_us"] = json!(250);
    assert_eq!(path(&doc), Err(TimingsError::InconsistentReconciliation(vec![Stage::RemoteAndLocal])), "concurrent children summed");

    let mut doc = document();
    doc["spans"][0]["unattributed_us"] = json!(10);
    assert_eq!(path(&doc), Err(TimingsError::InconsistentReconciliation(vec![Stage::Startup])), "a leaf claims its own time");

    let mut doc = document();
    doc["spans"][0]["elapsed_us"] = json!(11);
    assert_eq!(path(&doc), Err(TimingsError::InconsistentReconciliation(vec![])), "a changed child unbalances its parent");
}

#[test]
fn worker_report_summary_follows_the_launches() {
    let report = |index: u32, report: LaunchReport| WorkerReport { launch_index: index, attempt_id: ATTEMPT_0.into(), report };
    let complete = || LaunchReport::Complete(worker_timings());
    let summary = summarize_worker_reports;

    assert_eq!(summary(&[report(0, complete())], false, false), WorkerReportStatus::Complete);
    assert_eq!(summary(&[report(0, complete()), report(1, complete())], false, false), WorkerReportStatus::Complete, "a retry");
    assert_eq!(summary(&[report(0, LaunchReport::Invalid), report(1, complete())], false, false), WorkerReportStatus::Partial);
    assert_eq!(summary(&[report(0, LaunchReport::Partial(worker_timings()))], false, false), WorkerReportStatus::Partial);
    assert_eq!(summary(&[report(0, LaunchReport::Missing), report(1, LaunchReport::Invalid)], false, false), WorkerReportStatus::Invalid, "the latest launch");
    assert_eq!(summary(&[report(0, LaunchReport::Invalid), report(1, LaunchReport::Missing)], false, false), WorkerReportStatus::Missing, "the latest launch");
    assert_eq!(summary(&[], false, false), WorkerReportStatus::Missing);
    assert_eq!(summary(&[], true, false), WorkerReportStatus::Adopted);
    assert_eq!(summary(&[report(0, LaunchReport::Invalid)], true, false), WorkerReportStatus::Adopted, "no owned report");
    assert_eq!(summary(&[report(0, complete())], true, false), WorkerReportStatus::Complete, "adoption does not hide an owned report");
    assert_eq!(summary(&[report(0, complete())], true, true), WorkerReportStatus::OriginChanged);
}

#[test]
fn worker_reports_must_be_ordered_launches_matching_their_summary() {
    let decode = |doc: &Value| Timings::from_json(&doc.to_string());
    let invalid = |doc: &Value| matches!(decode(doc), Err(TimingsError::InvalidWorkerReports(_)));

    let mut doc = document();
    doc["worker_reports"][0]["launch_index"] = json!(1);
    assert!(invalid(&doc), "launch indexes repeat");

    let mut doc = document();
    doc["worker_report_status"] = json!("complete");
    assert!(invalid(&doc), "one launch has no report");

    let mut doc = document();
    doc["worker_reports"][0]["status"] = json!("complete");
    assert!(invalid(&doc), "a usable status with no report");

    let mut doc = document();
    doc["worker_reports"][1]["status"] = json!("missing");
    assert!(invalid(&doc), "an unusable status with a report");

    let mut doc = document();
    doc["worker_reports"][0]["status"] = json!("adopted");
    assert!(invalid(&doc), "adopted is a listing status");

    let mut doc = document();
    doc["worker_reports"][0]["attempt_id"] = json!("https://example.invalid/secret");
    assert!(invalid(&doc), "only an attempt id identifies a launch");

    let mut doc = document();
    doc["worker_reports"] = json!([]);
    doc["worker_report_status"] = json!("origin_changed");
    assert!(decode(&doc).is_ok(), "suppressed reports");
    doc["worker_report_status"] = json!("adopted");
    assert!(decode(&doc).is_ok());
    doc["worker_report_status"] = json!("invalid");
    assert!(invalid(&doc), "invalid needs a launch");
}

// --- Input Robustness Matrix ------------------------------------------------

/// One edit of the fixture document.
enum Edit {
    Remove(Target, &'static str),
    Set(Target, &'static str, Value),
    /// Writes the key twice with its own value.
    Repeat(Target, &'static str),
    Append(&'static str),
}

#[derive(Clone, Copy)]
enum Target {
    Root,
    /// `graph_history`: a sequential parent with a count and a remainder.
    Graph,
    /// `regather`: a concurrent parent.
    Regather,
    /// The second launch, which has a complete report.
    Launch,
}

impl Target {
    fn of(self, doc: &mut Value) -> &mut Value {
        match self {
            Target::Root => doc,
            Target::Graph => &mut doc["spans"][1]["children"][2],
            Target::Regather => &mut doc["spans"][2],
            Target::Launch => &mut doc["worker_reports"][1],
        }
    }
}

enum Outcome {
    Reject,
    Accept(fn(&Timings)),
}

fn apply(edits: &[Edit]) -> String {
    let mut doc = document();
    let mut repeated = None;
    let mut suffix = "";
    for edit in edits {
        match edit {
            Edit::Remove(target, key) => {
                target.of(&mut doc).as_object_mut().unwrap().remove(*key).expect("the fixture has the key");
            }
            Edit::Set(target, key, value) => target.of(&mut doc)[*key] = value.clone(),
            Edit::Repeat(target, key) => {
                let object = target.of(&mut doc);
                let text = object.to_string();
                let member = format!("{}:{}", json!(key), object[*key]);
                repeated = Some(format!("{{{member},{}", &text[1..]));
                *object = json!("@@repeated@@");
            }
            Edit::Append(text) => suffix = text,
        }
    }
    let mut text = doc.to_string();
    if let Some(object) = repeated {
        text = text.replace(r#""@@repeated@@""#, &object);
    }
    text + suffix
}

#[test]
fn the_decoder_matrix_rejects_every_malformed_cell() {
    use Edit::{Append, Remove, Repeat, Set};
    use Outcome::{Accept, Reject};
    use Target::{Graph, Launch, Regather, Root};

    let graph_path = [Stage::RemoteAndLocal, Stage::GraphHistory];
    let control = Timings::from_json(&apply(&[])).expect("control: the unedited fixture decodes");
    assert_eq!(control, fixture());
    assert_eq!(control.span(&graph_path).unwrap().git_calls(), Some(12));

    let mut cells: Vec<(&str, Vec<Edit>, Outcome)> = Vec::new();
    // Unsigned integer fields share one row of cells.
    for (target, key) in [
        (Root, "format_version"),
        (Root, "total_us"),
        (Root, "unattributed_us"),
        (Root, "over_attributed_us"),
        (Graph, "elapsed_us"),
        (Graph, "unattributed_us"),
        (Graph, "over_attributed_us"),
    ] {
        cells.push(("absent", vec![Remove(target, key)], Reject));
        cells.push(("null", vec![Set(target, key, Value::Null)], Reject));
        cells.push(("wrong type", vec![Set(target, key, json!("1"))], Reject));
        cells.push(("negative", vec![Set(target, key, json!(-1))], Reject));
        cells.push(("fractional", vec![Set(target, key, json!(1.5))], Reject));
        cells.push(("repeated key", vec![Repeat(target, key)], Reject));
    }
    cells.push(("format_version 2", vec![Set(Root, "format_version", json!(2))], Reject));
    for (target, key) in [(Root, "scope"), (Root, "worker_report_status"), (Graph, "stage"), (Graph, "children_kind")] {
        cells.push(("absent", vec![Remove(target, key)], Reject));
        cells.push(("null", vec![Set(target, key, Value::Null)], Reject));
        cells.push(("wrong type", vec![Set(target, key, json!(1))], Reject));
        cells.push(("unknown value", vec![Set(target, key, json!("parallel"))], Reject));
        cells.push(("empty", vec![Set(target, key, json!(""))], Reject));
        cells.push(("repeated key", vec![Repeat(target, key)], Reject));
    }
    for (target, key) in [(Root, "spans"), (Root, "worker_reports"), (Graph, "children")] {
        cells.push(("absent", vec![Remove(target, key)], Reject));
        cells.push(("null", vec![Set(target, key, Value::Null)], Reject));
        cells.push(("wrong type", vec![Set(target, key, json!({}))], Reject));
        cells.push(("one wrong element", vec![Set(target, key, one_wrong_element(target, key))], Reject));
        cells.push(("every element wrong", vec![Set(target, key, json!([1, 2, 3]))], Reject));
        cells.push(("repeated key", vec![Repeat(target, key)], Reject));
    }
    cells.push((
        "git_calls absent is unknown",
        vec![Remove(Graph, "git_calls")],
        Accept(|timings| assert_eq!(timings.span(&[Stage::RemoteAndLocal, Stage::GraphHistory]).unwrap().git_calls(), None)),
    ));
    cells.push(("git_calls null", vec![Set(Graph, "git_calls", Value::Null)], Reject));
    cells.push(("git_calls wrong type", vec![Set(Graph, "git_calls", json!("12"))], Reject));
    cells.push(("git_calls negative", vec![Set(Graph, "git_calls", json!(-1))], Reject));
    cells.push(("git_calls fractional", vec![Set(Graph, "git_calls", json!(1.5))], Reject));
    cells.push(("git_calls repeated key", vec![Repeat(Graph, "git_calls")], Reject));
    cells.push((
        "git_calls zero is measured",
        vec![Set(Graph, "git_calls", json!(0))],
        Accept(|timings| assert_eq!(timings.span(&[Stage::RemoteAndLocal, Stage::GraphHistory]).unwrap().git_calls(), Some(0))),
    ));
    cells.push((
        "children empty",
        vec![Set(Regather, "children", json!([]))],
        Accept(|timings| assert!(timings.span(&[Stage::Regather]).unwrap().children().is_empty())),
    ));
    cells.push((
        "spans empty",
        vec![Set(Root, "spans", json!([])), Set(Root, "unattributed_us", json!(1000))],
        Accept(|timings| assert!(timings.spans().is_empty())),
    ));
    cells.push((
        "worker_reports empty with no worker",
        vec![Set(Root, "worker_reports", json!([])), Remove(Root, "worker_report_status")],
        Accept(|timings| assert_eq!((timings.worker_reports().len(), timings.worker_report_status()), (0, None))),
    ));
    cells.push(("worker_reports empty under a partial summary", vec![Set(Root, "worker_reports", json!([]))], Reject));
    cells.push(("report absent under a usable status", vec![Remove(Launch, "report")], Reject));
    cells.push(("report null", vec![Set(Launch, "report", Value::Null)], Reject));
    cells.push(("report wrong type", vec![Set(Launch, "report", json!([]))], Reject));
    cells.push(("report empty", vec![Set(Launch, "report", json!({}))], Reject));
    cells.push(("report repeated key", vec![Repeat(Launch, "report")], Reject));
    cells.push((
        "report absent under missing",
        vec![Remove(Launch, "report"), Set(Launch, "status", json!("missing")), Set(Root, "worker_report_status", json!("missing"))],
        Accept(|timings| assert_eq!(timings.worker_reports()[1].report, LaunchReport::Missing)),
    ));
    cells.push(("stage is a label", vec![Set(Graph, "stage", json!("graph history"))], Reject));
    cells.push(("trailing garbage", vec![Append(" x")], Reject));
    cells.push(("a second document", vec![Append("{}")], Reject));
    cells.push((
        "an unknown field is ignored",
        vec![Set(Graph, "note", json!({"anything": [1]})), Set(Root, "extra", json!(true))],
        Accept(|timings| assert_eq!(timings, &fixture())),
    ));

    let mut failures = Vec::new();
    for (name, edits, outcome) in &cells {
        let text = apply(edits);
        let field = match &edits[0] {
            Remove(_, key) | Set(_, key, _) | Repeat(_, key) => key,
            Append(_) => &"document",
        };
        match (Timings::from_json(&text), outcome) {
            (Ok(_), Reject) => failures.push(format!("{field} / {name}: accepted")),
            (Err(error), Accept(_)) => failures.push(format!("{field} / {name}: rejected ({error})")),
            (Ok(timings), Accept(check)) => check(&timings),
            (Err(_), Reject) => {}
        }
    }
    assert!(failures.is_empty(), "{} matrix cells failed:\n{}", failures.len(), failures.join("\n"));
}

/// The fixture's array at `key` with its first element replaced by a number.
fn one_wrong_element(target: Target, key: &str) -> Value {
    let mut doc = document();
    let mut array = target.of(&mut doc)[key].clone();
    array[0] = json!(123);
    array
}
