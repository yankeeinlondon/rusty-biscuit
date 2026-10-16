//! Input robustness matrix for a document's `loop:` configuration, read from
//! YAML frontmatter through `claudine compose`.
//!
//! Every cell is one textual edit of the counting fixture, so the control row
//! proves each edit is load-bearing. Cells assert the public result only: the
//! exit status, how many prompts the provider received, and their text.
//! The contract table these rows implement, including the cells later grammar
//! work adds, is the feature's `phase-1/matrix-contracts.md`.

use std::fs;

use crate::common::{CliProcessFixture, launch_recorder, strip_ansi};

/// The counting document: three passes, ended by its `until` condition,
/// under a cap that the condition beats.
const CONTROL: &str = "---\nn: 1\nloop:\n  until: \"n >= 3\"\n  action: \"increment(n)\"\n  max: 4\n---\nIteration {{ n }}\n";

/// The observable result of running one cell.
#[derive(Debug, PartialEq)]
enum Expect {
    /// Exit 0 after this many launches.
    Completes(usize),
    /// A non-zero exit after this many launches: the cap was reached.
    Exhausts(usize),
    /// A configuration error before any launch, naming the document.
    Rejects,
}

/// Apply one edit to the control fixture; the edited text must be present so
/// the cell changes what it claims to.
fn edit(from: &str, to: &str) -> String {
    assert!(CONTROL.contains(from), "edit target {from:?} is not in the control fixture");
    CONTROL.replacen(from, to, 1)
}

fn check(document: &str, expected: Expect) {
    let fixture = CliProcessFixture::named("loop-matrix");
    let source = fixture.cwd().join("count.md");
    fs::write(&source, document).unwrap();
    let recordings = launch_recorder::install(&fixture, "claude", "printf 'ok\\n'");
    let output = fixture
        .command()
        .args(["compose", "--claude", "--yolo", "--silent"])
        .arg(&source)
        .output()
        .unwrap();
    let launches = launch_recorder::prompts(&recordings);
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    let observed = match (output.status.success(), launches.len()) {
        (true, count) => Expect::Completes(count),
        (false, 0) => Expect::Rejects,
        (false, count) => Expect::Exhausts(count),
    };
    assert_eq!(observed, expected, "status {:?}; stderr:\n{stderr}", output.status);
    match expected {
        Expect::Rejects => assert!(stderr.contains("count.md"), "a rejection names its document:\n{stderr}"),
        Expect::Completes(count) | Expect::Exhausts(count) => {
            let iterations: Vec<String> = (1..=count).map(|n| format!("Iteration {n}")).collect();
            let received: Vec<&str> = launches.iter().map(|prompt| prompt.trim()).collect();
            if document.contains("increment(n)") {
                assert_eq!(received, iterations);
            }
        }
    }
    assert!(!fixture.audio_spool().exists());
}

macro_rules! matrix {
    ($($name:ident: $document:expr => $expected:expr;)*) => {
        $(
            #[test]
            fn $name() {
                check(AsRef::<str>::as_ref(&$document), $expected);
            }
        )*
    };
}

