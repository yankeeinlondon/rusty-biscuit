//! Level 2: a provider overlay launched from a real terminal leaves the user
//! home alone.
//!
//! `level1_provider_overlay_home.rs` proves the overlay contracts with piped
//! stdio, which only reaches Claudine's non-interactive launch. A user types
//! `claudine codex --repo` at a terminal: stdin and stdout are a TTY, no prompt
//! is given, and the provider inherits the terminal as an interactive session.
//! This binary runs that launch inside a real terminal against a fake `codex`
//! and reads back what the provider and the tools it runs observed:
//!
//! - the provider really inherited the terminal (both stdin and stdout are TTYs,
//!   and its banner is drawn in the pane);
//! - the provider and nested `git`, `gpg`, and `gh` see the launch home
//!   variables unchanged, and no home variable names `/dev/null`, `NUL`, or
//!   the overlay;
//! - `CODEX_HOME` names this launch's own filesystem-backed overlay root, which
//!   carries the user's settings and a nested directory at depth, hides the
//!   `--repo`-isolated `skills` class, keeps SQLite state at the user's own
//!   Codex root, and is removed when the launch ends. The provider records
//!   what it reaches while it runs, since the root is gone afterwards.
//!
//! The spec is `fixes/2026-09-12-shadow-home/spec.md` → Testing → L2.
//!
//! ## Hermetic launch
//!
//! The pane's shell belongs to the harness, not to the L1 command builder, so
//! the Unix launcher starts `claudine` under `env -i` with only the fixture's
//! variables. An ambient `CODEX_HOME` or `CLAUDINE_*` in the developer's shell
//! cannot reach the child. No terminal window is opened or focused: Unix uses a
//! detached tmux session.
//!
//! ## Native Windows
//!
//! The Windows variant runs in a background WezTerm `cmd.exe` pane with a
//! rustc-built recorder, since a `.cmd` provider is a batch file with Rust's
//! argument restrictions. It also proves recursive materialization *by copy*:
//! the provider writes through the overlay's nested file, and the user's file
//! must not change.
//!
//! Claudine resolves the default provider source and overlay storage through
//! `dirs::home_dir()`, which on native Windows reads the known-folder profile
//! and ignores `USERPROFILE`. The launcher therefore names both roots
//! explicitly — the user's Codex root through `CODEX_HOME`, overlay storage
//! through `CLAUDINE_OVERLAY_DIR` — so neither resolves to the runner's real
//! profile, while the home variables still reach the child unchanged.
//!
//! Run via the canonical recipe: `just test-l2 provider_overlay_capture`.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use biscuit_test_harness::TerminalHarness;
use serial_test::serial;

use crate::common;
use common::{CliProcessFixture, claudine_bin, minimal_system_path, wait_for_exit_marker, write};

/// Drawn by the fake provider on the terminal it inherited.
const PROVIDER_BANNER: &str = "fake codex session attached";

/// The nested ordinary tools a provider runs.
const NESTED_TOOLS: [&str; 3] = ["git", "gpg", "gh"];

/// A directory two levels below a mirrored top-level entry, so only a
/// recursive materialization makes it reachable.
const NESTED_RULE: [&str; 4] = ["rules", "team", "nested", "deep.md"];
const NESTED_RULE_TEXT: &str = "nested rule from the user";

/// Seeds the user's Codex root: settings, a nested directory, a `--repo`
/// isolated class, and SQLite state that must stay outside the overlay.
fn seed_codex_home(fixture: &CliProcessFixture) -> PathBuf {
    fixture.seed_user_config();
    let codex = fixture.home().join(".codex");
    write(&codex.join("config.toml"), "model = \"fixture\"\n");
    write(&nested_rule(&codex), NESTED_RULE_TEXT);
    write(&codex.join("skills").join("user").join("SKILL.md"), "user skill");
    write(&codex.join("state_5.sqlite"), "sqlite");
    codex
}

fn nested_rule(root: &Path) -> PathBuf {
    NESTED_RULE.iter().fold(root.to_path_buf(), |path, segment| path.join(segment))
}

fn unique_marker() -> String {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    format!("OVERLAY_{}_{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed))
}

fn read_record(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|error| panic!("{} was not recorded: {error}", path.display()))
}

