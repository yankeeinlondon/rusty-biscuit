#![cfg(unix)]

//! Integration tests: compose provider selection, env-agent resolution, interactive prompt seeding, and MCP runtime.
//!
//! Split out of the `wrap_commands.rs` god file; shared fixtures live in
//! `common::wrap`.

use std::fs;
use tempfile::tempdir;
mod common;
use common::wrap::*;
use common::{CliProcessFixture, strip_ansi, write_executable};

#[cfg(unix)]
#[test]
fn explicit_provider_flag_bypasses_chooser() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    let args_path = workspace.path().join("args.txt");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let md_file = workspace.path().join("test.md");
    fs::write(&md_file, "---\ntitle: test\n---\nPrompt body\n").unwrap();

    write_executable(
        &path_dir.join("codex"),
        r#"#!/bin/sh
printf '%s\n' "$@" > "$CLAUDINE_ARGS_FILE"
exit 0
"#,
    );
    write_executable(
        &path_dir.join("claude"),
        r#"#!/bin/sh
exit 99
"#,
    );

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("CLAUDINE_RENDEZVOUS_REPORT", "false")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .env("CLAUDINE_ARGS_FILE", &args_path)
        .current_dir(workspace.path())
        .args([
            "compose",
            "--codex",
            "--exclude",
            "codex",
            md_file.to_str().unwrap(),
        ])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    let plain = strip_ansi(&stderr);
    assert!(
        plain.contains("Codex") && plain.contains("Compose"),
        "should show Codex provider in the Claudine header but stderr was: {plain}"
    );
}

#[cfg(unix)]
#[test]
fn compose_uses_wrapper_grade_execution() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let md_file = workspace.path().join("test.md");
    fs::write(&md_file, "---\ntitle: test\n---\nHello compose\n").unwrap();

    write_executable(
        &path_dir.join("codex"),
        r#"#!/bin/sh
printf 'AGENT=%s\n' "$AGENT" >&2
exit 0
"#,
    );

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("CLAUDINE_RENDEZVOUS_REPORT", "false")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .current_dir(workspace.path())
        .args(["compose", "--codex", md_file.to_str().unwrap()])
        .assert()
        .success();

    // Wrapper-grade execution injects AGENT env
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    assert!(
        stderr.contains("AGENT=codex"),
        "compose should inject AGENT env via wrapper pipeline; stderr was: {stderr}"
    );
}

#[cfg(unix)]
#[test]
fn compose_resolves_env_agent_in_body_template() {
    // Regression: `{{env.AGENT}}` in a compose body must resolve to the
    // chosen provider's slug, since AGENT is now set in the parent
    // process env *before* templates render.
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let md_file = workspace.path().join("test.md");
    fs::write(
        &md_file,
        "---\ntitle: agent env test\n---\nrunning on {{env.AGENT}}\n",
    )
    .unwrap();

    let stdin_path = workspace.path().join("stdin.txt");
    write_executable(
        &path_dir.join("codex"),
        r#"#!/bin/sh
/bin/cat > "$CLAUDINE_STDIN_FILE"
exit 0
"#,
    );

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("CLAUDINE_RENDEZVOUS_REPORT", "false")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .env("CLAUDINE_STDIN_FILE", &stdin_path)
        .current_dir(workspace.path())
        .args(["compose", "--codex", md_file.to_str().unwrap()])
        .assert()
        .success();

    let stdin = fs::read_to_string(&stdin_path).unwrap();
    assert!(
        stdin.contains("running on codex"),
        "{{{{env.AGENT}}}} should resolve to the chosen provider during \
         compose body rendering; prompt was: {stdin:?}"
    );
}

