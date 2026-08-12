//! Performance harness for `claudine __complete` and ENTER-path autocomplete.
//!
//! Phase 6 of the `2026-04-24-improved-shell-completions` feature and
//! Phase 5 of the `2026-06-14-auto-complete` feature. The harness builds
//! a Cargo workspace fixture that mirrors rusty-biscuit scale (~48
//! packages, ~2000 markdown files), warms the OS file cache, then runs
//! the completion engine `ITERATIONS` times against three representative
//! TAB cursor slots:
//!
//! - `claudine compose <TAB>` (empty partial, repo-prompts only)
//! - `claudine compose pla<TAB>` (3-char partial, fuzzy + dirs)
//! - `claudine inline-compose <TAB>` (extras: `docs/` + skill peers)
//!
//! For each slot the harness records p50, p95 and p99 wall-clock latency
//! and prints a single-line summary on stdout. The pass criterion comes
//! from spec §8.1: **p95 ≤ 100 ms** on the reference monorepo.
//!
//! The harness is `#[ignore]`d so `just test` does not pay the fixture
//! cost on every run. Invoke explicitly:
//!
//! ```sh
//! cargo test -p claudine-cli --test completion_perf -- --ignored --nocapture
//! ```
//!
//! Optionally set `CLAUDINE_COMPLETION_PROFILE=1` and
//! `RUST_LOG=claudine::completion=trace` to capture per-phase timing
//! spans during the harness run. The harness itself does not enable
//! these — they are picked up by the spawned `claudine` binary if set in
//! the parent environment.
//!
//! ## ENTER-path autocomplete latency
//!
//! On Unix the harness also measures the runtime autocomplete path that
//! fires when `claudine compose|inline-compose|sequence <partial>` fails
//! to resolve the file reference. A PTY-spawned `claudine compose pla`
//! enters the two-pane chooser; the test cancels with `Esc` and records
//! the wall-clock time from spawn to visible candidate list. This shares
//! the same bounded walker as the TAB path, so the p95 target is the
//! same ~100 ms-class budget.

use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};


#[cfg(unix)]
use expectrl::{Expect, Session};

mod common;
use common::TestWorkspace;
use common::completion::{fake_home, write_file as write};

/// Number of timed iterations per scenario.
///
/// Kept modest because each iteration spawns a fresh `claudine` process —
/// the goal is a representative p95, not a saturating microbenchmark.
const ITERATIONS: usize = 50;

/// Number of warmup iterations discarded before timing begins. Warms the
/// OS page cache for the fixture directories so first-run filesystem cost
/// does not skew the p50.
const WARMUP: usize = 5;

/// Pass criterion from spec §8.1.
const TARGET_P95_MS: u128 = 100;

/// Trigger criterion from spec §8.3 — if p95 exceeds this, Phase 6
/// implements the fallback cache.
const CACHE_TRIGGER_MS: u128 = 150;

// ---------------------------------------------------------------------
// Fixture construction — Cargo workspace mirroring rusty-biscuit scale.
// ---------------------------------------------------------------------

const PACKAGES: usize = 48;
const PROMPTS_PER_PKG: usize = 12;
const DOCS_PER_PKG: usize = 12;
const REPO_PROMPTS: usize = 60;
const REPO_DOCS: usize = 60;
const SKILL_FILES: usize = 80;

