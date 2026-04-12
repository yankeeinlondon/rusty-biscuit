//! Safe program installation executor.
//!
//! This module provides safe command execution for installing programs via
//! package managers. It includes input validation, dry-run mode, and user
//! confirmation before executing any system-modifying commands.
//!
//! ## Security Features
//!
//! - **Input sanitization**: Rejects package names with shell metacharacters
//! - **Command allowlist**: Only executes known package manager commands
//! - **Dry-run mode**: Shows commands without executing
//! - **Confirmation prompts**: Requires explicit user confirmation
//! - **Timeout handling**: Commands timeout after 30 seconds by default
//!
//! ## Examples
//!
//! ```ignore
//! use sniff::programs::installer::{InstallOptions, execute_install};
//! use sniff::programs::types::InstallationMethod;
//!
//! let method = InstallationMethod::Brew("ripgrep");
//! let opts = InstallOptions::dry_run();
//!
//! // This will print the command without executing
//! execute_install(&method, &opts)?;
//! ```

use std::path::PathBuf;
use std::process::{Command, Output};

use strum::IntoEnumIterator;

use crate::error::SniffInstallationError;
use crate::os::{OsType, detect_os_type};
use crate::programs::enums::{LanguagePackageManager, OsPackageManager};
use crate::programs::host_capability::HostCapabilities;
use crate::programs::schema::ProgramMetadata;
use crate::programs::types::InstallationMethod;

/// The astral.sh installer URL for the current platform.
///
/// Unix platforms use the POSIX `sh` install script. Native Windows uses
/// the PowerShell install script. WSL is detected as Linux and falls
/// through to the Unix branch.
pub(crate) fn astral_installer_url() -> &'static str {
    match detect_os_type() {
        OsType::Windows => "https://astral.sh/uv/install.ps1",
        _ => "https://astral.sh/uv/install.sh",
    }
}

/// Returns the POSIX shell one-liner that bootstraps uv on Unix-like hosts.
fn unix_uv_bootstrap_command() -> String {
    format!("curl -LsSf '{}' | sh", "https://astral.sh/uv/install.sh")
}

/// Returns the PowerShell one-liner that bootstraps uv on native Windows.
fn windows_uv_bootstrap_command() -> String {
    "powershell -ExecutionPolicy ByPass -c \"irm https://astral.sh/uv/install.ps1 | iex\""
        .to_string()
}

/// Resolves the `uv` binary after (possible) bootstrap.
///
/// Resolution order:
/// 1. Bare `uv` on `PATH` (covers pre-existing installs and users who
///    added `~/.local/bin` to PATH).
/// 2. `~/.local/bin/uv` (Unix) or `%USERPROFILE%\.local\bin\uv.exe`
///    (Windows) — the astral installer's documented default location.
///
/// Returns `None` if neither path resolves.
fn resolve_uv_binary() -> Option<PathBuf> {
    if let Ok(path) = which::which("uv") {
        return Some(path);
    }
    let home = dirs::home_dir()?;
    let candidate = if cfg!(target_os = "windows") {
        home.join(".local").join("bin").join("uv.exe")
    } else {
        home.join(".local").join("bin").join("uv")
    };
    if candidate.exists() {
        Some(candidate)
    } else {
        None
    }
}

/// Renders the two-step (or one-step) command string for `UvWithInstall`.
///
/// When `uv` is already on PATH, only the `uv tool install` line is
/// rendered. When absent, the astral bootstrap line is rendered first.
/// The `uv` path used in the install line is whichever path `resolve_uv_binary`
/// would pick (or `~/.local/bin/uv` as the post-bootstrap default).
fn render_uv_with_install_command(pkg: &str, version: Option<&str>) -> String {
    let uv_present = which::which("uv").is_ok();
    let uv_path_str = if uv_present {
        resolve_uv_binary()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "uv".to_string())
    } else if cfg!(target_os = "windows") {
        "%USERPROFILE%\\.local\\bin\\uv.exe".to_string()
    } else {
        "~/.local/bin/uv".to_string()
    };

    let target = match version {
        Some(v) => format!("'{}@{}'", pkg, v),
        None => format!("'{}'", pkg),
    };
    let install_line = format!("{} tool install {}", uv_path_str, target);

    if uv_present {
        install_line
    } else {
        let bootstrap = if cfg!(target_os = "windows") {
            windows_uv_bootstrap_command()
        } else {
            unix_uv_bootstrap_command()
        };
        format!("{}\n{}", bootstrap, install_line)
    }
}

