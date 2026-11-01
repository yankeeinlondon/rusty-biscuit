//! Record and report the tests Nextest runs past the workspace's slow threshold.
//!
//! - `record` reads one Nextest JUnit report and appends a line to the slow-test
//!   log for every test whose duration reached the threshold. The canonical
//!   test recipes call it after each Nextest invocation (`just/devops.just`).
//!   It also appends the invocation to `target/slow-tests/runs.jsonl` under
//!   the working tree's state (`sniff`'s `WorkingState::id`: `HEAD` plus the
//!   content of every changed path) and rebuilds `target/slow-tests/current.md`
//!   from every run of that state, written even when nothing was slow, which
//!   `prompts/slow.md` reads. Running `just test` and then `just sanity` in an
//!   unchanged tree therefore leaves both tiers' slow tests in one summary.
//! - `import` downloads a GitHub Actions run's `junit-*` artifacts and records
//!   them in the same log, so hosted-runner timings sit beside local ones.
//! - `report` prints the log grouped by test, slowest first, with each test's
//!   previous duration and the change between them.
//! - `fresh` and `verify` are the checks `prompts/slow.md` runs from its
//!   hooks: whether `current.md` still describes the working tree, and whether
//!   every scope of a baseline state was re-run in a later state with no slow
//!   test left.
//!
//! The threshold is `[profile.default] slow-timeout.period` in
//! `.config/nextest.toml`, for every profile and every source, so a local
//! `local-evidence` run and an imported CI run (whose own profile marks SLOW at
//! 30s) are measured against one number. A test with an override that raises its
//! own period is still recorded once it passes the base threshold.
//!
//! The log is JSON Lines at
//! `~/.rusty-biscuit/test-timings/<host>/<owner>/<repo>/slow-tests.jsonl`, keyed
//! on the `origin` remote so every worktree and clone of one repository shares
//! it and `cargo clean` does not erase it. `BISCUIT_SLOW_TESTS_LOG` names a file
//! instead. See `docs/testing-strategy.md` ("Slow-test log").

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, bail, Context, Result};
use biscuit_terminal::prelude::{Table, TableColumn, Terminal, TerminalRenderable, WordWrap};
use quick_xml::events::Event;
use quick_xml::Reader;
use serde::{Deserialize, Serialize};
use sniff::filesystem::git::{GitRepo, WorkingState};

const LOG_ENV: &str = "BISCUIT_SLOW_TESTS_LOG";

/// One `Run` per recorded Nextest invocation in this checkout, as JSON Lines,
/// relative to the workspace root.
const RUNS: &str = "target/slow-tests/runs.jsonl";

/// Every run of the current working-tree state, merged (see `current_document`).
const CURRENT: &str = "target/slow-tests/current.md";

/// Groups every Nextest invocation one recipe makes into one run of a scope.
const GROUP_ENV: &str = "BISCUIT_SLOW_TESTS_GROUP";

/// One slow test observation: one line of the log.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct SlowTest {
    /// Unix seconds: when a local run was recorded, or when a CI run started.
    recorded_at: u64,
    /// `local` or `ci`.
    source: String,
    /// Groups the records of one Nextest run: the report's run UUID locally,
    /// `gh:<run id>/<attempt>` for an imported CI run.
    run: String,
    /// The OS name (`windows`, `macos`, `linux`) locally; the CI environment
    /// (`ubuntu-latest`, ...) for an imported run.
    environment: String,
    /// The local host name, or `github` for an imported run.
    host: String,
    commit: String,
    tier: String,
    package: String,
    binary_id: String,
    test: String,
    duration_s: f64,
    threshold_s: f64,
    /// `pass`, `fail`, `flaky`, or `skip`.
    outcome: String,
}

/// One `<testcase>` of a Nextest JUnit report.
#[derive(Debug, PartialEq)]
struct Case {
    binary_id: String,
    test: String,
    duration_s: f64,
    outcome: &'static str,
}

#[derive(Debug, Default)]
struct Junit {
    /// The `<testsuites uuid>` Nextest stamps on each run, when present.
    run_uuid: Option<String>,
    cases: Vec<Case>,
}

/// Where a batch of records came from; every field but the per-test ones.
struct Origin {
    recorded_at: u64,
    source: String,
    run: String,
    environment: String,
    host: String,
    commit: String,
    tier: String,
}

