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
    pub pre_dispatch: Duration,
    pub prep_phase: Duration,
    pub environment_setup: Duration,
    pub substages: Vec<SubstageTiming>,
}

/// A single named sub-stage timing within environment setup.
#[derive(Debug, Clone)]
pub(crate) struct SubstageTiming {
    pub name: &'static str,
    pub elapsed: Duration,
}

#[derive(Debug)]
pub(crate) struct StartupTimings {
    pub arg_parsing: Duration,
    pub tracing_init: Duration,
    pub config_loading: Duration,
    /// Duration from process start to subcommand dispatch (Phase A).
    pub pre_dispatch: Duration,
    /// Duration from subcommand entry to the start of `execute_composition_request_inner` (Phase B).
    pub prep_phase: Duration,
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
    substages: Vec<SubstageTiming>,
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
            substages: Vec::new(),
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

    /// Record a named sub-stage timing.
    #[allow(dead_code)]
    pub fn mark_substage(&mut self, name: &'static str, elapsed: Duration) {
        self.substages.push(SubstageTiming { name, elapsed });
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
                pre_dispatch: self.startup.pre_dispatch,
                prep_phase: self.startup.prep_phase,
                environment_setup: self.env_setup_elapsed,
                substages: self.substages,
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
    substages: Vec<SubstageTiming>,
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
            substages: Vec::new(),
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
            substages: Vec::new(),
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

    /// Record a named sub-stage timing.
    pub fn mark_substage(&mut self, name: &'static str, elapsed: Duration) {
        self.substages.push(SubstageTiming { name, elapsed });
    }

    /// Set the agent execution perf.
    pub fn set_agent_perf(&mut self, perf: AgentExecutionPerf) {
        self.agent_perf = Some(perf);
    }

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
                pre_dispatch: self.startup.pre_dispatch,
                prep_phase: self.startup.prep_phase,
                environment_setup: self.env_setup_elapsed,
                substages: self.substages,
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

/// A single row within a report section.
struct Row {
    label: String,
    value: String,
    indent: usize,
    /// Raw duration this row represents when it should contribute to a
    /// section's [`TotalKind::Sum`]. `None` for non-numeric rows (e.g., a
    /// launch count) and for overlapping diagnostic rows that would
    /// double-count if summed (e.g., substages of `environment setup`,
    /// or sub-buckets of `pre-dispatch`).
    sum_contribution: Option<Duration>,
}

/// How a section total should be computed.
enum TotalKind {
    /// Sum the raw durations of rows that opted into the sum via
    /// `Row::sum_contribution`. Overlapping diagnostic rows must set
    /// `sum_contribution = None` to avoid double-counting.
    Sum,
    /// Use a specific pre-computed value (for Agent Execution use
    /// `total_elapsed`; for Composition Report use `ComposePerfReport::total`).
    Fixed(Duration),
}

/// A report section with rows and an optional total.
struct Section {
    title: &'static str,
    rows: Vec<Row>,
    total: Option<(TotalKind, &'static str)>,
}

impl Section {
    fn new(title: &'static str) -> Self {
        Self {
            title,
            rows: Vec::new(),
            total: None,
        }
    }

    fn with_total(mut self, kind: TotalKind, label: &'static str) -> Self {
        self.total = Some((kind, label));
        self
    }

    /// Add a row whose raw duration contributes to the section sum.
    fn push_sum(&mut self, label: impl Into<String>, duration: Duration) {
        self.rows.push(Row {
            label: label.into(),
            value: fmt_duration(duration),
            indent: 2,
            sum_contribution: Some(duration),
        });
    }

    /// Add a non-numeric row (e.g., launch count) — never contributes to a sum.
    fn push_value(&mut self, label: impl Into<String>, value: impl Into<String>) {
        self.rows.push(Row {
            label: label.into(),
            value: value.into(),
            indent: 2,
            sum_contribution: None,
        });
    }

    /// Add a duration row that does **not** contribute to the section sum.
    ///
    /// Use for rows whose duration overlaps another row already counted
    /// (e.g., `arg parsing` is a sub-bucket of `pre-dispatch`).
    fn push_diagnostic(&mut self, label: impl Into<String>, duration: Duration) {
        self.rows.push(Row {
            label: label.into(),
            value: fmt_duration(duration),
            indent: 2,
            sum_contribution: None,
        });
    }

    /// Add an indented diagnostic row (sub-stage). Never contributes to
    /// the section sum because the parent row already does.
    fn push_indented(&mut self, label: impl Into<String>, duration: Duration) {
        self.rows.push(Row {
            label: label.into(),
            value: fmt_duration(duration),
            indent: 4,
            sum_contribution: None,
        });
    }
}

/// Compute the total duration for a section based on its [`TotalKind`].
fn compute_total(kind: &TotalKind, rows: &[Row]) -> Duration {
    match kind {
        TotalKind::Fixed(d) => *d,
        TotalKind::Sum => rows
            .iter()
            .filter_map(|r| r.sum_contribution)
            .fold(Duration::ZERO, |acc, d| acc + d),
    }
}

/// Render a single [`Section`] into the given string buffer.
fn render_section(body: &mut String, section: &Section) {
    if section.rows.is_empty() {
        return;
    }

    body.push_str("\n<b>");
    body.push_str(section.title);
    body.push_str("</b>\n");

    // Compute per-section column widths.
    let label_width = section
        .rows
        .iter()
        .map(|r| r.label.chars().count() + r.indent)
        .max()
        .unwrap_or(0)
        .max(4)
        + 2; // 2-space gutter between label and value column

    let value_width = section
        .rows
        .iter()
        .map(|r| r.value.chars().count())
        .max()
        .unwrap_or(0)
        .max(4);

    for row in &section.rows {
        body.push_str(&format!(
            "{:indent$}{:<label_width$}{:>value_width$}\n",
            "",
            row.label,
            row.value,
            indent = row.indent,
            label_width = label_width,
            value_width = value_width,
        ));
    }

    if let Some((ref kind, label)) = section.total {
        let total = compute_total(kind, &section.rows);
        let total_value = fmt_duration(total);
        let sep_width = label_width + value_width;
        let sep = "═".repeat(sep_width);
        body.push_str(&format!("{:indent$}{sep}\n", "", indent = 2,));
        body.push_str(&format!(
            "{:indent$}{:<label_width$}{:>value_width$}\n",
            "",
            format!("<b>{}</b>", label),
            total_value,
            indent = 2,
            label_width = label_width,
            value_width = value_width,
        ));
    }
}

/// Render a [`CommandPerfReport`] to a styled string suitable for stderr.
pub(crate) fn render_perf_report(report: &CommandPerfReport) -> String {
    use biscuit_terminal::components::block_quote::BlockQuote;
    use biscuit_terminal::components::prose::Prose;
    use biscuit_terminal::components::renderable::TerminalRenderable as _;
    use biscuit_terminal::utils::color::{Color, Tailwind};

    let mut body = String::new();

    // Title line
    body.push_str(&format!(
        "<b>Performance</b> <dim>(elapsed {})</dim>\n",
        fmt_duration(report.total_elapsed)
    ));

    // CLI Overhead section.
    //
    // Only `pre-dispatch`, `prep phase`, and `environment setup` are
    // disjoint and add up to the wall-clock CLI overhead; the rest are
    // diagnostic sub-buckets that overlap their parent row and must NOT
    // be counted into the section TOTAL.
    let mut cli = Section::new("CLI Overhead").with_total(TotalKind::Sum, "TOTAL:");
    cli.push_sum("pre-dispatch:", report.cli.pre_dispatch);
    cli.push_sum("prep phase:", report.cli.prep_phase);
    cli.push_diagnostic("arg parsing:", report.cli.arg_parsing);
    cli.push_diagnostic("config loading:", report.cli.config_loading);
    cli.push_diagnostic("tracing init:", report.cli.tracing_init);
    cli.push_sum("environment setup:", report.cli.environment_setup);
    for sub in &report.cli.substages {
        cli.push_indented(format!("{}:", sub.name), sub.elapsed);
    }
    render_section(&mut body, &cli);

    // Composition Report section: use the authoritative `compose.total`
    // produced by the darkmatter compose pass rather than summing
    // individual stage rows. The stage rows are diagnostic — their sum
    // can drift from `total` due to fmt rounding and stages that overlap
    // by design (e.g., transclusion calling back into compose).
    if let Some(compose) = &report.composition {
        let mut comp = Section::new("Composition Report")
            .with_total(TotalKind::Fixed(compose.total), "TOTAL:");
        for metric in &compose.metrics {
            comp.push_diagnostic(format!("{}:", metric.stage), metric.elapsed);
        }
        render_section(&mut body, &comp);
    }

    // Agent Execution section
    if let Some(agent) = &report.agent {
        let mut exec = Section::new("Agent Execution")
            .with_total(TotalKind::Fixed(agent.total_elapsed), "TOTAL:");
        exec.push_value("launches:", agent.launches.to_string());
        if let Some(latency) = agent.first_response_latency {
            exec.push_diagnostic("first response:", latency);
        } else {
            exec.push_value("first response:", "--".to_string());
        }
        if let Some(api) = agent.provider_api_duration {
            exec.push_diagnostic("provider api duration:", api);
        }
        render_section(&mut body, &exec);
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
                pre_dispatch: Duration::from_micros(500),
                prep_phase: Duration::from_millis(50),
                environment_setup: Duration::from_millis(312),
                substages: vec![
                    SubstageTiming {
                        name: "target resolution",
                        elapsed: Duration::from_millis(100),
                    },
                    SubstageTiming {
                        name: "header env plan",
                        elapsed: Duration::from_millis(50),
                    },
                    SubstageTiming {
                        name: "child env build",
                        elapsed: Duration::from_millis(80),
                    },
                    SubstageTiming {
                        name: "mcp composition",
                        elapsed: Duration::ZERO,
                    },
                    SubstageTiming {
                        name: "argv assembly",
                        elapsed: Duration::from_millis(30),
                    },
                    SubstageTiming {
                        name: "system prompt",
                        elapsed: Duration::from_millis(40),
                    },
                    SubstageTiming {
                        name: "stream + prompt delivery",
                        elapsed: Duration::from_millis(12),
                    },
                ],
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
            rendered.contains("pre-dispatch:"),
            "missing pre-dispatch: {rendered}"
        );
        assert!(
            rendered.contains("prep phase:"),
            "missing prep phase: {rendered}"
        );
        assert!(
            rendered.contains("arg parsing:"),
            "missing arg parsing: {rendered}"
        );
        assert!(
            rendered.contains("target resolution:"),
            "missing target resolution: {rendered}"
        );
        assert!(rendered.contains("TOTAL:"), "missing TOTAL: {rendered}");
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
            rendered.contains("provider api duration:"),
            "missing provider api duration: {rendered}"
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
                pre_dispatch: Duration::ZERO,
                prep_phase: Duration::ZERO,
                environment_setup: Duration::ZERO,
                substages: vec![],
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
                pre_dispatch: Duration::ZERO,
                prep_phase: Duration::ZERO,
                environment_setup: Duration::ZERO,
                substages: vec![],
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
            rendered.contains("first response:") && rendered.contains(" --"),
            "expected '--' fallback: {rendered}"
        );
        assert!(
            !rendered.contains("provider api duration:"),
            "should omit api duration when none: {rendered}"
        );
    }

    #[test]
    fn sequence_perf_accumulator_empty() {
        let startup = StartupTimings {
            arg_parsing: Duration::from_millis(1),
            tracing_init: Duration::from_millis(2),
            config_loading: Duration::from_millis(3),
            pre_dispatch: Duration::from_micros(100),
            prep_phase: Duration::from_millis(5),
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
            pre_dispatch: Duration::ZERO,
            prep_phase: Duration::ZERO,
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
            pre_dispatch: Duration::ZERO,
            prep_phase: Duration::ZERO,
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
            pre_dispatch: Duration::ZERO,
            prep_phase: Duration::ZERO,
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
            pre_dispatch: Duration::from_micros(100),
            prep_phase: Duration::from_millis(5),
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
            pre_dispatch: Duration::ZERO,
            prep_phase: Duration::ZERO,
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
            pre_dispatch: Duration::ZERO,
            prep_phase: Duration::ZERO,
        };
        let compose = darkmatter::markdown::compose::ComposePerfReport {
            total: Duration::from_millis(100),
            metrics: vec![],
        };
        let collector = CommandPerfCollector::new_with_composition("Test", startup, Some(compose));
        let report = collector.into_report(Duration::from_secs(1));
        assert!(report.composition.is_some());
    }

    /// Strip ANSI CSI escapes so snapshot assertions stay stable across
    /// terminal capability detection. Mirrors the helper used by the
    /// integration tests in `tests/common/mod.rs`.
    fn strip_ansi(input: &str) -> String {
        let mut out = String::with_capacity(input.len());
        let mut chars = input.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '\u{1b}' {
                if chars.peek() == Some(&'[') {
                    chars.next();
                    for code in chars.by_ref() {
                        if ('@'..='~').contains(&code) {
                            break;
                        }
                    }
                }
                continue;
            }
            out.push(ch);
        }
        out
    }

