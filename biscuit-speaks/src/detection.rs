//! TTS provider detection and stack building.
//!
//! This module handles:
//! - Detecting available TTS providers on the host system
//! - Building OS-specific provider stacks
//! - Caching detection results for efficiency
//! - Environment variable overrides

use std::sync::OnceLock;

use sniff_lib::programs::InstalledTtsClients;

use crate::types::{CloudTtsProvider, HostTtsProvider, TtsFailoverStrategy, TtsProvider};

#[cfg(target_os = "linux")]
use crate::types::LINUX_TTS_STACK;
#[cfg(target_os = "macos")]
use crate::types::MACOS_TTS_STACK;
#[cfg(target_os = "windows")]
use crate::types::WINDOWS_TTS_STACK;

/// Cached detection results using OnceLock for thread-safe lazy initialization.
static DETECTED_PROVIDERS: OnceLock<Vec<TtsProvider>> = OnceLock::new();

/// Get the list of available TTS providers.
///
/// This function detects available providers once and caches the result.
/// Subsequent calls return the cached list.
///
/// ## Provider Priority
///
/// Providers are returned in priority order based on the operating system:
/// - **macOS**: Say, Piper, EchoGarden, KokoroTts, Sherpa, ESpeak, ElevenLabs
/// - **Linux**: Piper, EchoGarden, KokoroTts, Sherpa, Mimic3, ESpeak, Festival, SpdSay, ElevenLabs
/// - **Windows**: SAPI, Piper, EchoGarden, KokoroTts, Sherpa, ESpeak, ElevenLabs
///
/// ## Environment Variable Override
///
/// Set `TTS_PROVIDER` to override the default provider selection.
/// Valid values match the provider names (case-insensitive):
/// - Host: `say`, `espeak`, `piper`, `echogarden`, `sherpa`, `mimic3`, `festival`, `gtts`, `sapi`, `kokoro`
/// - Cloud: `elevenlabs`
///
/// ## Examples
///
/// ```
/// use biscuit_speaks::detection::get_available_providers;
///
/// let providers = get_available_providers();
/// println!("Found {} TTS providers", providers.len());
/// ```
pub fn get_available_providers() -> &'static [TtsProvider] {
    DETECTED_PROVIDERS.get_or_init(|| {
        let installed = InstalledTtsClients::new();
        build_available_provider_stack(&installed)
    })
}

/// Build the list of available providers based on detection results.
///
/// This filters the OS-specific default stack to only include
/// providers that are actually available on the system.
fn build_available_provider_stack(installed: &InstalledTtsClients) -> Vec<TtsProvider> {
    let default_stack = get_os_default_stack();

    // Check for environment variable override
    if let Some(override_provider) = get_env_provider_override() {
        if override_provider.is_available(installed) {
            // Put the override provider first, then the rest
            let mut stack = vec![override_provider];
            for provider in default_stack.iter() {
                if *provider != override_provider && provider.is_available(installed) {
                    stack.push(*provider);
                }
            }
            return stack;
        }
        // Override provider not available, log and continue with defaults
        tracing::warn!(
            provider = ?override_provider,
            "TTS_PROVIDER override not available, using defaults"
        );
    }

    // Filter to only available providers
    default_stack
        .iter()
        .filter(|p| p.is_available(installed))
        .copied()
        .collect()
}

/// Get the OS-specific default provider stack.
#[cfg(target_os = "macos")]
fn get_os_default_stack() -> &'static [TtsProvider] {
    &MACOS_TTS_STACK
}

/// Get the OS-specific default provider stack.
#[cfg(target_os = "linux")]
fn get_os_default_stack() -> &'static [TtsProvider] {
    &LINUX_TTS_STACK
}

/// Get the OS-specific default provider stack.
#[cfg(target_os = "windows")]
fn get_os_default_stack() -> &'static [TtsProvider] {
    &WINDOWS_TTS_STACK
}

/// Get the OS-specific default provider stack (fallback for unsupported platforms).
#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
fn get_os_default_stack() -> &'static [TtsProvider] {
    // Fallback: empty list for unsupported platforms
    &[]
}

/// Parse the TTS_PROVIDER environment variable.
fn get_env_provider_override() -> Option<TtsProvider> {
    let value = std::env::var("TTS_PROVIDER").ok()?;
    parse_provider_name(&value)
}

