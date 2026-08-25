//! End-to-end coverage for group execution, serial and parallel.
//!
//! Sequence Plus phases 9 and 10 (`claudine/features/2026-07-11-sequence-plus/`).
//!
//! A group is only observable end to end through its members' side effects:
//! which shell commands ran, in which order, whether the ones after a failure
//! ran at all, and whether the sequence continued past the failed group. Every
//! test here drives the real `claudine sequence` invocation path.

#![cfg(unix)]

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
    let output = assert_cmd::Command::cargo_bin("claudine").unwrap()
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

/// Run with ANSI intact, so bar colors can be compared across the two channels.
///
/// `FORCE_COLOR=1` is what routes claudine through an optimistic color terminal
/// when neither stream is a TTY; without it the palette collapses and a body
/// line cannot be matched to its header by color.
fn run_in_color(workspace: &Path, path_dir: &Path, args: &[&str]) -> (String, String, i32) {
    let output = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("FORCE_COLOR", "1")
        // `NO_COLOR` is absolute for this CLI (`log::colors_disabled`), so
        // `FORCE_COLOR` cannot out-vote an inherited one the way it does in
        // bare `biscuit-terminal` detection. Without this the child renders
        // colorless on a `NO_COLOR` host and every bar compares equal.
        .env_remove("NO_COLOR")
        .env("COLUMNS", "100")
        .env("HOME", workspace)
        .env("PATH", augmented_path(path_dir))
        .current_dir(workspace)
        .args(args)
        .assert()
        .get_output()
        .clone();
    (
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
        output.status.code().unwrap_or(-1),
    )
}

