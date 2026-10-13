//! Level 2 real-Claudine coverage for **AC28** of
//! `darkmatter/features/2026-09-09-more-context`: the lazy `current.*` root in
//! Claudine lifecycle handlers, under a real `claudine compose`.
//!
//! Four contracts, each a separate test:
//!
//! - **every event** — a handler that references `current.<key>` resolves in
//!   all seven lifecycle events. Three runs are needed, because `success`,
//!   `failure`, and `blocked` are mutually exclusive within one iteration.
//! - **per-event freshness (Q2)** — a fact that changes *between* two events is
//!   observed afresh by each event, while the eager `ctx` mirror of the same key
//!   stays frozen at its launch value.
//! - **shipped prompt** — a real shipped prompt whose lifecycle handlers
//!   reference a lazy root composes end to end.
//! - **shipped prompt, file-changes provider** — `prompts/format.md` counts
//!   `current.dirty_files` *after* its own `shell` step dirtied the repository,
//!   so the run traverses the lazy FileChanges refresh, not only the DateTime
//!   path `plan.md` exercises.
//!
//! ## Why this is Level 2, and what the L1 tests cannot say
//!
//! `claudine/lib/src/composition/lifecycle/executor/tests/event_time_interpolation.rs`
//! drives the executor in process with a mock shell and a recording emitter. It
//! proves the executor resolves the lazy root; it cannot prove that the
//! binary's `InvocationContext` installs a refresh authority that reaches every
//! event, that the authority survives the composition-preflight `blocked` path
//! (which builds its own execution context), or that the loop gate gets one.
//! Those are `claudine-cli` wiring facts, and only a real run exercises them.
//!
//! ## Focus
//!
//! The backend is tmux, which is headless: it creates a detached session and
//! never opens, raises, or activates a window. This file names no
//! `SpawnVisibility::Foreground`, no `focus_spawned_pane`, and no GUI
//! automation, so running it cannot take focus from whoever is at the machine.
//!
//! ## Skip-clean
//!
//! `require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux)` skips
//! when tmux is absent and hard-fails under
//! `BISCUIT_TEST_REQUIRED_BACKENDS=tmux`. Run via `just test-l2`.

#![cfg(unix)]

use crate::common;
// The shipped prompts these tests compose are frozen copies, intentionally not
// kept in step with `prompts/`.
use common::prompt_staging::{frozen_prompts_dir, stage_frozen_prompts};
use common::wrap::seed_minimal_config;
use common::{augmented_path, helper_command, write_executable};

use biscuit_test_harness::TerminalHarness;
use biscuit_test_harness::tmux::{TmuxHarness, kill_session_by_name, spawn_shell_session};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};
use tempfile::tempdir;
use test_toolkit::{Backend, Level, require_level};

/// The branch the fixture repository starts on.
const BRANCH_BEFORE: &str = "ac28-before";

/// A fixture-owned git repository, fake provider, and home directory.
struct Fixture {
    _root: tempfile::TempDir,
    home: PathBuf,
    bin: PathBuf,
    spool: PathBuf,
    repo: PathBuf,
}

impl Fixture {
    fn events_path(&self) -> PathBuf {
        self.repo.join("events.log")
    }

    fn events(&self) -> Vec<String> {
        fs::read_to_string(self.events_path())
            .unwrap_or_default()
            .lines()
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty())
            .collect()
    }
}

