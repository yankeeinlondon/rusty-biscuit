use std::ffi::OsString;
use std::time::Duration;

use biscuit_terminal::components::metrics_tree::{
    MetricMarker, MetricNode, MetricShare, MetricValue,
};

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
/// `prep_phase`; their per-step compose work is metered during environment
/// setup (Phase 1c) and rendered under `environment setup → step preparation`
/// (TM-3) — the window that actually contains it. The per-step execution
/// wall-clock the `steps` node measures does **not** contain composition, so
/// nesting compose there would let a slow compose exceed its displayed parent
/// (G-2). [`UnderEnvSetup`](Self::UnderEnvSetup) is the sequence marker; the
/// merged `report.composition` aggregate is not rendered (it would double-count
/// the per-step `step preparation` detail).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CompositionPlacement {
    UnderPrep,
    UnderEnvSetup,
}

/// Project a substage's nested breakdown into a `Breakdown` [`PerfNode`]
/// subtree, recursively. Every node is `Breakdown` so the whole subtree
/// itemizes its `Structural` ancestor without entering reconciliation (TR-1).
fn substage_breakdown_node(s: &SubstageTiming) -> PerfNode {
    PerfNode::branch(
        s.name,
        s.elapsed,
        NodeRole::Breakdown,
        s.children.iter().map(substage_breakdown_node).collect(),
    )
}

/// Assemble the [`PerfNode`] tree for a built [`CommandPerfReport`] (TM-2).
///
/// The root `Performance` node carries true wall-clock; its `Structural`
/// children (`pre-dispatch`, `prep phase`, `environment setup`, and — when the
/// agent ran — `agent execution`) reconcile to it via the synthetic
/// `unattributed` remainder (TR-1 / TR-3). Diagnostic sub-buckets and compose
/// stages are `Breakdown` children, nested where the time is actually spent so
/// no cost appears twice (RC-2 / RC-4).
fn build_perf_tree(report: &CommandPerfReport, placement: CompositionPlacement) -> PerfNode {
    let cli = &report.cli;
    let is_sequence = !report.sequence_steps.is_empty();

    // pre-dispatch — Structural, with diagnostic Breakdown sub-buckets.
    let pre_dispatch = PerfNode::branch(
        "pre-dispatch",
        cli.pre_dispatch,
        NodeRole::Structural,
        vec![
            PerfNode::leaf("arg parsing", cli.arg_parsing, NodeRole::Breakdown),
            PerfNode::leaf("tracing init", cli.tracing_init, NodeRole::Breakdown),
            PerfNode::leaf("config loading", cli.config_loading, NodeRole::Breakdown),
        ],
    );

    // composition subtree: authoritative total + Breakdown children. Stages are
    // grouped under their `ComposePhase` (DM-2), the shell-expansion stage gains
    // per-`::shell` directive spans (DM-3), and the context-capture cost attaches
    // as its own subtree (DM-4). DM-1 carries `calls` where a stage ran > 1.
    let composition = report
        .composition
        .as_ref()
        .map(|compose| (compose.total, build_composition_children(compose)));

    // prep phase — Structural. The P-5a named work units (`frontmatter load`,
    // `schema validation`, `prep context`, `shell approval`) are disjoint
    // sub-windows of the prep window, so they carve `prep_phase` as reconciling
    // Structural children; the rest falls to `prep → unattributed`. Single-shot
    // also nests `composition` (the metered prepare pass) beneath it as a
    // reconciling Structural child (RC-2); sequence leaves prep bare.
    let mut prep = PerfNode::leaf("prep phase", cli.prep_phase, NodeRole::Structural);
    for unit in &cli.prep_substages {
        prep.children.push(PerfNode::leaf(
            unit.name,
            unit.elapsed,
            NodeRole::Structural,
        ));
    }
    if placement == CompositionPlacement::UnderPrep
        && let Some((total, children)) = &composition
    {
        // Context capture happened during prep, before the metered compose
        // window — attach it under `prep phase`, not `composition` (OQ-3
        // Option C). It is a `Breakdown` node (parallel per-group timings whose
        // sum overstates the wall-clock), so it does not enter reconciliation.
        if let Some(capture) = report
            .composition
            .as_ref()
            .and_then(build_context_capture_node)
        {
            prep.children.push(capture);
        }
        prep.children.push(PerfNode::branch(
            "composition",
            *total,
            NodeRole::Structural,
            children.clone(),
        ));
    }
    let prep = finalize_reconciling(prep);

    // environment setup — Structural; substages are now a true Structural carve
    // of the parent window (TR-2 option a). The substage checkpoint chain is a
    // strict subset of the env-setup window (chain origin is captured after the
    // collector starts the window, and the window closes after the last
    // substage reset), so Σ substages ≤ env_setup and the remainder absorbs the
    // µs head/tail gap.
    // Each substage is a `Structural` carve of the env-setup window; its
    // `children` subtree (only `child env build` carries any) itemizes where
    // that substage's time went and nests as `Breakdown` at every depth so it is
    // displayed and percentaged without entering the substage's reconciliation
    // (TR-1).
    let env_children: Vec<PerfNode> = cli
        .substages
        .iter()
        .map(|s| {
            PerfNode::branch(
                s.name,
                s.elapsed,
                NodeRole::Structural,
                s.children.iter().map(substage_breakdown_node).collect(),
            )
        })
        .collect();
    let mut env_setup_node = PerfNode::branch(
        "environment setup",
        cli.environment_setup,
        NodeRole::Structural,
        env_children,
    );
    // A sequence composes every step up front, during this env-setup window
    // (Phase 1c), so each step's composition cost lives here — not in the
    // per-step execution window the `steps` node measures. Attaching it under
    // env setup as a `step preparation` Breakdown keeps the timeline honest: a
    // slow per-step compose can never exceed its displayed parent (G-2), the bug
    // a `steps → step N → composition` nesting reintroduced. Breakdown, so it is
    // displayed and percentaged without entering reconciliation — the env-setup
    // remainder keeps the authoritative total.
    if is_sequence && let Some(step_prep) = build_step_preparation_node(&report.sequence_steps) {
        env_setup_node.children.push(step_prep);
    }
    let env_setup = finalize_reconciling(env_setup_node);

    let mut top = vec![pre_dispatch, prep, env_setup];

    if is_sequence {
        // Sequence: a `steps` Structural node whose per-step children carry the
        // step's execution wall-clock and reconcile to the sequence headline,
        // leaving inter-step orchestration in the root remainder (TM-3). Per-step
        // composition is shown under `environment setup → step preparation` (it
        // was metered there), and the merged `composition` / aggregated `agent`
        // views on the report are not emitted as separate nodes.
        top.push(build_steps_node(&report.sequence_steps));
    } else if let Some(agent) = &report.agent {
        // agent execution — Structural when the agent ran (omitted on dry runs;
        // the P-5 `—` leaf is a Phase 5 rendering concern).
        let mut children = Vec::new();
        if let Some(latency) = agent.first_response_latency {
            children.push(PerfNode::leaf(
                "first response",
                latency,
                NodeRole::Breakdown,
            ));
        }
        if let Some(api) = agent.provider_api_duration {
            children.push(PerfNode::leaf(
                "provider api duration",
                api,
                NodeRole::Breakdown,
            ));
        }
        top.push(finalize_reconciling(PerfNode::branch(
            "agent execution",
            agent.total_elapsed,
            NodeRole::Structural,
            children,
        )));
    }

    finalize_reconciling(PerfNode::branch(
        "Performance",
        report.total_elapsed,
        NodeRole::Structural,
        top,
    ))
}

/// Build the `steps` Structural node for a sequence run (TM-3).
///
/// Each step becomes a `step N: <name>` Structural child whose total is the
/// step's execution wall-clock; the per-step wall-clocks therefore sum to the
/// `steps` total and, with `pre-dispatch` / `environment setup`, reconcile to
/// the sequence headline plus an orchestration remainder. The agent breakdown
/// hangs off each step as `Breakdown` detail on the execution the step total
/// already counts. Per-step composition is **not** nested here: it was metered
/// during environment setup (Phase 1c), not inside the execution window, so it
/// is rendered under `environment setup → step preparation`
/// ([`build_step_preparation_node`]). Nesting it under a step's execution total
/// — which does not contain it — would let a slow compose appear larger than its
/// parent (G-2), the exact contradiction this split removes.
fn build_steps_node(steps: &[SequenceStepPerf]) -> PerfNode {
    let total: Duration = steps.iter().map(|s| s.wall_clock).sum();
    let children = steps
        .iter()
        .map(|step| {
            let mut step_children = Vec::new();
            if let Some(agent) = &step.agent_perf {
                let mut agent_children = Vec::new();
                if let Some(latency) = agent.first_response_latency {
                    agent_children.push(PerfNode::leaf(
                        "first response",
                        latency,
                        NodeRole::Breakdown,
                    ));
                }
                if let Some(api) = agent.provider_api_duration {
                    agent_children.push(PerfNode::leaf(
                        "provider api duration",
                        api,
                        NodeRole::Breakdown,
                    ));
                }
                step_children.push(PerfNode::branch(
                    "agent",
                    agent.total_elapsed,
                    NodeRole::Breakdown,
                    agent_children,
                ));
            }
            PerfNode::branch(
                format!("step {}: {}", step.step_index + 1, step.step_name),
                step.wall_clock,
                NodeRole::Structural,
                step_children,
            )
        })
        .collect();
    finalize_reconciling(PerfNode::branch(
        "steps",
        total,
        NodeRole::Structural,
        children,
    ))
}

