use std::fs;
use std::path::Path;

use predicates::str::contains;
mod common;
use common::{TestWorkspace, init_git_repo, write};

fn setup_skill(base: &Path, name: &str, description: &str) {
    write(
        &base.join(format!("{name}/SKILL.md")),
        &format!("---\ndescription: {description}\n---\n# {name}\nBody text.\n"),
    );
}

// ── Smoke tests ──────────────────────────────────────────────────────

#[test]
fn skills_subcommand_runs_without_panic() {
    let workspace = TestWorkspace::new();
    let home_dir = workspace.path().join("home");
    let cwd = workspace.path().join("project");
    fs::create_dir_all(&home_dir).unwrap();
    fs::create_dir_all(&cwd).unwrap();

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .current_dir(&cwd)
        .env("HOME", &home_dir)
        .env("NO_COLOR", "1")
        .arg("skills")
        .assert()
        .success();
}

#[test]
fn skills_with_verbose_flag_runs_without_panic() {
    let workspace = TestWorkspace::new();
    let home_dir = workspace.path().join("home");
    let cwd = workspace.path().join("project");
    fs::create_dir_all(&home_dir).unwrap();
    fs::create_dir_all(&cwd).unwrap();

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .current_dir(&cwd)
        .env("HOME", &home_dir)
        .env("NO_COLOR", "1")
        .args(["skills", "-v"])
        .assert()
        .success();
}

#[test]
fn skills_with_global_verbose_flag_runs_without_panic() {
    let workspace = TestWorkspace::new();
    let home_dir = workspace.path().join("home");
    let cwd = workspace.path().join("project");
    fs::create_dir_all(&home_dir).unwrap();
    fs::create_dir_all(&cwd).unwrap();

    // Global -v before the subcommand
    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .current_dir(&cwd)
        .env("HOME", &home_dir)
        .env("NO_COLOR", "1")
        .args(["-v", "skills"])
        .assert()
        .success();
}

// ── Empty state ──────────────────────────────────────────────────────

#[test]
fn skills_shows_no_skills_message_when_empty() {
    let workspace = TestWorkspace::new();
    let home_dir = workspace.path().join("home");
    let cwd = workspace.path().join("empty-project");
    fs::create_dir_all(&home_dir).unwrap();
    fs::create_dir_all(&cwd).unwrap();

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .current_dir(&cwd)
        .env("HOME", &home_dir)
        .env("NO_COLOR", "1")
        .arg("skills")
        .assert()
        .success()
        .stdout(contains("No skills found"));
}

// ── Listing skills ───────────────────────────────────────────────────

#[test]
fn skills_lists_repo_scoped_skill_from_nested_directory() {
    let workspace = TestWorkspace::new();
    let repo_root = workspace.path().join("repo");
    let cwd = repo_root.join("nested");
    let skills_dir = repo_root.join(".claude/skills");
    fs::create_dir_all(&repo_root).unwrap();
    assert!(init_git_repo(&repo_root));
    fs::create_dir_all(&cwd).unwrap();
    setup_skill(&skills_dir, "my-tool", "A useful tool");

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .current_dir(&cwd)
        .env("NO_COLOR", "1")
        .arg("skills")
        .assert()
        .success()
        .stdout(contains("my-tool"));
}

#[test]
fn skills_lists_repo_scoped_skill() {
    let workspace = TestWorkspace::new();
    let home_dir = workspace.path().join("home");
    let repo_root = workspace.path().join("repo");
    let skills_dir = repo_root.join(".claude/skills");
    fs::create_dir_all(&home_dir).unwrap();
    setup_skill(&skills_dir, "repo-skill", "Repo-scoped skill");

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .current_dir(&repo_root)
        .env("HOME", &home_dir)
        .env("NO_COLOR", "1")
        .arg("skills")
        .assert()
        .success()
        .stdout(contains("repo-skill"));
}

// ── Filtering ────────────────────────────────────────────────────────

