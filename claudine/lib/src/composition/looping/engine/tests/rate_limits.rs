//! rate limits loop-engine tests.

use super::*;

fn throttled(message: Option<&str>, reset_in_secs: Option<i64>) -> RateLimitInfo {
    RateLimitInfo {
        is_throttled: Some(true),
        retry_after_ms: None,
        message: message.map(str::to_string),
        reset_at: reset_in_secs.map(|s| chrono::Utc::now() + chrono::Duration::seconds(s)),
    }
}

#[test]
fn rate_limit_continue_policy_proceeds_without_pausing() {
    // While-condition exits after 2 successful iterations. Even though
    // iteration 1 carries a rate-limit trailer, the `Continue` policy
    // means we don't pause and we don't abort.
    let config = LoopConfig {
        condition: LoopCondition::While("counter < 2".into()),
        actions: vec![LoopAction::Increment("counter".into())],
        max_iterations: None,
        fail_fast: None,
        on_rate_limit: Some(OnRateLimit::Continue),
    };

    let observed = RefCell::new(Vec::new());
    let start = std::time::Instant::now();
    let result = execute_loop_with_config(
        Path::new("loop.md"),
        &config,
        object(json!({"counter": 0})),
        LoopExecutionOptions::default(),
        |ctx| {
            observed.borrow_mut().push(ctx.iteration);
            Ok(LoopIterationOutput::success("ok")
                .with_rate_limit(Some(throttled(Some("hit cap"), Some(60)))))
        },
    )
    .unwrap();
    let elapsed = start.elapsed();

    assert!(result.error.is_none(), "got: {result:?}");
    assert_eq!(result.iteration_count, 2);
    assert!(
        elapsed < std::time::Duration::from_millis(500),
        "Continue policy should not sleep; elapsed = {elapsed:?}"
    );
}

#[test]
fn rate_limit_abort_policy_halts_with_structured_error() {
    let config = LoopConfig {
        condition: LoopCondition::While("counter < 5".into()),
        actions: vec![LoopAction::Increment("counter".into())],
        max_iterations: None,
        fail_fast: None,
        on_rate_limit: Some(OnRateLimit::Abort),
    };

    let result = execute_loop_with_config(
        Path::new("loop.md"),
        &config,
        object(json!({"counter": 0})),
        LoopExecutionOptions::default(),
        |_ctx| {
            Ok(LoopIterationOutput::success("ok")
                .with_rate_limit(Some(throttled(Some("usage cap"), Some(60))))
                .with_attribution(Some("k2p6".into()), Some("kimi-for-coding".into())))
        },
    )
    .unwrap();

    assert_eq!(result.iteration_count, 1);
    match result.error {
        Some(CompositionError::LoopRateLimited {
            iteration,
            provider,
            model,
            reset_at,
            message,
            ..
        }) => {
            assert_eq!(iteration, 1);
            assert_eq!(provider.as_deref(), Some("k2p6"));
            assert_eq!(model.as_deref(), Some("kimi-for-coding"));
            assert!(reset_at.is_some());
            assert_eq!(message.as_deref(), Some("usage cap"));
        }
        other => panic!("expected LoopRateLimited, got {other:?}"),
    }
}

#[test]
fn rate_limit_pause_with_no_reset_falls_back_to_abort() {
    // No `reset_at` → Pause cannot wait an unbounded amount, so we
    // abort cleanly with the same structured error.
    let config = LoopConfig {
        condition: LoopCondition::While("counter < 5".into()),
        actions: vec![LoopAction::Increment("counter".into())],
        max_iterations: None,
        fail_fast: None,
        on_rate_limit: Some(OnRateLimit::Pause),
    };

    let result = execute_loop_with_config(
        Path::new("loop.md"),
        &config,
        object(json!({"counter": 0})),
        LoopExecutionOptions::default(),
        |_ctx| {
            Ok(LoopIterationOutput::success("ok")
                .with_rate_limit(Some(throttled(Some("no reset clock"), None))))
        },
    )
    .unwrap();

    assert_eq!(result.iteration_count, 1);
    assert!(
        matches!(
            result.error,
            Some(CompositionError::LoopRateLimited { reset_at: None, .. })
        ),
        "got: {:?}",
        result.error
    );
}

#[test]
fn rate_limit_pause_skipped_on_final_iteration() {
    // When the loop is already going to exit (is_last == true), the
    // engine must not pause — it would block for nothing.
    let config = LoopConfig {
        condition: LoopCondition::While("counter < 1".into()),
        actions: vec![LoopAction::Increment("counter".into())],
        max_iterations: None,
        fail_fast: None,
        on_rate_limit: Some(OnRateLimit::Pause),
    };

    let start = std::time::Instant::now();
    let result = execute_loop_with_config(
        Path::new("loop.md"),
        &config,
        object(json!({"counter": 0})),
        LoopExecutionOptions::default(),
        |_ctx| {
            // Iteration 1 IS the last (counter goes 0 → 1, condition fails next round).
            Ok(LoopIterationOutput::success("ok")
                .with_rate_limit(Some(throttled(Some("trailer on last"), Some(300)))))
        },
    )
    .unwrap();
    let elapsed = start.elapsed();

    assert!(result.error.is_none(), "got: {result:?}");
    assert!(
        elapsed < std::time::Duration::from_millis(500),
        "should skip pause on last iteration; elapsed = {elapsed:?}"
    );
}

