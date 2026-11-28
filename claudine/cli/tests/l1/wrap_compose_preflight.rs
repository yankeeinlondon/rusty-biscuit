//! Integration tests: compose shell preflight gating.
//!
//! Split out of the `wrap_commands.rs` god file; shared fixtures live in
//! `common::wrap`.

use claudine::provider::Provider;
use std::fs;
use crate::common;
use common::{CliProcessFixture, strip_ansi, write_executable};

/// Non-TTY shell gate: an unapproved `::shell` command (no approval handler,
/// no whitelist) makes a real `compose` exit non-zero naming the offending
/// command before the provider starts. This is the CI gate "working
/// correctly". The dry run asks for no approval: it runs no command, reports
/// the directive as not checked, and exits 0.
#[cfg(unix)]
#[test]
fn compose_non_tty_unapproved_shell_is_gated_and_the_dry_run_runs_nothing() {
    let fixture = CliProcessFixture::named("compose-dry-run-shell-gate");
    fixture.seed_user_config();

    let md_file = fixture.cwd().join("template.md");
    let marker = fixture.cwd().join("shell-ran");
    fs::write(
        &md_file,
        format!("---\ntitle: shell gate\n---\n::shell touch {}\n", marker.display()),
    )
    .unwrap();

    // Provider stub so target resolution succeeds before preflight aborts.
    write_executable(
        &fixture.bin_dir().join("goose"),
        "#!/bin/sh\necho 'provider should not run' >&2\nexit 99\n",
    );

    let collapse = |stderr: &[u8]| {
        // The prose renderer hard-wraps the styled error block inside a box,
        // so drop the box-border glyphs and collapse whitespace runs before
        // matching (wrapping width is terminal-dependent).
        strip_ansi(&String::from_utf8_lossy(stderr))
            .replace('┃', " ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    };

    let run = fixture.command().args(["compose", "--goose", md_file.to_str().unwrap()]).output().unwrap();
    let collapsed = collapse(&run.stderr);
    assert!(!run.status.success(), "the unapproved command must stop the run; stderr was:\n{collapsed}");
    assert!(
        collapsed.contains("requires approval but no approval handler is available"),
        "expected the gate message; stderr was:\n{collapsed}"
    );
    assert!(
        !collapsed.contains("provider should not run"),
        "provider must not execute when the gate fires; stderr was:\n{collapsed}"
    );

    let dry_run = fixture
        .command()
        .args(["compose", "--goose", "--dry-run", md_file.to_str().unwrap()])
        .output()
        .unwrap();
    let collapsed = collapse(&dry_run.stderr);
    assert_eq!(dry_run.status.code(), Some(0), "stderr was:\n{collapsed}");
    assert!(collapsed.contains("not checked: `::shell` directive"), "stderr was:\n{collapsed}");
    assert!(!collapsed.contains("provider should not run"), "stderr was:\n{collapsed}");
    assert!(!marker.exists(), "neither run may execute the command");
}

/// `--dry-run --yolo` still runs no shell command: `--yolo` approves
/// commands for a real run, and the dry run runs none, so the directive is
/// previewed unevaluated.
#[cfg(unix)]
#[test]
fn compose_dry_run_yolo_runs_no_shell_command() {
    let fixture = CliProcessFixture::named("compose-dry-run-yolo");
    fixture.seed_user_config();

    let md_file = fixture.cwd().join("template.md");
    let marker = fixture.cwd().join("shell-ran");
    fs::write(
        &md_file,
        format!("---\ntitle: yolo\n---\n::shell touch {}\n", marker.display()),
    )
    .unwrap();

    write_executable(
        &fixture.bin_dir().join("goose"),
        "#!/bin/sh\necho 'provider should not run' >&2\nexit 99\n",
    );

    let assert = fixture
        .command()
        .args([
            "compose",
            "--goose",
            "--dry-run",
            "--yolo",
            md_file.to_str().unwrap(),
        ])
        .assert()
        .success();

    let plain = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stdout));
    assert!(
        plain.contains("[not evaluated in a dry run]"),
        "the directive is previewed unevaluated; stdout was:\n{plain}"
    );
    assert!(!marker.exists(), "the dry run must not run the command");
}

