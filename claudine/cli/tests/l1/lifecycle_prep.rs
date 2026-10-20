//! `prep` observed through the CLI: the target runs to completion as an
//! invocation of its own and the caller's event continues; only the target's
//! effects on disk reach the caller.
//!
//! Events append to `trace.log`; documents launch a recording stub provider.

use std::fs;
use std::time::Duration;

use crate::common::signal::SignalledRun;
use crate::common::{CliProcessFixture, launch_recorder, strip_ansi};

struct Run {
    status: std::process::ExitStatus,
    trace: Vec<String>,
    prompts: Vec<String>,
    stderr: String,
}

impl Run {
    /// Stderr with the status block's border and line wrapping removed.
    fn prose(&self) -> String {
        self.stderr.replace('┃', " ").split_whitespace().collect::<Vec<_>>().join(" ")
    }
}

/// Run `claudine <command> --claude --yolo <entry>` with `files` written into
/// the fixture (a name may include a directory).
fn run(command: &str, files: &[(&str, &str)], entry: &str) -> Run {
    let fixture = CliProcessFixture::named("lifecycle-prep");
    for (name, text) in files {
        let path = fixture.cwd().join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
    let recordings = launch_recorder::install(&fixture, "claude", "printf 'ok\\n'");
    let output = fixture
        .command()
        .args([command, "--claude", "--yolo"])
        .arg(fixture.cwd().join(entry))
        .output()
        .unwrap();
    let trace = fs::read_to_string(fixture.cwd().join("trace.log"))
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect();
    let prompts = launch_recorder::prompts(&recordings).iter().map(|prompt| prompt.trim().to_string()).collect();
    assert!(!fixture.audio_spool().exists());
    Run { status: output.status, trace, prompts, stderr: strip_ansi(&String::from_utf8_lossy(&output.stderr)) }
}

const HELPER: &str = r#"---
stage: none
initialize:
  - append_line: [trace.log, "helper initialize {{ stage }}"]
finalize:
  - append_line: [trace.log, "helper finalize"]
---
Helper for {{ stage }}
"#;

/// After the target returns, the event continues with the next item, in the
/// same list and after the conditional branch that made the call. The
/// overlay reaches the target; calling the same helper again is legal; the
/// target resolves relative to the document that authored the directive.
#[test]
fn prep_returns_to_the_next_item_and_a_repeated_call_is_legal() {
    let caller = r#"---
start:
  - append_line: [trace.log, "caller before"]
  - prep: { target: "helper.md", with: { stage: "all" } }
  - append_line: [trace.log, "caller between"]
  - when: "true"
    then:
      - prep: "helper.md"
  - append_line: [trace.log, "caller after"]
finalize:
  - append_line: [trace.log, "caller finalize"]
---
Caller body
"#;
    let decoy = "---\ninitialize:\n  - append_line: [trace.log, \"wrong helper\"]\n---\nWrong\n";
    let run = run("compose", &[("docs/caller.md", caller), ("docs/helper.md", HELPER), ("helper.md", decoy)], "docs/caller.md");
    assert_eq!(run.status.code(), Some(0), "stderr:\n{}", run.stderr);
    assert_eq!(
        run.trace,
        [
            "caller before",
            "helper initialize all",
            "helper finalize",
            "caller between",
            "helper initialize none",
            "helper finalize",
            "caller after",
            "caller finalize",
        ]
    );
    assert_eq!(run.prompts, ["Helper for all", "Helper for none", "Caller body"]);
}

/// The target's output is reported under its own invocation only: the
/// caller's `seq.outputs`, frontmatter, and runtime layer are unchanged, and
/// none of the caller's runtime roots reach the target.
#[test]
fn prep_returns_disk_only_with_isolated_runtime_roots() {
    let caller = r#"---
leak: unchanged
start:
  - prep: "@helper.md"
---
Caller sees {{ leak }}
"#;
    let helper = r#"---
initialize:
  - append_line: [trace.log, "helper outputs {{ length(seq.outputs) }}"]
  - set: { leak: "from helper" }
---
Helper body
"#;
    let seq = r#"---
leak: unchanged
sequence:
  - name: first
    side_effect: { append_line: [trace.log, "first"] }
  - name: caller
    prompt: "@caller.md"
  - name: after
    side_effect: { append_line: [trace.log, "outputs {{ length(seq.outputs) }} {{ leak }}"] }
---
"#;
    let run = run("sequence", &[("caller.md", caller), ("helper.md", helper), ("seq.md", seq)], "seq.md");
    assert_eq!(run.status.code(), Some(0), "stderr:\n{}", run.stderr);
    assert_eq!(run.trace, ["first", "helper outputs 0", "outputs 2 unchanged"]);
    assert_eq!(run.prompts, ["Helper body", "Caller sees unchanged"]);
}

/// A `prep` from `initialize` prepares and runs the target first; the
/// caller's own preflight and composition then read the files it wrote.
#[test]
fn prep_from_initialize_runs_before_the_callers_preparation() {
    let caller = r#"---
initialize:
  - prep: "@writer.md"
---
Value {{ frontmatter('note.md', 'value') }}
"#;
    let writer = "---\ninitialize:\n  - set_frontmatter: [note.md, value, after]\n  - skip\n---\nNever sent\n";
    let note = "---\nvalue: before\n---\n";
    let run = run("compose", &[("caller.md", caller), ("writer.md", writer), ("note.md", note)], "caller.md");
    assert_eq!(run.status.code(), Some(0), "stderr:\n{}", run.stderr);
    // The writer skipped from its `initialize`: a successful call that
    // launched nothing.
    assert_eq!(run.prompts, ["Value after"]);
}

/// A target that fails fails the caller's event with the target's failure as
/// its cause.
#[test]
fn a_failed_target_fails_the_callers_event_with_its_cause() {
    let caller = "---\nstart:\n  - prep: \"@broken.md\"\n  - append_line: [trace.log, \"never\"]\n---\nCaller\n";
    let broken = "---\nstart:\n  - error: \"helper broke\"\n---\nBroken\n";
    let run = run("compose", &[("caller.md", caller), ("broken.md", broken)], "caller.md");
    assert_eq!(run.status.code(), Some(1), "stderr:\n{}", run.stderr);
    assert!(run.trace.is_empty(), "the event ends at the failed call: {:?}", run.trace);
    assert!(run.prompts.is_empty(), "nothing launches: {:?}", run.prompts);
    let prose = run.prose();
    assert!(prose.contains("`prep` of `@broken.md` failed"), "stderr:\n{}", run.stderr);
    assert!(prose.contains("helper broke"), "the cause is attached; stderr:\n{}", run.stderr);
}

/// Calls and handoffs share one cycle domain: a target cannot call, or hand
/// off to, a document with an active call.
#[test]
fn a_call_or_handoff_back_to_an_active_caller_is_refused() {
    let a = "---\nstart:\n  - prep: \"@b.md\"\n---\nA\n";
    let b_calls = "---\nstart:\n  - prep: \"@a.md\"\n---\nB\n";
    let run_calls = run("compose", &[("a.md", a), ("b.md", b_calls)], "a.md");
    assert_eq!(run_calls.status.code(), Some(1), "stderr:\n{}", run_calls.stderr);
    assert!(run_calls.prose().contains("`prep` of `@a.md` would re-enter an active call"), "stderr:\n{}", run_calls.stderr);
    assert!(run_calls.prompts.is_empty(), "{:?}", run_calls.prompts);

    let b_proxies = "---\nstart:\n  - proxy: \"@a.md\"\n---\nB\n";
    let run_proxy = run("compose", &[("a.md", a), ("b.md", b_proxies)], "a.md");
    assert_eq!(run_proxy.status.code(), Some(1), "stderr:\n{}", run_proxy.stderr);
    assert!(run_proxy.prose().contains("forms a cycle or exceeds the"), "stderr:\n{}", run_proxy.stderr);
    assert!(run_proxy.prompts.is_empty(), "{:?}", run_proxy.prompts);
}

/// A target's `break` cannot reach a loop of the caller's: a target ending
/// its own loop with code `0` returns normally and the caller's loop goes on,
/// while a non-zero code fails the call.
#[test]
fn a_targets_break_stays_in_the_target_and_its_code_decides_the_call() {
    let looping_target = |code: u8| {
        format!("---\nloop:\n  until: \"false\"\n  max: 5\nstart:\n  - break: {{ code: {code} }}\n---\nNever sent\n")
    };
    let seq = r#"---
loop:
  until: "loop.count >= 2"
  max: 3
sequence:
  - name: call
    side_effect: { append_line: [trace.log, "pass {{ loop.count }}"] }
  - name: caller
    prompt: "@caller.md"
---
"#;
    let caller = "---\nstart:\n  - prep: \"@target.md\"\n---\nCaller {{ loop.count }}\n";
    let returns = run("sequence", &[("seq.md", seq), ("caller.md", caller), ("target.md", &looping_target(0))], "seq.md");
    assert_eq!(returns.status.code(), Some(0), "stderr:\n{}", returns.stderr);
    assert_eq!(returns.trace, ["pass 1", "pass 2"], "the caller's loop ran both passes");
    assert_eq!(returns.prompts, ["Caller 1", "Caller 2"]);

    let fails = run("sequence", &[("seq.md", seq), ("caller.md", caller), ("target.md", &looping_target(5))], "seq.md");
    assert_eq!(fails.status.code(), Some(1), "stderr:\n{}", fails.stderr);
    assert!(fails.prose().contains("it exited with status 5"), "stderr:\n{}", fails.stderr);
    assert_eq!(fails.trace, ["pass 1"]);
}

/// Ctrl-C during a target cancels the run: the call never returns
/// successfully, and the process exits 130.
#[test]
fn ctrl_c_during_a_target_cancels_the_run() {
    let fixture = CliProcessFixture::named("lifecycle-prep-interrupt");
    fs::write(fixture.cwd().join("caller.md"), "---\nstart:\n  - prep: \"@waiter.md\"\n  - append_line: [trace.log, \"returned\"]\n---\nCaller\n").unwrap();
    fs::write(
        fixture.cwd().join("waiter.md"),
        "---\nstart:\n  - append_line: [started.log, \"started\"]\n  - shell: \"until [ -f never.done ]; do sleep 0.05; done\"\n---\nWaiter\n",
    )
    .unwrap();
    let _recordings = launch_recorder::install(&fixture, "claude", "printf 'ok\\n'");
    let mut command = fixture.command_std();
    command.args(["compose", "--claude", "--yolo"]).arg(fixture.cwd().join("caller.md"));
    let mut run = SignalledRun::spawn(command, fixture.workspace_path());
    run.wait_for_path(&fixture.cwd().join("started.log"), Duration::from_secs(20));
    run.interrupt();
    let exit = run.wait_for_exit(Duration::from_secs(15));
    assert_eq!(exit.status.code(), Some(130), "stderr:\n{}", exit.stderr);
    assert!(!fixture.cwd().join("trace.log").exists(), "the caller's event did not continue");
    assert!(!fixture.audio_spool().exists());
}

/// A target may hand off; the document it hands to owns the rest of the
/// target's run, and control returns to the caller when that run ends.
#[test]
fn a_target_that_hands_off_returns_when_the_handoff_completes() {
    let caller = "---\nstart:\n  - append_line: [trace.log, \"caller before\"]\n  - prep: \"@helper.md\"\n  - append_line: [trace.log, \"caller after\"]\n---\nCaller\n";
    let helper = "---\nstart:\n  - proxy: \"@next.md\"\n---\nHelper\n";
    let next = "---\nfinalize:\n  - append_line: [trace.log, \"next finalize\"]\n---\nNext\n";
    let run = run("compose", &[("caller.md", caller), ("helper.md", helper), ("next.md", next)], "caller.md");
    assert_eq!(run.status.code(), Some(0), "stderr:\n{}", run.stderr);
    assert_eq!(run.trace, ["caller before", "next finalize", "caller after"]);
    assert_eq!(run.prompts, ["Next", "Caller"]);
}

/// A call that fails in `failure` does not re-enter `failure` or replace the
/// original error, and `finalize` still runs once.
#[test]
fn a_failed_call_in_failure_keeps_the_original_error_and_cleanup_runs_once() {
    let caller = r#"---
start:
  - error: "original failure"
failure:
  - append_line: [trace.log, "failure"]
  - prep: "@broken.md"
finalize:
  - append_line: [trace.log, "finalize"]
---
Caller
"#;
    let broken = "---\nstart:\n  - error: \"helper broke\"\n---\nBroken\n";
    let run = run("compose", &[("caller.md", caller), ("broken.md", broken)], "caller.md");
    assert_eq!(run.status.code(), Some(1), "stderr:\n{}", run.stderr);
    assert_eq!(run.trace, ["failure", "finalize"]);
    assert!(run.prose().contains("original failure"), "the original error stays primary; stderr:\n{}", run.stderr);
    assert!(run.prompts.is_empty(), "{:?}", run.prompts);
}

/// Calling the same helper on every pass of an enclosing loop is legal: each
/// call returned and released its place in the chain.
#[test]
fn a_helper_called_on_every_outer_pass_runs_each_time() {
    let seq = r#"---
loop:
  until: "loop.count >= 3"
  max: 3
sequence:
  - name: caller
    prompt: "@caller.md"
---
"#;
    let caller = "---\nstart:\n  - prep: \"@helper.md\"\n---\nCaller {{ loop.count }}\n";
    let helper = "---\nfinalize:\n  - append_line: [trace.log, \"helper\"]\n---\nHelper\n";
    let run = run("sequence", &[("seq.md", seq), ("caller.md", caller), ("helper.md", helper)], "seq.md");
    assert_eq!(run.status.code(), Some(0), "stderr:\n{}", run.stderr);
    assert_eq!(run.trace, ["helper", "helper", "helper"]);
    assert_eq!(run.prompts, ["Helper", "Caller 1", "Helper", "Caller 2", "Helper", "Caller 3"]);
}
