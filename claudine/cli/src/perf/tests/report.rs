use super::*;
use std::time::Duration;

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
    assert!(rendered.contains("├─"), "missing tree connectors: {rendered}");
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
        group_tasks: Vec::new(),
    });
    acc.add_step(SequenceStepPerf {
        step_index: 1,
        step_name: "step2".into(),
        wall_clock: Duration::from_millis(250),
        compose_perf: Some(compose2),
        agent_perf: None,
        group_tasks: Vec::new(),
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
        group_tasks: Vec::new(),
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
        group_tasks: Vec::new(),
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
fn command_perf_collector_renders_request_owned_discovery_counts() {
    let startup = StartupTimings {
        arg_parsing: Duration::ZERO,
        tracing_init: Duration::ZERO,
        config_loading: Duration::ZERO,
        pre_dispatch: Duration::ZERO,
        prep_phase: Duration::ZERO,
        process_start: std::time::Instant::now(),
        prep_substages: Vec::new(),
    };
    let work = claudine::invocation_context::InvocationWorkSnapshot {
        git_root_discoveries: 1,
        topology_probes: 2,
        topology_reuses: 3,
        ..Default::default()
    };
    let mut collector = CommandPerfCollector::new("Test", startup);
    collector.set_invocation_work(&work);
    collector.mark_env_setup_complete();

    let report = collector.into_report_with_elapsed(Duration::from_secs(1));
    let note = report.notes.join("\n");
    assert!(note.contains("Git discoveries 1"), "{note}");
    assert!(note.contains("topology probes 2"), "{note}");
    assert!(note.contains("topology reuses 3"), "{note}");

    let rendered = strip_ansi(&render_perf_report(&report));
    assert!(rendered.contains("source context work"), "{rendered}");
    assert!(rendered.contains("topology probes 2"), "{rendered}");
    assert!(rendered.contains("topology reuses 3"), "{rendered}");
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
