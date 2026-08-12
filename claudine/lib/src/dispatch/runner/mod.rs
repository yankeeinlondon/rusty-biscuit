use std::path::PathBuf;
use std::time::Duration;

use darkmatter::markdown::compose::expression::{evaluate, is_truthy, parse_condition};
use tracing::{debug, info_span, warn};

use crate::actions::{CompiledMapper, HookAction, HookDecision, HookResponse};
use crate::config::claudine_config::{ClaudineConfig, Gender};
use crate::dispatch::expression::EventMetaConditionLookup;
use crate::dispatch::template::interpolate;
use crate::error::Result;
use crate::events::{AgenticEvent, EventMeta};
use crate::messaging::RuntimeMessagingSettings;
use crate::protect::decision::ProtectDecision;

mod bash;
mod decisions;
mod mappers;
mod null_strip;
mod protect;
mod report;
mod speak;

use bash::{BASH_ACTION_TIMEOUT, execute_bash, run_command_blocking};
use decisions::{dot_lookup, parse_decision, should_replace_selected};
use mappers::{CommandOutput, apply_mapper};
use null_strip::strip_nulls;
use protect::{attach_protect_context, decision_for_short_circuit, should_short_circuit_call};
use report::execute_report;
use speak::execute_speak_from_claudine;

/// Configuration sources supported by the action runner.
#[derive(Clone, Copy)]
pub(crate) enum DispatchConfig<'a> {
    Canonical(&'a ClaudineConfig),
}

impl DispatchConfig<'_> {
    fn execute_speak(
        self,
        message_template: &str,
        voice_override: Option<&str>,
        gender_override: Option<Gender>,
        meta: &EventMeta,
    ) {
        match self {
            Self::Canonical(config) => execute_speak_from_claudine(
                message_template,
                voice_override,
                gender_override,
                meta,
                config,
            ),
        }
    }
}

/// Outcome of evaluating a hook action's optional `when` condition.
enum WhenOutcome {
    /// No `when` was provided, or evaluation produced a truthy result.
    Run,
    /// Evaluation produced a falsy result; the action should be skipped.
    SkipFalse,
    /// The expression failed to parse or evaluate; the action is skipped
    /// non-fatally and a warning has already been emitted.
    SkipInvalid,
}

/// Evaluate an action's `when` expression against the live [`EventMeta`].
///
/// `when` is optional: when absent, the action always runs. When present,
/// the expression is parsed and evaluated through [`EventMetaConditionLookup`]
/// which layers Darkmatter's lazy `ctx.*` capture on top of
/// [`EventMetaExpressionLookup`]. Falsy results yield [`WhenOutcome::SkipFalse`];
/// parse or evaluation errors yield [`WhenOutcome::SkipInvalid`] with a
/// `tracing::warn!` so operators can spot a broken condition without
/// breaking the rest of the binding.
#[allow(dead_code)]
fn evaluate_when(when: Option<&str>, meta: &EventMeta) -> WhenOutcome {
    let work_dir: PathBuf = meta
        .cwd
        .as_deref()
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let lookup = EventMetaConditionLookup::new(meta, work_dir.as_path());
    evaluate_when_with_lookup(when, &lookup)
}

/// Evaluate an action's `when` expression using a pre-built [`EventMetaConditionLookup`].
///
/// This keeps the parse/evaluate plumbing but reuses the cached context groups,
/// avoiding repeated `ctx.*` captures across multiple actions in the same binding.
fn evaluate_when_with_lookup(
    when: Option<&str>,
    lookup: &EventMetaConditionLookup<'_>,
) -> WhenOutcome {
    let Some(expr) = when else {
        return WhenOutcome::Run;
    };

    let parsed = match parse_condition(expr) {
        Ok(parsed) => parsed,
        Err(error) => {
            warn!(
                expression = expr,
                %error,
                "Hook action `when` failed to parse; skipping action",
            );
            return WhenOutcome::SkipInvalid;
        }
    };

    match evaluate(&parsed, lookup) {
        Ok(value) if is_truthy(&value) => WhenOutcome::Run,
        Ok(_) => WhenOutcome::SkipFalse,
        Err(error) => {
            warn!(
                expression = expr,
                %error,
                "Hook action `when` failed to evaluate; skipping action",
            );
            WhenOutcome::SkipInvalid
        }
    }
}

