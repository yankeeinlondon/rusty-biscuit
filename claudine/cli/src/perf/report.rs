//! Report-model conversion: the single-shot and sequence collectors that turn
//! raw startup / environment / step timings into a [`CommandPerfReport`].
//!
//! Both collectors sample the wall-clock headline from the threaded
//! `process_start` baseline (TM-1) and run the TR-4 reconciliation
//! `debug_assert` ([`super::tree::debug_assert_reconciles`]) on the produced
//! report in debug builds.

use std::time::Duration;

use super::tree::debug_assert_reconciles;
use super::{
    AgentExecutionPerf, CliOverheadReport, CommandPerfReport, CompositionPlacement,
    SequenceStepPerf, StartupTimings, SubstageTiming,
};

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
    invocation_work: Option<InvocationWorkCounts>,
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
            invocation_work: None,
        }
    }

    /// Retain the latest request-owned discovery counts for the rendered report.
    pub fn set_invocation_work(
        &mut self,
        work: &claudine::invocation_context::InvocationWorkSnapshot,
    ) {
        self.invocation_work = Some(InvocationWorkCounts::from(work));
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
        self.substages.push(SubstageTiming::new(name, elapsed));
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
    ///
    /// The headline is sampled here from the threaded wall-clock baseline
    /// (`startup.process_start`), so every emit site shares one zero point
    /// instead of a fresh mid-flight timer (TM-1).
    pub fn into_report(self) -> CommandPerfReport {
        let total_elapsed = self.startup.process_start.elapsed();
        let report = self.into_report_with_elapsed(total_elapsed);
        debug_assert_reconciles(&report);
        report
    }

    /// Build the report against a caller-supplied wall-clock total.
    ///
    /// Production goes through [`into_report`](Self::into_report); this seam
    /// exists so tests can assert against a deterministic headline.
    pub(super) fn into_report_with_elapsed(self, total_elapsed: Duration) -> CommandPerfReport {
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
        if let Some(work) = self.invocation_work {
            notes.push(work.note());
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
                prep_substages: self.startup.prep_substages,
            },
            composition,
            agent: if self.dry_run { None } else { agent },
            notes,
            placement: CompositionPlacement::UnderStep,
            sequence_steps: self.steps,
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
    invocation_work: Option<InvocationWorkCounts>,
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
            invocation_work: None,
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
            invocation_work: None,
        }
    }

    /// Retain the latest request-owned discovery counts for the rendered report.
    pub fn set_invocation_work(
        &mut self,
        work: &claudine::invocation_context::InvocationWorkSnapshot,
    ) {
        self.invocation_work = Some(InvocationWorkCounts::from(work));
    }

    /// Capture the elapsed time since construction as environment setup.
    pub fn mark_env_setup_complete(&mut self) {
        if let Some(started) = self.env_setup_started_at.take() {
            self.env_setup_elapsed = started.elapsed();
        }
    }

    /// Record a named sub-stage timing.
    pub fn mark_substage(&mut self, name: &'static str, elapsed: Duration) {
        self.substages.push(SubstageTiming::new(name, elapsed));
    }

    /// Record a named sub-stage timing carrying a `Breakdown` of where its own
    /// measured time went (e.g. `child env build` → `shadow home sync` → `repo
    /// root detect`). The children are projected as nested `Breakdown` nodes, so
    /// they itemize the substage without entering reconciliation (TR-1).
    #[allow(dead_code)]
    pub fn mark_substage_with_children(
        &mut self,
        name: &'static str,
        elapsed: Duration,
        children: Vec<SubstageTiming>,
    ) {
        self.substages.push(SubstageTiming {
            name,
            elapsed,
            children,
        });
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
    ///
    /// The headline is sampled here from the threaded wall-clock baseline
    /// (`startup.process_start`), so every emit site shares one zero point
    /// instead of a fresh mid-flight timer (TM-1).
    pub fn into_report(self) -> CommandPerfReport {
        let total_elapsed = self.startup.process_start.elapsed();
        let report = self.into_report_with_elapsed(total_elapsed);
        debug_assert_reconciles(&report);
        report
    }

    /// Build the report against a caller-supplied wall-clock total.
    ///
    /// Production goes through [`into_report`](Self::into_report); this seam
    /// exists so tests can assert against a deterministic headline.
    pub(super) fn into_report_with_elapsed(self, total_elapsed: Duration) -> CommandPerfReport {
        let mut notes = if self.dry_run {
            vec!["Agent execution skipped (dry run)".into()]
        } else {
            vec![]
        };
        if let Some(work) = self.invocation_work {
            notes.push(work.note());
        }
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
                prep_substages: self.startup.prep_substages,
            },
            composition: self.composition_perf,
            agent: if self.dry_run { None } else { self.agent_perf },
            notes,
            placement: CompositionPlacement::UnderPrep,
            sequence_steps: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
struct InvocationWorkCounts {
    git_root_discoveries: usize,
    topology_probes: usize,
    topology_reuses: usize,
    launch_context_constructions: usize,
    launch_context_extensions: usize,
    ambient_fallbacks: usize,
    prepared_context_consumers: Vec<(String, usize)>,
}

impl From<&claudine::invocation_context::InvocationWorkSnapshot> for InvocationWorkCounts {
    fn from(work: &claudine::invocation_context::InvocationWorkSnapshot) -> Self {
        Self {
            git_root_discoveries: work.git_root_discoveries,
            topology_probes: work.topology_probes,
            topology_reuses: work.topology_reuses,
            launch_context_constructions: work.launch_context_constructions,
            launch_context_extensions: work.launch_context_extensions,
            ambient_fallbacks: work.ambient_fallbacks,
            prepared_context_consumers: work
                .prepared_context_consumers
                .iter()
                .map(|(name, count)| (name.clone(), *count))
                .collect(),
        }
    }
}

impl InvocationWorkCounts {
    fn note(self) -> String {
        format!(
            "source context work: Git discoveries {}, topology probes {}, topology reuses {}; \
             launch captures {} (extensions {}), ambient fallbacks {}, prepared consumers [{}]",
            self.git_root_discoveries,
            self.topology_probes,
            self.topology_reuses,
            self.launch_context_constructions,
            self.launch_context_extensions,
            self.ambient_fallbacks,
            self.prepared_context_consumers
                .into_iter()
                .map(|(name, count)| {
                    if count == 1 {
                        name
                    } else {
                        format!("{name} ({count})")
                    }
                })
                .collect::<Vec<_>>()
                .join(", "),
        )
    }
}

/// Format a duration using the most readable unit for the magnitude.
///
/// One decimal place at every unit so durations read consistently with the
/// `--perf` tree's value column.
pub(super) fn fmt_duration(d: Duration) -> String {
    let micros = d.as_micros();
    if micros < 1_000 {
        format!("{:.1}µs", micros as f64)
    } else if micros < 1_000_000 {
        format!("{:.1}ms", micros as f64 / 1_000.0)
    } else {
        format!("{:.1}s", d.as_secs_f64())
    }
}
