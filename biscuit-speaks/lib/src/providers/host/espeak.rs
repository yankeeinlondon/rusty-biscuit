//! eSpeak/eSpeak-NG TTS provider.
//!
//! Uses the `espeak-ng` or `espeak` command for text-to-speech.
//! Common on Linux systems, also available on macOS and Windows.

use std::process::Stdio;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::errors::TtsError;
use crate::traits::{TtsExecutor, TtsVoiceInventory};
use crate::types::{
    Gender, HostTtsProvider, Language, SpeakResult, SpeedLevel, TtsConfig, TtsProvider, Voice,
    VoiceQuality,
};

/// Default speaking rate for eSpeak in words per minute.
const DEFAULT_RATE_WPM: f32 = 175.0;

/// Characters spoken per second at the default rate (~5 chars/word).
const DEFAULT_CHARS_PER_SEC: f32 = DEFAULT_RATE_WPM * 5.0 / 60.0;

/// Fixed floor added to every speech deadline.
const SPEECH_DEADLINE_FLOOR: Duration = Duration::from_secs(10);

/// eSpeak/eSpeak-NG TTS provider.
///
/// This provider uses `espeak-ng` (preferred) or `espeak` for TTS.
///
/// ## Voice Selection
///
/// - `-v` flag sets the voice/language (e.g., "en", "en-us", "en+f3")
/// - Gender can be specified with suffixes: +m1..+m7 (male), +f1..+f5 (female)
///
/// ## Examples
///
/// ```ignore
/// use biscuit_speaks::providers::host::ESpeakProvider;
/// use biscuit_speaks::{TtsExecutor, TtsConfig};
///
/// let provider = ESpeakProvider::new();
/// provider.speak("Hello, world!", &TtsConfig::default()).await?;
/// ```
#[derive(Debug, Clone)]
pub struct ESpeakProvider {
    /// The binary to use (espeak-ng or espeak).
    binary: String,
}

impl Default for ESpeakProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ESpeakProvider {
    /// Create a new ESpeakProvider, auto-detecting the available binary.
    pub fn new() -> Self {
        // Prefer espeak-ng over espeak
        let binary = if which::which("espeak-ng").is_ok() {
            "espeak-ng".to_string()
        } else {
            "espeak".to_string()
        };
        Self { binary }
    }

    /// Create a provider with a specific binary name.
    pub fn with_binary(binary: impl Into<String>) -> Self {
        Self {
            binary: binary.into(),
        }
    }

    /// Convert a SpeedLevel to words per minute for the `-s` flag.
    ///
    /// Returns `None` for normal speed (use system default).
    fn resolve_rate(speed: SpeedLevel) -> Option<u32> {
        match speed {
            SpeedLevel::Normal => None, // Use default
            _ => {
                let rate = (DEFAULT_RATE_WPM * speed.value()).round() as u32;
                Some(rate)
            }
        }
    }

    /// Build the voice argument based on config.
    fn build_voice_arg(&self, config: &TtsConfig) -> String {
        // Start with language
        let lang = config.language.code_prefix();

        // If a specific voice is requested, use it directly
        if let Some(voice) = &config.requested_voice {
            return voice.clone();
        }

        // Otherwise, build from language + gender
        let gender_suffix = match config.gender {
            Gender::Male => "+m3",
            Gender::Female => "+f3",
            Gender::Any => "",
        };

        format!("{}{}", lang, gender_suffix)
    }

    /// Deadline for one espeak run: double the estimated speech time plus a
    /// fixed floor. Legitimate speech always finishes well inside it; only a
    /// run making no audio progress (wedged audio daemon) can exceed it.
    fn speech_deadline(text: &str, speed: SpeedLevel) -> Duration {
        let chars_per_sec = DEFAULT_CHARS_PER_SEC * speed.value();
        let estimated_secs = text.len() as f32 / chars_per_sec;
        SPEECH_DEADLINE_FLOOR + Duration::from_secs_f32(estimated_secs * 2.0)
    }

