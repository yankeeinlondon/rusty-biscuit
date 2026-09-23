//! `has_alias`, `has_builtin_function`, `has_user_function`, `can_execute`,
//! and `has_agentic_cli` through the normal `md compose` path.
//!
//! The login shell is always a fixture: a stub named for its dialect, or the
//! system `/bin/bash` started with the fixture home and no inherited
//! environment, so no test reads a developer profile.

mod common;

use common::{CliProcessFixture, write_executable};
use predicates::prelude::*;

/// Writes a `sh` stub named `bash` that logs each launch and answers from a
/// fixed table, so the bash dialect is selected without a real bash.
#[cfg(unix)]
fn stub_bash(fixture: &CliProcessFixture) -> (std::path::PathBuf, std::path::PathBuf) {
    let log = fixture.tmp_dir().join("shell-launches.log");
    let shell = fixture.tmp_dir().join("shells").join("bash");
    write_executable(
        &shell,
        &format!(
            "#!/bin/sh\necho launched >> '{}'\ncase \"$DARKMATTER_SHELL_PROBE_NAME\" in\n  \
             ll) echo darkmatter-shell-probe:alias ;;\n  \
             cd) echo darkmatter-shell-probe:builtin ;;\n  \
             mkcd) echo darkmatter-shell-probe:function ;;\n\
             esac\n",
            log.display()
        ),
    );
    (shell, log)
}

#[cfg(unix)]
fn launches(log: &std::path::Path) -> usize {
    std::fs::read_to_string(log).map(|text| text.lines().count()).unwrap_or(0)
}

#[cfg(unix)]
#[test]
fn shell_predicates_answer_from_the_request_login_shell() {
    let fixture = CliProcessFixture::named("shell_predicates_answer_from_the_request_login_shell");
    let (shell, log) = stub_bash(&fixture);
    let document = fixture.write_file(
        "cwd/doc.md",
        "{{ has_alias(\"ll\") }} {{ has_builtin_function(\"cd\") }} {{ has_user_function(\"mkcd\") }} \
         {{ has_alias(\"cd\") }} {{ can_execute(\"nothing-here\") }}\n",
    );

    fixture
        .command()
        .env("SHELL", &shell)
        .arg("compose")
        .arg(&document)
        .assert()
        .success()
        .stdout(predicate::str::contains("true true true false false"));
    // One launch per call in each pass that composes the body: reference
    // validation and the terminal compose. Shell-command preflight launches
    // none (`darkmatter/lib/tests/shell_probe_preflight.rs`).
    assert_eq!(launches(&log), 10);
}

/// A name `has_binary` finds is executable without launching the profile.
#[cfg(unix)]
#[test]
fn can_execute_finds_a_binary_without_launching_the_shell() {
    let fixture = CliProcessFixture::named("can_execute_finds_a_binary_without_launching_the_shell");
    let (shell, log) = stub_bash(&fixture);
    write_executable(&fixture.bin_dir().join("fixture-tool"), "#!/bin/sh\n");
    let document = fixture.write_file("cwd/doc.md", "tool={{ can_execute(\"fixture-tool\") }}\n");

    fixture
        .command()
        .env("SHELL", &shell)
        .arg("compose")
        .arg(&document)
        .assert()
        .success()
        .stdout(predicate::str::contains("tool=true"));
    assert_eq!(launches(&log), 0, "a found binary must short-circuit the shell probe");
}

/// The real system bash with an empty fixture home: builtins are recognized,
/// and a command-substitution-shaped name is inert data.
#[cfg(unix)]
#[test]
fn a_real_shell_never_executes_the_probed_name() {
    let bash = std::path::Path::new("/bin/bash");
    if !bash.exists() {
        eprintln!("skipping: /bin/bash is not installed on this host");
        return;
    }
    let fixture = CliProcessFixture::named("a_real_shell_never_executes_the_probed_name");
    let marker = fixture.tmp_dir().join("pwned");
    let document = fixture.write_file(
        "cwd/doc.md",
        &format!(
            "cd={{{{ has_builtin_function(\"cd\") }}}} injected={{{{ can_execute(\"`touch {m}`\") }}}} \
             separated={{{{ has_alias(\";touch {m}\") }}}}\n",
            m = marker.display()
        ),
    );

    // No inherited environment: the shell sees only the fixture home.
    fixture
        .command_builder()
        .inherit_no_env()
        .build()
        .env("SHELL", bash)
        .arg("compose")
        .arg(&document)
        .assert()
        .success()
        .stdout(predicate::str::contains("cd=true injected=false separated=false"));
    assert!(!marker.exists(), "the probed name was executed");
}

#[test]
fn shell_predicates_without_a_login_shell_are_false() {
    let fixture = CliProcessFixture::named("shell_predicates_without_a_login_shell_are_false");
    let document = fixture.write_file(
        "cwd/doc.md",
        "{{ has_alias(\"ll\") }} {{ has_user_function(\"mkcd\") }} {{ can_execute(\"nothing-here\") }}\n",
    );

    let mut command = fixture.command_builder().fake_only_path().build();
    command.env_remove("SHELL");
    // On Windows the absent `$SHELL` falls back to `pwsh`/`powershell`, which
    // the fake-only PATH does not contain.
    command
        .arg("compose")
        .arg(&document)
        .assert()
        .success()
        .stdout(predicate::str::contains("false false false"));
}

/// AC18 through compose: provider aliases agree, only the fixture `PATH` is
/// consulted, and an unknown name fails the compose.
#[test]
fn has_agentic_cli_reads_the_request_path_index() {
    let fixture = CliProcessFixture::named("has_agentic_cli_reads_the_request_path_index");
    #[cfg(unix)]
    write_executable(&fixture.bin_dir().join("kimi"), "#!/bin/sh\n");
    #[cfg(windows)]
    write_executable(&fixture.bin_dir().join("kimi.cmd"), "@echo off\r\n");
    let document = fixture.write_file(
        "cwd/doc.md",
        "kimi_code={{ has_agentic_cli(\"kimi_code\") }} kimi={{ has_agentic_cli(\"kimi\") }} \
         claude={{ has_agentic_cli(\"claude\") }}\n",
    );

    // Fake-only PATH: a host-installed `claude` must not be visible.
    fixture
        .command_builder()
        .fake_only_path()
        .build()
        .arg("compose")
        .arg(&document)
        .assert()
        .success()
        .stdout(predicate::str::contains("kimi_code=true kimi=true claude=false"));

    let unknown = fixture.write_file("cwd/unknown.md", "{{ has_agentic_cli(\"not_a_provider\") }}\n");
    fixture
        .command_builder()
        .fake_only_path()
        .build()
        .arg("compose")
        .arg(&unknown)
        .assert()
        .failure()
        .stderr(predicate::str::contains("not_a_provider"));
}
