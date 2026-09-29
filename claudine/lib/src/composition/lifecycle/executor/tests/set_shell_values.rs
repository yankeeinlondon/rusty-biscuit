//! Lifecycle `set` values that run a command (R9): whole-value `$( … )`
//! assignments are resolved and approved at preflight and run when their
//! action executes.
//!
//! Commands are portable: `git -C <repo>` in a private repository, and this
//! test binary re-executed in a helper mode that counts its runs or never
//! exits.

use super::*;

use crate::composition::RuntimeState;
use crate::composition::lifecycle::collect_lifecycle_shell_commands;
use crate::composition::preflight::{resolve_lifecycle_shell_approvals, resolve_lifecycle_shell_commands};
use crate::harness::ShellApprovalOptions;

const HELPER_ARG: &str = "claudine-set-shell-helper=";

/// What a re-executed copy of this test binary does instead of testing.
#[test]
fn helper_process_entrypoint() {
    let Some(mode) = std::env::args().find_map(|arg| arg.strip_prefix(HELPER_ARG).map(str::to_owned))
    else {
        return;
    };
    if mode == "sleep" {
        loop {
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }
    // Exits 0 on its first run and 1 on every later one.
    if let Some(marker) = mode.strip_prefix("first-ok:") {
        let first = std::fs::OpenOptions::new().write(true).create_new(true).open(marker).is_ok();
        std::process::exit(if first { 0 } else { 1 });
    }
    // Announces that it runs, then exits 0 once told to stop.
    if let Some(dir) = mode.strip_prefix("await-stop:") {
        let dir = Path::new(dir);
        std::fs::write(dir.join("started"), "").expect("record start");
        while !dir.join("stop").exists() {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        std::process::exit(0);
    }
    let path = mode.strip_prefix("count:").expect("count:<path>");
    use std::io::Write;
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut file| file.write_all(b"."))
        .expect("record run");
    std::process::exit(0);
}

/// A `$( … )` body running this binary in `mode`.
fn helper(mode: &str) -> String {
    let exe = std::env::current_exe().expect("current test executable");
    let module = module_path!()
        .split_once("::")
        .map(|(_, rest)| rest)
        .expect("module path has a crate segment");
    format!(
        "'{}' --exact {module}::helper_process_entrypoint --nocapture {HELPER_ARG}{mode}",
        exe.display()
    )
}

/// A clean, committed Git repository the commands address with `git -C`.
struct Repo {
    dir: tempfile::TempDir,
}

impl Repo {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        for args in [
            &["init", "-q"][..],
            &["config", "user.email", "t@example.com"],
            &["config", "user.name", "t"],
            &["config", "commit.gpgsign", "false"],
        ] {
            git(dir.path(), args);
        }
        std::fs::write(dir.path().join("tracked.txt"), "one\n").unwrap();
        git(dir.path(), &["add", "tracked.txt"]);
        git(dir.path(), &["commit", "-q", "-m", "init"]);
        Self { dir }
    }

    /// `git -C <repo> <args>` as `$( … )` body text.
    fn git(&self, args: &str) -> String {
        format!("git -C '{}' {args}", self.dir.path().display())
    }

    fn counter(&self) -> String {
        helper(&format!("count:{}", self.dir.path().join("runs").display()))
    }

    fn runs(&self) -> usize {
        std::fs::read_to_string(self.dir.path().join("runs")).map_or(0, |text| text.len())
    }

    fn source(&self) -> std::path::PathBuf {
        self.dir.path().join("t.md")
    }
}

fn git(dir: &Path, args: &[&str]) {
    let status = std::process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .unwrap();
    assert!(status.success(), "git {args:?}");
}

