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

/// An expression error in a later task's `with` selection input is a typo,
/// not a later value: preflight fails naming the task's property before the
/// first task launches, for `agent` and `model` alike.
#[test]
fn an_invalid_with_selection_expression_fails_before_any_task_launches() {
    for key in ["agent", "model"] {
        let (fixture, record) = stage(
            &format!("seq-task-providers-invalid-with-{key}"),
            &[
                ("plain.md", "Token PLAIN.\n"),
                (
                    "seq.md",
                    &format!(
                        "---\nagent: claude\nsequence:\n  - name: first\n    prompt: plain.md\n  - name: second\n    prompt: plain.md\n    with:\n      {key}: '{{{{ no_such_function() }}}}'\n---\n"
                    ),
                ),
            ],
            &["claude"],
        );
        let run = run(&fixture, &record, &[]);
        assert_eq!(run.code, Some(1), "{key}; stderr:\n{}", run.stderr);
        assert!(run.launches.is_empty(), "{key}: a task launched: {:?}", run.launches);
        assert!(run.stderr.contains("task provider selection failed"), "{key}; stderr:\n{}", run.stderr);
        assert!(run.stderr.contains(&format!("tasks[1].with.{key}")), "{key}; stderr:\n{}", run.stderr);
        assert!(run.stderr.contains("seq.md"), "{key}; stderr:\n{}", run.stderr);
        assert!(run.stderr.contains("no_such_function"), "{key}; stderr:\n{}", run.stderr);
    }
}

/// The document-hint control: the same expression in a later task's prompt
/// frontmatter fails before any task launches too (the document's own
/// frontmatter evaluation reports it first), naming the document.
#[test]
fn an_invalid_document_selection_expression_fails_before_any_task_launches() {
    for key in ["agent", "model"] {
        let (fixture, record) = stage(
            &format!("seq-task-providers-invalid-document-{key}"),
            &[
                ("plain.md", "Token PLAIN.\n"),
                ("bad.md", &format!("---\n{key}: '{{{{ no_such_function() }}}}'\n---\nToken BAD.\n")),
                (
                    "seq.md",
                    "---\nagent: claude\nsequence:\n  - name: first\n    prompt: plain.md\n  - name: second\n    prompt: bad.md\n---\n",
                ),
            ],
            &["claude"],
        );
        let run = run(&fixture, &record, &[]);
        assert_eq!(run.code, Some(1), "{key}; stderr:\n{}", run.stderr);
        assert!(run.launches.is_empty(), "{key}: a task launched: {:?}", run.launches);
        assert!(run.stderr.contains("bad.md"), "{key}; stderr:\n{}", run.stderr);
        assert!(run.stderr.contains("no_such_function"), "{key}; stderr:\n{}", run.stderr);
    }
}

/// The expected outcome of one [`selection_cell!`].
enum Expect {
    /// Exit 0, launching exactly these stubs in order.
    Launches(&'static [&'static str]),
    /// Exit 1 before any launch, with this text on stderr.
    FailsBeforeLaunch(&'static str),
}

/// One edit of a single fixture: the sequence names `agent: claude`, and one
/// task runs `task.md` with `frontmatter` and `with:` entries `with` (YAML
/// flow), under caller `args`. Claude and Goose are installed; Gemini is not.
fn selection_cell(name: &str, frontmatter: &str, with: &str, args: &[&str], expect: Expect) {
    let with = if with.is_empty() { String::new() } else { format!("    with: {with}\n") };
    let (fixture, record) = stage(
        &format!("seq-task-providers-cell-{name}"),
        &[
            ("task.md", &format!("---\ntitle: t\n{frontmatter}---\nToken TASK.\n")),
            ("seq.md", &format!("---\nagent: claude\nsequence:\n  - name: only\n    prompt: task.md\n{with}---\n")),
        ],
        &["claude", "goose"],
    );
    let run = run(&fixture, &record, args);
    match expect {
        Expect::Launches(expected) => {
            assert_eq!(run.code, Some(0), "stderr:\n{}", run.stderr);
            assert_eq!(binaries(&run), expected, "stderr:\n{}", run.stderr);
        }
        Expect::FailsBeforeLaunch(text) => {
            assert_eq!(run.code, Some(1), "stderr:\n{}", run.stderr);
            assert!(run.launches.is_empty(), "a task launched: {:?}", run.launches);
            assert!(run.stderr.contains(text), "expected `{text}`; stderr:\n{}", run.stderr);
        }
    }
}

/// One test per cell, so each row's process runs and reports on its own.
macro_rules! selection_cells {
    ($($name:ident: $frontmatter:expr, $with:expr, $args:expr => $expect:expr;)+) => {
        $(
            #[test]
            fn $name() {
                selection_cell(stringify!($name), $frontmatter, $with, $args, $expect);
            }
        )+
    };
}