fn has_line(record: &str, expected: &str) -> bool {
    record.lines().any(|line| line.trim_end_matches('\r') == expected)
}

fn line(name: &str, value: impl AsRef<Path>) -> String {
    format!("{name}={}", value.as_ref().display())
}

/// No home variable may hold a null device or a Claudine overlay path.
fn assert_no_home_sentinel(who: &str, record: &str) {
    for recorded in record.lines().map(|line| line.trim_end_matches('\r')) {
        let Some((name, value)) = recorded.split_once('=') else {
            continue;
        };
        if !["HOME", "USERPROFILE", "HOMEDRIVE", "HOMEPATH"].contains(&name) {
            continue;
        }
        assert!(
            value != "/dev/null" && !value.eq_ignore_ascii_case("NUL") && !value.contains(".claudine"),
            "{who} saw a moved {name}: {value}\n{record}"
        );
    }
}

/// Everything both platforms assert about one completed launch whose overlay
/// roots were created under `launches`.
fn assert_overlay_launch(fixture: &CliProcessFixture, profile: &Path, launches: &Path, frame: &str, status: &str) {
    assert_eq!(status, "0", "claudine codex --repo failed in the pane:\n{frame}");
    assert!(
        frame.contains(PROVIDER_BANNER),
        "the provider's output never reached the terminal:\n{frame}"
    );

    let provider = read_record(&fixture.cwd().join("child-env.txt"));
    assert!(
        has_line(&provider, "STDIN_TTY=yes") && has_line(&provider, "STDOUT_TTY=yes"),
        "the provider did not inherit the terminal, so this was not the interactive launch:\n{provider}"
    );

    let home = line("HOME", fixture.home());
    let user_profile = line("USERPROFILE", profile);
    let observers = std::iter::once(("provider".to_string(), provider.clone())).chain(
        NESTED_TOOLS
            .iter()
            .map(|tool| (tool.to_string(), read_record(&fixture.cwd().join(format!("nested-env.{tool}"))))),
    );
    for (who, record) in observers {
        assert!(has_line(&record, &home), "{who} saw a moved HOME:\n{record}");
        assert!(has_line(&record, &user_profile), "{who} saw a moved USERPROFILE:\n{record}");
        assert!(has_line(&record, "HOMEDRIVE=<unset>"), "{who} saw a synthesized HOMEDRIVE:\n{record}");
        assert!(has_line(&record, "HOMEPATH=<unset>"), "{who} saw a synthesized HOMEPATH:\n{record}");
        assert_no_home_sentinel(&who, &record);
    }

    let codex = fixture.home().join(".codex");
    let overlay = provider
        .lines()
        .find_map(|recorded| recorded.trim_end_matches('\r').strip_prefix("CODEX_HOME="))
        .map(PathBuf::from)
        .unwrap_or_else(|| panic!("CODEX_HOME was not recorded:\n{provider}"));
    assert_eq!(
        overlay.parent(),
        Some(launches.join("codex").as_path()),
        "CODEX_HOME must name a launch root of its own:\n{provider}"
    );
    assert!(has_line(&provider, &line("CODEX_SQLITE_HOME", &codex)), "{provider}");
    assert!(
        has_line(&provider, "CONFIG=model = \"fixture\""),
        "the overlay must carry the user's Codex settings:\n{provider}"
    );
    assert!(
        has_line(&provider, &format!("NESTED_RULE={NESTED_RULE_TEXT}")),
        "a nested directory must be reachable through the overlay:\n{provider}"
    );
    assert!(
        has_line(&provider, "SKILL=<missing>"),
        "--repo must hide the user's skills from Codex:\n{provider}"
    );

    assert!(has_line(&provider, "OVERLAY_SQLITE=absent"), "SQLite state was mirrored into the overlay:\n{provider}");
    assert!(!overlay.exists(), "the ended launch left its overlay root: {}", overlay.display());
    assert_eq!(
        fs::read_to_string(codex.join("config.toml")).unwrap(),
        "model = \"fixture\"\n",
        "the user's Codex settings were modified"
    );
}

