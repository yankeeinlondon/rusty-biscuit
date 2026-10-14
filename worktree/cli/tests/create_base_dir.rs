//! `wt create` refuses an unusable base directory before touching the
//! repository, and names where the value came from.

mod base_dir_fixture;

use std::fs;

use base_dir_fixture::{FOREIGN, Fixture, assert_refused};

#[test]
fn the_fixture_home_config_is_the_one_wt_reads() {
    let fixture = Fixture::new();
    let base = fixture.dir("wts");
    fixture.write_config(base.to_str().unwrap());

    let output = fixture.create("feat/control", None);

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(base.join("repo").join("feat-control").is_dir());
    assert!(String::from_utf8_lossy(&output.stdout).starts_with("cd:"));
}

#[test]
fn a_foreign_saved_base_dir_is_refused_without_changing_anything() {
    let fixture = Fixture::new();
    let config = fixture.write_config(FOREIGN);
    let before = fixture.repo_state();

    let output = fixture.create("fix/wsl", None);

    let stderr = assert_refused(&output, &fixture, &before);
    assert!(stderr.contains(&format!("Base directory '{FOREIGN}' from the config file")), "{stderr}");
    assert!(stderr.contains(&fixture.config_path().display().to_string()), "{stderr}");
    assert!(stderr.contains("Set base_dir in"), "{stderr}");
    assert!(stderr.contains("Edit ~/.worktree.json"), "{stderr}");
    assert_eq!(fs::read(fixture.config_path()).unwrap(), config, "the config is never rewritten");
    assert!(!fixture.repo.join(FOREIGN).exists());
}

#[test]
fn an_invalid_wt_is_refused_even_when_the_config_is_valid() {
    let fixture = Fixture::new();
    let base = fixture.dir("wts");
    fixture.write_config(base.to_str().unwrap());
    let before = fixture.repo_state();

    for wt in [FOREIGN, "wt", "  "] {
        let output = fixture.create("feat/env", Some(wt));

        let stderr = assert_refused(&output, &fixture, &before);
        assert!(stderr.contains("from the WT environment variable"), "{wt:?}: {stderr}");
        assert!(stderr.contains("Editing ~/.worktree.json alone does not override a nonempty WT"), "{wt:?}: {stderr}");
        assert!(!stderr.contains("Edit ~/.worktree.json"), "{wt:?}: {stderr}");
        assert!(!base.join("repo").exists(), "{wt:?}: fell back to the config");
    }
}

#[test]
fn a_missing_saved_base_dir_names_the_config_and_the_mount() {
    let fixture = Fixture::new();
    let missing = fixture.root.path().join("unmounted").join("wt");
    let config = fixture.write_config(missing.to_str().unwrap());
    let before = fixture.repo_state();

    let output = fixture.create("feat/missing", None);

    let stderr = assert_refused(&output, &fixture, &before);
    assert!(stderr.contains("does not exist"), "{stderr}");
    assert!(stderr.contains("restore the mount"), "{stderr}");
    assert!(stderr.contains(&fixture.config_path().display().to_string()), "{stderr}");
    assert!(!missing.exists(), "the missing directory is never recreated");
    assert_eq!(fs::read(fixture.config_path()).unwrap(), config);
}
