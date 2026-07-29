//! End-to-end coverage for just-in-time serial sequence orchestration.
//!
//! Sequence Plus phase 8 (`claudine/features/2026-07-11-sequence-plus/`).
//!
//! Every test here distinguishes *eager* preparation from *just-in-time*
//! preparation, which is only observable through effects that do not exist when
//! the sequence starts: a body a prior step wrote back, a frontmatter value a
//! prior step `set`, an `outputs` entry a prior step committed, and a file an
//! earlier step created on disk. A sequence that composed every step up front
//! would render step 2 against the pre-run document and fail these.

#![cfg(unix)]

use std::fs;
use std::path::Path;
use tempfile::tempdir;
mod common;
use common::{augmented_path, strip_ansi, write_executable};

/// Install a fake `goose` that runs `script` as its body.
///
/// `$HOME` is the workspace, so a script can leave state behind for a later
/// step to observe — which is how "this ran at its turn" becomes assertable.
fn fake_goose(workspace: &Path, script: &str) -> std::path::PathBuf {
    let path_dir = workspace.join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    write_executable(&path_dir.join("goose"), &format!("#!/bin/sh\n{script}\n"));
    path_dir
}

/// A fake provider that echoes an incrementing counter, so each launch's output
/// is distinguishable and the launch count is readable from disk afterwards.
fn counting_goose(workspace: &Path) -> std::path::PathBuf {
    fake_goose(
        workspace,
        r#"count=0
if [ -f "$HOME/n.txt" ]; then
  IFS= read -r count < "$HOME/n.txt"
fi
count=$((count + 1))
printf '%s' "$count" > "$HOME/n.txt"
printf 'run-%s\n' "$count"
exit 0"#,
    )
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

// ============================================================================
// Live-disk chaining
// ============================================================================

/// The ratified live-disk contract: a step re-reads the source at its turn, so
/// a frontmatter value an earlier step's agent wrote is composed into the later
/// step's prompt.
///
/// Under eager preparation every step is composed before step 1 launches, so
/// step 2 would render the pre-run `marker` and this fails.
#[test]
fn a_later_step_composes_against_the_edit_an_earlier_step_made() {
    let workspace = tempdir().unwrap();
    let md = workspace.path().join("seq.md");
    // The provider rewrites the sequence document's own frontmatter on its
    // first (and only its first) launch.
    let path_dir = fake_goose(
        workspace.path(),
        r#"if [ ! -f "$HOME/done" ]; then
  : > "$HOME/done"
  printf -- '---\nmarker: rewritten\nsequence:\n  - alpha\n  - beta\nstart:\n  info: "marker is {{ doc.marker }}"\n---\nBody {{state}}.\n' > "$HOME/seq.md"
fi
printf 'ok\n'
exit 0"#,
    );
    fs::write(
        &md,
        "---\nmarker: original\nsequence:\n  - alpha\n  - beta\nstart:\n  info: 'marker is {{ doc.marker }}'\n---\nBody {{state}}.\n",
    )
    .unwrap();

    let (_, stderr, code) = run(
        workspace.path(),
        &path_dir,
        &["sequence", "--goose", md.to_str().unwrap()],
    );

    assert_eq!(code, 0, "sequence should succeed; stderr:\n{stderr}");
    assert!(
        stderr.contains("marker is original"),
        "step 1 composes the pre-run document; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("marker is rewritten"),
        "step 2 must re-read the live file and see step 1's edit; stderr:\n{stderr}"
    );
}

/// The *step list* is snapshotted at preflight and is never re-derived, even
/// though the document itself is re-read at each boundary.
///
/// The provider rewrites `sequence:` to three entries during step 1; the run
/// must still end after two steps.
#[test]
fn a_mid_run_edit_to_the_sequence_list_does_not_change_the_plan() {
    let workspace = tempdir().unwrap();
    let md = workspace.path().join("seq.md");
    let path_dir = fake_goose(
        workspace.path(),
        r#"if [ ! -f "$HOME/done" ]; then
  : > "$HOME/done"
  printf -- '---\nsequence:\n  - alpha\n  - beta\n  - gamma\n---\nBody {{state}}.\n' > "$HOME/seq.md"
fi
printf 'ok\n'
exit 0"#,
    );
    fs::write(
        &md,
        "---\nsequence:\n  - alpha\n  - beta\n---\nBody {{state}}.\n",
    )
    .unwrap();

    let (_, stderr, code) = run(
        workspace.path(),
        &path_dir,
        &["sequence", "--goose", md.to_str().unwrap()],
    );

    assert_eq!(code, 0, "sequence should succeed; stderr:\n{stderr}");
    assert!(
        stderr.contains("2 step(s)"),
        "the snapshot decides the plan; stderr:\n{stderr}"
    );
    assert!(
        !stderr.contains("3/3") && !stderr.contains("gamma"),
        "a mid-run edit to `sequence:` must not add a step; stderr:\n{stderr}"
    );
}

/// A `::shell` expansion in the body re-runs at each step's turn, so it reads
/// the filesystem as of that moment rather than as of sequence start.
#[test]
fn a_body_shell_expansion_reads_the_filesystem_at_its_turn() {
    let workspace = tempdir().unwrap();
    let md = workspace.path().join("seq.md");
    let path_dir = fake_goose(
        workspace.path(),
        r#"printf 'step-ran\n' >> "$HOME/log.txt"
printf 'ok\n'
exit 0"#,
    );
    fs::write(workspace.path().join("log.txt"), "").unwrap();
    fs::write(
        &md,
        "---\nsequence:\n  - alpha\n  - beta\nstart:\n  info: 'lines so far'\n---\nSeen so far: ::shell wc -l < log.txt\n",
    )
    .unwrap();
    // Pre-approve the command so the run is non-interactive.
    fs::write(
        workspace.path().join(".darkmatter-shell-whitelist"),
        "prefix wc\n",
    )
    .unwrap();

    let (_, stderr, code) = run(
        workspace.path(),
        &path_dir,
        &["sequence", "--goose", md.to_str().unwrap()],
    );

    assert_eq!(code, 0, "sequence should succeed; stderr:\n{stderr}");
    let log = fs::read_to_string(workspace.path().join("log.txt")).unwrap();
    assert_eq!(
        log.lines().count(),
        2,
        "both steps must have launched; log:\n{log}"
    );
}

// ============================================================================
// Runtime layers across steps
// ============================================================================

/// A `set` in step 1's lifecycle is visible to step 2's *composition*, not just
/// to step 2's lifecycle: the mutation layer sits under the reserved overlay and
/// above the live document, so the body interpolates the mutated value.
#[test]
fn a_set_from_an_earlier_step_is_composed_into_a_later_step() {
    let workspace = tempdir().unwrap();
    let path_dir = counting_goose(workspace.path());
    let md = workspace.path().join("seq.md");
    fs::write(
        &md,
        "---\nphase: initial\nsequence:\n  - alpha\n  - beta\nsuccess:\n  stack:\n    - action: {set: [phase, advanced]}\nstart:\n  info: 'phase={{ doc.phase }}'\n---\nBody for {{state}} in {{ doc.phase }}.\n",
    )
    .unwrap();

    let (_, stderr, code) = run(
        workspace.path(),
        &path_dir,
        &["sequence", "--goose", md.to_str().unwrap()],
    );

    assert_eq!(code, 0, "sequence should succeed; stderr:\n{stderr}");
    assert!(
        stderr.contains("phase=initial"),
        "step 1 predates the mutation; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("phase=advanced"),
        "step 2 must compose against the accumulated mutation; stderr:\n{stderr}"
    );
    // The mutation is in-memory only: the document on disk is untouched.
    let on_disk = fs::read_to_string(&md).unwrap();
    assert!(
        on_disk.contains("phase: initial"),
        "`set` must never write the file; document:\n{on_disk}"
    );
}

/// The reserved overlay outranks every lower layer: a `set` targeting a
/// reserved key is refused, and `state` keeps naming the step.
///
/// The assertion reads `state.name` rather than `{{state}}`: string-context
/// name coercion is a compose-time option, and a lifecycle `info:` resolves
/// after composition, where the typed object is what is in scope.
#[test]
fn the_reserved_overlay_outranks_a_runtime_mutation() {
    let workspace = tempdir().unwrap();
    let path_dir = counting_goose(workspace.path());
    let md = workspace.path().join("seq.md");
    fs::write(
        &md,
        "---\nsequence:\n  - alpha\n  - beta\nsuccess:\n  stack:\n    - action: {set: [state, hijacked]}\nstart:\n  info: 'state={{ state.name }}'\n---\nBody {{state}}.\n",
    )
    .unwrap();

    let (_, stderr, _) = run(
        workspace.path(),
        &path_dir,
        &["sequence", "--goose", md.to_str().unwrap()],
    );

    assert!(
        stderr.contains("state=alpha") && stderr.contains("state=beta"),
        "the overlay must keep naming each step; stderr:\n{stderr}"
    );
    assert!(
        !stderr.contains("state=hijacked"),
        "a reserved key must not be writable by `set`; stderr:\n{stderr}"
    );
}

/// A later step composes `{{ last(outputs) }}` against the entry the previous
/// step committed — the accumulator is a live runtime layer, not a preflight
/// snapshot.
#[test]
fn a_later_step_composes_the_previous_steps_output() {
    let workspace = tempdir().unwrap();
    let path_dir = counting_goose(workspace.path());
    let md = workspace.path().join("seq.md");
    fs::write(
        &md,
        "---\nsequence:\n  - alpha\n  - beta\nstart:\n  info: 'prior=[{{ last(outputs) }}]'\n---\nBody {{state}}.\n",
    )
    .unwrap();

    let (_, stderr, code) = run(
        workspace.path(),
        &path_dir,
        &["sequence", "--goose", md.to_str().unwrap()],
    );

    assert_eq!(code, 0, "sequence should succeed; stderr:\n{stderr}");
    assert!(
        stderr.contains("prior=[]"),
        "step 1 sees an empty accumulator; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("prior=[run-1]"),
        "step 2 must see step 1's committed entry; stderr:\n{stderr}"
    );
}

// ============================================================================
// Failure policy
// ============================================================================

/// A mid-run composition failure is a failure of *that step*: under
/// `fail_fast: false` the remaining steps still run.
#[test]
fn a_step_failure_lets_later_steps_run_when_fail_fast_is_false() {
    let workspace = tempdir().unwrap();
    // Fail the first launch, succeed afterwards.
    let path_dir = fake_goose(
        workspace.path(),
        r#"if [ ! -f "$HOME/first" ]; then
  : > "$HOME/first"
  printf 'boom\n'
  exit 3
fi
printf 'ok\n'
exit 0"#,
    );
    let md = workspace.path().join("seq.md");
    fs::write(
        &md,
        "---\nsequence:\n  - alpha\n  - beta\n---\nBody {{state}}.\n",
    )
    .unwrap();

    let (_, stderr, code) = run(
        workspace.path(),
        &path_dir,
        &[
            "sequence",
            "--goose",
            "--fail-fast",
            "false",
            md.to_str().unwrap(),
        ],
    );

    assert_ne!(code, 0, "a failed step must fail the run; stderr:\n{stderr}");
    assert!(
        stderr.contains("1/2") && stderr.contains("2/2"),
        "both steps must be attempted; stderr:\n{stderr}"
    );
}

/// The same fixture under `fail_fast: true` stops at the first failure and
/// never launches step 2.
#[test]
fn a_step_failure_stops_the_run_when_fail_fast_is_true() {
    let workspace = tempdir().unwrap();
    let path_dir = fake_goose(
        workspace.path(),
        r#"printf 'ran\n' >> "$HOME/launches.txt"
printf 'boom\n'
exit 3"#,
    );
    let md = workspace.path().join("seq.md");
    fs::write(
        &md,
        "---\nsequence:\n  - alpha\n  - beta\n---\nBody {{state}}.\n",
    )
    .unwrap();

    let (_, stderr, code) = run(
        workspace.path(),
        &path_dir,
        &[
            "sequence",
            "--goose",
            "--fail-fast",
            "true",
            md.to_str().unwrap(),
        ],
    );

    assert_ne!(code, 0, "a failed step must fail the run; stderr:\n{stderr}");
    let launches = fs::read_to_string(workspace.path().join("launches.txt")).unwrap();
    assert_eq!(
        launches.lines().count(),
        1,
        "fail-fast must not launch step 2; launches:\n{launches}"
    );
    assert!(
        !stderr.contains("2/2"),
        "step 2 must not start; stderr:\n{stderr}"
    );
}

// ============================================================================
// Explicit tasks replace the body
// ============================================================================

/// A step declaring `shell:` runs that command instead of the document body —
/// no provider is launched for it — and its stdout becomes the step's `outputs`
/// entry, visible to the next step.
#[test]
fn a_shell_task_step_replaces_the_body_and_contributes_its_stdout() {
    let workspace = tempdir().unwrap();
    let path_dir = fake_goose(
        workspace.path(),
        r#"printf 'launched\n' >> "$HOME/launches.txt"
printf 'ok\n'
exit 0"#,
    );
    let md = workspace.path().join("seq.md");
    fs::write(
        &md,
        "---\nsequence:\n  - name: work\n    shell: \"echo from-shell\"\n  - name: report\nstart:\n  info: 'prior=[{{ last(outputs) }}]'\n---\nBody {{state}}.\n",
    )
    .unwrap();
    fs::write(
        workspace.path().join(".darkmatter-shell-whitelist"),
        "prefix echo\n",
    )
    .unwrap();

    let (_, stderr, code) = run(
        workspace.path(),
        &path_dir,
        &["sequence", "--goose", md.to_str().unwrap()],
    );

    assert_eq!(code, 0, "sequence should succeed; stderr:\n{stderr}");
    let launches = fs::read_to_string(workspace.path().join("launches.txt")).unwrap_or_default();
    assert_eq!(
        launches.lines().count(),
        1,
        "only the bodyless step launches a provider; launches:\n{launches}"
    );
    assert!(
        stderr.contains("prior=[from-shell]"),
        "the shell task's stdout must be the accumulator entry; stderr:\n{stderr}"
    );
}

/// A failing `shell:` task fails its step, and `fail_fast` governs the run
/// exactly as it does for a failing provider step.
#[test]
fn a_failing_shell_task_fails_its_step() {
    let workspace = tempdir().unwrap();
    let path_dir = fake_goose(workspace.path(), "printf 'ok\\n'\nexit 0");
    let md = workspace.path().join("seq.md");
    fs::write(
        &md,
        "---\nsequence:\n  - name: work\n    shell: \"false\"\n---\nBody {{state}}.\n",
    )
    .unwrap();
    fs::write(
        workspace.path().join(".darkmatter-shell-whitelist"),
        "prefix false\n",
    )
    .unwrap();

    let (_, stderr, code) = run(
        workspace.path(),
        &path_dir,
        &["sequence", "--goose", md.to_str().unwrap()],
    );

    assert_ne!(code, 0, "a failing task must fail the run; stderr:\n{stderr}");
    assert!(
        stderr.contains("failed"),
        "the step must be reported failed; stderr:\n{stderr}"
    );
}

// ============================================================================
// Dry-run
// ============================================================================

/// Dry-run performs the full preflight and then composes each step against the
/// *initial* state, launching nothing and writing nothing back.
#[test]
fn dry_run_composes_every_step_and_launches_nothing() {
    let workspace = tempdir().unwrap();
    let path_dir = fake_goose(
        workspace.path(),
        r#"printf 'launched\n' >> "$HOME/launches.txt"
exit 0"#,
    );
    let md = workspace.path().join("seq.md");
    let original = "---\nprompt: 'Do {{state}}'\nsequence:\n  - alpha\n  - beta\n---\nBody.\n";
    fs::write(&md, original).unwrap();

    let (_, stderr, code) = run(
        workspace.path(),
        &path_dir,
        &["sequence", "--goose", "--dry-run", md.to_str().unwrap()],
    );

    assert_eq!(code, 0, "dry-run should succeed; stderr:\n{stderr}");
    assert!(
        !workspace.path().join("launches.txt").exists(),
        "dry-run must launch no provider; stderr:\n{stderr}"
    );
    assert_eq!(
        fs::read_to_string(&md).unwrap(),
        original,
        "dry-run must not write the document back"
    );
    assert!(
        stderr.contains("1/2") && stderr.contains("2/2"),
        "every step must be composed and rendered; stderr:\n{stderr}"
    );
}

/// Dry-run composes against empty `outputs` and no mutations, so a late-binding
/// reference renders as it would for the very first step — every step alike.
#[test]
fn dry_run_composes_every_step_against_the_initial_state() {
    let workspace = tempdir().unwrap();
    let path_dir = fake_goose(workspace.path(), "exit 0");
    let md = workspace.path().join("seq.md");
    fs::write(
        &md,
        "---\nsequence:\n  - alpha\n  - beta\n---\nPrior was [{{ last(outputs) }}].\n",
    )
    .unwrap();

    let (stdout, stderr, code) = run(
        workspace.path(),
        &path_dir,
        &["sequence", "--goose", "--dry-run", md.to_str().unwrap()],
    );

    assert_eq!(code, 0, "dry-run should succeed; stderr:\n{stderr}");
    assert_eq!(
        stdout.matches("Prior was [].").count(),
        2,
        "both rendered documents must show the initial empty accumulator; \
         stdout:\n{stdout}"
    );
}

// ============================================================================
// Interruption
// ============================================================================

/// Ctrl+C observed *between* steps stops the run at the next boundary and exits
/// `130`, having reported the work that did complete.
///
/// The fake provider signals the orchestrator directly, which is the same flag
/// a real Ctrl+C sets; the boundary check is what the just-in-time loop performs
/// before composing the next step.
#[test]
fn an_interrupt_between_steps_exits_130_with_a_partial_summary() {
    let workspace = tempdir().unwrap();
    let path_dir = fake_goose(
        workspace.path(),
        r#"printf 'launched\n' >> "$HOME/launches.txt"
kill -INT "$PPID" 2>/dev/null
printf 'ok\n'
exit 0"#,
    );
    let md = workspace.path().join("seq.md");
    fs::write(
        &md,
        "---\nsequence:\n  - alpha\n  - beta\n---\nBody {{state}}.\n",
    )
    .unwrap();

    let (_, stderr, code) = run(
        workspace.path(),
        &path_dir,
        &["sequence", "--goose", md.to_str().unwrap()],
    );

    assert_eq!(
        code, 130,
        "an interrupted sequence must exit 130; stderr:\n{stderr}"
    );
    let launches = fs::read_to_string(workspace.path().join("launches.txt")).unwrap();
    assert_eq!(
        launches.lines().count(),
        1,
        "step 2 must not launch after the interrupt; launches:\n{launches}"
    );
    assert!(
        stderr.contains("1/2"),
        "the partial summary must still report step 1; stderr:\n{stderr}"
    );
}
