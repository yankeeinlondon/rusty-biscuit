//! Time-to-first-feedback (TTFF) gate for the compose pipeline.
//!
//! Spec reference: `claudine/features/2026-05-09-perf/spec.md` Goal 1 —
//! "≤ 50 ms from `main()` entry to the first stderr line on a
//! representative compose invocation".
//!
//! From outside the process we can only measure wall-clock from spawn
//! to first stderr byte, which adds OS/dynamic-linker overhead on top
//! of `main()` itself. The threshold below is therefore an
//! intentionally generous **perf gate**: it does not assert the spec's
//! ideal 50 ms ceiling but it does fail loudly if a regression pushes
//! the receipt banner past a usable feedback latency for a debug build.
//! The honest measurement of `main()` → first byte stays in
//! `claudine compose --perf` traces; this test is the safety net
//! guarding against accidental insertion of expensive work before the
//! receipt banner.
//!
//! The test is `#[ignore]`d by default because it requires the binary
//! to be built and is sensitive to host load. Run explicitly with
//! `cargo test -p claudine-cli --test compose_ttff_perf -- --ignored`.

use std::fs;
use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use tempfile::tempdir;

mod common;
use common::{augmented_path, write_executable};

/// Documented upper bound for the receipt banner.
///
/// The spec's true target is ≤ 50 ms from `main()` entry; this gate
/// adds spawn / dynamic-linker / OS scheduling overhead and applies to
/// debug builds (which dominate local test runs and CI). Tightening
/// this value as W0/W1 work matures is encouraged.
const TTFF_BUDGET: Duration = Duration::from_millis(1500);

#[cfg(unix)]
#[test]
#[ignore = "perf gate; run explicitly with --ignored"]
fn compose_emits_first_stderr_byte_within_budget() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    // Goose binary that just exits 0 — the receipt banner must arrive
    // long before the agent itself runs, so the agent body is irrelevant.
    write_executable(
        &path_dir.join("goose"),
        r#"#!/bin/sh
exit 0
"#,
    );
    let md_file = workspace.path().join("prompt.md");
    fs::write(&md_file, "# perf body\n").unwrap();

    let bin = env!("CARGO_BIN_EXE_claudine");
    let mut cmd = Command::new(bin);
    cmd.env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .args(["compose", "--goose", md_file.to_str().unwrap()])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());

    let started = Instant::now();
    let mut child = cmd.spawn().expect("spawn claudine");
    let mut stderr = child.stderr.take().expect("piped stderr");

    // Block on the first non-empty stderr read. The pipe is unbuffered
    // so this returns the first line as soon as the child writes it.
    let mut buf = [0u8; 256];
    let n = stderr.read(&mut buf).expect("read stderr");
    let ttff = started.elapsed();

    assert!(n > 0, "first stderr read returned 0 bytes (process exited silently)");

    // Drain the rest in the background so the child doesn't stall
    // waiting on a full pipe.
    std::thread::spawn(move || {
        let mut sink = Vec::new();
        let _ = stderr.read_to_end(&mut sink);
    });
    let _ = child.wait();

    assert!(
        ttff <= TTFF_BUDGET,
        "first stderr byte took {ttff:?}, budget {TTFF_BUDGET:?}.\n\
         If this regression is intentional, raise the budget *with the \
         spec rationale*; otherwise investigate which step now runs \
         before the receipt banner in `run_compose_inner`.",
    );
    let received = String::from_utf8_lossy(&buf[..n]);
    assert!(
        received.contains("→ Composing")
            || received.contains("Composing"),
        "first stderr chunk should be the receipt banner; got: {received:?}",
    );
}