fn main() -> ExitCode {
    match run(std::env::args().skip(1).collect()) {
        Ok(code) => code,
        Err(err) => {
            eprintln!("slow-tests: {err:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: Vec<String>) -> Result<ExitCode> {
    let Some((command, rest)) = args.split_first() else {
        bail!("usage: slow-tests <record|import|report|fresh|verify> [options]");
    };
    let options = Options::parse(rest)?;
    let done = |result: Result<()>| result.map(|()| ExitCode::SUCCESS);
    match command.as_str() {
        "record" => done(record(&options)),
        "import" => done(import(&options)),
        "report" => done(report(&options)),
        "fresh" => fresh(),
        "verify" => verify(&options),
        other => bail!("unknown subcommand `{other}`; expected record, import, report, fresh, or verify"),
    }
}

/// Flags shared by the subcommands; each reads the ones it needs.
#[derive(Debug, Default)]
struct Options {
    junit: Option<PathBuf>,
    tier: Option<String>,
    threshold: Option<f64>,
    log: Option<PathBuf>,
    area: Option<PathBuf>,
    packages: Vec<String>,
    environment: Option<String>,
    limit: Option<usize>,
    repo: Option<String>,
    runs: Vec<String>,
    baseline: Option<String>,
}

impl Options {
    fn parse(args: &[String]) -> Result<Self> {
        let mut options = Options::default();
        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            let mut value = |flag: &str| {
                iter.next()
                    .cloned()
                    .ok_or_else(|| anyhow!("{flag} needs a value"))
            };
            match arg.as_str() {
                "--junit" => options.junit = Some(value(arg)?.into()),
                "--tier" => options.tier = Some(value(arg)?),
                "--threshold" => {
                    options.threshold = Some(value(arg)?.parse().context("--threshold takes seconds")?)
                }
                "--log" => options.log = Some(value(arg)?.into()),
                "--area" => options.area = Some(value(arg)?.into()),
                "--package" | "-p" => options.packages.push(value(arg)?),
                "--env" => options.environment = Some(value(arg)?),
                "--limit" => options.limit = Some(value(arg)?.parse().context("--limit takes a count")?),
                "--repo" => options.repo = Some(value(arg)?),
                "--baseline" => options.baseline = Some(value(arg)?),
                flag if flag.starts_with('-') => bail!("unknown option `{flag}`"),
                run_id => options.runs.push(run_id.to_owned()),
            }
        }
        Ok(options)
    }
}

// ---------------------------------------------------------------------------
// record

fn record(options: &Options) -> Result<()> {
    let junit_path = options.junit.as_deref().ok_or_else(|| anyhow!("record needs --junit <report>"))?;
    let tier = options.tier.clone().ok_or_else(|| anyhow!("record needs --tier <tier>"))?;
    let root = workspace_root()?;
    let threshold = resolve_threshold(options, &root)?;
    let xml = fs::read_to_string(junit_path).with_context(|| format!("reading {}", junit_path.display()))?;
    let junit = parse_junit(&xml)?;
    let recorded_at = now();
    let state = working_state(&root);
    let commit = match &state {
        Ok(state) => state.head.clone().unwrap_or_default(),
        Err(_) => git(&root, &["rev-parse", "HEAD"]).unwrap_or_default(),
    };
    let origin = Origin {
        recorded_at,
        source: "local".into(),
        run: junit.run_uuid.clone().unwrap_or_else(|| format!("local:{recorded_at}")),
        environment: std::env::consts::OS.into(),
        host: host_name(),
        commit,
        tier,
    };
    let slow = slow_records(&junit, &origin, threshold);
    let log = log_path(options, &root)?;
    append(&log, &slow)?;

    match state {
        Ok(state) => {
            let group = std::env::var(GROUP_ENV)
                .ok()
                .filter(|group| !group.is_empty())
                .unwrap_or_else(|| origin.run.clone());
            let directory = relative_directory(&root, &std::env::current_dir()?);
            let run = Run {
                state: state.id.clone(),
                recorded_at,
                group,
                tier: origin.tier.clone(),
                directory,
                packages: junit.cases.iter().map(|case| package_of(&case.binary_id).to_owned()).collect(),
                tests: junit.cases.len(),
                threshold_s: threshold,
                slow: slow.clone(),
            };
            let runs_path = root.join(RUNS);
            append(&runs_path, std::slice::from_ref(&run))?;
            let runs: Vec<Run> = read_jsonl(&runs_path)?;
            let current = root.join(CURRENT);
            fs::write(&current, current_document(&state.id, &scopes(&runs, &state.id)))
                .with_context(|| format!("writing {}", current.display()))?;
        }
        // The log keeps the timings; only the per-state summary needs the state.
        Err(err) => eprintln!("⚠️  this run is not in {CURRENT}: the working-tree state is unknown ({err:#})"),
    }

    // Said even when nothing was slow, so a clean run is visibly a recorded one.
    if slow.is_empty() {
        println!(
            "🐢 {} run recorded: {} test(s), none at or above {}s",
            origin.tier,
            junit.cases.len(),
            format_seconds(threshold)
        );
    } else {
        println!(
            "🐌 {} run recorded: {} of {} test(s) at or above {}s, added to {}",
            origin.tier,
            slow.len(),
            junit.cases.len(),
            format_seconds(threshold),
            log.display()
        );
    }
    Ok(())
}

/// The records of `junit`'s tests at or above `threshold`, slowest first.
fn slow_records(junit: &Junit, origin: &Origin, threshold: f64) -> Vec<SlowTest> {
    let mut slow: Vec<SlowTest> = junit
        .cases
        .iter()
        .filter(|case| case.duration_s >= threshold)
        .map(|case| SlowTest {
            recorded_at: origin.recorded_at,
            source: origin.source.clone(),
            run: origin.run.clone(),
            environment: origin.environment.clone(),
            host: origin.host.clone(),
            commit: origin.commit.clone(),
            tier: origin.tier.clone(),
            package: package_of(&case.binary_id).to_owned(),
            binary_id: case.binary_id.clone(),
            test: case.test.clone(),
            duration_s: (case.duration_s * 1000.0).round() / 1000.0,
            threshold_s: threshold,
            outcome: case.outcome.to_owned(),
        })
        .collect();
    slow.sort_by(|a, b| b.duration_s.total_cmp(&a.duration_s));
    slow
}

// ---------------------------------------------------------------------------
// runs by working-tree state

/// One recorded Nextest invocation in this checkout: a line of `RUNS`.
///
/// Merged by `scopes`, the same type describes one *scope*: every invocation
/// one recipe made for one tier in one directory.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct Run {
    /// `sniff::filesystem::git::WorkingState::id` when the run was recorded.
    state: String,
    recorded_at: u64,
    /// `GROUP_ENV`, or the Nextest run id when no recipe set it.
    group: String,
    tier: String,
    /// The directory the recipe ran in, relative to the workspace root and
    /// `/`-separated; `.` is the root.
    directory: String,
    packages: BTreeSet<String>,
    tests: usize,
    threshold_s: f64,
    slow: Vec<SlowTest>,
}

/// The scopes recorded in `state`, by tier and then directory.
///
/// A scope is the latest group recorded for one tier in one directory, so
/// re-running `just test` in an unchanged tree replaces that scope's results
/// instead of adding to them. `sanity`, `test-l2`, and the other per-package
/// tiers run Nextest once per package under one group, and those invocations
/// merge into one scope.
fn scopes(runs: &[Run], state: &str) -> Vec<Run> {
    let mut latest: BTreeMap<(&str, &str), (u64, &str)> = BTreeMap::new();
    for run in runs.iter().filter(|run| run.state == state) {
        let entry = latest
            .entry((run.tier.as_str(), run.directory.as_str()))
            .or_insert((run.recorded_at, run.group.as_str()));
        if run.recorded_at >= entry.0 {
            *entry = (run.recorded_at, run.group.as_str());
        }
    }
    latest
        .into_iter()
        .map(|((tier, directory), (_, group))| {
            let mut merged: Option<Run> = None;
            for run in runs.iter().filter(|run| {
                run.state == state && run.tier == tier && run.directory == directory && run.group == group
            }) {
                match &mut merged {
                    None => merged = Some(run.clone()),
                    Some(scope) => {
                        scope.recorded_at = scope.recorded_at.max(run.recorded_at);
                        scope.packages.extend(run.packages.iter().cloned());
                        scope.tests += run.tests;
                        scope.slow.extend(run.slow.iter().cloned());
                    }
                }
            }
            let mut scope = merged.expect("every key came from a run");
            scope.slow.sort_by(|a, b| b.duration_s.total_cmp(&a.duration_s));
            scope
        })
        .collect()
}

/// The canonical recipe that re-runs `scope`, from the directory it ran in.
///
/// An area recipe covers the area's packages; at the workspace root `just
/// test` takes the packages to run, so they are named.
fn rerun_command(scope: &Run) -> String {
    let recipe = match scope.tier.as_str() {
        "L1" => "test",
        "L2" => "test-l2",
        "L3" => "test-l3",
        "browser" => "test-browser",
        "real" => "test-real",
        other => other,
    };
    if scope.directory == "." {
        let packages: Vec<&str> = scope.packages.iter().map(String::as_str).collect();
        format!("just {recipe} {}", packages.join(" ")).trim_end().to_owned()
    } else {
        format!("cd {} && just {recipe}", scope.directory)
    }
}

/// A Markdown rendering of every scope recorded in `state`, slow tests or not.
///
/// Its frontmatter is a contract: `prompts/slow.md` reads `state` and `slow`
/// from `initialize`, where no shell can run, and records `state` as the
/// baseline that `verify` later proves was re-run.
fn current_document(state: &str, scopes: &[Run]) -> String {
    let quote = |text: &str| serde_json::to_string(text).unwrap_or_default();
    let tiers: BTreeSet<&str> = scopes.iter().map(|scope| scope.tier.as_str()).collect();
    let tests: usize = scopes.iter().map(|scope| scope.tests).sum();
    let slow: usize = scopes.iter().map(|scope| scope.slow.len()).sum();
    let recorded_at = scopes.iter().map(|scope| scope.recorded_at).max().unwrap_or(0);
    let threshold = scopes.iter().map(|scope| scope.threshold_s).fold(0.0, f64::max);
    let mut out = format!(
        "---\nstate: {}\nrecorded: {}\nscopes: {}\ntiers: {}\ntests: {}\nslow: {}\nthreshold_s: {}\n---\n\n",
        quote(state),
        quote(&format_timestamp(recorded_at)),
        scopes.len(),
        quote(&tiers.into_iter().collect::<Vec<_>>().join(", ")),
        tests,
        slow,
        threshold,
    );
    out.push_str(&format!(
        "Every test run recorded while the working tree was in state `{state}`: one section per tier and directory, the latest run of each.\n"
    ));
    for scope in scopes {
        let packages: Vec<&str> = scope.packages.iter().map(String::as_str).collect();
        out.push_str(&format!(
            "\n## {} in `{}`\n\n- Packages: {}\n- Re-run: `{}`\n- Recorded: {}, {} test(s), {} at or above {}s\n\n",
            scope.tier,
            scope.directory,
            packages.join(", "),
            rerun_command(scope),
            format_timestamp(scope.recorded_at),
            scope.tests,
            scope.slow.len(),
            format_seconds(scope.threshold_s),
        ));
        if scope.slow.is_empty() {
            out.push_str("No test in this scope was slow.\n");
            continue;
        }
        out.push_str("| Test | Binary | Duration | Outcome |\n|---|---|---|---|\n");
        for record in &scope.slow {
            out.push_str(&format!(
                "| `{}` | `{}` | {}s | {} |\n",
                record.test.replace('|', "\\|"),
                record.binary_id,
                format_seconds(record.duration_s),
                record.outcome
            ));
        }
    }
    out
}

/// One line describing `CURRENT`, if this checkout has one.
fn current_line(root: &Path) -> Option<String> {
    let fields = frontmatter_fields(&root.join(CURRENT))?;
    let field = |key: &str| fields.get(key).map(String::as_str).unwrap_or("?");
    Some(format!(
        "Runs of working-tree state {} in this checkout: {} ({} scope(s)), {} test(s), {} at or above {}s.",
        field("state"),
        field("tiers"),
        field("scopes"),
        field("tests"),
        field("slow"),
        field("threshold_s"),
    ))
}

/// The flat `key: value` frontmatter of a document this tool wrote.
fn frontmatter_fields(path: &Path) -> Option<BTreeMap<String, String>> {
    let text = fs::read_to_string(path).ok()?;
    let text = text.replace("\r\n", "\n");
    let frontmatter = text.strip_prefix("---\n")?.split("\n---").next()?;
    Some(
        frontmatter
            .lines()
            .filter_map(|line| line.split_once(": "))
            .map(|(key, value)| {
                let value = serde_json::from_str::<String>(value).unwrap_or_else(|_| value.to_owned());
                (key.to_owned(), value)
            })
            .collect(),
    )
}

/// `directory` relative to `root`, `/`-separated; `.` for the root itself or
/// a directory outside it.
fn relative_directory(root: &Path, directory: &Path) -> String {
    let canonical = |path: &Path| normalize(&fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf()));
    let (root, directory) = (canonical(root), canonical(directory));
    match directory.strip_prefix(&root).map(|rest| rest.trim_matches('/')) {
        Some(rest) if !rest.is_empty() => rest.to_owned(),
        _ => ".".into(),
    }
}