// `null` is no `agent` of the task's own, wherever it is written: provider
// selection gives the task the enclosing sequence's provider. A caller setter
// (`null` or `[]`) also clears the sequence's own hint, so nothing is left to
// inherit and the ordinary missing-provider gate fails without a terminal. An
// empty string is an invalid name, never absence. `model: null`/`[]` is no
// model hint. A document or `with` `agent: []` is covered below.
selection_cells! {
    empty_hint_frontmatter_null_inherits: "agent: null\n", "", &[] => Expect::Launches(&["claude"]);
    empty_hint_with_null_inherits: "agent: gemini\n", "{ agent: null }", &[] => Expect::Launches(&["claude"]);
    empty_hint_setter_null_uses_the_missing_provider_gate: "agent: goose\n", "", &["agent=null"]
        => Expect::FailsBeforeLaunch("agent resolution failed");
    empty_hint_setter_list_uses_the_missing_provider_gate: "agent: goose\n", "", &["agent=[]"]
        => Expect::FailsBeforeLaunch("agent resolution failed");
    empty_hint_absent_inherits: "", "", &[] => Expect::Launches(&["claude"]);
    empty_hint_empty_string_is_an_invalid_name: "", "{ agent: '' }", &[]
        => Expect::FailsBeforeLaunch("task provider selection failed");
    empty_hint_model_null_and_list_are_no_model: "model: null\n", "{ model: [] }", &[] => Expect::Launches(&["claude"]);
    nonempty_hint_scalar_launches_its_provider: "", "{ agent: goose }", &[] => Expect::Launches(&["goose"]);
    nonempty_hint_list_launches_its_one_installed_member: "", "{ agent: [gemini, goose] }", &[]
        => Expect::Launches(&["goose"]);
}

/// A static `agent: []` in a later task's prompt frontmatter or `with` breaks
/// the launch schema's rule that an agent list needs at least one entry, so
/// preflight fails naming where it was written and no task launches, not even
/// the earlier one. `property` is the source-rooted property, `hint_file` the
/// file that wrote it.
fn assert_empty_agent_list_fails_preflight(name: &str, frontmatter: &str, with: &str, hint_file: &str, property: &str) {
    let with = if with.is_empty() { String::new() } else { format!("    with: {with}\n") };
    let (fixture, record) = stage(
        &format!("seq-task-providers-empty-list-{name}"),
        &[
            ("plain.md", "Token PLAIN.\n"),
            ("task.md", &format!("---\ntitle: t\n{frontmatter}---\nToken TASK.\n")),
            (
                "seq.md",
                &format!(
                    "---\nagent: claude\nsequence:\n  - name: first\n    prompt: plain.md\n  - name: second\n    prompt: task.md\n{with}---\n"
                ),
            ),
        ],
        &["claude", "goose"],
    );
    let run = run(&fixture, &record, &[]);
    assert_eq!(run.code, Some(1), "stderr:\n{}", run.stderr);
    assert!(run.launches.is_empty(), "a task launched, the earlier one included: {:?}", run.launches);
    assert!(run.stderr.contains("task provider selection failed"), "stderr:\n{}", run.stderr);
    // The rendered diagnostic wraps where the temp path's length puts it, so
    // match prose on a border-free, whitespace-collapsed copy and the path on
    // an unbroken fragment.
    let flat = run.stderr.split_whitespace().filter(|word| *word != "┃").collect::<Vec<_>>().join(" ");
    assert!(flat.contains(&format!("`{property}` in")), "stderr:\n{}", run.stderr);
    assert!(run.stderr.contains(&format!("/cwd/{hint_file}")), "stderr:\n{}", run.stderr);
    assert!(flat.contains("needs at least one entry"), "stderr:\n{}", run.stderr);
    assert!(!run.stderr.contains("could not be evaluated"), "stderr:\n{}", run.stderr);
    assert!(!run.stderr.contains("is not valid"), "the launch schema reported it: {}", run.stderr);
}

#[test]
fn empty_hint_frontmatter_list_fails_preflight_before_any_task_launches() {
    assert_empty_agent_list_fails_preflight("frontmatter", "agent: []\n", "", "task.md", "agent");
}

#[test]
fn empty_hint_with_list_fails_preflight_before_any_task_launches() {
    assert_empty_agent_list_fails_preflight("with", "agent: gemini\n", "{ agent: [] }", "seq.md", "tasks[1].with.agent");
}

/// A body step of a referenced sequence document whose `agent` is `[]`
/// composes that document, which the launch schema would reject after the
/// earlier task ran: preflight fails naming the document instead. A caller
/// setter replacing `agent` leaves nothing to reject.
#[test]
fn empty_hint_nested_sequence_list_fails_its_body_step_before_any_task_launches() {
    let files: &[(&str, &str)] = &[
        ("plain.md", "Token PLAIN.\n"),
        ("inner.md", "---\nagent: []\nsequence:\n  - name: body\n---\nToken INNER.\n"),
        (
            "seq.md",
            "---\nagent: claude\nsequence:\n  - name: first\n    prompt: plain.md\n  - name: second\n    sequence: inner.md\n---\n",
        ),
    ];
    let (fixture, record) = stage("seq-task-providers-empty-list-nested", files, &["claude"]);
    let failed = run(&fixture, &record, &[]);
    assert_eq!(failed.code, Some(1), "stderr:\n{}", failed.stderr);
    assert!(failed.launches.is_empty(), "a task launched, the earlier one included: {:?}", failed.launches);
    assert!(failed.stderr.contains("task provider selection failed"), "stderr:\n{}", failed.stderr);
    assert!(failed.stderr.contains("with `agent` in"), "stderr:\n{}", failed.stderr);
    assert!(failed.stderr.contains("/cwd/inner.md"), "stderr:\n{}", failed.stderr);
    assert!(failed.stderr.contains("needs at least one entry"), "stderr:\n{}", failed.stderr);

    let (fixture, record) = stage("seq-task-providers-empty-list-nested-setter", files, &["claude"]);
    let replaced = run(&fixture, &record, &["agent=claude"]);
    assert_eq!(replaced.code, Some(0), "stderr:\n{}", replaced.stderr);
    assert_eq!(binaries(&replaced), ["claude", "claude"], "stderr:\n{}", replaced.stderr);
}