/// Default timeout for installation commands (30 seconds).
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Characters that are not allowed in package names.
const SHELL_METACHARACTERS: &[char] = &[
    ';', '&', '|', '`', '$', '(', ')', '{', '}', '[', ']', '<', '>', '"', '\'', '\\', '\n', '\r',
    '\t', '*', '?', '!', '#', '~', '^',
];

/// Options for program installation.
#[derive(Debug, Clone)]
pub struct InstallOptions {
    /// Show command without executing.
    pub dry_run: bool,
    /// Skip user confirmation prompt.
    pub skip_confirm: bool,
    /// Timeout in seconds for the installation command.
    pub timeout_secs: u64,
    /// Whether the caller has explicitly approved executing a RemoteBash method.
    ///
    /// Defaults to `false`. The plan executor returns
    /// `SniffInstallationError::RemoteBashConsentRequired` if the selected
    /// option is `RemoteBash` and this flag is `false`.
    pub approve_remote_bash: bool,
}

impl Default for InstallOptions {
    fn default() -> Self {
        Self {
            dry_run: false,
            skip_confirm: false,
            timeout_secs: DEFAULT_TIMEOUT_SECS,
            approve_remote_bash: false,
        }
    }
}

impl InstallOptions {
    /// Creates options for a dry-run (no execution).
    pub fn dry_run() -> Self {
        Self {
            dry_run: true,
            ..Default::default()
        }
    }

    /// Creates options that skip confirmation (for automated use).
    ///
    /// ## Warning
    ///
    /// Use with caution - this will execute commands without user confirmation.
    pub fn auto_confirm() -> Self {
        Self {
            skip_confirm: true,
            ..Default::default()
        }
    }

    /// Sets the timeout for the installation command.
    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }

    /// Sets whether RemoteBash execution is pre-approved.
    pub fn with_approve_remote_bash(mut self, approve: bool) -> Self {
        self.approve_remote_bash = approve;
        self
    }
}

/// Result of an installation attempt.
#[derive(Debug)]
pub struct InstallResult {
    /// The command that was (or would be) executed.
    pub command: String,
    /// Whether the command was actually executed (false for dry-run).
    pub executed: bool,
    /// Exit code if executed (None for dry-run).
    pub exit_code: Option<i32>,
    /// stdout output if executed.
    pub stdout: String,
    /// stderr output if executed.
    pub stderr: String,
}

impl InstallResult {
    fn dry_run(command: String) -> Self {
        Self {
            command,
            executed: false,
            exit_code: None,
            stdout: String::new(),
            stderr: String::new(),
        }
    }

    fn from_output(command: String, output: Output) -> Self {
        Self {
            command,
            executed: true,
            exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        }
    }
}

/// Captured outcome of an install attempt, preserving stdout/stderr on both
/// success and non-zero-exit failures so the interview layer can render
/// structured output. See
/// `sniff/features/2026-04-12-better-interview-for-install/tech-design.md`.
#[derive(Debug, Clone)]
pub struct InstallCapturedResult {
    pub command: String,
    pub executed: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
}

/// Two-arm outcome from a captured install run.
///
/// `Completed` covers dry-run, success, and non-zero exit (including spawn
/// failures folded into `stderr`). `SetupError` is reserved for invalid
/// inputs where no command could meaningfully run.
#[derive(Debug)]
pub enum InstallCapturedOutcome {
    Completed(InstallCapturedResult),
    SetupError(SniffInstallationError),
}

pub(crate) fn method_available(method: &InstallationMethod, host: &HostCapabilities) -> bool {
    if method.is_remote_bash() {
        return host.has_bash;
    }

    // `UvWithInstall` is runnable when bash is available on Unix or the
    // host is native Windows (PowerShell always present). It does NOT
    // require `uv` to be on the host — the whole point is to bootstrap it.
    if matches!(method, InstallationMethod::UvWithInstall(_)) {
        return host.os_type == OsType::Windows || host.has_bash;
    }

    let binary = method.manager_binary();

    if method.is_os_package_manager() {
        OsPackageManager::iter()
            .any(|mgr| mgr.binary_name() == binary && host.os_pkg_mgrs.is_installed(mgr))
    } else {
        LanguagePackageManager::iter()
            .any(|mgr| mgr.binary_name() == binary && host.lang_pkg_mgrs.is_installed(mgr))
    }
}

#[allow(dead_code)]
pub(crate) fn select_best_method<'a>(
    methods: &'a [InstallationMethod],
    host: &HostCapabilities,
) -> Option<&'a InstallationMethod> {
    if let Some(method) = methods
        .iter()
        .find(|method| method.is_os_package_manager() && method_available(method, host))
    {
        return Some(method);
    }

    methods
        .iter()
        .find(|method| !method.is_os_package_manager() && method_available(method, host))
}

