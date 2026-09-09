mod common;

use common::CliProcessFixture;
use predicates::prelude::*;

fn rendering_command(fixture: &CliProcessFixture) -> assert_cmd::Command {
    let mut command = fixture.command();
    command
        .env("COLUMNS", "80")
        .env("LINES", "24")
        .env("TERM", "dumb")
        .env_remove("COLORTERM")
        .env("NO_COLOR", "1")
        .env_remove("FORCE_COLOR");
    command
}

#[test]
fn test_compose_compact() {
    let fixture = CliProcessFixture::named("test_compose_compact");
    let input = "---\n---\n\n- item 1\n\n- item 2\n\n- item 3";
    rendering_command(&fixture)
        .args(["compose", "--compact", "-"])
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("- item 1\n- item 2\n- item 3"));
}

#[test]
fn test_compose_loose() {
    let fixture = CliProcessFixture::named("test_compose_loose");
    let input = "---\n---\n\n- item 1\n- item 2\n- item 3";
    rendering_command(&fixture)
        .args(["compose", "--loose", "-"])
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("- item 1\n\n- item 2\n\n- item 3"));
}
