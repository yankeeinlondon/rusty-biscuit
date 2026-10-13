//! Resolved synthesis settings and the shared rules preparation follows.
//!
//! A provider resolves a request once per operation into a
//! [`ResolvedSynthesis`], and both its cache key and its synthesizer read that
//! one value. `Speak::prepare` retains it so later playback reuses the file
//! without resolving defaults again.

use std::path::Path;

use serde::{Deserialize, Serialize};
use tempfile::TempPath;

use crate::errors::TtsError;
use crate::providers::cloud::ElevenLabsProvider;
use crate::providers::host::{
    EchogardenEngine, EchogardenProvider, GttsProvider, KokoroTtsProvider, SayProvider,
    SayResolution as Resolution,
};
use crate::types::{
    CloudTtsProvider, HostTtsProvider, SpeakResult, SpeedLevel, TtsConfig, TtsProvider,
};

/// Whether Playa's dry-run switch is set, in any spelling Playa accepts.
///
/// Read without the `playa` feature too: dry-run preparation must do no work
/// whether or not this build can play audio.
pub(crate) fn dry_run_enabled() -> bool {
    matches!(
        std::env::var("PLAYA_DRY_RUN").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes")
    )
}

/// The provider-specific settings that identify one synthesized file.
///
/// Never merged into `TtsConfig`: a resolved voice belongs to the provider
/// that resolved it, and Say's absolute rate has no `SpeedLevel` spelling.
/// Serializable because a prepared request crosses the detached helper
/// handoff.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum ResolvedSynthesis {
    Say { voice: String, rate_wpm: u32 },
    Kokoro { voice: String },
    Gtts { language: String },
    Echogarden {
        engine: EchogardenEngine,
        voice: String,
        speed: SpeedLevel,
    },
    ElevenLabs {
        voice: String,
        model: String,
        speed: SpeedLevel,
    },
}

/// Playback with settings a preparation retained.
///
/// Each method goes straight to the provider that resolved the settings and
/// skips its default resolution, so a changed system or account default
/// cannot change what a prepared object plays. `config` supplies only the
/// playback inputs (volume, and speed where it is not baked in) and the
/// metadata hints. A missing cache file is synthesized again with the
/// retained settings.
impl ResolvedSynthesis {
    /// The provider that resolved these settings and plays them.
    pub(crate) fn provider(&self) -> TtsProvider {
        match self {
            Self::Say { .. } => TtsProvider::Host(HostTtsProvider::Say),
            Self::Kokoro { .. } => TtsProvider::Host(HostTtsProvider::KokoroTts),
            Self::Gtts { .. } => TtsProvider::Host(HostTtsProvider::Gtts),
            Self::Echogarden { .. } => TtsProvider::Host(HostTtsProvider::EchoGarden),
            Self::ElevenLabs { .. } => TtsProvider::Cloud(CloudTtsProvider::ElevenLabs),
        }
    }

    pub(crate) async fn speak(&self, text: &str, config: &TtsConfig) -> Result<(), TtsError> {
        match self {
            Self::Say { voice, rate_wpm } => {
                SayProvider::speak_resolved(text, &Resolution::retained(voice, *rate_wpm), config)
                    .await
            }
            Self::Kokoro { voice } => KokoroTtsProvider::new().speak_resolved(text, voice, config).await,
            Self::Gtts { language } => GttsProvider::new().speak_resolved(text, language, config).await,
            Self::Echogarden { engine, voice, speed } => {
                EchogardenProvider::with_engine(*engine)
                    .speak_resolved(text, Some(voice), *speed, config)
                    .await
            }
            Self::ElevenLabs { voice, model, speed } => {
                ElevenLabsProvider::new()?
                    .speak_resolved(text, voice, model, *speed, config)
                    .await
            }
        }
    }