/// Validates that a package name is safe for shell execution.
///
/// ## Errors
///
/// Returns an error if the package name contains shell metacharacters.
fn validate_package_name(pkg: &str) -> Result<(), SniffInstallationError> {
    if pkg.is_empty() {
        return Err(SniffInstallationError::InstallationError {
            pkg: pkg.to_string(),
            cmd: "".to_string(),
        });
    }

    for c in SHELL_METACHARACTERS {
        if pkg.contains(*c) {
            return Err(SniffInstallationError::InstallationError {
                pkg: pkg.to_string(),
                cmd: format!("Package name contains invalid character: '{}'", c),
            });
        }
    }

    Ok(())
}

/// Validates a remote-bash URL and returns it unchanged on success.
///
/// Rejects URLs that are not `https://`, contain single quotes, backslashes,
/// or control characters. This keeps the URL safe to interpolate into a
/// single-quoted `sh -c` string without shell-escape risk.
fn validate_remote_bash_url(url: &str) -> Result<(), SniffInstallationError> {
    if !url.starts_with("https://") {
        return Err(SniffInstallationError::InstallationError {
            pkg: url.to_string(),
            cmd: "remote-bash URL must use https://".to_string(),
        });
    }
    if url.contains('\'') || url.contains('\\') || url.chars().any(|c| c.is_control()) {
        return Err(SniffInstallationError::InstallationError {
            pkg: url.to_string(),
            cmd: "remote-bash URL contains forbidden characters".to_string(),
        });
    }
    Ok(())
}

/// Builds the install command for a package manager method.
fn build_install_command(
    method: &InstallationMethod,
) -> Result<Vec<String>, SniffInstallationError> {
    if let InstallationMethod::RemoteBash(url) = method {
        validate_remote_bash_url(url)?;
        // Single-quote the URL. Single quotes in POSIX sh are strictly literal,
        // and we've already rejected URLs containing `'`, so the URL cannot
        // break out of the quoted argument.
        let shell_cmd = format!("curl -sSfL '{url}' | bash");
        return Ok(vec!["sh".into(), "-c".into(), shell_cmd]);
    }

    let pkg = method.package_name();
    validate_package_name(pkg)?;

    let cmd = match method {
        // OS Package Managers
        InstallationMethod::Brew(pkg) => vec!["brew".into(), "install".into(), (*pkg).into()],
        InstallationMethod::Apt(pkg) => {
            vec![
                "sudo".into(),
                "apt".into(),
                "install".into(),
                "-y".into(),
                (*pkg).into(),
            ]
        }
        InstallationMethod::Nala(pkg) => {
            vec![
                "sudo".into(),
                "nala".into(),
                "install".into(),
                "-y".into(),
                (*pkg).into(),
            ]
        }
        InstallationMethod::Dnf(pkg) => {
            vec![
                "sudo".into(),
                "dnf".into(),
                "install".into(),
                "-y".into(),
                (*pkg).into(),
            ]
        }
        InstallationMethod::Pacman(pkg) => {
            vec![
                "sudo".into(),
                "pacman".into(),
                "-S".into(),
                "--noconfirm".into(),
                (*pkg).into(),
            ]
        }
        InstallationMethod::Winget(pkg) => {
            vec![
                "winget".into(),
                "install".into(),
                "--accept-package-agreements".into(),
                (*pkg).into(),
            ]
        }
        InstallationMethod::Chocolatey(pkg) => {
            vec!["choco".into(), "install".into(), "-y".into(), (*pkg).into()]
        }
        InstallationMethod::Scoop(pkg) => vec!["scoop".into(), "install".into(), (*pkg).into()],
        InstallationMethod::Nix(pkg) => vec!["nix-env".into(), "-iA".into(), (*pkg).into()],

        // Language Package Managers
        InstallationMethod::Cargo(pkg) => vec!["cargo".into(), "install".into(), (*pkg).into()],
        InstallationMethod::Npm(pkg) => {
            vec!["npm".into(), "install".into(), "-g".into(), (*pkg).into()]
        }
        InstallationMethod::Pnpm(pkg) => {
            vec!["pnpm".into(), "add".into(), "-g".into(), (*pkg).into()]
        }
        InstallationMethod::Yarn(pkg) => {
            vec!["yarn".into(), "global".into(), "add".into(), (*pkg).into()]
        }
        InstallationMethod::Bun(pkg) => {
            vec!["bun".into(), "add".into(), "-g".into(), (*pkg).into()]
        }
        InstallationMethod::Pip(pkg) => vec!["pip".into(), "install".into(), (*pkg).into()],
        InstallationMethod::Uv(pkg) => {
            vec!["uv".into(), "tool".into(), "install".into(), (*pkg).into()]
        }
        InstallationMethod::Poetry(pkg) => {
            // Poetry doesn't have global install; use pip instead
            vec!["pip".into(), "install".into(), (*pkg).into()]
        }
        InstallationMethod::GoModules(pkg) => vec!["go".into(), "install".into(), (*pkg).into()],
        InstallationMethod::Composer(pkg) => {
            vec![
                "composer".into(),
                "global".into(),
                "require".into(),
                (*pkg).into(),
            ]
        }
        InstallationMethod::SwiftPm(_) => {
            // Swift PM doesn't have global package install
            return Err(SniffInstallationError::NotInstallableOnOs {
                pkg: pkg.to_string(),
                os: "any".to_string(),
            });
        }
        InstallationMethod::LuaRocks(pkg) => {
            vec!["luarocks".into(), "install".into(), (*pkg).into()]
        }
        InstallationMethod::VcPkg(pkg) => vec!["vcpkg".into(), "install".into(), (*pkg).into()],
        InstallationMethod::Conan(pkg) => vec!["conan".into(), "install".into(), (*pkg).into()],
        InstallationMethod::Nuget(pkg) => vec![
            "dotnet".into(),
            "tool".into(),
            "install".into(),
            "-g".into(),
            (*pkg).into(),
        ],
        InstallationMethod::Hex(pkg) => vec![
            "mix".into(),
            "archive.install".into(),
            "hex".into(),
            (*pkg).into(),
        ],
        InstallationMethod::Cpan(pkg) => vec!["cpan".into(), (*pkg).into()],
        InstallationMethod::Cpanm(pkg) => vec!["cpanm".into(), (*pkg).into()],

        // Remote Bash is handled before this match statement.
        InstallationMethod::RemoteBash(_) => unreachable!("handled above"),

        // UvWithInstall is handled by `execute_uv_with_install`; this function
        // is only reached if called directly (e.g. from `get_install_command`).
        InstallationMethod::UvWithInstall(_) => {
            unreachable!("UvWithInstall is handled via execute_uv_with_install")
        }
    };

    Ok(cmd)
}

