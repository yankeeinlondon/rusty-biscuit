//! Performance harness for `claudine __complete`.
//!
//! Phase 6 of the `2026-04-24-improved-shell-completions` feature. The
//! harness builds a Cargo workspace fixture that mirrors rusty-biscuit
//! scale (~48 packages, ~2000 markdown files), warms the OS file cache,
//! then runs the completion engine `ITERATIONS` times against three
//! representative cursor slots:
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
//! `RUST_LOG=claudine::completion=trace` to capture per-phase tracing
//! spans during the harness run. The harness itself does not enable
//! these — they are picked up by the spawned `claudine` binary if set in
//! the parent environment.

use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

use assert_cmd::cargo::cargo_bin_cmd;

mod common;
use common::TestWorkspace;

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

fn write(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

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

fn fake_home(root: &Path) -> std::path::PathBuf {
    let home = root.parent().unwrap_or(root).join(format!(
        "{}-home-{}",
        root.file_name().and_then(|n| n.to_str()).unwrap_or("perf"),
        std::process::id(),
    ));
    fs::create_dir_all(&home).unwrap();
    home
}

fn run_complete_once(cwd: &Path, home: &Path, argv_tail: &[&str]) -> Duration {
    let current = argv_tail.len();
    let reference = cargo_bin_cmd!("claudine");
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
/// The function classifies the run into one of three regions:
///
/// - **Pass** — `p95 ≤ 100 ms`. The default no-cache plan holds.
/// - **Warning** — `100 ms < p95 ≤ 150 ms`. Outside the spec target but
///   inside the cache trigger; emit a warning so the team can decide
///   whether to tune the no-cache path further.
/// - **Cache trigger** — `p95 > 150 ms`. The harness panics so a CI run
///   forces the team to land the fallback cache from spec §8.3.
fn assert_perf_target(label: &str, stats: Stats) {
    if stats.p95 <= TARGET_P95_MS {
        println!(
            "[perf] {label}: PASS (p95 {} ms ≤ {} ms)",
            stats.p95, TARGET_P95_MS
        );
        return;
    }
    if stats.p95 <= CACHE_TRIGGER_MS {
        println!(
            "[perf] {label}: WARN (p95 {} ms > {} ms target but ≤ {} ms cache trigger)",
            stats.p95, TARGET_P95_MS, CACHE_TRIGGER_MS,
        );
        return;
    }
    panic!(
        "[perf] {label}: FAIL (p95 {} ms > {} ms cache trigger — implement cache.rs per spec §8.3)",
        stats.p95, CACHE_TRIGGER_MS,
    );
}
