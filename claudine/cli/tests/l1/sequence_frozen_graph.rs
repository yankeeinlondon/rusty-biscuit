//! A container's discovered children stay frozen for the invocation: an
//! earlier task that adds a child to a container's source, using a command the
//! invocation already approved, never changes which children run or their order.

use std::fs;

use crate::common::{CliProcessFixture, strip_ansi};

/// Writes each `(name, contents)` file into a private workspace, runs the
/// `root` document as a sequence, and returns its stdout and exit code.
fn run_sequence(root: &str, files: &[(&str, &str)]) -> (String, String, i32) {
    let fixture = CliProcessFixture::named("sequence-frozen-graph");
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
