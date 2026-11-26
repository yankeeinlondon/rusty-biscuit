//! A referenced sequence document is a document: its events and rows read
//! its own prepared values (its properties, defaulted and converted by its
//! `$schema`), whether it runs directly or from a caller. Invocation setters
//! and runtime mutations layer above them, as they do for a directly run
//! document; the caller's frontmatter does not reach it.

use std::fs;

use crate::common::{CliProcessFixture, strip_ansi};

/// Writes each `(name, contents)` file (creating its folders) into a private
/// workspace, runs `root` as a sequence with `extra` arguments, and returns
/// stdout, stderr, and the exit code.
fn run_sequence(root: &str, files: &[(&str, &str)], extra: &[&str]) -> (String, String, i32) {
    let fixture = CliProcessFixture::named("sequence-referenced-state");
    // Rows run a templated `echo`, which Windows has no program for.
    #[cfg(feature = "test-fixtures")]
    fixture.install_echo();
    for (name, contents) in files {
        let path = fixture.cwd().join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }
    let output = fixture.command().args(["sequence", "--goose", "--yolo", "--silent"]).args(extra)
        .arg(fixture.cwd().join(root)).assert().get_output().clone();
    assert!(!fixture.audio_spool().exists());
    (strip_ansi(&String::from_utf8_lossy(&output.stdout)),
     strip_ansi(&String::from_utf8_lossy(&output.stderr)), output.status.code().unwrap_or(-1))
}

macro_rules! referenced_state {
    ($name:ident, root: $root:literal, files: [$(($file:literal, $contents:expr)),* $(,)?], $(args: [$($arg:literal),*],)? expected: $expected:literal) => {
        #[test]
        fn $name() {
            let (stdout, stderr, code) = run_sequence($root, &[$(($file, $contents)),*], &[$($($arg),*)?]);
            assert_eq!(code, 0, "{stderr}\n{stdout}");
            assert_eq!(stdout.trim(), $expected, "{stderr}");
        }
    };
}

/// The reviewer's `child.md`: a property beside the event that reads it.
const CHILD_MD: &str = "---\nlocal_value: child-value\nstart: [{ stdout: \"{{ local_value }}\" }]\nsequence: []\n---\n";

referenced_state!(direct_root_sequence_reads_its_own_property,
    root: "child.md",
    files: [("child.md", CHILD_MD)],
    expected: "child-value");

referenced_state!(inline_nested_sequence_reads_its_authoring_root_property,
    root: "root.md",
    files: [("root.md", "---\nlocal_value: root-value\nsequence:\n  - name: nested\n    sequence: []\n    start: [{ stdout: \"{{ local_value }}\" }]\n---\n")],
    expected: "root-value");

referenced_state!(referenced_markdown_sequence_reads_a_target_only_property,
    root: "root.md",
    files: [
        ("root.md", "---\nsequence: [{ name: child, sequence: child.md }]\n---\n"),
        ("child.md", CHILD_MD),
    ],
    expected: "child-value");

referenced_state!(referenced_markdown_sequence_keeps_its_value_over_the_caller,
    root: "root.md",
    files: [
        ("root.md", "---\nlocal_value: caller-value\nsequence: [{ name: child, sequence: child.md }]\n---\n"),
        ("child.md", CHILD_MD),
    ],
    expected: "child-value");

referenced_state!(referenced_yaml_sequence_keeps_its_value_over_the_caller,
    root: "root.md",
    files: [
        ("root.md", "---\nlocal_value: caller-value\nsequence: [{ name: child, sequence: child.yaml }]\n---\n"),
        ("child.yaml", "kind: sequence\nlocal_value: child-value\nstart: [{ stdout: \"{{ local_value }}\" }]\nsequence: []\n"),
    ],
    expected: "child-value");