/// A hint reading a key only a lifecycle `set` creates (no static default) is
/// decided at launch: it does not gate the run, and launches on the value
/// written at runtime.
#[test]
fn a_task_agent_read_from_a_key_only_a_runtime_write_creates_is_decided_at_launch() {
    let (fixture, record) = stage(
        "seq-task-providers-runtime-only-write",
        &[
            ("first.md", "---\nagent: claude\nsuccess:\n  - set:\n      chosen: goose\n---\nToken FIRST.\n"),
            ("second.md", "Token SECOND.\n"),
            (
                "seq.md",
                "---\nsequence:\n  - name: first\n    prompt: first.md\n  - name: second\n    prompt: second.md\n    with:\n      agent: \"{{ chosen }}\"\n---\n",
            ),
        ],
        &["claude", "goose"],
    );
    let run = run(&fixture, &record, &[]);
    assert_eq!(run.code, Some(0), "stderr:\n{}", run.stderr);
    assert_eq!(binaries(&run), ["claude", "goose"], "stderr:\n{}", run.stderr);
}

// -- a higher-precedence source makes a circular hint irrelevant ---------------
//
// A hint reading the identity it selects (`env.AGENT` for `agent`, `env.MODEL`
// for `model`) cannot decide a launch, but it only has to when nothing above
// it does. Provider and model precedence apply separately: `--<provider>` /
// `--provider` replaces any `agent` hint; `--model`, a provider-specific model
// variable, or `MODEL` replaces any `model` hint. The composed prompt still
// reads `env.AGENT`/`env.MODEL`, which then name the launched identity.

/// Run `seq.md` with `args` and the extra environment `envs` set on the child.
fn run_with_env(fixture: &CliProcessFixture, record: &Path, args: &[&str], envs: &[(&str, &str)]) -> Run {
    let output = fixture
        .command()
        .args(["sequence", "seq.md", "--yolo"])
        .args(args)
        .envs(envs.iter().copied())
        .output()
        .unwrap();
    let envs: Vec<String> = (0..)
        .map_while(|n| fs::read_to_string(record.join(format!("env-{n}"))).ok())
        .collect();
    let launches = launch_recorder::launches(record)
        .into_iter()
        .zip(envs)
        .zip(launch_recorder::prompts(record))
        .map(|((argv, env), prompt)| (argv[0].clone(), env, format!("{} {}", argv[1..].join(" "), prompt.trim())))
        .collect();
    Run {
        code: output.status.code(),
        stderr: strip_ansi(&String::from_utf8_lossy(&output.stderr)).split_whitespace().collect::<Vec<_>>().join(" "),
        launches,
    }
}

/// Stage one prompt task: `task.md` carries `frontmatter`, the sequence's own
/// frontmatter is `sequence_frontmatter`, and the task's `with:` is `with`
/// (YAML flow, or empty). Claude and Goose are installed.
fn stage_circular(name: &str, sequence_frontmatter: &str, frontmatter: &str, with: &str) -> (CliProcessFixture, PathBuf) {
    let with = if with.is_empty() { String::new() } else { format!("    with: {with}\n") };
    stage(
        &format!("seq-task-providers-override-{name}"),
        &[
            ("task.md", &format!("---\ntitle: t\n{frontmatter}---\nToken TASK.\n")),
            ("seq.md", &format!("---\n{sequence_frontmatter}sequence:\n  - name: only\n    prompt: task.md\n{with}---\n")),
        ],
        &["claude", "goose"],
    )
}

/// The run launched exactly one Claude, with no error on stderr.
fn assert_one_claude(run: &Run) {
    assert_eq!(run.code, Some(0), "stderr:\n{}", run.stderr);
    assert_eq!(binaries(run), ["claude"], "stderr:\n{}", run.stderr);
    assert!(!run.stderr.contains("Error"), "stderr:\n{}", run.stderr);
    assert!(run.launches[0].1.starts_with("claude|"), "AGENT|MODEL={}", run.launches[0].1);
}

/// The one Claude launch received `explicit-model` as its argument and its
/// `MODEL`.
fn assert_explicit_model(run: &Run) {
    assert_one_claude(run);
    let (_, env, argv) = &run.launches[0];
    assert_eq!(env, "claude|explicit-model", "stderr:\n{}", run.stderr);
    assert!(argv.contains("--model explicit-model"), "argv: {argv}");
}

/// R2-F2.1: a document `agent` reading `env.AGENT` under `--claude`.
#[test]
fn a_provider_flag_replaces_a_circular_document_agent() {
    let (fixture, record) = stage_circular("doc-agent-flag", "", "agent: '{{ env.AGENT }}'\n", "");
    assert_one_claude(&run(&fixture, &record, &["--claude"]));
}

/// R2-F2.1, the other flag spelling.
#[test]
fn a_provider_option_replaces_a_circular_document_agent() {
    let (fixture, record) = stage_circular("doc-agent-provider", "", "agent: '{{ env.AGENT }}'\n", "");
    assert_one_claude(&run(&fixture, &record, &["--provider", "claude"]));
}