/// The rendered gutter of one framed line: everything through the bar glyph,
/// SGR sequences included. This *is* the attribution token.
fn bar_prefix(line: &str) -> Option<&str> {
    line.find('│').map(|byte| &line[..byte + '│'.len_utf8()])
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

/// A parallel group really overlaps its members: each task waits until every
/// sibling has started before any of them can finish. Serial execution cannot
/// satisfy that barrier.
#[test]
fn a_parallel_group_overlaps_its_members() {
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
      execution: parallel
      tasks:
        - name: slow-one
          shell: "touch started-one; attempts=0; while [ ! -f started-two ] || [ ! -f started-three ]; do attempts=$((attempts + 1)); [ $attempts -lt 500 ] || exit 90; sleep 0.01; done; printf 'one\n' >> trace.txt"
        - name: slow-two
          shell: "touch started-two; attempts=0; while [ ! -f started-one ] || [ ! -f started-three ]; do attempts=$((attempts + 1)); [ $attempts -lt 500 ] || exit 90; sleep 0.01; done; printf 'two\n' >> trace.txt"
        - name: slow-three
          shell: "touch started-three; attempts=0; while [ ! -f started-one ] || [ ! -f started-two ]; do attempts=$((attempts + 1)); [ $attempts -lt 500 ] || exit 90; sleep 0.01; done; printf 'three\n' >> trace.txt"
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
    let mut lines = trace(workspace.path());
    lines.sort();
    assert_eq!(lines, vec!["one", "three", "two"], "every member must run");
}

/// `max_parallel` bounds the overlap: the first pair holds its slots briefly
/// and records a violation if either second-wave task starts before those slots
/// are released.
#[test]
fn max_parallel_bounds_the_overlap() {
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
      execution: parallel
      max_parallel: 2
      tasks:
        - name: a
          shell: "touch started-a; attempts=0; while [ ! -f started-b ]; do attempts=$((attempts + 1)); [ $attempts -lt 500 ] || exit 90; sleep 0.01; done; sleep 0.5; if [ -f started-c ] || [ -f started-d ]; then touch cap-exceeded; fi; printf 'a\n' >> trace.txt"
        - name: b
          shell: "touch started-b; attempts=0; while [ ! -f started-a ]; do attempts=$((attempts + 1)); [ $attempts -lt 500 ] || exit 90; sleep 0.01; done; sleep 0.5; if [ -f started-c ] || [ -f started-d ]; then touch cap-exceeded; fi; printf 'b\n' >> trace.txt"
        - name: c
          shell: "touch started-c; attempts=0; while [ ! -f started-d ]; do attempts=$((attempts + 1)); [ $attempts -lt 500 ] || exit 90; sleep 0.01; done; printf 'c\n' >> trace.txt"
        - name: d
          shell: "touch started-d; attempts=0; while [ ! -f started-c ]; do attempts=$((attempts + 1)); [ $attempts -lt 500 ] || exit 90; sleep 0.01; done; printf 'd\n' >> trace.txt"
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
    assert_eq!(trace(workspace.path()).len(), 4);
    assert!(
        !workspace.path().join("cap-exceeded").exists(),
        "max_parallel allowed a second-wave task to overlap the first pair",
    );
}

/// A parallel group commits one nested `outputs` entry in declaration order,
/// even when the members finish in the opposite order.
#[test]
fn a_parallel_group_commits_a_declaration_ordered_nested_entry() {
    let workspace = tempdir().unwrap();
    let path_dir = fake_goose(workspace.path());
    fs::write(
        workspace.path().join("reader.md"),
        "---\nstart:\n  info: 'entry is {{ doc.entry }}'\n---\n\nReader body.\n",
    )
    .unwrap();
    let md = workspace.path().join("seq.md");
    // `first` sleeps the longest, so completion order is the reverse of
    // declaration order. Indexing the nested entry positionally is what proves
    // the slots follow declaration order rather than arrival order.
    fs::write(
        &md,
        r#"---
sequence:
  - name: alpha
    group:
      name: bundle
      execution: parallel
      tasks:
        - name: first
          shell: "sleep 1; printf 'from-first\n'"
        - name: second
          shell: "printf 'from-second\n'"
  - name: beta
    prompt: reader.md
    params:
      entry: "slot0={{ last(outputs)[0] }} slot1={{ last(outputs)[1] }}"
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
        stderr.contains("entry is slot0=from-first slot1=from-second"),
        "the nested entry's slots must follow declaration order, not the \
         reversed completion order; stderr:\n{stderr}"
    );
}

/// A failing member does not cancel its siblings, and the group's failure is
/// governed by sequence-level `fail_fast` exactly as a serial group's is.
#[test]
fn a_failed_parallel_member_lets_its_siblings_finish() {
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
      execution: parallel
      tasks:
        - name: boom
          shell: "printf 'boom\n' >> trace.txt; exit 3"
        - name: survivor
          shell: "sleep 1 && printf 'survivor\n' >> trace.txt"
  - name: beta
    shell: "printf 'later-step\n' >> trace.txt"
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

    assert_ne!(code, 0, "a failed member fails the group; stderr:\n{stderr}");
    let lines = trace(workspace.path());
    assert!(
        lines.contains(&"survivor".to_string()),
        "the sibling must run to completion after the failure; trace: {lines:?}",
    );
    assert!(
        lines.contains(&"later-step".to_string()),
        "`fail_fast: false` continues the sequence past a failed group; trace: {lines:?}",
    );
}

/// Two parallel members writing the same key resolve to the later-declared one
/// and warn on stderr naming the key and both tasks.
#[test]
fn a_contested_key_in_a_parallel_group_warns_and_resolves_by_declaration_order() {
    let workspace = tempdir().unwrap();
    let path_dir = fake_goose(workspace.path());
    fs::write(
        workspace.path().join("reader.md"),
        "---\nstart:\n  info: 'shared is {{ shared }}'\n---\n\nReader body.\n",
    )
    .unwrap();
    let md = workspace.path().join("seq.md");
    // The later-declared task finishes first, so a completion-order merge would
    // leave `from-early` behind.
    fs::write(
        &md,
        r#"---
shared: initial
sequence:
  - name: alpha
    group:
      name: bundle
      execution: parallel
      tasks:
        - name: early
          shell: "sleep 1; printf 'e\n'"
          setup:
            - action:
                - set: [shared, from-early]
        - name: late
          shell: "printf 'l\n'"
          setup:
            - action:
                - set: [shared, from-late]
  - name: beta
    prompt: reader.md
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
        stderr.contains("shared is from-late"),
        "the later-declared task must win regardless of completion order; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("shared") && stderr.contains("early") && stderr.contains("late"),
        "the collision must warn naming the key and both tasks; stderr:\n{stderr}"
    );
}

/// Concurrent members must be attributable without color: each announces itself
/// by name and closes with its own outcome and duration on stderr, and its body
/// output lands framed on stdout in between.
#[test]
fn parallel_group_members_are_attributed_across_both_channels() {
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
      execution: parallel
      tasks:
        - name: fetch-data
          shell: "printf 'fetched\n'"
        - name: render-page
          shell: "printf 'rendered\n'"
---

Body.
"#,
    )
    .unwrap();

    let (stdout, stderr, code) = run(
        workspace.path(),
        &path_dir,
        &["sequence", "--goose", "--yolo", "seq.md"],
    );

    assert_eq!(code, 0, "stderr: {stderr}");
    for name in ["fetch-data", "render-page"] {
        assert!(
            stderr.contains(&format!("▶ {name}")),
            "no header for `{name}` in stderr: {stderr}"
        );
        assert!(
            stderr.lines().any(|l| l.contains(name) && l.contains("succeeded")),
            "no success footer for `{name}` in stderr: {stderr}"
        );
    }
    // Status framing is stderr's job; stdout stays task/provider data only.
    assert!(
        !stdout.contains("▶ fetch-data"),
        "a header leaked onto stdout: {stdout}"
    );

    // The body is the point: each task's actual output must reach stdout inside
    // the same bar geometry its header and footer used, not fall outside the
    // framing (spec → *Reporting Concurrency*).
    for payload in ["fetched", "rendered"] {
        let framed = stdout
            .lines()
            .find(|line| line.contains(payload))
            .unwrap_or_else(|| panic!("`{payload}` never reached stdout: {stdout}"));
        assert!(
            framed.starts_with("│ ") || framed.starts_with("  "),
            "body line `{framed}` is outside the task framing"
        );
        assert!(
            !stderr.contains(payload),
            "task data leaked onto stderr: {stderr}"
        );
    }
}

