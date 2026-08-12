//! Subprocess execution for installation methods.
//!
//! Owns spawning logic: `execute_install`, `execute_versioned_install`,
//! their captured variants, and the special-cased `UvWithInstall`
//! bootstrap. Pure command building lives in `command`.

use std::process::Command;
use std::time::Duration;

use crate::error::SniffInstallationError;
use crate::process::{CapturedOutput, ProcessError, run_command_with_timeout};
use crate::programs::contract::InstallationMethod;

use super::command::{
    build_install_command, build_versioned_install_command, render_uv_with_install_command,
    resolve_uv_binary, validate_package_name,
};
use super::options::{
    InstallCapturedOutcome, InstallCapturedResult, InstallOptions, InstallResult,
};

/// Executes an installation command for the given method.
///
/// ## Arguments
///
/// * `method` - The installation method to use
/// * `opts` - Options controlling execution (dry-run, confirmation, timeout)
///
/// ## Returns
///
/// An `InstallResult` containing the command and execution details.
///
/// ## Errors
///
/// Returns an error if:
/// - The package name contains invalid characters
/// - The installation method is not supported (e.g., RemoteBash)
/// - The command execution fails
/// - The command was killed at its deadline
///   ([`SniffInstallationError::InstallationTimedOut`], never conflated with
///   an ordinary non-zero exit)
///
/// ## Examples
///
/// ```ignore
/// let method = InstallationMethod::Brew("ripgrep");
/// let result = execute_install(&method, &InstallOptions::dry_run())?;
/// println!("Would run: {}", result.command);
/// ```
pub fn execute_install(
    method: &InstallationMethod,
    opts: &InstallOptions,
) -> Result<InstallResult, SniffInstallationError> {
    install_result_from_outcome(method, opts, execute_install_captured(method, opts))
}

/// Projects a captured outcome onto the legacy `Result` contract.
///
/// A timed-out attempt becomes [`SniffInstallationError::InstallationTimedOut`]
/// rather than `PackageManagerFailed`, because the package manager never
/// reported a verdict and a detached descendant may still be running.
fn install_result_from_outcome(
    method: &InstallationMethod,
    opts: &InstallOptions,
    outcome: InstallCapturedOutcome,
) -> Result<InstallResult, SniffInstallationError> {
    match outcome {
        InstallCapturedOutcome::SetupError(e) => Err(e),
        InstallCapturedOutcome::Completed(r) if r.success => Ok(InstallResult {
            command: r.command,
            executed: r.executed,
            exit_code: r.exit_code,
            stdout: r.stdout,
            stderr: r.stderr,
        }),
        InstallCapturedOutcome::Completed(r) if r.timed_out => {
            Err(SniffInstallationError::InstallationTimedOut {
                pkg: method.package_name().to_string(),
                manager: method.manager_name().to_string(),
                timeout_secs: opts.timeout_secs,
            })
        }
        InstallCapturedOutcome::Completed(r) => Err(SniffInstallationError::PackageManagerFailed {
            pkg: method.package_name().to_string(),
            manager: method.manager_name().to_string(),
            msg: r.stderr,
        }),
    }
}

/// Executes a versioned installation command.
///
/// ## Arguments
///
/// * `method` - The installation method to use
/// * `version` - The version to install
/// * `opts` - Options controlling execution
///
/// ## Errors
///
/// Returns an error if versioned installation is not supported for this method,
/// or [`SniffInstallationError::InstallationTimedOut`] if the command was
/// killed at its deadline.
pub fn execute_versioned_install(
    method: &InstallationMethod,
    version: &str,
    opts: &InstallOptions,
) -> Result<InstallResult, SniffInstallationError> {
    install_result_from_outcome(
        method,
        opts,
        execute_versioned_install_captured(method, version, opts),
    )
}

type CommandRunner<'a> =
    dyn Fn(&mut Command, Duration) -> Result<CapturedOutput, ProcessError> + Sync + 'a;

