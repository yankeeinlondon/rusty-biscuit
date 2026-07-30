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
//!     RUN_SHELL_TESTS=1 cargo test -p biscuit-tui-cli --test completions_shell

use std::env;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use std::time::SystemTime;

static TEMP_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

fn make_temp_dir(label: &str) -> PathBuf {
    let counter = TEMP_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("system clock should be after Unix epoch")
        .as_nanos();
    let mut dir = env::temp_dir();
    dir.push(format!(
        "question_shell_tests_{}_{}_{}_{}",
        std::process::id(),
        counter,
        nanos,
        label
    ));
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
    assert_cmd::cargo::cargo_bin("question").to_path_buf()
}

fn generate_completion_script(shell: &str) -> String {
    let binary = question_binary();
    let output = std::process::Command::new(&binary)
        .args(["completions", shell])
        .output()
        .expect("failed to run `question completions {shell}`");
    assert!(
        output.status.success(),
        "question completions {shell} failed"
    );
    String::from_utf8(output.stdout).expect("completion script is valid UTF-8")
}

// ---------------------------------------------------------------------------
// zsh tests
// ---------------------------------------------------------------------------

#[cfg(unix)]
mod zsh {
    use super::*;
    use expectrl::{Session, session::OsSession};
    use std::fs;

    fn spawn_zsh_with_completions() -> OsSession {
        let script = generate_completion_script("zsh");
        let tmp_dir = make_temp_dir("zsh");
        let fpath_dir = tmp_dir.join("fpath");
        fs::create_dir_all(&fpath_dir).expect("create fpath dir");
        fs::write(fpath_dir.join("_question"), script).expect("write _question completion script");
        fs::write(
            tmp_dir.join(".zshrc"),
            format!(
                "fpath=(\"{}\" $fpath)\nautoload -U compinit\ncompinit -u\nsource \"{}/_question\"\ncompdef _question question\nPS1='%# '\n",
                fpath_dir.display(),
                fpath_dir.display()
            ),
        )
        .expect("write zsh startup file");

        let mut command = Command::new("zsh");
        command.arg("-i").env("ZDOTDIR", &tmp_dir);
        let mut session = Session::spawn(command).expect("spawn zsh");
        session.set_expect_timeout(Some(Duration::from_secs(10)));
        // Give zsh time to initialise compinit
        std::thread::sleep(Duration::from_millis(500));
        let _ = read_all_available(&mut session);
        session
    }

