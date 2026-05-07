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
fn evaluate_when_with_lookup(when: Option<&str>, lookup: &EventMetaConditionLookup<'_>) -> WhenOutcome {
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
mod tests {
    use std::collections::HashMap;

    use chrono::Utc;

    use super::*;
    use crate::config::claudine_config::{ClaudineConfig, TtsValue};
    use crate::events::{EnvironmentContext, GitContext, HardwareContext, OsContext, RepoContext};
    use crate::provider::Provider;
    use std::path::PathBuf;

    fn meta() -> EventMeta {
        EventMeta {
            provider: Provider::Claude,
            event: crate::events::AgenticEvent::BeforeTool,
            timestamp: Utc::now(),
            session_id: Some("test-session".to_string()),
            cwd: Some("/tmp".to_string()),
            tool_name: Some("Bash".to_string()),
            tool_input: None,
            tool_response: None,
            error: None,
            prompt: None,
            agent_type: None,
            notification_type: None,
            notification_message: None,
            extra: HashMap::new(),
            env: EnvironmentContext::default(),
        }
    }

    fn claudine_config_with_tts(tts: TtsValue) -> ClaudineConfig {
        ClaudineConfig {
            tts,
            messenger: None,
            logging: true,
            protect: Default::default(),
            actions: HashMap::new(),
            matchers: HashMap::new(),
            preferred_agent: Some(Provider::Claude),
            canonical_provider: None,
            models: HashMap::new(),
            default_sounds: Default::default(),
        }
    }

    fn make_meta_for_when_tests() -> EventMeta {
        let mut m = meta();
        m.tool_name = Some("Bash".to_string());
        m
    }

    fn meta_with_full_env() -> EventMeta {
        let mut m = make_meta_for_when_tests();
        m.env.os = OsContext {
            os_type: "macos".to_string(),
            name: "macOS".to_string(),
            version: "15.3".to_string(),
            kernel: "Darwin 25.3.0".to_string(),
            hostname: "test-host".to_string(),
            linux_family: None,
            package_managers: vec!["brew".to_string()],
        };
        m.env.hardware = HardwareContext {
            arch: "aarch64".to_string(),
            cpu: "Apple M4 Max".to_string(),
            cores: 16,
            memory_bytes: 68_719_476_736,
            memory_available_bytes: 34_359_738_368,
        };
        m.env.git = Some(GitContext {
            repo_root: PathBuf::from("/tmp/project"),
            branch: Some("main".to_string()),
            is_dirty: true,
            staged_count: 0,
            unstaged_count: 0,
            untracked_count: 0,
            head_sha: Some("abc123def".to_string()),
            head_message: Some("feat: add feature".to_string()),
            user_name: None,
            user_email: None,
            remote_name: Some("origin".to_string()),
            remote_url: None,
            hosting_provider: Some("github".to_string()),
            repo_name: Some("rusty-biscuit".to_string()),
            repo_org: Some("anthropics".to_string()),
        });
        m.env.repo = Some(RepoContext {
            is_monorepo: true,
            monorepo_tool: Some("cargo_workspace".to_string()),
            root: PathBuf::from("/tmp/project"),
            packages: vec!["lib".to_string(), "cli".to_string()],
        });
        m.env.primary_language = Some("Rust".to_string());
        m
    }

    #[tokio::test]
    async fn message_action_skipped_when_no_route() {
        let actions = vec![HookAction::Message {
            message: "test notification".to_string(),
            image: None,
            when: None,
        }];

        let messaging = crate::messaging::RuntimeMessagingSettings::default();

        let result = execute_actions(
            &actions,
            None,
            &meta(),
            DispatchConfig::Canonical(&ClaudineConfig::default()),
            &messaging,
            false,
            None,
        )
        .await
        .unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn message_action_does_not_block() {
        let actions = vec![
            HookAction::Message {
                message: "notify".to_string(),
                image: None,
                when: None,
            },
            HookAction::Report {
                handler: None,
                when: None,
            },
        ];

        let messaging = crate::messaging::RuntimeMessagingSettings::default();

        let result = execute_actions(
            &actions,
            None,
            &meta(),
            DispatchConfig::Canonical(&ClaudineConfig::default()),
            &messaging,
            true,
            None,
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn execute_actions_runs_bash_action_with_canonical_config() {
        let config = claudine_config_with_tts(TtsValue::Boolean(false));
        let messaging = RuntimeMessagingSettings::default();

        let actions = vec![
            HookAction::Bash {
                command: "echo".to_string(),
                params: "hello".to_string(),
                when: None,
            },
            HookAction::Report {
                handler: None,
                when: None,
            },
        ];

        let result = execute_actions(
            &actions,
            None,
            &meta(),
            DispatchConfig::Canonical(&config),
            &messaging,
            true,
            None,
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn execute_actions_message_action_with_canonical_config() {
        let config = claudine_config_with_tts(TtsValue::Boolean(false));
        let messaging = RuntimeMessagingSettings::default();

        let actions = vec![HookAction::Message {
            message: "test".to_string(),
            image: None,
            when: None,
        }];

        let result = execute_actions(
            &actions,
            None,
            &meta(),
            DispatchConfig::Canonical(&config),
            &messaging,
            false,
            None,
        )
        .await
        .unwrap();

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn blocking_call_command_failure_fails_closed() {
        let config = claudine_config_with_tts(TtsValue::Boolean(false));
        let messaging = RuntimeMessagingSettings::default();
        let actions = vec![HookAction::Call {
            command: "__claudine_missing_command__".to_string(),
            args: None,
            timeout_ms: Some(25),
            mapper: None,
            when: None,
        }];

        let result = execute_actions(
            &actions,
            None,
            &meta(),
            DispatchConfig::Canonical(&config),
            &messaging,
            true,
            None,
        )
        .await
        .unwrap()
        .expect("blocking call failure must synthesize a deny response");

        assert_eq!(result.decision, Some(HookDecision::Deny));
        let reason = result.reason.expect("deny response should explain failure");
        assert!(reason.contains("call action command failed"));
        assert!(reason.contains("__claudine_missing_command__"));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn blocking_call_without_actionable_response_falls_through() {
        let config = claudine_config_with_tts(TtsValue::Boolean(false));
        let messaging = RuntimeMessagingSettings::default();
        let actions = vec![HookAction::Call {
            command: "sh".to_string(),
            args: Some(vec!["-c".to_string(), "exit 1".to_string()]),
            timeout_ms: Some(250),
            mapper: None,
            when: None,
        }];

        let result = execute_actions(
            &actions,
            None,
            &meta(),
            DispatchConfig::Canonical(&config),
            &messaging,
            true,
            None,
        )
        .await
        .unwrap();

        assert!(
            result.is_none(),
            "exit-code mapper status 1 should fall through instead of producing an implicit allow"
        );
    }

    // =========================================================================
    // `when` condition tests (Phase 3 of leverage-dm-parser)
    // =========================================================================

    #[tokio::test]
    async fn when_condition_true_executes_action_and_can_block() {
        let config = claudine_config_with_tts(TtsValue::Boolean(false));
        let messaging = RuntimeMessagingSettings::default();
        let actions = vec![HookAction::Call {
            command: "__claudine_missing_when_true__".to_string(),
            args: None,
            timeout_ms: Some(50),
            mapper: None,
            when: Some("tool_name == 'Bash'".to_string()),
        }];

        let result = execute_actions(
            &actions,
            None,
            &make_meta_for_when_tests(),
            DispatchConfig::Canonical(&config),
            &messaging,
            true,
            None,
        )
        .await
        .unwrap()
        .expect("call action with truthy when should still run and synthesize a deny");

        assert_eq!(result.decision, Some(HookDecision::Deny));
    }

    #[tokio::test]
    async fn when_condition_false_skips_call_action_and_no_blocking_response() {
        let config = claudine_config_with_tts(TtsValue::Boolean(false));
        let messaging = RuntimeMessagingSettings::default();
        let actions = vec![HookAction::Call {
            command: "__claudine_missing_when_false__".to_string(),
            args: None,
            timeout_ms: Some(50),
            mapper: None,
            when: Some("tool_name == 'Read'".to_string()),
        }];

        let result = execute_actions(
            &actions,
            None,
            &make_meta_for_when_tests(),
            DispatchConfig::Canonical(&config),
            &messaging,
            true,
            None,
        )
        .await
        .unwrap();

        assert!(
            result.is_none(),
            "skipped Call action must not produce a blocking response",
        );
    }

    #[tokio::test]
    async fn when_invalid_expression_skips_action_non_fatally() {
        let config = claudine_config_with_tts(TtsValue::Boolean(false));
        let messaging = RuntimeMessagingSettings::default();
        let actions = vec![HookAction::Call {
            command: "__claudine_missing_when_invalid__".to_string(),
            args: None,
            timeout_ms: Some(50),
            mapper: None,
            when: Some("&& this is not a valid condition".to_string()),
        }];

        let result = execute_actions(
            &actions,
            None,
            &make_meta_for_when_tests(),
            DispatchConfig::Canonical(&config),
            &messaging,
            true,
            None,
        )
        .await;

        let outcome = result.expect("invalid `when` expression must not error the runner");
        assert!(
            outcome.is_none(),
            "invalid `when` should skip the action without producing a blocking response",
        );
    }

    #[tokio::test]
    async fn when_skipped_call_does_not_replace_prior_selected_response() {
        // First action is a failing Call producing a deny; second action
        // is a Call that would normally produce Continue but is skipped
        // by a falsy `when` and therefore must not overwrite the deny.
        let config = claudine_config_with_tts(TtsValue::Boolean(false));
        let messaging = RuntimeMessagingSettings::default();
        let actions = vec![
            HookAction::Call {
                command: "__claudine_when_first__".to_string(),
                args: None,
                timeout_ms: Some(50),
                mapper: None,
                when: None,
            },
            HookAction::Call {
                command: "echo".to_string(),
                args: Some(vec!["allow".to_string()]),
                timeout_ms: Some(500),
                mapper: None,
                when: Some("tool_name == 'Read'".to_string()),
            },
        ];

        let result = execute_actions(
            &actions,
            None,
            &make_meta_for_when_tests(),
            DispatchConfig::Canonical(&config),
            &messaging,
            true,
            None,
        )
        .await
        .unwrap()
        .expect("first call should produce a blocking deny");

        assert_eq!(result.decision, Some(HookDecision::Deny));
    }

    #[tokio::test]
    async fn when_env_var_resolves_via_env_namespace() {
        let config = claudine_config_with_tts(TtsValue::Boolean(false));
        let messaging = RuntimeMessagingSettings::default();

        let key = "CLAUDINE_DISPATCH_WHEN_ENV_VAR_PRESENT";
        // SAFETY: tests that touch process env should run serially within a
        // module; the var is unique per test scope and removed below.
        unsafe {
            std::env::set_var(key, "yes");
        }

        let actions = vec![HookAction::Call {
            command: "__claudine_missing_when_env__".to_string(),
            args: None,
            timeout_ms: Some(50),
            mapper: None,
            when: Some(format!("env.{key} == 'yes'")),
        }];

        let result = execute_actions(
            &actions,
            None,
            &make_meta_for_when_tests(),
            DispatchConfig::Canonical(&config),
            &messaging,
            true,
            None,
        )
        .await
        .unwrap();

        unsafe {
            std::env::remove_var(key);
        }

        let response = result.expect("env-backed condition should be truthy and let the call run");
        assert_eq!(response.decision, Some(HookDecision::Deny));
    }

    #[tokio::test]
    async fn when_missing_env_var_is_falsy_and_skips_action() {
        let config = claudine_config_with_tts(TtsValue::Boolean(false));
        let messaging = RuntimeMessagingSettings::default();

        unsafe {
            std::env::remove_var("CLAUDINE_DISPATCH_WHEN_ENV_VAR_ABSENT");
        }

        let actions = vec![HookAction::Call {
            command: "__claudine_missing_when_env_absent__".to_string(),
            args: None,
            timeout_ms: Some(50),
            mapper: None,
            when: Some("env.CLAUDINE_DISPATCH_WHEN_ENV_VAR_ABSENT".to_string()),
        }];

        let result = execute_actions(
            &actions,
            None,
            &make_meta_for_when_tests(),
            DispatchConfig::Canonical(&config),
            &messaging,
            true,
            None,
        )
        .await
        .unwrap();

        assert!(
            result.is_none(),
            "missing env var should evaluate falsy and skip the call action",
        );
    }

    #[tokio::test]
    async fn when_ctx_fields_do_not_require_precomputed_event_metadata() {
        // `ctx.*` paths are resolved lazily by Darkmatter's shortcut
        // lookup. The condition should evaluate without panicking even
        // though Claudine's EventMeta does not precompute these fields.
        let config = claudine_config_with_tts(TtsValue::Boolean(false));
        let messaging = RuntimeMessagingSettings::default();
        let actions = vec![HookAction::Call {
            command: "__claudine_missing_when_ctx_weak__".to_string(),
            args: None,
            timeout_ms: Some(50),
            mapper: None,
            when: Some("ctx.today != ''".to_string()),
        }];

        let result = execute_actions(
            &actions,
            None,
            &make_meta_for_when_tests(),
            DispatchConfig::Canonical(&config),
            &messaging,
            true,
            None,
        )
        .await;

        let response = result
            .expect("ctx.* condition should evaluate without erroring the runner")
            .expect("ctx.today != '' should be truthy and let the call fire");
        assert_eq!(response.decision, Some(HookDecision::Deny));
    }

    #[tokio::test]
    async fn when_ctx_today_resolves() {
        // Regression test: ctx.today should resolve truthy through the
        // EventMetaConditionLookup composite and allow the action to run.
        let config = claudine_config_with_tts(TtsValue::Boolean(false));
        let messaging = RuntimeMessagingSettings::default();
        let actions = vec![HookAction::Call {
            command: "__claudine_missing_when_ctx_today__".to_string(),
            args: None,
            timeout_ms: Some(50),
            mapper: None,
            when: Some("ctx.today != ''".to_string()),
        }];

        let result = execute_actions(
            &actions,
            None,
            &make_meta_for_when_tests(),
            DispatchConfig::Canonical(&config),
            &messaging,
            true,
            None,
        )
        .await
        .unwrap()
        .expect("ctx.today != '' should be truthy and let the call fire");

        assert_eq!(result.decision, Some(HookDecision::Deny));
    }

    // =========================================================================
    // `when` evaluation tests via EventMetaConditionLookup
    // =========================================================================

    #[tokio::test]
    async fn when_git_branch_matches_main_resolves_truthy() {
        let config = claudine_config_with_tts(TtsValue::Boolean(false));
        let messaging = RuntimeMessagingSettings::default();
        let actions = vec![HookAction::Call {
            command: "__claudine_missing_when_git_branch__".to_string(),
            args: None,
            timeout_ms: Some(50),
            mapper: None,
            when: Some("git.branch == 'main'".to_string()),
        }];

        let result = execute_actions(
            &actions,
            None,
            &meta_with_full_env(),
            DispatchConfig::Canonical(&config),
            &messaging,
            true,
            None,
        )
        .await
        .unwrap()
        .expect("git.branch == 'main' should resolve truthy and let the call fire");

        assert_eq!(result.decision, Some(HookDecision::Deny));
    }

    #[tokio::test]
    async fn when_git_is_dirty_resolves_as_boolean() {
        let config = claudine_config_with_tts(TtsValue::Boolean(false));
        let messaging = RuntimeMessagingSettings::default();
        let mut meta = meta_with_full_env();
        if let Some(git) = meta.env.git.as_mut() {
            git.is_dirty = false;
        }

        let actions = vec![HookAction::Call {
            command: "__claudine_missing_when_not_dirty__".to_string(),
            args: None,
            timeout_ms: Some(50),
            mapper: None,
            when: Some("!git.is_dirty".to_string()),
        }];

        let result = execute_actions(
            &actions,
            None,
            &meta,
            DispatchConfig::Canonical(&config),
            &messaging,
            true,
            None,
        )
        .await
        .unwrap()
        .expect("!git.is_dirty should resolve truthy when is_dirty is false");

        assert_eq!(result.decision, Some(HookDecision::Deny));
    }

    #[tokio::test]
    async fn when_hardware_cores_numeric_comparison() {
        // Pins that `hardware.cores` is exposed as a JSON Number, not a
        // string — otherwise `> 8` would fail to evaluate.
        let config = claudine_config_with_tts(TtsValue::Boolean(false));
        let messaging = RuntimeMessagingSettings::default();
        let actions = vec![HookAction::Call {
            command: "__claudine_missing_when_cores__".to_string(),
            args: None,
            timeout_ms: Some(50),
            mapper: None,
            when: Some("hardware.cores > 8".to_string()),
        }];

        let result = execute_actions(
            &actions,
            None,
            &meta_with_full_env(),
            DispatchConfig::Canonical(&config),
            &messaging,
            true,
            None,
        )
        .await
        .unwrap()
        .expect("hardware.cores > 8 should resolve truthy with cores=16");

        assert_eq!(result.decision, Some(HookDecision::Deny));
    }

    #[tokio::test]
    async fn when_project_language_matches() {
        let config = claudine_config_with_tts(TtsValue::Boolean(false));
        let messaging = RuntimeMessagingSettings::default();
        let actions = vec![HookAction::Call {
            command: "__claudine_missing_when_project_language__".to_string(),
            args: None,
            timeout_ms: Some(50),
            mapper: None,
            when: Some("project.language == 'Rust'".to_string()),
        }];

        let result = execute_actions(
            &actions,
            None,
            &meta_with_full_env(),
            DispatchConfig::Canonical(&config),
            &messaging,
            true,
            None,
        )
        .await
        .unwrap()
        .expect("project.language == 'Rust' should resolve truthy");

        assert_eq!(result.decision, Some(HookDecision::Deny));
    }

    #[tokio::test]
    async fn when_nested_tool_input_path() {
        // `tool_input` is a top-level field on EventMeta and resolves
        // directly through EventMetaExpressionLookup. This test guards
        // against regression in the expression evaluation path.
        let config = claudine_config_with_tts(TtsValue::Boolean(false));
        let messaging = RuntimeMessagingSettings::default();
        let mut meta = make_meta_for_when_tests();
        meta.tool_input = Some(serde_json::json!({"command": "npm test"}));

        let actions = vec![HookAction::Call {
            command: "__claudine_missing_when_tool_input__".to_string(),
            args: None,
            timeout_ms: Some(50),
            mapper: None,
            when: Some("tool_input.command == 'npm test'".to_string()),
        }];

        let result = execute_actions(
            &actions,
            None,
            &meta,
            DispatchConfig::Canonical(&config),
            &messaging,
            true,
            None,
        )
        .await
        .unwrap()
        .expect("nested tool_input.command should resolve truthy");

        assert_eq!(result.decision, Some(HookDecision::Deny));
    }

    #[tokio::test]
    async fn when_extra_dot_path_resolves() {
        let config = claudine_config_with_tts(TtsValue::Boolean(false));
        let messaging = RuntimeMessagingSettings::default();
        let mut meta = make_meta_for_when_tests();
        meta.extra
            .insert("attempt".to_string(), serde_json::json!(3));

        let actions = vec![HookAction::Call {
            command: "__claudine_missing_when_extra_attempt__".to_string(),
            args: None,
            timeout_ms: Some(50),
            mapper: None,
            when: Some("extra.attempt > 1".to_string()),
        }];

        let result = execute_actions(
            &actions,
            None,
            &meta,
            DispatchConfig::Canonical(&config),
            &messaging,
            true,
            None,
        )
        .await
        .unwrap()
        .expect("extra.attempt > 1 should resolve truthy with attempt=3");

        assert_eq!(result.decision, Some(HookDecision::Deny));
    }

    #[tokio::test]
    async fn when_tool_response_path_resolves() {
        let config = claudine_config_with_tts(TtsValue::Boolean(false));
        let messaging = RuntimeMessagingSettings::default();
        let mut meta = make_meta_for_when_tests();
        meta.tool_response = Some(serde_json::json!({"exit_code": 0}));

        let actions = vec![HookAction::Call {
            command: "__claudine_missing_when_tool_response__".to_string(),
            args: None,
            timeout_ms: Some(50),
            mapper: None,
            when: Some("tool_response.exit_code == 0".to_string()),
        }];

        let result = execute_actions(
            &actions,
            None,
            &meta,
            DispatchConfig::Canonical(&config),
            &messaging,
            true,
            None,
        )
        .await
        .unwrap()
        .expect("tool_response.exit_code == 0 should resolve truthy and let the call fire");

        assert_eq!(result.decision, Some(HookDecision::Deny));
    }

    #[tokio::test]
    async fn when_env_fallback_syntax_works() {
        let config = claudine_config_with_tts(TtsValue::Boolean(false));
        let messaging = RuntimeMessagingSettings::default();

        let actions = vec![HookAction::Call {
            command: "__claudine_missing_when_fallback__".to_string(),
            args: None,
            timeout_ms: Some(50),
            mapper: None,
            when: Some("env.CLAUDINE_TEST_MISSING || 'default' == 'default'".to_string()),
        }];

        let result = execute_actions(
            &actions,
            None,
            &make_meta_for_when_tests(),
            DispatchConfig::Canonical(&config),
            &messaging,
            true,
            None,
        )
        .await
        .unwrap()
        .expect("env fallback syntax should evaluate truthy and let the call fire");

        assert_eq!(result.decision, Some(HookDecision::Deny));
    }

    #[tokio::test]
    async fn when_ctx_year_resolves() {
        // Regression test: ctx.year should resolve truthy through the
        // EventMetaConditionLookup composite and allow the action to run.
        let config = claudine_config_with_tts(TtsValue::Boolean(false));
        let messaging = RuntimeMessagingSettings::default();
        let actions = vec![HookAction::Call {
            command: "__claudine_missing_when_ctx_year__".to_string(),
            args: None,
            timeout_ms: Some(50),
            mapper: None,
            when: Some("ctx.year != ''".to_string()),
        }];

        let result = execute_actions(
            &actions,
            None,
            &make_meta_for_when_tests(),
            DispatchConfig::Canonical(&config),
            &messaging,
            true,
            None,
        )
        .await
        .unwrap()
        .expect("ctx.year != '' should be truthy and let the call fire");

        assert_eq!(result.decision, Some(HookDecision::Deny));
    }

    #[tokio::test]
    async fn when_missing_git_block_is_falsy() {
        // When `meta.env.git` is None, `git.branch` resolves to Null
        // through EventMetaExpressionLookup and the condition is falsy.
        let config = claudine_config_with_tts(TtsValue::Boolean(false));
        let messaging = RuntimeMessagingSettings::default();
        let mut meta = make_meta_for_when_tests();
        meta.env.git = None;

        let actions = vec![HookAction::Call {
            command: "__claudine_missing_when_no_git__".to_string(),
            args: None,
            timeout_ms: Some(50),
            mapper: None,
            when: Some("git.branch == 'main'".to_string()),
        }];

        let result = execute_actions(
            &actions,
            None,
            &meta,
            DispatchConfig::Canonical(&config),
            &messaging,
            true,
            None,
        )
        .await
        .unwrap();

        assert!(
            result.is_none(),
            "missing git block must be falsy and skip the call action",
        );
    }
}
