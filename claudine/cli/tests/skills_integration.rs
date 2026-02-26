use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use assert_cmd::cargo::cargo_bin_cmd;
use predicates::str::contains;

struct TestWorkspace {
    root: PathBuf,
}

static TEST_NONCE: AtomicU64 = AtomicU64::new(0);

impl TestWorkspace {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let unique = TEST_NONCE.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "claudine-skills-it-{}-{nonce}-{unique}",
            process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        Self { root }
    }

    fn path(&self) -> &Path {
        &self.root
    }
}

impl Drop for TestWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn write(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

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

    cargo_bin_cmd!("claudine")
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

    cargo_bin_cmd!("claudine")
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
    cargo_bin_cmd!("claudine")
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

    cargo_bin_cmd!("claudine")
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
fn skills_lists_user_scoped_skill() {
    let workspace = TestWorkspace::new();
    let home_dir = workspace.path().join("home");
    let cwd = workspace.path().join("project");
    let skills_dir = home_dir.join(".claude/skills");
    fs::create_dir_all(&cwd).unwrap();
    setup_skill(&skills_dir, "my-tool", "A useful tool");

    cargo_bin_cmd!("claudine")
        .current_dir(&cwd)
        .env("HOME", &home_dir)
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

    cargo_bin_cmd!("claudine")
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

    cargo_bin_cmd!("claudine")
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

    cargo_bin_cmd!("claudine")
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
    let home_dir = workspace.path().join("home");
    let cwd = workspace.path().join("project");
    let skills_dir = home_dir.join(".claude/skills");
    fs::create_dir_all(&cwd).unwrap();
    setup_skill(&skills_dir, "my-tool", "A useful testing tool");

    cargo_bin_cmd!("claudine")
        .current_dir(&cwd)
        .env("HOME", &home_dir)
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

    cargo_bin_cmd!("claudine")
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

    cargo_bin_cmd!("claudine")
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
    let home_dir = workspace.path().join("home");
    let cwd = workspace.path().join("project");
    let skills_dir = home_dir.join(".claude/skills");
    fs::create_dir_all(&cwd).unwrap();
    setup_skill(&skills_dir, "fixable", "A fixable skill");

    cargo_bin_cmd!("claudine")
        .current_dir(&cwd)
        .env("HOME", &home_dir)
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

    let output = cargo_bin_cmd!("claudine")
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
    let home_dir = workspace.path().join("home");
    let cwd = workspace.path().join("project");
    let skills_dir = home_dir.join(".claude/skills");
    fs::create_dir_all(&cwd).unwrap();
    // Create a skill with an extra file so the tree is non-trivial
    setup_skill(&skills_dir, "my-tool", "A useful tool");
    fs::write(
        skills_dir.join("my-tool/details.md"),
        "# Extra detail file\n",
    )
    .unwrap();

    let output = cargo_bin_cmd!("claudine")
        .current_dir(&cwd)
        .env("HOME", &home_dir)
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

    let output = cargo_bin_cmd!("claudine")
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
    let home_dir = workspace.path().join("home");
    let cwd = workspace.path().join("project");
    let skills_dir = home_dir.join(".claude/skills");
    fs::create_dir_all(&cwd).unwrap();
    setup_skill(&skills_dir, "my-tool", "A tool");

    let output = cargo_bin_cmd!("claudine")
        .current_dir(&cwd)
        .env("HOME", &home_dir)
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

    let output = cargo_bin_cmd!("claudine")
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
fn skills_not_in_git_repo_shows_user_only_hint() {
    let workspace = TestWorkspace::new();
    let home_dir = workspace.path().join("home");
    // Use a temp dir that is definitely not a git repo
    let cwd = workspace.path().join("not-a-repo");
    let skills_dir = home_dir.join(".claude/skills");
    fs::create_dir_all(&cwd).unwrap();
    setup_skill(&skills_dir, "my-tool", "A tool");

    let output = cargo_bin_cmd!("claudine")
        .current_dir(&cwd)
        .env("HOME", &home_dir)
        .env("NO_COLOR", "1")
        .arg("skills")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("not") && stdout.contains("git"),
        "footer should indicate CWD is not a git repo, got:\n{stdout}"
    );
}