/// Build the `step preparation` Breakdown subtree for a sequence run (TM-3).
///
/// A sequence composes every step up front, during the shared environment-setup
/// phase (Phase 1c), so each step's composition cost lives inside the
/// `environment setup` window — not the per-step execution window the `steps`
/// node measures. Each step that recorded composition perf becomes a
/// `step N: <name>` child whose total is `compose_perf.total` and whose children
/// are the phase-grouped compose stages ([`build_composition_children`]).
///
/// Every node is `Breakdown`: composition itemizes part of the env-setup window
/// but does not carve it (other Phase-1 work — target resolution, shell
/// approval — shares the window), so it is displayed and percentaged without
/// entering reconciliation. Because env setup genuinely contains all per-step
/// compose work (`Σ compose ≤ environment setup`), `step preparation` never
/// exceeds its parent, so a slow compose can no longer be rendered larger than
/// the node it hangs under (G-2). Returns `None` when no step recorded
/// composition perf.
fn build_step_preparation_node(steps: &[SequenceStepPerf]) -> Option<PerfNode> {
    let step_nodes: Vec<PerfNode> = steps
        .iter()
        .filter_map(|step| {
            let compose = step.compose_perf.as_ref()?;
            Some(PerfNode::branch(
                format!("step {}: {}", step.step_index + 1, step.step_name),
                compose.total,
                NodeRole::Breakdown,
                build_composition_children(compose),
            ))
        })
        .collect();
    if step_nodes.is_empty() {
        return None;
    }
    let total: Duration = step_nodes.iter().map(|n| n.total).sum();
    Some(PerfNode::branch(
        "step preparation",
        total,
        NodeRole::Breakdown,
        step_nodes,
    ))
}

/// Build the `Breakdown` children of the `composition` node from the enriched
/// `ComposePerfReport` (Milestone B).
///
/// Stages are grouped under their [`ComposePhase`](darkmatter::markdown::compose::ComposePhase)
/// (DM-2) in pipeline order, and the shell-expansion stage carries one leaf per
/// executed `::shell` directive (DM-3). Every node is `Breakdown`: the
/// `composition` node keeps its own authoritative total, so these enrichments
/// are displayed and percentaged without entering reconciliation (TR-1).
///
/// Context-capture cost (DM-4) is **not** attached here. The compose context is
/// captured by the caller before darkmatter's perf collector starts, so that
/// time is outside `compose_perf.total`; per OQ-3 Option C it attaches to
/// `prep phase` instead (see [`build_context_capture_node`]).
fn build_composition_children(
    compose: &darkmatter::markdown::compose::ComposePerfReport,
) -> Vec<PerfNode> {
    use darkmatter::markdown::compose::ComposePhase;

    let phase_label = |phase: ComposePhase| match phase {
        ComposePhase::InlinePre => "inline pre",
        ComposePhase::Transclusion => "transclusion",
        ComposePhase::InlinePost => "inline post",
        ComposePhase::Finalization => "finalization",
    };

    let mut children = Vec::new();
    for phase in [
        ComposePhase::InlinePre,
        ComposePhase::Transclusion,
        ComposePhase::InlinePost,
        ComposePhase::Finalization,
    ] {
        let stages: Vec<PerfNode> = compose
            .metrics
            .iter()
            .filter(|m| m.stage.phase() == phase)
            .map(|m| stage_leaf(m, compose))
            .collect();
        if stages.is_empty() {
            continue;
        }
        let total = stages.iter().map(|s| s.total).sum();
        children.push(PerfNode::branch(
            phase_label(phase),
            total,
            NodeRole::Breakdown,
            stages,
        ));
    }

    children
}

/// Build the `context capture` subtree (DM-4 / OQ-3 Option C) from the compose
/// report's per-group capture timings, or `None` when no group required I/O.
///
/// The context (git, repo, OS, hardware via sniff) is captured by the caller
/// during the prep window — *before* darkmatter's compose perf collector starts
/// — so attributing it to `composition` would misrepresent the timeline. It is
/// attached under `prep phase` instead.
///
/// The role is `Breakdown`, not `Structural`: the per-group timings are measured
/// independently and the groups are captured concurrently, so their sum
/// overstates the real wall-clock window (parallel work double-counts). It is
/// therefore displayed and percentaged but excluded from reconciliation (TR-1);
/// the true capture wall-clock stays in `prep → unattributed`. This is exactly
/// the `Breakdown` contract — siblings may overlap the parent.
fn build_context_capture_node(
    compose: &darkmatter::markdown::compose::ComposePerfReport,
) -> Option<PerfNode> {
    if compose.capture_timings.is_empty() {
        return None;
    }
    let leaves: Vec<PerfNode> = compose
        .capture_timings
        .iter()
        .map(|(name, elapsed)| PerfNode::leaf(name.clone(), *elapsed, NodeRole::Breakdown))
        .collect();
    let total = leaves.iter().map(|l| l.total).sum();
    Some(PerfNode::branch(
        "context capture",
        total,
        NodeRole::Breakdown,
        leaves,
    ))
}

/// Cap a `::shell` command display for the perf tree so a verbose command does
/// not dominate the label column. `MetricsTree` also truncates to the terminal
/// width, but that cap floats with the terminal; this fixed cap keeps the shell
/// row readable even on a wide terminal where the component would not trim.
fn truncate_command(command: &str) -> String {
    const MAX_COMMAND_CHARS: usize = 56;
    if command.chars().count() <= MAX_COMMAND_CHARS {
        return command.to_string();
    }
    let kept: String = command.chars().take(MAX_COMMAND_CHARS - 1).collect();
    format!("{kept}…")
}

/// One compose-stage `Breakdown` leaf, carrying `calls` where `> 1` (DM-1) and,
/// for the shell-expansion stage, the redacted per-directive spans as children
/// (DM-3) so the dominant `::shell` command — not just the aggregate — can be
/// flagged `HOT`.
fn stage_leaf(
    m: &darkmatter::markdown::compose::ComposePerfMetric,
    compose: &darkmatter::markdown::compose::ComposePerfReport,
) -> PerfNode {
    use darkmatter::markdown::compose::ComposeStage;

    let mut node = PerfNode::leaf(m.stage.to_string(), m.elapsed, NodeRole::Breakdown);
    if m.calls > 1 {
        node.calls = Some(m.calls);
    }
    if m.stage == ComposeStage::ShellExpansion && !compose.shell_spans.is_empty() {
        node.children = compose
            .shell_spans
            .iter()
            .map(|span| {
                PerfNode::leaf(
                    format!("shell · {}", truncate_command(&span.command_display)),
                    span.elapsed,
                    NodeRole::Breakdown,
                )
            })
            .collect();
    }
    node
}

/// Append the synthetic `unattributed` remainder to a node that has
/// `Structural` children, so TR-1 holds exactly by construction (TR-3):
/// `unattributed.total = max(0, node.total − Σ Structural children)`.
///
/// Nodes with only `Breakdown` children keep their authoritative total and
/// gain no remainder (their children may overlap or under-cover by design).
fn finalize_reconciling(mut node: PerfNode) -> PerfNode {
    let has_structural = node.children.iter().any(|c| c.role == NodeRole::Structural);
    if !has_structural {
        return node;
    }
    let structural_sum: Duration = node
        .children
        .iter()
        .filter(|c| c.role == NodeRole::Structural)
        .map(|c| c.total)
        .sum();
    let remainder = node.total.saturating_sub(structural_sum);
    node.children.push(PerfNode::leaf(
        "unattributed",
        remainder,
        NodeRole::Unattributed,
    ));
    node
}

/// Walk the tree asserting the TR-1 reconciliation invariant at every node
/// (TR-4 walker). Returns `false` on the first node whose `Structural`
/// children plus `Unattributed` remainder drift from its `total` beyond
/// `tolerance` — the `78.6ms`-headline-vs-`1.57s`-body bug class.
fn tree_reconciles(node: &PerfNode, tolerance: Duration) -> bool {
    node_reconciles(node, tolerance) && node.children.iter().all(|c| tree_reconciles(c, tolerance))
}