/// Runs the astral.sh uv bootstrap script for the current platform.
fn run_uv_bootstrap(
    timeout: Duration,
    runner: &CommandRunner<'_>,
) -> Result<CapturedOutput, ProcessError> {
    let mut command = if cfg!(target_os = "windows") {
        let mut command = Command::new("powershell");
        command
            .arg("-ExecutionPolicy")
            .arg("ByPass")
            .arg("-c")
            .arg("irm https://astral.sh/uv/install.ps1 | iex");
        command
    } else {
        let mut command = Command::new("sh");
        command
            .arg("-c")
            .arg("curl -LsSf 'https://astral.sh/uv/install.sh' | sh");
        command
    };
    runner(&mut command, timeout)
}

/// Executes an installation command and captures stdout/stderr without
/// ever returning an error — spawn failures are folded into `stderr` and
/// returned as `Completed { success: false }`.
///
/// ## Returns
///
/// - `InstallCapturedOutcome::Completed` for dry-run, success, non-zero exit,
///   and spawn failures.
/// - `InstallCapturedOutcome::SetupError` only when the input is invalid (e.g.
///   package name contains shell metacharacters) so no command could be built.
///
/// ## Notes
///
/// A `Completed` result with `timed_out` set was killed at its deadline. The
/// installer's process tree was terminated on a best-effort basis: on Unix a
/// descendant that forked and detached with `setsid()` between sniff's samples
/// survives and may still be modifying the host. Third-party package managers
/// and downloaded installer scripts are exactly the kind of code that can do
/// this, so `timed_out` is not a promise that the install stopped. See the
/// `process` module documentation for the per-platform guarantee.
pub fn execute_install_captured(
    method: &InstallationMethod,
    opts: &InstallOptions,
) -> InstallCapturedOutcome {
    execute_install_captured_with_runner(method, opts, &run_command_with_timeout)
}

fn execute_install_captured_with_runner(
    method: &InstallationMethod,
    opts: &InstallOptions,
    runner: &CommandRunner<'_>,
) -> InstallCapturedOutcome {
    if let InstallationMethod::UvWithInstall(pkg) = method {
        return execute_uv_with_install_captured_with_runner(
            pkg,
            None,
            opts,
            runner,
            &|| which::which("uv").is_ok(),
            &resolve_uv_binary,
        );
    }

    let cmd_parts = match build_install_command(method) {
        Ok(parts) => parts,
        Err(e) => return InstallCapturedOutcome::SetupError(e),
    };
    let command = cmd_parts.join(" ");

    if opts.dry_run {
        return InstallCapturedOutcome::Completed(InstallCapturedResult {
            command,
            executed: false,
            exit_code: None,
            stdout: String::new(),
            stderr: String::new(),
            success: true,
            timed_out: false,
        });
    }

    let program = &cmd_parts[0];
    let args = &cmd_parts[1..];

    let mut child = Command::new(program);
    child.args(args);
    match runner(&mut child, Duration::from_secs(opts.timeout_secs)) {
        Ok(output) => InstallCapturedOutcome::Completed(InstallCapturedResult {
            command,
            executed: true,
            exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            success: output.status.success(),
            timed_out: false,
        }),
        Err(e) => InstallCapturedOutcome::Completed(InstallCapturedResult {
            command,
            executed: matches!(e, ProcessError::Timeout),
            exit_code: None,
            stdout: String::new(),
            stderr: e.to_string(),
            success: false,
            timed_out: matches!(e, ProcessError::Timeout),
        }),
    }
}

/// Executes a versioned installation command and captures stdout/stderr without
/// ever returning an error — spawn failures are folded into `stderr` and
/// returned as `Completed { success: false }`.
///
/// ## Returns
///
/// - `InstallCapturedOutcome::Completed` for dry-run, success, non-zero exit,
///   and spawn failures.
/// - `InstallCapturedOutcome::SetupError` only when the input is invalid (e.g.
///   package name contains shell metacharacters) so no command could be built.
///
/// ## Notes
///
/// A `Completed` result with `timed_out` set was killed at its deadline. The
/// installer's process tree was terminated on a best-effort basis: on Unix a
/// descendant that forked and detached with `setsid()` between sniff's samples
/// survives and may still be modifying the host. Third-party package managers
/// and downloaded installer scripts are exactly the kind of code that can do
/// this, so `timed_out` is not a promise that the install stopped. See the
/// `process` module documentation for the per-platform guarantee.
pub fn execute_versioned_install_captured(
    method: &InstallationMethod,
    version: &str,
    opts: &InstallOptions,
) -> InstallCapturedOutcome {
    execute_versioned_install_captured_with_runner(
        method,
        version,
        opts,
        &run_command_with_timeout,
    )
}

