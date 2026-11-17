//! A container's discovered children stay frozen for the invocation: an
//! earlier task that adds a child to a container's source, using a command the
//! invocation already approved, never changes which children run or their order.
//! Nor does a `$state` added to a source on disk, or a value a template read
//! changing across a loop pass or retry: rows keep their planned ids and
//! template-derived state, and are never typed again.

use std::fs;

use crate::common::{CliProcessFixture, strip_ansi};

/// Writes each `(name, contents)` file into a private workspace, runs the
/// `root` document as a sequence, and returns its stdout and exit code.
fn run_sequence(root: &str, files: &[(&str, &str)]) -> (String, String, i32) {
    let fixture = CliProcessFixture::named("sequence-frozen-graph");
    // Rows run a templated `echo`, which Windows has no program for.
    #[cfg(feature = "test-fixtures")]
    fixture.install_echo();
    for (name, contents) in files {
        fs::write(fixture.cwd().join(name), contents).unwrap();
    }
    let output = fixture.command().args(["sequence", "--goose", "--yolo", "--silent"])
        .arg(fixture.cwd().join(root)).assert().get_output().clone();
    assert!(!fixture.audio_spool().exists());
    (strip_ansi(&String::from_utf8_lossy(&output.stdout)),
     strip_ansi(&String::from_utf8_lossy(&output.stderr)), output.status.code().unwrap_or(-1))
}

/// Replaces the path an `append_line` step reports as its output with
/// `<rewrite>`, so expectations do not depend on the private workspace path.
///
/// The path is redacted in the text, before JSON parsing: the `stdout` action
/// prints `\\` as `\`, so a Windows path reaches stdout with bare backslashes.
fn redact_rewrites(stdout: &str) -> String {
    let mut redacted = String::with_capacity(stdout.len());
    let mut rest = stdout;
    while let Some(open) = rest.find('"') {
        let Some(len) = rest[open + 1..].find('"') else { break };
        let quoted = &rest[open + 1..open + 1 + len];
        redacted.push_str(&rest[..open]);
        if quoted.ends_with(".yaml") {
            redacted.push_str("\"<rewrite>\"");
        } else {
            redacted.push_str(&rest[open..open + len + 2]);
        }
        rest = &rest[open + len + 2..];
    }
    redacted.push_str(rest);
    let value: serde_json::Value = serde_json::from_str(&redacted)
        .unwrap_or_else(|error| panic!("stdout is not JSON ({error}): {stdout}"));
    value.to_string()
}

macro_rules! frozen_graph {
    ($name:ident, root: $root:literal, files: [$(($file:literal, $contents:expr)),* $(,)?], expected: $expected:literal) => {
        #[test]
        fn $name() {
            let (stdout, stderr, code) = run_sequence($root, &[$(($file, $contents)),*]);
            assert_eq!(code, 0, "{stderr}\n{stdout}");
            assert_eq!(redact_rewrites(stdout.trim()), $expected, "{stderr}");
        }
    };
}

/// A root that rewrites `$target` by appending `$line`, then runs `$step`.
macro_rules! rewriting_root {
    ($target:literal, $line:literal, $step:literal) => {
        concat!("---\nsuccess: [{ stdout: \"{{ seq.outputs }}\" }]\nsequence:\n",
            "  - name: rewrite\n    side_effect: { append_line: [", $target, ", '", $line, "'] }\n",
            "  - ", $step, "\n---\n")
    };
}

