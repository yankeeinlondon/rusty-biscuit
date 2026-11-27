//! A prompt's checks read what the run writes before the prompt composes as
//! known only at launch: a key an earlier task's or the root's lifecycle
//! `set` writes, a later step's `seq.outputs`, and a loop pass's `loop.*`.
//! The dry run leaves each check that reads one `not checked` (a `::block`
//! stays as a conditional, a schema verdict on the key is not judged, a
//! `with:` value derived from it is not evaluated) instead of judging the
//! value it holds before the run. Each row pairs the dry run with the real
//! non-interactive run of the same files: a write of the read key passes
//! both, and a write of an unrelated `other` leaves the value static, which
//! both refuse. Static values stay checked: the step's position, a literal,
//! the pre-flight reference walk, and a standalone document's own
//! `initialize`, whose writes do not reach its own composition. A pass the
//! real run makes before any task starts (a prompt's approval discovery,
//! Phase 1c's probe of a root step) reads such a value as it is then, so
//! the dry run reports what that pass refuses, while its preview still
//! leaves the later value unchecked.
//!
//! The real runs use the compiled `claudine-fake-goose`, which appends one
//! line per launch to `PROVIDER_MARKER`.

use std::fs;
use std::path::PathBuf;

use serde_json::Value;

use crate::common::{CliProcessFixture, strip_ansi};

