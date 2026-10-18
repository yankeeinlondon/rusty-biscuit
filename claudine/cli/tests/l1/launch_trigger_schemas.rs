//! CLI-boundary coverage for Claudine's trigger schemas in the launch verdict.
//!
//! A document with no `$schema` is judged by Claudine's embedded
//! `claudine.yaml` payload (`inline-compose.yaml` for an inline prompt, and
//! `sequence.yaml` for a sequence document):
//! an invalid value of a Claudine control blocks `compose` and
//! `inline-compose` before any provider launches and is never dropped, while an
//! unrelated authored optional value keeps the drop-and-warn cleanup. The same
//! verdict judges proxy targets, sequence step documents, and retried attempts
//! at their fresh reads, runs after `initialize`, and inline completion reuses
//! the schema frozen at launch.
//!
//! Darkmatter validation coerces scalars to text, so every rejected value here
//! is a list or mapping where the payload declares a scalar.

use std::fs;
use std::path::{Path, PathBuf};

use crate::common;
use common::{CliProcessFixture, InlineAgentStub, launch_recorder, sh_quote, strip_ansi, write_executable};

/// The cleanup warning the launch cleaner prints when it drops a value.
const DROPPED: &str = "dropped optional schema property";

/// Collapse whitespace and block gutters so substring assertions survive the
/// renderer's terminal-width wrapping.
fn normalize(text: &str) -> String {
    text.replace(['┃', '│'], " ").split_whitespace().collect::<Vec<_>>().join(" ")
}

struct Run {
    code: Option<i32>,
    /// Normalized, ANSI-free stderr.
    stderr: String,
}

/// A fixture whose `goose` writes a marker on any launch.
fn marker_fixture(name: &str) -> (CliProcessFixture, PathBuf) {
    let fixture = CliProcessFixture::named(name);
    let marker = fixture.cwd().join("provider-launched.txt");
    write_executable(
        &fixture.bin_dir().join("goose"),
        &format!("#!/bin/sh\necho touched >> {}\nexit 0\n", sh_quote(&marker.display().to_string())),
    );
    (fixture, marker)
}

fn write_doc(fixture: &CliProcessFixture, name: &str, content: &str) -> PathBuf {
    let path = fixture.cwd().join(name);
    fs::write(&path, content).unwrap();
    path
}

fn run(fixture: &CliProcessFixture, subcommand: &str, doc: &Path, extra: &[&str]) -> Run {
    let output = fixture
        .command()
        .args([subcommand, "--goose", doc.to_str().unwrap()])
        .args(extra)
        .output()
        .unwrap();
    Run {
        code: output.status.code(),
        stderr: normalize(&strip_ansi(&String::from_utf8_lossy(&output.stderr))),
    }
}

fn events(fixture: &CliProcessFixture) -> Vec<String> {
    fs::read_to_string(fixture.cwd().join("events.log"))
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect()
}

/// The prompt goose received: the argument after `-t`.
fn goose_prompt(argv: &[String]) -> &str {
    let at = argv.iter().position(|arg| arg == "-t").expect("goose run -t <prompt>");
    argv[at + 1].as_str()
}

fn assert_schema_rejection(run: &Run, needles: &[&str], what: &str) {
    assert_eq!(run.code, Some(1), "{what} exits 1:\n{}", run.stderr);
    assert!(run.stderr.contains("schema validation"), "{what} renders the schema verdict:\n{}", run.stderr);
    for needle in needles {
        assert!(run.stderr.contains(needle), "{what} names `{needle}`:\n{}", run.stderr);
    }
    assert!(!run.stderr.contains(DROPPED), "{what} must not scrub the control:\n{}", run.stderr);
}

// ---------------------------------------------------------------------------
// An invalid Claudine control blocks the launch, under both commands
// ---------------------------------------------------------------------------

struct Row {
    name: &'static str,
    /// Frontmatter lines after `prompt:`.
    frontmatter: &'static str,
    /// What the diagnostic must name.
    needles: &'static [&'static str],
}

