//! The recovering walk ([`check_sequence_tree`]): every task is reached in
//! authored order, and each failure stays with the task or leg it belongs to
//! while its siblings are still read.

use std::fs;
use std::path::Path;

use darkmatter::markdown::compose::ContextRequirements;
use tempfile::TempDir;

use super::*;
use crate::diagnostics::Diagnostic as _;
use crate::invocation_context::InvocationContext;

fn write(dir: &Path, name: &str, text: &str) {
    fs::write(dir.join(name), text).unwrap();
}

fn check(dir: &Path, root: &str) -> SequenceCheck {
    let path = dir.join(root);
    let source = crate::composition::resolve_fixture_source(&path.display().to_string()).unwrap();
    let invocation = InvocationContext::capture_at(&crate::test_support::snapshot(), dir).unwrap();
    let source_context = invocation.derive_composition_source(&source).unwrap();
    let compose_context = invocation.capture_launch_context(&ContextRequirements::for_document(&source.markdown));
    check_sequence_tree(SequenceCheckRequest {
        source: &source,
        compose_context,
        invocation: &invocation,
        source_context: &source_context,
        caller_layers: None,
        launch_dir: Some(dir),
    })
}

/// One line per task: `identity name property file`, then `!` and the
/// failure's code for each failure (`!ref` when it carries a reference
/// finding).
fn rows(check: &SequenceCheck) -> Vec<String> {
    check
        .tasks
        .iter()
        .map(|task| {
            let file = task.site.file.file_name().unwrap().to_string_lossy().into_owned();
            let mut row = format!("{} {} {} {file}", task.site.identity, task.site.name, task.site.property);
            for failure in &task.failures {
                row.push_str(if failure.finding.is_some() { " !ref " } else { " ! " });
                row.push_str(failure.error.code());
            }
            row
        })
        .collect()
}

fn root_codes(check: &SequenceCheck) -> Vec<String> {
    check.root_failures.iter().map(|failure| failure.error.code().to_string()).collect()
}

/// Writes `files` into a fresh directory and checks its `seq.md`.
fn check_files(files: &[(&str, &str)]) -> SequenceCheck {
    let dir = TempDir::new().unwrap();
    for (name, text) in files {
        write(dir.path(), name, text);
    }
    check(dir.path(), "seq.md")
}

/// Every failure's message, task by task, then the root's.
fn messages(check: &SequenceCheck) -> Vec<String> {
    check.tasks.iter().flat_map(|task| &task.failures)
        .chain(&check.root_failures)
        .map(|failure| failure.error.to_string())
        .collect()
}

const OK: (&str, &str) = ("ok.md", "ok\n");

/// A root `seq.md` whose first task references `child` and whose second,
/// `tail`, is a control the walk must still reach.
fn referencing(child: &str) -> String {
    format!("---\nsequence:\n  - name: rs\n    sequence: ./{child}\n  - name: tail\n    prompt: ./ok.md\n---\n")
}

#[track_caller]
fn assert_rows(check: &SequenceCheck, expected: &[&str]) {
    assert_eq!(rows(check), expected, "failures: {:#?}", messages(check));
}

#[track_caller]
fn assert_message(check: &SequenceCheck, needle: &str) {
    let messages = messages(check);
    assert!(messages.iter().any(|message| message.contains(needle)), "no failure says `{needle}`: {messages:#?}");
}

#[test]
fn a_failed_second_of_three_tasks_leaves_the_third_read() {
    let check = check_files(&[OK, ("seq.md", "---\nsequence:\n  - name: one\n    prompt: ./ok.md\n  - name: two\n    prompt: ./missing.md\n  - name: three\n    prompt: ./ok.md\n---\n")]);
    assert_eq!(check.list, RootList::Read);
    assert_rows(&check, &[
        "tasks[0] one sequence[0] seq.md",
        "tasks[1] two sequence[1] seq.md !ref composition.failed",
        "tasks[2] three sequence[2] seq.md",
    ]);
    assert!(check.root_failures.is_empty());
    assert_eq!(check.tasks[2].prompt.as_deref().and_then(Path::file_name), Some("ok.md".as_ref()));
}

