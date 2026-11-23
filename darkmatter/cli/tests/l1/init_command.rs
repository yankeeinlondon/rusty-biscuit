//! `md init` through the real binary: the formatted report, a repository
//! request outside a repository, an existing file's `permissions` reference
//! and `register` list, and the temporary legacy-file migration any `md`
//! command performs. The setup rules themselves are proven in-process
//! (`darkmatter/lib/tests/l1/config_init.rs`).

use crate::common;

use common::CliProcessFixture;
use serde_json::Value;
use std::process::Output;

fn run(fixture: &CliProcessFixture, args: &[&str]) -> Output {
    fixture.command().args(args).output().expect("md runs")
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// Every file under the fixture's home and launch directory.
fn files(fixture: &CliProcessFixture) -> Vec<std::path::PathBuf> {
    let mut found = Vec::new();
    for dir in [fixture.home(), fixture.cwd()] {
        for entry in std::fs::read_dir(dir).expect("a fixture directory") {
            found.push(entry.expect("an entry").path());
        }
    }
    found.sort();
    found
}

#[test]
fn init_creates_the_user_file_and_reports_it_without_a_missing_warning() {
    let fixture = CliProcessFixture::named("init-formatted").without_user_config();
    let path = fixture.home().join(".darkmatter.yaml");

    let output = run(&fixture, &["init"]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr_of(&output));
    assert!(output.stderr.is_empty(), "no `dm.config.missing` for the file init creates: {}", stderr_of(&output));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Created") && stdout.contains(&path.display().to_string()), "{stdout}");
    let text = std::fs::read_to_string(&path).expect("the user file");
    assert!(text.contains("permissions:") && text.contains("network: deny") && text.contains("register: []"), "{text}");

    let again = run(&fixture, &["init"]);
    assert_eq!(again.status.code(), Some(0), "{}", stderr_of(&again));
    assert!(String::from_utf8_lossy(&again.stdout).contains("Unchanged"));
    assert_eq!(std::fs::read_to_string(&path).expect("the user file"), text, "a second run writes nothing");
}

#[test]
fn init_repo_outside_a_repository_fails_before_writing() {
    let fixture = CliProcessFixture::named("init-repo-outside").without_user_config();
    let before = files(&fixture);

    let output = run(&fixture, &["init", "--repo"]);

    assert_eq!(output.status.code(), Some(1));
    assert!(stderr_of(&output).contains("not inside a repository"), "{}", stderr_of(&output));
    assert_eq!(files(&fixture), before, "zero writes, the user file included");
}

#[test]
fn a_legacy_shell_whitelist_is_migrated_by_any_md_command() {
    let fixture = CliProcessFixture::named("init-legacy-migration").without_user_config();
    let legacy = fixture.write_file("home/.darkmatter-shell-whitelist", "prefix ls\n");

    let output = run(&fixture, &["list-themes", "--output", "json"]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr_of(&output));
    assert!(output.stderr.is_empty(), "a data output owns stderr too: {}", stderr_of(&output));
    let value: Value = serde_json::from_slice(&output.stdout).expect("one JSON value");
    let codes: Vec<&str> =
        value["diagnostics"].as_array().expect("diagnostics").iter().filter_map(|record| record["code"].as_str()).collect();
    assert!(codes.contains(&"dm.config.migration"), "{value:#}");
    assert!(!codes.contains(&"dm.config.missing"), "the migration wrote the user file: {value:#}");
    assert!(!legacy.exists(), "the legacy file is removed once migrated");
    let text = std::fs::read_to_string(fixture.home().join(".darkmatter.yaml")).expect("the user file");
    assert!(text.contains("prefix ls"), "{text}");
}

#[test]
fn init_on_an_existing_file_keeps_a_claudine_permissions_reference_and_the_register_list() {
    let fixture = CliProcessFixture::named("init-keeps-reference").without_user_config();
    fixture.write_file("home/.claudine/config.yaml", "permissions:\n    network: deny\n");
    let original = "permissions: ~/.claudine/config.yaml # Claudine's\nregister:\n    - ~/.claudine/registrations/darkmatter/manifest.yaml\n    # mine\n    - ~/hosts/mine.yaml\n";
    let path = fixture.write_file("home/.darkmatter.yaml", original);

    let output = run(&fixture, &["init"]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr_of(&output));
    let text = std::fs::read_to_string(&path).expect("the user file");
    assert!(text.starts_with(original), "the reference, entries, and comments stay:\n{text}");
}