/// Parses `frontmatter` and fixes its lifecycle shell bytes, as canonical
/// preparation does.
fn prepared(repo: &Repo, frontmatter: &Value) -> LifecycleConfig {
    let mut config = parse_lifecycle_config(frontmatter, &repo.source()).unwrap();
    let context = ComposeContext::capture_for_content(repo.dir.path(), "");
    resolve_lifecycle_shell_commands(&mut config, frontmatter, &context, &repo.source(), None, None)
        .unwrap();
    config
}

struct Run {
    outcome: LifecycleEventOutcome,
    events: Vec<Emitted>,
    mutations: Map<String, Value>,
}

fn run(
    config: &LifecycleConfig,
    signals: &[LifecycleSignal],
    base: &Map<String, Value>,
    shell: &dyn ShellRunner,
) -> Run {
    let live = std::sync::Mutex::new(base.clone());
    let runtime = RuntimeState::new();
    let (_dir, engine) = temp_engine();
    let recorder = Recorder::default();
    let harness = Harness::default();
    let mut outcome = LifecycleEventOutcome::default();
    for signal in signals {
        let context = ctx_with_runtime(
            *signal,
            base,
            &live,
            &runtime,
            &engine,
            shell,
            &recorder,
            &harness,
            Path::new("t.md"),
        );
        outcome = context.execute_event(config);
    }
    Run {
        outcome,
        events: recorder.events(),
        mutations: runtime.snapshot().mutations.into_iter().collect(),
    }
}

fn start(actions: Value) -> Value {
    json!({ "start": { "stack": actions } })
}

#[test]
fn a_start_set_runs_its_command_and_gates_the_next_item() {
    let repo = Repo::new();
    let frontmatter = start(json!([
        { "action": [ { "set": { "clean": format!("$({})::ok", repo.git("diff --quiet")) } } ] },
        { "when": "clean", "action": [ { "message": "tree is clean" } ] },
    ]));
    let config = prepared(&repo, &frontmatter);
    let result = run(&config, &[LifecycleSignal::Start], &map(json!({})), &MockShell::new(0));
    assert_eq!(result.outcome, LifecycleEventOutcome::default());
    assert_eq!(result.mutations.get("clean"), Some(&json!(true)));
    assert_eq!(result.events, [Emitted::Message("tree is clean".to_string())]);
}

#[test]
fn a_result_object_reaches_a_later_when_through_dotted_access() {
    let repo = Repo::new();
    let frontmatter = start(json!([
        { "action": [ { "set": { "diff": format!("$({})::result", repo.git("rev-parse --verify nope")) } } ] },
        { "when": "!diff.ok && diff.code == 128", "action": [
            { "message": "stdout=[{{ diff.stdout }}] fatal={{ starts_with(diff.stderr, 'fatal:') }}" }
        ] },
    ]));
    let config = prepared(&repo, &frontmatter);
    let result = run(&config, &[LifecycleSignal::Start], &map(json!({})), &MockShell::new(0));
    assert_eq!(result.outcome, LifecycleEventOutcome::default());
    assert_eq!(
        result.events,
        [Emitted::Message("stdout=[] fatal=true".to_string())]
    );
    let diff = &result.mutations["diff"];
    assert_eq!((diff["ok"].clone(), diff["code"].clone()), (json!(false), json!(128)));
}

#[test]
fn preflight_approves_the_bare_command_under_its_property() {
    let repo = Repo::new();
    let frontmatter = start(json!([
        { "action": [ { "set": { "clean": format!("$({})::ok::timeout:30", repo.git("diff --quiet")) } } ] },
    ]));
    let config = prepared(&repo, &frontmatter);
    let commands = collect_lifecycle_shell_commands(&config);
    assert_eq!(commands.len(), 1, "{commands:?}");
    let (command, property) = &commands[0];
    assert!(command.ends_with("diff --quiet"), "{command}");
    assert!(!command.contains("::"), "the suffix is never approved bytes: {command}");
    assert_eq!(property, "start.stack[0].action[0].set.clean");
}