// ---------------------------------------------------------------------------
// fresh / verify: the checks `prompts/slow.md` runs from its hooks

/// Exit 0 when `CURRENT` describes the working tree as it is now.
fn fresh() -> Result<ExitCode> {
    let root = workspace_root()?;
    let live = working_state(&root)?;
    let recorded = frontmatter_fields(&root.join(CURRENT)).and_then(|fields| fields.get("state").cloned());
    let (verdict, code) = freshness(recorded.as_deref(), &live.id);
    println!("{verdict}");
    Ok(code)
}

fn freshness(recorded: Option<&str>, live: &str) -> (String, ExitCode) {
    match recorded {
        None => (format!("no test run is recorded in {CURRENT}; run a test recipe first"), ExitCode::FAILURE),
        Some(recorded) if recorded == live => (
            format!("fresh: {CURRENT} describes the working tree (state {live})"),
            ExitCode::SUCCESS,
        ),
        Some(recorded) => (
            format!(
                "stale: {CURRENT} describes state {recorded}, but the working tree has changed since (now {live}); re-run the tests"
            ),
            ExitCode::FAILURE,
        ),
    }
}

/// Exit 0 when every scope recorded in the baseline state was re-run, over
/// at least the same packages, in the working tree's current (different)
/// state, and no scope of the current state has a slow test.
fn verify(options: &Options) -> Result<ExitCode> {
    let baseline = options.baseline.as_deref().ok_or_else(|| anyhow!("verify needs --baseline <state>"))?;
    let root = workspace_root()?;
    let live = working_state(&root)?;
    let runs: Vec<Run> = read_jsonl(&root.join(RUNS))?;
    let problems = proof_problems(&runs, baseline, &live.id);
    if problems.is_empty() {
        println!(
            "proven: every scope of state {baseline} re-ran in the current state {} with no slow tests",
            live.id
        );
        return Ok(ExitCode::SUCCESS);
    }
    println!("not proven:");
    for problem in problems {
        println!("- {problem}");
    }
    Ok(ExitCode::FAILURE)
}

