//! The passive document check (`commands/document_check.rs`) on `sequence`,
//! `compose`, and `inline-compose`: it reports every problem it finds on
//! stderr, previews what composed on stdout, runs no command, side effect, or
//! provider, writes nothing, and exits 2 when any finding is an error or a
//! warning. The JSON report replaces both outputs with one envelope on stdout.
//!
//! The check does not yet replace `--dry-run`; these runs turn it on through
//! the `test-fixtures` seam `CLAUDINE_TEST_PASSIVE_DRY_RUN` (`1`, or `json`
//! for the JSON report, which `run` selects for an argument list naming the
//! planned `--json`).

use std::fs;
use std::path::Path;

use serde_json::Value;

use crate::common::{CliProcessFixture, strip_ansi};

const PASSIVE: &str = "CLAUDINE_TEST_PASSIVE_DRY_RUN";

struct Run {
    status: std::process::ExitStatus,
    stdout: String,
    stderr: String,
}

fn run(fixture: &CliProcessFixture, args: &[&str]) -> Run {
    // `--json` is the planned flag; until it ships the seam selects JSON.
    let format = if args.contains(&"--json") { "json" } else { "1" };
    let args: Vec<&str> = args.iter().copied().filter(|arg| *arg != "--json").collect();
    let output = fixture.command().env("NO_COLOR", "1").env(PASSIVE, format).args(&args).output().unwrap();
    assert!(!fixture.audio_spool().exists());
    Run {
        status: output.status,
        stdout: strip_ansi(&String::from_utf8_lossy(&output.stdout)),
        stderr: strip_ansi(&String::from_utf8_lossy(&output.stderr)),
    }
}