#[test]
fn approval_applies_shell_policy_to_a_set_command() {
    let repo = Repo::new();
    let options = ShellApprovalOptions {
        policy_root: Some(repo.dir.path().to_path_buf()),
        ..Default::default()
    };
    for (value, expected) in [
        ("$(rm -rf scratch)::ok", "blacklisted"),
        ("$(git status)::ok", "approval"),
    ] {
        let config = prepared(&repo, &start(json!([{ "action": [{ "set": { "v": value } }] }])));
        let error = resolve_lifecycle_shell_approvals(
            &config,
            &repo.source(),
            &[LifecycleSignal::Start],
            &options,
        )
        .expect_err(value)
        .to_string()
        .to_lowercase();
        assert!(error.contains(expected), "{value}: {error}");
    }
}

#[test]
fn a_false_guard_and_an_unresolved_value_run_nothing() {
    let repo = Repo::new();
    let guarded = start(json!([
        { "when": "false", "action": [ { "set": { "v": format!("$({})::ok", repo.counter()) } } ] },
    ]));
    let result = run(&prepared(&repo, &guarded), &[LifecycleSignal::Start], &map(json!({})), &MockShell::new(0));
    assert_eq!(result.outcome, LifecycleEventOutcome::default());
    assert_eq!(repo.runs(), 0);
    assert!(!result.mutations.contains_key("v"));

    // Parsed but never resolved at preflight: refused, never run.
    let unresolved = start(json!([
        { "action": [ { "set": { "v": format!("$({})::ok", repo.counter()) } } ] },
    ]));
    let config = parse_lifecycle_config(&unresolved, &repo.source()).unwrap();
    let result = run(&config, &[LifecycleSignal::Start], &map(json!({})), &MockShell::new(0));
    let error = result.outcome.evaluation_error.expect("an unresolved value is refused");
    assert!(error.msg.contains("never resolved"), "{}", error.msg);
    assert_eq!(repo.runs(), 0);
}

#[test]
fn a_disabled_runner_refuses_the_set_before_anything_runs() {
    let repo = Repo::new();
    let frontmatter = start(json!([
        { "action": [ { "set": { "v": format!("$({})::ok", repo.counter()) }, "no_error": true } ] },
    ]));
    let config = prepared(&repo, &frontmatter);
    let result = run(&config, &[LifecycleSignal::Start], &map(json!({})), &DisabledShellRunner);
    let error = result.outcome.evaluation_error.expect("the early-route prohibition holds");
    assert!(error.msg.contains("forbidden during initialize"), "{}", error.msg);
    assert_eq!(repo.runs(), 0);
    assert!(!result.mutations.contains_key("v"));
}

#[test]
fn a_failed_mapping_writes_no_destination() {
    let repo = Repo::new();
    let frontmatter = start(json!([
        { "action": [ { "set": {
            "plain": "kept?",
            "ran": format!("$({})::ok", repo.counter()),
            "failed": format!("$({})", repo.git("rev-parse --verify nope")),
        } } ] },
    ]));
    let config = prepared(&repo, &frontmatter);
    let base = map(json!({ "plain": "before" }));
    let result = run(&config, &[LifecycleSignal::Start], &base, &MockShell::new(0));
    let error = result.outcome.action_error.expect("an unsuffixed non-zero exit fails the set");
    assert!(error.msg.contains("exit 128"), "{}", error.msg);
    assert!(result.mutations.is_empty(), "{:?}", result.mutations);
    // The command that succeeded ran; its effect is not undone.
    assert_eq!(repo.runs(), 1);
}