/// Builds the versioned install command for a package manager method.
fn build_versioned_install_command(
    method: &InstallationMethod,
    version: &str,
) -> Result<Vec<String>, SniffInstallationError> {
    let pkg = method.package_name();
    validate_package_name(pkg)?;
    validate_package_name(version)?; // Also validate version string

    let cmd = match method {
        // OS Package Managers with version support
        InstallationMethod::Brew(pkg) => {
            vec![
                "brew".into(),
                "install".into(),
                format!("{}@{}", pkg, version),
            ]
        }
        InstallationMethod::Chocolatey(pkg) => {
            vec![
                "choco".into(),
                "install".into(),
                "-y".into(),
                (*pkg).into(),
                "--version".into(),
                version.into(),
            ]
        }
        InstallationMethod::Scoop(pkg) => {
            // Scoop doesn't support versioned install directly
            return Err(SniffInstallationError::InstallationError {
                pkg: pkg.to_string(),
                cmd: "Scoop does not support versioned installation".to_string(),
            });
        }

        // Language Package Managers with version support
        InstallationMethod::Cargo(pkg) => {
            vec![
                "cargo".into(),
                "install".into(),
                (*pkg).into(),
                "--version".into(),
                version.into(),
            ]
        }
        InstallationMethod::Npm(pkg) => {
            vec![
                "npm".into(),
                "install".into(),
                "-g".into(),
                format!("{}@{}", pkg, version),
            ]
        }
        InstallationMethod::Pnpm(pkg) => {
            vec![
                "pnpm".into(),
                "add".into(),
                "-g".into(),
                format!("{}@{}", pkg, version),
            ]
        }
        InstallationMethod::Yarn(pkg) => {
            vec![
                "yarn".into(),
                "global".into(),
                "add".into(),
                format!("{}@{}", pkg, version),
            ]
        }
        InstallationMethod::Bun(pkg) => {
            vec![
                "bun".into(),
                "add".into(),
                "-g".into(),
                format!("{}@{}", pkg, version),
            ]
        }
        InstallationMethod::Pip(pkg) => {
            vec![
                "pip".into(),
                "install".into(),
                format!("{}=={}", pkg, version),
            ]
        }
        InstallationMethod::Uv(pkg) => {
            vec![
                "uv".into(),
                "tool".into(),
                "install".into(),
                format!("{}@{}", pkg, version),
            ]
        }
        InstallationMethod::GoModules(pkg) => {
            // Go modules use @version syntax
            let versioned = if pkg.contains('@') {
                pkg.to_string()
            } else {
                format!("{}@{}", pkg.trim_end_matches("@latest"), version)
            };
            vec!["go".into(), "install".into(), versioned]
        }

        // OS package managers that don't support versioned install well
        InstallationMethod::Apt(_)
        | InstallationMethod::Nala(_)
        | InstallationMethod::Dnf(_)
        | InstallationMethod::Pacman(_)
        | InstallationMethod::Nix(_)
        | InstallationMethod::Winget(_) => {
            return Err(SniffInstallationError::InstallationError {
                pkg: pkg.to_string(),
                cmd: format!(
                    "{} does not support versioned installation",
                    method.manager_name()
                ),
            });
        }

        // Others that don't support versioning
        InstallationMethod::Poetry(_)
        | InstallationMethod::SwiftPm(_)
        | InstallationMethod::Composer(_)
        | InstallationMethod::LuaRocks(_)
        | InstallationMethod::VcPkg(_)
        | InstallationMethod::Conan(_)
        | InstallationMethod::Nuget(_)
        | InstallationMethod::Hex(_)
        | InstallationMethod::Cpan(_)
        | InstallationMethod::Cpanm(_) => {
            return Err(SniffInstallationError::InstallationError {
                pkg: pkg.to_string(),
                cmd: format!(
                    "{} does not support versioned installation",
                    method.manager_name()
                ),
            });
        }

        // Remote Bash never supports versioned install
        InstallationMethod::RemoteBash(url) => {
            return Err(SniffInstallationError::InstallationError {
                pkg: url.to_string(),
                cmd: "Remote bash installation does not support versioning".to_string(),
            });
        }

        // UvWithInstall is handled by `execute_uv_with_install`; this function
        // is only reached if called directly (e.g. from `get_versioned_install_command`).
        InstallationMethod::UvWithInstall(_) => {
            unreachable!("UvWithInstall is handled via execute_uv_with_install")
        }
    };

    Ok(cmd)
}

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
    if let InstallationMethod::UvWithInstall(pkg) = method {
        return execute_uv_with_install(pkg, None, opts);
    }

    let cmd_parts = build_install_command(method)?;
    let cmd_str = cmd_parts.join(" ");

    if opts.dry_run {
        return Ok(InstallResult::dry_run(cmd_str));
    }

    // Execute the command
    let program = &cmd_parts[0];
    let args = &cmd_parts[1..];

    let output = Command::new(program).args(args).output().map_err(|e| {
        SniffInstallationError::PackageManagerFailed {
            pkg: method.package_name().to_string(),
            manager: method.manager_name().to_string(),
            msg: e.to_string(),
        }
    })?;

    let result = InstallResult::from_output(cmd_str, output);

    if result.exit_code != Some(0) {
        return Err(SniffInstallationError::PackageManagerFailed {
            pkg: method.package_name().to_string(),
            manager: method.manager_name().to_string(),
            msg: result.stderr.clone(),
        });
    }

    Ok(result)
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
    if let InstallationMethod::UvWithInstall(pkg) = method {
        return execute_uv_with_install(pkg, Some(version), opts);
    }

    let cmd_parts = build_versioned_install_command(method, version)?;
    let cmd_str = cmd_parts.join(" ");

    if opts.dry_run {
        return Ok(InstallResult::dry_run(cmd_str));
    }

    let program = &cmd_parts[0];
    let args = &cmd_parts[1..];

    let output = Command::new(program).args(args).output().map_err(|e| {
        SniffInstallationError::PackageManagerFailed {
            pkg: method.package_name().to_string(),
            manager: method.manager_name().to_string(),
            msg: e.to_string(),
        }
    })?;

    let result = InstallResult::from_output(cmd_str, output);

    if result.exit_code != Some(0) {
        return Err(SniffInstallationError::PackageManagerFailed {
            pkg: method.package_name().to_string(),
            manager: method.manager_name().to_string(),
            msg: result.stderr.clone(),
        });
    }

    Ok(result)
}

