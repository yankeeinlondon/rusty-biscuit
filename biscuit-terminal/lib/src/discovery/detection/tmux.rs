//! What a visible tmux (`TMUX` set) says about graphics passthrough: its
//! version, `allow-passthrough` for this pane, and the terminals of the
//! clients attached to this pane's session.
//!
//! The parsers are pure; [`read_facts`] runs the read-only tmux commands.
//! Anything that cannot be read is unknown, never "off". The developer guide
//! is `biscuit-terminal/docs/discovery/graphics-routes.md`.

use std::time::Duration;

use super::route::ToolVersion;
use super::versions::run_with_deadline;

/// How long each tmux command may take before its answer counts as unknown.
const TMUX_COMMAND_DEADLINE: Duration = Duration::from_millis(500);

/// The tmux facts the graphics route reads.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TmuxFacts {
    /// `tmux -V`; `None` when unreadable.
    pub version: Option<ToolVersion>,
    /// `allow-passthrough` for this pane.
    pub passthrough: Passthrough,
    /// The clients attached to this pane's session; `None` when the list
    /// could not be read, empty when no client is attached.
    pub clients: Option<Vec<TmuxClient>>,
}

/// The pane's `allow-passthrough` value.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Passthrough {
    /// `on` or `all`.
    On,
    /// `off`.
    Off,
    /// The option could not be read (no executable, no socket access, an
    /// unsupported option, or an unexpected value).
    #[default]
    Unknown,
}

/// One client attached to the pane's session.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TmuxClient {
    /// `#{client_termname}`: the `TERM` the client was started with.
    pub termname: String,
    /// `#{client_termtype}`: the outer terminal's own answer to tmux's
    /// identity query; empty when it did not answer.
    pub termtype: String,
}

/// Parses `tmux show-options -Apv … allow-passthrough` output.
#[must_use]
pub fn parse_passthrough(output: &str) -> Passthrough {
    match output.trim() {
        "on" | "all" => Passthrough::On,
        "off" => Passthrough::Off,
        _ => Passthrough::Unknown,
    }
}

/// Parses `tmux list-clients -F '#{client_termname}\t#{client_termtype}'`
/// output, one client per non-empty line.
#[must_use]
pub fn parse_clients(output: &str) -> Vec<TmuxClient> {
    output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let (termname, termtype) = line.split_once('\t').unwrap_or((line, ""));
            TmuxClient {
                termname: termname.trim().to_string(),
                termtype: termtype.trim().to_string(),
            }
        })
        .collect()
}

/// Reads the facts for `pane` (`TMUX_PANE`) from the tmux this process runs
/// in. Each command runs without a shell and under a deadline. Without a
/// pane ID only the version is read: an untargeted query could describe
/// some other pane.
pub(crate) fn read_facts(pane: Option<&str>) -> TmuxFacts {
    let version = run_with_deadline("tmux", &["-V"], TMUX_COMMAND_DEADLINE)
        .as_deref()
        .and_then(ToolVersion::parse);
    let Some(pane) = pane.filter(|p| !p.is_empty()) else {
        return TmuxFacts {
            version,
            ..TmuxFacts::default()
        };
    };
    let passthrough = run_with_deadline(
        "tmux",
        &["show-options", "-Apv", "-t", pane, "allow-passthrough"],
        TMUX_COMMAND_DEADLINE,
    )
    .as_deref()
    .map_or(Passthrough::Unknown, parse_passthrough);
    // `-t <pane>` lists the clients of the session holding the pane, never
    // the server's other sessions.
    let clients = run_with_deadline(
        "tmux",
        &[
            "list-clients",
            "-t",
            pane,
            "-F",
            "#{client_termname}\t#{client_termtype}",
        ],
        TMUX_COMMAND_DEADLINE,
    )
    .as_deref()
    .map(parse_clients);
    TmuxFacts {
        version,
        passthrough,
        clients,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passthrough_values() {
        assert_eq!(parse_passthrough("on\n"), Passthrough::On);
        assert_eq!(parse_passthrough("all\n"), Passthrough::On);
        assert_eq!(parse_passthrough("off\n"), Passthrough::Off);
        assert_eq!(parse_passthrough(""), Passthrough::Unknown);
        assert_eq!(parse_passthrough("invalid option: allow-passthrough"), Passthrough::Unknown);
        assert_eq!(parse_passthrough("ON"), Passthrough::Unknown);
    }

    #[test]
    fn client_lines() {
        assert_eq!(
            parse_clients("xterm-kitty\tkitty(0.39.1)\nwezterm\t\n\n"),
            vec![
                TmuxClient {
                    termname: "xterm-kitty".into(),
                    termtype: "kitty(0.39.1)".into(),
                },
                TmuxClient {
                    termname: "wezterm".into(),
                    termtype: String::new(),
                },
            ]
        );
        assert_eq!(parse_clients(""), Vec::new());
        assert_eq!(
            parse_clients("xterm-256color"),
            vec![TmuxClient {
                termname: "xterm-256color".into(),
                termtype: String::new(),
            }]
        );
    }
}
