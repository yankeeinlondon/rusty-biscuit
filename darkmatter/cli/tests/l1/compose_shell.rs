use crate::common;

use common::CliProcessFixture;
use predicates::prelude::*;

#[test]
fn test_compose_with_a_home_allowed_command_succeeds() {
    let fixture = CliProcessFixture::named("test_compose_with_a_home_allowed_command_succeeds");
    let md_path = fixture.write_file("cwd/test.md", "# Test\n::shell echo hello\n");
    fixture.shell_permissions(&["prefix echo"]);

    // `echo` shell expansion is the behavior under test, so expose the host shell.
    fixture
        .command_builder()
        .host_path()
        .build()
        .arg("compose")
        .arg(&md_path)
        .assert()
        .success()
        .stdout(predicate::str::contains("hello"));
}

/// An exclusion in the home configuration makes an otherwise allowed command
/// ask; with no terminal to ask on, the run fails naming the exclusion and
/// runs nothing.
#[test]
fn test_compose_with_an_excluded_command_requires_approval() {
    let fixture = CliProcessFixture::named("test_compose_with_an_excluded_command_requires_approval");
    let md_path = fixture.write_file("cwd/test.md", "# Test\n::shell echo secret\n");
    let config = fixture.shell_permissions(&["prefix echo", "!exact echo secret"]);
    let before = std::fs::read(&config).unwrap();

    fixture
        .command_builder()
        .host_path()
        .build()
        .arg("compose")
        .arg(&md_path)
        .assert()
        .failure()
        .stderr(predicate::str::contains("Approval required for 'echo secret'"))
        .stderr(predicate::str::contains("excluded by '!exact echo secret'"))
        .stdout(predicate::str::contains("secret").not());
    assert_eq!(std::fs::read(&config).unwrap(), before, "a run never edits the exclusion");
}

#[test]
fn test_compose_with_blacklisted_command_fails() {
    let fixture = CliProcessFixture::named("test_compose_with_blacklisted_command_fails");
    let md_path = fixture.write_file("cwd/test.md", "# Test\n::shell rm -rf /\n");

    fixture
        .command()
        .arg("compose")
        .arg(&md_path)
        .assert()
        .failure()
        .stderr(predicate::str::contains("Blacklisted").or(predicate::str::contains("dangerous")));
}

#[test]
fn test_compose_stdin_unapproved_command_fails_with_guidance() {
    let fixture =
        CliProcessFixture::named("test_compose_stdin_unapproved_command_fails_with_guidance");
    // A rule that approves the command goes in the user's configuration,
    // for a stdin document as for a file.
    let config_path = fixture.home().join(".darkmatter.yaml");

    fixture
        .command()
        .arg("compose")
        .arg("-")
        .write_stdin("# Test\n::shell echo hello\n")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Approval required for 'echo hello'.",
        ))
        .stderr(predicate::str::contains(
            "To allow in non-interactive mode, add one of these to `permissions.shell-commands` in",
        ))
        .stderr(predicate::str::contains(
            config_path.display().to_string(),
        ))
        .stderr(predicate::str::contains("exact echo hello"))
        .stderr(predicate::str::contains("prefix echo"));
}

#[test]
fn test_compose_file_unapproved_command_fails_with_guidance() {
    let fixture =
        CliProcessFixture::named("test_compose_file_unapproved_command_fails_with_guidance");
    let md_path = fixture.write_file("cwd/test.md", "# Test\n::shell echo hello\n");
    let config_path = fixture.home().join(".darkmatter.yaml");

    fixture
        .command()
        .arg("compose")
        .arg(&md_path)
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Approval required for 'echo hello'.",
        ))
        .stderr(predicate::str::contains(
            "To allow in non-interactive mode, add one of these to `permissions.shell-commands` in",
        ))
        .stderr(predicate::str::contains(
            config_path.display().to_string(),
        ))
        .stderr(predicate::str::contains("exact echo hello"))
        .stderr(predicate::str::contains("prefix echo"));
}

#[test]
fn test_compose_with_nonexistent_command_fails() {
    let fixture = CliProcessFixture::named("test_compose_with_nonexistent_command_fails");
    let md_path = fixture.write_file("cwd/test.md", "# Test\n::shell nonexistent_command_xyz\n");
    fixture.shell_permissions(&["prefix nonexistent_command_xyz"]);
    // Missing executables exhaust PATH. Isolate lookup so WSL does not probe
    // Windows-backed mounts whose latency is unrelated to this CLI contract.
    // Command absence is the assertion, so restrict lookup to the empty fixture bin.
    fixture
        .command_builder()
        .fake_only_path()
        .build()
        .arg("compose")
        .arg(&md_path)
        .assert()
        .failure()
        .stderr(predicate::str::contains("Command not found"));
}

