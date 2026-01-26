//! Kokoro TTS provider.
//!
//! Uses the `kokoro-tts` CLI tool for high-quality neural text-to-speech.
//! Requires model files to be downloaded and available.

use std::process::Stdio;

use tracing::debug;

use crate::errors::TtsError;
use crate::traits::{TtsExecutor, TtsVoiceInventory};
use crate::types::{Gender, HostTtsProvider, Language, SpeakResult, TtsConfig, TtsProvider, Voice, VoiceQuality};

/// Kokoro TTS provider.
///
/// This provider uses the `kokoro-tts` CLI tool for high-quality neural
/// text-to-speech using the Kokoro-82M model.
///
/// ## Model Requirements
///
/// Kokoro TTS requires two model files:
/// - `kokoro-v1.0.onnx` - The ONNX model
/// - `voices-v1.0.bin` - Voice embeddings
///
/// These can be downloaded from the Kokoro TTS releases page.
///
/// ## Voice Selection
///
/// Voices use a 2-character prefix convention:
/// - First char: language code (a=American, b=British, j=Japanese, etc.)
/// - Second char: gender (f=Female, m=Male)
///
/// Example: `af_heart` = American Female voice named "heart"
///
/// ## Examples
///
/// ```ignore
/// use biscuit_speaks::providers::host::KokoroTtsProvider;
/// use biscuit_speaks::{TtsExecutor, TtsConfig};
///
/// let provider = KokoroTtsProvider::new();
/// provider.speak("Hello, world!", &TtsConfig::default()).await?;
/// ```
#[derive(Debug, Clone, Default)]
pub struct KokoroTtsProvider {
    /// Custom path to the model file.
    model_path: Option<String>,
    /// Custom path to the voices file.
    voices_path: Option<String>,
}

impl KokoroTtsProvider {
    /// Provider name constant for error messages.
    const PROVIDER_NAME: &'static str = "kokoro-tts";

    /// Default voice for when no specific voice is requested.
    const DEFAULT_VOICE: &'static str = "af_heart";

    /// Create a new KokoroTtsProvider with default settings.
    ///
    /// Checks for `KOKORO_MODEL` and `KOKORO_VOICES` environment variables
    /// for custom model paths.
    pub fn new() -> Self {
        let model_path = std::env::var("KOKORO_MODEL").ok();
        let voices_path = std::env::var("KOKORO_VOICES").ok();
        Self {
            model_path,
            voices_path,
        }
    }

    /// Create a provider with custom model and voices file paths.
    pub fn with_paths(model_path: impl Into<String>, voices_path: impl Into<String>) -> Self {
        Self {
            model_path: Some(model_path.into()),
            voices_path: Some(voices_path.into()),
        }
    }

    /// Resolve the voice to use based on config (simple fallback).
    fn resolve_voice(config: &TtsConfig) -> &str {
        if let Some(voice) = &config.requested_voice {
            return voice.as_str();
        }

        // Select default voice based on gender
        match config.gender {
            Gender::Male => "am_adam",
            Gender::Female => Self::DEFAULT_VOICE,
            Gender::Any => Self::DEFAULT_VOICE,
        }
    }

    /// Select the best voice from the Kokoro voice list based on config constraints.
    fn select_best_voice(voices: &[Voice], config: &TtsConfig) -> Option<Voice> {
        let mut candidates: Vec<&Voice> = voices.iter().collect();

        // Filter by language if specified
        let target_language = &config.language;
        let filtered_by_lang: Vec<&Voice> = candidates
            .iter()
            .filter(|v| {
                v.languages.iter().any(|lang| {
                    match (lang, target_language) {
                        (Language::English, Language::English) => true,
                        (Language::Custom(a), Language::Custom(b)) => {
                            let a_lower = a.to_lowercase();
                            let b_lower = b.to_lowercase();
                            a_lower == b_lower
                                || a_lower.starts_with(&format!("{}-", b_lower))
                                || b_lower.starts_with(&format!("{}-", a_lower))
                        }
                        (Language::English, Language::Custom(c))
                        | (Language::Custom(c), Language::English) => {
                            c.to_lowercase().starts_with("en")
                        }
                    }
                })
            })
            .copied()
            .collect();

        if !filtered_by_lang.is_empty() {
            candidates = filtered_by_lang;
        }

        // Filter by gender if specified (not Any)
        if config.gender != Gender::Any {
            let gender_matches: Vec<&Voice> = candidates
                .iter()
                .filter(|v| v.gender == config.gender)
                .copied()
                .collect();

            if !gender_matches.is_empty() {
                candidates = gender_matches;
            }
        }

        // All Kokoro voices are Excellent quality, so just pick the first
        // (which will be the first in the list, typically "af_heart" for female English)
        candidates.first().cloned().cloned()
    }

