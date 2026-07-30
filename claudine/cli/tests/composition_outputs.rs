//! End-to-end coverage for the invocation-local runtime layer: the `outputs`
//! accumulator and the `set` side effect, driven through the real
//! `claudine compose` / `claudine inline-compose` / `claudine sequence`
//! invocation paths with a fake provider on `PATH`.
//!
//! Sequence Plus phase 6 (`claudine/features/2026-07-11-sequence-plus/`).

#![cfg(unix)]

use std::fs;
use std::path::Path;
use tempfile::tempdir;
mod common;
use common::{augmented_path, strip_ansi, write_executable};

/// Install a fake `goose` that prints `text` on stdout and exits `code`.
///
/// The wrapped-execution pipeline treats that stdout as the provider's final
/// assistant text, which is exactly what an `outputs` entry is made of.
fn fake_goose(workspace: &Path, text: &str, code: i32) -> std::path::PathBuf {
    let path_dir = workspace.join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    write_executable(
        &path_dir.join("goose"),
        &format!("#!/bin/sh\nprintf '%s\\n' '{text}'\nexit {code}\n"),
    );
    path_dir
}

/// Run a claudine subcommand against `file` and return its plain stderr.
fn run(workspace: &Path, path_dir: &Path, args: &[&str]) -> (String, String) {
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
    )
}

// ============================================================================
// `outputs` in a standalone compose
// ============================================================================

/// A standalone `compose` document reads `{{ last(outputs) }}` in its body and
/// gets an empty accumulator rather than an undefined-root failure or a raw
/// span leaking into the delivered prompt.
#[test]
fn standalone_compose_body_sees_an_initialized_empty_accumulator() {
    let workspace = tempdir().unwrap();
    let path_dir = fake_goose(workspace.path(), "provider said hello", 0);
    let md = workspace.path().join("doc.md");
    fs::write(&md, "---\ntitle: t\n---\nPrevious was [{{ last(outputs) }}].\n").unwrap();

    let (_, stderr) = run(
        workspace.path(),
        &path_dir,
        &["compose", "--goose", md.to_str().unwrap()],
    );

    assert!(
        !stderr.contains("{{"),
        "no raw span may survive into the run; stderr:\n{stderr}"
    );
    assert!(
        !stderr.to_lowercase().contains("undefined"),
        "`outputs` must be a defined root; stderr:\n{stderr}"
    );
}

/// The `success` hook sees the accumulator with exactly one element — this
/// run's output — so `{{ last(outputs) }}` means the same thing standalone as
/// it does mid-sequence.
#[test]
fn success_sees_this_runs_output_appended() {
    let workspace = tempdir().unwrap();
    let path_dir = fake_goose(workspace.path(), "the summary line", 0);
    let md = workspace.path().join("doc.md");
    fs::write(
        &md,
        "---\ntitle: t\nsuccess:\n  info: 'count={{ length(outputs) }} last={{ last(outputs) }}'\n---\nBody.\n",
    )
    .unwrap();

    let (_, stderr) = run(
        workspace.path(),
        &path_dir,
        &["compose", "--goose", md.to_str().unwrap()],
    );

    assert!(
        stderr.contains("count=1 last=the summary line"),
        "success must observe the appended entry; stderr:\n{stderr}"
    );
}

/// The entry survives into `finalize`, which the spec says sees whatever has
/// accumulated at that point — including the successful run's output.
#[test]
fn finalize_after_success_sees_the_accumulated_entry() {
    let workspace = tempdir().unwrap();
    let path_dir = fake_goose(workspace.path(), "final text", 0);
    let md = workspace.path().join("doc.md");
    fs::write(
        &md,
        "---\ntitle: t\nfinalize:\n  info: 'final={{ last(outputs) }}'\n---\nBody.\n",
    )
    .unwrap();

    let (_, stderr) = run(
        workspace.path(),
        &path_dir,
        &["compose", "--goose", md.to_str().unwrap()],
    );

    assert!(
        stderr.contains("final=final text"),
        "finalize must observe the entry; stderr:\n{stderr}"
    );
}