frozen_graph!(root_sequence_keeps_its_steps_when_its_own_file_changes,
    root: "root.yaml",
    files: [("root.yaml", "kind: sequence\nsuccess: [{ stdout: \"{{ seq.outputs }}\" }]\nsequence:\n  - name: rewrite\n    side_effect: { append_line: [root.yaml, '  - { name: added, shell: echo child }'] }\n  - { name: original, shell: echo child }\n")],
    expected: r#"["<rewrite>","child"]"#);

frozen_graph!(referenced_nested_sequence_keeps_its_discovered_children,
    root: "root.md",
    files: [
        ("root.md", rewriting_root!("child.yaml", "  - { name: added, shell: echo child }", "{ name: nested, sequence: child.yaml }")),
        ("child.yaml", "kind: sequence\nsequence:\n  - { name: original, shell: echo child }\n"),
    ],
    expected: r#"["<rewrite>",["child"]]"#);

frozen_graph!(inline_nested_sequence_keeps_its_discovered_children,
    root: "root.md",
    files: [
        ("root.md", rewriting_root!("outer.yaml", "      - { name: added, shell: echo child }", "{ name: outer, sequence: outer.yaml }")),
        ("outer.yaml", "kind: sequence\nsequence:\n  - name: nested\n    sequence:\n      - { name: original, shell: echo child }\n"),
    ],
    expected: r#"["<rewrite>",[["child"]]]"#);

frozen_graph!(inline_group_keeps_its_discovered_legs,
    root: "root.md",
    files: [
        ("root.md", rewriting_root!("outer.yaml", "        - { name: added, shell: echo child }", "{ name: outer, sequence: outer.yaml }")),
        ("outer.yaml", "kind: sequence\nsequence:\n  - name: checks\n    group:\n      name: checks\n      tasks:\n        - { name: original, shell: echo child }\n"),
    ],
    expected: r#"["<rewrite>",[{"original":"child"}]]"#);

frozen_graph!(referenced_group_keeps_its_discovered_legs,
    root: "root.md",
    files: [
        ("root.md", rewriting_root!("group.yaml", "  - { name: added, shell: echo child }", "{ name: checks, group: group.yaml }")),
        ("group.yaml", "kind: group\nname: checks\ntasks:\n  - { name: original, shell: echo child }\n"),
    ],
    expected: r#"["<rewrite>",{"original":"child"}]"#);

frozen_graph!(catalog_group_keeps_its_discovered_legs,
    root: "root.md",
    files: [
        ("root.md", rewriting_root!("catalog.yaml", "      - { name: added, shell: echo child }", "{ name: checks, group: checks@catalog.yaml }")),
        ("catalog.yaml", "kind: group-catalog\ngroups:\n  - name: checks\n    tasks:\n      - { name: original, shell: echo child }\n"),
    ],
    expected: r#"["<rewrite>",{"original":"child"}]"#);

frozen_graph!(external_task_sequence_keeps_its_discovered_children,
    root: "root.md",
    files: [
        ("root.md", rewriting_root!("child.yaml", "  - { name: added, shell: echo child }", "{ name: nested, task: task.yaml }")),
        ("task.yaml", "kind: task\nsequence: child.yaml\n"),
        ("child.yaml", "kind: sequence\nsequence:\n  - { name: original, shell: echo child }\n"),
    ],
    expected: r#"["<rewrite>",["child"]]"#);

frozen_graph!(external_task_group_keeps_its_discovered_legs,
    root: "root.md",
    files: [
        ("root.md", rewriting_root!("group.yaml", "  - { name: added, shell: echo child }", "{ name: checks, task: task.yaml }")),
        ("task.yaml", "kind: task\ngroup: group.yaml\n"),
        ("group.yaml", "kind: group\nname: checks\ntasks:\n  - { name: original, shell: echo child }\n"),
    ],
    expected: r#"["<rewrite>",{"original":"child"}]"#);

frozen_graph!(nested_sequence_retry_runs_its_discovered_children,
    root: "root.md",
    files: [
        ("root.md", "---\nready: false\nsuccess: [{ stdout: \"{{ seq.outputs }}\" }]\nsequence:\n  - { name: nested, sequence: child.yaml }\n---\n"),
        ("child.yaml", "kind: sequence\nfailure: [{ retry: { max_attempts: 1, with: { ready: true } } }]\nsequence:\n  - { name: original, shell: echo child }\n  - name: rewrite\n    side_effect: { append_line: [child.yaml, '  - { name: added, shell: echo child }'] }\n  - name: gate\n    sequence: []\n    start: [{ when: \"!ready\", then: [{ error: not-ready }] }]\n"),
    ],
    expected: r#"[["child","<rewrite>",[]]]"#);

frozen_graph!(root_retry_runs_the_discovered_children_of_a_changed_reference,
    root: "root.md",
    files: [
        ("root.md", "---\nready: false\nfailure: [{ retry: { max_attempts: 1, with: { ready: true } } }]\nsuccess: [{ stdout: \"{{ seq.outputs }}\" }]\nsequence:\n  - { name: nested, sequence: child.yaml }\n  - name: rewrite\n    side_effect: { append_line: [child.yaml, '  - { name: added, shell: echo child }'] }\n  - name: gate\n    sequence: []\n    start: [{ when: \"!ready\", then: [{ error: not-ready }] }]\n---\n"),
        ("child.yaml", "kind: sequence\nsequence:\n  - { name: original, shell: echo child }\n"),
    ],
    expected: r#"[["child"],"<rewrite>",[]]"#);

frozen_graph!(group_retry_runs_its_discovered_legs,
    root: "root.md",
    files: [
        ("root.md", "---\nready: false\nsuccess: [{ stdout: \"{{ seq.outputs }}\" }]\nsequence:\n  - { name: checks, group: group.yaml }\n---\n"),
        ("group.yaml", "kind: group\nname: checks\nfailure: [{ retry: { max_attempts: 1, with: { ready: true } } }]\ntasks:\n  - name: gate\n    sequence:\n      - { name: original, shell: echo child }\n      - name: rewrite\n        side_effect: { append_line: [group.yaml, '  - { name: added, shell: echo child }'] }\n      - name: check\n        sequence: []\n        start: [{ when: \"!ready\", then: [{ error: not-ready }] }]\n"),
    ],
    expected: r#"[{"gate":["child","<rewrite>",[]]}]"#);

// ============================================================================
// Declarations and document inputs across rereads, passes, and retries
// ============================================================================

// A `$state` added to a referenced document's root never types the rows its
// reread keeps: they lack `color`, and still run.
frozen_graph!(a_reread_root_state_declaration_never_retypes_a_referenced_documents_rows,
    root: "root.md",
    files: [
        ("root.md", rewriting_root!("child.yaml", "$state: { color: string(required) }", "{ name: nested, sequence: child.yaml }")),
        ("child.yaml", "kind: sequence\nsequence:\n  - { name: a, shell: \"echo {{ seq.who }}\" }\ntemplate:\n  who: original\n"),
    ],
    expected: r#"["<rewrite>",["original"]]"#);

frozen_graph!(a_reread_state_declaration_never_retypes_the_roots_inline_container,
    root: "root.yaml",
    files: [("root.yaml", "kind: sequence\nsuccess: [{ stdout: \"{{ seq.outputs }}\" }]\nsequence:\n  - name: rewrite\n    side_effect: { append_line: [root.yaml, '    $state: { color: string(required) }'] }\n  - name: outer\n    sequence:\n      - { name: a, shell: \"echo {{ seq.who }}\" }\n    template:\n      who: original\n")],
    expected: r#"["<rewrite>",["original"]]"#);

// Regression: a nested reread read only its own rows as authored, so an
// inline container one level down was normalized again under its current
// `$state` and its frozen rows failed state validation.
frozen_graph!(a_reread_state_declaration_never_retypes_an_inline_container_inside_a_referenced_document,
    root: "root.md",
    files: [
        ("root.md", rewriting_root!("outer.yaml", "    $state: { color: string(required) }", "{ name: nested, sequence: outer.yaml }")),
        ("outer.yaml", "kind: sequence\nsequence:\n  - name: outer\n    sequence:\n      - { name: a, shell: \"echo {{ seq.who }}\" }\n    template:\n      who: original\n"),
    ],
    expected: r#"["<rewrite>",[["original"]]]"#);

// Regression: as above, through a group's reread of its changed file.
frozen_graph!(a_reread_state_declaration_never_retypes_a_group_legs_inline_container,
    root: "root.md",
    files: [
        ("root.md", rewriting_root!("group.yaml", "    $state: { color: string(required) }", "{ name: checks, group: group.yaml }")),
        ("group.yaml", "kind: group\nname: checks\ntasks:\n  - name: leg\n    sequence:\n      - { name: a, shell: \"echo {{ seq.who }}\" }\n    template:\n      who: original\n"),
    ],
    expected: r#"["<rewrite>",{"leg":["original"]}]"#);

// A root retry after a `$state` was added to a referenced document runs the
// first attempt's graph: its rows, which lack `color`, are not typed again.
frozen_graph!(a_root_retry_after_a_state_declaration_edit_runs_the_untyped_rows,
    root: "root.md",
    files: [
        ("root.md", "---\nready: false\nfailure: [{ retry: { max_attempts: 1, with: { ready: true } } }]\nsuccess: [{ stdout: \"{{ seq.outputs }}\" }]\nsequence:\n  - { name: nested, sequence: child.yaml }\n  - name: rewrite\n    side_effect: { append_line: [child.yaml, '$state: { color: string(required) }'] }\n  - name: gate\n    sequence: []\n    start: [{ when: \"!ready\", then: [{ error: not-ready }] }]\n---\n"),
        ("child.yaml", "kind: sequence\nsequence:\n  - { name: a, shell: \"echo {{ seq.who }}\" }\ntemplate:\n  who: original\n"),
    ],
    expected: r#"[["original"],"<rewrite>",[]]"#);

// A template is filled when the graph is built: `mark`'s write is read
// directly on pass 2 but never refills `who`.
frozen_graph!(a_template_keeps_its_planned_value_across_loop_passes,
    root: "root.md",
    files: [("root.md", "---\nstamp: planned\nsuccess: [{ stdout: \"{{ seq.outputs }}\" }]\nsequence:\n  - name: inner\n    loop: { until: \"loop.count >= 2\" }\n    template:\n      who: \"{{ stamp }}\"\n    sequence:\n      - { name: a, shell: \"echo {{ seq.id }}:{{ seq.who }}-{{ stamp }}\" }\n      - { name: mark, side_effect: { set: { stamp: rewritten } } }\n---\n")],
    expected: r#"[["a:planned-planned","{\"stamp\":\"planned\"}","a:planned-rewritten","{\"stamp\":\"rewritten\"}"]]"#);

// A retry's `with:` changes the referenced document's input `reviewer`:
// the retried row reads the new value directly, while its id and the
// template-derived `who` keep what was planned.
frozen_graph!(a_retry_that_changes_a_document_input_keeps_the_planned_ids_and_template_state,
    root: "root.md",
    files: [
        ("root.md", "---\nready: false\nsuccess: [{ stdout: \"{{ seq.outputs }}\" }]\nsequence:\n  - { name: nested, sequence: child.yaml }\n---\n"),
        ("child.yaml", "kind: sequence\n$schema:\n  reviewer: string(required)\nreviewer: own\ntemplate:\n  who: \"{{ reviewer }}\"\nfailure: [{ retry: { max_attempts: 1, with: { reviewer: changed, ready: true } } }]\nsequence:\n  - { name: a, shell: \"echo {{ seq.id }}:{{ seq.who }}/{{ reviewer }}\" }\n  - name: gate\n    sequence: []\n    start: [{ when: \"!ready\", then: [{ error: not-ready }] }]\n"),
    ],
    expected: r#"[["a:own/changed",[]]]"#);

/// A root whose first container fails its inline `$schema` check, and whose
/// `failure` prints the refusal, runs the `edit` actions, and then retries
/// with the missing value.
fn failed_admission_root(edit: &str) -> String {
    format!("---\nfailure:\n  - stdout: \"refused: {{{{ err.msg }}}}\"\n{edit}  - retry: {{ max_attempts: 1, with: {{ reviewer: ada }} }}\nsuccess: [{{ stdout: \"{{{{ seq.outputs }}}}\" }}]\nsequence:\n  - name: checked\n    $schema: {{ reviewer: string(required) }}\n    sequence: [{{ name: sign, shell: echo signed }}]\n  - {{ name: nested, sequence: child.yaml }}\n---\n")
}

const ADMISSION_CHILD: &str = "kind: sequence\nsequence:\n  - { name: original, shell: echo child }\n";

/// The control: the refused container is retried and the frozen graph runs.
#[test]
fn a_retry_after_a_failed_inline_check_runs_the_frozen_graph() {
    let (stdout, stderr, code) = run_sequence("root.md", &[
        ("root.md", &failed_admission_root("")), ("child.yaml", ADMISSION_CHILD),
    ]);
    assert_eq!(code, 0, "{stderr}\n{stdout}");
    let lines: Vec<&str> = stdout.lines().map(str::trim).filter(|line| !line.is_empty()).collect();
    assert_eq!(lines.len(), 2, "one refusal, then the retried run's outputs: {stdout}");
    assert!(lines[0].starts_with("refused:") && lines[0].contains("failed its `$schema` check at `reviewer`"), "{stdout}");
    assert_eq!(lines[1], r#"[["signed"],["child"]]"#, "{stderr}");
}

/// A failed admission is not an approval boundary: a command its `failure`
/// writes into a referenced source before retrying is refused, `--yolo`
/// notwithstanding, and never runs.
#[test]
fn a_failed_inline_check_never_reopens_command_approval() {
    let (stdout, stderr, code) = run_sequence("root.md", &[
        ("root.md", &failed_admission_root("  - append_line: [child.yaml, '  - { name: added, shell: echo forbidden }']\n")),
        ("child.yaml", ADMISSION_CHILD),
    ]);
    assert_ne!(code, 0, "{stderr}\n{stdout}");
    let flat = stderr.replace('\u{2503}', " ").split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(stdout.contains("refused:") && stdout.contains("failed its `$schema` check at `reviewer`"), "{stdout}");
    assert!(flat.contains("shell command `echo forbidden`"), "{flat}");
    assert!(flat.contains("introduced an unapproved command after sequence approval closed"), "{flat}");
    assert!(!stdout.lines().any(|line| line.trim() == "forbidden"), "{stdout}");
}

/// The check an earlier step tightens: appending a line to the file that
/// declares it adds a required property under the container's `$schema`,
/// which is authored last, so the container is refused at its turn. The
/// root's `failure` sees whether the container's `initialize` wrote.
const TIGHTENED: &str = "reviewer: string(required)";

/// A root that appends `TIGHTENED`, indented by `indent`, to `target`, and
/// then runs `rest` (the rest of its `sequence:` list).
fn tightening_root(target: &str, indent: usize, rest: &str) -> String {
    format!(
        "kind: sequence\ninitialized: false\nworked: false\nfailure:\n  - stdout: \"handled initialized={{{{ initialized }}}} worked={{{{ worked }}}}: {{{{ err.msg }}}}\"\nsuccess:\n  - stdout: \"done worked={{{{ worked }}}}\"\nsequence:\n  - name: tighten\n    side_effect: {{ append_line: [{target}, '{}{TIGHTENED}'] }}\n{rest}",
        " ".repeat(indent),
    )
}

/// An inline container, indented by `indent`, whose `$schema` is its last key.
fn checked_container(indent: usize) -> String {
    let pad = " ".repeat(indent);
    format!("{pad}- name: inner\n{pad}  initialize:\n{pad}    - stdout: INIT-RAN\n{pad}    - set: {{ initialized: true }}\n{pad}  sequence:\n{pad}    - {{ name: work, side_effect: {{ set: {{ worked: true }} }} }}\n{pad}  $schema:\n{pad}    rank: number\n")
}

/// The refused container printed nothing, neither its `initialize` nor its
/// child wrote, and the root's `failure` handled the refusal.
fn assert_refused_before_initialize(stdout: &str, stderr: &str, code: i32) {
    assert_ne!(code, 0, "{stderr}\n{stdout}");
    assert!(!stdout.contains("INIT-RAN"), "{stdout}");
    assert!(stdout.contains("handled initialized=false worked=false"), "{stdout}\n{stderr}");
    assert!(stdout.contains("failed its `$schema` check at `reviewer`"), "{stdout}");
}

#[test]
fn a_check_tightened_by_an_earlier_step_refuses_the_container_before_initialize() {
    let root = tightening_root("root.yaml", 6, &checked_container(2));
    let (stdout, stderr, code) = run_sequence("root.yaml", &[("root.yaml", &root)]);
    assert_refused_before_initialize(&stdout, &stderr, code);
}

#[test]
fn a_tightened_check_on_a_group_leg_refuses_the_leg_before_initialize() {
    let root = tightening_root("root.yaml", 12,
        &format!("  - name: legs\n    group:\n      name: legs\n      tasks:\n{}", checked_container(8)));
    let (stdout, stderr, code) = run_sequence("root.yaml", &[("root.yaml", &root)]);
    assert_refused_before_initialize(&stdout, &stderr, code);
}

#[test]
fn a_tightened_check_in_a_task_file_refuses_the_container_before_initialize() {
    let root = tightening_root("task.yaml", 2, "  - { name: inner, task: task.yaml }\n");
    let task = "kind: task\nname: inner\ninitialize:\n  - stdout: INIT-RAN\n  - set: { initialized: true }\nsequence:\n  - { name: work, side_effect: { set: { worked: true } } }\n$schema:\n  rank: number\n";
    let (stdout, stderr, code) = run_sequence("root.yaml", &[("root.yaml", &root), ("task.yaml", task)]);
    assert_refused_before_initialize(&stdout, &stderr, code);
}

#[test]
fn a_tightened_check_in_a_referenced_document_refuses_its_child_before_initialize() {
    let root = tightening_root("child.yaml", 6, "  - { name: nested, sequence: child.yaml }\n");
    let child = format!("kind: sequence\nsequence:\n{}", checked_container(2));
    let (stdout, stderr, code) = run_sequence("root.yaml", &[("root.yaml", &root), ("child.yaml", &child)]);
    assert_refused_before_initialize(&stdout, &stderr, code);
}

/// The control: an unedited check admits the container, whose events and
/// child run.
#[test]
fn an_unedited_check_admits_the_container() {
    let root = tightening_root("unrelated.yaml", 0, &checked_container(2));
    let (stdout, stderr, code) = run_sequence("root.yaml", &[("root.yaml", &root), ("unrelated.yaml", "")]);
    assert_eq!(code, 0, "{stderr}\n{stdout}");
    assert!(stdout.contains("INIT-RAN") && stdout.contains("done worked=true"), "{stdout}");
}
