//! Level-1 process coverage for per-document environment capture through the
//! real entry routes: each sequence step, each serial and parallel group leg,
//! and each `proxy` target captures its own map before its `initialize`, and
//! every stage of that document reads it.
//!
//! The maps are supplied, never exported: the launch document reads the
//! child's own environment, and `CLAUDINE_TEST_DOCUMENT_ENVIRONMENTS` (a
//! `test-fixtures` seam) scripts what each later document captures, in
//! capture order, and logs every capture so a test can count them. No test
//! here changes its own process environment.
//!
//! Every document line reads its map twice, through `env.*` and through an
//! environment-expanded file reference (`{{DOC_ROOT}}/title.md`), so the two
//! consumers must agree.

use std::fs;
use std::path::{Path, PathBuf};

use crate::common;

use common::{CliProcessFixture, write};
#[cfg(unix)]
use common::write_executable;
use serde_json::{Value, json};

/// The fragment a document line uses to read its map through both consumers.
const READS: &str = "env={{ env.DOC_MARK }} ref={{ markdown_title(\"{{DOC_ROOT}}/title.md\") }}";

/// A Claude stub that reports one successful result and exits 0.
fn write_succeeding_claude(bin_dir: &Path) {
    #[cfg(unix)]
    write_executable(
        &bin_dir.join("claude"),
        r#"#!/bin/sh
printf '%s\n' '{"type":"system","subtype":"init","session_id":"session-1","model":"claude-test"}'
printf '%s\n' '{"type":"result","subtype":"success","result":"done","session_id":"session-1","is_error":false}'
exit 0
"#,
    );

    #[cfg(windows)]
    write(
        &bin_dir.join("claude.cmd"),
        "@echo off\r\n\
echo {\"type\":\"system\",\"subtype\":\"init\",\"session_id\":\"session-1\",\"model\":\"claude-test\"}\r\n\
echo {\"type\":\"result\",\"subtype\":\"success\",\"result\":\"done\",\"session_id\":\"session-1\",\"is_error\":false}\r\n\
exit /b 0\r\n",
    );
}

struct Fixture {
    inner: CliProcessFixture,
}

struct Run {
    code: Option<i32>,
    output: String,
}

