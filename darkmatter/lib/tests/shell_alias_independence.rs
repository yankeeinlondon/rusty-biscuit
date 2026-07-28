//! Composition and preflight must resolve `::shell` executables without
//! consulting the user's shell.
//!
//! Guards the requirements in
//! `darkmatter/fixes/2026-07-27-alias-resolution-hang/spec.md`: R1 (no
//! alias-query process), R2 (preflight and execution agree on the authored
//! executable), R3 (no interactive startup file is sourced), and R5 (results
//! do not depend on `$SHELL`). The controlling-terminal reproduction that
//! motivated them needs a real PTY and lives at
//! `darkmatter/cli/tests/level2_shell_alias_hang.rs`; everything here is
//! in-process, so it is the tier that runs in CI.
//!
//! Each test points `$SHELL` at a fixture script that records its own
//! invocation. A fixture stands in for a real interactive shell on purpose:
//! spawning one from a test process that may itself sit in a background
//! process group is the very hang under repair, so the durable guard must not
//! depend on it.

// Windows has no `$SHELL` convention and no executable-bit shebang dispatch,
// so the fixture-script technique has no faithful `.cmd` equivalent. The
// production path being guarded is `std::env::var("SHELL")`, which is unset
// on Windows and therefore already inert there.
#![cfg(unix)]

use darkmatter::markdown::compose::shell_expansion::types::{
    ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest,
};
use darkmatter::markdown::compose::{ComposeOptions, ShellExpansionError, collect_shell_commands};
use darkmatter::markdown::{Markdown, MarkdownError};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tempfile::TempDir;
use test_toolkit::EnvGuard;

/// Executable that resolves nowhere, so every path under test takes the
/// `which::which` miss branch.
const MISSING_EXECUTABLE: &str = "nonexistent_command_xyz";

/// Argument the aliasing fixture claims [`MISSING_EXECUTABLE`] expands to.
/// Distinctive enough that its appearance anywhere proves an alias rewrite
/// happened, and it cannot collide with a legitimately authored `echo`.
const ALIAS_REWRITE_MARKER: &str = "ALIAS_REWRITE_MARKER";

struct AllowAllHandler;

impl ShellApprovalHandler for AllowAllHandler {
    fn approve(
        &self,
        _request: ShellApprovalRequest,
    ) -> Result<ShellApprovalDecision, ShellExpansionError> {
        Ok(ShellApprovalDecision::AllowOnce)
    }
}

/// What a fixture `$SHELL` reports when asked to resolve an alias.
#[derive(Clone, Copy)]
enum AliasReply {
    /// Exits non-zero, the shape of a shell that knows no such alias.
    None,
    /// Reports [`MISSING_EXECUTABLE`] as an alias for `echo ALIAS_REWRITE_MARKER`.
    /// `echo` is on `PATH`, which is what makes the current implementation
    /// accept the rewrite.
    Rewrite,
}

/// A fixture `$SHELL` plus the sentinels that prove whether it ran.
struct ShellFixture {
    /// Path to install into `$SHELL`.
    program: PathBuf,
    /// Written by the fixture on every invocation.
    invocations: PathBuf,
    /// Written by the rc file the fixture sources, standing in for the
    /// `~/.bashrc` an interactive shell would run.
    rc_sourced: PathBuf,
}

impl ShellFixture {
    fn install(dir: &Path, name: &str, reply: AliasReply) -> Self {
        let program = dir.join(name);
        let invocations = dir.join(format!("{name}.invocations"));
        let rc_sourced = dir.join(format!("{name}.rc-sourced"));
        let rc = dir.join(format!("{name}.rc"));

        write_script(
            &rc,
            &format!("printf 'sourced\\n' >> {}\n", shell_quote(&rc_sourced)),
        );

        let reply_line = match reply {
            AliasReply::None => "exit 1\n".to_string(),
            AliasReply::Rewrite => format!(
                "printf \"alias {MISSING_EXECUTABLE}='echo {ALIAS_REWRITE_MARKER}'\\n\"\nexit 0\n"
            ),
        };
        write_script(
            &program,
            &format!(
                "printf '%s\\n' \"$*\" >> {invocations}\n. {rc}\n{reply_line}",
                invocations = shell_quote(&invocations),
                rc = shell_quote(&rc),
            ),
        );

        Self {
            program,
            invocations,
            rc_sourced,
        }
    }

    fn guard(&self) -> EnvGuard {
        EnvGuard::set_safe("SHELL", &self.program)
    }

    fn assert_never_ran(&self, context: &str) {
        assert!(
            !self.invocations.exists(),
            "{context}: composition spawned $SHELL; recorded invocations:\n{}",
            std::fs::read_to_string(&self.invocations).unwrap_or_default()
        );
        assert!(
            !self.rc_sourced.exists(),
            "{context}: composition sourced interactive shell configuration"
        );
    }
}

fn write_script(path: &Path, body: &str) {
    std::fs::write(path, format!("#!/bin/sh\n{body}")).expect("write fixture script");
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
        .expect("mark fixture script executable");
}

fn shell_quote(path: &Path) -> String {
    format!("'{}'", path.display().to_string().replace('\'', "'\\''"))
}