#[cfg(unix)]
#[test]
fn wrapper_resolves_env_agent_in_system_prompt() {
    // Direct wrappers must set AGENT before the parent renders any
    // system-prompt.md template. Use the Claude wrapper because its
    // non-interactive system-prompt delivery writes the composed prompt
    // to a temp file passed via --append-system-prompt-file, which the
    // shim can capture verbatim.
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let system_prompt = workspace.path().join("system-prompt.md");
    fs::write(&system_prompt, "you are running on {{env.AGENT}}\n").unwrap();

    let captured_prompt = workspace.path().join("captured_prompt.txt");
    let captured_args = workspace.path().join("captured_args.txt");
    write_executable(
        &path_dir.join("claude"),
        r#"#!/bin/sh
printf '%s\n' "$@" > "$CLAUDINE_CAPTURED_ARGS"
: > "$CLAUDINE_CAPTURED_PROMPT"
while [ "$#" -gt 0 ]; do
    case "$1" in
        --append-system-prompt-file|--system-prompt-file)
            shift
            if [ -n "$1" ] && [ -f "$1" ]; then
                /bin/cat "$1" >> "$CLAUDINE_CAPTURED_PROMPT"
            else
                printf 'TMPFILE_MISSING:%s\n' "$1" >> "$CLAUDINE_CAPTURED_PROMPT"
            fi
            ;;
    esac
    shift
done
exit 0
"#,
    );

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .env("CLAUDINE_CAPTURED_PROMPT", &captured_prompt)
        .env("CLAUDINE_CAPTURED_ARGS", &captured_args)
        .current_dir(workspace.path())
        .args(["claude", "--", "ping"])
        .assert()
        .success();

    let argv = fs::read_to_string(&captured_args).unwrap_or_default();
    let captured = fs::read_to_string(&captured_prompt).unwrap_or_default();
    assert!(
        captured.contains("running on claude"),
        "system-prompt template should resolve {{{{env.AGENT}}}} to the \
         wrapper's provider slug; argv: {argv:?}; captured prompt: {captured:?}"
    );
}

#[cfg(unix)]
#[test]
fn compose_preflight_error_includes_source_provenance() {
    let fixture = CliProcessFixture::named("compose-preflight-provenance");
    seed_minimal_config(fixture.home());

    // Markdown with a ::shell directive that is NOT whitelisted.
    let md_file = fixture.cwd().join("template.md");
    fs::write(
        &md_file,
        "---\ntitle: provenance test\n---\n::shell curl https://example.com\n",
    )
    .unwrap();

    // Provider binary (should never be reached — preflight should abort first).
    write_executable(
        &fixture.bin_dir().join("codex"),
        "#!/bin/sh\necho 'ERROR: provider should not run' >&2\nexit 99\n",
    );

    // Run without --interactive so preflight has no approval handler →
    // the non-whitelisted command triggers a clear error with provenance.
    let assert = fixture
        .command()
        .args(["compose", "--codex", md_file.to_str().unwrap()])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    let plain = strip_ansi(&stderr);

    // Error message should mention the source file name (provenance).
    assert!(
        plain.contains("template.md"),
        "preflight error should include the source file name for provenance; stderr was:\n{plain}"
    );
    // Error message should mention the denied command.
    assert!(
        plain.contains("curl"),
        "preflight error should identify the denied command; stderr was:\n{plain}"
    );
    // Provider should NOT have run.
    assert!(
        !plain.contains("ERROR: provider should not run"),
        "provider binary should not execute when preflight fails; stderr was:\n{plain}"
    );
}

#[cfg(unix)]
#[test]
fn compose_interactive_claude_seeds_prompt_as_positional_arg() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    let args_path = workspace.path().join("args.txt");
    fs::create_dir_all(&path_dir).unwrap();
    seed_minimal_config(workspace.path());

    let md_file = workspace.path().join("test.md");
    fs::write(&md_file, "---\ntitle: test\n---\n- Hello Claude\n").unwrap();

    write_executable(
        &path_dir.join("claude"),
        r#"#!/bin/sh
printf '%s\n' "$@" > "$CLAUDINE_ARGS_FILE"
exit 0
"#,
    );

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("CLAUDINE_RENDEZVOUS_REPORT", "false")
        .env("HOME", workspace.path())
        .env("PATH", &path_dir)
        .env("CLAUDINE_ARGS_FILE", &args_path)
        .current_dir(workspace.path())
        .args([
            "compose",
            "--interactive",
            "--claude",
            md_file.to_str().unwrap(),
        ])
        .assert()
        .success();

    let args = fs::read_to_string(&args_path).unwrap();
    let lines = args.lines().collect::<Vec<_>>();
    let separator_index = lines
        .iter()
        .position(|line| *line == "--")
        .expect("interactive compose should separate leading-dash Claude prompts with --");
    let prompt_index = lines
        .iter()
        .position(|line| *line == "- Hello Claude")
        .expect("interactive compose should pass Claude the composed prompt as a positional arg");
    assert!(
        separator_index < prompt_index,
        "`--` should appear before the leading-dash prompt; args: {args}"
    );
}