/// Check TR-1 at a single node. Nodes with no `Structural` children are
/// vacuously reconciling (their `Breakdown` children may overlap or
/// under-cover the parent by design).
fn node_reconciles(node: &PerfNode, tolerance: Duration) -> bool {
    if !node.children.iter().any(|c| c.role == NodeRole::Structural) {
        return true;
    }
    let structural_sum: Duration = node
        .children
        .iter()
        .filter(|c| c.role == NodeRole::Structural)
        .map(|c| c.total)
        .sum();
    let unattributed: Duration = node
        .children
        .iter()
        .filter(|c| c.role == NodeRole::Unattributed)
        .map(|c| c.total)
        .sum();
    let expected = structural_sum + unattributed;
    node.total.abs_diff(expected) <= tolerance
}

/// In debug builds, assert the assembled tree satisfies TR-1 at every
/// reconciling node (TR-4 runtime half). Compiled out of release builds, so
/// it carries zero cost and cannot panic in the field.
fn debug_assert_reconciles(report: &CommandPerfReport) {
    debug_assert!(
        tree_reconciles(
            &build_perf_tree(report, report.placement),
            Duration::from_millis(1)
        ),
        "perf report failed TR-1 reconciliation: headline {:?} disagrees with Σ structural buckets",
        report.total_elapsed,
    );
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
    fn into_report_with_elapsed(self, total_elapsed: Duration) -> CommandPerfReport {
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
                prep_substages: self.startup.prep_substages,
            },
            composition,
            agent: if self.dry_run { None } else { agent },
            notes,
            placement: CompositionPlacement::UnderEnvSetup,
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
    fn into_report_with_elapsed(self, total_elapsed: Duration) -> CommandPerfReport {
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
            notes: if self.dry_run {
                vec!["Agent execution skipped (dry run)".into()]
            } else {
                vec![]
            },
            placement: CompositionPlacement::UnderPrep,
            sequence_steps: Vec::new(),
        }
    }
}

/// Format a duration using the most readable unit for the magnitude.
///
/// One decimal place at every unit so durations read consistently with the
/// `--perf` tree's value column.
fn fmt_duration(d: Duration) -> String {
    let micros = d.as_micros();
    if micros < 1_000 {
        format!("{:.1}µs", micros as f64)
    } else if micros < 1_000_000 {
        format!("{:.1}ms", micros as f64 / 1_000.0)
    } else {
        format!("{:.1}s", d.as_secs_f64())
    }
}

/// True when an `unattributed` remainder is small enough to omit (TR-3 render
/// rule): below `1ms` **and** below `1%` of wall-clock. A material remainder is
/// always kept — it honestly signals an uninstrumented window.
fn is_remainder_noise(total: Duration, wall: Duration) -> bool {
    if total >= Duration::from_millis(1) {
        return false;
    }
    if wall.is_zero() {
        return true;
    }
    total.as_secs_f64() / wall.as_secs_f64() * 100.0 < 1.0
}

/// Drop below-threshold `unattributed` remainders so they don't clutter the
/// tree (TR-3). Pruning happens before connectors are computed so the
/// `├─`/`└─` choice reflects the rows that actually render.
fn prune_unattributed_noise(node: &mut PerfNode, wall: Duration) {
    node.children
        .retain(|c| !(c.role == NodeRole::Unattributed && is_remainder_noise(c.total, wall)));
    for child in &mut node.children {
        prune_unattributed_noise(child, wall);
    }
}

/// Insert the dry-run `agent execution` placeholder just before the root's
/// `unattributed` remainder (P-5). Zero total → `0%` share; the row collector
/// renders it as the `—` leaf.
fn inject_dry_run_agent(tree: &mut PerfNode) {
    let pos = tree
        .children
        .iter()
        .position(|c| c.role == NodeRole::Unattributed)
        .unwrap_or(tree.children.len());
    tree.children.insert(
        pos,
        PerfNode::leaf("agent execution", Duration::ZERO, NodeRole::Structural),
    );
}

/// Largest non-`Unattributed` leaf total in the tree, if any (P-3 candidate).
fn max_leaf_total(node: &PerfNode) -> Option<Duration> {
    if node.children.is_empty() {
        return (node.role != NodeRole::Unattributed).then_some(node.total);
    }
    node.children.iter().filter_map(max_leaf_total).max()
}

/// Flag the first leaf matching `target` as `HOT`.
fn set_hot(node: &mut PerfNode, target: Duration) -> bool {
    if node.children.is_empty() {
        if node.role != NodeRole::Unattributed && node.total == target {
            node.marker = Some(Marker::Hot);
            return true;
        }
        return false;
    }
    node.children.iter_mut().any(|c| set_hot(c, target))
}

/// Mark the dominant leaf `HOT`, but only if it clears the materiality floor
/// (≥20% of wall-clock); otherwise there is no hot spot worth pointing at (P-3).
fn mark_dominant_leaf(tree: &mut PerfNode, wall: Duration) {
    if wall.is_zero() {
        return;
    }
    if let Some(max) = max_leaf_total(tree)
        && !max.is_zero()
        && max.as_secs_f64() / wall.as_secs_f64() >= 0.20
    {
        set_hot(tree, max);
    }
}

/// Project a [`PerfNode`] into a biscuit-terminal [`MetricNode`] (BT-1).
///
/// The claudine tree decides *content* — which value, which share, which row is
/// hot, and the dry-run placeholder/annotation — while the shared component owns
/// connectors, column alignment, and the highlight glyph. `wall` is the report
/// total used for the share-of-wall-clock column (P-2); the root reads `100%`.
fn to_metric_node(node: &PerfNode, wall: Duration, is_root: bool, dry_run: bool) -> MetricNode {
    // The injected dry-run placeholder is the only `agent execution` leaf in a
    // dry-run report (the real agent node is omitted), so this match is exact.
    let is_dry_agent = dry_run && node.children.is_empty() && node.label == "agent execution";

    let value = if is_dry_agent {
        MetricValue::Placeholder
    } else {
        MetricValue::Duration(node.total)
    };
    let share = if is_root {
        MetricShare::Full
    } else if is_dry_agent || wall.is_zero() {
        MetricShare::Unknown
    } else {
        MetricShare::Of(node.total.as_secs_f64() / wall.as_secs_f64())
    };

    let children = node
        .children
        .iter()
        .map(|c| to_metric_node(c, wall, false, dry_run))
        .collect();

    let mut metric = MetricNode::branch(node.label.clone(), value, share, children);
    metric.calls = node.calls;
    metric.emphasize = is_root;
    if node.marker == Some(Marker::Hot) {
        metric.marker = Some(MetricMarker::Highlight);
    }
    if is_dry_agent {
        metric.note = Some("(dry run)".to_string());
    }
    metric
}

/// Render a built [`CommandPerfReport`] to stderr.
///
/// The single stderr write for every `--perf` emit site. Routing all
/// sites through here keeps the report a human-facing, stderr-only
/// artifact (G-8) and prevents the headline-sampling logic from drifting
/// back apart across the wrapper, composition, and sequence paths (TM-1).
pub(crate) fn emit_report(report: &CommandPerfReport) {
    eprint!("{}", render_perf_report(report));
}