/// R2-F2.2: the same expression in the task's `with.agent`.
#[test]
fn a_provider_flag_replaces_a_circular_with_agent() {
    let (fixture, record) = stage_circular("with-agent-flag", "", "", "{ agent: '{{ env.AGENT }}' }");
    assert_one_claude(&run(&fixture, &record, &["--claude"]));
}

/// R2-F2.3: a document `model` reading `env.MODEL` under `--model`, with a
/// static Claude agent.
#[test]
fn an_explicit_model_replaces_a_circular_document_model() {
    let (fixture, record) = stage_circular("doc-model-flag", "agent: claude\n", "model: '{{ env.MODEL }}'\n", "");
    assert_explicit_model(&run(&fixture, &record, &["--model", "explicit-model"]));
}

/// R2-F2.4 (control): a caller setter `agent` replaces the circular hint.
#[test]
fn a_caller_agent_replaces_a_circular_document_agent() {
    let (fixture, record) = stage_circular("doc-agent-setter", "", "agent: '{{ env.AGENT }}'\n", "");
    assert_one_claude(&run(&fixture, &record, &["agent=claude"]));
}

/// R2-F2.5 (control): a caller setter `model` replaces the circular hint.
#[test]
fn a_caller_model_replaces_a_circular_document_model() {
    let (fixture, record) = stage_circular("doc-model-setter", "agent: claude\n", "model: '{{ env.MODEL }}'\n", "");
    assert_explicit_model(&run(&fixture, &record, &["model=explicit-model"]));
}

/// R2-F2.6 (control): a noncircular dynamic `agent` under `--claude`.
#[test]
fn a_provider_flag_replaces_an_agent_decided_at_launch() {
    let (fixture, record) = stage(
        "seq-task-providers-override-dynamic-flag",
        &[
            ("plain.md", "Token PLAIN.\n"),
            (
                "seq.md",
                "---\nsequence:\n  - name: pick\n    shell: echo goose\n  - name: run\n    prompt: plain.md\n    with:\n      agent: \"{{ seq.outputs[0] }}\"\n---\n",
            ),
        ],
        &["claude", "goose"],
    );
    assert_one_claude(&run(&fixture, &record, &["--claude"]));
}

/// R2-F2.8: the task's `with.model` reading `env.MODEL` under `--model`.
#[test]
fn an_explicit_model_replaces_a_circular_with_model() {
    let (fixture, record) = stage_circular("with-model-flag", "agent: claude\n", "", "{ model: '{{ env.MODEL }}' }");
    assert_explicit_model(&run(&fixture, &record, &["--model", "explicit-model"]));
}

/// R2-F2.9: a captured generic `MODEL` outranks the document's circular
/// `model`.
#[test]
fn a_captured_model_variable_replaces_a_circular_document_model() {
    let (fixture, record) = stage_circular("doc-model-env", "agent: claude\n", "model: '{{ env.MODEL }}'\n", "");
    assert_explicit_model(&run_with_env(&fixture, &record, &[], &[("MODEL", "explicit-model")]));
}

/// R2-F2.9: so does Claude's own model variable, `CLAUDE_MODEL`.
#[test]
fn a_captured_provider_model_variable_replaces_a_circular_document_model() {
    let (fixture, record) = stage_circular("doc-model-claude-env", "agent: claude\n", "model: '{{ env.MODEL }}'\n", "");
    assert_explicit_model(&run_with_env(&fixture, &record, &[], &[("CLAUDE_MODEL", "explicit-model")]));
}

/// A group leg and a nested sequence's task take the flag the same way.
#[test]
fn a_provider_flag_replaces_circular_agents_in_legs_and_nested_tasks() {
    let (fixture, record) = stage(
        "seq-task-providers-override-containers",
        &[
            ("self.md", "---\nagent: '{{ env.AGENT }}'\n---\nToken SELF.\n"),
            ("nested/inner.md", "---\nsequence:\n  - name: deep\n    prompt: ../self.md\n---\n"),
            (
                "seq.md",
                "---\nsequence:\n  - name: batch\n    group:\n      tasks:\n        - name: leg\n          prompt: self.md\n        - name: other\n          prompt: self.md\n          with:\n            agent: '{{ env.AGENT }}'\n  - name: inner\n    sequence: nested/inner.md\n---\n",
            ),
        ],
        &["claude", "goose"],
    );
    let run = run(&fixture, &record, &["--claude"]);
    assert_eq!(run.code, Some(0), "stderr:\n{}", run.stderr);
    assert_eq!(binaries(&run), ["claude", "claude", "claude"], "stderr:\n{}", run.stderr);
}

/// A provider flag decides only the provider: a circular `model` with no
/// higher model source still fails before any launch, and the hint names the
/// model sources rather than the flag the caller passed.
#[test]
fn a_provider_flag_does_not_replace_a_circular_model() {
    let (fixture, record) = stage_circular("doc-model-provider-only", "", "model: '{{ env.MODEL }}'\n", "");
    let run = run(&fixture, &record, &["--claude"]);
    assert_eq!(run.code, Some(1), "stderr:\n{}", run.stderr);
    assert!(run.launches.is_empty(), "a task launched: {:?}", run.launches);
    assert!(run.stderr.contains("task provider selection failed"), "stderr:\n{}", run.stderr);
    assert!(run.stderr.contains("env.MODEL"), "stderr:\n{}", run.stderr);
    // The rendered hint wraps behind a gutter.
    let text = run.stderr.replace("┃ ", "");
    assert!(text.contains("pass --model"), "stderr:\n{}", run.stderr);
    assert!(!text.contains("pass --claude"), "stderr:\n{}", run.stderr);
}