#[test]
fn skills_filters_by_name() {
    let workspace = TestWorkspace::new();
    let home_dir = workspace.path().join("home");
    let cwd = workspace.path().join("project");
    let skills_dir = home_dir.join(".claude/skills");
    fs::create_dir_all(&cwd).unwrap();
    setup_skill(&skills_dir, "alpha", "Alpha skill");
    setup_skill(&skills_dir, "beta", "Beta skill");
    setup_skill(&skills_dir, "gamma", "Gamma skill");

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .current_dir(&cwd)
        .env("HOME", &home_dir)
        .env("NO_COLOR", "1")
        .args(["skills", "beta"])
        .assert()
        .success()
        .stdout(contains("beta"));
}

#[test]
fn skills_filter_no_match_shows_message() {
    let workspace = TestWorkspace::new();
    let home_dir = workspace.path().join("home");
    let cwd = workspace.path().join("project");
    let skills_dir = home_dir.join(".claude/skills");
    fs::create_dir_all(&cwd).unwrap();
    setup_skill(&skills_dir, "alpha", "Alpha skill");

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .current_dir(&cwd)
        .env("HOME", &home_dir)
        .env("NO_COLOR", "1")
        .args(["skills", "zzzznotfound"])
        .assert()
        .success()
        .stdout(contains("No skills matching"));
}

// ── Verbose output ───────────────────────────────────────────────────

#[test]
fn skills_verbose_shows_descriptions() {
    let workspace = TestWorkspace::new();
    let repo_root = workspace.path().join("repo");
    let cwd = repo_root.join("nested");
    let skills_dir = repo_root.join(".claude/skills");
    fs::create_dir_all(&repo_root).unwrap();
    assert!(init_git_repo(&repo_root));
    fs::create_dir_all(&cwd).unwrap();
    setup_skill(&skills_dir, "my-tool", "A useful testing tool");

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .current_dir(&cwd)
        .env("NO_COLOR", "1")
        .args(["-v", "skills"])
        .assert()
        .success()
        .stdout(contains("my-tool"))
        .stdout(contains("A useful testing tool"));
}

// ── Fix flag ─────────────────────────────────────────────────────────

#[test]
fn skills_fix_flag_accepted() {
    let workspace = TestWorkspace::new();
    let home_dir = workspace.path().join("home");
    let cwd = workspace.path().join("project");
    fs::create_dir_all(&home_dir).unwrap();
    fs::create_dir_all(&cwd).unwrap();

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .current_dir(&cwd)
        .env("HOME", &home_dir)
        .env("NO_COLOR", "1")
        .args(["skills", "--fix"])
        .assert()
        .success();
}

#[test]
fn skills_apply_flag_accepted() {
    let workspace = TestWorkspace::new();
    let home_dir = workspace.path().join("home");
    let cwd = workspace.path().join("project");
    fs::create_dir_all(&home_dir).unwrap();
    fs::create_dir_all(&cwd).unwrap();

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .current_dir(&cwd)
        .env("HOME", &home_dir)
        .env("NO_COLOR", "1")
        .args(["skills", "--apply"])
        .assert()
        .success();
}

#[test]
fn skills_fix_shows_summary() {
    let workspace = TestWorkspace::new();
    let repo_root = workspace.path().join("repo");
    let cwd = repo_root.join("nested");
    let skills_dir = repo_root.join(".claude/skills");
    fs::create_dir_all(&repo_root).unwrap();
    assert!(init_git_repo(&repo_root));
    fs::create_dir_all(&cwd).unwrap();
    write(
        &repo_root.join(".claudine/config.json"),
        r#"{ "canonical_provider": "claude" }"#,
    );
    setup_skill(&skills_dir, "fixable", "A fixable skill");

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .current_dir(&cwd)
        .env("NO_COLOR", "1")
        .args(["skills", "--fix"])
        .assert()
        .success()
        .stdout(contains("Fix Summary"));
}