    pub(crate) async fn speak_with_result(
        &self,
        text: &str,
        config: &TtsConfig,
    ) -> Result<SpeakResult, TtsError> {
        match self {
            Self::Say { voice, rate_wpm } => {
                SayProvider
                    .speak_resolved_with_result(text, Resolution::retained(voice, *rate_wpm), config)
                    .await
            }
            Self::Kokoro { voice } => {
                KokoroTtsProvider::new()
                    .speak_resolved_with_result(text, voice, config)
                    .await
            }
            Self::Gtts { language } => {
                GttsProvider::new()
                    .speak_resolved_with_result(text, language, config)
                    .await
            }
            Self::Echogarden { engine, voice, speed } => {
                EchogardenProvider::with_engine(*engine)
                    .speak_resolved_with_result(text, Some(voice), *speed, config)
                    .await
            }
            Self::ElevenLabs { voice, model, speed } => {
                ElevenLabsProvider::new()?
                    .speak_resolved_with_result(text, voice, model, *speed, config)
                    .await
            }
        }
    }

    /// A ready job, synthesizing the file first if it is missing. Runs in the
    /// detached preparation helper, never in the requesting process.
    #[cfg(feature = "playa")]
    pub(crate) async fn job(
        &self,
        text: &str,
        config: &TtsConfig,
    ) -> Result<playa::detached::SpoolJob, TtsError> {
        match self {
            Self::Say { voice, rate_wpm } => {
                SayProvider::resolved_job(text, &Resolution::retained(voice, *rate_wpm), config).await
            }
            Self::Kokoro { voice } => KokoroTtsProvider::new().resolved_job(text, voice, config).await,
            Self::Gtts { language } => GttsProvider::new().resolved_job(text, language, config).await,
            Self::Echogarden { engine, voice, speed } => {
                EchogardenProvider::with_engine(*engine)
                    .resolved_job(text, Some(voice), *speed, config)
                    .await
            }
            Self::ElevenLabs { voice, model, speed } => {
                ElevenLabsProvider::new()?
                    .resolved_job(text, voice, model, *speed, config)
                    .await
            }
        }
    }

    /// A ready job when the file is still cached; no subprocess or request.
    #[cfg(feature = "playa")]
    pub(crate) fn cached_job(
        &self,
        text: &str,
        config: &TtsConfig,
    ) -> Option<playa::detached::SpoolJob> {
        match self {
            Self::Say { voice, rate_wpm } => SayProvider::cached_resolved_job(
                text,
                &Resolution::retained(voice, *rate_wpm),
                config,
            ),
            Self::Kokoro { voice } => KokoroTtsProvider::cached_resolved_job(text, voice, config),
            Self::Gtts { language } => GttsProvider::cached_resolved_job(text, language, config),
            Self::Echogarden { engine, voice, speed } => EchogardenProvider::with_engine(*engine)
                .cached_resolved_job(text, voice, *speed, config),
            Self::ElevenLabs { voice, model, speed } => {
                ElevenLabsProvider::cached_resolved_job(text, voice, model, *speed, config)
            }
        }
    }
}

/// Create the file a synthesizer writes before it is published at `cache_path`.
///
/// It sits beside the cache entry so publication is a same-filesystem rename,
/// and its name never matches a cache entry. Dropping it deletes the partial
/// output, which is how a failed or cancelled synthesis leaves no cache entry.
pub(crate) fn staging_file(cache_path: &Path) -> Result<TempPath, TtsError> {
    let dir = cache_path.parent().unwrap_or_else(|| Path::new("."));
    let suffix = cache_path
        .extension()
        .map(|extension| format!(".{}", extension.to_string_lossy()))
        .unwrap_or_default();
    tempfile::Builder::new()
        .prefix(".biscuit-speaks-staging-")
        .suffix(&suffix)
        .tempfile_in(dir)
        .map(tempfile::NamedTempFile::into_temp_path)
        .map_err(|source| TtsError::TempFileError { source })
}