/// Render a [`CommandPerfReport`] to a styled string suitable for stderr.
///
/// The report is one reconciling tree (TM-2): a `Performance` root over
/// `Structural` buckets that sum back to wall-clock. The renderer walks it
/// generically — box-drawing connectors from depth (P-1), a unit-aligned
/// duration column (P-4), a share-of-wall-clock percent column (P-2), and a
/// single `HOT` marker on the dominant leaf (P-3) — so the imbalance that
/// matters reads as the report's headline finding rather than noise.
pub(crate) fn render_perf_report(report: &CommandPerfReport) -> String {
    use biscuit_terminal::components::block_quote::BlockQuote;
    use biscuit_terminal::components::metrics_tree::MetricsTree;
    use biscuit_terminal::components::renderable::TerminalRenderable as _;
    use biscuit_terminal::terminal::Terminal;
    use biscuit_terminal::utils::color::{Color, Tailwind};

    let mut tree = build_perf_tree(report, report.placement);
    let wall = tree.total;
    // Dry run: the agent was skipped, so the tree carries no agent node; the
    // note flags it. We render a `—` placeholder leaf in its place (P-5).
    //
    // A sequence carries no top-level agent node even when agents ran (execution
    // lives under the `steps` node), so the injected `—` leaf and the note
    // suppression do not apply — the dry-run note renders as a trailing note and
    // the per-step subtrees already show the absence of agent children.
    let dry_run = report.sequence_steps.is_empty()
        && report.agent.is_none()
        && report.notes.iter().any(|n| n.contains("dry run"));

    prune_unattributed_noise(&mut tree, wall);
    if dry_run {
        inject_dry_run_agent(&mut tree);
    }
    mark_dominant_leaf(&mut tree, wall);

    // Notes other than the dry-run note, which the `—` leaf already conveys.
    let notes: Vec<String> = report
        .notes
        .iter()
        .filter(|n| !(dry_run && n.contains("dry run")))
        .cloned()
        .collect();

    // The shared component (BT-1) owns connectors, the unit-aligned value
    // column (P-4), the share column (P-2), and the single HOT marker (P-3);
    // claudine only projects its tree and chooses the highlighted row.
    let metrics = MetricsTree::new(to_metric_node(&tree, wall, true, dry_run)).with_notes(notes);

    // Render at the real terminal width minus the `▌ ` border so MetricsTree
    // caps its label column to exactly what survives inside the BlockQuote;
    // otherwise a long `::shell` label inflates the shared column and every row
    // wraps onto two lines (the `render_optimistic(None)` 80-col assumption hid
    // this until a verbose command appeared). The border is 2 columns wide.
    let term_width = Terminal::default().width();
    let inner_width = term_width.saturating_sub(2);
    let rendered = metrics.render_optimistic(Some(inner_width));

    let mut block = BlockQuote::from(rendered)
        .with_left_block_color(Color::Tailwind(Tailwind::Yellow400))
        .with_border("▌ ")
        .render_optimistic(Some(term_width));
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
                    SubstageTiming::new("target resolution", Duration::from_millis(100)),
                    SubstageTiming::new("header env plan", Duration::from_millis(50)),
                    SubstageTiming::new("child env build", Duration::from_millis(80)),
                    SubstageTiming::new("mcp composition", Duration::ZERO),
                    SubstageTiming::new("argv assembly", Duration::from_millis(30)),
                    SubstageTiming::new("system prompt", Duration::from_millis(40)),
                    SubstageTiming::new("stream + prompt delivery", Duration::from_millis(12)),
                ],
                prep_substages: vec![],
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
                ..Default::default()
            }),
            agent: Some(AgentExecutionPerf {
                launches: 1,
                total_elapsed: Duration::from_millis(2790),
                first_response_latency: Some(Duration::from_millis(1120)),
                provider_api_duration: Some(Duration::from_millis(2330)),
            }),
            notes: vec![],
            placement: CompositionPlacement::UnderPrep,
            sequence_steps: Vec::new(),
        };

        let rendered = strip_ansi(&render_perf_report(&report));
        // Every structural bucket and nested breakdown appears as a tree row.
        for label in [
            "Performance",
            "pre-dispatch",
            "prep phase",
            "arg parsing",
            "target resolution",
            "composition",
            "interpolation",
            "environment setup",
            "agent execution",
            "first response",
            "provider api duration",
        ] {
            assert!(rendered.contains(label), "missing {label}: {rendered}");
        }
        // Box-drawing connectors (P-1) and the wall-clock share column (P-2).
        assert!(
            rendered.contains("├─"),
            "missing tree connectors: {rendered}"
        );
        assert!(rendered.contains("100%"), "missing root share: {rendered}");
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
                prep_substages: vec![],
            },
            composition: None,
            agent: None,
            notes: vec!["partial metrics".into()],
            placement: CompositionPlacement::UnderPrep,
            sequence_steps: Vec::new(),
        };

        let rendered = strip_ansi(&render_perf_report(&report));
        assert!(
            !rendered.contains("composition"),
            "should omit composition: {rendered}"
        );
        // No agent ran and this is not a dry run, so no agent row is injected.
        assert!(
            !rendered.contains("agent execution"),
            "should omit agent: {rendered}"
        );
        assert!(
            rendered.contains("partial metrics"),
            "missing note: {rendered}"
        );
    }

    #[test]
    fn render_perf_report_omits_agent_breakdown_when_latency_missing() {
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
                prep_substages: vec![],
            },
            composition: None,
            agent: Some(AgentExecutionPerf {
                launches: 1,
                total_elapsed: Duration::from_millis(500),
                first_response_latency: None,
                provider_api_duration: None,
            }),
            notes: vec![],
            placement: CompositionPlacement::UnderPrep,
            sequence_steps: Vec::new(),
        };

        let rendered = strip_ansi(&render_perf_report(&report));
        // The agent ran, so its bucket appears — but with no latency or API
        // breakdown the node is a bare leaf rather than a parent with rows.
        assert!(
            rendered.contains("agent execution"),
            "missing agent execution row: {rendered}"
        );
        assert!(
            !rendered.contains("first response"),
            "should omit first response when none: {rendered}"
        );
        assert!(
            !rendered.contains("provider api duration"),
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
            process_start: std::time::Instant::now(),
            prep_substages: Vec::new(),
        };
        let mut acc = SequencePerfAccumulator::new(startup);
        acc.mark_env_setup_complete();
        let report = acc.into_report_with_elapsed(Duration::from_secs(1));
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
            process_start: std::time::Instant::now(),
            prep_substages: Vec::new(),
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
            ..Default::default()
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
            ..Default::default()
        };

        acc.add_step(SequenceStepPerf {
            step_index: 0,
            step_name: "step1".into(),
            wall_clock: Duration::from_millis(150),
            compose_perf: Some(compose1),
            agent_perf: None,
        });
        acc.add_step(SequenceStepPerf {
            step_index: 1,
            step_name: "step2".into(),
            wall_clock: Duration::from_millis(250),
            compose_perf: Some(compose2),
            agent_perf: None,
        });

        let report = acc.into_report_with_elapsed(Duration::from_secs(1));
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
            process_start: std::time::Instant::now(),
            prep_substages: Vec::new(),
        };
        let mut acc = SequencePerfAccumulator::new(startup);
        acc.mark_env_setup_complete();

        acc.add_step(SequenceStepPerf {
            step_index: 0,
            step_name: "step1".into(),
            wall_clock: Duration::from_millis(1100),
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
            wall_clock: Duration::from_millis(1100),
            compose_perf: None,
            agent_perf: Some(AgentExecutionPerf {
                launches: 1,
                total_elapsed: Duration::from_secs(1),
                first_response_latency: Some(Duration::from_millis(1000)),
                provider_api_duration: Some(Duration::from_millis(900)),
            }),
        });

        let report = acc.into_report_with_elapsed(Duration::from_secs(5));
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
            process_start: std::time::Instant::now(),
            prep_substages: Vec::new(),
        };
        let mut acc = SequencePerfAccumulator::new(startup);
        acc.mark_env_setup_complete();
        acc.set_partial();
        let report = acc.into_report_with_elapsed(Duration::from_secs(1));
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
            process_start: std::time::Instant::now(),
            prep_substages: Vec::new(),
        };
        let mut collector = CommandPerfCollector::new("Test", startup);
        collector.mark_env_setup_complete();
        collector.set_agent_perf(AgentExecutionPerf {
            launches: 1,
            total_elapsed: Duration::from_secs(1),
            first_response_latency: Some(Duration::from_millis(100)),
            provider_api_duration: Some(Duration::from_millis(200)),
        });
        let report = collector.into_report_with_elapsed(Duration::from_secs(2));
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
            process_start: std::time::Instant::now(),
            prep_substages: Vec::new(),
        };
        let mut collector = CommandPerfCollector::new("Test", startup);
        collector.set_dry_run();
        let report = collector.into_report_with_elapsed(Duration::from_secs(1));
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
            process_start: std::time::Instant::now(),
            prep_substages: Vec::new(),
        };
        let compose = darkmatter::markdown::compose::ComposePerfReport {
            total: Duration::from_millis(100),
            metrics: vec![],
            ..Default::default()
        };
        let collector = CommandPerfCollector::new_with_composition("Test", startup, Some(compose));
        let report = collector.into_report_with_elapsed(Duration::from_secs(1));
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

    /// Snapshot-style coverage for the rendered tree report.
    ///
    /// Re-expresses the guarantees the legacy section layout locked, now
    /// against the nested tree (Phase 5): the headline carries true
    /// wall-clock; microsecond rows still show their value; long labels keep
    /// a gutter (P-4); `composition` mirrors `compose.total` and appears
    /// **once**, nested under `prep phase` (RC-2, no double-count); the
    /// dominant leaf is flagged `HOT` (P-3); and the dry run renders an `—`
    /// agent leaf (P-5).
    #[test]
    fn render_perf_report_snapshot_locks_tree_layout() {
        // The motivating dry-run compose, re-grounded on true wall-clock so
        // the tree reconciles: pre-dispatch + prep + env ≤ wall.
        let report = CommandPerfReport {
            title: "Compose",
            total_elapsed: Duration::from_millis(1600),
            cli: CliOverheadReport {
                arg_parsing: Duration::from_micros(1_500),
                config_loading: Duration::from_micros(871),
                tracing_init: Duration::from_micros(166),
                pre_dispatch: Duration::from_micros(1_800),
                prep_phase: Duration::from_millis(1500),
                environment_setup: Duration::from_millis(65),
                substages: vec![
                    SubstageTiming::new("target resolution", Duration::from_micros(45)),
                    SubstageTiming::new("system prompt", Duration::from_millis(60)),
                    // Longest label in the tree — exercises the shared label
                    // column and the label/value gutter.
                    SubstageTiming::new("stream + prompt delivery", Duration::from_micros(19)),
                ],
                prep_substages: vec![],
            },
            composition: Some(darkmatter::markdown::compose::ComposePerfReport {
                // `compose.total` is the source of truth — it is NOT the sum
                // of `metrics[*].elapsed`. The `composition` row mirrors it.
                total: Duration::from_micros(970_500),
                metrics: vec![
                    darkmatter::markdown::compose::ComposePerfMetric {
                        stage: darkmatter::markdown::compose::ComposeStage::ShellExpansion,
                        elapsed: Duration::from_micros(970_500),
                        calls: 1,
                    },
                    darkmatter::markdown::compose::ComposePerfMetric {
                        stage: darkmatter::markdown::compose::ComposeStage::Interpolation,
                        elapsed: Duration::from_micros(8),
                        calls: 1,
                    },
                ],
                ..Default::default()
            }),
            agent: None,
            notes: vec!["Agent execution skipped (dry run)".into()],
            placement: CompositionPlacement::UnderPrep,
            sequence_steps: Vec::new(),
        };

        let plain = strip_ansi(&render_perf_report(&report));
        let lines: Vec<&str> = plain.lines().collect();

        // The headline is true wall-clock (1.60s) at 100% — no longer the
        // tiny post-prep window the old broken capture showed.
        let title_line = lines
            .iter()
            .find(|l| l.contains("Performance"))
            .unwrap_or_else(|| panic!("missing title; got:\n{plain}"));
        assert!(
            title_line.contains("1.6s") && title_line.contains("100%"),
            "headline must read 1.6s @ 100%; got: {title_line:?}"
        );

        // Microsecond row renders with its value (one decimal place).
        let micro_line = lines
            .iter()
            .find(|l| l.contains("target resolution"))
            .unwrap_or_else(|| panic!("missing target resolution row; got:\n{plain}"));
        assert!(
            micro_line.contains("45.0µs"),
            "microsecond value missing; got: {micro_line:?}"
        );

        // Long label keeps a gutter before its value (P-4 alignment).
        let long_label = lines
            .iter()
            .find(|l| l.contains("stream + prompt delivery"))
            .unwrap_or_else(|| panic!("missing long label row; got:\n{plain}"));
        assert!(
            long_label.contains("delivery ") && long_label.contains("19.0µs"),
            "long label collided with or dropped its value; got: {long_label:?}"
        );

        // `composition` mirrors `compose.total` (970.5ms) and is nested under
        // `prep phase` with a tree connector — not a peer section.
        let comp_line = lines
            .iter()
            .find(|l| l.contains("composition"))
            .unwrap_or_else(|| panic!("missing composition row; got:\n{plain}"));
        assert!(
            comp_line.contains("970.5ms") && comp_line.contains("├─"),
            "composition must mirror compose.total and nest; got: {comp_line:?}"
        );

        // RC-2: the shell-expansion cost appears exactly once (it used to be
        // double-counted as both a prep cost and a peer Composition Report).
        assert_eq!(
            plain.matches("shell expansion").count(),
            1,
            "shell expansion must appear once, not double-counted; got:\n{plain}"
        );

        // P-3: the dominant leaf (shell expansion, ~61% of wall) is flagged.
        let hot_line = lines
            .iter()
            .find(|l| l.contains("▇ HOT"))
            .unwrap_or_else(|| panic!("missing HOT marker; got:\n{plain}"));
        assert!(
            hot_line.contains("shell expansion"),
            "HOT must flag the dominant leaf; got: {hot_line:?}"
        );

        // P-5: dry run renders an `—` agent leaf, and the standalone note is
        // folded into that leaf rather than printed separately.
        let agent_line = lines
            .iter()
            .find(|l| l.contains("agent execution"))
            .unwrap_or_else(|| panic!("missing agent execution row; got:\n{plain}"));
        assert!(
            agent_line.contains("—") && agent_line.contains("(dry run)"),
            "dry-run agent must render as an — leaf; got: {agent_line:?}"
        );
    }

    #[test]
    fn fmt_duration_sub_second() {
        assert_eq!(fmt_duration(Duration::from_micros(420)), "420.0µs");
        assert_eq!(fmt_duration(Duration::from_millis(5)), "5.0ms");
        assert_eq!(fmt_duration(Duration::from_millis(18)), "18.0ms");
    }

    #[test]
    fn fmt_duration_second_and_above() {
        assert_eq!(fmt_duration(Duration::from_millis(1200)), "1.2s");
        assert_eq!(fmt_duration(Duration::from_secs_f64(2.333)), "2.3s");
        assert_eq!(fmt_duration(Duration::from_secs(12)), "12.0s");
    }

    // --- Phase 3: PerfNode tree model + TR-1 reconciliation -----------------

    /// Build a report with the fixed `pre-dispatch` sub-buckets and the
    /// caller's top-level windows. Used by the tree-shape and reconciliation
    /// tests below.
    fn perf_report(
        total_elapsed: Duration,
        pre_dispatch: Duration,
        prep_phase: Duration,
        environment_setup: Duration,
        substages: Vec<SubstageTiming>,
        composition: Option<darkmatter::markdown::compose::ComposePerfReport>,
        agent: Option<AgentExecutionPerf>,
    ) -> CommandPerfReport {
        CommandPerfReport {
            title: "Test",
            total_elapsed,
            cli: CliOverheadReport {
                arg_parsing: Duration::from_millis(4),
                config_loading: Duration::from_millis(3),
                tracing_init: Duration::from_millis(1),
                pre_dispatch,
                prep_phase,
                environment_setup,
                substages,
                prep_substages: vec![],
            },
            composition,
            agent,
            notes: vec![],
            placement: CompositionPlacement::UnderPrep,
            sequence_steps: Vec::new(),
        }
    }

    fn child<'a>(node: &'a PerfNode, label: &str) -> Option<&'a PerfNode> {
        node.children.iter().find(|c| c.label == label)
    }

    fn compose_report() -> darkmatter::markdown::compose::ComposePerfReport {
        darkmatter::markdown::compose::ComposePerfReport {
            total: Duration::from_millis(300),
            metrics: vec![
                darkmatter::markdown::compose::ComposePerfMetric {
                    stage: darkmatter::markdown::compose::ComposeStage::Interpolation,
                    elapsed: Duration::from_millis(20),
                    calls: 1,
                },
                darkmatter::markdown::compose::ComposePerfMetric {
                    stage: darkmatter::markdown::compose::ComposeStage::ShellExpansion,
                    elapsed: Duration::from_millis(250),
                    calls: 3,
                },
            ],
            ..Default::default()
        }
    }

    #[test]
    fn perf_tree_assembles_with_roles() {
        let report = perf_report(
            Duration::from_secs(2),
            Duration::from_millis(10),
            Duration::from_millis(500),
            Duration::from_millis(100),
            vec![
                SubstageTiming::new("target resolution", Duration::from_millis(30)),
                SubstageTiming::new("system prompt", Duration::from_millis(40)),
            ],
            Some(compose_report()),
            Some(AgentExecutionPerf {
                launches: 1,
                total_elapsed: Duration::from_secs(1),
                first_response_latency: Some(Duration::from_millis(120)),
                provider_api_duration: Some(Duration::from_millis(800)),
            }),
        );

        let tree = build_perf_tree(&report, CompositionPlacement::UnderPrep);

        assert_eq!(tree.label, "Performance");
        assert_eq!(tree.role, NodeRole::Structural);
        assert_eq!(tree.total, Duration::from_secs(2));

        // Top-level Structural buckets are present.
        for label in [
            "pre-dispatch",
            "prep phase",
            "environment setup",
            "agent execution",
        ] {
            let node = child(&tree, label).unwrap_or_else(|| panic!("missing {label}"));
            assert_eq!(
                node.role,
                NodeRole::Structural,
                "{label} should be Structural"
            );
        }
        // Root reconciles with a synthetic remainder.
        assert!(
            child(&tree, "unattributed").is_some(),
            "root needs unattributed"
        );

        // pre-dispatch sub-buckets are Breakdown (RC-4: nesting is the signal).
        let pre = child(&tree, "pre-dispatch").unwrap();
        for label in ["arg parsing", "tracing init", "config loading"] {
            assert_eq!(
                child(pre, label).unwrap().role,
                NodeRole::Breakdown,
                "{label} should be Breakdown"
            );
        }

        // composition nests under prep phase as a reconciling Structural child
        // (RC-2), and prep gains its own remainder.
        let prep = child(&tree, "prep phase").unwrap();
        let composition = child(prep, "composition").expect("composition under prep");
        assert_eq!(composition.role, NodeRole::Structural);
        assert_eq!(composition.total, Duration::from_millis(300));
        assert!(
            child(prep, "unattributed").is_some(),
            "prep needs unattributed"
        );

        // DM-2: stages are grouped under their ComposePhase; both stages here
        // are Inline-Pre, so they nest one level below `composition`.
        let inline_pre = child(composition, "inline pre").expect("inline pre phase group");
        assert_eq!(inline_pre.role, NodeRole::Breakdown);

        // DM-1: `calls` is carried on Breakdown stage leaves only when > 1.
        let shell = child(inline_pre, "shell expansion").expect("shell expansion stage");
        assert_eq!(shell.role, NodeRole::Breakdown);
        assert_eq!(shell.calls, Some(3));
        let interp = child(inline_pre, "interpolation").expect("interpolation stage");
        assert_eq!(interp.calls, None, "calls omitted when == 1");

        // environment setup substages are a Structural carve of the parent
        // window (TR-2 option a, Phase 4).
        let env = child(&tree, "environment setup").unwrap();
        assert_eq!(
            child(env, "target resolution").unwrap().role,
            NodeRole::Structural
        );
        assert!(
            child(env, "unattributed").is_some(),
            "env setup needs a remainder once substages are Structural"
        );
    }

    // --- Phase 7 / Milestone B: darkmatter enrichment (DM-2…DM-4) -----------

    /// DM-2: stages from all four phases nest under their phase group, and the
    /// compose subtree still reconciles (enrichment nodes are `Breakdown`, so
    /// they never enter the reconciliation sum — TR-1 holds on `composition`'s
    /// authoritative total).
    #[test]
    fn composition_groups_stages_by_phase() {
        use darkmatter::markdown::compose::{ComposePerfMetric, ComposePerfReport, ComposeStage};

        let compose = ComposePerfReport {
            total: Duration::from_millis(400),
            metrics: vec![
                ComposePerfMetric {
                    stage: ComposeStage::Interpolation, // InlinePre
                    elapsed: Duration::from_millis(20),
                    calls: 1,
                },
                ComposePerfMetric {
                    stage: ComposeStage::TransclusionApply, // Transclusion
                    elapsed: Duration::from_millis(40),
                    calls: 1,
                },
                ComposePerfMetric {
                    stage: ComposeStage::Cleanup, // InlinePost
                    elapsed: Duration::from_millis(5),
                    calls: 1,
                },
                ComposePerfMetric {
                    stage: ComposeStage::LinkNormalization, // Finalization
                    elapsed: Duration::from_millis(3),
                    calls: 1,
                },
            ],
            ..Default::default()
        };
        let report = perf_report(
            Duration::from_secs(2),
            Duration::from_millis(10),
            Duration::from_millis(500),
            Duration::from_millis(100),
            vec![],
            Some(compose),
            None,
        );
        let tree = build_perf_tree(&report, CompositionPlacement::UnderPrep);
        let prep = child(&tree, "prep phase").unwrap();
        let composition = child(prep, "composition").unwrap();

        let inline_pre = child(composition, "inline pre").expect("inline pre group");
        assert!(child(inline_pre, "interpolation").is_some());
        assert!(child(composition, "transclusion").is_some());
        assert!(child(composition, "inline post").is_some());
        assert!(child(composition, "finalization").is_some());
        // The phase node sums its stage children.
        assert_eq!(inline_pre.total, Duration::from_millis(20));

        assert!(tree_reconciles(&tree, Duration::from_millis(1)));
    }

    /// DM-3: a per-`::shell` span is rendered as a child of the shell-expansion
    /// stage, and the dominant span — not the aggregate stage — is flagged HOT.
    #[test]
    fn composition_shell_span_becomes_hot_leaf() {
        use darkmatter::markdown::compose::{
            ComposePerfMetric, ComposePerfReport, ComposeStage, ShellCommandSpan,
        };

        let compose = ComposePerfReport {
            total: Duration::from_millis(980),
            metrics: vec![ComposePerfMetric {
                stage: ComposeStage::ShellExpansion,
                elapsed: Duration::from_millis(970),
                calls: 2,
            }],
            shell_spans: vec![
                ShellCommandSpan {
                    command_display: "curl https://example.com".into(),
                    command_hash: "abc123".into(),
                    elapsed: Duration::from_millis(960),
                },
                ShellCommandSpan {
                    command_display: "date".into(),
                    command_hash: "def456".into(),
                    elapsed: Duration::from_millis(10),
                },
            ],
            ..Default::default()
        };
        let report = perf_report(
            Duration::from_millis(1600),
            Duration::from_millis(10),
            Duration::from_millis(1000),
            Duration::from_millis(50),
            vec![],
            Some(compose),
            None,
        );

        // Tree shape: the span is a child of the shell-expansion stage.
        let tree = build_perf_tree(&report, CompositionPlacement::UnderPrep);
        let prep = child(&tree, "prep phase").unwrap();
        let composition = child(prep, "composition").unwrap();
        let inline_pre = child(composition, "inline pre").unwrap();
        let shell = child(inline_pre, "shell expansion").unwrap();
        assert!(
            child(shell, "shell · curl https://example.com").is_some(),
            "dominant directive must appear as a span leaf"
        );

        // Rendering: the HOT marker lands on the dominant span, not the stage.
        let plain = strip_ansi(&render_perf_report(&report));
        let hot_line = plain
            .lines()
            .find(|l| l.contains("▇ HOT"))
            .unwrap_or_else(|| panic!("missing HOT marker; got:\n{plain}"));
        assert!(
            hot_line.contains("shell · curl"),
            "HOT must flag the dominant directive; got: {hot_line:?}"
        );
    }

    /// DM-4 (OQ-3 Option C): context-capture timings attach under `prep phase`
    /// — not `composition` — because the context is captured before the metered
    /// compose window starts, so the time is prep time, not composition time.
    /// It is a `Breakdown` node: per-group timings are captured concurrently, so
    /// their sum overstates the wall-clock and must not enter reconciliation.
    #[test]
    fn context_capture_attaches_under_prep_not_composition() {
        use darkmatter::markdown::compose::{ComposePerfMetric, ComposePerfReport, ComposeStage};

        // Capture timings are deliberately inflated so their sum (900ms) far
        // exceeds the prep window (500ms): groups are captured concurrently, so
        // the sum overstates the wall-clock. A `Structural` role would overflow
        // prep and fail TR-1 — the exact flake this `Breakdown` role prevents.
        let compose = ComposePerfReport {
            total: Duration::from_millis(300),
            metrics: vec![ComposePerfMetric {
                stage: ComposeStage::Interpolation,
                elapsed: Duration::from_millis(20),
                calls: 1,
            }],
            capture_timings: vec![
                ("git".to_string(), Duration::from_millis(300)),
                ("hardware".to_string(), Duration::from_millis(300)),
                ("repo".to_string(), Duration::from_millis(300)),
            ],
            ..Default::default()
        };
        let report = perf_report(
            Duration::from_secs(2),
            Duration::from_millis(10),
            Duration::from_millis(500),
            Duration::from_millis(100),
            vec![],
            Some(compose),
            None,
        );
        let tree = build_perf_tree(&report, CompositionPlacement::UnderPrep);
        let prep = child(&tree, "prep phase").unwrap();

        // Capture is a Breakdown child of prep (displayed, not reconciled).
        let capture = child(prep, "context capture").expect("context capture under prep");
        assert_eq!(capture.role, NodeRole::Breakdown);
        assert_eq!(capture.total, Duration::from_millis(900));
        assert!(child(capture, "git").is_some());
        assert!(child(capture, "repo").is_some());

        // It must NOT appear under composition anymore.
        let composition = child(prep, "composition").unwrap();
        assert!(
            child(composition, "context capture").is_none(),
            "capture must not be nested under composition"
        );

        // The whole tree still reconciles even though the capture sum dwarfs
        // the prep window — because capture is excluded from reconciliation.
        assert!(tree_reconciles(&tree, Duration::from_millis(1)));
    }

    #[test]
    fn perf_tree_reconciles_compose() {
        let report = perf_report(
            Duration::from_secs(2),
            Duration::from_millis(10),
            Duration::from_millis(500),
            Duration::from_millis(100),
            vec![],
            Some(compose_report()),
            None,
        );
        let tree = build_perf_tree(&report, CompositionPlacement::UnderPrep);
        assert!(tree_reconciles(&tree, Duration::from_millis(1)));
    }

    #[test]
    fn perf_tree_reconciles_wrapper() {
        // Wrapper: no composition, prep phase unstamped, agent ran.
        let report = perf_report(
            Duration::from_secs(3),
            Duration::from_millis(20),
            Duration::ZERO,
            Duration::from_millis(150),
            vec![SubstageTiming::new(
                "system prompt",
                Duration::from_millis(64),
            )],
            None,
            Some(AgentExecutionPerf {
                launches: 1,
                total_elapsed: Duration::from_millis(2500),
                first_response_latency: Some(Duration::from_millis(900)),
                provider_api_duration: Some(Duration::from_millis(2300)),
            }),
        );
        let tree = build_perf_tree(&report, CompositionPlacement::UnderPrep);
        assert!(tree_reconciles(&tree, Duration::from_millis(1)));
    }

    #[test]
    fn perf_tree_dry_run_headline_equals_sum_of_structural_plus_remainder() {
        // The motivating shape, re-grounded on true wall-clock: the headline
        // is the real elapsed (~1.6s), so it MUST equal Σ top-level Structural
        // plus the remainder — the 78.6ms-vs-1.57s contradiction is impossible.
        let report = perf_report(
            Duration::from_millis(1600),
            Duration::from_millis(7),
            Duration::from_millis(1500),
            Duration::from_millis(65),
            vec![],
            Some(darkmatter::markdown::compose::ComposePerfReport {
                total: Duration::from_micros(970_500),
                metrics: vec![darkmatter::markdown::compose::ComposePerfMetric {
                    stage: darkmatter::markdown::compose::ComposeStage::ShellExpansion,
                    elapsed: Duration::from_micros(970_500),
                    calls: 1,
                }],
                ..Default::default()
            }),
            None,
        );
        let tree = build_perf_tree(&report, CompositionPlacement::UnderPrep);
        assert!(tree_reconciles(&tree, Duration::from_millis(1)));

        let structural_sum: Duration = tree
            .children
            .iter()
            .filter(|c| c.role == NodeRole::Structural)
            .map(|c| c.total)
            .sum();
        let remainder = child(&tree, "unattributed").unwrap().total;
        assert_eq!(structural_sum + remainder, tree.total);
        // remainder = 1600 − (7 + 1500 + 65) = 28ms.
        assert_eq!(remainder, Duration::from_millis(28));
    }

    /// Build a sequence `CommandPerfReport` with the given per-step data and
    /// top-level windows. Sequence reports carry no merged `composition` /
    /// aggregated `agent` node — execution is rendered per step (TM-3).
    fn sequence_report(
        total_elapsed: Duration,
        pre_dispatch: Duration,
        environment_setup: Duration,
        steps: Vec<SequenceStepPerf>,
    ) -> CommandPerfReport {
        CommandPerfReport {
            title: "Sequence",
            total_elapsed,
            cli: CliOverheadReport {
                arg_parsing: Duration::from_millis(4),
                config_loading: Duration::from_millis(3),
                tracing_init: Duration::from_millis(1),
                pre_dispatch,
                prep_phase: Duration::ZERO,
                environment_setup,
                substages: vec![],
                prep_substages: vec![],
            },
            composition: None,
            agent: None,
            notes: vec![],
            placement: CompositionPlacement::UnderEnvSetup,
            sequence_steps: steps,
        }
    }

    fn step_perf(
        index: usize,
        name: &str,
        wall_clock: Duration,
        compose: Option<darkmatter::markdown::compose::ComposePerfReport>,
        agent: Option<AgentExecutionPerf>,
    ) -> SequenceStepPerf {
        SequenceStepPerf {
            step_index: index,
            step_name: name.to_string(),
            wall_clock,
            compose_perf: compose,
            agent_perf: agent,
        }
    }

    /// TM-3: a sequence renders a `steps` Structural node whose per-step
    /// children carry the step's execution wall-clock and reconcile to the
    /// headline, leaving orchestration in the root remainder. Each step nests
    /// only the `agent` execution detail as `Breakdown` (`steps → step N →
    /// agent`); per-step composition is rendered under `environment setup → step
    /// preparation`, the window that actually metered it.
    #[test]
    fn perf_tree_sequence_builds_per_step_subtrees() {
        let compose = darkmatter::markdown::compose::ComposePerfReport {
            total: Duration::from_millis(40),
            metrics: vec![darkmatter::markdown::compose::ComposePerfMetric {
                stage: darkmatter::markdown::compose::ComposeStage::LinkResolve,
                elapsed: Duration::from_millis(5),
                calls: 1,
            }],
            ..Default::default()
        };
        let agent = AgentExecutionPerf {
            launches: 1,
            total_elapsed: Duration::from_millis(900),
            first_response_latency: Some(Duration::from_millis(300)),
            provider_api_duration: Some(Duration::from_millis(700)),
        };
        // env setup (100ms) contains both steps' compose work (Σ = 80ms), as it
        // does in a real run.
        let report = sequence_report(
            Duration::from_secs(2),
            Duration::from_millis(10),
            Duration::from_millis(100),
            vec![
                step_perf(
                    0,
                    "alpha",
                    Duration::from_millis(950),
                    Some(compose.clone()),
                    Some(agent),
                ),
                step_perf(
                    1,
                    "beta",
                    Duration::from_millis(800),
                    Some(compose),
                    Some(agent),
                ),
            ],
        );

        let tree = build_perf_tree(&report, CompositionPlacement::UnderEnvSetup);
        assert!(tree_reconciles(&tree, Duration::from_millis(1)));

        // No top-level merged composition or aggregate agent node.
        assert!(child(&tree, "composition").is_none());
        assert!(child(&tree, "agent execution").is_none());
        let prep = child(&tree, "prep phase").unwrap();
        assert!(child(prep, "composition").is_none());

        // `steps` is a Structural node summing the per-step wall-clocks.
        let steps = child(&tree, "steps").expect("steps node");
        assert_eq!(steps.role, NodeRole::Structural);
        assert_eq!(steps.total, Duration::from_millis(1750));

        // step N → agent (Breakdown); composition is NOT nested here.
        let alpha = child(steps, "step 1: alpha").expect("step 1 subtree");
        assert_eq!(alpha.role, NodeRole::Structural);
        assert_eq!(alpha.total, Duration::from_millis(950));
        assert!(
            child(alpha, "composition").is_none(),
            "composition must not nest under the execution window"
        );
        assert_eq!(child(alpha, "agent").unwrap().role, NodeRole::Breakdown);
        assert!(child(steps, "step 2: beta").is_some());

        // Per-step composition lives under environment setup → step preparation.
        let env = child(&tree, "environment setup").expect("env setup");
        let step_prep = child(env, "step preparation").expect("step preparation node");
        assert_eq!(step_prep.role, NodeRole::Breakdown);
        assert_eq!(step_prep.total, Duration::from_millis(80));
        let prep_alpha = child(step_prep, "step 1: alpha").expect("prepared step 1");
        assert_eq!(prep_alpha.role, NodeRole::Breakdown);
        assert_eq!(prep_alpha.total, Duration::from_millis(40));
        assert!(child(step_prep, "step 2: beta").is_some());
    }

    /// Walk the whole tree asserting the G-2 timeline contract: no child's total
    /// exceeds its parent's (within a 1ms formatting tolerance). The
    /// reconciliation walker only checks Structural sums; this also catches a
    /// `Breakdown` child rendered larger than the parent it itemizes — the
    /// review-2 sequence regression.
    fn assert_no_child_exceeds_parent(node: &PerfNode) {
        for c in &node.children {
            assert!(
                c.total <= node.total + Duration::from_millis(1),
                "child {:?} ({:?}) exceeds parent {:?} ({:?})",
                c.label,
                c.total,
                node.label,
                node.total,
            );
            assert_no_child_exceeds_parent(c);
        }
    }

    /// Regression (review-2): a slow per-step composition must never exceed its
    /// displayed parent. Composition is metered during environment setup, not
    /// the per-step execution window, so nesting it under `steps → step N`
    /// (whose total is the tiny execution wall-clock) let a 900ms compose render
    /// beneath a 5ms step — child larger than parent. It now attaches under
    /// `environment setup → step preparation`, whose window genuinely contains
    /// it, so every child fits its parent (G-2).
    #[test]
    fn perf_tree_sequence_slow_compose_never_exceeds_parent() {
        // The pathological shape: compose dominates (900ms) while execution is
        // trivial (5ms) — a dry-run or cache-fast agent after a slow shell
        // expansion.
        let slow_compose = darkmatter::markdown::compose::ComposePerfReport {
            total: Duration::from_millis(900),
            metrics: vec![darkmatter::markdown::compose::ComposePerfMetric {
                stage: darkmatter::markdown::compose::ComposeStage::ShellExpansion,
                elapsed: Duration::from_millis(900),
                calls: 1,
            }],
            ..Default::default()
        };
        // env setup (950ms) contains the Phase-1 compose work, so it is ≥ the
        // 900ms compose, exactly as in a real run.
        let report = sequence_report(
            Duration::from_millis(1000),
            Duration::from_millis(10),
            Duration::from_millis(950),
            vec![step_perf(
                0,
                "alpha",
                Duration::from_millis(5),
                Some(slow_compose),
                None,
            )],
        );
        let tree = build_perf_tree(&report, CompositionPlacement::UnderEnvSetup);
        assert!(tree_reconciles(&tree, Duration::from_millis(1)));

        // The step's execution node carries only the 5ms wall-clock — no
        // composition child it could be dwarfed by.
        let steps = child(&tree, "steps").expect("steps node");
        let alpha = child(steps, "step 1: alpha").expect("step 1 execution node");
        assert_eq!(alpha.total, Duration::from_millis(5));
        assert!(
            child(alpha, "composition").is_none(),
            "composition must not nest under the execution window it would exceed"
        );

        // It attaches under environment setup → step preparation, whose window
        // contains it (900ms ≤ 950ms env setup).
        let env = child(&tree, "environment setup").expect("env setup");
        let step_prep = child(env, "step preparation").expect("step preparation node");
        let prep_alpha = child(step_prep, "step 1: alpha").expect("prepared step 1");
        assert_eq!(prep_alpha.total, Duration::from_millis(900));

        // The core invariant: no node's child exceeds it anywhere in the tree.
        assert_no_child_exceeds_parent(&tree);
    }

    /// TM-3 reconciliation across a partial sequence: only the completed step's
    /// wall-clock contributes; interrupted work falls to the root remainder, and
    /// the tree still reconciles.
    #[test]
    fn perf_tree_sequence_partial_reconciles() {
        let report = sequence_report(
            Duration::from_secs(1),
            Duration::from_millis(10),
            Duration::from_millis(50),
            vec![step_perf(
                0,
                "alpha",
                Duration::from_millis(400),
                None,
                None,
            )],
        );
        let tree = build_perf_tree(&report, CompositionPlacement::UnderEnvSetup);
        assert!(tree_reconciles(&tree, Duration::from_millis(1)));

        let steps = child(&tree, "steps").expect("steps node");
        assert_eq!(steps.total, Duration::from_millis(400));
        // Orchestration + interrupted work lands in the root remainder:
        // 1000 − (10 + 50 + 400) = 540ms.
        assert_eq!(
            child(&tree, "unattributed").unwrap().total,
            Duration::from_millis(540)
        );
    }

    #[test]
    fn perf_tree_detects_structural_overflow() {
        // The exact pre-fix bug: a tiny post-prep headline (78.6ms) over a
        // body whose Structural buckets sum to 1.57s. Reconciliation must
        // REJECT this — the remainder clamps to zero and the sum overshoots.
        let report = perf_report(
            Duration::from_micros(78_600),
            Duration::from_millis(7),
            Duration::from_millis(1500),
            Duration::from_millis(65),
            vec![],
            None,
            None,
        );
        let tree = build_perf_tree(&report, CompositionPlacement::UnderPrep);
        assert!(
            !tree_reconciles(&tree, Duration::from_millis(1)),
            "structural buckets overflowing wall-clock must fail TR-1"
        );
    }

    // --- Phase 4: same-clock env setup + named prep children ----------------

    #[test]
    fn perf_tree_env_setup_substages_carve_parent() {
        // TR-2 option a: substages are Structural and sum to ≤ the parent
        // window; the remainder absorbs the µs head/tail gap. Here substages
        // total 90ms inside a 100ms window → 10ms remainder.
        let report = perf_report(
            Duration::from_secs(1),
            Duration::from_millis(10),
            Duration::ZERO,
            Duration::from_millis(100),
            vec![
                SubstageTiming::new("target resolution", Duration::from_millis(30)),
                SubstageTiming::new("mcp composition", Duration::ZERO),
                SubstageTiming::new("system prompt", Duration::from_millis(60)),
            ],
            None,
            None,
        );
        let tree = build_perf_tree(&report, CompositionPlacement::UnderPrep);
        assert!(tree_reconciles(&tree, Duration::from_millis(1)));

        let env = child(&tree, "environment setup").unwrap();
        for label in ["target resolution", "mcp composition", "system prompt"] {
            assert_eq!(
                child(env, label).unwrap().role,
                NodeRole::Structural,
                "{label} should be Structural"
            );
        }
        // The zero-duration substage stays representable as a Structural leaf.
        assert_eq!(child(env, "mcp composition").unwrap().total, Duration::ZERO);
        assert_eq!(
            child(env, "unattributed").unwrap().total,
            Duration::from_millis(10)
        );
    }

    #[test]
    fn perf_tree_child_env_build_breakdown_nests_without_breaking_reconciliation() {
        // `child env build` carries a nested breakdown (env sanitize / shadow
        // home sync → repo root detect). The substage stays Structural and
        // reconciles against `environment setup`; its whole breakdown subtree is
        // Breakdown at every depth, so a near-100% `repo root detect` child can
        // never make the substage exceed its parent (TR-1).
        let substages = vec![
            SubstageTiming::new("target resolution", Duration::from_millis(2)),
            SubstageTiming {
                name: "child env build",
                elapsed: Duration::from_millis(700),
                children: vec![
                    SubstageTiming::new("env sanitize", Duration::from_micros(300)),
                    SubstageTiming {
                        name: "shadow home sync",
                        elapsed: Duration::from_millis(699),
                        children: vec![SubstageTiming::new(
                            "repo root detect",
                            Duration::from_millis(698),
                        )],
                    },
                ],
            },
        ];
        let report = perf_report(
            Duration::from_secs(1),
            Duration::from_millis(10),
            Duration::ZERO,
            Duration::from_millis(710),
            substages,
            None,
            None,
        );

        let tree = build_perf_tree(&report, CompositionPlacement::UnderPrep);
        assert!(
            tree_reconciles(&tree, Duration::from_millis(1)),
            "child env build breakdown must not break TR-1"
        );

        let env = child(&tree, "environment setup").unwrap();
        let ceb = child(env, "child env build").unwrap();
        assert_eq!(ceb.role, NodeRole::Structural);

        let shadow = child(ceb, "shadow home sync").unwrap();
        assert_eq!(shadow.role, NodeRole::Breakdown);
        let detect = child(shadow, "repo root detect").unwrap();
        assert_eq!(detect.role, NodeRole::Breakdown);
        assert_eq!(detect.total, Duration::from_millis(698));
    }

    #[test]
    fn perf_tree_prep_named_children_reconcile() {
        // P-5a: named prep work units are Structural children carving the prep
        // window. `shell approval` is the dominant leaf (the un-metered shell
        // preflight pass); `composition` (the metered prepare pass) nests
        // alongside them. All disjoint subsets → Σ ≤ prep_phase, remainder
        // absorbs the rest.
        let mut report = perf_report(
            Duration::from_secs(2),
            Duration::from_millis(10),
            Duration::from_millis(1680),
            Duration::from_millis(70),
            vec![],
            Some(darkmatter::markdown::compose::ComposePerfReport {
                total: Duration::from_micros(5_900),
                metrics: vec![],
                ..Default::default()
            }),
            None,
        );
        report.cli.prep_substages = vec![
            SubstageTiming::new("frontmatter load", Duration::from_millis(3)),
            SubstageTiming::new("schema validation", Duration::from_millis(2)),
            SubstageTiming::new("prep context", Duration::from_millis(8)),
            SubstageTiming::new("shell approval", Duration::from_millis(1600)),
        ];

        let tree = build_perf_tree(&report, CompositionPlacement::UnderPrep);
        assert!(tree_reconciles(&tree, Duration::from_millis(1)));

        let prep = child(&tree, "prep phase").unwrap();
        for label in [
            "frontmatter load",
            "schema validation",
            "prep context",
            "shell approval",
            "composition",
        ] {
            assert_eq!(
                child(prep, label)
                    .unwrap_or_else(|| panic!("missing prep child {label}"))
                    .role,
                NodeRole::Structural,
                "{label} should be Structural under prep"
            );
        }
        // remainder = 1680 − (3 + 2 + 8 + 1600 + 5.9) = 61.1ms.
        assert_eq!(
            child(prep, "unattributed").unwrap().total,
            Duration::from_micros(61_100)
        );
    }
}