fn proof_problems(runs: &[Run], baseline: &str, live: &str) -> Vec<String> {
    let before = scopes(runs, baseline);
    if before.is_empty() {
        return vec![format!("no run of the baseline state {baseline} is recorded in {RUNS}")];
    }
    if live == baseline {
        return vec![format!(
            "the working tree is unchanged since the baseline (state {live}), so nothing has been fixed"
        )];
    }
    let after = scopes(runs, live);
    let mut problems = Vec::new();
    for scope in &before {
        let label = format!("{} in `{}`", scope.tier, scope.directory);
        match after
            .iter()
            .find(|candidate| candidate.tier == scope.tier && candidate.directory == scope.directory)
        {
            None => problems.push(format!(
                "{label} has not run since the working tree last changed: `{}`",
                rerun_command(scope)
            )),
            Some(rerun) => {
                let missing: Vec<&str> = scope.packages.difference(&rerun.packages).map(String::as_str).collect();
                if !missing.is_empty() {
                    problems.push(format!(
                        "{label} re-ran without {}: `{}`",
                        missing.join(", "),
                        rerun_command(scope)
                    ));
                }
            }
        }
    }
    for scope in &after {
        if !scope.slow.is_empty() {
            let names: Vec<String> = scope
                .slow
                .iter()
                .map(|record| format!("{} {} ({}s)", record.binary_id, record.test, format_seconds(record.duration_s)))
                .collect();
            problems.push(format!(
                "{} in `{}` still has {} slow test(s): {}",
                scope.tier,
                scope.directory,
                scope.slow.len(),
                names.join("; ")
            ));
        }
    }
    problems
}