/// Atomically replace `cache_path` with a completed staging file.
///
/// Concurrent publishers each rename a complete file, so the last rename wins
/// and readers never observe a partial entry.
pub(crate) fn publish(staged: TempPath, cache_path: &Path) -> Result<(), TtsError> {
    staged
        .persist(cache_path)
        .map_err(|error| TtsError::TempFileError {
            source: error.error,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn every_variant() -> Vec<ResolvedSynthesis> {
        vec![
            ResolvedSynthesis::Say {
                voice: "Ava (Premium)".into(),
                rate_wpm: 175,
            },
            ResolvedSynthesis::Kokoro {
                voice: "af_heart".into(),
            },
            ResolvedSynthesis::Gtts {
                language: "en".into(),
            },
            ResolvedSynthesis::Echogarden {
                engine: EchogardenEngine::Vits,
                voice: "en_US-amy-medium".into(),
                speed: SpeedLevel::Explicit(1.5),
            },
            ResolvedSynthesis::ElevenLabs {
                voice: "21m00Tcm4TlvDq8ikWAM".into(),
                model: "eleven_flash_v2_5".into(),
                speed: SpeedLevel::Normal,
            },
        ]
    }

    #[test]
    fn every_variant_survives_a_json_round_trip() {
        for settings in every_variant() {
            let json = serde_json::to_string(&settings).unwrap();
            let decoded: ResolvedSynthesis = serde_json::from_str(&json).unwrap();
            assert_eq!(decoded, settings, "{json}");
            let again = serde_json::to_string(&decoded).unwrap();
            assert_eq!(again, json);
        }
    }

    #[test]
    fn each_variant_names_the_provider_that_resolved_it() {
        let providers: Vec<_> = every_variant().iter().map(ResolvedSynthesis::provider).collect();
        assert_eq!(
            providers,
            [
                TtsProvider::Host(HostTtsProvider::Say),
                TtsProvider::Host(HostTtsProvider::KokoroTts),
                TtsProvider::Host(HostTtsProvider::Gtts),
                TtsProvider::Host(HostTtsProvider::EchoGarden),
                TtsProvider::Cloud(CloudTtsProvider::ElevenLabs),
            ]
        );
    }

    #[test]
    fn the_wire_shape_names_the_provider_and_its_settings() {
        let json = serde_json::to_value(ResolvedSynthesis::Echogarden {
            engine: EchogardenEngine::Kokoro,
            voice: "Heart".into(),
            speed: SpeedLevel::Fast,
        })
        .unwrap();
        assert_eq!(
            json,
            serde_json::json!({"echogarden": {"engine": "kokoro", "voice": "Heart", "speed": "fast"}})
        );
    }

    #[test]
    fn missing_null_mistyped_and_unknown_fields_are_rejected() {
        for invalid in [
            r#"{"kokoro":{}}"#,
            r#"{"kokoro":{"voice":null}}"#,
            r#"{"kokoro":{"voice":7}}"#,
            r#"{"kokoro":{"voice":"af_heart","speed":"fast"}}"#,
            r#"{"say":{"voice":"Alex","rate_wpm":-1}}"#,
            r#"{"piper":{"voice":"x"}}"#,
            r#"{"kokoro":{"voice":"af_heart"}} trailing"#,
        ] {
            assert!(
                serde_json::from_str::<ResolvedSynthesis>(invalid).is_err(),
                "{invalid} must not decode"
            );
        }
    }

    #[test]
    #[serial_test::serial]
    fn dry_run_accepts_exactly_playas_spellings() {
        for (value, expected) in [
            (Some("1"), true),
            (Some("true"), true),
            (Some("TRUE"), true),
            (Some("yes"), true),
            (Some("True"), false),
            (Some("0"), false),
            (Some(""), false),
            (None, false),
        ] {
            let _guard = match value {
                Some(value) => test_toolkit::EnvGuard::set_safe("PLAYA_DRY_RUN", value),
                None => test_toolkit::EnvGuard::remove_safe("PLAYA_DRY_RUN"),
            };
            assert_eq!(dry_run_enabled(), expected, "{value:?}");
        }
    }

    #[test]
    fn a_dropped_staging_file_leaves_nothing_and_a_published_one_replaces_the_entry() {
        let dir = tempfile::tempdir().unwrap();
        let entry = dir.path().join("biscuit-speaks-0123456789abcdef.wav");

        let staged = staging_file(&entry).unwrap();
        std::fs::write(&staged, b"partial").unwrap();
        let name = staged.file_name().unwrap().to_string_lossy().into_owned();
        assert!(name.starts_with(".biscuit-speaks-staging-") && name.ends_with(".wav"), "{name}");
        drop(staged);
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);

        for content in [b"first".as_slice(), b"second"] {
            let staged = staging_file(&entry).unwrap();
            std::fs::write(&staged, content).unwrap();
            publish(staged, &entry).unwrap();
            assert_eq!(std::fs::read(&entry).unwrap(), content);
        }
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }
}