#[test]
fn skills_fix_does_not_show_fix_hint() {
    let workspace = TestWorkspace::new();
    let home_dir = workspace.path().join("home");
    let cwd = workspace.path().join("project");
    let skills_dir = home_dir.join(".claude/skills");
    fs::create_dir_all(&cwd).unwrap();
    setup_skill(&skills_dir, "my-skill", "A skill");

    let output = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .current_dir(&cwd)
        .env("HOME", &home_dir)
        .env("NO_COLOR", "1")
        .args(["skills", "--fix"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    // After running --fix, should not suggest using --fix again
    assert!(
        !stdout.contains("use --fix"),
        "should not show --fix hint when already running with --fix"
    );
}

// ── Detail view ──────────────────────────────────────────────────────

#[test]
fn skills_detail_view_shows_filesystem() {
    let workspace = TestWorkspace::new();
    let repo_root = workspace.path().join("repo");
    let cwd = repo_root.join("nested");
    let skills_dir = repo_root.join(".claude/skills");
    fs::create_dir_all(&repo_root).unwrap();
    assert!(init_git_repo(&repo_root));
    fs::create_dir_all(&cwd).unwrap();
    // Create a skill with an extra file so the tree is non-trivial
    setup_skill(&skills_dir, "my-tool", "A useful tool");
    fs::write(
        skills_dir.join("my-tool/details.md"),
        "# Extra detail file\n",
    )
    .unwrap();

    let output = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .current_dir(&cwd)
        .env("NO_COLOR", "1")
        .args(["skills", "my-tool"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Detail view renders a FileSystem tree which includes "SKILL.md"
    assert!(
        stdout.contains("SKILL.md"),
        "detail view should show filesystem tree with SKILL.md, got:\n{stdout}"
    );
}

// ── Exception filtering ──────────────────────────────────────────────

#[test]
fn skills_filter_suppresses_unrelated_exceptions() {
    let workspace = TestWorkspace::new();
    let home_dir = workspace.path().join("home");
    let cwd = workspace.path().join("project");
    let skills_dir = home_dir.join(".claude/skills");
    fs::create_dir_all(&cwd).unwrap();
    setup_skill(&skills_dir, "alpha", "Alpha skill");
    // Create a skill with a broken link (exception source)
    write(
        &skills_dir.join("broken-tool/SKILL.md"),
        "---\ndescription: Broken tool\n---\nSee [missing](./gone.md) for more.\n",
    );

    let output = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .current_dir(&cwd)
        .env("HOME", &home_dir)
        .env("NO_COLOR", "1")
        .args(["skills", "alpha"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    // When filtering to "alpha", broken-tool exceptions should not appear
    assert!(
        !stdout.contains("broken-tool"),
        "filtered output should not show unrelated exceptions, got:\n{stdout}"
    );
}

// ── Footer messages ──────────────────────────────────────────────────

#[test]
fn skills_footer_shows_filter_hint() {
    let workspace = TestWorkspace::new();
    let repo_root = workspace.path().join("repo");
    let cwd = repo_root.join("nested");
    let skills_dir = repo_root.join(".claude/skills");
    fs::create_dir_all(&repo_root).unwrap();
    assert!(init_git_repo(&repo_root));
    fs::create_dir_all(&cwd).unwrap();
    setup_skill(&skills_dir, "my-tool", "A tool");

    let output = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .current_dir(&cwd)
        .env("NO_COLOR", "1")
        .arg("skills")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("filters"),
        "footer should show filter hint when no filters used, got:\n{stdout}"
    );
}

#[test]
fn skills_footer_hides_filter_hint_with_filter() {
    let workspace = TestWorkspace::new();
    let home_dir = workspace.path().join("home");
    let cwd = workspace.path().join("project");
    let skills_dir = home_dir.join(".claude/skills");
    fs::create_dir_all(&cwd).unwrap();
    setup_skill(&skills_dir, "my-tool", "A tool");

    let output = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .current_dir(&cwd)
        .env("HOME", &home_dir)
        .env("NO_COLOR", "1")
        .args(["skills", "my-tool"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("filters"),
        "footer should not show filter hint when filters are used, got:\n{stdout}"
    );
}

#[test]
fn skills_in_git_repo_does_not_show_user_only_hint() {
    let workspace = TestWorkspace::new();
    let repo_root = workspace.path().join("repo");
    let cwd = repo_root.join("nested");
    let skills_dir = repo_root.join(".claude/skills");
    fs::create_dir_all(&repo_root).unwrap();
    assert!(init_git_repo(&repo_root));
    fs::create_dir_all(&cwd).unwrap();
    setup_skill(&skills_dir, "my-tool", "A tool");

    let output = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .current_dir(&cwd)
        .env("NO_COLOR", "1")
        .arg("skills")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("current working directory is not a git repo"),
        "git-repo footer must not claim only user scope is available, got:\n{stdout}"
    );
}

// ── Negation filters ─────────────────────────────────────────────

#[test]
fn skills_negation_with_dash_prefix_excludes_match() {
    let workspace = TestWorkspace::new();
    let repo_root = workspace.path().join("repo");
    let cwd = repo_root.join("nested");
    let skills_dir = repo_root.join(".claude/skills");
    fs::create_dir_all(&repo_root).unwrap();
    assert!(init_git_repo(&repo_root));
    fs::create_dir_all(&cwd).unwrap();
    setup_skill(&skills_dir, "alpha", "Alpha skill");
    setup_skill(&skills_dir, "beta", "Beta skill");
    setup_skill(&skills_dir, "gamma", "Gamma skill");

    let output = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .current_dir(&cwd)
        .env("NO_COLOR", "1")
        .args(["skills", "--", "-beta"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("alpha"),
        "should show alpha, got:\n{stdout}"
    );
    assert!(
        !stdout.contains("beta"),
        "should NOT show beta, got:\n{stdout}"
    );
    assert!(
        stdout.contains("gamma"),
        "should show gamma, got:\n{stdout}"
    );
}

#[test]
fn skills_negation_with_bang_prefix_excludes_match() {
    let workspace = TestWorkspace::new();
    let repo_root = workspace.path().join("repo");
    let cwd = repo_root.join("nested");
    let skills_dir = repo_root.join(".claude/skills");
    fs::create_dir_all(&repo_root).unwrap();
    assert!(init_git_repo(&repo_root));
    fs::create_dir_all(&cwd).unwrap();
    setup_skill(&skills_dir, "alpha", "Alpha skill");
    setup_skill(&skills_dir, "beta", "Beta skill");
    setup_skill(&skills_dir, "gamma", "Gamma skill");

    let output = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .current_dir(&cwd)
        .env("NO_COLOR", "1")
        .args(["skills", "!beta"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("alpha"),
        "should show alpha, got:\n{stdout}"
    );
    assert!(
        !stdout.contains("beta"),
        "should NOT show beta, got:\n{stdout}"
    );
    assert!(
        stdout.contains("gamma"),
        "should show gamma, got:\n{stdout}"
    );
}

// ── Exact filters ────────────────────────────────────────────────

#[test]
fn skills_exact_filter_matches_only_full_name() {
    let workspace = TestWorkspace::new();
    let home_dir = workspace.path().join("home");
    let cwd = workspace.path().join("project");
    let skills_dir = home_dir.join(".claude/skills");
    fs::create_dir_all(&cwd).unwrap();
    setup_skill(&skills_dir, "alpha", "Alpha skill");
    setup_skill(&skills_dir, "alpha-extended", "Alpha extended skill");

    let output = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .current_dir(&cwd)
        .env("HOME", &home_dir)
        .env("NO_COLOR", "1")
        .args(["skills", "alpha!"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("alpha"),
        "should show alpha, got:\n{stdout}"
    );
    assert!(
        !stdout.contains("alpha-extended"),
        "should NOT show alpha-extended with exact filter, got:\n{stdout}"
    );
}

// ── Combined positive + negation ─────────────────────────────────

#[test]
fn skills_combined_positive_and_negation() {
    let workspace = TestWorkspace::new();
    let repo_root = workspace.path().join("repo");
    let cwd = repo_root.join("nested");
    let skills_dir = repo_root.join(".claude/skills");
    fs::create_dir_all(&repo_root).unwrap();
    assert!(init_git_repo(&repo_root));
    fs::create_dir_all(&cwd).unwrap();
    setup_skill(&skills_dir, "alpha", "Alpha skill");
    setup_skill(&skills_dir, "beta", "Beta skill");
    setup_skill(&skills_dir, "gamma", "Gamma skill");

    // "a" matches alpha, beta, and gamma; "-alpha" excludes alpha
    let output = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .current_dir(&cwd)
        .env("NO_COLOR", "1")
        .args(["skills", "--", "a", "-alpha"])
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("gamma"),
        "should show gamma (contains 'a'), got:\n{stdout}"
    );
    assert!(
        stdout.contains("beta"),
        "should show beta (contains 'a'), got:\n{stdout}"
    );
    // "alpha" is excluded by negation despite matching positive "a"
    assert!(
        !stdout.contains("alpha"),
        "should NOT show alpha (negated), got:\n{stdout}"
    );
}
