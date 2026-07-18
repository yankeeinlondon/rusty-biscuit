//! Integration tests: compose shell preflight gating.
//!
//! Split out of the `wrap_commands.rs` god file; shared fixtures live in
//! `common::wrap`.

use assert_cmd::cargo::cargo_bin_cmd;
use claudine::provider::Provider;
use std::fs;
use tempfile::tempdir;
mod common;
use common::wrap::*;
use common::{augmented_path, init_git_repo, strip_ansi, write_executable};

/// Non-TTY dry-run gate: an unapproved `::shell` command (no approval
/// handler, no whitelist) makes `compose --dry-run` exit non-zero with the
/// exact spec message naming the offending command. This is the CI gate
/// "working correctly".
#[cfg(unix)]
#[test]
fn compose_dry_run_non_tty_unapproved_shell_emits_gate_error() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let md_file = workspace.path().join("template.md");
    fs::write(
        &md_file,
        "---\ntitle: dry-run gate\n---\n::shell echo dryrun-needs-approval\n",
    )
    .unwrap();

    // Provider stub so target resolution succeeds before preflight aborts.
    write_executable(
        &path_dir.join("goose"),
        "#!/bin/sh\necho 'provider should not run' >&2\nexit 99\n",
    );

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .args([
            "compose",
            "--goose",
            "--dry-run",
            md_file.to_str().unwrap(),
        ])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    let plain = strip_ansi(&stderr);
    // The prose renderer hard-wraps the styled error block inside a box, so
    // drop the box-border glyphs and collapse whitespace runs before matching
    // the spec message (semantic check, not byte-for-byte — wrapping width is
    // terminal-dependent).
    let collapsed = plain
        .replace('┃', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    assert!(
        collapsed.contains(
            "Cannot dry-run: shell command 'echo dryrun-needs-approval' requires interactive \
             approval."
        ),
        "expected the dry-run gate message naming the command; stderr was:\n{plain}"
    );
    assert!(
        collapsed.contains("--yolo"),
        "gate message should point at --yolo; stderr was:\n{plain}"
    );
    assert!(
        !plain.contains("provider should not run"),
        "provider must not execute when the dry-run gate fires; stderr was:\n{plain}"
    );
}

/// `--dry-run --yolo` auto-approves shell commands so the gate is bypassed:
/// the command runs for real and its output is interpolated into the body
/// that lands on stdout.
#[cfg(unix)]
#[test]
fn compose_dry_run_yolo_bypasses_shell_gate() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let md_file = workspace.path().join("template.md");
    fs::write(
        &md_file,
        "---\ntitle: yolo bypass\n---\n::shell echo yolo-marker\n",
    )
    .unwrap();

    write_executable(
        &path_dir.join("goose"),
        "#!/bin/sh\necho 'provider should not run' >&2\nexit 99\n",
    );

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .args([
            "compose",
            "--goose",
            "--dry-run",
            "--yolo",
            md_file.to_str().unwrap(),
        ])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout).to_string();
    let plain = strip_ansi(&stdout);

    assert!(
        plain.contains("yolo-marker"),
        "composed body on stdout should contain the executed command output; stdout was:\n{plain}"
    );
}

