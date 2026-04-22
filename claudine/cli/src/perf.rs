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

#[allow(dead_code)]
pub(crate) struct PerfBootstrap {
    pub enabled: bool,
    pub command_kind: Option<PerfCommandKind>,
    pub started_at: Option<Instant>,
}

#[allow(dead_code)]
pub(crate) struct CliOverheadReport {
    pub arg_parsing: Duration,
    pub config_loading: Duration,
    pub tracing_init: Duration,
    pub environment_setup: Duration,
}

#[derive(Debug)]
#[allow(dead_code)]
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

/// Aggregated performance for a single child-process launch.
///
/// Used by direct wrappers, compose, inline-compose, and sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) struct AgentExecutionPerf {
    pub launches: usize,
    pub total_elapsed: Duration,
    pub first_response_latency: Option<Duration>,
    pub provider_api_duration: Option<Duration>,
}

/// Full performance report for one command invocation.
#[allow(dead_code)]
pub(crate) struct CommandPerfReport {
    pub title: &'static str,
    pub total_elapsed: Duration,
    pub cli: CliOverheadReport,
    pub composition: Option<darkmatter::markdown::compose::ComposePerfReport>,
    pub agent: Option<AgentExecutionPerf>,
    pub notes: Vec<String>,
}

/// Format a duration using the most readable unit for the magnitude.
#[allow(dead_code)]
fn fmt_duration(d: Duration) -> String {
    let micros = d.as_micros();
    if micros < 1_000 {
        format!("{}µs", micros)
    } else if micros < 1_000_000 {
        format!("{:.1}ms", micros as f64 / 1_000.0)
    } else {
        format!("{:.2}s", d.as_secs_f64())
    }
}

