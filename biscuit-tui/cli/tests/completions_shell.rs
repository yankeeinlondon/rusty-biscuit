//! PTY-driven shell completion tests.
//!
//! These tests spawn real `zsh` and `bash` shells, install the generated
//! completion script into a temporary `fpath` / `bash_completion.d`
//! directory, and assert candidate lists for interactive `<TAB>` scenarios.
//!
//! They are gated behind the `RUN_SHELL_TESTS=1` environment variable so
//! that default `cargo test` stays fast and does not depend on shell
//! availability.
//!
//! Run with:
//!     RUN_SHELL_TESTS=1 cargo test -p tui-chrome-cli --test completions_shell

use std::env;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

fn make_temp_dir() -> PathBuf {
    let mut dir = env::temp_dir();
    dir.push(format!("question_shell_tests_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn shell_tests_enabled() -> bool {
    env::var("RUN_SHELL_TESTS").as_deref() == Ok("1")
}

fn skip_if_not_enabled() -> bool {
    if !shell_tests_enabled() {
        eprintln!("skipping: set RUN_SHELL_TESTS=1 to enable shell completion PTY tests");
        return true;
    }
    false
}

fn question_binary() -> PathBuf {
    assert_cmd::cargo::cargo_bin("question")
}

fn generate_completion_script(shell: &str) -> String {
    let binary = question_binary();
    let output = std::process::Command::new(&binary)
        .args(["completions", shell])
        .output()
        .expect("failed to run `question completions {shell}`");
    assert!(output.status.success(), "question completions {shell} failed");
    String::from_utf8(output.stdout).expect("completion script is valid UTF-8")
}

// ---------------------------------------------------------------------------
// zsh tests
// ---------------------------------------------------------------------------

#[cfg(unix)]
mod zsh {
    use super::*;
    use expectrl::{session::OsSession, spawn};
    use std::fs;

    fn spawn_zsh_with_completions() -> OsSession {
        let script = generate_completion_script("zsh");
        let tmp_dir = make_temp_dir();
        let fpath_dir = tmp_dir.join("fpath");
        fs::create_dir(&fpath_dir).expect("create fpath dir");
        fs::write(fpath_dir.join("_question"), script).expect("write _question completion script");

        let fpath = fpath_dir.to_str().unwrap();
        let zsh_cmd = format!(
            "zsh -i -c 'FPATH={}:$FPATH; autoload -U compinit && compinit -u; exec zsh -i'",
            fpath
        );
        let mut session = spawn(&zsh_cmd).expect("spawn zsh");
        session.set_expect_timeout(Some(Duration::from_secs(10)));
        // Give zsh time to initialise compinit
        std::thread::sleep(Duration::from_millis(500));
        session
    }

    fn read_all_available(session: &mut OsSession) -> String {
        let mut buf = Vec::new();
        let mut scratch = [0u8; 4096];
        // Read with a short timeout to drain what is already available
        session.set_expect_timeout(Some(Duration::from_millis(300)));
        loop {
            match session.read(&mut scratch) {
                Ok(0) => break,
                Ok(n) => buf.extend_from_slice(&scratch[..n]),
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut => break,
                Err(_) => break,
            }
        }
        session.set_expect_timeout(Some(Duration::from_secs(10)));
        String::from_utf8_lossy(&buf).into_owned()
    }

    #[test]
    fn hotkey_prefix_quoted_bracket_tab() {
        if skip_if_not_enabled() { return; }
        let mut session = spawn_zsh_with_completions();
        session.write_all(b"question choose-one \"[\t").expect("send command+tab");
        std::thread::sleep(Duration::from_millis(400));
        let output = read_all_available(&mut session);
        assert_candidates_contain_only_hotkey_prefixes(&output);
    }

    #[test]
    fn hotkey_prefix_bare_bracket_tab() {
        if skip_if_not_enabled() { return; }
        let mut session = spawn_zsh_with_completions();
        // Unquoted `[` in zsh requires escaping or quoting to avoid glob
        // behaviour.  We use a single-quoted string here.
        session.write_all(b"question choose-one '['\t").expect("send command+tab");
        std::thread::sleep(Duration::from_millis(400));
        let output = read_all_available(&mut session);
        assert_candidates_contain_only_hotkey_prefixes(&output);
    }

    #[test]
    fn post_positional_flag_completion() {
        if skip_if_not_enabled() { return; }
        let mut session = spawn_zsh_with_completions();
        session
            .write_all(b"question choose-one a b c d --border --border-label X --\t")
            .expect("send command+tab");
        std::thread::sleep(Duration::from_millis(400));
        let output = read_all_available(&mut session);
        // The candidate set must include well-known flags, proving that
        // `--` did not disable further option suggestions.
        assert!(
            output.contains("--csv") || output.contains("--list") || output.contains("--rows")
                || output.contains("--numeric-hot-keys") || output.contains("--no-filter")
                || output.contains("--required") || output.contains("--file")
                || output.contains("--md"),
            "post-positional tab should suggest option flags, got: {output:?}"
        );
    }

    #[test]
    fn root_subcommand_completion() {
        if skip_if_not_enabled() { return; }
        let mut session = spawn_zsh_with_completions();
        session.write_all(b"question \t").expect("send command+tab");
        std::thread::sleep(Duration::from_millis(400));
        let output = read_all_available(&mut session);
        assert!(
            output.contains("choose-one") && output.contains("choose-many"),
            "root tab should list subcommands, got: {output:?}"
        );
    }

    fn assert_candidates_contain_only_hotkey_prefixes(output: &str) {
        // We expect the three hotkey prefixes to appear and no command/file
        // fallback pollution such as `[` or `[[`.
        assert!(
            output.contains("[CTRL+") && output.contains("[ALT+") && output.contains("[OPT+"),
            "hotkey prefixes should be present, got: {output:?}"
        );
        // Reject the common zsh fallback pollution
        let bad = [" [\n", " [[\n", "[`"];
        for b in &bad {
            assert!(
                !output.contains(b),
                "must not contain fallback pollution {b:?}, got: {output:?}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// bash tests
// ---------------------------------------------------------------------------

#[cfg(unix)]
mod bash {
    use super::*;
    use expectrl::{session::OsSession, spawn};
    use std::fs;

    fn bash_completion_script_path() -> Option<PathBuf> {
        let candidates = [
            "/usr/share/bash-completion/bash_completion",
            "/usr/local/etc/bash_completion",
            "/opt/homebrew/etc/bash_completion",
            "/opt/homebrew/etc/profile.d/bash_completion.sh",
        ];
        for p in &candidates {
            if Path::new(p).exists() {
                return Some(PathBuf::from(p));
            }
        }
        // Try to find via brew prefix
        if let Ok(output) = std::process::Command::new("brew")
            .args(["--prefix", "bash-completion@2"])
            .output()
        {
            if output.status.success() {
                let prefix = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let p = PathBuf::from(&prefix).join("etc/profile.d/bash_completion.sh");
                if p.exists() {
                    return Some(p);
                }
            }
        }
        None
    }

    fn spawn_bash_with_completions() -> Option<OsSession> {
        let script = generate_completion_script("bash");
        let tmp_dir = make_temp_dir();
        let comp_dir = tmp_dir.join("bash_completion.d");
        fs::create_dir(&comp_dir).expect("create bash_completion.d dir");
        fs::write(comp_dir.join("question"), script).expect("write question completion script");

        let comp_dir_str = comp_dir.to_str().unwrap();
        let bash_cmd = format!(
            "bash --norc -i -c 'for f in {}/question; do source \"$f\"; done; exec bash --norc -i'",
            comp_dir_str
        );
        let mut session = spawn(&bash_cmd).ok()?;
        session.set_expect_timeout(Some(Duration::from_secs(10)));
        std::thread::sleep(Duration::from_millis(300));

        // If bash-completion is available, source it for richer behaviour
        if let Some(bcomp) = bash_completion_script_path() {
            let cmd = format!("source '{}'\n", bcomp.to_str().unwrap());
            let _ = session.write_all(cmd.as_bytes());
            std::thread::sleep(Duration::from_millis(300));
        }

        Some(session)
    }

    fn read_all_available(session: &mut OsSession) -> String {
        let mut buf = Vec::new();
        let mut scratch = [0u8; 4096];
        session.set_expect_timeout(Some(Duration::from_millis(300)));
        loop {
            match session.read(&mut scratch) {
                Ok(0) => break,
                Ok(n) => buf.extend_from_slice(&scratch[..n]),
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut => break,
                Err(_) => break,
            }
        }
        session.set_expect_timeout(Some(Duration::from_secs(10)));
        String::from_utf8_lossy(&buf).into_owned()
    }

    #[test]
    fn hotkey_prefix_bracket_tab() {
        if skip_if_not_enabled() { return; }
        let mut session = match spawn_bash_with_completions() {
            Some(s) => s,
            None => {
                eprintln!("skipping: could not spawn bash with completions");
                return;
            }
        };
        session.write_all(b"question choose-one '[\t\t").expect("send command+tab");
        std::thread::sleep(Duration::from_millis(400));
        let output = read_all_available(&mut session);
        assert_candidates_contain_only_hotkey_prefixes(&output);
    }

    #[test]
    fn post_positional_flag_completion() {
        if skip_if_not_enabled() { return; }
        let mut session = match spawn_bash_with_completions() {
            Some(s) => s,
            None => {
                eprintln!("skipping: could not spawn bash with completions");
                return;
            }
        };
        session
            .write_all(b"question choose-one a b c d --border --border-label X --\t\t")
            .expect("send command+tab");
        std::thread::sleep(Duration::from_millis(400));
        let output = read_all_available(&mut session);
        assert!(
            output.contains("--csv") || output.contains("--list") || output.contains("--rows")
                || output.contains("--numeric-hot-keys") || output.contains("--no-filter")
                || output.contains("--required") || output.contains("--file")
                || output.contains("--md"),
            "post-positional tab should suggest option flags, got: {output:?}"
        );
    }

    #[test]
    fn root_subcommand_completion() {
        if skip_if_not_enabled() { return; }
        let mut session = match spawn_bash_with_completions() {
            Some(s) => s,
            None => {
                eprintln!("skipping: could not spawn bash with completions");
                return;
            }
        };
        session.write_all(b"question \t\t").expect("send command+tab");
        std::thread::sleep(Duration::from_millis(400));
        let output = read_all_available(&mut session);
        assert!(
            output.contains("choose-one") && output.contains("choose-many"),
            "root tab should list subcommands, got: {output:?}"
        );
    }

    fn assert_candidates_contain_only_hotkey_prefixes(output: &str) {
        assert!(
            output.contains("[CTRL+") && output.contains("[ALT+") && output.contains("[OPT+"),
            "hotkey prefixes should be present, got: {output:?}"
        );
        assert!(
            !output.contains(" [\n") && !output.contains(" [[\n"),
            "must not contain fallback pollution, got: {output:?}"
        );
    }
}

#[cfg(not(unix))]
mod skip_unix_only {
    use super::*;

    #[test]
    fn shell_tests_are_unix_only() {
        if skip_if_not_enabled() { return; }
        eprintln!("shell completion PTY tests are unix-only");
    }
}