/// A group inside a group: the bad leg of the inner group is recorded at its
/// own identity, and its sibling, the outer group's other leg, and the next
/// root task are all reached, each container before its children.
#[test]
fn nested_groups_record_a_bad_leg_at_its_own_identity() {
    let check = check_files(&[OK, ("seq.md", "---\nsequence:\n  - name: outer\n    group:\n      tasks:\n        - name: inner\n          group:\n            tasks:\n              - name: deep-bad\n                prompt: ./gone.md\n              - name: deep-ok\n                prompt: ./ok.md\n        - name: side\n          prompt: ./ok.md\n  - name: after\n    prompt: ./ok.md\n---\n")]);
    assert_rows(&check, &[
        "tasks[0] outer sequence[0] seq.md",
        "tasks[0].group.tasks[0] inner sequence[0].group.tasks[0] seq.md",
        "tasks[0].group.tasks[0].group.tasks[0] deep-bad sequence[0].group.tasks[0].group.tasks[0] seq.md !ref composition.failed",
        "tasks[0].group.tasks[0].group.tasks[1] deep-ok sequence[0].group.tasks[0].group.tasks[1] seq.md",
        "tasks[0].group.tasks[1] side sequence[0].group.tasks[1] seq.md",
        "tasks[1] after sequence[1] seq.md",
    ]);
    let frames: Vec<&str> = check.tasks[2].site.frames.iter().map(|frame| frame.name.as_str()).collect();
    assert_eq!(frames, ["outer", "inner"]);
}

/// A selected catalog entry keeps its authored index and name in the
/// catalog; an entry the catalog does not define fails only its task.
#[test]
fn a_catalog_entry_keeps_its_authored_place() {
    let check = check_files(&[
        OK,
        ("cat.yaml", "kind: group-catalog\ngroups:\n  - name: first\n    tasks:\n      - name: c1\n        prompt: ./ok.md\n  - name: second\n    tasks:\n      - name: c2\n        prompt: ./gone.md\n      - name: c3\n        prompt: ./ok.md\n"),
        ("seq.md", "---\nsequence:\n  - name: pick\n    group: second@./cat.yaml\n  - name: nope\n    group: third@./cat.yaml\n---\n"),
    ]);
    assert_rows(&check, &[
        "tasks[0] pick sequence[0] seq.md",
        "tasks[0].group.tasks[0] c2 groups[1].tasks[0] cat.yaml !ref composition.failed",
        "tasks[0].group.tasks[1] c3 groups[1].tasks[1] cat.yaml",
        "tasks[1] nope sequence[1] seq.md ! composition.failed",
    ]);
    assert_message(&check, "group `third` is not defined in catalog");
    assert!(check.files.iter().any(|file| file.ends_with("cat.yaml")), "the catalog is judged: {:?}", check.files);
}

/// A catalog whose `groups` is not a list fails the task naming it.
#[test]
fn a_catalog_without_a_groups_list_fails_its_task() {
    let check = check_files(&[OK, ("cat.yaml", "kind: group-catalog\ngroups: 4\n"), ("seq.md", "---\nsequence:\n  - name: cg\n    group: g@./cat.yaml\n  - name: tail\n    prompt: ./ok.md\n---\n")]);
    assert_rows(&check, &["tasks[0] cg sequence[0] seq.md ! composition.failed", "tasks[1] tail sequence[1] seq.md"]);
    assert_message(&check, "must declare a `groups:` list, got number");
}

/// A sequence file that references itself is a cycle at the inner task; its
/// sibling and the root's next task are still read.
#[test]
fn a_cycle_fails_only_the_task_that_closes_it() {
    let check = check_files(&[
        OK,
        ("loop.yaml", "kind: sequence\nsequence:\n  - name: again\n    sequence: ./loop.yaml\n  - name: sib\n    prompt: ./ok.md\n"),
        ("seq.md", "---\nsequence:\n  - name: enter\n    sequence: ./loop.yaml\n  - name: after\n    prompt: ./ok.md\n---\n"),
    ]);
    assert_rows(&check, &[
        "tasks[0] enter sequence[0] seq.md",
        "tasks[0].sequence[0] again sequence[0] loop.yaml ! composition.failed",
        "tasks[0].sequence[1] sib sequence[1] loop.yaml",
        "tasks[1] after sequence[1] seq.md",
    ]);
    assert_message(&check, "sequence reference cycle");
}

