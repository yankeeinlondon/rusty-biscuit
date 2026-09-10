//! seed state loop-engine tests.

use super::*;
use super::super::super::seed::build_loop_seed_with_lifecycle;
use crate::composition::prepare::prepare_direct;

#[test]
fn build_loop_seed_resolves_control_variables_and_omits_derived() {
    let source = make_source(&[
        ("phase", json!("{{ initial_phase || 1 }}")),
        ("total_phases", json!("{{ 6 }}")),
        (
            "pass_icon",
            json!("{{ _loop_is_last ? '✅' : '🧑‍💻' }}"),
        ),
        (
            "loop",
            json!({"until": "phase > total_phases", "action": "increment(phase)"}),
        ),
    ]);
    let config = resolve_loop_config(&source).unwrap().unwrap();
    let options = PrepareOptions {
        set_overrides: Some(json!({"initial_phase": 1})),
        ..PrepareOptions::default()
    };

    let seed =
        build_loop_seed(&source, &config, options, CompositionMode::ChainedDocument).unwrap();

    assert_eq!(seed.get("phase"), Some(&json!(1)));
    assert_eq!(seed.get("total_phases"), Some(&json!(6)));
    assert!(!seed.contains_key("pass_icon"), "derived keys must not be lifted into the seed");
    assert_eq!(seed.get("initial_phase"), Some(&json!(1)));
}

/// The control-variable-only seed drops every lifecycle event block, so
/// parsing lifecycle from it yields an empty config — the root cause of the
/// loop-path lifecycle bug. `build_loop_seed_with_lifecycle` instead parses
/// lifecycle from the full composed frontmatter, so the event blocks and
/// the `loop:` gate concerns survive even though they are absent from the
/// seed.
#[test]
fn build_loop_seed_with_lifecycle_carries_event_blocks_dropped_from_seed() {
    let source = make_source(&[
        ("phase", json!(1)),
        (
            "loop",
            json!({
                "until": "phase > 2",
                "action": "increment(phase)",
                "stack": [{"action": {"append_line": ["events.log", "gate"]}}],
            }),
        ),
        ("initialize", json!({"stack": [{"action": {"append_line": ["events.log", "initialize"]}}]})),
        ("start", json!({"stack": [{"action": {"append_line": ["events.log", "start"]}}]})),
        ("finalize", json!({"stack": [{"action": {"append_line": ["events.log", "finalize"]}}]})),
    ]);
    let config = resolve_loop_config(&source).unwrap().unwrap();

    let result = build_loop_seed_with_lifecycle(
        &source,
        &config,
        PrepareOptions::default(),
        CompositionMode::ChainedDocument,
    )
    .unwrap();

    // The seed itself carries only control variables, never event blocks.
    assert!(!result.seed.contains_key("initialize"));
    assert!(!result.seed.contains_key("finalize"));

    // The parsed lifecycle, however, carries every event block plus the
    // `loop:` gate concerns — exactly what the loop runner needs.
    assert!(!result.lifecycle.is_empty(), "lifecycle must not be empty");
    assert!(result.lifecycle.stacks.initialize.is_some());
    assert!(result.lifecycle.stacks.start.is_some());
    assert!(result.lifecycle.stacks.finalize.is_some());
    assert!(
        result.lifecycle.stacks.loop_gate.is_some(),
        "the loop gate's lifecycle concerns must survive into the parsed config"
    );
}

