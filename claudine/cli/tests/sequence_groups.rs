//! End-to-end coverage for serial group execution.
//!
//! Sequence Plus phase 9 (`claudine/features/2026-07-11-sequence-plus/`).
//!
//! A group is only observable end to end through its members' side effects:
//! which shell commands ran, in which order, whether the ones after a failure
//! ran at all, and whether the sequence continued past the failed group. Every
//! test here drives the real `claudine sequence` invocation path.

#![cfg(unix)]

use assert_cmd::cargo::cargo_bin_cmd;
use std::fs;
use std::path::Path;
use tempfile::tempdir;
mod common;
use common::{augmented_path, strip_ansi, write_executable};

/// Install a fake `goose` that succeeds and echoes a fixed line.
fn fake_goose(workspace: &Path) -> std::path::PathBuf {
    let path_dir = workspace.join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    write_executable(
        &path_dir.join("goose"),
        "#!/bin/sh\nprintf 'agent-said\\n'\nexit 0\n",
    );
    path_dir
}

fn run(workspace: &Path, path_dir: &Path, args: &[&str]) -> (String, String, i32) {
    let output = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace)
        .env("PATH", augmented_path(path_dir))
        .current_dir(workspace)
        .args(args)
        .assert()
        .get_output()
        .clone();
    (
        strip_ansi(&String::from_utf8_lossy(&output.stdout)),
        strip_ansi(&String::from_utf8_lossy(&output.stderr)),
        output.status.code().unwrap_or(-1),
    )
}

/// Read the trace file group tasks append to, one line per command.
fn trace(workspace: &Path) -> Vec<String> {
    fs::read_to_string(workspace.join("trace.txt"))
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect()
}

/// An inline serial group runs its tasks in declaration order and reports each
/// one by name in the step breakdown.
#[test]
fn an_inline_serial_group_runs_its_tasks_in_declaration_order() {
    let workspace = tempdir().unwrap();
    let path_dir = fake_goose(workspace.path());
    let md = workspace.path().join("seq.md");
    fs::write(
        &md,
        r#"---
sequence:
  - name: alpha
    group:
      name: bundle
      tasks:
        - name: first
          shell: "printf 'one\n' >> trace.txt"
        - name: second
          shell: "printf 'two\n' >> trace.txt"
---

Body.
"#,
    )
    .unwrap();

    let (_, stderr, code) = run(
        workspace.path(),
        &path_dir,
        &["sequence", "--goose", "--yolo", md.to_str().unwrap()],
    );

    assert_eq!(code, 0, "stderr:\n{stderr}");
    assert_eq!(trace(workspace.path()), vec!["one", "two"]);
    assert!(
        stderr.contains("first") && stderr.contains("second"),
        "the step breakdown names every member task; stderr:\n{stderr}"
    );
}

/// A `kind: group` file reached by reference behaves exactly like the same
/// group written inline.
#[test]
fn a_group_file_reference_executes_the_same_bundle() {
    let workspace = tempdir().unwrap();
    let path_dir = fake_goose(workspace.path());
    let bundle = workspace.path().join("bundle");
    fs::create_dir(&bundle).unwrap();
    fs::write(
        bundle.join("group.yaml"),
        "kind: group\nname: bundle\ntasks:\n  - name: first\n    shell: \"printf 'one\\n' >> trace.txt\"\n  - name: second\n    shell: \"printf 'two\\n' >> trace.txt\"\n",
    )
    .unwrap();
    let md = workspace.path().join("seq.md");
    fs::write(
        &md,
        "---\nsequence:\n  - name: alpha\n    group: bundle/group.yaml\n---\n\nBody.\n",
    )
    .unwrap();

    let (_, stderr, code) = run(
        workspace.path(),
        &path_dir,
        &["sequence", "--goose", "--yolo", md.to_str().unwrap()],
    );

    assert_eq!(code, 0, "stderr:\n{stderr}");
    assert_eq!(trace(workspace.path()), vec!["one", "two"]);
}

/// A named entry in a `kind: group-catalog` file is a third spelling of the
/// same bundle.
#[test]
fn a_named_catalog_entry_executes_the_same_bundle() {
    let workspace = tempdir().unwrap();
    let path_dir = fake_goose(workspace.path());
    fs::write(
        workspace.path().join("catalog.yaml"),
        "kind: group-catalog\ngroups:\n  - name: other\n    tasks:\n      - shell: \"printf 'wrong\\n' >> trace.txt\"\n  - name: bundle\n    tasks:\n      - name: first\n        shell: \"printf 'one\\n' >> trace.txt\"\n      - name: second\n        shell: \"printf 'two\\n' >> trace.txt\"\n",
    )
    .unwrap();
    let md = workspace.path().join("seq.md");
    fs::write(
        &md,
        "---\nsequence:\n  - name: alpha\n    group: bundle@catalog.yaml\n---\n\nBody.\n",
    )
    .unwrap();

    let (_, stderr, code) = run(
        workspace.path(),
        &path_dir,
        &["sequence", "--goose", "--yolo", md.to_str().unwrap()],
    );

    assert_eq!(code, 0, "stderr:\n{stderr}");
    assert_eq!(trace(workspace.path()), vec!["one", "two"]);
}