/// Each reference to one file is its own occurrence, with its own failure.
#[test]
fn a_repeated_reference_is_checked_at_each_occurrence() {
    let check = check_files(&[
        OK,
        ("child.yaml", "kind: sequence\nsequence:\n  - name: c\n    prompt: ./gone.md\n"),
        ("seq.md", "---\nsequence:\n  - name: a\n    sequence: ./child.yaml\n  - name: b\n    sequence: ./child.yaml\n---\n"),
    ]);
    assert_rows(&check, &[
        "tasks[0] a sequence[0] seq.md",
        "tasks[0].sequence[0] c sequence[0] child.yaml !ref composition.failed",
        "tasks[1] b sequence[1] seq.md",
        "tasks[1].sequence[0] c sequence[0] child.yaml !ref composition.failed",
    ]);
    assert_eq!(check.files.len(), 1, "the file is read once: {:?}", check.files);
}

/// A root list from a command is known only once it runs: no task is read.
#[test]
fn a_command_backed_root_list_is_left_unread() {
    let check = check_files(&[("seq.md", "---\nsequence: \"$(echo hi)\"\n---\n")]);
    assert_eq!(check.list, RootList::Command);
    assert!(check.tasks.is_empty() && check.root_failures.is_empty(), "{:#?}", messages(&check));
}

/// A root `sequence` of the wrong whole type, or authored null, is a root
/// failure and no task is read; an empty list is a zero-task plan; no key
/// composes the body.
#[test]
fn the_root_list_shape_decides_what_is_read() {
    for (text, kind) in [("5", "number"), ("null", "null")] {
        let check = check_files(&[("seq.md", &format!("---\nsequence: {text}\n---\n"))]);
        assert_eq!(check.list, RootList::Unread, "sequence: {text}");
        assert!(check.tasks.is_empty());
        assert_eq!(root_codes(&check), ["composition.failed"]);
        assert_message(&check, &format!("expected a list, a file reference, an expression, or a shell expansion, got {kind}"));
    }
    let empty = check_files(&[("seq.md", "---\nsequence: []\n---\n")]);
    assert_eq!((empty.list, empty.tasks.len(), empty.root_failures.len()), (RootList::Read, 0, 0));
    let body = check_files(&[("seq.md", "---\nagent: claude\n---\nBody\n")]);
    assert_eq!((body.list, body.tasks.len(), body.root_failures.len()), (RootList::Body, 0, 0));
}

/// Formal root items: each invalid shape is recorded at its own position,
/// never coerced into a body step, and the valid item after them is read.
#[test]
fn invalid_root_items_keep_their_positions() {
    let check = check_files(&[OK, ("seq.md", "---\nsequence:\n  - null\n  - 17\n  - [a]\n  - {prompt: ./ok.md}\n  - name: two-exec\n    prompt: ./ok.md\n    shell: echo hi\n  - name: seq-pair\n    sequence: []\n    shell: echo\n  - name: empty-ref\n    prompt: \"\"\n  - name: null-exec\n    prompt: null\n  - name: \"\"\n    prompt: ./ok.md\n  - name: 7\n    prompt: ./ok.md\n  - name: last\n    prompt: ./ok.md\n---\n")]);
    assert_rows(&check, &[
        "tasks[0] step-1 sequence[0] seq.md ! composition.failed",
        "tasks[1] step-2 sequence[1] seq.md ! composition.failed",
        "tasks[2] step-3 sequence[2] seq.md ! composition.failed",
        "tasks[3] step-4 sequence[3] seq.md ! composition.failed",
        "tasks[4] two-exec sequence[4] seq.md ! composition.failed",
        "tasks[5] seq-pair sequence[5] seq.md ! composition.failed",
        "tasks[6] empty-ref sequence[6] seq.md ! composition.failed",
        "tasks[7] null-exec sequence[7] seq.md ! composition.failed",
        "tasks[8] step-9 sequence[8] seq.md ! composition.failed",
        "tasks[9] step-10 sequence[9] seq.md ! composition.failed",
        "tasks[10] last sequence[10] seq.md",
    ]);
    for needle in [
        "sequence item 1 is null",
        "step 2 must be a string or object, got number",
        "step 3 must be a string or object, got array",
        "step 4 is missing its required `name` property",
        "declares more than one executable field (prompt, shell)",
        "declares more than one executable field (sequence, shell)",
        "must be a non-empty file reference, got string",
        "must be a non-empty file reference, got null",
        "step 10 has a `name` of type number",
    ] {
        assert_message(&check, needle);
    }
}

