//! Graph-stage timings of `wt list --perf` on the [`GraphFixture`] shapes:
//! ordinary history, older essential connections, multiple selected
//! branches, and the observed sparse-lanes history.
//!
//! `wt` gathers and renders the graph only when stderr is a terminal and
//! `TERM_PROGRAM` names an image-capable emulator, so each run goes through
//! `script` in a pseudo-terminal (120×40, or 200×60 for the observed
//! sparse-lanes history) with `TERM_PROGRAM=ghostty`. Nothing is asserted
//! about the durations (the graph has no numeric budget; reviewers compare the
//! recorded medians); the tests prove both stages run on every shape and
//! print their medians.
//!
//! A pseudo-terminal never answers terminal queries. Ghostty is the
//! Kitty-protocol emulator whose image path asks no cursor-position question
//! (the others wait out a 1 s timeout here), which keeps the render stage
//! under a second, where `--perf` prints tenths of a millisecond instead of
//! tenths of a second. The one-commit floor row measures what fixed cost
//! remains; compare the other rows against it and against their own earlier
//! medians.
//!
//! Set `WT_GRAPH_PERF_SAMPLES` (default 1) for recorded measurements, and run
//! the release profile so rasterization does not dominate:
//! `WT_GRAPH_PERF_SAMPLES=10 just test-perf perf_graph --cargo-profile release`.
#![cfg(unix)]

mod perf_support;

use std::process::{Command, Stdio};
use std::time::Duration;

use serial_test::serial;

use perf_support::graph::GraphFixture;
use perf_support::stage_from_perf;

const GATHER: &str = "graph gather";
const RENDER: &str = "graph image render (biscuit-terminal)";

fn samples() -> usize {
    std::env::var("WT_GRAPH_PERF_SAMPLES")
        .ok()
        .and_then(|value| value.parse().ok())
        .filter(|count| *count > 0)
        .unwrap_or(1)
}

fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r"'\''"))
}

/// Runs `wt list --perf` in a `columns`×`rows` pseudo-terminal and returns
/// everything the terminal received.
fn list_perf_in_pty(fixture: &GraphFixture, columns: u32, rows: u32) -> String {
    let wt = fixture.wt_command();
    let program = wt.get_program().to_string_lossy().into_owned();
    let inner = format!("stty cols {columns} rows {rows}; exec {} list --perf", quote(&program));
    let mut command = Command::new("script");
    if cfg!(target_os = "macos") {
        command.args(["-q", "/dev/null", "/bin/sh", "-c", &inner]);
    } else {
        command.args(["-qec", &format!("/bin/sh -c {}", quote(&inner)), "/dev/null"]);
    }
    for (name, value) in wt.get_envs() {
        match value {
            Some(value) => command.env(name, value),
            None => command.env_remove(name),
        };
    }
    let output = command
        .current_dir(fixture.run_from())
        .env("TERM_PROGRAM", "ghostty")
        .env_remove("KITTY_WINDOW_ID")
        .stdin(Stdio::null())
        .output()
        .expect("script should run");
    let text = String::from_utf8_lossy(&output.stdout).into_owned();
    assert!(output.status.success(), "wt list --perf failed under script for {}:\n{text}", fixture.name);
    text
}

fn median(mut values: Vec<Duration>) -> Duration {
    values.sort();
    values[values.len() / 2]
}

/// The stage durations of `samples` runs, after one warm-up run when there is
/// a median to protect (git's and the comparison cache's state).
fn stage_samples(fixture: &GraphFixture, columns: u32, rows: u32, samples: usize) -> (Vec<Duration>, Vec<Duration>) {
    if samples > 1 {
        list_perf_in_pty(fixture, columns, rows);
    }
    let mut gathers = Vec::new();
    let mut renders = Vec::new();
    for _ in 0..samples {
        let output = list_perf_in_pty(fixture, columns, rows);
        let stage = |name: &str| {
            stage_from_perf(&output, name).unwrap_or_else(|| panic!("no `{name}` stage for {}:\n{output}", fixture.name))
        };
        gathers.push(stage(GATHER));
        renders.push(stage(RENDER));
    }
    (gathers, renders)
}

