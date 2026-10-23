//! The report section `wt remove` prints, before its question, when other
//! processes are using the checkout, rendered from reports injected through
//! the library's real policy ([`assess`]): the spec's example, every evidence
//! location, untrusted text, the listing limits, the coverage notes; and the
//! refusal for a checkout the usage query could not verify, with its exit
//! code.

use std::path::PathBuf;
use std::time::Duration;

use biscuit_terminal::components::prose::{LineBreaks, Prose};
use biscuit_terminal::components::renderable::TerminalRenderable as _;
use biscuit_terminal::discovery::detection::ColorDepth;
use biscuit_terminal::terminal::Terminal;
use serde_json::{Value, json};
use sniff::filesystem::query::PathUsageError;
use worktree::remove::usage::{CoverageSummary, OwnedHandles, UsageClearance, UsageRefusal, assess};
use worktree_cli::commands::remove::usage::{coverage_note, holders_markup, refusal_markup, usage_refusal};

const TARGET: &str = "/work/fix-path-spelling";

/// A report about [`TARGET`] holding `processes`, with `coverage` as
/// `(mechanism, status)` pairs.
fn report(outcome: &str, processes: Vec<Value>, coverage: &[(&str, &str)], budget_exhausted: bool) -> Value {
    json!({
        "schema_version": 1,
        "target": {
            "requested": {"display": TARGET},
            "resolved": {"display": TARGET},
            "kind": "directory",
            "identity": {"kind": "unix", "device": "7", "inode": "3"},
            "recursive": true
        },
        "outcome": outcome,
        "started_at": "2026-10-12T00:00:00Z",
        "finished_at": "2026-10-12T00:00:01Z",
        "budget_ms": 2000,
        "elapsed_us": 1000,
        "budget_exhausted": budget_exhausted,
        "processes": processes,
        "coverage": coverage.iter().map(|(mechanism, status)| json!({
            "mechanism": mechanism,
            "scope": "target_tree",
            "status": status,
            "attempted": null,
            "succeeded": null,
            "reason": null,
            "truncation": null,
            "limitations": []
        })).collect::<Vec<_>>(),
        "limitations": []
    })
}

fn process(pid: u32, name: Option<&str>, executable: Option<&str>, evidence: Vec<Value>) -> Value {
    json!({
        "pid": pid,
        "creation_token": if name.is_some() { json!("77") } else { Value::Null },
        "start_time": null,
        "identity_uncertain": name.is_none(),
        "name": name.map(|name| json!({"display": name})),
        "executable": executable.map(|path| json!({"display": path})),
        "user": "501",
        "evidence": evidence
    })
}

fn evidence(kind: &str, mechanism: &str, paths: &[&str]) -> Value {
    json!({
        "kind": kind,
        "mechanism": mechanism,
        "matched_paths": paths.iter().map(|path| json!({"display": path})).collect::<Vec<_>>(),
        "observed_path": paths.first().map(|path| json!({"display": path})),
        "match_basis": "identity",
        "identity": {"kind": "unix", "device": "7", "inode": "9"},
        "descriptor": "12",
        "watch": null,
        "access": null,
        "event_only": null
    })
}

/// The refusal for `report`, through the library's policy.
fn found(report: Value) -> UsageClearance {
    let report = serde_json::from_value(report).expect("valid report");
    assess(report, &OwnedHandles::default())
}

fn holders(report: Value) -> String {
    render(holders_markup(&found(report)).expect("a holder"))
}

fn render(markup: String) -> String {
    let terminal = Terminal::builder().is_tty(false).osc_link_support(false).width(100).color_depth(ColorDepth::None).build();
    Prose::new(markup).with_line_breaks(LineBreaks::Hard).render(&terminal)
}

/// macOS's usual coverage: handles of other users' processes unreadable,
/// FSEvents and polling not enumerable.
const MACOS_COVERAGE: &[(&str, &str)] = &[
    ("process_enumeration", "complete"),
    ("open_handles", "partial"),
    ("working_directories", "partial"),
    ("fsevents", "unsupported"),
    ("polling", "unsupported"),
];

#[test]
fn the_specs_example() {
    insta::assert_snapshot!(holders(report(
        "usable",
        vec![process(8124, Some("node"), Some("node"), vec![evidence(
            "open_handle",
            "open_handles",
            &["/work/fix-path-spelling/.gitnexus/index.db"],
        )])],
        MACOS_COVERAGE,
        false,
    )));
}

/// Nothing is said when no other process was seen.
#[test]
fn no_holder_says_nothing() {
    assert_eq!(holders_markup(&found(report("usable", vec![], MACOS_COVERAGE, true))), None);
}

