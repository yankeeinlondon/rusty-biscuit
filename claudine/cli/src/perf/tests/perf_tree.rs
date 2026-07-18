use super::*;
use std::time::Duration;

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
    for label in ["pre-dispatch", "prep phase", "environment setup", "agent execution"] {
        let node = child(&tree, label).unwrap_or_else(|| panic!("missing {label}"));
        assert_eq!(node.role, NodeRole::Structural, "{label} should be Structural");
    }
    // Root reconciles with a synthetic remainder.
    assert!(child(&tree, "unattributed").is_some(), "root needs unattributed");

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
    assert!(child(prep, "unattributed").is_some(), "prep needs unattributed");

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
        vec![SubstageTiming::new("system prompt", Duration::from_millis(64))],
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
        placement: CompositionPlacement::UnderStep,
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
            step_perf(1, "beta", Duration::from_millis(800), Some(compose), Some(agent)),
        ],
    );

    let tree = build_perf_tree(&report, CompositionPlacement::UnderStep);
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

    // step N → composition + agent, both Breakdown detail on the step's own
    // window: just-in-time orchestration composes the step inside it.
    let alpha = child(steps, "step 1: alpha").expect("step 1 subtree");
    assert_eq!(alpha.role, NodeRole::Structural);
    assert_eq!(alpha.total, Duration::from_millis(950));
    let alpha_compose = child(alpha, "composition").expect("step 1 composition");
    assert_eq!(alpha_compose.role, NodeRole::Breakdown);
    assert_eq!(alpha_compose.total, Duration::from_millis(40));
    assert_eq!(child(alpha, "agent").unwrap().role, NodeRole::Breakdown);
    assert!(child(steps, "step 2: beta").is_some());

    // Nothing hangs off environment setup any more: it no longer contains
    // per-step composition.
    let env = child(&tree, "environment setup").expect("env setup");
    assert!(child(env, "step preparation").is_none());
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
/// displayed parent (G-2).
///
/// The shape that used to break this — a 900ms compose under a 5ms step — is
/// unreachable once composition happens *at the step's turn*: the step's
/// wall-clock brackets its own compose, so a compose-dominated step is simply a
/// long step. This asserts that on the same pathological ratio.
#[test]
fn perf_tree_sequence_slow_compose_never_exceeds_parent() {
    // Compose dominates (900ms) while execution is trivial — a dry-run or
    // cache-fast agent after a slow shell expansion.
    let slow_compose = darkmatter::markdown::compose::ComposePerfReport {
        total: Duration::from_millis(900),
        metrics: vec![darkmatter::markdown::compose::ComposePerfMetric {
            stage: darkmatter::markdown::compose::ComposeStage::ShellExpansion,
            elapsed: Duration::from_millis(900),
            calls: 1,
        }],
        ..Default::default()
    };
    // The step's wall-clock (905ms) brackets its own compose (900ms), exactly
    // as in a real just-in-time run; env setup no longer holds compose work.
    let report = sequence_report(
        Duration::from_millis(1000),
        Duration::from_millis(10),
        Duration::from_millis(50),
        vec![step_perf(
            0,
            "alpha",
            Duration::from_millis(905),
            Some(slow_compose),
            None,
        )],
    );
    let tree = build_perf_tree(&report, CompositionPlacement::UnderStep);
    assert!(tree_reconciles(&tree, Duration::from_millis(1)));

    // The compose nests under the step whose window contains it.
    let steps = child(&tree, "steps").expect("steps node");
    let alpha = child(steps, "step 1: alpha").expect("step 1 execution node");
    assert_eq!(alpha.total, Duration::from_millis(905));
    assert_eq!(
        child(alpha, "composition").expect("step composition").total,
        Duration::from_millis(900)
    );

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
        vec![step_perf(0, "alpha", Duration::from_millis(400), None, None)],
    );
    let tree = build_perf_tree(&report, CompositionPlacement::UnderStep);
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
