//! L1 contract: a provider overlay redirects the provider through its own
//! selector and never moves the user home.
//!
//! Every launch drives the real `claudine` binary against a fake provider that
//! records its environment, so the assertions read what the child actually
//! received. The fixture home is private; no real credential store is touched.
//!
//! Every launch owns a fresh overlay root under `~/.claudine/overlays/<slug>/`,
//! removed when the launch ends, so a test reads the overlay through what the
//! provider recorded while it ran rather than from disk afterwards.
//!
//! The spec is `fixes/2026-09-12-shadow-home/spec.md`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Output;
use crate::common;
use common::wrap::{make_server, seed_catalog, seed_defaults, seed_empty_provider_state};
use common::{CliProcessFixture, write, write_executable};

/// Records the variables under test, then runs the nested tools so a
/// descendant's view is recorded too.
const RECORD_ENV: &str = r#"#!/bin/sh
{
  printf 'HOME=%s\n' "$HOME"
  printf 'USERPROFILE=%s\n' "${USERPROFILE-<unset>}"
  printf 'CODEX_HOME=%s\n' "${CODEX_HOME-<unset>}"
  printf 'CODEX_SQLITE_HOME=%s\n' "${CODEX_SQLITE_HOME-<unset>}"
  printf 'CLAUDE_CONFIG_DIR=%s\n' "${CLAUDE_CONFIG_DIR-<unset>}"
  printf 'CLAUDE_SECURESTORAGE_CONFIG_DIR=%s\n' "${CLAUDE_SECURESTORAGE_CONFIG_DIR-<unset>}"
  if [ -n "$CODEX_HOME" ] && [ -f "$CODEX_HOME/config.toml" ]; then
    printf 'CODEX_CONFIG=%s\n' "$(cat "$CODEX_HOME/config.toml")"
  fi
  if [ -n "$CLAUDE_CONFIG_DIR" ] && [ -f "$CLAUDE_CONFIG_DIR/.claude.json" ]; then
    printf 'CLAUDE_STATE=%s\n' "$(cat "$CLAUDE_CONFIG_DIR/.claude.json")"
  fi
  if [ -n "$CLAUDE_CONFIG_DIR" ] && [ -d "$CLAUDE_CONFIG_DIR" ]; then
    printf 'CLAUDE_CONFIG_DIR_EXISTS=yes\n'
  fi
} > "$CLAUDINE_ENV_FILE"
"$CLAUDINE_NESTED_BIN/run-nested"
exit 0
"#;

/// The ordinary development tools a provider runs. They live in a directory
/// only the provider puts on its `PATH`, so Claudine's own `git` calls never
/// reach them, and they record the home they observe instead of touching a
/// real credential store.
const NESTED_TOOLS: [&str; 3] = ["git", "gpg", "gh"];

/// A nested ordinary tool: records the home variables it observes, byte for
/// byte, in a file named after itself.
const NESTED_TOOL: &str = r#"#!/bin/sh
{
  printf 'HOME=%s\n' "$HOME"
  printf 'USERPROFILE=%s\n' "${USERPROFILE-<unset>}"
  printf 'HOMEDRIVE=%s\n' "${HOMEDRIVE-<unset>}"
  printf 'HOMEPATH=%s\n' "${HOMEPATH-<unset>}"
} > "$CLAUDINE_NESTED_FILE.$(basename "$0")"
exit 0
"#;

/// Runs every nested tool the way a provider's shell tool would.
const RUN_NESTED: &str = r#"#!/bin/sh
PATH="$CLAUDINE_NESTED_BIN:$PATH"
export PATH
git status
gpg --list-keys
gh auth status
"#;

/// Records argv, so the file existing at all proves the child was spawned.
const RECORD_SPAWN: &str = r#"#!/bin/sh
printf '%s\n' "$@" > "$CLAUDINE_ARGS_FILE"
printf 'HOME=%s\n' "$HOME" >> "$CLAUDINE_ARGS_FILE"
exit 0
"#;

struct Recorded {
    child: String,
    /// `(tool, recorded environment)` for each of [`NESTED_TOOLS`]; empty text
    /// when the tool never ran.
    nested: Vec<(&'static str, String)>,
}

/// The variables every fixture command wires for the recording scripts.
fn recording_command(fixture: &CliProcessFixture) -> assert_cmd::Command {
    let nested_bin = fixture.workspace_path().join("nested-bin");
    for tool in NESTED_TOOLS {
        write_executable(&nested_bin.join(tool), NESTED_TOOL);
    }
    write_executable(&nested_bin.join("run-nested"), RUN_NESTED);
    let mut command = fixture.command();
    command
        .env("CLAUDINE_ENV_FILE", fixture.cwd().join("child-env.txt"))
        .env("CLAUDINE_NESTED_FILE", fixture.cwd().join("nested-env"))
        .env("CLAUDINE_NESTED_BIN", &nested_bin)
        .env("USERPROFILE", fixture.home().join("profile with space"));
    command
}

fn recorded(fixture: &CliProcessFixture) -> Option<Recorded> {
    let child = fs::read(fixture.cwd().join("child-env.txt")).ok()?;
    let nested = NESTED_TOOLS
        .iter()
        .map(|tool| {
            let file = fixture.cwd().join(format!("nested-env.{tool}"));
            (*tool, String::from_utf8_lossy(&fs::read(file).unwrap_or_default()).into_owned())
        })
        .collect();
    Some(Recorded {
        child: String::from_utf8_lossy(&child).into_owned(),
        nested,
    })
}

fn launch(fixture: &CliProcessFixture, args: &[&str]) -> (Output, Option<Recorded>) {
    let output = recording_command(fixture).args(args).output().unwrap();
    (output, recorded(fixture))
}

fn line(name: &str, value: impl AsRef<Path>) -> String {
    format!("{name}={}", value.as_ref().display())
}

fn assert_home_preserved(fixture: &CliProcessFixture, recorded: &Recorded) {
    let home = line("HOME", fixture.home());
    let profile = line("USERPROFILE", fixture.home().join("profile with space"));
    let observers = std::iter::once(("provider", &recorded.child))
        .chain(recorded.nested.iter().map(|(tool, text)| (*tool, text)));
    for (who, text) in observers {
        assert!(text.lines().any(|l| l == home), "{who} saw a moved HOME:\n{text}");
        assert!(text.lines().any(|l| l == profile), "{who} saw a moved USERPROFILE:\n{text}");
    }
}

/// The overlay root `selector` named in `recorded`, after checking it is a
/// launch root of its own under `~/.claudine/overlays/<slug>/` — never the
/// legacy `~/.claudine/<agent_offset>` storage — and that the launch's end
/// removed it.
fn launch_root(fixture: &CliProcessFixture, recorded: &str, selector: &str, slug: &str) -> PathBuf {
    let value = value_of(recorded, selector)
        .unwrap_or_else(|| panic!("{selector} was not recorded:\n{recorded}"));
    let root = PathBuf::from(value);
    let launches = fixture.home().join(".claudine").join("overlays").join(slug);
    assert_eq!(
        root.parent(),
        Some(launches.as_path()),
        "{selector} must name a launch root under {}:\n{recorded}",
        launches.display()
    );
    assert!(!root.exists(), "the ended launch left its overlay root {}", root.display());
    root
}

fn success(output: &Output) {
    assert!(
        output.status.success(),
        "claudine failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// `--repo` isolation for Codex: `CODEX_HOME` names the overlay built from
/// `~/.codex`, SQLite stays at the pre-overlay root, and neither the provider
/// nor a tool it runs sees a different home.
#[test]
fn codex_repo_overlay_uses_codex_home_and_leaves_the_user_home() {
    let fixture = CliProcessFixture::named("overlay-home-codex-repo");
    fixture.seed_user_config();
    write(&fixture.home().join(".codex").join("config.toml"), "model = \"fixture\"\n");
    write_executable(&fixture.bin_dir().join("codex"), RECORD_ENV);

    let (output, recorded) = launch(&fixture, &["codex", "--repo", "--", "--version"]);
    success(&output);
    let recorded = recorded.expect("codex was not spawned");

    assert_home_preserved(&fixture, &recorded);
    let child = &recorded.child;
    launch_root(&fixture, child, "CODEX_HOME", "codex");
    assert!(
        child
            .lines()
            .any(|l| l == line("CODEX_SQLITE_HOME", fixture.home().join(".codex"))),
        "{child}"
    );
    assert!(
        child.lines().any(|l| l == "CODEX_CONFIG=model = \"fixture\""),
        "the overlay must carry the user's Codex settings:\n{child}"
    );
}

/// The roots the native-Windows L2 launch names, because the known-folder home
/// ignores its fixture `USERPROFILE`: an explicit `CODEX_HOME` source and a
/// `CLAUDINE_OVERLAY_DIR` storage parent. Nothing lands under the home.
#[test]
fn explicit_codex_home_and_overlay_dir_keep_the_overlay_out_of_the_home() {
    let fixture = CliProcessFixture::named("overlay-home-explicit-roots");
    fixture.seed_user_config();
    let source = fixture.workspace_path().join("codex-source");
    let launches = fixture.workspace_path().join("overlay-launches");
    write(&source.join("config.toml"), "model = \"explicit\"\n");
    write_executable(&fixture.bin_dir().join("codex"), RECORD_ENV);

    let output = recording_command(&fixture)
        .env("CODEX_HOME", &source)
        .env("CLAUDINE_OVERLAY_DIR", &launches)
        .args(["codex", "--repo", "--", "--version"])
        .output()
        .unwrap();
    success(&output);
    let recorded = recorded(&fixture).expect("codex was not spawned");

    assert_home_preserved(&fixture, &recorded);
    let child = &recorded.child;
    let root = PathBuf::from(value_of(child, "CODEX_HOME").expect("CODEX_HOME recorded"));
    assert_eq!(root.parent(), Some(launches.join("codex").as_path()), "{child}");
    assert!(!root.exists(), "the ended launch left its overlay root {}", root.display());
    assert!(child.lines().any(|l| l == "CODEX_CONFIG=model = \"explicit\""), "{child}");
    assert!(child.lines().any(|l| l == line("CODEX_SQLITE_HOME", &source)), "{child}");
    assert!(
        !fixture.home().join(".claudine").join("overlays").exists(),
        "overlay storage was created under the home despite the override"
    );
}

/// Claude reads `.claude.json` and its credential store relative to
/// `CLAUDE_CONFIG_DIR`, so the overlay carries the state file and pins the
/// credential store to its default entry.
#[test]
fn claude_repo_overlay_keeps_state_and_credential_store_reachable() {
    let fixture = CliProcessFixture::named("overlay-home-claude-repo");
    fixture.seed_user_config();
    write(&fixture.home().join(".claude").join("settings.json"), "{}\n");
    write(&fixture.home().join(".claude.json"), "{\"userID\":\"fixture\"}");
    write_executable(&fixture.bin_dir().join("claude"), RECORD_ENV);

    let (output, recorded) = launch(&fixture, &["claude", "--repo", "--", "--version"]);
    success(&output);
    let recorded = recorded.expect("claude was not spawned");

    assert_home_preserved(&fixture, &recorded);
    let child = &recorded.child;
    launch_root(&fixture, child, "CLAUDE_CONFIG_DIR", "claude");
    assert!(
        child.lines().any(|l| l == "CLAUDE_SECURESTORAGE_CONFIG_DIR="),
        "the credential store must stay at its default entry:\n{child}"
    );
    assert!(
        child.lines().any(|l| l == "CLAUDE_STATE={\"userID\":\"fixture\"}"),
        "the overlay must carry the user's .claude.json:\n{child}"
    );
}

/// A launch that needs no overlay sets no selector at all.
#[test]
fn a_launch_without_overlay_reasons_sets_no_selector() {
    let fixture = CliProcessFixture::named("overlay-home-none");
    fixture.seed_user_config();
    write_executable(&fixture.bin_dir().join("claude"), RECORD_ENV);

    let (output, recorded) = launch(&fixture, &["claude", "--", "--version"]);
    success(&output);
    let recorded = recorded.expect("claude was not spawned");

    assert_home_preserved(&fixture, &recorded);
    assert!(recorded.child.contains("CLAUDE_CONFIG_DIR=<unset>"), "{}", recorded.child);
    assert!(!fixture.home().join(".claudine").join("overlays").exists());
}

/// Audit refusal row 1: Antigravity has no provider-owned selector, so `--repo`
/// refuses before spawn — and only that mode. A plain launch still runs with
/// the user's home.
#[test]
fn antigravity_refuses_only_the_repo_mode() {
    let fixture = CliProcessFixture::named("overlay-home-antigravity");
    fixture.seed_user_config();
    let args_file = fixture.cwd().join("args.txt");
    write_executable(&fixture.bin_dir().join("agy"), RECORD_SPAWN);

    let refused = fixture
        .command()
        .env("CLAUDINE_ARGS_FILE", &args_file)
        .args(["antigravity", "--repo", "summarize"])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&refused.stderr);
    assert!(!refused.status.success(), "--repo must not launch:\n{stderr}");
    assert!(stderr.contains("provider.overlay_unsupported"), "{stderr}");
    assert!(stderr.contains("--repo"), "the refusal names the mode:\n{stderr}");
    assert!(!stderr.to_lowercase().contains("credential"), "{stderr}");
    assert!(!args_file.exists(), "the provider was spawned despite the refusal");

    fixture
        .command()
        .env("CLAUDINE_ARGS_FILE", &args_file)
        .args(["antigravity", "summarize"])
        .output()
        .unwrap();
    let recorded = fs::read_to_string(&args_file).expect("a plain launch must still spawn");
    assert!(
        recorded.lines().any(|l| l == line("HOME", fixture.home())),
        "{recorded}"
    );
}

/// An overlay that cannot be built stops the launch with the typed diagnostic.
/// The old behavior warned and launched with `HOME=/dev/null`.
#[test]
fn a_failed_overlay_stops_the_launch_without_a_null_home() {
    let fixture = CliProcessFixture::named("overlay-home-failed");
    fixture.seed_user_config();
    write(&fixture.home().join(".claude").join("settings.json"), "{}\n");
    // The storage root cannot be created where a regular file already sits.
    write(&fixture.home().join(".claudine").join("overlays"), "not a directory");
    write_executable(&fixture.bin_dir().join("claude"), RECORD_ENV);

    let (output, recorded) = launch(&fixture, &["claude", "--repo", "--", "--version"]);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success(), "a failed overlay must not launch:\n{stderr}");
    assert!(recorded.is_none(), "the provider was spawned despite the failure");
    assert!(stderr.contains("provider.overlay_failed"), "{stderr}");
    assert!(!stderr.contains("/dev/null"), "{stderr}");
    assert!(!stderr.contains("shadow HOME"), "{stderr}");
    assert!(!stderr.to_lowercase().contains("credential"), "{stderr}");
}

/// A provider root the user named but that does not exist is the same
/// pre-spawn failure, at the source-root stage, and creates no overlay storage.
#[test]
fn a_missing_explicit_source_root_stops_the_launch_before_building_storage() {
    let fixture = CliProcessFixture::named("overlay-home-no-source");
    fixture.seed_user_config();
    write_executable(&fixture.bin_dir().join("claude"), RECORD_ENV);
    let named = fixture.home().join("named claude");

    let output = recording_command(&fixture)
        .env("CLAUDE_CONFIG_DIR", &named)
        .args(["claude", "--repo", "--", "--version"])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success(), "{stderr}");
    assert!(recorded(&fixture).is_none(), "the provider was spawned despite the failure");
    assert!(stderr.contains("provider.overlay_failed"), "{stderr}");
    assert!(!fixture.home().join(".claudine").join("overlays").exists());
    assert!(!named.exists(), "the named source root was created");
}

