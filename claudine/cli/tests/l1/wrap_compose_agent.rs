//! Integration tests: compose agent resolution reporting and validation/handler engagement banners.
//!
//! Split out of the `wrap_commands.rs` god file; shared fixtures live in
//! `common::wrap`.

use predicates::str::contains;
use std::fs;
use crate::common;
use common::{CliProcessFixture, strip_ansi, write_executable};

#[cfg(unix)]
#[test]
#[serial_test::serial]
fn repo_scoped_config_favorite_selects_provider() {
    // Verifies that a repo-level .claudine/config.yaml with a linking
    // preference is consulted during composition selection so the
    // favorite provider wins over interactive selection.
    let fixture = CliProcessFixture::named("compose-repo-favorite");
    let args_file = fixture.cwd().join("goose-args.txt");
    fixture.seed_user_config();

    // Initialize a git repo so repo root detection works
    fixture.initialize_repository();

    // Create repo-local config with goose as the preferred agent
    let config_dir = fixture.cwd().join(".claudine");
    fs::create_dir_all(&config_dir).unwrap();
    fs::write(
        config_dir.join("config.yaml"),
        r#"{"preferred_agent":"goose"}"#,
    )
    .unwrap();

    let md_file = fixture.cwd().join("test.md");
    fs::write(&md_file, "---\ntitle: test\n---\nPrompt body\n").unwrap();

    // Install both providers — without the config favorite, multiple
    // installed providers would require interactive selection.
    write_executable(
        &fixture.bin_dir().join("goose"),
        r#"#!/bin/sh
printf '%s\n' "$@" > "$CLAUDINE_ARGS_FILE"
exit 0
"#,
    );
    write_executable(
        &fixture.bin_dir().join("codex"),
        r#"#!/bin/sh
exit 99
"#,
    );

    let assert = fixture
        .command()
        .env("CLAUDINE_ARGS_FILE", &args_file)
        .args(["compose", md_file.to_str().unwrap()])
        .assert()
        .code(1);

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    let plain = strip_ansi(&stderr);
    // Non-TTY with no agent hint aborts with the no-agent message;
    // the config favorite is no longer used as a non-TTY fallback.
    assert!(
        plain.contains("didn't specify the Agent"),
        "non-TTY no-agent should abort with the no-agent message; stderr was: {plain}"
    );
}

#[cfg(unix)]
#[test]
fn agent_hint_resolved_early_in_non_tty() {
    // Verifies that an `agent` hint is resolved during preparation
    // (not at launch), so prefix matches like "c" resolve to the
    // first match (Claude) instead of being treated as ambiguous.
    let fixture = CliProcessFixture::named("compose-agent-hint-prefix");
    fixture.seed_user_config();

    let md_file = fixture.cwd().join("test.md");
    fs::write(&md_file, "---\ntitle: test\nagent: c\n---\nPrompt\n").unwrap();

    // Install both claude and codex; "c" resolves to Claude (first prefix match)
    write_executable(&fixture.bin_dir().join("claude"), "#!/bin/sh\nexit 0\n");
    write_executable(&fixture.bin_dir().join("codex"), "#!/bin/sh\nexit 99\n");

    // Write empty stdin via a file to prevent TTY detection
    let stdin_file = fixture.cwd().join("empty-stdin.txt");
    fs::write(&stdin_file, "").unwrap();

    fixture
        .command()
        .pipe_stdin(&stdin_file)
        .unwrap()
        .args(["compose", md_file.to_str().unwrap()])
        .assert()
        .success();
}

#[cfg(unix)]
#[test]
fn unknown_agent_hint_is_non_fatal_and_aborts_in_non_tty() {
    // Invalid `agent` values are no longer fatal during preparation.
    // In non-TTY mode the invalid hint is discarded and the run aborts
    // because no provider can be resolved, mirroring the no-agent
    // non-TTY behavior until the Phase 3 live-path messaging lands.
    let fixture = CliProcessFixture::named("compose-unknown-agent-hint");
    fixture.seed_user_config();

    let md_file = fixture.cwd().join("test.md");
    fs::write(
        &md_file,
        "---\ntitle: test\nagent: unknown-provider\n---\nPrompt\n",
    )
    .unwrap();

    write_executable(&fixture.bin_dir().join("claude"), "#!/bin/sh\nexit 0\n");

    let stdin_file = fixture.cwd().join("empty-stdin.txt");
    fs::write(&stdin_file, "").unwrap();

    fixture
        .command()
        .pipe_stdin(&stdin_file)
        .unwrap()
        .args(["compose", md_file.to_str().unwrap()])
        .assert()
        .code(1)
        .stderr(contains("agent resolution failed"));
}

