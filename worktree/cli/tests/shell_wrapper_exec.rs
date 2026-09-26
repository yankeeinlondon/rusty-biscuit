//! Runs the generated POSIX wrapper in a real shell against a stub `wt` that
//! prints protocol lines and logs every call, so the wrapper's handling of
//! `cd:` and `remove-handoff:` is proven by behavior, not by its text.
//!
//! bash is present on every Unix host and zsh on every macOS host. fish and
//! PowerShell are exercised by the L2 wrapper tests.
#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::Command;

use clap_complete::Shell;

// Logs the wrapper announcement and each argument in brackets, so argument
// boundaries are visible; the handoff call prints nothing.
const STUB: &str = r#"#!/bin/sh
{
    printf 'WT_SHELL_WRAPPER=%s ' "${WT_SHELL_WRAPPER-unset}"
    for arg in "$@"; do printf '[%s]' "$arg"; done
    printf '\n'
} >> "$STUB_LOG"
if [ "$1" = "remove" ] && [ "$2" = "--handoff" ]; then
    exit 0
fi
cat "$STUB_OUTPUT"
exit "${STUB_RC:-0}"
"#;

struct Fixture {
    dir: tempfile::TempDir,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("bin");
        fs::create_dir(&bin).unwrap();
        let stub = bin.join("wt");
        fs::write(&stub, STUB).unwrap();
        fs::set_permissions(&stub, fs::Permissions::from_mode(0o755)).unwrap();
        Self { dir }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }

    /// Sources the wrapper, runs `wt go x`, then prints the shell's directory
    /// after a marker line.
    fn run(&self, shell: Shell, program: &str, stub_output: &str, stub_rc: i32) -> Outcome {
        fs::write(self.path("wrapper.sh"), wrapper(shell)).unwrap();
        fs::write(self.path("stub-output"), stub_output).unwrap();
        let _ = fs::remove_file(self.path("stub.log"));
        let script = "\
            . \"$WRAPPER\"\n\
            cd \"$START\" || exit 90\n\
            wt go x\n\
            rc=$?\n\
            printf 'PWD-MARKER\\n%s\\n' \"$(pwd -P)\"\n\
            exit $rc\n";
        let start = self.path("start");
        fs::create_dir_all(&start).unwrap();
        let mut command = Command::new(program);
        if shell == Shell::Zsh {
            command.arg("-f");
        }
        let output = command
            .arg("-c")
            .arg(script)
            .current_dir(self.dir.path())
            .env_clear()
            .env("PATH", format!("{}:/usr/bin:/bin", self.path("bin").display()))
            .env("HOME", self.dir.path())
            .env("WRAPPER", self.path("wrapper.sh"))
            .env("START", &start)
            .env("STUB_LOG", self.path("stub.log"))
            .env("STUB_OUTPUT", self.path("stub-output"))
            .env("STUB_RC", stub_rc.to_string())
            .output()
            .unwrap_or_else(|e| panic!("{program} must be installed: {e}"));
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let (printed, pwd) = stdout
            .split_once("PWD-MARKER\n")
            .unwrap_or_else(|| panic!("no marker in {stdout:?}"));
        Outcome {
            code: output.status.code().unwrap_or(-1),
            printed: printed.to_string(),
            pwd: PathBuf::from(pwd.trim_end()),
            calls: fs::read_to_string(self.path("stub.log"))
                .unwrap_or_default()
                .lines()
                .map(str::to_string)
                .collect(),
        }
    }
}

struct Outcome {
    code: i32,
    printed: String,
    pwd: PathBuf,
    calls: Vec<String>,
}

fn wrapper(shell: Shell) -> String {
    worktree_cli::shell_integration::wrapper(shell, "unused").unwrap()
}

fn canonical(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap()
}

fn shells() -> Vec<(Shell, &'static str)> {
    let mut shells = vec![(Shell::Bash, "bash")];
    if cfg!(target_os = "macos") {
        shells.push((Shell::Zsh, "zsh"));
    }
    shells
}

#[test]
fn wrapper_changes_directory_then_runs_the_handoff_with_the_literal_token() {
    for (shell, program) in shells() {
        let fixture = Fixture::new();
        let dest = fixture.path("dest dir/café-ü");
        fs::create_dir_all(&dest).unwrap();
        let pwned = fixture.path("pwned");
        let token = format!("tok$(touch {})`id`;x", pwned.display());
        let output = format!(
            "plain line\ncd:{}\nremove-handoff:{token}\nlast line\n",
            dest.display()
        );

        let outcome = fixture.run(shell, program, &output, 0);

        assert_eq!(outcome.code, 0, "{program}");
        assert_eq!(outcome.pwd, canonical(&dest), "{program}");
        assert_eq!(outcome.printed, "plain line\nlast line\n", "{program}");
        assert_eq!(
            outcome.calls,
            vec![
                "WT_SHELL_WRAPPER=1 [go][x]".to_string(),
                format!("WT_SHELL_WRAPPER=1 [remove][--handoff][{token}]"),
            ],
            "{program}"
        );
        assert!(!pwned.exists(), "{program} evaluated the token");
    }
}

#[test]
fn wrapper_stops_before_the_handoff_when_cd_fails() {
    for (shell, program) in shells() {
        let fixture = Fixture::new();
        let missing = fixture.path("missing");
        let output = format!("cd:{}\nremove-handoff:tok\n", missing.display());

        let outcome = fixture.run(shell, program, &output, 0);

        assert_eq!(outcome.code, 1, "{program}");
        assert_eq!(outcome.pwd, canonical(&fixture.path("start")), "{program}");
        assert_eq!(outcome.calls, vec!["WT_SHELL_WRAPPER=1 [go][x]"], "{program}");
    }
}

#[test]
fn wrapper_passes_a_failure_through_without_acting_on_protocol_lines() {
    for (shell, program) in shells() {
        let fixture = Fixture::new();
        let dest = fixture.path("dest");
        fs::create_dir_all(&dest).unwrap();
        let output = format!("cd:{}\nremove-handoff:tok\nwhy it failed\n", dest.display());

        let outcome = fixture.run(shell, program, &output, 4);

        assert_eq!(outcome.code, 4, "{program}");
        assert_eq!(outcome.pwd, canonical(&fixture.path("start")), "{program}");
        assert_eq!(outcome.printed, "why it failed\n", "{program}");
        assert_eq!(outcome.calls, vec!["WT_SHELL_WRAPPER=1 [go][x]"], "{program}");
    }
}

#[test]
fn wrapper_with_no_protocol_lines_prints_output_and_stays() {
    for (shell, program) in shells() {
        let fixture = Fixture::new();

        let empty = fixture.run(shell, program, "", 0);
        assert_eq!(empty.code, 0, "{program}");
        assert_eq!(empty.printed, "", "{program}");
        assert_eq!(empty.pwd, canonical(&fixture.path("start")), "{program}");

        let table = fixture.run(shell, program, "row 1\n\nrow 3\n", 0);
        assert_eq!(table.code, 0, "{program}");
        assert_eq!(table.printed, "row 1\n\nrow 3\n", "{program}");
    }
}
