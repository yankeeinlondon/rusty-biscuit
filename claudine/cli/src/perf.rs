use std::ffi::OsString;
use std::time::Duration;

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
}

pub(crate) struct CliOverheadReport {
    pub arg_parsing: Duration,
    pub config_loading: Duration,
    pub tracing_init: Duration,
    pub environment_setup: Duration,
}

#[derive(Debug)]
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
    }
}

/// Aggregated performance for a single child-process launch.
///
/// Used by direct wrappers, compose, inline-compose, and sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

/// Per-step performance data collected during a sequence run.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct SequenceStepPerf {
    pub step_index: usize,
    pub step_name: String,
    pub compose_perf: Option<darkmatter::markdown::compose::ComposePerfReport>,
    pub agent_perf: Option<AgentExecutionPerf>,
}

/// Accumulates performance data across all steps of a sequence.
///
/// Produces a single [`CommandPerfReport`] at the end of the run.
pub(crate) struct SequencePerfAccumulator {
    startup: StartupTimings,
    env_setup_started_at: Option<std::time::Instant>,
    env_setup_elapsed: Duration,
    steps: Vec<SequenceStepPerf>,
    dry_run: bool,
    partial: bool,
}

impl SequencePerfAccumulator {
    /// Start a new accumulator with the given startup timings.
    ///
    /// The environment-setup timer begins immediately.
    pub fn new(startup: StartupTimings) -> Self {
        Self {
            startup,
            env_setup_started_at: Some(std::time::Instant::now()),
            env_setup_elapsed: Duration::ZERO,
            steps: Vec::new(),
            dry_run: false,
            partial: false,
        }
    }

    /// Capture the elapsed time since construction as environment setup.
    pub fn mark_env_setup_complete(&mut self) {
        if let Some(started) = self.env_setup_started_at.take() {
            self.env_setup_elapsed = started.elapsed();
        }
    }

    /// Append a step's performance data.
    pub fn add_step(&mut self, step: SequenceStepPerf) {
        self.steps.push(step);
    }

    /// Mark the sequence as a dry run (agent execution was skipped).
    pub fn set_dry_run(&mut self) {
        self.dry_run = true;
    }

    /// Mark the sequence as partially completed (interrupted or fail-fast).
    pub fn set_partial(&mut self) {
        self.partial = true;
    }

    /// Consume the accumulator and build the final [`CommandPerfReport`].
    pub fn into_report(self, total_elapsed: Duration) -> CommandPerfReport {
        // Merge composition perf across all steps
        let mut composition: Option<darkmatter::markdown::compose::ComposePerfReport> = None;
        for step in &self.steps {
            if let Some(ref step_compose) = step.compose_perf {
                match composition {
                    Some(ref mut c) => c.merge(step_compose),
                    None => composition = Some(step_compose.clone()),
                }
            }
        }

        // Aggregate agent execution perf across all steps
        let mut agent: Option<AgentExecutionPerf> = None;
        let mut first_response_latencies: Vec<Duration> = Vec::new();
        let mut provider_api_total = Duration::ZERO;

        for step in &self.steps {
            if let Some(ref step_agent) = step.agent_perf {
                first_response_latencies.extend(step_agent.first_response_latency);
                if let Some(api) = step_agent.provider_api_duration {
                    provider_api_total += api;
                }
                match agent {
                    Some(ref mut a) => {
                        a.launches += step_agent.launches;
                        a.total_elapsed += step_agent.total_elapsed;
                    }
                    None => {
                        agent = Some(AgentExecutionPerf {
                            launches: step_agent.launches,
                            total_elapsed: step_agent.total_elapsed,
                            first_response_latency: None,
                            provider_api_duration: None,
                        });
                    }
                }
            }
        }

        let provider_api_total_opt = if provider_api_total > Duration::ZERO {
            Some(provider_api_total)
        } else {
            None
        };

        // Compute average and min first-response latency across all steps
        let mut notes = Vec::new();
        if !first_response_latencies.is_empty() {
            let total_latency: Duration = first_response_latencies.iter().sum();
            let avg = total_latency / first_response_latencies.len() as u32;
            let min = *first_response_latencies
                .iter()
                .min()
                .expect("non-empty checked above");
            notes.push(format!(
                "first response avg: {}, min: {}",
                fmt_duration(avg),
                fmt_duration(min)
            ));
            if let Some(ref mut a) = agent {
                a.first_response_latency = Some(avg);
            }
        }

        if let Some(ref mut a) = agent {
            a.provider_api_duration = provider_api_total_opt;
        }

        if self.partial {
            notes.push("partial sequence metrics".into());
        }
        if self.dry_run {
            notes.push("Agent execution skipped (dry run)".into());
        }

        CommandPerfReport {
            title: "Sequence",
            total_elapsed,
            cli: CliOverheadReport {
                arg_parsing: self.startup.arg_parsing,
                config_loading: self.startup.config_loading,
                tracing_init: self.startup.tracing_init,
                environment_setup: self.env_setup_elapsed,
            },
            composition,
            agent: if self.dry_run { None } else { agent },
            notes,
        }
    }
}