fn seed_workspace(root: &Path) {
    fs::create_dir_all(root.join(".git")).unwrap();

    let members: Vec<String> = (0..PACKAGES).map(|i| format!("pkg{i:02}/lib")).collect();
    let members_list = members
        .iter()
        .map(|m| format!("    \"{m}\""))
        .collect::<Vec<_>>()
        .join(",\n");
    write(
        &root.join("Cargo.toml"),
        &format!("[workspace]\nresolver = \"2\"\nmembers = [\n{members_list}\n]\n"),
    );

    for (i, member) in members.iter().enumerate() {
        let pkg_dir = root.join(member);
        fs::create_dir_all(pkg_dir.join("src")).unwrap();
        write(
            &pkg_dir.join("Cargo.toml"),
            &format!("[package]\nname = \"pkg{i:02}\"\nversion = \"0.0.0\"\nedition = \"2024\"\n"),
        );
        write(&pkg_dir.join("src").join("lib.rs"), "");

        for j in 0..PROMPTS_PER_PKG {
            write(
                &pkg_dir.join("prompts").join(format!("plan-{j:02}.md")),
                "---\ntitle: Sample\n---\nBody.\n",
            );
        }
        let pkg_area = root.join(format!("pkg{i:02}"));
        for kind in &["docs", "features", "fixes", "reviews"] {
            for j in 0..(DOCS_PER_PKG / 4) {
                write(
                    &pkg_area.join(kind).join(format!("note-{j:02}.md")),
                    "# note\n",
                );
            }
        }
    }

    for j in 0..REPO_PROMPTS {
        write(
            &root.join("prompts").join(format!("repo-prompt-{j:02}.md")),
            "---\ntitle: Repo Prompt\n---\nBody.\n",
        );
    }

    for j in 0..REPO_DOCS {
        write(&root.join("docs").join(format!("doc-{j:02}.md")), "# doc\n");
    }

    let skill_dirs = [
        ".claude/skills",
        ".codex/skills",
        ".gemini/skills",
        ".opencode/skills",
        ".goose/skills",
        ".qwen/skills",
        ".kimi/skills",
    ];
    for dir in skill_dirs {
        for j in 0..(SKILL_FILES / skill_dirs.len()) {
            write(
                &root
                    .join(dir)
                    .join(format!("skill-{j:02}"))
                    .join("SKILL.md"),
                "---\nprompt: Use this skill.\n---\nBody.\n",
            );
        }
    }

    write(
        &root.join(".claudine").join("prompts").join("admin.md"),
        "---\ntitle: Admin\n---\n",
    );
}

// ---------------------------------------------------------------------
// Timed invocation helpers.
// ---------------------------------------------------------------------

fn run_complete_once(cwd: &Path, home: &Path, argv_tail: &[&str]) -> Duration {
    let current = argv_tail.len();
    let reference = assert_cmd::Command::cargo_bin("claudine").unwrap();
    let program = reference.get_program().to_os_string();
    let mut cmd = Command::new(program);
    cmd.current_dir(cwd)
        .env("HOME", home)
        .env("NO_COLOR", "1")
        .env_remove("COMPLETE")
        .env_remove("_CLAP_COMPLETE_INDEX")
        .env_remove("_CLAP_IFS")
        .arg("__complete")
        .arg("--current")
        .arg(current.to_string())
        .arg("--")
        .arg("claudine");
    for arg in argv_tail {
        cmd.arg(arg);
    }
    let started = Instant::now();
    let output = cmd.output().expect("completion subprocess to run");
    let elapsed = started.elapsed();
    assert!(
        output.status.success(),
        "completion subprocess failed: status={:?}, stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr),
    );
    elapsed
}

#[derive(Debug, Clone, Copy)]
struct Stats {
    p50: u128,
    p95: u128,
    p99: u128,
    max: u128,
    mean: u128,
}

fn compute_stats(samples: &[Duration]) -> Stats {
    let mut millis: Vec<u128> = samples.iter().map(Duration::as_millis).collect();
    millis.sort_unstable();
    let p = |q: f64| -> u128 {
        let idx = ((millis.len() as f64 - 1.0) * q).round() as usize;
        millis[idx]
    };
    let mean = millis.iter().sum::<u128>() / millis.len() as u128;
    Stats {
        p50: p(0.50),
        p95: p(0.95),
        p99: p(0.99),
        max: *millis.last().unwrap(),
        mean,
    }
}

fn measure(label: &str, cwd: &Path, home: &Path, argv_tail: &[&str]) -> Stats {
    for _ in 0..WARMUP {
        let _ = run_complete_once(cwd, home, argv_tail);
    }
    let samples: Vec<Duration> = (0..ITERATIONS)
        .map(|_| run_complete_once(cwd, home, argv_tail))
        .collect();
    let stats = compute_stats(&samples);
    println!(
        "[perf] {label:32} mean={mean}ms p50={p50}ms p95={p95}ms p99={p99}ms max={max}ms (n={n})",
        mean = stats.mean,
        p50 = stats.p50,
        p95 = stats.p95,
        p99 = stats.p99,
        max = stats.max,
        n = ITERATIONS,
    );
    stats
}

// ---------------------------------------------------------------------
// Test entry points.
// ---------------------------------------------------------------------

#[test]
#[ignore = "performance harness; run explicitly with --ignored"]
fn perf_compose_empty_partial_meets_target() {
    let ws = TestWorkspace::named("complete-perf-compose-empty");
    seed_workspace(ws.path());
    let home = fake_home(ws.path());

    let stats = measure("compose empty partial", ws.path(), &home, &["compose", ""]);
    assert_perf_target("compose empty partial", stats);
}