/// Run `git` from the test process with the inherited `GIT_*` plumbing removed,
/// pinned to a fixture identity so a host `commit.gpgsign` cannot make the
/// fixture commit prompt for a passphrase or fail.
fn git(dir: &Path, args: &[&str]) {
    let status = helper_command("git")
        .args([
            "-c",
            "user.name=Claudine Test",
            "-c",
            "user.email=claudine@example.invalid",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .current_dir(dir)
        .status()
        .unwrap_or_else(|error| panic!("git {args:?} failed to start: {error}"));
    assert!(status.success(), "git {args:?} failed in {}", dir.display());
}

fn write_provider(fixture: &Fixture, exit_code: i32) {
    write_executable(
        &fixture.bin.join("goose"),
        &format!("#!/bin/sh\ncat > /dev/null\nexit {exit_code}\n"),
    );
}

fn stage() -> Fixture {
    let root = tempdir().unwrap();
    let home = root.path().join("home");
    let bin = root.path().join("bin");
    let repo = root.path().join("repo");
    for dir in [&home, &bin, &repo] {
        fs::create_dir_all(dir).unwrap();
    }
    seed_minimal_config(&home);

    git(&repo, &["init", "-q", "-b", BRANCH_BEFORE]);
    git(
        &repo,
        &["commit", "-q", "--allow-empty", "-m", "ac28 fixture"],
    );

    let fixture = Fixture {
        _root: root,
        home,
        bin,
        spool: repo.parent().expect("fixture root").join("audio-spool"),
        repo,
    };
    // Drains stdin the way the real wrappers expect; the exit code selects the
    // terminal event the run reaches.
    write_provider(&fixture, 0);
    fixture
}

/// One lifecycle stack that records `event` with the lazy root resolved.
fn recorder(event: &str) -> String {
    format!(
        "  stack:\n    - action: {{append_line: [\"events.log\", \
         \"{event} branch={{{{ current.branch }}}}\"]}}\n"
    )
}

/// A document whose every lifecycle event records the lazy root.
///
/// `extra_frontmatter` is how a caller makes the run take a different terminal
/// path: a malformed harness key routes to `blocked`.
fn probe_document(extra_frontmatter: &str) -> String {
    let mut document = String::from("---\ntitle: AC28 lifecycle probe\n");
    document.push_str(extra_frontmatter);
    for event in [
        "initialize",
        "start",
        "success",
        "blocked",
        "failure",
        "finalize",
    ] {
        document.push_str(event);
        document.push_str(":\n");
        document.push_str(&recorder(event));
    }
    // The loop gate is the seventh event. `max: 1` holds the run to a single
    // iteration; the gate still fires once, which is what AC28 asks about.
    document.push_str("loop:\n  max: 1\n  while: \"false\"\n");
    document.push_str(&recorder("loop"));
    document.push_str("---\nBody\n");
    document
}

/// Write `document` into the fixture repository and return its file name.
fn write_document(fixture: &Fixture, name: &str, document: &str) -> String {
    fs::write(fixture.repo.join(name), document).unwrap();
    name.to_string()
}

/// Compose `document_name` from `launch_dir` inside a real tmux pane.
///
/// Waits until `ready` reports the run has produced everything the caller will
/// assert on, then captures and tears the pane down. Returns the pane text.
fn compose_in_tmux(
    fixture: &Fixture,
    launch_dir: &Path,
    document_name: &str,
    setters: &str,
    ready: impl Fn(&str) -> bool,
) -> String {
    static SEQ: AtomicU32 = AtomicU32::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);

    let session = format!("biscuit_l2_ac28_{}_{seq}", std::process::id());
    spawn_shell_session(&session, 180, 60).expect("failed to spawn tmux session");
    let mut harness = TmuxHarness::attach(&session);
    let _ = biscuit_test_harness::wait_for_prompt(&mut harness);

    // `--yolo` auto-approves the lifecycle `shell` action at pre-flight; the
    // pane has a TTY, so without it a run carrying one would sit on a prompt.
    // Setters follow the document because both are positionals.
    let command = format!(
        "cd {dir} && NO_COLOR='1' HOME='{home}' PATH='{path}' \
         CLAUDINE_RENDEZVOUS_REPORT='false' PLAYA_DRY_RUN='1' PLAYA_SPOOL_DIR='{spool}' \
         {claudine} compose --goose --yolo {document_name}{tail}",
        dir = launch_dir.display(),
        home = fixture.home.display(),
        path = augmented_path(&fixture.bin).to_string_lossy(),
        spool = fixture.spool.display(),
        claudine = common::claudine_bin(),
        tail = if setters.is_empty() {
            String::new()
        } else {
            format!(" {setters}")
        },
    );
    harness
        .send_command_with_env(&command, &[])
        .expect("send compose command");

    let deadline = Instant::now() + Duration::from_secs(20);
    let mut pane = String::new();
    while Instant::now() < deadline {
        pane = harness
            .capture()
            .map(|frame| frame.plain)
            .unwrap_or_default();
        if ready(&pane) {
            // One redraw, so the last line the run wrote is in the frame.
            std::thread::sleep(Duration::from_millis(200));
            pane = harness
                .capture()
                .map(|frame| frame.plain)
                .unwrap_or_default();
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    kill_session_by_name(&session);
    pane
}

/// Ready when the shared `events.log` holds at least `wanted` lines.
fn at_least(path: PathBuf, wanted: usize) -> impl Fn(&str) -> bool {
    move |_pane: &str| {
        fs::read_to_string(&path).map_or(0, |text| {
            text.lines().filter(|line| !line.trim().is_empty()).count()
        }) >= wanted
    }
}

/// The recorded `event` line, or a panic naming what was recorded instead.
fn event_line<'a>(lines: &'a [String], event: &str) -> &'a str {
    lines
        .iter()
        .find(|line| line.starts_with(&format!("{event} ")))
        .unwrap_or_else(|| panic!("no `{event}` line was recorded; events.log was {lines:?}"))
        .as_str()
}

/// AC28, first clause: a handler referencing `current.<key>` resolves in
/// **every** lifecycle event.
///
/// Three runs of the same document, because `success`, `failure`, and `blocked`
/// are mutually exclusive within one iteration: a clean provider exit reaches
/// `success` (and then the `loop` gate), a non-zero exit reaches `failure`, and
/// a harness key the plan parser rejects blocks before the provider is invoked.
/// Each run is its own test, in its own fixture, so nextest can run them side by
/// side; `the_three_runs_reach_all_seven_events` keeps their union whole.
mod level2_ac28_lazy_roots_resolve_in_every_lifecycle_event {
    use super::*;

    const SUCCESS_EVENTS: &[&str] = &["initialize", "start", "success", "finalize", "loop"];
    const FAILURE_EVENTS: &[&str] = &["initialize", "start", "failure", "finalize"];
    const BLOCKED_EVENTS: &[&str] = &["initialize", "blocked", "finalize"];

    /// Composes the probe document once and asserts the run recorded exactly
    /// `events`, each resolving the lazy root.
    fn assert_run(provider_exit: i32, extra_frontmatter: &str, events: &[&str]) {
        require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

        let fixture = stage();
        write_provider(&fixture, provider_exit);
        let document = write_document(&fixture, "probe.md", &probe_document(extra_frontmatter));
        let pane = compose_in_tmux(
            &fixture,
            &fixture.repo,
            &document,
            "",
            at_least(fixture.events_path(), events.len()),
        );
        let run = fixture.events();

        let expected = format!("branch={BRANCH_BEFORE}");
        assert_eq!(
            run.len(),
            events.len(),
            "the run must record exactly {events:?}; it recorded {run:?}; pane:\n{pane}"
        );
        for event in events {
            assert_eq!(
                event_line(&run, event),
                format!("{event} {expected}"),
                "the `{event}` handler must resolve the lazy root; the run \
                 recorded {run:?}; pane:\n{pane}"
            );
        }
    }

    #[test]
    fn success_run() {
        assert_run(0, "", SUCCESS_EVENTS);
    }

    #[test]
    fn failure_run() {
        assert_run(3, "", FAILURE_EVENTS);
    }

    #[test]
    fn blocked_run() {
        assert_run(3, "step_timeout: \"not-a-duration\"\n", BLOCKED_EVENTS);
    }

    #[test]
    fn the_three_runs_reach_all_seven_events() {
        for event in [
            "initialize",
            "start",
            "success",
            "blocked",
            "failure",
            "finalize",
            "loop",
        ] {
            assert!(
                [SUCCESS_EVENTS, FAILURE_EVENTS, BLOCKED_EVENTS]
                    .iter()
                    .any(|events| events.contains(&event)),
                "AC28 covers all seven lifecycle events; no run reaches `{event}`"
            );
        }
    }
}

/// AC28, Q2: each lifecycle event observes the fact as it stands when *that*
/// event evaluates, while the eager mirror of the same key stays frozen.
///
/// A `shell` action in the `start` stack moves the repository onto a new branch
/// between `start` and `success`, so the run straddles the change: `initialize`
/// and `start` see the original branch, `success` and `finalize` see the new
/// one, and `ctx.branch` reports the launch value throughout.
#[test]
fn level2_ac28_lazy_roots_refresh_between_lifecycle_events() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    const BRANCH_AFTER: &str = "ac28-after";
    let fixture = stage();

    let mut document = String::from("---\ntitle: AC28 per-event freshness\n");
    for event in ["initialize", "start", "success", "finalize"] {
        document.push_str(event);
        document.push_str(":\n  stack:\n    - action: {append_line: [\"events.log\", \"");
        document.push_str(event);
        document.push_str(" lazy={{ current.branch }} eager={{ ctx.branch }}\"]}\n");
        if event == "start" {
            document.push_str(&format!(
                "    - action: {{shell: \"git checkout -q -b {BRANCH_AFTER}\"}}\n"
            ));
        }
    }
    document.push_str("---\nBody\n");
    let doc = write_document(&fixture, "doc.md", &document);

    let pane = compose_in_tmux(
        &fixture,
        &fixture.repo,
        &doc,
        "",
        at_least(fixture.events_path(), 4),
    );

    let lines = fixture.events();
    for (event, branch) in [
        ("initialize", BRANCH_BEFORE),
        ("start", BRANCH_BEFORE),
        ("success", BRANCH_AFTER),
        ("finalize", BRANCH_AFTER),
    ] {
        assert_eq!(
            event_line(&lines, event),
            format!("{event} lazy={branch} eager={BRANCH_BEFORE}"),
            "`{event}` must read `current.branch` afresh while `ctx.branch` stays \
             at the launch value; events.log was {lines:?}; pane:\n{pane}"
        );
    }
}

