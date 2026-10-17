//! `claudine sequence --dry-run` previews without effects: `shell:` and
//! `side_effect:` tasks are rendered, not run, loops preview one pass, and no
//! lifecycle event or `prep` target runs. `--dry-run-shell` runs the `shell:`
//! tasks once, and nothing else.

use std::fs;

use crate::common::{CliProcessFixture, strip_ansi};

struct Run {
    status: std::process::ExitStatus,
    stdout: String,
    stderr: String,
}

fn run(fixture: &CliProcessFixture, flags: &[&str]) -> Run {
    let output = fixture
        .command()
        .args(["sequence", "--goose", "--yolo"])
        .args(flags)
        .arg(fixture.cwd().join("seq.md"))
        .output()
        .unwrap();
    assert!(!fixture.audio_spool().exists());
    Run {
        status: output.status,
        stdout: strip_ansi(&String::from_utf8_lossy(&output.stdout)),
        stderr: strip_ansi(&String::from_utf8_lossy(&output.stderr)),
    }
}

fn prose(text: &str) -> String {
    text.replace('┃', " ").split_whitespace().collect::<Vec<_>>().join(" ")
}

/// A scratch repository with one untracked file and a sequence that stages
/// it. A fresh repository has no index file until something is staged.
#[cfg(unix)]
fn staging_fixture() -> CliProcessFixture {
    let fixture = CliProcessFixture::named("sequence-dry-run-git");
    assert!(crate::common::init_git_repo(fixture.cwd()), "git init");
    fs::write(fixture.cwd().join("change.txt"), "changed\n").unwrap();
    fs::write(fixture.cwd().join("seq.md"), "---\nsequence:\n  - name: stage\n    shell: \"git add change.txt\"\n---\n").unwrap();
    assert!(!fixture.cwd().join(".git/index").exists(), "nothing is staged yet");
    fixture
}

/// Whether the scratch repository's index records `change.txt`, read from the
/// index file itself.
#[cfg(unix)]
fn staged(fixture: &CliProcessFixture) -> bool {
    fs::read(fixture.cwd().join(".git/index"))
        .is_ok_and(|index| index.windows(b"change.txt".len()).any(|window| window == b"change.txt"))
}

/// `--dry-run` renders `git add` and leaves the index untouched.
#[cfg(unix)]
#[test]
fn a_dry_run_renders_a_shell_task_and_leaves_the_index_untouched() {
    let fixture = staging_fixture();
    let run = run(&fixture, &["--dry-run"]);
    assert!(run.status.success(), "stderr:\n{}", run.stderr);
    assert!(prose(&run.stderr).contains("rendered (dry-run): would run `git add change.txt`"), "stderr:\n{}", run.stderr);
    assert!(prose(&run.stderr).contains("Sequence previewed (dry-run): 1 step(s) rendered, none run"), "stderr:\n{}", run.stderr);
    assert!(!staged(&fixture), "the index is untouched");
}

/// `--dry-run --dry-run-shell` runs the `shell:` task once, so it stages.
#[cfg(unix)]
#[test]
fn dry_run_shell_runs_the_shell_task_and_stages() {
    let fixture = staging_fixture();
    let run = run(&fixture, &["--dry-run", "--dry-run-shell"]);
    assert!(run.status.success(), "stderr:\n{}", run.stderr);
    assert!(!run.stderr.contains("would run `git add"), "stderr:\n{}", run.stderr);
    assert!(staged(&fixture), "the shell task staged the file");
}

/// `--dry-run-shell` means nothing without `--dry-run`: a usage error that
/// runs nothing.
#[test]
fn dry_run_shell_without_dry_run_is_a_usage_error() {
    let fixture = CliProcessFixture::named("sequence-dry-run-usage");
    fs::write(fixture.cwd().join("seq.md"), "---\nsequence:\n  - name: a\n    side_effect: { append_line: [trace.log, a] }\n---\n").unwrap();
    let run = run(&fixture, &["--dry-run-shell"]);
    assert_eq!(run.status.code(), Some(2), "stderr:\n{}", run.stderr);
    assert!(run.stderr.contains("--dry-run"), "stderr:\n{}", run.stderr);
    assert!(!fixture.cwd().join("trace.log").exists());
}