/// R2-F2.7 at the binary: a circular `agent` with nothing above it still
/// fails before any launch, located at the hint.
#[test]
fn a_circular_agent_with_no_override_fails_before_any_launch() {
    let (fixture, record) = stage_circular("doc-agent-none", "agent: claude\n", "", "{ agent: '{{ env.AGENT }}' }");
    let run = run(&fixture, &record, &[]);
    assert_eq!(run.code, Some(1), "stderr:\n{}", run.stderr);
    assert!(run.launches.is_empty(), "a task launched: {:?}", run.launches);
    assert!(run.stderr.contains("tasks[0].with.agent"), "stderr:\n{}", run.stderr);
    assert!(run.stderr.contains("env.AGENT"), "stderr:\n{}", run.stderr);
}

/// A failed or empty-list `agent` is still an authoring error under a
/// provider flag: the later task's prompt would evaluate it at launch and
/// fail after the first task ran, so preflight fails before either launches.
#[test]
fn a_provider_flag_does_not_excuse_a_failed_or_empty_agent() {
    for (name, agent) in [("failed", "'{{ no_such_function() }}'"), ("empty", "[]")] {
        let (fixture, record) = stage(
            &format!("seq-task-providers-override-{name}"),
            &[
                ("plain.md", "Token PLAIN.\n"),
                (
                    "seq.md",
                    &format!(
                        "---\nsequence:\n  - name: first\n    prompt: plain.md\n  - name: second\n    prompt: plain.md\n    with:\n      agent: {agent}\n---\n"
                    ),
                ),
            ],
            &["claude"],
        );
        let run = run(&fixture, &record, &["--claude"]);
        assert_eq!(run.code, Some(1), "{name}; stderr:\n{}", run.stderr);
        assert!(run.launches.is_empty(), "{name}: a task launched: {:?}", run.launches);
        assert!(run.stderr.contains("tasks[1].with.agent"), "{name}; stderr:\n{}", run.stderr);
        assert!(!run.stderr.replace("┃ ", "").contains("pass --claude"), "{name}; stderr:\n{}", run.stderr);
    }
}

// -- the composed prompt names the identity that launches ----------------------
//
// R2-F1: a task's identity is selected from its document's effective
// frontmatter before its body renders, so the prompt's `env.AGENT`/`env.MODEL`,
// the child's `AGENT`/`MODEL`, and the spawned provider agree, whether the
// hint is decided at launch, read from a document an earlier task rewrote, or
// static.

/// A body that reports the identity it was composed under.
const IDENTITY_BODY: &str = "BODY_AGENT={{ env.AGENT }}\n";

/// Each launch spawned `expected` in order, and its prompt and child `AGENT`
/// name the provider that spawned.
fn assert_identity_agrees(run: &Run, expected: &[&str]) {
    assert_eq!(run.code, Some(0), "stderr:\n{}", run.stderr);
    assert_eq!(binaries(run), expected, "stderr:\n{}", run.stderr);
    for (binary, env, prompt) in &run.launches {
        assert!(env.starts_with(&format!("{binary}|")), "{binary} launched with AGENT|MODEL={env}");
        assert!(prompt.contains(&format!("BODY_AGENT={binary}")), "{binary} received: {prompt}");
    }
}

