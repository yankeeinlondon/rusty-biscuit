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
#[derive(Debug, Clone, Copy, PartialEq)]
enum Expect {
    /// Exit 0 after this many launches.
    Completes(usize),
    /// A non-zero exit after this many launches: the cap was reached.
    Exhausts(usize),
    /// A configuration error before any launch, naming the document.
    Rejects,
    /// [`Expect::Rejects`] whose message also says this.
    RejectsWith(&'static str),
}

/// Apply one edit to the control fixture; the edited text must be present so
/// the cell changes what it claims to.
fn edit(from: &str, to: &str) -> String {
    assert!(CONTROL.contains(from), "edit target {from:?} is not in the control fixture");
    CONTROL.replacen(from, to, 1)
}

/// How a cell runs beyond the document: caller setters after the file, and
/// variables for the child.
#[derive(Default)]
struct Invocation {
    setters: &'static [&'static str],
    env: &'static [(&'static str, &'static str)],
}

/// The rendered error block's text with its border and line wrapping removed,
/// so a phrase can be matched wherever the block wrapped it.
fn unwrapped(stderr: &str) -> String {
    stderr.replace('┃', " ").split_whitespace().collect::<Vec<_>>().join(" ")
}

fn check(document: &str, expected: Expect, invocation: Invocation) {
    let fixture = CliProcessFixture::named("loop-matrix");
    let source = fixture.cwd().join("count.md");
    fs::write(&source, document).unwrap();
    let recordings = launch_recorder::install(&fixture, "claude", "printf 'ok\\n'");
    let mut command = fixture.command();
    command.args(["compose", "--claude", "--yolo", "--silent"]).arg(&source).args(invocation.setters);
    for (name, value) in invocation.env {
        command.env(name, value);
    }
    let output = command.output().unwrap();
    let launches = launch_recorder::prompts(&recordings);
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    let observed = match (output.status.success(), launches.len()) {
        (true, count) => Expect::Completes(count),
        (false, 0) => match expected {
            Expect::RejectsWith(_) => expected,
            _ => Expect::Rejects,
        },
        (false, count) => Expect::Exhausts(count),
    };
    assert_eq!(observed, expected, "status {:?}; stderr:\n{stderr}", output.status);
    match expected {
        Expect::Rejects => assert!(stderr.contains("count.md"), "a rejection names its document:\n{stderr}"),
        Expect::RejectsWith(phrase) => {
            assert!(stderr.contains("count.md"), "a rejection names its document:\n{stderr}");
            assert!(unwrapped(&stderr).contains(phrase), "the rejection says {phrase:?}:\n{stderr}");
        }
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
    ($($name:ident: $document:expr $(, $invocation:expr)? => $expected:expr;)*) => {
        $(
            #[test]
            fn $name() {
                #[allow(unused_mut, unused_assignments)]
                let mut invocation = Invocation::default();
                $(invocation = $invocation;)?
                check(AsRef::<str>::as_ref(&$document), $expected, invocation);
            }
        )*
    };
}

/// The replacement a removed `loop.fail_fast` names.
const FAIL_FAST_REPLACEMENT: &str = "a failed iteration is an unhandled failure unless a `failure` event handles it";

/// The control fixture's loop block, for cells that replace all of it.
const LOOP_BLOCK: &str = "loop:\n  until: \"n >= 3\"\n  action: \"increment(n)\"\n  max: 4\n";

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
    until_misspelled_is_rejected: edit("until:", "untill:") => Expect::RejectsWith("unknown `loop.untill` key (did you mean `until`?)");
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

    loop_null_is_rejected: edit(LOOP_BLOCK, "loop: null\n") => Expect::RejectsWith("`loop` must be an object");
    loop_empty_map_is_rejected: edit(LOOP_BLOCK, "loop: {}\n") => Expect::RejectsWith("`loop` must define either `while` or `until`");
    condition_absent_is_rejected: edit("  until: \"n >= 3\"\n", "") => Expect::RejectsWith("`loop` must define either `while` or `until`");
    until_null_is_rejected: edit("until: \"n >= 3\"", "until: null") => Expect::RejectsWith("`loop.until` must be a string, got null");
    while_null_is_rejected: edit("until: \"n >= 3\"", "while: null") => Expect::RejectsWith("`loop.while` must be a string, got null");
    max_null_is_rejected: edit("max: 4", "max: null") => Expect::RejectsWith("`loop.max` must be a positive integer, got null");
    action_null_is_rejected: edit("action: \"increment(n)\"", "action: null") => Expect::RejectsWith("`loop.action` must be a string");
    actions_null_is_rejected: edit("action: \"increment(n)\"", "actions: null") => Expect::RejectsWith("`loop.actions` must be a string");
    both_conditions_are_rejected: edit("  max: 4\n", "  max: 4\n  while: \"n < 3\"\n") => Expect::RejectsWith("`loop.while` and `loop.until` are mutually exclusive");
    both_action_aliases_are_rejected: edit("  max: 4\n", "  max: 4\n  actions: \"increment(n)\"\n") => Expect::RejectsWith("`loop.action` and `loop.actions` are aliases");
    action_empty_string_is_rejected: edit("action: \"increment(n)\"", "action: \"\"") => Expect::RejectsWith("use `[]` for no action");

    fail_fast_true_is_removed: edit("  max: 4\n", "  max: 4\n  fail_fast: true\n") => Expect::RejectsWith(FAIL_FAST_REPLACEMENT);
    fail_fast_false_is_removed: edit("  max: 4\n", "  max: 4\n  fail_fast: false\n") => Expect::RejectsWith(FAIL_FAST_REPLACEMENT);
    fail_fast_null_is_removed: edit("  max: 4\n", "  max: 4\n  fail_fast: null\n") => Expect::RejectsWith(FAIL_FAST_REPLACEMENT);

    on_rate_limit_null_is_rejected: edit("  max: 4\n", "  max: 4\n  on_rate_limit: null\n") => Expect::RejectsWith("`loop.on_rate_limit` must be a string, got null");

    max_template_resolves_once: edit("  max: 4\n", "  max: \"{{ cap }}\"\ncap: 4\n") => Expect::Completes(3);
    max_template_follows_a_caller_setter:
        edit("  action: \"increment(n)\"\n  max: 4\n", "  max: \"{{ cap }}\"\ncap: 4\n"),
        Invocation { setters: &["cap=2"], ..Invocation::default() }
        => Expect::Exhausts(2);
    max_template_unresolved_is_rejected: edit("max: 4", "max: \"{{ cap }}\"") => Expect::RejectsWith("`loop.max` template `{{ cap }}` resolved to null");
    max_template_zero_is_rejected: edit("  max: 4\n", "  max: \"{{ cap }}\"\ncap: 0\n") => Expect::RejectsWith("resolved to 0");
    max_template_fraction_is_rejected: edit("  max: 4\n", "  max: \"{{ cap }}\"\ncap: 1.5\n") => Expect::RejectsWith("resolved to 1.5; expected a positive integer");

    max_env_unparsable_is_rejected:
        CONTROL,
        Invocation { env: &[("CLAUDINE_MAX_ITERATIONS", "ten")], ..Invocation::default() }
        => Expect::RejectsWith("invalid `CLAUDINE_MAX_ITERATIONS` value `ten`");
    max_env_zero_is_rejected:
        CONTROL,
        Invocation { env: &[("CLAUDINE_MAX_ITERATIONS", "0")], ..Invocation::default() }
        => Expect::RejectsWith("the cap must be greater than zero");
}