const ROWS: &[Row] = &[
    Row {
        name: "misspelled_long_form_key",
        frontmatter: "start:\n    - shell: { cmd: \"git status\" }\n",
        needles: &["/start/0/shell", "cmd"],
    },
    Row {
        name: "unknown_say_parameter",
        frontmatter: "success:\n    - say: {txet: hi}\n",
        needles: &["/success/0/say", "txet"],
    },
    Row {
        name: "empty_agent_list",
        frontmatter: "agent: []\n",
        needles: &["/agent"],
    },
    Row {
        name: "numeric_loop",
        frontmatter: "loop: 42\n",
        needles: &["/loop"],
    },
];

fn assert_row_blocks_launch(row_name: &str, subcommand: &str) {
    let row = ROWS
        .iter()
        .find(|row| row.name == row_name)
        .unwrap_or_else(|| panic!("no table row `{row_name}`"));
    let (fixture, marker) = marker_fixture("launch-trigger-schemas");
    let doc = write_doc(
        &fixture,
        "prompt.md",
        &format!("---\nprompt: review the diff\n{}---\nBody\n", row.frontmatter),
    );

    let result = run(&fixture, subcommand, &doc, &[]);

    assert_schema_rejection(&result, row.needles, &format!("`{subcommand}`/`{row_name}`"));
    assert!(!marker.exists(), "`{subcommand}`/`{row_name}` must not launch the provider:\n{}", result.stderr);
}

macro_rules! invalid_control_rows {
    ($($row:ident),* $(,)?) => {
        /// Every row the macro generated tests for, in table order.
        const GENERATED: &[&str] = &[$(stringify!($row)),*];

        $(
            mod $row {
                #[test]
                fn compose() {
                    super::assert_row_blocks_launch(stringify!($row), "compose");
                }

                #[test]
                fn inline_compose() {
                    super::assert_row_blocks_launch(stringify!($row), "inline-compose");
                }
            }
        )*
    };
}

invalid_control_rows!(misspelled_long_form_key, unknown_say_parameter, empty_agent_list, numeric_loop);

#[test]
fn every_table_row_generates_its_tests() {
    let table: Vec<&str> = ROWS.iter().map(|row| row.name).collect();
    assert_eq!(table, GENERATED, "add the row to `invalid_control_rows!` too");
}

// ---------------------------------------------------------------------------
// An authored `$schema` merges with the payload; phase rules are unchanged
// ---------------------------------------------------------------------------

/// `compose` reports an omitted authored required property as missing, and
/// once the caller supplies it the payload still judges the Claudine controls
/// beside it.
#[test]
fn compose_reports_an_omitted_authored_property_and_still_applies_the_payload() {
    let (fixture, marker) = marker_fixture("launch-trigger-schemas-authored");
    let valid = write_doc(
        &fixture,
        "valid.md",
        "---\n$schema:\n  topic: string(required)\nstart:\n    - info: hi\n---\nPlan {{ topic }}\n",
    );
    let missing = run(&fixture, "compose", &valid, &[]);
    assert_eq!(missing.code, Some(1), "{}", missing.stderr);
    assert!(missing.stderr.contains("missing properties"), "{}", missing.stderr);
    assert!(missing.stderr.contains("`topic`"), "{}", missing.stderr);

    let invalid = write_doc(
        &fixture,
        "invalid.md",
        "---\n$schema:\n  topic: string(required)\nagent: []\n---\nPlan {{ topic }}\n",
    );
    let supplied = run(&fixture, "compose", &invalid, &["topic=diffs"]);
    assert_schema_rejection(&supplied, &["/agent"], "the supplied run");
    assert!(!supplied.stderr.contains("missing properties"), "{}", supplied.stderr);
    assert!(!marker.exists(), "neither run launches the provider");
}

const DEFERRED_DOC: &str = "---\n$schema:\n  topic: string(required)\nprompt: Write about the topic\nstart:\n    - info: hi\n---\nBody\n";

/// `SchemaPhase::Launch` still defers a required non-eager absence to the
/// agent with the payload selected.
#[test]
fn inline_compose_defers_a_required_non_eager_property() {
    let fixture = CliProcessFixture::named("launch-trigger-schemas-deferred");
    let doc = write_doc(&fixture, "prompt.md", DEFERRED_DOC);
    InlineAgentStub::new(&doc)
        .frontmatter_additions("topic: handsets\n")
        .install(fixture.bin_dir(), "goose");

    let result = run(&fixture, "inline-compose", &doc, &[]);

    assert_eq!(result.code, Some(0), "{}", result.stderr);
    assert!(fs::read_to_string(&doc).unwrap().contains("topic: handsets"));
}

