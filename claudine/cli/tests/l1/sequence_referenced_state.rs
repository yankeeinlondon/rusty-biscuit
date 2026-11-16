//! A referenced sequence document is a document: its events and children read
//! its own authored frontmatter, whether it runs directly or from a caller.
//! Invocation setters and runtime mutations layer above that authored state,
//! as they do for a directly run document; the caller's authored frontmatter
//! does not reach it.

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