#[cfg(unix)]
mod unix {
    use super::*;
    use biscuit_test_harness::tmux::TmuxHarness;
    use common::{sh_quote, write_executable};
    use test_toolkit::{Backend, Level, require_level};

    /// The fake `codex`: records what it observes, then runs the nested tools
    /// from a directory only it puts on `PATH`, so Claudine's own `git` calls
    /// never reach them.
    const RECORD_CODEX: &str = r#"#!/bin/sh
# Sampled before the redirect below, which would otherwise be what `-t` sees.
if [ -t 0 ]; then stdin_tty=yes; else stdin_tty=no; fi
if [ -t 1 ]; then stdout_tty=yes; else stdout_tty=no; fi
{
  printf 'HOME=%s\n' "${HOME-<unset>}"
  printf 'USERPROFILE=%s\n' "${USERPROFILE-<unset>}"
  printf 'HOMEDRIVE=%s\n' "${HOMEDRIVE-<unset>}"
  printf 'HOMEPATH=%s\n' "${HOMEPATH-<unset>}"
  printf 'CODEX_HOME=%s\n' "${CODEX_HOME-<unset>}"
  printf 'CODEX_SQLITE_HOME=%s\n' "${CODEX_SQLITE_HOME-<unset>}"
  printf 'STDIN_TTY=%s\nSTDOUT_TTY=%s\n' "$stdin_tty" "$stdout_tty"
  printf 'CONFIG=%s\n' "$(cat "$CODEX_HOME/config.toml" 2>/dev/null || echo '<missing>')"
  printf 'NESTED_RULE=%s\n' "$(cat "$CODEX_HOME/rules/team/nested/deep.md" 2>/dev/null || echo '<missing>')"
  printf 'SKILL=%s\n' "$(cat "$CODEX_HOME/skills/user/SKILL.md" 2>/dev/null || echo '<missing>')"
  if [ -e "$CODEX_HOME/state_5.sqlite" ]; then echo 'OVERLAY_SQLITE=present'; else echo 'OVERLAY_SQLITE=absent'; fi
  printf 'RULES_LINK=%s\n' "$(readlink "$CODEX_HOME/rules" 2>/dev/null || echo '<not a link>')"
} > "$CLAUDINE_ENV_FILE"
PATH="$CLAUDINE_NESTED_BIN:$PATH" git status
PATH="$CLAUDINE_NESTED_BIN:$PATH" gpg --list-keys
PATH="$CLAUDINE_NESTED_BIN:$PATH" gh auth status
echo 'fake codex session attached'
exit 0
"#;

    const NESTED_TOOL: &str = r#"#!/bin/sh
{
  printf 'HOME=%s\n' "${HOME-<unset>}"
  printf 'USERPROFILE=%s\n' "${USERPROFILE-<unset>}"
  printf 'HOMEDRIVE=%s\n' "${HOMEDRIVE-<unset>}"
  printf 'HOMEPATH=%s\n' "${HOMEPATH-<unset>}"
} > "$CLAUDINE_NESTED_FILE.$(basename "$0")"
exit 0
"#;

