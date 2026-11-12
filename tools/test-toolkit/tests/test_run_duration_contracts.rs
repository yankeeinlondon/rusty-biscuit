//! Contract tests for the duration line `just/shell.just`'s `_test_run_scope`
//! gives every test recipe: one line, last, however the recipes nest, and never
//! at the cost of the run's exit code.
//!
//! Each test drives a real `just` against a throwaway justfile importing the
//! repo's real recipe. Nothing here needs Cargo or Nextest.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

const LINE_PREFIX: &str = "🕖 this test run took ";

const RECIPES: &str = r#"
one:
    #!/usr/bin/env bash
    set -euo pipefail
    eval "$(just _test_run_scope)"
    echo one-body

# Linewise, the shape of a recipe that chains several engines.
chained:
    @eval "$(just _test_run_scope)"; \
        just one; \
        just one; \
        echo chained-after

failing:
    #!/usr/bin/env bash
    set -euo pipefail
    eval "$(just _test_run_scope)"
    exit 7

own-trap:
    #!/usr/bin/env bash
    set -euo pipefail
    eval "$(just _test_run_scope)"
    trap 'echo own-trap-cleanup; _test_run_finish' EXIT

long:
    #!/usr/bin/env bash
    set -euo pipefail
    eval "$(just _test_run_scope)"
    BISCUIT_TEST_RUN_STARTED=$(( $(date +%s) - 61 ))
"#;

fn run(recipe: &str, env: &[(&str, &str)]) -> Output {
    let fixture = TempDir::new().expect("create fixture directory");
    let shell_just = biscuit_test_harness::manifest_dir!()
        .ancestors()
        .map(|dir| dir.join("just").join("shell.just"))
        .find(|path| path.is_file())
        .expect("could not locate just/shell.just from CARGO_MANIFEST_DIR");
    // `just` accepts `/`-separated import paths on every platform.
    let import = shell_just.to_string_lossy().replace('\\', "/");
    write_justfile(fixture.path(), &import);

    let mut command = Command::new("just");
    command.arg(recipe).current_dir(fixture.path());
    // This test's own Nextest run and any hosted runner would silence the line.
    for key in ["NEXTEST_RUN_ID", "GITHUB_ACTIONS", "BISCUIT_TEST_RUN_STARTED"] {
        command.env_remove(key);
    }
    command.envs(env.iter().copied());
    command.output().expect("run just in the fixture directory")
}

fn write_justfile(dir: &Path, import: &str) {
    let justfile = format!(
        "set shell := [\"bash\", \"-eu\", \"-o\", \"pipefail\", \"-c\"]\nimport '{import}'\n{RECIPES}"
    );
    fs::write(dir.join("justfile"), justfile).expect("write fixture justfile");
}

fn stdout_lines(output: &Output) -> Vec<String> {
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::to_owned)
        .collect()
}

fn duration_lines(lines: &[String]) -> usize {
    lines.iter().filter(|line| line.starts_with(LINE_PREFIX)).count()
}

#[test]
fn a_recipe_reports_its_duration_last() {
    let output = run("one", &[]);
    let lines = stdout_lines(&output);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(duration_lines(&lines), 1, "{lines:?}");
    assert!(lines[lines.len() - 1].starts_with(LINE_PREFIX), "{lines:?}");
}

#[test]
fn chained_recipes_report_once_after_all_their_output() {
    let output = run("chained", &[]);
    let lines = stdout_lines(&output);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(duration_lines(&lines), 1, "{lines:?}");
    assert_eq!(lines[lines.len() - 2], "chained-after", "{lines:?}");
    assert!(lines[lines.len() - 1].starts_with(LINE_PREFIX), "{lines:?}");
}

#[test]
fn a_failed_run_reports_and_keeps_its_exit_code() {
    let output = run("failing", &[]);
    let lines = stdout_lines(&output);
    assert_eq!(output.status.code(), Some(7), "{output:?}");
    assert_eq!(duration_lines(&lines), 1, "{lines:?}");
}

#[test]
fn a_recipe_with_its_own_exit_trap_reports_through_finish() {
    let output = run("own-trap", &[]);
    let lines = stdout_lines(&output);
    assert!(output.status.success(), "{output:?}");
    assert!(lines.iter().any(|line| line == "own-trap-cleanup"), "{lines:?}");
    assert_eq!(duration_lines(&lines), 1, "{lines:?}");
}

#[test]
fn durations_read_as_minutes_and_seconds_with_singular_units() {
    let output = run("long", &[]);
    let lines = stdout_lines(&output);
    // The seconds may drift on a loaded host, so only the minutes are exact.
    let last = &lines[lines.len() - 1];
    assert!(last.starts_with("🕖 this test run took 1 minute, "), "{lines:?}");
    assert!(last.ends_with(" second") || last.ends_with(" seconds"), "{lines:?}");
}

#[test]
fn a_hosted_run_prints_no_duration() {
    let output = run("chained", &[("GITHUB_ACTIONS", "true")]);
    let lines = stdout_lines(&output);
    assert!(output.status.success(), "{output:?}");
    assert_eq!(duration_lines(&lines), 0, "{lines:?}");
}
