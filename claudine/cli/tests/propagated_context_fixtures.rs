//! Positive real-CLI coverage for ambient context intentionally enabled by a fixture.

#[cfg(unix)]
use std::fs;

mod common;
use common::{CliProcessFixture, strip_ansi, write, write_dry_run_provider_stub, write_executable};

fn verbose_stderr(assertion: &assert_cmd::assert::Assert) -> String {
    strip_ansi(&String::from_utf8_lossy(&assertion.get_output().stderr))
}

#[test]
fn isolated_fixture_can_opt_in_to_repo_prompt_and_appendix_discovery() {
    let fixture = CliProcessFixture::named("context-repo-prompt");
    fixture.initialize_repository();
    fixture.seed_user_config();
    fixture.write_root_system_prompt("REPO_SYSTEM_PROMPT_SENTINEL");
    fixture.write_repo_appendix("REPO_APPENDIX_SENTINEL");
    write_dry_run_provider_stub(fixture.bin_dir(), "codex");

    let assertion = fixture
        .command()
        .env("CLAUDINE_SYSTEM_PROMPT", "verbose")
        .env("PATHEXT", ".COM;.EXE;.BAT;.CMD")
        .args(["codex", "--dry-run", "inspect the repository"])
        .assert()
        .success();
    let stderr = verbose_stderr(&assertion);

    assert!(stderr.contains("REPO_SYSTEM_PROMPT_SENTINEL"), "{stderr}");
    assert!(stderr.contains("REPO_APPENDIX_SENTINEL"), "{stderr}");
}

/// `$HOME` is the isolation facility this fixture requires: it seeds the
/// user-global prompt under the fixture home and asserts the child process
/// discovers it.
///
/// Unix-only for the reason recorded in the Phase 4 home/config discovery
/// note — `dirs` 6 reads the Windows profile through the Known Folder API, so
/// the fixture's `HOME`/`USERPROFILE` overrides are inert there and the child
/// would read the real user profile. That inertness is the specified native
/// behavior and must not be replaced with HOME-first production discovery.
/// The user-home leg of standard discovery is covered on every platform by
/// `system_prompt::resolve::tests::standard_discovery_user_home_is_injectable`.
#[cfg(unix)]
#[test]
fn isolated_fixture_can_opt_in_to_user_prompt_discovery() {
    let fixture = CliProcessFixture::named("context-user-prompt");
    fixture.seed_user_config();
    fixture.write_user_system_prompt("USER_SYSTEM_PROMPT_SENTINEL");
    write_dry_run_provider_stub(fixture.bin_dir(), "codex");

    let assertion = fixture
        .command()
        .env("CLAUDINE_SYSTEM_PROMPT", "verbose")
        .env("PATHEXT", ".COM;.EXE;.BAT;.CMD")
        .args(["codex", "--dry-run", "inspect the directory"])
        .assert()
        .success();
    let stderr = verbose_stderr(&assertion);

    assert!(stderr.contains("USER_SYSTEM_PROMPT_SENTINEL"), "{stderr}");
}

#[test]
fn isolated_fixture_can_opt_in_to_package_scoped_prompt_discovery() {
    let fixture = CliProcessFixture::named("context-package-prompt");
    fixture.initialize_repository();
    fixture.seed_user_config();
    fixture.write_root_system_prompt("REPO_SCOPE_SENTINEL");
    write(
        &fixture.cwd().join("Cargo.toml"),
        "[workspace]\nmembers = [\"area/pkg\", \"area/other\"]\nresolver = \"2\"\n",
    );
    let package = fixture.cwd().join("area/pkg");
    write(
        &package.join("Cargo.toml"),
        "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    write(
        &fixture.cwd().join("area/other/Cargo.toml"),
        "[package]\nname = \"fixture-other\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    );
    write(
        &package.join("system-prompt.md"),
        "PACKAGE_SCOPE_SENTINEL",
    );
    write_dry_run_provider_stub(fixture.bin_dir(), "codex");

    let assertion = fixture
        .command()
        .current_dir(&package)
        .env("CLAUDINE_SYSTEM_PROMPT", "verbose")
        .env("PATHEXT", ".COM;.EXE;.BAT;.CMD")
        .args(["codex", "--dry-run", "inspect the package"])
        .assert()
        .success();
    let stderr = verbose_stderr(&assertion);

    assert!(stderr.contains("PACKAGE_SCOPE_SENTINEL"), "{stderr}");
    assert!(!stderr.contains("REPO_SCOPE_SENTINEL"), "{stderr}");
}

#[test]
fn isolated_fixture_can_opt_in_to_provider_memory_discovery() {
    let fixture = CliProcessFixture::named("context-provider-memory");
    fixture.initialize_repository();
    fixture.seed_user_config();
    fixture.write_provider_memory(
        "CLAUDE.md",
        "---\ntimeout: 5m\n---\nMemory body.\n",
    );

    #[cfg(windows)]
    write_executable(
        &fixture.bin_dir().join("claude.cmd"),
        "@echo off\r\nexit /b 0\r\n",
    );
    #[cfg(not(windows))]
    write_executable(
        &fixture.bin_dir().join("claude"),
        "#!/bin/sh\nexit 0\n",
    );

    let assertion = fixture
        .command()
        .env("PATHEXT", ".COM;.EXE;.BAT;.CMD")
        .args(["claude", "run the request"])
        .assert()
        .success();
    let stderr = verbose_stderr(&assertion);

    assert!(stderr.contains("CLAUDE.md"), "{stderr}");
}

#[cfg(unix)]
#[test]
fn isolated_fixture_can_opt_in_to_shadow_home_repo_resources() {
    let fixture = CliProcessFixture::named("context-shadow-home");
    fixture.initialize_repository();
    fixture.seed_user_config();
    fs::create_dir_all(fixture.home().join(".codex")).unwrap();
    write(
        &fixture.cwd().join(".claude/commands/review.md"),
        "---\ndescription: review\n---\n",
    );
    let capture = fixture.cwd().join("shadow-home.txt");
    write_executable(
        &fixture.bin_dir().join("codex"),
        r#"#!/bin/sh
{
  printf 'HOME=%s\n' "$HOME"
  if [ -e "$HOME/.codex/prompts/review.md" ]; then
    printf 'HAS_REPO_PROMPT=1\n'
  else
    printf 'HAS_REPO_PROMPT=0\n'
  fi
} > "$CLAUDINE_CONTEXT_CAPTURE"
"#,
    );

    fixture
        .command()
        .env("CLAUDINE_CONTEXT_CAPTURE", &capture)
        .args(["codex", "--", "--version"])
        .assert()
        .success();

    let captured = fs::read_to_string(capture).unwrap();
    assert!(
        captured.contains(&format!("HOME={}", fixture.home().join(".claudine").display())),
        "{captured}"
    );
    assert!(captured.contains("HAS_REPO_PROMPT=1"), "{captured}");
}
