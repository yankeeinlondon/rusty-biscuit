//! Whether the current process can reach a graphical display session.
//!
//! A GUI program (an editor such as VS Code or Zed) can open a window only
//! when the process has a display to draw on. Callers that choose between a
//! terminal and a GUI program ask this before launching anything.
//!
//! The value is deliberately **not** part of [`OsInfo`](super::OsInfo), host
//! inventory, or any cache: the answer belongs to one process's environment,
//! so it is produced on demand by an explicit call to
//! [`detect_display_session`].
//!
//! ## Notes
//!
//! - macOS and Windows always report [`DisplaySession::Available`]: their
//!   window servers do not advertise themselves through the environment, and
//!   a GUI launch from a remote shell there fails in the program, not here.
//! - Linux, including WSL1 and WSL2, reports `Available` only when
//!   `DISPLAY` or `WAYLAND_DISPLAY` is set to a non-empty value. WSLg sets
//!   both, so WSL follows the same rule as native Linux.
//! - Every other OS reports [`DisplaySession::Unavailable`].

use std::ffi::OsString;

use super::{OsType, RuntimeEnvironment, detect_os_type, detect_runtime_environment};

/// Whether a graphical display session is reachable from this process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DisplaySession {
    /// A GUI program can open a window.
    Available,
    /// No display session is reachable; a GUI program cannot open a window.
    Unavailable,
}

impl DisplaySession {
    /// Whether a GUI program can open a window.
    pub fn is_available(self) -> bool {
        matches!(self, Self::Available)
    }
}

/// The display session of the current process.
///
/// Reads the OS, the WSL runtime, and the process environment. Cheap,
/// synchronous, and uncached; it never spawns a process or connects to a
/// display server.
pub fn detect_display_session() -> DisplaySession {
    display_session_from(detect_os_type(), detect_runtime_environment(), |name| {
        std::env::var_os(name)
    })
}

/// The display-session rule for an explicit OS, runtime, and environment.
///
/// `var` returns an environment variable's value, `None` when unset. The
/// runtime is accepted so a caller states the WSL case explicitly; WSL uses
/// the Linux rule (see the [module notes](self#notes)).
pub fn display_session_from(
    os: OsType,
    _runtime: RuntimeEnvironment,
    var: impl Fn(&str) -> Option<OsString>,
) -> DisplaySession {
    match os {
        OsType::MacOS | OsType::Windows => DisplaySession::Available,
        OsType::Linux => {
            let set = |name: &str| var(name).is_some_and(|value| !value.is_empty());
            if set("DISPLAY") || set("WAYLAND_DISPLAY") {
                DisplaySession::Available
            } else {
                DisplaySession::Unavailable
            }
        }
        _ => DisplaySession::Unavailable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(pairs: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<OsString> {
        move |name| {
            pairs
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| OsString::from(value))
        }
    }

    #[test]
    fn macos_and_windows_always_have_a_display() {
        for os in [OsType::MacOS, OsType::Windows] {
            assert_eq!(
                display_session_from(os, RuntimeEnvironment::Native, env(&[])),
                DisplaySession::Available,
                "{os:?}"
            );
        }
    }

    #[test]
    fn linux_with_x11_display_is_available() {
        let session = display_session_from(OsType::Linux, RuntimeEnvironment::Native, env(&[("DISPLAY", ":0")]));
        assert_eq!(session, DisplaySession::Available);
    }

    #[test]
    fn linux_with_wayland_display_is_available() {
        let session = display_session_from(
            OsType::Linux,
            RuntimeEnvironment::Native,
            env(&[("WAYLAND_DISPLAY", "wayland-0")]),
        );
        assert_eq!(session, DisplaySession::Available);
    }

    #[test]
    fn linux_with_both_variables_empty_is_unavailable() {
        let session = display_session_from(
            OsType::Linux,
            RuntimeEnvironment::Native,
            env(&[("DISPLAY", ""), ("WAYLAND_DISPLAY", "")]),
        );
        assert_eq!(session, DisplaySession::Unavailable);
    }

    #[test]
    fn linux_with_one_empty_and_one_set_is_available() {
        let session = display_session_from(
            OsType::Linux,
            RuntimeEnvironment::Native,
            env(&[("DISPLAY", ""), ("WAYLAND_DISPLAY", "wayland-1")]),
        );
        assert_eq!(session, DisplaySession::Available);
    }

    #[test]
    fn linux_with_neither_variable_is_unavailable() {
        let session = display_session_from(OsType::Linux, RuntimeEnvironment::Native, env(&[]));
        assert_eq!(session, DisplaySession::Unavailable);
        assert!(!session.is_available());
    }

    #[test]
    fn wsl_follows_the_linux_rule() {
        for runtime in [RuntimeEnvironment::Wsl1, RuntimeEnvironment::Wsl2] {
            assert_eq!(
                display_session_from(OsType::Linux, runtime, env(&[("DISPLAY", ":0"), ("WAYLAND_DISPLAY", "wayland-0")])),
                DisplaySession::Available,
                "{runtime:?} with WSLg variables"
            );
            assert_eq!(
                display_session_from(OsType::Linux, runtime, env(&[])),
                DisplaySession::Unavailable,
                "{runtime:?} without WSLg"
            );
        }
    }

    #[test]
    fn other_operating_systems_are_unavailable() {
        let session = display_session_from(OsType::FreeBSD, RuntimeEnvironment::Native, env(&[("DISPLAY", ":0")]));
        assert_eq!(session, DisplaySession::Unavailable);
    }

    #[test]
    fn display_session_serializes_in_snake_case() {
        assert_eq!(serde_json::to_string(&DisplaySession::Available).unwrap(), "\"available\"");
        assert_eq!(serde_json::to_string(&DisplaySession::Unavailable).unwrap(), "\"unavailable\"");
    }

    #[test]
    fn detecting_the_process_display_session_does_not_panic() {
        let _ = detect_display_session();
    }
}
