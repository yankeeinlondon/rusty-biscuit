//! Level 2 real-Claudine coverage for **AC28** of
//! `darkmatter/features/2026-09-09-more-context`: the lazy `current.*` and
//! `current_env.*` roots in Claudine lifecycle handlers, under a real
//! `claudine compose`.
//!
//! Five contracts, each a separate test:
//!
//! - **every event** — a handler that references `current.<key>` and
//!   `current_env.<KEY>` resolves in all seven lifecycle events. Three runs are
//!   needed, because `success`, `failure`, and `blocked` are mutually exclusive
//!   within one iteration.
//! - **live environment refresh** — the composing process mutates its own
//!   environment *after* the eager `env` snapshot is taken, and every event
//!   sees the new value through `current_env` while `env` still reports the
//!   launch value.
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
//! proves the executor rereads the environment; it cannot prove that the
//! binary's `InvocationContext` installs a refresh authority that reaches every
//! event, that the authority survives the composition-preflight `blocked` path
//! (which builds its own execution context), or that the loop gate gets one.
//! Those are `claudine-cli` wiring facts, and only a real run exercises them.
//!
//! ## What is not provable at this level
//!
//! AC28's strongest reading of the environment clause — a dedicated key mutated
//! **between** two lifecycle events of one run — has no product surface in a
//! real `claudine` process. A subprocess cannot mutate its parent's
//! environment, and the binary exposes no action that writes its own. The `PWD`
//! sync exercised below is the one in-process environment mutation a real run
//! makes, so it is the live-refresh evidence available here; the
//! between-evaluations variant stays with the in-process executor test named
//! above. `level2_ac28_lazy_roots_refresh_between_lifecycle_events` covers the
//! per-event half of the clause with a fact that *can* change mid-run.
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

mod common;
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

/// The environment key the fixture exports on the `claudine` command line, so a
/// handler has a `current_env.<KEY>` to read that no host could supply.
const ENV_KEY: &str = "CLAUDINE_TEST_AC28_KEY";
const ENV_VALUE: &str = "ac28-sentinel";

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

/// One lifecycle stack that records `event` with both lazy roots resolved.
fn recorder(event: &str) -> String {
    format!(
        "  stack:\n    - action: {{append_line: [\"events.log\", \
         \"{event} branch={{{{ current.branch }}}} key={{{{ current_env.{ENV_KEY} }}}}\"]}}\n"
    )
}

/// A document whose every lifecycle event records both lazy roots.
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
        "cd {dir} && NO_COLOR='1' HOME='{home}' PATH='{path}' {ENV_KEY}='{ENV_VALUE}' \
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

/// AC28, first clause: a handler referencing `current.<key>` and
/// `current_env.<KEY>` resolves both in **every** lifecycle event.
///
/// Three runs of the same document, because `success`, `failure`, and `blocked`
/// are mutually exclusive within one iteration: a clean provider exit reaches
/// `success` (and then the `loop` gate), a non-zero exit reaches `failure`, and
/// a harness key the plan parser rejects blocks before the provider is invoked.
#[test]
fn level2_ac28_lazy_roots_resolve_in_every_lifecycle_event() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let fixture = stage();
    let ok = write_document(&fixture, "ok.md", &probe_document(""));
    let blocked = write_document(
        &fixture,
        "blocked.md",
        &probe_document("step_timeout: \"not-a-duration\"\n"),
    );

    let success_pane = compose_in_tmux(
        &fixture,
        &fixture.repo,
        &ok,
        "",
        at_least(fixture.events_path(), 5),
    );
    let success_run = fixture.events();

    write_provider(&fixture, 3);
    let failure_pane = compose_in_tmux(
        &fixture,
        &fixture.repo,
        &ok,
        "",
        at_least(fixture.events_path(), success_run.len() + 4),
    );
    let failure_run = fixture.events()[success_run.len()..].to_vec();

    let recorded_so_far = success_run.len() + failure_run.len();
    let blocked_pane = compose_in_tmux(
        &fixture,
        &fixture.repo,
        &blocked,
        "",
        at_least(fixture.events_path(), recorded_so_far + 3),
    );
    let blocked_run = fixture.events()[recorded_so_far..].to_vec();

    let expected = format!("branch={BRANCH_BEFORE} key={ENV_VALUE}");
    let runs: [(&Vec<String>, &[&str], &String); 3] = [
        (
            &success_run,
            &["initialize", "start", "success", "finalize", "loop"],
            &success_pane,
        ),
        (
            &failure_run,
            &["initialize", "start", "failure", "finalize"],
            &failure_pane,
        ),
        (
            &blocked_run,
            &["initialize", "blocked", "finalize"],
            &blocked_pane,
        ),
    ];
    let mut reached: Vec<String> = Vec::new();
    for (run, events, pane) in runs {
        assert_eq!(
            run.len(),
            events.len(),
            "the run must record exactly {events:?}; it recorded {run:?}; pane:\n{pane}"
        );
        for event in events {
            assert_eq!(
                event_line(run, event),
                format!("{event} {expected}"),
                "the `{event}` handler must resolve both lazy roots; the run \
                 recorded {run:?}; pane:\n{pane}"
            );
            reached.push((*event).to_string());
        }
    }

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
            reached.iter().any(|seen| seen == event),
            "AC28 covers all seven lifecycle events; `{event}` was never reached \
             across the three runs, which reached {reached:?}"
        );
    }
}