matrix! {
    control_condition_ends_the_loop: CONTROL => Expect::Completes(3);

    loop_absent_runs_once: edit("loop:\n  until: \"n >= 3\"\n  action: \"increment(n)\"\n  max: 4\n", "") => Expect::Completes(1);
    loop_scalar_is_rejected: edit("loop:\n  until: \"n >= 3\"\n  action: \"increment(n)\"\n  max: 4\n", "loop: 5\n") => Expect::Rejects;
    loop_list_is_rejected: edit("loop:\n  until: \"n >= 3\"\n  action: \"increment(n)\"\n  max: 4\n", "loop: [until]\n") => Expect::Rejects;
    loop_every_member_wrong_is_rejected: edit("  until: \"n >= 3\"\n  action: \"increment(n)\"\n  max: 4\n", "  until: 5\n  action: 7\n  max: true\n") => Expect::Rejects;
    loop_duplicate_key_is_rejected: edit("---\nn: 1\n", "---\nn: 1\nloop:\n  until: \"n >= 2\"\n") => Expect::Rejects;
    loop_invalid_yaml_is_rejected: edit("  max: 4\n", "  max: 4\n  : [\n") => Expect::Rejects;
    loop_unknown_key_is_rejected: edit("  max: 4\n", "  max: 4\n  colour: red\n") => Expect::Rejects;

    until_empty_is_rejected: edit("until: \"n >= 3\"", "until: \"\"") => Expect::Rejects;
    until_misspelled_is_rejected: edit("until:", "untill:") => Expect::Rejects;
    until_list_is_rejected: edit("until: \"n >= 3\"", "until: [\"n >= 3\"]") => Expect::Rejects;
    until_map_is_rejected: edit("until: \"n >= 3\"", "until: {n: 3}") => Expect::Rejects;
    until_duplicate_key_is_rejected: edit("  max: 4\n", "  max: 4\n  until: \"n >= 2\"\n") => Expect::Rejects;

    while_replaces_until: edit("until: \"n >= 3\"", "while: \"n < 3\"") => Expect::Completes(3);
    while_list_is_rejected: edit("until: \"n >= 3\"", "while: [\"n < 3\"]") => Expect::Rejects;

    max_absent_uses_the_default: edit("  max: 4\n", "") => Expect::Completes(3);
    max_zero_is_rejected: edit("max: 4", "max: 0") => Expect::Rejects;
    max_negative_is_rejected: edit("max: 4", "max: -1") => Expect::Rejects;
    max_fractional_is_rejected: edit("max: 4", "max: 1.5") => Expect::Rejects;
    max_overflowing_is_rejected: edit("max: 4", "max: 99999999999999999999999") => Expect::Rejects;
    max_text_is_rejected: edit("max: 4", "max: four") => Expect::Rejects;
    max_empty_is_rejected: edit("max: 4", "max: \"\"") => Expect::Rejects;
    max_list_is_rejected: edit("max: 4", "max: [4]") => Expect::Rejects;
    max_duplicate_key_is_rejected: edit("  max: 4\n", "  max: 4\n  max: 5\n") => Expect::Rejects;

    action_absent_never_advances: edit("  action: \"increment(n)\"\n", "") => Expect::Exhausts(4);
    action_empty_list_is_a_no_op: edit("action: \"increment(n)\"", "action: []") => Expect::Exhausts(4);
    action_list_form_advances: edit("action: \"increment(n)\"", "action: [\"increment(n)\"]") => Expect::Completes(3);
    action_empty_map_is_rejected: edit("action: \"increment(n)\"", "action: {}") => Expect::Rejects;
    action_number_is_rejected: edit("action: \"increment(n)\"", "action: 5") => Expect::Rejects;
    action_one_wrong_element_is_rejected: edit("action: \"increment(n)\"", "action: [\"increment(n)\", 5]") => Expect::Rejects;
    action_every_element_wrong_is_rejected: edit("action: \"increment(n)\"", "action: [5, 6]") => Expect::Rejects;
    action_duplicate_key_is_rejected: edit("  max: 4\n", "  max: 4\n  action: \"increment(n)\"\n") => Expect::Rejects;

    actions_alias_advances: edit("action: \"increment(n)\"", "actions: \"increment(n)\"") => Expect::Completes(3);

    gate_empty_list_is_a_no_op: edit("  max: 4\n", "  max: 4\n  gate: []\n") => Expect::Completes(3);
    gate_mixed_with_loose_verbs_is_rejected: edit("  max: 4\n", "  max: 4\n  gate: []\n  stderr: hi\n") => Expect::Rejects;
    loop_when_is_rejected: edit("  max: 4\n", "  max: 4\n  when: \"n > 0\"\n") => Expect::Rejects;
}