/// A list of only invalid items still reports each one.
#[test]
fn an_all_invalid_root_list_reports_every_item() {
    let check = check_files(&[("seq.md", "---\nsequence:\n  - null\n  - 3\n---\n")]);
    assert_rows(&check, &["tasks[0] step-1 sequence[0] seq.md ! composition.failed", "tasks[1] step-2 sequence[1] seq.md ! composition.failed"]);
}

/// `with: {}` is an explicit empty overlay; a wrong-type `with` or `when`
/// fails at its property; a mixed or all-invalid inline group reports each
/// bad leg beside the good ones; a `tasks` that is not a list fails the group.
#[test]
fn task_and_group_fields_fail_at_their_property() {
    let check = check_files(&[OK, ("seq.md", "---\nsequence:\n  - name: w\n    prompt: ./ok.md\n    with: {}\n  - name: wbad\n    prompt: ./ok.md\n    with: 5\n  - name: whenbad\n    prompt: ./ok.md\n    when: 5\n  - name: mixed\n    group:\n      tasks:\n        - 4\n        - name: fine\n          prompt: ./ok.md\n        - name: nul\n          prompt: null\n  - name: allbad\n    group:\n      tasks: [1, null]\n  - name: notlist\n    group:\n      tasks: 3\n---\n")]);
    assert_rows(&check, &[
        "tasks[0] w sequence[0] seq.md",
        "tasks[1] wbad sequence[1] seq.md ! composition.lifecycle_invalid",
        "tasks[2] whenbad sequence[2] seq.md ! composition.failed",
        "tasks[3] mixed sequence[3] seq.md",
        "tasks[3].group.tasks[0] step-1 sequence[3].group.tasks[0] seq.md ! composition.failed",
        "tasks[3].group.tasks[1] fine sequence[3].group.tasks[1] seq.md",
        "tasks[3].group.tasks[2] nul sequence[3].group.tasks[2] seq.md ! composition.failed",
        "tasks[4] allbad sequence[4] seq.md",
        "tasks[4].group.tasks[0] step-1 sequence[4].group.tasks[0] seq.md ! composition.failed",
        "tasks[4].group.tasks[1] step-2 sequence[4].group.tasks[1] seq.md ! composition.failed",
        "tasks[5] notlist sequence[5] seq.md ! composition.failed",
    ]);
    assert!(check.tasks[0].with.is_empty());
    assert_message(&check, "`sequence[1].with` must be a mapping, got number");
    assert_message(&check, "`when` must be an expression string, got number");
    assert_message(&check, "`tasks` must be a list, got number");
}