fn execute_versioned_install_captured_with_runner(
    method: &InstallationMethod,
    version: &str,
    opts: &InstallOptions,
    runner: &CommandRunner<'_>,
) -> InstallCapturedOutcome {
    if let InstallationMethod::UvWithInstall(pkg) = method {
        return execute_uv_with_install_captured_with_runner(
            pkg,
            Some(version),
            opts,
            runner,
            &|| which::which("uv").is_ok(),
            &resolve_uv_binary,
        );
    }

    let cmd_parts = match build_versioned_install_command(method, version) {
        Ok(parts) => parts,
        Err(e) => return InstallCapturedOutcome::SetupError(e),
    };
    let command = cmd_parts.join(" ");

    if opts.dry_run {
        return InstallCapturedOutcome::Completed(InstallCapturedResult {
            command,
            executed: false,
            exit_code: None,
            stdout: String::new(),
            stderr: String::new(),
            success: true,
            timed_out: false,
        });
    }

    let program = &cmd_parts[0];
    let args = &cmd_parts[1..];

    let mut child = Command::new(program);
    child.args(args);
    match runner(&mut child, Duration::from_secs(opts.timeout_secs)) {
        Ok(output) => InstallCapturedOutcome::Completed(InstallCapturedResult {
            command,
            executed: true,
            exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            success: output.status.success(),
            timed_out: false,
        }),
        Err(e) => InstallCapturedOutcome::Completed(InstallCapturedResult {
            command,
            executed: matches!(e, ProcessError::Timeout),
            exit_code: None,
            stdout: String::new(),
            stderr: e.to_string(),
            success: false,
            timed_out: matches!(e, ProcessError::Timeout),
        }),
    }
}

