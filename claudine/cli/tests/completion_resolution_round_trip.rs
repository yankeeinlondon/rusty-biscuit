//! Real completion-to-composition resolution round trips.
//!
//! These Level 1 subprocess tests keep the producer and consumer honest: the
//! exact magic token emitted by `claudine __complete` is passed unchanged to a
//! second `claudine compose --dry-run` process. The composed body identifies
//! which ordered magic root execution selected.

use assert_cmd::cargo::cargo_bin_cmd;

mod common;
use common::completion::{
    fake_home, run_complete_with_home, seed_cargo_workspace_members, write_file,
};
use common::{TestWorkspace, init_git_repo};

fn emitted_magic_token(cwd: &std::path::Path, home: &std::path::Path, partial: &str) -> String {
    let candidates = run_complete_with_home(cwd, home, &["compose", partial]);
    assert_eq!(
        candidates.len(),
        1,
        "fixture should emit one unambiguous magic candidate: {candidates:?}",
    );
    candidates.into_iter().next().unwrap()
}

fn compose_dry_run(cwd: &std::path::Path, home: &std::path::Path, token: &str) -> String {
    let output = cargo_bin_cmd!("claudine")
        .env("HOME", home)
        .env("NO_COLOR", "1")
        .current_dir(cwd)
        .args(["compose", "--dry-run", token])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    String::from_utf8(output).expect("dry-run stdout is UTF-8")
}

#[test]
fn package_area_collision_round_trips_the_completion_value_unchanged() {
    let workspace = TestWorkspace::named("completion-resolution-area-collision");
    seed_cargo_workspace_members(workspace.path(), &["area/lib", "area/cli"]);
    assert!(init_git_repo(workspace.path()), "fixture requires a real git repository");
    let launch = workspace.path().join("area/lib");
    let home = fake_home(workspace.path());

    write_file(
        &workspace.path().join("area/prompts/plan.md"),
        "PACKAGE_AREA_MAGIC_ROOT\n",
    );
    write_file(
        &workspace.path().join("prompts/plan.md"),
        "REPOSITORY_MAGIC_ROOT\n",
    );

    let token = emitted_magic_token(&launch, &home, "@plan");
    assert_eq!(token, "@plan.md");
    let composed = compose_dry_run(&launch, &home, &token);
    assert!(
        composed.contains("PACKAGE_AREA_MAGIC_ROOT"),
        "runtime must select the package-area file completion inspected; stdout:\n{composed}",
    );
    assert!(
        !composed.contains("REPOSITORY_MAGIC_ROOT"),
        "the lower-priority repository collision must not win; stdout:\n{composed}",
    );
}

#[test]
fn discrete_package_only_prompt_round_trips_the_completion_value_unchanged() {
    let workspace = TestWorkspace::named("completion-resolution-discrete-package");
    seed_cargo_workspace_members(workspace.path(), &["tools/leaf", "area/lib"]);
    assert!(init_git_repo(workspace.path()), "fixture requires a real git repository");
    let launch = workspace.path().join("tools/leaf");
    let home = fake_home(workspace.path());

    write_file(
        &launch.join("prompts/package-only.md"),
        "DISCRETE_PACKAGE_MAGIC_ROOT\n",
    );

    let token = emitted_magic_token(&launch, &home, "@package-only");
    assert_eq!(token, "@package-only.md");
    let composed = compose_dry_run(&launch, &home, &token);
    assert!(
        composed.contains("DISCRETE_PACKAGE_MAGIC_ROOT"),
        "runtime must consume the discrete-package root completion used; stdout:\n{composed}",
    );
}
