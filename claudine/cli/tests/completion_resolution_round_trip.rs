//! Real completion-to-composition resolution round trips.
//!
//! These Level 1 subprocess tests keep the producer and consumer honest: the
//! exact magic token emitted by `claudine __complete` is passed unchanged to a
//! second `claudine compose --dry-run` process. The composed body identifies
//! which ordered magic root execution selected.


mod common;
use common::completion::{run_complete_with_home, seed_cargo_workspace_members, write_file};
use common::CliProcessFixture;

fn emitted_magic_token(cwd: &std::path::Path, home: &std::path::Path, partial: &str) -> String {
    let candidates = run_complete_with_home(cwd, home, &["compose", partial]);
    assert_eq!(
        candidates.len(),
        1,
        "fixture should emit one unambiguous magic candidate: {candidates:?}",
    );
    candidates.into_iter().next().unwrap()
}

fn compose_dry_run(fixture: &CliProcessFixture, launch: &std::path::Path, token: &str) -> String {
    let output = fixture
        .command_builder()
        // The launch directory decides which magic root wins, which is the
        // round trip's whole subject.
        .ambient_context(launch)
        .build()
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
    let fixture = CliProcessFixture::named("completion-resolution-area-collision");
    seed_cargo_workspace_members(fixture.cwd(), &["area/lib", "area/cli"]);
    fixture.initialize_repository();
    let launch = fixture.cwd().join("area/lib");
    let home = fixture.home().to_path_buf();

    write_file(
        &fixture.cwd().join("area/prompts/plan.md"),
        "PACKAGE_AREA_MAGIC_ROOT\n",
    );
    write_file(
        &fixture.cwd().join("prompts/plan.md"),
        "REPOSITORY_MAGIC_ROOT\n",
    );

    let token = emitted_magic_token(&launch, &home, "@plan");
    assert_eq!(token, "@plan.md");
    let composed = compose_dry_run(&fixture, &launch, &token);
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
    let fixture = CliProcessFixture::named("completion-resolution-discrete-package");
    seed_cargo_workspace_members(fixture.cwd(), &["tools/leaf", "area/lib"]);
    fixture.initialize_repository();
    let launch = fixture.cwd().join("tools/leaf");
    let home = fixture.home().to_path_buf();

    write_file(
        &launch.join("prompts/package-only.md"),
        "DISCRETE_PACKAGE_MAGIC_ROOT\n",
    );

    let token = emitted_magic_token(&launch, &home, "@package-only");
    assert_eq!(token, "@package-only.md");
    let composed = compose_dry_run(&fixture, &launch, &token);
    assert!(
        composed.contains("DISCRETE_PACKAGE_MAGIC_ROOT"),
        "runtime must consume the discrete-package root completion used; stdout:\n{composed}",
    );
}
