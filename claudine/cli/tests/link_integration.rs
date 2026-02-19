use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

use assert_cmd::cargo::cargo_bin_cmd;
use predicates::str::contains;

struct TestWorkspace {
    root: PathBuf,
}

impl TestWorkspace {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("claudine-link-it-{}-{nonce}", process::id()));
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

#[cfg(unix)]
#[test]
fn link_repo_apply_shows_sections_and_creates_repo_symlink() {
    let workspace = TestWorkspace::new();
    let home_dir = workspace.path().join("home");
    let repo_root = workspace.path().join("repo");
    fs::create_dir_all(&home_dir).unwrap();
    fs::create_dir_all(&repo_root).unwrap();

    // Repo config with repo-scope canonical providers for linking analysis.
    let config = serde_json::json!({
        "version": "1.0",
        "settings": {
            "linking": {
                "canonical_provider": {
                    "repo_skill": "claude",
                    "repo_command": "claude",
                    "repo_agent": "claude",
                    "repo_script": "codex"
                }
            }
        },
        "providers": {}
    });
    write(
        &repo_root.join(".claudine/config.json"),
        &serde_json::to_string_pretty(&config).unwrap(),
    );

    // Canonical repo-scope skill from Claude that should be linked to Codex during --apply.
    write(
        &repo_root.join(".claude/skills/repo-skill/SKILL.md"),
        "---\ndescription: Repo-scoped skill\n---\n# Repo Skill\n",
    );

    cargo_bin_cmd!("claudine")
        .current_dir(&repo_root)
        .env("HOME", &home_dir)
        .env("NO_COLOR", "1")
        .args(["link", "--scope", "repo", "--apply"])
        .assert()
        .success()
        .stdout(contains("Link Strategy Analysis"))
        .stdout(contains("scope: repo, mode: apply"))
        .stdout(contains("Skills"))
        .stdout(contains("Apply Totals"));

    let codex_skill_link = repo_root.join(".codex/skills/repo-skill");
    let metadata = fs::symlink_metadata(&codex_skill_link).unwrap();
    assert!(metadata.file_type().is_symlink());
    let link_target = fs::read_link(&codex_skill_link).unwrap();
    assert!(link_target.is_relative());
}