#[test]
fn rate_limit_default_policy_is_pause() {
    // Neither options nor config set on_rate_limit. With no reset_at,
    // the default Pause falls back to Abort.
    let config = LoopConfig {
        condition: LoopCondition::While("counter < 5".into()),
        actions: vec![LoopAction::Increment("counter".into())],
        max_iterations: None,
        fail_fast: None,
        on_rate_limit: None,
    };

    let result = execute_loop_with_config(
        Path::new("loop.md"),
        &config,
        object(json!({"counter": 0})),
        LoopExecutionOptions::default(),
        |_ctx| {
            Ok(LoopIterationOutput::success("ok").with_rate_limit(Some(throttled(None, None))))
        },
    )
    .unwrap();

    assert!(
        matches!(result.error, Some(CompositionError::LoopRateLimited { .. })),
        "default should be Pause→Abort fallback; got: {:?}",
        result.error
    );
}

#[test]
fn rate_limit_pause_sleeps_until_reset_then_continues() {
    // With Pause policy the engine must sleep until `reset_at` (plus the
    // safety margin) before running the next iteration. We inject a zero
    // margin and a 1s reset so the test verifies the wait-then-continue
    // behaviour without burning the production 5s margin.
    let config = LoopConfig {
        condition: LoopCondition::While("counter < 2".into()),
        actions: vec![LoopAction::Increment("counter".into())],
        max_iterations: None,
        fail_fast: None,
        on_rate_limit: Some(OnRateLimit::Pause),
    };

    let start = std::time::Instant::now();
    let result = execute_loop_with_config(
        Path::new("loop.md"),
        &config,
        object(json!({"counter": 0})),
        LoopExecutionOptions {
            pause_reset_margin: Some(std::time::Duration::ZERO),
            ..LoopExecutionOptions::default()
        },
        |ctx| {
            let rl = if ctx.iteration == 1 {
                // 1s reset + 0 margin → the engine pauses ~1s before
                // proceeding to iteration 2.
                Some(throttled(Some("brief cap"), Some(1)))
            } else {
                None
            };
            Ok(LoopIterationOutput::success("ok").with_rate_limit(rl))
        },
    )
    .unwrap();
    let elapsed = start.elapsed();

    assert!(result.error.is_none(), "got: {result:?}");
    assert_eq!(result.iteration_count, 2);
    // 1s reset + 0 margin → it must have waited, but not unbounded.
    assert!(
        elapsed >= std::time::Duration::from_millis(500),
        "expected ~1s pause; elapsed = {elapsed:?}"
    );
    assert!(
        elapsed < std::time::Duration::from_secs(10),
        "pause should not be unbounded; elapsed = {elapsed:?}"
    );
}

#[test]
fn rate_limit_pause_is_interrupt_aware() {
    // When the interrupt_check callback returns true, the pause exits
    // immediately and the engine returns Proceed (caller will see the
    // interrupt on the next iteration via its wrapped executor).
    use std::sync::atomic::{AtomicBool, Ordering};

    // Static flag because LoopExecutionOptions.interrupt_check is a
    // bare `fn() -> bool` (Copy).
    static FIRED: AtomicBool = AtomicBool::new(false);
    FIRED.store(true, Ordering::SeqCst);
    fn always_interrupted() -> bool {
        FIRED.load(Ordering::SeqCst)
    }

    let config = LoopConfig {
        condition: LoopCondition::While("counter < 2".into()),
        actions: vec![LoopAction::Increment("counter".into())],
        max_iterations: None,
        fail_fast: None,
        on_rate_limit: Some(OnRateLimit::Pause),
    };

    let start = std::time::Instant::now();
    let _result = execute_loop_with_config(
        Path::new("loop.md"),
        &config,
        object(json!({"counter": 0})),
        LoopExecutionOptions {
            interrupt_check: Some(always_interrupted),
            ..LoopExecutionOptions::default()
        },
        |ctx| {
            let rl = if ctx.iteration == 1 {
                // Long reset to prove the interrupt cut it short.
                Some(throttled(Some("long cap"), Some(60)))
            } else {
                None
            };
            Ok(LoopIterationOutput::success("ok").with_rate_limit(rl))
        },
    )
    .unwrap();
    let elapsed = start.elapsed();

    assert!(
        elapsed < std::time::Duration::from_secs(5),
        "interrupt should cut pause short; elapsed = {elapsed:?}"
    );
    FIRED.store(false, Ordering::SeqCst);
}

// ── Seeded-loop integration tests ────────────────────────────────────