/// `start` fires before the provider has produced anything, so it sees only
/// prior entries — none, in a standalone run.
#[test]
fn start_sees_only_prior_entries() {
    let workspace = tempdir().unwrap();
    let path_dir = fake_goose(workspace.path(), "later output", 0);
    let md = workspace.path().join("doc.md");
    fs::write(
        &md,
        "---\ntitle: t\nstart:\n  info: 'at-start={{ length(outputs) }}'\n---\nBody.\n",
    )
    .unwrap();

    let (_, stderr) = run(
        workspace.path(),
        &path_dir,
        &["compose", "--goose", md.to_str().unwrap()],
    );

    assert!(
        stderr.contains("at-start=0"),
        "start must not observe the not-yet-produced output; stderr:\n{stderr}"
    );
}

/// A failed run commits no entry: `failure` and `finalize` both see an empty
/// accumulator, not a partial one.
#[test]
fn a_failed_run_commits_no_entry() {
    let workspace = tempdir().unwrap();
    let path_dir = fake_goose(workspace.path(), "partial work", 7);
    let md = workspace.path().join("doc.md");
    fs::write(
        &md,
        "---\ntitle: t\nfailure:\n  info: 'at-failure={{ length(outputs) }}'\nfinalize:\n  info: 'at-finalize={{ length(outputs) }}'\n---\nBody.\n",
    )
    .unwrap();

    let (_, stderr) = run(
        workspace.path(),
        &path_dir,
        &["compose", "--goose", md.to_str().unwrap()],
    );

    assert!(
        stderr.contains("at-failure=0"),
        "a failure appends nothing; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("at-finalize=0"),
        "finalize after failure sees only prior entries; stderr:\n{stderr}"
    );
}

/// The captured entry is the undecorated stdout payload: interior newlines and
/// indentation survive, no trailing transport newline remains, and none of
/// Claudine's own status rendering leaks in.
///
/// The exact one-newline-only trimming rule is pinned by
/// `composition::runtime_state::tests` — here the provider profile has already
/// normalized its capture, so this asserts the boundary, not the rule.
#[test]
fn the_captured_entry_is_undecorated_and_preserves_interior_whitespace() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    // Two content lines plus a trailing blank line: only the final transport
    // newline is stripped, so the authored blank line survives.
    write_executable(
        &path_dir.join("goose"),
        "#!/bin/sh\nprintf 'line one\\n  line two\\n\\n'\nexit 0\n",
    );
    let md = workspace.path().join("doc.md");
    fs::write(
        &md,
        "---\ntitle: t\nsuccess:\n  info: 'captured=[{{ last(outputs) }}]'\n---\nBody.\n",
    )
    .unwrap();

    let (_, stderr) = run(
        workspace.path(),
        &path_dir,
        &["compose", "--goose", md.to_str().unwrap()],
    );

    assert!(
        stderr.contains("captured=[line one\n  line two]"),
        "interior newline and indentation kept, no trailing newline; stderr:\n{stderr}"
    );
    assert!(
        !stderr.contains("captured=[\u{1b}"),
        "the entry must carry no ANSI decoration; stderr:\n{stderr}"
    );
}

/// Provider stderr is not part of the captured entry — only the stdout
/// payload is.
#[test]
fn provider_stderr_is_excluded_from_the_captured_entry() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    write_executable(
        &path_dir.join("goose"),
        "#!/bin/sh\nprintf 'diagnostic noise\\n' >&2\nprintf 'real answer\\n'\nexit 0\n",
    );
    let md = workspace.path().join("doc.md");
    fs::write(
        &md,
        "---\ntitle: t\nsuccess:\n  info: 'captured=[{{ last(outputs) }}]'\n---\nBody.\n",
    )
    .unwrap();

    let (_, stderr) = run(
        workspace.path(),
        &path_dir,
        &["compose", "--goose", md.to_str().unwrap()],
    );

    assert!(
        stderr.contains("captured=[real answer]"),
        "the entry is stdout only; stderr:\n{stderr}"
    );
}