#[cfg(unix)]
#[test]
fn compose_interactive_kimi_seeds_prompt_with_prompt_flag() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    let args_path = workspace.path().join("args.txt");
    fs::create_dir_all(&path_dir).unwrap();

    let md_file = workspace.path().join("test.md");
    fs::write(&md_file, "---\ntitle: test\n---\nHello Kimi\n").unwrap();

    write_executable(
        &path_dir.join("kimi"),
        r#"#!/bin/sh
printf '%s\n' "$@" > "$CLAUDINE_ARGS_FILE"
exit 0
"#,
    );

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("CLAUDINE_RENDEZVOUS_REPORT", "false")
        .env("PATH", &path_dir)
        .env("CLAUDINE_ARGS_FILE", &args_path)
        .current_dir(workspace.path())
        .args([
            "compose",
            "--interactive",
            "--kimi",
            md_file.to_str().unwrap(),
        ])
        .assert()
        .success();

    let args = fs::read_to_string(&args_path).unwrap();
    let collected: Vec<_> = args.lines().collect();
    assert!(
        collected
            .windows(2)
            .any(|window| window == ["--prompt", "Hello Kimi"]),
        "interactive compose should seed Kimi via --prompt; args: {args}"
    );
}

#[cfg(unix)]
#[test]
fn compose_supports_mcp_runtime_and_tag_cleanup() {
    let workspace = tempdir().unwrap();
    let home = workspace.path().join("home");
    let path_dir = workspace.path().join("bin");
    let stdin_path = workspace.path().join("stdin.txt");
    let env_path = workspace.path().join("env.txt");
    fs::create_dir_all(&path_dir).unwrap();
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(home.join(".codex")).unwrap();

    let md_file = workspace.path().join("test.md");
    fs::write(
        &md_file,
        "---\ntitle: test\n---\nUse #calendar for this task\n",
    )
    .unwrap();

    seed_catalog(&home, &[make_server("calendar")]);
    seed_defaults(&home, &["calendar"]);
    seed_empty_provider_state(&home);

    write_executable(
        &path_dir.join("codex"),
        r#"#!/bin/sh
/bin/cat > "$CLAUDINE_STDIN_FILE"
{
  printf 'HOME=%s\n' "$HOME"
} > "$CLAUDINE_ENV_FILE"
exit 0
"#,
    );

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("HOME", &home)
        .env("NO_COLOR", "1")
        .env("CLAUDINE_RENDEZVOUS_REPORT", "false")
        .env("PATH", &path_dir)
        .env("CLAUDINE_STDIN_FILE", &stdin_path)
        .env("CLAUDINE_ENV_FILE", &env_path)
        .current_dir(workspace.path())
        .args(["compose", "--codex", "--mcp", md_file.to_str().unwrap()])
        .assert()
        .success();

    let prompt = fs::read_to_string(&stdin_path).unwrap();
    assert!(
        !prompt.contains("#calendar"),
        "MCP tags should be stripped before prompt delivery; prompt: {prompt}"
    );

    let env_lines = fs::read_to_string(&env_path).unwrap();
    assert!(
        env_lines.contains(&format!("HOME={}", home.join(".claudine").display())),
        "runtime MCP for codex should use a shadow HOME; env: {env_lines}"
    );
}