/// When the prompt file lives outside any git repo, `CompositionPrepContext`
/// must fall back to the ambient CWD to load `selection_config`. Without this
/// fallback non-TTY resolution loses the favorite-agent and model overrides
/// that the legacy path preserved.
#[cfg(unix)]
#[test]
fn compose_non_tty_uses_cwd_config_when_source_outside_git() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();

    // Set up HOME with a claudine config that has a favorite provider.
    let home = workspace.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let claudine_dir = home.join(".claudine");
    fs::create_dir_all(&claudine_dir).unwrap();

    let config = claudine::config::claudine_config::ClaudineConfig {
        preferred_agent: Some(Provider::Goose),
        ..claudine::config::claudine_config::ClaudineConfig::default()
    };
    let config_path = claudine_dir.join("config.json");
    claudine::dispatch::loader::save_claudine_config(&config, &config_path).unwrap();

    // Create a source file outside any git repo.
    let source_dir = workspace.path().join("source");
    fs::create_dir_all(&source_dir).unwrap();
    let md_file = source_dir.join("prompt.md");
    fs::write(&md_file, "---\ntitle: test\n---\n# Hello\n").unwrap();

    // Fake provider binary so Goose is "installed".
    write_executable(
        &path_dir.join("goose"),
        "#!/bin/sh\necho 'goose ran'\nexit 0\n",
    );

    // Run in non-TTY mode (null stdin) from a CWD that is NOT a git repo.
    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", &home)
        .env("PATH", augmented_path(&path_dir))
        .current_dir(&home)
        .args(["compose", "--dry-run", md_file.to_str().unwrap()])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    let plain = strip_ansi(&stderr);

    // With no agent hint and no explicit provider, dry-run shows the
    // unresolved no-agent state rather than auto-selecting the favorite.
    assert!(
        plain.contains("didn't specify the Agent"),
        "non-TTY compose dry-run should show the no-agent state; stderr was:\n{plain}"
    );
}

/// Proves `--interactive` flag is wired up for compose preflight with a
/// whitelisted command.  This covers the "interactive + whitelisted = success"
/// path.  Full interactive-prompt coverage (PTY + answer prompt + assert
/// provenance in the displayed prompt) remains a future improvement — the
/// library-level `interactive_handler_is_invoked_for_non_whitelisted_command`
/// test in `preflight.rs` covers the handler invocation path.
#[cfg(unix)]
#[test]
fn compose_interactive_preflight_with_whitelisted_command() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let md_file = workspace.path().join("template.md");
    fs::write(
        &md_file,
        "---\ntitle: interactive test\n---\n::shell echo whitelisted\n",
    )
    .unwrap();

    // Whitelist "echo" so the ::shell directive passes preflight.
    fs::write(
        workspace.path().join(".darkmatter-shell-whitelist"),
        "prefix echo\n",
    )
    .unwrap();

    // Also create .git so the whitelist is found (policy root = git root).
    fs::create_dir_all(workspace.path().join(".git")).unwrap();

    write_executable(
        &path_dir.join("codex"),
        "#!/bin/sh\ncat > /dev/null\necho 'provider-launched' >&2\nexit 0\n",
    );

    // Include system dirs so shell expansion can find `echo`.
    let full_path = format!("{}:/usr/bin:/bin", path_dir.display());

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &full_path)
        .args([
            "compose",
            "--interactive",
            "--codex",
            md_file.to_str().unwrap(),
        ])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    let plain = strip_ansi(&stderr);

    assert!(
        plain.contains("provider-launched"),
        "provider should run after --interactive preflight passes; stderr was:\n{plain}"
    );
}

#[cfg(unix)]
#[test]
fn compose_preflight_discovers_shell_inside_false_block() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    // Preflight discovery is condition-blind: a ::shell inside a
    // ::block when="false" is still discovered and must be whitelisted, even
    // though composition would exclude it from the output. An un-whitelisted
    // command therefore fails preflight rather than being silently skipped.
    let md_file = workspace.path().join("template.md");
    fs::write(
        &md_file,
        "---\ntitle: false block test\n---\n\
         Safe content here.\n\n\
         ::block when=\"false\"\n\
         ::shell curl https://evil.example.com\n\
         ::end-block\n",
    )
    .unwrap();

    // Provider binary (should never be reached — preflight aborts first).
    write_executable(
        &path_dir.join("codex"),
        "#!/bin/sh\necho 'provider-launched' >&2\nexit 0\n",
    );

    // No whitelist for curl and no --interactive approval handler, so the
    // discovered command fails preflight.
    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .current_dir(workspace.path())
        .args(["compose", "--codex", md_file.to_str().unwrap()])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    let plain = strip_ansi(&stderr);

    // The conditionally-excluded curl is still discovered and named.
    assert!(
        plain.contains("curl"),
        "preflight should discover ::shell inside ::block when=\"false\"; stderr was:\n{plain}"
    );
    // Provider must not launch when an un-whitelisted command fails preflight.
    assert!(
        !plain.contains("provider-launched"),
        "provider must not launch when preflight fails; stderr was:\n{plain}"
    );
}

