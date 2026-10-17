//! Input robustness matrix for the sequence and group task grammar: container
//! `loop:` (on a sequence document's root, a group object, and an inline
//! nested sequence's task entry), task `when:`, misplaced task `loop`, and
//! JSON-family duplicate keys, one traversal per task-source format.
//!
//! Every cell is one textual edit of its format's control fixture, so the
//! control row proves each edit is load-bearing. The controls need no
//! provider: each task is a `side_effect:` appending a line to a trace file,
//! so a cell's effect count is the number of trace lines afterwards. Cells
//! assert the public result only: the exit status, that count, and a stderr
//! message naming the file that holds the edit.
//!
//! A valid container `loop:` and a valid task `when:` are refused before
//! anything runs: their run-time behavior is planned, and running the body
//! once or ignoring the guard would be wrong. The contract table is the
//! feature's `phase-1/matrix-contracts.md`.

use std::fs;

use crate::common::{CliProcessFixture, strip_ansi};

/// Every file a control's side effects append to.
const TRACES: &[&str] = &["first.trace", "leg.trace", "inner.trace", "second.trace"];

/// The observable result of running one cell.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Expect {
    /// Exit 0 after this many side effects.
    Runs(usize),
    /// A non-zero exit before any side effect; stderr names `file` and says
    /// `says`.
    Rejects { file: &'static str, says: &'static str },
    /// A non-zero exit after this many side effects, saying `says`.
    FailsAfter { effects: usize, says: &'static str },
}

/// The files of one format's control: the document `claudine sequence` runs
/// and every file it reads.
struct Control {
    root: &'static str,
    files: &'static [(&'static str, &'static str)],
}

impl Control {
    fn unedited(&self) -> Vec<(&'static str, String)> {
        self.files.iter().map(|(name, text)| (*name, (*text).to_string())).collect()
    }

    /// The control with one edit to `file`; the edited text must be present
    /// so the cell changes what it claims to.
    fn edit(&self, file: &str, from: &str, to: &str) -> Vec<(&'static str, String)> {
        let mut files = self.unedited();
        let (_, text) = files.iter_mut().find(|(name, _)| *name == file).expect("the control has the file");
        assert!(text.contains(from), "edit target {from:?} is not in {file}");
        *text = text.replacen(from, to, 1);
        files
    }
}

/// Text with the error block's border and all whitespace removed, so a
/// phrase matches wherever the block wrapped it, including after a hyphen.
fn unwrapped(text: &str) -> String {
    text.replace('┃', "").split_whitespace().collect()
}