/// Executes a `UvWithInstall` method and captures all output.
///
/// Conditionally bootstraps uv via astral.sh, then runs `uv tool install`.
/// Spawn/bootstrap failures are folded into `stderr` rather than propagated as errors.
fn execute_uv_with_install_captured_with_runner(
    pkg: &str,
    version: Option<&str>,
    opts: &InstallOptions,
    runner: &CommandRunner<'_>,
    uv_on_path: &dyn Fn() -> bool,
    resolve_uv: &dyn Fn() -> Option<std::path::PathBuf>,
) -> InstallCapturedOutcome {
    if let Err(e) = validate_package_name(pkg) {
        return InstallCapturedOutcome::SetupError(e);
    }
    if let Some(v) = version
        && let Err(e) = validate_package_name(v)
    {
        return InstallCapturedOutcome::SetupError(e);
    }

    let command_str = render_uv_with_install_command(pkg, version);

    if opts.dry_run {
        return InstallCapturedOutcome::Completed(InstallCapturedResult {
            command: command_str,
            executed: false,
            exit_code: None,
            stdout: String::new(),
            stderr: String::new(),
            success: true,
            timed_out: false,
        });
    }

    let mut combined_stdout = String::new();
    let mut combined_stderr = String::new();

    // Step 1: bootstrap uv if absent.
    let timeout = Duration::from_secs(opts.timeout_secs);
    if !uv_on_path() {
        let bootstrap_output = match run_uv_bootstrap(timeout, runner) {
            Ok(out) => out,
            Err(e) => {
                return InstallCapturedOutcome::Completed(InstallCapturedResult {
                    command: command_str,
                    executed: matches!(e, ProcessError::Timeout),
                    exit_code: None,
                    stdout: String::new(),
                    stderr: e.to_string(),
                    success: false,
                    timed_out: matches!(e, ProcessError::Timeout),
                });
            }
        };

        combined_stdout.push_str(&String::from_utf8_lossy(&bootstrap_output.stdout));
        combined_stderr.push_str(&String::from_utf8_lossy(&bootstrap_output.stderr));

        if !bootstrap_output.status.success() {
            return InstallCapturedOutcome::Completed(InstallCapturedResult {
                command: command_str,
                executed: true,
                exit_code: bootstrap_output.status.code(),
                stdout: combined_stdout,
                stderr: combined_stderr,
                success: false,
                timed_out: false,
            });
        }
    }

    // Step 2: resolve the uv binary.
    let uv_path = match resolve_uv() {
        Some(p) => p,
        None => {
            return InstallCapturedOutcome::Completed(InstallCapturedResult {
                command: command_str,
                executed: false,
                exit_code: None,
                stdout: combined_stdout,
                stderr: "uv could not be located on PATH or at ~/.local/bin/uv after bootstrap"
                    .into(),
                success: false,
                timed_out: false,
            });
        }
    };

    // Step 3: run `uv tool install <target>`.
    let target = match version {
        Some(v) => format!("{}@{}", pkg, v),
        None => pkg.to_string(),
    };

    let mut child = Command::new(&uv_path);
    child.arg("tool").arg("install").arg(&target);
    match runner(&mut child, timeout) {
        Ok(output) => {
            combined_stdout.push_str(&String::from_utf8_lossy(&output.stdout));
            combined_stderr.push_str(&String::from_utf8_lossy(&output.stderr));
            InstallCapturedOutcome::Completed(InstallCapturedResult {
                command: command_str,
                executed: true,
                exit_code: output.status.code(),
                stdout: combined_stdout,
                stderr: combined_stderr,
                success: output.status.success(),
                timed_out: false,
            })
        }
        Err(e) => {
            combined_stderr.push_str(&e.to_string());
            InstallCapturedOutcome::Completed(InstallCapturedResult {
                command: command_str,
                executed: matches!(e, ProcessError::Timeout),
                exit_code: None,
                stdout: combined_stdout,
                stderr: combined_stderr,
                success: false,
                timed_out: matches!(e, ProcessError::Timeout),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const INSTALL_TIMEOUT_CHILD: &str =
        "programs::install::execute::tests::install_timeout_child";

    fn fixture_args() -> Vec<std::ffi::OsString> {
        [INSTALL_TIMEOUT_CHILD, "--exact", "--ignored", "--nocapture"]
            .into_iter()
            .map(Into::into)
            .collect()
    }

    fn run_timeout_fixture(timeout: Duration) -> Result<CapturedOutput, ProcessError> {
        let executable = std::env::current_exe().expect("current test executable should resolve");
        let mut fixture = Command::new(executable);
        fixture.args(fixture_args());
        run_command_with_timeout(&mut fixture, timeout)
    }

    fn assert_timeout(outcome: InstallCapturedOutcome) {
        let InstallCapturedOutcome::Completed(result) = outcome else {
            panic!("timed-out execution should complete with a captured failure");
        };
        assert!(result.executed);
        assert!(!result.success);
        assert!(
            result.timed_out,
            "a deadline kill must be distinguishable from an ordinary failure"
        );
        assert_eq!(result.exit_code, None);
        assert_eq!(result.stderr, "timed out");
    }

    #[test]
    #[ignore = "subprocess fixture invoked by installation timeout tests"]
    fn install_timeout_child() {
        std::thread::sleep(Duration::from_secs(30));
    }

    #[test]
    fn ordinary_install_honors_requested_timeout() {
        let method = InstallationMethod::Brew("ripgrep");
        let runner = |command: &mut Command, timeout| {
            assert_eq!(command.get_program(), "brew");
            assert_eq!(
                command.get_args().collect::<Vec<_>>(),
                ["install", "ripgrep"].map(std::ffi::OsStr::new)
            );
            run_timeout_fixture(timeout)
        };

        let outcome = execute_install_captured_with_runner(
            &method,
            &InstallOptions::default().with_timeout(0),
            &runner,
        );

        assert_timeout(outcome);
    }

    #[test]
    fn versioned_install_honors_requested_timeout() {
        let method = InstallationMethod::Cargo("bat");
        let runner = |command: &mut Command, timeout| {
            assert_eq!(command.get_program(), "cargo");
            assert_eq!(
                command.get_args().collect::<Vec<_>>(),
                ["install", "bat", "--version", "0.24.0"].map(std::ffi::OsStr::new)
            );
            run_timeout_fixture(timeout)
        };

        let outcome = execute_versioned_install_captured_with_runner(
            &method,
            "0.24.0",
            &InstallOptions::default().with_timeout(0),
            &runner,
        );

        assert_timeout(outcome);
    }

    #[test]
    fn uv_install_honors_requested_timeout() {
        let runner = |command: &mut Command, timeout| {
            assert_eq!(command.get_program(), "uv-fixture");
            assert_eq!(
                command.get_args().collect::<Vec<_>>(),
                ["tool", "install", "aider-chat"].map(std::ffi::OsStr::new)
            );
            run_timeout_fixture(timeout)
        };

        let outcome = execute_uv_with_install_captured_with_runner(
            "aider-chat",
            None,
            &InstallOptions::default().with_timeout(0),
            &runner,
            &|| true,
            &|| Some("uv-fixture".into()),
        );

        assert_timeout(outcome);
    }

    #[test]
    fn uv_bootstrap_honors_requested_timeout() {
        let runner = |command: &mut Command, timeout| {
            if cfg!(target_os = "windows") {
                assert_eq!(command.get_program(), "powershell");
            } else {
                assert_eq!(command.get_program(), "sh");
            }
            run_timeout_fixture(timeout)
        };

        let outcome = execute_uv_with_install_captured_with_runner(
            "aider-chat",
            None,
            &InstallOptions::default().with_timeout(0),
            &runner,
            &|| false,
            &|| panic!("uv resolution must not run after bootstrap timeout"),
        );

        assert_timeout(outcome);
    }

    #[test]
    fn legacy_install_reports_timeout_as_distinct_error() {
        let method = InstallationMethod::Brew("ripgrep");
        let opts = InstallOptions::default().with_timeout(0);
        let runner = |_: &mut Command, timeout| run_timeout_fixture(timeout);

        let outcome = execute_install_captured_with_runner(&method, &opts, &runner);
        let err = install_result_from_outcome(&method, &opts, outcome)
            .expect_err("a killed installer must not report success");

        match err {
            SniffInstallationError::InstallationTimedOut {
                pkg,
                manager,
                timeout_secs,
            } => {
                assert_eq!(pkg, "ripgrep");
                assert_eq!(manager, "brew");
                assert_eq!(timeout_secs, 0);
            }
            other => panic!("timeout must not be conflated with a package-manager failure: {other}"),
        }
    }

    #[test]
    fn legacy_versioned_install_reports_timeout_as_distinct_error() {
        let method = InstallationMethod::Cargo("bat");
        let opts = InstallOptions::default().with_timeout(0);
        let runner = |_: &mut Command, timeout| run_timeout_fixture(timeout);

        let outcome =
            execute_versioned_install_captured_with_runner(&method, "0.24.0", &opts, &runner);
        let err = install_result_from_outcome(&method, &opts, outcome)
            .expect_err("a killed installer must not report success");

        assert!(
            matches!(err, SniffInstallationError::InstallationTimedOut { .. }),
            "expected InstallationTimedOut, got {err}"
        );
    }

    #[test]
    fn legacy_install_still_reports_ordinary_failure_as_package_manager_failed() {
        // Contrast case for the timeout arm: a non-timeout failure must keep
        // the pre-existing error so the new variant stays load-bearing.
        let method = InstallationMethod::Brew("ripgrep");
        let opts = InstallOptions::default();
        let runner = |_: &mut Command, _| {
            Err(ProcessError::Spawn(std::io::Error::other(
                "brew is not installed",
            )))
        };

        let outcome = execute_install_captured_with_runner(&method, &opts, &runner);
        let err = install_result_from_outcome(&method, &opts, outcome)
            .expect_err("a spawn failure must not report success");

        match err {
            SniffInstallationError::PackageManagerFailed { msg, .. } => {
                assert!(msg.contains("brew is not installed"), "msg: {msg}");
            }
            other => panic!("expected PackageManagerFailed, got {other}"),
        }
    }

    #[test]
    fn test_remote_bash_dry_run_returns_command_without_executing() {
        let method = InstallationMethod::RemoteBash("https://sh.rustup.rs");
        let result = execute_install(&method, &InstallOptions::dry_run()).unwrap();
        assert!(!result.executed);
        assert!(
            result
                .command
                .contains("curl -sSfL 'https://sh.rustup.rs' | bash")
        );
    }

    #[test]
    fn test_dry_run_returns_command_without_executing() {
        let method = InstallationMethod::Brew("ripgrep");
        let result = execute_install(&method, &InstallOptions::dry_run()).unwrap();
        assert!(!result.executed);
        assert_eq!(result.command, "brew install ripgrep");
        assert!(result.exit_code.is_none());
    }

    #[test]
    fn test_dry_run_does_not_execute() {
        let method = InstallationMethod::Brew("ripgrep");
        let opts = InstallOptions::dry_run();
        let result = execute_install(&method, &opts).unwrap();
        assert!(!result.executed);
        assert!(result.command.contains("brew"));
    }

    #[test]
    fn uv_with_install_dry_run_renders_install_line() {
        let method = InstallationMethod::UvWithInstall("aider-chat");
        let result = execute_install(&method, &InstallOptions::dry_run()).unwrap();
        assert!(!result.executed);
        // Must always include the uv tool install line.
        assert!(result.command.contains("tool install 'aider-chat'"));
    }

    #[test]
    fn uv_with_install_versioned_renders_at_version() {
        let method = InstallationMethod::UvWithInstall("aider-chat");
        let result =
            execute_versioned_install(&method, "0.50.0", &InstallOptions::dry_run()).unwrap();
        assert!(!result.executed);
        assert!(
            result.command.contains("tool install 'aider-chat@0.50.0'"),
            "rendered command: {}",
            result.command
        );
    }

    #[test]
    fn execute_install_captured_dry_run_returns_completed_with_command() {
        let method = InstallationMethod::Brew("ripgrep");
        let outcome = execute_install_captured(&method, &InstallOptions::dry_run());
        match outcome {
            InstallCapturedOutcome::Completed(r) => {
                assert!(!r.executed);
                assert!(r.success);
                assert_eq!(r.command, "brew install ripgrep");
                assert_eq!(r.exit_code, None);
            }
            other => panic!("dry run must be Completed, got {:?}", other),
        }
    }

    #[test]
    fn execute_install_captured_setup_error_on_invalid_package() {
        let method = InstallationMethod::Brew("bad;pkg");
        let outcome = execute_install_captured(&method, &InstallOptions::default());
        assert!(matches!(outcome, InstallCapturedOutcome::SetupError(_)));
    }

    #[test]
    fn execute_install_captured_uv_with_install_dry_run_includes_install_line() {
        let method = InstallationMethod::UvWithInstall("aider-chat");
        let outcome = execute_install_captured(&method, &InstallOptions::dry_run());
        match outcome {
            InstallCapturedOutcome::Completed(r) => {
                assert!(!r.executed);
                assert!(r.success);
                assert!(r.command.contains("tool install 'aider-chat'"));
            }
            other => panic!("expected Completed, got {:?}", other),
        }
    }

    #[test]
    fn execute_versioned_install_captured_dry_run() {
        let method = InstallationMethod::Cargo("bat");
        let outcome =
            execute_versioned_install_captured(&method, "0.24.0", &InstallOptions::dry_run());
        match outcome {
            InstallCapturedOutcome::Completed(r) => {
                assert!(!r.executed);
                assert!(r.success);
                assert!(r.command.contains("bat"));
                assert!(r.command.contains("0.24.0"));
            }
            other => panic!("expected Completed, got {:?}", other),
        }
    }

    #[test]
    fn execute_versioned_install_captured_uv_with_install_includes_versioned_target() {
        let method = InstallationMethod::UvWithInstall("aider-chat");
        let outcome =
            execute_versioned_install_captured(&method, "0.50.0", &InstallOptions::dry_run());
        match outcome {
            InstallCapturedOutcome::Completed(r) => {
                assert!(r.command.contains("tool install 'aider-chat@0.50.0'"));
            }
            other => panic!("expected Completed, got {:?}", other),
        }
    }

    #[test]
    fn legacy_execute_install_dry_run_still_returns_install_result() {
        // Ensures the refactored wrapper preserves existing behavior.
        let method = InstallationMethod::Brew("ripgrep");
        let result = execute_install(&method, &InstallOptions::dry_run()).unwrap();
        assert!(!result.executed);
        assert_eq!(result.command, "brew install ripgrep");
    }
}
