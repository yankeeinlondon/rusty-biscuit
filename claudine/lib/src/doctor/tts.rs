//! The `tts` check: which speech provider hook speech would use, judged by
//! local availability and base quality.
//!
//! Enablement and the configured provider come from the validated user
//! configuration through `dispatch::loader::bridge_tts_settings`, as hook
//! `speak` reads them. Candidates are biscuit-speaks' detected providers in
//! runtime order; a recognized `tts.provider` pins runtime selection to that
//! provider alone (`TtsFailoverStrategy::SpecificProvider`), and an
//! unrecognized one falls back to automatic selection. No speech engine is
//! built and nothing is prewarmed or played.

use biscuit_speaks::{CloudTtsProvider, HostTtsProvider, TtsProvider, VoiceQuality};

use super::config::Unusable;
use super::report::{Finding, Scope};
use super::{CheckOutcome, Pass, Severity};
use crate::config::ClaudineConfig;
use crate::dispatch::loader::bridge_tts_settings;

/// The registry callback: the user configuration of this pass against the
/// providers detected on this host.
pub(crate) fn check_tts(pass: &Pass<'_>) -> CheckOutcome {
    assess(&pass.config().user_config, biscuit_speaks::get_available_providers())
}

/// What the settings ask for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Selection {
    /// Hook speech is off; lifecycle `say` still selects automatically.
    Disabled,
    Automatic,
    /// A recognized `tts.provider`.
    Pinned(TtsProvider),
    /// A `tts.provider` biscuit-speaks does not know; runtime selects
    /// automatically.
    Unrecognized,
}

/// The finding for `user_config` when `available` are the detected
/// providers, in runtime order.
pub(super) fn assess(user_config: &Result<ClaudineConfig, Unusable>, available: &[TtsProvider]) -> CheckOutcome {
    let Ok(config) = user_config else {
        return CheckOutcome::from(vec![
            Finding::new(Severity::Info, Scope::User, "speech settings could not be determined")
                .note("the user configuration is not usable; the config row says why"),
        ]);
    };
    let selection = match bridge_tts_settings(config).tts {
        None => Selection::Disabled,
        Some(settings) => match settings.provider.as_deref() {
            None => Selection::Automatic,
            Some(name) => biscuit_speaks::parse_provider_name(name).map_or(Selection::Unrecognized, Selection::Pinned),
        },
    };

    let mut findings = Vec::new();
    if selection == Selection::Unrecognized {
        findings.push(
            Finding::new(Severity::Warn, Scope::User, "the configured speech provider is not recognized")
                .note("hook speech selects a provider automatically instead")
                .remedy("set `tts.provider` to a supported provider name, or remove it"),
        );
    }

    let candidates: Vec<TtsProvider> = match selection {
        // Runtime keeps a pinned provider only when it is available.
        Selection::Pinned(provider) => available.iter().copied().filter(|candidate| *candidate == provider).collect(),
        _ => available.to_vec(),
    };
    let enabled = selection != Selection::Disabled;
    let best = highest_rated(&candidates);
    let ok_summary = match (selection, best) {
        (Selection::Disabled, Some(best)) => {
            format!("hook speech is off; {} is available to lifecycle `say` ({} quality)", name_of(best), quality_name(best))
        }
        (_, Some(best)) => format!("hook speech uses {} ({} quality)", name_of(best), quality_name(best)),
        (_, None) => String::new(),
    };

    match best {
        None => {
            let summary = match selection {
                Selection::Pinned(provider) => format!("the configured speech provider {} is not available", name_of(provider)),
                Selection::Disabled => "hook speech is off and no speech provider is available".to_string(),
                _ => "hook speech is on but no speech provider is available".to_string(),
            };
            let mut finding = Finding::new(if enabled { Severity::Error } else { Severity::Info }, Scope::User, summary);
            if let Selection::Pinned(_) = selection {
                finding = finding.note("hook speech uses only the configured provider and does not fall back");
            }
            findings.push(finding.remedy(no_candidate_remedy(selection)));
        }
        Some(best) => {
            let quality = biscuit_speaks::provider_base_quality(&best);
            let mut finding = if is_good(quality) {
                Finding::new(Severity::Ok, Scope::User, ok_summary.clone())
            } else {
                let severity = if enabled { Severity::Warn } else { Severity::Info };
                match selection {
                    Selection::Pinned(_) => Finding::new(severity, Scope::User, format!("the configured speech provider, {}, has {} quality", name_of(best), quality_name(best)))
                        .remedy("set `tts.provider` to a higher-quality installed provider, or remove it to select automatically"),
                    _ => Finding::new(severity, Scope::User, format!("the best available speech provider, {}, has {} quality", name_of(best), quality_name(best)))
                        .remedy("install a higher-quality provider such as Kokoro, Echogarden, or Piper"),
                }
            };
            if let Some(first) = candidates.first().filter(|first| **first != best) {
                finding = finding.note(format!(
                    "automatic selection tries {} ({} quality) first",
                    name_of(*first),
                    quality_name(*first)
                ));
            }
            findings.push(finding);
        }
    }

    // Listed last: the same scope as the findings above, and no path.
    let mut notes = Finding::new(Severity::Ok, Scope::User, "provider detection checks local prerequisites only")
        .note("an executable or a credential being present does not prove a working voice, network access, or audio playback");
    if !enabled {
        notes = notes.note("hook speech is off because `tts` is absent, null, or false; lifecycle `say` can still speak");
    }
    findings.push(notes);

    CheckOutcome { ok_summary, findings }
}