fn check(control: &Control, files: Vec<(&'static str, String)>, expected: Expect) {
    let fixture = CliProcessFixture::named("sequence-grammar-matrix");
    for (name, text) in &files {
        fs::write(fixture.cwd().join(name), text).unwrap();
    }
    let output = fixture
        .command()
        // Not `--silent`: a run-time failure's message is a progress line.
        .args(["sequence", "--goose", "--yolo"])
        .arg(fixture.cwd().join(control.root))
        .output()
        .unwrap();
    let effects: usize = TRACES
        .iter()
        .filter_map(|trace| fs::read_to_string(fixture.cwd().join(trace)).ok())
        .map(|text| text.lines().count())
        .sum();
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    let flat = unwrapped(&stderr);
    match expected {
        Expect::Runs(count) => {
            assert!(output.status.success(), "status {:?}; stderr:\n{stderr}", output.status);
            assert_eq!(effects, count, "stderr:\n{stderr}");
        }
        Expect::Rejects { file, says } => {
            assert!(!output.status.success(), "the cell must fail; stderr:\n{stderr}");
            assert_eq!(effects, 0, "a rejection runs nothing; stderr:\n{stderr}");
            assert!(flat.contains(&unwrapped(file)), "a rejection names {file}:\n{stderr}");
            assert!(flat.contains(&unwrapped(says)), "the rejection says {says:?}:\n{stderr}");
        }
        Expect::FailsAfter { effects: count, says } => {
            assert!(!output.status.success(), "the cell must fail; stderr:\n{stderr}");
            assert_eq!(effects, count, "stderr:\n{stderr}");
            assert!(flat.contains(&unwrapped(says)), "the failure says {says:?}:\n{stderr}");
        }
    }
    assert!(!fixture.audio_spool().exists());
}

macro_rules! matrix {
    ($control:expr; $($name:ident: $files:expr => $expected:expr;)*) => {
        $(
            #[test]
            fn $name() {
                check(&$control, $files, $expected);
            }
        )*
    };
}

/// What a valid container loop or task guard is refused with.
const NOT_YET: &str = "is valid but is not yet supported at run time";
/// The replacement a removed `loop.fail_fast` names.
const FAIL_FAST: &str = "a loop has no `fail_fast`";
/// Why a container loop refuses `on_rate_limit`.
const RATE_LIMIT: &str = "applies only to a document loop";
/// A whole `loop:` of the wrong type.
const LOOP_NOT_OBJECT: &str = "must be an object with `until` or `while`";
/// An empty `loop: {}`.
const LOOP_EMPTY: &str = "must define either `while` or `until`";
/// A guard that is not a string.
const WHEN_NOT_STRING: &str = "`when` must be an expression string";
/// An empty guard.
const WHEN_EMPTY: &str = "`when` must not be an empty expression";
/// A guard that does not parse.
const WHEN_UNPARSABLE: &str = "`when` is not a valid expression";
/// `when:` at a sequence document's root.
const ROOT_WHEN: &str = "a sequence document's body is not a step";
/// `when:` on a group object.
const GROUP_WHEN: &str = "`when` is a task key; put it beside `group:`";
/// The hints a misplaced task `loop` names.
const LOOP_ON_LEAF: &str = "shell and side-effect tasks repeat only inside a looping container";
const LOOP_BESIDE_GROUP: &str = "a group's loop goes on its group object";
const LOOP_ON_GROUP_REF: &str = "put `loop:` in that group file's object";
const LOOP_ON_SEQUENCE_REF: &str = "put `loop:` at that document's root";
const LOOP_ON_PROMPT: &str = "a prompt task repeats through its document's own frontmatter `loop:`";
/// A group leg that is not an object.
const LEG_NOT_OBJECT: &str = "task 1 must be an object";
/// A group's `start: [break]`, refused when it runs.
const BREAK_REFUSED: &str = "is not yet supported at run time";
/// The inline nested sequence a JSON-family nested-loop cell writes.
macro_rules! nested_inner {
    () => {
        "\"sequence\": [{\"name\": \"inner\", \"side_effect\": {\"append_line\": [\"inner.trace\", \"ran\"]}}]"
    };
}

/// YAML frontmatter: a Markdown sequence document with an inline group and an
/// inline nested sequence.
mod yaml_frontmatter {
    use super::*;

    const SEQ: &str = "---\nsequence:\n  - name: first\n    side_effect: { append_line: [first.trace, ran] }\n  - name: fan\n    group:\n      name: g\n      tasks:\n        - name: leg\n          side_effect: { append_line: [leg.trace, ran] }\n  - name: nested\n    sequence:\n      - name: inner\n        side_effect: { append_line: [inner.trace, ran] }\n---\n";
    const CONTROL: Control = Control { root: "seq.md", files: &[("seq.md", SEQ)] };

    fn edit(from: &str, to: &str) -> Vec<(&'static str, String)> {
        CONTROL.edit("seq.md", from, to)
    }

    const ROOT: &str = "---\nsequence:\n";
    const GROUP: &str = "      name: g\n";
    const FIRST: &str = "  - name: first\n";
    const NESTED: &str = "  - name: nested\n";
    const LEG: &str = "        - name: leg\n";

    matrix! { CONTROL;
        control_runs_every_task: CONTROL.unedited() => Expect::Runs(3);

        root_loop_is_refused: edit(ROOT, "---\nloop:\n  until: \"true\"\nsequence:\n") => Expect::Rejects { file: "seq.md", says: "the sequence `loop` at `loop` is valid but is not yet supported at run time" };
        root_loop_null_is_rejected: edit(ROOT, "---\nloop: null\nsequence:\n") => Expect::Rejects { file: "seq.md", says: LOOP_NOT_OBJECT };
        root_loop_scalar_is_rejected: edit(ROOT, "---\nloop: 5\nsequence:\n") => Expect::Rejects { file: "seq.md", says: LOOP_NOT_OBJECT };
        root_loop_empty_is_rejected: edit(ROOT, "---\nloop: {}\nsequence:\n") => Expect::Rejects { file: "seq.md", says: LOOP_EMPTY };
        root_loop_unknown_key_is_rejected: edit(ROOT, "---\nloop:\n  untill: \"true\"\nsequence:\n") => Expect::Rejects { file: "seq.md", says: "unknown `loop.untill` key (did you mean `until`?)" };
        root_loop_fail_fast_is_removed: edit(ROOT, "---\nloop:\n  until: \"true\"\n  fail_fast: false\nsequence:\n") => Expect::Rejects { file: "seq.md", says: FAIL_FAST };
        root_loop_on_rate_limit_is_rejected: edit(ROOT, "---\nloop:\n  until: \"true\"\n  on_rate_limit: pause\nsequence:\n") => Expect::Rejects { file: "seq.md", says: RATE_LIMIT };
        root_loop_duplicate_key_is_rejected: edit(ROOT, "---\nloop:\n  until: \"true\"\n  until: \"false\"\nsequence:\n") => Expect::Rejects { file: "seq.md", says: "duplicate" };
        root_when_is_rejected: edit(ROOT, "---\nwhen: \"true\"\nsequence:\n") => Expect::Rejects { file: "seq.md", says: ROOT_WHEN };
        root_when_null_is_rejected: edit(ROOT, "---\nwhen: null\nsequence:\n") => Expect::Rejects { file: "seq.md", says: ROOT_WHEN };

        group_loop_is_refused: edit(GROUP, "      name: g\n      loop:\n        until: \"true\"\n") => Expect::Rejects { file: "seq.md", says: "the group `loop` at `sequence[1].group.loop` is valid but is not yet supported at run time" };
        group_loop_null_is_rejected: edit(GROUP, "      name: g\n      loop: null\n") => Expect::Rejects { file: "seq.md", says: LOOP_NOT_OBJECT };
        group_loop_list_is_rejected: edit(GROUP, "      name: g\n      loop: [until]\n") => Expect::Rejects { file: "seq.md", says: LOOP_NOT_OBJECT };
        group_loop_empty_is_rejected: edit(GROUP, "      name: g\n      loop: {}\n") => Expect::Rejects { file: "seq.md", says: LOOP_EMPTY };
        group_loop_every_member_wrong_is_rejected: edit(GROUP, "      name: g\n      loop: { until: 5, action: 7, max: true }\n") => Expect::Rejects { file: "seq.md", says: "sequence[1].group.loop" };
        group_loop_unknown_key_is_rejected: edit(GROUP, "      name: g\n      loop: { until: \"true\", colour: red }\n") => Expect::Rejects { file: "seq.md", says: "unknown `sequence[1].group.loop.colour` key" };
        group_loop_fail_fast_is_removed: edit(GROUP, "      name: g\n      loop: { until: \"true\", fail_fast: true }\n") => Expect::Rejects { file: "seq.md", says: FAIL_FAST };
        group_loop_on_rate_limit_is_rejected: edit(GROUP, "      name: g\n      loop: { until: \"true\", on_rate_limit: pause }\n") => Expect::Rejects { file: "seq.md", says: RATE_LIMIT };
        group_when_is_rejected: edit(GROUP, "      name: g\n      when: \"true\"\n") => Expect::Rejects { file: "seq.md", says: GROUP_WHEN };
        group_leg_not_an_object_is_rejected: edit("        - name: leg\n          side_effect: { append_line: [leg.trace, ran] }\n", "        - leg\n") => Expect::Rejects { file: "seq.md", says: LEG_NOT_OBJECT };
        group_start_break_is_refused_when_it_runs: edit(GROUP, "      name: g\n      start: [break]\n") => Expect::FailsAfter { effects: 1, says: BREAK_REFUSED };

        leg_when_is_refused: edit(LEG, "        - name: leg\n          when: \"true\"\n") => Expect::Rejects { file: "seq.md", says: "task `when` at `sequence[1].group.tasks[0].when` is valid but is not yet supported at run time" };
        leg_when_null_is_rejected: edit(LEG, "        - name: leg\n          when: null\n") => Expect::Rejects { file: "seq.md", says: WHEN_NOT_STRING };

        step_when_is_refused: edit(FIRST, "  - name: first\n    when: \"true\"\n") => Expect::Rejects { file: "seq.md", says: "task `when` at `sequence[0].when` is valid but is not yet supported at run time" };
        step_when_false_is_refused: edit(FIRST, "  - name: first\n    when: \"false\"\n") => Expect::Rejects { file: "seq.md", says: NOT_YET };
        step_when_null_is_rejected: edit(FIRST, "  - name: first\n    when: null\n") => Expect::Rejects { file: "seq.md", says: WHEN_NOT_STRING };
        step_when_boolean_is_rejected: edit(FIRST, "  - name: first\n    when: false\n") => Expect::Rejects { file: "seq.md", says: WHEN_NOT_STRING };
        step_when_number_is_rejected: edit(FIRST, "  - name: first\n    when: 1\n") => Expect::Rejects { file: "seq.md", says: WHEN_NOT_STRING };
        step_when_list_is_rejected: edit(FIRST, "  - name: first\n    when: [\"true\"]\n") => Expect::Rejects { file: "seq.md", says: WHEN_NOT_STRING };
        step_when_map_is_rejected: edit(FIRST, "  - name: first\n    when: { a: 1 }\n") => Expect::Rejects { file: "seq.md", says: WHEN_NOT_STRING };
        step_when_empty_is_rejected: edit(FIRST, "  - name: first\n    when: \"\"\n") => Expect::Rejects { file: "seq.md", says: WHEN_EMPTY };
        step_when_unparsable_is_rejected: edit(FIRST, "  - name: first\n    when: \"n >=\"\n") => Expect::Rejects { file: "seq.md", says: WHEN_UNPARSABLE };
        step_when_duplicate_key_is_rejected: edit(FIRST, "  - name: first\n    when: \"true\"\n    when: \"false\"\n") => Expect::Rejects { file: "seq.md", says: "duplicate" };

        loop_on_a_side_effect_task_is_misplaced: edit(FIRST, "  - name: first\n    loop: { until: \"true\" }\n") => Expect::Rejects { file: "seq.md", says: LOOP_ON_LEAF };
        loop_beside_an_inline_group_is_misplaced: edit("  - name: fan\n", "  - name: fan\n    loop: { until: \"true\" }\n") => Expect::Rejects { file: "seq.md", says: LOOP_BESIDE_GROUP };
        loop_on_a_prompt_task_is_misplaced: edit("    side_effect: { append_line: [first.trace, ran] }\n", "    prompt: p.md\n    loop: { until: \"true\" }\n") => Expect::Rejects { file: "seq.md", says: LOOP_ON_PROMPT };
        loop_beside_a_group_reference_is_misplaced: edit("    group:\n      name: g\n      tasks:\n        - name: leg\n          side_effect: { append_line: [leg.trace, ran] }\n", "    group: g.yaml\n    loop: { until: \"true\" }\n") => Expect::Rejects { file: "seq.md", says: LOOP_ON_GROUP_REF };
        loop_beside_a_sequence_reference_is_misplaced: edit("    sequence:\n      - name: inner\n        side_effect: { append_line: [inner.trace, ran] }\n", "    sequence: other.yaml\n    loop: { until: \"true\" }\n") => Expect::Rejects { file: "seq.md", says: LOOP_ON_SEQUENCE_REF };

        nested_loop_is_refused: edit(NESTED, "  - name: nested\n    loop:\n      until: \"true\"\n") => Expect::Rejects { file: "seq.md", says: "the sequence `loop` at `sequence[2].loop` is valid but is not yet supported at run time" };
        nested_loop_null_is_rejected: edit(NESTED, "  - name: nested\n    loop: null\n") => Expect::Rejects { file: "seq.md", says: LOOP_NOT_OBJECT };
        nested_loop_empty_is_rejected: edit(NESTED, "  - name: nested\n    loop: {}\n") => Expect::Rejects { file: "seq.md", says: LOOP_EMPTY };
        nested_loop_unknown_key_is_rejected: edit(NESTED, "  - name: nested\n    loop: { untill: \"true\" }\n") => Expect::Rejects { file: "seq.md", says: "unknown `sequence[2].loop.untill` key (did you mean `until`?)" };
        nested_loop_fail_fast_is_removed: edit(NESTED, "  - name: nested\n    loop: { until: \"true\", fail_fast: null }\n") => Expect::Rejects { file: "seq.md", says: FAIL_FAST };
        nested_loop_on_rate_limit_is_rejected: edit(NESTED, "  - name: nested\n    loop: { until: \"true\", on_rate_limit: abort }\n") => Expect::Rejects { file: "seq.md", says: RATE_LIMIT };

        invalid_frontmatter_is_rejected: edit("      tasks:\n", "      tasks:\n  : [\n") => Expect::Rejects { file: "seq.md", says: "seq.md" };
    }
}

/// Standalone YAML: `claudine sequence steps.yaml`, with a `kind: group` file
/// referenced from a step.
mod standalone_yaml {
    use super::*;

    const STEPS: &str = "sequence:\n  - name: first\n    side_effect: { append_line: [first.trace, ran] }\n  - name: fan\n    group: g.yaml\n  - name: nested\n    sequence:\n      - name: inner\n        side_effect: { append_line: [inner.trace, ran] }\n";
    const GROUP_FILE: &str = "kind: group\nname: g\ntasks:\n  - name: leg\n    side_effect: { append_line: [leg.trace, ran] }\n";
    const CONTROL: Control = Control { root: "steps.yaml", files: &[("steps.yaml", STEPS), ("g.yaml", GROUP_FILE)] };

    fn steps(from: &str, to: &str) -> Vec<(&'static str, String)> {
        CONTROL.edit("steps.yaml", from, to)
    }

    fn group(from: &str, to: &str) -> Vec<(&'static str, String)> {
        CONTROL.edit("g.yaml", from, to)
    }

    const FIRST: &str = "  - name: first\n";
    const NESTED: &str = "  - name: nested\n";
    const NAME: &str = "name: g\n";

    matrix! { CONTROL;
        control_runs_every_task: CONTROL.unedited() => Expect::Runs(3);

        root_loop_is_refused: steps("sequence:\n", "loop:\n  until: \"true\"\nsequence:\n") => Expect::Rejects { file: "steps.yaml", says: "the sequence `loop` at `loop` is valid but is not yet supported at run time" };
        root_loop_null_is_rejected: steps("sequence:\n", "loop: null\nsequence:\n") => Expect::Rejects { file: "steps.yaml", says: LOOP_NOT_OBJECT };
        root_loop_empty_is_rejected: steps("sequence:\n", "loop: {}\nsequence:\n") => Expect::Rejects { file: "steps.yaml", says: LOOP_EMPTY };
        root_loop_fail_fast_is_removed: steps("sequence:\n", "loop: { until: \"true\", fail_fast: true }\nsequence:\n") => Expect::Rejects { file: "steps.yaml", says: FAIL_FAST };
        root_loop_on_rate_limit_is_rejected: steps("sequence:\n", "loop: { until: \"true\", on_rate_limit: continue }\nsequence:\n") => Expect::Rejects { file: "steps.yaml", says: RATE_LIMIT };
        root_when_is_rejected: steps("sequence:\n", "when: \"true\"\nsequence:\n") => Expect::Rejects { file: "steps.yaml", says: ROOT_WHEN };
        root_duplicate_key_is_rejected: steps("sequence:\n", "loop: { until: \"true\" }\nloop: { until: \"false\" }\nsequence:\n") => Expect::Rejects { file: "steps.yaml", says: "duplicate" };
        root_second_document_is_rejected: steps("        side_effect: { append_line: [inner.trace, ran] }\n", "        side_effect: { append_line: [inner.trace, ran] }\n---\nx: 1\n") => Expect::Rejects { file: "steps.yaml", says: "more than one document" };

        group_file_loop_is_refused: group(NAME, "name: g\nloop:\n  until: \"true\"\n") => Expect::Rejects { file: "g.yaml", says: "the group `loop` at `loop` is valid but is not yet supported at run time" };
        group_file_loop_null_is_rejected: group(NAME, "name: g\nloop: null\n") => Expect::Rejects { file: "g.yaml", says: LOOP_NOT_OBJECT };
        group_file_loop_empty_is_rejected: group(NAME, "name: g\nloop: {}\n") => Expect::Rejects { file: "g.yaml", says: LOOP_EMPTY };
        group_file_loop_unknown_key_is_rejected: group(NAME, "name: g\nloop: { untill: \"true\" }\n") => Expect::Rejects { file: "g.yaml", says: "unknown `loop.untill` key (did you mean `until`?)" };
        group_file_loop_fail_fast_is_removed: group(NAME, "name: g\nloop: { until: \"true\", fail_fast: false }\n") => Expect::Rejects { file: "g.yaml", says: FAIL_FAST };
        group_file_loop_on_rate_limit_is_rejected: group(NAME, "name: g\nloop: { until: \"true\", on_rate_limit: pause }\n") => Expect::Rejects { file: "g.yaml", says: RATE_LIMIT };
        group_file_loop_duplicate_key_is_rejected: group(NAME, "name: g\nloop:\n  until: \"true\"\n  until: \"false\"\n") => Expect::Rejects { file: "g.yaml", says: "duplicate" };
        group_file_when_is_rejected: group(NAME, "name: g\nwhen: \"true\"\n") => Expect::Rejects { file: "g.yaml", says: GROUP_WHEN };
        group_file_leg_not_an_object_is_rejected: group("  - name: leg\n    side_effect: { append_line: [leg.trace, ran] }\n", "  - leg\n") => Expect::Rejects { file: "g.yaml", says: LEG_NOT_OBJECT };
        group_file_second_document_is_rejected: group("    side_effect: { append_line: [leg.trace, ran] }\n", "    side_effect: { append_line: [leg.trace, ran] }\n---\nx: 1\n") => Expect::Rejects { file: "g.yaml", says: "more than one document" };
        group_file_start_break_is_refused_when_it_runs: group(NAME, "name: g\nstart: [break]\n") => Expect::FailsAfter { effects: 1, says: BREAK_REFUSED };

        leg_when_is_refused: group("  - name: leg\n", "  - name: leg\n    when: \"true\"\n") => Expect::Rejects { file: "g.yaml", says: "task `when` at `tasks[0].when` is valid but is not yet supported at run time" };
        leg_when_null_is_rejected: group("  - name: leg\n", "  - name: leg\n    when: null\n") => Expect::Rejects { file: "g.yaml", says: WHEN_NOT_STRING };

        step_when_is_refused: steps(FIRST, "  - name: first\n    when: \"true\"\n") => Expect::Rejects { file: "steps.yaml", says: "task `when` at `sequence[0].when` is valid but is not yet supported at run time" };
        step_when_null_is_rejected: steps(FIRST, "  - name: first\n    when: null\n") => Expect::Rejects { file: "steps.yaml", says: WHEN_NOT_STRING };
        step_when_list_is_rejected: steps(FIRST, "  - name: first\n    when: [\"true\"]\n") => Expect::Rejects { file: "steps.yaml", says: WHEN_NOT_STRING };
        step_when_empty_is_rejected: steps(FIRST, "  - name: first\n    when: \"\"\n") => Expect::Rejects { file: "steps.yaml", says: WHEN_EMPTY };
        step_when_unparsable_is_rejected: steps(FIRST, "  - name: first\n    when: \"(\"\n") => Expect::Rejects { file: "steps.yaml", says: WHEN_UNPARSABLE };

        loop_on_a_side_effect_task_is_misplaced: steps(FIRST, "  - name: first\n    loop: { until: \"true\" }\n") => Expect::Rejects { file: "steps.yaml", says: LOOP_ON_LEAF };
        loop_beside_a_group_reference_is_misplaced: steps("    group: g.yaml\n", "    group: g.yaml\n    loop: { until: \"true\" }\n") => Expect::Rejects { file: "steps.yaml", says: LOOP_ON_GROUP_REF };

        nested_loop_is_refused: steps(NESTED, "  - name: nested\n    loop:\n      until: \"true\"\n") => Expect::Rejects { file: "steps.yaml", says: "the sequence `loop` at `sequence[2].loop` is valid but is not yet supported at run time" };
        nested_loop_empty_is_rejected: steps(NESTED, "  - name: nested\n    loop: {}\n") => Expect::Rejects { file: "steps.yaml", says: LOOP_EMPTY };
        nested_loop_null_is_rejected: steps(NESTED, "  - name: nested\n    loop: null\n") => Expect::Rejects { file: "steps.yaml", says: LOOP_NOT_OBJECT };
    }
}

/// JSON and JSON5 share one traversal shape: a Markdown root reaches a formal
/// sequence document (`sequence: tasks.<ext>`), whose second step references
/// a `kind: group` file in the same format.
///
/// The formal document carries no inline nested sequence in its control: a
/// root-referenced formal document's nested container is reread against the
/// invoking Markdown file, which does not hold it. The nested-loop cells edit
/// a step into one instead, and are refused before that reread.
macro_rules! json_family {
    ($module:ident, $doc:literal, $tasks_name:literal, $group_name:literal, $tasks:expr, $group:expr,
     first: $first:literal, first_step: $first_step:literal, fan: $fan:literal, fan_step: $fan_step:literal,
     root: $root:literal, group_name_key: $gname:literal,
     leg: $leg:literal, leg_body: $leg_body:literal, end: $end:literal $(, extra: { $($extra:tt)* })?) => {
        mod $module {
            use super::*;

            const CONTROL: Control = Control {
                root: "seq.md",
                files: &[("seq.md", $doc), ($tasks_name, $tasks), ($group_name, $group)],
            };

            fn tasks(from: &str, to: &str) -> Vec<(&'static str, String)> {
                CONTROL.edit($tasks_name, from, to)
            }

            fn group(from: &str, to: &str) -> Vec<(&'static str, String)> {
                CONTROL.edit($group_name, from, to)
            }

            /// Insert `key_value` as the first member after `anchor`.
            fn after(anchor: &str, key_value: &str) -> String {
                format!("{anchor} {key_value},")
            }

            matrix! { CONTROL;
                control_runs_every_task: CONTROL.unedited() => Expect::Runs(2);

                root_loop_is_refused: tasks($root, &after($root, "\"loop\": {\"until\": \"true\"}")) => Expect::Rejects { file: $tasks_name, says: "the sequence `loop` at `loop` is valid but is not yet supported at run time" };
                root_loop_null_is_rejected: tasks($root, &after($root, "\"loop\": null")) => Expect::Rejects { file: $tasks_name, says: LOOP_NOT_OBJECT };
                root_loop_scalar_is_rejected: tasks($root, &after($root, "\"loop\": 5")) => Expect::Rejects { file: $tasks_name, says: LOOP_NOT_OBJECT };
                root_loop_empty_is_rejected: tasks($root, &after($root, "\"loop\": {}")) => Expect::Rejects { file: $tasks_name, says: LOOP_EMPTY };
                root_loop_unknown_key_is_rejected: tasks($root, &after($root, "\"loop\": {\"untill\": \"true\"}")) => Expect::Rejects { file: $tasks_name, says: "unknown `loop.untill` key (did you mean `until`?)" };
                root_loop_fail_fast_is_removed: tasks($root, &after($root, "\"loop\": {\"until\": \"true\", \"fail_fast\": true}")) => Expect::Rejects { file: $tasks_name, says: FAIL_FAST };
                root_loop_on_rate_limit_is_rejected: tasks($root, &after($root, "\"loop\": {\"until\": \"true\", \"on_rate_limit\": \"pause\"}")) => Expect::Rejects { file: $tasks_name, says: RATE_LIMIT };
                root_loop_duplicate_key_is_rejected: tasks($root, &after($root, "\"loop\": {\"until\": \"true\", \"until\": \"false\"}")) => Expect::Rejects { file: $tasks_name, says: "duplicate key `until` at `loop`" };
                root_when_is_rejected: tasks($root, &after($root, "\"when\": \"true\"")) => Expect::Rejects { file: $tasks_name, says: ROOT_WHEN };
                root_trailing_content_is_rejected: tasks($end, concat!($end, "{}")) => Expect::Rejects { file: $tasks_name, says: $tasks_name };

                step_when_is_refused: tasks($first, &after($first, "\"when\": \"true\"")) => Expect::Rejects { file: $tasks_name, says: "task `when` at `sequence[0].when` is valid but is not yet supported at run time" };
                step_when_null_is_rejected: tasks($first, &after($first, "\"when\": null")) => Expect::Rejects { file: $tasks_name, says: WHEN_NOT_STRING };
                step_when_number_is_rejected: tasks($first, &after($first, "\"when\": 0")) => Expect::Rejects { file: $tasks_name, says: WHEN_NOT_STRING };
                step_when_list_is_rejected: tasks($first, &after($first, "\"when\": [\"true\"]")) => Expect::Rejects { file: $tasks_name, says: WHEN_NOT_STRING };
                step_when_empty_is_rejected: tasks($first, &after($first, "\"when\": \"\"")) => Expect::Rejects { file: $tasks_name, says: WHEN_EMPTY };
                step_when_unparsable_is_rejected: tasks($first, &after($first, "\"when\": \"n >=\"")) => Expect::Rejects { file: $tasks_name, says: WHEN_UNPARSABLE };
                step_duplicate_key_is_rejected: tasks($first, &after($first, "\"name\": \"again\"")) => Expect::Rejects { file: $tasks_name, says: "duplicate key `name` at `sequence[0]`" };

                loop_on_a_side_effect_task_is_misplaced: tasks($first, &after($first, "\"loop\": {\"until\": \"true\"}")) => Expect::Rejects { file: $tasks_name, says: LOOP_ON_LEAF };
                loop_beside_a_group_reference_is_misplaced: tasks($fan, &after($fan, "\"loop\": {\"until\": \"true\"}")) => Expect::Rejects { file: $tasks_name, says: LOOP_ON_GROUP_REF };
                loop_beside_a_sequence_reference_is_misplaced: tasks($fan_step, "{\"name\": \"fan\", \"sequence\": \"other.json\", \"loop\": {\"until\": \"true\"}}") => Expect::Rejects { file: $tasks_name, says: LOOP_ON_SEQUENCE_REF };
                nested_loop_is_refused: tasks($first_step, concat!("{\"name\": \"first\", \"loop\": {\"until\": \"true\"}, ", nested_inner!(), "}")) => Expect::Rejects { file: $tasks_name, says: "the sequence `loop` at `sequence[0].loop` is valid but is not yet supported at run time" };
                nested_loop_empty_is_rejected: tasks($first_step, concat!("{\"name\": \"first\", \"loop\": {}, ", nested_inner!(), "}")) => Expect::Rejects { file: $tasks_name, says: LOOP_EMPTY };
                nested_loop_on_rate_limit_is_rejected: tasks($first_step, concat!("{\"name\": \"first\", \"loop\": {\"until\": \"true\", \"on_rate_limit\": \"pause\"}, ", nested_inner!(), "}")) => Expect::Rejects { file: $tasks_name, says: RATE_LIMIT };

                group_file_loop_is_refused: group($gname, &after($gname, "\"loop\": {\"until\": \"true\"}")) => Expect::Rejects { file: $group_name, says: "the group `loop` at `loop` is valid but is not yet supported at run time" };
                group_file_loop_null_is_rejected: group($gname, &after($gname, "\"loop\": null")) => Expect::Rejects { file: $group_name, says: LOOP_NOT_OBJECT };
                group_file_loop_empty_is_rejected: group($gname, &after($gname, "\"loop\": {}")) => Expect::Rejects { file: $group_name, says: LOOP_EMPTY };
                group_file_loop_unknown_key_is_rejected: group($gname, &after($gname, "\"loop\": {\"until\": \"true\", \"colour\": \"red\"}")) => Expect::Rejects { file: $group_name, says: "unknown `loop.colour` key" };
                group_file_loop_fail_fast_is_removed: group($gname, &after($gname, "\"loop\": {\"until\": \"true\", \"fail_fast\": null}")) => Expect::Rejects { file: $group_name, says: FAIL_FAST };
                group_file_loop_on_rate_limit_is_rejected: group($gname, &after($gname, "\"loop\": {\"until\": \"true\", \"on_rate_limit\": \"abort\"}")) => Expect::Rejects { file: $group_name, says: RATE_LIMIT };
                group_file_loop_duplicate_key_is_rejected: group($gname, &after($gname, "\"loop\": {\"until\": \"true\", \"max\": 2, \"max\": 3}")) => Expect::Rejects { file: $group_name, says: "duplicate key `max` at `loop`" };
                group_file_when_is_rejected: group($gname, &after($gname, "\"when\": \"true\"")) => Expect::Rejects { file: $group_name, says: GROUP_WHEN };
                group_file_leg_not_an_object_is_rejected: group($leg_body, "\"leg\"") => Expect::Rejects { file: $group_name, says: LEG_NOT_OBJECT };
                group_file_start_break_is_refused_when_it_runs: group($gname, &after($gname, "\"start\": [\"break\"]")) => Expect::FailsAfter { effects: 1, says: BREAK_REFUSED };

                leg_when_is_refused: group($leg, &after($leg, "\"when\": \"true\"")) => Expect::Rejects { file: $group_name, says: "task `when` at `tasks[0].when` is valid but is not yet supported at run time" };
                leg_when_map_is_rejected: group($leg, &after($leg, "\"when\": {\"a\": 1}")) => Expect::Rejects { file: $group_name, says: WHEN_NOT_STRING };
                leg_duplicate_key_is_rejected: group($leg, &after($leg, "\"name\": \"leg\"")) => Expect::Rejects { file: $group_name, says: "duplicate key `name` at `tasks[0]`" };

                $($($extra)*)?
            }
        }
    };
}

json_family!(json, "---\nsequence: tasks.json\n---\n", "tasks.json", "g.json",
    "{\n  \"sequence\": [\n    {\"name\": \"first\", \"side_effect\": {\"append_line\": [\"first.trace\", \"ran\"]}},\n    {\"name\": \"fan\", \"group\": \"g.json\"}\n  ]\n}\n",
    "{\"kind\": \"group\", \"name\": \"g\", \"tasks\": [{\"name\": \"leg\", \"side_effect\": {\"append_line\": [\"leg.trace\", \"ran\"]}}]}\n",
    first: "{\"name\": \"first\",", first_step: "{\"name\": \"first\", \"side_effect\": {\"append_line\": [\"first.trace\", \"ran\"]}}",
    fan: "{\"name\": \"fan\",", fan_step: "{\"name\": \"fan\", \"group\": \"g.json\"}",
    root: "{\n", group_name_key: "\"name\": \"g\",",
    leg: "{\"name\": \"leg\",", leg_body: "{\"name\": \"leg\", \"side_effect\": {\"append_line\": [\"leg.trace\", \"ran\"]}}", end: "  ]\n}");

json_family!(json5, "---\nsequence: tasks.json5\n---\n", "tasks.json5", "g.json5",
    "// The task source: comments are content, not trailing data.\n{\n  sequence: [\n    {\"name\": \"first\", side_effect: {append_line: ['first.trace', 'ran']}},\n    {\"name\": \"fan\", group: 'g.json5'}, // trailing comma\n  ],\n}\n",
    "{kind: 'group', \"name\": \"g\", tasks: [{\"name\": \"leg\", side_effect: {append_line: ['leg.trace', 'ran']}}]}\n",
    first: "{\"name\": \"first\",", first_step: "{\"name\": \"first\", side_effect: {append_line: ['first.trace', 'ran']}}",
    fan: "{\"name\": \"fan\",", fan_step: "{\"name\": \"fan\", group: 'g.json5'}",
    root: "{\n", group_name_key: "\"name\": \"g\",",
    leg: "{\"name\": \"leg\",", leg_body: "{\"name\": \"leg\", side_effect: {append_line: ['leg.trace', 'ran']}}", end: "  ],\n}",
    extra: {
        block_comment_inside_a_task_still_runs: tasks("{\"name\": \"first\",", "{\"name\": \"first\", /* still content */") => Expect::Runs(2);
    });

/// JSONL and NDJSON reach a task list only through an operator
/// (`tasks.jsonl::name(label)`), so they carry no sequence-document root, no
/// `kind:` files, and no offsets (an offset is its own typed error); those
/// cells are N/A. Edits touch record 1 only, and record 2 stays a valid
/// record in every cell.
macro_rules! line_delimited {
    ($module:ident, $file:literal) => {
        mod $module {
            use super::*;

            const RECORD_1: &str = "{\"label\": \"fan\", \"group\": {\"name\": \"g\", \"tasks\": [{\"name\": \"leg\", \"side_effect\": {\"append_line\": [\"leg.trace\", \"ran\"]}}]}}";
            const RECORDS: &str = concat!(
                "{\"label\": \"fan\", \"group\": {\"name\": \"g\", \"tasks\": [{\"name\": \"leg\", \"side_effect\": {\"append_line\": [\"leg.trace\", \"ran\"]}}]}}\n",
                "{\"label\": \"second\", \"side_effect\": {\"append_line\": [\"second.trace\", \"ran\"]}}\n",
            );
            const CONTROL: Control = Control {
                root: "seq.md",
                files: &[("seq.md", concat!("---\nsequence: \"", $file, "::name(label)\"\n---\n")), ($file, RECORDS)],
            };

            fn record_1(from: &str, to: &str) -> Vec<(&'static str, String)> {
                assert!(RECORD_1.contains(from), "edit target {from:?} is not in record 1");
                CONTROL.edit($file, from, to)
            }

            const TASK: &str = "{\"label\": \"fan\",";
            const GROUP: &str = "{\"name\": \"g\",";
            const LEG: &str = "{\"name\": \"leg\",";

            matrix! { CONTROL;
                control_runs_both_records: CONTROL.unedited() => Expect::Runs(2);

                task_when_is_refused: record_1(TASK, "{\"label\": \"fan\", \"when\": \"true\",") => Expect::Rejects { file: $file, says: "task `when` at `[0].when` is valid but is not yet supported at run time" };
                task_when_null_is_rejected: record_1(TASK, "{\"label\": \"fan\", \"when\": null,") => Expect::Rejects { file: $file, says: WHEN_NOT_STRING };
                task_when_boolean_is_rejected: record_1(TASK, "{\"label\": \"fan\", \"when\": true,") => Expect::Rejects { file: $file, says: WHEN_NOT_STRING };
                task_when_empty_is_rejected: record_1(TASK, "{\"label\": \"fan\", \"when\": \"\",") => Expect::Rejects { file: $file, says: WHEN_EMPTY };
                task_when_unparsable_is_rejected: record_1(TASK, "{\"label\": \"fan\", \"when\": \"n >=\",") => Expect::Rejects { file: $file, says: WHEN_UNPARSABLE };
                task_duplicate_key_is_rejected: record_1(TASK, "{\"label\": \"fan\", \"label\": \"again\",") => Expect::Rejects { file: $file, says: "line 1: duplicate key `label` at `<root>`" };
                task_trailing_content_is_rejected: record_1("]}}", "]}} junk") => Expect::Rejects { file: $file, says: "line 1" };
                loop_beside_an_inline_group_is_misplaced: record_1(TASK, "{\"label\": \"fan\", \"loop\": {\"until\": \"true\"},") => Expect::Rejects { file: $file, says: LOOP_BESIDE_GROUP };

                group_loop_is_refused: record_1(GROUP, "{\"name\": \"g\", \"loop\": {\"until\": \"true\"},") => Expect::Rejects { file: $file, says: "the group `loop` at `[0].group.loop` is valid but is not yet supported at run time" };
                group_loop_null_is_rejected: record_1(GROUP, "{\"name\": \"g\", \"loop\": null,") => Expect::Rejects { file: $file, says: LOOP_NOT_OBJECT };
                group_loop_list_is_rejected: record_1(GROUP, "{\"name\": \"g\", \"loop\": [],") => Expect::Rejects { file: $file, says: LOOP_NOT_OBJECT };
                group_loop_empty_is_rejected: record_1(GROUP, "{\"name\": \"g\", \"loop\": {},") => Expect::Rejects { file: $file, says: LOOP_EMPTY };
                group_loop_unknown_key_is_rejected: record_1(GROUP, "{\"name\": \"g\", \"loop\": {\"untill\": \"true\"},") => Expect::Rejects { file: $file, says: "unknown `[0].group.loop.untill` key (did you mean `until`?)" };
                group_loop_fail_fast_is_removed: record_1(GROUP, "{\"name\": \"g\", \"loop\": {\"until\": \"true\", \"fail_fast\": false},") => Expect::Rejects { file: $file, says: FAIL_FAST };
                group_loop_on_rate_limit_is_rejected: record_1(GROUP, "{\"name\": \"g\", \"loop\": {\"until\": \"true\", \"on_rate_limit\": \"pause\"},") => Expect::Rejects { file: $file, says: RATE_LIMIT };
                group_loop_duplicate_key_is_rejected: record_1(GROUP, "{\"name\": \"g\", \"loop\": {\"while\": \"true\", \"while\": \"false\"},") => Expect::Rejects { file: $file, says: "line 1: duplicate key `while` at `group.loop`" };
                group_when_is_rejected: record_1(GROUP, "{\"name\": \"g\", \"when\": \"true\",") => Expect::Rejects { file: $file, says: GROUP_WHEN };
                group_leg_not_an_object_is_rejected: record_1("{\"name\": \"leg\", \"side_effect\": {\"append_line\": [\"leg.trace\", \"ran\"]}}", "\"leg\"") => Expect::Rejects { file: $file, says: LEG_NOT_OBJECT };

                leg_when_is_refused: record_1(LEG, "{\"name\": \"leg\", \"when\": \"true\",") => Expect::Rejects { file: $file, says: "task `when` at `[0].group.tasks[0].when` is valid but is not yet supported at run time" };
                leg_when_list_is_rejected: record_1(LEG, "{\"name\": \"leg\", \"when\": [],") => Expect::Rejects { file: $file, says: WHEN_NOT_STRING };

                nested_loop_is_refused: record_1(RECORD_1, "{\"label\": \"fan\", \"loop\": {\"until\": \"true\"}, \"sequence\": [{\"name\": \"inner\", \"side_effect\": {\"append_line\": [\"inner.trace\", \"ran\"]}}]}") => Expect::Rejects { file: $file, says: "the sequence `loop` at `[0].loop` is valid but is not yet supported at run time" };
                nested_loop_empty_is_rejected: record_1(RECORD_1, "{\"label\": \"fan\", \"loop\": {}, \"sequence\": [{\"name\": \"inner\", \"side_effect\": {\"append_line\": [\"inner.trace\", \"ran\"]}}]}") => Expect::Rejects { file: $file, says: LOOP_EMPTY };
            }
        }
    };
}

line_delimited!(jsonl, "tasks.jsonl");
line_delimited!(ndjson, "tasks.ndjson");
