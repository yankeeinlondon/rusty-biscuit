//! `wt create`'s config-file errors and its noninteractive refusal to run
//! setup: each is rendered once and exits 1.

mod base_dir_fixture;

use std::fs;

use base_dir_fixture::{Fixture, assert_refused};

#[test]
fn a_malformed_config_exits_1_after_one_rendering() {
    let fixture = Fixture::new();
    fs::write(fixture.config_path(), b"{ \"base_dir\": ").unwrap();
    let before = fixture.repo_state();

    let output = fixture.create("feat/bad", None);

    let stderr = assert_refused(&output, &fixture, &before);
    assert_eq!(stderr.matches("has an invalid format").count(), 1, "{stderr}");
    assert_eq!(stderr.matches("Edit ~/.worktree.json").count(), 1, "{stderr}");
    assert!(stderr.contains("in an interactive terminal"), "{stderr}");
    assert_eq!(fs::read(fixture.config_path()).unwrap(), b"{ \"base_dir\": ");
}

#[test]
fn a_config_naming_a_repository_root_exits_1_after_one_rendering() {
    let fixture = Fixture::new();
    let repo_root = fixture.dir("other-repo");
    fs::create_dir(repo_root.join(".git")).unwrap();
    fixture.write_config(repo_root.to_str().unwrap());
    let before = fixture.repo_state();

    let output = fixture.create("feat/root", None);

    let stderr = assert_refused(&output, &fixture, &before);
    assert_eq!(stderr.matches("is the root of a Git repository").count(), 1, "{stderr}");
    assert_eq!(stderr.matches("Edit ~/.worktree.json").count(), 1, "{stderr}");
    assert!(!repo_root.join("repo").exists());
}

#[test]
fn markup_in_a_saved_value_is_shown_literally() {
    let fixture = Fixture::new();
    let value = "<red>w_t</red>";
    fixture.write_config(value);
    let before = fixture.repo_state();

    let output = fixture.create("feat/markup", None);

    let stderr = assert_refused(&output, &fixture, &before);
    assert!(stderr.contains(&format!("Base directory '{value}' from the config file")), "{stderr}");
}

/// A home path with quotes and angle brackets would break an unquoted link
/// attribute and leak the tag into the message.
#[cfg(unix)]
#[test]
fn the_config_link_survives_a_home_path_with_markup_characters() {
    let mut fixture = Fixture::new();
    fixture.home = fixture.dir("ho\"me'<x>");
    fixture.write_config("wt");
    let before = fixture.repo_state();

    let output = fixture.create("feat/link", None);

    let stderr = assert_refused(&output, &fixture, &before);
    assert!(stderr.contains("Edit ~/.worktree.json"), "{stderr}");
    assert!(!stderr.contains("<a") && !stderr.contains("</a>"), "{stderr}");
}

#[test]
fn no_configuration_and_no_terminal_exits_1_without_setup() {
    let fixture = Fixture::new();
    let before = fixture.repo_state();

    let output = fixture.create("feat/none", None);

    let stderr = assert_refused(&output, &fixture, &before);
    assert!(stderr.contains("No worktree base directory is configured"), "{stderr}");
    assert!(stderr.contains("Set the WT environment variable"), "{stderr}");
    assert!(!stderr.contains("Choose a base directory"), "{stderr}");
    assert!(!stderr.contains("Let's set one up"), "{stderr}");
    assert!(!fixture.config_path().exists());
}