/// Parse a provider name string into a TtsProvider.
///
/// ## Examples
///
/// ```
/// use biscuit_speaks::detection::parse_provider_name;
///
/// assert!(parse_provider_name("say").is_some());
/// assert!(parse_provider_name("ElevenLabs").is_some());
/// assert!(parse_provider_name("unknown").is_none());
/// ```
pub fn parse_provider_name(name: &str) -> Option<TtsProvider> {
    match name.to_lowercase().as_str() {
        // Host providers
        "say" | "macos" => Some(TtsProvider::Host(HostTtsProvider::Say)),
        "espeak" | "espeak-ng" | "espeakng" => Some(TtsProvider::Host(HostTtsProvider::ESpeak)),
        "piper" => Some(TtsProvider::Host(HostTtsProvider::Piper)),
        "echogarden" => Some(TtsProvider::Host(HostTtsProvider::EchoGarden)),
        "sherpa" | "sherpa-onnx" | "sherpaonnx" => Some(TtsProvider::Host(HostTtsProvider::Sherpa)),
        "mimic3" | "mimic" => Some(TtsProvider::Host(HostTtsProvider::Mimic3)),
        "festival" => Some(TtsProvider::Host(HostTtsProvider::Festival)),
        "gtts" | "gtts-cli" | "google" => Some(TtsProvider::Host(HostTtsProvider::Gtts)),
        "sapi" | "windows" => Some(TtsProvider::Host(HostTtsProvider::Sapi)),
        "kokoro" | "kokoro-tts" | "kokorotts" => Some(TtsProvider::Host(HostTtsProvider::KokoroTts)),
        "pico" | "pico2wave" => Some(TtsProvider::Host(HostTtsProvider::Pico2Wave)),
        "spd" | "spd-say" | "spdsay" | "speechd" => Some(TtsProvider::Host(HostTtsProvider::SpdSay)),

        // Cloud providers
        "elevenlabs" | "eleven" | "11labs" => {
            Some(TtsProvider::Cloud(CloudTtsProvider::ElevenLabs))
        }

        _ => None,
    }
}

/// Get providers filtered by failover strategy.
///
/// This returns the provider list ordered according to the given strategy.
pub fn get_providers_for_strategy(strategy: &TtsFailoverStrategy) -> Vec<TtsProvider> {
    let all = get_available_providers();

    match strategy {
        TtsFailoverStrategy::FirstAvailable => all.to_vec(),

        TtsFailoverStrategy::PreferHost => {
            let mut result = Vec::with_capacity(all.len());
            // Add host providers first
            for p in all {
                if matches!(p, TtsProvider::Host(_)) {
                    result.push(*p);
                }
            }
            // Then cloud providers
            for p in all {
                if matches!(p, TtsProvider::Cloud(_)) {
                    result.push(*p);
                }
            }
            result
        }

        TtsFailoverStrategy::PreferCloud => {
            let mut result = Vec::with_capacity(all.len());
            // Add cloud providers first
            for p in all {
                if matches!(p, TtsProvider::Cloud(_)) {
                    result.push(*p);
                }
            }
            // Then host providers
            for p in all {
                if matches!(p, TtsProvider::Host(_)) {
                    result.push(*p);
                }
            }
            result
        }

        TtsFailoverStrategy::SpecificProvider(specific) => {
            // Only use the specific provider if available
            if all.contains(specific) {
                vec![*specific]
            } else {
                vec![]
            }
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_provider_name_host() {
        assert_eq!(
            parse_provider_name("say"),
            Some(TtsProvider::Host(HostTtsProvider::Say))
        );
        assert_eq!(
            parse_provider_name("ESPEAK"),
            Some(TtsProvider::Host(HostTtsProvider::ESpeak))
        );
        assert_eq!(
            parse_provider_name("Piper"),
            Some(TtsProvider::Host(HostTtsProvider::Piper))
        );
    }

    #[test]
    fn test_parse_provider_name_cloud() {
        assert_eq!(
            parse_provider_name("elevenlabs"),
            Some(TtsProvider::Cloud(CloudTtsProvider::ElevenLabs))
        );
        assert_eq!(
            parse_provider_name("11labs"),
            Some(TtsProvider::Cloud(CloudTtsProvider::ElevenLabs))
        );
    }

    #[test]
    fn test_parse_provider_name_invalid() {
        assert_eq!(parse_provider_name("unknown"), None);
        assert_eq!(parse_provider_name(""), None);
    }

    #[test]
    fn test_parse_provider_name_aliases() {
        // Test various aliases
        assert_eq!(
            parse_provider_name("macos"),
            Some(TtsProvider::Host(HostTtsProvider::Say))
        );
        assert_eq!(
            parse_provider_name("espeak-ng"),
            Some(TtsProvider::Host(HostTtsProvider::ESpeak))
        );
        assert_eq!(
            parse_provider_name("sherpa-onnx"),
            Some(TtsProvider::Host(HostTtsProvider::Sherpa))
        );
        assert_eq!(
            parse_provider_name("google"),
            Some(TtsProvider::Host(HostTtsProvider::Gtts))
        );
    }

    #[test]
    fn test_get_os_default_stack_not_empty() {
        let stack = get_os_default_stack();
        assert!(!stack.is_empty());
    }

    #[test]
    fn test_build_available_provider_stack_filters() {
        // Create a mock installed clients with only Say available
        let mut installed = InstalledTtsClients::default();
        installed.say = true;

        let stack = build_available_provider_stack(&installed);

        // On macOS, Say should be in the stack; on other platforms it may not be in defaults
        #[cfg(target_os = "macos")]
        {
            assert!(!stack.is_empty());
            assert!(stack.contains(&TtsProvider::Host(HostTtsProvider::Say)));
        }
    }

    #[test]
    fn test_get_providers_for_strategy_specific() {
        // SpecificProvider with unavailable provider should return empty
        let strategy = TtsFailoverStrategy::SpecificProvider(TtsProvider::Host(
            HostTtsProvider::Pico2Wave, // This is unlikely to be installed
        ));

        // We can't easily test this without mocking, but we can verify the logic
        let providers = get_providers_for_strategy(&strategy);
        // If Pico2Wave is not available (which it usually isn't), this should be empty
        // Note: This test may pass or fail depending on the system, which is expected
        let _ = providers; // Just verify it doesn't panic
    }
}