/// The working tree's state, through `sniff`.
fn working_state(root: &Path) -> Result<WorkingState> {
    let repo = GitRepo::discover(root)
        .with_context(|| format!("opening the Git repository at {}", root.display()))?
        .ok_or_else(|| anyhow!("{} is not in a Git repository", root.display()))?;
    repo.working_state().context("reading the working-tree state")
}

/// The package half of a Nextest binary id (`pkg`, `pkg::bin/name`, `pkg::it`).
fn package_of(binary_id: &str) -> &str {
    binary_id.split("::").next().unwrap_or(binary_id)
}

// ---------------------------------------------------------------------------
// import

/// The parts of `gh run view --json` that `import` reads.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RunView {
    head_sha: String,
    attempt: u64,
    created_at: String,
}

/// One line of a staged `manifest.jsonl` (see `_stage_junit`).
#[derive(Deserialize)]
struct ManifestRecord {
    tier: String,
    xml: String,
    environment: String,
    report_present: bool,
}

fn import(options: &Options) -> Result<()> {
    if options.runs.is_empty() {
        bail!("import needs at least one GitHub Actions run id");
    }
    let root = workspace_root()?;
    let threshold = resolve_threshold(options, &root)?;
    let log = log_path(options, &root)?;
    let existing: Vec<SlowTest> = read_jsonl(&log)?;
    for run_id in &options.runs {
        let mut view_args = vec!["run", "view", run_id.as_str(), "--json", "headSha,attempt,createdAt"];
        if let Some(repo) = &options.repo {
            view_args.extend(["--repo", repo.as_str()]);
        }
        let view: RunView = serde_json::from_str(&gh(&root, &view_args)?)
            .with_context(|| format!("parsing `gh run view {run_id}`"))?;
        let run = format!("gh:{run_id}/{}", view.attempt);
        if existing.iter().any(|record| record.run == run) {
            println!("run {run_id} (attempt {}) is already in {}", view.attempt, log.display());
            continue;
        }

        let download = std::env::temp_dir().join(format!("slow-tests-{run_id}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&download);
        let download_text = download.to_string_lossy().into_owned();
        let mut download_args = vec!["run", "download", run_id.as_str(), "--pattern", "junit-*", "--dir", download_text.as_str()];
        if let Some(repo) = &options.repo {
            download_args.extend(["--repo", repo.as_str()]);
        }
        gh(&root, &download_args)?;
        let records = records_from_artifacts(&download, &run, &view, threshold);
        let _ = fs::remove_dir_all(&download);
        let records = records?;

        append(&log, &records)?;
        println!(
            "🐌 run {run_id}: {} slow test(s) at or above {}s recorded in {}",
            records.len(),
            format_seconds(threshold),
            log.display()
        );
    }
    Ok(())
}

/// Every slow test in a directory of downloaded `junit-*` artifacts.
///
/// Each artifact is one staging tree: `manifest.jsonl` names its reports and
/// carries the environment, and the reports sit beside it.
fn records_from_artifacts(dir: &Path, run: &str, view: &RunView, threshold: f64) -> Result<Vec<SlowTest>> {
    let recorded_at = parse_rfc3339(&view.created_at).unwrap_or_else(now);
    let mut records = Vec::new();
    let mut artifacts: Vec<PathBuf> = fs::read_dir(dir)
        .with_context(|| format!("reading {}", dir.display()))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.join("manifest.jsonl").is_file())
        .collect();
    artifacts.sort();
    for artifact in artifacts {
        let manifest = fs::read_to_string(artifact.join("manifest.jsonl"))?;
        for line in manifest.lines().filter(|line| !line.trim().is_empty()) {
            let entry: ManifestRecord = serde_json::from_str(line)
                .with_context(|| format!("parsing {}", artifact.join("manifest.jsonl").display()))?;
            if !entry.report_present {
                continue;
            }
            let xml_path = artifact.join(&entry.xml);
            let Ok(xml) = fs::read_to_string(&xml_path) else {
                continue;
            };
            let junit = parse_junit(&xml).with_context(|| format!("parsing {}", xml_path.display()))?;
            let origin = Origin {
                recorded_at,
                source: "ci".into(),
                run: run.to_owned(),
                environment: entry.environment,
                host: "github".into(),
                commit: view.head_sha.clone(),
                tier: entry.tier,
            };
            records.extend(slow_records(&junit, &origin, threshold));
        }
    }
    Ok(records)
}

// ---------------------------------------------------------------------------
// report

/// One test's history in the log, for one environment.
#[derive(Debug)]
struct History<'a> {
    latest: &'a SlowTest,
    previous: Option<&'a SlowTest>,
    worst: f64,
    count: usize,
}