/// `inline-compose` shares the accumulator contract with `compose`.
#[test]
fn inline_compose_success_sees_this_runs_output() {
    let workspace = tempdir().unwrap();
    let path_dir = fake_goose(workspace.path(), "generated body", 0);
    let md = workspace.path().join("doc.md");
    fs::write(
        &md,
        "---\ntitle: t\nprompt: Write something.\nsuccess:\n  info: 'inline-last={{ last(outputs) }}'\n---\nold body\n",
    )
    .unwrap();

    let (_, stderr) = run(
        workspace.path(),
        &path_dir,
        &["inline-compose", "--goose", md.to_str().unwrap()],
    );

    assert!(
        stderr.contains("inline-last=generated body"),
        "inline-compose must commit its output too; stderr:\n{stderr}"
    );
}

// ============================================================================
// `set` — the runtime mutation layer
// ============================================================================

/// A `set` in `start` is visible to a later event in the same run, and never
/// reaches the document on disk.
#[test]
fn set_is_visible_to_a_later_event_and_writes_no_file() {
    let workspace = tempdir().unwrap();
    let path_dir = fake_goose(workspace.path(), "ok", 0);
    let md = workspace.path().join("doc.md");
    let source = "---\ntitle: t\nphase: plan\nstart:\n  stack:\n    - action: {set: [phase, build]}\nsuccess:\n  info: 'phase={{ phase }}'\n---\nBody.\n";
    fs::write(&md, source).unwrap();

    let (_, stderr) = run(
        workspace.path(),
        &path_dir,
        &["compose", "--goose", md.to_str().unwrap()],
    );

    assert!(
        stderr.contains("phase=build"),
        "the mutation must be visible to `success`; stderr:\n{stderr}"
    );
    assert_eq!(
        fs::read_to_string(&md).unwrap(),
        source,
        "`set` is in-memory only — the document on disk must be byte-identical"
    );
}

/// A `set` targeting a reserved root key fails the run before the provider
/// launches. The refusal's typed message is asserted at the unit level
/// (`executor::tests::runtime_set`); the setup-phase surface here is the
/// existing generic `lifecycle start failed`, shared with every other
/// setup-stack side-effect failure.
#[test]
fn set_targeting_a_reserved_key_fails_the_run_before_launch() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    let witness = workspace.path().join("launched.txt");
    write_executable(
        &path_dir.join("goose"),
        &format!(
            "#!/bin/sh\nprintf 'launched\\n' >> '{}'\nexit 0\n",
            witness.display()
        ),
    );
    let md = workspace.path().join("doc.md");
    fs::write(
        &md,
        "---\ntitle: t\nstart:\n  stack:\n    - action: {set: [outputs, hijacked]}\n---\nBody.\n",
    )
    .unwrap();

    let output = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .current_dir(workspace.path())
        .args(["compose", "--goose", md.to_str().unwrap()])
        .assert()
        .failure()
        .get_output()
        .clone();

    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    assert!(
        stderr.contains("lifecycle start failed"),
        "the setup stack must fail the run; stderr:\n{stderr}"
    );
    assert!(!witness.exists(), "no provider may launch after a refused `set`");
}

/// A whole-value span keeps its type through `set`, so a later expression sees
/// a boolean rather than the string `"true"`.
#[test]
fn set_preserves_a_whole_value_type_end_to_end() {
    let workspace = tempdir().unwrap();
    let path_dir = fake_goose(workspace.path(), "ok", 0);
    let md = workspace.path().join("doc.md");
    fs::write(
        &md,
        "---\ntitle: t\nstart:\n  stack:\n    - action: {set: [ready, '{{ true }}']}\nsuccess:\n  stack:\n    - when: ready\n      action: {info: 'ready-is-boolean-true'}\n---\nBody.\n",
    )
    .unwrap();

    let (_, stderr) = run(
        workspace.path(),
        &path_dir,
        &["compose", "--goose", md.to_str().unwrap()],
    );

    assert!(
        stderr.contains("ready-is-boolean-true"),
        "the `when:` guard must see a truthy typed value; stderr:\n{stderr}"
    );
}

// ============================================================================
// Accumulation across a sequence
// ============================================================================