/// Writes `body` as `doc.md` and returns the options composition should use.
fn document(dir: &TempDir, body: &str) -> (Markdown, ComposeOptions) {
    let path = dir.path().join("doc.md");
    std::fs::write(&path, body).expect("write markdown fixture");
    let options = ComposeOptions::new()
        .with_source_file(path.clone())
        .with_shell_policy_root(dir.path())
        .with_shell_working_directory(dir.path().to_path_buf())
        .with_shell_approval_handler(Arc::new(AllowAllHandler));
    let markdown = Markdown::try_from(path.as_path()).expect("read markdown fixture");
    (markdown, options)
}

/// Unwraps the executable named by a `CommandNotFound`, failing with the
/// actual error otherwise.
fn command_not_found(err: MarkdownError) -> String {
    match err {
        MarkdownError::ShellExpansion(inner) => match *inner {
            ShellExpansionError::CommandNotFound { command, .. } => command,
            other => panic!("expected CommandNotFound, got {other:?}"),
        },
        other => panic!("expected a shell-expansion error, got {other:?}"),
    }
}

// ── R1 / R3: no alias-query process ──────────────────────────────────

#[test]
#[serial_test::serial]
fn compose_and_preflight_never_spawn_the_user_shell() {
    let dir = TempDir::new().unwrap();
    let shell = ShellFixture::install(dir.path(), "fixture-shell", AliasReply::None);
    let _guard = shell.guard();

    let (markdown, options) = document(&dir, &format!("# Test\n::shell {MISSING_EXECUTABLE}\n"));

    let entries = collect_shell_commands(&markdown, &options).expect("preflight collection");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].executable, MISSING_EXECUTABLE);
    shell.assert_never_ran("preflight");

    let err = markdown
        .compose_with(options)
        .expect_err("an unresolvable executable must fail composition");
    assert_eq!(command_not_found(err), MISSING_EXECUTABLE);
    shell.assert_never_ran("compose");
}

// ── R2: preflight and execution resolve identically ──────────────────

/// Darkmatter rejects `|` pipes, so a multi-action directive is the pipeline
/// shape; `&&` and `||` are its two chain forms. All three take different
/// branches of both the preflight collector and the runtime preparation, so
/// each is exercised.
#[test]
#[serial_test::serial]
fn preflight_and_runtime_report_the_authored_executable() {
    let shapes = [
        ("single", format!("::shell {MISSING_EXECUTABLE} --flag\n")),
        (
            "and-chain",
            format!("::shell {MISSING_EXECUTABLE} --flag && echo after\n"),
        ),
        (
            "or-chain",
            format!("::shell {MISSING_EXECUTABLE} --flag || echo fallback\n"),
        ),
    ];

    for (label, body) in shapes {
        let dir = TempDir::new().unwrap();
        // The aliasing fixture is what makes this test discriminating: a
        // shell that reports no alias would leave the authored executable in
        // place even under the implementation being removed.
        let shell = ShellFixture::install(dir.path(), "fixture-shell", AliasReply::Rewrite);
        let _guard = shell.guard();

        let (markdown, options) = document(&dir, &format!("# Test\n{body}"));

        let entries = collect_shell_commands(&markdown, &options).expect("preflight collection");
        let authored = entries
            .iter()
            .find(|entry| entry.executable == MISSING_EXECUTABLE)
            .unwrap_or_else(|| {
                panic!("{label}: preflight lost the authored executable; collected {entries:#?}")
            });
        assert_eq!(
            authored.args,
            vec!["--flag".to_string()],
            "{label}: preflight must keep the authored argv"
        );
        assert!(
            !entries
                .iter()
                .any(|entry| entry.normalized.contains(ALIAS_REWRITE_MARKER)),
            "{label}: preflight recorded a rewritten alias target; collected {entries:#?}"
        );

        let err = markdown
            .compose_with(options)
            .expect_err("an unresolvable executable must fail composition");
        assert_eq!(
            command_not_found(err),
            MISSING_EXECUTABLE,
            "{label}: execution must report the authored executable"
        );
    }
}

// ── R3 / R5: independent of shell configuration ──────────────────────

#[test]
#[serial_test::serial]
fn compose_result_is_independent_of_shell_configuration() {
    let quiet_dir = TempDir::new().unwrap();
    let quiet = ShellFixture::install(quiet_dir.path(), "quiet-shell", AliasReply::None);
    let aliasing_dir = TempDir::new().unwrap();
    let aliasing =
        ShellFixture::install(aliasing_dir.path(), "aliasing-shell", AliasReply::Rewrite);

    let quiet_result = compose_missing_command(&quiet);
    let aliasing_result = compose_missing_command(&aliasing);

    assert_eq!(
        quiet_result, aliasing_result,
        "changing $SHELL changed the composition result"
    );
    assert_eq!(quiet_result, MISSING_EXECUTABLE);

    quiet.assert_never_ran("quiet $SHELL");
    aliasing.assert_never_ran("aliasing $SHELL");
}

/// Composes a fresh copy of the same document under `shell`, returning the
/// executable its `CommandNotFound` names.
///
/// ## Notes
///
/// Each call gets its own directory so the two runs cannot share a
/// compose-cache entry keyed on the source path — the caching would otherwise
/// hide a genuine `$SHELL` dependence.
fn compose_missing_command(shell: &ShellFixture) -> String {
    let _guard = shell.guard();
    let dir = TempDir::new().unwrap();
    let (markdown, options) = document(&dir, &format!("# Test\n::shell {MISSING_EXECUTABLE}\n"));
    let err = markdown
        .compose_with(options)
        .expect_err("an unresolvable executable must fail composition");
    command_not_found(err)
}
