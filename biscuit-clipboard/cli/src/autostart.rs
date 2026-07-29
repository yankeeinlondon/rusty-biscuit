//! Autostart install/uninstall shim for the `clipper` background
//! service.
//!
//! This module renders a per-user manifest (launchd plist on macOS,
//! systemd user unit on Linux, `.cmd` shim in the Startup folder on
//! Windows) that re-launches `clipper` at login. Live activation
//! (`launchctl bootstrap`, `systemctl --user enable --now`) is left to
//! the user — this module only writes the manifest file. A `--dry-run`
//! mode prints the manifest body to stdout without touching disk.
//!
//! ## Notes
//!
//! - `CLIP_AUTOSTART_PREFIX` env var redirects the install root for
//!   tests, so the real `~/Library/LaunchAgents` etc. is never touched
//!   during `cargo test`.
//! - Idempotent: running `install` twice is a no-op (returns
//!   `Outcome::AlreadyInstalled`).
//! - Cross-platform: targets other than macOS / Linux / Windows return
//!   `Error::UnsupportedPlatform`.

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Environment variable that redirects the install root. Used by tests
/// to avoid writing into the real user home.
pub const CLIP_AUTOSTART_PREFIX_ENV: &str = "CLIP_AUTOSTART_PREFIX";

/// Identifier embedded in the macOS launchd plist.
#[cfg(target_os = "macos")]
pub const AUTOSTART_LABEL: &str = "com.biscuit.clipper";

/// Options controlling install/uninstall behaviour.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// When `true`, render the manifest and report the path that
    /// *would* be written, but do not touch disk.
    pub dry_run: bool,
    /// Optional override for the binary path embedded in the manifest.
    /// When `None`, the binary path is resolved via
    /// [`biscuit_clipboard::client::find_clipper_binary`].
    pub binary_path_override: Option<String>,
    /// Optional override for the install root. Falls back to the
    /// `CLIP_AUTOSTART_PREFIX` env var, then the platform default.
    pub prefix_override: Option<PathBuf>,
}

impl Options {
    /// Convenience constructor for `--dry-run` invocations.
    #[allow(dead_code)]
    pub fn dry_run() -> Self {
        Self {
            dry_run: true,
            ..Self::default()
        }
    }
}

/// Errors surfaced by the autostart module.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Current platform is not one of macOS, Linux, or Windows.
    #[allow(dead_code)]
    #[error("autostart is not supported on this platform")]
    UnsupportedPlatform,
    /// Could not resolve the user's home directory.
    #[error("could not determine user home directory")]
    NoHomeDir,
    /// Filesystem failure while writing the manifest.
    #[error("filesystem error: {0}")]
    Io(#[from] io::Error),
}

/// Outcome of an install/uninstall operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Manifest was just written to disk (install) / removed (uninstall).
    Wrote,
    /// Manifest already existed (install) / didn't exist (uninstall).
    AlreadyInstalled,
    /// Nothing was written; only rendering happened (`--dry-run`).
    DryRun,
}

impl fmt::Display for Outcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Outcome::Wrote => write!(f, "wrote"),
            Outcome::AlreadyInstalled => write!(f, "already installed"),
            Outcome::DryRun => write!(f, "dry-run"),
        }
    }
}

/// Result of an install/uninstall call.
#[derive(Debug, Clone)]
pub struct Report {
    /// Absolute path of the manifest file (written or that would be
    /// written).
    pub manifest_path: PathBuf,
    /// Body of the manifest. Always populated for `install`; for
    /// `uninstall` callers should ignore.
    pub manifest_body: String,
    /// What actually happened.
    pub outcome: Outcome,
    /// Human-readable next-step hint (e.g. `launchctl load …`).
    pub next_step: Option<String>,
}

/// Install the platform autostart manifest.
///
/// ## Returns
///
/// A [`Report`] describing the manifest path, body, and outcome.
///
/// ## Errors
///
/// Returns [`Error::UnsupportedPlatform`] on non-{macOS, Linux, Windows}
/// targets and [`Error::Io`] on filesystem failures.
pub fn install(opts: Options) -> Result<Report, Error> {
    let binary = resolve_binary(&opts);
    let prefix = resolve_prefix(opts.prefix_override.as_deref())?;
    let manifest = render_manifest(&binary, &prefix)?;

    if opts.dry_run {
        return Ok(Report {
            manifest_path: manifest.path,
            manifest_body: manifest.body,
            outcome: Outcome::DryRun,
            next_step: Some(manifest.next_step),
        });
    }

    if manifest.path.exists() {
        return Ok(Report {
            manifest_path: manifest.path,
            manifest_body: manifest.body,
            outcome: Outcome::AlreadyInstalled,
            next_step: Some(manifest.next_step),
        });
    }

    if let Some(parent) = manifest.path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&manifest.path, &manifest.body)?;

    Ok(Report {
        manifest_path: manifest.path,
        manifest_body: manifest.body,
        outcome: Outcome::Wrote,
        next_step: Some(manifest.next_step),
    })
}