    /// Writes the launcher the pane runs, returning its path.
    ///
    /// The exit marker is printed by the script rather than typed with `$?`:
    /// on a host that loads Atuin AI, a typed `?` opens its overlay and wedges
    /// the pane.
    fn write_launcher(fixture: &CliProcessFixture, profile: &Path, marker: &str) -> PathBuf {
        let nested_bin = fixture.workspace_path().join("nested-bin");
        for tool in NESTED_TOOLS {
            write_executable(&nested_bin.join(tool), NESTED_TOOL);
        }
        write_executable(&fixture.bin_dir().join("codex"), RECORD_CODEX);

        let path = std::env::join_paths(
            std::iter::once(fixture.bin_dir().to_path_buf()).chain(minimal_system_path()),
        )
        .expect("join fixture PATH");
        let quoted = |value: &Path| sh_quote(&value.display().to_string());
        let variables = [
            ("HOME", quoted(fixture.home())),
            ("USERPROFILE", quoted(profile)),
            ("PATH", quoted(Path::new(&path))),
            ("TERM", "xterm-256color".to_string()),
            ("NO_COLOR", "1".to_string()),
            ("CLAUDINE_RENDEZVOUS_REPORT", "false".to_string()),
            ("PLAYA_DRY_RUN", "1".to_string()),
            ("PLAYA_SPOOL_DIR", quoted(&fixture.audio_spool())),
            ("CLAUDINE_ENV_FILE", quoted(&fixture.cwd().join("child-env.txt"))),
            ("CLAUDINE_NESTED_FILE", quoted(&fixture.cwd().join("nested-env"))),
            ("CLAUDINE_NESTED_BIN", quoted(&nested_bin)),
        ]
        .map(|(name, value)| format!("{name}={value}"))
        .join(" ");
        let script = format!(
            "#!/bin/sh\nprintf '\\033[2J\\033[H'\ncd {cwd} || exit 1\n\
             if /usr/bin/env -i {variables} {claudine} codex --repo; then\n  \
             printf '\\n{marker}:0\\n'\nelse\n  printf '\\n{marker}:1\\n'\nfi\n",
            cwd = quoted(fixture.cwd()),
            claudine = sh_quote(claudine_bin()),
        );
        let launcher = fixture.workspace_path().join("launch.sh");
        write_executable(&launcher, &script);
        launcher
    }

    #[test]
    #[serial(level2_terminal)]
    fn level2_tmux_codex_repo_overlay_keeps_the_user_home_in_an_interactive_launch() {
        require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

        let fixture = CliProcessFixture::named("l2-overlay-codex");
        let codex = seed_codex_home(&fixture);
        let profile = fixture.home().join("profile with space");
        let marker = unique_marker();
        let launcher = write_launcher(&fixture, &profile, &marker);

        let mut harness = TmuxHarness::shared_or_spawn().expect("tmux harness");
        harness
            .send_text(format!("/bin/sh {}\n", sh_quote(&launcher.display().to_string())).as_bytes())
            .expect("send launcher");
        let (frame, status) = wait_for_exit_marker(&mut harness, &marker, Duration::from_secs(30));

        let launches = fixture.home().join(".claudine").join("overlays");
        assert_overlay_launch(&fixture, &profile, &launches, &frame.plain, &status);

        // Unix mirrors each top-level entry as a symbolic link to the user's
        // entry, which is what makes the nested rule reachable at depth.
        let provider = read_record(&fixture.cwd().join("child-env.txt"));
        assert!(has_line(&provider, &line("RULES_LINK", codex.join("rules"))), "{provider}");
    }
}

#[cfg(windows)]
mod windows {
    use super::*;
    use std::time::Instant;

    use biscuit_test_harness::wezterm::WezTermHarness;
    use test_toolkit::{Backend, Level, require_level};