/// The deferral does not suspend the payload: an invalid control beside the
/// deferred property still blocks the launch.
#[test]
fn inline_compose_deferral_does_not_suspend_the_payload() {
    let (fixture, marker) = marker_fixture("launch-trigger-schemas-deferred-invalid");
    let doc = write_doc(&fixture, "prompt.md", &DEFERRED_DOC.replace("start:", "model: {x: 1}\nstart:"));

    let result = run(&fixture, "inline-compose", &doc, &[]);

    assert_schema_rejection(&result, &["/model"], "inline-compose");
    assert!(!result.stderr.contains("missing properties"), "{}", result.stderr);
    assert!(!marker.exists(), "{}", result.stderr);
}

#[test]
fn inline_compose_rejects_a_missing_required_eager_property() {
    let (fixture, marker) = marker_fixture("launch-trigger-schemas-eager");
    let doc = write_doc(
        &fixture,
        "prompt.md",
        &DEFERRED_DOC.replace("string(required)", "string(required;eager)"),
    );

    let result = run(&fixture, "inline-compose", &doc, &[]);

    assert_eq!(result.code, Some(1), "{}", result.stderr);
    assert!(result.stderr.contains("missing properties"), "{}", result.stderr);
    assert!(result.stderr.contains("`topic`"), "{}", result.stderr);
    assert!(!marker.exists(), "{}", result.stderr);
}

// ---------------------------------------------------------------------------
// Valid documents launch as before
// ---------------------------------------------------------------------------

/// Lifecycle stacks in both shapes, linking keys, and application data.
const VALID_FRONTMATTER: &str = concat!(
    "start:\n    - info: hi\n",
    "success:\n    say: done\n",
    "name: reviewer\n",
    "allowed-tools: [Read, Grep]\n",
    "argument-hint: \"[file]\"\n",
    "ticket: 123\n",
);

#[test]
fn compose_launches_a_valid_document_with_its_application_data() {
    let fixture = CliProcessFixture::named("launch-trigger-schemas-valid");
    let launches = launch_recorder::install(&fixture, "goose", "exit 0");
    let doc = write_doc(&fixture, "prompt.md", &format!("---\n{VALID_FRONTMATTER}---\nFix ticket {{{{ ticket }}}}.\n"));

    let result = run(&fixture, "compose", &doc, &[]);

    assert_eq!(result.code, Some(0), "{}", result.stderr);
    assert!(!result.stderr.contains(DROPPED), "{}", result.stderr);
    let argv = launch_recorder::launches(&launches);
    assert_eq!(argv.len(), 1, "{}", result.stderr);
    assert_eq!(goose_prompt(&argv[0]).trim(), "Fix ticket 123.");
    assert!(!fixture.audio_spool().exists(), "lifecycle audio must stay unpublished");
}

#[test]
fn inline_compose_launches_a_valid_document_and_keeps_its_application_data() {
    let fixture = CliProcessFixture::named("launch-trigger-schemas-valid-inline");
    let argv_file = fixture.cwd().join("argv.txt");
    let doc = write_doc(
        &fixture,
        "prompt.md",
        &format!("---\nprompt: Summarize ticket {{{{ ticket }}}}\n{VALID_FRONTMATTER}---\nBody\n"),
    );
    let prelude = format!("printf '%s\\n' \"$*\" > {}\n", sh_quote(&argv_file.display().to_string()));
    InlineAgentStub::new(&doc).prelude(&prelude).install(fixture.bin_dir(), "goose");

    let result = run(&fixture, "inline-compose", &doc, &[]);

    assert_eq!(result.code, Some(0), "{}", result.stderr);
    assert!(!result.stderr.contains(DROPPED), "{}", result.stderr);
    let argv = fs::read_to_string(&argv_file).expect("the provider launched");
    assert!(argv.contains("Summarize ticket 123"), "{argv}");
    let written = fs::read_to_string(&doc).unwrap();
    for kept in ["ticket: 123", "name: reviewer", "allowed-tools: [Read, Grep]"] {
        assert!(written.contains(kept), "`{kept}` survives the run:\n{written}");
    }
    assert!(!fixture.audio_spool().exists(), "lifecycle audio must stay unpublished");
}