/// The first failed task stops the group and the remaining members do not run;
/// with sequence `fail_fast: false` the *sequence* still continues.
#[test]
fn a_failed_task_stops_the_group_but_fail_fast_false_continues_the_sequence() {
    let workspace = tempdir().unwrap();
    let path_dir = fake_goose(workspace.path());
    let md = workspace.path().join("seq.md");
    fs::write(
        &md,
        r#"---
fail_fast: false
sequence:
  - name: alpha
    group:
      name: bundle
      tasks:
        - name: ok
          shell: "printf 'one\n' >> trace.txt"
        - name: boom
          shell: "printf 'two\n' >> trace.txt; exit 3"
        - name: never
          shell: "printf 'three\n' >> trace.txt"
  - name: beta
    shell: "printf 'later\n' >> trace.txt"
---

Body.
"#,
    )
    .unwrap();

    let (_, stderr, code) = run(
        workspace.path(),
        &path_dir,
        &["sequence", "--goose", "--yolo", md.to_str().unwrap()],
    );

    assert_eq!(code, 1, "a failed group fails the run; stderr:\n{stderr}");
    assert_eq!(
        trace(workspace.path()),
        vec!["one", "two", "later"],
        "`never` must not run, but the next sequence step must; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("1 succeeded") && stderr.contains("1 failed"),
        "stderr:\n{stderr}"
    );
}

/// Sequence-level `fail_fast` is the only continuation control: with it on, a
/// failed group stops the sequence immediately.
#[test]
fn fail_fast_true_stops_the_sequence_after_a_failed_group() {
    let workspace = tempdir().unwrap();
    let path_dir = fake_goose(workspace.path());
    let md = workspace.path().join("seq.md");
    fs::write(
        &md,
        r#"---
fail_fast: true
sequence:
  - name: alpha
    group:
      name: bundle
      tasks:
        - name: boom
          shell: "printf 'one\n' >> trace.txt; exit 3"
  - name: beta
    shell: "printf 'later\n' >> trace.txt"
---

Body.
"#,
    )
    .unwrap();

    let (_, stderr, code) = run(
        workspace.path(),
        &path_dir,
        &["sequence", "--goose", "--yolo", md.to_str().unwrap()],
    );

    assert_eq!(code, 1, "stderr:\n{stderr}");
    assert_eq!(trace(workspace.path()), vec!["one"], "stderr:\n{stderr}");
}

/// Group variables reach a member task's prompt document, and the scope ends
/// with the group: the next step referencing `group.*` fails rather than
/// reading a stale value.
#[test]
fn group_variables_reach_members_and_do_not_leak_to_the_next_step() {
    let workspace = tempdir().unwrap();
    let path_dir = fake_goose(workspace.path());
    fs::write(
        workspace.path().join("member.md"),
        "---\nstart:\n  info: 'label is {{ doc.label }}'\n---\n\nMember body.\n",
    )
    .unwrap();
    let md = workspace.path().join("seq.md");
    fs::write(
        &md,
        r#"---
fail_fast: false
sequence:
  - name: alpha
    group:
      name: bundle
      variables:
        label: release
      tasks:
        - name: member
          prompt: member.md
          params:
            label: "{{ group.label }}"
  - name: beta
    prompt: member.md
    params:
      label: "{{ group.label }}"
---

Body.
"#,
    )
    .unwrap();

    let (_, stderr, code) = run(
        workspace.path(),
        &path_dir,
        &["sequence", "--goose", "--yolo", md.to_str().unwrap()],
    );

    assert!(
        stderr.contains("label is release"),
        "the member must read the group scope; stderr:\n{stderr}"
    );
    assert_eq!(
        code, 1,
        "the later step's `group.*` reference must fail; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("1 succeeded") && stderr.contains("1 failed"),
        "stderr:\n{stderr}"
    );
}

/// A serial group grows `outputs` entry by entry, so a later task reads the
/// previous task's output through `{{ last(outputs) }}` — whether that previous
/// task was a sibling in this group or an earlier sequence step.
#[test]
fn outputs_chain_through_a_serial_group() {
    let workspace = tempdir().unwrap();
    let path_dir = fake_goose(workspace.path());
    fs::write(
        workspace.path().join("member.md"),
        "---\nstart:\n  info: 'prior is {{ doc.prior }}'\n---\n\nMember body.\n",
    )
    .unwrap();
    let md = workspace.path().join("seq.md");
    fs::write(
        &md,
        r#"---
sequence:
  - name: alpha
    group:
      name: bundle
      tasks:
        - name: producer
          shell: "printf 'produced\n'"
        - name: consumer
          prompt: member.md
          params:
            prior: "{{ last(outputs) }}"
---

Body.
"#,
    )
    .unwrap();

    let (_, stderr, code) = run(
        workspace.path(),
        &path_dir,
        &["sequence", "--goose", "--yolo", md.to_str().unwrap()],
    );

    assert_eq!(code, 0, "stderr:\n{stderr}");
    assert!(
        stderr.contains("prior is produced"),
        "the second task must read the first task's committed output; stderr:\n{stderr}"
    );
}

/// A group carrying `loop:` is rejected during preflight — before any member
/// runs — because iteration commit semantics are not ratified.
#[test]
fn a_group_loop_is_rejected_before_anything_runs() {
    let workspace = tempdir().unwrap();
    let path_dir = fake_goose(workspace.path());
    let md = workspace.path().join("seq.md");
    fs::write(
        &md,
        r#"---
sequence:
  - name: alpha
    group:
      name: bundle
      loop:
        while: "true"
      tasks:
        - shell: "printf 'ran\n' >> trace.txt"
---

Body.
"#,
    )
    .unwrap();

    let (_, stderr, code) = run(
        workspace.path(),
        &path_dir,
        &["sequence", "--goose", "--yolo", md.to_str().unwrap()],
    );

    assert_ne!(code, 0, "stderr:\n{stderr}");
    assert!(
        stderr.contains("loop"),
        "the rejection must name the construct; stderr:\n{stderr}"
    );
    assert!(
        trace(workspace.path()).is_empty(),
        "preflight rejection means nothing ran; stderr:\n{stderr}"
    );
}
