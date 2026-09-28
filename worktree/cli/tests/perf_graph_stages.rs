//! Graph-stage timings of `wt list --perf` on the [`GraphFixture`] shapes:
//! ordinary history, older essential connections, and multiple selected
//! branches.
//!
//! `wt` gathers and renders the graph only when stderr is a terminal and
//! `TERM_PROGRAM` names an image-capable emulator, so each run goes through
//! `script` in a 120×40 pseudo-terminal with `TERM_PROGRAM=ghostty`. Nothing is
//! asserted about the durations (the graph has no numeric budget; reviewers
//! compare the recorded medians); the test proves both stages run on every
//! shape and prints their medians.
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

/// Runs `wt list --perf` in a 120×40 pseudo-terminal and returns everything
/// the terminal received.
fn list_perf_in_pty(fixture: &GraphFixture) -> String {
    let wt = fixture.wt_command();
    let program = wt.get_program().to_string_lossy().into_owned();
    let inner = format!("stty cols 120 rows 40; exec {} list --perf", quote(&program));
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

#[test]
#[serial]
fn perf_graph_stages_are_reported_for_every_graph_fixture() {
    let samples = samples();
    let mut table = vec![format!("| Fixture | Samples | {GATHER} (median) | {RENDER} (median) |"), "|---|---|---|---|".into()];
    for fixture in GraphFixture::all() {
        // Warm git's and the comparison cache's state when there is a median to protect.
        if samples > 1 {
            list_perf_in_pty(&fixture);
        }
        let mut gathers = Vec::new();
        let mut renders = Vec::new();
        for _ in 0..samples {
            let output = list_perf_in_pty(&fixture);
            let stage = |name: &str| {
                stage_from_perf(&output, name)
                    .unwrap_or_else(|| panic!("no `{name}` stage for {}:\n{output}", fixture.name))
            };
            gathers.push(stage(GATHER));
            renders.push(stage(RENDER));
        }
        table.push(format!(
            "| {} | {samples} | {:.1?} | {:.1?} |",
            fixture.name,
            median(gathers),
            median(renders)
        ));
    }
    println!("{}", table.join("\n"));
}