/// A control and one invalid child at each alternate entry point: a
/// referenced group, an external task, a selected catalog entry, and a
/// referenced sequence. Each bad child is its own record; the walk reaches
/// every control and the root's last task.
#[test]
fn every_entry_point_reports_its_bad_child_beside_its_control() {
    let check = check_files(&[
        OK,
        ("grp.yaml", "kind: group\ntasks:\n  - name: g-ok\n    prompt: ./ok.md\n  - 9\n"),
        ("t.yaml", "kind: task\nprompt: ./ok.md\nwith: 3\n"),
        ("cat.yaml", "kind: group-catalog\ngroups:\n  - name: g\n    tasks:\n      - name: k-ok\n        prompt: ./ok.md\n      - null\n"),
        ("child.yaml", "kind: sequence\nsequence:\n  - name: s-ok\n    prompt: ./ok.md\n  - 8\n"),
        ("seq.md", "---\nsequence:\n  - name: rg\n    group: ./grp.yaml\n  - name: tf\n    task: ./t.yaml\n  - name: cg\n    group: g@./cat.yaml\n  - name: rs\n    sequence: ./child.yaml\n  - name: tail\n    prompt: ./ok.md\n---\n"),
    ]);
    assert_rows(&check, &[
        "tasks[0] rg sequence[0] seq.md",
        "tasks[0].group.tasks[0] g-ok tasks[0] grp.yaml",
        "tasks[0].group.tasks[1] step-2 tasks[1] grp.yaml ! composition.failed",
        "tasks[1] tf sequence[1] seq.md ! composition.lifecycle_invalid",
        "tasks[2] cg sequence[2] seq.md",
        "tasks[2].group.tasks[0] k-ok groups[0].tasks[0] cat.yaml",
        "tasks[2].group.tasks[1] step-2 groups[0].tasks[1] cat.yaml ! composition.failed",
        "tasks[3] rs sequence[3] seq.md",
        "tasks[3].sequence[0] s-ok sequence[0] child.yaml",
        "tasks[3].sequence[1] step-2 sequence[1] child.yaml ! composition.failed",
        "tasks[4] tail sequence[4] seq.md",
    ]);
    let files: Vec<_> = check.files.iter().map(|file| file.file_name().unwrap().to_string_lossy().into_owned()).collect();
    assert_eq!(files, ["grp.yaml", "t.yaml", "cat.yaml", "child.yaml"]);
}

/// Lenient foreign-data rows: a bare name, a number, and a boolean are
/// steps named by their scalar; a nameless object takes its one-based
/// ordinal; a null row is still invalid, alone.
#[test]
fn lenient_rows_keep_their_projection_and_refuse_only_null() {
    let check = check_files(&[OK, ("rows.yaml", "- just-a-name\n- 5\n- true\n- {prompt: ./ok.md}\n- null\n"), ("seq.md", &referencing("rows.yaml"))]);
    assert_rows(&check, &[
        "tasks[0] rs sequence[0] seq.md",
        "tasks[0].sequence[0] just-a-name sequence[0] rows.yaml",
        "tasks[0].sequence[1] 5 sequence[1] rows.yaml",
        "tasks[0].sequence[2] true sequence[2] rows.yaml",
        "tasks[0].sequence[3] 4 sequence[3] rows.yaml",
        "tasks[0].sequence[4] step-5 sequence[4] rows.yaml ! composition.failed",
        "tasks[1] tail sequence[1] seq.md",
    ]);
    assert_message(&check, "sequence item 5 is null");
}

/// Every loader: a file that was read and failed to parse fails only the
/// task referencing it, with the loader's located cause; a file that does
/// not exist is a reference finding instead.
#[test]
fn a_document_that_fails_to_parse_fails_only_its_referencing_task() {
    let cases: [(&str, &str, &str); 9] = [
        ("child.yaml", "kind: sequence\nsequence:\n  - name: a\n    name: b\n    prompt: ./ok.md\n", "duplicate entry with key \"name\" at line 3 column 5"),
        ("child.yaml", "kind: sequence\nsequence:\n  - name: a\n    prompt: ./ok.md\n]]]\n", "did not find expected key at line 5 column 1"),
        ("child.yaml", "kind: sequence\nsequence: []\n---\nkind: sequence\n", "more than one document"),
        ("rows.json", "[{\"name\": \"a\", \"name\": \"b\", \"prompt\": \"./ok.md\"}]\n", "duplicate key `name` at `[0]`"),
        ("rows.json5", "[{name: 'a'}] xx\n", "line 1 column 15"),
        // The blank line does not shift the later line's number.
        ("rows.jsonl", "{\"name\": \"a\", \"prompt\": \"./ok.md\"}\n\n{\"name\": \"b\", \"name\": \"c\"}\n", "line 3: duplicate key `name`"),
        ("rows.ndjson", "{\"name\": \"a\"}\n{oops\n", "line 2: key must be a string"),
        ("child.md", "---\nkind: sequence\nsequence:\n  - name: a\n    name: b\n---\n", "duplicate entry with key \"name\""),
        ("child.yml", "kind: sequence\nsequence: [\n", "YAML parse error"),
    ];
    for (name, text, cause) in cases {
        let check = check_files(&[OK, (name, text), ("seq.md", &referencing(name))]);
        assert_rows(&check, &["tasks[0] rs sequence[0] seq.md ! composition.failed", "tasks[1] tail sequence[1] seq.md"]);
        assert_message(&check, cause);
        assert!(check.files.iter().any(|file| file.ends_with(name)), "{name} was read");
    }
    let missing = check_files(&[OK, ("seq.md", &referencing("nope.yaml"))]);
    assert_rows(&missing, &["tasks[0] rs sequence[0] seq.md !ref composition.failed", "tasks[1] tail sequence[1] seq.md"]);
    assert!(missing.files.is_empty(), "nothing was read: {:?}", missing.files);
}