#[test]
#[ignore = "performance harness; run explicitly with --ignored"]
fn perf_compose_long_prefix_meets_target() {
    let ws = TestWorkspace::named("complete-perf-compose-long");
    seed_workspace(ws.path());
    let home = fake_home(ws.path());

    let stats = measure(
        "compose long prefix `pla`",
        ws.path(),
        &home,
        &["compose", "pla"],
    );
    assert_perf_target("compose long prefix `pla`", stats);
}

#[test]
#[ignore = "performance harness; run explicitly with --ignored"]
fn perf_inline_compose_empty_partial_meets_target() {
    let ws = TestWorkspace::named("complete-perf-inline-empty");
    seed_workspace(ws.path());
    let home = fake_home(ws.path());

    let stats = measure(
        "inline-compose empty partial",
        ws.path(),
        &home,
        &["inline-compose", ""],
    );
    assert_perf_target("inline-compose empty partial", stats);
}

/// Validate the spec target against the recorded p95.
///
/// The function classifies the run into one of two regions:
///
/// - **Pass** — `p95 ≤ 100 ms`. The default no-cache plan holds.
/// - **Fail** — `p95 > 100 ms`. The harness panics so the regression is
///   visible in CI. When `p95` additionally exceeds the `150 ms` cache
///   trigger, the panic message notes that the fallback cache from
///   spec §8.3 should be implemented.
fn assert_perf_target(label: &str, stats: Stats) {
    if stats.p95 <= TARGET_P95_MS {
        println!(
            "[perf] {label}: PASS (p95 {} ms ≤ {} ms)",
            stats.p95, TARGET_P95_MS
        );
        return;
    }
    if stats.p95 > CACHE_TRIGGER_MS {
        panic!(
            "[perf] {label}: FAIL (p95 {} ms > {} ms target; also exceeds {} ms cache trigger — implement fallback cache per spec §8.3)",
            stats.p95, TARGET_P95_MS, CACHE_TRIGGER_MS,
        );
    }
    panic!(
        "[perf] {label}: FAIL (p95 {} ms > {} ms target)",
        stats.p95, TARGET_P95_MS,
    );
}

// ----------------------------------------------------------------------
// ENTER-path autocomplete latency (Unix-only; requires a PTY)
// ----------------------------------------------------------------------

/// Number of timed iterations for the ENTER-path scenario.
///
/// Kept lower than the TAB path because each iteration must spawn a PTY,
/// wait for the `biscuit-tui` chooser to render, and cancel with `Esc`.
#[cfg(unix)]
const ENTER_ITERATIONS: usize = 20;

#[cfg(unix)]
const ENTER_WARMUP: usize = 3;