#[test]
fn each_executed_assignment_has_a_fresh_result_cache() {
    let repo = Repo::new();
    let value = format!("$({})::ok", repo.counter());
    let frontmatter = json!({
        "start": { "stack": [ { "action": [ { "set": { "a": value, "b": value } } ] } ] },
        "success": { "stack": [ { "action": [ { "set": { "c": value } } ] } ] },
    });
    let config = prepared(&repo, &frontmatter);
    run(&config, &[LifecycleSignal::Start], &map(json!({})), &MockShell::new(0));
    assert_eq!(repo.runs(), 1, "values of one assignment share one run");
    let result = run(&config, &[LifecycleSignal::Start, LifecycleSignal::Success], &map(json!({})), &MockShell::new(0));
    assert_eq!(repo.runs(), 3, "`success` runs the command again after `start`");
    assert_eq!(result.mutations.get("c"), Some(&json!(true)));
}

#[test]
fn a_later_event_recaptures_a_changed_result() {
    let repo = Repo::new();
    let value = format!(
        "$({})::ok",
        helper(&format!("first-ok:{}", repo.dir.path().join("ran-once").display()))
    );
    let frontmatter = json!({
        "start": { "stack": [ { "action": [ { "set": { "at_start": value } } ] } ] },
        "success": { "stack": [ { "action": [ { "set": { "at_success": value } } ] } ] },
    });
    let config = prepared(&repo, &frontmatter);
    let result = run(&config, &[LifecycleSignal::Start, LifecycleSignal::Success], &map(json!({})), &MockShell::new(0));
    assert_eq!(result.outcome, LifecycleEventOutcome::default());
    assert_eq!(result.mutations.get("at_start"), Some(&json!(true)));
    assert_eq!(
        result.mutations.get("at_success"),
        Some(&json!(false)),
        "`success` reads the command's new answer, not `start`'s"
    );
}