/// One row per case: the staged files, the stubs that launch in order, and
/// the completion lines stderr must carry. Claude and Goose are installed.
macro_rules! identity_rows {
    ($($(#[$doc:meta])* $name:ident: [$($file:expr => $text:expr),+ $(,)?] => $expected:expr, [$($line:expr),* $(,)?];)+) => {
        $(
            $(#[$doc])*
            #[test]
            fn $name() {
                let (fixture, record) = stage(
                    &format!("seq-task-identity-{}", stringify!($name)),
                    &[$(($file, AsRef::<str>::as_ref(&$text))),+],
                    &["claude", "goose"],
                );
                let run = run(&fixture, &record, &[]);
                assert_identity_agrees(&run, $expected);
                $(assert!(run.stderr.contains($line), "missing `{}`; stderr:\n{}", $line, run.stderr);)*
            }
        )+
    };
}

identity_rows! {
    /// R2-F1.1: `with.agent` from an earlier shell output, root fallback Claude.
    identity_dynamic_with_agent: [
        "body.md" => IDENTITY_BODY,
        "seq.md" => r#"---
agent: claude
sequence:
  - name: pick
    shell: echo goose
  - name: run
    prompt: body.md
    with:
      agent: "{{ seq.outputs[0] }}"
---
"#,
    ] => &["goose"], ["step 2/2 succeeded (via Goose)"];

    /// R2-F1.2: the same expression in the prompt's own frontmatter.
    identity_dynamic_document_agent: [
        "body.md" => format!("---\nagent: \"{{{{ seq.outputs[0] }}}}\"\n---\n{IDENTITY_BODY}"),
        "seq.md" => "---\nagent: claude\nsequence:\n  - name: pick\n    shell: echo goose\n  - name: run\n    prompt: body.md\n---\n",
    ] => &["goose"], ["step 2/2 succeeded (via Goose)"];

    /// R2-F1.4: a concurrent leg's `with.agent` from a group variable.
    identity_group_leg_agent: [
        "body.md" => IDENTITY_BODY,
        "seq.md" => r#"---
agent: claude
sequence:
  - name: batch
    group:
      variables:
        choice: goose
      tasks:
        - name: leg
          prompt: body.md
          with:
            agent: "{{ group.choice }}"
---
"#,
    ] => &["goose"], [];

    /// R2-F1.5: an earlier shell replaces the prompt with one naming Goose,
    /// so its statically predicted Claude no longer applies.
    identity_rewritten_document_agent: [
        "task.md" => format!("---\nagent: claude\n---\n{IDENTITY_BODY}"),
        "goose.md" => format!("---\nagent: goose\n---\n{IDENTITY_BODY}"),
        "seq.md" => "---\nagent: claude\nsequence:\n  - name: swap\n    shell: cp goose.md task.md\n  - name: run\n    prompt: task.md\n---\n",
    ] => &["goose"], ["step 2/2 succeeded (via Goose)"];

    /// R2-F1.6 (control): a static own provider.
    identity_static_own_agent: [
        "task.md" => format!("---\nagent: goose\n---\n{IDENTITY_BODY}"),
        "seq.md" => "---\nagent: claude\nsequence:\n  - name: run\n    prompt: task.md\n---\n",
    ] => &["goose"], ["step 1/1 succeeded (via Goose)"];

    /// R2-F1.7 (control): an external `kind: task` file with a static
    /// `with.agent`.
    identity_external_task_static_agent: [
        "body.md" => IDENTITY_BODY,
        "task.yaml" => "kind: task\nprompt: body.md\nwith:\n  agent: goose\n",
        "seq.md" => "---\nagent: claude\nsequence:\n  - name: run\n    task: task.yaml\n---\n",
    ] => &["goose"], [];

    /// R2-F1.8 (control): a looping prompt with a static own provider.
    identity_looping_prompt_static_agent: [
        "task.md" => format!("---\nagent: goose\nloop:\n  max: 1\n  until: 'loop.count >= 1'\n---\n{IDENTITY_BODY}"),
        "seq.md" => "---\nagent: claude\nsequence:\n  - name: run\n    prompt: task.md\n---\n",
    ] => &["goose"], [];

    /// R2-F1.9 (control): a Claude prompt proxies to a Goose prompt; each
    /// body names its own provider.
    identity_proxy_target: [
        "claude.md" => format!("---\nagent: claude\nsuccess:\n  - {{proxy: goose.md}}\n---\n{IDENTITY_BODY}"),
        "goose.md" => format!("---\nagent: goose\n---\n{IDENTITY_BODY}"),
        "seq.md" => "---\nsequence:\n  - name: run\n    prompt: claude.md\n---\n",
    ] => &["claude", "goose"], [];

    /// R2-F1.10 (control): a body step composes its sequence document under
    /// that document's provider.
    identity_body_step: [
        "seq.md" => format!("---\nagent: goose\nsequence:\n  - name: body\n---\n{IDENTITY_BODY}"),
    ] => &["goose"], [];

    /// R2-F1.11: the producer and the dynamic task inside an inline nested
    /// sequence.
    identity_inline_nested_dynamic_agent: [
        "body.md" => IDENTITY_BODY,
        "seq.md" => r#"---
agent: claude
sequence:
  - name: inner
    sequence:
      - name: pick
        shell: echo goose
      - name: run
        prompt: body.md
        with:
          agent: "{{ seq.outputs[0] }}"
---
"#,
    ] => &["goose"], [];

    /// R2-F1.12: R2-F1.1 with no root `agent` to fall back on.
    identity_dynamic_with_agent_without_fallback: [
        "body.md" => IDENTITY_BODY,
        "seq.md" => r#"---
sequence:
  - name: pick
    shell: echo goose
  - name: run
    prompt: body.md
    with:
      agent: "{{ seq.outputs[0] }}"
---
"#,
    ] => &["goose"], ["step 2/2 succeeded (via Goose)"];

    /// R2-F1.13: an external `kind: task` file carrying the dynamic hint.
    identity_external_task_dynamic_agent: [
        "body.md" => IDENTITY_BODY,
        "task.yaml" => "kind: task\nprompt: body.md\nwith:\n  agent: \"{{ seq.outputs[0] }}\"\n",
        "seq.md" => "---\nagent: claude\nsequence:\n  - name: pick\n    shell: echo goose\n  - name: run\n    task: task.yaml\n---\n",
    ] => &["goose"], [];

    /// R2-F1.14: the producer and the dynamic task in a file-reference nested
    /// sequence.
    identity_file_nested_dynamic_agent: [
        "body.md" => IDENTITY_BODY,
        "nested/inner.md" => r#"---
sequence:
  - name: pick
    shell: echo goose
  - name: run
    prompt: ../body.md
    with:
      agent: "{{ seq.outputs[0] }}"
---
"#,
        "seq.md" => "---\nagent: claude\nsequence:\n  - name: inner\n    sequence: nested/inner.md\n---\n",
    ] => &["goose"], [];
}

/// R2-F1.3: `with.model` from an earlier shell output renders in the body as
/// the model the child receives.
#[test]
fn identity_dynamic_with_model() {
    let (fixture, record) = stage(
        "seq-task-identity-dynamic-model",
        &[
            ("body.md", "BODY_MODEL={{ env.MODEL }}\n"),
            (
                "seq.md",
                "---\nagent: claude\nsequence:\n  - name: pick\n    shell: echo new-model\n  - name: run\n    prompt: body.md\n    with:\n      model: \"{{ seq.outputs[0] }}\"\n---\n",
            ),
        ],
        &["claude", "goose"],
    );
    let run = run(&fixture, &record, &[]);
    assert_eq!(run.code, Some(0), "stderr:\n{}", run.stderr);
    assert_eq!(binaries(&run), ["claude"], "stderr:\n{}", run.stderr);
    let (_, env, prompt) = &run.launches[0];
    assert_eq!(env, "claude|new-model", "stderr:\n{}", run.stderr);
    assert!(prompt.contains("--model new-model"), "argv and prompt: {prompt}");
    assert!(prompt.contains("BODY_MODEL=new-model"), "argv and prompt: {prompt}");
}

// -- a frontmatter command's identity reaches the body -------------------------
//
// R3-F1: an `agent`/`model` a frontmatter `$( … )` produces is selected inside
// the one full preparation, after the command runs and before the body
// renders, so the body names the identity that launches.

/// A prompt whose own `agent` is a frontmatter command answering Goose.
fn shell_agent_task() -> String {
    format!("---\nagent: '$(echo goose)'\n---\n{IDENTITY_BODY}")
}

identity_rows! {
    /// R3-F1.1: the prompt's own `agent` command, root fallback Claude.
    identity_shell_document_agent: [
        "task.md" => shell_agent_task(),
        "seq.md" => "---\nagent: claude\nsequence:\n  - name: run\n    prompt: task.md\n---\n",
    ] => &["goose"], ["step 1/1 succeeded (via Goose)"];

    /// R3-F1.3: a frontmatter key runs the command and `agent` reads it.
    identity_shell_indirect_document_agent: [
        "task.md" => format!("---\nchoice: '$(echo goose)'\nagent: '{{{{ choice }}}}'\n---\n{IDENTITY_BODY}"),
        "seq.md" => "---\nagent: claude\nsequence:\n  - name: run\n    prompt: task.md\n---\n",
    ] => &["goose"], ["step 1/1 succeeded (via Goose)"];

    /// R3-F1.4: the same prompt as a concurrent group leg.
    identity_shell_group_leg_agent: [
        "task.md" => shell_agent_task(),
        "seq.md" => "---\nagent: claude\nsequence:\n  - name: batch\n    group:\n      tasks:\n        - name: leg\n          prompt: task.md\n---\n",
    ] => &["goose"], [];

    /// R3-F1.5: the same prompt through an external `kind: task` file.
    identity_shell_external_task_agent: [
        "task.md" => shell_agent_task(),
        "task.yaml" => "kind: task\nprompt: task.md\n",
        "seq.md" => "---\nagent: claude\nsequence:\n  - name: run\n    task: task.yaml\n---\n",
    ] => &["goose"], [];

    /// R3-F1.6: the same prompt inside an inline nested sequence.
    identity_shell_inline_nested_agent: [
        "task.md" => shell_agent_task(),
        "seq.md" => "---\nagent: claude\nsequence:\n  - name: inner\n    sequence:\n      - name: run\n        prompt: task.md\n---\n",
    ] => &["goose"], [];

    /// R3-F1.7: the same prompt inside a file-reference nested sequence.
    identity_shell_file_nested_agent: [
        "task.md" => shell_agent_task(),
        "nested/inner.md" => "---\nsequence:\n  - name: run\n    prompt: ../task.md\n---\n",
        "seq.md" => "---\nagent: claude\nsequence:\n  - name: inner\n    sequence: nested/inner.md\n---\n",
    ] => &["goose"], [];

    /// R3-F1.8: R3-F1.1 with no root `agent` to fall back on.
    identity_shell_document_agent_without_fallback: [
        "task.md" => shell_agent_task(),
        "seq.md" => "---\nsequence:\n  - name: run\n    prompt: task.md\n---\n",
    ] => &["goose"], ["step 1/1 succeeded (via Goose)"];
}

/// Stage one prompt task, `task.md`, whose body reports both identity values.
fn stage_identity_model(name: &str, frontmatter: &str, with: &str) -> (CliProcessFixture, PathBuf) {
    let with = if with.is_empty() { String::new() } else { format!("    with: {with}\n") };
    stage(
        &format!("seq-task-identity-{name}"),
        &[
            ("task.md", &format!("---\n{frontmatter}---\nBODY_AGENT={{{{ env.AGENT }}}}\nBODY_MODEL={{{{ env.MODEL }}}}\n")),
            ("seq.md", &format!("---\nagent: claude\nsequence:\n  - name: run\n    prompt: task.md\n{with}---\n")),
        ],
        &["claude", "goose"],
    )
}

/// The run launched one Claude with `model` as its `--model`, its `MODEL`,
/// and the model its body names.
fn assert_claude_model_agrees(run: &Run, model: &str) {
    assert_one_claude(run);
    let (_, env, prompt) = &run.launches[0];
    assert_eq!(env, &format!("claude|{model}"), "stderr:\n{}", run.stderr);
    assert!(prompt.contains(&format!("--model {model}")), "argv and prompt: {prompt}");
    assert!(prompt.contains("BODY_AGENT=claude"), "argv and prompt: {prompt}");
    assert!(prompt.contains(&format!("BODY_MODEL={model}")), "argv and prompt: {prompt}");
}

/// R3-F1.2: the prompt's own `model` command.
#[test]
fn identity_shell_document_model() {
    let (fixture, record) = stage_identity_model("shell-document-model", "model: '$(echo fresh-model)'\n", "");
    assert_claude_model_agrees(&run(&fixture, &record, &[]), "fresh-model");
}

/// R3-F1.9: a resolved `with.model` that merely looks like a command is data:
/// the child and the body both receive it as written, and nothing runs it.
#[test]
fn identity_literal_command_looking_with_model() {
    let (fixture, record) = stage_identity_model("literal-with-model", "", "{model: '$(echo x)'}");
    assert_claude_model_agrees(&run(&fixture, &record, &[]), "$(echo x)");
}

/// R3-F1.10 (control): the same text as `with.agent` is an unknown provider,
/// located at the task, and nothing launches.
#[test]
fn identity_literal_command_looking_with_agent_fails_without_a_launch() {
    let (fixture, record) = stage_identity_model("literal-with-agent", "", "{agent: '$(echo goose)'}");
    let run = run(&fixture, &record, &[]);
    assert_ne!(run.code, Some(0), "stderr:\n{}", run.stderr);
    assert!(run.launches.is_empty(), "launched: {:?}", run.launches);
    assert!(run.stderr.contains("$(echo goose)"), "stderr:\n{}", run.stderr);
    assert!(run.stderr.contains("run"), "the task is named; stderr:\n{}", run.stderr);
}

/// R3-F1.11 (control): `--claude` outranks the document's agent command for
/// the body and the child alike.
#[test]
fn identity_shell_document_agent_under_a_provider_flag() {
    let (fixture, record) = stage_identity_model("shell-agent-claude-flag", "agent: '$(echo goose)'\n", "");
    let run = run(&fixture, &record, &["--claude"]);
    assert_one_claude(&run);
    assert!(run.launches[0].2.contains("BODY_AGENT=claude"), "prompt: {}", run.launches[0].2);
}

/// R3-F1.12 (control): `--model` outranks the document's model command for
/// the body and the child alike.
#[test]
fn identity_shell_document_model_under_an_explicit_model() {
    let (fixture, record) =
        stage_identity_model("shell-model-explicit", "model: '$(echo fresh-model)'\n", "");
    let run = run(&fixture, &record, &["--model", "explicit-model"]);
    assert_explicit_model(&run);
    assert!(run.launches[0].2.contains("BODY_MODEL=explicit-model"), "prompt: {}", run.launches[0].2);
}

/// R3-F1.16: selecting the identity adds no compose pass: the frontmatter
/// command that picks the agent and a body command each run once.
#[test]
fn identity_shell_selection_runs_each_command_once() {
    let (fixture, record) = stage(
        "seq-task-identity-shell-runs-once",
        &[
            ("pick.sh", "#!/bin/sh\necho fm >> runs.log\necho goose\n"),
            ("body.sh", "#!/bin/sh\necho body >> runs.log\necho from-body\n"),
            ("task.md", &format!("---\nagent: '$(./pick.sh)'\n---\n{IDENTITY_BODY}\n::shell ./body.sh\n")),
            ("seq.md", "---\nagent: claude\nsequence:\n  - name: run\n    prompt: task.md\n---\n"),
        ],
        &["claude", "goose"],
    );
    for script in ["pick.sh", "body.sh"] {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(fixture.cwd().join(script), fs::Permissions::from_mode(0o755)).unwrap();
    }
    let run = run(&fixture, &record, &[]);
    assert_identity_agrees(&run, &["goose"]);
    assert!(run.launches[0].2.contains("from-body"), "prompt: {}", run.launches[0].2);
    let runs = fs::read_to_string(fixture.cwd().join("runs.log")).unwrap();
    assert_eq!(runs.lines().collect::<Vec<_>>(), ["fm", "body"], "each command once, frontmatter first");
}

/// R3-F1 sibling: a retried attempt rereads the document with the identity the
/// first attempt's frontmatter command selected, so its body names it too.
#[test]
fn identity_shell_document_agent_on_retry() {
    let fixture = CliProcessFixture::named("seq-task-identity-shell-retry");
    fixture.seed_user_config();
    // Goose fails its first launch, so the document's `failure` stack retries.
    let record = launch_recorder::install(
        &fixture,
        "goose",
        &format!("if [ \"$n\" = 0 ]; then exit 1; fi; {RECORD_ENV}"),
    );
    launch_recorder::install(&fixture, "claude", RECORD_ENV);
    fs::write(
        fixture.cwd().join("flaky.md"),
        format!("---\nagent: '$(echo goose)'\nfailure:\n  - {{retry: {{max_attempts: 1}}}}\n---\n{IDENTITY_BODY}"),
    )
    .unwrap();
    fs::write(fixture.cwd().join("seq.md"), "---\nagent: claude\nsequence:\n  - name: flaky\n    prompt: flaky.md\n---\n").unwrap();
    let output = fixture.command().args(["sequence", "seq.md", "--yolo"]).output().unwrap();
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    assert_eq!(output.status.code(), Some(0), "stderr:\n{stderr}");
    let launches = launch_recorder::launches(&record);
    let launched: Vec<&str> = launches.iter().map(|argv| argv[0].as_str()).collect();
    assert_eq!(launched, ["goose", "goose"], "stderr:\n{stderr}");
    // Goose receives its prompt in argv (`--text`).
    for (attempt, argv) in launches.iter().enumerate() {
        assert!(argv.iter().any(|arg| arg == "BODY_AGENT=goose"), "attempt {attempt} argv: {argv:?}");
    }
    assert_eq!(fs::read_to_string(record.join("env-1")).unwrap(), "goose|unset", "the retried child");
}