/// Generic perf collector for single-shot commands (wrapper, compose, inline-compose).
///
/// Holds startup timings, environment-setup duration, optional composition perf,
/// and optional agent execution perf. Produces a [`CommandPerfReport`] on completion.
#[derive(Debug)]
pub(crate) struct CommandPerfCollector {
    title: &'static str,
    startup: StartupTimings,
    env_setup_started_at: Option<std::time::Instant>,
    env_setup_elapsed: Duration,
    agent_perf: Option<AgentExecutionPerf>,
    composition_perf: Option<darkmatter::markdown::compose::ComposePerfReport>,
    dry_run: bool,
}

impl CommandPerfCollector {
    /// Start a new collector with the given title and startup timings.
    ///
    /// The environment-setup timer begins immediately.
    pub fn new(title: &'static str, startup: StartupTimings) -> Self {
        Self {
            title,
            startup,
            env_setup_started_at: Some(std::time::Instant::now()),
            env_setup_elapsed: Duration::ZERO,
            agent_perf: None,
            composition_perf: None,
            dry_run: false,
        }
    }

    /// Start a new collector that also holds composition perf from the outset.
    pub fn new_with_composition(
        title: &'static str,
        startup: StartupTimings,
        composition_perf: Option<darkmatter::markdown::compose::ComposePerfReport>,
    ) -> Self {
        Self {
            title,
            startup,
            env_setup_started_at: Some(std::time::Instant::now()),
            env_setup_elapsed: Duration::ZERO,
            agent_perf: None,
            composition_perf,
            dry_run: false,
        }
    }

    /// Capture the elapsed time since construction as environment setup.
    pub fn mark_env_setup_complete(&mut self) {
        if let Some(started) = self.env_setup_started_at.take() {
            self.env_setup_elapsed = started.elapsed();
        }
    }

    /// Set the agent execution perf.
    pub fn set_agent_perf(&mut self, perf: AgentExecutionPerf) {
        self.agent_perf = Some(perf);
    }

    /// Get the current agent execution perf, if any.
    pub fn agent_perf(&self) -> Option<AgentExecutionPerf> {
        self.agent_perf
    }

    /// Mark this run as a dry run (skips agent execution in the report).
    pub fn set_dry_run(&mut self) {
        self.dry_run = true;
        self.mark_env_setup_complete();
    }

    /// Consume the collector and build the final [`CommandPerfReport`].
    pub fn into_report(self, total_elapsed: Duration) -> CommandPerfReport {
        CommandPerfReport {
            title: self.title,
            total_elapsed,
            cli: CliOverheadReport {
                arg_parsing: self.startup.arg_parsing,
                config_loading: self.startup.config_loading,
                tracing_init: self.startup.tracing_init,
                environment_setup: self.env_setup_elapsed,
            },
            composition: self.composition_perf,
            agent: if self.dry_run { None } else { self.agent_perf },
            notes: if self.dry_run {
                vec!["Agent execution skipped (dry run)".into()]
            } else {
                vec![]
            },
        }
    }
}