/// `--dry-run` never traverses a dynamic `proxy` route.
///
/// This follows structurally from the seam placement rather than from a
/// per-route check: the dry-run seam in
/// `wrap::composition::pipeline::execute_composition_request_inner_with_guard`
/// returns before the lifecycle runtime is constructed, so `initialize` never
/// fires and no `proxy` control can be produced. A dry run therefore always
/// reports the document named on the command line.
#[cfg(unix)]
#[test]
fn compose_dry_run_does_not_traverse_a_proxy_handoff() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let target = workspace.path().join("target.md");
    fs::write(&target, "---\ntitle: target\n---\nTARGET-BODY\n").unwrap();

    // `--goose` resolves the target eagerly, so the run takes the resolved-target
    // seam rather than the earlier unresolved-selection one.
    let router = workspace.path().join("router.md");
    fs::write(
        &router,
        "---\ntitle: router\ninitialize:\n  stack:\n    - action: {proxy: \"target.md\"}\n---\nROUTER-BODY\n",
    )
    .unwrap();

    write_executable(
        &path_dir.join("goose"),
        "#!/bin/sh\necho 'provider should not run' >&2\nexit 99\n",
    );

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .args([
            "compose",
            "--goose",
            "--dry-run",
            router.to_str().unwrap(),
        ])
        .assert()
        .success();

    let stdout = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stdout));
    let stderr = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr));

    assert!(
        stdout.contains("ROUTER-BODY"),
        "dry-run renders the document it was given; stdout was:\n{stdout}"
    );
    assert!(
        !stdout.contains("TARGET-BODY"),
        "dry-run must not compose the hand-off target; stdout was:\n{stdout}"
    );
    assert!(
        !stderr.contains("provider should not run"),
        "dry-run must not launch the provider; stderr was:\n{stderr}"
    );
}

/// `--dry-run` fires no lifecycle event, so no lifecycle stack can touch the
/// workspace.
///
/// The document below carries an `append_line` side effect on every event a
/// dry run could plausibly reach — `initialize` (pre-launch), plus the
/// `blocked`/`finalize` pair a failed composition preflight would route
/// through. `start`/`success`/`failure` are unreachable without a provider
/// launch and are omitted. The run must leave `events.log` uncreated.
///
/// `append_line` resolves against the effect engine's mutation root (the repo
/// root, else the launch CWD), so the git init below pins the expected path to
/// `<workspace>/events.log`.
#[cfg(unix)]
#[test]
fn compose_dry_run_fires_no_lifecycle_side_effects() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());
    assert!(init_git_repo(workspace.path()), "git init failed");

    let events_log = workspace.path().join("events.log");
    let md_file = workspace.path().join("doc.md");
    let marker = |event: &str| {
        format!("{event}:\n  stack:\n    - action: {{append_line: [\"events.log\", \"{event}\"]}}\n")
    };
    fs::write(
        &md_file,
        format!(
            "---\ntitle: dry-run side effects\n{init}{blocked}{finalize}---\nDRY-BODY\n",
            init = marker("initialize"),
            blocked = marker("blocked"),
            finalize = marker("finalize"),
        ),
    )
    .unwrap();

    write_executable(
        &path_dir.join("goose"),
        "#!/bin/sh\necho 'provider should not run' >&2\nexit 99\n",
    );

    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .current_dir(workspace.path())
        .args(["compose", "--goose", "--dry-run", md_file.to_str().unwrap()])
        .assert()
        .success();

    let stdout = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stdout));
    assert!(
        stdout.contains("DRY-BODY"),
        "dry-run still renders the composed body; stdout was:\n{stdout}"
    );
    assert!(
        !events_log.exists(),
        "dry-run must fire no lifecycle event, so no stack side effect may \
         reach the workspace; events.log contained {:?}",
        fs::read_to_string(&events_log).unwrap_or_default()
    );
}