/// Executes an `UvWithInstall` method: conditionally bootstraps uv via
/// astral.sh, then runs `uv tool install <pkg>` (optionally `@version`).
///
/// The decision to run the bootstrap step is made at *call time* via a
/// fresh `which uv` probe — not from cached host capabilities — so a user
/// who installed uv between plan-build and execute does not trigger a
/// redundant bootstrap.
fn execute_uv_with_install(
    pkg: &str,
    version: Option<&str>,
    opts: &InstallOptions,
) -> Result<InstallResult, SniffInstallationError> {
    validate_package_name(pkg)?;
    if let Some(v) = version {
        validate_package_name(v)?;
    }

    let command_str = render_uv_with_install_command(pkg, version);

    if opts.dry_run {
        return Ok(InstallResult::dry_run(command_str));
    }

    // Step 1: bootstrap uv if absent.
    let mut combined_stdout = String::new();
    let mut combined_stderr = String::new();

    if which::which("uv").is_err() {
        let bootstrap_output =
            run_uv_bootstrap().map_err(|e| SniffInstallationError::PackageManagerFailed {
                pkg: pkg.to_string(),
                manager: "uv".to_string(),
                msg: e.to_string(),
            })?;
        combined_stdout.push_str(&String::from_utf8_lossy(&bootstrap_output.stdout));
        combined_stderr.push_str(&String::from_utf8_lossy(&bootstrap_output.stderr));
        if !bootstrap_output.status.success() {
            return Err(SniffInstallationError::PackageManagerFailed {
                pkg: pkg.to_string(),
                manager: "uv".to_string(),
                msg: format!("astral.sh uv bootstrap failed: {}", combined_stderr),
            });
        }
    }

    // Step 2: resolve the uv binary and run `uv tool install`.
    let uv_path = resolve_uv_binary().ok_or_else(|| SniffInstallationError::InstallationError {
        pkg: pkg.to_string(),
        cmd: "uv could not be located on PATH or at ~/.local/bin/uv after bootstrap".to_string(),
    })?;

    let target = match version {
        Some(v) => format!("{}@{}", pkg, v),
        None => pkg.to_string(),
    };

    let output = Command::new(&uv_path)
        .arg("tool")
        .arg("install")
        .arg(&target)
        .output()
        .map_err(|e| SniffInstallationError::PackageManagerFailed {
            pkg: pkg.to_string(),
            manager: "uv".to_string(),
            msg: e.to_string(),
        })?;

    combined_stdout.push_str(&String::from_utf8_lossy(&output.stdout));
    combined_stderr.push_str(&String::from_utf8_lossy(&output.stderr));

    if !output.status.success() {
        return Err(SniffInstallationError::PackageManagerFailed {
            pkg: pkg.to_string(),
            manager: "uv".to_string(),
            msg: combined_stderr,
        });
    }

    Ok(InstallResult {
        command: command_str,
        executed: true,
        exit_code: output.status.code(),
        stdout: combined_stdout,
        stderr: combined_stderr,
    })
}