#[test]
fn build_loop_seed_inline_mode_resolves_prompt_frontmatter_with_empty_body() {
    let source = make_source_with_body(
        &[
            ("prompt", json!("Build phase {{phase}}")),
            ("phase", json!("{{ start_phase || 1 }}")),
            (
                "loop",
                json!({"while": "phase < 2", "action": "increment(phase)"}),
            ),
        ],
        "",
    );
    let config = resolve_loop_config(&source).unwrap().unwrap();

    // Inline mode composes the `prompt:` frontmatter value as the body,
    // so an empty document body still resolves and the control variable
    // `phase` lifts into the seed.
    let seed = build_loop_seed(
        &source,
        &config,
        PrepareOptions::default(),
        CompositionMode::InlineFrontmatterPrompt,
    )
    .expect("inline seed should resolve from prompt frontmatter with empty body");
    assert_eq!(seed.get("phase"), Some(&json!(1)));

    // Direct mode composes the document body itself, which is empty, so
    // seeding fails before iteration 1 with `ComposedBodyEmpty`. This
    // locks in the mode distinction that motivates parameterizing
    // `build_loop_seed` by `CompositionMode`.
    let direct = build_loop_seed(
        &source,
        &config,
        PrepareOptions::default(),
        CompositionMode::ChainedDocument,
    );
    assert!(
        matches!(direct, Err(CompositionError::ComposedBodyEmpty { .. })),
        "direct mode with empty body should fail seed resolution; got {direct:?}"
    );
}

#[test]
fn seeded_loop_repro_runs_to_completion_with_live_derived_variable() {
    let source = make_source_with_body(
        &[
            ("phase", json!("{{ start_phase || 1 }}")),
            ("total_phases", json!(6)),
            (
                "pass_icon",
                json!("{{ _loop_is_last ? '✅' : '🧑‍💻' }}"),
            ),
            (
                "loop",
                json!({"until": "phase > total_phases", "action": "increment(phase)"}),
            ),
        ],
        "Implement Phase {{ phase }} of {{ total_phases }}",
    );
    let config = resolve_loop_config(&source).unwrap().unwrap();
    let seed = build_loop_seed(
        &source,
        &config,
        PrepareOptions::default(),
        CompositionMode::ChainedDocument,
    )
    .unwrap();

    let captured = RefCell::new(Vec::new());
    let result = execute_loop_with_config(
        &source.resolved_path,
        &config,
        seed,
        LoopExecutionOptions::default(),
        |ctx| {
            let prepared = prepare_direct(
                &source,
                PrepareOptions {
                    set_overrides: Some(ctx.as_set_overrides()),
                    ..PrepareOptions::default()
                },
            )?;
            let pass_icon = prepared
                .effective_frontmatter
                .as_object()
                .and_then(|fm| fm.get("pass_icon"))
                .cloned();
            let body = prepared.prompt.clone();
            captured.borrow_mut().push((
                ctx.iteration,
                ctx.frontmatter.get("phase").cloned(),
                body,
                pass_icon,
            ));
            Ok(LoopIterationOutput::success(prepared.prompt))
        },
    )
    .unwrap();

    assert!(result.error.is_none(), "expected clean run, got {result:?}");
    assert_eq!(result.iteration_count, 6);
    assert_eq!(result.final_frontmatter.get("phase"), Some(&json!(7)));

    let seen = captured.into_inner();
    assert_eq!(seen.len(), 6);
    for (index, (iteration, phase, body, pass_icon)) in seen.iter().enumerate() {
        let n = index + 1;
        assert_eq!(*iteration, n);
        assert_eq!(*phase, Some(json!(n)));
        assert_eq!(body.trim(), format!("Implement Phase {n} of 6"));
        let expected_icon = if n == 6 { "✅" } else { "🧑‍💻" };
        assert_eq!(
            pass_icon.as_ref().and_then(|v| v.as_str()),
            Some(expected_icon),
            "pass_icon on iteration {n} should be {expected_icon}"
        );
    }
}

#[test]
fn seeded_loop_reports_honest_error_for_non_numeric_control_variable() {
    let source = make_source_with_body(
        &[
            ("area", json!("claudine")),
            (
                "loop",
                json!({"while": "true", "action": "increment(area)"}),
            ),
        ],
        "work",
    );
    let config = resolve_loop_config(&source).unwrap().unwrap();
    let seed = build_loop_seed(
        &source,
        &config,
        PrepareOptions::default(),
        CompositionMode::ChainedDocument,
    )
    .unwrap();

    let result = execute_loop_with_config(
        &source.resolved_path,
        &config,
        seed,
        LoopExecutionOptions::default(),
        |_ctx| Ok(LoopIterationOutput::success("ok")),
    )
    .unwrap();

    assert_eq!(result.iteration_count, 1);
    match result.error {
        Some(CompositionError::InvalidIncrementType {
            property,
            found,
            value_excerpt,
            ..
        }) => {
            assert_eq!(property, "area");
            assert_eq!(found, "string");
            assert!(
                value_excerpt.contains("claudine"),
                "excerpt should quote the value: {value_excerpt}"
            );
        }
        other => panic!("expected InvalidIncrementType, got {other:?}"),
    }
}

