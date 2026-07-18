use std::ffi::OsString;
use std::time::Duration;

use crate::argv;

// Behavior submodules: model conversion (`report`), tree assembly (`tree`), and
// rendering (`render`). The data model below is shared by all three.
mod render;
mod report;
mod tree;

pub(crate) use render::emit_report;
pub(crate) use report::{CommandPerfCollector, SequencePerfAccumulator};

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
    /// Named, non-overlapping prep work units carved from `prep_phase`
    /// (P-5a). Each is a true `Structural` child of `prep phase`; the
    /// remainder lands in `prep → unattributed`. Empty for wrapper and
    /// sequence paths, which never stamp `prep_phase`.
    pub prep_substages: Vec<SubstageTiming>,
}

/// A single named sub-stage timing within environment setup.
///
/// `children` itemize where a substage's own measured time went — currently
/// only `child env build`, which carries `env sanitize` and `shadow home sync`
/// (and, under that, the `repo root detect` that dominates it). They render as
/// `Breakdown` nodes nested under the substage, so the substage keeps its
/// authoritative `Structural` total and the children stay out of reconciliation
/// (TR-1): a slow child can never make the substage exceed the `environment
/// setup` window it carves.
#[derive(Debug, Clone)]
pub(crate) struct SubstageTiming {
    pub name: &'static str,
    pub elapsed: Duration,
    pub children: Vec<SubstageTiming>,
}

impl SubstageTiming {
    /// A leaf substage timing with no breakdown — the common case.
    pub fn new(name: &'static str, elapsed: Duration) -> Self {
        Self {
            name,
            elapsed,
            children: Vec::new(),
        }
    }
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
    /// The single wall-clock zero point, captured at the top of `run()`.
    ///
    /// Threaded end to end so the headline is computed once at report-build
    /// time as `process_start.elapsed()` rather than from a fresh mid-flight
    /// timer started after prep has already run (TM-1).
    pub process_start: std::time::Instant,
    /// Named prep work units measured during the `compose_entry` → request
    /// window (P-5a), threaded from `compose.rs` into the report so they
    /// become `Structural` children of `prep phase`. Empty when prep is not
    /// instrumented (wrapper, sequence).
    pub prep_substages: Vec<SubstageTiming>,
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
    /// Where the merged composition subtree attaches when the tree is built
    /// (TM-3). Set by the collector that produced the report; consumed by the
    /// renderer and the debug reconciliation assertion so neither has to guess
    /// the command shape.
    pub placement: CompositionPlacement,
    /// Per-step performance for a sequence run (TM-3). Empty for single-shot
    /// commands. When present, the tree gains a `steps` Structural node whose
    /// children are per-step subtrees and `report.composition` / `report.agent`
    /// (the merged/aggregated views) are not rendered as separate nodes — they
    /// would double-count work already shown per step.
    pub sequence_steps: Vec<SequenceStepPerf>,
}

/// The reconciliation role of a [`PerfNode`] (TM-2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NodeRole {
    /// A disjoint bucket that contributes to its parent's reconciliation
    /// sum (TR-1) — e.g. `pre-dispatch`, `prep phase`, `environment setup`,
    /// `agent execution`.
    Structural,
    /// A child that itemizes its parent's cost but whose siblings may
    /// overlap or under-cover the parent (e.g. darkmatter compose stages).
    /// Shown and percentaged, but excluded from the reconciliation sum; the
    /// parent keeps its own authoritative measured total.
    Breakdown,
    /// Synthetic remainder absorbing a reconciling node's unattributed time
    /// (TR-3).
    Unattributed,
}

/// A marker flagging a node for emphasis in the rendered tree (P-3).
///
/// Populated by the Phase 5 renderer (dominant-leaf highlight); the Phase 3
/// model carries the field so the tree shape is stable before rendering lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // `Hot` is constructed by the Phase 5 renderer (P-3).
pub(crate) enum Marker {
    Hot,
}

/// One node in the unified performance tree (TM-2).
///
/// The whole report is a single tree rooted at the wall-clock `Performance`
/// node. Every child is a fraction of its parent; for `Structural` children
/// the parent reconciles to their sum plus an `Unattributed` remainder
/// (TR-1 / TR-3). `Breakdown` children are displayed but excluded from that
/// sum.
#[derive(Debug, Clone)]
#[allow(dead_code)] // `marker` is populated by the Phase 5 renderer (P-3).
pub(crate) struct PerfNode {
    pub label: String,
    pub total: Duration,
    pub role: NodeRole,
    pub marker: Option<Marker>,
    /// Number of times the underlying stage ran, when `> 1` (DM-1). Carried
    /// on `Breakdown` compose-stage leaves so the renderer can surface it.
    pub calls: Option<usize>,
    pub children: Vec<PerfNode>,
}

impl PerfNode {
    fn leaf(label: impl Into<String>, total: Duration, role: NodeRole) -> Self {
        Self {
            label: label.into(),
            total,
            role,
            marker: None,
            calls: None,
            children: Vec::new(),
        }
    }

    fn branch(
        label: impl Into<String>,
        total: Duration,
        role: NodeRole,
        children: Vec<PerfNode>,
    ) -> Self {
        Self {
            label: label.into(),
            total,
            role,
            marker: None,
            calls: None,
            children,
        }
    }
}

/// Where the single-shot composition subtree attaches in the tree.
///
/// Single-shot compose stamps `prep_phase` to span the compose pass
/// (`prep_phase ⊇ compose_perf.total`, RC-2), so composition nests beneath
/// `prep phase` ([`UnderPrep`](Self::UnderPrep)). Sequence runs never stamp
/// `prep_phase`: a step is composed just in time, inside the execution window
/// `steps → step N` measures, so its composition nests under that step
/// ([`UnderStep`](Self::UnderStep)). The merged `report.composition` aggregate
/// is not rendered — it would double-count the per-step detail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CompositionPlacement {
    UnderPrep,
    UnderStep,
}

/// Per-step performance data collected during a sequence run.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct SequenceStepPerf {
    pub step_index: usize,
    pub step_name: String,
    /// Wall-clock of the step's Phase-2 execution window (agent run + render).
    /// Per-step compose work happens earlier, during the shared environment-
    /// setup phase, so it is not part of this window (TM-3).
    pub wall_clock: Duration,
    pub compose_perf: Option<darkmatter::markdown::compose::ComposePerfReport>,
    pub agent_perf: Option<AgentExecutionPerf>,
    /// When the step ran a group, one entry per member task in declaration
    /// order; empty for every other step.
    ///
    /// These are `Breakdown` detail, never reconciling: a *parallel* group's
    /// member durations overlap, so they routinely sum past the step's own
    /// wall-clock and cannot be treated as a partition of it.
    pub group_tasks: Vec<SequenceTaskPerf>,
}

/// One group member task's contribution to the `--perf` tree.
#[derive(Debug, Clone)]
pub(crate) struct SequenceTaskPerf {
    pub name: String,
    pub duration: Duration,
}

#[cfg(test)]
mod tests {
    use super::*;
    // The tree-assembly, conversion, and rendering helpers moved into child
    // modules; bring the ones the suites drive directly back into scope.
    use super::render::render_perf_report;
    use super::report::fmt_duration;
    use super::tree::{build_perf_tree, tree_reconciles};

    mod bootstrap;
    mod perf_tree;
    mod report;
}