/// The shared JSON5 path admits comments and trailing commas, and an
/// explicit YAML null at a referenced root is refused like an inline one.
#[test]
fn loader_controls_read_their_admitted_syntax() {
    let check = check_files(&[OK, ("rows.json5", "// rows\n[{name: 'a', prompt: './ok.md'},]\n"), ("seq.md", &referencing("rows.json5"))]);
    assert_rows(&check, &["tasks[0] rs sequence[0] seq.md", "tasks[0].sequence[0] a sequence[0] rows.json5", "tasks[1] tail sequence[1] seq.md"]);
    let null = check_files(&[OK, ("child.yaml", "kind: sequence\nsequence: null\n"), ("seq.md", &referencing("child.yaml"))]);
    assert_eq!(null.tasks.len(), 2, "{:#?}", rows(&null));
    assert!(!null.tasks[0].failures.is_empty(), "a null referenced list is invalid: {:#?}", rows(&null));
    assert!(null.tasks[1].failures.is_empty());
}

/// The root's own refusals do not wait for its list: a removed read in the
/// body is reported even when the list comes from a command, and once when
/// the list is read.
#[test]
fn root_refusals_are_made_before_the_list_is_read() {
    let command = check_files(&[("seq.md", "---\nsequence: '$(must-not-run)'\n---\n{{ outputs }}\n")]);
    assert_eq!(command.list, RootList::Command);
    assert_eq!(root_codes(&command), ["composition.lifecycle_invalid"], "{:#?}", messages(&command));
    assert_message(&command, "seq.outputs");

    let read = check_files(&[OK, ("seq.md", "---\nsequence:\n  - name: one\n    prompt: ./ok.md\n---\n{{ outputs }}\n")]);
    assert_eq!(root_codes(&read), ["composition.lifecycle_invalid"], "reported once: {:#?}", messages(&read));
    assert_rows(&read, &["tasks[0] one sequence[0] seq.md"]);
}

/// A referenced prompt whose single-pass lifecycle literal nests a span is
/// refused at each task that composes it, as a real run's graph check does;
/// the task beside it is unaffected.
#[test]
fn a_prompts_nested_lifecycle_literal_is_refused_at_its_task() {
    let check = check_files(&[
        OK,
        ("bad.md", "---\nsuccess:\n    say: \"{{ ok ? 'done in {{area}}' : 'failed' }}\"\n---\nBody\n"),
        ("seq.md", "---\nsequence:\n  - name: fine\n    prompt: ./ok.md\n  - name: nested\n    prompt: ./bad.md\n---\n"),
    ]);
    assert_rows(&check, &["tasks[0] fine sequence[0] seq.md", "tasks[1] nested sequence[1] seq.md ! composition.lifecycle_invalid"]);
    assert_message(&check, "nests `{{area}}` inside the string literal");
    assert!(check.tasks[1].prompt.is_some(), "the prompt was read");
}
