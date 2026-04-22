use std::ffi::OsString;
use std::time::{Duration, Instant};

use crate::argv;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PerfCommandKind {
    Wrapper,
    Compose,
    InlineCompose,
    Sequence,
}

pub(crate) struct PerfBootstrap {
    pub enabled: bool,
    pub command_kind: Option<PerfCommandKind>,
    pub started_at: Option<Instant>,
}

pub(crate) struct CliOverheadReport {
    pub arg_parsing: Duration,
    pub config_loading: Duration,
    pub tracing_init: Duration,
    pub environment_setup: Duration,
}

pub(crate) struct StartupTimings {
    pub arg_parsing: Duration,
    pub tracing_init: Duration,
    pub config_loading: Duration,
}

pub(crate) fn scan_perf_bootstrap(raw: &[OsString]) -> PerfBootstrap {
    if raw.len() < 2 || argv::completion_mode_active() {
        return PerfBootstrap {
            enabled: false,
            command_kind: None,
            started_at: None,
        };
    }

    let stop = raw
        .iter()
        .position(|t| t.to_str() == Some("--"))
        .unwrap_or(raw.len());

    let has_perf = (1..stop).any(|i| raw[i].to_str() == Some("--perf"));

    if !has_perf {
        return PerfBootstrap {
            enabled: false,
            command_kind: None,
            started_at: None,
        };
    }

    let command_kind = if argv::find_subcommand(raw, argv::WRAPPER_SUBCOMMANDS).is_some() {
        Some(PerfCommandKind::Wrapper)
    } else {
        match argv::find_subcommand(raw, argv::COMPOSITION_SUBCOMMANDS) {
            Some((_, "compose")) => Some(PerfCommandKind::Compose),
            Some((_, "inline-compose")) => Some(PerfCommandKind::InlineCompose),
            Some((_, "sequence")) => Some(PerfCommandKind::Sequence),
            _ => None,
        }
    };

    PerfBootstrap {
        enabled: command_kind.is_some(),
        command_kind,
        started_at: Some(Instant::now()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(tokens: &[&str]) -> Vec<OsString> {
        tokens.iter().map(OsString::from).collect()
    }

    #[test]
    fn bootstrap_enabled_for_wrapper_with_perf() {
        let raw = argv(&["claudine", "codex", "prompt", "--perf"]);
        let bootstrap = scan_perf_bootstrap(&raw);
        assert!(bootstrap.enabled);
        assert_eq!(bootstrap.command_kind, Some(PerfCommandKind::Wrapper));
        assert!(bootstrap.started_at.is_some());
    }

    #[test]
    fn bootstrap_enabled_for_compose_with_perf() {
        let raw = argv(&["claudine", "compose", "--perf", "file.md"]);
        let bootstrap = scan_perf_bootstrap(&raw);
        assert!(bootstrap.enabled);
        assert_eq!(bootstrap.command_kind, Some(PerfCommandKind::Compose));
    }

    #[test]
    fn bootstrap_enabled_for_inline_compose_with_perf() {
        let raw = argv(&["claudine", "inline-compose", "--perf", "file.md"]);
        let bootstrap = scan_perf_bootstrap(&raw);
        assert!(bootstrap.enabled);
        assert_eq!(bootstrap.command_kind, Some(PerfCommandKind::InlineCompose));
    }

    #[test]
    fn bootstrap_enabled_for_sequence_with_perf() {
        let raw = argv(&["claudine", "sequence", "--perf", "file.md"]);
        let bootstrap = scan_perf_bootstrap(&raw);
        assert!(bootstrap.enabled);
        assert_eq!(bootstrap.command_kind, Some(PerfCommandKind::Sequence));
    }

    #[test]
    fn bootstrap_disabled_without_perf() {
        let raw = argv(&["claudine", "codex", "prompt"]);
        let bootstrap = scan_perf_bootstrap(&raw);
        assert!(!bootstrap.enabled);
        assert!(bootstrap.command_kind.is_none());
    }

    #[test]
    fn bootstrap_disabled_for_hooks_with_perf() {
        let raw = argv(&["claudine", "hooks", "--perf"]);
        let bootstrap = scan_perf_bootstrap(&raw);
        assert!(!bootstrap.enabled);
        assert!(bootstrap.command_kind.is_none());
    }

    #[test]
    fn bootstrap_disabled_for_logs_with_perf() {
        let raw = argv(&["claudine", "logs", "--perf"]);
        let bootstrap = scan_perf_bootstrap(&raw);
        assert!(!bootstrap.enabled);
    }

    #[test]
    fn bootstrap_ignores_perf_after_dash_dash() {
        let raw = argv(&["claudine", "codex", "--", "--perf"]);
        let bootstrap = scan_perf_bootstrap(&raw);
        assert!(!bootstrap.enabled);
    }

    #[test]
    fn bootstrap_disabled_for_empty_argv() {
        let raw = argv(&["claudine"]);
        let bootstrap = scan_perf_bootstrap(&raw);
        assert!(!bootstrap.enabled);
    }

    #[test]
    fn bootstrap_uses_first_matching_kind_for_wrapper() {
        let raw = argv(&["claudine", "claude", "--perf"]);
        let bootstrap = scan_perf_bootstrap(&raw);
        assert_eq!(bootstrap.command_kind, Some(PerfCommandKind::Wrapper));
    }
}
