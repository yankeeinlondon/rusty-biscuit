use std::path::PathBuf;
use std::sync::LazyLock;
use std::time::Duration;

#[cfg(test)]
use biscuit_speaks::SpeedLevel;
use biscuit_speaks::{TtsConfig, TtsFailoverStrategy};
use darkmatter::markdown::compose::conditions::evaluate_condition_against;
use regex::Regex;
use serde_json::{Map, Value};
use tokio::process::Command;
use tracing::{debug, info_span, warn};

/// Matches any Handlebars-style `{{...}}` placeholder so we can count
/// them when diagnosing whitespace-split interpolated values.
static BASH_PARAMS_PLACEHOLDER_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\{\{[^{}]*\}\}").expect("bash-params placeholder regex is valid")
});

/// Sentinel token substituted for each `{{...}}` during the diagnostic
/// "expected token count" render. Chosen to be unlikely to appear in any
/// real config and free of shell metacharacters.
const BASH_PARAMS_SENTINEL: &str = "__CLAUDINE_TEMPLATE_SENTINEL__";

/// Upper bound on how long a single bash action may run before the
/// dispatch loop gives up on it. Bash actions used to be fire-and-forget
/// via `tokio::spawn`, which silently lost work whenever Claudine was
/// invoked as a `handle` hook under [`CLAUDINE_HANDLE_DEADLINE_SECONDS`].
/// Awaiting inline with this bound lets the hook handler complete the
/// action reliably while still protecting long-lived wrapper sessions
/// from a hung bash action stalling the event pipeline.
const BASH_ACTION_TIMEOUT: Duration = Duration::from_secs(3);

use super::template::interpolate;
use crate::actions::bash_executor;
use crate::actions::{
    CompiledMapper, HookAction, HookDecision, HookResponse, Mapper, ReportFormat, ReportHandler,
};
use crate::config::claudine_config::{ClaudineConfig, Gender, TtsValue, VoiceSelection};
use crate::error::Result;
use crate::events::{AgenticEvent, EventMeta};
use crate::messaging::RuntimeMessagingSettings;
use crate::services::protect::decision::ProtectDecision;

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
/// the expression is parsed and evaluated through Darkmatter's
/// [`evaluate_condition_against`] shortcut against the event meta
/// serialized as JSON. Falsy results yield [`WhenOutcome::SkipFalse`];
/// parse or evaluation errors yield [`WhenOutcome::SkipInvalid`] with a
/// `tracing::warn!` so operators can spot a broken condition without
/// breaking the rest of the binding.
fn evaluate_when(when: Option<&str>, meta: &EventMeta, meta_json: &Value) -> WhenOutcome {
    let Some(expr) = when else {
        return WhenOutcome::Run;
    };

    let work_dir: PathBuf = meta
        .cwd
        .as_deref()
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    match evaluate_condition_against(expr, meta_json, work_dir.as_path()) {
        Ok(true) => WhenOutcome::Run,
        Ok(false) => WhenOutcome::SkipFalse,
        Err(error) => {
            warn!(
                expression = expr,
                %error,
                "Hook action `when` expression failed to parse or evaluate; skipping action",
            );
            WhenOutcome::SkipInvalid
        }
    }
}