/// Remove the platform autostart manifest.
///
/// ## Returns
///
/// A [`Report`]; [`Outcome::AlreadyInstalled`] is returned (re-using the
/// enum) when the manifest didn't exist — semantically "nothing to do".
///
/// ## Errors
///
/// Returns [`Error::UnsupportedPlatform`] on non-{macOS, Linux, Windows}
/// targets and [`Error::Io`] on filesystem failures.
pub fn uninstall(opts: Options) -> Result<Report, Error> {
    let binary = resolve_binary(&opts);
    let prefix = resolve_prefix(opts.prefix_override.as_deref())?;
    let manifest = render_manifest(&binary, &prefix)?;

    if opts.dry_run {
        return Ok(Report {
            manifest_path: manifest.path,
            manifest_body: manifest.body,
            outcome: Outcome::DryRun,
            next_step: Some(format!("would remove {}", manifest_path_display(&prefix))),
        });
    }

    if !manifest.path.exists() {
        return Ok(Report {
            manifest_path: manifest.path,
            manifest_body: String::new(),
            outcome: Outcome::AlreadyInstalled,
            next_step: None,
        });
    }

    fs::remove_file(&manifest.path)?;

    Ok(Report {
        manifest_path: manifest.path,
        manifest_body: String::new(),
        outcome: Outcome::Wrote,
        next_step: None,
    })
}

fn resolve_binary(opts: &Options) -> String {
    opts.binary_path_override
        .clone()
        .unwrap_or_else(biscuit_clipboard::client::find_clipper_binary)
}

fn resolve_prefix(override_path: Option<&Path>) -> Result<PathBuf, Error> {
    if let Some(p) = override_path {
        return Ok(p.to_path_buf());
    }
    if let Some(env) = std::env::var_os(CLIP_AUTOSTART_PREFIX_ENV) {
        return Ok(PathBuf::from(env));
    }
    default_prefix()
}

fn manifest_path_display(prefix: &Path) -> String {
    prefix.display().to_string()
}

struct RenderedManifest {
    path: PathBuf,
    body: String,
    next_step: String,
}

#[cfg(target_os = "macos")]
fn default_prefix() -> Result<PathBuf, Error> {
    let home = dirs::home_dir().ok_or(Error::NoHomeDir)?;
    Ok(home)
}

#[cfg(target_os = "macos")]
fn render_manifest(binary: &str, prefix: &Path) -> Result<RenderedManifest, Error> {
    let path = prefix
        .join("Library")
        .join("LaunchAgents")
        .join(format!("{AUTOSTART_LABEL}.plist"));
    let body = render_macos_plist(binary);
    let next_step = format!(
        "launchctl load {} (or log out/in to start at login)",
        path.display()
    );
    Ok(RenderedManifest {
        path,
        body,
        next_step,
    })
}

/// Render a macOS launchd LaunchAgent plist for the `clipper` binary.
#[cfg(target_os = "macos")]
pub(crate) fn render_macos_plist(binary: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
  <dict>
    <key>Label</key>
    <string>{AUTOSTART_LABEL}</string>
    <key>ProgramArguments</key>
    <array>
      <string>{binary}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>ProcessType</key>
    <string>Background</string>
  </dict>
</plist>
"#
    )
}

#[cfg(target_os = "linux")]
fn default_prefix() -> Result<PathBuf, Error> {
    let home = dirs::home_dir().ok_or(Error::NoHomeDir)?;
    Ok(home)
}

#[cfg(target_os = "linux")]
fn render_manifest(binary: &str, prefix: &Path) -> Result<RenderedManifest, Error> {
    let path = prefix
        .join(".config")
        .join("systemd")
        .join("user")
        .join("clipper.service");
    let body = render_linux_unit(binary);
    let next_step =
        "systemctl --user daemon-reload && systemctl --user enable --now clipper".to_string();
    Ok(RenderedManifest {
        path,
        body,
        next_step,
    })
}

/// Render a systemd user unit for the `clipper` binary.
#[cfg(target_os = "linux")]
pub(crate) fn render_linux_unit(binary: &str) -> String {
    format!(
        "[Unit]\n\
Description=Biscuit Clipboard background service\n\
After=default.target\n\
\n\
[Service]\n\
Type=simple\n\
ExecStart={binary}\n\
Restart=on-failure\n\
RestartSec=2\n\
\n\
[Install]\n\
WantedBy=default.target\n"
    )
}

#[cfg(target_os = "windows")]
fn default_prefix() -> Result<PathBuf, Error> {
    // %APPDATA% (Roaming) is the documented Startup-folder root.
    let appdata = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .or_else(|| dirs::config_dir())
        .ok_or(Error::NoHomeDir)?;
    Ok(appdata)
}

