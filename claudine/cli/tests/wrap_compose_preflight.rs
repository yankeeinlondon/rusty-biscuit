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
use common::{augmented_path, strip_ansi, write_executable};

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
/// The Phase 6 plan asserted dry-run "fires no lifecycle events and therefore
/// never traverses a dynamic proxy route". The first half is false and
/// deliberately so: `level2_lifecycle_dispatch`'s
/// `..._blocked_preflight_dry_run_shell_audit_fires_blocked_and_finalize_stacks`
/// pins `initialize` → `blocked` → `finalize` as the dry-run contract. So a
/// router's `initialize` *can* hand off during a dry run, and the conclusion
/// has to be earned rather than inherited: the dry-run seam consumes the
/// transition and renders the document it was given. Without that, the
/// hand-off would be the silently-dropped proxy the transition type exists to
/// prevent.
#[cfg(unix)]
#[test]
fn compose_dry_run_does_not_traverse_a_proxy_handoff() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let target = workspace.path().join("target.md");
    fs::write(&target, "---\ntitle: target\n---\nTARGET-BODY\n").unwrap();

    // `--goose` below makes the target resolve eagerly, which is the path that
    // reaches `initialize` on a dry run.
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