// ---------------------------------------------------------------------------
// Claudine controls are never scrubbed; application values still are
// ---------------------------------------------------------------------------

#[test]
fn compose_does_not_scrub_a_malformed_optional_control() {
    let (fixture, marker) = marker_fixture("launch-trigger-schemas-no-scrub");
    let doc = write_doc(&fixture, "prompt.md", "---\nprompt: hi\ntimeout: [1]\n---\nBody\n");

    let result = run(&fixture, "compose", &doc, &[]);

    assert_schema_rejection(&result, &["/timeout"], "compose");
    assert!(!marker.exists(), "{}", result.stderr);
}

#[test]
fn inline_compose_does_not_scrub_a_malformed_optional_control() {
    let (fixture, marker) = marker_fixture("launch-trigger-schemas-no-scrub-inline");
    let doc = write_doc(&fixture, "prompt.md", "---\nprompt: hi\ntimeout: [1]\n---\nBody\n");

    let result = run(&fixture, "inline-compose", &doc, &[]);

    assert_schema_rejection(&result, &["/timeout"], "inline-compose");
    assert!(!marker.exists(), "{}", result.stderr);
}

#[test]
fn an_invalid_authored_optional_application_value_is_still_dropped() {
    let (fixture, marker) = marker_fixture("launch-trigger-schemas-drop");
    let doc = write_doc(
        &fixture,
        "prompt.md",
        "---\n$schema:\n  notes: string\nnotes: {a: 1}\nstart:\n    - info: hi\n---\nBody\n",
    );

    let result = run(&fixture, "compose", &doc, &[]);

    assert_eq!(result.code, Some(0), "{}", result.stderr);
    assert!(result.stderr.contains("dropped optional schema property `notes`"), "{}", result.stderr);
    assert!(marker.exists(), "the run launches:\n{}", result.stderr);
}

// ---------------------------------------------------------------------------
// `initialize` precedes the verdict; `start` never follows a failed one
// ---------------------------------------------------------------------------

/// `agent: []` rather than a malformed `start` action: the lifecycle parser
/// reads every stack before `initialize` runs, so a malformed action fails
/// there and `initialize` never runs.
const INITIALIZE_FRONTMATTER: &str = concat!(
    "initialize:\n    - append_line: [\"events.log\", \"initialize\"]\n",
    "start:\n    - append_line: [\"events.log\", \"start\"]\n",
    "agent: []\n",
);

fn assert_initialize_precedes_the_verdict(subcommand: &str, own_frontmatter: &str) {
    let (fixture, marker) = marker_fixture("launch-trigger-schemas-initialize");
    let doc = write_doc(&fixture, "prompt.md", &format!("---\n{own_frontmatter}{INITIALIZE_FRONTMATTER}---\nBody\n"));

    let result = run(&fixture, subcommand, &doc, &[]);

    assert_schema_rejection(&result, &["/agent"], subcommand);
    assert_eq!(events(&fixture), ["initialize"], "{}", result.stderr);
    assert!(!marker.exists(), "{}", result.stderr);
}

#[test]
fn compose_runs_initialize_before_the_verdict_and_never_start() {
    assert_initialize_precedes_the_verdict("compose", "");
}

#[test]
fn inline_compose_runs_initialize_before_the_verdict_and_never_start() {
    assert_initialize_precedes_the_verdict("inline-compose", "prompt: hi\n");
}

// ---------------------------------------------------------------------------
// Fresh-read boundaries share the verdict
// ---------------------------------------------------------------------------

