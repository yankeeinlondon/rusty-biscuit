mod common;

use common::md_cmd;
use predicates::prelude::*;

#[test]
fn test_compose_with_whitelisted_command_succeeds() {
    let temp_dir = tempfile::TempDir::new().unwrap();

    // Write a markdown file with a shell directive
    let md_path = temp_dir.path().join("test.md");
    std::fs::write(&md_path, "# Test\n::shell echo hello\n").unwrap();

    // Write a whitelist
    let whitelist_path = temp_dir.path().join(".darkmatter-shell-whitelist");
    std::fs::write(&whitelist_path, "prefix echo\n").unwrap();

    md_cmd()
        .arg("compose")
        .arg(&md_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("hello"));
}

#[test]
fn test_compose_with_blacklisted_command_fails() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let md_path = temp_dir.path().join("test.md");
    std::fs::write(&md_path, "# Test\n::shell rm -rf /\n").unwrap();

    md_cmd()
        .arg("compose")
        .arg(&md_path)
        .assert()
        .failure()
        .stderr(predicate::str::contains("Blacklisted").or(predicate::str::contains("dangerous")));
}

#[test]
fn test_compose_stdin_unapproved_command_fails_with_guidance() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    #[cfg(not(windows))]
    let whitelist_path = temp_dir.path().join(".darkmatter-shell-whitelist");
    #[cfg(windows)]
    let whitelist_path = std::path::PathBuf::from(
        std::env::var_os("USERPROFILE").expect("Windows user profile is available"),
    )
    .join(".darkmatter-shell-whitelist");

    md_cmd()
        .current_dir(temp_dir.path())
        .env("HOME", temp_dir.path())
        .arg("compose")
        .arg("-")
        .write_stdin("# Test\n::shell echo hello\n")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Approval required for 'echo hello'.",
        ))
        .stderr(predicate::str::contains(
            "To allow in non-interactive mode, add one of these to",
        ))
        .stderr(predicate::str::contains(
            whitelist_path.display().to_string(),
        ))
        .stderr(predicate::str::contains("exact echo hello"))
        .stderr(predicate::str::contains("prefix echo"));
}

#[test]
fn test_compose_file_unapproved_command_fails_with_guidance() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let md_path = temp_dir.path().join("test.md");
    let whitelist_path = temp_dir.path().join(".darkmatter-shell-whitelist");

    std::fs::write(&md_path, "# Test\n::shell echo hello\n").unwrap();

    md_cmd()
        .current_dir(temp_dir.path())
        .arg("compose")
        .arg(&md_path)
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Approval required for 'echo hello'.",
        ))
        .stderr(predicate::str::contains(
            "To allow in non-interactive mode, add one of these to",
        ))
        .stderr(predicate::str::contains(
            whitelist_path.display().to_string(),
        ))
        .stderr(predicate::str::contains("exact echo hello"))
        .stderr(predicate::str::contains("prefix echo"));
}

#[test]
fn test_compose_with_nonexistent_command_fails() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let md_path = temp_dir.path().join("test.md");
    std::fs::write(&md_path, "# Test\n::shell nonexistent_command_xyz\n").unwrap();

    let whitelist_path = temp_dir.path().join(".darkmatter-shell-whitelist");
    std::fs::write(&whitelist_path, "prefix nonexistent_command_xyz\n").unwrap();
    // Missing executables exhaust PATH. Isolate lookup so WSL does not probe
    // Windows-backed mounts whose latency is unrelated to this CLI contract.
    let isolated_path = std::env::join_paths([temp_dir.path()]).unwrap();

    md_cmd()
        .env("PATH", isolated_path)
        .arg("compose")
        .arg(&md_path)
        .assert()
        .failure()
        .stderr(predicate::str::contains("Command not found"));
}

#[test]
fn test_compose_timeout_flag_fails_timed_out_shell() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let md_path = temp_dir.path().join("test.md");
    let whitelist_path = temp_dir.path().join(".darkmatter-shell-whitelist");

    std::fs::write(&md_path, "# Test\n::shell sleep 2\n").unwrap();
    std::fs::write(&whitelist_path, "prefix sleep\n").unwrap();

    md_cmd()
        .arg("compose")
        .arg(&md_path)
        .arg("--timeout")
        .arg("1")
        .assert()
        .failure()
        .stderr(predicate::str::contains("timed out"));
}

#[test]
fn test_compose_allow_shell_timeout_emits_warning() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let md_path = temp_dir.path().join("test.md");
    let whitelist_path = temp_dir.path().join(".darkmatter-shell-whitelist");

    std::fs::write(&md_path, "# Test\n::shell sleep 2\nAfter\n").unwrap();
    std::fs::write(&whitelist_path, "prefix sleep\n").unwrap();

    md_cmd()
        .arg("compose")
        .arg(&md_path)
        .arg("--timeout")
        .arg("1")
        .arg("--allow-shell-timeout")
        .assert()
        .success()
        .stdout(predicate::str::contains("# Test"))
        .stdout(predicate::str::contains("After"))
        .stderr(predicate::str::contains("timed out"))
        .stderr(predicate::str::contains("replaced with an empty"));
}

#[test]
fn test_compose_shell_reports_discovered_commands_without_executing() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let root_path = temp_dir.path().join("root.md");
    let child_path = temp_dir.path().join("child.md");

    std::fs::write(
        &root_path,
        "---\nroot_cmd: \"$(echo root-frontmatter)\"\n---\n# Root\n::shell echo root-body\n::file ./child.md\n",
    )
    .unwrap();
    std::fs::write(
        &child_path,
        "---\nchild_cmd: \"$(echo child-frontmatter)\"\n---\n# Child\n::shell echo child-body\n",
    )
    .unwrap();

    md_cmd()
        .arg("compose")
        .arg(&root_path)
        .arg("--shell")
        .assert()
        .success()
        .stdout(predicate::str::contains("Shell commands discovered: 4"))
        .stdout(predicate::str::contains("echo root-frontmatter"))
        .stdout(predicate::str::contains("frontmatter.root_cmd"))
        .stdout(predicate::str::contains("echo root-body"))
        .stdout(predicate::str::contains("echo child-frontmatter"))
        .stdout(predicate::str::contains("frontmatter.child_cmd"))
        .stdout(predicate::str::contains("echo child-body"))
        .stdout(predicate::str::contains("root-frontmatter\n").not())
        .stdout(predicate::str::contains("child-frontmatter\n").not());
}
