//! Characterization of the post-checked loop: when the condition is read and
//! what each iteration's body sees.

use super::*;

/// The counting table from the loop contract, with `n: 0` and
/// `action: increment(n)`: the gate reads the state the finished iteration ran
/// with, and the action applies only when the gate continues.
#[test]
fn post_checked_condition_counts_iterations_and_body_state() {
    let rows: [(LoopCondition, &[i64]); 6] = [
        (LoopCondition::While("n < 0".into()), &[0]),
        (LoopCondition::While("n < 1".into()), &[0, 1]),
        (LoopCondition::While("n < 2".into()), &[0, 1, 2]),
        (LoopCondition::Until("n == 2".into()), &[0, 1, 2]),
        // The count rows, spelled with the engine's `_loop_count`.
        (LoopCondition::Until("_loop_count >= 3".into()), &[0, 1, 2]),
        (LoopCondition::Until("_loop_count > 3".into()), &[0, 1, 2, 3]),
    ];

    for (condition, expected_bodies) in rows {
        let label = format!("{condition:?}");
        let config = LoopConfig {
            condition,
            actions: vec![LoopAction::Increment("n".into())],
            max_iterations: None,
            fail_fast: None,
            on_rate_limit: None,
        };
        let observed = RefCell::new(Vec::new());

        let result = run_loop(
            Path::new("loop.md"),
            &config,
            object(json!({"n": 0})),
            LoopExecutionOptions::default(),
            |ctx| {
                let prompt_values = ctx
                    .as_layered_overrides(&crate::composition::LayeredOverrides::new())
                    .to_value();
                observed
                    .borrow_mut()
                    .push((prompt_values["_loop_count"].clone(), prompt_values["n"].clone()));
                Ok(LoopIterationOutput::success("ok"))
            },
        )
        .unwrap();

        let expected: Vec<(Value, Value)> = expected_bodies
            .iter()
            .enumerate()
            .map(|(index, n)| (json!(index + 1), json!(n)))
            .collect();
        assert!(result.error.is_none(), "{label}: {result:?}");
        assert_eq!(
            result.iteration_count,
            expected_bodies.len(),
            "{label}: iteration count"
        );
        assert_eq!(*observed.borrow(), expected, "{label}: (_loop_count, n) per body");
        assert_eq!(
            result.final_frontmatter.get("n"),
            expected_bodies.last().map(|n| json!(n)).as_ref(),
            "{label}: the stopping gate applies no action"
        );
    }
}
