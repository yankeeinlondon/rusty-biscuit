//! Performance-tree assembly and reconciliation.
//!
//! Turns a built [`CommandPerfReport`] into the single reconciling
//! [`PerfNode`] tree (TM-2) that the renderer walks. The reconciliation
//! invariant (TR-1 / TR-3 / TR-4) is enforced here so the rendered tree's
//! headline can never silently disagree with the sum of its structural
//! buckets.

use std::time::Duration;

use super::{CommandPerfReport, CompositionPlacement, NodeRole, PerfNode, SequenceStepPerf, SequenceTaskPerf, SubstageTiming};

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
pub(super) fn build_perf_tree(
    report: &CommandPerfReport,
    placement: CompositionPlacement,
) -> PerfNode {
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
        prep.children
            .push(PerfNode::leaf(unit.name, unit.elapsed, NodeRole::Structural));
    }
    if placement == CompositionPlacement::UnderPrep
        && let Some((total, children)) = &composition
    {
        // Context capture happened during prep, before the metered compose
        // window — attach it under `prep phase`, not `composition` (OQ-3
        // Option C). It is a `Breakdown` node (parallel per-group timings whose
        // sum overstates the wall-clock), so it does not enter reconciliation.
        if let Some(capture) = report.composition.as_ref().and_then(build_context_capture_node) {
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
    let env_setup_node = PerfNode::branch(
        "environment setup",
        cli.environment_setup,
        NodeRole::Structural,
        env_children,
    );
    let env_setup = finalize_reconciling(env_setup_node);

    let mut top = vec![pre_dispatch, prep, env_setup];

    if is_sequence {
        // Sequence: a `steps` Structural node whose per-step children carry the
        // step's execution wall-clock and reconcile to the sequence headline,
        // leaving inter-step orchestration in the root remainder (TM-3). Per-step
        // composition nests under its own step, because that is where it now
        // runs; the merged `composition` / aggregated `agent` views on the report
        // are not emitted as separate nodes.
        top.push(build_steps_node(&report.sequence_steps));
    } else if let Some(agent) = &report.agent {
        // agent execution — Structural when the agent ran (omitted on dry runs;
        // the P-5 `—` leaf is a Phase 5 rendering concern).
        let mut children = Vec::new();
        if let Some(latency) = agent.first_response_latency {
            children.push(PerfNode::leaf("first response", latency, NodeRole::Breakdown));
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
/// already counts.
///
/// Per-step composition nests here too, as a `composition` Breakdown child. Under
/// just-in-time orchestration a step is composed *at its turn*, inside the window
/// `wall_clock` measures, so the nesting is arithmetically sound: the G-2 hazard
/// — a compose rendered larger than the node it hangs under — only existed while
/// composition ran up front, outside every step's window.
fn build_steps_node(steps: &[SequenceStepPerf]) -> PerfNode {
    let total: Duration = steps.iter().map(|s| s.wall_clock).sum();
    let children = steps
        .iter()
        .map(|step| {
            let mut step_children = Vec::new();
            if let Some(compose) = &step.compose_perf {
                step_children.push(PerfNode::branch(
                    "composition",
                    compose.total,
                    NodeRole::Breakdown,
                    build_composition_children(compose),
                ));
            }
            if let Some(agent) = &step.agent_perf {
                let mut agent_children = Vec::new();
                if let Some(latency) = agent.first_response_latency {
                    agent_children
                        .push(PerfNode::leaf("first response", latency, NodeRole::Breakdown));
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
            if !step.group_tasks.is_empty() {
                step_children.push(build_group_node(&step.group_tasks));
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

/// Build the `group` node holding a step's per-task timings.
///
/// Everything here is `Breakdown` — including the `group` node itself — because
/// a parallel group's members overlap. Their durations sum past the step's own
/// wall-clock whenever concurrency did its job, so they are attribution detail
/// rather than a partition the reconciler can check. The node's own total is the
/// longest member: the shortest the group could possibly have taken.
fn build_group_node(tasks: &[SequenceTaskPerf]) -> PerfNode {
    let longest = tasks
        .iter()
        .map(|task| task.duration)
        .max()
        .unwrap_or_default();
    PerfNode::branch(
        "group",
        longest,
        NodeRole::Breakdown,
        tasks
            .iter()
            .map(|task| PerfNode::leaf(task.name.clone(), task.duration, NodeRole::Breakdown))
            .collect(),
    )
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
    let has_structural = node
        .children
        .iter()
        .any(|c| c.role == NodeRole::Structural);
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
    node.children
        .push(PerfNode::leaf("unattributed", remainder, NodeRole::Unattributed));
    node
}

/// Walk the tree asserting the TR-1 reconciliation invariant at every node
/// (TR-4 walker). Returns `false` on the first node whose `Structural`
/// children plus `Unattributed` remainder drift from its `total` beyond
/// `tolerance` — the `78.6ms`-headline-vs-`1.57s`-body bug class.
pub(super) fn tree_reconciles(node: &PerfNode, tolerance: Duration) -> bool {
    node_reconciles(node, tolerance)
        && node.children.iter().all(|c| tree_reconciles(c, tolerance))
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
pub(super) fn debug_assert_reconciles(report: &CommandPerfReport) {
    debug_assert!(
        tree_reconciles(
            &build_perf_tree(report, report.placement),
            Duration::from_millis(1)
        ),
        "perf report failed TR-1 reconciliation: headline {:?} disagrees with Σ structural buckets",
        report.total_elapsed,
    );
}