/// Execute hook actions in declaration order.
///
/// Returns the selected blocking response from `call` actions when applicable.
pub(crate) async fn execute_actions(
    actions: &[HookAction],
    compiled_mappers: Option<&[Option<CompiledMapper>]>,
    meta: &EventMeta,
    config: DispatchConfig<'_>,
    messaging: &RuntimeMessagingSettings,
    can_block: bool,
    protect_decision: Option<&ProtectDecision>,
) -> Result<Option<HookResponse>> {
    let mut selected_response: Option<HookResponse> = None;

    // Build the composite lookup once so that `ctx.*` captures are cached
    // across all actions in this binding.
    let work_dir: PathBuf = meta
        .cwd
        .as_deref()
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let lookup = EventMetaConditionLookup::new(meta, work_dir.as_path());

    for (index, action) in actions.iter().enumerate() {
        // Pre-execution `when` gate. Falsy or invalid conditions skip the
        // action without affecting `selected_response`, which guarantees
        // a skipped `Call` cannot replace a previously selected blocking
        // response.
        match evaluate_when_with_lookup(action.when(), &lookup) {
            WhenOutcome::Run => {}
            WhenOutcome::SkipFalse => {
                debug!(
                    action_index = index,
                    action_kind = action.type_slug(),
                    expression = action.when().unwrap_or_default(),
                    "Hook action skipped by falsy `when` condition",
                );
                continue;
            }
            WhenOutcome::SkipInvalid => continue,
        }

        match action {
            HookAction::Speak {
                message,
                voice,
                gender,
                when: _,
            } => {
                let _action_span = info_span!(
                    "hook_action",
                    action_index = index,
                    action_kind = "speak",
                    blocking = can_block,
                    timeout_ms = tracing::field::Empty,
                    target_kind = tracing::field::Empty,
                    command = tracing::field::Empty,
                )
                .entered();
                config.execute_speak(message, voice.as_deref(), *gender, meta);
            }
            HookAction::Report { handler, when: _ } => {
                let _action_span = info_span!(
                    "hook_action",
                    action_index = index,
                    action_kind = "report",
                    blocking = can_block,
                    timeout_ms = tracing::field::Empty,
                    target_kind = tracing::field::Empty,
                    command = tracing::field::Empty,
                )
                .entered();
                execute_report(handler.as_ref(), meta, can_block);
            }
            HookAction::Bash {
                command,
                params,
                when: _,
            } => {
                let _action_span = info_span!(
                    "hook_action",
                    action_index = index,
                    action_kind = "bash",
                    blocking = can_block,
                    timeout_ms = BASH_ACTION_TIMEOUT.as_millis() as u64,
                    target_kind = tracing::field::Empty,
                    command = %command,
                )
                .entered();
                execute_bash(command, params, meta).await;
            }
            HookAction::Call {
                command,
                args,
                timeout_ms,
                mapper,
                when: _,
            } => {
                let timeout = timeout_ms
                    .map(Duration::from_millis)
                    .unwrap_or(Duration::from_secs(60));
                let _action_span = info_span!(
                    "hook_action",
                    action_index = index,
                    action_kind = "call",
                    blocking = can_block,
                    timeout_ms = timeout.as_millis(),
                    target_kind = tracing::field::Empty,
                    command = %command,
                )
                .entered();
                if should_short_circuit_call(protect_decision) {
                    let Some(decision) = protect_decision else {
                        continue;
                    };

                    let response = HookResponse {
                        decision: Some(decision_for_short_circuit(decision)),
                        reason: Some("call action short-circuited by protect: blocked".to_string()),
                        ..HookResponse::default()
                    };

                    debug!("Short-circuiting call action due to protect block");
                    if can_block && should_replace_selected(selected_response.as_ref(), &response) {
                        selected_response = Some(response);
                    }
                    continue;
                }

                let cmd = interpolate(command, meta);
                let rendered_args = args.as_ref().map(|items| {
                    items
                        .iter()
                        .map(|arg| interpolate(arg, meta))
                        .collect::<Vec<_>>()
                });

                let compiled_mapper = compiled_mappers
                    .and_then(|mappers| mappers.get(index))
                    .and_then(Option::as_ref);

                match tokio::time::timeout(
                    timeout,
                    run_command_blocking(&cmd, rendered_args.as_deref()),
                )
                .await
                {
                    Ok(Ok(output)) => match apply_mapper(compiled_mapper, mapper.as_ref(), &output)
                    {
                        Ok(response) => {
                            let response = attach_protect_context(response, protect_decision);
                            if can_block {
                                if !response_has_blocking_effect(&response) {
                                    debug!(
                                        %cmd,
                                        "Call response on blocking event had no actionable decision or payload; falling through"
                                    );
                                } else if should_replace_selected(
                                    selected_response.as_ref(),
                                    &response,
                                ) {
                                    selected_response = Some(response);
                                }
                            } else {
                                debug!(%cmd, "Call response produced on non-blocking event and discarded");
                            }
                        }
                        Err(error) => {
                            warn!(%cmd, %error, "Call mapper failed");
                            if can_block {
                                let response = blocking_call_failure_response(
                                    &cmd,
                                    format!("call action mapper failed: {error}"),
                                );
                                if should_replace_selected(selected_response.as_ref(), &response) {
                                    selected_response = Some(response);
                                }
                            }
                        }
                    },
                    Ok(Err(error)) => {
                        warn!(%cmd, %error, "Call command failed");
                        if can_block {
                            let response = blocking_call_failure_response(
                                &cmd,
                                format!("call action command failed: {error}"),
                            );
                            if should_replace_selected(selected_response.as_ref(), &response) {
                                selected_response = Some(response);
                            }
                        }
                    }
                    Err(_) => {
                        warn!(%cmd, timeout_ms = timeout.as_millis(), "Call command timed out");
                        if can_block {
                            let response = blocking_call_failure_response(
                                &cmd,
                                format!(
                                    "call action command timed out after {}ms",
                                    timeout.as_millis()
                                ),
                            );
                            if should_replace_selected(selected_response.as_ref(), &response) {
                                selected_response = Some(response);
                            }
                        }
                    }
                }
            }
            HookAction::SoundEffect {
                effect,
                volume,
                speed,
                when: _,
            } => {
                let _action_span = info_span!(
                    "hook_action",
                    action_index = index,
                    action_kind = "sound_effect",
                    blocking = can_block,
                    timeout_ms = tracing::field::Empty,
                    target_kind = tracing::field::Empty,
                    command = tracing::field::Empty,
                )
                .entered();
                execute_sound_effect(effect, *volume, *speed);
            }
            HookAction::Message {
                message,
                image,
                when: _,
            } => {
                let _action_span = info_span!(
                    "hook_action",
                    action_index = index,
                    action_kind = "message",
                    blocking = can_block,
                    timeout_ms = tracing::field::Empty,
                    target_kind = tracing::field::Empty,
                    command = tracing::field::Empty,
                )
                .entered();
                crate::messaging::execute_message(message, image.as_deref(), meta, messaging);
            }
        }
    }

    Ok(selected_response)
}