/// A body line's bar must be the *same* bar its own header carried.
///
/// This is the assertion that makes interleaved concurrent output readable: the
/// header names the task and the body carries only the color, so if the two do
/// not agree the attribution is a lie. Colors are compared across the two
/// channels because that is exactly how a reader resolves them on a terminal.
#[test]
fn parallel_body_lines_carry_their_own_tasks_bar_color() {
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
      execution: parallel
      tasks:
        - name: fetch-data
          shell: "printf 'fetched\n'"
        - name: render-page
          shell: "printf 'rendered\n'"
---

Body.
"#,
    )
    .unwrap();

    let (stdout, stderr, code) = run_in_color(
        workspace.path(),
        &path_dir,
        &["sequence", "--goose", "--yolo", "seq.md"],
    );
    assert_eq!(code, 0, "stderr:\n{stderr}");

    let mut bars = Vec::new();
    for (name, payload) in [("fetch-data", "fetched"), ("render-page", "rendered")] {
        let header = stderr
            .lines()
            .find(|line| strip_ansi(line).contains(&format!("▶ {name}")))
            .unwrap_or_else(|| panic!("no header for `{name}`:\n{stderr}"));
        let body = stdout
            .lines()
            .find(|line| line.contains(payload))
            .unwrap_or_else(|| panic!("`{payload}` never reached stdout:\n{stdout}"));

        let header_bar = bar_prefix(header)
            .unwrap_or_else(|| panic!("header for `{name}` drew no bar: {header:?}"));
        let body_bar =
            bar_prefix(body).unwrap_or_else(|| panic!("body for `{name}` drew no bar: {body:?}"));
        assert_eq!(
            header_bar, body_bar,
            "`{name}`'s body line is attributed to a different task than its header"
        );
        bars.push(body_bar.to_string());
    }

    assert_ne!(
        bars[0], bars[1],
        "two concurrent tasks drew the same bar, so their body lines are indistinguishable"
    );
}