    /// Resolve voice to a Voice struct with full metadata.
    fn resolve_voice_full(&self, config: &TtsConfig) -> Voice {
        let voice_arg = self.build_voice_arg(config);
        let gender = config.gender;
        let language = config.language.clone();

        Voice::new(&voice_arg)
            .with_gender(gender)
            .with_quality(VoiceQuality::Low) // eSpeak uses formant synthesis
            .with_language(language)
            .with_identifier(&voice_arg)
    }
}

impl ESpeakProvider {
    /// Run espeak with an explicit deadline (see [`TtsExecutor::speak`]).
    async fn speak_bounded(
        &self,
        text: &str,
        config: &TtsConfig,
        deadline: Duration,
    ) -> Result<(), TtsError> {
        let mut cmd = tokio::process::Command::new(&self.binary);

        // Voice selection
        let voice = self.build_voice_arg(config);
        cmd.arg("-v").arg(&voice);

        // Speed (rate) selection
        if let Some(rate) = Self::resolve_rate(config.speed) {
            cmd.arg("-s").arg(rate.to_string());
        }

        // Use stdin for text input
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| TtsError::ProcessSpawnFailed {
            provider: self.binary.clone(),
            source: e,
        })?;

        // Write text to stdin
        let mut stdin = child.stdin.take().ok_or_else(|| TtsError::StdinPipeError {
            provider: self.binary.clone(),
        })?;

        stdin
            .write_all(text.as_bytes())
            .await
            .map_err(|_| TtsError::StdinWriteError {
                provider: self.binary.clone(),
            })?;

        // CRITICAL: Drop stdin to send EOF signal
        drop(stdin);

        // espeak plays through the host audio daemon itself. A daemon socket
        // that accepts connections but never services them (e.g. the WSLg
        // PulseServer in a headless CI guest) blocks espeak indefinitely, so
        // the wait is bounded and the child killed on expiry.
        let mut stderr_pipe = child.stderr.take();
        let mut stderr_buf = Vec::new();
        let wait_result = tokio::time::timeout(deadline, async {
            if let Some(stderr) = stderr_pipe.as_mut() {
                let _ = stderr.read_to_end(&mut stderr_buf).await;
            }
            child.wait().await
        })
        .await;

        let status = match wait_result {
            Ok(waited) => waited.map_err(|e| TtsError::IoError { source: e })?,
            Err(_elapsed) => {
                let _ = child.start_kill();
                let _ = child.wait().await;
                return Err(TtsError::ProcessTimeout {
                    provider: self.binary.clone(),
                    timeout_secs: deadline.as_secs(),
                });
            }
        };

        if status.success() {
            Ok(())
        } else {
            Err(TtsError::ProcessFailed {
                provider: self.binary.clone(),
                stderr: String::from_utf8_lossy(&stderr_buf).to_string(),
            })
        }
    }
}

impl TtsExecutor for ESpeakProvider {
    async fn speak(&self, text: &str, config: &TtsConfig) -> Result<(), TtsError> {
        self.speak_bounded(text, config, Self::speech_deadline(text, config.speed))
            .await
    }

    async fn is_ready(&self) -> bool {
        // Check if the binary exists and is executable
        which::which(&self.binary).is_ok()
    }

    fn info(&self) -> &str {
        "eSpeak/eSpeak-NG: Open source speech synthesizer with formant synthesis. \
         Supports many languages with compact voice data. Quality is robotic but \
         reliable and fast."
    }

    async fn speak_with_result(
        &self,
        text: &str,
        config: &TtsConfig,
    ) -> Result<SpeakResult, TtsError> {
        // Resolve the voice first
        let voice = self.resolve_voice_full(config);

        // Call speak
        self.speak(text, config).await?;

        // Return the result
        Ok(SpeakResult::new(
            TtsProvider::Host(HostTtsProvider::ESpeak),
            voice,
        ))
    }
}