    /// Check if the kokoro-tts binary exists on the system.
    fn binary_exists() -> bool {
        which::which("kokoro-tts").is_ok()
    }

    /// Check if the kokoro-tts binary exists with model files configured.
    ///
    /// This runs `kokoro-tts --help-voices` which requires model files to be present.
    /// Unlike `--help`, this command will fail if model files are missing.
    async fn models_available() -> bool {
        // First check for custom model paths via environment variables
        let model_configured = std::env::var("KOKORO_MODEL").is_ok();
        let voices_configured = std::env::var("KOKORO_VOICES").is_ok();

        // If both env vars are set, assume models are available
        // (actual validation happens at speak time)
        if model_configured && voices_configured {
            return true;
        }

        // Otherwise, try running --help-voices which requires models
        let result = tokio::process::Command::new("kokoro-tts")
            .arg("--help-voices")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await;

        matches!(result, Ok(status) if status.success())
    }

    /// Parse a voice name prefix to extract gender and language.
    ///
    /// The format is `[language][gender]_[name]`:
    /// - language: a=American, b=British, j=Japanese, z=Mandarin, etc.
    /// - gender: f=Female, m=Male
    fn parse_voice_prefix(voice: &str) -> (Gender, Language) {
        let prefix = voice.get(0..2).unwrap_or("");

        let gender = match prefix.chars().nth(1) {
            Some('f') => Gender::Female,
            Some('m') => Gender::Male,
            _ => Gender::Any,
        };

        let language = match prefix.chars().next() {
            Some('a') => Language::English, // American English
            Some('b') => Language::English, // British English
            Some('j') => Language::Custom("ja".into()),
            Some('z') => Language::Custom("zh".into()),
            Some('e') => Language::Custom("es".into()),
            Some('f') => Language::Custom("fr".into()),
            Some('h') => Language::Custom("hi".into()),
            Some('i') => Language::Custom("it".into()),
            Some('p') => Language::Custom("pt-BR".into()),
            _ => Language::English,
        };

        (gender, language)
    }
}

impl TtsExecutor for KokoroTtsProvider {
    async fn speak(&self, text: &str, config: &TtsConfig) -> Result<(), TtsError> {
        // Create a temporary directory for input and output files
        // Note: We use a tempdir instead of NamedTempFile because some CLIs
        // don't properly handle pre-existing files for output.
        let temp_dir = tempfile::tempdir().map_err(|e| TtsError::TempFileError { source: e })?;

        // Create input text file
        let input_path = temp_dir.path().join("input.txt");
        tokio::fs::write(&input_path, text).await?;

        // Define output audio path (file doesn't exist yet)
        let output_path = temp_dir.path().join("output.wav");

        let voice = Self::resolve_voice(config);

        let mut cmd = tokio::process::Command::new("kokoro-tts");
        cmd.arg(&input_path);
        cmd.arg(&output_path);
        cmd.arg("--voice").arg(voice);
        cmd.arg("--format").arg("wav");

        // Add custom model paths if specified
        if let Some(model_path) = &self.model_path {
            cmd.arg("--model").arg(model_path);
        }
        if let Some(voices_path) = &self.voices_path {
            cmd.arg("--voices").arg(voices_path);
        }

        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        debug!(
            provider = Self::PROVIDER_NAME,
            voice = voice,
            "Running kokoro-tts"
        );

        let output = cmd.output().await.map_err(|e| TtsError::ProcessSpawnFailed {
            provider: Self::PROVIDER_NAME.into(),
            source: e,
        })?;

        if !output.status.success() {
            // kokoro-tts writes errors to stdout, not stderr
            let error_output = if output.stderr.is_empty() {
                String::from_utf8_lossy(&output.stdout).to_string()
            } else {
                String::from_utf8_lossy(&output.stderr).to_string()
            };
            return Err(TtsError::ProcessFailed {
                provider: Self::PROVIDER_NAME.into(),
                stderr: error_output,
            });
        }

        // Play the generated audio file
        #[cfg(feature = "playa")]
        {
            crate::playback::play_audio_file(&output_path, crate::types::AudioFormat::Wav, config).await
        }
        #[cfg(not(feature = "playa"))]
        {
            // Playback requires the playa feature
            let _ = output_path;
            Err(TtsError::NoAudioPlayer)
        }
    }