#[test]
fn a_proxy_target_with_an_invalid_control_fails_before_its_provider() {
    let (fixture, marker) = marker_fixture("launch-trigger-schemas-proxy");
    write_doc(
        &fixture,
        "target.md",
        "---\ninitialize:\n    - append_line: [\"events.log\", \"target initialize\"]\nagent: []\n---\ntarget body\n",
    );
    let source = write_doc(
        &fixture,
        "prompt.md",
        "---\nstart:\n    - append_line: [\"events.log\", \"source start\"]\n    - proxy: ./target.md\n---\nsource body\n",
    );

    let result = run(&fixture, "compose", &source, &[]);

    assert_schema_rejection(&result, &["/agent", "target.md"], "the proxy target");
    assert_eq!(events(&fixture), ["source start", "target initialize"], "{}", result.stderr);
    assert!(!marker.exists(), "{}", result.stderr);
}

#[test]
fn a_sequence_step_document_with_an_invalid_control_fails_before_its_provider() {
    let fixture = CliProcessFixture::named("launch-trigger-schemas-sequence");
    let launches = launch_recorder::install(&fixture, "goose", "exit 0");
    write_doc(&fixture, "good.md", "---\ntitle: Good\n---\nGOOD-STEP\n");
    write_doc(&fixture, "bad.md", "---\nagent: []\n---\nBAD-STEP\n");
    let sequence = write_doc(
        &fixture,
        "seq.md",
        "---\nsequence:\n    - name: one\n      prompt: good.md\n    - name: two\n      prompt: bad.md\n---\nBody.\n",
    );

    let result = run(&fixture, "sequence", &sequence, &[]);

    assert_eq!(result.code, Some(1), "{}", result.stderr);
    assert!(result.stderr.contains("/agent") && result.stderr.contains("bad.md"), "{}", result.stderr);
    let argv = launch_recorder::launches(&launches);
    assert_eq!(argv.len(), 1, "only the valid step launches:\n{}", result.stderr);
    assert!(goose_prompt(&argv[0]).contains("GOOD-STEP"), "{argv:?}");
}

/// A sequence document whose only control is `sequence` is judged by the
/// embedded `sequence.yaml` payload, imports included, with no schema on
/// disk: a malformed task field fails the run before any step launches. The
/// unedited sequence launches its one step.
#[test]
fn a_sequence_document_is_judged_by_the_embedded_sequence_trigger_before_any_step() {
    let fixture = CliProcessFixture::named("launch-trigger-schemas-sequence-root");
    let launches = launch_recorder::install(&fixture, "goose", "exit 0");
    write_doc(&fixture, "step.md", "---\ntitle: Step\n---\nSTEP-BODY\n");
    let valid = write_doc(&fixture, "valid.md", "---\nsequence:\n    - name: one\n      prompt: step.md\n---\nBody.\n");
    let malformed = write_doc(
        &fixture,
        "malformed.md",
        "---\nsequence:\n    - name: one\n      prompt: step.md\n      operation: [review]\n---\nBody.\n",
    );

    let control = run(&fixture, "sequence", &valid, &[]);
    assert_eq!(control.code, Some(0), "{}", control.stderr);
    assert_eq!(launch_recorder::launches(&launches).len(), 1, "{}", control.stderr);

    let result = run(&fixture, "sequence", &malformed, &[]);
    assert_eq!(result.code, Some(1), "{}", result.stderr);
    assert!(result.stderr.contains("/sequence/0/operation"), "{}", result.stderr);
    assert_eq!(launch_recorder::launches(&launches).len(), 1, "the malformed sequence launches nothing:\n{}", result.stderr);
}

/// The first attempt edits the document to carry an invalid control and fails;
/// `failure: [retry]` re-reads it, and the retried attempt fails the verdict
/// instead of launching.
#[test]
fn a_retried_attempt_rereads_the_document_through_the_verdict() {
    let fixture = CliProcessFixture::named("launch-trigger-schemas-retry");
    let doc = fixture.cwd().join("prompt.md");
    let edited = "---\nagent: []\nfailure:\n    - retry: 1\n---\nretry body\n";
    let launches = launch_recorder::install(
        &fixture,
        "goose",
        &format!(
            "[ \"$n\" = 0 ] && {{ printf '%s' {} > {}; exit 1; }}\nexit 0",
            sh_quote(edited),
            sh_quote(&doc.display().to_string())
        ),
    );
    fs::write(&doc, "---\nfailure:\n    - retry: 1\n---\nretry body\n").unwrap();

    let result = run(&fixture, "compose", &doc, &[]);

    assert_schema_rejection(&result, &["/agent"], "the retried attempt");
    assert_eq!(launch_recorder::launches(&launches).len(), 1, "{}", result.stderr);
}