fn response_has_blocking_effect(response: &HookResponse) -> bool {
    response.decision.is_some()
        || response.updated_input.is_some()
        || response.additional_context.is_some()
        || response.raw.is_some()
}

fn blocking_call_failure_response(command: &str, reason: String) -> HookResponse {
    HookResponse {
        decision: Some(HookDecision::Deny),
        reason: Some(format!("{reason} ({command})")),
        ..HookResponse::default()
    }
}

/// Determine which default sound (if any) should play for the given event.
///
/// Maps canonical events to sound categories:
/// - `success`: SessionEnd, TurnComplete
/// - `attention`: HumanInTheLoop
/// - `error`: triggered by protect blocks
pub fn default_sound_for_event<'a>(
    event: &AgenticEvent,
    config: &'a ClaudineConfig,
    was_blocked: bool,
) -> Option<&'a str> {
    if was_blocked {
        return config.default_sounds.error.as_deref();
    }
    match event {
        AgenticEvent::SessionEnd | AgenticEvent::TurnComplete => {
            config.default_sounds.success.as_deref()
        }
        AgenticEvent::HumanInTheLoop => config.default_sounds.attention.as_deref(),
        _ => None,
    }
}

/// Play the appropriate default sound for an event, if configured.
pub(crate) fn play_default_sound_for_event(
    event: &AgenticEvent,
    config: &ClaudineConfig,
    was_blocked: bool,
) {
    if let Some(name) = default_sound_for_event(event, config, was_blocked) {
        execute_sound_effect(name, 1.0, 1.0);
    }
}

fn execute_sound_effect(name: &str, volume: f32, speed: f32) {
    let effect = match playa::SoundEffect::from_name(name) {
        Some(effect) => effect,
        None => {
            warn!(%name, "Unknown sound effect name");
            return;
        }
    };

    tokio::task::spawn_blocking(move || {
        let playback = match playa::Playa::from_bytes(effect.bytes().to_vec()) {
            Ok(player) => player.volume(volume).speed(speed).play(),
            Err(error) => {
                warn!(%error, "Failed to construct sound effect player");
                return;
            }
        };
        if let Err(error) = playback {
            warn!(%error, "Sound effect playback failed");
        }
    });
}

#[cfg(test)]
mod tests;