/// Run one ENTER-path autocomplete iteration and return the wall-clock
/// time from PTY spawn until the chooser's candidate list is visible.
///
/// The partial `pla` is intentionally non-resolving so
/// `resolve_composition_source` falls through to
/// `autocomplete_operation_file`. The function cancels the chooser with
/// `Esc` as soon as a candidate path appears, so provider resolution and
/// composition execution never run.
#[cfg(unix)]
fn run_enter_once(cwd: &Path, home: &Path, partial: &str) -> Duration {
    // The init wizard intercepts stdin when no user config exists; stage
    // an empty config so the default `prompt_for_missing = true` applies.
    let config_dir = home.join(".claudine");
    fs::create_dir_all(&config_dir).unwrap();
    fs::write(config_dir.join("config.json"), "{}").unwrap();

    let program = assert_cmd::Command::cargo_bin("claudine").unwrap().get_program().to_os_string();
    let mut cmd = Command::new(program);
    cmd.current_dir(cwd)
        .env("HOME", home)
        .env("NO_COLOR", "1")
        .env_remove("COMPLETE")
        .env_remove("_CLAP_COMPLETE_INDEX")
        .env_remove("_CLAP_IFS")
        .args(["compose", partial]);

    let mut session = Session::spawn(cmd).expect("PTY session for ENTER path");
    let started = Instant::now();

    // Drain until the chooser renders a candidate path. The partial `repo`
    // matches the repo-root `repo-prompt-NN.md` files, so the rendered
    // candidate labels contain `repo-prompt`.
    let marker = "repo-prompt";
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut buf = Vec::new();
    let mut scratch = [0u8; 4096];
    loop {
        if Instant::now() > deadline {
            let text = String::from_utf8_lossy(&buf);
            panic!(
                "ENTER-path chooser did not render within 10 s; captured {} bytes:\n{text}",
                buf.len()
            );
        }
        match session.try_read(&mut scratch) {
            Ok(0) => {
                // No data yet; keep polling. EOF only happens when the
                // child exits, which should not occur before we cancel.
            }
            Ok(n) => {
                buf.extend_from_slice(&scratch[..n]);
                let text = String::from_utf8_lossy(&buf);
                if text.contains(marker) {
                    break;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(_) => break,
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let elapsed = started.elapsed();

    // Cancel the chooser so the process exits cleanly. In raw mode Esc is
    // the single ESC byte; the chooser returns `AutocompleteCancelled` and
    // the CLI renders the non-TTY remediation block and exits.
    session.send(b"\x1b").ok();
    let _ = session.expect(expectrl::Eof);

    elapsed
}

#[cfg(unix)]
fn measure_enter(label: &str, cwd: &Path, home: &Path, partial: &str) -> Stats {
    for _ in 0..ENTER_WARMUP {
        let _ = run_enter_once(cwd, home, partial);
    }
    let samples: Vec<Duration> = (0..ENTER_ITERATIONS)
        .map(|_| run_enter_once(cwd, home, partial))
        .collect();
    let stats = compute_stats(&samples);
    println!(
        "[perf] {label:32} mean={mean}ms p50={p50}ms p95={p95}ms p99={p99}ms max={max}ms (n={n})",
        mean = stats.mean,
        p50 = stats.p50,
        p95 = stats.p95,
        p99 = stats.p99,
        max = stats.max,
        n = ENTER_ITERATIONS,
    );
    stats
}

#[cfg(unix)]
#[test]
#[ignore = "performance harness; run explicitly with --ignored"]
fn perf_enter_compose_partial_meets_target() {
    let ws = TestWorkspace::named("enter-autocomplete-perf-compose");
    seed_workspace(ws.path());
    let home = fake_home(ws.path());

    let stats = measure_enter(
        "enter compose partial `repo`",
        ws.path(),
        &home,
        "repo",
    );
    assert_perf_target("enter compose partial `repo`", stats);
}

#[cfg(not(unix))]
#[test]
#[ignore = "performance harness; run explicitly with --ignored"]
fn perf_enter_compose_partial_meets_target() {
    // The ENTER-path latency scenario requires a PTY, which is not
    // available in a portable way on Windows. The TAB-path scenarios
    // above still run on every platform.
}

// ----------------------------------------------------------------------
// assert_perf_target contract — fast, platform-agnostic regression tests.
//
// These build `Stats` directly and drive `assert_perf_target` without
// spawning any subprocess, so they run in the default (non-ignored)
// suite. They pin the spec §8.1 p95 budget: any p95 over 100 ms fails,
// which closes the loophole that let a 149 ms run pass as a "warning".
// ----------------------------------------------------------------------

fn stats_with_p95(p95: u128) -> Stats {
    Stats {
        p50: p95,
        p95,
        p99: p95,
        max: p95,
        mean: p95,
    }
}

fn assert_panics(p95: u128) {
    let result = std::panic::catch_unwind(|| {
        assert_perf_target("probe", stats_with_p95(p95));
    });
    assert!(result.is_err(), "expected p95={} ms to fail the target", p95);
}

fn assert_passes(p95: u128) {
    let result = std::panic::catch_unwind(|| {
        assert_perf_target("probe", stats_with_p95(p95));
    });
    assert!(result.is_ok(), "expected p95={} ms to pass the target", p95);
}

#[test]
fn assert_perf_target_passes_well_under_target() {
    assert_passes(50);
}

#[test]
fn assert_perf_target_passes_at_target_boundary() {
    // The budget is inclusive: p95 == TARGET_P95_MS must still pass.
    assert_passes(TARGET_P95_MS);
}

#[test]
fn assert_perf_target_fails_just_over_target() {
    assert_panics(101);
}

#[test]
fn assert_perf_target_fails_in_old_warning_range() {
    // p95 = 149 ms used to fall in the passing "warning" region; it must
    // now fail. This is the exact regression the review called out.
    assert_panics(149);
}

#[test]
fn assert_perf_target_fails_over_cache_trigger_with_diagnostic() {
    let result = std::panic::catch_unwind(|| {
        assert_perf_target("probe", stats_with_p95(151));
    });
    let payload = result.unwrap_err();
    let msg = payload
        .downcast_ref::<String>()
        .map(|s| s.as_str())
        .or_else(|| payload.downcast_ref::<&'static str>().copied())
        .unwrap_or("");
    assert!(
        msg.contains("100"),
        "failure msg should mention the 100ms target: {msg}"
    );
    assert!(
        msg.contains("cache") || msg.contains("§8.3"),
        "failure msg should mention the cache fallback: {msg}"
    );
}