fn report(options: &Options) -> Result<()> {
    let root = workspace_root()?;
    let log = log_path(options, &root)?;
    let records: Vec<SlowTest> = read_jsonl(&log)?;
    let mut packages = options.packages.clone();
    if let Some(area) = &options.area {
        packages.extend(area_packages(&root, area)?);
    }
    let histories = histories(&records, &packages, options.environment.as_deref());

    let term = Terminal::new();
    let last_run = current_line(&root)
        .unwrap_or_else(|| "No run has been recorded in this checkout yet.".into());
    if histories.is_empty() {
        println!(
            "No slow tests (at or above {}s) have been recorded for this selection.",
            format_seconds(resolve_threshold(options, &root)?)
        );
        println!("{last_run}");
        println!("Log: {}", log.display());
        return Ok(());
    }
    let limit = options.limit.unwrap_or(30);
    let rows = histories
        .iter()
        .take(limit)
        .map(|history| {
            let latest = history.latest;
            vec![
                format!("{} {}", latest.binary_id, latest.test).into(),
                latest.tier.clone().into(),
                latest.environment.clone().into(),
                format!("{}s", format_seconds(latest.duration_s)).into(),
                history
                    .previous
                    .map(|previous| format!("{}s", format_seconds(previous.duration_s)))
                    .unwrap_or_else(|| "—".into())
                    .into(),
                trend(history).into(),
                format!("{}s", format_seconds(history.worst)).into(),
                history.count.to_string().into(),
                format_date(latest.recorded_at).into(),
            ]
        })
        .collect::<Vec<_>>();
    // A test path has no spaces, so the test column breaks on its separators;
    // the history columns give way before it does on a narrow terminal.
    let optional = |header: &str| TableColumn::new(header).drop_when_space_is_limited(None::<String>);
    println!(
        "{}",
        Table::new()
            .with_columns(vec![
                TableColumn::new("test").with_word_wrap(WordWrap::BespokeProse(None, vec![':', '_', '/', ' '], Some(2))),
                optional("tier"),
                TableColumn::new("env"),
                TableColumn::new("last"),
                TableColumn::new("prev"),
                TableColumn::new("change"),
                optional("worst"),
                optional("seen"),
                optional("last seen"),
            ])
            .with_data(rows)
            .render(&term)
    );
    if histories.len() > limit {
        println!("… {} more; pass --limit to see them.", histories.len() - limit);
    }
    println!("{last_run}");
    println!("Log: {}", log.display());
    Ok(())
}

/// Group `records` by environment and test, newest record first within each,
/// and order the groups by their latest duration, slowest first.
fn histories<'a>(records: &'a [SlowTest], packages: &[String], environment: Option<&str>) -> Vec<History<'a>> {
    let mut groups: BTreeMap<(&str, &str, &str), Vec<&SlowTest>> = BTreeMap::new();
    for record in records {
        if !packages.is_empty() && !packages.contains(&record.package) {
            continue;
        }
        if environment.is_some_and(|environment| environment != record.environment) {
            continue;
        }
        groups
            .entry((record.environment.as_str(), record.binary_id.as_str(), record.test.as_str()))
            .or_default()
            .push(record);
    }
    let mut histories: Vec<History<'a>> = groups
        .into_values()
        .map(|mut group| {
            group.sort_by_key(|record| std::cmp::Reverse(record.recorded_at));
            History {
                latest: group[0],
                previous: group.get(1).copied(),
                worst: group.iter().map(|record| record.duration_s).fold(0.0, f64::max),
                count: group.len(),
            }
        })
        .collect();
    histories.sort_by(|a, b| b.latest.duration_s.total_cmp(&a.latest.duration_s));
    histories
}

/// The latest duration against the previous one, as a signed percentage.
fn trend(history: &History<'_>) -> String {
    match history.previous {
        Some(previous) if previous.duration_s > 0.0 => {
            let change = (history.latest.duration_s - previous.duration_s) / previous.duration_s * 100.0;
            format!("{change:+.0}%")
        }
        _ => "new".into(),
    }
}