/// Render a [`CommandPerfReport`] to a styled string suitable for stderr.
#[allow(dead_code)]
pub(crate) fn render_perf_report(report: &CommandPerfReport) -> String {
    use biscuit_terminal::components::block_quote::BlockQuote;
    use biscuit_terminal::components::prose::Prose;
    use biscuit_terminal::components::renderable::Renderable as _;
    use biscuit_terminal::utils::color::{Color, Tailwind};

    let mut body = String::new();

    // Title line
    body.push_str(&format!(
        "<b>Performance</b> <dim>(elapsed {})</dim>\n",
        fmt_duration(report.total_elapsed)
    ));

    // CLI Overhead
    body.push_str("\n<b>CLI Overhead</b>\n");
    body.push_str(&format!(
        "  {:20}{}\n",
        "arg parsing:",
        fmt_duration(report.cli.arg_parsing)
    ));
    body.push_str(&format!(
        "  {:20}{}\n",
        "config loading:",
        fmt_duration(report.cli.config_loading)
    ));
    body.push_str(&format!(
        "  {:20}{}\n",
        "tracing init:",
        fmt_duration(report.cli.tracing_init)
    ));
    body.push_str(&format!(
        "  {:20}{}\n",
        "environment setup:",
        fmt_duration(report.cli.environment_setup)
    ));

    // Composition Report
    if let Some(compose) = &report.composition {
        body.push_str("\n<b>Composition Report</b>\n");
        body.push_str(&format!(
            "  {:20}{}\n",
            "total:",
            fmt_duration(compose.total)
        ));
        for metric in &compose.metrics {
            body.push_str(&format!(
                "  {:20}{}\n",
                format!("{}:", metric.stage),
                fmt_duration(metric.elapsed)
            ));
        }
    }

    // Agent Execution
    if let Some(agent) = &report.agent {
        body.push_str("\n<b>Agent Execution</b>\n");
        body.push_str(&format!(
            "  {:20}{}\n",
            "launches:",
            agent.launches
        ));
        if let Some(latency) = agent.first_response_latency {
            body.push_str(&format!(
                "  {:20}{}\n",
                "first response:",
                fmt_duration(latency)
            ));
        } else {
            body.push_str(&format!(
                "  {:20}{}\n",
                "first response:",
                "--"
            ));
        }
        body.push_str(&format!(
            "  {:20}{}\n",
            "total execution:",
            fmt_duration(agent.total_elapsed)
        ));
        if let Some(api) = agent.provider_api_duration {
            body.push_str(&format!(
                "  {:20}{}\n",
                "provider api:",
                fmt_duration(api)
            ));
        }
    }

    for note in &report.notes {
        body.push_str(&format!("\n<i>{note}</i>\n"));
    }

    let rendered = Prose::new(body.trim_end()).render_optimistic(None);
    let mut block = BlockQuote::from(rendered)
        .with_left_block_color(Color::Tailwind(Tailwind::Yellow400))
        .with_border("▌ ")
        .render_optimistic(None);
    if !block.ends_with('\n') {
        block.push('\n');
    }
    block
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

    #[test]
    fn render_perf_report_includes_all_sections() {
        let report = CommandPerfReport {
            title: "test",
            total_elapsed: Duration::from_millis(3420),
            cli: CliOverheadReport {
                arg_parsing: Duration::from_micros(2100),
                config_loading: Duration::from_millis(18),
                tracing_init: Duration::from_micros(4500),
                environment_setup: Duration::from_millis(312),
            },
            composition: Some(darkmatter::markdown::compose::ComposePerfReport {
                total: Duration::from_millis(280),
                metrics: vec![
                    darkmatter::markdown::compose::ComposePerfMetric {
                        stage: darkmatter::markdown::compose::ComposeStage::Interpolation,
                        elapsed: Duration::from_micros(20_400),
                        calls: 1,
                    },
                    darkmatter::markdown::compose::ComposePerfMetric {
                        stage: darkmatter::markdown::compose::ComposeStage::ShellExpansion,
                        elapsed: Duration::from_millis(90),
                        calls: 1,
                    },
                ],
            }),
            agent: Some(AgentExecutionPerf {
                launches: 1,
                total_elapsed: Duration::from_millis(2790),
                first_response_latency: Some(Duration::from_millis(1120)),
                provider_api_duration: Some(Duration::from_millis(2330)),
            }),
            notes: vec![],
        };

        let rendered = render_perf_report(&report);
        assert!(rendered.contains("Performance"), "missing title: {rendered}");
        assert!(rendered.contains("CLI Overhead"), "missing CLI Overhead: {rendered}");
        assert!(rendered.contains("arg parsing:"), "missing arg parsing: {rendered}");
        assert!(rendered.contains("Composition Report"), "missing Composition Report: {rendered}");
        assert!(rendered.contains("interpolation:"), "missing interpolation: {rendered}");
        assert!(rendered.contains("Agent Execution"), "missing Agent Execution: {rendered}");
        assert!(rendered.contains("first response:"), "missing first response: {rendered}");
        assert!(rendered.contains("provider api:"), "missing provider api: {rendered}");
    }

    #[test]
    fn render_perf_report_omits_composition_when_none() {
        let report = CommandPerfReport {
            title: "test",
            total_elapsed: Duration::from_secs(1),
            cli: CliOverheadReport {
                arg_parsing: Duration::ZERO,
                config_loading: Duration::ZERO,
                tracing_init: Duration::ZERO,
                environment_setup: Duration::ZERO,
            },
            composition: None,
            agent: None,
            notes: vec!["partial metrics".into()],
        };

        let rendered = render_perf_report(&report);
        assert!(
            !rendered.contains("Composition Report"),
            "should omit composition: {rendered}"
        );
        assert!(!rendered.contains("Agent Execution"), "should omit agent: {rendered}");
        assert!(rendered.contains("partial metrics"), "missing note: {rendered}");
    }

    #[test]
    fn render_perf_report_shows_dashes_for_missing_latency() {
        let report = CommandPerfReport {
            title: "test",
            total_elapsed: Duration::from_secs(1),
            cli: CliOverheadReport {
                arg_parsing: Duration::ZERO,
                config_loading: Duration::ZERO,
                tracing_init: Duration::ZERO,
                environment_setup: Duration::ZERO,
            },
            composition: None,
            agent: Some(AgentExecutionPerf {
                launches: 1,
                total_elapsed: Duration::from_millis(500),
                first_response_latency: None,
                provider_api_duration: None,
            }),
            notes: vec![],
        };

        let rendered = render_perf_report(&report);
        assert!(rendered.contains("first response:     --"), "expected '--' fallback: {rendered}");
        assert!(!rendered.contains("provider api:"), "should omit api when none: {rendered}");
    }

    #[test]
    fn fmt_duration_sub_second() {
        assert_eq!(fmt_duration(Duration::from_micros(420)), "420µs");
        assert_eq!(fmt_duration(Duration::from_millis(5)), "5.0ms");
        assert_eq!(fmt_duration(Duration::from_millis(18)), "18.0ms");
    }

    #[test]
    fn fmt_duration_second_and_above() {
        assert_eq!(fmt_duration(Duration::from_millis(1200)), "1.20s");
        assert_eq!(fmt_duration(Duration::from_secs_f64(2.333)), "2.33s");
        assert_eq!(fmt_duration(Duration::from_secs(12)), "12.00s");
    }
}
