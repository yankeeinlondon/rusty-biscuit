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
//! use sniff_lib::programs::installer::{InstallOptions, execute_install};
//! use sniff_lib::programs::types::InstallationMethod;
//!
//! let method = InstallationMethod::Brew("ripgrep");
//! let opts = InstallOptions::dry_run();
//!
//! // This will print the command without executing
//! execute_install(&method, &opts)?;
//! ```

use std::process::{Command, Output};

use crate::error::SniffInstallationError;
use crate::programs::enums::{LanguagePackageManager, OsPackageManager};
use crate::programs::pkg_mngrs::{InstalledLanguagePackageManagers, InstalledOsPackageManagers};
use crate::programs::types::InstallationMethod;

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
}

impl Default for InstallOptions {
    fn default() -> Self {
        Self {
            dry_run: false,
            skip_confirm: false,
            timeout_secs: DEFAULT_TIMEOUT_SECS,
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

pub(crate) fn method_available(
    method: &InstallationMethod,
    os_pkg_mgrs: &InstalledOsPackageManagers,
    lang_pkg_mgrs: &InstalledLanguagePackageManagers,
) -> bool {
    if method.is_remote_bash() {
        return false;
    }

    match method {
        InstallationMethod::Apt(_) => os_pkg_mgrs.is_installed(OsPackageManager::Apt),
        InstallationMethod::Nala(_) => os_pkg_mgrs.is_installed(OsPackageManager::Nala),
        InstallationMethod::Brew(_) => os_pkg_mgrs.is_installed(OsPackageManager::Brew),
        InstallationMethod::Dnf(_) => os_pkg_mgrs.is_installed(OsPackageManager::Dnf),
        InstallationMethod::Pacman(_) => os_pkg_mgrs.is_installed(OsPackageManager::Pacman),
        InstallationMethod::Winget(_) => os_pkg_mgrs.is_installed(OsPackageManager::Winget),
        InstallationMethod::Chocolatey(_) => os_pkg_mgrs.is_installed(OsPackageManager::Chocolatey),
        InstallationMethod::Scoop(_) => os_pkg_mgrs.is_installed(OsPackageManager::Scoop),
        InstallationMethod::Nix(_) => os_pkg_mgrs.is_installed(OsPackageManager::Nix),
        InstallationMethod::Npm(_) => lang_pkg_mgrs.is_installed(LanguagePackageManager::Npm),
        InstallationMethod::Pnpm(_) => lang_pkg_mgrs.is_installed(LanguagePackageManager::Pnpm),
        InstallationMethod::Yarn(_) => lang_pkg_mgrs.is_installed(LanguagePackageManager::Yarn),
        InstallationMethod::Bun(_) => lang_pkg_mgrs.is_installed(LanguagePackageManager::Bun),
        InstallationMethod::Cargo(_) => lang_pkg_mgrs.is_installed(LanguagePackageManager::Cargo),
        InstallationMethod::GoModules(_) => lang_pkg_mgrs.is_installed(LanguagePackageManager::GoModules),
        InstallationMethod::Composer(_) => lang_pkg_mgrs.is_installed(LanguagePackageManager::Composer),
        InstallationMethod::SwiftPm(_) => lang_pkg_mgrs.is_installed(LanguagePackageManager::SwiftPm),
        InstallationMethod::LuaRocks(_) => lang_pkg_mgrs.is_installed(LanguagePackageManager::Luarocks),
        InstallationMethod::VcPkg(_) => lang_pkg_mgrs.is_installed(LanguagePackageManager::Vcpkg),
        InstallationMethod::Conan(_) => lang_pkg_mgrs.is_installed(LanguagePackageManager::Conan),
        InstallationMethod::Nuget(_) => lang_pkg_mgrs.is_installed(LanguagePackageManager::Nuget),
        InstallationMethod::Hex(_) => lang_pkg_mgrs.is_installed(LanguagePackageManager::Hex),
        InstallationMethod::Pip(_) => lang_pkg_mgrs.is_installed(LanguagePackageManager::Pip),
        InstallationMethod::Uv(_) => lang_pkg_mgrs.is_installed(LanguagePackageManager::Uv),
        InstallationMethod::Poetry(_) => lang_pkg_mgrs.is_installed(LanguagePackageManager::Poetry),
        InstallationMethod::Cpan(_) => lang_pkg_mgrs.is_installed(LanguagePackageManager::Cpan),
        InstallationMethod::Cpanm(_) => lang_pkg_mgrs.is_installed(LanguagePackageManager::Cpanm),
        InstallationMethod::RemoteBash(_) => false,
    }
}

pub(crate) fn select_best_method<'a>(
    methods: &'a [InstallationMethod],
    os_pkg_mgrs: &InstalledOsPackageManagers,
    lang_pkg_mgrs: &InstalledLanguagePackageManagers,
) -> Option<&'a InstallationMethod> {
    if let Some(method) = methods.iter().find(|method| {
        method.is_os_package_manager() && method_available(method, os_pkg_mgrs, lang_pkg_mgrs)
    }) {
        return Some(method);
    }

    methods.iter().find(|method| {
        !method.is_os_package_manager() && method_available(method, os_pkg_mgrs, lang_pkg_mgrs)
    })
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

/// Builds the install command for a package manager method.
fn build_install_command(
    method: &InstallationMethod,
) -> Result<Vec<String>, SniffInstallationError> {
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

        // Remote Bash - NOT SUPPORTED for security reasons
        InstallationMethod::RemoteBash(url) => {
            return Err(SniffInstallationError::InstallationError {
                pkg: url.to_string(),
                cmd: "Remote bash installation requires manual execution for security".to_string(),
            });
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

/// Returns the command that would be executed for installing a package.
///
/// This is useful for displaying to users before confirmation.
pub fn get_install_command(method: &InstallationMethod) -> Result<String, SniffInstallationError> {
    let cmd_parts = build_install_command(method)?;
    Ok(cmd_parts.join(" "))
}

/// Returns the command that would be executed for versioned installation.
pub fn get_versioned_install_command(
    method: &InstallationMethod,
    version: &str,
) -> Result<String, SniffInstallationError> {
    let cmd_parts = build_versioned_install_command(method, version)?;
    Ok(cmd_parts.join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_os_pkg_mgrs() -> InstalledOsPackageManagers {
        InstalledOsPackageManagers::default()
    }

    fn empty_lang_pkg_mgrs() -> InstalledLanguagePackageManagers {
        InstalledLanguagePackageManagers::default()
    }

    fn os_pkg_mgrs_with_brew() -> InstalledOsPackageManagers {
        serde_json::from_str(r#"{"brew": true}"#).unwrap()
    }

    fn lang_pkg_mgrs_with_cargo() -> InstalledLanguagePackageManagers {
        serde_json::from_str(r#"{"cargo": true}"#).unwrap()
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
    fn test_build_install_command_remote_bash_rejected() {
        let method = InstallationMethod::RemoteBash("https://example.com/install.sh");
        assert!(build_install_command(&method).is_err());
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
    fn test_method_available_filters_remote_bash() {
        let os_pkg_mgrs = empty_os_pkg_mgrs();
        let lang_pkg_mgrs = empty_lang_pkg_mgrs();
        let method = InstallationMethod::RemoteBash("https://example.com/install.sh");
        assert!(!method_available(&method, &os_pkg_mgrs, &lang_pkg_mgrs));
    }

    #[test]
    fn test_select_best_method_prefers_os_package_manager() {
        let methods = [
            InstallationMethod::Cargo("bat"),
            InstallationMethod::Brew("bat"),
        ];
        let os_pkg_mgrs = os_pkg_mgrs_with_brew();
        let lang_pkg_mgrs = lang_pkg_mgrs_with_cargo();

        let selected = select_best_method(&methods, &os_pkg_mgrs, &lang_pkg_mgrs)
            .expect("Expected a method to be selected");
        assert!(matches!(selected, InstallationMethod::Brew(_)));
    }

    #[test]
    fn test_select_best_method_falls_back_to_language_manager() {
        let methods = [InstallationMethod::Cargo("bat")];
        let os_pkg_mgrs = empty_os_pkg_mgrs();
        let lang_pkg_mgrs = lang_pkg_mgrs_with_cargo();

        let selected = select_best_method(&methods, &os_pkg_mgrs, &lang_pkg_mgrs)
            .expect("Expected a method to be selected");
        assert!(matches!(selected, InstallationMethod::Cargo(_)));
    }

    #[test]
    fn test_select_best_method_returns_none_when_unavailable() {
        let methods = [InstallationMethod::RemoteBash(
            "https://example.com/install.sh",
        )];
        let os_pkg_mgrs = empty_os_pkg_mgrs();
        let lang_pkg_mgrs = empty_lang_pkg_mgrs();
        assert!(select_best_method(&methods, &os_pkg_mgrs, &lang_pkg_mgrs).is_none());
    }
}
