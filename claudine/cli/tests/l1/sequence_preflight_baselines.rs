//! Baselines of how a real `claudine sequence` run fails before its first
//! task, captured before the preflight walker gained its recovering mode.
//!
//! The passive dry run walks the same tree with recovery, collecting every
//! failure; a real run must keep stopping at its first one, with the same
//! typed failure reaching the root's lifecycle. Each case pins the exit
//! status, the `err.*` facets the root's `finalize` reads (written to
//! `events.log`, the run's only persisted record of the failure), and that no
//! provider launched. The last case is the one where the dry run's grouped
//! display order (task by task) differs from the order a real run meets the
//! failures in: the graph refusal of the second task wins over the first
//! task's missing input.
//!
//! Error prose is deliberately not pinned here; only the facets a lifecycle
//! handler matches on are.
//!
//! Since the real run reports every preflight problem together, each case
//! also pins the report it prints in place of the error block: its rows, and
//! that the failure it returns (`err.msg`, the "stopped at" line) is still
//! the one the baseline recorded, whichever row the report shows first.

use std::fs;

use crate::common::{CliProcessFixture, strip_ansi};

struct Outcome {
    code: Option<i32>,
    /// `finalize`'s record of `err.code err.category err.disposition
    /// err.origin`, one line per run.
    facets: Vec<String>,
    stderr: String,
    /// How many times a provider launched.
    launches: usize,
    launched: bool,
}

const FINALIZE: &str = "finalize:\n  - append_line: [\"events.log\", \"{{ err.code }} {{ err.category }} {{ err.disposition }} {{ err.origin }}\"]\n  - append_line: [\"events.log\", \"msg: {{ err.msg }}\"]\n";

/// The heading of the report a real run prints when preflight stops it.
const REPORT_HEADING: &str = "Sequence preflight found";