/// A provider that has never run has no default root. The overlay starts empty
/// and the launch proceeds, as it did before the overlay rework.
#[test]
fn a_missing_default_source_root_launches_with_an_empty_overlay() {
    let fixture = CliProcessFixture::named("overlay-home-no-default-source");
    fixture.seed_user_config();
    write_executable(&fixture.bin_dir().join("claude"), RECORD_ENV);

    let (output, recorded) = launch(&fixture, &["claude", "--repo", "--", "--version"]);
    success(&output);
    let recorded = recorded.expect("claude was not spawned");

    assert_home_preserved(&fixture, &recorded);
    launch_root(&fixture, &recorded.child, "CLAUDE_CONFIG_DIR", "claude");
    assert!(
        recorded.child.lines().any(|l| l == "CLAUDE_CONFIG_DIR_EXISTS=yes"),
        "the overlay was not created:\n{}",
        recorded.child
    );
    assert!(!fixture.home().join(".claude").exists(), "the user's root was created");
}

/// Records the config file each MCP-injecting provider reads, where its
/// selector says to read it from, plus the argv it received.
const RECORD_MCP: &str = r#"#!/bin/sh
{
  printf 'HOME=%s\n' "$HOME"
  printf 'USERPROFILE=%s\n' "${USERPROFILE-<unset>}"
  printf 'CODEX_HOME=%s\n' "${CODEX_HOME-<unset>}"
  printf 'CODEX_SQLITE_HOME=%s\n' "${CODEX_SQLITE_HOME-<unset>}"
  printf 'GEMINI_CLI_HOME=%s\n' "${GEMINI_CLI_HOME-<unset>}"
  printf 'OPENCODE_CONFIG_DIR=%s\n' "${OPENCODE_CONFIG_DIR-<unset>}"
  printf 'ARGS=%s\n' "$*"
  if [ -n "$CODEX_HOME" ]; then
    for state in "$CODEX_HOME"/*.sqlite*; do
      if [ -e "$state" ]; then printf 'OVERLAY_STATE=%s\n' "$(basename "$state")"; fi
    done
  fi
  if [ -n "$CODEX_HOME" ] && [ -f "$CODEX_HOME/config.toml" ]; then
    printf 'CODEX_CONFIG<<\n%s\n>>\n' "$(cat "$CODEX_HOME/config.toml")"
  fi
  if [ -n "$GEMINI_CLI_HOME" ] && [ -f "$GEMINI_CLI_HOME/.gemini/settings.json" ]; then
    printf 'GEMINI_SETTINGS<<\n%s\n>>\n' "$(cat "$GEMINI_CLI_HOME/.gemini/settings.json")"
  fi
  if [ -n "$GEMINI_CLI_HOME" ]; then
    for sidecar in mcp-server-enablement.json mcp-oauth-tokens.json; do
      copy="$GEMINI_CLI_HOME/.gemini/$sidecar"
      if [ -f "$copy" ] && [ ! -L "$copy" ]; then
        printf 'GEMINI_SIDECAR=%s:%s\n' "$sidecar" "$(cat "$copy")"
      fi
    done
  fi
  if [ -n "$OPENCODE_CONFIG_CONTENT" ]; then
    printf 'OPENCODE_CONFIG_CONTENT=%s\n' "$OPENCODE_CONFIG_CONTENT"
  fi
  printf 'KILO_CONFIG_DIR=%s\n' "${KILO_CONFIG_DIR-<unset>}"
  if [ -n "$KILO_CONFIG_CONTENT" ]; then
    printf 'KILO_CONFIG_CONTENT=%s\n' "$KILO_CONFIG_CONTENT"
  fi
} > "$CLAUDINE_ENV_FILE"
"$CLAUDINE_NESTED_BIN/run-nested"
exit 0
"#;

fn seed_mcp_catalog(fixture: &CliProcessFixture, ids: &[&str]) {
    let servers: Vec<_> = ids.iter().map(|id| make_server(id)).collect();
    seed_catalog(fixture.home(), &servers);
    seed_defaults(fixture.home(), ids);
    seed_empty_provider_state(fixture.home());
}

/// Audit F1 regression: `codex --mcp` wrote `~/.claudine/.codex/.codex/config.toml`,
/// one level below `$CODEX_HOME`, so Codex never saw the injected servers. The
/// injected file must be the one `CODEX_HOME` names, keep the user's settings,
/// and leave the user's own `config.toml` untouched — twice in a row, each in a
/// root of its own that the launch's end removes.
#[test]
fn codex_mcp_injects_servers_into_the_config_codex_home_names() {
    let fixture = CliProcessFixture::named("overlay-home-codex-mcp");
    fixture.seed_user_config();
    let user_config = fixture.home().join(".codex").join("config.toml");
    write(&user_config, "model = \"fixture\"\n");
    write(&fixture.home().join(".codex").join("state_5.sqlite"), "live");
    seed_mcp_catalog(&fixture, &["calendar"]);
    write_executable(&fixture.bin_dir().join("codex"), RECORD_MCP);

    let mut roots = Vec::new();
    for attempt in 1..=2 {
        let (output, recorded) =
            launch(&fixture, &["codex", "--mcp", "--", "exec", "fix #calendar bugs"]);
        success(&output);
        let recorded = recorded.expect("codex was not spawned");
        assert_home_preserved(&fixture, &recorded);

        let child = &recorded.child;
        roots.push(launch_root(&fixture, child, "CODEX_HOME", "codex"));
        assert!(
            child.contains("[mcp_servers.calendar]"),
            "attempt {attempt}: Codex must read the injected server from $CODEX_HOME:\n{child}"
        );
        assert!(
            child.contains("model = \"fixture\""),
            "attempt {attempt}: injection must keep the user's Codex settings:\n{child}"
        );
        assert_eq!(
            fs::read_to_string(&user_config).unwrap(),
            "model = \"fixture\"\n",
            "attempt {attempt}: injection wrote through to the user's config"
        );
        assert!(
            child.lines().any(|l| l == line("CODEX_SQLITE_HOME", fixture.home().join(".codex"))),
            "attempt {attempt}: SQLite state must stay at the pre-overlay root:\n{child}"
        );
        assert!(!child.contains("OVERLAY_STATE="), "attempt {attempt}: SQLite in the overlay:\n{child}");
    }
    assert_ne!(roots[0], roots[1], "two launches shared one overlay root");
}

/// Phase 11 regression, found by `level2_lifecycle_control`'s MCP resume rows:
/// `codex --mcp` for a user with no `~/.codex` refused with "provider source
/// root does not exist". The servers must reach the config `CODEX_HOME` names.
#[test]
fn codex_mcp_without_a_codex_root_injects_into_an_empty_overlay() {
    let fixture = CliProcessFixture::named("overlay-home-codex-mcp-fresh");
    fixture.seed_user_config();
    seed_mcp_catalog(&fixture, &["calendar"]);
    write_executable(&fixture.bin_dir().join("codex"), RECORD_MCP);
    assert!(!fixture.home().join(".codex").exists(), "fixture check: no Codex root");

    let (output, recorded) = launch(&fixture, &["codex", "--mcp", "--", "exec", "fix #calendar bugs"]);
    success(&output);
    let recorded = recorded.expect("codex was not spawned");
    assert_home_preserved(&fixture, &recorded);

    let child = &recorded.child;
    launch_root(&fixture, child, "CODEX_HOME", "codex");
    assert!(
        child.contains("[mcp_servers.calendar]"),
        "Codex must read the injected server from $CODEX_HOME:\n{child}"
    );
    assert!(
        child.lines().any(|l| l == line("CODEX_SQLITE_HOME", fixture.home().join(".codex"))),
        "SQLite state must stay at the pre-overlay root:\n{child}"
    );
}

/// Audit F1 for the `ParentOfProviderDir` shape: Gemini reads
/// `$GEMINI_CLI_HOME/.gemini/settings.json`. Its MCP sidecars are private copies
/// in that same directory, so Gemini's writes never reach the user's files.
#[test]
fn gemini_mcp_injects_servers_under_the_gemini_cli_home_root() {
    let fixture = CliProcessFixture::named("overlay-home-gemini-mcp");
    fixture.seed_user_config();
    let user_root = fixture.home().join(".gemini");
    write(&user_root.join("settings.json"), "{\"theme\":\"fixture\"}");
    write(&user_root.join("mcp-server-enablement.json"), "{\"old\":false}");
    write(&user_root.join("mcp-oauth-tokens.json"), "[]");
    seed_mcp_catalog(&fixture, &["linear"]);
    write_executable(&fixture.bin_dir().join("gemini"), RECORD_MCP);

    let (output, recorded) = launch(
        &fixture,
        &["gemini", "--mcp", "--", "--prompt", "fix #linear auth"],
    );
    success(&output);
    let recorded = recorded.expect("gemini was not spawned");
    assert_home_preserved(&fixture, &recorded);

    let child = &recorded.child;
    launch_root(&fixture, child, "GEMINI_CLI_HOME", "gemini");
    assert!(child.contains("--allowed-mcp-server-names linear"), "{child}");
    let settings = child
        .split("GEMINI_SETTINGS<<\n")
        .nth(1)
        .and_then(|rest| rest.split("\n>>").next())
        .unwrap_or_else(|| panic!("Gemini found no settings under $GEMINI_CLI_HOME:\n{child}"));
    let settings: serde_json::Value = serde_json::from_str(settings).unwrap();
    assert!(settings["mcpServers"]["linear"].is_object(), "{settings}");
    assert_eq!(settings["theme"], "fixture", "{settings}");

    for (sidecar, content) in [
        ("mcp-server-enablement.json", "{\"old\":false}"),
        ("mcp-oauth-tokens.json", "[]"),
    ] {
        assert!(
            child.lines().any(|l| l.starts_with(&format!("GEMINI_SIDECAR={sidecar}:"))),
            "{sidecar} must be a private copy, not a link:\n{child}"
        );
        assert_eq!(fs::read_to_string(user_root.join(sidecar)).unwrap(), content);
    }
    assert_eq!(
        fs::read_to_string(user_root.join("settings.json")).unwrap(),
        "{\"theme\":\"fixture\"}",
        "injection wrote through to the user's settings"
    );
}

/// L1 test 7: OpenCode's MCP injection is inline, so no overlay directory and
/// no `OPENCODE_CONFIG_DIR` selector appear.
#[test]
fn opencode_mcp_injects_inline_without_an_overlay() {
    let fixture = CliProcessFixture::named("overlay-home-opencode-mcp");
    fixture.seed_user_config();
    write(&fixture.home().join(".opencode").join("opencode.json"), "{}");
    seed_mcp_catalog(&fixture, &["github"]);
    write_executable(&fixture.bin_dir().join("opencode"), RECORD_MCP);

    let output = recording_command(&fixture)
        .env("OPENCODE_MODEL", "test-model")
        .args(["opencode", "--mcp", "--", "run", "debug #github sync"])
        .output()
        .unwrap();
    success(&output);
    let recorded = recorded(&fixture).expect("opencode was not spawned");
    assert_home_preserved(&fixture, &recorded);
    let child = &recorded.child;

    assert!(child.contains("OPENCODE_CONFIG_DIR=<unset>"), "{child}");
    assert!(child.contains("\"github\""), "the server must arrive inline:\n{child}");
    let overlays = fixture.home().join(".claudine").join("overlays");
    assert!(!overlays.exists(), "OpenCode MCP built overlay storage at {}", overlays.display());
}

/// The researched `KILO_CONFIG_CONTENT` document a `--mcp` launch with one
/// catalog `server` and a user-exported `{"theme":"fixture"}` must hand Kilo.
fn expected_kilo_config(server: &str) -> serde_json::Value {
    serde_json::json!({
        "theme": "fixture",
        "mcp": {
            server: { "type": "local", "command": ["npx", "-y", format!("@test/{server}")] }
        }
    })
}

/// The `KILO_CONFIG_CONTENT` value `recorded` holds, parsed.
fn kilo_config(recorded: &str) -> serde_json::Value {
    let raw = value_of(recorded, "KILO_CONFIG_CONTENT")
        .unwrap_or_else(|| panic!("Kilo received no KILO_CONFIG_CONTENT:\n{recorded}"));
    serde_json::from_str(raw).unwrap()
}

/// Kilo's published `mcp: composable_injection` verdict holds at the process
/// boundary: `kilo --mcp` launches, the servers arrive inline in the researched
/// `KILO_CONFIG_CONTENT` shape merged over the user's exported config, and no
/// overlay directory or `KILO_CONFIG_DIR` selector appears.
#[test]
fn kilo_mcp_injects_inline_without_an_overlay() {
    let fixture = CliProcessFixture::named("overlay-home-kilo-mcp");
    fixture.seed_user_config();
    seed_mcp_catalog(&fixture, &["github"]);
    write_executable(&fixture.bin_dir().join("kilo"), RECORD_MCP);

    let output = recording_command(&fixture)
        .env("KILO_CONFIG_CONTENT", r#"{"theme":"fixture"}"#)
        .args(["kilo", "--mcp", "--", "run", "debug #github sync"])
        .output()
        .unwrap();
    success(&output);
    let recorded = recorded(&fixture).expect("kilo was not spawned");
    assert_home_preserved(&fixture, &recorded);
    let child = &recorded.child;

    assert!(child.lines().any(|l| l == "KILO_CONFIG_DIR=<unset>"), "{child}");
    assert!(!child.contains("OPENCODE_CONFIG_CONTENT="), "{child}");
    assert_eq!(kilo_config(child), expected_kilo_config("github"));
    let overlays = fixture.home().join(".claudine").join("overlays");
    assert!(!overlays.exists(), "Kilo MCP built overlay storage at {}", overlays.display());
}

/// The composition route hands the injector the same provider-visible root:
/// `compose --gemini --mcp` lands its settings where `GEMINI_CLI_HOME` points.
#[test]
fn compose_gemini_mcp_injects_servers_under_the_gemini_cli_home_root() {
    let fixture = CliProcessFixture::named("overlay-home-compose-gemini-mcp");
    fixture.seed_user_config();
    write(&fixture.home().join(".gemini").join("settings.json"), "{}");
    seed_mcp_catalog(&fixture, &["linear"]);
    write_executable(&fixture.bin_dir().join("gemini"), RECORD_MCP);
    let document = fixture.cwd().join("task.md");
    write(&document, "---\ntitle: task\n---\nUse #linear for this task\n");

    let (output, recorded) = launch(
        &fixture,
        &["compose", "--gemini", "--mcp", document.to_str().unwrap()],
    );
    success(&output);
    let recorded = recorded.expect("gemini was not spawned");
    assert_home_preserved(&fixture, &recorded);
    let child = &recorded.child;
    launch_root(&fixture, child, "GEMINI_CLI_HOME", "gemini");
    assert!(
        child.contains("GEMINI_SETTINGS<<") && child.contains("\"linear\""),
        "Gemini must read the injected server from $GEMINI_CLI_HOME/.gemini:\n{child}"
    );
}

// -- provider transitions (Invariant 7) ----------------------------------------------

/// Records the overlay variables and the MCP configuration each provider can
/// read, per provider, so two attempts in one run keep separate evidence.
/// Codex fails, which hands the run to the `failure` stack; on the retry route
/// it first moves the document's `agent:` to `$CLAUDINE_RETARGET`.
const RECORD_TRANSITION: &str = r#"#!/bin/sh
name=$(basename "$0")
{
  printf 'HOME=%s\n' "$HOME"
  printf 'CODEX_HOME=%s\n' "${CODEX_HOME-<unset>}"
  printf 'CODEX_SQLITE_HOME=%s\n' "${CODEX_SQLITE_HOME-<unset>}"
  printf 'GEMINI_CLI_HOME=%s\n' "${GEMINI_CLI_HOME-<unset>}"
  printf 'OPENCODE_CONFIG_DIR=%s\n' "${OPENCODE_CONFIG_DIR-<unset>}"
  if [ -n "$CODEX_HOME" ] && [ -f "$CODEX_HOME/config.toml" ]; then
    printf 'CODEX_CONFIG<<\n%s\n>>\n' "$(cat "$CODEX_HOME/config.toml")"
  fi
  if [ -n "$GEMINI_CLI_HOME" ] && [ -f "$GEMINI_CLI_HOME/.gemini/settings.json" ]; then
    printf 'GEMINI_SETTINGS<<\n%s\n>>\n' "$(cat "$GEMINI_CLI_HOME/.gemini/settings.json")"
  fi
  if [ -n "$OPENCODE_CONFIG_CONTENT" ]; then
    printf 'OPENCODE_CONFIG_CONTENT=%s\n' "$OPENCODE_CONFIG_CONTENT"
  fi
  printf 'KILO_CONFIG_DIR=%s\n' "${KILO_CONFIG_DIR-<unset>}"
  if [ -n "$KILO_CONFIG_CONTENT" ]; then
    printf 'KILO_CONFIG_CONTENT=%s\n' "$KILO_CONFIG_CONTENT"
  fi
} > "$CLAUDINE_ENV_FILE.$name"
cat > /dev/null
if [ "$name" = codex ]; then
  if [ -n "$CLAUDINE_RETARGET" ]; then
    sed -i.bak "s/^agent: codex$/agent: $CLAUDINE_RETARGET/" "$CLAUDINE_ROUTER"
  fi
  exit 1
fi
exit 0
"#;

/// How the run leaves Codex.
#[derive(Clone, Copy)]
enum Transition {
    /// A `failure` proxy to a target document, which re-prepares the target
    /// through the composition pipeline.
    Proxy,
    /// A `failure` retry of a document whose `agent:` the failed attempt
    /// moved, which rebuilds the launch plan inside the harness loop.
    Retry,
}

/// Run a `--mcp` composition that opens on Codex and moves to a `target`
/// provider from `failure`, returning each provider's recorded environment.
fn codex_mcp_transition(
    fixture: &CliProcessFixture,
    transition: Transition,
    target: &str,
    ambient: &[(&str, &Path)],
) -> (String, String) {
    fixture.seed_user_config();
    write(&fixture.home().join(".codex").join("config.toml"), "model = \"fixture\"\n");
    write(&fixture.home().join(".gemini").join("settings.json"), "{\"theme\":\"fixture\"}");
    seed_mcp_catalog(fixture, &["linear"]);
    for provider in ["codex", target] {
        write_executable(&fixture.bin_dir().join(provider), RECORD_TRANSITION);
    }
    let router = fixture.cwd().join("router.md");
    let failure = match transition {
        Transition::Proxy => "{proxy: './target.md'}",
        Transition::Retry => "{retry: 1}",
    };
    write(
        &router,
        &format!(
            "---\nagent: codex\nfailure:\n  stack:\n    - action: {failure}\n---\nOpen on Codex with #linear.\n"
        ),
    );
    write(
        &fixture.cwd().join("target.md"),
        &format!("---\nagent: {target}\n---\nFinish with #linear.\n"),
    );

    let env_file = fixture.cwd().join("child-env");
    let mut command = fixture.command();
    command
        .env("CLAUDINE_ENV_FILE", &env_file)
        .env("PLAYA_DRY_RUN", "1")
        // OpenCode refuses a non-interactive run without a model.
        .env("OPENCODE_MODEL", "test-model")
        .args(["compose", "--mcp", router.to_str().unwrap()]);
    if let Transition::Retry = transition {
        command.env("CLAUDINE_RETARGET", target).env("CLAUDINE_ROUTER", &router);
    }
    for (name, value) in ambient {
        command.env(name, value);
    }
    let output = command.output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    let read = |name: &str| {
        fs::read_to_string(env_file.with_extension(name))
            .unwrap_or_else(|_| panic!("{name} was not spawned; stderr:\n{stderr}"))
    };
    (read("codex"), read(target))
}

/// Checkpoint 8 and L1 test 11: a Codex → OpenCode proxy or retry hands OpenCode no
/// Codex selector and no pinned Codex SQLite state, and OpenCode receives its
/// servers inline. Codex itself ran under its overlay, and neither provider saw
/// a moved home.
#[test]
fn a_codex_to_opencode_transition_leaves_no_codex_selector_in_the_opencode_child() {
    for (transition, name) in [(Transition::Proxy, "proxy"), (Transition::Retry, "retry")] {
        let fixture = CliProcessFixture::named(&format!("overlay-home-{name}-codex-opencode"));
        let (codex, opencode) = codex_mcp_transition(&fixture, transition, "opencode", &[]);

        launch_root(&fixture, &codex, "CODEX_HOME", "codex");
        assert!(
            codex.lines().any(|l| l == line("CODEX_SQLITE_HOME", fixture.home().join(".codex"))),
            "{codex}"
        );
        let home = line("HOME", fixture.home());
        for (who, text) in [("codex", &codex), ("opencode", &opencode)] {
            assert!(text.lines().any(|l| l == home), "{who} saw a moved HOME:\n{text}");
        }
        for unset in ["CODEX_HOME=<unset>", "CODEX_SQLITE_HOME=<unset>", "OPENCODE_CONFIG_DIR=<unset>"] {
            assert!(opencode.lines().any(|l| l == unset), "{unset}:\n{opencode}");
        }
        assert!(
            opencode.contains("OPENCODE_CONFIG_CONTENT=") && opencode.contains("linear"),
            "{name}: OpenCode's servers arrive inline:\n{opencode}"
        );
    }
}

/// The same transition restores explicit ambient Codex selectors verbatim
/// (paths with a space) rather than deleting them: they are the user's roots,
/// not the overlay's.
#[test]
fn a_codex_to_opencode_transition_restores_explicit_ambient_codex_selectors() {
    for (transition, name) in [(Transition::Proxy, "proxy"), (Transition::Retry, "retry")] {
        let fixture = CliProcessFixture::named(&format!("overlay-home-{name}-codex-ambient"));
        let codex_home = fixture.home().join("my codex");
        let sqlite_home = fixture.home().join("my sqlite");
        fs::create_dir_all(&codex_home).unwrap();
        fs::create_dir_all(&sqlite_home).unwrap();
        let (codex, opencode) = codex_mcp_transition(
            &fixture,
            transition,
            "opencode",
            &[("CODEX_HOME", &codex_home), ("CODEX_SQLITE_HOME", &sqlite_home)],
        );

        launch_root(&fixture, &codex, "CODEX_HOME", "codex");
        assert!(codex.lines().any(|l| l == line("CODEX_SQLITE_HOME", &sqlite_home)), "{codex}");
        assert!(opencode.lines().any(|l| l == line("CODEX_HOME", &codex_home)), "{opencode}");
        assert!(
            opencode.lines().any(|l| l == line("CODEX_SQLITE_HOME", &sqlite_home)),
            "{name}: {opencode}"
        );
    }
}

/// A Codex → Kilo proxy re-prepares Kilo through the composition MCP fold, and
/// a retry rebuilds its launch plan; both hand Kilo its servers inline, merged
/// over the user's exported `KILO_CONFIG_CONTENT`, with no Codex selector.
#[test]
fn a_codex_to_kilo_transition_injects_inline_kilo_config() {
    for (transition, name) in [(Transition::Proxy, "proxy"), (Transition::Retry, "retry")] {
        let fixture = CliProcessFixture::named(&format!("overlay-home-{name}-codex-kilo"));
        let (_codex, kilo) = codex_mcp_transition(
            &fixture,
            transition,
            "kilo",
            &[("KILO_CONFIG_CONTENT", Path::new(r#"{"theme":"fixture"}"#))],
        );

        for unset in ["CODEX_HOME=<unset>", "CODEX_SQLITE_HOME=<unset>", "KILO_CONFIG_DIR=<unset>"] {
            assert!(kilo.lines().any(|l| l == unset), "{name}: {unset}:\n{kilo}");
        }
        assert_eq!(kilo_config(&kilo), expected_kilo_config("linear"), "{name}");
    }
}

/// A Codex → Gemini proxy or retry builds Gemini's own overlay and injects the servers
/// where `GEMINI_CLI_HOME` makes Gemini read them — not into the Codex overlay
/// the invocation opened with (Phase 7's `rebuild_mcp` finding).
#[test]
fn a_codex_to_gemini_transition_injects_into_the_gemini_overlay() {
    for (transition, name) in [(Transition::Proxy, "proxy"), (Transition::Retry, "retry")] {
        let fixture = CliProcessFixture::named(&format!("overlay-home-{name}-codex-gemini"));
        let (codex, gemini) = codex_mcp_transition(&fixture, transition, "gemini", &[]);

        assert!(codex.contains("[mcp_servers.linear]"), "Codex read its injected server:\n{codex}");
        let codex_root = launch_root(&fixture, &codex, "CODEX_HOME", "codex");
        let gemini_root = launch_root(&fixture, &gemini, "GEMINI_CLI_HOME", "gemini");
        assert!(gemini.lines().any(|l| l == "CODEX_HOME=<unset>"), "{gemini}");
        assert!(gemini.lines().any(|l| l == "CODEX_SQLITE_HOME=<unset>"), "{gemini}");
        assert!(gemini.lines().any(|l| l == line("HOME", fixture.home())), "{gemini}");
        let settings = gemini
            .split("GEMINI_SETTINGS<<\n")
            .nth(1)
            .and_then(|rest| rest.split("\n>>").next())
            .unwrap_or_else(|| panic!("Gemini found no settings under $GEMINI_CLI_HOME:\n{gemini}"));
        let settings: serde_json::Value = serde_json::from_str(settings).unwrap();
        assert!(settings["mcpServers"]["linear"].is_object(), "{settings}");
        assert_eq!(settings["theme"], "fixture", "{settings}");
        assert_ne!(codex_root, gemini_root, "{name}: Gemini ran in the Codex overlay");
        assert!(
            !codex.contains("GEMINI_SETTINGS<<"),
            "{name}: Gemini's settings were written into the Codex overlay:\n{codex}"
        );
    }
}

/// Collapses the prose word-wrap and strips styling, so a phrase or path is
/// found wherever the terminal width broke the line.
fn flattened(output: &Output) -> String {
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let plain = biscuit_terminal::discovery::eval::strip_ansi_codes(&text);
    plain.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Under `--repo --perf` the user-facing output describes a provider overlay:
/// the info line names Codex's selector and the directory Codex reads from and
/// says the home is unchanged, and the perf tree labels the child-env substage
/// `provider overlay`. Nothing mentions a shadow home or credentials.
#[test]
fn codex_repo_perf_output_names_the_provider_overlay_not_a_home() {
    let fixture = CliProcessFixture::named("overlay-home-vocabulary");
    fixture.seed_user_config();
    write(&fixture.home().join(".codex").join("config.toml"), "model = \"fixture\"\n");
    write_executable(&fixture.bin_dir().join("codex"), RECORD_ENV);

    let (output, recorded) = launch(&fixture, &["codex", "--repo", "--perf", "--", "--version"]);
    success(&output);
    let recorded = recorded.expect("codex was not spawned");
    assert_home_preserved(&fixture, &recorded);

    let text = flattened(&output);
    let overlay = launch_root(&fixture, &recorded.child, "CODEX_HOME", "codex");
    // Word wrap may break the temp path at a hyphen, so the sentence is
    // compared with every space removed.
    let expected = format!(
        "CODEX_HOME points the provider at its overlay in {}; your home directory is unchanged.",
        biscuit_file::to_portable_string(&overlay)
    );
    let unspaced = |s: &str| s.split_whitespace().collect::<String>();
    assert!(unspaced(&text).contains(&unspaced(&expected)), "{text}");
    assert!(text.contains("provider overlay"), "{text}");
    assert!(text.contains("repo root detect"), "{text}");
    let lower = text.to_lowercase();
    for stale in ["shadow", "preserve authentication", "credential"] {
        assert!(!lower.contains(stale), "{stale:?} in output:\n{text}");
    }
}

/// A stripped API-key variable and an overlay problem stay two distinguishable
/// causes. On a successful `--repo` launch the removed key is reported as a
/// removed variable with the `--include` remedy, separately from the overlay
/// line. When the overlay fails, the diagnostic is `provider.overlay_failed`
/// and says nothing about the key. The key's value is never printed.
#[test]
fn a_removed_api_key_and_an_overlay_failure_are_distinct_diagnostics() {
    const SECRET: &str = "sk-fixture-never-printed";

    let fixture = CliProcessFixture::named("overlay-home-api-key");
    fixture.seed_user_config();
    write(&fixture.home().join(".claude").join("settings.json"), "{}\n");
    write_executable(&fixture.bin_dir().join("claude"), RECORD_SPAWN);
    let args_file = fixture.cwd().join("args.txt");
    let run = || {
        fixture
            .command()
            .env("CLAUDINE_ARGS_FILE", &args_file)
            .env("ANTHROPIC_API_KEY", SECRET)
            .args(["claude", "--repo", "--", "--version"])
            .output()
            .unwrap()
    };

    let launched = run();
    success(&launched);
    assert!(args_file.exists(), "claude was not spawned");
    let text = flattened(&launched);
    assert!(text.contains("ANTHROPIC_API_KEY"), "{text}");
    assert!(text.contains("potentially dangerous ENV variables were removed"), "{text}");
    assert!(text.contains("--include"), "{text}");
    assert!(text.contains("CLAUDE_CONFIG_DIR points the provider at its overlay"), "{text}");
    assert!(!text.contains(SECRET), "{text}");

    fs::remove_file(&args_file).unwrap();
    fs::remove_dir_all(fixture.home().join(".claudine").join("overlays")).unwrap();
    write(&fixture.home().join(".claudine").join("overlays"), "not a directory");

    let failed = run();
    assert!(!failed.status.success(), "a failed overlay must not launch");
    assert!(!args_file.exists(), "the provider was spawned despite the failure");
    let text = flattened(&failed);
    assert!(text.contains("provider.overlay_failed"), "{text}");
    for unrelated in ["ANTHROPIC_API_KEY", "--include", SECRET] {
        assert!(!text.contains(unrelated), "{unrelated:?} in the overlay failure:\n{text}");
    }
    assert!(!text.to_lowercase().contains("credential"), "{text}");
}

// -- the contract matrix (spec → Testing → L1 contract tests) -------------------------

/// Records every home variable, every provider-owned overlay selector, the
/// provider-visible root's entries, the `marker` file an explicit-source test
/// seeds, and which seeded user-scoped `<class>/user.md` resources the provider
/// can reach — then runs the nested tools.
const RECORD_OVERLAY: &str = r#"#!/bin/sh
{
  for name in HOME USERPROFILE HOMEDRIVE HOMEPATH CODEX_HOME CODEX_SQLITE_HOME \
    CLAUDE_CONFIG_DIR CLAUDE_SECURESTORAGE_CONFIG_DIR GEMINI_CLI_HOME KIMI_CODE_HOME \
    PI_CODING_AGENT_DIR QWEN_HOME OPENCODE_CONFIG_DIR; do
    eval "set=\${$name+x} value=\${$name}"
    if [ -n "$set" ]; then printf '%s=%s\n' "$name" "$value"; else printf '%s=<unset>\n' "$name"; fi
  done
  visible="${CODEX_HOME:-${CLAUDE_CONFIG_DIR:-${KIMI_CODE_HOME:-${PI_CODING_AGENT_DIR:-${QWEN_HOME:-}}}}}"
  if [ -n "$GEMINI_CLI_HOME" ]; then visible="$GEMINI_CLI_HOME/.gemini"; fi
  if [ -n "$visible" ] && [ -d "$visible" ]; then
    printf 'VISIBLE=%s\n' "$(ls -A "$visible" | tr '\n' ' ')"
    if [ -f "$visible/marker" ]; then printf 'MARKER=%s\n' "$(cat "$visible/marker")"; fi
    printf 'USER_RESOURCES='
    for class in skills commands agents hooks prompts; do
      if [ -e "$visible/$class/user.md" ]; then printf '%s ' "$class"; fi
    done
    printf '\n'
  fi
} > "$CLAUDINE_ENV_FILE"
"$CLAUDINE_NESTED_BIN/run-nested"
exit 0
"#;

/// A provider that can hold a filesystem overlay for `--repo`: its binary, its
/// selector, the default source root it reads without Claudine, and the slug
/// its launch roots are grouped under in `~/.claudine/overlays`.
struct OverlayProvider {
    binary: &'static str,
    selector: &'static str,
    /// Home-relative components of the pre-overlay source root.
    source: &'static [&'static str],
    slug: &'static str,
}

/// Every provider whose `repo_resources` verdict is a native root.
const REPO_OVERLAY_PROVIDERS: [OverlayProvider; 6] = [
    OverlayProvider { binary: "claude", selector: "CLAUDE_CONFIG_DIR", source: &[".claude"], slug: "claude" },
    OverlayProvider { binary: "codex", selector: "CODEX_HOME", source: &[".codex"], slug: "codex" },
    OverlayProvider { binary: "gemini", selector: "GEMINI_CLI_HOME", source: &[".gemini"], slug: "gemini" },
    OverlayProvider { binary: "kimi", selector: "KIMI_CODE_HOME", source: &[".kimi-code"], slug: "kimi" },
    OverlayProvider { binary: "pi", selector: "PI_CODING_AGENT_DIR", source: &[".pi", "agent"], slug: "pi" },
    OverlayProvider { binary: "qwen", selector: "QWEN_HOME", source: &[".qwen"], slug: "qwen" },
];

fn joined(root: &Path, parts: &[&str]) -> std::path::PathBuf {
    parts.iter().fold(root.to_path_buf(), |path, part| path.join(part))
}

fn value_of<'a>(recorded: &'a str, name: &str) -> Option<&'a str> {
    let prefix = format!("{name}=");
    recorded.lines().find_map(|l| l.strip_prefix(prefix.as_str()))
}

/// Invariant 1, read from what a process actually observed: no home variable
/// holds a null device or a path inside Claudine's overlay storage.
fn assert_no_home_sentinel(who: &str, text: &str) {
    for name in ["HOME", "USERPROFILE", "HOMEDRIVE", "HOMEPATH"] {
        let Some(value) = value_of(text, name) else { continue };
        assert!(
            value != "/dev/null"
                && !value.eq_ignore_ascii_case("NUL")
                && !Path::new(value).components().any(|c| c.as_os_str() == ".claudine"),
            "{who} saw {name}={value}"
        );
    }
}

/// Contracts 1, 2, and 10: every activation reason (`repo_resources` for each
/// provider that supports it, `repo_prompt`, and `mcp` through both a
/// filesystem and an inline injector), on the direct and composition routes,
/// hands the provider and its nested `git`, `gpg`, and `gh` the launch home
/// unchanged. `HOMEDRIVE`/`HOMEPATH` are absent at launch and must stay absent
/// rather than be synthesized. The overlay reaches the provider only through
/// its own selector.
#[test]
fn every_activation_reason_leaves_the_launch_home_variables_unchanged() {
    struct Case {
        name: &'static str,
        binary: &'static str,
        args: Vec<String>,
        seed: fn(&CliProcessFixture),
        /// The selector and the slug its launch root is grouped under; `None`
        /// for a launch that must set no filesystem selector.
        selector: Option<(&'static str, &'static str)>,
    }
    let repo = |provider: &OverlayProvider| Case {
        name: provider.binary,
        binary: provider.binary,
        args: vec![provider.binary.into(), "--repo".into(), "--".into(), "--version".into()],
        seed: |fixture| {
            for provider in &REPO_OVERLAY_PROVIDERS {
                write(&joined(fixture.home(), provider.source).join("settings.json"), "{}");
            }
        },
        selector: Some((provider.selector, provider.slug)),
    };
    let mut cases: Vec<Case> = REPO_OVERLAY_PROVIDERS.iter().map(repo).collect();
    cases.extend([
        Case {
            name: "codex repo_prompt",
            binary: "codex",
            args: vec!["codex".into(), "--".into(), "--version".into()],
            seed: |fixture| {
                fs::create_dir_all(fixture.home().join(".codex")).unwrap();
                write(&fixture.cwd().join(".claude/commands/review.md"), "---\ndescription: review\n---\n");
            },
            selector: Some(("CODEX_HOME", "codex")),
        },
        Case {
            name: "codex mcp",
            binary: "codex",
            args: ["codex", "--mcp", "--", "exec", "fix #calendar bugs"].map(String::from).to_vec(),
            seed: |fixture| {
                fs::create_dir_all(fixture.home().join(".codex")).unwrap();
                seed_mcp_catalog(fixture, &["calendar"]);
            },
            selector: Some(("CODEX_HOME", "codex")),
        },
        Case {
            name: "gemini mcp",
            binary: "gemini",
            args: ["gemini", "--mcp", "--", "--prompt", "fix #calendar bugs"].map(String::from).to_vec(),
            seed: |fixture| {
                write(&fixture.home().join(".gemini").join("settings.json"), "{}");
                seed_mcp_catalog(fixture, &["calendar"]);
            },
            selector: Some(("GEMINI_CLI_HOME", "gemini")),
        },
        Case {
            name: "opencode inline mcp",
            binary: "opencode",
            args: ["opencode", "--mcp", "--", "run", "fix #calendar bugs"].map(String::from).to_vec(),
            seed: |fixture| seed_mcp_catalog(fixture, &["calendar"]),
            selector: None,
        },
        Case {
            name: "compose codex repo_resources",
            binary: "codex",
            args: vec![],
            seed: |fixture| {
                fs::create_dir_all(fixture.home().join(".codex")).unwrap();
                write(&fixture.cwd().join("task.md"), "---\ntitle: task\n---\nDo the task.\n");
            },
            selector: Some(("CODEX_HOME", "codex")),
        },
    ]);

    for mut case in cases {
        let fixture = CliProcessFixture::named("overlay-home-reasons");
        fixture.seed_user_config();
        (case.seed)(&fixture);
        if case.args.is_empty() {
            let document = fixture.cwd().join("task.md");
            case.args = vec!["compose".into(), "--codex".into(), "--repo".into(), document.display().to_string()];
        }
        write_executable(&fixture.bin_dir().join(case.binary), RECORD_OVERLAY);

        let output = recording_command(&fixture)
            .env("OPENCODE_MODEL", "test-model")
            .args(&case.args)
            .output()
            .unwrap();
        let name = case.name;
        assert!(
            output.status.success(),
            "{name}: claudine failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let recorded = recorded(&fixture).unwrap_or_else(|| panic!("{name}: provider was not spawned"));
        assert_home_preserved(&fixture, &recorded);
        let observers = std::iter::once(("provider", &recorded.child))
            .chain(recorded.nested.iter().map(|(tool, text)| (*tool, text)));
        for (who, text) in observers {
            assert_no_home_sentinel(&format!("{name}: {who}"), text);
            for absent in ["HOMEDRIVE=<unset>", "HOMEPATH=<unset>"] {
                assert!(text.lines().any(|l| l == absent), "{name}: {who} synthesized {absent}:\n{text}");
            }
        }

        let child = &recorded.child;
        match case.selector {
            Some((selector, slug)) => {
                launch_root(&fixture, child, selector, slug);
            }
            None => {
                for selector in ["CODEX_HOME", "CLAUDE_CONFIG_DIR", "GEMINI_CLI_HOME", "OPENCODE_CONFIG_DIR"] {
                    assert_eq!(value_of(child, selector), Some("<unset>"), "{name}: {child}");
                }
            }
        }
    }
}

/// Contract 3: an explicit provider root the user exported is the overlay's
/// **source**. The selector the provider receives points at the overlay in the
/// provider's own path shape, the overlay carries the explicit root's content
/// (not the default root's), and the explicit root is not written to.
#[test]
fn an_explicit_provider_root_is_the_overlay_source_and_the_selector_names_the_overlay() {
    for provider in &REPO_OVERLAY_PROVIDERS {
        let name = provider.binary;
        let fixture = CliProcessFixture::named("overlay-home-explicit-root");
        fixture.seed_user_config();
        write(&joined(fixture.home(), provider.source).join("marker"), "default root");
        // Gemini's selector names the parent of `.gemini`; every other one names
        // the provider directory itself.
        let explicit = fixture.home().join(format!("explicit {name} root"));
        let explicit_provider_dir = match provider.binary {
            "gemini" => explicit.join(".gemini"),
            _ => explicit.clone(),
        };
        write(&explicit_provider_dir.join("marker"), "explicit root");
        write(&explicit_provider_dir.join("skills").join("user.md"), "user skill");
        write_executable(&fixture.bin_dir().join(name), RECORD_OVERLAY);

        let output = recording_command(&fixture)
            .env(provider.selector, &explicit)
            .args([name, "--repo", "--", "--version"])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{name}: claudine failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let recorded = recorded(&fixture).unwrap_or_else(|| panic!("{name} was not spawned"));
        assert_home_preserved(&fixture, &recorded);
        let child = &recorded.child;

        // The selector must name the overlay, not the explicit root.
        launch_root(&fixture, child, provider.selector, provider.slug);
        assert_eq!(
            value_of(child, "MARKER"),
            Some("explicit root"),
            "{name}: the overlay must be built from the explicit root:\n{child}"
        );
        assert!(value_of(child, "VISIBLE").is_some(), "{name}: no overlay at the visible root:\n{child}");
        let mut entries: Vec<_> = fs::read_dir(&explicit_provider_dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        entries.sort();
        assert_eq!(entries, ["marker", "skills"], "{name}: the explicit root was written to");
        if name == "claude" {
            assert_eq!(
                value_of(child, "CLAUDE_SECURESTORAGE_CONFIG_DIR"),
                Some(explicit.display().to_string().as_str()),
                "the credential store follows the user's explicit config dir:\n{child}"
            );
        }
    }
}

/// Contract 4: `--repo` hides exactly the resource classes documented for each
/// provider in `docs/topics/repo-isolation.md` — no more (settings and the
/// classes the table does not name stay visible) and no less.
#[test]
fn a_repo_overlay_hides_exactly_the_documented_resource_classes() {
    const CANDIDATES: [&str; 5] = ["skills", "commands", "agents", "hooks", "prompts"];
    for (provider, hidden) in [
        ("claude", &["skills", "commands", "agents", "hooks"][..]),
        ("codex", &["skills", "agents", "prompts"][..]),
        ("gemini", &["skills", "agents"][..]),
        ("kimi", &["skills", "agents"][..]),
        ("pi", &["skills", "commands", "agents", "hooks"][..]),
        ("qwen", &["skills", "commands"][..]),
    ] {
        let spec = REPO_OVERLAY_PROVIDERS.iter().find(|p| p.binary == provider).unwrap();
        let fixture = CliProcessFixture::named("overlay-home-isolation");
        fixture.seed_user_config();
        let source = joined(fixture.home(), spec.source);
        write(&source.join("settings.json"), "{}");
        for class in CANDIDATES {
            write(&source.join(class).join("user.md"), "user-scoped");
        }
        write_executable(&fixture.bin_dir().join(provider), RECORD_OVERLAY);

        let (output, recorded) = launch(&fixture, &[provider, "--repo", "--", "--version"]);
        success(&output);
        let recorded = recorded.unwrap_or_else(|| panic!("{provider} was not spawned"));
        let child = &recorded.child;
        let visible: Vec<&str> = value_of(child, "VISIBLE")
            .unwrap_or_else(|| panic!("{provider}: no provider-visible root:\n{child}"))
            .split_whitespace()
            .collect();
        let reachable: Vec<&str> = value_of(child, "USER_RESOURCES").unwrap_or_default().split_whitespace().collect();

        assert!(visible.contains(&"settings.json"), "{provider} lost its settings: {visible:?}");
        // A hidden class may still exist as a repo-scoped replacement (Codex
        // rebuilds `prompts`), so visibility is judged by the user's own file.
        for class in CANDIDATES {
            assert_eq!(
                !reachable.contains(&class),
                hidden.contains(&class),
                "{provider}: user `{class}` reachability differs from the documented table: {reachable:?}"
            );
        }
        // The user's own tree is untouched by the masking.
        for class in CANDIDATES {
            assert!(source.join(class).join("user.md").is_file(), "{provider}: {class} was removed");
        }
    }
}

/// Contract 8, driven from the audit's pre-spawn refusal list: each
/// unsupported `(provider, repo_resources)` pair fails with the typed
/// diagnostic before the fake provider records a spawn — on the direct and the
/// composition route — and no overlay storage is created. A provider with no
/// MCP overlay verdict (`claude --mcp`) keeps its existing export
/// guidance rather than gaining an overlay refusal (audit D1).
#[test]
fn every_refused_provider_and_reason_fails_before_the_provider_is_spawned() {
    for (provider, binary) in [
        ("antigravity", "agy"),
        ("opencode", "opencode"),
        ("kilo", "kilo"),
        ("goose", "goose"),
    ] {
        for route in ["direct", "compose"] {
            let fixture = CliProcessFixture::named("overlay-home-refusal");
            fixture.seed_user_config();
            let args_file = fixture.cwd().join("args.txt");
            write_executable(&fixture.bin_dir().join(binary), RECORD_SPAWN);
            let document = fixture.cwd().join("task.md");
            write(&document, "---\ntitle: task\n---\nDo the task.\n");
            let flag = format!("--{provider}");
            let args: Vec<&str> = match route {
                "direct" => vec![provider, "--repo", "summarize"],
                _ => vec!["compose", flag.as_str(), "--repo", document.to_str().unwrap()],
            };

            let refused = fixture
                .command()
                .env("CLAUDINE_ARGS_FILE", &args_file)
                .env("OPENCODE_MODEL", "test-model")
                .args(&args)
                .output()
                .unwrap();
            let text = flattened(&refused);
            assert!(!refused.status.success(), "{provider} {route}: --repo launched:\n{text}");
            assert!(text.contains("provider.overlay_unsupported"), "{provider} {route}:\n{text}");
            assert!(!text.to_lowercase().contains("credential"), "{provider} {route}:\n{text}");
            assert!(!args_file.exists(), "{provider} {route}: spawned despite the refusal");
            assert!(
                !fixture.home().join(".claudine").join("overlays").exists(),
                "{provider} {route}: a refused launch built overlay storage"
            );
        }
    }

    let fixture = CliProcessFixture::named("overlay-home-refusal-mcp");
    fixture.seed_user_config();
    seed_mcp_catalog(&fixture, &["calendar"]);
    let args_file = fixture.cwd().join("args.txt");
    write_executable(&fixture.bin_dir().join("claude"), RECORD_SPAWN);
    let output = fixture
        .command()
        .env("CLAUDINE_ARGS_FILE", &args_file)
        .args(["claude", "--mcp", "--", "--print", "fix #calendar bugs"])
        .output()
        .unwrap();
    // Claude has no runtime injector: the pre-existing export guidance answers,
    // not an overlay refusal, and no overlay storage is built for it.
    let text = flattened(&output);
    assert!(text.contains("claudine mcp export claude --apply"), "{text}");
    assert!(!text.contains("provider.overlay_unsupported"), "{text}");
    assert!(!fixture.home().join(".claudine").join("overlays").exists());
}

/// Contract 9 with an unwritable storage root rather than a file in the way:
/// the launch stops with `provider.overlay_failed`, nothing is spawned, and no
/// null home is ever handed out.
#[test]
fn an_unwritable_storage_root_stops_the_launch_with_the_typed_diagnostic() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = CliProcessFixture::named("overlay-home-unwritable");
    fixture.seed_user_config();
    write(&fixture.home().join(".codex").join("config.toml"), "model = \"fixture\"\n");
    write_executable(&fixture.bin_dir().join("codex"), RECORD_OVERLAY);
    let storage = fixture.home().join(".claudine");
    let read_only = fs::Permissions::from_mode(0o555);
    fs::set_permissions(&storage, read_only).unwrap();
    // A privileged runner ignores the mode, so the premise is checked, not assumed.
    let writable = fs::write(storage.join("probe"), "").is_ok();
    let (output, recorded) = launch(&fixture, &["codex", "--repo", "--", "--version"]);
    fs::set_permissions(&storage, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(
        !writable,
        "test premise unavailable: a 0o555 directory stayed writable, so this runner is privileged \
         (root ignores file modes). Run L1 as an unprivileged user; the privilege-independent failure \
         is `a_failed_overlay_stops_the_launch_without_a_null_home` in this binary, and the typed \
         projection is `provider_overlay::tests::a_materialization_failure_projects_its_stage_and_publishes_its_cause` \
         in the library."
    );

    let text = flattened(&output);
    assert!(!output.status.success(), "an unwritable overlay must not launch:\n{text}");
    assert!(recorded.is_none(), "the provider was spawned despite the failure");
    assert!(text.contains("provider.overlay_failed"), "{text}");
    for forbidden in ["/dev/null", "shadow", "credential"] {
        assert!(!text.to_lowercase().contains(forbidden), "{forbidden:?} in:\n{text}");
    }
    assert!(!storage.join("overlays").exists(), "storage appeared under a read-only root");
}

/// Edge matrix: an absent launch home variable stays absent (never synthesized
/// from `HOME`), and a non-UTF-8 value reaches the provider and a nested tool
/// byte for byte, under an overlay launch.
#[test]
fn absent_and_non_utf8_home_variables_pass_through_an_overlay_launch_verbatim() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let non_utf8 = OsString::from_vec(b"/Users/caf\xe9 profile".to_vec());
    for profile in [None, Some(non_utf8)] {
        let fixture = CliProcessFixture::named("overlay-home-edge");
        fixture.seed_user_config();
        fs::create_dir_all(fixture.home().join(".codex")).unwrap();
        write_executable(&fixture.bin_dir().join("codex"), RECORD_OVERLAY);

        let mut command = recording_command(&fixture);
        match &profile {
            Some(value) => command.env("USERPROFILE", value),
            None => command.env_remove("USERPROFILE"),
        };
        let output = command.args(["codex", "--repo", "--", "--version"]).output().unwrap();
        success(&output);

        let mut expected = b"USERPROFILE=".to_vec();
        match &profile {
            Some(value) => expected.extend_from_slice(value.as_encoded_bytes()),
            None => expected.extend_from_slice(b"<unset>"),
        }
        let child = fs::read(fixture.cwd().join("child-env.txt")).expect("codex was not spawned");
        let nested = fs::read(fixture.cwd().join("nested-env.git")).expect("git did not run");
        for (who, bytes) in [("provider", &child), ("nested git", &nested)] {
            assert!(
                bytes.split(|b| *b == b'\n').any(|l| l == expected.as_slice()),
                "{who} saw a changed USERPROFILE ({profile:?}):\n{}",
                String::from_utf8_lossy(bytes)
            );
            let home = format!("HOME={}", fixture.home().display());
            assert!(
                bytes.split(|b| *b == b'\n').any(|l| l == home.as_bytes()),
                "{who}: {}",
                String::from_utf8_lossy(bytes)
            );
        }
    }
}

// -- per-launch overlay roots (review 1, finding 2) ---------------------------------

/// Records what the provider can reach at the *end* of its life. With
/// `CLAUDINE_RELEASE` set it first announces itself through `CLAUDINE_STARTED`
/// and waits for the release file, so another launch can run start to finish
/// inside its lifetime.
const RECORD_AT_EXIT: &str = r#"#!/bin/sh
if [ -n "$CLAUDINE_RELEASE" ]; then
  : > "$CLAUDINE_STARTED"
  tries=0
  while [ ! -e "$CLAUDINE_RELEASE" ] && [ "$tries" -lt 600 ]; do
    sleep 0.1
    tries=$((tries + 1))
  done
fi
{
  printf 'CODEX_HOME=%s\n' "${CODEX_HOME-<unset>}"
  printf 'GEMINI_CLI_HOME=%s\n' "${GEMINI_CLI_HOME-<unset>}"
  root="${CODEX_HOME:-}"
  if [ -n "$GEMINI_CLI_HOME" ]; then root="$GEMINI_CLI_HOME/.gemini"; fi
  if [ -n "$root" ] && [ -d "$root" ]; then
    printf 'VISIBLE=%s\n' "$(ls -A "$root" | tr '\n' ' ')"
    if [ -e "$root/skills" ]; then printf 'REACHABLE=skills\n'; fi
    if [ -d "$root/prompts" ]; then printf 'PROMPTS=%s\n' "$(ls -A "$root/prompts" | tr '\n' ' ')"; fi
    if [ -f "$root/config.toml" ]; then printf 'CODEX_CONFIG=%s\n' "$(tr '\n' ' ' < "$root/config.toml")"; fi
    if [ -f "$root/settings.json" ]; then printf 'GEMINI_SETTINGS=%s\n' "$(tr '\n' ' ' < "$root/settings.json")"; fi
  fi
} > "$CLAUDINE_ENV_FILE"
exit 0
"#;

/// One claudine invocation: its arguments and, when it matters, the
/// repository it is launched from.
struct Invocation<'a> {
    args: &'a [&'a str],
    repo: Option<&'a Path>,
}

impl<'a> Invocation<'a> {
    fn new(args: &'a [&'a str]) -> Self {
        Self { args, repo: None }
    }

    fn in_repo(args: &'a [&'a str], repo: &'a Path) -> Self {
        Self { args, repo: Some(repo) }
    }

    fn builder<'f>(&self, fixture: &'f CliProcessFixture) -> common::ClaudineCommandBuilder<'f> {
        match self.repo {
            // The launch context is the subject: each repository carries its
            // own Codex prompts, which the overlay must scope to that launch.
            Some(repo) => fixture.command_builder().ambient_context(repo),
            None => fixture.command_builder(),
        }
    }
}

/// Run `invocation` to completion and return what its provider recorded.
fn record_one(fixture: &CliProcessFixture, name: &str, invocation: &Invocation<'_>) -> String {
    let env_file = fixture.workspace_path().join(format!("{name}.env"));
    let output = invocation
        .builder(fixture)
        .build()
        .env("CLAUDINE_ENV_FILE", &env_file)
        .args(invocation.args)
        .output()
        .unwrap();
    success(&output);
    fs::read_to_string(&env_file).unwrap_or_else(|_| panic!("{name}: the provider was not spawned"))
}

/// Writes the release file on drop, so a failing assertion never leaves the
/// outer provider waiting out its full timeout.
struct Release(PathBuf);

impl Drop for Release {
    fn drop(&mut self) {
        let _ = fs::write(&self.0, "");
    }
}

/// Start `outer` and hold its provider alive, run `inner` start to finish while
/// it lives, then release `outer`. Returns `(outer, inner)` as each provider
/// recorded it at its own exit, so `outer` reflects everything `inner` did —
/// including `inner`'s cleanup.
fn overlapping(
    fixture: &CliProcessFixture,
    outer: &Invocation<'_>,
    inner: &Invocation<'_>,
) -> (String, String) {
    use std::process::Stdio;
    use std::time::{Duration, Instant};

    let work = fixture.workspace_path().join("overlap");
    fs::create_dir_all(&work).unwrap();
    let (started, env_file, stderr) = (work.join("started"), work.join("outer.env"), work.join("outer.stderr"));
    let release = Release(work.join("release"));

    let mut command = outer.builder(fixture).build_std();
    command
        .env("CLAUDINE_ENV_FILE", &env_file)
        .env("CLAUDINE_STARTED", &started)
        .env("CLAUDINE_RELEASE", &release.0)
        .args(outer.args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(fs::File::create(&stderr).unwrap());
    let mut child = command.spawn().unwrap();
    let outer_stderr = || fs::read_to_string(&stderr).unwrap_or_default();

    let deadline = Instant::now() + Duration::from_secs(60);
    while !started.exists() {
        if let Some(status) = child.try_wait().unwrap() {
            panic!("the outer launch exited ({status}) before its provider started:\n{}", outer_stderr());
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("the outer provider never started:\n{}", outer_stderr());
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    let inner_recorded = record_one(fixture, "inner", inner);
    drop(release);
    let status = child.wait().unwrap();
    assert!(status.success(), "the outer launch failed:\n{}", outer_stderr());
    let outer_recorded = fs::read_to_string(&env_file).expect("the outer provider recorded nothing");
    (outer_recorded, inner_recorded)
}

fn seed_mcp_servers(fixture: &CliProcessFixture, ids: &[&str]) {
    let servers: Vec<_> = ids.iter().map(|id| make_server(id)).collect();
    seed_catalog(fixture.home(), &servers);
    seed_defaults(fixture.home(), &[]);
    seed_empty_provider_state(fixture.home());
}

/// Every entry under `root` with its bytes, for proving storage unchanged.
fn tree_snapshot(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut out = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path.clone());
                out.push((path, Vec::new()));
            } else {
                let bytes = fs::read(&path).unwrap();
                out.push((path, bytes));
            }
        }
    }
    out.sort();
    out
}

/// A launch without `--repo` mirrors the user's `skills`; a later `--repo`
/// launch must still hide them, because it builds its own root rather than
/// inheriting what the earlier launch placed there. Compatible legacy storage
/// under `~/.claudine/.codex` is neither read nor changed by either launch.
#[test]
fn a_repo_launch_after_a_non_repo_launch_cannot_reach_the_earlier_skills() {
    let fixture = CliProcessFixture::named("overlay-launch-repo-transition");
    fixture.seed_user_config();
    write(&fixture.home().join(".codex").join("config.toml"), "model = \"fixture\"\n");
    write(&fixture.home().join(".codex").join("skills").join("review").join("SKILL.md"), "user skill");
    let legacy = fixture.home().join(".claudine").join(".codex");
    write(&legacy.join("config.toml"), "model = \"legacy\"\n");
    write(&legacy.join("skills").join("legacy").join("SKILL.md"), "legacy skill");
    let legacy_before = tree_snapshot(&legacy);
    seed_mcp_servers(&fixture, &["calendar"]);
    write_executable(&fixture.bin_dir().join("codex"), RECORD_AT_EXIT);

    let without_repo = record_one(
        &fixture,
        "without-repo",
        &Invocation::new(&["codex", "--use", "calendar", "--", "exec", "go"]),
    );
    assert!(
        without_repo.lines().any(|l| l == "REACHABLE=skills"),
        "fixture check: a launch without --repo mirrors the user's skills:\n{without_repo}"
    );

    let with_repo = record_one(&fixture, "with-repo", &Invocation::new(&["codex", "--repo", "--", "--version"]));
    assert!(
        !with_repo.lines().any(|l| l == "REACHABLE=skills"),
        "--repo reached skills an earlier launch mirrored:\n{with_repo}"
    );
    assert!(with_repo.contains("CODEX_CONFIG=model = \"fixture\""), "{with_repo}");
    for recorded in [&without_repo, &with_repo] {
        assert!(!recorded.contains("legacy"), "legacy storage reached a launch:\n{recorded}");
        launch_root(&fixture, recorded, "CODEX_HOME", "codex");
    }
    assert_eq!(tree_snapshot(&legacy), legacy_before, "legacy storage changed");
}

/// An entry the user deletes from their provider root between launches is gone
/// from the next launch's view — even on Unix, where the earlier view held a
/// link to it.
#[test]
fn a_source_entry_deleted_between_launches_is_absent_from_the_next_view() {
    let fixture = CliProcessFixture::named("overlay-launch-source-deletion");
    fixture.seed_user_config();
    let source = fixture.home().join(".codex");
    write(&source.join("config.toml"), "model = \"fixture\"\n");
    write(&source.join("rules").join("default.rules"), "rule");
    write_executable(&fixture.bin_dir().join("codex"), RECORD_AT_EXIT);
    let repo = Invocation::new(&["codex", "--repo", "--", "--version"]);
    let visible = |recorded: &str| -> Vec<String> {
        value_of(recorded, "VISIBLE")
            .unwrap_or_else(|| panic!("no provider-visible root:\n{recorded}"))
            .split_whitespace()
            .map(str::to_string)
            .collect()
    };

    let before = record_one(&fixture, "before", &repo);
    assert!(visible(&before).contains(&"rules".to_string()), "fixture check:\n{before}");

    fs::remove_dir_all(source.join("rules")).unwrap();
    let after = record_one(&fixture, "after", &repo);
    assert!(!visible(&after).contains(&"rules".to_string()), "a deleted source entry survived:\n{after}");
    assert!(visible(&after).contains(&"config.toml".to_string()), "{after}");
}

/// Two overlapping Codex launches from different repositories each see only
/// their own repository's prompts. The launch that ends first rebuilds and
/// removes nothing the still-running launch reads.
#[test]
fn overlapping_launches_in_two_repositories_each_see_only_their_own_prompts() {
    let fixture = CliProcessFixture::named("overlay-launch-two-repos");
    fixture.seed_user_config();
    write(&fixture.home().join(".codex").join("config.toml"), "model = \"fixture\"\n");
    let repos = ["repo-a", "repo-b"].map(|name| fixture.workspace_path().join(name));
    for (repo, prompt) in repos.iter().zip(["a.md", "b.md"]) {
        write(&repo.join(".codex").join("prompts").join(prompt), "---\ndescription: prompt\n---\n");
        assert!(common::init_git_repo(repo), "git is required to scope a launch to a repository");
    }
    write_executable(&fixture.bin_dir().join("codex"), RECORD_AT_EXIT);

    let args = ["codex", "--", "--version"];
    let (outer, inner) = overlapping(
        &fixture,
        &Invocation::in_repo(&args, &repos[0]),
        &Invocation::in_repo(&args, &repos[1]),
    );

    let prompts = |recorded: &str| value_of(recorded, "PROMPTS").unwrap_or_default().to_string();
    assert_eq!(prompts(&outer).trim(), "a.md", "repo-a's launch:\n{outer}");
    assert_eq!(prompts(&inner).trim(), "b.md", "repo-b's launch:\n{inner}");
    assert!(outer.contains("CODEX_CONFIG=model = \"fixture\""), "repo-b's exit broke repo-a's view:\n{outer}");
    assert_ne!(
        launch_root(&fixture, &outer, "CODEX_HOME", "codex"),
        launch_root(&fixture, &inner, "CODEX_HOME", "codex")
    );
}

/// Two overlapping Codex launches with different MCP server sets each read
/// only their own servers, and the injection cleanup of the launch that ends
/// first does not remove the configuration the other is still reading.
#[test]
fn overlapping_launches_with_two_mcp_server_sets_each_keep_their_own_config() {
    let fixture = CliProcessFixture::named("overlay-launch-two-mcp-sets");
    fixture.seed_user_config();
    write(&fixture.home().join(".codex").join("config.toml"), "model = \"fixture\"\n");
    seed_mcp_servers(&fixture, &["calendar", "linear"]);
    write_executable(&fixture.bin_dir().join("codex"), RECORD_AT_EXIT);

    let (outer, inner) = overlapping(
        &fixture,
        &Invocation::new(&["codex", "--use", "calendar", "--", "exec", "go"]),
        &Invocation::new(&["codex", "--use", "linear", "--", "exec", "go"]),
    );

    let config = |recorded: &str| value_of(recorded, "CODEX_CONFIG").unwrap_or_default().to_string();
    assert!(config(&outer).contains("[mcp_servers.calendar]"), "the calendar launch lost its server:\n{outer}");
    assert!(!config(&outer).contains("linear"), "the calendar launch read linear's config:\n{outer}");
    assert!(config(&inner).contains("[mcp_servers.linear]"), "{inner}");
    assert!(!config(&inner).contains("calendar"), "the linear launch read calendar's config:\n{inner}");
    launch_root(&fixture, &outer, "CODEX_HOME", "codex");
    launch_root(&fixture, &inner, "CODEX_HOME", "codex");
}

/// Two identical overlapping Gemini MCP launches: the one that ends first runs
/// its injector cleanup and drops its overlay root while the other provider is
/// still running, and the survivor still reads its injected settings.
#[test]
fn a_launch_ending_inside_another_launchs_lifetime_leaves_that_overlay_intact() {
    let fixture = CliProcessFixture::named("overlay-launch-overlapping-lifetimes");
    fixture.seed_user_config();
    write(&fixture.home().join(".gemini").join("settings.json"), "{\"theme\":\"fixture\"}");
    seed_mcp_servers(&fixture, &["linear"]);
    write_executable(&fixture.bin_dir().join("gemini"), RECORD_AT_EXIT);
    let launch = ["gemini", "--use", "linear", "--", "--prompt", "go"];

    let (outer, inner) = overlapping(&fixture, &Invocation::new(&launch), &Invocation::new(&launch));

    for (who, recorded) in [("surviving", &outer), ("first-ending", &inner)] {
        let settings = value_of(recorded, "GEMINI_SETTINGS")
            .unwrap_or_else(|| panic!("the {who} launch lost its settings:\n{recorded}"));
        assert!(settings.contains("\"linear\"") && settings.contains("fixture"), "{who}: {settings}");
    }
    assert_ne!(
        launch_root(&fixture, &outer, "GEMINI_CLI_HOME", "gemini"),
        launch_root(&fixture, &inner, "GEMINI_CLI_HOME", "gemini")
    );
}

/// Invariant 4 across a disposable root: a provider that rotates its token by
/// writing a sibling and renaming it over `auth.json` — which on Unix replaces
/// the overlay's link rather than writing through it — leaves the user's
/// source holding the rotated token after the launch ends, while a file the
/// provider created stays behind with the removed root.
#[test]
fn a_token_the_provider_rotates_during_a_launch_reaches_the_source_after_exit() {
    const ROTATE_TOKEN: &str = r#"#!/bin/sh
printf '{"token":"rotated"}' > "$CODEX_HOME/auth.json.tmp"
mv "$CODEX_HOME/auth.json.tmp" "$CODEX_HOME/auth.json"
printf '{}' > "$CODEX_HOME/created-by-provider.json"
printf 'CODEX_HOME=%s\n' "$CODEX_HOME" > "$CLAUDINE_ENV_FILE"
exit 0
"#;
    let fixture = CliProcessFixture::named("overlay-launch-token-rotation");
    fixture.seed_user_config();
    let source = fixture.home().join(".codex");
    write(&source.join("config.toml"), "model = \"fixture\"\n");
    write(&source.join("auth.json"), "{\"token\":\"old\"}");
    write_executable(&fixture.bin_dir().join("codex"), ROTATE_TOKEN);

    let recorded = record_one(&fixture, "rotation", &Invocation::new(&["codex", "--repo", "--", "--version"]));

    launch_root(&fixture, &recorded, "CODEX_HOME", "codex");
    assert_eq!(fs::read_to_string(source.join("auth.json")).unwrap(), "{\"token\":\"rotated\"}");
    assert!(!source.join("created-by-provider.json").exists(), "a new provider entry was written back");
    assert_eq!(fs::read_to_string(source.join("config.toml")).unwrap(), "model = \"fixture\"\n");
}

/// Invariant 4 when persistence fails: a token the provider rotates but
/// Claudine cannot write back — here a read-only source directory, standing in
/// for a permission error, sharing violation, or full disk — is never deleted
/// with the launch root. The launch says so on stderr, naming the recoverable
/// copy and never its contents, and the next launch's sweep leaves it alone.
#[test]
fn a_rotated_token_that_cannot_be_written_back_stays_recoverable() {
    use std::os::unix::fs::PermissionsExt;

    const ROTATE_TOKEN: &str = r#"#!/bin/sh
printf 'rotated-secret-value' > "$CODEX_HOME/auth.json.tmp"
mv "$CODEX_HOME/auth.json.tmp" "$CODEX_HOME/auth.json"
printf 'CODEX_HOME=%s\n' "$CODEX_HOME" > "$CLAUDINE_ENV_FILE"
exit 0
"#;
    let fixture = CliProcessFixture::named("overlay-launch-failed-write-back");
    fixture.seed_user_config();
    let source = fixture.home().join(".codex");
    write(&source.join("config.toml"), "model = \"fixture\"\n");
    write(&source.join("auth.json"), "old-secret-value");
    write_executable(&fixture.bin_dir().join("codex"), ROTATE_TOKEN);
    let env_file = fixture.workspace_path().join("failed-write-back.env");

    fs::set_permissions(&source, fs::Permissions::from_mode(0o555)).unwrap();
    // A privileged runner ignores the mode, so the premise is checked, not assumed.
    let writable = fs::write(source.join("probe"), "").is_ok();
    let output = fixture
        .command()
        .env("CLAUDINE_ENV_FILE", &env_file)
        .args(["codex", "--repo", "--", "--version"])
        .output()
        .unwrap();
    fs::set_permissions(&source, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(
        !writable,
        "test premise unavailable: a 0o555 directory stayed writable, so this runner is privileged \
         (root ignores file modes). Run L1 as an unprivileged user; the deterministic, cross-platform \
         failure cases are `provider_overlay::tests::lease` in the library."
    );

    let text = flattened(&output);
    let recorded = fs::read_to_string(&env_file).unwrap_or_else(|_| panic!("the provider was not spawned:\n{text}"));
    let root = PathBuf::from(value_of(&recorded, "CODEX_HOME").expect("CODEX_HOME recorded"));
    let kept = root.join("auth.json");
    assert_eq!(fs::read_to_string(source.join("auth.json")).unwrap(), "old-secret-value");
    assert_eq!(
        fs::read_to_string(&kept).unwrap_or_else(|error| panic!("the rotated token was deleted ({error}):\n{text}")),
        "rotated-secret-value"
    );
    assert!(text.contains("could not be written back"), "the failure was not reported:\n{text}");
    assert!(text.contains(&kept.display().to_string()), "the report must name the recoverable copy:\n{text}");
    assert!(!text.contains("secret-value"), "the report leaked token contents:\n{text}");

    // The next launch sweeps abandoned roots before building its own.
    write_executable(&fixture.bin_dir().join("codex"), RECORD_SPAWN);
    let next = fixture
        .command()
        .env("CLAUDINE_ARGS_FILE", fixture.workspace_path().join("next.args"))
        .args(["codex", "--repo", "--", "--version"])
        .output()
        .unwrap();
    success(&next);
    assert_eq!(fs::read_to_string(&kept).unwrap(), "rotated-secret-value", "a sweep removed the retained root");
}