impl Run {
    /// The output with box glyphs and hard wraps removed.
    fn collapsed(&self) -> String {
        self.output
            .replace('┃', " ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }
}

impl Fixture {
    /// A workspace with one titled directory per map name, so a file
    /// reference through `{{DOC_ROOT}}` names which map it read.
    fn new(name: &str, maps: &[&str]) -> Self {
        let inner = CliProcessFixture::named(name);
        inner.seed_user_config();
        write_succeeding_claude(inner.bin_dir());
        let fixture = Self { inner };
        for map in maps.iter().chain(["launch"].iter()) {
            write(&fixture.root(map).join("title.md"), &format!("# Title {map}\n"));
        }
        fixture
    }

    fn cwd(&self) -> &Path {
        self.inner.cwd()
    }

    fn root(&self, map: &str) -> PathBuf {
        self.cwd().join("maps").join(map)
    }

    fn write_doc(&self, name: &str, content: &str) -> PathBuf {
        let path = self.cwd().join(name);
        write(&path, content);
        path
    }

    /// The overlay that makes a document read map `name`.
    fn map(&self, name: &str) -> Value {
        json!({ "DOC_MARK": name, "DOC_ROOT": self.root(name).display().to_string() })
    }

    /// What a line written under map `name` reads.
    fn reads(&self, name: &str) -> String {
        format!("env={name} ref=Title {name}")
    }

    /// Run `claudine <args>` launched under map `launch`, with `captures`
    /// scripting the documents entered after launch, in capture order.
    fn run(&self, args: &[&str], captures: &[Value]) -> Run {
        self.run_launched_with(args, captures, &[])
    }

    /// [`Self::run`] with `extra` added to the launch map.
    fn run_launched_with(&self, args: &[&str], captures: &[Value], extra: &[(&str, &str)]) -> Run {
        let script = self.cwd().join("environments.json");
        fs::write(&script, serde_json::to_string(captures).unwrap()).unwrap();
        let output = self
            .inner
            .command()
            .env("CLAUDINE_TEST_DOCUMENT_ENVIRONMENTS", &script)
            .env("DOC_MARK", "launch")
            .env("DOC_ROOT", self.root("launch"))
            .env("DOC_STABLE", "stable")
            .envs(extra.iter().copied())
            .args(args)
            .output()
            .expect("claudine runs");
        Run {
            code: output.status.code(),
            output: common::strip_ansi(&format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            )),
        }
    }

    fn events(&self) -> Vec<String> {
        fs::read_to_string(self.cwd().join("events.log"))
            .unwrap_or_default()
            .lines()
            .map(str::to_string)
            .collect()
    }

    /// How many documents captured an environment after launch.
    fn captures(&self) -> usize {
        fs::read_to_string(self.cwd().join("environments.json.captures"))
            .unwrap_or_default()
            .lines()
            .count()
    }
}

/// A side-effect task appending `label` and what its document reads.
fn appending_task(label: &str) -> String {
    format!(
        "side_effect:\n      append_line:\n        - events.log\n        - '{label} {READS}'\n"
    )
}

/// One lifecycle stack item appending `label` and what its document reads.
fn trail(event: &str, label: &str) -> String {
    format!(
        "{event}:\n        - append_line: [\"events.log\", '{label} {READS}']\n"
    )
}

/// A lifecycle stack item appending `label` and then proxying to `./target.md`.
fn trail_then_proxy(event: &str, label: &str) -> String {
    format!(
        "{event}:\n        - append_line: [\"events.log\", '{label} {READS}']\n        - proxy: ./target.md\n"
    )
}

#[test]
fn successive_sequence_steps_each_read_the_map_they_captured() {
    let fixture = Fixture::new("doc-env-steps", &["a", "b"]);
    let sequence = fixture.write_doc(
        "seq.md",
        &format!(
            "---\nsequence:\n  - name: first\n    {}  - name: second\n    {}---\n",
            appending_task("first"),
            appending_task("second"),
        ),
    );

    let run = fixture.run(
        &["sequence", "--claude", sequence.to_str().unwrap()],
        &[fixture.map("a"), fixture.map("b")],
    );

    assert_eq!(run.code, Some(0), "{}", run.output);
    assert_eq!(
        fixture.events(),
        vec![
            format!("first {}", fixture.reads("a")),
            format!("second {}", fixture.reads("b")),
        ],
        "the second step reads the map it captured, and the first keeps its own"
    );
    assert_eq!(fixture.captures(), 2, "one capture per step");
}

#[test]
fn bounded_group_legs_each_capture_their_own_map() {
    group_legs_row("doc-env-serial", "serial");
}

#[test]
fn parallel_group_legs_each_capture_their_own_map() {
    group_legs_row("doc-env-parallel", "parallel");
}

/// A group step whose two legs append what they read. The step captures
/// first, then each leg in declaration order before any leg starts.
fn group_legs_row(name: &str, execution: &str) {
    let execution = if execution == "serial" { "max_parallel: 1\n      " } else { "" };
    let fixture = Fixture::new(name, &["step", "left", "right"]);
    let sequence = fixture.write_doc(
        "seq.md",
        &format!(
            "---\nsequence:\n  - name: grouped\n    group:\n      name: legs\n      {execution}tasks:\n        - name: left\n          {}        - name: right\n          {}---\n",
            appending_task("left").replace("\n  ", "\n          "),
            appending_task("right").replace("\n  ", "\n          "),
        ),
    );

    let run = fixture.run(
        &["sequence", "--claude", sequence.to_str().unwrap()],
        &[fixture.map("step"), fixture.map("left"), fixture.map("right")],
    );

    assert_eq!(run.code, Some(0), "{}", run.output);
    let mut events = fixture.events();
    // Parallel legs finish in any order; what each read is the contract.
    events.sort();
    assert_eq!(
        events,
        vec![
            format!("left {}", fixture.reads("left")),
            format!("right {}", fixture.reads("right")),
        ]
    );
    assert_eq!(fixture.captures(), 3, "the step and each leg capture once");
}

/// The proxy source runs under the launch map; the target captures its own
/// before its `initialize`, and its every event reads it.
#[test]
fn a_proxy_target_captures_its_own_map_before_its_initialize() {
    let fixture = Fixture::new("doc-env-proxy", &["target"]);
    fixture.write_doc(
        "target.md",
        &format!(
            "---\n{}{}{}---\ntarget body\n",
            trail("initialize", "target initialize"),
            trail("success", "target success"),
            trail("finalize", "target finalize"),
        ),
    );
    let source = fixture.write_doc(
        "source.md",
        &format!("---\n{}---\nsource body\n", trail_then_proxy("success", "source success")),
    );

    let run = fixture.run(
        &["compose", "--claude", source.to_str().unwrap()],
        &[fixture.map("target")],
    );

    assert_eq!(run.code, Some(0), "{}", run.output);
    assert_eq!(
        fixture.events(),
        vec![
            format!("source success {}", fixture.reads("launch")),
            format!("target initialize {}", fixture.reads("target")),
            format!("target success {}", fixture.reads("target")),
            format!("target finalize {}", fixture.reads("target")),
        ]
    );
    assert_eq!(fixture.captures(), 1, "only the target captures after launch");
}

/// Loop iterations, a `retry`, and `finalize` re-enter one document; none of
/// them captures again, and each reads the map the document entered with.
#[test]
fn re_entering_a_document_reads_its_map_without_capturing_again() {
    let fixture = Fixture::new("doc-env-reentry", &["target"]);
    fixture.write_doc(
        "target.md",
        &format!(
            "---\nn: 0\nloop:\n    while: \"n < 1\"\n    action: \"increment(n)\"\n{}success:\n        - when: \"!file_exists('retried.txt')\"\n          then:\n              - append_line: [\"retried.txt\", \"once\"]\n              - append_line: [\"events.log\", 'target retry {READS}']\n              - retry: 1\n        - append_line: [\"events.log\", 'target success {READS}']\n{}---\nloop body\n",
            trail("start", "target start"),
            trail("finalize", "target finalize"),
        ),
    );
    let source = fixture.write_doc(
        "source.md",
        &format!("---\n{}---\nsource body\n", trail_then_proxy("success", "source success")),
    );

    let run = fixture.run(
        &["compose", "--claude", source.to_str().unwrap()],
        &[fixture.map("target")],
    );

    assert_eq!(run.code, Some(0), "{}", run.output);
    let events = fixture.events();
    assert_eq!(events[0], format!("source success {}", fixture.reads("launch")));
    let target: Vec<&String> = events[1..].iter().collect();
    for event in [
        "target start",
        "target retry",
        "target success",
        "target finalize",
    ] {
        assert!(
            target.iter().any(|line| line.starts_with(event)),
            "`{event}` never ran: {events:?}\n{}",
            run.output
        );
    }
    assert_eq!(
        target.iter().filter(|line| line.starts_with("target start")).count(),
        3,
        "one retried attempt and a second loop pass must both start: {events:?}"
    );
    assert_eq!(
        target.iter().filter(|line| line.starts_with("target success")).count(),
        2,
        "both loop passes must succeed: {events:?}"
    );
    for line in &target {
        assert!(line.ends_with(&fixture.reads("target")), "{line} read another map: {events:?}");
    }
    assert_eq!(fixture.captures(), 1, "re-entry captures nothing new");
}

/// A first-task command sequence preflight approved as resolved bytes runs
/// only as approved. A step whose map leaves the text identical runs it; one
/// whose map changes the text is refused before it runs, naming the command.
/// (A later task's command is approved by its shape instead, and its `>>`
/// redirection would not be a valid template.)
fn assert_step_approved_text(map_change: &str) {
    let command = "echo ran-{{ env.DOC_STABLE }}>>events.log";
    let fixture = Fixture::new("doc-env-approval", &["a"]);
    let sequence = fixture.write_doc(
        "seq.md",
        &format!(
            "---\nsequence:\n  - name: first\n    shell: '{command}'\n  - name: second\n    {}---\n",
            appending_task("second"),
        ),
    );
    let args = ["sequence", "--claude", "--yolo", sequence.to_str().unwrap()];

    let captured = match map_change {
        "unchanged" => json!({}),
        "unrelated" => fixture.map("a"),
        "command" => json!({ "DOC_STABLE": "moved" }),
        _ => panic!("unknown environment change: {map_change}"),
    };
    let run = fixture.run(&args, &[captured, fixture.map("a")]);
    if map_change == "command" {
        assert_ne!(run.code, Some(0), "{}", run.output);
        assert!(fixture.events().is_empty(), "the changed command never ran: {:?}", fixture.events());
        let collapsed = run.collapsed();
        for expected in ["echo ran-{{ env.DOC_STABLE }}", "was approved at sequence preflight"] {
            assert!(collapsed.contains(expected), "`{expected}` missing from:\n{}", run.output);
        }
    } else {
        assert_eq!(run.code, Some(0), "{}", run.output);
        assert_eq!(fixture.events().first().map(String::as_str), Some("ran-stable"));
    }
}

macro_rules! step_approval_environment_tests {
    ($($test:ident => $change:literal),* $(,)?) => {
        $(
            #[test]
            fn $test() {
                assert_step_approved_text($change);
            }
        )*

        #[test]
        fn step_approval_cases_cover_both_unchanged_text_states_and_changed_text() {
            assert_eq!([$($change),*], ["unchanged", "unrelated", "command"]);
        }
    };
}

step_approval_environment_tests! {
    a_step_runs_approved_text_only_while_its_map_leaves_it_unchanged => "unchanged",
    a_step_runs_approved_text_after_an_unrelated_environment_change => "unrelated",
    a_step_refuses_text_changed_by_its_captured_environment => "command",
}

/// A prompt task's document, the file it transcludes, and the provider it
/// launches all read the map of the leg it runs in.
#[cfg(unix)]
#[test]
fn a_prompt_task_and_its_transclusion_read_their_legs_map() {
    let fixture = Fixture::new("doc-env-prompt", &["step", "left", "right"]);
    let launches = common::launch_recorder::install(
        &fixture.inner,
        "claude",
        "printf '%s' \"$DOC_MARK\" > \"$dir/env-$n\"\n\
         printf '%s\\n' '{\"type\":\"result\",\"subtype\":\"success\",\"result\":\"done\",\"session_id\":\"s\",\"is_error\":false}'",
    );
    fixture.write_doc("child.md", &format!("child {READS}\n"));
    fixture.write_doc("prompt.md", &format!("---\n---\nprompt {READS}\n\n::file ./child.md\n"));
    let sequence = fixture.write_doc(
        "seq.md",
        "---\nsequence:\n  - name: grouped\n    group:\n      name: legs\n      max_parallel: 1\n      tasks:\n        - name: left\n          prompt: ./prompt.md\n        - name: right\n          prompt: ./prompt.md\n---\n",
    );

    let run = fixture.run(
        &["sequence", "--claude", sequence.to_str().unwrap()],
        &[fixture.map("step"), fixture.map("left"), fixture.map("right")],
    );

    assert_eq!(run.code, Some(0), "{}", run.output);
    let prompts = common::launch_recorder::prompts(&launches);
    assert_eq!(prompts.len(), 2, "{}", run.output);
    for (prompt, leg) in prompts.iter().zip(["left", "right"]) {
        let reads = fixture.reads(leg);
        assert!(prompt.contains(&format!("prompt {reads}")), "{leg}: {prompt}");
        assert!(prompt.contains(&format!("child {reads}")), "{leg}'s transclusion: {prompt}");
    }
    for (n, leg) in ["left", "right"].iter().enumerate() {
        assert_eq!(
            fs::read_to_string(launches.join(format!("env-{n}"))).unwrap(),
            *leg,
            "the provider launched for {leg} starts from its map"
        );
    }
    assert_eq!(fixture.captures(), 3);
}

/// A failure handler that retries re-enters the target it belongs to: it reads
/// the target's map, and the retried attempt captures nothing new.
#[cfg(unix)]
#[test]
fn a_failure_handler_and_its_retry_read_the_documents_map() {
    let fixture = Fixture::new("doc-env-failure", &["target"]);
    // The first launch fails; every later one succeeds.
    common::launch_recorder::install(
        &fixture.inner,
        "claude",
        "if [ \"$n\" -eq 0 ]; then exit 1; fi\n\
         printf '%s\\n' '{\"type\":\"result\",\"subtype\":\"success\",\"result\":\"done\",\"session_id\":\"s\",\"is_error\":false}'",
    );
    fixture.write_doc(
        "target.md",
        &format!(
            "---\nfailure:\n        - append_line: [\"events.log\", 'target failure {READS}']\n        - retry: 1\n{}{}---\ntarget body\n",
            trail("success", "target success"),
            trail("finalize", "target finalize"),
        ),
    );
    let source = fixture.write_doc(
        "source.md",
        &format!("---\n{}---\nsource body\n", trail_then_proxy("initialize", "source initialize")),
    );

    let run = fixture.run(
        &["compose", "--claude", source.to_str().unwrap()],
        &[fixture.map("target")],
    );

    assert_eq!(run.code, Some(0), "{}", run.output);
    let target = fixture.reads("target");
    assert_eq!(
        fixture.events(),
        vec![
            format!("source initialize {}", fixture.reads("launch")),
            format!("target failure {target}"),
            format!("target success {target}"),
            format!("target finalize {target}"),
        ]
    );
    assert_eq!(fixture.captures(), 1, "the retried attempt captures nothing new");
}

/// How a shell command reads `DOC_MARK` from its own environment, never
/// through an expression, so its text is identical under every map.
#[cfg(unix)]
const SHELL_MARK: &str = "$DOC_MARK";
#[cfg(windows)]
const SHELL_MARK: &str = "%DOC_MARK%";

/// Each command in a shell task runs under the task's captured environment.
fn shell_task(label: &str, indent: &str) -> String {
    let echo = |stage: &str| format!("echo {label}-{stage} {SHELL_MARK}>>events.log");
    format!(
        "shell:\n{indent}  - '{setup}'\n{indent}  - '{shell}'\n{indent}  - '{teardown}'\n",
        shell = echo("task"), setup = echo("setup"), teardown = echo("teardown"),
    )
}

/// What a [`shell_task`] labelled `label` appends under map `map`, in order.
fn shell_task_events(_fixture: &Fixture, label: &str, map: &str) -> Vec<String> {
    vec![
        format!("{label}-setup {map}"),
        format!("{label}-task {map}"),
        format!("{label}-teardown {map}"),
    ]
}

/// Every command in a shell list reads the step's captured environment.
#[test]
fn task_shells_start_from_the_map_their_step_captured() {
    let fixture = Fixture::new("doc-env-task-shell", &["a", "b"]);
    let sequence = fixture.write_doc(
        "seq.md",
        &format!(
            "---\nsequence:\n  - name: first\n    {}  - name: second\n    {}---\n",
            shell_task("first", "    "),
            shell_task("second", "    "),
        ),
    );

    let run = fixture.run(
        &["sequence", "--claude", "--yolo", sequence.to_str().unwrap()],
        &[fixture.map("a"), fixture.map("b")],
    );

    assert_eq!(run.code, Some(0), "{}", run.output);
    let mut expected = shell_task_events(&fixture, "first", "a");
    expected.extend(shell_task_events(&fixture, "second", "b"));
    assert_eq!(fixture.events(), expected);
    assert_eq!(fixture.captures(), 2, "one capture per step");
}

#[test]
fn bounded_group_leg_shells_start_from_their_legs_map() {
    group_leg_shells_row("doc-env-serial-shell", "serial");
}

#[test]
fn parallel_group_leg_shells_start_from_their_legs_map() {
    group_leg_shells_row("doc-env-parallel-shell", "parallel");
}

/// A group step whose two `shell:` legs each run under the map they captured.
fn group_leg_shells_row(name: &str, execution: &str) {
    let execution = if execution == "serial" { "max_parallel: 1\n      " } else { "" };
    let fixture = Fixture::new(name, &["step", "left", "right"]);
    let sequence = fixture.write_doc(
        "seq.md",
        &format!(
            "---\nsequence:\n  - name: grouped\n    group:\n      name: legs\n      {execution}tasks:\n        - name: left\n          {}        - name: right\n          {}---\n",
            // One log per leg: on Windows, two legs appending to one file at
            // once fail with a sharing violation.
            shell_task("left", "          ").replace("events.log", "left.log"),
            shell_task("right", "          ").replace("events.log", "right.log"),
        ),
    );

    let run = fixture.run(
        &["sequence", "--claude", "--yolo", sequence.to_str().unwrap()],
        &[fixture.map("step"), fixture.map("left"), fixture.map("right")],
    );

    assert_eq!(run.code, Some(0), "{}", run.output);
    let mut events: Vec<String> = ["left.log", "right.log"]
        .iter()
        .flat_map(|log| fs::read_to_string(fixture.cwd().join(log)).unwrap_or_default()
            .lines().map(str::to_string).collect::<Vec<_>>())
        .collect();
    // Parallel legs interleave; what each child saw is the contract.
    events.sort();
    let mut expected = shell_task_events(&fixture, "left", "left");
    expected.extend(shell_task_events(&fixture, "right", "right"));
    expected.sort();
    assert_eq!(events, expected);
    assert_eq!(fixture.captures(), 3, "the step and each leg capture once");
}

/// Each body step selects its model from the map it captured, not the one
/// preflight planned under the launch map. The step's
/// `::shell` child, its `start` shell action, and a `set` value's `$( … )`
/// all start from that map too.
#[cfg(unix)]
#[test]
fn a_later_step_selects_its_model_and_starts_its_shells_from_its_own_map() {
    let fixture = Fixture::new("doc-env-step-model", &["first-map", "second-map"]);
    let launches = common::launch_recorder::install(
        &fixture.inner,
        "claude",
        "printf '%s\\n' '{\"type\":\"result\",\"subtype\":\"success\",\"result\":\"done\",\"session_id\":\"s\",\"is_error\":false}'",
    );
    fixture.write_doc(
        "step.md",
        &format!(
            "---\nstart:\n        - set:\n              shell_mark: \"$(printenv DOC_MARK)\"\n        - shell: 'echo start-shell $DOC_MARK>>events.log'\n        - append_line: [\"events.log\", 'start set={{{{ shell_mark }}}} {READS}']\n---\nmodel={{{{ env.MODEL }}}} {READS}\n\n::shell printenv DOC_MARK\n"
        ),
    );
    let sequence = fixture.write_doc(
        "seq.md",
        "---\nsequence:\n  - { name: first, prompt: step.md }\n  - { name: second, prompt: step.md }\n---\n",
    );
    let map = |name: &str, model: &str| {
        let mut map = fixture.map(name);
        map["MODEL"] = json!(model);
        map
    };

    let run = fixture.run_launched_with(
        &["sequence", "--claude", "--yolo", sequence.to_str().unwrap()],
        &[map("first-map", "model-a"), map("second-map", "model-b")],
        &[("MODEL", "model-launch")],
    );

    assert_eq!(run.code, Some(0), "{}", run.output);
    let argv = common::launch_recorder::launches(&launches);
    let prompts = common::launch_recorder::prompts(&launches);
    assert_eq!(argv.len(), 2, "{}", run.output);
    for ((args, prompt), (map, model)) in argv
        .iter()
        .zip(&prompts)
        .zip([("first-map", "model-a"), ("second-map", "model-b")])
    {
        assert_eq!(
            common::launch_recorder::occurrences(args, &["--model", model]),
            1,
            "{map} launches with {model}: {args:?}"
        );
        assert!(
            prompt.contains(&format!("model={model} {}", fixture.reads(map))),
            "{map}: {prompt}"
        );
        assert!(prompt.lines().any(|line| line.trim() == map), "{map}'s `::shell`: {prompt}");
    }
    let mut expected = Vec::new();
    for map in ["first-map", "second-map"] {
        expected.push(format!("start-shell {map}"));
        expected.push(format!("start set={map} {}", fixture.reads(map)));
    }
    assert_eq!(fixture.events(), expected);
    assert_eq!(fixture.captures(), 2, "one capture per step");
}

/// Each prompt leg selects its model from the map it captured, not the one
/// preflight planned under the launch map, and `MODEL` in its document names
/// the model it launches with.
#[cfg(unix)]
#[test]
fn each_prompt_leg_selects_its_model_from_its_own_map() {
    let fixture = Fixture::new("doc-env-leg-model", &["step", "left", "right"]);
    let launches = common::launch_recorder::install(
        &fixture.inner,
        "claude",
        "printf '%s\\n' '{\"type\":\"result\",\"subtype\":\"success\",\"result\":\"done\",\"session_id\":\"s\",\"is_error\":false}'",
    );
    fixture.write_doc("prompt.md", "---\n---\nmodel={{ env.MODEL }}\n");
    let sequence = fixture.write_doc(
        "seq.md",
        "---\nsequence:\n  - name: grouped\n    group:\n      name: legs\n      max_parallel: 1\n      tasks:\n        - name: left\n          prompt: ./prompt.md\n        - name: right\n          prompt: ./prompt.md\n---\n",
    );
    let map = |name: &str, model: &str| {
        let mut map = fixture.map(name);
        map["MODEL"] = json!(model);
        map
    };

    let run = fixture.run_launched_with(
        &["sequence", "--claude", sequence.to_str().unwrap()],
        &[map("step", "model-step"), map("left", "model-left"), map("right", "model-right")],
        &[("MODEL", "model-launch")],
    );

    assert_eq!(run.code, Some(0), "{}", run.output);
    let argv = common::launch_recorder::launches(&launches);
    let prompts = common::launch_recorder::prompts(&launches);
    assert_eq!(argv.len(), 2, "{}", run.output);
    for ((args, prompt), model) in argv.iter().zip(&prompts).zip(["model-left", "model-right"]) {
        assert_eq!(
            common::launch_recorder::occurrences(args, &["--model", model]),
            1,
            "{model}: {args:?}"
        );
        assert!(prompt.contains(&format!("model={model}")), "{model}: {prompt}");
    }
    assert_eq!(fixture.captures(), 3);
}

/// A `resume` continues the target's session under the map the target
/// captured: its handler, the re-fired `start`, the follow-up message, and the
/// resumed provider all read it, and the seam's next map is never captured.
#[cfg(unix)]
#[test]
fn a_resumed_attempt_reads_the_map_its_document_captured() {
    let fixture = Fixture::new("doc-env-resume", &["target", "other"]);
    // The first launch opens session `s` and fails; a resume of `s` succeeds.
    let launches = common::launch_recorder::install(
        &fixture.inner,
        "claude",
        "printf '%s' \"$DOC_MARK\" > \"$dir/env-$n\"\n\
         printf '%s\\n' '{\"type\":\"system\",\"subtype\":\"init\",\"session_id\":\"s\",\"model\":\"claude-test\"}'\n\
         if [ \"$n\" -eq 0 ]; then exit 1; fi\n\
         printf '%s\\n' '{\"type\":\"result\",\"subtype\":\"success\",\"result\":\"done\",\"session_id\":\"s\",\"is_error\":false}'",
    );
    fixture.write_doc(
        "target.md",
        &format!(
            "---\n{}failure:\n        - append_line: [\"events.log\", 'target failure {READS}']\n        - resume: 'continue {{{{ env.DOC_MARK }}}}'\n{}---\ntarget body\n",
            trail("start", "target start"),
            trail("success", "target success"),
        ),
    );
    let source = fixture.write_doc(
        "source.md",
        &format!("---\n{}---\nsource body\n", trail_then_proxy("initialize", "source initialize")),
    );

    let run = fixture.run(
        &["compose", "--claude", source.to_str().unwrap()],
        &[fixture.map("target"), fixture.map("other")],
    );

    assert_eq!(run.code, Some(0), "{}", run.output);
    let target = fixture.reads("target");
    assert_eq!(
        fixture.events(),
        vec![
            format!("source initialize {}", fixture.reads("launch")),
            format!("target start {target}"),
            format!("target failure {target}"),
            format!("target start {target}"),
            format!("target success {target}"),
        ],
        "{}",
        run.output
    );
    let argv = common::launch_recorder::launches(&launches);
    assert_eq!(argv.len(), 2, "{}", run.output);
    assert!(
        argv[1].iter().any(|arg| arg == "s"),
        "the second launch resumes session `s`: {argv:?}"
    );
    let prompts = common::launch_recorder::prompts(&launches);
    assert!(prompts[1].contains("continue target"), "the follow-up reads the target's map: {prompts:?}");
    for n in 0..2 {
        assert_eq!(
            fs::read_to_string(launches.join(format!("env-{n}"))).unwrap(),
            "target",
            "launch {n} starts from the target's map"
        );
    }
    assert_eq!(fixture.captures(), 1, "the resumed attempt captures nothing new");
}