/// Format a duration using the most readable unit for the magnitude.
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
        body.push_str(&format!("  {:20}{}\n", "launches:", agent.launches));
        if let Some(latency) = agent.first_response_latency {
            body.push_str(&format!(
                "  {:20}{}\n",
                "first response:",
                fmt_duration(latency)
            ));
        } else {
            body.push_str(&format!("  {:20}{}\n", "first response:", "--"));
        }
        body.push_str(&format!(
            "  {:20}{}\n",
            "total execution:",
            fmt_duration(agent.total_elapsed)
        ));
        if let Some(api) = agent.provider_api_duration {
            body.push_str(&format!("  {:20}{}\n", "provider api:", fmt_duration(api)));
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
        assert!(
            rendered.contains("Performance"),
            "missing title: {rendered}"
        );
        assert!(
            rendered.contains("CLI Overhead"),
            "missing CLI Overhead: {rendered}"
        );
        assert!(
            rendered.contains("arg parsing:"),
            "missing arg parsing: {rendered}"
        );
        assert!(
            rendered.contains("Composition Report"),
            "missing Composition Report: {rendered}"
        );
        assert!(
            rendered.contains("interpolation:"),
            "missing interpolation: {rendered}"
        );
        assert!(
            rendered.contains("Agent Execution"),
            "missing Agent Execution: {rendered}"
        );
        assert!(
            rendered.contains("first response:"),
            "missing first response: {rendered}"
        );
        assert!(
            rendered.contains("provider api:"),
            "missing provider api: {rendered}"
        );
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
        assert!(
            !rendered.contains("Agent Execution"),
            "should omit agent: {rendered}"
        );
        assert!(
            rendered.contains("partial metrics"),
            "missing note: {rendered}"
        );
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
        assert!(
            rendered.contains("first response:     --"),
            "expected '--' fallback: {rendered}"
        );
        assert!(
            !rendered.contains("provider api:"),
            "should omit api when none: {rendered}"
        );
    }

    #[test]
    fn sequence_perf_accumulator_empty() {
        let startup = StartupTimings {
            arg_parsing: Duration::from_millis(1),
            tracing_init: Duration::from_millis(2),
            config_loading: Duration::from_millis(3),
        };
        let mut acc = SequencePerfAccumulator::new(startup);
        acc.mark_env_setup_complete();
        let report = acc.into_report(Duration::from_secs(1));
        assert_eq!(report.title, "Sequence");
        assert!(report.composition.is_none());
        assert!(report.agent.is_none());
    }

    #[test]
    fn sequence_perf_accumulator_merges_composition() {
        let startup = StartupTimings {
            arg_parsing: Duration::ZERO,
            tracing_init: Duration::ZERO,
            config_loading: Duration::ZERO,
        };
        let mut acc = SequencePerfAccumulator::new(startup);
        acc.mark_env_setup_complete();

        let compose1 = darkmatter::markdown::compose::ComposePerfReport {
            total: Duration::from_millis(100),
            metrics: vec![
                darkmatter::markdown::compose::ComposePerfMetric {
                    stage: darkmatter::markdown::compose::ComposeStage::Interpolation,
                    elapsed: Duration::from_millis(10),
                    calls: 1,
                },
                darkmatter::markdown::compose::ComposePerfMetric {
                    stage: darkmatter::markdown::compose::ComposeStage::ShellExpansion,
                    elapsed: Duration::from_millis(20),
                    calls: 1,
                },
            ],
        };
        let compose2 = darkmatter::markdown::compose::ComposePerfReport {
            total: Duration::from_millis(200),
            metrics: vec![
                darkmatter::markdown::compose::ComposePerfMetric {
                    stage: darkmatter::markdown::compose::ComposeStage::Interpolation,
                    elapsed: Duration::from_millis(30),
                    calls: 2,
                },
                darkmatter::markdown::compose::ComposePerfMetric {
                    stage: darkmatter::markdown::compose::ComposeStage::TransclusionApply,
                    elapsed: Duration::from_millis(40),
                    calls: 1,
                },
            ],
        };

        acc.add_step(SequenceStepPerf {
            step_index: 0,
            step_name: "step1".into(),
            compose_perf: Some(compose1),
            agent_perf: None,
        });
        acc.add_step(SequenceStepPerf {
            step_index: 1,
            step_name: "step2".into(),
            compose_perf: Some(compose2),
            agent_perf: None,
        });

        let report = acc.into_report(Duration::from_secs(1));
        let compose = report.composition.expect("should have composition");
        assert_eq!(compose.total, Duration::from_millis(300));

        let interp = compose
            .metrics
            .iter()
            .find(|m| m.stage == darkmatter::markdown::compose::ComposeStage::Interpolation)
            .expect("interpolation metric");
        assert_eq!(interp.elapsed, Duration::from_millis(40));
        assert_eq!(interp.calls, 3);

        let shell = compose
            .metrics
            .iter()
            .find(|m| m.stage == darkmatter::markdown::compose::ComposeStage::ShellExpansion)
            .expect("shell expansion metric");
        assert_eq!(shell.elapsed, Duration::from_millis(20));
        assert_eq!(shell.calls, 1);

        let trans = compose
            .metrics
            .iter()
            .find(|m| m.stage == darkmatter::markdown::compose::ComposeStage::TransclusionApply)
            .expect("transclusion metric");
        assert_eq!(trans.elapsed, Duration::from_millis(40));
        assert_eq!(trans.calls, 1);
    }

    #[test]
    fn sequence_perf_accumulator_aggregates_agent_perf() {
        let startup = StartupTimings {
            arg_parsing: Duration::ZERO,
            tracing_init: Duration::ZERO,
            config_loading: Duration::ZERO,
        };
        let mut acc = SequencePerfAccumulator::new(startup);
        acc.mark_env_setup_complete();

        acc.add_step(SequenceStepPerf {
            step_index: 0,
            step_name: "step1".into(),
            compose_perf: None,
            agent_perf: Some(AgentExecutionPerf {
                launches: 1,
                total_elapsed: Duration::from_secs(1),
                first_response_latency: Some(Duration::from_millis(500)),
                provider_api_duration: Some(Duration::from_millis(800)),
            }),
        });
        acc.add_step(SequenceStepPerf {
            step_index: 1,
            step_name: "step2".into(),
            compose_perf: None,
            agent_perf: Some(AgentExecutionPerf {
                launches: 1,
                total_elapsed: Duration::from_secs(1),
                first_response_latency: Some(Duration::from_millis(1000)),
                provider_api_duration: Some(Duration::from_millis(900)),
            }),
        });

        let report = acc.into_report(Duration::from_secs(5));
        let agent = report.agent.expect("should have agent perf");
        assert_eq!(agent.launches, 2);
        assert_eq!(agent.total_elapsed, Duration::from_secs(2));
        assert_eq!(
            agent.first_response_latency,
            Some(Duration::from_millis(750))
        );
        assert_eq!(
            agent.provider_api_duration,
            Some(Duration::from_millis(1700))
        );

        let notes = report.notes.join(", ");
        assert!(
            notes.contains("first response avg:"),
            "missing avg note: {notes}"
        );
        assert!(notes.contains("min:"), "missing min note: {notes}");
    }

    #[test]
    fn sequence_perf_accumulator_partial_note() {
        let startup = StartupTimings {
            arg_parsing: Duration::ZERO,
            tracing_init: Duration::ZERO,
            config_loading: Duration::ZERO,
        };
        let mut acc = SequencePerfAccumulator::new(startup);
        acc.mark_env_setup_complete();
        acc.set_partial();
        let report = acc.into_report(Duration::from_secs(1));
        let notes = report.notes.join(", ");
        assert!(
            notes.contains("partial sequence metrics"),
            "missing partial note: {notes}"
        );
    }

    #[test]
    fn command_perf_collector_full_report() {
        let startup = StartupTimings {
            arg_parsing: Duration::from_millis(1),
            tracing_init: Duration::from_millis(2),
            config_loading: Duration::from_millis(3),
        };
        let mut collector = CommandPerfCollector::new("Test", startup);
        collector.mark_env_setup_complete();
        collector.set_agent_perf(AgentExecutionPerf {
            launches: 1,
            total_elapsed: Duration::from_secs(1),
            first_response_latency: Some(Duration::from_millis(100)),
            provider_api_duration: Some(Duration::from_millis(200)),
        });
        let report = collector.into_report(Duration::from_secs(2));
        assert_eq!(report.title, "Test");
        assert!(report.agent.is_some());
        assert_eq!(report.agent.unwrap().launches, 1);
    }

    #[test]
    fn command_perf_collector_dry_run() {
        let startup = StartupTimings {
            arg_parsing: Duration::ZERO,
            tracing_init: Duration::ZERO,
            config_loading: Duration::ZERO,
        };
        let mut collector = CommandPerfCollector::new("Test", startup);
        collector.set_dry_run();
        let report = collector.into_report(Duration::from_secs(1));
        assert!(report.agent.is_none());
        assert!(report.notes.iter().any(|n| n.contains("dry run")));
    }

    #[test]
    fn command_perf_collector_with_composition() {
        let startup = StartupTimings {
            arg_parsing: Duration::ZERO,
            tracing_init: Duration::ZERO,
            config_loading: Duration::ZERO,
        };
        let compose = darkmatter::markdown::compose::ComposePerfReport {
            total: Duration::from_millis(100),
            metrics: vec![],
        };
        let collector = CommandPerfCollector::new_with_composition("Test", startup, Some(compose));
        let report = collector.into_report(Duration::from_secs(1));
        assert!(report.composition.is_some());
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
