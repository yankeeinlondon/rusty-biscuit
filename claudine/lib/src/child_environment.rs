//! Process-entry launch state contributed to every Claudine child.

use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

/// Environment variable carrying Claudine's immutable launch directory.
pub const AGENT_CWD_ENV: &str = "AGENT_CWD";

static PROCESS_LAUNCH_CWD: OnceLock<Result<PathBuf, ChildEnvironmentError>> = OnceLock::new();

/// How the current Claudine process obtains its launch directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchDirectoryMode {
    /// A normal invocation owns its entry CWD and ignores inherited state.
    Ordinary,
    /// A provider hook retains the absolute launch directory supplied by its wrapper.
    ProviderHook,
}

/// Failure to capture a trustworthy process-entry launch directory.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ChildEnvironmentError {
    /// The process entry CWD could not be read.
    #[error("failed to capture the Claudine entry working directory: {source}")]
    CurrentDirectory {
        /// The operating-system failure returned at process entry.
        #[source]
        source: Arc<std::io::Error>,
    },
    /// A hook inherited a value that cannot represent an absolute launch directory.
    #[error("{AGENT_CWD_ENV} must be an absolute path for `claudine handle`; got `{value}`")]
    NonAbsoluteHookValue {
        /// The invalid inherited value.
        value: String,
    },
    /// A supplied entry CWD violated the process-boundary contract.
    #[error("the Claudine entry working directory must be absolute; got `{value}`")]
    NonAbsoluteEntryDirectory {
        /// The invalid captured value.
        value: String,
    },
}

/// Capture and retain the current process's immutable launch directory.
///
/// Call this at the CLI entry boundary before any command can change the
/// process directory or spawn a child. Repeated calls return the first result.
pub fn initialize_process_launch_directory(
    mode: LaunchDirectoryMode,
) -> Result<&'static Path, ChildEnvironmentError> {
    process_launch_directory_with(mode, || {
        std::env::current_dir().map_err(|source| ChildEnvironmentError::CurrentDirectory {
            source: Arc::new(source),
        })
    }, std::env::var_os(AGENT_CWD_ENV))
}

impl PartialEq for ChildEnvironmentError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::CurrentDirectory { source: left },
                Self::CurrentDirectory { source: right },
            ) => left.kind() == right.kind() && left.raw_os_error() == right.raw_os_error(),
            (
                Self::NonAbsoluteHookValue { value: left },
                Self::NonAbsoluteHookValue { value: right },
            )
            | (
                Self::NonAbsoluteEntryDirectory { value: left },
                Self::NonAbsoluteEntryDirectory { value: right },
            ) => left == right,
            _ => false,
        }
    }
}

impl Eq for ChildEnvironmentError {}

fn process_launch_directory_with(
    mode: LaunchDirectoryMode,
    current_dir: impl FnOnce() -> Result<PathBuf, ChildEnvironmentError>,
    inherited: Option<OsString>,
) -> Result<&'static Path, ChildEnvironmentError> {
    let result = PROCESS_LAUNCH_CWD.get_or_init(|| {
        let entry_cwd = current_dir()?;
        capture_launch_directory(mode, entry_cwd, inherited.as_deref())
    });
    result
        .as_deref()
        .map_err(Clone::clone)
}

/// Resolve one launch directory from explicit process-entry inputs.
///
/// This pure form is public so callers and tests can verify the mode-specific
/// boundary contract without mutating process-global state.
pub fn capture_launch_directory(
    mode: LaunchDirectoryMode,
    entry_cwd: PathBuf,
    inherited: Option<&OsStr>,
) -> Result<PathBuf, ChildEnvironmentError> {
    if !entry_cwd.is_absolute() {
        return Err(ChildEnvironmentError::NonAbsoluteEntryDirectory {
            value: entry_cwd.to_string_lossy().into_owned(),
        });
    }
    match (mode, inherited) {
        (LaunchDirectoryMode::ProviderHook, Some(value)) => {
            let path = PathBuf::from(value);
            if !path.is_absolute() {
                return Err(ChildEnvironmentError::NonAbsoluteHookValue {
                    value: value.to_string_lossy().into_owned(),
                });
            }
            Ok(path)
        }
        _ => Ok(entry_cwd),
    }
}

mod private {
    pub trait Sealed {}
}

/// A child command or complete child-environment map.
pub trait ChildEnvironmentTarget: private::Sealed {
    /// Overwrite the child-visible value with the captured launch directory.
    fn set_agent_cwd(&mut self, launch_cwd: &Path);
}

impl private::Sealed for std::process::Command {}

impl ChildEnvironmentTarget for std::process::Command {
    fn set_agent_cwd(&mut self, launch_cwd: &Path) {
        self.env(AGENT_CWD_ENV, launch_cwd);
    }
}

impl private::Sealed for tokio::process::Command {}

impl ChildEnvironmentTarget for tokio::process::Command {
    fn set_agent_cwd(&mut self, launch_cwd: &Path) {
        self.env(AGENT_CWD_ENV, launch_cwd);
    }
}