/// `text` with each run of whitespace as one space, so a wrapped row reads as
/// one line.
fn prose(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

/// A fixture holding `tests/fixtures/check/<name>/` in its launch directory.
fn check_fixture(name: &str) -> CliProcessFixture {
    let fixture = CliProcessFixture::named("dry-run-check");
    copy_tree(&biscuit_test_harness::manifest_dir!().join("tests/fixtures/check").join(name), fixture.cwd());
    fixture
}

/// An executable named `binary` on the fixture's `PATH`, which the check
/// only looks up and never runs.
fn install_provider_binary(fixture: &CliProcessFixture, binary: &str) {
    let path = fixture.bin_dir().join(format!("{binary}{}", std::env::consts::EXE_SUFFIX));
    fs::write(&path, "#!/bin/sh\nexit 0\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    }
}

/// `value` with the launch directory spelled `/work` and `/` separators, so a
/// JSON snapshot is the same on every host.
fn portable(value: &mut Value, launch: &[String]) {
    match value {
        Value::String(text) => {
            let mut normalized = text.replace('\\', "/");
            for prefix in launch {
                normalized = normalized.replace(prefix.as_str(), "/work");
            }
            *text = normalized;
        }
        Value::Array(items) => items.iter_mut().for_each(|item| portable(item, launch)),
        Value::Object(map) => map.values_mut().for_each(|item| portable(item, launch)),
        _ => {}
    }
}

fn launch_spellings(fixture: &CliProcessFixture) -> Vec<String> {
    let mut spellings = vec![fixture.cwd().to_string_lossy().replace('\\', "/")];
    if let Ok(canonical) = fixture.cwd().canonicalize() {
        let canonical = canonical.to_string_lossy().replace('\\', "/");
        spellings.push(canonical.trim_start_matches("//?/").to_string());
        spellings.push(canonical);
    }
    spellings.sort_by_key(|spelling| std::cmp::Reverse(spelling.len()));
    spellings
}

/// Run 1 of the motivating session: two misplaced selection keys and a
/// missing provider on `find`, and a task with no executable over a YAML
/// file. Every problem is reported, `find` is still previewed, and the run
/// exits 2.
#[test]
fn fleet_run1_reports_every_problem_and_previews_the_composable_task() {
    let fixture = check_fixture("fleet-run1");
    let run = run(&fixture, &["sequence", "renderers/_fleet.yaml", "--dry-run"]);
    assert_eq!(run.status.code(), Some(2), "stderr:\n{}", run.stderr);
    insta::assert_snapshot!(run.stderr);
    assert!(run.stdout.contains("write a short overview to ./overview.md"), "stdout:\n{}", run.stdout);
    assert!(!run.stderr.contains('┃'), "a dry run never prints the standard error report:\n{}", run.stderr);
}

/// Run 2: the missing prompt file no longer hides `find`'s findings, and the
/// JSON envelope carries the same findings as data.
#[test]
fn fleet_run2_names_the_missing_prompt_file_and_keeps_checking() {
    let fixture = check_fixture("fleet-run2");
    let human = run(&fixture, &["sequence", "renderers/_fleet.yaml", "--dry-run"]);
    assert_eq!(human.status.code(), Some(2), "stderr:\n{}", human.stderr);
    insta::assert_snapshot!("fleet_run2_human", human.stderr);

    let json = run(&fixture, &["sequence", "renderers/_fleet.yaml", "--dry-run", "--json"]);
    assert_eq!(json.status.code(), Some(2), "stderr:\n{}", json.stderr);
    assert!(json.stderr.is_empty(), "JSON mode prints nothing on stderr:\n{}", json.stderr);
    let mut envelope: Value = serde_json::from_str(&json.stdout).unwrap();
    assert_eq!(envelope["summary"], serde_json::json!({ "errors": 2, "warnings": 2, "info": 3 }));
    portable(&mut envelope, &launch_spellings(&fixture));
    insta::assert_json_snapshot!("fleet_run2_json", envelope);
}

/// The check continues past a task that fails: the third task is previewed
/// after the second names a missing file, and an unknown provider is an
/// error distinct from no provider at all.
#[test]
fn a_failed_task_does_not_stop_the_tasks_after_it() {
    let fixture = CliProcessFixture::named("dry-run-continue");
    install_provider_binary(&fixture, "claude");
    fs::write(fixture.cwd().join("one.md"), "first prompt\n").unwrap();
    fs::write(fixture.cwd().join("three.md"), "third prompt\n").unwrap();
    fs::write(
        fixture.cwd().join("seq.yaml"),
        "sequence:\n  - name: one\n    prompt: ./one.md\n    with: { agent: nosuch }\n  - name: two\n    prompt: ./two.md\n  - name: three\n    prompt: ./three.md\n    with: { agent: claude }\n",
    )
    .unwrap();
    let run = run(&fixture, &["sequence", "seq.yaml", "--dry-run"]);
    assert_eq!(run.status.code(), Some(2), "stderr:\n{}", run.stderr);
    assert!(run.stderr.contains("`nosuch` (with.agent) is not a known provider"), "stderr:\n{}", run.stderr);
    assert!(run.stderr.contains("the prompt file does not exist"), "stderr:\n{}", run.stderr);
    assert!(run.stderr.contains("Previewed 2 of 3 tasks."), "stderr:\n{}", run.stderr);
    assert!(run.stdout.contains("first prompt") && run.stdout.contains("third prompt"), "stdout:\n{}", run.stdout);
}

/// A `shell:` task, a `side_effect:` task, and a prompt's frontmatter
/// command are shown as not checked and never run; with nothing else to
/// report the dry run exits 0.
#[test]
fn commands_and_side_effects_never_run() {
    let fixture = CliProcessFixture::named("dry-run-effects");
    install_provider_binary(&fixture, "claude");
    fs::write(fixture.cwd().join("prompt.md"), "---\nagent: claude\nstamp: $(touch shell-marker)\n---\nstamp {{ stamp }}\n")
        .unwrap();
    fs::write(
        fixture.cwd().join("seq.md"),
        "---\nsequence:\n  - name: run\n    shell: \"touch task-marker\"\n  - name: write\n    side_effect: { append_line: [effect-marker, a] }\n  - name: ask\n    prompt: ./prompt.md\n---\n",
    )
    .unwrap();
    let run = run(&fixture, &["sequence", "seq.md", "--dry-run"]);
    assert_eq!(run.status.code(), Some(0), "stderr:\n{}", run.stderr);
    assert!(run.stderr.contains("not checked: command (a dry run runs no commands)"), "stderr:\n{}", run.stderr);
    assert!(run.stderr.contains("not checked: side effect (a dry run dispatches no side effects)"), "stderr:\n{}", run.stderr);
    assert!(run.stderr.contains("not checked: preview (it depends on a command a dry run skips)"), "stderr:\n{}", run.stderr);
    for marker in ["shell-marker", "task-marker", "effect-marker"] {
        assert!(!fixture.cwd().join(marker).exists(), "{marker} was created");
    }
}

/// A dry run with no configuration creates none: no `~/.darkmatter.yaml`,
/// no `~/.claudine/`, and nothing in the launch directory.
#[test]
fn a_dry_run_creates_no_configuration() {
    let fixture = check_fixture("fleet-run1").without_darkmatter_config();
    let before: Vec<_> = fs::read_dir(fixture.cwd()).unwrap().map(|entry| entry.unwrap().file_name()).collect();
    let run = run(&fixture, &["sequence", "renderers/_fleet.yaml", "--dry-run"]);
    assert_eq!(run.status.code(), Some(2), "stderr:\n{}", run.stderr);
    assert_eq!(fs::read_dir(fixture.home()).unwrap().count(), 0, "the home directory is untouched");
    let after: Vec<_> = fs::read_dir(fixture.cwd()).unwrap().map(|entry| entry.unwrap().file_name()).collect();
    assert_eq!(before, after);
}

/// `compose` checks one document: an installed provider and a composable
/// body exit 0 with the preview on stdout; a provider not on `PATH` and
/// OpenCode without a model are errors.
#[test]
fn compose_checks_the_provider_and_its_model_requirement() {
    let fixture = CliProcessFixture::named("dry-run-compose");
    fs::write(fixture.cwd().join("doc.md"), "---\nagent: claude\n---\nHello {{ who }}\n").unwrap();
    let missing = run(&fixture, &["compose", "doc.md", "who=world", "--dry-run"]);
    assert_eq!(missing.status.code(), Some(2), "stderr:\n{}", missing.stderr);
    assert!(prose(&missing.stderr).contains("is not installed: `claude` is not on PATH"), "stderr:\n{}", missing.stderr);

    install_provider_binary(&fixture, "claude");
    let clean = run(&fixture, &["compose", "doc.md", "who=world", "--dry-run"]);
    assert_eq!(clean.status.code(), Some(0), "stderr:\n{}", clean.stderr);
    assert_eq!(clean.stdout.trim(), "Hello world");
    assert!(clean.stderr.contains("Dry run found 0 errors and 0 warnings; 0 checks not run. Previewed 1 of 1 task."));

    install_provider_binary(&fixture, "opencode");
    let model = fixture
        .command()
        .env("NO_COLOR", "1")
        .env(PASSIVE, "1")
        .env_remove("OPENCODE_MODEL")
        .env_remove("MODEL")
        .args(["compose", "doc.md", "who=world", "--dry-run", "--provider", "opencode"])
        .output()
        .unwrap();
    let stderr = strip_ansi(&String::from_utf8_lossy(&model.stderr));
    assert_eq!(model.status.code(), Some(2), "stderr:\n{stderr}");
    assert!(stderr.contains("OpenCode needs a model in a non-interactive run"), "stderr:\n{stderr}");
}

/// `inline-compose --dry-run` leaves the input file byte-for-byte intact.
#[test]
fn inline_compose_dry_run_leaves_the_file_intact() {
    let fixture = CliProcessFixture::named("dry-run-inline");
    install_provider_binary(&fixture, "claude");
    let text = "---\nagent: claude\nprompt: Summarize this\n---\nBody\n";
    fs::write(fixture.cwd().join("doc.md"), text).unwrap();
    let run = run(&fixture, &["inline-compose", "doc.md", "--dry-run"]);
    assert_eq!(run.status.code(), Some(0), "stderr:\n{}", run.stderr);
    assert_eq!(fs::read_to_string(fixture.cwd().join("doc.md")).unwrap(), text);
}

/// A root document that does not exist is a finding in the report, not the
/// standard error report.
#[test]
fn a_missing_root_document_is_a_finding() {
    let fixture = CliProcessFixture::named("dry-run-missing-root");
    let run = run(&fixture, &["sequence", "nope.md", "--dry-run", "--json"]);
    assert_eq!(run.status.code(), Some(2), "stderr:\n{}", run.stderr);
    let envelope: Value = serde_json::from_str(&run.stdout).unwrap();
    assert_eq!(envelope["diagnostics"][0]["code"], "dm.reference.missing_document");
    assert_eq!(envelope["outcome"], "failure");
}

/// `--dry-run-shell` is refused by the passive check in every position on
/// every document command, with no report and nothing run.
#[test]
fn dry_run_shell_is_refused_by_the_passive_check() {
    let fixture = CliProcessFixture::named("dry-run-shell-removed");
    fs::write(fixture.cwd().join("seq.md"), "---\nsequence:\n  - name: a\n    shell: \"touch marker\"\n---\n").unwrap();
    for command in ["sequence", "compose", "inline-compose"] {
        for args in [
            vec![command, "--dry-run-shell", "--dry-run", "seq.md"],
            vec![command, "seq.md", "--dry-run", "--dry-run-shell"],
        ] {
            let run = run(&fixture, &args);
            assert_eq!(run.status.code(), Some(2), "{args:?}: stderr:\n{}", run.stderr);
            assert!(run.stderr.contains("`--dry-run-shell` was removed"), "{args:?}: stderr:\n{}", run.stderr);
            assert!(run.stdout.is_empty(), "{args:?}: stdout:\n{}", run.stdout);
            assert!(!run.stderr.contains("Dry run of"), "{args:?}");
        }
    }
    assert!(!fixture.cwd().join("marker").exists());
}