/// Serial members share the parallel geometry with nothing drawn in the gutter,
/// so switching modes does not shift the stream sideways.
#[test]
fn serial_and_parallel_group_frames_share_one_left_edge() {
    let column_of_header = |execution: &str| -> usize {
        let workspace = tempdir().unwrap();
        let path_dir = fake_goose(workspace.path());
        let md = workspace.path().join("seq.md");
        fs::write(
            &md,
            format!(
                r#"---
sequence:
  - name: alpha
    group:
      name: bundle
      execution: {execution}
      tasks:
        - name: only-task
          shell: "printf 'x\n'"
---

Body.
"#
            ),
        )
        .unwrap();
        let (_, stderr, code) = run(
        workspace.path(),
        &path_dir,
        &["sequence", "--goose", "--yolo", "seq.md"],
    );
        assert_eq!(code, 0, "stderr: {stderr}");
        let line = stderr
            .lines()
            .find(|line| line.contains("▶ only-task"))
            .unwrap_or_else(|| panic!("no header line in {stderr}"));
        let byte = line.find('▶').unwrap();
        line[..byte].chars().count()
    };

    assert_eq!(
        column_of_header("serial"),
        column_of_header("parallel"),
        "serial and parallel headers start at different columns"
    );
}

/// `--silent` suppresses the framing without changing what runs.
#[test]
fn a_silent_group_run_emits_no_task_frames() {
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
      execution: parallel
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
        &["sequence", "--goose", "--yolo", "seq.md", "--silent"],
    );

    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(!stderr.contains("▶ first"), "frames survived --silent: {stderr}");
    let mut ran = trace(workspace.path());
    ran.sort();
    assert_eq!(ran, vec!["one", "two"], "silencing changed what ran");
}

/// `--perf` attributes a group's wall-clock to its member tasks by name.
#[test]
fn perf_reports_the_group_task_hierarchy() {
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
      execution: parallel
      tasks:
        - name: fetch-data
          shell: "printf 'fetched\n'"
        - name: render-page
          shell: "printf 'rendered\n'"
---

Body.
"#,
    )
    .unwrap();

    let (_, stderr, code) = run(
        workspace.path(),
        &path_dir,
        &["sequence", "--goose", "--yolo", "seq.md", "--perf"],
    );

    assert_eq!(code, 0, "stderr: {stderr}");
    assert!(stderr.contains("group"), "no group node in perf tree: {stderr}");
    for name in ["fetch-data", "render-page"] {
        assert!(
            stderr.contains(name),
            "`{name}` missing from the perf tree: {stderr}"
        );
    }
}