/// When the prompt file lives outside any git repo, `CompositionPrepContext`
/// must fall back to the ambient CWD to load `selection_config`. Without this
/// fallback non-TTY resolution loses the favorite-agent and model overrides
/// that the legacy path preserved.
#[cfg(unix)]
#[test]
fn compose_non_tty_uses_cwd_config_when_source_outside_git() {
    let fixture = CliProcessFixture::named("compose-cwd-config-outside-git");

    // Set up HOME with a claudine config that has a favorite provider.
    let claudine_dir = fixture.home().join(".claudine");
    fs::create_dir_all(&claudine_dir).unwrap();

    let config = claudine::config::claudine_config::ClaudineConfig {
        preferred_agent: Some(Provider::Goose),
        ..claudine::config::claudine_config::ClaudineConfig::default()
    };
    let config_path = claudine_dir.join("config.yaml");
    claudine::config::persist::initialize_config(
        &config_path,
        claudine::config::schema::ConfigScope::User,
        &claudine::config::claudine_config::ClaudineConfig::default(),
        &config,
    )
    .unwrap();

    // Create a source file outside any git repo.
    let source_dir = fixture.cwd().join("source");
    fs::create_dir_all(&source_dir).unwrap();
    let md_file = source_dir.join("prompt.md");
    fs::write(&md_file, "---\ntitle: test\n---\n# Hello\n").unwrap();

    // Fake provider binary so Goose is "installed".
    write_executable(
        &fixture.bin_dir().join("goose"),
        "#!/bin/sh\necho 'goose ran'\nexit 0\n",
    );

    // Run in non-TTY mode (null stdin) from a CWD that is NOT a git repo.
    let output = fixture
        .command()
        .args(["compose", "--dry-run", "--json", md_file.to_str().unwrap()])
        .output()
        .unwrap();

    // With no agent hint and no explicit provider, a non-interactive run
    // has no provider: the check reports that rather than auto-selecting
    // the favorite.
    let envelope: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(output.status.code(), Some(2), "{envelope}");
    let codes: Vec<&str> =
        envelope["diagnostics"].as_array().unwrap().iter().filter_map(|finding| finding["code"].as_str()).collect();
    assert!(codes.contains(&"composition.provider_unresolved"), "{envelope}");
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
    let fixture = CliProcessFixture::named("compose-interactive-whitelist");
    fixture.seed_user_config();

    let md_file = fixture.cwd().join("template.md");
    fs::write(
        &md_file,
        "---\ntitle: interactive test\n---\n::shell echo whitelisted\n",
    )
    .unwrap();

    // Allow "echo" so the ::shell directive passes preflight.
    common::write_shell_permissions(fixture.home(), "prefix echo\n");

    write_executable(
        &fixture.bin_dir().join("codex"),
        "#!/bin/sh\ncat > /dev/null\necho 'provider-launched' >&2\nexit 0\n",
    );

    // The default PATH already carries `/usr/bin:/bin` behind the fixture bin,
    // so shell expansion still finds `echo`.
    let assert = fixture
        .command()
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
    let fixture = CliProcessFixture::named("compose-preflight-false-block");
    fixture.seed_user_config();

    // Preflight discovery is condition-blind: a ::shell inside a
    // ::block when="false" is still discovered and must be whitelisted, even
    // though composition would exclude it from the output. An un-whitelisted
    // command therefore fails preflight rather than being silently skipped.
    let md_file = fixture.cwd().join("template.md");
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
        &fixture.bin_dir().join("codex"),
        "#!/bin/sh\necho 'provider-launched' >&2\nexit 0\n",
    );

    // No whitelist for curl and no --interactive approval handler, so the
    // discovered command fails preflight.
    let assert = fixture
        .command()
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
    let fixture = CliProcessFixture::named("compose-dry-run-proxy");
    fixture.seed_user_config();

    let target = fixture.cwd().join("target.md");
    fs::write(&target, "---\ntitle: target\n---\nTARGET-BODY\n").unwrap();

    // `--goose` resolves the target eagerly, so the run takes the resolved-target
    // seam rather than the earlier unresolved-selection one.
    let router = fixture.cwd().join("router.md");
    fs::write(
        &router,
        "---\ntitle: router\ninitialize:\n  - {proxy: \"target.md\"}\n---\nROUTER-BODY\n",
    )
    .unwrap();

    write_executable(
        &fixture.bin_dir().join("goose"),
        "#!/bin/sh\necho 'provider should not run' >&2\nexit 99\n",
    );

    let assert = fixture
        .command()
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
    let fixture = CliProcessFixture::named("compose-dry-run-no-side-effects");
    fixture.seed_user_config();
    fixture.initialize_repository();

    let events_log = fixture.cwd().join("events.log");
    let md_file = fixture.cwd().join("doc.md");
    let marker = |event: &str| {
        format!("{event}:\n  - {{append_line: [\"events.log\", \"{event}\"]}}\n")
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
        &fixture.bin_dir().join("goose"),
        "#!/bin/sh\necho 'provider should not run' >&2\nexit 99\n",
    );

    let assert = fixture
        .command()
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