    async fn is_ready(&self) -> bool {
        if !Self::binary_exists() {
            return false;
        }

        // Check if model files are available
        Self::models_available().await
    }

    fn info(&self) -> &str {
        "Kokoro TTS - High-quality neural TTS using the Kokoro-82M model with 54 voices across 9 languages"
    }

    async fn speak_with_result(
        &self,
        text: &str,
        config: &TtsConfig,
    ) -> Result<SpeakResult, TtsError> {
        // If a specific voice was requested, use it directly
        if let Some(voice_name) = &config.requested_voice {
            self.speak(text, config).await?;

            let (gender, language) = Self::parse_voice_prefix(voice_name);
            let voice = Voice::new(voice_name)
                .with_gender(gender)
                .with_quality(VoiceQuality::Excellent)
                .with_language(language);

            return Ok(SpeakResult::new(
                TtsProvider::Host(HostTtsProvider::KokoroTts),
                voice,
            ));
        }

        // No specific voice requested - select the best one based on constraints
        let voices = self.list_voices().await?;
        let selected_voice = Self::select_best_voice(&voices, config).ok_or_else(|| {
            TtsError::VoiceEnumerationFailed {
                provider: Self::PROVIDER_NAME.into(),
                message: "No matching voice found for the given constraints".into(),
            }
        })?;

        // Update config to use the selected voice
        let mut config_with_voice = config.clone();
        config_with_voice.requested_voice = Some(selected_voice.name.clone());

        // Speak with the selected voice
        self.speak(text, &config_with_voice).await?;

        Ok(SpeakResult::new(
            TtsProvider::Host(HostTtsProvider::KokoroTts),
            selected_voice,
        ))
    }
}

impl TtsVoiceInventory for KokoroTtsProvider {
    /// List all available Kokoro TTS voices.
    ///
    /// Kokoro has a fixed set of 54 voices that are built into the model.
    /// This returns the complete list with inferred gender and language.
    async fn list_voices(&self) -> Result<Vec<Voice>, TtsError> {
        // Kokoro has a fixed set of voices built into the model.
        // Rather than parsing --help-voices (which requires model files),
        // we use the known voice list from the model documentation.
        Ok(get_kokoro_voices())
    }
}