/// Serialize an [`EventMeta`] to a JSON value for `when` evaluation.
///
/// Falls back to [`Value::Null`] on the (effectively unreachable)
/// serialization failure so a transient encoding issue cannot abort the
/// dispatch loop.
fn event_meta_to_json(meta: &EventMeta) -> Value {
    serde_json::to_value(meta).unwrap_or_else(|err| {
        warn!(%err, "serializing EventMeta for `when` evaluation failed; using null payload");
        Value::Null
    })
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
    let meta_json = event_meta_to_json(meta);

    for (index, action) in actions.iter().enumerate() {
        // Pre-execution `when` gate. Falsy or invalid conditions skip the
        // action without affecting `selected_response`, which guarantees
        // a skipped `Call` cannot replace a previously selected blocking
        // response.
        match evaluate_when(action.when(), meta, &meta_json) {
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

/// Speak a message using TTS with [`ClaudineConfig`]-aware voice resolution.
///
/// Voice selection priority:
/// 1. Action-level `voice` override (if present on the `Speak` action)
/// 2. Config-level voice resolved through gendered voice pairs
/// 3. Config-level single voice
/// 4. Auto-detect (no explicit voice)
fn execute_speak_from_claudine(
    message_template: &str,
    voice_override: Option<&str>,
    gender_override: Option<Gender>,
    meta: &EventMeta,
    config: &ClaudineConfig,
) {
    let text = interpolate(message_template, meta);
    if text.is_empty() {
        return;
    }

    if matches!(config.tts, TtsValue::Boolean(false)) {
        return;
    }

    let tts = tts_config_from_claudine(config, voice_override, gender_override);

    tokio::spawn(async move {
        if let Err(error) = biscuit_speaks::Speak::new(text)
            .with_config(tts)
            .play()
            .await
        {
            warn!(%error, "TTS playback failed");
        }
    });
}

/// Build a [`TtsConfig`] from [`ClaudineConfig`] with optional per-action overrides.
///
/// Resolution order for voice:
/// 1. `voice_override` from the action (always wins)
/// 2. Gendered voice pair from config, using action `gender_override` → config default gender
/// 3. Single voice from config
/// 4. No explicit voice (auto-detect)
fn tts_config_from_claudine(
    config: &ClaudineConfig,
    voice_override: Option<&str>,
    gender_override: Option<Gender>,
) -> TtsConfig {
    let mut tts = TtsConfig::new();

    if let Some(voice) = voice_override {
        tts = tts.with_voice(voice);
    }

    match &config.tts {
        TtsValue::Boolean(false) => {}
        TtsValue::Boolean(true) => {}
        TtsValue::Config(settings) => {
            if voice_override.is_none() {
                if let Some(provider) = biscuit_speaks::parse_provider_name(&settings.provider) {
                    tts = tts.with_failover(TtsFailoverStrategy::SpecificProvider(provider));
                }

                let gender = gender_override.unwrap_or(settings.gender);
                match &settings.voice {
                    Some(VoiceSelection::Single(v)) => {
                        tts = tts.with_voice(v);
                    }
                    Some(VoiceSelection::Gendered { male, female }) => {
                        let voice = match gender {
                            Gender::Male => male.as_str(),
                            Gender::Female => female.as_str(),
                        };
                        tts = tts.with_voice(voice);
                    }
                    None => {}
                }
            } else if let Some(provider) = biscuit_speaks::parse_provider_name(&settings.provider) {
                tts = tts.with_failover(TtsFailoverStrategy::SpecificProvider(provider));
            }
        }
    }

    tts
}

fn should_replace_selected(current: Option<&HookResponse>, candidate: &HookResponse) -> bool {
    match current {
        None => true,
        Some(existing) => {
            let existing_is_continue = matches!(existing.decision, Some(HookDecision::Continue));
            let candidate_is_continue = matches!(candidate.decision, Some(HookDecision::Continue));
            existing_is_continue && !candidate_is_continue
        }
    }
}

fn should_short_circuit_call(decision: Option<&ProtectDecision>) -> bool {
    decision.is_some_and(|d| d.is_blocked())
}

fn decision_for_short_circuit(_decision: &ProtectDecision) -> HookDecision {
    HookDecision::Deny
}

fn attach_protect_context(
    response: HookResponse,
    _protect_decision: Option<&ProtectDecision>,
) -> HookResponse {
    // Simplified: new protect doesn't attach context to responses
    response
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

/// Speak a message via biscuit-speaks TTS (fire-and-forget).
#[cfg(test)]
fn tts_config_from_settings(settings: Option<&crate::events::TtsSettings>) -> TtsConfig {
    let mut config = TtsConfig::new();
    let Some(settings) = settings else {
        return config;
    };

    if let Some(voice) = settings.voice.as_deref() {
        config = config.with_voice(voice);
    }

    if let Some(rate) = settings.rate {
        config = config.with_speed(SpeedLevel::Explicit(rate));
    }

    if let Some(provider) = settings.provider.as_deref() {
        if let Some(provider) = biscuit_speaks::parse_provider_name(provider) {
            config = config.with_failover(TtsFailoverStrategy::SpecificProvider(provider));
        } else {
            warn!(
                provider,
                "Unknown TTS provider in settings; falling back to automatic provider selection"
            );
        }
    }

    config
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

/// Execute a validated command asynchronously using direct spawning.
///
/// ## Interpolation Contract
///
/// Template placeholders in `command` and `params` are expanded by
/// [`interpolate`] as raw string substitution. The rendered `params`
/// string is then split by [`shell_words::split`] into discrete `argv`
/// entries. This means:
///
/// - Interpolated values that contain spaces **will** split into
///   multiple arguments unless the config author quoted them in the
///   `params` template (e.g., `--message '{{tool_name}}'`).
/// - No shell metacharacter interpretation occurs because the command
///   is spawned directly via `Command::new().args()`, not through
///   `sh -c`.
/// - The `shell_escape()` helper in `bash_executor` is intentionally
///   not used here — it is for callers that build `sh -c` strings.
async fn execute_bash(command: &str, params: &str, meta: &EventMeta) {
    let cmd = interpolate(command, meta);

    let validated = match bash_executor::validate_command(&cmd) {
        Ok(v) => v,
        Err(error) => {
            warn!(%cmd, %error, "Bash action blocked by validation");
            return;
        }
    };

    let rendered_params = interpolate(params, meta);

    // Parse rendered_params using shell-words to correctly handle quoted arguments
    // and interpolated values containing spaces (e.g., `--message 'hello world'`).
    let param_args: Vec<String> = if rendered_params.is_empty() {
        vec![]
    } else {
        match shell_words::split(&rendered_params) {
            Ok(args) => args,
            Err(e) => {
                warn!(%rendered_params, %e, "Failed to parse bash action params");
                return;
            }
        }
    };

    // Diagnostic: if the rendered params split into more tokens than the
    // same template with every placeholder replaced by a single sentinel
    // word, an interpolated value contained whitespace that the author
    // likely did not intend (e.g. `--message {{tool_name}}` with
    // `tool_name = "my tool"` silently becomes `["--message", "my",
    // "tool"]`). The canonical fix is to quote the placeholder in the
    // template. We only warn, so existing configs keep working.
    warn_on_silent_token_split(params, &rendered_params, &param_args);

    debug!(?validated, ?param_args, "Awaiting bash action inline");

    // Run inline with a bounded timeout. Previously this was a
    // `tokio::spawn(...)` fire-and-forget, which silently lost work
    // whenever the dispatch completed before the spawned task ran
    // (notably the `handle` hook path under `CLAUDINE_HANDLE_DEADLINE_SECONDS`).
    let action = async {
        match &validated {
            bash_executor::ValidatedCommand::Direct(executable) => {
                Command::new(executable).args(&param_args).output().await
            }
            bash_executor::ValidatedCommand::Interpreted {
                interpreter,
                interpreter_args,
                script,
            } => {
                let mut cmd = Command::new(interpreter);
                cmd.args(interpreter_args);
                cmd.arg(script);
                cmd.args(&param_args);
                cmd.output().await
            }
        }
    };
    match tokio::time::timeout(BASH_ACTION_TIMEOUT, action).await {
        Ok(Ok(output)) if !output.status.success() => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!(?validated, %stderr, "Bash action exited with non-zero status");
        }
        Ok(Ok(_)) => {}
        Ok(Err(error)) => {
            warn!(?validated, %error, "Bash action failed to spawn");
        }
        Err(_) => {
            warn!(
                ?validated,
                timeout_ms = BASH_ACTION_TIMEOUT.as_millis() as u64,
                "Bash action timed out",
            );
        }
    }
}

/// Emit a diagnostic `warn!` when an interpolated bash-action param split
/// on whitespace in a way the template author likely did not intend.
///
/// We render the template a second time with each `{{...}}` replaced by
/// [`BASH_PARAMS_SENTINEL`], split both forms via `shell_words`, and
/// compare token counts. A mismatch means the interpolation introduced
/// extra argv slots — useful to surface because the resulting command
/// runs silently with the wrong arguments.
fn warn_on_silent_token_split(template: &str, rendered: &str, actual: &[String]) {
    if template.is_empty() {
        return;
    }
    let sentinel_template = BASH_PARAMS_PLACEHOLDER_RE
        .replace_all(template, BASH_PARAMS_SENTINEL)
        .into_owned();
    let Ok(expected_tokens) = shell_words::split(&sentinel_template) else {
        return;
    };
    if expected_tokens.len() != actual.len() {
        warn!(
            template,
            rendered,
            expected_tokens = expected_tokens.len(),
            actual_tokens = actual.len(),
            "Bash action param template split differently after interpolation. \
             Quote your placeholders (e.g. `'{{value}}'`) to keep each one in a \
             single argv slot, or migrate the config to an array-form `params` \
             so interpolation happens per-item."
        );
    }
}

fn execute_report(handler: Option<&ReportHandler>, meta: &EventMeta, blocking: bool) {
    let output = match handler {
        Some(handler) => format_report(handler, meta),
        None => format!(
            "[{}] {} ({})",
            meta.event.as_pascal_case(),
            meta.tool_name.as_deref().unwrap_or("-"),
            meta.provider
        ),
    };
    // Route report output to stderr on blocking events to avoid corrupting
    // the machine-facing provider response payload on stdout.
    if blocking {
        eprintln!("{output}");
    } else {
        println!("{output}");
    }
}

fn terminal_meta_json(meta: &EventMeta) -> String {
    let mut value = terminal_meta_value(meta);
    strip_nulls(&mut value);
    serde_json::to_string(&value).unwrap_or_else(|err| {
        warn!(%err, "serializing terminal event metadata failed; falling back to empty object");
        "{}".to_string()
    })
}

fn terminal_meta_value(meta: &EventMeta) -> Value {
    let mut object = Map::new();

    object.insert(
        "provider".to_string(),
        Value::String(meta.provider.as_slug().to_string()),
    );
    object.insert(
        "event".to_string(),
        Value::String(meta.event.as_pascal_case().to_string()),
    );
    object.insert(
        "timestamp".to_string(),
        serde_json::to_value(meta.timestamp).unwrap_or_else(|err| {
            warn!(%err, "serializing event timestamp failed; substituting null");
            Value::Null
        }),
    );

    if let Some(value) = &meta.session_id {
        object.insert("session_id".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = &meta.cwd {
        object.insert("cwd".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = &meta.tool_name {
        object.insert("tool_name".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = &meta.tool_input {
        object.insert("tool_input".to_string(), value.clone());
    }
    if let Some(value) = &meta.tool_response {
        object.insert("tool_response".to_string(), value.clone());
    }
    if let Some(value) = &meta.error {
        object.insert("error".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = &meta.prompt {
        object.insert("prompt".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = &meta.agent_type {
        object.insert("agent_type".to_string(), Value::String(value.clone()));
    }
    if let Some(value) = &meta.notification_type {
        object.insert(
            "notification_type".to_string(),
            Value::String(value.clone()),
        );
    }
    if let Some(value) = &meta.notification_message {
        object.insert(
            "notification_message".to_string(),
            Value::String(value.clone()),
        );
    }

    object.insert(
        "extra".to_string(),
        serde_json::to_value(&meta.extra).unwrap_or_else(|err| {
            warn!(%err, "serializing event `extra` map failed; substituting empty object");
            Value::Object(Map::new())
        }),
    );
    object.insert(
        "env".to_string(),
        serde_json::to_value(&meta.env).unwrap_or_else(|err| {
            warn!(%err, "serializing event `env` failed; substituting null");
            Value::Null
        }),
    );

    Value::Object(object)
}

fn strip_nulls(value: &mut Value) {
    strip_nulls_recursive(value, 0);
}

const STRIP_NULLS_MAX_DEPTH: u32 = 64;

fn strip_nulls_recursive(value: &mut Value, depth: u32) {
    if depth >= STRIP_NULLS_MAX_DEPTH {
        return;
    }
    match value {
        Value::Object(map) => {
            let mut to_remove = Vec::new();
            for (key, nested) in map.iter_mut() {
                strip_nulls_recursive(nested, depth + 1);
                if nested.is_null() {
                    to_remove.push(key.clone());
                }
            }
            for key in to_remove {
                map.remove(&key);
            }
        }
        Value::Array(items) => {
            for nested in items.iter_mut() {
                strip_nulls_recursive(nested, depth + 1);
            }
            items.retain(|item| !item.is_null());
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

fn format_report(handler: &ReportHandler, meta: &EventMeta) -> String {
    if let Some(template) = &handler.template {
        let mut output = interpolate(template, meta);
        if handler.include_metadata {
            let json = terminal_meta_json(meta);
            output.push(' ');
            output.push_str(&json);
        }
        return output;
    }

    match handler.format {
        ReportFormat::Json => terminal_meta_json(meta),
        ReportFormat::Compact => format!(
            "[{}] {}",
            meta.event.as_pascal_case(),
            meta.tool_name.as_deref().unwrap_or("-")
        ),
        ReportFormat::Text => format!(
            "Event: {}, Provider: {}, Tool: {}",
            meta.event.as_pascal_case(),
            meta.provider,
            meta.tool_name.as_deref().unwrap_or("-")
        ),
    }
}

#[derive(Debug)]
struct CommandOutput {
    status: std::process::ExitStatus,
    stdout: String,
    stderr: String,
}

async fn run_command_blocking(command: &str, args: Option<&[String]>) -> Result<CommandOutput> {
    if sniff::programs::find_program(command).is_none() {
        return Err(crate::error::ClaudineError::LinkingError(format!(
            "command not found on PATH: {command}"
        )));
    }

    let mut cmd = Command::new(command);
    if let Some(args) = args {
        cmd.args(args);
    }

    let output = cmd.output().await?;
    Ok(CommandOutput {
        status: output.status,
        stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
    })
}

fn apply_mapper(
    compiled_mapper: Option<&CompiledMapper>,
    fallback_mapper: Option<&Mapper>,
    output: &CommandOutput,
) -> Result<HookResponse> {
    if let Some(compiled_mapper) = compiled_mapper {
        return match compiled_mapper {
            CompiledMapper::JsonField { field } => map_json_field(field, output),
            CompiledMapper::JsonObject => map_json_object(output),
            CompiledMapper::ExitCode => Ok(map_exit_code(output)),
            CompiledMapper::Regex { pattern } => map_regex_with_compiled(pattern, output),
        };
    }

    match fallback_mapper.unwrap_or(&Mapper::ExitCode) {
        Mapper::ExitCode => Ok(map_exit_code(output)),
        Mapper::JsonField { field } => map_json_field(field, output),
        Mapper::JsonObject => map_json_object(output),
        Mapper::Regex { pattern } => {
            tracing::debug!(
                pattern,
                "apply_mapper: compiled_mapper not provided, falling back to per-call regex compilation"
            );
            let regex = Regex::new(pattern)?;
            map_regex_with_compiled(&regex, output)
        }
    }
}

/// Map a `Call` action's child exit status onto a `HookResponse`.
///
/// - `0` → [`HookDecision::Allow`].
/// - `2` → [`HookDecision::Deny`].
/// - Anything else → no decision (the caller falls through to the next
///   action). A `tracing::warn!` is emitted so operators can distinguish
///   "command crashed" (`139`), "not found" (`127`), "permission denied"
///   (`126`), and user-defined anomalous exits from a clean success.
fn map_exit_code(output: &CommandOutput) -> HookResponse {
    let code = output.status.code().unwrap_or(1);
    let decision = match code {
        0 => Some(HookDecision::Allow),
        2 => Some(HookDecision::Deny),
        _ => {
            warn!(
                exit_code = code,
                stderr = %output.stderr,
                "map_exit_code: unexpected exit status — no decision emitted",
            );
            None
        }
    };

    let reason = if !output.stdout.is_empty() {
        Some(output.stdout.clone())
    } else if !output.stderr.is_empty() {
        Some(output.stderr.clone())
    } else {
        None
    };

    HookResponse {
        decision,
        reason,
        ..HookResponse::default()
    }
}

fn map_json_field(field: &str, output: &CommandOutput) -> Result<HookResponse> {
    let parsed: serde_json::Value = serde_json::from_str(&output.stdout)?;
    let value = dot_lookup(&parsed, field).ok_or_else(|| {
        crate::error::ClaudineError::TemplateError(format!("mapper field not found: {field}"))
    })?;

    let decision = parse_decision(value);
    let reason = parsed
        .get("reason")
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned);

    Ok(HookResponse {
        decision,
        reason,
        ..HookResponse::default()
    })
}

fn map_json_object(output: &CommandOutput) -> Result<HookResponse> {
    if output.stdout.is_empty() {
        return Ok(HookResponse::default());
    }

    if let Ok(response) = serde_json::from_str::<HookResponse>(&output.stdout) {
        return Ok(response);
    }

    let raw = serde_json::from_str::<serde_json::Value>(&output.stdout)?;
    Ok(HookResponse {
        raw: Some(raw),
        ..HookResponse::default()
    })
}

fn map_regex_with_compiled(regex: &Regex, output: &CommandOutput) -> Result<HookResponse> {
    let captures = regex.captures(&output.stdout).ok_or_else(|| {
        crate::error::ClaudineError::TemplateError("regex mapper produced no match".to_string())
    })?;

    let decision = captures
        .name("decision")
        .map(|capture| parse_decision(&serde_json::Value::String(capture.as_str().to_string())))
        .unwrap_or(None);
    let reason = captures
        .name("reason")
        .map(|capture| capture.as_str().to_string());

    Ok(HookResponse {
        decision,
        reason,
        additional_context: captures
            .name("context")
            .map(|capture| capture.as_str().to_string()),
        ..HookResponse::default()
    })
}

fn parse_decision(value: &serde_json::Value) -> Option<HookDecision> {
    let text = value.as_str()?.to_ascii_lowercase();
    match text.as_str() {
        "allow" | "approved" | "approve" => Some(HookDecision::Allow),
        "deny" | "denied" | "reject" | "rejected" => Some(HookDecision::Deny),
        "ask" => Some(HookDecision::Ask),
        "continue" => Some(HookDecision::Continue),
        _ => None,
    }
}

fn dot_lookup<'a>(value: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
    path.split('.').try_fold(value, |acc, key| acc.get(key))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::os::unix::process::ExitStatusExt;

    use biscuit_speaks::{HostTtsProvider, TtsFailoverStrategy, TtsProvider};
    use chrono::Utc;

    use super::*;
    use crate::config::claudine_config::{
        ClaudineConfig, Gender, TtsConfigSettings, TtsValue, VoiceSelection,
    };
    use crate::events::{EnvironmentContext, TtsSettings};
    use crate::provider::Provider;
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

    #[test]
    fn mapper_exit_code_deny() {
        let output = CommandOutput {
            status: std::process::ExitStatus::from_raw(2 << 8),
            stdout: "blocked".to_string(),
            stderr: String::new(),
        };

        let mapped = apply_mapper(None, None, &output).unwrap();
        assert_eq!(mapped.decision, Some(HookDecision::Deny));
        assert_eq!(mapped.reason.as_deref(), Some("blocked"));
    }

    #[test]
    fn mapper_exit_code_allow_on_zero() {
        let output = CommandOutput {
            status: std::process::ExitStatus::from_raw(0),
            stdout: "ok".to_string(),
            stderr: String::new(),
        };
        let mapped = apply_mapper(None, None, &output).unwrap();
        assert_eq!(mapped.decision, Some(HookDecision::Allow));
    }

    #[test]
    fn mapper_exit_code_unknown_is_none_and_warns() {
        // Exit codes other than 0/2 (crashes, command-not-found, etc.)
        // must NOT silently map to Allow. The dispatcher instead emits
        // no decision and logs a warn!, so the caller falls through to
        // the next action.
        for code in [1_i32, 127, 139, 200] {
            let output = CommandOutput {
                status: std::process::ExitStatus::from_raw(code << 8),
                stdout: String::new(),
                stderr: format!("boom at {code}"),
            };
            let mapped = apply_mapper(None, None, &output).unwrap();
            assert_eq!(
                mapped.decision, None,
                "unexpected exit code {code} must not produce a decision",
            );
            assert_eq!(mapped.reason.as_deref(), Some(&*format!("boom at {code}")));
        }
    }

    #[test]
    fn mapper_json_field() {
        let output = CommandOutput {
            status: std::process::ExitStatus::from_raw(0),
            stdout: r#"{"decision":"deny","reason":"nope"}"#.to_string(),
            stderr: String::new(),
        };

        let mapped = apply_mapper(
            None,
            Some(&Mapper::JsonField {
                field: "decision".to_string(),
            }),
            &output,
        )
        .unwrap();

        assert_eq!(mapped.decision, Some(HookDecision::Deny));
        assert_eq!(mapped.reason.as_deref(), Some("nope"));
    }

    #[test]
    fn stop_overrides_continue() {
        let continue_response = HookResponse {
            decision: Some(HookDecision::Continue),
            ..HookResponse::default()
        };
        let deny_response = HookResponse {
            decision: Some(HookDecision::Deny),
            ..HookResponse::default()
        };

        assert!(should_replace_selected(
            Some(&continue_response),
            &deny_response
        ));
        assert!(!should_replace_selected(
            Some(&deny_response),
            &continue_response
        ));
    }

    #[test]
    fn terminal_meta_json_uses_pascal_case_and_omits_none() {
        let json = terminal_meta_json(&meta());
        let value: Value = serde_json::from_str(&json).unwrap();

        assert_eq!(value["event"], "BeforeTool");
        assert_eq!(value["provider"], "claude");
        assert!(value.get("tool_input").is_none());
        assert!(value.get("tool_response").is_none());
        assert!(value.get("error").is_none());
        assert!(value.get("prompt").is_none());
        assert!(value.get("notification_type").is_none());
        assert!(value.get("notification_message").is_none());
        assert!(value["env"].get("git").is_none());
        assert!(value["env"].get("repo").is_none());
        assert!(value["env"].get("primary_language").is_none());
    }

    #[test]
    fn report_json_uses_terminal_serialization() {
        let output = format_report(
            &ReportHandler {
                format: ReportFormat::Json,
                template: None,
                include_metadata: false,
            },
            &meta(),
        );

        let value: Value = serde_json::from_str(&output).unwrap();
        assert_eq!(value["event"], "BeforeTool");
        assert!(value.get("tool_input").is_none());
    }

    #[test]
    fn report_template_resolves_darkmatter_expressions() {
        // Verifies the runner-level template path uses the shared expression
        // engine (Darkmatter) end-to-end: a fallback expression and a simple
        // variable both resolve correctly via format_report.
        let output = format_report(
            &ReportHandler {
                format: ReportFormat::Text,
                template: Some("{{provider}} ran {{tool_name || \"unknown-tool\"}}".to_string()),
                include_metadata: false,
            },
            &meta(),
        );
        assert_eq!(output, "claude ran Bash");
    }

    #[test]
    fn tts_config_applies_provider_voice_and_rate() {
        let settings = TtsSettings {
            provider: Some("say".to_string()),
            voice: Some("Samantha".to_string()),
            rate: Some(1.4),
        };

        let config = tts_config_from_settings(Some(&settings));

        assert_eq!(config.requested_voice.as_deref(), Some("Samantha"));
        assert_eq!(config.speed, SpeedLevel::Explicit(1.4));
        assert!(matches!(
            config.failover_strategy,
            TtsFailoverStrategy::SpecificProvider(TtsProvider::Host(HostTtsProvider::Say))
        ));
    }

    #[test]
    fn tts_config_keeps_default_failover_for_unknown_provider() {
        let settings = TtsSettings {
            provider: Some("not-a-provider".to_string()),
            voice: None,
            rate: None,
        };

        let config = tts_config_from_settings(Some(&settings));
        assert!(matches!(
            config.failover_strategy,
            TtsFailoverStrategy::FirstAvailable
        ));
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

    // =========================================================================
    // execute_actions and tts_config_from_claudine tests
    // =========================================================================

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

    #[test]
    fn tts_config_from_claudine_boolean_true_is_auto_detect() {
        let config = claudine_config_with_tts(TtsValue::Boolean(true));
        let tts = tts_config_from_claudine(&config, None, None);
        assert!(matches!(
            tts.failover_strategy,
            TtsFailoverStrategy::FirstAvailable
        ));
        assert!(tts.requested_voice.is_none());
    }

    #[test]
    fn tts_config_from_claudine_boolean_false_returns_default() {
        let config = claudine_config_with_tts(TtsValue::Boolean(false));
        let tts = tts_config_from_claudine(&config, None, None);
        assert!(tts.requested_voice.is_none());
    }

    #[test]
    fn tts_config_from_claudine_single_voice() {
        let config = claudine_config_with_tts(TtsValue::Config(TtsConfigSettings {
            provider: "say".to_string(),
            voice: Some(VoiceSelection::Single("Samantha".to_string())),
            gender: Gender::Female,
        }));
        let tts = tts_config_from_claudine(&config, None, None);
        assert_eq!(tts.requested_voice.as_deref(), Some("Samantha"));
        assert!(matches!(
            tts.failover_strategy,
            TtsFailoverStrategy::SpecificProvider(_)
        ));
    }

    #[test]
    fn tts_config_from_claudine_gendered_voice_female_default() {
        let config = claudine_config_with_tts(TtsValue::Config(TtsConfigSettings {
            provider: "say".to_string(),
            voice: Some(VoiceSelection::Gendered {
                male: "Alex".to_string(),
                female: "Samantha".to_string(),
            }),
            gender: Gender::Female,
        }));
        let tts = tts_config_from_claudine(&config, None, None);
        assert_eq!(tts.requested_voice.as_deref(), Some("Samantha"));
    }

    #[test]
    fn tts_config_from_claudine_gendered_voice_male_default() {
        let config = claudine_config_with_tts(TtsValue::Config(TtsConfigSettings {
            provider: "say".to_string(),
            voice: Some(VoiceSelection::Gendered {
                male: "Alex".to_string(),
                female: "Samantha".to_string(),
            }),
            gender: Gender::Male,
        }));
        let tts = tts_config_from_claudine(&config, None, None);
        assert_eq!(tts.requested_voice.as_deref(), Some("Alex"));
    }

    #[test]
    fn tts_config_from_claudine_gender_override_selects_male_voice() {
        let config = claudine_config_with_tts(TtsValue::Config(TtsConfigSettings {
            provider: "say".to_string(),
            voice: Some(VoiceSelection::Gendered {
                male: "Alex".to_string(),
                female: "Samantha".to_string(),
            }),
            gender: Gender::Female,
        }));
        let tts = tts_config_from_claudine(&config, None, Some(Gender::Male));
        assert_eq!(tts.requested_voice.as_deref(), Some("Alex"));
    }

    #[test]
    fn tts_config_from_claudine_voice_override_wins_over_config() {
        let config = claudine_config_with_tts(TtsValue::Config(TtsConfigSettings {
            provider: "say".to_string(),
            voice: Some(VoiceSelection::Single("Samantha".to_string())),
            gender: Gender::Female,
        }));
        let tts = tts_config_from_claudine(&config, Some("Karen"), None);
        assert_eq!(tts.requested_voice.as_deref(), Some("Karen"));
    }

    #[test]
    fn tts_config_from_claudine_voice_override_sets_provider() {
        let config = claudine_config_with_tts(TtsValue::Config(TtsConfigSettings {
            provider: "say".to_string(),
            voice: Some(VoiceSelection::Single("Samantha".to_string())),
            gender: Gender::Female,
        }));
        let tts = tts_config_from_claudine(&config, Some("Karen"), None);
        assert_eq!(tts.requested_voice.as_deref(), Some("Karen"));
        assert!(matches!(
            tts.failover_strategy,
            TtsFailoverStrategy::SpecificProvider(_)
        ));
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

    fn make_meta_for_when_tests() -> EventMeta {
        let mut m = meta();
        m.tool_name = Some("Bash".to_string());
        m
    }

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
        let actions = vec![HookAction::Report {
            handler: None,
            when: Some("ctx.today != ''".to_string()),
        }];

        let result = execute_actions(
            &actions,
            None,
            &make_meta_for_when_tests(),
            DispatchConfig::Canonical(&config),
            &messaging,
            false,
            None,
        )
        .await;

        assert!(
            result.is_ok(),
            "ctx.* condition should evaluate without erroring the runner",
        );
    }

    // =========================================================================
    // shell_words argv interpolation contract tests
    // =========================================================================

    #[test]
    fn shell_words_preserves_spaces_in_quoted_params() {
        let raw = "--message 'my tool'";
        let args = shell_words::split(raw).unwrap();
        assert_eq!(args, vec!["--message", "my tool"]);
    }

    #[test]
    fn shell_words_splits_unquoted_spaces() {
        let raw = "--message my tool";
        let args = shell_words::split(raw).unwrap();
        assert_eq!(args, vec!["--message", "my", "tool"]);
    }

    #[test]
    fn shell_words_handles_metacharacters_safely() {
        let raw = "--path /tmp/$(whoami)";
        let args = shell_words::split(raw).unwrap();
        assert_eq!(args, vec!["--path", "/tmp/$(whoami)"]);
    }

    #[test]
    fn shell_words_handles_quotes_in_values() {
        let raw = r#"--message "it's a test""#;
        let args = shell_words::split(raw).unwrap();
        assert_eq!(args, vec!["--message", "it's a test"]);
    }

    #[test]
    fn shell_words_empty_params_produces_no_args() {
        let raw = "";
        let args: Vec<String> = if raw.is_empty() {
            vec![]
        } else {
            shell_words::split(raw).unwrap()
        };
        assert!(args.is_empty());
    }

    // =========================================================================
    // Bash execution security tests
    // =========================================================================

    #[test]
    fn validate_command_advisory_for_rm() {
        // `BLOCKED_COMMANDS` is an advisory speed bump only; `rm` is no
        // longer rejected here. Real command gating belongs to
        // `ProtectService`.
        if which::which("rm").is_ok() {
            assert!(bash_executor::validate_command("rm").is_ok());
        }
    }

    #[test]
    fn validate_command_allows_echo() {
        assert!(bash_executor::validate_command("echo").is_ok());
    }

    #[test]
    fn shell_escape_neutralizes_injection() {
        let escaped = bash_executor::shell_escape("$(rm -rf /)");
        assert_eq!(escaped, "'$(rm -rf /)'");
    }

    #[test]
    fn shell_escape_handles_semicolon_injection() {
        let escaped = bash_executor::shell_escape("; rm -rf /");
        assert_eq!(escaped, "'; rm -rf /'");
    }

    #[test]
    fn shell_escape_handles_backtick_injection() {
        let escaped = bash_executor::shell_escape("`rm -rf /`");
        assert_eq!(escaped, "'`rm -rf /`'");
    }
}
