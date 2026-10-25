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
//! Darkmatter validation coerces scalars to text, so a rejected scalar here is
//! one no conversion accepts (`pr_list: three`), a malformed template, or a
//! `null` for a non-nullable parameter; the other rejected values are lists or
//! mappings where the payload declares a scalar. A deferred generated-verb
//! operand passes the verdict and is bound to its parameter type when its
//! event fires, before the action dispatches.
//!
//! The marker provider is the compiled `claudine-fake-goose`, so every test
//! that only asks whether the provider launched runs on every OS. The tests
//! marked `#[cfg(unix)]` need a provider that records its prompt, edits the
//! document, or acts as an inline agent, which only the shell stubs do.

use std::fs;
use std::path::{Path, PathBuf};

use crate::common;
use common::{CliProcessFixture, strip_ansi};
#[cfg(unix)]
use common::{InlineAgentStub, launch_recorder, sh_quote};

/// The cleanup warning the launch cleaner prints when it drops a value.
const DROPPED: &str = "dropped optional schema property";

macro_rules! launch_verdict_cases {
    ($schedule:ident, $helper:ident; $($(#[$meta:meta])* $name:ident: $case:literal),+ $(,)?) => {
        const $schedule: &[bool] = &[$($case),+];

        $(
            $(#[$meta])*
            #[test]
            fn $name() {
                $helper($case);
            }
        )+
    };
}


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

/// The file the marker provider appends one line to per launch; [`run`] names
/// it in `PROVIDER_MARKER` for every command.
fn marker_path(fixture: &CliProcessFixture) -> PathBuf {
    fixture.cwd().join("provider-launched.txt")
}

/// A fixture whose `goose` appends a line to [`marker_path`] on each launch.
///
/// A built binary, not a script: a `.cmd` shim cannot carry Claudine's prompt
/// argument on Windows ("batch file arguments are invalid").
fn marker_fixture(name: &str) -> (CliProcessFixture, PathBuf) {
    let fixture = CliProcessFixture::named(name);
    fs::copy(
        biscuit_test_harness::bin_exe!("claudine-fake-goose"),
        fixture.bin_dir().join(format!("goose{}", std::env::consts::EXE_SUFFIX)),
    )
    .expect("the fake provider is built with `test-fixtures`");
    let marker = marker_path(&fixture);
    (fixture, marker)
}

/// How many times the marker provider launched.
fn provider_launches(marker: &Path) -> usize {
    fs::read_to_string(marker).map_or(0, |log| log.lines().count())
}

fn write_doc(fixture: &CliProcessFixture, name: &str, content: &str) -> PathBuf {
    let path = fixture.cwd().join(name);
    fs::write(&path, content).unwrap();
    path
}

fn run(fixture: &CliProcessFixture, subcommand: &str, doc: &Path, extra: &[&str]) -> Run {
    let output = fixture
        .command()
        .env("PROVIDER_MARKER", marker_path(fixture))
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
#[cfg(unix)]
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
    // Generated per-verb types: the lifecycle parser does not type-check a
    // literal operand, so only the schema verdict stops these before launch.
    Row {
        name: "generated_verb_wrong_type",
        frontmatter: "start:\n    - pr_list: three\n",
        needles: &["/start/0/pr_list", "three"],
    },
    Row {
        name: "generated_verb_malformed_template",
        frontmatter: "start:\n    - pr_list: \"{{ 1 + }}\"\n",
        needles: &["/start/0/pr_list", "{{ 1 + }}"],
    },
    // The operand's type accepts text, so only the template's syntax is wrong.
    Row {
        name: "generated_verb_malformed_string_template",
        frontmatter: "start:\n    - ensure_file: \"{{ 1 + }}\"\n",
        needles: &["/start/0/ensure_file", "malformed template `{{ 1 + }}`"],
    },
    // A verb the parser owns itself reads the same strings as templates.
    Row {
        name: "handwritten_verb_malformed_template",
        frontmatter: "start:\n    - say: \"{{ 1 + }}\"\n",
        needles: &["/start/0/say", "malformed template `{{ 1 + }}`"],
    },
    Row {
        name: "with_overlay_malformed_template",
        frontmatter: "start:\n    - proxy: { target: next.md, with: { note: \"{{ 1 + }}\" } }\n",
        needles: &["/start/0/proxy/with/note", "malformed template `{{ 1 + }}`"],
    },
    Row {
        name: "generated_verb_bad_sibling_beside_deferred",
        frontmatter: "x: 2\nstart:\n    - min: [three, \"{{ fm.x }}\"]\n",
        needles: &["/start/0/min/0", "three"],
    },
    Row {
        name: "optional_nonnullable_null",
        frontmatter: "start:\n    - ping: [\"127.0.0.1\", null]\n",
        needles: &["/start/0/ping/1", "null"],
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
    assert!(
        !result.stderr.to_lowercase().contains("dropped"),
        "`{subcommand}`/`{row_name}` keeps the control key:\n{}",
        result.stderr
    );
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

invalid_control_rows!(
    misspelled_long_form_key,
    unknown_say_parameter,
    empty_agent_list,
    numeric_loop,
    generated_verb_wrong_type,
    generated_verb_malformed_template,
    generated_verb_malformed_string_template,
    handwritten_verb_malformed_template,
    with_overlay_malformed_template,
    generated_verb_bad_sibling_beside_deferred,
    optional_nonnullable_null,
);

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
launch_verdict_cases! {
    AUTHORED_PROPERTY_SCHEDULE, authored_property_case;
    compose_reports_an_omitted_authored_property_and_still_applies_the_payload: false,
    compose_supplied_authored_property_still_checks_the_trigger_payload: true,
}

fn authored_property_case(supplied: bool) {
    let (fixture, marker) = marker_fixture("launch-trigger-schemas-authored");
    let control = if supplied { "agent: []\n" } else { "start:\n    - info: hi\n" };
    let doc = write_doc(
        &fixture,
        "prompt.md",
        &format!("---\n$schema:\n  topic: string(required)\n{control}---\nPlan {{{{ topic }}}}\n"),
    );
    let setters = if supplied { &["topic=diffs"][..] } else { &[] };
    let result = run(&fixture, "compose", &doc, setters);
    if supplied {
        assert_schema_rejection(&result, &["/agent"], "the supplied run");
        assert!(!result.stderr.contains("missing properties"), "{}", result.stderr);
    } else {
        assert_eq!(result.code, Some(1), "{}", result.stderr);
        assert!(result.stderr.contains("missing properties"), "{}", result.stderr);
        assert!(result.stderr.contains("`topic`"), "{}", result.stderr);
    }
    assert!(!marker.exists(), "the rejected run launches no provider");
}

const DEFERRED_DOC: &str = "---\n$schema:\n  topic: string(required)\nprompt: Write about the topic\nstart:\n    - info: hi\n---\nBody\n";

/// `SchemaPhase::Launch` still defers a required non-eager absence to the
/// agent with the payload selected.
#[cfg(unix)]
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
#[cfg(unix)]
const VALID_FRONTMATTER: &str = concat!(
    "start:\n    - info: hi\n",
    "success:\n    say: done\n",
    "name: reviewer\n",
    "allowed-tools: [Read, Grep]\n",
    "argument-hint: \"[file]\"\n",
    "ticket: 123\n",
);

#[cfg(unix)]
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

#[cfg(unix)]
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

/// A wrong-typed generated literal passes the lifecycle parser, so
/// `initialize` still runs; the verdict then blocks `start` and the provider.
#[test]
fn compose_runs_initialize_before_a_generated_verb_verdict() {
    let (fixture, marker) = marker_fixture("launch-trigger-schemas-initialize-generated");
    let doc = write_doc(
        &fixture,
        "prompt.md",
        concat!(
            "---\n",
            "initialize:\n    - append_line: [\"events.log\", \"initialize\"]\n",
            "start:\n    - append_line: [\"events.log\", \"start\"]\n    - pr_list: three\n",
            "---\nBody\n",
        ),
    );

    let result = run(&fixture, "compose", &doc, &[]);

    assert_schema_rejection(&result, &["/start/1/pr_list", "three"], "compose");
    assert_eq!(events(&fixture), ["initialize"], "{}", result.stderr);
    assert!(!marker.exists(), "{}", result.stderr);
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

/// A step document whose control only the launch schema rejects fails at its
/// turn, before its provider. (A static `agent: []` is not used here: preflight
/// rejects it before any step launches.)
#[cfg(unix)]
#[test]
fn a_sequence_step_document_with_an_invalid_control_fails_before_its_provider() {
    let fixture = CliProcessFixture::named("launch-trigger-schemas-sequence");
    let launches = launch_recorder::install(&fixture, "goose", "exit 0");
    write_doc(&fixture, "good.md", "---\ntitle: Good\n---\nGOOD-STEP\n");
    write_doc(&fixture, "bad.md", "---\nagent: goose\ninteractive: sometimes\n---\nBAD-STEP\n");
    let sequence = write_doc(
        &fixture,
        "seq.md",
        "---\nsequence:\n    - name: one\n      prompt: good.md\n    - name: two\n      prompt: bad.md\n---\nBody.\n",
    );

    let result = run(&fixture, "sequence", &sequence, &[]);

    assert_eq!(result.code, Some(1), "{}", result.stderr);
    assert!(result.stderr.contains("/interactive") && result.stderr.contains("bad.md"), "{}", result.stderr);
    let argv = launch_recorder::launches(&launches);
    assert_eq!(argv.len(), 1, "only the valid step launches:\n{}", result.stderr);
    assert!(goose_prompt(&argv[0]).contains("GOOD-STEP"), "{argv:?}");
}

/// A sequence document whose only control is `sequence` is judged by the
/// embedded `sequence.yaml` payload, imports included, with no schema on
/// disk: a malformed task field fails the run before any step launches. The
/// unedited sequence launches its one step.
launch_verdict_cases! {
    SEQUENCE_TRIGGER_SCHEDULE, sequence_trigger_case;
    #[cfg(unix)]
    a_sequence_document_is_judged_by_the_embedded_sequence_trigger_before_any_step: false,
    #[cfg(unix)]
    an_unedited_sequence_passes_the_embedded_trigger_and_launches_its_step: true,
}

#[cfg(unix)]
fn sequence_trigger_case(valid: bool) {
    let fixture = CliProcessFixture::named("launch-trigger-schemas-sequence-root");
    let launches = launch_recorder::install(&fixture, "goose", "exit 0");
    write_doc(&fixture, "step.md", "---\ntitle: Step\n---\nSTEP-BODY\n");
    let extra = if valid { "" } else { "      operation: [review]\n" };
    let doc = write_doc(
        &fixture,
        "sequence.md",
        &format!("---\nsequence:\n    - name: one\n      prompt: step.md\n{extra}---\nBody.\n"),
    );
    let result = run(&fixture, "sequence", &doc, &[]);
    if valid {
        assert_eq!(result.code, Some(0), "{}", result.stderr);
        assert_eq!(launch_recorder::launches(&launches).len(), 1, "{}", result.stderr);
    } else {
        assert_eq!(result.code, Some(1), "{}", result.stderr);
        assert!(result.stderr.contains("/sequence/0/operation"), "{}", result.stderr);
        assert!(launch_recorder::launches(&launches).is_empty(), "the malformed sequence launches nothing:\n{}", result.stderr);
    }
}

/// The first attempt edits the document to carry an invalid control and fails;
/// `failure: [retry]` re-reads it, and the retried attempt fails the verdict
/// instead of launching.
#[cfg(unix)]
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
#[cfg(unix)]
const COMPLETION_VERDICT: &str = "does not satisfy its schema at completion";

/// The stderr of a completion verdict that failed on `property`, a property
/// only the trigger payload declares: it is listed by name, and the block
/// does not claim the document has a `$schema` or a raw JSON Schema.
#[cfg(unix)]
fn assert_completion_names(stderr: &str, property: &str) {
    assert!(stderr.contains(COMPLETION_VERDICT), "{stderr}");
    assert!(stderr.contains("Problems:"), "{stderr}");
    assert!(stderr.contains(&format!("`{property}`:")), "names `{property}`:\n{stderr}");
    assert!(!stderr.contains("raw JSON Schema"), "{stderr}");
    assert!(!stderr.contains("its `$schema`"), "{stderr}");
}

/// The agent's valid `model` passes completion and an invalid one fails it,
/// so the payload frozen at launch judges what the agent wrote.
launch_verdict_cases! {
    INLINE_COMPLETION_SCHEDULE, inline_completion_case;
    #[cfg(unix)]
    inline_completion_rejects_an_invalid_control_the_agent_wrote: false,
    #[cfg(unix)]
    inline_completion_accepts_a_valid_control_the_agent_wrote: true,
}

#[cfg(unix)]
fn inline_completion_case(valid: bool) {
    let fixture = CliProcessFixture::named("launch-trigger-schemas-completion");
    let doc = write_doc(&fixture, "prompt.md", "---\nprompt: Write the body\n---\nBody\n");
    let addition = if valid { "model: gpt-x\n" } else { "model: {x: 1}\n" };
    InlineAgentStub::new(&doc).frontmatter_additions(addition).install(fixture.bin_dir(), "goose");
    let result = run(&fixture, "inline-compose", &doc, &[]);
    assert_eq!(result.code, Some(if valid { 0 } else { 1 }), "{}", result.stderr);
    if !valid {
        assert_completion_names(&result.stderr, "model");
    }
}

/// With an authored `$schema`, the completion block keeps its property table
/// and still lists a problem on a property only the payload declares.
#[cfg(unix)]
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
#[cfg(unix)]
fn write_git_skeleton(dir: &Path) {
    let git = dir.join(".git");
    fs::create_dir_all(git.join("objects")).unwrap();
    fs::create_dir_all(git.join("refs/heads")).unwrap();
    fs::write(git.join("HEAD"), "ref: refs/heads/main\n").unwrap();
    fs::write(git.join("config"), "[core]\n\trepositoryformatversion = 0\n\tbare = false\n").unwrap();
}

#[test]
fn a_malformed_disk_trigger_blocks_launch_instead_of_using_the_embedded_copy() {
    let (fixture, marker) = marker_fixture("launch-trigger-schemas-broken-disk");
    write_git_skeleton(fixture.cwd());
    let schemas = fixture.cwd().join("schemas");
    fs::create_dir_all(&schemas).unwrap();
    fs::write(schemas.join("claudine.yaml"), "kind: schema-trigger\nmatch: 42\n$schema: {}\n").unwrap();
    let doc = write_doc(&fixture, "prompt.md", "---\nprompt: review the diff\n---\nBody\n");

    let result = run(&fixture, "inline-compose", &doc, &[]);

    assert_eq!(result.code, Some(1), "{}", result.stderr);
    assert!(result.stderr.contains("claudine.yaml"), "the failed source is visible:\n{}", result.stderr);
    assert!(result.stderr.contains("match"), "the trigger grammar failure is visible:\n{}", result.stderr);
    assert!(!marker.exists(), "the embedded fallback must not permit a launch:\n{}", result.stderr);
}

/// A disk `claudine.yaml` that replaces the embedded one and loosens
/// `description` to any value.
#[cfg(unix)]
const LOOSENED_CLAUDINE_SCHEMA: &str = "kind: schema-trigger\nmatch:\n    group:\n        - prompt: string(required)\n$schema:\n    description: \"unknown -> loosened\"\n";

/// An agent that loosens `description` on disk mid-run and writes a mapping
/// into it: this run's completion verdict still uses the launch schema, and
/// the next run reloads the disk file.
#[cfg(unix)]
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
launch_verdict_cases! {
    TEMPLATED_CONTROL_SCHEDULE, templated_control_case;
    a_templated_control_is_judged_after_composition: true,
    a_templated_control_rejects_its_composed_non_member: false,
}

fn templated_control_case(valid: bool) {
    let (fixture, marker) = marker_fixture("launch-trigger-schemas-templated");
    let value = if valid { "" } else { "m: bogus\n" };
    let doc = write_doc(
        &fixture,
        "prompt.md",
        &format!("---\nmode: \"{{{{ m }}}}\"\n{value}start:\n    - info: hi\n---\nBody\n"),
    );
    let setters = if valid { &["--set", r#"{"m":"append"}"#][..] } else { &[] };
    let result = run(&fixture, "compose", &doc, setters);
    if valid {
        assert_eq!(result.code, Some(0), "{}", result.stderr);
        assert!(marker.exists(), "{}", result.stderr);
    } else {
        assert_schema_rejection(&result, &["/mode", "bogus"], "the composed value");
        assert!(!marker.exists(), "{}", result.stderr);
    }
}

#[test]
fn every_split_launch_verdict_has_its_positive_and_negative_case() {
    for schedule in [AUTHORED_PROPERTY_SCHEDULE, SEQUENCE_TRIGGER_SCHEDULE, INLINE_COMPLETION_SCHEDULE, TEMPLATED_CONTROL_SCHEDULE] {
        let mut cases = schedule.to_vec();
        cases.sort();
        assert_eq!(cases, [false, true], "schedule each verdict case exactly once");
    }
}

#[cfg(unix)]
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

// ---------------------------------------------------------------------------
// Deferred generated-verb arguments pass preflight and bind at event time
// ---------------------------------------------------------------------------

/// The entries of the fixture's working directory, to prove a failed run
/// created nothing there. Claudine's own `.claudine-tmp` scratch directory,
/// made by every run, is not an effect.
fn cwd_entries(fixture: &CliProcessFixture) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(fixture.cwd())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name != ".claudine-tmp")
        .collect();
    names.sort();
    names
}

/// The rendering of an event-time binding failure: the same mapping as any
/// other raising `start` expression (exit 1, nothing launched).
fn assert_binding_failure(run: &Run, site: &str, needles: &[&str], what: &str) {
    assert_eq!(run.code, Some(1), "{what} exits 1:\n{}", run.stderr);
    assert!(run.stderr.contains("lifecycle evaluation error"), "{what}:\n{}", run.stderr);
    assert!(run.stderr.contains(&format!("in `{site}`")), "{what} names `{site}`:\n{}", run.stderr);
    assert!(run.stderr.contains("crashed expression"), "{what}:\n{}", run.stderr);
    for needle in needles {
        assert!(run.stderr.contains(needle), "{what} names `{needle}`:\n{}", run.stderr);
    }
    assert!(!run.stderr.contains("schema validation"), "{what} passed the launch verdict:\n{}", run.stderr);
}

const DEFERRED_SCALAR: &str = "start:\n    - ensure_file: \"{{ target }}\"\n";

#[test]
fn a_deferred_scalar_argument_passes_preflight_and_binds_at_event_time() {
    let (fixture, marker) = marker_fixture("launch-trigger-schemas-deferred-scalar");
    let doc = write_doc(&fixture, "prompt.md", &format!("---\ntarget: notes.md\n{DEFERRED_SCALAR}---\nBody\n"));

    let result = run(&fixture, "compose", &doc, &[]);

    assert_eq!(result.code, Some(0), "{}", result.stderr);
    assert!(!result.stderr.contains(DROPPED), "{}", result.stderr);
    assert!(fixture.cwd().join("notes.md").is_file(), "the effect dispatched:\n{}", result.stderr);
    assert_eq!(provider_launches(&marker), 1, "{}", result.stderr);
    assert!(!fixture.audio_spool().exists(), "lifecycle audio must stay unpublished");
}

/// A long-form side effect and an array-form expression verb, each with a
/// deferred operand, evaluate to values of their parameters' types.
#[test]
fn deferred_structured_arguments_pass_preflight_and_bind_at_event_time() {
    let (fixture, marker) = marker_fixture("launch-trigger-schemas-deferred-structured");
    let doc = write_doc(
        &fixture,
        "prompt.md",
        concat!(
            "---\ntarget: notes.md\nbody: \"# Notes\"\nx: 2\n",
            "start:\n",
            "    - min: [1, \"{{ fm.x }}\"]\n",
            "    - ensure_file: { file: \"{{ target }}\", content: \"{{ body }}\" }\n",
            "---\nBody\n",
        ),
    );

    let result = run(&fixture, "compose", &doc, &[]);

    assert_eq!(result.code, Some(0), "{}", result.stderr);
    assert!(!result.stderr.contains(DROPPED), "{}", result.stderr);
    let written = fs::read_to_string(fixture.cwd().join("notes.md")).expect("the effect dispatched");
    assert_eq!(written.trim_end(), "# Notes");
    assert_eq!(provider_launches(&marker), 1, "{}", result.stderr);
    assert!(!fixture.audio_spool().exists(), "lifecycle audio must stay unpublished");
}

fn assert_wrong_evaluated_type_fails_before_dispatch(subcommand: &str, own_frontmatter: &str) {
    let (fixture, marker) = marker_fixture("launch-trigger-schemas-deferred-wrong-type");
    let doc = write_doc(
        &fixture,
        "prompt.md",
        &format!("---\n{own_frontmatter}target: [a, b]\n{DEFERRED_SCALAR}---\nBody\n"),
    );
    let before = cwd_entries(&fixture);

    let result = run(&fixture, subcommand, &doc, &[]);

    assert_binding_failure(
        &result,
        "start[0].ensure_file",
        &["expected string", "received list"],
        subcommand,
    );
    assert_eq!(cwd_entries(&fixture), before, "`{subcommand}` dispatched nothing:\n{}", result.stderr);
    assert!(!marker.exists(), "{}", result.stderr);
    let authored = fs::read_to_string(&doc).unwrap();
    assert!(authored.contains("ensure_file: \"{{ target }}\""), "the operand is unchanged:\n{authored}");
}

#[test]
fn compose_fails_a_wrong_evaluated_type_before_dispatch() {
    assert_wrong_evaluated_type_fails_before_dispatch("compose", "");
}

#[test]
fn inline_compose_fails_a_wrong_evaluated_type_before_dispatch() {
    assert_wrong_evaluated_type_fails_before_dispatch("inline-compose", "prompt: hi\n");
}

/// An expression verb's failed conversion stops the event at that action,
/// mapped exactly like the same call raising inside a text template.
#[test]
fn a_failed_expression_verb_conversion_maps_like_any_raising_start_expression() {
    let (fixture, marker) = marker_fixture("launch-trigger-schemas-deferred-conversion");
    let next = "    - append_line: [after.log, ran]\n";
    let verb = write_doc(
        &fixture,
        "verb.md",
        &format!("---\nx: three\nstart:\n    - min: [1, \"{{{{ fm.x }}}}\"]\n{next}---\nBody\n"),
    );
    let template = write_doc(
        &fixture,
        "template.md",
        &format!("---\nx: three\nstart:\n    - info: \"{{{{ min(1, fm.x) }}}}\"\n{next}---\nBody\n"),
    );

    let by_verb = run(&fixture, "compose", &verb, &[]);
    let by_template = run(&fixture, "compose", &template, &[]);

    let reason = "(argument 2): expected number; received text \"three\"";
    assert_binding_failure(&by_verb, "start[0].min", &[reason], "the verb");
    assert_binding_failure(&by_template, "start[0].info", &[reason], "the template");
    assert!(!fixture.cwd().join("after.log").exists(), "the event stopped at the failed action");
    assert!(!marker.exists(), "neither run launches the provider");
}

/// `no_error` tolerates a dispatch failure, never a binding failure.
#[test]
fn no_error_does_not_suppress_a_binding_failure() {
    let (fixture, marker) = marker_fixture("launch-trigger-schemas-deferred-no-error");
    let doc = write_doc(
        &fixture,
        "prompt.md",
        concat!(
            "---\ntarget: [a, b]\nstart:\n",
            "    - ensure_file: { file: \"{{ target }}\", no_error: true }\n",
            "    - append_line: [after.log, ran]\n",
            "---\nBody\n",
        ),
    );

    let result = run(&fixture, "compose", &doc, &[]);

    assert_binding_failure(&result, "start[0].ensure_file", &["expected string"], "compose");
    assert!(!fixture.cwd().join("after.log").exists(), "{}", result.stderr);
    assert!(!marker.exists(), "{}", result.stderr);
}

/// The dry run shows the operand as authored and reports it deferred to its
/// event; nothing is dispatched or launched.
#[test]
fn dry_run_shows_the_deferred_operand_as_authored() {
    let (fixture, marker) = marker_fixture("launch-trigger-schemas-deferred-dry-run");
    let doc = write_doc(&fixture, "prompt.md", &format!("---\ntarget: notes.md\n{DEFERRED_SCALAR}---\nBody\n"));

    let output = fixture
        .command()
        .env("PROVIDER_MARKER", &marker)
        .args(["compose", "--goose", doc.to_str().unwrap(), "--dry-run"])
        .output()
        .unwrap();

    let text = normalize(&strip_ansi(&format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )));
    assert_eq!(output.status.code(), Some(0), "{text}");
    assert!(text.contains("ensure_file: '{{ target }}'"), "the operand is shown as authored:\n{text}");
    assert!(text.contains("interpolated at event-time: start"), "{text}");
    assert!(!fixture.cwd().join("notes.md").exists(), "{text}");
    assert!(!marker.exists(), "{text}");
}