// ---------------------------------------------------------------------------
// Inline completion judges with the schema frozen at launch
// ---------------------------------------------------------------------------

/// The completion verdict's summary; a payload problem is listed beneath it
/// by property.
const COMPLETION_VERDICT: &str = "does not satisfy its schema at completion";

/// The stderr of a completion verdict that failed on `property`, a property
/// only the trigger payload declares: it is listed by name, and the block
/// does not claim the document has a `$schema` or a raw JSON Schema.
fn assert_completion_names(stderr: &str, property: &str) {
    assert!(stderr.contains(COMPLETION_VERDICT), "{stderr}");
    assert!(stderr.contains("Problems:"), "{stderr}");
    assert!(stderr.contains(&format!("`{property}`:")), "names `{property}`:\n{stderr}");
    assert!(!stderr.contains("raw JSON Schema"), "{stderr}");
    assert!(!stderr.contains("its `$schema`"), "{stderr}");
}

/// The agent's valid `model` passes completion and an invalid one fails it,
/// so the payload frozen at launch judges what the agent wrote.
#[test]
fn inline_completion_rejects_an_invalid_control_the_agent_wrote() {
    let fixture = CliProcessFixture::named("launch-trigger-schemas-completion");
    let source = "---\nprompt: Write the body\n---\nBody\n";
    let doc = write_doc(&fixture, "prompt.md", source);
    InlineAgentStub::new(&doc)
        .frontmatter_additions("model: gpt-x\n")
        .install(fixture.bin_dir(), "goose");
    let valid = run(&fixture, "inline-compose", &doc, &[]);
    assert_eq!(valid.code, Some(0), "{}", valid.stderr);

    fs::write(&doc, source).unwrap();
    InlineAgentStub::new(&doc)
        .frontmatter_additions("model: {x: 1}\n")
        .install(fixture.bin_dir(), "goose");
    let invalid = run(&fixture, "inline-compose", &doc, &[]);
    assert_eq!(invalid.code, Some(1), "{}", invalid.stderr);
    assert_completion_names(&invalid.stderr, "model");
}

/// With an authored `$schema`, the completion block keeps its property table
/// and still lists a problem on a property only the payload declares.
#[test]
fn inline_completion_lists_a_payload_problem_beside_the_authored_table() {
    let fixture = CliProcessFixture::named("launch-trigger-schemas-completion-authored");
    let doc = write_doc(
        &fixture,
        "prompt.md",
        "---\n$schema:\n  notes: string\nprompt: Write the body\n---\nBody\n",
    );
    InlineAgentStub::new(&doc)
        .frontmatter_additions("model: {x: 1}\n")
        .install(fixture.bin_dir(), "goose");
    let run = run(&fixture, "inline-compose", &doc, &[]);
    assert_eq!(run.code, Some(1), "{}", run.stderr);
    assert!(run.stderr.contains(COMPLETION_VERDICT), "{}", run.stderr);
    assert!(run.stderr.contains("notes: string"), "the authored table is kept:\n{}", run.stderr);
    assert!(run.stderr.contains("Problems:"), "{}", run.stderr);
    assert!(run.stderr.contains("`model`:"), "the payload problem is listed:\n{}", run.stderr);
}

/// Make `dir` a repository by layout alone, so `{root}/schemas` is searched.
fn write_git_skeleton(dir: &Path) {
    let git = dir.join(".git");
    fs::create_dir_all(git.join("objects")).unwrap();
    fs::create_dir_all(git.join("refs/heads")).unwrap();
    fs::write(git.join("HEAD"), "ref: refs/heads/main\n").unwrap();
    fs::write(git.join("config"), "[core]\n\trepositoryformatversion = 0\n\tbare = false\n").unwrap();
}

/// A disk `claudine.yaml` that replaces the embedded one and loosens
/// `description` to any value.
const LOOSENED_CLAUDINE_SCHEMA: &str = "kind: trigger-schema\nmatch:\n    any:\n        - prompt: string(required)\n$schema:\n    description: \"unknown -> loosened\"\n";