/// Runs `claudine sequence seq.md` over `files` with a recording `goose`
/// provider on `PATH`.
fn run(files: &[(&str, &str)]) -> Outcome {
    let fixture = CliProcessFixture::named("sequence-preflight-baseline");
    fixture.seed_user_config();
    for (name, text) in files {
        let path = fixture.cwd().join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
    let marker = fixture.cwd().join("launched.log");
    let goose = fixture.bin_dir().join(format!("goose{}", std::env::consts::EXE_SUFFIX));
    fs::copy(biscuit_test_harness::bin_exe!("claudine-fake-goose"), &goose).unwrap();
    let output = fixture
        .command()
        .env("NO_COLOR", "1")
        .env("PROVIDER_MARKER", &marker)
        .args(["sequence", "seq.md"])
        .output()
        .unwrap();
    let facets = fs::read_to_string(fixture.cwd().join("events.log"))
        .map(|text| text.lines().map(str::to_string).collect())
        .unwrap_or_default();
    let launches = fs::read_to_string(&marker).map(|text| text.lines().count()).unwrap_or(0);
    Outcome {
        code: output.status.code(),
        facets,
        stderr: strip_ansi(&String::from_utf8_lossy(&output.stderr)),
        launches,
        launched: marker.exists(),
    }
}

/// `cause` is part of the typed failure's `err.msg`, which tells apart
/// failures that share the catch-all `composition.failed` facets today; the
/// report's last line names the same failure. `rows` are the report's
/// findings, in the order it shows them.
fn assert_baseline(outcome: &Outcome, code: i32, facets: &str, cause: &str, rows: &[&str]) {
    assert_eq!(outcome.code, Some(code), "stderr:\n{}", outcome.stderr);
    assert_eq!(outcome.facets.first().map(String::as_str), Some(facets), "stderr:\n{}", outcome.stderr);
    assert_eq!(outcome.facets.len(), 2, "finalize ran once; stderr:\n{}", outcome.stderr);
    let msg = flat(&outcome.facets[1]);
    assert!(msg.contains(cause), "`err.msg` is the recorded failure `{cause}`: {msg}");
    let stderr = flat(&outcome.stderr);
    // The closing line names the failure unless one of the rows already does.
    if let Some(stopped) = stderr.split("The run stopped at:").nth(1) {
        assert!(stopped.contains(cause), "the report names the failure that stopped the run, `{cause}`; stderr:\n{}", outcome.stderr);
    }
    assert_eq!(outcome.stderr.matches(REPORT_HEADING).count(), 1, "the report renders once; stderr:\n{}", outcome.stderr);
    assert!(!outcome.stderr.contains('┃'), "the report replaces the error block; stderr:\n{}", outcome.stderr);
    let mut rest = stderr.as_str();
    for row in rows {
        let at = rest.find(row).unwrap_or_else(|| panic!("the report shows `{row}` after the rows before it; stderr:\n{}", outcome.stderr));
        rest = &rest[at + row.len()..];
    }
    assert!(!outcome.launched, "no provider launches before preflight passes; stderr:\n{}", outcome.stderr);
}

/// `text` with its whitespace runs collapsed to single spaces.
fn flat(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// One case per preflight boundary a real run can stop at, each pinning the
/// typed failure the boundary returns (its `finalize` facets and `err.msg`)
/// and the report's rows, so a report never changes which failure a
/// lifecycle handler sees. [`BOUNDARIES`](boundaries::BOUNDARIES) is the list
/// of boundaries; a guard keeps it equal to the cases.
mod boundaries {
    use super::*;

    macro_rules! boundary_cases {
        ($($(#[$doc:meta])* $name:ident: files = $files:expr, facets = $facets:expr, cause = $cause:expr, rows = $rows:expr;)*) => {
            $($(#[$doc])* #[test] fn $name() {
                let outcome = run($files);
                assert_baseline(&outcome, 1, $facets, $cause, $rows);
            })*
            pub(super) const CASES: &[&str] = &[$(stringify!($name)),*];
        };
    }

    /// The boundaries a real sequence run stops at before its first task.
    pub(super) const BOUNDARIES: &[&str] =
        &["graph", "inputs", "prompt_load", "provider", "sequence_resolution", "transclusion"];

    boundary_cases! {
        /// The task graph refuses a group whose `tasks` list is empty.
        graph: files = &[(
            "seq.md",
            &format!("---\nagent: goose\nsequence:\n  - name: fan\n    group:\n      tasks: []\n{FINALIZE}---\n"),
        )],
        facets = "composition.failed composition correctable author",
        cause = "is invalid: `tasks` must declare at least one task",
        rows = &["task fan (sequence[0])", "is invalid: `tasks` must declare at least one task"];

        /// A `prompt:` names a file that does not exist.
        prompt_load: files = &[(
            "seq.md",
            &format!("---\nagent: goose\nsequence:\n  - name: find\n    prompt: ./missing.md\n{FINALIZE}---\n"),
        )],
        facets = "composition.failed composition correctable author",
        cause = "failed to load external sequence file: `./missing.md`: file not found",
        rows = &["task find (sequence[0])", "the prompt file does not exist", "at seq.md:5:13", "hint: create missing.md before a real run"];

        /// A `sequence:` task names a sequence document that does not exist.
        sequence_resolution: files = &[(
            "seq.md",
            &format!("---\nagent: goose\nsequence:\n  - name: nested\n    sequence: ./child.md\n{FINALIZE}---\n"),
        )],
        facets = "composition.failed composition correctable author",
        cause = "failed to load external sequence file: `./child.md`: file not found",
        rows = &["task nested (sequence[0])", "the sequence file does not exist", "at seq.md:5:15"];

        /// A prompt document includes a file that does not exist.
        transclusion: files = &[
            ("ask.md", "Context:\n\n::file ./absent.md\n"),
            ("seq.md", &format!("---\nagent: goose\nsequence:\n  - name: ask\n    prompt: ./ask.md\n{FINALIZE}---\n")),
        ],
        facets = "composition.failed composition correctable author",
        cause = "pre-flight discovery failed: Transclusion error:",
        rows = &["task ask (sequence[0])", "File not found: ./absent.md", "at ask.md:3:1"];

        /// A task's `with.agent` names no provider.
        provider: files = &[
            ("ask.md", "ask\n"),
            ("seq.md", &format!("---\nsequence:\n  - name: ask\n    prompt: ./ask.md\n    with:\n      agent: nosuch\n{FINALIZE}---\n")),
        ],
        facets = "composition.failed composition correctable author",
        cause = "cannot select a provider from `tasks[0].with.agent`",
        rows = &["task ask (sequence[0])", "`nosuch` (with.agent) is not a known provider", "3. with.agent `nosuch`: not a known provider"];

        /// The root's required input, missing in a non-interactive run.
        inputs: files = &[(
            "seq.md",
            &format!("---\nagent: goose\n$schema:\n  topic: string(required)\nsequence:\n  - name: one\n{FINALIZE}---\nWrite about {{{{ topic }}}}.\n"),
        )],
        facets = "composition.missing_properties composition correctable author",
        cause = "sequence schema validation failed for 1 step(s) or document(s); missing required properties",
        rows = &["seq.md", "`topic` (string) is required and has no value; a non-interactive run fails here"];
    }

    /// The root's missing input is reported once: its body is not also
    /// previewed without it.
    #[test]
    fn a_missing_root_input_is_reported_once() {
        let outcome = run(&[(
            "seq.md",
            &format!("---\nagent: goose\n$schema:\n  topic: string(required)\nsequence:\n  - name: one\n{FINALIZE}---\nWrite about {{{{ topic }}}}.\n"),
        )]);
        assert!(!outcome.stderr.contains("is a required property"), "stderr:\n{}", outcome.stderr);
        assert!(outcome.stderr.contains("Sequence preflight found 1 error and 0 warnings"), "stderr:\n{}", outcome.stderr);
    }

    #[test]
    fn every_preflight_boundary_has_its_case() {
        let mut cases = CASES.to_vec();
        cases.sort_unstable();
        assert_eq!(cases, BOUNDARIES);
    }
}

/// A missing root document is no preflight stop: nothing is walked, no
/// lifecycle runs, and the error renders as it always has, with no report.
#[test]
fn a_missing_root_document_is_reported_as_before() {
    let fixture = CliProcessFixture::named("sequence-preflight-baseline");
    fixture.seed_user_config();
    let output = fixture.command().env("NO_COLOR", "1").args(["sequence", "./absent.md"]).output().unwrap();
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    assert_eq!(output.status.code(), Some(1), "stderr:\n{stderr}");
    assert!(!stderr.contains(REPORT_HEADING), "stderr:\n{stderr}");
    assert!(stderr.contains("absent.md"), "stderr:\n{stderr}");
}

/// The first task misses an input and the second names a missing file. A
/// dry run lists the first task's finding first; a real run stops at the
/// second task's graph refusal, which comes before input collection.
#[test]
fn a_later_graph_refusal_wins_over_an_earlier_missing_input() {
    let outcome = run(&[
        ("needs.md", "---\n$schema:\n  topic: string(required)\n---\nWrite about {{ topic }}.\n"),
        (
            "seq.md",
            &format!(
                "---\nagent: goose\nsequence:\n  - name: one\n    prompt: ./needs.md\n  - name: two\n    prompt: ./missing.md\n{FINALIZE}---\n"
            ),
        ),
    ]);
    assert_baseline(&outcome, 1, "composition.failed composition correctable author",
        "failed to load external sequence file: `./missing.md`: file not found",
        &["task one (sequence[0])", "`topic` (string) is required and has no value", "task two (sequence[1])", "the prompt file does not exist"]);
    // No row says the failure in its own words, so the closing line names it.
    assert!(flat(&outcome.stderr).contains("The run stopped at: failed to load external sequence file"), "stderr:\n{}", outcome.stderr);
}

/// A task fails and the root's `failure` retries; between the attempts an
/// earlier task broke a sequence file the root names, so the retry's
/// preflight, reread under the first attempt's closed approvals, stops.
/// `blocked` and `finalize` each run once, for that failure, and the report
/// is printed once.
#[test]
fn a_retry_that_stops_at_preflight_reports_once() {
    let outcome = run(&[
        ("child.yaml", "kind: sequence\nsequence:\n  - { name: original, side_effect: { append_line: [events.log, child] } }\n"),
        (
            "seq.md",
            &format!(
                "---\nagent: goose\nready: false\nfailure: [{{ retry: {{ max_attempts: 1, with: {{ ready: true }} }} }}]\nblocked: [{{ append_line: [events.log, blocked] }}]\nsequence:\n  - {{ name: nested, sequence: child.yaml }}\n  - name: rewrite\n    side_effect: {{ append_line: [child.yaml, '  - {{ name: broken'] }}\n  - name: gate\n    sequence: []\n    start: [{{ when: \"!ready\", then: [{{ error: not-ready }}] }}]\n{FINALIZE}---\n"
            ),
        ),
    ]);
    assert_eq!(outcome.code, Some(1), "stderr:\n{}", outcome.stderr);
    let [first, blocked, facets, msg] = outcome.facets.as_slice() else {
        panic!("the first attempt's task, `blocked` once, then `finalize` once: {:?}", outcome.facets);
    };
    assert_eq!([first.as_str(), blocked.as_str(), facets.as_str()], ["child", "blocked", "composition.failed composition correctable author"]);
    assert!(msg.contains("failed to load external sequence file:") && msg.contains("YAML parse error"), "{msg}");
    assert_eq!(outcome.stderr.matches(REPORT_HEADING).count(), 1, "stderr:\n{}", outcome.stderr);
    let stderr = flat(&outcome.stderr);
    assert!(stderr.contains("task nested (sequence[0])"), "stderr:\n{}", outcome.stderr);
    assert!(!outcome.stderr.contains('┃'), "stderr:\n{}", outcome.stderr);
    assert!(!outcome.launched);
}

/// A root input the root's `initialize` writes is no problem in the report:
/// the report reads the state the run stopped in.
#[test]
fn an_input_initialize_writes_is_not_reported_missing() {
    let outcome = run(&[(
        "seq.md",
        &format!(
            "---\nagent: goose\n$schema:\n  topic: string(required)\ninitialize:\n  - set: {{ topic: tides }}\nsequence:\n  - name: find\n    prompt: ./missing.md\n{FINALIZE}---\nWrite about {{{{ topic }}}}.\n"
        ),
    )]);
    assert_baseline(&outcome, 1, "composition.failed composition correctable author",
        "failed to load external sequence file: `./missing.md`: file not found", &[]);
    assert!(!outcome.stderr.contains("topic"), "`topic` was written by `initialize`; stderr:\n{}", outcome.stderr);
}

/// A real run's preflight passes and an earlier task makes the file a later
/// task's prompt reads: nothing is reported, and both tasks run. A problem a
/// task meets only at its turn is never moved to preflight.
#[test]
fn a_file_an_earlier_task_writes_is_not_a_preflight_problem() {
    let outcome = run(&[
        ("later.md", "::block when=\"file_exists('./made.txt')\"\nWrite the summary.\n::end-block\n"),
        (
            "seq.md",
            "---\nagent: goose\nsequence:\n  - name: make\n    side_effect: { append_line: [made.txt, ready] }\n  - name: later\n    prompt: ./later.md\n---\n",
        ),
    ]);
    assert_eq!(outcome.code, Some(0), "stderr:\n{}", outcome.stderr);
    assert!(!outcome.stderr.contains(REPORT_HEADING), "stderr:\n{}", outcome.stderr);
    assert_eq!(outcome.launches, 1, "the later task launched; stderr:\n{}", outcome.stderr);
}

/// A warning no stage of a real run shows (a step's `agent:`, which the task
/// reads as data) is printed once, before the first task, and the run goes
/// on.
#[test]
fn an_advisory_warning_is_printed_once_before_the_first_task() {
    let outcome = run(&[
        ("ask.md", "ask\n"),
        (
            "seq.md",
            "---\nagent: goose\nsequence:\n  - name: ask\n    agent: claude\n    prompt: ./ask.md\n  - name: again\n    prompt: ./ask.md\n---\n",
        ),
    ]);
    assert_eq!(outcome.code, Some(0), "stderr:\n{}", outcome.stderr);
    assert_eq!(outcome.stderr.matches(REPORT_HEADING).count(), 1, "stderr:\n{}", outcome.stderr);
    let stderr = flat(&outcome.stderr);
    assert!(stderr.contains("Sequence preflight found 0 errors and 1 warning; the sequence runs"), "stderr:\n{}", outcome.stderr);
    assert!(stderr.contains("`agent: claude` is step data (read as `seq.agent`), not the task's provider"), "stderr:\n{}", outcome.stderr);
    let warned = outcome.stderr.find(REPORT_HEADING).unwrap();
    let first_task = outcome.stderr.find("[1/2] starting ask").expect("the first task starts");
    assert!(warned < first_task, "printed before the first task; stderr:\n{}", outcome.stderr);
    assert_eq!(outcome.launches, 2, "both tasks ran; stderr:\n{}", outcome.stderr);
}