    /// One recorder for every role, selected by its own file stem: `codex`
    /// records the provider's view and runs the nested tools; any other stem
    /// records a nested tool's home variables.
    const RECORDER_SOURCE: &str = r##"use std::io::{IsTerminal, Write};
use std::path::PathBuf;

fn var(name: &str) -> String {
    std::env::var_os(name).map_or_else(|| "<unset>".to_string(), |value| value.to_string_lossy().into_owned())
}

fn read(root: &PathBuf, segments: &[&str]) -> String {
    let path = segments.iter().fold(root.clone(), |path, segment| path.join(segment));
    std::fs::read_to_string(path).map_or_else(|_| "<missing>".to_string(), |text| text.trim().to_string())
}

fn main() {
    let exe = std::env::current_exe().expect("recorder path");
    let role = exe.file_stem().expect("recorder stem").to_string_lossy().into_owned();
    let mut record = String::new();
    for name in ["HOME", "USERPROFILE", "HOMEDRIVE", "HOMEPATH"] {
        record.push_str(&format!("{name}={}\n", var(name)));
    }
    if role != "codex" {
        std::fs::write(format!("{}.{role}", var("CLAUDINE_NESTED_FILE")), record).expect("record nested tool");
        return;
    }

    for name in ["CODEX_HOME", "CODEX_SQLITE_HOME"] {
        record.push_str(&format!("{name}={}\n", var(name)));
    }
    let yes_no = |tty: bool| if tty { "yes" } else { "no" };
    record.push_str(&format!("STDIN_TTY={}\n", yes_no(std::io::stdin().is_terminal())));
    record.push_str(&format!("STDOUT_TTY={}\n", yes_no(std::io::stdout().is_terminal())));
    let overlay = PathBuf::from(var("CODEX_HOME"));
    record.push_str(&format!("CONFIG={}\n", read(&overlay, &["config.toml"])));
    record.push_str(&format!("NESTED_RULE={}\n", read(&overlay, &["rules", "team", "nested", "deep.md"])));
    record.push_str(&format!("SKILL={}\n", read(&overlay, &["skills", "user", "SKILL.md"])));
    let present = |exists: bool| if exists { "present" } else { "absent" };
    record.push_str(&format!("OVERLAY_SQLITE={}\n", present(overlay.join("state_5.sqlite").exists())));
    let rules_kind = match std::fs::symlink_metadata(overlay.join("rules")) {
        Ok(meta) if meta.is_dir() && !meta.file_type().is_symlink() => "directory",
        Ok(_) => "link or file",
        Err(_) => "<missing>",
    };
    record.push_str(&format!("RULES_KIND={rules_kind}\n"));
    std::fs::write(var("CLAUDINE_ENV_FILE"), record).expect("record provider");

    // Write through the overlay's copy: a hard link or junction would carry
    // this into the user's file.
    let nested = ["rules", "team", "nested", "deep.md"].iter().fold(overlay, |path, segment| path.join(segment));
    let _ = std::fs::OpenOptions::new().append(true).open(nested).and_then(|mut file| file.write_all(b" + overlay edit"));

    let nested_bin = PathBuf::from(var("CLAUDINE_NESTED_BIN"));
    let path = std::env::join_paths(
        std::iter::once(nested_bin).chain(std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())),
    )
    .expect("nested PATH");
    for (tool, args) in [("git", ["status", ""]), ("gpg", ["--list-keys", ""]), ("gh", ["auth", "status"])] {
        let _ = std::process::Command::new(tool)
            .args(args.iter().filter(|arg| !arg.is_empty()))
            .env("PATH", &path)
            .status();
    }
    println!("fake codex session attached");
}
"##;

    fn compile_recorder(fixture: &CliProcessFixture) -> PathBuf {
        let source = fixture.workspace_path().join("recorder.rs");
        write(&source, RECORDER_SOURCE);
        let recorder = fixture.workspace_path().join("recorder.exe");
        let output = std::process::Command::new("rustc")
            .arg("--edition=2024")
            .arg(&source)
            .arg("-o")
            .arg(&recorder)
            .output()
            .expect("rustc must build the Windows recorder");
        assert!(
            output.status.success(),
            "recorder compilation failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        recorder
    }

    /// Writes the launcher the pane runs, returning its path.
    ///
    /// `setlocal` scopes every assignment to the launcher, and each ambient
    /// `CLAUDINE_*` and Codex selector is cleared before the fixture values are
    /// set, so the pane's inherited environment cannot reach the child.
    fn write_launcher(
        fixture: &CliProcessFixture,
        profile: &Path,
        codex: &Path,
        launches: &Path,
        marker: &str,
    ) -> PathBuf {
        let recorder = compile_recorder(fixture);
        let nested_bin = fixture.workspace_path().join("nested-bin");
        fs::create_dir_all(&nested_bin).unwrap();
        for tool in NESTED_TOOLS {
            fs::copy(&recorder, nested_bin.join(format!("{tool}.exe"))).unwrap();
        }
        fs::copy(&recorder, fixture.bin_dir().join("codex.exe")).unwrap();

        let path = std::env::join_paths(
            std::iter::once(fixture.bin_dir().to_path_buf()).chain(minimal_system_path()),
        )
        .expect("join fixture PATH");
        let home = fixture.home().display().to_string();
        let assignments = [
            ("HOME", home.clone()),
            ("USERPROFILE", profile.display().to_string()),
            ("APPDATA", fixture.home().join("AppData").join("Roaming").display().to_string()),
            ("LOCALAPPDATA", fixture.home().join("AppData").join("Local").display().to_string()),
            ("PATH", path.to_string_lossy().into_owned()),
            // The known-folder profile would otherwise supply both roots.
            ("CODEX_HOME", codex.display().to_string()),
            ("CLAUDINE_OVERLAY_DIR", launches.display().to_string()),
            ("NO_COLOR", "1".to_string()),
            ("CLAUDINE_RENDEZVOUS_REPORT", "false".to_string()),
            ("PLAYA_DRY_RUN", "1".to_string()),
            ("PLAYA_SPOOL_DIR", fixture.audio_spool().display().to_string()),
            ("CLAUDINE_ENV_FILE", fixture.cwd().join("child-env.txt").display().to_string()),
            ("CLAUDINE_NESTED_FILE", fixture.cwd().join("nested-env").display().to_string()),
            ("CLAUDINE_NESTED_BIN", nested_bin.display().to_string()),
        ]
        .map(|(name, value)| format!("set \"{name}={value}\"\r\n"))
        .concat();
        let script = format!(
            "@echo off\r\nsetlocal\r\ncls\r\n\
             for /f \"delims==\" %%V in ('set CLAUDINE_ 2^>nul') do set \"%%V=\"\r\n\
             for %%V in (CODEX_HOME CODEX_SQLITE_HOME HOMEDRIVE HOMEPATH XDG_CONFIG_HOME) do set \"%%V=\"\r\n\
             {assignments}cd /d \"{cwd}\"\r\n\
             \"{claudine}\" codex --repo\r\n\
             if errorlevel 1 (echo {marker}:1) else (echo {marker}:0)\r\n",
            cwd = fixture.cwd().display(),
            claudine = claudine_bin(),
        );
        let launcher = fixture.workspace_path().join("launch.cmd");
        write(&launcher, &script);
        launcher
    }

    /// Polls until `cmd.exe` draws its `>` prompt: `spawn_program` returns
    /// before the shell reads input, and text typed earlier is lost.
    fn wait_for_cmd_prompt(harness: &mut WezTermHarness, deadline: Duration) {
        let started = Instant::now();
        loop {
            let frame = harness.capture().expect("capture WezTerm pane");
            if frame.plain.lines().any(|line| line.trim_end().ends_with('>')) {
                return;
            }
            assert!(
                started.elapsed() < deadline,
                "cmd.exe drew no prompt within {deadline:?}:\n{}",
                frame.plain
            );
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    #[test]
    #[serial(level2_terminal)]
    fn level2_wezterm_windows_codex_repo_overlay_copies_nested_directories_and_keeps_the_user_home() {
        require_level!(Level::L2, WezTermHarness::available(), Backend::WezTerm);

        let fixture = CliProcessFixture::named("l2-overlay-codex-win");
        // The per-user known folders resolve beneath the profile only when
        // these exist (`os` skill, windows.md).
        for folder in ["Local", "Roaming"] {
            fs::create_dir_all(fixture.home().join("AppData").join(folder)).unwrap();
        }
        let codex = seed_codex_home(&fixture);
        let profile = fixture.home().to_path_buf();
        let launches = fixture.workspace_path().join("overlay-launches");
        let marker = unique_marker();
        let launcher = write_launcher(&fixture, &profile, &codex, &launches, &marker);

        let mut harness = WezTermHarness::new();
        harness.spawn_program("cmd.exe", &[]).expect("spawn WezTerm cmd.exe pane");
        wait_for_cmd_prompt(&mut harness, Duration::from_secs(30));
        harness
            .send_text(format!("call \"{}\"\r\n", launcher.display()).as_bytes())
            .expect("send launcher");
        let (frame, status) = wait_for_exit_marker(&mut harness, &marker, Duration::from_secs(60));

        assert_overlay_launch(&fixture, &profile, &launches, &frame.plain, &status);

        let provider = read_record(&fixture.cwd().join("child-env.txt"));
        assert!(
            has_line(&provider, "RULES_KIND=directory"),
            "native Windows must materialize a real directory, not a link or junction:\n{provider}"
        );
        assert_eq!(
            fs::read_to_string(nested_rule(&codex)).unwrap(),
            NESTED_RULE_TEXT,
            "a write through the overlay reached the user's file, so it was not copied"
        );
    }
}
