mod common;

use common::CliProcessFixture;
use predicates::prelude::*;

#[test]
fn test_compose_perf_emits_report_to_stderr() {
    let fixture = CliProcessFixture::named("test_compose_perf_emits_report_to_stderr");
    fixture
        .command()
        .args(["compose", "-", "--perf"])
        .write_stdin("# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello"))
        .stdout(predicate::str::contains("World"))
        .stderr(predicate::str::contains("Command Setup"))
        .stderr(predicate::str::contains("Compose Pipeline"))
        .stderr(predicate::str::contains("elapsed"));
}

#[test]
fn test_compose_without_perf_no_report_on_stderr() {
    let fixture = CliProcessFixture::named("test_compose_without_perf_no_report_on_stderr");
    fixture
        .command()
        .args(["compose", "-"])
        .write_stdin("# Hello\n\nWorld")
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello"))
        .stderr(predicate::str::contains("Command Setup").not());
}