impl private::Sealed for HashMap<OsString, OsString> {}

impl ChildEnvironmentTarget for HashMap<OsString, OsString> {
    fn set_agent_cwd(&mut self, launch_cwd: &Path) {
        #[cfg(windows)]
        self.retain(|key, _| !key.to_string_lossy().eq_ignore_ascii_case(AGENT_CWD_ENV));
        self.insert(OsString::from(AGENT_CWD_ENV), launch_cwd.as_os_str().to_owned());
    }
}

/// Contribute the immutable process-entry launch directory to one child.
///
/// This is the sole writer of Claudine's child-visible `AGENT_CWD` contract.
pub fn contribute_child_environment(
    target: &mut impl ChildEnvironmentTarget,
) -> Result<(), ChildEnvironmentError> {
    let launch_cwd = initialize_process_launch_directory(LaunchDirectoryMode::Ordinary)?;
    target.set_agent_cwd(launch_cwd);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinary_capture_ignores_stale_inherited_value() {
        let entry = absolute_fixture("ordinary-entry");
        let stale = absolute_fixture("stale-parent");
        assert_eq!(
            capture_launch_directory(LaunchDirectoryMode::Ordinary, entry.clone(), Some(stale.as_os_str())).unwrap(),
            entry
        );
    }

    #[test]
    fn provider_hook_adopts_absolute_inherited_value() {
        let entry = absolute_fixture("hook-entry");
        let inherited = absolute_fixture("wrapper-launch");
        assert_eq!(
            capture_launch_directory(LaunchDirectoryMode::ProviderHook, entry, Some(inherited.as_os_str())).unwrap(),
            inherited
        );
    }

    #[test]
    fn provider_hook_falls_back_to_entry_when_value_is_missing() {
        let entry = absolute_fixture("hook-fallback");
        assert_eq!(
            capture_launch_directory(LaunchDirectoryMode::ProviderHook, entry.clone(), None).unwrap(),
            entry
        );
    }

    #[test]
    fn provider_hook_rejects_original_relative_input() {
        let error = capture_launch_directory(
            LaunchDirectoryMode::ProviderHook,
            absolute_fixture("hook-entry"),
            Some(OsStr::new("relative/path")),
        )
        .unwrap_err();
        assert_eq!(
            error,
            ChildEnvironmentError::NonAbsoluteHookValue {
                value: "relative/path".to_string(),
            }
        );
    }

    #[test]
    fn environment_map_overwrites_a_stale_value() {
        let launch = absolute_fixture("map-launch");
        let mut env = HashMap::from([(
            OsString::from(AGENT_CWD_ENV),
            OsString::from("stale/value"),
        )]);
        env.set_agent_cwd(&launch);
        assert_eq!(env.get(OsStr::new(AGENT_CWD_ENV)), Some(&launch.into_os_string()));
    }

    #[test]
    fn repeated_reentry_contributions_remain_identical() {
        let expected = initialize_process_launch_directory(LaunchDirectoryMode::Ordinary)
            .unwrap()
            .as_os_str()
            .to_owned();
        for stale in ["retry", "resume", "loop", "sequence"] {
            let mut env = HashMap::from([(
                OsString::from(AGENT_CWD_ENV),
                OsString::from(stale),
            )]);
            contribute_child_environment(&mut env).unwrap();
            assert_eq!(env.get(OsStr::new(AGENT_CWD_ENV)), Some(&expected));
        }
    }

    #[test]
    fn standard_child_observes_the_captured_absolute_launch_directory() {
        let expected = initialize_process_launch_directory(LaunchDirectoryMode::Ordinary)
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let mut command = agent_cwd_echo_command();
        command.env(AGENT_CWD_ENV, "stale/value");
        contribute_child_environment(&mut command).unwrap();
        let output = command.output().unwrap();
        assert!(output.status.success());
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), expected);
        assert!(Path::new(&expected).is_absolute());
    }

    #[tokio::test]
    async fn tokio_child_observes_the_same_launch_directory() {
        let expected = initialize_process_launch_directory(LaunchDirectoryMode::Ordinary)
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let mut command = tokio::process::Command::from(agent_cwd_echo_command());
        contribute_child_environment(&mut command).unwrap();
        let output = command.output().await.unwrap();
        assert!(output.status.success());
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), expected);
    }

    #[cfg(windows)]
    fn agent_cwd_echo_command() -> std::process::Command {
        let mut command = std::process::Command::new("cmd.exe");
        command.args(["/D", "/C", "echo %AGENT_CWD%"]);
        command
    }

    #[cfg(not(windows))]
    fn agent_cwd_echo_command() -> std::process::Command {
        let mut command = std::process::Command::new("sh");
        command.args(["-c", "printf %s \"$AGENT_CWD\""]);
        command
    }

    fn absolute_fixture(name: &str) -> PathBuf {
        std::env::temp_dir().join(name)
    }
}