/// AC28, environment clause: the composing process changes its own environment
/// after the eager `env` snapshot is taken, and the lazy mirror sees it.
///
/// The change is real product behavior, not a test poke. Launched from a
/// subdirectory, the wrapper switches to the child working directory and syncs
/// `PWD` to match (`switch_process_cwd`, because several providers trust `PWD`
/// over `getcwd`). `env.PWD` therefore reports the launch directory for the
/// whole run while `current_env.PWD` reports the directory the process is
/// actually in — in every event. Reading only one of the two roots could not
/// tell a live reread from a frozen snapshot that happened to be right.
#[test]
fn level2_ac28_current_env_sees_an_in_process_change_the_env_snapshot_missed() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let fixture = stage();
    let launch_dir = fixture.repo.join("nested/launch-dir");
    fs::create_dir_all(&launch_dir).unwrap();

    let mut document = String::from("---\ntitle: AC28 environment refresh\n");
    for event in ["initialize", "start", "success", "finalize"] {
        document.push_str(event);
        document.push_str(":\n  stack:\n    - action: {append_line: [\"events.log\", \"");
        document.push_str(event);
        document.push_str(" live={{ current_env.PWD }} frozen={{ env.PWD }}\"]}\n");
    }
    document.push_str("---\nBody\n");
    fs::write(launch_dir.join("doc.md"), &document).unwrap();

    let pane = compose_in_tmux(
        &fixture,
        &launch_dir,
        "doc.md",
        "",
        at_least(fixture.events_path(), 4),
    );

    let lines = fixture.events();
    let repo_root = fs::canonicalize(&fixture.repo).unwrap();
    let launch = launch_dir.display().to_string();
    for event in ["initialize", "start", "success", "finalize"] {
        let line = event_line(&lines, event);
        let live = line
            .split("live=")
            .nth(1)
            .and_then(|rest| rest.split(" frozen=").next())
            .unwrap_or_default();
        let frozen = line.split(" frozen=").nth(1).unwrap_or_default();
        assert_eq!(
            fs::canonicalize(live).ok().as_deref(),
            Some(repo_root.as_path()),
            "`current_env.PWD` in `{event}` must be the directory the process \
             moved to; events.log was {lines:?}; pane:\n{pane}"
        );
        assert_eq!(
            frozen, launch,
            "`env.PWD` in `{event}` must still be the launch directory the eager \
             snapshot captured; events.log was {lines:?}; pane:\n{pane}"
        );
        assert_ne!(
            live, frozen,
            "the fixture only says anything while the two directories differ; \
             events.log was {lines:?}"
        );
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

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("claudine/cli parent")
        .parent()
        .expect("workspace root")
        .to_path_buf()
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

    let shipped = fs::read_to_string(workspace_root().join("prompts/plan.md"))
        .expect("the shipped planning prompt must be readable");
    assert!(
        shipped.contains("current.time"),
        "this test's subject is the shipped prompt's lazy-root reference; \
         `prompts/plan.md` no longer has one"
    );

    let fixture = stage();
    fs::create_dir_all(fixture.repo.join("features/f1")).unwrap();
    fs::write(fixture.repo.join("features/f1/spec.md"), "# fixture spec\n").unwrap();
    let real = write_document(&fixture, "plan.md", &shipped);
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
    let shipped = fs::read_to_string(workspace_root().join("prompts/format.md"))
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