#[cfg(target_os = "windows")]
fn render_manifest(binary: &str, prefix: &Path) -> Result<RenderedManifest, Error> {
    let path = prefix
        .join("Microsoft")
        .join("Windows")
        .join("Start Menu")
        .join("Programs")
        .join("Startup")
        .join("clipper.cmd");
    let body = render_windows_cmd(binary);
    let next_step = format!(
        "{} will run on next login (or run it now from the Startup folder)",
        path.display()
    );
    Ok(RenderedManifest {
        path,
        body,
        next_step,
    })
}

/// Render a `.cmd` shim that the Windows Startup folder will execute on
/// login.
#[cfg(target_os = "windows")]
pub(crate) fn render_windows_cmd(binary: &str) -> String {
    format!("@echo off\r\nstart \"\" \"{binary}\"\r\n")
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
fn default_prefix() -> Result<PathBuf, Error> {
    Err(Error::UnsupportedPlatform)
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
fn render_manifest(_binary: &str, _prefix: &Path) -> Result<RenderedManifest, Error> {
    Err(Error::UnsupportedPlatform)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_prefix() -> tempfile::TempDir {
        tempfile::tempdir().expect("tempdir")
    }

    fn options_with(prefix: &Path, dry_run: bool) -> Options {
        Options {
            dry_run,
            binary_path_override: Some("/usr/local/bin/clipper".to_string()),
            prefix_override: Some(prefix.to_path_buf()),
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn macos_plist_contains_binary_path_and_label() {
        let body = render_macos_plist("/usr/local/bin/clipper");
        assert!(body.contains("<string>/usr/local/bin/clipper</string>"));
        assert!(body.contains(&format!("<string>{AUTOSTART_LABEL}</string>")));
        assert!(body.contains("<key>RunAtLoad</key>"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_unit_contains_exec_start() {
        let body = render_linux_unit("/usr/local/bin/clipper");
        assert!(body.contains("ExecStart=/usr/local/bin/clipper"));
        assert!(body.contains("[Install]"));
        assert!(body.contains("WantedBy=default.target"));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_cmd_contains_binary_path() {
        let body = render_windows_cmd("C:\\bin\\clipper.exe");
        assert!(body.contains("C:\\bin\\clipper.exe"));
        assert!(body.contains("@echo off"));
    }

    #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
    #[test]
    fn dry_run_does_not_touch_disk() {
        let prefix = temp_prefix();
        let report = install(options_with(prefix.path(), true)).expect("install dry-run");
        assert_eq!(report.outcome, Outcome::DryRun);
        assert!(!report.manifest_body.is_empty());
        assert!(
            !report.manifest_path.exists(),
            "dry-run must not write to disk; manifest_path = {}",
            report.manifest_path.display()
        );
    }

    #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
    #[test]
    fn install_then_uninstall_round_trip() {
        let prefix = temp_prefix();
        let install_report = install(options_with(prefix.path(), false)).expect("install");
        assert_eq!(install_report.outcome, Outcome::Wrote);
        assert!(install_report.manifest_path.exists());

        // Manifest body actually contains the binary path we passed.
        let on_disk = std::fs::read_to_string(&install_report.manifest_path).unwrap();
        assert!(on_disk.contains("/usr/local/bin/clipper") || on_disk.contains("clipper"));

        let uninstall_report = uninstall(options_with(prefix.path(), false)).expect("uninstall");
        assert_eq!(uninstall_report.outcome, Outcome::Wrote);
        assert!(!uninstall_report.manifest_path.exists());
    }

    #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
    #[test]
    fn install_is_idempotent() {
        let prefix = temp_prefix();
        let first = install(options_with(prefix.path(), false)).expect("install #1");
        assert_eq!(first.outcome, Outcome::Wrote);

        let second = install(options_with(prefix.path(), false)).expect("install #2");
        assert_eq!(second.outcome, Outcome::AlreadyInstalled);
        assert_eq!(first.manifest_path, second.manifest_path);
    }

    #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
    #[test]
    fn uninstall_when_not_installed_is_a_noop() {
        let prefix = temp_prefix();
        let report = uninstall(options_with(prefix.path(), false)).expect("uninstall");
        assert_eq!(report.outcome, Outcome::AlreadyInstalled);
    }

    #[cfg(any(target_os = "macos", target_os = "linux", target_os = "windows"))]
    #[test]
    fn install_creates_parent_directories() {
        let prefix = temp_prefix();
        // Sanity: the platform-specific subdirectories under `prefix`
        // do not yet exist — install() must create them.
        let report = install(options_with(prefix.path(), false)).expect("install");
        assert!(report.manifest_path.exists());
        let parent = report.manifest_path.parent().unwrap();
        assert!(parent.exists());
    }
}
