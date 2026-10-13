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
use crate::providers::host::EchogardenEngine;
use crate::types::SpeedLevel;

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