impl TtsVoiceInventory for ESpeakProvider {
    async fn list_voices(&self) -> Result<Vec<Voice>, TtsError> {
        // Try the current binary first
        let output = tokio::process::Command::new(&self.binary)
            .arg("--voices")
            .output()
            .await
            .map_err(|e| TtsError::VoiceEnumerationFailed {
                provider: self.binary.clone(),
                message: format!("Failed to run '{} --voices': {}", self.binary, e),
            })?;

        if !output.status.success() {
            return Err(TtsError::VoiceEnumerationFailed {
                provider: self.binary.clone(),
                message: format!(
                    "Command failed with status {}: {}",
                    output.status,
                    String::from_utf8_lossy(&output.stderr)
                ),
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        parse_espeak_voices(&stdout, &self.binary)
    }

    async fn default_voice(&self, gender: Gender) -> Result<Voice, TtsError> {
        let (name, voice_gender) = match gender {
            Gender::Male => ("en+m3", Gender::Male),
            Gender::Female | Gender::Any => ("en+f3", Gender::Female),
        };
        Ok(Voice::new(name)
            .with_gender(voice_gender)
            .with_quality(VoiceQuality::Low)
            .with_language(Language::English))
    }
}

/// Parse the output of `espeak-ng --voices` or `espeak --voices`.
///
/// The output is a fixed-width columnar format:
/// ```text
/// Pty Language       Age/Gender VoiceName          File                 Other Languages
///  5  af                 -/M    Afrikaans          gmw/af
///  5  am                 -/-    Amharic            sem/am
/// ```
///
/// Column positions (0-indexed, approximate):
/// - Pty: 0-2 (priority)
/// - Language: 4-18 (language code)
/// - Age/Gender: 19-30 (format: "age/gender" where age is "-" or number, gender is M/F/-)
/// - VoiceName: 31-49 (display name)
/// - File: 50-70 (voice file path)
/// - Other Languages: 71+ (additional language codes)
fn parse_espeak_voices(output: &str, provider: &str) -> Result<Vec<Voice>, TtsError> {
    let mut voices = Vec::new();

    for line in output.lines() {
        // Skip header line and empty lines
        if line.starts_with("Pty") || line.trim().is_empty() {
            continue;
        }

        // Parse the fixed-width columns defensively
        if let Some(voice) = parse_espeak_voice_line(line) {
            voices.push(voice);
        } else {
            // Log unparseable lines but continue (defensive parsing)
            tracing::debug!(
                provider = provider,
                line = line,
                "Skipping unparseable eSpeak voice line"
            );
        }
    }

    Ok(voices)
}

/// Parse a single line from eSpeak voice output.
///
/// Returns `None` if the line cannot be parsed (defensive parsing).
///
/// The espeak/espeak-ng output format is whitespace-separated with these columns:
/// 1. Pty (priority number)
/// 2. Language (language code like "af", "en-gb", "cmn")
/// 3. Age/Gender (format: "--/M", "--/F", "--/-", or "-/M", "-/F", "-/-")
/// 4. VoiceName (display name, may contain underscores for spaces)
/// 5. File (voice file path)
/// 6. Other Languages (optional additional language codes)
///
/// We use whitespace splitting which is more robust than fixed-column positions,
/// as the exact column widths can vary between versions.
fn parse_espeak_voice_line(line: &str) -> Option<Voice> {
    // Split on whitespace
    let parts: Vec<&str> = line.split_whitespace().collect();

    // Need at least 5 parts: Pty, Language, Age/Gender, VoiceName, File
    if parts.len() < 5 {
        return None;
    }

    // parts[0] = priority (numeric)
    // Validate it's a number to skip header line
    if parts[0].parse::<u32>().is_err() {
        return None;
    }

    // parts[1] = language code
    let language_str = parts[1];
    if language_str.is_empty() {
        return None;
    }

    // parts[2] = Age/Gender (e.g., "--/M", "-/F", "--/-")
    let age_gender_str = parts[2];
    let gender = parse_gender_from_age_gender(age_gender_str);

    // parts[3] = Voice name (may have underscores instead of spaces)
    let voice_name_str = parts[3];
    if voice_name_str.is_empty() {
        return None;
    }

    // Parse language to Language enum
    let language = parse_language_code(language_str);

    // Build the voice
    let voice = Voice::new(voice_name_str)
        .with_gender(gender)
        .with_quality(VoiceQuality::Low) // Formant synthesis = low quality
        .with_language(language)
        .with_identifier(language_str); // Use language code as identifier for eSpeak

    Some(voice)
}

/// Parse gender from the Age/Gender field (e.g., "-/M", "-/F", "-/-", "55/M").
///
/// The format is "age/gender" where:
/// - age is "-" (unknown) or a number
/// - gender is "M" (male), "F" (female), or "-" (unknown/any)
fn parse_gender_from_age_gender(field: &str) -> Gender {
    // Split on '/' and take the gender part
    let parts: Vec<&str> = field.split('/').collect();
    if parts.len() < 2 {
        return Gender::Any;
    }

    match parts[1].trim() {
        "M" => Gender::Male,
        "F" => Gender::Female,
        _ => Gender::Any,
    }
}

/// Parse a language code string to a Language enum.
///
/// Maps common codes to known languages, falls back to Custom for others.
fn parse_language_code(code: &str) -> Language {
    // Extract base language code (before any hyphen or variant)
    let base_code = code.split('-').next().unwrap_or(code);

    match base_code {
        "en" => Language::English,
        _ => Language::Custom(code.to_string()),
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Language;

    #[test]
    fn test_espeak_provider_default() {
        let provider = ESpeakProvider::default();
        assert!(provider.binary == "espeak-ng" || provider.binary == "espeak");
    }

    #[test]
    fn test_espeak_provider_with_binary() {
        let provider = ESpeakProvider::with_binary("espeak-ng");
        assert_eq!(provider.binary, "espeak-ng");
    }

    #[test]
    fn test_build_voice_arg_default() {
        let provider = ESpeakProvider::with_binary("espeak-ng");
        let config = TtsConfig::default();
        let voice = provider.build_voice_arg(&config);
        assert_eq!(voice, "en"); // English, any gender
    }

    #[test]
    fn test_build_voice_arg_with_gender() {
        let provider = ESpeakProvider::with_binary("espeak-ng");

        let config = TtsConfig::new().with_gender(Gender::Female);
        let voice = provider.build_voice_arg(&config);
        assert_eq!(voice, "en+f3");

        let config = TtsConfig::new().with_gender(Gender::Male);
        let voice = provider.build_voice_arg(&config);
        assert_eq!(voice, "en+m3");
    }

    #[test]
    fn test_build_voice_arg_with_language() {
        let provider = ESpeakProvider::with_binary("espeak-ng");
        let config = TtsConfig::new().with_language(Language::Custom("de".into()));
        let voice = provider.build_voice_arg(&config);
        assert_eq!(voice, "de");
    }

    #[test]
    fn test_build_voice_arg_explicit_voice() {
        let provider = ESpeakProvider::with_binary("espeak-ng");
        let config = TtsConfig::new().with_voice("en-gb+f4");
        let voice = provider.build_voice_arg(&config);
        assert_eq!(voice, "en-gb+f4");
    }

    #[test]
    fn test_resolve_rate_normal() {
        assert_eq!(ESpeakProvider::resolve_rate(SpeedLevel::Normal), None);
    }

    #[test]
    fn test_resolve_rate_fast() {
        // Fast = 1.25x, so 175 * 1.25 = 219 (rounded)
        let rate = ESpeakProvider::resolve_rate(SpeedLevel::Fast).unwrap();
        assert_eq!(rate, 219);
    }

    #[test]
    fn test_resolve_rate_slow() {
        // Slow = 0.75x, so 175 * 0.75 = 131 (rounded)
        let rate = ESpeakProvider::resolve_rate(SpeedLevel::Slow).unwrap();
        assert_eq!(rate, 131);
    }

    #[test]
    fn test_resolve_rate_explicit() {
        // 2x speed = 175 * 2.0 = 350
        let rate = ESpeakProvider::resolve_rate(SpeedLevel::Explicit(2.0)).unwrap();
        assert_eq!(rate, 350);
    }

    #[test]
    fn test_info() {
        let provider = ESpeakProvider::with_binary("espeak-ng");
        let info = provider.info();
        assert!(info.contains("eSpeak"));
        assert!(info.contains("formant"));
    }

    // ========================================================================
    // Speech deadline tests
    // ========================================================================

    #[test]
    fn test_speech_deadline_has_floor() {
        let deadline = ESpeakProvider::speech_deadline("", SpeedLevel::Normal);
        assert_eq!(deadline, SPEECH_DEADLINE_FLOOR);
    }

    #[test]
    fn test_speech_deadline_scales_with_text_length() {
        let short = ESpeakProvider::speech_deadline("test", SpeedLevel::Normal);
        let long = ESpeakProvider::speech_deadline(&"word ".repeat(500), SpeedLevel::Normal);
        assert!(long > short);
        // 2500 chars at ~14.6 chars/sec is ~171s of speech; the deadline must
        // comfortably cover it.
        assert!(long > Duration::from_secs(300));
    }

    #[test]
    fn test_speech_deadline_faster_speech_shortens_deadline() {
        let text = "some reasonably long sentence to speak aloud";
        let normal = ESpeakProvider::speech_deadline(text, SpeedLevel::Normal);
        let fast = ESpeakProvider::speech_deadline(text, SpeedLevel::Fast);
        assert!(fast < normal);
    }

    /// A child that never exits (wedged audio daemon) must be killed at the
    /// deadline and reported as `ProcessTimeout`, not awaited forever.
    #[cfg(unix)]
    #[tokio::test]
    async fn test_speak_bounded_kills_wedged_child() {
        use std::os::unix::fs::PermissionsExt;

        let script = std::env::temp_dir().join(format!(
            "biscuit-speaks-fake-espeak-{}.sh",
            std::process::id()
        ));
        std::fs::write(&script, "#!/bin/sh\nsleep 30\n").unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();

        let provider = ESpeakProvider::with_binary(script.to_string_lossy());
        let started = std::time::Instant::now();
        let result = provider
            .speak_bounded("test", &TtsConfig::default(), Duration::from_millis(200))
            .await;
        let elapsed = started.elapsed();

        std::fs::remove_file(&script).ok();

        assert!(
            matches!(result, Err(TtsError::ProcessTimeout { .. })),
            "expected ProcessTimeout, got {result:?}"
        );
        assert!(
            elapsed < Duration::from_secs(5),
            "bounded wait took {elapsed:?}"
        );
    }

    // ========================================================================
    // Voice parsing tests
    // ========================================================================

    /// Real output sample from `espeak-ng --voices` (subset)
    /// Format is whitespace-separated: Pty Language Age/Gender VoiceName File [OtherLanguages]
    const ESPEAK_NG_VOICES_SAMPLE: &str = "\
Pty Language Age/Gender VoiceName File Other Languages
5 af --/M Afrikaans gmw/af
5 am --/- Amharic sem/am
5 an --/M Aragonese roa/an
2 ar --/M Arabic sem/ar
5 as --/- Assamese inc/as
5 az --/- Azerbaijani trk/az
5 bg --/- Bulgarian zls/bg
5 bn --/M Bengali inc/bn
5 bs --/M Bosnian zls/bs
5 ca --/M Catalan roa/ca
5 cmn --/M Chinese_(Mandarin) sit/cmn
5 cs --/- Czech zlw/cs
2 cy --/M Welsh cel/cy
5 da --/M Danish gmq/da
5 de --/M German gmw/de
5 en --/M English gmw/en
5 en-gb --/M English_(GB) gmw/en-GB
5 en-us --/M English_(USA) gmw/en-US
5 es --/M Spanish roa/es
5 fr --/M French roa/fr
5 hi --/M Hindi inc/hi
5 it --/M Italian roa/it
5 ja --/M Japanese jpx/ja
5 ko --/M Korean kor/ko
5 nl --/M Dutch gmw/nl
5 pl --/M Polish zlw/pl
5 pt --/M Portuguese roa/pt
5 ru --/M Russian zle/ru
5 zh --/M Chinese sit/zh
";

    #[test]
    fn test_parse_espeak_voices_basic() {
        let voices = parse_espeak_voices(ESPEAK_NG_VOICES_SAMPLE, "espeak-ng").unwrap();

        // Should have parsed all voice lines (not the header)
        assert!(!voices.is_empty());
        assert!(
            voices.len() >= 25,
            "Expected at least 25 voices, got {}",
            voices.len()
        );
    }

    #[test]
    fn test_parse_espeak_voices_gender_male() {
        let voices = parse_espeak_voices(ESPEAK_NG_VOICES_SAMPLE, "espeak-ng").unwrap();

        // Afrikaans should be male (--/M)
        let afrikaans = voices.iter().find(|v| v.name == "Afrikaans").unwrap();
        assert_eq!(afrikaans.gender, Gender::Male);

        // English should be male (--/M)
        let english = voices.iter().find(|v| v.name == "English").unwrap();
        assert_eq!(english.gender, Gender::Male);
    }

    #[test]
    fn test_parse_espeak_voices_gender_any() {
        let voices = parse_espeak_voices(ESPEAK_NG_VOICES_SAMPLE, "espeak-ng").unwrap();

        // Amharic has --/- which means any/unknown gender
        let amharic = voices.iter().find(|v| v.name == "Amharic").unwrap();
        assert_eq!(amharic.gender, Gender::Any);

        // Czech also has --/-
        let czech = voices.iter().find(|v| v.name == "Czech").unwrap();
        assert_eq!(czech.gender, Gender::Any);
    }

    #[test]
    fn test_parse_espeak_voices_quality_low() {
        let voices = parse_espeak_voices(ESPEAK_NG_VOICES_SAMPLE, "espeak-ng").unwrap();

        // All eSpeak voices should be low quality (formant synthesis)
        for voice in &voices {
            assert_eq!(
                voice.quality,
                VoiceQuality::Low,
                "Voice {} should have Low quality",
                voice.name
            );
        }
    }

    #[test]
    fn test_parse_espeak_voices_language_english() {
        let voices = parse_espeak_voices(ESPEAK_NG_VOICES_SAMPLE, "espeak-ng").unwrap();

        // Find English voices
        let english = voices.iter().find(|v| v.name == "English").unwrap();
        assert_eq!(english.languages, vec![Language::English]);

        // en-gb should still map to English (eSpeak uses underscores in names)
        let english_gb = voices.iter().find(|v| v.name == "English_(GB)").unwrap();
        assert_eq!(english_gb.languages, vec![Language::English]);
    }

    #[test]
    fn test_parse_espeak_voices_language_custom() {
        let voices = parse_espeak_voices(ESPEAK_NG_VOICES_SAMPLE, "espeak-ng").unwrap();

        // German should be Custom("de")
        let german = voices.iter().find(|v| v.name == "German").unwrap();
        assert_eq!(german.languages, vec![Language::Custom("de".into())]);

        // French should be Custom("fr")
        let french = voices.iter().find(|v| v.name == "French").unwrap();
        assert_eq!(french.languages, vec![Language::Custom("fr".into())]);
    }

    #[test]
    fn test_parse_espeak_voices_identifier() {
        let voices = parse_espeak_voices(ESPEAK_NG_VOICES_SAMPLE, "espeak-ng").unwrap();

        // Identifier should be the language code
        let german = voices.iter().find(|v| v.name == "German").unwrap();
        assert_eq!(german.identifier, Some("de".into()));

        let english_gb = voices.iter().find(|v| v.name == "English_(GB)").unwrap();
        assert_eq!(english_gb.identifier, Some("en-gb".into()));
    }

    #[test]
    fn test_parse_espeak_voices_multilingual() {
        let voices = parse_espeak_voices(ESPEAK_NG_VOICES_SAMPLE, "espeak-ng").unwrap();

        // Chinese_(Mandarin) should be parsed correctly (eSpeak uses underscores)
        let mandarin = voices
            .iter()
            .find(|v| v.name == "Chinese_(Mandarin)")
            .unwrap();
        assert_eq!(mandarin.gender, Gender::Male);
        assert_eq!(mandarin.languages, vec![Language::Custom("cmn".into())]);
    }

    // ========================================================================
    // Gender parsing tests
    // ========================================================================

    #[test]
    fn test_parse_gender_from_age_gender_male() {
        assert_eq!(parse_gender_from_age_gender("-/M"), Gender::Male);
        assert_eq!(parse_gender_from_age_gender("55/M"), Gender::Male);
        assert_eq!(parse_gender_from_age_gender(" -/M "), Gender::Male);
    }

    #[test]
    fn test_parse_gender_from_age_gender_female() {
        assert_eq!(parse_gender_from_age_gender("-/F"), Gender::Female);
        assert_eq!(parse_gender_from_age_gender("25/F"), Gender::Female);
    }

    #[test]
    fn test_parse_gender_from_age_gender_any() {
        assert_eq!(parse_gender_from_age_gender("-/-"), Gender::Any);
        assert_eq!(parse_gender_from_age_gender("-/X"), Gender::Any);
        assert_eq!(parse_gender_from_age_gender(""), Gender::Any);
        assert_eq!(parse_gender_from_age_gender("invalid"), Gender::Any);
    }

    // ========================================================================
    // Language code parsing tests
    // ========================================================================

    #[test]
    fn test_parse_language_code_english() {
        assert_eq!(parse_language_code("en"), Language::English);
        assert_eq!(parse_language_code("en-gb"), Language::English);
        assert_eq!(parse_language_code("en-us"), Language::English);
        assert_eq!(parse_language_code("en-AU"), Language::English);
    }

    #[test]
    fn test_parse_language_code_custom() {
        assert_eq!(parse_language_code("de"), Language::Custom("de".into()));
        assert_eq!(parse_language_code("fr"), Language::Custom("fr".into()));
        assert_eq!(parse_language_code("cmn"), Language::Custom("cmn".into()));
        assert_eq!(
            parse_language_code("zh-yue"),
            Language::Custom("zh-yue".into())
        );
    }

    // ========================================================================
    // Edge cases and defensive parsing
    // ========================================================================

    #[test]
    fn test_parse_espeak_voice_line_too_short() {
        // Lines shorter than 32 chars should return None
        assert!(parse_espeak_voice_line("short").is_none());
        assert!(parse_espeak_voice_line("").is_none());
        assert!(parse_espeak_voice_line(" 5  en").is_none());
    }

    #[test]
    fn test_parse_espeak_voice_line_header() {
        // Header line should not produce a voice - the first field "Pty" is not numeric
        let header = "Pty Language Age/Gender VoiceName File Other Languages";
        let result = parse_espeak_voice_line(header);
        assert!(result.is_none(), "Header line should not parse to a voice");
    }

    #[test]
    fn test_parse_espeak_voices_empty() {
        let voices = parse_espeak_voices("", "espeak-ng").unwrap();
        assert!(voices.is_empty());
    }

    #[test]
    fn test_parse_espeak_voices_header_only() {
        let output = "Pty Language       Age/Gender VoiceName          File                 Other Languages\n";
        let voices = parse_espeak_voices(output, "espeak-ng").unwrap();
        assert!(voices.is_empty());
    }

    /// Test with output that has some malformed lines mixed in
    #[test]
    fn test_parse_espeak_voices_with_malformed_lines() {
        let output = "\
Pty Language Age/Gender VoiceName File Other
5 en --/M English gmw/en
malformed line here
5 de --/M German gmw/de
another bad line
5 fr --/F French roa/fr
";
        let voices = parse_espeak_voices(output, "espeak-ng").unwrap();

        // Should parse the valid lines, skip the invalid ones
        assert_eq!(voices.len(), 3);
        assert!(voices.iter().any(|v| v.name == "English"));
        assert!(voices.iter().any(|v| v.name == "German"));
        assert!(voices.iter().any(|v| v.name == "French"));
    }

    /// Test with female voice (from espeak-ng variant voices)
    #[test]
    fn test_parse_espeak_voice_female() {
        let output = "\
Pty Language Age/Gender VoiceName File Other
5 en --/F English_(female) gmw/en+f3
";
        let voices = parse_espeak_voices(output, "espeak-ng").unwrap();

        assert_eq!(voices.len(), 1);
        let voice = &voices[0];
        assert_eq!(voice.name, "English_(female)");
        assert_eq!(voice.gender, Gender::Female);
    }

    /// Test from actual espeak (older) output format
    /// Legacy espeak may have slightly different formatting but same whitespace structure
    const ESPEAK_LEGACY_VOICES_SAMPLE: &str = "\
Pty Language Age/Gender VoiceName File Other
5 af --/M afrikaans other/af
5 en --/M english default
5 en-gb --/M english other/en-gb
5 en-us --/M english-us other/en-us
5 de --/M german other/de
5 fr --/M french other/fr
";

    #[test]
    fn test_parse_espeak_legacy_voices() {
        let voices = parse_espeak_voices(ESPEAK_LEGACY_VOICES_SAMPLE, "espeak").unwrap();

        assert_eq!(voices.len(), 6);

        // Legacy espeak uses lowercase voice names
        let english = voices.iter().find(|v| v.name == "english").unwrap();
        assert_eq!(english.gender, Gender::Male);
        assert_eq!(english.languages, vec![Language::English]);
    }

    // ========================================================================
    // default_voice tests
    // ========================================================================

    #[tokio::test]
    async fn test_default_voice_male() {
        let provider = ESpeakProvider::new();
        let voice = provider.default_voice(Gender::Male).await.unwrap();
        assert_eq!(voice.name, "en+m3");
        assert_eq!(voice.gender, Gender::Male);
        assert_eq!(voice.quality, VoiceQuality::Low);
    }

    #[tokio::test]
    async fn test_default_voice_female() {
        let provider = ESpeakProvider::new();
        let voice = provider.default_voice(Gender::Female).await.unwrap();
        assert_eq!(voice.name, "en+f3");
        assert_eq!(voice.gender, Gender::Female);
        assert_eq!(voice.quality, VoiceQuality::Low);
    }

    #[tokio::test]
    async fn test_default_voice_any() {
        let provider = ESpeakProvider::new();
        let voice = provider.default_voice(Gender::Any).await.unwrap();
        assert_eq!(voice.name, "en+f3");
        assert_eq!(voice.gender, Gender::Female);
        assert_eq!(voice.quality, VoiceQuality::Low);
    }

    // ========================================================================
    // Integration test (only runs if espeak-ng is installed)
    // ========================================================================

    #[tokio::test]
    async fn test_is_ready_with_valid_binary() {
        // This test checks if espeak-ng is available on the system
        let provider = ESpeakProvider::with_binary("espeak-ng");
        let is_ready = provider.is_ready().await;

        // We can't assert true/false since it depends on the system
        // Just ensure it doesn't panic
        let _ = is_ready;
    }

    #[tokio::test]
    async fn test_is_ready_with_invalid_binary() {
        let provider = ESpeakProvider::with_binary("nonexistent-espeak-binary-xyz");
        let is_ready = provider.is_ready().await;

        // Should return false for non-existent binary
        assert!(!is_ready);
    }

    #[tokio::test]
    #[ignore] // Only run manually when espeak-ng is installed
    async fn test_list_voices_integration() {
        let provider = ESpeakProvider::new();

        if !provider.is_ready().await {
            eprintln!("Skipping test: espeak-ng not installed");
            return;
        }

        let voices = provider.list_voices().await.unwrap();

        // Should have voices
        assert!(!voices.is_empty(), "Expected at least one voice");

        // All should be low quality
        for voice in &voices {
            assert_eq!(voice.quality, VoiceQuality::Low);
        }

        // Should have an English voice
        assert!(
            voices
                .iter()
                .any(|v| v.languages.contains(&Language::English)),
            "Expected at least one English voice"
        );

        println!("Found {} voices", voices.len());
        for voice in voices.iter().take(10) {
            println!(
                "  - {} ({:?}): {:?}",
                voice.name, voice.gender, voice.languages
            );
        }
    }
}