/// Several processes, each evidence kind, several names for one object,
/// an object no path names, and a process with no proven lifetime; complete
/// coverage adds no note.
#[test]
fn every_evidence_location_and_holder_shape() {
    let mut unnamed = evidence("open_handle", "open_handles", &[]);
    unnamed["observed_path"] = json!({"display": "/tmp/elsewhere.log"});
    insta::assert_snapshot!(holders(report(
        "usable",
        vec![
            process(311, Some("rust-analyzer"), Some("/opt/bin/rust-analyzer"), vec![
                evidence("watch_registration", "inotify", &["/work/fix-path-spelling/src"]),
                evidence("open_handle", "open_handles", &["/work/fix-path-spelling/a.txt", "/work/fix-path-spelling/b.txt"]),
                unnamed,
            ]),
            process(8124, None, None, vec![
                evidence("working_directory", "working_directories", &["/work/fix-path-spelling"]),
                evidence("loaded_module", "loaded_modules", &["/work/fix-path-spelling/target/debug/tool"]),
            ]),
        ],
        &[("open_handles", "complete"), ("working_directories", "complete")],
        false,
    )));
}

/// Process names and paths can hold markup and control characters; they are
/// shown literally and escaped, never interpreted.
#[test]
fn untrusted_text_is_shown_literally() {
    let rendered = holders(report(
        "usable",
        vec![process(
            8124,
            Some("<red>evil</red>\u{1b}[31m"),
            Some("/bin/*bold*"),
            vec![evidence("open_handle", "open_handles", &["/work/fix-path-spelling/new\nline_`x`.txt"])],
        )],
        &[("open_handles", "complete")],
        false,
    ));

    insta::assert_snapshot!(rendered);
    assert!(!rendered.contains('\u{1b}') && rendered.contains("/new\\nline_`x`.txt"), "{rendered:?}");
}

/// Past the limits, processes and evidence are counted, not listed.
#[test]
fn long_lists_are_counted() {
    let many: Vec<Value> = (0..7).map(|n| evidence("open_handle", "open_handles", &[&format!("{TARGET}/f{n}")])).collect();
    let mut processes = vec![process(100, Some("busy"), Some("busy"), many)];
    processes.extend((101..112).map(|pid| process(pid, Some("worker"), Some("worker"), vec![evidence("open_handle", "open_handles", &[TARGET])])));

    insta::assert_snapshot!(holders(report("usable", processes, &[], false)));
}

/// Each coverage shape the summary can take, as its note.
#[test]
fn coverage_notes() {
    let cases: Vec<(&str, Value)> = vec![
        ("complete", report("usable", vec![], &[("open_handles", "complete")], false)),
        ("macOS", report("usable", vec![], MACOS_COVERAGE, false)),
        ("budget ran out", report("usable", vec![], &[("open_handles", "partial"), ("loaded_modules", "not_attempted")], true)),
        ("failed", report("unavailable", vec![], &[("open_handles", "failed"), ("working_directories", "failed")], false)),
        ("unsupported", report("unsupported", vec![], &[("open_handles", "unsupported")], false)),
    ];
    let mut omitted = report("usable", vec![], &[("open_handles", "complete")], false);
    omitted["coverage"][0]["truncation"] = json!({"per_process_limit": 1000, "total_limit": 10000, "omitted": 12});

    let notes: Vec<String> = cases
        .into_iter()
        .chain([("evidence omitted", omitted)])
        .map(|(case, value)| {
            let summary = CoverageSummary::of(&serde_json::from_value(value).unwrap());
            format!("{case}: {:?}", coverage_note(&summary))
        })
        .collect();

    insta::assert_snapshot!(notes.join("\n"));
}

/// A checkout the query could not verify refuses, for every typed error,
/// with exit 3 and no flag that would override it.
#[test]
fn an_unverified_checkout_refuses() {
    let path = PathBuf::from(TARGET);
    let errors = [
        PathUsageError::MissingTarget { path: path.clone() },
        PathUsageError::UnsupportedTargetKind { path: path.clone(), file_type: "fifo" },
        PathUsageError::RootValidationTimeout { path: path.clone(), budget: Duration::from_secs(2) },
        PathUsageError::RootIdentity { path: path.clone(), source: std::io::Error::other("permission <denied>") },
        PathUsageError::InvalidOption { message: "zero deadline".to_string() },
    ];

    let rendered: Vec<String> = errors
        .into_iter()
        .map(UsageRefusal::Unverified)
        .map(|refusal| {
            let error = usage_refusal(&refusal, "fix-path-spelling", "fix/path-spelling");
            assert_eq!(worktree_cli::exit::exit_code(&error), 3, "{refusal:?}");
            let markup = refusal_markup(&refusal, "fix-path-spelling", "fix/path-spelling");
            assert!(!markup.contains("--force"), "{markup}");
            render(markup)
        })
        .collect();

    insta::assert_snapshot!(rendered.join("\n\n"));
}