/// Get the complete list of Kokoro TTS voices.
///
/// This returns the hardcoded list of 54 voices from the Kokoro-82M model.
/// The voices are categorized by language and gender based on their prefixes.
fn get_kokoro_voices() -> Vec<Voice> {
    // American English Female voices (11)
    let af_voices = [
        "af_heart",
        "af_alloy",
        "af_aoede",
        "af_bella",
        "af_jessica",
        "af_kore",
        "af_nicole",
        "af_nova",
        "af_river",
        "af_sarah",
        "af_sky",
    ];

    // American English Male voices (9)
    let am_voices = [
        "am_adam",
        "am_echo",
        "am_eric",
        "am_fenrir",
        "am_liam",
        "am_michael",
        "am_onyx",
        "am_puck",
        "am_santa",
    ];

    // British English Female voices (4)
    let bf_voices = ["bf_alice", "bf_emma", "bf_isabella", "bf_lily"];

    // British English Male voices (4)
    let bm_voices = ["bm_daniel", "bm_fable", "bm_george", "bm_lewis"];

    // Japanese Female voices (4)
    let jf_voices = ["jf_alpha", "jf_gongitsune", "jf_nezumi", "jf_tebukuro"];

    // Japanese Male voices (1)
    let jm_voices = ["jm_kumo"];

    // Mandarin Chinese Female voices (4)
    let zf_voices = ["zf_xiaobei", "zf_xiaoni", "zf_xiaoxiao", "zf_xiaoyi"];

    // Mandarin Chinese Male voices (4)
    let zm_voices = ["zm_yunjian", "zm_yunxi", "zm_yunxia", "zm_yunyang"];

    // Spanish Female voices (1)
    let ef_voices = ["ef_dora"];

    // Spanish Male voices (2)
    let em_voices = ["em_alex", "em_santa"];

    // French Female voices (1)
    let ff_voices = ["ff_siwis"];

    // Hindi Female voices (2)
    let hf_voices = ["hf_alpha", "hf_beta"];

    // Hindi Male voices (2)
    let hm_voices = ["hm_omega", "hm_psi"];

    // Italian Female voices (1)
    let if_voices = ["if_sara"];

    // Italian Male voices (1)
    let im_voices = ["im_nicola"];

    // Brazilian Portuguese Female voices (1)
    let pf_voices = ["pf_dora"];

    // Brazilian Portuguese Male voices (2)
    let pm_voices = ["pm_alex", "pm_santa"];

    // Combine all voices into a single list
    let all_voice_names: Vec<&str> = [
        &af_voices[..],
        &am_voices[..],
        &bf_voices[..],
        &bm_voices[..],
        &jf_voices[..],
        &jm_voices[..],
        &zf_voices[..],
        &zm_voices[..],
        &ef_voices[..],
        &em_voices[..],
        &ff_voices[..],
        &hf_voices[..],
        &hm_voices[..],
        &if_voices[..],
        &im_voices[..],
        &pf_voices[..],
        &pm_voices[..],
    ]
    .concat();

    all_voice_names
        .into_iter()
        .map(|name| {
            let (gender, language) = KokoroTtsProvider::parse_voice_prefix(name);
            Voice::new(name)
                .with_gender(gender)
                .with_quality(VoiceQuality::Excellent) // Neural TTS = excellent quality
                .with_language(language)
        })
        .collect()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Basic provider tests
    // ========================================================================

    #[test]
    fn test_kokoro_provider_default() {
        let provider = KokoroTtsProvider::default();
        assert!(provider.model_path.is_none());
        assert!(provider.voices_path.is_none());
    }

    #[test]
    fn test_kokoro_provider_with_paths() {
        let provider =
            KokoroTtsProvider::with_paths("/path/to/model.onnx", "/path/to/voices.bin");
        assert_eq!(provider.model_path, Some("/path/to/model.onnx".into()));
        assert_eq!(provider.voices_path, Some("/path/to/voices.bin".into()));
    }

    #[test]
    fn test_resolve_voice_requested() {
        let config = TtsConfig::new().with_voice("bf_emma");
        assert_eq!(KokoroTtsProvider::resolve_voice(&config), "bf_emma");
    }

    #[test]
    fn test_resolve_voice_gender_male() {
        let config = TtsConfig::new().with_gender(Gender::Male);
        assert_eq!(KokoroTtsProvider::resolve_voice(&config), "am_adam");
    }

    #[test]
    fn test_resolve_voice_gender_female() {
        let config = TtsConfig::new().with_gender(Gender::Female);
        assert_eq!(KokoroTtsProvider::resolve_voice(&config), "af_heart");
    }

    #[test]
    fn test_resolve_voice_gender_any() {
        let config = TtsConfig::new();
        assert_eq!(KokoroTtsProvider::resolve_voice(&config), "af_heart");
    }

    #[test]
    fn test_info() {
        let provider = KokoroTtsProvider::new();
        let info = provider.info();
        assert!(info.contains("Kokoro"));
        assert!(info.contains("neural"));
    }

    // ========================================================================
    // Voice prefix parsing tests
    // ========================================================================

    #[test]
    fn test_parse_prefix_american_female() {
        let (gender, language) = KokoroTtsProvider::parse_voice_prefix("af_heart");
        assert_eq!(gender, Gender::Female);
        assert_eq!(language, Language::English);
    }

    #[test]
    fn test_parse_prefix_american_male() {
        let (gender, language) = KokoroTtsProvider::parse_voice_prefix("am_adam");
        assert_eq!(gender, Gender::Male);
        assert_eq!(language, Language::English);
    }

    #[test]
    fn test_parse_prefix_british_female() {
        let (gender, language) = KokoroTtsProvider::parse_voice_prefix("bf_emma");
        assert_eq!(gender, Gender::Female);
        assert_eq!(language, Language::English);
    }

    #[test]
    fn test_parse_prefix_british_male() {
        let (gender, language) = KokoroTtsProvider::parse_voice_prefix("bm_daniel");
        assert_eq!(gender, Gender::Male);
        assert_eq!(language, Language::English);
    }

    #[test]
    fn test_parse_prefix_japanese_female() {
        let (gender, language) = KokoroTtsProvider::parse_voice_prefix("jf_alpha");
        assert_eq!(gender, Gender::Female);
        assert_eq!(language, Language::Custom("ja".into()));
    }

    #[test]
    fn test_parse_prefix_japanese_male() {
        let (gender, language) = KokoroTtsProvider::parse_voice_prefix("jm_kumo");
        assert_eq!(gender, Gender::Male);
        assert_eq!(language, Language::Custom("ja".into()));
    }

    #[test]
    fn test_parse_prefix_mandarin_female() {
        let (gender, language) = KokoroTtsProvider::parse_voice_prefix("zf_xiaobei");
        assert_eq!(gender, Gender::Female);
        assert_eq!(language, Language::Custom("zh".into()));
    }

    #[test]
    fn test_parse_prefix_mandarin_male() {
        let (gender, language) = KokoroTtsProvider::parse_voice_prefix("zm_yunxi");
        assert_eq!(gender, Gender::Male);
        assert_eq!(language, Language::Custom("zh".into()));
    }

    #[test]
    fn test_parse_prefix_spanish_female() {
        let (gender, language) = KokoroTtsProvider::parse_voice_prefix("ef_dora");
        assert_eq!(gender, Gender::Female);
        assert_eq!(language, Language::Custom("es".into()));
    }

    #[test]
    fn test_parse_prefix_spanish_male() {
        let (gender, language) = KokoroTtsProvider::parse_voice_prefix("em_alex");
        assert_eq!(gender, Gender::Male);
        assert_eq!(language, Language::Custom("es".into()));
    }

    #[test]
    fn test_parse_prefix_french_female() {
        let (gender, language) = KokoroTtsProvider::parse_voice_prefix("ff_siwis");
        assert_eq!(gender, Gender::Female);
        assert_eq!(language, Language::Custom("fr".into()));
    }

    #[test]
    fn test_parse_prefix_hindi_female() {
        let (gender, language) = KokoroTtsProvider::parse_voice_prefix("hf_alpha");
        assert_eq!(gender, Gender::Female);
        assert_eq!(language, Language::Custom("hi".into()));
    }

    #[test]
    fn test_parse_prefix_hindi_male() {
        let (gender, language) = KokoroTtsProvider::parse_voice_prefix("hm_omega");
        assert_eq!(gender, Gender::Male);
        assert_eq!(language, Language::Custom("hi".into()));
    }

    #[test]
    fn test_parse_prefix_italian_female() {
        let (gender, language) = KokoroTtsProvider::parse_voice_prefix("if_sara");
        assert_eq!(gender, Gender::Female);
        assert_eq!(language, Language::Custom("it".into()));
    }

    #[test]
    fn test_parse_prefix_italian_male() {
        let (gender, language) = KokoroTtsProvider::parse_voice_prefix("im_nicola");
        assert_eq!(gender, Gender::Male);
        assert_eq!(language, Language::Custom("it".into()));
    }

    #[test]
    fn test_parse_prefix_portuguese_female() {
        let (gender, language) = KokoroTtsProvider::parse_voice_prefix("pf_dora");
        assert_eq!(gender, Gender::Female);
        assert_eq!(language, Language::Custom("pt-BR".into()));
    }

    #[test]
    fn test_parse_prefix_portuguese_male() {
        let (gender, language) = KokoroTtsProvider::parse_voice_prefix("pm_alex");
        assert_eq!(gender, Gender::Male);
        assert_eq!(language, Language::Custom("pt-BR".into()));
    }

    #[test]
    fn test_parse_prefix_unknown() {
        let (gender, language) = KokoroTtsProvider::parse_voice_prefix("xx_unknown");
        assert_eq!(gender, Gender::Any);
        assert_eq!(language, Language::English);
    }

    #[test]
    fn test_parse_prefix_short_string() {
        let (gender, language) = KokoroTtsProvider::parse_voice_prefix("a");
        assert_eq!(gender, Gender::Any);
        assert_eq!(language, Language::English);
    }

    #[test]
    fn test_parse_prefix_empty() {
        let (gender, language) = KokoroTtsProvider::parse_voice_prefix("");
        assert_eq!(gender, Gender::Any);
        assert_eq!(language, Language::English);
    }

    // ========================================================================
    // Voice list tests
    // ========================================================================

    #[test]
    fn test_get_kokoro_voices_count() {
        let voices = get_kokoro_voices();
        assert_eq!(voices.len(), 54, "Kokoro should have 54 voices");
    }

    #[test]
    fn test_get_kokoro_voices_all_excellent_quality() {
        let voices = get_kokoro_voices();
        for voice in &voices {
            assert_eq!(
                voice.quality,
                VoiceQuality::Excellent,
                "Voice {} should have Excellent quality",
                voice.name
            );
        }
    }

    #[test]
    fn test_get_kokoro_voices_american_english_female_count() {
        let voices = get_kokoro_voices();
        let af_count = voices
            .iter()
            .filter(|v| v.name.starts_with("af_"))
            .count();
        assert_eq!(af_count, 11, "Should have 11 American English female voices");
    }

    #[test]
    fn test_get_kokoro_voices_american_english_male_count() {
        let voices = get_kokoro_voices();
        let am_count = voices
            .iter()
            .filter(|v| v.name.starts_with("am_"))
            .count();
        assert_eq!(am_count, 9, "Should have 9 American English male voices");
    }

    #[test]
    fn test_get_kokoro_voices_british_count() {
        let voices = get_kokoro_voices();
        let british_count = voices
            .iter()
            .filter(|v| v.name.starts_with("bf_") || v.name.starts_with("bm_"))
            .count();
        assert_eq!(british_count, 8, "Should have 8 British English voices");
    }

    #[test]
    fn test_get_kokoro_voices_japanese_count() {
        let voices = get_kokoro_voices();
        let japanese_count = voices
            .iter()
            .filter(|v| v.name.starts_with("jf_") || v.name.starts_with("jm_"))
            .count();
        assert_eq!(japanese_count, 5, "Should have 5 Japanese voices");
    }

    #[test]
    fn test_get_kokoro_voices_mandarin_count() {
        let voices = get_kokoro_voices();
        let mandarin_count = voices
            .iter()
            .filter(|v| v.name.starts_with("zf_") || v.name.starts_with("zm_"))
            .count();
        assert_eq!(mandarin_count, 8, "Should have 8 Mandarin Chinese voices");
    }

    #[test]
    fn test_get_kokoro_voices_has_default_voice() {
        let voices = get_kokoro_voices();
        let has_default = voices
            .iter()
            .any(|v| v.name == KokoroTtsProvider::DEFAULT_VOICE);
        assert!(
            has_default,
            "Voice list should include default voice {}",
            KokoroTtsProvider::DEFAULT_VOICE
        );
    }

    #[test]
    fn test_voice_af_heart_properties() {
        let voices = get_kokoro_voices();
        let af_heart = voices.iter().find(|v| v.name == "af_heart").unwrap();

        assert_eq!(af_heart.gender, Gender::Female);
        assert_eq!(af_heart.quality, VoiceQuality::Excellent);
        assert_eq!(af_heart.languages, vec![Language::English]);
    }

    #[test]
    fn test_voice_am_adam_properties() {
        let voices = get_kokoro_voices();
        let am_adam = voices.iter().find(|v| v.name == "am_adam").unwrap();

        assert_eq!(am_adam.gender, Gender::Male);
        assert_eq!(am_adam.quality, VoiceQuality::Excellent);
        assert_eq!(am_adam.languages, vec![Language::English]);
    }

    #[test]
    fn test_voice_bf_emma_properties() {
        let voices = get_kokoro_voices();
        let bf_emma = voices.iter().find(|v| v.name == "bf_emma").unwrap();

        assert_eq!(bf_emma.gender, Gender::Female);
        assert_eq!(bf_emma.languages, vec![Language::English]);
    }

    #[test]
    fn test_voice_jf_alpha_properties() {
        let voices = get_kokoro_voices();
        let jf_alpha = voices.iter().find(|v| v.name == "jf_alpha").unwrap();

        assert_eq!(jf_alpha.gender, Gender::Female);
        assert_eq!(jf_alpha.languages, vec![Language::Custom("ja".into())]);
    }

    #[test]
    fn test_voice_zf_xiaobei_properties() {
        let voices = get_kokoro_voices();
        let zf_xiaobei = voices.iter().find(|v| v.name == "zf_xiaobei").unwrap();

        assert_eq!(zf_xiaobei.gender, Gender::Female);
        assert_eq!(zf_xiaobei.languages, vec![Language::Custom("zh".into())]);
    }

    // ========================================================================
    // List voices trait test
    // ========================================================================

    #[tokio::test]
    async fn test_list_voices_returns_all_voices() {
        let provider = KokoroTtsProvider::new();
        let voices = provider.list_voices().await.unwrap();

        assert_eq!(voices.len(), 54);

        // Verify some specific voices exist
        assert!(voices.iter().any(|v| v.name == "af_heart"));
        assert!(voices.iter().any(|v| v.name == "am_adam"));
        assert!(voices.iter().any(|v| v.name == "bf_emma"));
        assert!(voices.iter().any(|v| v.name == "jm_kumo"));
    }

    // ========================================================================
    // Is ready test
    // ========================================================================

    #[tokio::test]
    async fn test_is_ready_checks_binary() {
        let provider = KokoroTtsProvider::new();
        // Just verify it doesn't panic - result depends on whether kokoro-tts is installed
        let _ = provider.is_ready().await;
    }

    // ========================================================================
    // Integration tests - require kokoro-tts to be installed with model files
    // ========================================================================

    #[tokio::test]
    #[ignore] // Only run manually when kokoro-tts is installed with models
    async fn test_speak_integration() {
        let provider = KokoroTtsProvider::new();

        if !provider.is_ready().await {
            eprintln!("Skipping test: kokoro-tts not installed");
            return;
        }

        let config = TtsConfig::default();
        let result = provider
            .speak("Hello from Kokoro TTS test.", &config)
            .await;

        // This may fail if model files aren't present
        match result {
            Ok(()) => println!("Speech generated successfully"),
            Err(e) => eprintln!("Speech generation failed (model files may be missing): {}", e),
        }
    }

    /// Regression test: Error messages should be captured from stdout.
    ///
    /// Bug: kokoro-tts writes error messages to stdout instead of stderr.
    /// The provider was only capturing stderr, resulting in empty error messages
    /// when kokoro-tts failed (e.g., due to missing model files).
    ///
    /// This test verifies that when kokoro-tts fails, the error message is
    /// properly captured and included in the TtsError.
    #[tokio::test]
    async fn test_error_message_captured_from_stdout() {
        // Skip if kokoro-tts is not installed
        if !KokoroTtsProvider::binary_exists() {
            return;
        }

        // Skip if models are available (we want to test failure case)
        if KokoroTtsProvider::models_available().await {
            return;
        }

        let provider = KokoroTtsProvider::new();
        let config = TtsConfig::default();
        let result = provider.speak("Test", &config).await;

        // Should fail with a meaningful error message
        match result {
            Ok(()) => panic!("Expected failure when model files are missing"),
            Err(TtsError::ProcessFailed { provider: _, stderr }) => {
                // The error message should not be empty
                assert!(!stderr.is_empty(), "Error message should not be empty");
                // Should contain helpful information about missing models
                assert!(
                    stderr.contains("model") || stderr.contains("Error"),
                    "Error should mention missing models or contain 'Error'"
                );
            }
            Err(e) => panic!("Unexpected error type: {:?}", e),
        }
    }
}