#[test]
fn test_compose_timeout_flag_fails_timed_out_shell() {
    let fixture = CliProcessFixture::named("test_compose_timeout_flag_fails_timed_out_shell");
    let md_path = fixture.write_file("cwd/test.md", "# Test\n::shell sleep 2\n");
    fixture.shell_permissions(&["prefix sleep"]);

    // A real `sleep` shell process is required to exercise timeout handling.
    fixture
        .command_builder()
        .host_path()
        .build()
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
    let fixture = CliProcessFixture::named("test_compose_allow_shell_timeout_emits_warning");
    let md_path = fixture.write_file("cwd/test.md", "# Test\n::shell sleep 2\nAfter\n");
    fixture.shell_permissions(&["prefix sleep"]);

    // A real `sleep` shell process is required to exercise timeout recovery.
    fixture
        .command_builder()
        .host_path()
        .build()
        .arg("compose")
        .arg(&md_path)
        .arg("--timeout")
        .arg("1")
        .args(["--allow", "shell-timeout"])
        .assert()
        .success()
        .stdout(predicate::str::contains("# Test"))
        .stdout(predicate::str::contains("After"))
        .stderr(predicate::str::contains("timed out"))
        .stderr(predicate::str::contains("replaced with an empty"));
}

#[test]
fn test_compose_shell_reports_discovered_commands_without_executing() {
    let fixture = CliProcessFixture::named(
        "test_compose_shell_reports_discovered_commands_without_executing",
    );
    let root_path = fixture.write_file(
        "cwd/root.md",
        "---\nroot_cmd: \"$(echo root-frontmatter)\"\n---\n# Root\n::shell echo root-body\n::file ./child.md\n",
    );
    fixture.write_file(
        "cwd/child.md",
        "---\nchild_cmd: \"$(echo child-frontmatter)\"\n---\n# Child\n::shell echo child-body\n",
    );

    fixture
        .command()
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

/// A shell command never runs with a missing value: `md compose` fails
/// before running anything, naming the command and the value, on each
/// Darkmatter shell surface. The `git init` marker proves no command ran.
#[test]
fn test_compose_refuses_a_missing_shell_value_before_running_anything() {
    let fixture = CliProcessFixture::named("test_compose_refuses_a_missing_shell_value");
    fixture.shell_permissions(&["prefix git", "prefix ls"]);
    for (name, body, command) in [
        ("body.md", "---\ntarget: null\n---\n::shell git init --quiet ran\n::shell ls {{ target }}\n", "ls {{ target }}"),
        ("absent.md", "::shell git init --quiet ran\n::shell ls {{ target }}\n", "ls {{ target }}"),
        (
            "frontmatter.md",
            "---\nlisting: $(ls {{ target }})\n---\n::shell git init --quiet ran\n",
            "ls {{ target }}",
        ),
        (
            "block.md",
            "::shell git init --quiet ran\n::shell-block\nls {{ fm.target }}\n::end-block\n",
            "ls {{ fm.target }}",
        ),
    ] {
        let md_path = fixture.write_file(format!("cwd/{name}"), body);
        // `git` is the recording executable, so expose the host's.
        fixture
            .command_builder()
            .host_path()
            .build()
            .arg("compose")
            .arg(&md_path)
            .assert()
            .failure()
            .stderr(predicate::str::contains(command))
            .stderr(predicate::str::contains("which is missing"));
        assert!(!fixture.cwd().join("ran").exists(), "{name}: a command ran");
    }
}

#[test]
fn test_compose_runs_a_shell_command_whose_value_is_an_empty_string() {
    let fixture = CliProcessFixture::named("test_compose_runs_an_empty_string_shell_value");
    fixture.shell_permissions(&["prefix git"]);
    let md_path = fixture.write_file(
        "cwd/test.md",
        "---\nsuffix: \"\"\n---\n::shell git init --quiet ran{{ suffix }}\n",
    );
    fixture
        .command_builder()
        .host_path()
        .build()
        .arg("compose")
        .arg(&md_path)
        .assert()
        .success();
    assert!(fixture.cwd().join("ran").join(".git").exists());
}