fn no_candidate_remedy(selection: Selection) -> &'static str {
    match selection {
        Selection::Pinned(_) => "install the configured provider, or remove `tts.provider` to select automatically",
        _ => "install a local provider such as Piper or eSpeak NG, or set an ElevenLabs API key",
    }
}

/// The first candidate of the best base quality: on a tie, the one runtime
/// tries first.
fn highest_rated(candidates: &[TtsProvider]) -> Option<TtsProvider> {
    candidates.iter().copied().fold(None, |best, candidate| match best {
        Some(best) if biscuit_speaks::provider_base_quality(&best).rank() <= biscuit_speaks::provider_base_quality(&candidate).rank() => {
            Some(best)
        }
        _ => Some(candidate),
    })
}

fn is_good(quality: VoiceQuality) -> bool {
    quality.rank() <= VoiceQuality::Good.rank()
}

fn quality_name(provider: TtsProvider) -> &'static str {
    match biscuit_speaks::provider_base_quality(&provider) {
        VoiceQuality::Excellent => "excellent",
        VoiceQuality::Good => "good",
        VoiceQuality::Moderate => "moderate",
        VoiceQuality::Low => "low",
        _ => "unknown",
    }
}

/// The provider's name as its documentation spells it.
fn name_of(provider: TtsProvider) -> &'static str {
    match provider {
        TtsProvider::Host(host) => match host {
            HostTtsProvider::Say => "macOS say",
            HostTtsProvider::EchoGarden => "Echogarden",
            HostTtsProvider::Sherpa => "Sherpa-ONNX",
            HostTtsProvider::ESpeak => "eSpeak NG",
            HostTtsProvider::Sapi => "Windows SAPI",
            HostTtsProvider::Festival => "Festival",
            HostTtsProvider::Pico2Wave => "Pico2Wave",
            HostTtsProvider::Mimic3 => "Mimic 3",
            HostTtsProvider::KokoroTts => "Kokoro",
            HostTtsProvider::Gtts => "gTTS",
            HostTtsProvider::SpdSay => "Speech Dispatcher",
            HostTtsProvider::Piper => "Piper",
            _ => "a local provider",
        },
        TtsProvider::Cloud(cloud) => match cloud {
            CloudTtsProvider::ElevenLabs => "ElevenLabs",
            _ => "a cloud provider",
        },
        _ => "a provider",
    }
}

#[cfg(test)]
#[path = "tts_tests.rs"]
mod tests;