/// An agent that loosens `description` on disk mid-run and writes a mapping
/// into it: this run's completion verdict still uses the launch schema, and
/// the next run reloads the disk file.
#[test]
fn an_agent_written_disk_schema_takes_effect_on_the_next_run() {
    let fixture = CliProcessFixture::named("launch-trigger-schemas-frozen");
    write_git_skeleton(fixture.cwd());
    let schemas = fixture.cwd().join("schemas");
    fs::create_dir_all(&schemas).unwrap();
    let doc = write_doc(&fixture, "prompt.md", "---\nprompt: Write the body\n---\nBody\n");
    let prelude = format!(
        "printf '%s' {} > {}\n",
        sh_quote(LOOSENED_CLAUDINE_SCHEMA),
        sh_quote(&schemas.join("claudine.yaml").display().to_string())
    );
    InlineAgentStub::new(&doc)
        .prelude(&prelude)
        .frontmatter_additions("description: {x: 1}\n")
        .install(fixture.bin_dir(), "goose");

    let first = run(&fixture, "inline-compose", &doc, &[]);
    assert_eq!(first.code, Some(1), "the launch schema still rejects the mapping:\n{}", first.stderr);
    assert_completion_names(&first.stderr, "description");
    assert!(schemas.join("claudine.yaml").exists(), "the agent wrote the schema");

    fs::write(&doc, "---\nprompt: Write the body\ndescription: {x: 1}\n---\nBody\n").unwrap();
    InlineAgentStub::new(&doc).body("Second body\n").install(fixture.bin_dir(), "goose");
    let second = run(&fixture, "inline-compose", &doc, &[]);
    assert_eq!(second.code, Some(0), "the next run loads the loosened schema:\n{}", second.stderr);
    assert!(fs::read_to_string(&doc).unwrap().contains("Second body"));
}

// ---------------------------------------------------------------------------
// Pending values are judged after composition
// ---------------------------------------------------------------------------

/// `mode: "{{ m }}"` is not judged as authored: a supplied member launches,
/// and a frontmatter non-member fails on the composed value.
#[test]
fn a_templated_control_is_judged_after_composition() {
    let (fixture, marker) = marker_fixture("launch-trigger-schemas-templated");
    let supplied = write_doc(&fixture, "supplied.md", "---\nmode: \"{{ m }}\"\nstart:\n    - info: hi\n---\nBody\n");
    let launched = run(&fixture, "compose", &supplied, &["--set", r#"{"m":"append"}"#]);
    assert_eq!(launched.code, Some(0), "{}", launched.stderr);
    assert!(marker.exists(), "{}", launched.stderr);
    fs::remove_file(&marker).unwrap();

    let composed = write_doc(
        &fixture,
        "composed.md",
        "---\nmode: \"{{ m }}\"\nm: bogus\nstart:\n    - info: hi\n---\nBody\n",
    );
    let rejected = run(&fixture, "compose", &composed, &[]);
    assert_schema_rejection(&rejected, &["/mode", "bogus"], "the composed value");
    assert!(!marker.exists(), "{}", rejected.stderr);
}

#[test]
fn a_templated_model_and_a_lifecycle_span_launch() {
    let fixture = CliProcessFixture::named("launch-trigger-schemas-spans");
    // Claudine hands goose its model through `GOOSE_MODEL`.
    let launches = launch_recorder::install(&fixture, "goose", "printf '%s' \"$GOOSE_MODEL\" > \"$dir/model\"\nexit 0");
    let doc = write_doc(
        &fixture,
        "prompt.md",
        "---\nmodel: \"{{ chosen }}\"\nx: hello\nstart:\n    - say: \"{{ x }}\"\n---\nBody\n",
    );

    let result = run(&fixture, "compose", &doc, &["--set", r#"{"chosen":"gpt-x"}"#]);

    assert_eq!(result.code, Some(0), "{}", result.stderr);
    let argv = launch_recorder::launches(&launches);
    assert_eq!(argv.len(), 1, "{}", result.stderr);
    assert_eq!(
        fs::read_to_string(launches.join("model")).unwrap(),
        "gpt-x",
        "the composed model reaches the provider"
    );
    assert!(!fixture.audio_spool().exists(), "lifecycle audio must stay unpublished");
}
