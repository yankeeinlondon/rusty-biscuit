//! Each sequence task runs on its own provider, through the shipped binary.
//!
//! Preflight predicts every `prompt:` task's provider from its own `agent`
//! and gates the run only on a task with none; the launch then runs each
//! task on its own provider and the step status line names the provider that
//! spawned. Every provider here is a recording stub (`common::launch_recorder`),
//! so the tests are Unix-only; nothing reaches a real agent. The launching
//! stubs are Claude and Goose, whose plain-text stdin protocol the recorder
//! speaks; Gemini stands for a provider that is not installed.

use std::fs;
use std::path::{Path, PathBuf};

use crate::common::{CliProcessFixture, launch_recorder, strip_ansi};

/// The stub behavior: record `AGENT` and `MODEL` as the child saw them, then
/// answer with a line naming the launch.
const RECORD_ENV: &str = "printf '%s|%s' \"$AGENT\" \"${MODEL-unset}\" > \"$dir/env-$n\"; printf 'answer %s\\n' \"$n\"";

struct Run {
    code: Option<i32>,
    stderr: String,
    /// `(binary, AGENT|MODEL, argv and stdin)` per launch, in launch order.
    launches: Vec<(String, String, String)>,
}

fn stage(name: &str, files: &[(&str, &str)], stubs: &[&str]) -> (CliProcessFixture, PathBuf) {
    let fixture = CliProcessFixture::named(name);
    fixture.seed_user_config();
    let mut record = PathBuf::new();
    for stub in stubs {
        record = launch_recorder::install(&fixture, stub, RECORD_ENV);
    }
    for (file, text) in files {
        let path = fixture.cwd().join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
    (fixture, record)
}

fn run(fixture: &CliProcessFixture, record: &Path, args: &[&str]) -> Run {
    // The default PATH is the fixture's stubs plus the system set, which
    // installs no provider of its own.
    let output = fixture
        .command()
        .args(["sequence", "seq.md", "--yolo"])
        .args(args)
        .output()
        .unwrap();
    let envs: Vec<String> = (0..)
        .map_while(|n| fs::read_to_string(record.join(format!("env-{n}"))).ok())
        .collect();
    let launches = launch_recorder::launches(record)
        .into_iter()
        .zip(envs)
        .zip(launch_recorder::prompts(record))
        // A provider gets its prompt on stdin or in argv (Goose's `--text`).
        .map(|((argv, env), prompt)| (argv[0].clone(), env, format!("{} {}", argv[1..].join(" "), prompt.trim())))
        .collect();
    Run {
        code: output.status.code(),
        // Rendered lines wrap at the terminal width.
        stderr: strip_ansi(&String::from_utf8_lossy(&output.stderr)).split_whitespace().collect::<Vec<_>>().join(" "),
        launches,
    }
}

fn binaries(run: &Run) -> Vec<&str> {
    run.launches.iter().map(|(binary, _, _)| binary.as_str()).collect()
}

/// Acceptance 4: a looping sequence with no document `agent`; one task passes
/// its `agent` through `with`, one runs a prompt declaring its own. The run
/// starts with no picker or error, each stub receives only its own task's
/// prompt on every pass, and each status line names it.
#[test]
fn tasks_name_their_providers_in_a_looping_sequence_with_no_document_agent() {
    let (fixture, record) = stage(
        "seq-task-providers-ac4",
        &[
            ("implement.md", "Token IMPLEMENT.\n"),
            ("review.md", "---\nagent: claude\n---\nToken REVIEW.\n"),
            (
                "seq.md",
                "---\nloop:\n  max: 2\n  until: 'loop.count >= 2'\nsequence:\n  - name: implement\n    prompt: implement.md\n    with:\n      agent: goose\n  - name: review\n    prompt: review.md\n---\n",
            ),
        ],
        &["claude", "goose"],
    );
    let run = run(&fixture, &record, &[]);
    assert_eq!(run.code, Some(0), "stderr:\n{}", run.stderr);
    assert!(!run.stderr.contains("agent resolution failed"), "stderr:\n{}", run.stderr);
    assert_eq!(binaries(&run), ["goose", "claude", "goose", "claude"], "stderr:\n{}", run.stderr);
    for (binary, env, prompt) in &run.launches {
        let token = if binary == "goose" { "IMPLEMENT" } else { "REVIEW" };
        assert!(prompt.contains(token), "{binary} received another task's prompt: {prompt}");
        assert!(env.starts_with(&format!("{binary}|")), "{binary} launched with AGENT={env}");
    }
    assert_eq!(run.stderr.matches("step 1/2 succeeded (via Goose)").count(), 2, "stderr:\n{}", run.stderr);
    assert_eq!(run.stderr.matches("step 2/2 succeeded (via Claude)").count(), 2, "stderr:\n{}", run.stderr);
}

/// Acceptance 5, the caller row: a caller setter `agent` outranks a task's
/// `with.agent`, and a nested file-reference sequence's own `agent` applies
/// to its tasks ahead of the root's.
#[test]
fn a_caller_agent_outranks_with_and_a_nested_sequence_agent_applies_to_its_tasks() {
    let files: &[(&str, &str)] = &[
        ("plain.md", "Token PLAIN.\n"),
        ("nested/inner.md", "---\nagent: goose\nsequence:\n  - name: deep\n    prompt: ../plain.md\n---\n"),
        (
            "seq.md",
            "---\nagent: claude\nsequence:\n  - name: first\n    prompt: plain.md\n    with:\n      agent: goose\n  - name: outer\n    prompt: plain.md\n  - name: inner\n    sequence: nested/inner.md\n---\n",
        ),
    ];
    let (fixture, record) = stage("seq-task-providers-precedence", files, &["claude", "goose"]);
    let plain = run(&fixture, &record, &[]);
    assert_eq!(plain.code, Some(0), "stderr:\n{}", plain.stderr);
    assert_eq!(binaries(&plain), ["goose", "claude", "goose"], "stderr:\n{}", plain.stderr);

    let (fixture, record) = stage("seq-task-providers-caller", files, &["claude", "goose"]);
    let caller = run(&fixture, &record, &["--set", r#"{"agent":"claude"}"#]);
    assert_eq!(caller.code, Some(0), "stderr:\n{}", caller.stderr);
    assert_eq!(binaries(&caller), ["claude", "claude", "claude"], "stderr:\n{}", caller.stderr);
}

/// Acceptance 5, model rows: switching provider drops the sequence's model,
/// which belongs to the provider it names, and no stale `MODEL` reaches the
/// child; a task's own `model` still applies.
#[test]
fn a_provider_switch_drops_the_inherited_model() {
    let (fixture, record) = stage(
        "seq-task-providers-model",
        &[
            ("plain.md", "Token PLAIN.\n"),
            ("own.md", "---\nmodel: own-model\n---\nToken OWN.\n"),
            (
                "seq.md",
                "---\nagent: claude\nmodel: claude-model\nsequence:\n  - name: same\n    prompt: plain.md\n  - name: switched\n    prompt: plain.md\n    with:\n      agent: goose\n  - name: owned\n    prompt: own.md\n    with:\n      agent: goose\n---\n",
            ),
        ],
        &["claude", "goose"],
    );
    let run = run(&fixture, &record, &[]);
    assert_eq!(run.code, Some(0), "stderr:\n{}", run.stderr);
    let envs: Vec<&str> = run.launches.iter().map(|(_, env, _)| env.as_str()).collect();
    assert_eq!(envs, ["claude|claude-model", "goose|unset", "goose|own-model"], "stderr:\n{}", run.stderr);
}

/// Acceptance 6: a task naming a provider that is not installed fails
/// preflight, naming the task and its source, before any task launches, even
/// when its `when` is false.
#[test]
fn an_uninstalled_task_provider_fails_before_any_task_launches() {
    let (fixture, record) = stage(
        "seq-task-providers-uninstalled",
        &[
            ("plain.md", "Token PLAIN.\n"),
            (
                "seq.md",
                "---\nagent: claude\nsequence:\n  - name: first\n    prompt: plain.md\n  - name: later\n    prompt: plain.md\n    when: 'false'\n    with:\n      agent: gemini\n---\n",
            ),
        ],
        &["claude"],
    );
    let run = run(&fixture, &record, &[]);
    assert_eq!(run.code, Some(1), "stderr:\n{}", run.stderr);
    assert!(run.launches.is_empty(), "a task launched: {:?}", run.launches);
    assert!(run.stderr.contains("task provider selection failed"), "stderr:\n{}", run.stderr);
    assert!(run.stderr.contains("tasks[1].with.agent"), "stderr:\n{}", run.stderr);
    assert!(run.stderr.contains("seq.md"), "stderr:\n{}", run.stderr);
}

/// Acceptance 6: a graph of only shell and side-effect tasks runs without a
/// provider, a picker, or a provider-resolution error.
#[test]
fn a_shell_and_side_effect_graph_needs_no_provider() {
    let (fixture, record) = stage(
        "seq-task-providers-shell-only",
        &[(
            "seq.md",
            "---\nsequence:\n  - name: stage\n    shell: \"printf staged > staged.flag\"\n  - name: note\n    side_effect:\n      append_line: ['notes.log', 'noted']\n---\n",
        )],
        &[],
    );
    let run = run(&fixture, &record, &[]);
    assert_eq!(run.code, Some(0), "stderr:\n{}", run.stderr);
    assert!(fixture.cwd().join("staged.flag").exists(), "stderr:\n{}", run.stderr);
    assert!(!run.stderr.contains("agent resolution failed"), "stderr:\n{}", run.stderr);
}

/// Acceptance 8: a task whose `agent` reads `seq.outputs` is decided at
/// launch, does not gate the run (there is no document `agent`), and
/// launches on the value composed at its turn. A skipped task claims no
/// provider.
#[test]
fn a_task_agent_read_from_an_earlier_output_is_decided_at_launch() {
    let (fixture, record) = stage(
        "seq-task-providers-at-launch",
        &[
            ("plain.md", "Token PLAIN.\n"),
            (
                "seq.md",
                "---\nsequence:\n  - name: pick\n    shell: echo goose\n  - name: run\n    prompt: plain.md\n    with:\n      agent: \"{{ seq.outputs[0] }}\"\n  - name: never\n    prompt: plain.md\n    when: 'false'\n    with:\n      agent: goose\n---\n",
            ),
        ],
        &["claude", "goose"],
    );
    let run = run(&fixture, &record, &[]);
    assert_eq!(run.code, Some(0), "stderr:\n{}", run.stderr);
    assert_eq!(binaries(&run), ["goose"], "stderr:\n{}", run.stderr);
    assert!(run.launches[0].1.starts_with("goose|"), "AGENT={}", run.launches[0].1);
    assert!(run.stderr.contains("step 2/3 succeeded (via Goose)"), "stderr:\n{}", run.stderr);
    assert!(!run.stderr.contains("step 3/3 succeeded (via"), "a skipped task claimed a provider:\n{}", run.stderr);
}

/// Acceptance 8: a runtime-mutated value is decided at launch too: the first
/// task's `success` writes `chosen`, and the second task's `agent` reads it.
#[test]
fn a_task_agent_read_from_a_runtime_write_is_decided_at_launch() {
    let (fixture, record) = stage(
        "seq-task-providers-runtime-write",
        &[
            ("first.md", "---\nagent: claude\nsuccess:\n  - set:\n      chosen: goose\n---\nToken FIRST.\n"),
            ("second.md", "Token SECOND.\n"),
            (
                "seq.md",
                "---\nchosen: claude\nsequence:\n  - name: first\n    prompt: first.md\n  - name: second\n    prompt: second.md\n    with:\n      agent: \"{{ chosen }}\"\n---\n",
            ),
        ],
        &["claude", "goose"],
    );
    let run = run(&fixture, &record, &[]);
    assert_eq!(run.code, Some(0), "stderr:\n{}", run.stderr);
    assert_eq!(binaries(&run), ["claude", "goose"], "stderr:\n{}", run.stderr);
}

/// Acceptance 8: an unavailable dynamic hint fails before that task spawns;
/// the earlier task has already run.
#[test]
fn an_unavailable_dynamic_hint_fails_before_its_task_spawns() {
    let (fixture, record) = stage(
        "seq-task-providers-dynamic-unavailable",
        &[
            ("plain.md", "Token PLAIN.\n"),
            (
                "seq.md",
                "---\nagent: claude\nsequence:\n  - name: first\n    prompt: plain.md\n  - name: pick\n    shell: echo gemini\n  - name: run\n    prompt: plain.md\n    with:\n      agent: \"{{ seq.outputs[1] }}\"\n---\n",
            ),
        ],
        &["claude"],
    );
    let run = run(&fixture, &record, &[]);
    assert_eq!(run.code, Some(1), "stderr:\n{}", run.stderr);
    assert_eq!(binaries(&run), ["claude"], "the dynamic task spawned; stderr:\n{}", run.stderr);
    assert!(run.stderr.contains("Gemini is not installed"), "stderr:\n{}", run.stderr);
}

/// Acceptance 8: a retry whose `with` switches the provider reports each
/// attempt's actual identity, and the completion line names the final one.
#[test]
fn a_retry_that_switches_provider_reports_every_attempt() {
    let fixture = CliProcessFixture::named("seq-task-providers-retry");
    fixture.seed_user_config();
    // The first launch fails, so the document's `failure` stack retries it.
    let record = launch_recorder::install(&fixture, "claude", "exit 1");
    launch_recorder::install(&fixture, "goose", "printf 'answer %s\\n' \"$n\"");
    fs::write(
        fixture.cwd().join("flaky.md"),
        "---\nagent: claude\nfailure:\n  - {retry: {max_attempts: 1, with: {agent: goose}}}\n---\nToken FLAKY.\n",
    )
    .unwrap();
    fs::write(fixture.cwd().join("seq.md"), "---\nsequence:\n  - name: flaky\n    prompt: flaky.md\n---\n").unwrap();
    let output = fixture.command().args(["sequence", "seq.md", "--yolo"]).output().unwrap();
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    let launched: Vec<String> = launch_recorder::launches(&record).into_iter().map(|argv| argv[0].clone()).collect();
    assert_eq!(launched, ["claude", "goose"], "stderr:\n{stderr}");
    assert_eq!(output.status.code(), Some(0), "stderr:\n{stderr}");
    assert!(
        stderr.contains("step 1/1 succeeded (via Goose; attempts: Claude → Goose)"),
        "stderr:\n{stderr}"
    );
}

/// Preflight's dry-run rows show each task's prediction, or that its turn
/// decides it, and a preview reports no provider as having run.
#[test]
fn a_dry_run_lists_each_tasks_predicted_provider() {
    let (fixture, record) = stage(
        "seq-task-providers-dry-run",
        &[
            ("plain.md", "Token PLAIN.\n"),
            ("own.md", "---\nagent: goose\n---\nToken OWN.\n"),
            (
                "seq.md",
                "---\nagent: claude\nsequence:\n  - name: own\n    prompt: own.md\n  - name: inherits\n    prompt: plain.md\n  - name: pick\n    shell: echo goose\n  - name: dynamic\n    prompt: plain.md\n    with:\n      agent: \"{{ seq.outputs[2] }}\"\n---\n",
            ),
        ],
        &["claude", "goose"],
    );
    let run = run(&fixture, &record, &["--dry-run"]);
    assert_eq!(run.code, Some(0), "stderr:\n{}", run.stderr);
    assert!(run.launches.is_empty(), "a preview launched: {:?}", run.launches);
    for row in ["own → Goose", "inherits → Claude (from the sequence)", "dynamic → decided at launch"] {
        assert!(run.stderr.contains(row), "missing `{row}`; stderr:\n{}", run.stderr);
    }
    assert!(!run.stderr.contains("succeeded (via"), "a preview claimed a provider:\n{}", run.stderr);
}

/// Acceptance 8, concurrent legs: a leg's `agent` decided at launch reads the
/// group's start snapshot, so a sibling's runtime write never moves it.
#[test]
fn a_concurrent_leg_hint_reads_the_group_start_snapshot() {
    let (fixture, record) = stage(
        "seq-task-providers-leg-snapshot",
        &[
            ("first.md", "---\nagent: claude\nsuccess:\n  - set:\n      chosen: goose\n---\nToken FIRST.\n"),
            ("sibling.md", "---\nagent: claude\nsuccess:\n  - set:\n      chosen: claude\n---\nToken SIBLING.\n"),
            ("leg.md", "Token LEG.\n"),
            (
                "seq.md",
                "---\nchosen: claude\nsequence:\n  - name: first\n    prompt: first.md\n  - name: batch\n    group:\n      tasks:\n        - name: sibling\n          prompt: sibling.md\n        - name: leg\n          prompt: leg.md\n          with:\n            agent: \"{{ chosen }}\"\n---\n",
            ),
        ],
        &["claude", "goose"],
    );
    let run = run(&fixture, &record, &[]);
    assert_eq!(run.code, Some(0), "stderr:\n{}", run.stderr);
    let leg: Vec<&str> = run
        .launches
        .iter()
        .filter(|(_, _, prompt)| prompt.contains("LEG"))
        .map(|(binary, _, _)| binary.as_str())
        .collect();
    assert_eq!(leg, ["goose"], "stderr:\n{}", run.stderr);
}