/// What the dry run concludes about the reader.
enum Dry {
    /// Exit 0, with a `not checked` row naming each check, and no error.
    Unchecked(&'static [&'static str]),
    /// Exit 0 and no `not checked` row: every check was made and passed.
    Proven,
    /// Exit 2 with an error row of this code.
    Refused(&'static str),
    /// Exit 2 with an error row of this code, and exactly these checks
    /// `not checked` as known only at launch.
    RefusedLeaving(&'static str, &'static [&'static str]),
    /// Exit 2 with this code as the only error, the refusal of a pass the
    /// run makes before any task starts, while each of these later checks
    /// stays `not checked` as known only at launch: no verdict is drawn
    /// from the value the run writes first.
    RefusedEarlier(&'static str, &'static [&'static str]),
}

/// A sequence whose `first` task runs `writer.md`, which writes `key` on
/// success, then `reader`, with `reader` as its prompt and `with` (if any)
/// as its `with:`.
fn two_tasks(key: &str, reader: &str, with: &str, root: &str) -> Vec<(&'static str, String)> {
    vec![
        (
            "seq.md",
            format!("---\nagent: goose\n{root}sequence:\n  - name: first\n    prompt: writer.md\n  - name: reader\n    prompt: reader.md\n{with}---\n"),
        ),
        ("writer.md", format!("---\nsuccess:\n  - set:\n      {key}: a\n---\n\nWrite.\n")),
        ("reader.md", reader.to_string()),
    ]
}

const GUARDED: &str = "---\ntopic: missing\n---\n\n::block when=\"topic == 'a'\"\nRead.\n::end-block\n";
const SCHEMA: &str = "---\ntopic: missing\n$schema:\n  topic: enum(a)\n---\n\nRead {{ topic }}.\n";
const SCHEMA_ONLY: &str = "---\n$schema:\n  topic: enum(a)\n---\n\nRead {{ topic }}.\n";

fn guarded(condition: &str) -> String {
    format!("---\n---\n\n::block when=\"{condition}\"\nRead.\n::end-block\n")
}

/// R2-F2.1: the reader's whole body is guarded by `topic == 'a'`.
fn page_block(key: &str) -> Vec<(&'static str, String)> {
    two_tasks(key, GUARDED, "", "")
}

/// R2-F2.2: the reader's schema refuses its own `topic: missing`.
fn schema(key: &str) -> Vec<(&'static str, String)> {
    two_tasks(key, SCHEMA, "", "")
}

/// R2-F2.3: the reader's `topic` is the task's `with:`, evaluated from the
/// sequence's `choice`, which reads the sequence's `topic`.
fn with_chain(key: &str) -> Vec<(&'static str, String)> {
    two_tasks(key, SCHEMA_ONLY, "    with: { topic: \"{{ choice }}\" }\n", "topic: missing\nchoice: \"{{ topic }}\"\n")
}

/// R2-F2.3 under another name: the reader's `subject` is evaluated from the
/// written `topic`, which the reader itself never names.
fn with_chain_renamed(key: &str) -> Vec<(&'static str, String)> {
    two_tasks(
        key,
        "---
$schema:
  subject: enum(a)
---

Read {{ subject }}.
",
        "    with: { subject: \"{{ choice }}\" }\n",
        "topic: missing\nchoice: \"{{ topic }}\"\n",
    )
}

/// R2-F2.4: the task's `with:` is the literal the schema admits.
fn with_literal(key: &str) -> Vec<(&'static str, String)> {
    two_tasks(key, SCHEMA_ONLY, "    with: { topic: a }\n", "topic: missing\n")
}

/// R2-F2.6: a root body step, guarded by `topic == 'a'`, after the root's
/// `initialize` writes `key`.
fn root_initialize(key: &str) -> Vec<(&'static str, String)> {
    vec![(
        "seq.md",
        format!(
            "---\nagent: goose\ntopic: missing\ninitialize:\n  - set:\n      {key}: a\nsequence:\n  - name: body\n---\n\n::block when=\"topic == 'a'\"\nRead.\n::end-block\n"
        ),
    )]
}

/// A root body step, guarded by `topic == 'a'`, after a prompt task whose
/// `success` writes `key`. Phase 1c composes the step before any task runs,
/// when `topic` still holds `missing`.
fn root_body_after_a_write(key: &str) -> Vec<(&'static str, String)> {
    vec![
        (
            "seq.md",
            "---\nagent: goose\ntopic: missing\nsequence:\n  - name: first\n    prompt: writer.md\n  - name: body\n---\n\n::block when=\"topic == 'a'\"\nRead.\n::end-block\n".to_string(),
        ),
        ("writer.md", format!("---\nsuccess:\n  - set:\n      {key}: a\n---\n\nWrite.\n")),
    ]
}

/// R2-F2.7 / R2-F2.8: the reader's body is guarded by `condition`.
fn sequence_state(condition: &str) -> Vec<(&'static str, String)> {
    two_tasks("other", &guarded(condition), "", "")
}

/// R2-F2.9: a standalone document whose `initialize` writes `key` and whose
/// body is wholly guarded by `topic == 'a'`. The document composes with its
/// own values, so both runs refuse the empty body whichever key is written.
fn standalone(key: &str) -> Vec<(&'static str, String)> {
    vec![(
        "doc.md",
        format!(
            "---\nagent: goose\ntopic: missing\ninitialize:\n  - set:\n      {key}: a\n---\n\n::block when=\"topic == 'a'\"\nRead.\n::end-block\n"
        ),
    )]
}

/// R2-F2.10: the reader includes `./{{ topic }}.md`; only `a.md` exists.
fn reference(key: &str) -> Vec<(&'static str, String)> {
    let mut files = two_tasks(key, "---\ntopic: missing\n---\n\n::file ./{{ topic }}.md\n", "", "");
    files.push(("a.md", "Included.\n".to_string()));
    files
}

/// R2-F2.11: the reader sits in a looping nested sequence and reads its
/// pass's `loop.count`.
fn looping() -> Vec<(&'static str, String)> {
    vec![
        (
            "seq.md",
            "---\nagent: goose\nsequence:\n  - name: outer\n    loop:\n      max: 2\n      until: \"loop.count >= 2\"\n    sequence:\n      - name: reader\n        prompt: reader.md\n---\n".to_string(),
        ),
        ("reader.md", guarded("loop.count >= 1")),
    ]
}

fn staged(name: &str, files: &[(&str, String)]) -> CliProcessFixture {
    let fixture = CliProcessFixture::named(name);
    fs::copy(
        biscuit_test_harness::bin_exe!("claudine-fake-goose"),
        fixture.bin_dir().join(format!("goose{}", std::env::consts::EXE_SUFFIX)),
    )
    .expect("the fake provider is built with `test-fixtures`");
    for (path, text) in files {
        fs::write(fixture.cwd().join(path), text).unwrap();
    }
    fixture
}

fn marker(fixture: &CliProcessFixture) -> PathBuf {
    fixture.cwd().join("provider-launched.txt")
}

fn launches(fixture: &CliProcessFixture) -> usize {
    fs::read_to_string(marker(fixture)).map_or(0, |log| log.lines().count())
}

fn dry_run(fixture: &CliProcessFixture, command: &[&str]) -> (Option<i32>, Value) {
    let output = fixture
        .command()
        .env("PROVIDER_MARKER", marker(fixture))
        .args(command)
        .args(["--dry-run", "--json"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let envelope = serde_json::from_str(&stdout).unwrap_or_else(|error| panic!("{error}: {stdout}"));
    assert_eq!(launches(fixture), 0, "a dry run launched a provider");
    (output.status.code(), envelope)
}

fn real_run(fixture: &CliProcessFixture, command: &[&str]) -> (Option<i32>, String) {
    let output = fixture.command().env("PROVIDER_MARKER", marker(fixture)).args(command).output().unwrap();
    (output.status.code(), strip_ansi(&String::from_utf8_lossy(&output.stderr)))
}

/// Every row as `(code, severity, message)`.
fn rows(envelope: &Value) -> Vec<(String, String, String)> {
    envelope["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            let text = |key: &str| row[key].as_str().unwrap_or_default().to_string();
            (text("code"), text("severity"), text("message"))
        })
        .collect()
}

fn assert_dry(code: Option<i32>, envelope: &Value, expected: &Dry) {
    let rows = rows(envelope);
    let unchecked: Vec<&String> =
        rows.iter().filter(|(code, _, _)| code == "dm.check.not_checked").map(|(_, _, message)| message).collect();
    let errors: Vec<&String> = rows.iter().filter(|(_, severity, _)| severity == "error").map(|(code, _, _)| code).collect();
    match expected {
        Dry::Unchecked(checks) => {
            assert_eq!(code, Some(0), "{rows:#?}");
            assert!(errors.is_empty(), "{rows:#?}");
            for check in *checks {
                assert!(
                    unchecked.iter().any(|message| message.contains(&format!("not checked: {check}"))
                        && (message.contains("known only at launch") || message.contains("known only when the task runs"))),
                    "`{check}` must be unchecked as known only at launch: {rows:#?}"
                );
            }
        }
        Dry::Proven => {
            assert_eq!(code, Some(0), "{rows:#?}");
            assert!(errors.is_empty() && unchecked.is_empty(), "{rows:#?}");
        }
        Dry::Refused(expected) => {
            assert_eq!(code, Some(2), "{rows:#?}");
            assert!(errors.iter().any(|code| code == expected), "expected `{expected}`: {rows:#?}");
        }
        Dry::RefusedEarlier(expected, checks) => {
            assert_eq!(code, Some(2), "{rows:#?}");
            assert_eq!(errors.iter().map(|code| code.as_str()).collect::<Vec<_>>(), [*expected], "{rows:#?}");
            for check in *checks {
                assert!(
                    unchecked.iter().any(|message| message.contains(&format!("not checked: {check}")) && message.contains("known only at launch")),
                    "`{check}` must be unchecked as known only at launch: {rows:#?}"
                );
            }
        }
        Dry::RefusedLeaving(expected, checks) => {
            assert_eq!(code, Some(2), "{rows:#?}");
            assert!(errors.iter().any(|code| code == expected), "expected `{expected}`: {rows:#?}");
            assert_eq!(unchecked.len(), checks.len(), "{rows:#?}");
            for check in *checks {
                assert!(
                    unchecked.iter().any(|message| message.contains(&format!("not checked: {check}")) && message.contains("known only at launch")),
                    "`{check}` must be unchecked as known only at launch: {rows:#?}"
                );
            }
        }
    }
}

/// One `#[test]` per row: the dry run's conclusion, then the real run's exit
/// code (and how many tasks launched) for the same files.
macro_rules! later_state {
    ($($name:ident: $files:expr, $command:expr, $dry:expr, real: $real:expr, launched: $launched:expr;)*) => {$(
        #[test]
        fn $name() {
            let fixture = staged(concat!("later-state-", stringify!($name)), &$files);
            let command: &[&str] = &$command;
            let (code, envelope) = dry_run(&fixture, command);
            assert_dry(code, &envelope, &$dry);
            let real: Option<i32> = $real;
            if let Some(expected) = real {
                let (code, stderr) = real_run(&fixture, command);
                assert_eq!(code, Some(expected), "{stderr}");
                assert_eq!(launches(&fixture), $launched, "{stderr}");
            }
        }
    )*};
}

const SEQUENCE: [&str; 2] = ["sequence", "seq.md"];

later_state! {
    page_block_after_a_write_of_its_key_is_conditional: page_block("topic"), SEQUENCE,
        Dry::Unchecked(&["`::block` condition"]), real: Some(0), launched: 2;
    page_block_after_an_unrelated_write_is_the_proven_empty_body: page_block("other"), SEQUENCE,
        Dry::Refused("dm.compose.empty_body"), real: Some(1), launched: 1;
    schema_after_a_write_of_its_key_is_not_judged: schema("topic"), SEQUENCE,
        Dry::Unchecked(&["schema check", "interpolation"]), real: Some(0), launched: 2;
    schema_after_an_unrelated_write_is_the_proven_constraint: schema("other"), SEQUENCE,
        Dry::Refused("dm.schema.constraint"), real: Some(1), launched: 1;
    with_chain_to_a_written_key_is_known_at_the_tasks_turn: with_chain("topic"), SEQUENCE,
        Dry::Unchecked(&["input `topic`", "interpolation"]), real: Some(0), launched: 2;
    with_chain_beside_an_unrelated_write_is_the_proven_constraint: with_chain("other"), SEQUENCE,
        Dry::Refused("dm.schema.constraint"), real: Some(1), launched: 1;
    with_chain_under_another_name_is_known_at_the_tasks_turn: with_chain_renamed("topic"), SEQUENCE,
        Dry::Unchecked(&["input `subject`"]), real: Some(0), launched: 2;
    with_chain_under_another_name_beside_an_unrelated_write_is_proven_wrong: with_chain_renamed("other"), SEQUENCE,
        Dry::Refused("dm.schema.constraint"), real: Some(1), launched: 1;
    with_literal_beside_an_unrelated_write_is_proven: with_literal("other"), SEQUENCE,
        Dry::Proven, real: Some(0), launched: 2;
    // The write replaces the literal at the task's turn, so the reader's
    // reads of it wait for launch; both runs pass.
    with_literal_under_a_write_of_its_key_passes_both_runs: with_literal("topic"), SEQUENCE,
        Dry::Unchecked(&["interpolation"]), real: Some(0), launched: 2;
    root_initialize_writing_the_key_leaves_the_body_conditional: root_initialize("topic"), SEQUENCE,
        Dry::Unchecked(&["`::block` condition"]), real: Some(0), launched: 1;
    root_initialize_writing_another_key_is_the_proven_empty_body: root_initialize("other"), SEQUENCE,
        Dry::Refused("dm.compose.empty_body"), real: Some(1), launched: 0;
    // Phase 1c composes a root step before any task, with only the root
    // `initialize` writes: a later task's write of the key is not there yet.
    a_root_body_step_after_a_tasks_write_of_its_key_is_the_probes_empty_body: root_body_after_a_write("topic"), SEQUENCE,
        Dry::RefusedEarlier("dm.compose.empty_body", &["`::block` condition"]), real: Some(1), launched: 0;
    a_later_steps_outputs_are_known_only_at_launch: sequence_state("length(seq.outputs) > 0"), SEQUENCE,
        Dry::Unchecked(&["`::block` condition"]), real: Some(0), launched: 2;
    a_true_step_position_composes: sequence_state("seq.index == 2"), SEQUENCE,
        Dry::Proven, real: Some(0), launched: 2;
    a_false_step_position_is_the_proven_empty_body: sequence_state("seq.index == 1"), SEQUENCE,
        Dry::Refused("dm.compose.empty_body"), real: Some(1), launched: 1;
    a_standalone_initialize_writing_the_key_keeps_the_proven_empty_body: standalone("topic"), ["compose", "doc.md"],
        Dry::Refused("dm.compose.empty_body"), real: Some(1), launched: 0;
    a_standalone_initialize_writing_another_key_keeps_the_proven_empty_body: standalone("other"), ["compose", "doc.md"],
        Dry::Refused("dm.compose.empty_body"), real: Some(1), launched: 0;
    a_reference_after_a_write_of_its_key_is_still_checked_before_the_run: reference("topic"), SEQUENCE,
        Dry::Refused("dm.reference.missing_transclusion"), real: Some(1), launched: 0;
    a_reference_after_an_unrelated_write_is_checked_before_the_run: reference("other"), SEQUENCE,
        Dry::Refused("dm.reference.missing_transclusion"), real: Some(1), launched: 0;
    a_loop_pass_value_is_known_only_at_launch: looping(), SEQUENCE,
        Dry::Unchecked(&["`::block` condition"]), real: Some(0), launched: 2;
}

/// R2-F2.9 for `inline-compose`: a document's own `initialize` write does not
/// reach the values it composes with, so the preview shows the value it
/// holds, as the real run's prompt does.
#[test]
fn an_inline_documents_own_initialize_write_does_not_reach_its_prompt() {
    let files = [(
        "inline.md",
        "---\nagent: goose\ntopic: missing\ninitialize:\n  - set:\n      topic: a\nprompt: \"About {{ topic }}.\"\n---\n".to_string(),
    )];
    let fixture = staged("later-state-inline-initialize", &files);
    let output = fixture.command().args(["inline-compose", "inline.md", "--dry-run"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.status.code(), Some(0), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(stdout.contains("About missing."), "{stdout}");
}

/// R3-F1: one read of a launch-time member in each spelling that selects
/// it. A read is deferred by the member it selects, so every spelling of
/// `seq.outputs` agrees, and the same spellings of the static `seq.index`
/// stay checkable.
macro_rules! spellings {
    ($member:literal) => {
        [concat!("seq.", $member), concat!("seq['", $member, "']"), concat!("(seq).", $member)]
    };
}

/// R3-F1.2: the reader interpolates `expression` in its body.
fn interpolated(expression: &str) -> Vec<(&'static str, String)> {
    two_tasks("other", &format!("---\n---\n\nRead {{{{ {expression} }}}}.\n"), "", "")
}

/// R3-F1.3: the reader's `ready` is `yes` only when `condition` holds, and
/// its schema admits only `yes`.
fn enum_property(condition: &str) -> Vec<(&'static str, String)> {
    two_tasks(
        "other",
        &format!("---\nready: \"{{{{ {condition} ? 'yes' : 'no' }}}}\"\n$schema:\n  ready: enum(yes)\n---\n\nRead {{{{ ready }}}}.\n"),
        "",
        "",
    )
}

/// R3-F1.3: the reader's frontmatter `step` is `expression` alone, with no
/// schema, so only its evaluation can refuse it.
fn frontmatter_value(expression: &str) -> Vec<(&'static str, String)> {
    two_tasks("other", &format!("---\nstep: \"{{{{ {expression} }}}}\"\n---\n\nRead {{{{ step }}}}.\n"), "", "")
}

/// R3-F1.3: the reader includes a child whose frontmatter `step` is
/// `expression`; the reader itself reads no host root.
fn included_frontmatter_value(expression: &str) -> Vec<(&'static str, String)> {
    let mut files = two_tasks("other", "---\n---\n\n::file ./child.md\n", "", "");
    files.push(("child.md", format!("---\nstep: \"{{{{ {expression} }}}}\"\n---\n\nRead {{{{ step }}}}.\n")));
    files
}

/// R3-F1.4: the reader is an inline document whose frontmatter `prompt:`
/// is wholly guarded by `condition`.
fn inline_prompt(condition: &str) -> Vec<(&'static str, String)> {
    two_tasks(
        "other",
        &format!("---\nprompt: |\n  ::block when=\"{condition}\"\n  Read.\n  ::end-block\n---\n"),
        "",
        "",
    )
}

/// R3-F1.4: the second task is the root's own body, wholly guarded by
/// `condition`.
fn root_body(condition: &str) -> Vec<(&'static str, String)> {
    vec![
        (
            "seq.md",
            format!("---\nagent: goose\nsequence:\n  - name: first\n    prompt: writer.md\n  - name: body\n---\n\n::block when=\"{condition}\"\nRead.\n::end-block\n"),
        ),
        ("writer.md", "---\nsuccess:\n  - set:\n      other: a\n---\n\nWrite.\n".to_string()),
    ]
}

/// R3-F1.5: the reader's schema admits only `topic: a`, and the task's
/// `with:` gives it `a` only when `condition` holds.
fn with_input(condition: &str) -> Vec<(&'static str, String)> {
    two_tasks("other", SCHEMA_ONLY, &format!("    with: {{ topic: \"{{{{ {condition} ? 'a' : 'b' }}}}\" }}\n"), "")
}

/// R3-F1.6: the reader sits in a looping nested sequence and its body is
/// guarded by `condition`.
fn looping_reader(condition: &str) -> Vec<(&'static str, String)> {
    vec![
        (
            "seq.md",
            "---\nagent: goose\nsequence:\n  - name: outer\n    loop:\n      max: 2\n      until: \"loop.count >= 2\"\n    sequence:\n      - name: reader\n        prompt: reader.md\n---\n".to_string(),
        ),
        ("reader.md", guarded(condition)),
    ]
}

/// R3-F1.7: the reader includes `./a.md` only when `condition` holds, and
/// `./missing.md` otherwise; only `a.md` exists.
fn conditional_reference(condition: &str) -> Vec<(&'static str, String)> {
    let mut files = two_tasks("other", &format!("---\n---\n\n::file ./{{{{ {condition} ? 'a' : 'missing' }}}}.md\n"), "", "");
    files.push(("a.md", "Included.\n".to_string()));
    files
}

/// R3-F1.8: the reader declares its own `seq:` and guards its body by
/// `condition`.
fn authored_seq(condition: &str) -> Vec<(&'static str, String)> {
    two_tasks(
        "other",
        &format!("---\nseq:\n  outputs: [static]\n---\n\n::block when=\"{condition}\"\nRead.\n::end-block\n"),
        "",
        "",
    )
}

/// One module per child row and one `#[test]` per spelling (`s0` … `s3`):
/// each spelling's dry run, then its real run when one is given, in its
/// own fixture. A row has at most four spellings; a test past the row's
/// last spelling has nothing to run.
macro_rules! every_spelling {
    ($($name:ident: $reads:expr, $files:expr, $command:expr, $dry:expr, real: $real:expr, launched: $launched:expr;)*) => {$(
        mod $name {
            use super::*;

            fn spelling(index: usize) {
                let reads = $reads;
                let Some(read) = reads.get(index) else { return };
                let fixture = staged(&format!("{}-{index}", stringify!($name)), &$files(read));
                let command: &[&str] = &$command;
                let (code, envelope) = dry_run(&fixture, command);
                assert_dry(code, &envelope, &$dry);
                let real: Option<i32> = $real;
                if let Some(expected) = real {
                    let (code, stderr) = real_run(&fixture, command);
                    assert_eq!(code, Some(expected), "`{read}`: {stderr}");
                    assert_eq!(launches(&fixture), $launched, "`{read}`: {stderr}");
                }
            }

            #[test]
            fn s0() { spelling(0) }
            #[test]
            fn s1() { spelling(1) }
            #[test]
            fn s2() { spelling(2) }
            #[test]
            fn s3() { spelling(3) }

            #[test]
            fn at_most_four_spellings() {
                assert!($reads.len() <= 4, "add a test for each spelling past the fourth");
            }
        }
    )*};
}

const OUTPUTS: [&str; 4] = ["seq.outputs", "seq['outputs']", "(seq).outputs", "seq['out' + 'puts']"];

fn has(read: &str) -> String {
    format!("length({read}) > 0")
}

every_spelling! {
    // R3-F1.1
    a_page_block_on_later_outputs_is_conditional_in_every_spelling:
        OUTPUTS.map(has), |read: &String| sequence_state(read), SEQUENCE,
        Dry::Unchecked(&["`::block` condition"]), real: Some(0), launched: 2;
    a_page_block_on_the_step_position_composes_in_every_spelling:
        spellings!("index").map(|read| format!("{read} == 2")), |read: &String| sequence_state(read), SEQUENCE,
        Dry::Proven, real: None, launched: 0;
    a_page_block_on_a_false_step_position_is_empty_in_every_spelling:
        spellings!("index").map(|read| format!("{read} == 1")), |read: &String| sequence_state(read), SEQUENCE,
        Dry::Refused("dm.compose.empty_body"), real: None, launched: 0;
    // R3-F1.2
    an_interpolation_of_later_outputs_is_unchecked_in_every_spelling:
        OUTPUTS.map(has), |read: &String| interpolated(read), SEQUENCE,
        Dry::Unchecked(&["interpolation"]), real: None, launched: 0;
    an_interpolation_of_the_step_position_composes_in_every_spelling:
        spellings!("index"), |read: &&str| interpolated(read), SEQUENCE,
        Dry::Proven, real: None, launched: 0;
    // R3-F1.3: approval discovery composes the reader's frontmatter before
    // any task, with no `seq` bound, so `length(null)` stops the run there;
    // the enum verdict on the later value is never drawn.
    an_enum_property_on_later_outputs_is_refused_by_discovery_and_not_judged_in_every_spelling:
        OUTPUTS.map(has), |read: &String| enum_property(read), SEQUENCE,
        Dry::RefusedEarlier("dm.expression.evaluation_failure", &["frontmatter value"]), real: Some(1), launched: 0;
    an_enum_property_on_a_true_step_position_passes_in_every_spelling:
        spellings!("index").map(|read| format!("{read} == 2")), |read: &String| enum_property(read), SEQUENCE,
        Dry::Proven, real: Some(0), launched: 2;
    an_enum_property_on_a_false_step_position_is_refused_in_every_spelling:
        spellings!("index").map(|read| format!("{read} == 1")), |read: &String| enum_property(read), SEQUENCE,
        Dry::Refused("dm.schema.constraint"), real: Some(1), launched: 1;
    // Discovery binds no `seq`, so a static member reads `null` there too:
    // arithmetic on it stops the run before any task, though the preview,
    // with the step's position bound, composes it.
    a_frontmatter_value_failing_on_an_unbound_step_position_is_refused_by_discovery_in_every_spelling:
        spellings!("index").map(|read| format!("{read} + 1")), |read: &String| frontmatter_value(read), SEQUENCE,
        Dry::RefusedEarlier("dm.expression.evaluation_failure", &[]), real: Some(1), launched: 0;
    // Discovery fails only what it evaluates: the body's arithmetic on the
    // unbound position is not one, so both runs pass.
    a_body_interpolation_on_the_step_position_passes_both_runs_in_every_spelling:
        spellings!("index").map(|read| format!("{read} + 1")), |read: &String| interpolated(read), SEQUENCE,
        Dry::Proven, real: Some(0), launched: 2;
    // Its null evaluation succeeds, so neither run refuses it.
    a_frontmatter_value_comparing_the_step_position_passes_in_every_spelling:
        spellings!("index").map(|read| format!("{read} == 2")), |read: &String| frontmatter_value(read), SEQUENCE,
        Dry::Proven, real: Some(0), launched: 2;
    a_frontmatter_value_reading_no_host_root_passes:
        ["1 + 1"], |read: &&str| frontmatter_value(read), SEQUENCE,
        Dry::Proven, real: Some(0), launched: 2;
    // The scan does not open an included child, so any inclusion is walked.
    an_included_frontmatter_value_failing_on_an_unbound_step_position_matches_the_real_run_in_every_spelling:
        spellings!("index").map(|read| format!("{read} + 1")), |read: &String| included_frontmatter_value(read), SEQUENCE,
        Dry::RefusedEarlier("dm.expression.evaluation_failure", &[]), real: Some(1), launched: 0;
    // R3-F1.4
    an_inline_prompt_on_later_outputs_is_conditional_in_every_spelling:
        OUTPUTS.map(has), |read: &String| inline_prompt(read), SEQUENCE,
        Dry::Unchecked(&["`::block` condition"]), real: None, launched: 0;
    an_inline_prompt_on_a_false_step_position_is_empty_in_every_spelling:
        spellings!("index").map(|read| format!("{read} == 1")), |read: &String| inline_prompt(read), SEQUENCE,
        Dry::Refused("dm.compose.empty_body"), real: None, launched: 0;
    // Phase 1c composes a root step before any task, with its position and
    // no outputs yet, so the body is empty there; the preview of the later
    // composition stays conditional.
    a_root_body_step_on_later_outputs_is_the_probes_empty_body_in_every_spelling:
        OUTPUTS.map(has), |read: &String| root_body(read), SEQUENCE,
        Dry::RefusedEarlier("dm.compose.empty_body", &["`::block` condition"]), real: Some(1), launched: 0;
    a_root_body_step_on_a_true_step_position_composes_in_every_spelling:
        spellings!("index").map(|read| format!("{read} == 2")), |read: &String| root_body(read), SEQUENCE,
        Dry::Proven, real: Some(0), launched: 2;
    a_root_body_step_on_a_false_step_position_is_empty_in_every_spelling:
        spellings!("index").map(|read| format!("{read} == 1")), |read: &String| root_body(read), SEQUENCE,
        Dry::Refused("dm.compose.empty_body"), real: Some(1), launched: 0;
    // R3-F1.5
    a_with_input_on_later_outputs_is_known_at_the_tasks_turn_in_every_spelling:
        OUTPUTS.map(has), |read: &String| with_input(read), SEQUENCE,
        Dry::Unchecked(&["input `topic`"]), real: None, launched: 0;
    // R3-F1.6
    the_run_token_is_unchecked_in_every_spelling:
        spellings!("run_id"), |read: &&str| interpolated(read), SEQUENCE,
        Dry::Unchecked(&["interpolation"]), real: None, launched: 0;
    a_loop_pass_value_is_known_only_at_launch_in_every_spelling:
        ["loop.count", "loop['count']", "(loop).count"].map(|read| format!("{read} >= 1")), |read: &String| looping_reader(read), SEQUENCE,
        Dry::Unchecked(&["`::block` condition"]), real: Some(0), launched: 2;
    // R3-F1.7
    a_reference_on_later_outputs_is_checked_against_the_runs_first_state_in_every_spelling:
        OUTPUTS.map(has), |read: &String| conditional_reference(read), SEQUENCE,
        Dry::RefusedLeaving("dm.reference.missing_transclusion", &["interpolation", "transclusion"]), real: Some(1), launched: 0;
    a_reference_on_the_step_position_is_checked_against_the_runs_first_state_in_every_spelling:
        spellings!("index").map(|read| format!("{read} == 2")), |read: &String| conditional_reference(read), SEQUENCE,
        Dry::RefusedLeaving("dm.reference.missing_transclusion", &[]), real: Some(1), launched: 0;
    // R3-F1.8
    an_authored_seq_property_stays_static_in_every_spelling:
        ["fm.seq.outputs", "fm['seq'].outputs", "fm.seq['outputs']"].map(has), |read: &String| authored_seq(read), SEQUENCE,
        Dry::Proven, real: None, launched: 0;
    the_sequence_namespace_beside_an_authored_seq_property_is_deferred:
        ["seq.outputs", "seq['outputs']"].map(has), |read: &String| authored_seq(read), SEQUENCE,
        Dry::Unchecked(&["`::block` condition"]), real: None, launched: 0;
}
