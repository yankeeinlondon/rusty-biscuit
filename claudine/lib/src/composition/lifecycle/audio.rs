/// Whether `name` is an embedded Playa sound effect.
///
/// The parser checks literal `effect` names through this predicate, which its
/// caller supplies, so Darkmatter's lifecycle parser names no Playa type.
pub(super) fn is_known_effect(name: &str) -> bool {
    playa::SoundEffect::from_name(name).is_some()
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
