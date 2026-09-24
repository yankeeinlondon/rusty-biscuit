//! `@` prompt references launched from a directory nested under `$HOME`.
//!
//! Mirrors running `claudine compose @prompts/commit.md` from `~/config/sh`:
//! the launch directory is below the user's home, is a plain Git repository
//! or no repository at all, and has no Cargo workspace. The user prompt tier
//! `~/.claudine/prompts/` must answer both the path-shaped `@prompts/<x>`
//! form and the concise `@<x>` form, while a closer repository prompt wins.

use std::path::PathBuf;

use crate::common;

use common::{CliProcessFixture, init_git_repo, strip_ansi, write};

/// Stage `$HOME/config/sh` as the launch directory and a user-tier prompt.
fn stage(prefix: &str, repository: bool) -> (CliProcessFixture, PathBuf) {
    let fixture = CliProcessFixture::named(prefix);
    let launch = fixture.home().join("config").join("sh");
    std::fs::create_dir_all(&launch).unwrap();
    if repository {
        assert!(init_git_repo(&launch), "git init failed at {}", launch.display());
    }
    write(
        &fixture.home().join(".claudine").join("prompts").join("probe.md"),
        "Tier=[user]\n",
    );
    (fixture, launch)
}

fn compose_dry_run(fixture: &CliProcessFixture, launch: &std::path::Path, reference: &str) -> String {
    let output = fixture
        .command_builder()
        .ambient_context(launch)
        .build()
        .args(["compose", "--dry-run", reference])
        .assert()
        .success()
        .get_output()
        .clone();
    strip_ansi(&String::from_utf8_lossy(&output.stdout))
}

#[test]
fn path_shaped_reference_reaches_user_tier_from_plain_repository_under_home() {
    let (fixture, launch) = stage("prompt-tiers-repo-path", true);
    let stdout = compose_dry_run(&fixture, &launch, "@prompts/probe.md");
    assert!(stdout.contains("Tier=[user]"), "{stdout}");
}

#[test]
fn concise_reference_reaches_user_tier_from_plain_repository_under_home() {
    let (fixture, launch) = stage("prompt-tiers-repo-concise", true);
    let stdout = compose_dry_run(&fixture, &launch, "@probe.md");
    assert!(stdout.contains("Tier=[user]"), "{stdout}");
}

#[test]
fn both_reference_forms_reach_user_tier_outside_any_repository() {
    let (fixture, launch) = stage("prompt-tiers-no-repo", false);
    for reference in ["@prompts/probe.md", "@probe.md"] {
        let stdout = compose_dry_run(&fixture, &launch, reference);
        assert!(stdout.contains("Tier=[user]"), "{reference}: {stdout}");
    }
}

#[test]
fn repository_prompt_wins_over_user_tier_for_both_forms() {
    let (fixture, launch) = stage("prompt-tiers-local-wins", true);
    write(&launch.join("prompts").join("probe.md"), "Tier=[repo]\n");
    write(
        &launch.join(".claudine").join("prompts").join("probe.md"),
        "Tier=[repo-claudine]\n",
    );
    for reference in ["@prompts/probe.md", "@probe.md"] {
        let stdout = compose_dry_run(&fixture, &launch, reference);
        assert!(stdout.contains("Tier=[repo]"), "{reference}: {stdout}");
    }
}

#[test]
fn repository_claudine_tier_wins_over_user_tier_for_path_shaped_form() {
    let (fixture, launch) = stage("prompt-tiers-repo-claudine", true);
    write(
        &launch.join(".claudine").join("prompts").join("probe.md"),
        "Tier=[repo-claudine]\n",
    );
    let stdout = compose_dry_run(&fixture, &launch, "@prompts/probe.md");
    assert!(stdout.contains("Tier=[repo-claudine]"), "{stdout}");
}