/// The `--json` dry-run report of `claudine compose doc.md` with only
/// `installed` providers on a fake-only `PATH`, as `(exit code, codes,
/// envelope)`.
#[cfg(unix)]
fn compose_dry_run_report(frontmatter: &str, installed: &[&str]) -> (Option<i32>, Vec<String>, serde_json::Value) {
    let fixture = CliProcessFixture::named("compose-dry-run-agent-list");
    fixture.seed_user_config();
    let md_file = fixture.cwd().join("doc.md");
    fs::write(&md_file, format!("---\n{frontmatter}---\nBODY\n")).unwrap();
    for slug in installed {
        write_executable(&fixture.bin_dir().join(slug), "#!/bin/sh\nexit 0\n");
    }

    // fake-only PATH: the assertions depend on which providers are *not*
    // found, so the fixture bin must be the only place a provider can come
    // from.
    let output = fixture
        .command_builder()
        .fake_only_path()
        .build()
        .args(["compose", "--dry-run", "--json", md_file.to_str().unwrap()])
        .output()
        .unwrap();
    let envelope: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let codes = envelope["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|finding| finding["code"].as_str().unwrap().to_string())
        .collect();
    (output.status.code(), codes, envelope)
}

/// End-to-end (Finding 1): a frontmatter `agent` list resolving to exactly one
/// installed provider selects that provider without prompting. A list whose
/// one installed entry is OpenCode is judged as OpenCode, which needs a model,
/// proving the list resolved rather than staying unresolved.
#[cfg(unix)]
#[test]
fn compose_dry_run_list_one_installed_selects_that_provider() {
    let (code, codes, envelope) = compose_dry_run_report("agent: [claude, gemini]\n", &["claude"]);
    assert_eq!(code, Some(0), "{envelope}");
    assert!(codes.is_empty(), "{envelope}");

    let (code, codes, envelope) = compose_dry_run_report("agent: [opencode, gemini]\n", &["opencode"]);
    assert_eq!(code, Some(2), "{envelope}");
    assert_eq!(codes, ["composition.model_required"], "{envelope}");
}

/// End-to-end (Finding 2): a single-entry all-invalid `agent` list
/// (`agent: [not-real]`) is a list with no installed provider, not the
/// single-invalid scalar finding.
#[cfg(unix)]
#[test]
fn compose_dry_run_single_entry_invalid_list_is_zero_installed() {
    let (code, codes, envelope) = compose_dry_run_report("agent: [not-real]\n", &["goose"]);
    assert_eq!(code, Some(2), "{envelope}");
    assert!(codes.iter().any(|code| code == "composition.provider_not_installed"), "{envelope}");
    assert!(!codes.iter().any(|code| code == "composition.provider_unknown"), "{envelope}");
    assert!(envelope.to_string().contains("not-real"), "the listed suggestion is named: {envelope}");
}

/// End-to-end (Findings 3/4): `--silent` governs status verbosity only — it
/// must not suppress the live no-TTY agent-resolution report, and the run must
/// still abort with a non-zero exit.
#[cfg(unix)]
#[test]
fn compose_silent_does_not_suppress_agent_resolution_report() {
    let fixture = CliProcessFixture::named("compose-silent-agent-report");
    fixture.seed_user_config();

    // No agent hint, no explicit provider → live no-TTY abort.
    let md_file = fixture.cwd().join("doc.md");
    fs::write(&md_file, "---\ntitle: t\n---\nPrompt body\n").unwrap();
    write_executable(&fixture.bin_dir().join("goose"), "#!/bin/sh\nexit 0\n");

    let stdin_file = fixture.cwd().join("empty-stdin.txt");
    fs::write(&stdin_file, "").unwrap();

    let assert = fixture
        .command()
        .pipe_stdin(&stdin_file)
        .unwrap()
        .args(["compose", "--silent", md_file.to_str().unwrap()])
        .assert()
        .code(1);

    let stderr = strip_ansi(&String::from_utf8_lossy(&assert.get_output().stderr));
    assert!(
        stderr.contains("didn't specify the Agent"),
        "--silent must not suppress the agent-resolution report; stderr was:\n{stderr}"
    );
}

// ---------------------------------------------------------------------------
// Per-provider dry-run regression tests (Task 18)
//
// These tests are the structural guard that would have caught the original
// Gemini/Qwen drift: composition pipelines that silently bailed because
// `apply_non_interactive` re-read args before the prompt was injected.
// A successful dry-run (exit 0 + "DRY RUN" in output) proves the full
// extraction → delivery → output pipeline ran without error.
// ---------------------------------------------------------------------------

#[cfg(unix)]
fn assert_direct_wrap_dry_run_delivers_prompt(provider_slug: &str) {
    let fixture = CliProcessFixture::named("direct-wrap-dry-run");
    fixture.seed_user_config();

    // fake-only PATH over an empty fixture bin: the proof is that dry-run
    // neither resolves nor spawns the provider, so nothing may be findable.
    let output = fixture
        .command_builder()
        .fake_only_path()
        .build()
        .env("OPENCODE_MODEL", "test-model")
        .args([provider_slug, "--dry-run", "hello"])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "`claudine {provider_slug} --dry-run hello` failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let normalized = strip_ansi(&combined);
    assert!(
        normalized.contains("DRY RUN"),
        "`claudine {provider_slug} --dry-run hello` did not emit a DRY RUN section:\n{normalized}"
    );
}

macro_rules! direct_wrap_dry_run_test {
    ($name:ident, $provider:literal) => {
        #[cfg(unix)]
        #[test]
        fn $name() {
            assert_direct_wrap_dry_run_delivers_prompt($provider);
        }
    };
}

direct_wrap_dry_run_test!(direct_wrap_dry_run_delivers_prompt_for_claude, "claude");
direct_wrap_dry_run_test!(direct_wrap_dry_run_delivers_prompt_for_codex, "codex");
direct_wrap_dry_run_test!(direct_wrap_dry_run_delivers_prompt_for_gemini, "gemini");
direct_wrap_dry_run_test!(direct_wrap_dry_run_delivers_prompt_for_kimi, "kimi");
direct_wrap_dry_run_test!(direct_wrap_dry_run_delivers_prompt_for_opencode, "opencode");
direct_wrap_dry_run_test!(direct_wrap_dry_run_delivers_prompt_for_qwen, "qwen");
direct_wrap_dry_run_test!(direct_wrap_dry_run_delivers_prompt_for_goose, "goose");