/// The workspace packages whose manifests sit under `area`.
fn area_packages(root: &Path, area: &Path) -> Result<Vec<String>> {
    let output = Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(root)
        .output()
        .context("running cargo metadata")?;
    if !output.status.success() {
        bail!("cargo metadata failed: {}", String::from_utf8_lossy(&output.stderr).trim());
    }
    let metadata: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let area = normalize(&fs::canonicalize(area).unwrap_or_else(|_| area.to_path_buf()));
    let packages = metadata["packages"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .filter_map(|package| {
            let manifest = Path::new(package["manifest_path"].as_str()?);
            let manifest = normalize(&fs::canonicalize(manifest).unwrap_or_else(|_| manifest.to_path_buf()));
            manifest.starts_with(&area).then(|| package["name"].as_str().map(str::to_owned))?
        })
        .collect();
    Ok(packages)
}

/// A path as a lowercase, forward-slash string, so a Windows verbatim prefix or
/// drive-letter case cannot defeat a `starts_with`.
fn normalize(path: &Path) -> String {
    let text = path.to_string_lossy().replace('\\', "/");
    let text = text.strip_prefix("//?/").unwrap_or(&text);
    if cfg!(windows) {
        text.to_lowercase()
    } else {
        text.to_owned()
    }
}

// ---------------------------------------------------------------------------
// JUnit

/// Parse a Nextest JUnit report into its test cases.
///
/// Nextest names each `<testsuite>` after its binary id and each `<testcase>`
/// after the test; `time` is the test's own wall time in seconds.
fn parse_junit(xml: &str) -> Result<Junit> {
    let mut reader = Reader::from_str(xml);
    let mut junit = Junit::default();
    let mut buf = Vec::new();
    let mut suite = String::new();
    let mut open: Option<Case> = None;
    let mut saw_root = false;
    loop {
        let event = reader
            .read_event_into(&mut buf)
            .map_err(|err| anyhow!("malformed JUnit XML at byte {}: {err}", reader.buffer_position()))?;
        match event {
            Event::Start(ref element) | Event::Empty(ref element) => {
                let empty = matches!(event, Event::Empty(_));
                match element.local_name().as_ref() {
                    b"testsuites" => {
                        saw_root = true;
                        junit.run_uuid = attribute(element, b"uuid")?;
                    }
                    b"testsuite" => suite = attribute(element, b"name")?.unwrap_or_default(),
                    b"testcase" => {
                        let binary_id = if suite.is_empty() {
                            attribute(element, b"classname")?.unwrap_or_default()
                        } else {
                            suite.clone()
                        };
                        let case = Case {
                            binary_id,
                            test: attribute(element, b"name")?.unwrap_or_default(),
                            duration_s: attribute(element, b"time")?
                                .and_then(|time| time.parse().ok())
                                .unwrap_or(0.0),
                            outcome: "pass",
                        };
                        // Nextest reports each setup script as an `@setup-script:`
                        // suite; it is not a test.
                        if case.binary_id.starts_with("@setup-script:") {
                            open = None;
                        } else if empty {
                            junit.cases.push(case);
                        } else {
                            open = Some(case);
                        }
                    }
                    b"failure" | b"error" => set_outcome(&mut open, "fail"),
                    b"flakyFailure" | b"flakyError" => set_outcome(&mut open, "flaky"),
                    b"skipped" => set_outcome(&mut open, "skip"),
                    _ => {}
                }
            }
            Event::End(ref element) if element.local_name().as_ref() == b"testcase" => {
                junit.cases.extend(open.take());
            }
            Event::Eof => break,
            _ => {}
        }
        buf.clear();
    }
    if !saw_root {
        bail!("not a JUnit document: no <testsuites> element");
    }
    Ok(junit)
}

/// A failure outranks a flake, which outranks a skip.
fn set_outcome(open: &mut Option<Case>, outcome: &'static str) {
    let rank = |outcome: &str| match outcome {
        "fail" => 3,
        "flaky" => 2,
        "skip" => 1,
        _ => 0,
    };
    if let Some(case) = open {
        if rank(outcome) > rank(case.outcome) {
            case.outcome = outcome;
        }
    }
}

fn attribute(element: &quick_xml::events::BytesStart<'_>, name: &[u8]) -> Result<Option<String>> {
    for attr in element.attributes() {
        let attr = attr?;
        if attr.key.local_name().as_ref() == name {
            return Ok(Some(attr.unescape_value()?.into_owned()));
        }
    }
    Ok(None)
}

// ---------------------------------------------------------------------------
// threshold

fn resolve_threshold(options: &Options, root: &Path) -> Result<f64> {
    if let Some(threshold) = options.threshold {
        return Ok(threshold);
    }
    let path = root.join(".config/nextest.toml");
    let text = fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    default_slow_period(&text).with_context(|| format!("reading slow-timeout from {}", path.display()))
}

/// `[profile.default] slow-timeout`'s period, in seconds. Nextest accepts both
/// `slow-timeout = "5s"` and `slow-timeout = { period = "5s", ... }`.
fn default_slow_period(config: &str) -> Result<f64> {
    let config: toml::Table = toml::from_str(config)?;
    let timeout = config
        .get("profile")
        .and_then(|profile| profile.get("default"))
        .and_then(|default| default.get("slow-timeout"))
        .ok_or_else(|| anyhow!("[profile.default] declares no slow-timeout"))?;
    let period = match timeout {
        toml::Value::String(period) => period.as_str(),
        toml::Value::Table(table) => table
            .get("period")
            .and_then(toml::Value::as_str)
            .ok_or_else(|| anyhow!("slow-timeout has no period"))?,
        _ => bail!("slow-timeout is neither a duration nor a table"),
    };
    parse_duration(period)
}

/// A Nextest duration (`500ms`, `5s`, `2m`, `1h`) in seconds.
fn parse_duration(text: &str) -> Result<f64> {
    let text = text.trim();
    let split = text.find(|c: char| !c.is_ascii_digit() && c != '.').unwrap_or(text.len());
    let (number, unit) = text.split_at(split);
    let number: f64 = number.parse().with_context(|| format!("`{text}` is not a duration"))?;
    let scale = match unit.trim() {
        "ms" => 0.001,
        "s" | "" => 1.0,
        "m" | "min" => 60.0,
        "h" => 3600.0,
        other => bail!("unknown duration unit `{other}` in `{text}`"),
    };
    Ok(number * scale)
}

// ---------------------------------------------------------------------------
// log

fn log_path(options: &Options, root: &Path) -> Result<PathBuf> {
    if let Some(log) = &options.log {
        return Ok(log.clone());
    }
    if let Some(log) = std::env::var_os(LOG_ENV).filter(|log| !log.is_empty()) {
        return Ok(PathBuf::from(log));
    }
    let home = home_dir().ok_or_else(|| anyhow!("no home directory; set {LOG_ENV}"))?;
    let identity = git(root, &["remote", "get-url", "origin"])
        .map(|remote| repository_identity(&remote))
        .unwrap_or_default();
    let mut path = home.join(".rusty-biscuit").join("test-timings");
    let segments: Vec<&str> = identity.split('/').filter(|segment| !segment.is_empty() && *segment != "..").collect();
    if segments.is_empty() {
        path.push("unknown-repository");
    } else {
        path.extend(segments);
    }
    Ok(path.join("slow-tests.jsonl"))
}

/// `host/owner/repo` for a Git remote in URL or scp-like spelling.
fn repository_identity(remote: &str) -> String {
    let mut remote = remote.trim();
    if let Some((_, rest)) = remote.split_once("://") {
        remote = rest;
    }
    let mut identity = remote.to_owned();
    if !remote.contains("://") {
        if let Some((host, path)) = remote.split_once(':') {
            if !host.contains('/') {
                identity = format!("{host}/{path}");
            }
        }
    }
    if let Some((user, rest)) = identity.split_once('@') {
        if !user.contains('/') {
            identity = rest.to_owned();
        }
    }
    let identity = identity.trim_end_matches('/');
    identity.strip_suffix(".git").unwrap_or(identity).to_owned()
}

fn home_dir() -> Option<PathBuf> {
    let variable = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
    std::env::var_os(variable).filter(|home| !home.is_empty()).map(PathBuf::from)
}

/// Append `records` in one write, so concurrent recorders cannot interleave
/// within a line.
fn append<T: Serialize>(log: &Path, records: &[T]) -> Result<()> {
    if records.is_empty() {
        return Ok(());
    }
    if let Some(parent) = log.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    let mut text = String::new();
    for record in records {
        text.push_str(&serde_json::to_string(record)?);
        text.push('\n');
    }
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(log)
        .and_then(|mut file| file.write_all(text.as_bytes()))
        .with_context(|| format!("appending to {}", log.display()))
}

/// Every record in the log; a missing log is empty, and an unreadable line is
/// skipped rather than hiding the rest.
fn read_jsonl<T: serde::de::DeserializeOwned>(log: &Path) -> Result<Vec<T>> {
    let text = match fs::read_to_string(log) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(err).with_context(|| format!("reading {}", log.display())),
    };
    Ok(text.lines().filter_map(|line| serde_json::from_str(line).ok()).collect())
}

