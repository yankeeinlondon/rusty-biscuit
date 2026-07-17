//! Subprocess execution for installation methods.
//!
//! Owns spawning logic: `execute_install`, `execute_versioned_install`,
//! their captured variants, and the special-cased `UvWithInstall`
//! bootstrap. Pure command building lives in `command`.

use std::process::{Command, Output};

use crate::error::SniffInstallationError;
use crate::performance::{self, counters};
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
    match execute_install_captured(method, opts) {
        InstallCapturedOutcome::SetupError(e) => Err(e),
        InstallCapturedOutcome::Completed(r) if r.success => Ok(InstallResult {
            command: r.command,
            executed: r.executed,
            exit_code: r.exit_code,
            stdout: r.stdout,
            stderr: r.stderr,
        }),
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
/// Returns an error if versioned installation is not supported for this method.
pub fn execute_versioned_install(
    method: &InstallationMethod,
    version: &str,
    opts: &InstallOptions,
) -> Result<InstallResult, SniffInstallationError> {
    match execute_versioned_install_captured(method, version, opts) {
        InstallCapturedOutcome::SetupError(e) => Err(e),
        InstallCapturedOutcome::Completed(r) if r.success => Ok(InstallResult {
            command: r.command,
            executed: r.executed,
            exit_code: r.exit_code,
            stdout: r.stdout,
            stderr: r.stderr,
        }),
        InstallCapturedOutcome::Completed(r) => Err(SniffInstallationError::PackageManagerFailed {
            pkg: method.package_name().to_string(),
            manager: method.manager_name().to_string(),
            msg: r.stderr,
        }),
    }
}

/// Runs the astral.sh uv bootstrap script for the current platform.
fn run_uv_bootstrap() -> std::io::Result<Output> {
    performance::increment_counter(counters::PROC_SPAWNS, 1);
    if cfg!(target_os = "windows") {
        Command::new("powershell")
            .arg("-ExecutionPolicy")
            .arg("ByPass")
            .arg("-c")
            .arg("irm https://astral.sh/uv/install.ps1 | iex")
            .output()
    } else {
        Command::new("sh")
            .arg("-c")
            .arg("curl -LsSf 'https://astral.sh/uv/install.sh' | sh")
            .output()
    }
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
pub fn execute_install_captured(
    method: &InstallationMethod,
    opts: &InstallOptions,
) -> InstallCapturedOutcome {
    if let InstallationMethod::UvWithInstall(pkg) = method {
        return execute_uv_with_install_captured(pkg, None, opts);
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
        });
    }

    let program = &cmd_parts[0];
    let args = &cmd_parts[1..];

    performance::increment_counter(counters::PROC_SPAWNS, 1);
    match Command::new(program).args(args).output() {
        Ok(output) => InstallCapturedOutcome::Completed(InstallCapturedResult {
            command,
            executed: true,
            exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            success: output.status.success(),
        }),
        Err(e) => InstallCapturedOutcome::Completed(InstallCapturedResult {
            command,
            executed: false,
            exit_code: None,
            stdout: String::new(),
            stderr: e.to_string(),
            success: false,
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
pub fn execute_versioned_install_captured(
    method: &InstallationMethod,
    version: &str,
    opts: &InstallOptions,
) -> InstallCapturedOutcome {
    if let InstallationMethod::UvWithInstall(pkg) = method {
        return execute_uv_with_install_captured(pkg, Some(version), opts);
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
        });
    }

    let program = &cmd_parts[0];
    let args = &cmd_parts[1..];

    performance::increment_counter(counters::PROC_SPAWNS, 1);
    match Command::new(program).args(args).output() {
        Ok(output) => InstallCapturedOutcome::Completed(InstallCapturedResult {
            command,
            executed: true,
            exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            success: output.status.success(),
        }),
        Err(e) => InstallCapturedOutcome::Completed(InstallCapturedResult {
            command,
            executed: false,
            exit_code: None,
            stdout: String::new(),
            stderr: e.to_string(),
            success: false,
        }),
    }
}

/// Executes a `UvWithInstall` method and captures all output.
///
/// Mirrors the logic of the captured install path but for `UvWithInstall`,
/// conditionally bootstrapping uv via astral.sh then running `uv tool install`.
/// Spawn/bootstrap failures are folded into `stderr` rather than propagated as errors.
fn execute_uv_with_install_captured(
    pkg: &str,
    version: Option<&str>,
    opts: &InstallOptions,
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
        });
    }

    let mut combined_stdout = String::new();
    let mut combined_stderr = String::new();

    // Step 1: bootstrap uv if absent.
    if which::which("uv").is_err() {
        let bootstrap_output = match run_uv_bootstrap() {
            Ok(out) => out,
            Err(e) => {
                return InstallCapturedOutcome::Completed(InstallCapturedResult {
                    command: command_str,
                    executed: false,
                    exit_code: None,
                    stdout: String::new(),
                    stderr: e.to_string(),
                    success: false,
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
            });
        }
    }

    // Step 2: resolve the uv binary.
    let uv_path = match resolve_uv_binary() {
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
            });
        }
    };

    // Step 3: run `uv tool install <target>`.
    let target = match version {
        Some(v) => format!("{}@{}", pkg, v),
        None => pkg.to_string(),
    };

    performance::increment_counter(counters::PROC_SPAWNS, 1);
    match Command::new(&uv_path)
        .arg("tool")
        .arg("install")
        .arg(&target)
        .output()
    {
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
            })
        }
        Err(e) => {
            combined_stderr.push_str(&e.to_string());
            InstallCapturedOutcome::Completed(InstallCapturedResult {
                command: command_str,
                executed: false,
                exit_code: None,
                stdout: combined_stdout,
                stderr: combined_stderr,
                success: false,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