/// AC28, shipped-prompt clause: a real shipped prompt whose lifecycle handlers
/// reference a lazy root composes end to end under a real Claudine run.
///
/// `prompts/plan.md` references `{{ current.time }}` in its `start`, `success`,
/// and `failure` handlers. The positive run is load-bearing because an
/// unresolvable lazy root is a *crashed* expression rather than a quiet null:
/// the control run — the same shipped prompt with `current.time` rewritten to a
/// key no catalog holds — halts with a lifecycle evaluation error. Reaching
/// `success` therefore means the lazy root resolved.
#[test]
fn level2_ac28_shipped_prompt_that_references_a_lazy_root_composes() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let shipped = fs::read_to_string(frozen_prompts_dir().join("plan.md"))
        .expect("the shipped planning prompt must be readable");
    assert!(
        shipped.contains("current.time"),
        "this test's subject is the shipped prompt's lazy-root reference; \
         `prompts/plan.md` no longer has one"
    );

    let fixture = stage();
    fs::create_dir_all(fixture.repo.join("features/f1")).unwrap();
    fs::write(fixture.repo.join("features/f1/spec.md"), "# fixture spec\n").unwrap();
    // What `plan.md` transcludes must sit beside both documents composed below.
    stage_frozen_prompts(&fixture.repo, &["plan.md"]);
    let real = "plan.md".to_string();
    let control = write_document(
        &fixture,
        "plan-control.md",
        &shipped.replace("current.time", "current.not_a_context_key"),
    );
    let setters = "spec=features/f1/spec.md plan=features/f1/plan.md";
    let settled = |pane: &str| {
        pane.contains("has been created") || pane.contains("lifecycle evaluation error")
    };

    let pane = compose_in_tmux(&fixture, &fixture.repo, &real, setters, settled);
    assert!(
        pane.contains("has been created"),
        "the shipped planning prompt must compose through its `current.time` \
         handlers and reach `success`; pane:\n{pane}"
    );
    assert!(
        !fixture.spool.exists(),
        "composing a shipped prompt must publish no audio; the private spool at \
         {} was created",
        fixture.spool.display()
    );

    let control_pane = compose_in_tmux(&fixture, &fixture.repo, &control, setters, settled);
    assert!(
        control_pane.contains("lifecycle evaluation error"),
        "the control proves the assertion above distinguishes: an unresolvable \
         lazy root in the same shipped prompt must halt the run; \
         pane:\n{control_pane}"
    );
}