    fn read_all_available(session: &mut OsSession) -> String {
        let mut buf = Vec::new();
        let mut scratch = [0u8; 4096];
        let deadline = std::time::Instant::now() + Duration::from_millis(900);
        // Read with a short timeout to drain what is already available.
        session.set_expect_timeout(Some(Duration::from_millis(300)));
        while std::time::Instant::now() < deadline {
            match session.try_read(&mut scratch) {
                Ok(0) => break,
                Ok(n) => buf.extend_from_slice(&scratch[..n]),
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(20));
                }
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut => break,
                Err(_) => break,
            }
        }
        session.set_expect_timeout(Some(Duration::from_secs(10)));
        String::from_utf8_lossy(&buf).into_owned()
    }

    #[test]
    fn hotkey_prefix_quoted_bracket_tab() {
        if skip_if_not_enabled() {
            return;
        }
        let mut session = spawn_zsh_with_completions();
        session
            .write_all(b"question choose-one \"[\t")
            .expect("send command+tab");
        std::thread::sleep(Duration::from_millis(400));
        let output = read_all_available(&mut session);
        assert_candidates_contain_only_hotkey_prefixes(&output);
    }

    #[test]
    fn hotkey_prefix_bare_bracket_tab() {
        if skip_if_not_enabled() {
            return;
        }
        let mut session = spawn_zsh_with_completions();
        // Unquoted `[` in zsh requires escaping or quoting to avoid glob
        // behaviour.  We use a single-quoted string here.
        session
            .write_all(b"question choose-one '['\t")
            .expect("send command+tab");
        std::thread::sleep(Duration::from_millis(400));
        let output = read_all_available(&mut session);
        assert_candidates_contain_only_hotkey_prefixes(&output);
    }

    #[test]
    fn empty_option_position_does_not_offer_hotkey_prefixes() {
        if skip_if_not_enabled() {
            return;
        }
        let mut session = spawn_zsh_with_completions();
        session
            .write_all(b"question choose-one \t")
            .expect("send command+tab");
        std::thread::sleep(Duration::from_millis(400));
        let output = read_all_available(&mut session);
        assert!(
            !output.contains("[CTRL+") && !output.contains("[ALT+") && !output.contains("[OPT+"),
            "empty option position must not offer hotkey prefixes, got: {output:?}"
        );
    }

    #[test]
    fn post_positional_flag_completion() {
        if skip_if_not_enabled() {
            return;
        }
        let mut session = spawn_zsh_with_completions();
        session
            .write_all(b"question choose-one a b c d --border --border-label X --\t")
            .expect("send command+tab");
        std::thread::sleep(Duration::from_millis(400));
        let output = read_all_available(&mut session);
        // The candidate set must include well-known flags, proving that
        // `--` did not disable further option suggestions.
        assert!(
            output.contains("--csv")
                || output.contains("--list")
                || output.contains("--rows")
                || output.contains("--numeric-hot-keys")
                || output.contains("--no-filter")
                || output.contains("--required")
                || output.contains("--file")
                || output.contains("--md"),
            "post-positional tab should suggest option flags, got: {output:?}"
        );
    }

    #[test]
    fn root_subcommand_completion() {
        if skip_if_not_enabled() {
            return;
        }
        let mut session = spawn_zsh_with_completions();
        session.write_all(b"question \t").expect("send command+tab");
        std::thread::sleep(Duration::from_millis(400));
        let output = read_all_available(&mut session);
        assert!(
            output.contains("choose-one") && output.contains("choose-many"),
            "root tab should list subcommands, got: {output:?}"
        );
    }

    /// Reproduces the bug: typing `--active-color <SP><TAB>` should
    /// suggest the four enum values, not file paths or `[`.
    #[test]
    fn enum_value_completion_active_color_space_form() {
        if skip_if_not_enabled() {
            return;
        }
        let mut session = spawn_zsh_with_completions();
        session
            .write_all(b"question choose-many --active-color \t\t")
            .expect("send command+tab");
        std::thread::sleep(Duration::from_millis(600));
        let output = read_all_available(&mut session);
        for v in ["grey", "green", "yellow", "red"] {
            assert!(
                output.contains(v),
                "--active-color <SP><TAB> should suggest {v}, got: {output:?}"
            );
        }
    }

    #[test]
    fn enum_value_completion_sort_space_form() {
        if skip_if_not_enabled() {
            return;
        }
        let mut session = spawn_zsh_with_completions();
        session
            .write_all(b"question choose-many --sort \t\t")
            .expect("send command+tab");
        std::thread::sleep(Duration::from_millis(600));
        let output = read_all_available(&mut session);
        for v in ["natural", "inverse", "asc", "desc"] {
            assert!(
                output.contains(v),
                "--sort <SP><TAB> should suggest {v}, got: {output:?}"
            );
        }
    }

    #[test]
    fn enum_value_completion_after_positionals() {
        if skip_if_not_enabled() {
            return;
        }
        let mut session = spawn_zsh_with_completions();
        session
            .write_all(b"question choose-many foo bar --active-color \t\t")
            .expect("send command+tab");
        std::thread::sleep(Duration::from_millis(600));
        let output = read_all_available(&mut session);
        for v in ["grey", "green", "yellow", "red"] {
            assert!(
                output.contains(v),
                "--active-color <SP><TAB> after positionals should suggest {v}, got: {output:?}"
            );
        }
    }

    /// Cover every enum-typed flag advertised by the choose subcommands
    /// when the cursor sits at the option-value position after positional
    /// args have already been typed.  The catch-all positional completer
    /// must dispatch to the option's value action for each.
    #[test]
    fn enum_value_completion_all_flags_after_positionals() {
        if skip_if_not_enabled() {
            return;
        }
        let cases: &[(&str, &[&str])] = &[
            ("--active-color", &["grey", "green", "yellow", "red"]),
            ("--sort", &["natural", "inverse", "asc", "desc"]),
            ("--label-position", &["above", "below", "left", "right"]),
            (
                "--hotkey-badges",
                &["auto", "always", "never", "ctrl", "alt"],
            ),
            ("--output", &["raw", "json", "null"]),
            (
                "--label-convention",
                &[
                    "camel-case",
                    "pascal-case",
                    "kebab-case",
                    "snake-case",
                    "title-case",
                ],
            ),
            (
                "--value-convention",
                &[
                    "camel-case",
                    "pascal-case",
                    "kebab-case",
                    "snake-case",
                    "title-case",
                ],
            ),
            (
                "--border-style",
                &["none", "rounded", "sharp", "bold", "double", "block"],
            ),
        ];
        let mut session = spawn_zsh_with_completions();
        for (flag, expected) in cases {
            let cmd = format!("question choose-many a b c {flag} \t\t");
            session.write_all(cmd.as_bytes()).expect("send command+tab");
            std::thread::sleep(Duration::from_millis(700));
            let output = read_all_available(&mut session);
            for v in *expected {
                assert!(
                    output.contains(v),
                    "{flag} <SP><TAB> after positionals should suggest {v}, got: {output:?}"
                );
            }
            // Reset the line for the next iteration.
            session.write_all(b"\x15\r").expect("clear line");
            std::thread::sleep(Duration::from_millis(200));
            let _ = read_all_available(&mut session);
        }
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
    use expectrl::{Session, session::OsSession};
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
            && output.status.success()
        {
            let prefix = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let p = PathBuf::from(&prefix).join("etc/profile.d/bash_completion.sh");
            if p.exists() {
                return Some(p);
            }
        }
        None
    }

    fn spawn_bash_with_completions() -> Option<OsSession> {
        let script = generate_completion_script("bash");
        let tmp_dir = make_temp_dir("bash");
        let comp_dir = tmp_dir.join("bash_completion.d");
        fs::create_dir_all(&comp_dir).expect("create bash_completion.d dir");
        let completion_script = comp_dir.join("question");
        fs::write(&completion_script, script).expect("write question completion script");
        let mut bashrc = String::new();
        if let Some(bcomp) = bash_completion_script_path() {
            bashrc.push_str(&format!("source '{}'\n", bcomp.display()));
        }
        bashrc.push_str("bind 'set show-all-if-ambiguous on'\n");
        bashrc.push_str("bind 'set completion-query-items 0'\n");
        bashrc.push_str(&format!(
            "source '{}'\nPS1='$ '\n",
            completion_script.display()
        ));
        let bashrc_path = tmp_dir.join("bashrc");
        fs::write(&bashrc_path, bashrc).expect("write bash startup file");

        let mut command = Command::new("bash");
        command.arg("--rcfile").arg(&bashrc_path).arg("-i");
        let mut session = Session::spawn(command).ok()?;
        session.set_expect_timeout(Some(Duration::from_secs(10)));
        std::thread::sleep(Duration::from_millis(500));
        let _ = read_all_available(&mut session);

        Some(session)
    }

    fn read_all_available(session: &mut OsSession) -> String {
        let mut buf = Vec::new();
        let mut scratch = [0u8; 4096];
        let deadline = std::time::Instant::now() + Duration::from_millis(900);
        session.set_expect_timeout(Some(Duration::from_millis(300)));
        while std::time::Instant::now() < deadline {
            match session.try_read(&mut scratch) {
                Ok(0) => break,
                Ok(n) => buf.extend_from_slice(&scratch[..n]),
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(20));
                }
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut => break,
                Err(_) => break,
            }
        }
        session.set_expect_timeout(Some(Duration::from_secs(10)));
        String::from_utf8_lossy(&buf).into_owned()
    }

    #[test]
    fn hotkey_prefix_bracket_tab() {
        if skip_if_not_enabled() {
            return;
        }
        let mut session = match spawn_bash_with_completions() {
            Some(s) => s,
            None => {
                eprintln!("skipping: could not spawn bash with completions");
                return;
            }
        };
        session
            .write_all(b"question choose-one \\[\t\t")
            .expect("send command+tab");
        std::thread::sleep(Duration::from_millis(400));
        let output = read_all_available(&mut session);
        assert_candidates_contain_only_hotkey_prefixes(&output);
    }

    #[test]
    fn empty_option_position_does_not_offer_hotkey_prefixes() {
        if skip_if_not_enabled() {
            return;
        }
        let mut session = match spawn_bash_with_completions() {
            Some(s) => s,
            None => {
                eprintln!("skipping: could not spawn bash with completions");
                return;
            }
        };
        session
            .write_all(b"question choose-one \t\t")
            .expect("send command+tab");
        std::thread::sleep(Duration::from_millis(400));
        let output = read_all_available(&mut session);
        assert!(
            !output.contains("[CTRL+") && !output.contains("[ALT+") && !output.contains("[OPT+"),
            "empty option position must not offer hotkey prefixes, got: {output:?}"
        );
    }

    #[test]
    fn post_positional_flag_completion() {
        if skip_if_not_enabled() {
            return;
        }
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
            output.contains("--csv")
                || output.contains("--list")
                || output.contains("--rows")
                || output.contains("--numeric-hot-keys")
                || output.contains("--no-filter")
                || output.contains("--required")
                || output.contains("--file")
                || output.contains("--md"),
            "post-positional tab should suggest option flags, got: {output:?}"
        );
    }

    #[test]
    fn root_subcommand_completion() {
        if skip_if_not_enabled() {
            return;
        }
        let mut session = match spawn_bash_with_completions() {
            Some(s) => s,
            None => {
                eprintln!("skipping: could not spawn bash with completions");
                return;
            }
        };
        session
            .write_all(b"question \t\t")
            .expect("send command+tab");
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

    #[test]
    fn enum_value_completion_active_color_after_positionals() {
        if skip_if_not_enabled() {
            return;
        }
        let mut session = match spawn_bash_with_completions() {
            Some(s) => s,
            None => {
                eprintln!("skipping: could not spawn bash with completions");
                return;
            }
        };
        session
            .write_all(b"question choose-many foo bar --active-color \t\t")
            .expect("send command+tab");
        std::thread::sleep(Duration::from_millis(600));
        let output = read_all_available(&mut session);
        for v in ["grey", "green", "yellow", "red"] {
            assert!(
                output.contains(v),
                "--active-color <SP><TAB> after positionals should suggest {v}, got: {output:?}"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn make_temp_dir_returns_unique_existing_directories() {
        let first = make_temp_dir("unit");
        let second = make_temp_dir("unit");

        assert_ne!(first, second);
        assert!(first.is_dir());
        assert!(second.is_dir());
    }
}

#[cfg(not(unix))]
mod skip_unix_only {
    use super::*;

    #[test]
    fn shell_tests_are_unix_only() {
        if skip_if_not_enabled() {
            return;
        }
        eprintln!("shell completion PTY tests are unix-only");
    }
}
