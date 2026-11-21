//! The layout of the configuration Claudine's first run writes, through the
//! `claudine` binary: with no `~/.claudine/config.yaml`, a command that needs
//! configuration runs the headless setup (`init_wizard`), which creates the
//! file and moves the default `permissions` block into it. The fixture home
//! is outside any Git worktree, so the file takes the four-space default.
//! `claudine errors` is the command: it needs configuration and launches
//! nothing. The in-process matrix is `claudine::l1 config_indentation`.

use crate::common;
use common::CliProcessFixture;

fn run_errors(fixture: &CliProcessFixture) {
    let output = fixture
        .command()
        .env("COLUMNS", "200")
        .arg("errors")
        .write_stdin("")
        .output()
        .expect("claudine runs");
    assert!(output.status.success(), "claudine errors failed:\n{}", String::from_utf8_lossy(&output.stderr));
}

#[test]
fn the_first_run_writes_four_space_permissions_and_a_second_run_keeps_the_bytes() {
    let fixture = CliProcessFixture::named("first-run-layout");
    let path = fixture.home().join(".claudine/config.yaml");

    run_errors(&fixture);

    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("\npermissions:\n    file-read:\n        - ./*.txt\n"), "{text}");
    let written = simplified_schema::yaml_documents::load_yaml_document(&text, &path).unwrap().json;
    assert_eq!(written["permissions"], darkmatter::config::init::default_permissions(darkmatter::config::schema::ConfigScope::User));
    let reference = std::fs::read_to_string(fixture.home().join(".darkmatter.yaml")).unwrap();
    assert!(reference.contains("permissions: ~/.claudine/config.yaml"), "{reference}");

    run_errors(&fixture);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), text, "a configured run rewrites nothing");
}