// ---------------------------------------------------------------------------
// environment

fn workspace_root() -> Result<PathBuf> {
    let cwd = std::env::current_dir()?;
    cwd.ancestors()
        .find(|dir| dir.join(".config/nextest.toml").is_file())
        .map(Path::to_path_buf)
        .ok_or_else(|| anyhow!("no .config/nextest.toml above {}", cwd.display()))
}

fn git(root: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).current_dir(root).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn gh(root: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("gh")
        .args(args)
        .current_dir(root)
        .output()
        .context("running gh (is the GitHub CLI installed?)")?;
    if !output.status.success() {
        bail!("gh {} failed: {}", args.join(" "), String::from_utf8_lossy(&output.stderr).trim());
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn host_name() -> String {
    ["COMPUTERNAME", "HOSTNAME"]
        .iter()
        .find_map(|variable| std::env::var(variable).ok().filter(|name| !name.is_empty()))
        .or_else(|| {
            let output = Command::new("hostname").output().ok()?;
            Some(String::from_utf8_lossy(&output.stdout).trim().to_owned()).filter(|name| !name.is_empty())
        })
        .unwrap_or_else(|| "unknown".into())
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|elapsed| elapsed.as_secs()).unwrap_or(0)
}

fn format_seconds(seconds: f64) -> String {
    if seconds >= 10.0 {
        format!("{seconds:.0}")
    } else {
        format!("{seconds:.1}")
    }
}

// ---------------------------------------------------------------------------
// dates (UTC, proleptic Gregorian; Howard Hinnant's civil-day algorithms)

/// `YYYY-MM-DDTHH:MM:SSZ` (what `gh` emits) as Unix seconds.
fn parse_rfc3339(text: &str) -> Option<u64> {
    let (date, time) = text.trim().trim_end_matches('Z').split_once('T')?;
    let mut date = date.split('-').map(str::parse::<i64>);
    let (year, month, day) = (date.next()?.ok()?, date.next()?.ok()?, date.next()?.ok()?);
    let mut time = time.split(':');
    let hour: i64 = time.next()?.parse().ok()?;
    let minute: i64 = time.next()?.parse().ok()?;
    let second: f64 = time.next()?.parse().ok()?;
    let days = days_from_civil(year, month, day);
    u64::try_from(days * 86_400 + hour * 3600 + minute * 60 + second as i64).ok()
}

/// Unix seconds as `YYYY-MM-DD` (UTC).
fn format_date(seconds: u64) -> String {
    let (year, month, day) = civil_from_days((seconds / 86_400) as i64);
    format!("{year:04}-{month:02}-{day:02}")
}

/// Unix seconds as `YYYY-MM-DDTHH:MM:SSZ`.
fn format_timestamp(seconds: u64) -> String {
    let of_day = seconds % 86_400;
    format!(
        "{}T{:02}:{:02}:{:02}Z",
        format_date(seconds),
        of_day / 3600,
        of_day % 3600 / 60,
        of_day % 60
    )
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let day_of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let days = days + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days - era * 146_097;
    let year_of_era = (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 { month_index + 3 } else { month_index - 9 };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
#[path = "slow-tests-tests.rs"]
mod tests;
