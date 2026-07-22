//! Command building, validation, host eligibility, and prose builders.
//!
//! This module owns the pure logic that turns an `InstallationMethod` into
//! either an executable command vector or human-facing prose. It deliberately
//! avoids spawning subprocesses — see `execute` for that.

use std::path::PathBuf;

use strum::IntoEnumIterator;

use crate::error::SniffInstallationError;
use crate::os::{OsType, detect_os_type};
use crate::programs::contract::InstallationMethod;
use crate::programs::enums::{LanguagePackageManager, OsPackageManager};
use crate::programs::host_capability::HostCapabilities;
use crate::programs::schema::ProgramMetadata;

/// Characters that are not allowed in package names.
const SHELL_METACHARACTERS: &[char] = &[
    ';', '&', '|', '`', '$', '(', ')', '{', '}', '[', ']', '<', '>', '"', '\'', '\\', '\n', '\r',
    '\t', '*', '?', '!', '#', '~', '^',
];

/// The astral.sh installer URL for the current platform.
///
/// Unix platforms use the POSIX `sh` install script. Native Windows uses
/// the PowerShell install script. WSL is detected as Linux and falls
/// through to the Unix branch.
pub fn astral_installer_url() -> &'static str {
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
pub(super) fn resolve_uv_binary() -> Option<PathBuf> {
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
pub(super) fn render_uv_with_install_command(pkg: &str, version: Option<&str>) -> String {
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

/// Builds the user-facing announcement prose for an installation about to begin.
pub fn build_install_announcement(
    program: &str,
    website: &str,
    method: &InstallationMethod,
    command: &str,
) -> String {
    let program_link = format!(
        r#"<b><blue><a href="{website}">{program}</a></blue></b>"#,
        website = website,
        program = program,
    );
    let command_span = format!("<dim><green>{command}</green></dim>", command = command);

    match method {
        InstallationMethod::RemoteBash(url) => format!(
            "The {program_link} will be installed using the remote installer script at \
             <a href=\"{url}\">{url}</a> using the command: {command_span}",
            url = url,
        ),
        InstallationMethod::UvWithInstall(_) => {
            let astral_url = astral_installer_url();
            format!(
                "The {program_link} will be installed by bootstrapping <b>uv</b> from \
                 <a href=\"{astral_url}\">{astral_url}</a> if needed, then running: {command_span}",
            )
        }
        _ => {
            let manager = method.manager_name();
            format!(
                "The {program_link} will be installed through the <b>{manager}</b> package \
                 manager using the command: {command_span}",
            )
        }
    }
}

/// Builds the user-facing success status prose after a successful installation.
pub fn build_install_success_status(program: &str, website: &str) -> String {
    format!(
        r#"<b><blue><a href="{website}">{program}</a></blue></b> has been installed successfully"#,
        website = website,
        program = program,
    )
}

/// Builds the user-facing failure status prose after a failed installation.
pub fn build_install_failure_status(program: &str, website: &str) -> String {
    format!(
        r#"failed to install <b><blue><a href="{website}">{program}</a></blue></b>."#,
        website = website,
        program = program,
    )
}

/// Builds the user-facing warning prose for an installation that was killed at
/// its deadline.
///
/// The "may still be running" caveat is load-bearing, not hedging: on Unix
/// sniff signals the installer's process tree, but a descendant that forked and
/// detached with `setsid()` between samples survives the kill.
pub fn build_install_timeout_warning(program: &str, timeout_secs: u64) -> String {
    format!(
        "<yellow>Warning:</yellow> installing <b>{program}</b> did not finish within \
         <b>{timeout_secs}s</b> and was terminated. Termination is best-effort: an installer \
         process that detached from sniff may still be running and modifying this host. Check \
         for a partial <b>{program}</b> install before retrying.",
    )
}

/// Builds the prose for a retry-with-alternative-manager choice in the interview.
pub fn build_retry_choice_prose(method: &InstallationMethod) -> String {
    let manager = method.manager_name();
    format!("Try installing using <b>{manager}</b> instead")
}

/// Builds the prose for the quit/manual option in a retry dialog.
pub fn build_retry_quit_prose() -> String {
    "Quit (<i>and try manually if desired</i>)".to_string()
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
pub(super) fn validate_package_name(pkg: &str) -> Result<(), SniffInstallationError> {
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
pub(super) fn build_install_command(
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
        InstallationMethod::Maven(pkg) => {
            vec!["mvn".into(), "dependency:get".into(), (*pkg).into()]
        }
        InstallationMethod::Gem(pkg) => vec!["gem".into(), "install".into(), (*pkg).into()],
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
pub(super) fn build_versioned_install_command(
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
        | InstallationMethod::Composer(_)
        | InstallationMethod::SwiftPm(_)
        | InstallationMethod::LuaRocks(_)
        | InstallationMethod::VcPkg(_)
        | InstallationMethod::Conan(_)
        | InstallationMethod::Nuget(_)
        | InstallationMethod::Hex(_)
        | InstallationMethod::Maven(_)
        | InstallationMethod::Gem(_)
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
    fn test_get_install_command() {
        let method = InstallationMethod::Brew("ripgrep");
        let cmd = get_install_command(&method).unwrap();
        assert_eq!(cmd, "brew install ripgrep");
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
    fn test_get_install_command_returns_string() {
        let method = InstallationMethod::Brew("ripgrep");
        let cmd = get_install_command(&method).unwrap();
        assert!(cmd.contains("brew install ripgrep"));
    }

    #[test]
    fn uv_with_install_get_install_command_contains_install_line() {
        let method = InstallationMethod::UvWithInstall("conan");
        let cmd = get_install_command(&method).unwrap();
        assert!(cmd.contains("tool install 'conan'"));
    }

    #[test]
    fn announcement_package_manager_template() {
        let out = build_install_announcement(
            "Ripgrep",
            "https://github.com/BurntSushi/ripgrep",
            &InstallationMethod::Brew("ripgrep"),
            "brew install ripgrep",
        );
        assert!(out.contains("Ripgrep"));
        assert!(out.contains("https://github.com/BurntSushi/ripgrep"));
        assert!(out.contains("brew"));
        assert!(out.contains("brew install ripgrep"));
        assert!(out.contains("package manager"));
    }

    #[test]
    fn announcement_remote_bash_template() {
        let url = "https://sh.rustup.rs";
        let out = build_install_announcement(
            "Rustup",
            "https://rustup.rs",
            &InstallationMethod::RemoteBash(url),
            "curl -sSfL 'https://sh.rustup.rs' | bash",
        );
        assert!(out.contains("remote installer script"));
        assert!(out.contains(url));
        assert!(out.contains("curl -sSfL"));
    }

    #[test]
    fn announcement_uv_with_install_template() {
        let out = build_install_announcement(
            "Aider",
            "https://aider.chat",
            &InstallationMethod::UvWithInstall("aider-chat"),
            "uv tool install 'aider-chat'",
        );
        assert!(out.contains("bootstrapping"));
        assert!(out.contains("uv"));
        assert!(out.contains("astral.sh"));
    }

    #[test]
    fn success_status_mentions_installed_successfully() {
        let out = build_install_success_status("Ripgrep", "https://github.com/BurntSushi/ripgrep");
        assert!(out.contains("Ripgrep"));
        assert!(out.contains("installed successfully"));
    }

    #[test]
    fn failure_status_mentions_failed_to_install() {
        let out = build_install_failure_status("Ripgrep", "https://github.com/BurntSushi/ripgrep");
        assert!(out.to_lowercase().contains("failed to install"));
        assert!(out.contains("Ripgrep"));
    }

    #[test]
    fn retry_choice_prose_names_alternative() {
        let out = build_retry_choice_prose(&InstallationMethod::Cargo("bat"));
        assert!(out.contains("cargo"));
        assert!(out.contains("Try installing"));
    }

    #[test]
    fn retry_quit_prose_mentions_quit() {
        let out = build_retry_quit_prose();
        assert!(out.to_lowercase().contains("quit"));
        assert!(out.contains("manually"));
    }
}