#[test]
fn seeded_loop_doc_namespace_condition_retains_readonly_control_value() {
    let source = make_source_with_body(
        &[
            ("counter", json!(0)),
            ("total", json!(2)),
            (
                "loop",
                json!({"while": "doc.counter < doc.total", "action": "increment(counter)"}),
            ),
        ],
        "Step {{ counter }} of {{ total }}",
    );
    let config = resolve_loop_config(&source).unwrap().unwrap();
    let seed = build_loop_seed(
        &source,
        &config,
        PrepareOptions::default(),
        CompositionMode::ChainedDocument,
    )
    .unwrap();

    assert_eq!(seed.get("counter"), Some(&json!(0)));
    assert_eq!(seed.get("total"), Some(&json!(2)));

    let captured = RefCell::new(Vec::new());
    let result = execute_loop_with_config(
        &source.resolved_path,
        &config,
        seed,
        LoopExecutionOptions::default(),
        |ctx| {
            let prepared = prepare_direct(
                &source,
                PrepareOptions {
                    set_overrides: Some(ctx.as_set_overrides()),
                    ..PrepareOptions::default()
                },
            )?;
            let body = prepared.prompt.clone();
            captured.borrow_mut().push((ctx.iteration, body));
            Ok(LoopIterationOutput::success(prepared.prompt))
        },
    )
    .unwrap();

    assert!(result.error.is_none(), "expected clean run, got {result:?}");
    assert_eq!(result.iteration_count, 2);
    assert_eq!(result.final_frontmatter.get("counter"), Some(&json!(2)));

    let seen = captured.into_inner();
    assert_eq!(seen.len(), 2);
    for (index, (iteration, body)) in seen.iter().enumerate() {
        let n = index + 1;
        assert_eq!(*iteration, n);
        // Iteration N uses the counter value BEFORE the increment fires
        // at the end of the iteration (counter 0→1→2), so the rendered
        // body shows the starting counter for that pass.
        assert_eq!(body.trim(), format!("Step {} of 2", n - 1));
    }
}


/// R4 reaches the loop route through the seed compose.
///
/// The seed pass runs *before* the engine emits `initialize`, so it is one of
/// the reads that must withhold the verdict when the caller selected the
/// deferred stage. While the CLI hard-coded `defer_schema_verdict: false` for
/// the loop route, a document whose own `initialize` supplies a required
/// property was rejected here — before the event that would have supplied it —
/// even though the identical document without `loop:` ran to completion.
#[test]
fn loop_seed_read_honors_the_deferred_schema_verdict() {
    let source = make_source(&[
        ("$schema", json!({"count": "number(required)"})),
        ("phase", json!(1)),
        (
            "initialize",
            json!({"stack": [{"action": {"set_frontmatter": ["loop.md", "count", 7]}}]}),
        ),
        (
            "loop",
            json!({"until": "phase > 2", "action": "increment(phase)"}),
        ),
    ]);
    let config = resolve_loop_config(&source).unwrap().unwrap();

    let judged = build_loop_seed_with_lifecycle(
        &source,
        &config,
        PrepareOptions::default(),
        CompositionMode::ChainedDocument,
    );
    assert!(
        judged.is_err(),
        "the undeferred seed read owns the verdict, so an unauthored required \
         property must fail here — otherwise this row cannot tell the two \
         stages apart"
    );

    let deferred = build_loop_seed_with_lifecycle(
        &source,
        &config,
        PrepareOptions {
            defer_schema_verdict: true,
            ..PrepareOptions::default()
        },
        CompositionMode::ChainedDocument,
    );
    assert!(
        deferred.is_ok(),
        "a deferred seed read must not judge the document: `initialize` has not \
         run yet and is what supplies `count`; got {:?}",
        deferred.err()
    );
}