/// AC28, shipped-prompt clause, FileChanges half: `prompts/format.md` reads
/// `length(current.dirty_files)` in its `start` stack *after* the `shell` step
/// that runs the formatter, so a real run must observe the files that step
/// dirtied. `plan.md` above only reaches the DateTime path; this is the one
/// shipped prompt whose lazy read goes through the FileChanges refresh.
///
/// A fake `rust` executable first on the pane's `PATH` stands in for the
/// formatter and appends a newline to exactly [`DIRTIED`] of the three tracked
/// source files, so the expected count is neither `0`, nor `null`, nor the
/// number of tracked files. The prompt's `initialize` stack refuses a dirty
/// repository, so the documents are committed before the run and the dirtying
/// happens between `initialize` and the `stdout` read — which is also what
/// proves the stack evaluates each action when it runs rather than when the
/// event fires. The control run rewrites the lazy key to one the `current`
/// root does not hold and must halt with a lifecycle evaluation error.
#[test]
fn level2_ac28_shipped_format_prompt_counts_files_dirtied_by_its_own_shell_step() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    const DIRTIED: usize = 2;
    let shipped = fs::read_to_string(frozen_prompts_dir().join("format.md"))
        .expect("the shipped format prompt must be readable");
    assert!(
        shipped.contains("length(current.dirty_files)"),
        "this test's subject is the shipped prompt's lazy file-changes read; \
         `prompts/format.md` no longer has one"
    );

    let fixture = stage();
    let sources: Vec<PathBuf> = ["a", "b", "c"]
        .iter()
        .map(|name| fixture.repo.join("src").join(format!("{name}.rs")))
        .collect();
    fs::create_dir_all(fixture.repo.join("src")).unwrap();
    for (source, name) in sources.iter().zip(["a", "b", "c"]) {
        fs::write(source, format!("fn {name}() {{}}\n")).unwrap();
    }
    let real = write_document(&fixture, "format.md", &shipped);
    let control = write_document(
        &fixture,
        "format-control.md",
        &shipped.replace("current.dirty_files", "current.not_a_context_key"),
    );
    // An untracked document is itself a dirty file, which the prompt's
    // `initialize` gate refuses, so everything the run starts from is committed.
    git(&fixture.repo, &["add", "."]);
    git(&fixture.repo, &["commit", "-q", "-m", "format fixture"]);
    // The prompt's own `git commit` step runs without the `-c` overrides the
    // `git` helper passes, and the pane's `HOME` is the fixture home with no
    // global config, so the identity and the signing opt-out must live in the
    // repository config.
    for (key, value) in [
        ("user.name", "Claudine Test"),
        ("user.email", "claudine@example.invalid"),
        ("commit.gpgsign", "false"),
    ] {
        git(&fixture.repo, &["config", key, value]);
    }
    let dirtying = sources[..DIRTIED]
        .iter()
        .map(|source| format!("printf '\\n' >> '{}'\n", source.display()))
        .collect::<String>();
    write_executable(
        &fixture.bin.join("rust"),
        &format!("#!/bin/sh\n{dirtying}"),
    );

    let settled = |pane: &str| {
        pane.contains("have been committed") || pane.contains("lifecycle evaluation error")
    };
    // The pane wraps long prose lines, so assertions run over a
    // whitespace-collapsed copy.
    let collapse = |pane: &str| pane.split_whitespace().collect::<Vec<_>>().join(" ");

    let pane = compose_in_tmux(&fixture, &fixture.repo, &real, "", settled);
    let flat = collapse(&pane);
    assert!(
        flat.contains(&format!("resulting in {DIRTIED} being updated")),
        "the `start` stack's `length(current.dirty_files)` must count the \
         {DIRTIED} files the preceding `rust fmt --all` step dirtied; pane:\n{pane}"
    );
    // The line after the commit step only prints once that step exited 0, so
    // reaching it proves the prompt's own commit did not fail behind the count.
    assert!(
        flat.contains("The formatted files have been committed."),
        "the shipped format prompt must run its commit step after the count and \
         reach its final `stdout`; pane:\n{pane}"
    );
    assert!(
        !fixture.spool.exists(),
        "composing a shipped prompt must publish no audio; the private spool at \
         {} was created",
        fixture.spool.display()
    );

    let control_pane = compose_in_tmux(&fixture, &fixture.repo, &control, "", settled);
    assert!(
        control_pane.contains("lifecycle evaluation error"),
        "the control proves the assertion above distinguishes: a lazy key the \
         `current` root does not hold, in the same `stdout` action, must halt \
         the run; pane:\n{control_pane}"
    );
    assert!(
        !collapse(&control_pane).contains("being updated"),
        "the control's `stdout` action must not render at all once its \
         interpolation raised; pane:\n{control_pane}"
    );
}