    /// Snapshot-style coverage for the rendered perf report.
    ///
    /// Locks down behaviour the previous string-parse-based total
    /// calculation got wrong: microsecond rows must show their value
    /// (not zero), long labels must not abut their value, the CLI TOTAL
    /// must not double-count overlapping rows, and the Composition Report
    /// TOTAL must come from `compose.total` rather than summing rounded
    /// stage rows.
    #[test]
    fn render_perf_report_snapshot_locks_totals_and_alignment() {
        let report = CommandPerfReport {
            title: "Compose",
            total_elapsed: Duration::from_millis(420),
            cli: CliOverheadReport {
                arg_parsing: Duration::from_micros(1_500),
                config_loading: Duration::from_micros(871),
                tracing_init: Duration::from_micros(166),
                pre_dispatch: Duration::from_micros(1_800),
                prep_phase: Duration::from_micros(538_400),
                environment_setup: Duration::from_micros(419_300),
                substages: vec![
                    SubstageTiming {
                        name: "target resolution",
                        elapsed: Duration::from_micros(45),
                    },
                    SubstageTiming {
                        name: "header env plan",
                        elapsed: Duration::from_micros(144),
                    },
                    SubstageTiming {
                        name: "child env build",
                        elapsed: Duration::from_micros(298),
                    },
                    SubstageTiming {
                        name: "mcp composition",
                        elapsed: Duration::ZERO,
                    },
                    SubstageTiming {
                        name: "argv assembly",
                        elapsed: Duration::from_micros(3),
                    },
                    SubstageTiming {
                        // Long label that overflowed the legacy 20-char
                        // column and ran into the value with no space.
                        name: "system prompt",
                        elapsed: Duration::from_micros(418_800),
                    },
                    SubstageTiming {
                        name: "stream + prompt delivery",
                        elapsed: Duration::from_micros(19),
                    },
                ],
            },
            composition: Some(darkmatter::markdown::compose::ComposePerfReport {
                // `compose.total` is the source of truth — it is NOT the
                // sum of `metrics[*].elapsed`, on purpose. The TOTAL row
                // must mirror this value exactly, not a sum of rows.
                total: Duration::from_micros(165),
                metrics: vec![
                    darkmatter::markdown::compose::ComposePerfMetric {
                        stage: darkmatter::markdown::compose::ComposeStage::Interpolation,
                        elapsed: Duration::from_micros(8),
                        calls: 1,
                    },
                    darkmatter::markdown::compose::ComposePerfMetric {
                        stage: darkmatter::markdown::compose::ComposeStage::ShellExpansion,
                        elapsed: Duration::from_micros(3),
                        calls: 1,
                    },
                ],
            }),
            agent: None,
            notes: vec!["Agent execution skipped (dry run)".into()],
        };

        let plain = strip_ansi(&render_perf_report(&report));
        let lines: Vec<&str> = plain.lines().collect();

        // Title line carries the elapsed wall-clock; strip the leading
        // BlockQuote border ("▌ ") before asserting.
        let title_line = lines
            .iter()
            .find(|l| l.contains("Performance"))
            .unwrap_or_else(|| panic!("missing title; got:\n{plain}"));
        assert!(
            title_line.contains("(elapsed 420.0ms)"),
            "expected '(elapsed 420.0ms)'; got: {title_line:?}"
        );

        // Microsecond row must render with its value, not zero. This is
        // the row that the broken `parse_fmt_duration` previously
        // suppressed when computing the section total.
        let micro_line = lines
            .iter()
            .find(|l| l.contains("target resolution:"))
            .unwrap_or_else(|| panic!("missing target resolution row; got:\n{plain}"));
        assert!(
            micro_line.contains("45µs"),
            "microsecond value missing; got: {micro_line:?}"
        );

        // Long label must keep at least one space between label and value.
        let long_label = lines
            .iter()
            .find(|l| l.contains("stream + prompt delivery:"))
            .unwrap_or_else(|| panic!("missing long label row; got:\n{plain}"));
        assert!(
            long_label.contains("delivery: ") || long_label.contains("delivery:  "),
            "long label collided with value; got: {long_label:?}"
        );
        assert!(
            long_label.contains("19µs"),
            "long label row missing value; got: {long_label:?}"
        );

        // CLI Overhead TOTAL must equal pre_dispatch + prep_phase +
        // environment_setup (overlapping diagnostic rows excluded).
        // 1_800 + 538_400 + 419_300 = 959_500 µs ≈ 959.5ms.
        let cli_total_line = lines
            .iter()
            .skip_while(|l| !l.contains("CLI Overhead"))
            .find(|l| l.contains("TOTAL:"))
            .unwrap_or_else(|| panic!("missing CLI TOTAL; got:\n{plain}"));
        assert!(
            cli_total_line.contains("959.5ms"),
            "CLI TOTAL should be 959.5ms (sum of disjoint rows), not parsed-string sum; got: {cli_total_line:?}"
        );

        // Composition Report TOTAL must come from `compose.total` (165µs),
        // not from re-summing the formatted stage rows.
        let comp_total_line = lines
            .iter()
            .skip_while(|l| !l.contains("Composition Report"))
            .find(|l| l.contains("TOTAL:"))
            .unwrap_or_else(|| panic!("missing Composition TOTAL; got:\n{plain}"));
        assert!(
            comp_total_line.contains("165µs"),
            "Composition TOTAL should mirror compose.total; got: {comp_total_line:?}"
        );

        // Dry-run note replaces the agent section entirely.
        assert!(
            !plain.contains("Agent Execution"),
            "dry run should omit Agent Execution; got:\n{plain}"
        );
        assert!(
            plain.contains("Agent execution skipped (dry run)"),
            "missing dry-run note; got:\n{plain}"
        );
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