/// Runs the astral.sh uv bootstrap script for the current platform.
fn run_uv_bootstrap() -> std::io::Result<Output> {
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
/// Mirrors the logic of [`execute_uv_with_install`] but always returns
/// [`InstallCapturedOutcome`] — spawn/bootstrap failures are folded into
/// `stderr` rather than propagated as errors.
fn execute_uv_with_install_captured(
    pkg: &str,
    version: Option<&str>,
    opts: &InstallOptions,
) -> InstallCapturedOutcome {
    if let Err(e) = validate_package_name(pkg) {
        return InstallCapturedOutcome::SetupError(e);
    }
    if let Some(v) = version {
        if let Err(e) = validate_package_name(v) {
            return InstallCapturedOutcome::SetupError(e);
        }
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

/// Returns the command that would be executed for installing a package.
///
/// This is useful for displaying to users before confirmation.
pub fn get_install_command(method: &InstallationMethod) -> Result<String, SniffInstallationError> {
    if let InstallationMethod::UvWithInstall(pkg) = method {
        return Ok(render_uv_with_install_command(pkg, None));
    }
    let cmd_parts = build_install_command(method)?;
    Ok(cmd_parts.join(" "))
}

/// Returns the command that would be executed for versioned installation.
pub fn get_versioned_install_command(
    method: &InstallationMethod,
    version: &str,
) -> Result<String, SniffInstallationError> {
    if let InstallationMethod::UvWithInstall(pkg) = method {
        return Ok(render_uv_with_install_command(pkg, Some(version)));
    }
    let cmd_parts = build_versioned_install_command(method, version)?;
    Ok(cmd_parts.join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_host() -> HostCapabilities {
        HostCapabilities::default()
    }

    fn host_with_cargo() -> HostCapabilities {
        HostCapabilities {
            lang_pkg_mgrs: serde_json::from_str(r#"{"cargo": true}"#).unwrap(),
            ..HostCapabilities::default()
        }
    }

    fn host_with_brew_and_cargo() -> HostCapabilities {
        HostCapabilities {
            os_pkg_mgrs: serde_json::from_str(r#"{"brew": true}"#).unwrap(),
            lang_pkg_mgrs: serde_json::from_str(r#"{"cargo": true}"#).unwrap(),
            ..HostCapabilities::default()
        }
    }

    fn host_with_bash() -> HostCapabilities {
        HostCapabilities {
            has_bash: true,
            ..HostCapabilities::default()
        }
    }

    #[test]
    fn test_validate_package_name_valid() {
        assert!(validate_package_name("ripgrep").is_ok());
        assert!(validate_package_name("git-delta").is_ok());
        assert!(validate_package_name("fd-find").is_ok());
        assert!(validate_package_name("@angular/cli").is_ok());
    }

    #[test]
    fn test_validate_package_name_invalid() {
        assert!(validate_package_name("pkg;rm -rf /").is_err());
        assert!(validate_package_name("pkg && bad").is_err());
        assert!(validate_package_name("pkg`bad`").is_err());
        assert!(validate_package_name("$(bad)").is_err());
        assert!(validate_package_name("").is_err());
    }

    #[test]
    fn test_build_install_command_brew() {
        let method = InstallationMethod::Brew("ripgrep");
        let cmd = build_install_command(&method).unwrap();
        assert_eq!(cmd, vec!["brew", "install", "ripgrep"]);
    }

    #[test]
    fn test_build_install_command_cargo() {
        let method = InstallationMethod::Cargo("bat");
        let cmd = build_install_command(&method).unwrap();
        assert_eq!(cmd, vec!["cargo", "install", "bat"]);
    }

    #[test]
    fn test_build_install_command_npm() {
        let method = InstallationMethod::Npm("typescript");
        let cmd = build_install_command(&method).unwrap();
        assert_eq!(cmd, vec!["npm", "install", "-g", "typescript"]);
    }

    #[test]
    fn test_build_install_command_remote_bash_returns_curl_bash_pipeline() {
        let method = InstallationMethod::RemoteBash("https://example.com/install.sh");
        let cmd = build_install_command(&method).unwrap();
        assert_eq!(
            cmd,
            vec![
                "sh".to_string(),
                "-c".to_string(),
                "curl -sSfL 'https://example.com/install.sh' | bash".to_string(),
            ]
        );
    }

    #[test]
    fn test_build_install_command_remote_bash_rejects_non_https() {
        let method = InstallationMethod::RemoteBash("http://example.com/install.sh");
        assert!(build_install_command(&method).is_err());
    }

    #[test]
    fn test_build_install_command_remote_bash_rejects_single_quote_in_url() {
        let method = InstallationMethod::RemoteBash("https://example.com/install'.sh");
        assert!(build_install_command(&method).is_err());
    }

    #[test]
    fn test_build_install_command_remote_bash_rejects_backslash_in_url() {
        let method = InstallationMethod::RemoteBash("https://example.com/install\\.sh");
        assert!(build_install_command(&method).is_err());
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
    fn test_build_versioned_install_command_cargo() {
        let method = InstallationMethod::Cargo("bat");
        let cmd = build_versioned_install_command(&method, "0.24.0").unwrap();
        assert_eq!(cmd, vec!["cargo", "install", "bat", "--version", "0.24.0"]);
    }

    #[test]
    fn test_build_versioned_install_command_npm() {
        let method = InstallationMethod::Npm("typescript");
        let cmd = build_versioned_install_command(&method, "5.0.0").unwrap();
        assert_eq!(cmd, vec!["npm", "install", "-g", "typescript@5.0.0"]);
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
    fn test_get_install_command() {
        let method = InstallationMethod::Brew("ripgrep");
        let cmd = get_install_command(&method).unwrap();
        assert_eq!(cmd, "brew install ripgrep");
    }

    #[test]
    fn test_install_options_defaults() {
        let opts = InstallOptions::default();
        assert!(!opts.dry_run);
        assert!(!opts.skip_confirm);
        assert_eq!(opts.timeout_secs, DEFAULT_TIMEOUT_SECS);
    }

    #[test]
    fn test_method_available_remote_bash_requires_bash() {
        let method = InstallationMethod::RemoteBash("https://example.com/install.sh");
        assert!(!method_available(&method, &empty_host()));
        assert!(method_available(&method, &host_with_bash()));
    }

    #[test]
    fn test_select_best_method_prefers_os_package_manager() {
        let methods = [
            InstallationMethod::Cargo("bat"),
            InstallationMethod::Brew("bat"),
        ];
        let host = host_with_brew_and_cargo();

        let selected =
            select_best_method(&methods, &host).expect("Expected a method to be selected");
        assert!(matches!(selected, InstallationMethod::Brew(_)));
    }

    #[test]
    fn test_select_best_method_falls_back_to_language_manager() {
        let methods = [InstallationMethod::Cargo("bat")];
        let host = host_with_cargo();

        let selected =
            select_best_method(&methods, &host).expect("Expected a method to be selected");
        assert!(matches!(selected, InstallationMethod::Cargo(_)));
    }

    #[test]
    fn test_select_best_method_returns_none_when_unavailable() {
        let methods = [InstallationMethod::RemoteBash(
            "https://example.com/install.sh",
        )];
        // No bash, no managers → nothing runnable.
        assert!(select_best_method(&methods, &empty_host()).is_none());
    }

    #[test]
    fn test_select_best_method_picks_remote_bash_when_bash_available() {
        let methods = [InstallationMethod::RemoteBash(
            "https://example.com/install.sh",
        )];
        let host = host_with_bash();
        let selected = select_best_method(&methods, &host).expect("expected bash-backed choice");
        assert!(matches!(selected, InstallationMethod::RemoteBash(_)));
    }

    #[test]
    fn test_build_install_command_apt() {
        let method = InstallationMethod::Apt("ripgrep");
        let cmd = build_install_command(&method).unwrap();
        assert_eq!(cmd, vec!["sudo", "apt", "install", "-y", "ripgrep"]);
    }

    #[test]
    fn test_build_install_command_rejects_shell_metacharacters() {
        let method = InstallationMethod::Brew("ripgrep; rm -rf /");
        let result = build_install_command(&method);
        assert!(result.is_err());
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
    fn test_get_install_command_returns_string() {
        let method = InstallationMethod::Brew("ripgrep");
        let cmd = get_install_command(&method).unwrap();
        assert!(cmd.contains("brew install ripgrep"));
    }

    #[test]
    fn test_install_options_default_does_not_approve_remote_bash() {
        let opts = InstallOptions::default();
        assert!(!opts.approve_remote_bash);
    }

    #[test]
    fn test_install_options_with_approve_remote_bash_sets_flag() {
        let opts = InstallOptions::default().with_approve_remote_bash(true);
        assert!(opts.approve_remote_bash);
    }

    // =======================================================================
    // UvWithInstall execution tests
    // =======================================================================

    #[test]
    fn uv_with_install_dry_run_renders_install_line() {
        let method = InstallationMethod::UvWithInstall("aider-chat");
        let result = execute_install(&method, &InstallOptions::dry_run()).unwrap();
        assert!(!result.executed);
        // Must always include the uv tool install line.
        assert!(result.command.contains("tool install 'aider-chat'"));
    }

    #[test]
    fn uv_with_install_without_consent_returns_consent_error_via_plan() {
        use crate::programs::install_plan::{InstallPlan, InstallPlanOption, InstallPlanReason};
        let plan = InstallPlan {
            program: "aider".into(),
            website: "https://aider.chat",
            successful: true,
            options: vec![InstallPlanOption {
                kind: InstallationMethod::UvWithInstall("aider-chat"),
                requires_sudo: false,
                choose: true,
                reason_type: InstallPlanReason::Selected,
                reason: "chosen".into(),
            }],
        };
        let err = plan.execute(&InstallOptions::default()).unwrap_err();
        match err {
            SniffInstallationError::RemoteBashConsentRequired { url, .. } => {
                // url must be the astral installer URL, NOT the package name.
                assert!(
                    url.starts_with("https://astral.sh/uv/install."),
                    "expected astral installer URL, got {}",
                    url
                );
            }
            other => panic!("expected RemoteBashConsentRequired, got {:?}", other),
        }
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
    fn uv_with_install_get_install_command_contains_install_line() {
        let method = InstallationMethod::UvWithInstall("conan");
        let cmd = get_install_command(&method).unwrap();
        assert!(cmd.contains("tool install 'conan'"));
    }

    #[test]
    fn install_captured_outcome_completed_has_command_and_streams() {
        let ok = InstallCapturedResult {
            command: "brew install rg".into(),
            executed: true,
            exit_code: Some(0),
            stdout: "ok\n".into(),
            stderr: String::new(),
            success: true,
        };
        let outcome = InstallCapturedOutcome::Completed(ok);
        match outcome {
            InstallCapturedOutcome::Completed(r) => {
                assert_eq!(r.command, "brew install rg");
                assert!(r.success);
            }
            _ => panic!("expected Completed"),
        }
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
}