/// A loop whose condition waits for a file an agent would write previews one
/// pass and ends; its lifecycle events and its `prep` target never run, and
/// the target is listed.
#[test]
fn a_dry_run_previews_one_pass_of_a_file_dependent_loop_with_no_lifecycle_or_prep() {
    let fixture = CliProcessFixture::named("sequence-dry-run-loop");
    fs::write(
        fixture.cwd().join("seq.md"),
        r#"---
loop:
  until: "file_exists('agent-wrote.txt')"
  max: 50
initialize:
  - append_line: [trace.log, "initialize"]
  - prep: "@helper.md"
start:
  - append_line: [trace.log, "start"]
finalize:
  - append_line: [trace.log, "finalize"]
sequence:
  - name: work
    when: "loop.count > 1"
    side_effect: { append_line: [work.log, "pass {{ loop.count }}"] }
---
"#,
    )
    .unwrap();
    fs::write(fixture.cwd().join("helper.md"), "---\ninitialize:\n  - append_line: [helper.log, ran]\n---\nHelper\n").unwrap();
    let run = run(&fixture, &["--dry-run"]);
    assert!(run.status.success(), "stderr:\n{}", run.stderr);
    let prose = prose(&run.stderr);
    assert!(prose.contains("`loop` previews one pass (dry-run); repetition is not evaluated: until `file_exists('agent-wrote.txt')`, max 50"), "stderr:\n{}", run.stderr);
    assert!(prose.contains("prep target @helper.md (not run in a dry run)"), "stderr:\n{}", run.stderr);
    assert!(prose.contains("`work` runs when `loop.count > 1` (not evaluated in a dry run)"), "stderr:\n{}", run.stderr);
    for absent in ["trace.log", "helper.log", "work.log"] {
        assert!(!fixture.cwd().join(absent).exists(), "{absent} must not be written; stderr:\n{}", run.stderr);
    }
}

const SIDE_EFFECTS: &str = r#"---
flag: original
sequence:
  - name: write
    side_effect: { append_line: [scratch.txt, "{{ flag }}"] }
  - name: mark
    side_effect: { set: { flag: changed } }
  - name: read
    shell: "echo flag={{ flag }}"
---
"#;

fn side_effects_run(flags: &[&str]) -> (CliProcessFixture, Run) {
    let fixture = CliProcessFixture::named("sequence-dry-run-side-effects");
    fs::write(fixture.cwd().join("seq.md"), SIDE_EFFECTS).unwrap();
    let run = run(&fixture, flags);
    assert!(run.status.success(), "{flags:?}; stderr:\n{}", run.stderr);
    let prose = prose(&run.stderr);
    assert!(prose.contains(r#"rendered (dry-run): would run `{"append_line":["scratch.txt","{{ flag }}"]}`"#), "{flags:?}; stderr:\n{}", run.stderr);
    assert!(prose.contains(r#"rendered (dry-run): would run `{"set":{"flag":"changed"}}`"#), "{flags:?}; stderr:\n{}", run.stderr);
    assert!(!fixture.cwd().join("scratch.txt").exists(), "{flags:?}");
    (fixture, run)
}

/// A `side_effect:` task that writes a file and one that sets a runtime value
/// are rendered with their authored arguments and not run under `--dry-run`:
/// the file stays absent, and the `shell:` task is rendered too.
#[test]
fn side_effect_tasks_are_rendered_and_never_run_in_a_dry_run() {
    let (_fixture, run) = side_effects_run(&["--dry-run"]);
    assert!(prose(&run.stderr).contains("rendered (dry-run): would run `echo flag={{ flag }}`"), "stderr:\n{}", run.stderr);
    assert!(!run.stdout.contains("flag="), "stdout:\n{}", run.stdout);
}

/// `--dry-run-shell` runs the `shell:` task but still no `side_effect:` task:
/// the command reads the value the unrun `set` would have replaced.
// Asserts on what POSIX `sh` `echo` prints.
#[cfg(unix)]
#[test]
fn dry_run_shell_still_renders_side_effect_tasks() {
    let (_fixture, run) = side_effects_run(&["--dry-run", "--dry-run-shell"]);
    assert!(run.stdout.contains("flag=original"), "stdout:\n{}\nstderr:\n{}", run.stdout, run.stderr);
    assert!(!run.stdout.contains("flag=changed"), "stdout:\n{}", run.stdout);
}