#[test]
#[serial]
fn perf_graph_stages_are_reported_for_every_graph_fixture() {
    let samples = samples();
    let mut table = vec![format!("| Fixture | Samples | {GATHER} (median) | {RENDER} (median) |"), "|---|---|---|---|".into()];
    for fixture in GraphFixture::all() {
        let (gathers, renders) = stage_samples(&fixture, 120, 40, samples);
        table.push(format!(
            "| {} | {samples} | {:.1?} | {:.1?} |",
            fixture.name,
            median(gathers),
            median(renders)
        ));
    }
    println!("{}", table.join("\n"));
}

/// The observed sparse-lanes history at 200×60, the size the regression was
/// seen at: the render stage's median and its spread across samples, for the
/// before/after comparison of the tag-spacing change.
#[test]
#[serial]
fn perf_graph_stages_for_the_observed_sparse_lanes_at_200x60() {
    let samples = samples();
    let fixture = GraphFixture::observed_sparse_lanes();
    let (gathers, renders) = stage_samples(&fixture, 200, 60, samples);
    let spread = |values: &[Duration]| {
        let min = values.iter().min().expect("a sample");
        let max = values.iter().max().expect("a sample");
        format!("{min:.1?}–{max:.1?}")
    };
    println!(
        "| Fixture | Size | Samples | {GATHER} (median, min–max) | {RENDER} (median, min–max) |\n|---|---|---|---|---|\n| {} | 200×60 | {samples} | {:.1?} ({}) | {:.1?} ({}) |",
        fixture.name,
        median(gathers.clone()),
        spread(&gathers),
        median(renders.clone()),
        spread(&renders)
    );
}

/// The perf fixture is the observed-shape history (plan E1): merge parents,
/// `origin/main`, own-commit counts, and the recorded parents `wt` reads.
#[test]
fn observed_sparse_lanes_graph_fixture_has_the_observed_topology() {
    let fixture = GraphFixture::observed_sparse_lanes();
    let repo = fixture.main();
    let git = |args: &[&str]| {
        let output = Command::new("git").current_dir(repo).args(args).output().expect("git should be installed");
        assert!(output.status.success(), "git {args:?} failed");
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    };
    let at = |rev: &str| git(&["rev-parse", rev]);
    let count = |range: &str| git(&["rev-list", "--count", range]);

    assert_eq!(at("refs/remotes/origin/main"), at("main"));
    assert_eq!(at("HEAD"), at("main"), "listed from the main checkout");
    assert_eq!(at("main^2"), at("fix/sniff"), "M104 merges fix/sniff directly");
    let w1 = at("main^1^1^2");
    assert_eq!(git(&["merge-base", "fix/sniff", "main^1"]), w1, "fix/sniff forks at W1, which M103 merged");
    assert_eq!(at("fix/wt-ux~3^2"), at("main"), "B1 merges main back into fix/wt-ux");
    assert_eq!(count("main..feat/schema-enhancement"), "55");
    assert_eq!(count(&format!("{w1}..fix/sniff")), "94");
    assert_eq!(count("main..fix/wt-ux"), "68", "64 after W1, B1, and 3 more");
    // main~3 is d12 (M103's first parent), main~10 is d5, main~13 is d2.
    assert_eq!(count(&format!("main~3..{w1}")), "8", "w1..w8");
    assert_eq!(git(&["merge-base", "main~3", &w1]), at("main~10"), "fix/wt-ux forks at d5");
    assert_eq!(git(&["merge-base", "main", "feat/schema-enhancement"]), at("main~13"), "the schema branch forks at d2");
    assert_eq!(git(&["worktree", "list", "--porcelain"]).matches("worktree ").count(), 4);

    let store = worktree::fork_origin::ForkOriginStore::load_from(&fixture.fork_store());
    assert_eq!(store.get("fix/sniff").map(|fork| fork.base_branch.as_str()), Some("fix/wt-ux"));
    assert_eq!(store.get("fix/wt-ux").map(|fork| fork.base_branch.as_str()), Some("main"));
    assert_eq!(store.get("feat/schema-enhancement").map(|fork| fork.base_branch.as_str()), Some("main"));
}