referenced_state!(referenced_json_sequence_keeps_its_value_over_the_caller,
    root: "root.md",
    files: [
        ("root.md", "---\nlocal_value: caller-value\nsequence: [{ name: child, sequence: child.json }]\n---\n"),
        ("child.json", r#"{"kind":"sequence","local_value":"child-value","start":[{"stdout":"{{ local_value }}"}],"sequence":[]}"#),
    ],
    expected: "child-value");

referenced_state!(referenced_json5_sequence_keeps_its_value_over_the_caller,
    root: "root.md",
    files: [
        ("root.md", "---\nlocal_value: caller-value\nsequence: [{ name: child, sequence: child.json5 }]\n---\n"),
        ("child.json5", "{ kind: 'sequence', local_value: 'child-value', start: [{ stdout: '{{ local_value }}' }], sequence: [] }"),
    ],
    expected: "child-value");

referenced_state!(referenced_sequence_in_a_group_leg_keeps_its_value_over_the_caller,
    root: "root.md",
    files: [
        ("root.md", "---\nlocal_value: caller-value\nsequence:\n  - name: checks\n    group:\n      name: checks\n      tasks: [{ name: leg, sequence: child.md }]\n---\n"),
        ("child.md", CHILD_MD),
    ],
    expected: "child-value");

referenced_state!(external_task_sequence_keeps_its_value_over_the_caller,
    root: "root.md",
    files: [
        ("root.md", "---\nlocal_value: caller-value\nsequence: [{ name: child, task: task.yaml }]\n---\n"),
        ("task.yaml", "kind: task\nsequence: child.md\n"),
        ("child.md", CHILD_MD),
    ],
    expected: "child-value");

referenced_state!(referenced_sequence_initialize_reads_its_own_property,
    root: "root.md",
    files: [
        ("root.md", "---\nlocal_value: caller-value\nsequence: [{ name: child, sequence: child.md }]\n---\n"),
        ("child.md", "---\nlocal_value: child-value\ninitialize: [{ stdout: \"{{ local_value }}\" }]\nsequence: []\n---\n"),
    ],
    expected: "child-value");

referenced_state!(referenced_sequence_finalize_reads_its_own_property,
    root: "root.md",
    files: [
        ("root.md", "---\nlocal_value: caller-value\nsequence: [{ name: child, sequence: child.md }]\n---\n"),
        ("child.md", "---\nlocal_value: child-value\nfinalize: [{ stdout: \"{{ local_value }}\" }]\nsequence: []\n---\n"),
    ],
    expected: "child-value");

referenced_state!(referenced_sequence_condition_reads_its_own_property,
    root: "root.md",
    files: [
        ("root.md", "---\nlocal_value: caller-value\nsequence: [{ name: child, sequence: child.md }]\n---\n"),
        ("child.md", "---\nlocal_value: child-value\nstart:\n  - when: \"local_value == 'child-value'\"\n    then: [{ stdout: true-branch }]\n    else: [{ stdout: false-branch }]\nsequence: []\n---\n"),
    ],
    expected: "true-branch");

referenced_state!(referenced_sequence_runtime_set_copies_its_own_property,
    root: "root.md",
    files: [
        ("root.md", "---\nlocal_value: caller-value\nsequence: [{ name: child, sequence: child.md }]\n---\n"),
        ("child.md", "---\nlocal_value: child-value\nstart:\n  - set: { copied: \"{{ local_value }}\" }\n  - stdout: \"{{ copied }}\"\nsequence: []\n---\n"),
    ],
    expected: "child-value");

referenced_state!(referenced_sequence_children_read_its_own_property,
    root: "root.md",
    files: [
        ("root.md", "---\nlocal_value: caller-value\nsequence: [{ name: child, sequence: child.md }]\nsuccess: [{ stdout: \"{{ seq.outputs }}\" }]\n---\n"),
        ("child.md", "---\nlocal_value: child-value\nsequence:\n  - { name: leaf, shell: \"echo {{ local_value }}\" }\n---\n"),
    ],
    expected: r#"[["child-value"]]"#);

referenced_state!(inline_sequence_inside_a_referenced_document_reads_that_document,
    root: "root.md",
    files: [
        ("root.md", "---\nlocal_value: caller-value\nsequence: [{ name: child, sequence: child.md }]\n---\n"),
        ("child.md", "---\nlocal_value: child-value\nsequence:\n  - name: inner\n    sequence: []\n    start: [{ stdout: \"{{ local_value }}\" }]\n---\n"),
    ],
    expected: "child-value");

referenced_state!(referenced_sequence_does_not_read_caller_only_properties,
    root: "root.md",
    files: [
        ("root.md", "---\ncaller_only: caller-value\nsequence: [{ name: child, sequence: child.md }]\n---\n"),
        ("child.md", "---\nstart: [{ stdout: \"[{{ caller_only }}]\" }]\nsequence: []\n---\n"),
    ],
    expected: "[]");

referenced_state!(invocation_setter_outranks_the_referenced_property,
    root: "root.md",
    files: [
        ("root.md", "---\nlocal_value: caller-value\nsequence: [{ name: child, sequence: child.md }]\n---\n"),
        ("child.md", CHILD_MD),
    ],
    args: ["--set", r#"{"local_value":"setter-value"}"#],
    expected: "setter-value");

referenced_state!(caller_runtime_mutation_outranks_the_referenced_property,
    root: "root.md",
    files: [
        ("root.md", "---\nsequence:\n  - { name: change, side_effect: { set: { local_value: mutated-value } } }\n  - { name: child, sequence: child.md }\n---\n"),
        ("child.md", CHILD_MD),
    ],
    expected: "mutated-value");

referenced_state!(referenced_sequence_mutation_reaches_the_caller,
    root: "root.md",
    files: [
        ("root.md", "---\nlocal_value: caller-value\nsequence: [{ name: child, sequence: child.md }]\nsuccess: [{ stdout: \"{{ copied }}\" }]\n---\n"),
        ("child.md", "---\nlocal_value: child-value\nstart: [{ set: { copied: \"{{ local_value }}\" } }]\nsequence: []\n---\n"),
    ],
    expected: "child-value");

/// Writes the argv recorder and a sequence whose referenced `start` shell
/// reads the target property, and returns the fixture and its record file.
fn shell_fixture(name: &str) -> (CliProcessFixture, std::path::PathBuf) {
    let fixture = CliProcessFixture::named(name);
    fs::copy(biscuit_test_harness::bin_exe!("claudine-argv-recorder"),
        fixture.bin_dir().join(format!("argv-recorder{}", std::env::consts::EXE_SUFFIX))).unwrap();
    fs::write(fixture.cwd().join("root.md"),
        "---\nlocal_value: caller-value\nsequence: [{ name: child, sequence: child.md }]\n---\n").unwrap();
    fs::write(fixture.cwd().join("child.md"),
        "---\nlocal_value: child-value\nstart: [{ shell: 'argv-recorder {{ local_value }}' }]\nsequence: []\n---\n").unwrap();
    let records = fixture.cwd().join("argv.bin");
    (fixture, records)
}

#[test]
fn referenced_sequence_shell_executes_against_its_own_property() {
    use test_toolkit::argv_recorder::{ARGV_RECORDER_FILE, read_argv_records};
    let (fixture, records) = shell_fixture("sequence-referenced-shell");
    let output = fixture.command().env(ARGV_RECORDER_FILE, &records)
        .args(["sequence", "--goose", "--yolo", "--silent"]).arg(fixture.cwd().join("root.md"))
        .assert().get_output().clone();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(read_argv_records(&records).unwrap(), [vec!["child-value".to_string()]]);
    assert!(!fixture.audio_spool().exists());
}

/// Without `--yolo`, preflight discovers the referenced shell with the
/// target's value: an exact approval of that command is what lets it run.
#[test]
fn referenced_sequence_shell_is_discovered_with_its_own_property() {
    use test_toolkit::argv_recorder::{ARGV_RECORDER_FILE, read_argv_records};
    let (fixture, records) = shell_fixture("sequence-referenced-shell-approval");
    crate::common::write_shell_permissions(fixture.home(), "exact argv-recorder child-value\n");
    let output = fixture.command().env(ARGV_RECORDER_FILE, &records)
        .args(["sequence", "--goose", "--silent"]).arg(fixture.cwd().join("root.md"))
        .assert().get_output().clone();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(read_argv_records(&records).unwrap(), [vec!["child-value".to_string()]]);
    assert!(!fixture.audio_spool().exists());
}

// ============================================================================
// A referenced document's `$schema` inputs
// ============================================================================
//
// A referenced YAML sequence's root `$schema` declares that document's inputs.
// Its own value satisfies them, an invocation setter outranks that value, and
// the referring document's properties never stand in for them. The two
// reference routes are separate implementation paths: a root
// `sequence: child.yaml` keeps its rows as the root's rows (they run under
// the root's events), and fills its `template:` from the child's values; a
// nested `{ sequence: child.yaml }` task runs the child as its own document.
// Missing-input reports of a nested reference and of an empty root reference
// are proven by `sequence_sources_cli.rs::document_inputs`.

/// A child declaring `reviewer` as an input with its own value, whose
/// template copies it into each row.
const SCHEMA_CHILD: &str = "kind: sequence\n$schema:\n  reviewer: string(required) -> who signs off\nreviewer: own-value\ntemplate:\n  who: \"{{ reviewer }}\"\nstart: [{ stdout: \"{{ reviewer }} [{{ root_only }}]\" }]\nsequence:\n  - { name: sign, shell: \"echo {{ seq.who }}\" }\n";

/// The referring roots. Each declares a property the child never declares.
const ROOT_REFERENCE: &str = "---\nroot_only: root-value\nsuccess: [{ stdout: \"{{ seq.outputs }}\" }]\nsequence: lists/child.yaml\n---\n";
const NESTED_REFERENCE: &str = "---\nroot_only: root-value\nsequence: [{ name: child, sequence: lists/child.yaml }]\n---\n";

referenced_state!(a_root_reference_fills_its_template_from_its_own_input_value,
    root: "root.md",
    files: [("root.md", ROOT_REFERENCE), ("lists/child.yaml", SCHEMA_CHILD)],
    expected: r#"["own-value"]"#);

referenced_state!(a_setter_outranks_a_root_references_own_input_value,
    root: "root.md",
    files: [("root.md", ROOT_REFERENCE), ("lists/child.yaml", SCHEMA_CHILD)],
    args: ["reviewer=setter-value"],
    expected: r#"["setter-value"]"#);

// The child's `start` reads its own input and, as `[]`, the referring
// document's `root_only`, which never reaches it.
referenced_state!(a_nested_reference_reads_its_own_input_value_and_no_referrer_property,
    root: "root.md",
    files: [("root.md", NESTED_REFERENCE), ("lists/child.yaml", SCHEMA_CHILD)],
    expected: "own-value []");

referenced_state!(a_setter_outranks_a_nested_references_own_input_value,
    root: "root.md",
    files: [("root.md", NESTED_REFERENCE), ("lists/child.yaml", SCHEMA_CHILD)],
    args: ["reviewer=setter-value"],
    expected: "setter-value []");

/// A child whose input has no value, with an `append_line` in each of its
/// events so a run of any of them leaves `lists/events.log`.
const MISSING_CHILD: &str = "kind: sequence\n$schema:\n  reviewer: string(required) -> who signs off\ninitialize: [{ append_line: [events.log, child-initialize] }]\nstart: [{ append_line: [events.log, child-start] }]\nsequence:\n  - { name: sign, shell: \"echo {{ reviewer }}\" }\n";

/// Runs `root` (which also lacks its own `topic`) without setters and asserts
/// the one combined report names both owning documents, the child at
/// `property`, and both inputs, with no event of either document run.
fn assert_combined_report(root: &str, property: &str) {
    let root_doc = format!("---\n$schema:\n  topic: string(required) -> what is reviewed\nstart: [{{ append_line: [events.log, root-start] }}]\n{root}---\n");
    let fixture = CliProcessFixture::named("sequence-referenced-inputs");
    fs::create_dir_all(fixture.cwd().join("lists")).unwrap();
    fs::write(fixture.cwd().join("root.md"), root_doc).unwrap();
    fs::write(fixture.cwd().join("lists/child.yaml"), MISSING_CHILD).unwrap();

    let output = fixture.command().args(["sequence", "--goose", "--yolo"])
        .arg(fixture.cwd().join("root.md")).assert().failure().get_output().clone();

    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    let flat = stderr.replace('\u{2503}', " ").split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(flat.contains("Missing required schema properties in 2 documents of the sequence"), "{flat}");
    assert!(flat.contains("root.md - `topic`: string — what is reviewed"), "{flat}");
    assert!(flat.contains(&format!("child.yaml (`{property}`) - `reviewer`: string — who signs off")), "{flat}");
    for log in ["events.log", "lists/events.log"] {
        assert!(!fixture.cwd().join(log).exists(), "collection ran a lifecycle event ({log}): {flat}");
    }
    assert!(!fixture.audio_spool().exists());
}

#[test]
fn a_root_references_missing_input_joins_the_roots_in_one_report_and_runs_no_event() {
    assert_combined_report("sequence: lists/child.yaml\n", "sequence");
}

#[test]
fn a_nested_references_missing_input_joins_the_roots_in_one_report_and_runs_no_event() {
    assert_combined_report("sequence: [{ name: review, sequence: lists/child.yaml }]\n", "sequence[0].sequence");
}

// ============================================================================
// A referenced document's `template:` reads no property of its referrer
// ============================================================================
//
// The child's row prints the value its template filled. Without `--silent` a
// shell row's output is the only text on stdout. Every route a whole
// referenced document takes (root, nested, group leg, task file) leaves a key
// the child never defines undefined, so the template's fallback applies.

/// Runs `root` like [`run_sequence`] but without `--silent`, so a shell row's
/// output reaches stdout, and asserts it printed `expected` and nothing else.
fn assert_filled(root: &str, files: &[(&str, &str)], extra: &[&str], expected: &str) {
    let fixture = CliProcessFixture::named("sequence-referenced-template");
    #[cfg(feature = "test-fixtures")]
    fixture.install_echo();
    for (name, contents) in files {
        let path = fixture.cwd().join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }
    let output = fixture.command().args(["sequence", "--goose", "--yolo"]).args(extra)
        .arg(fixture.cwd().join(root)).assert().get_output().clone();
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    assert!(output.status.success(), "{stderr}");
    let stdout = strip_ansi(&String::from_utf8_lossy(&output.stdout));
    // A group leg's output carries a leg gutter; only the printed word counts.
    let words: Vec<&str> = stdout.split_whitespace().filter(|word| word.chars().all(char::is_alphanumeric)).collect();
    assert_eq!(words, [expected], "{stdout}\n{stderr}");
    assert!(!fixture.audio_spool().exists());
}

macro_rules! template_fill {
    ($name:ident, root: $root:literal, files: [$(($file:literal, $contents:expr)),* $(,)?], $(args: [$($arg:literal),*],)? expected: $expected:literal) => {
        #[test]
        fn $name() {
            assert_filled($root, &[$(($file, $contents)),*], &[$($($arg),*)?], $expected);
        }
    };
}

/// A child whose `template:` reads `reviewer`, a key it never defines, with a
/// fallback; a row's shell prints what the template filled.
const TEMPLATED_CHILD: &str = "kind: sequence\ntemplate:\n  who: \"{{ reviewer || 'none' }}\"\nsequence:\n  - name: work\n    shell: \"echo {{ seq.who }}\"\n";

template_fill!(a_directly_run_template_reads_its_fallback,
    root: "sub/child.yaml",
    files: [("sub/child.yaml", TEMPLATED_CHILD)],
    expected: "none");

template_fill!(a_root_referenced_template_never_reads_the_referrers_property,
    root: "root.yaml",
    files: [
        ("root.yaml", "kind: sequence\nreviewer: leaked\nsequence: sub/child.yaml\n"),
        ("sub/child.yaml", TEMPLATED_CHILD),
    ],
    expected: "none");

template_fill!(a_nested_referenced_template_never_reads_the_referrers_property,
    root: "root.yaml",
    files: [
        ("root.yaml", "kind: sequence\nreviewer: leaked\nsequence: [{ name: child, sequence: sub/child.yaml }]\n"),
        ("sub/child.yaml", TEMPLATED_CHILD),
    ],
    expected: "none");

template_fill!(a_group_leg_referenced_template_never_reads_the_referrers_property,
    root: "root.yaml",
    files: [
        ("root.yaml", "kind: sequence\nreviewer: leaked\nsequence:\n  - name: checks\n    group:\n      name: checks\n      tasks: [{ name: leg, sequence: sub/child.yaml }]\n"),
        ("sub/child.yaml", TEMPLATED_CHILD),
    ],
    expected: "none");

template_fill!(a_task_file_referenced_template_never_reads_the_referrers_property,
    root: "root.yaml",
    files: [
        ("root.yaml", "kind: sequence\nreviewer: leaked\nsequence: [{ name: child, task: task.yaml }]\n"),
        ("task.yaml", "kind: task\nsequence: sub/child.yaml\n"),
        ("sub/child.yaml", TEMPLATED_CHILD),
    ],
    expected: "none");

template_fill!(a_referenced_template_reads_its_own_property_over_nothing_of_the_referrers,
    root: "root.yaml",
    files: [
        ("root.yaml", "kind: sequence\nreviewer: leaked\nsequence: sub/child.yaml\n"),
        ("sub/child.yaml", &format!("reviewer: own\n{TEMPLATED_CHILD}")),
    ],
    expected: "own");

template_fill!(a_setter_fills_a_referenced_template_over_its_own_property,
    root: "root.yaml",
    files: [
        ("root.yaml", "kind: sequence\nreviewer: leaked\nsequence: sub/child.yaml\n"),
        ("sub/child.yaml", &format!("reviewer: own\n{TEMPLATED_CHILD}")),
    ],
    args: ["reviewer=caller"],
    expected: "caller");

// The research fleets' channel: the referrer parameterizes a shared roster by
// writing the value at run time, which is a layer the template does read.
template_fill!(a_root_initialize_write_fills_a_referenced_template,
    root: "root.yaml",
    files: [
        ("root.yaml", "kind: sequence\ninitialize: [{ set: { reviewer: written } }]\nsequence: sub/child.yaml\n"),
        ("sub/child.yaml", TEMPLATED_CHILD),
    ],
    expected: "written");

/// An inline container whose `template:` reads `reviewer`, as a list item
/// indented two spaces: its row's `$state` requires the filled `who`, and its
/// shell prints it.
const READS_REVIEWER: &str = "  - name: nested\n    $state:\n      who: string(required)\n    template:\n      who: \"{{ reviewer }}\"\n    sequence:\n      - name: work\n        shell: \"echo {{ seq.who }}\"\n";

// An inline container's template reads its owner's prepared values: a schema
// default (absent or `null`) and a derived value are filled, judged, and run
// as the owner's own preparation produces them.
template_fill!(an_inline_template_reads_an_absent_owner_value_by_its_default,
    root: "root.yaml",
    files: [("root.yaml", &format!("kind: sequence\n$schema:\n  reviewer: string(default(ada))\nsequence:\n{READS_REVIEWER}"))],
    expected: "ada");

template_fill!(an_inline_template_reads_a_null_owner_value_by_its_default,
    root: "root.yaml",
    files: [("root.yaml", &format!("kind: sequence\n$schema:\n  reviewer: string(default(ada))\nreviewer: null\nsequence:\n{READS_REVIEWER}"))],
    expected: "ada");

template_fill!(an_inline_template_reads_a_derived_owner_value,
    root: "root.yaml",
    files: [("root.yaml", &format!("kind: sequence\nsource: ada\nreviewer: \"{{{{ source }}}}\"\nsequence:\n{READS_REVIEWER}"))],
    expected: "ada");

template_fill!(a_referenced_documents_inline_template_reads_its_own_default,
    root: "root.yaml",
    files: [
        ("root.yaml", "kind: sequence\nreviewer: caller\nsequence:\n  - name: outer\n    sequence: sub/child.yaml\n"),
        ("sub/child.yaml", &format!("kind: sequence\n$schema:\n  reviewer: string(default(ada))\nsequence:\n{READS_REVIEWER}")),
    ],
    expected: "ada");

// ============================================================================
// A referenced document's `$schema` judges the values of its turn
// ============================================================================
//
// A whole referenced document is judged by its own `$schema` where it is
// prepared at run time, against its own properties under the setters and
// every runtime write so far, exactly as a directly run document is judged at
// each of its steps. An earlier write that breaks the document's contract
// therefore stops it before its shell runs, on every route that reaches it.
// `OBSERVED` is printed only by a row that ran.

/// A document requiring a numeric `rank`, authored valid, with one shell row.
const RANKED: &str = "kind: sequence\n$schema:\n  rank: number(required)\nrank: 1\nsequence:\n  - { name: observe, shell: echo OBSERVED }\n";

/// [`RANKED`] whose own first row writes `rank` as `value`.
fn self_writing(value: &str) -> String {
    format!("kind: sequence\n$schema:\n  rank: number(required)\nrank: 1\nsequence:\n  - {{ name: change, side_effect: {{ set: {{ rank: {value} }} }} }}\n  - {{ name: observe, shell: echo OBSERVED }}\n")
}

/// A root whose first row writes `rank` as `value` and whose second reaches
/// `sub/check.yaml` through `route` (a list item without its leading `- `).
fn writing_root(value: &str, route: &str) -> String {
    format!("kind: sequence\nsequence:\n  - {{ name: change, side_effect: {{ set: {{ rank: {value} }} }} }}\n  - {route}\n")
}

const NESTED: &str = "{ name: nested, sequence: sub/check.yaml }";
const GROUP_LEG: &str = "{ name: checks, group: { name: checks, tasks: [{ name: leg, sequence: sub/check.yaml }] } }";
const TASK_FILE: &str = "{ name: delegated, task: task.yaml }";
const TASK_YAML: &str = "kind: task\nsequence: sub/check.yaml\n";

/// Runs `root` without `--silent`, so a shell row's output reaches stdout,
/// and returns stdout, stderr with its frame and wrapping flattened, and the
/// exit code.
fn run_observed(root: &str, files: &[(&str, &str)]) -> (String, String, i32) {
    let fixture = CliProcessFixture::named("sequence-referenced-schema");
    #[cfg(feature = "test-fixtures")]
    fixture.install_echo();
    for (name, contents) in files {
        let path = fixture.cwd().join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }
    let output = fixture.command().args(["sequence", "--goose", "--yolo"])
        .arg(fixture.cwd().join(root)).assert().get_output().clone();
    assert!(!fixture.audio_spool().exists());
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    let flat = stderr.replace('\u{2503}', " ").split_whitespace().collect::<Vec<_>>().join(" ");
    (strip_ansi(&String::from_utf8_lossy(&output.stdout)), flat, output.status.code().unwrap_or(-1))
}

/// Asserts the run failed naming `/rank` and `declaring`, and ran no row.
fn assert_rejected(root: &str, files: &[(&str, &str)], declaring: &str) {
    let (stdout, stderr, code) = run_observed(root, files);
    assert_ne!(code, 0, "{stderr}\n{stdout}");
    assert!(!stdout.contains("OBSERVED"), "the row ran: {stdout}\n{stderr}");
    assert!(stderr.contains("/rank"), "{stderr}");
    assert!(stderr.contains(declaring), "the error names the declaring file: {stderr}");
}

/// Asserts the run succeeded and its row ran.
fn assert_observed(root: &str, files: &[(&str, &str)]) {
    let (stdout, stderr, code) = run_observed(root, files);
    assert_eq!(code, 0, "{stderr}\n{stdout}");
    assert!(stdout.contains("OBSERVED"), "{stdout}\n{stderr}");
}

#[test]
fn a_root_reference_judges_its_document_after_its_own_row_writes_an_invalid_value() {
    let child = self_writing("bad");
    assert_rejected("root.yaml", &[("root.yaml", "kind: sequence\nsequence: sub/check.yaml\n"), ("sub/check.yaml", &child)], "sub/check.yaml");
}

#[test]
fn a_nested_reference_judges_its_document_against_an_earlier_invalid_write() {
    assert_rejected("root.yaml", &[("root.yaml", &writing_root("bad", NESTED)), ("sub/check.yaml", RANKED)], "sub/check.yaml");
}

#[test]
fn a_group_leg_reference_judges_its_document_against_an_earlier_invalid_write() {
    assert_rejected("root.yaml", &[("root.yaml", &writing_root("bad", GROUP_LEG)), ("sub/check.yaml", RANKED)], "sub/check.yaml");
}

#[test]
fn a_task_file_reference_judges_its_document_against_an_earlier_invalid_write() {
    assert_rejected("root.yaml", &[
        ("root.yaml", &writing_root("bad", TASK_FILE)), ("task.yaml", TASK_YAML), ("sub/check.yaml", RANKED),
    ], "sub/check.yaml");
}

// Controls: the same schema and write in a directly run document, and in an
// inline container's check, are refused the same way.
#[test]
fn a_directly_run_document_judges_its_own_invalid_write() {
    let child = self_writing("bad");
    assert_rejected("sub/check.yaml", &[("sub/check.yaml", &child)], "sub/check.yaml");
}

#[test]
fn an_inline_container_check_judges_an_earlier_invalid_write() {
    let root = "kind: sequence\nrank: 1\nsequence:\n  - { name: change, side_effect: { set: { rank: bad } } }\n  - name: inner\n    $schema: { rank: number(required) }\n    sequence:\n      - { name: observe, shell: echo OBSERVED }\n";
    assert_rejected("root.yaml", &[("root.yaml", root)], "root.yaml");
}

// A valid write, and no write at all, run the row on every route.
#[test]
fn a_root_reference_runs_its_document_after_a_valid_write() {
    let child = self_writing("2");
    assert_observed("root.yaml", &[("root.yaml", "kind: sequence\nsequence: sub/check.yaml\n"), ("sub/check.yaml", &child)]);
}

#[test]
fn a_root_reference_accepts_its_documents_valid_authored_values() {
    assert_observed("root.yaml", &[("root.yaml", "kind: sequence\nsequence: sub/check.yaml\n"), ("sub/check.yaml", RANKED)]);
}

macro_rules! valid_route {
    ($written:ident, $authored:ident, $route:expr) => {
        #[test]
        fn $written() {
            assert_observed("root.yaml", &[("root.yaml", &writing_root("2", $route)), ("task.yaml", TASK_YAML), ("sub/check.yaml", RANKED)]);
        }

        #[test]
        fn $authored() {
            let root = format!("kind: sequence\nsequence:\n  - {}\n", $route);
            assert_observed("root.yaml", &[("root.yaml", &root), ("task.yaml", TASK_YAML), ("sub/check.yaml", RANKED)]);
        }
    };
}

valid_route!(a_nested_reference_runs_its_document_after_a_valid_write,
    a_nested_reference_accepts_its_documents_valid_authored_values, NESTED);
valid_route!(a_group_leg_reference_runs_its_document_after_a_valid_write,
    a_group_leg_reference_accepts_its_documents_valid_authored_values, GROUP_LEG);
valid_route!(a_task_file_reference_runs_its_document_after_a_valid_write,
    a_task_file_reference_accepts_its_documents_valid_authored_values, TASK_FILE);

// A document with no rows has no step whose preparation could catch it: it is
// judged where it is entered all the same.
const EMPTY: &str = "kind: sequence\n$schema:\n  rank: number(required)\nrank: 1\nsequence: []\n";

#[test]
fn an_empty_nested_document_is_judged_against_an_earlier_invalid_write() {
    assert_rejected("root.yaml", &[("root.yaml", &writing_root("bad", NESTED)), ("sub/check.yaml", EMPTY)], "sub/check.yaml");
}

#[test]
fn an_empty_root_referenced_document_is_judged_against_a_root_initialize_write() {
    assert_rejected("root.yaml", &[
        ("root.yaml", "kind: sequence\ninitialize: [{ set: { rank: bad } }]\nsequence: sub/check.yaml\n"), ("sub/check.yaml", EMPTY),
    ], "sub/check.yaml");
}

// The document's own `initialize` runs before the verdict, so it can still
// correct a value an earlier task wrote.
#[test]
fn a_referenced_documents_initialize_corrects_a_value_before_it_is_judged() {
    let child = "kind: sequence\n$schema:\n  rank: number(required)\nrank: 1\ninitialize: [{ set: { rank: 2 } }]\nsequence:\n  - { name: observe, shell: echo OBSERVED }\n";
    assert_observed("root.yaml", &[("root.yaml", &writing_root("bad", NESTED)), ("sub/check.yaml", child)]);
}

// Each loop pass enters the document again, and is judged again.
#[test]
fn a_looping_referenced_document_is_judged_again_at_its_next_pass() {
    let child = "kind: sequence\n$schema:\n  rank: number(required)\nrank: 1\nloop: { max: 2, while: \"true\" }\nsequence:\n  - { name: observe, shell: echo OBSERVED }\n  - { name: change, side_effect: { set: { rank: bad } } }\n";
    let (stdout, stderr, code) = run_observed("root.yaml", &[("root.yaml", &format!("kind: sequence\nsequence:\n  - {NESTED}\n")), ("sub/check.yaml", child)]);
    assert_ne!(code, 0, "{stderr}\n{stdout}");
    assert_eq!(stdout.matches("OBSERVED").count(), 1, "only the first pass ran its row: {stdout}\n{stderr}");
    assert!(stderr.contains("/rank") && stderr.contains("sub/check.yaml"), "{stderr}");
}

/// A referenced document whose events and an inline child's event print a
/// schema default and a converted number.
const PREPARED_CHILD: &str = "kind: sequence\n$schema:\n  who: string(default(ada))\n  rank: number\nrank: \"3\"\nstart: [{ stdout: \"start {{ who }} {{ rank + 1 }}\" }]\nsequence:\n  - name: inner\n    sequence: []\n    start: [{ stdout: \"child {{ who }} {{ rank + 1 }}\" }]\n";

// A referenced document's events and children read its prepared values: the
// default fills `who` and `rank` is the number its schema converts it to.
referenced_state!(a_referenced_documents_events_and_children_read_its_prepared_values,
    root: "root.yaml",
    files: [
        ("root.yaml", "kind: sequence\nsequence:\n  - { name: nested, sequence: sub/child.yaml }\n"),
        ("sub/child.yaml", PREPARED_CHILD),
    ],
    expected: "start ada 4\nchild ada 4");

referenced_state!(a_setter_layers_over_a_referenced_documents_prepared_values,
    root: "root.yaml",
    files: [
        ("root.yaml", "kind: sequence\nsequence:\n  - { name: nested, sequence: sub/child.yaml }\n"),
        ("sub/child.yaml", PREPARED_CHILD),
    ],
    args: ["who=grace"],
    expected: "start grace 4\nchild grace 4");

// ============================================================================
// Direct, root-reference, and nested runs agree
// ============================================================================
//
// The same document run directly, through the root's `sequence:` reference,
// and as a nested task reads the same values in its events and its rows: its
// prepared values, then the writes made so far. The invoking root's own
// values never reach it.

/// A document whose events print a schema default and a converted number.
const EVENTS_CHILD: &str = "kind: sequence\n$schema:\n  who: string(default(ada))\n  rank: number\nrank: \"3\"\ninitialize: [{ stdout: \"init {{ who }} {{ rank + 1 }}\" }]\nstart: [{ stdout: \"start {{ who }} {{ rank + 1 }}\" }]\nsuccess: [{ stdout: \"success {{ who }} {{ rank + 1 }}\" }]\nsequence: []\n";

/// Roots that define `rank` themselves and reach `sub/child.yaml` through
/// the root's own `sequence:` reference or a nested task.
const RANKED_ROOT_REFERENCE: &str = "kind: sequence\nrank: 9\nsequence: sub/child.yaml\n";
const RANKED_NESTED_REFERENCE: &str = "kind: sequence\nrank: 9\nsequence:\n  - { name: nested, sequence: sub/child.yaml }\n";

referenced_state!(a_directly_run_documents_events_read_its_prepared_values,
    root: "sub/child.yaml",
    files: [("sub/child.yaml", EVENTS_CHILD)],
    expected: "init ada 4\nstart ada 4\nsuccess ada 4");

referenced_state!(a_nested_documents_events_read_its_prepared_values_as_a_direct_run_does,
    root: "root.yaml",
    files: [("root.yaml", RANKED_NESTED_REFERENCE), ("sub/child.yaml", EVENTS_CHILD)],
    expected: "init ada 4\nstart ada 4\nsuccess ada 4");

/// A derived value an `initialize` write feeds is prepared again when the
/// document starts, directly and nested alike.
const DERIVED_CHILD: &str = "kind: sequence\n$schema:\n  name: string\ndesc: \"{{ name }}-x\"\ninitialize: [{ set: { name: red } }]\nstart: [{ stdout: \"start {{ desc }}\" }]\nsequence: []\n";

referenced_state!(a_directly_run_documents_start_reads_values_prepared_after_initialize,
    root: "sub/child.yaml",
    files: [("sub/child.yaml", DERIVED_CHILD)],
    expected: "start red-x");

referenced_state!(a_nested_documents_start_reads_values_prepared_after_initialize,
    root: "root.yaml",
    files: [("root.yaml", RANKED_NESTED_REFERENCE), ("sub/child.yaml", DERIVED_CHILD)],
    expected: "start red-x");

/// A document whose rows read its own `rank` (over the root's `rank: 9`), a
/// schema default, and a value one of its rows writes.
const ROWS_CHILD: &str = "kind: sequence\n$schema:\n  who: string(default(ada))\nrank: 1\nsequence:\n  - { name: first, shell: \"echo FIRST {{ rank }} {{ who }}\" }\n  - { name: write, side_effect: { set: { rank: 2 } } }\n  - { name: second, shell: \"echo SECOND {{ rank }} {{ who }}\" }\n  - { name: guarded, when: \"rank == 2\", shell: echo GUARDED }\n";

fn assert_rows_read_their_document(root: &str, files: &[(&str, &str)]) {
    let (stdout, stderr, code) = run_observed(root, files);
    assert_eq!(code, 0, "{stderr}\n{stdout}");
    let lines: Vec<&str> = stdout.lines().map(str::trim)
        .filter(|line| line.starts_with("FIRST") || line.starts_with("SECOND") || line.starts_with("GUARDED")).collect();
    assert_eq!(lines, ["FIRST 1 ada", "SECOND 2 ada", "GUARDED"], "{stdout}\n{stderr}");
}

#[test]
fn a_directly_run_documents_rows_read_its_prepared_values() {
    assert_rows_read_their_document("sub/child.yaml", &[("sub/child.yaml", ROWS_CHILD)]);
}

#[test]
fn a_root_reference_rows_read_their_documents_values_not_the_roots() {
    assert_rows_read_their_document("root.yaml", &[("root.yaml", RANKED_ROOT_REFERENCE), ("sub/child.yaml", ROWS_CHILD)]);
}

#[test]
fn a_nested_documents_rows_read_their_documents_values_not_the_roots() {
    assert_rows_read_their_document("root.yaml", &[("root.yaml", RANKED_NESTED_REFERENCE), ("sub/child.yaml", ROWS_CHILD)]);
}

// ============================================================================
// A root `initialize` write supplies a required input
// ============================================================================
//
// The root's `initialize` runs before any input is judged, so a required
// input it writes is supplied on every route: the root's own steps, an empty
// root, and a whole document the root (or one of its containers) references.

/// A YAML root requiring `who`, which only its `initialize` writes.
const INITIALIZED_ROOT: &str = "kind: sequence\n$schema:\n  who: string(required)\ninitialize: [{ set: { who: ada } }]\n";

/// A child requiring `who`, which it never defines; its row prints it.
const REQUIRING_CHILD: &str = "kind: sequence\n$schema:\n  who: string(required)\nsequence:\n  - { name: work, shell: \"echo {{ who }}\" }\n";

template_fill!(a_root_initialize_write_supplies_the_yaml_roots_required_input,
    root: "root.yaml",
    files: [("root.yaml", &format!("{INITIALIZED_ROOT}sequence:\n  - {{ name: work, shell: \"echo {{{{ who }}}}\" }}\n"))],
    expected: "ada");

template_fill!(a_root_initialize_write_supplies_the_markdown_roots_required_input,
    root: "root.md",
    files: [("root.md", "---\n$schema:\n  who: string(required)\ninitialize: [{ set: { who: ada } }]\nsequence:\n  - { name: work, shell: \"echo {{ who }}\" }\n---\n")],
    expected: "ada");

template_fill!(a_root_initialize_write_supplies_an_empty_roots_required_input,
    root: "root.yaml",
    files: [("root.yaml", &format!("{INITIALIZED_ROOT}sequence: []\nsuccess: [{{ stdout: \"{{{{ who }}}}\" }}]\n"))],
    expected: "ada");

template_fill!(a_root_initialize_write_supplies_a_root_references_required_input,
    root: "root.yaml",
    files: [
        ("root.yaml", "kind: sequence\ninitialize: [{ set: { who: ada } }]\nsequence: sub/child.yaml\n"),
        ("sub/child.yaml", REQUIRING_CHILD),
    ],
    expected: "ada");

template_fill!(a_root_initialize_write_supplies_a_nested_references_required_input,
    root: "root.yaml",
    files: [
        ("root.yaml", "kind: sequence\ninitialize: [{ set: { who: ada } }]\nsequence:\n  - { name: inner, sequence: sub/child.yaml }\n"),
        ("sub/child.yaml", REQUIRING_CHILD),
    ],
    expected: "ada");

template_fill!(a_root_initialize_write_supplies_a_group_legs_required_input,
    root: "root.yaml",
    files: [
        ("root.yaml", "kind: sequence\ninitialize: [{ set: { who: ada } }]\nsequence:\n  - name: checks\n    group:\n      name: checks\n      tasks: [{ name: leg, sequence: sub/child.yaml }]\n"),
        ("sub/child.yaml", REQUIRING_CHILD),
    ],
    expected: "ada");

/// Runs `root` without a terminal and asserts the missing-input report names
/// step 1 and `who`, and that the step's `echo NOT_RUN` did not run.
fn assert_who_reported_missing(root: &str, extra: &[&str]) {
    let fixture = CliProcessFixture::named("sequence-initialize-input");
    #[cfg(feature = "test-fixtures")]
    fixture.install_echo();
    fs::write(fixture.cwd().join("root.yaml"), root).unwrap();
    let output = fixture.command().args(["sequence", "--goose", "--yolo"]).args(extra)
        .arg(fixture.cwd().join("root.yaml")).assert().failure().get_output().clone();
    let stdout = strip_ansi(&String::from_utf8_lossy(&output.stdout));
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    let flat = stderr.replace('\u{2503}', " ").split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(flat.contains("Missing required schema properties in 1 step of the sequence"), "{flat}");
    assert!(flat.contains("- `who`: string"), "{flat}");
    assert!(!stdout.contains("NOT_RUN"), "{stdout}");
    assert!(!fixture.audio_spool().exists());
}

#[test]
fn a_required_root_input_nothing_writes_is_still_reported_missing() {
    assert_who_reported_missing("kind: sequence\n$schema:\n  who: string(required)\nsequence:\n  - { name: work, shell: echo NOT_RUN }\n", &[]);
}

// A dry run fires no `initialize`: the input it would write is `not
// checked`, never missing, since the real run reads it before collecting.
#[test]
fn a_dry_run_leaves_an_input_only_initialize_writes_not_checked() {
    let fixture = CliProcessFixture::named("sequence-initialize-input-dry-run");
    fs::write(fixture.cwd().join("root.yaml"), format!("{INITIALIZED_ROOT}sequence:\n  - {{ name: work, shell: echo NOT_RUN }}\n")).unwrap();
    let output = fixture.command().args(["sequence", "--goose", "--dry-run", "--json"]).arg(fixture.cwd().join("root.yaml")).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.status.code(), Some(0), "{stdout}");
    let envelope: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let diagnostics = envelope["diagnostics"].as_array().unwrap();
    assert!(!diagnostics.iter().any(|diagnostic| diagnostic["code"] == "composition.missing_properties"), "{stdout}");
    assert!(stdout.contains("input `who`"), "{stdout}");
    assert!(!fixture.audio_spool().exists());
}

// ============================================================================
// A derived value reaches a first-position shell as its document prepares it
// ============================================================================
//
// `greeting` is derived from `name`. A first-position shell reading it, as a
// row, a container's `start` shell, or a shell-valued `set`, runs with the
// prepared `hi ada`, never the expression text, and an inline container's
// reread finds the command it approved, on every route.

/// The declaring document's derived property, written before its rows.
const DERIVED: &str = "name: ada\ngreeting: \"hi {{ name }}\"\n";

/// `sub/child.yaml`, declaring `greeting`, whose body is `rest`.
fn derived_child(rest: &str) -> String {
    format!("kind: sequence\n{DERIVED}{rest}")
}

const FIRST_ROW: &str = "sequence:\n  - name: work\n    shell: \"echo {{ greeting }}\"\n";
const INLINE_CHILD: &str = "sequence:\n  - name: inner\n    sequence:\n      - name: work\n        shell: \"echo {{ greeting }}\"\n";
const START_SHELL: &str = "start: [{ shell: \"echo {{ greeting }}\" }]\nsequence: []\n";
const SHELL_VALUED_SET: &str = "start:\n  - set: { captured: \"$(echo {{ greeting }})\" }\n  - stdout: \"{{ captured }}\"\nsequence: []\n";
const INLINE_START_SHELL: &str = "sequence:\n  - name: inner\n    start: [{ shell: \"echo {{ greeting }}\" }]\n    sequence: []\n";
const GROUP_START_SHELL: &str = "sequence:\n  - name: fan\n    group:\n      name: fan\n      start: [{ shell: \"echo {{ greeting }}\" }]\n      tasks: [{ name: leg, side_effect: { set: { done: true } } }]\n";

/// The roots that reach `sub/child.yaml`.
const DERIVED_ROOT_REFERENCE: &str = "kind: sequence\nsequence: sub/child.yaml\n";
const DERIVED_NESTED_REFERENCE: &str = "kind: sequence\nsequence:\n  - { name: nested, sequence: sub/child.yaml }\n";
const DERIVED_GROUP_LEG_REFERENCE: &str = "kind: sequence\nsequence:\n  - name: fan\n    group:\n      name: fan\n      tasks: [{ name: leg, sequence: sub/child.yaml }]\n";
const DERIVED_TASK_FILE_REFERENCE: &str = "kind: sequence\nsequence:\n  - { name: via, task: task.yaml }\n";
const DERIVED_TASK_FILE: &str = "kind: task\nsequence: sub/child.yaml\n";

/// Asserts the run succeeded and printed `hi ada` (a group leg's output
/// framed by its gutter) and no expression text.
fn assert_prints_derived(root: &str, files: &[(&str, &str)]) {
    let (stdout, stderr, code) = run_observed(root, files);
    assert_eq!(code, 0, "{stderr}\n{stdout}");
    assert!(stdout.lines().any(|line| line.trim().trim_start_matches('\u{2502}').trim() == "hi ada"), "{stdout}\n{stderr}");
    assert!(!stdout.contains("{{"), "{stdout}\n{stderr}");
}

macro_rules! derived_shell {
    ($name:ident, body: $body:expr, route: direct) => {
        #[test]
        fn $name() {
            assert_prints_derived("sub/child.yaml", &[("sub/child.yaml", &derived_child($body))]);
        }
    };
    ($name:ident, body: $body:expr, route: $root:expr) => {
        #[test]
        fn $name() {
            assert_prints_derived("root.yaml", &[
                ("root.yaml", $root), ("sub/child.yaml", &derived_child($body)), ("task.yaml", DERIVED_TASK_FILE),
            ]);
        }
    };
}

derived_shell!(a_direct_first_row_prints_a_derived_value, body: FIRST_ROW, route: direct);
derived_shell!(a_root_reference_first_row_prints_a_derived_value, body: FIRST_ROW, route: DERIVED_ROOT_REFERENCE);
derived_shell!(a_nested_first_row_prints_a_derived_value, body: FIRST_ROW, route: DERIVED_NESTED_REFERENCE);
derived_shell!(a_group_leg_first_row_prints_a_derived_value, body: FIRST_ROW, route: DERIVED_GROUP_LEG_REFERENCE);
derived_shell!(a_task_file_first_row_prints_a_derived_value, body: FIRST_ROW, route: DERIVED_TASK_FILE_REFERENCE);
derived_shell!(a_direct_inline_child_runs_its_approved_derived_command, body: INLINE_CHILD, route: direct);
derived_shell!(a_root_reference_inline_child_runs_its_approved_derived_command, body: INLINE_CHILD, route: DERIVED_ROOT_REFERENCE);
derived_shell!(a_nested_inline_child_runs_its_approved_derived_command, body: INLINE_CHILD, route: DERIVED_NESTED_REFERENCE);
derived_shell!(a_group_leg_inline_child_runs_its_approved_derived_command, body: INLINE_CHILD, route: DERIVED_GROUP_LEG_REFERENCE);
derived_shell!(a_task_file_inline_child_runs_its_approved_derived_command, body: INLINE_CHILD, route: DERIVED_TASK_FILE_REFERENCE);
derived_shell!(a_direct_start_shell_prints_a_derived_value, body: START_SHELL, route: direct);
derived_shell!(a_nested_start_shell_prints_a_derived_value, body: START_SHELL, route: DERIVED_NESTED_REFERENCE);
derived_shell!(a_group_leg_start_shell_prints_a_derived_value, body: START_SHELL, route: DERIVED_GROUP_LEG_REFERENCE);
derived_shell!(a_task_file_start_shell_prints_a_derived_value, body: START_SHELL, route: DERIVED_TASK_FILE_REFERENCE);
derived_shell!(a_direct_inline_container_start_shell_prints_a_derived_value, body: INLINE_START_SHELL, route: direct);
derived_shell!(a_nested_inline_container_start_shell_prints_a_derived_value, body: INLINE_START_SHELL, route: DERIVED_NESTED_REFERENCE);
derived_shell!(a_direct_group_start_shell_prints_a_derived_value, body: GROUP_START_SHELL, route: direct);
derived_shell!(a_nested_group_start_shell_prints_a_derived_value, body: GROUP_START_SHELL, route: DERIVED_NESTED_REFERENCE);
derived_shell!(a_direct_shell_valued_set_captures_a_derived_value, body: SHELL_VALUED_SET, route: direct);
derived_shell!(a_nested_shell_valued_set_captures_a_derived_value, body: SHELL_VALUED_SET, route: DERIVED_NESTED_REFERENCE);
derived_shell!(a_group_leg_shell_valued_set_captures_a_derived_value, body: SHELL_VALUED_SET, route: DERIVED_GROUP_LEG_REFERENCE);
derived_shell!(a_task_file_shell_valued_set_captures_a_derived_value, body: SHELL_VALUED_SET, route: DERIVED_TASK_FILE_REFERENCE);

/// A derived value a `start` write changes before the first row runs is
/// read as the row runs with it, not frozen at launch.
#[test]
fn a_first_row_reads_a_derived_value_a_start_write_changes() {
    let child = "kind: sequence\nname: bob\ngreeting: \"hi {{ name }}\"\nstart: [{ set: { name: ada } }]\nsequence:\n  - name: work\n    shell: \"echo {{ greeting }}\"\n";
    assert_prints_derived("sub/child.yaml", &[("sub/child.yaml", child)]);
}

const LATER_ROW: &str = "sequence:\n  - name: begin\n    side_effect: { set: { started: true } }\n  - name: work\n    shell: \"echo {{ greeting }}\"\n";

derived_shell!(a_direct_later_row_prints_a_derived_value, body: LATER_ROW, route: direct);
derived_shell!(a_nested_later_row_prints_a_derived_value, body: LATER_ROW, route: DERIVED_NESTED_REFERENCE);

/// A literal property is read as written by a first-row shell and an inline
/// child alike.
#[test]
fn a_literal_property_is_read_as_written_by_first_position_shells() {
    for body in [FIRST_ROW, INLINE_CHILD] {
        let child = format!("kind: sequence\ngreeting: hi ada\n{body}");
        assert_prints_derived("sub/child.yaml", &[("sub/child.yaml", &child)]);
    }
}

/// A `side_effect:` shell is refused with its typed hint, a derived value
/// or not: it is no shell consumer of its own.
#[test]
fn a_side_effect_shell_reading_a_derived_value_is_refused_with_its_hint() {
    let child = derived_child("sequence:\n  - name: work\n    side_effect: { shell: \"echo {{ greeting }}\" }\n");
    let (stdout, stderr, code) = run_observed("sub/child.yaml", &[("sub/child.yaml", &child)]);
    assert_ne!(code, 0, "{stderr}\n{stdout}");
    assert!(stderr.contains("use a `shell:` task"), "{stderr}");
    assert!(!stdout.contains("hi "), "{stdout}");
}
