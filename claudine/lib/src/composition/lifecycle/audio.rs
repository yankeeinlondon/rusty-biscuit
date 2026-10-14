/// Whether `name` is an embedded Playa sound effect.
///
/// The parser checks literal `effect` names through this predicate, which its
/// caller supplies, so the lift-ready parser slice names no Playa type.
pub(super) fn is_known_effect(name: &str) -> bool {
    playa::SoundEffect::from_name(name).is_some()
}

impl LifecycleSignal {
    /// Returns the frontmatter property name for this signal.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use claudine::composition::LifecycleSignal;
    /// assert_eq!(LifecycleSignal::Start.property_name(), "start");
    /// assert_eq!(LifecycleSignal::Loop.property_name(), "loop");
    /// ```
    pub fn property_name(&self) -> &'static str {
        match self {
            Self::Initialize => "initialize",
            Self::Start => "start",
            Self::Success => "success",
            Self::Blocked => "blocked",
            Self::Failure => "failure",
            Self::Finalize => "finalize",
            Self::Loop => "loop",
        }
    }

    /// Returns the status state for this signal.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use claudine::composition::LifecycleSignal;
    /// # use biscuit_terminal::components::status::StatusState;
    /// assert_eq!(LifecycleSignal::Start.status_state(), StatusState::Info);
    /// assert_eq!(LifecycleSignal::Success.status_state(), StatusState::Success);
    /// assert_eq!(LifecycleSignal::Blocked.status_state(), StatusState::Error);
    /// assert_eq!(LifecycleSignal::Failure.status_state(), StatusState::Error);
    /// ```
    pub fn status_state(&self) -> StatusState {
        match self {
            Self::Initialize => StatusState::Info,
            Self::Start => StatusState::Info,
            Self::Success => StatusState::Success,
            Self::Blocked | Self::Failure => StatusState::Error,
            Self::Finalize => StatusState::Info,
            Self::Loop => StatusState::Info,
        }
    }

    /// Whether this event can ever observe an error and therefore may
    /// legitimately reference the lifecycle-stack-only `err` global.
    ///
    /// Per the spec's `err` static-scan rule:
    /// - `blocked` and `failure` always carry an error.
    /// - `finalize` *optionally* carries an error (success path: no error;
    ///   failure path: error present).
    /// - `initialize`, `start`, `success`, `loop` never carry an error.
    pub const fn can_carry_error(self) -> bool {
        matches!(self, Self::Blocked | Self::Failure | Self::Finalize)
    }

    /// Whether an *unintentional* action error during this event's stack
    /// must route the run to the `failure` event.
    ///
    /// Per the spec's action error-propagation table, setup-phase events
    /// (`initialize`, `start`, `blocked`) propagate an errored action to
    /// `failure` so the agent is never invoked with a broken environment.
    /// Terminal-phase events (`success`, `failure`, `finalize`, `loop`) log
    /// the error but leave the composition outcome unchanged.
    ///
    /// This governs only unintentional action errors. The explicit `Error`
    /// lifecycle action is a deliberate author choice and follows the
    /// separate "Where valid" transition table.
    pub const fn routes_action_error_to_failure(self) -> bool {
        matches!(self, Self::Initialize | Self::Start | Self::Blocked)
    }
}

/// Build the TTS configuration lifecycle speech uses from global settings.
///
/// Pre-warm builds its candidates with this same function: a prepared phrase
/// is reused only when playback resolves an identical configuration.
pub(crate) fn tts_config_from_settings(tts: Option<&TtsSettings>) -> TtsConfig {
    let mut config = TtsConfig::new();
    let Some(settings) = tts else {
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
                "Unknown TTS provider in settings; using automatic selection"
            );
        }
    }
    config
}

pub(super) fn enqueue_speech(text: &str, config: TtsConfig) {
    let text = text.to_string();
    let handle = tokio::runtime::Handle::try_current().ok();
    let result = std::thread::scope(|scope| {
        scope
            .spawn(move || {
                let speech = biscuit_speaks::Speak::new(text).with_config(config);
                if let Some(handle) = handle {
                    handle.block_on(speech.play_detached())
                } else {
                    let runtime = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .map_err(biscuit_speaks::TtsError::from)?;
                    runtime.block_on(speech.play_detached())
                }
            })
            .join()
    });
    match result {
        Ok(Ok(_)) => {}
        Ok(Err(error)) => warn!(%error, "Lifecycle TTS handoff failed"),
        Err(_) => warn!("Lifecycle TTS handoff panicked"),
    }
}

pub(super) fn enqueue_effect(name: &str) {
    let Some(effect) = playa::SoundEffect::from_name(name) else {
        warn!(%name, "Unknown sound effect in lifecycle notification");
        return;
    };
    match playa::Playa::from_bytes(effect.bytes().to_vec()) {
        Ok(player) => {
            if let Err(error) = player.play_detached() {
                warn!(%error, "Lifecycle sound effect handoff failed");
            }
        }
        Err(error) => warn!(%error, "Failed to construct lifecycle sound effect"),
    }
}

use super::*;
