//! Repository identity remains available without requesting Git-group variables.

mod common;

use common::{CliProcessFixture, strip_ansi, write};

#[test]
fn external_prompt_dry_run_preserves_repository_only_context() {
    let fixture = CliProcessFixture::named("compose-repository-context");
    fixture.initialize_repository();
    assert!(std::process::Command::new("git")
        .arg("-C")
        .arg(fixture.cwd())
        .args(["remote", "add", "origin", "https://example.com/team/fixture-repo.git"])
        .status()
        .unwrap()
        .success());
    let prompt = fixture.home().join("commit-probe.md");
    write(
        &prompt,
        "---\nsuccess:\n  message: 'git commits in the **{{ctx.repo}}** repo has completed'\n---\nRepository=[{{ctx.repo}}]\n",
    );
    let output = fixture.command()
        .args(["compose", "--dry-run"])
        .arg(&prompt)
        .assert()
        .success()
        .get_output()
        .clone();
    let stdout = strip_ansi(&String::from_utf8_lossy(&output.stdout));
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr));
    assert!(stdout.contains("Repository=[fixture-repo]"), "{stdout}");
    assert!(!stderr.contains("required capture evidence was not supplied"), "{stderr}");
}
