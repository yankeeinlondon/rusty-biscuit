mod common;

use common::CliProcessFixture;
use predicates::prelude::*;

#[test]
fn test_compose_with_whitelisted_command_succeeds() {
    let fixture = CliProcessFixture::named("test_compose_with_whitelisted_command_succeeds");
    let md_path = fixture.write_file("cwd/test.md", "# Test\n::shell echo hello\n");
    fixture.write_file("cwd/.darkmatter-shell-whitelist", "prefix echo\n");

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
    // Policy paths anchor on the launch context (repository root, then home,
    // then the compose base dir) on every platform; with HOME pointed at the
    // tempdir and no enclosing repository, the guidance names the tempdir —
    // Windows included, now that the fallback no longer consults USERPROFILE.
    let whitelist_path = fixture.cwd().join(".darkmatter-shell-whitelist");

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
    let fixture =
        CliProcessFixture::named("test_compose_file_unapproved_command_fails_with_guidance");
    let md_path = fixture.write_file("cwd/test.md", "# Test\n::shell echo hello\n");
    let whitelist_path = fixture.cwd().join(".darkmatter-shell-whitelist");

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
    let fixture = CliProcessFixture::named("test_compose_with_nonexistent_command_fails");
    let md_path = fixture.write_file("cwd/test.md", "# Test\n::shell nonexistent_command_xyz\n");
    fixture.write_file(
        "cwd/.darkmatter-shell-whitelist",
        "prefix nonexistent_command_xyz\n",
    );
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
    fixture.write_file("cwd/.darkmatter-shell-whitelist", "prefix sleep\n");

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
    fixture.write_file("cwd/.darkmatter-shell-whitelist", "prefix sleep\n");

    // A real `sleep` shell process is required to exercise timeout recovery.
    fixture
        .command_builder()
        .host_path()
        .build()
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
