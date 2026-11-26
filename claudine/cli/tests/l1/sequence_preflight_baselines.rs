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

use std::fs;

use crate::common::{CliProcessFixture, strip_ansi};

struct Outcome {
    code: Option<i32>,
    /// `finalize`'s record of `err.code err.category err.disposition
    /// err.origin`, one line per run.
    facets: Vec<String>,
    stderr: String,
    launched: bool,
}

const FINALIZE: &str = "finalize:\n  - append_line: [\"events.log\", \"{{ err.code }} {{ err.category }} {{ err.disposition }} {{ err.origin }}\"]\n";

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
    Outcome {
        code: output.status.code(),
        facets,
        stderr: strip_ansi(&String::from_utf8_lossy(&output.stderr)),
        launched: marker.exists(),
    }
}

/// `cause` is the text that names the typed failure in the rendered report,
/// which tells apart failures that share the catch-all `composition.failed`
/// facets today.
fn assert_baseline(outcome: &Outcome, code: i32, facets: &str, cause: &str) {
    assert_eq!(outcome.code, Some(code), "stderr:\n{}", outcome.stderr);
    assert_eq!(outcome.facets, vec![facets.to_string()], "stderr:\n{}", outcome.stderr);
    let flat = outcome.stderr.replace('┃', " ").split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(flat.contains(cause), "the failure names `{cause}`; stderr:\n{}", outcome.stderr);
    assert!(!outcome.launched, "no provider launches before preflight passes; stderr:\n{}", outcome.stderr);
}

/// A graph refusal: a group whose `tasks` list is empty.
#[test]
fn a_graph_refusal_reaches_finalize() {
    let outcome = run(&[(
        "seq.md",
        &format!("---\nagent: goose\nsequence:\n  - name: fan\n    group:\n      tasks: []\n{FINALIZE}---\n"),
    )]);
    assert_baseline(&outcome, 1, "composition.failed composition correctable author",
        "is invalid: `tasks` must declare at least one task");
}

/// A reference failure: a `prompt:` naming a file that does not exist.
#[test]
fn a_missing_prompt_reference_reaches_finalize() {
    let outcome = run(&[(
        "seq.md",
        &format!("---\nagent: goose\nsequence:\n  - name: find\n    prompt: ./missing.md\n{FINALIZE}---\n"),
    )]);
    assert_baseline(&outcome, 1, "composition.failed composition correctable author",
        "failed to load external sequence file: `./missing.md`: file not found");
}

/// A provider failure: a task's `with.agent` names no provider.
#[test]
fn an_unknown_task_provider_reaches_finalize() {
    let outcome = run(&[
        ("ask.md", "ask\n"),
        (
            "seq.md",
            &format!("---\nsequence:\n  - name: ask\n    prompt: ./ask.md\n    with:\n      agent: nosuch\n{FINALIZE}---\n"),
        ),
    ]);
    assert_baseline(&outcome, 1, "composition.failed composition correctable author",
        "references an invalid Agent provider 'nosuch'");
}

/// A Phase 1c failure: the root's required input, missing in a
/// non-interactive run.
#[test]
fn a_missing_root_input_reaches_finalize() {
    let outcome = run(&[(
        "seq.md",
        &format!(
            "---\nagent: goose\n$schema:\n  topic: string(required)\nsequence:\n  - name: one\n{FINALIZE}---\nWrite about {{{{ topic }}}}.\n"
        ),
    )]);
    assert_baseline(&outcome, 1, "composition.missing_properties composition correctable author",
        "Missing required schema properties in 1 step of the sequence");
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
        "failed to load external sequence file: `./missing.md`: file not found");
}