#[test]
fn an_interruption_while_a_set_command_runs_fails_the_set() {
    let repo = Repo::new();
    let dir = repo.dir.path().to_path_buf();
    let config = prepared(
        &repo,
        &start(json!([{ "action": [{
            "set": { "v": format!("$({})::ok::timeout:60", helper(&format!("await-stop:{}", dir.display()))) },
            "no_error": true,
        }] }])),
    );

    // Stands in for the user's Ctrl+C: Claudine records the interruption, and
    // the signal the terminal delivers to the process group ends the command.
    // Setting the flag alone cannot end a running command.
    let interrupter = std::thread::spawn(move || {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        while !dir.join("started").exists() {
            assert!(std::time::Instant::now() < deadline, "the `set` command never started");
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        crate::interrupt::mark_interrupted();
        std::fs::write(dir.join("stop"), "").unwrap();
    });
    let began = std::time::Instant::now();
    let result = run(&config, &[LifecycleSignal::Start], &map(json!({})), &MockShell::new(0));
    let elapsed = began.elapsed();
    interrupter.join().unwrap();
    crate::interrupt::clear_for_tests();

    assert!(result.outcome.action_error.is_none(), "{:?}", result.outcome);
    let error = result.outcome.evaluation_error.expect("`no_error` cannot forgive an interruption");
    assert!(error.msg.contains("interrupted"), "{}", error.msg);
    assert!(!result.mutations.contains_key("v"), "the command exited 0, yet nothing is written");
    assert!(elapsed < std::time::Duration::from_secs(30), "returned after {elapsed:?}");
}

#[test]
fn approved_bytes_do_not_follow_a_later_runtime_write() {
    let repo = Repo::new();
    let frontmatter = json!({
        "target": "HEAD",
        "start": { "stack": [
            { "action": [ { "set": { "target": "nope" } } ] },
            { "action": [ { "set": { "found": format!("$({})::ok", repo.git("rev-parse --verify {{ target }}")) } } ] },
        ] },
    });
    let config = prepared(&repo, &frontmatter);
    let base = map(json!({ "target": "HEAD" }));
    let result = run(&config, &[LifecycleSignal::Start], &base, &MockShell::new(0));
    assert_eq!(result.outcome, LifecycleEventOutcome::default());
    assert_eq!(result.mutations.get("target"), Some(&json!("nope")));
    assert_eq!(result.mutations.get("found"), Some(&json!(true)), "ran with the approved `HEAD`");
}

#[test]
fn every_value_reads_the_pre_write_state() {
    let repo = Repo::new();
    let frontmatter = start(json!([
        { "action": [ { "set": {
            "flag": false,
            "picked": format!("$( flag ? {} : 'was false' )::result", repo.git("rev-parse --is-inside-work-tree")),
        } } ] },
    ]));
    let config = prepared(&repo, &frontmatter);
    let result = run(&config, &[LifecycleSignal::Start], &map(json!({ "flag": true })), &MockShell::new(0));
    assert_eq!(result.outcome, LifecycleEventOutcome::default());
    assert_eq!(result.mutations["picked"]["stdout"], json!("true"), "the condition read `flag: true`");
    assert_eq!(result.mutations.get("flag"), Some(&json!(false)));
}

/// Runs `value` as the only `set` value and asserts it fails the action.
fn assert_set_fails(repo: &Repo, value: &str, fragment: &str) {
    let config = prepared(repo, &start(json!([{ "action": [{ "set": { "v": value } }] }])));
    let result = run(&config, &[LifecycleSignal::Start], &map(json!({})), &MockShell::new(0));
    let error = result.outcome.action_error.unwrap_or_else(|| panic!("{value}"));
    assert!(error.msg.to_lowercase().contains(fragment), "{value}: {}", error.msg);
    assert!(!result.mutations.contains_key("v"));
}

#[test]
fn a_timed_out_command_fails_the_set() {
    // Claudine has no allowed-timeout mode, so a result suffix does not forgive it.
    assert_set_fails(&Repo::new(), &format!("$({})::ok::timeout:1", helper("sleep")), "timed out");
}

#[test]
fn missing_and_interrupted_commands_fail_the_set() {
    let repo = Repo::new();
    assert_set_fails(&repo, "$(claudine-no-such-command-r9 run)::ok", "not found");

    // A user interruption keeps its cancellation outcome: `no_error` cannot
    // turn it into a recoverable value.
    let config = prepared(
        &repo,
        &start(json!([{ "action": [{ "set": { "v": format!("$({})::ok", repo.git("diff --quiet")) }, "no_error": true }] }])),
    );
    crate::interrupt::mark_interrupted();
    let result = run(&config, &[LifecycleSignal::Start], &map(json!({})), &MockShell::new(0));
    crate::interrupt::clear_for_tests();
    assert!(result.outcome.evaluation_error.is_some(), "{:?}", result.outcome);
    assert!(!result.mutations.contains_key("v"));
}

#[test]
fn nested_and_mixed_values_stay_data() {
    let repo = Repo::new();
    let frontmatter = start(json!([
        { "action": [ { "set": {
            "nested": { "sha": format!("$({})", repo.counter()) },
            "listed": [ format!("$({})", repo.counter()) ],
            "mixed": format!("sha $({})", repo.counter()),
        } } ] },
    ]));
    let config = prepared(&repo, &frontmatter);
    let result = run(&config, &[LifecycleSignal::Start], &map(json!({})), &MockShell::new(0));
    assert_eq!(result.outcome, LifecycleEventOutcome::default());
    assert_eq!(repo.runs(), 0);
    assert_eq!(result.mutations["mixed"], json!(format!("sha $({})", repo.counter())));
    assert_eq!(result.mutations["nested"]["sha"], json!(format!("$({})", repo.counter())));
}

#[test]
fn initialize_refuses_a_shell_assignment_in_any_item() {
    for guard in ["true", "false"] {
        let frontmatter = json!({ "initialize": { "stack": [
            { "action": [ { "message": "first" } ] },
            { "when": guard, "action": [ { "set": { "v": "$(git status)::ok" } } ] },
        ] } });
        let error = parse_lifecycle_config(&frontmatter, Path::new("t.md")).unwrap_err();
        assert!(
            matches!(&error, CompositionError::LifecycleActionPlacement { property, action, .. }
                if property == "initialize.stack[1].action" && action == "shell"),
            "{guard}: {error:?}"
        );
    }
    // A `start` assignment is legal beside an `initialize` block.
    let legal = json!({
        "initialize": { "stack": [ { "action": [ { "set": { "phase": "boot" } } ] } ] },
        "start": { "stack": [ { "action": [ { "set": { "v": "$(git status)::ok" } } ] } ] },
    });
    parse_lifecycle_config(&legal, Path::new("t.md")).unwrap();
}

/// The input robustness matrix for the lifecycle `set` shell-value reader: one
/// fixture, one edit per cell, asserted through the parsed configuration and
/// the executed event.
#[test]
fn set_shell_value_reader_robustness_matrix() {
    let repo = Repo::new();
    let command = repo.git("diff --quiet");
    enum Cell {
        Value(Value),
        ParseError(&'static [&'static str]),
    }
    const FIVE: &[&str] = &["`::ok`", "`::exit-code`", "`::result`", "`::timeout:<seconds>`", "`::no-cache`"];
    let cells: Vec<(&str, Value, Cell)> = vec![
        ("control", json!(format!("$({command})::ok")), Cell::Value(json!(true))),
        ("absent suffix", json!(format!("$({command})")), Cell::Value(json!(""))),
        ("::exit-code", json!(format!("$({command})::exit-code")), Cell::Value(json!(0))),
        ("::result", json!(format!("$({command})::result")), Cell::Value(json!({"ok": true, "code": 0, "stdout": "", "stderr": ""}))),
        ("explicit null", Value::Null, Cell::Value(Value::Null)),
        ("wrong type, whole field", json!(123), Cell::Value(json!(123))),
        ("wrong type, one element", json!(format!("$({command})::ok::bogus")), Cell::ParseError(FIVE)),
        ("wrong type, every element", json!(format!("$({command})::bogus")), Cell::ParseError(FIVE)),
        ("empty suffix", json!(format!("$({command})::")), Cell::ParseError(&["Empty suffix"])),
        ("empty command", json!("$()::ok"), Cell::ParseError(&["no shell command"])),
        ("duplicate result suffix", json!(format!("$({command})::ok::result")), Cell::ParseError(&["`::ok`", "`::result`"])),
        ("duplicate timeout", json!(format!("$({command})::timeout:5::timeout:9")), Cell::ParseError(&["`::timeout:5`", "`::timeout:9`"])),
        ("trailing content", json!(format!("$({command})::ok trailing")), Cell::ParseError(FIVE)),
        ("mixed value is data", json!(format!("sha $({command})")), Cell::Value(json!(format!("sha $({command})")))),
    ];
    for (name, value, cell) in cells {
        let frontmatter = start(json!([{ "action": [{ "set": { "v": value } }] }]));
        match cell {
            Cell::Value(expected) => {
                let config = prepared(&repo, &frontmatter);
                let result = run(&config, &[LifecycleSignal::Start], &map(json!({})), &MockShell::new(0));
                assert_eq!(result.outcome, LifecycleEventOutcome::default(), "{name}");
                assert_eq!(result.mutations.get("v"), Some(&expected), "{name}");
            }
            Cell::ParseError(fragments) => {
                let error = parse_lifecycle_config(&frontmatter, &repo.source())
                    .expect_err(name)
                    .to_string();
                for fragment in fragments {
                    assert!(error.contains(fragment), "{name}: {error}");
                }
            }
        }
    }

    // A duplicate destination key is a frontmatter parse error, never last-wins.
    let text = "---\nstart:\n  stack:\n    - action:\n        - set:\n            v: \"$(git status)::ok\"\n            v: \"$(git status)::ok\"\n---\n".to_string();
    let error = darkmatter::markdown::Markdown::try_from_content(text).expect_err("duplicate key");
    assert!(error.to_string().contains("duplicate"), "{error}");
}