/// One cell spans the whole sequence: after two steps the accumulator holds
/// both entries, in execution order.
#[test]
fn a_sequence_accumulates_one_entry_per_step() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    // Each launch emits a distinct line so the accumulator's order is visible.
    write_executable(
        &path_dir.join("goose"),
        r#"#!/bin/sh
count=0
if [ -f "$HOME/n.txt" ]; then
  IFS= read -r count < "$HOME/n.txt"
fi
count=$((count + 1))
printf '%s' "$count" > "$HOME/n.txt"
printf 'output-%s\n' "$count"
exit 0
"#,
    );
    let md = workspace.path().join("seq.md");
    fs::write(
        &md,
        "---\nsequence:\n  - alpha\n  - beta\nfinalize:\n  info: 'acc n={{ length(outputs) }} last={{ last(outputs) }}'\n---\nRun {{state}}.\n",
    )
    .unwrap();

    let (_, stderr) = run(
        workspace.path(),
        &path_dir,
        &["sequence", "--goose", md.to_str().unwrap()],
    );

    assert!(
        stderr.contains("acc n=1 last=output-1"),
        "step 1's entry must be committed; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("acc n=2 last=output-2"),
        "step 2 must see both entries, its own last; stderr:\n{stderr}"
    );
}

/// A failing step contributes no entry, so the accumulator stays aligned with
/// *successful* work rather than with attempted work.
#[test]
fn a_failing_sequence_step_contributes_no_entry() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    // Step 1 fails; step 2 succeeds.
    write_executable(
        &path_dir.join("goose"),
        r#"#!/bin/sh
count=0
if [ -f "$HOME/n.txt" ]; then
  IFS= read -r count < "$HOME/n.txt"
fi
count=$((count + 1))
printf '%s' "$count" > "$HOME/n.txt"
printf 'output-%s\n' "$count"
if [ "$count" = "1" ]; then exit 7; fi
exit 0
"#,
    );
    let md = workspace.path().join("seq.md");
    fs::write(
        &md,
        "---\nsequence:\n  - alpha\n  - beta\nfinalize:\n  info: 'acc n={{ length(outputs) }} last={{ last(outputs) }}'\n---\nRun {{state}}.\n",
    )
    .unwrap();

    let (_, stderr) = run(
        workspace.path(),
        &path_dir,
        &["sequence", "--goose", "--fail-fast", "false", md.to_str().unwrap()],
    );

    assert!(
        stderr.contains("acc n=1 last=output-2"),
        "only the succeeding step commits an entry, so the accumulator holds \
         exactly one — the second step's; stderr:\n{stderr}"
    );
    assert!(
        !stderr.contains("last=output-1"),
        "the failed step must not have committed an entry; stderr:\n{stderr}"
    );
}

// ============================================================================
// Accumulation across `--loop` iterations
// ============================================================================

/// One cell spans the whole `--loop` run: `outputs` grows per iteration and a
/// `set` written in iteration 1 survives the iteration-2 re-composition from
/// disk. `_loop_last_output` reports the same text the accumulator committed.
#[test]
fn a_loop_accumulates_outputs_and_retains_mutations_across_iterations() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    write_executable(
        &path_dir.join("goose"),
        r#"#!/bin/sh
count=0
if [ -f "$HOME/n.txt" ]; then
  IFS= read -r count < "$HOME/n.txt"
fi
count=$((count + 1))
printf '%s' "$count" > "$HOME/n.txt"
printf 'output-%s\n' "$count"
exit 0
"#,
    );
    let md = workspace.path().join("doc.md");
    fs::write(
        &md,
        "---\ntitle: t\nloop:\n  until: 'length(outputs) >= 2'\n  max: 3\nstart:\n  info: 'iter n={{ length(outputs) }} carried={{ carried }} prev={{ _loop_last_output }}'\n  stack:\n    - action: {set: [carried, 'from-iteration-1']}\ncarried: none\n---\nBody.\n",
    )
    .unwrap();

    let (_, stderr) = run(
        workspace.path(),
        &path_dir,
        &["compose", "--goose", md.to_str().unwrap()],
    );

    assert!(
        stderr.contains("iter n=0 carried=none prev="),
        "iteration 1 starts with an empty accumulator and the authored value; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("iter n=1 carried=from-iteration-1 prev=output-1"),
        "iteration 2 must see the committed entry and the retained mutation, \
         not the on-disk `carried: none`; stderr:\n{stderr}"
    );
}
