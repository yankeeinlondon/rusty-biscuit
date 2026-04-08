use std::collections::HashMap;

use claudine::actions::HookAction;
use claudine::config::claudine_config::{ClaudineConfig, DefaultSounds};
use claudine::dispatch::loader::{CanonicalRuntimeConfig, compile_canonical_runtime};
use claudine::dispatch::{DispatchOutcome, dispatch_canonical_with_runtime};
use claudine::events::{AgenticEvent, EventMeta, Provider};

fn make_config_with_action(event: AgenticEvent, action: HookAction) -> CanonicalRuntimeConfig {
    let mut actions = HashMap::new();
    actions.insert(event, vec![action]);

    let mut config = ClaudineConfig::default();
    config.protect.enabled = false;
    config.default_sounds = DefaultSounds::default();
    config.actions = actions;

    compile_canonical_runtime(config, None).unwrap()
}

/// Dispatching a `SoundEffect` action for an event that has no audio device
/// available (in CI) completes without error and returns no blocking response.
#[tokio::test]
async fn dispatch_sound_effect_action() {
    let runtime = make_config_with_action(
        AgenticEvent::HumanInTheLoop,
        HookAction::SoundEffect {
            effect: "confirmation".to_string(),
            volume: 0.0,
            speed: 1.0,
        },
    );

    let meta = EventMeta::new(Provider::Claude, AgenticEvent::HumanInTheLoop);

    let outcome = dispatch_canonical_with_runtime(
        Provider::Claude,
        AgenticEvent::HumanInTheLoop,
        meta,
        &runtime,
    )
    .await
    .unwrap();

    // HumanInTheLoop is non-blocking for the Claude adapter, so no exit code.
    assert_eq!(outcome.exit_code, None);
    // No protect decision was involved.
    assert!(outcome.protect_pre.is_none());
    assert!(outcome.protect_post.is_none());
}

/// When an event has no binding in the runtime, the outcome equals the default.
#[tokio::test]
async fn dispatch_no_binding_returns_default() {
    // The runtime only has a binding for HumanInTheLoop.
    let runtime = make_config_with_action(
        AgenticEvent::HumanInTheLoop,
        HookAction::SoundEffect {
            effect: "confirmation".to_string(),
            volume: 1.0,
            speed: 1.0,
        },
    );

    // Dispatch a different event — SessionStart has no binding.
    let meta = EventMeta::new(Provider::Claude, AgenticEvent::SessionStart);

    let outcome = dispatch_canonical_with_runtime(
        Provider::Claude,
        AgenticEvent::SessionStart,
        meta,
        &runtime,
    )
    .await
    .unwrap();

    assert_eq!(outcome, DispatchOutcome::default());
}

/// An empty actions list with a binding present does not produce a response.
#[tokio::test]
async fn dispatch_empty_actions_returns_non_blocking_ack() {
    let mut config = ClaudineConfig::default();
    config.protect.enabled = false;
    config.default_sounds = DefaultSounds::default();
    config.actions.insert(AgenticEvent::TurnComplete, vec![]);

    let runtime = compile_canonical_runtime(config, None).unwrap();
    let meta = EventMeta::new(Provider::Claude, AgenticEvent::TurnComplete);

    let outcome = dispatch_canonical_with_runtime(
        Provider::Claude,
        AgenticEvent::TurnComplete,
        meta,
        &runtime,
    )
    .await
    .unwrap();

    // No blocking response; no protect or exit code.
    assert_eq!(outcome.exit_code, None);
    assert!(outcome.protect_pre.is_none());
    assert!(outcome.protect_post.is_none());
}

/// Dispatching with no config at all returns the default outcome immediately.
#[tokio::test]
async fn dispatch_with_default_config_returns_default_for_every_event() {
    let config = ClaudineConfig::default();
    let runtime = compile_canonical_runtime(config, None).unwrap();

    for event in [
        AgenticEvent::SessionStart,
        AgenticEvent::TurnComplete,
        AgenticEvent::HumanInTheLoop,
    ] {
        let meta = EventMeta::new(Provider::Claude, event);
        let outcome =
            dispatch_canonical_with_runtime(Provider::Claude, event, meta, &runtime)
                .await
                .unwrap();
        assert_eq!(
            outcome,
            DispatchOutcome::default(),
            "expected default outcome for {event}"
        );
    }
}