/// A prompt task's provider output must land on the channel the command
/// contract puts it on, and stay attributed on both.
///
/// The pane test
/// (`level2_sequence_task_stream_capture::level2_parallel_prompt_streams_keep_task_attribution_in_tmux`)
/// proves both channels still carry the task's bar once merged; it structurally
/// cannot say *which* channel a line came from. This does, with two real pipes:
/// assistant text is data (stdout), reasoning and tool status are status
/// (stderr), and each carries its task's textual label under `NO_COLOR`.
#[test]
fn parallel_prompt_task_splits_data_and_status_across_channels() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    // `claude`, not `goose`: only a provider with a `stream_protocol` reaches
    // the semantic spawn where the task decorator is installed.
    write_executable(
        &path_dir.join("claude"),
        r#"#!/bin/sh
printf '%s\n' '{"type":"system","subtype":"init","session_id":"stub-1","model":"stub-model"}'
printf '%s\n' '{"type":"content_block_delta","delta":{"type":"thinking_delta","thinking":"weighing-options"}}'
printf '%s\n' '{"type":"assistant","message":{"content":[{"type":"text","text":"body-payload"}]}}'
printf '%s\n' '{"type":"tool_use","name":"Bash","input":{"command":"stub-tool-call"}}'
printf '%s\n' '{"type":"result","subtype":"success","is_error":false,"result":"done"}'
exit 0
"#,
    );

    fs::write(workspace.path().join("one.md"), "---\nprompt: hi\n---\n\nOne.\n").unwrap();
    fs::write(workspace.path().join("two.md"), "---\nprompt: hi\n---\n\nTwo.\n").unwrap();
    fs::write(
        workspace.path().join("seq.md"),
        r#"---
sequence:
  - name: alpha
    group:
      name: bundle
      execution: parallel
      tasks:
        - name: task-one
          prompt: one.md
        - name: task-two
          prompt: two.md
---

Body.
"#,
    )
    .unwrap();

    let output = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .current_dir(workspace.path())
        .args(["sequence", "--claude", "--yolo", "seq.md"])
        .assert()
        .get_output()
        .clone();
    let stdout = strip_ansi(&String::from_utf8_lossy(&output.stdout));
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));

    assert_eq!(output.status.code(), Some(0), "stderr: {stderr}");

    // Assistant text is data: stdout only, once per member.
    assert_eq!(
        stdout.matches("body-payload").count(),
        2,
        "each member's assistant text must reach stdout exactly once: {stdout}"
    );
    assert!(
        !stderr.contains("body-payload"),
        "assistant text leaked onto the status channel: {stderr}"
    );

    // Reasoning and tool status are status: stderr only.
    for marker in ["weighing-options", "stub-tool-call"] {
        assert_eq!(
            stderr.matches(marker).count(),
            2,
            "`{marker}` must reach stderr once per member: {stderr}"
        );
        assert!(
            !stdout.contains(marker),
            "`{marker}` leaked onto the data channel: {stdout}"
        );
    }

    // Attribution without color: every member names itself on the status
    // channel, so a reader can still tell the two interleaved streams apart.
    for task in ["task-one", "task-two"] {
        assert!(
            stderr
                .lines()
                .any(|line| line.contains(task) && line.contains("succeeded")),
            "`{task}` has no named outcome footer: {stderr}"
        );
    }
}

/// Concurrency must not merge two members' provider sessions into one JSONL
/// record stream. Each member's run keeps its own session identity.
#[test]
fn parallel_group_members_keep_separate_provider_sessions() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    // Each launch records the session id it was handed, one line per launch.
    write_executable(
        &path_dir.join("goose"),
        "#!/bin/sh\nprintf '%s\\n' \"${CLAUDINE_SESSION_ID:-none}\" >> \"$CLAUDINE_TEST_SESSIONS\"\nprintf 'agent-said\\n'\nexit 0\n",
    );
    let sessions = workspace.path().join("sessions.txt");
    fs::write(&sessions, "").unwrap();

    fs::write(workspace.path().join("one.md"), "---\nprompt: hi\n---\n\nOne.\n").unwrap();
    fs::write(workspace.path().join("two.md"), "---\nprompt: hi\n---\n\nTwo.\n").unwrap();
    fs::write(
        workspace.path().join("seq.md"),
        r#"---
sequence:
  - name: alpha
    group:
      name: bundle
      execution: parallel
      tasks:
        - name: first
          prompt: one.md
        - name: second
          prompt: two.md
---

Body.
"#,
    )
    .unwrap();

    let output = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .env("CLAUDINE_TEST_SESSIONS", &sessions)
        .current_dir(workspace.path())
        .args(["sequence", "--goose", "--yolo", "seq.md"])
        .assert()
        .get_output()
        .clone();
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));

    assert_eq!(output.status.code(), Some(0), "stderr: {stderr}");
    let recorded: Vec<String> = fs::read_to_string(&sessions)
        .unwrap()
        .lines()
        .map(str::to_string)
        .collect();
    assert_eq!(recorded.len(), 2, "expected one launch per member: {recorded:?}");
    let distinct: std::collections::HashSet<&String> = recorded.iter().collect();
    assert_eq!(
        distinct.len(),
        2,
        "two concurrent members shared one session id: {recorded:?}"
    );
}
