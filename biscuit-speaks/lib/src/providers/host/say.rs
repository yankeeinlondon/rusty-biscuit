//! macOS Say TTS provider.
//!
//! Uses the built-in `say` command on macOS for text-to-speech.
//!
//! Audio is cached by voice and words-per-minute rate. With no requested voice
//! or gender, the system voice comes from `system_defaults`; a request whose
//! voice or rate stays unresolved is synthesized fresh and never cached, so
//! `prepare` does nothing for it. The system rate is not read yet, so this
//! holds for every normal-speed request.

mod system_defaults;

use std::path::{Path, PathBuf};
use std::process::Stdio;

use tokio::io::AsyncWriteExt;
use tracing::{debug, trace};

use crate::audio_cache::CacheKey;
use crate::errors::TtsError;
use crate::gender_inference::infer_gender;
use crate::prepared::{ResolvedSynthesis, dry_run_enabled, publish, staging_file};
use crate::traits::{TtsExecutor, TtsVoiceInventory};
use crate::types::{
    Gender, HostTtsProvider, Language, SpeakResult, SpeedLevel, TtsConfig, TtsProvider, Voice,
    VoiceQuality,
};
use system_defaults::{OsSpeechDefaults, SystemSpeechDefaults};

/// The rate, in words per minute, an explicit speed multiplier scales.
///
/// Not the system's current rate, which `say` reads from user settings.
const DEFAULT_RATE_WPM: f32 = 175.0;

/// macOS Say TTS provider.
///
/// This provider uses the `say` command available on all macOS systems.
///
/// ## Voice Selection
///
/// The `-v` flag selects the voice by name (e.g., "Samantha", "Alex").
/// Speech is synthesized to PCM WAV and played through Playa so volume is
/// effective without changing the selected voice. Without `playa`, macOS
/// `afplay -v` provides volume-controlled playback.
///
/// ## Caching
///
/// Audio is reused through the shared audio cache only when both the voice
/// and the words-per-minute rate are known: requested, implied by gender or
/// speed, or read from the system's effective settings. Otherwise `say`
/// picks the missing setting and the audio is synthesized afresh each time.
///
/// ## Examples
///
/// ```ignore
/// use biscuit_speaks::providers::host::SayProvider;
/// use biscuit_speaks::{TtsExecutor, TtsConfig};
///
/// let provider = SayProvider;
/// provider.speak("Hello, world!", &TtsConfig::default()).await?;
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct SayProvider;

impl SayProvider {
    /// Provider name constant for error messages.
    const PROVIDER_NAME: &'static str = "say";

    fn resolve_voice(config: &TtsConfig) -> Option<&str> {
        if let Some(voice) = &config.requested_voice {
            return Some(voice.as_str());
        }

        match config.gender {
            Gender::Male => Some("Alex"),
            Gender::Female => Some("Samantha"),
            Gender::Any => None,
        }
    }

    /// Convert a SpeedLevel to words per minute for the `-r` flag.
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

    /// Select the best voice from a list based on config constraints.
    ///
    /// Selection priority:
    /// 1. Filter by language (if specified)
    /// 2. Filter by gender (if specified)
    /// 3. Sort by quality (highest first)
    /// 4. Return the first match
    fn select_best_voice(voices: &[Voice], config: &TtsConfig) -> Option<Voice> {
        let mut candidates: Vec<&Voice> = voices.iter().collect();

        // Filter by language if specified
        let target_language = &config.language;
        candidates.retain(|v| {
            v.languages.iter().any(|lang| {
                match (lang, target_language) {
                    (Language::English, Language::English) => true,
                    (Language::Custom(a), Language::Custom(b)) => {
                        // Match if same language code or if one is a prefix of the other
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
        });

        // Filter by gender if specified (not Any)
        if config.gender != Gender::Any {
            let gender_matches: Vec<&Voice> = candidates
                .iter()
                .filter(|v| v.gender == config.gender)
                .copied()
                .collect();

            // Only apply gender filter if we have matches
            if !gender_matches.is_empty() {
                candidates = gender_matches;
            }
        }

        // Sort by quality (highest first), then name alphabetically
        candidates.sort_by(|a, b| {
            a.quality
                .rank()
                .cmp(&b.quality.rank())
                .then_with(|| a.name.cmp(&b.name))
        });

        candidates.first().cloned().cloned()
    }

    /// Parse a single line of `say -v '?'` output into a Voice.
    ///
    /// The format is:
    /// ```text
    /// VoiceName           locale    # Sample text
    /// VoiceName (Qualifier) locale  # Sample text
    /// ```
    ///
    /// Returns `None` for lines that:
    /// - Cannot be parsed
    /// - Contain "Eloquence" (low quality robotic voices to filter out)
    fn parse_voice_line(line: &str) -> Option<Voice> {
        // Skip empty lines
        let line = line.trim();
        if line.is_empty() {
            return None;
        }

        // Split on '#' to separate metadata from sample text
        let parts: Vec<&str> = line.splitn(2, '#').collect();
        if parts.is_empty() {
            return None;
        }

        let metadata = parts[0].trim();

        // Split metadata to get voice name and locale
        // The format uses multiple spaces as delimiter, locale is the last token
        let tokens: Vec<&str> = metadata.split_whitespace().collect();
        if tokens.len() < 2 {
            trace!(line = line, "Skipping line with insufficient tokens");
            return None;
        }

        // Last token is the locale (e.g., "en_US", "fr_FR")
        let locale = *tokens.last()?;

        // Everything before the locale is the voice name
        // Need to reconstruct it carefully because of potential parentheses
        let name_end = metadata.rfind(locale)?;
        let name = metadata[..name_end].trim();

        if name.is_empty() {
            trace!(line = line, "Skipping line with empty name");
            return None;
        }

        // Filter out Eloquence voices (low quality robotic voices)
        if name.contains("Eloquence") {
            debug!(name = name, "Filtering out Eloquence voice");
            return None;
        }

        // Determine quality based on name suffixes
        let quality = if name.contains("(Enhanced)") || name.contains("(Premium)") {
            VoiceQuality::Good
        } else {
            VoiceQuality::Moderate
        };

        // Extract the base name for gender inference
        // Remove qualifiers like "(Enhanced)", "(Premium)", "(English (US))"
        let base_name = Self::extract_base_name(name);
        let gender = infer_gender(&base_name);

        // Parse locale to Language
        let language = Self::parse_locale_to_language(locale);

        Some(
            Voice::new(name)
                .with_gender(gender)
                .with_quality(quality)
                .with_language(language),
        )
    }

    /// Extract the base name from a voice name, removing qualifiers.
    ///
    /// Examples:
    /// - "Samantha (Enhanced)" -> "Samantha"
    /// - "Eddy (English (US))" -> "Eddy"
    /// - "Albert" -> "Albert"
    fn extract_base_name(name: &str) -> String {
        // Find the first opening parenthesis and take everything before it
        if let Some(paren_pos) = name.find('(') {
            name[..paren_pos].trim().to_string()
        } else {
            name.trim().to_string()
        }
    }

    /// Parse a locale string (e.g., "en_US") into a Language.
    fn parse_locale_to_language(locale: &str) -> Language {
        // Common English locale prefixes
        if locale.starts_with("en") {
            Language::English
        } else {
            // Return the full locale as a custom language
            Language::Custom(locale.replace('_', "-"))
        }
    }

    /// Resolve the voice and rate every entry point synthesizes with.
    ///
    /// A requested voice, then the gender's voice, then the system voice; an
    /// explicit speed's rate, then the system rate. The system voice is only
    /// looked up once the rate is known, since an unknown rate leaves the
    /// request unresolved either way.
    async fn resolve(config: &TtsConfig, defaults: &impl SystemSpeechDefaults) -> Resolution {
        let rate_wpm = match Self::resolve_rate(config.speed) {
            Some(rate) => Some(rate),
            None => defaults.rate_wpm().await,
        };
        let voice = match Self::resolve_voice(config) {
            Some(voice) => Some(voice.to_owned()),
            None if rate_wpm.is_some() => defaults.voice().await,
            None => None,
        };
        Resolution { voice, rate_wpm }
    }

    /// The shared-cache identity; the rate is part of it because `say` bakes
    /// it into the audio.
    fn cache_key(text: &str, voice: &str, rate_wpm: u32) -> CacheKey {
        CacheKey::new("say", format!("{voice};rate={rate_wpm}"), text, "wav")
    }

    /// The cache entry for a fully resolved request, if it is published.
    #[cfg_attr(not(feature = "playa"), allow(dead_code))]
    fn cached_path(text: &str, resolution: &Resolution) -> Option<PathBuf> {
        let (voice, rate_wpm) = resolution.concrete()?;
        let path = Self::cache_key(text, voice, rate_wpm).cache_path();
        path.is_file().then_some(path)
    }

    /// Playback settings for Say audio, whose rate is already baked in.
    #[cfg_attr(not(feature = "playa"), allow(dead_code))]
    fn playback_config(config: &TtsConfig) -> TtsConfig {
        let mut playback = config.clone();
        playback.speed = SpeedLevel::Normal;
        playback
    }

    /// Synthesize `resolution`, through the shared cache when it is fully
    /// resolved.
    ///
    /// A cache miss writes a staging file and publishes it atomically; a
    /// failure or a dropped future kills the child and deletes the staging
    /// file, so no partial entry is published. An unresolved request passes
    /// only the settings it knows, so `say` keeps choosing the rest.
    async fn synthesize(text: &str, resolution: &Resolution) -> Result<Synthesized, TtsError> {
        let Some((voice, rate_wpm)) = resolution.concrete() else {
            let output = tempfile::Builder::new()
                .prefix(".biscuit-speaks-uncached-")
                .suffix(".wav")
                .tempfile()
                .map_err(|source| TtsError::TempFileError { source })?
                .into_temp_path();
            Self::run_say(text, &output, resolution.voice.as_deref(), resolution.rate_wpm).await?;
            return Ok(Synthesized::Uncached(output));
        };

        let cache_path = Self::cache_key(text, voice, rate_wpm).cache_path();
        if cache_path.is_file() {
            debug!(cache_path = %cache_path.display(), "Cache hit for Say");
            return Ok(Synthesized::Cached { path: cache_path, hit: true });
        }
        debug!(cache_path = %cache_path.display(), "Cache miss for Say, generating");
        let staged = staging_file(&cache_path)?;
        Self::run_say(text, &staged, Some(voice), Some(rate_wpm)).await?;
        publish(staged, &cache_path)?;
        Ok(Synthesized::Cached { path: cache_path, hit: false })
    }

    /// Run `say` into `output`, feeding `text` on stdin.
    async fn run_say(
        text: &str,
        output_path: &Path,
        voice: Option<&str>,
        rate_wpm: Option<u32>,
    ) -> Result<(), TtsError> {
        let mut cmd = tokio::process::Command::new("say");

        cmd.arg("-o")
            .arg(output_path)
            .arg("--file-format=WAVE")
            .arg("--data-format=LEI16");

        if let Some(voice) = voice {
            cmd.arg("-v").arg(voice);
        }

        if let Some(rate) = rate_wpm {
            cmd.arg("-r").arg(rate.to_string());
        }

        cmd.kill_on_drop(true);
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| TtsError::ProcessSpawnFailed {
            provider: Self::PROVIDER_NAME.into(),
            source: e,
        })?;

        let mut stdin = child.stdin.take().ok_or_else(|| TtsError::StdinPipeError {
            provider: Self::PROVIDER_NAME.into(),
        })?;

        stdin
            .write_all(text.as_bytes())
            .await
            .map_err(|_| TtsError::StdinWriteError {
                provider: Self::PROVIDER_NAME.into(),
            })?;

        // Close stdin so say can observe EOF and finish synthesis.
        drop(stdin);

        let output = child
            .wait_with_output()
            .await
            .map_err(|e| TtsError::IoError { source: e })?;

        if !output.status.success() {
            return Err(TtsError::ProcessFailed {
                provider: Self::PROVIDER_NAME.into(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }
        // An empty file would otherwise be published and reused as speech.
        if std::fs::metadata(output_path).map_or(true, |metadata| metadata.len() == 0) {
            return Err(TtsError::ProcessFailed {
                provider: Self::PROVIDER_NAME.into(),
                stderr: "say produced no audio".into(),
            });
        }
        Ok(())
    }

    /// Play synthesized audio at the volume `config` requests.
    async fn play_report(
        audio: &Path,
        config: &TtsConfig,
    ) -> Result<crate::types::SpeakPlaybackReport, TtsError> {
        #[cfg(feature = "playa")]
        {
            crate::playback::play_audio_file_with_report(
                audio,
                crate::types::AudioFormat::Wav,
                &Self::playback_config(config),
            )
            .await
        }
        #[cfg(not(feature = "playa"))]
        {
            let started = std::time::Instant::now();
            let output = tokio::process::Command::new("afplay")
                .arg("-v")
                .arg(config.volume.value().to_string())
                .arg(audio)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::piped())
                .kill_on_drop(true)
                .output()
                .await?;
            if !output.status.success() {
                return Err(TtsError::ProcessFailed {
                    provider: "afplay".into(),
                    stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                });
            }
            Ok(crate::types::SpeakPlaybackReport {
                route: crate::types::SpeakPlaybackRoute::Host("afplay".into()),
                expected_millis: None,
                elapsed_millis: started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
                verdict: crate::types::SpeakPlaybackVerdict::Unverified,
            })
        }
    }

    /// Prepare the cache entry and report the settings it was prepared with.
    ///
    /// `None` when there is nothing to prepare: dry run, or a system voice or
    /// rate that cannot be resolved.
    pub(crate) async fn prepare_resolved(
        &self,
        text: &str,
        config: &TtsConfig,
    ) -> Result<Option<ResolvedSynthesis>, TtsError> {
        Self::prepare_with(text, config, &OsSpeechDefaults).await
    }

    async fn prepare_with(
        text: &str,
        config: &TtsConfig,
        defaults: &impl SystemSpeechDefaults,
    ) -> Result<Option<ResolvedSynthesis>, TtsError> {
        if dry_run_enabled() {
            return Ok(None);
        }
        let resolution = Self::resolve(config, defaults).await;
        let Some((voice, rate_wpm)) = resolution.concrete() else {
            return Ok(None);
        };
        Self::synthesize(text, &resolution).await?;
        Ok(Some(ResolvedSynthesis::Say {
            voice: voice.to_owned(),
            rate_wpm,
        }))
    }

    /// Play `resolution` at `config`'s volume, through the shared cache when
    /// it is fully resolved.
    pub(crate) async fn speak_resolved(
        text: &str,
        resolution: &Resolution,
        config: &TtsConfig,
    ) -> Result<(), TtsError> {
        let audio = Self::synthesize(text, resolution).await?;
        Self::play_report(audio.path(), config).await.map(|_| ())
    }

    /// [`Self::speak_resolved`], reporting the voice and cache entry played.
    ///
    /// Without a resolved voice, the best listed match is named and spoken
    /// with, so the metadata names the voice actually heard.
    pub(crate) async fn speak_resolved_with_result(
        &self,
        text: &str,
        mut resolution: Resolution,
        config: &TtsConfig,
    ) -> Result<SpeakResult, TtsError> {
        let listed = self.list_voices().await;
        let voice = match &resolution.voice {
            Some(name) => listed
                .ok()
                .and_then(|voices| voices.into_iter().find(|voice| voice.name == *name))
                .unwrap_or_else(|| Voice::new(name.clone()).with_language(Language::English)),
            None => {
                let selected = Self::select_best_voice(&listed?, config).ok_or_else(|| {
                    TtsError::VoiceEnumerationFailed {
                        provider: Self::PROVIDER_NAME.into(),
                        message: "No matching voice found for the given constraints".into(),
                    }
                })?;
                resolution.voice = Some(selected.name.clone());
                selected
            }
        };

        let audio = Self::synthesize(text, &resolution).await?;
        let playback = Self::play_report(audio.path(), config).await?;
        let result = SpeakResult::new(TtsProvider::Host(HostTtsProvider::Say), voice)
            .with_codec("wav")
            .with_playback(playback);
        Ok(match audio {
            Synthesized::Cached { path, hit } => result.with_audio_file(path).with_cache_hit(hit),
            Synthesized::Uncached(_) => result,
        })
    }

    /// A ready detached job for `resolution`, synthesizing it first on a miss.
    #[cfg(feature = "playa")]
    pub(crate) async fn resolved_job(
        text: &str,
        resolution: &Resolution,
        config: &TtsConfig,
    ) -> Result<playa::detached::SpoolJob, TtsError> {
        let path = match Self::synthesize(text, resolution).await? {
            Synthesized::Cached { path, .. } => path,
            // Playa may delete only spool-owned files, so this uncached audio
            // outlives the job.
            Synthesized::Uncached(path) => path.keep().map_err(|error| TtsError::TempFileError {
                source: error.error,
            })?,
        };
        Ok(crate::playa_bridge::file_job(path, &Self::playback_config(config)))
    }

    /// A ready detached job for a fully resolved request whose file is cached.
    #[cfg(feature = "playa")]
    pub(crate) fn cached_resolved_job(
        text: &str,
        resolution: &Resolution,
        config: &TtsConfig,
    ) -> Option<playa::detached::SpoolJob> {
        Self::cached_path(text, resolution)
            .map(|path| crate::playa_bridge::file_job(path, &Self::playback_config(config)))
    }

    /// Check if the `say` binary exists on the system.
    async fn say_binary_exists() -> bool {
        which::which("say").is_ok()
    }
}

/// The voice and rate a request resolved to; `None` is unresolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Resolution {
    voice: Option<String>,
    rate_wpm: Option<u32>,
}

impl Resolution {
    /// The settings a preparation retained, both known.
    pub(crate) fn retained(voice: &str, rate_wpm: u32) -> Self {
        Self {
            voice: Some(voice.to_owned()),
            rate_wpm: Some(rate_wpm),
        }
    }

    /// Both settings, when both are known: only then may the audio be cached.
    fn concrete(&self) -> Option<(&str, u32)> {
        Some((self.voice.as_deref()?, self.rate_wpm?))
    }
}

/// Audio a Say entry point plays.
enum Synthesized {
    /// A shared cache entry, and whether it already existed.
    Cached { path: PathBuf, hit: bool },
    /// Audio for an unresolved request, deleted when dropped.
    Uncached(tempfile::TempPath),
}

impl Synthesized {
    fn path(&self) -> &Path {
        match self {
            Self::Cached { path, .. } => path,
            Self::Uncached(path) => path,
        }
    }
}

impl TtsExecutor for SayProvider {
    async fn speak(&self, text: &str, config: &TtsConfig) -> Result<(), TtsError> {
        let resolution = Self::resolve(config, &OsSpeechDefaults).await;
        Self::speak_resolved(text, &resolution, config).await
    }

    async fn prepare(&self, text: &str, config: &TtsConfig) -> Result<(), TtsError> {
        self.prepare_resolved(text, config).await.map(|_| ())
    }

    /// Check if the Say provider is ready.
    ///
    /// Returns `true` only on macOS when the `say` binary exists.
    async fn is_ready(&self) -> bool {
        // Only available on macOS
        if !cfg!(target_os = "macos") {
            return false;
        }

        Self::say_binary_exists().await
    }

    /// Get provider information.
    fn info(&self) -> &str {
        "macOS Say - Built-in speech synthesis using the `say` command"
    }

    async fn speak_with_result(
        &self,
        text: &str,
        config: &TtsConfig,
    ) -> Result<SpeakResult, TtsError> {
        let resolution = Self::resolve(config, &OsSpeechDefaults).await;
        self.speak_resolved_with_result(text, resolution, config).await
    }

    #[cfg(feature = "playa")]
    async fn detached_job(
        &self,
        text: &str,
        config: &TtsConfig,
    ) -> Result<playa::detached::SpoolJob, TtsError> {
        let resolution = Self::resolve(config, &OsSpeechDefaults).await;
        Self::resolved_job(text, &resolution, config).await
    }

    #[cfg(feature = "playa")]
    async fn cached_detached_job(
        &self,
        text: &str,
        config: &TtsConfig,
    ) -> Result<Option<playa::detached::SpoolJob>, TtsError> {
        let resolution = Self::resolve(config, &OsSpeechDefaults).await;
        Ok(Self::cached_resolved_job(text, &resolution, config))
    }
}

impl TtsVoiceInventory for SayProvider {
    /// List all available voices from macOS `say`.
    ///
    /// Parses the output of `say -v '?'` to extract voice metadata.
    /// Filters out Eloquence voices (low quality robotic voices).
    async fn list_voices(&self) -> Result<Vec<Voice>, TtsError> {
        // Run `say -v '?'` to list voices
        let output = tokio::process::Command::new("say")
            .arg("-v")
            .arg("?")
            .output()
            .await
            .map_err(|e| TtsError::ProcessSpawnFailed {
                provider: Self::PROVIDER_NAME.into(),
                source: e,
            })?;

        if !output.status.success() {
            return Err(TtsError::VoiceEnumerationFailed {
                provider: Self::PROVIDER_NAME.into(),
                message: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let voices: Vec<Voice> = stdout.lines().filter_map(Self::parse_voice_line).collect();

        debug!(
            provider = Self::PROVIDER_NAME,
            voice_count = voices.len(),
            "Enumerated voices"
        );

        Ok(voices)
    }

    async fn default_voice(&self, gender: Gender) -> Result<Voice, TtsError> {
        let voices = self.list_voices().await?;

        let config = TtsConfig::new()
            .with_gender(gender)
            .with_language(Language::English);

        Self::select_best_voice(&voices, &config).ok_or_else(|| TtsError::VoiceEnumerationFailed {
            provider: Self::PROVIDER_NAME.into(),
            message: "No voices available".into(),
        })
    }
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
    fn test_say_provider_default() {
        let provider = SayProvider;
        let _ = provider; // Just ensure it compiles
    }

    #[test]
    fn test_resolve_voice_requested() {
        let config = TtsConfig::new().with_voice("Alex");
        assert_eq!(SayProvider::resolve_voice(&config), Some("Alex"));
    }

    #[test]
    fn test_resolve_voice_gender_male() {
        let config = TtsConfig::new().with_gender(Gender::Male);
        assert_eq!(SayProvider::resolve_voice(&config), Some("Alex"));
    }

    #[test]
    fn test_resolve_voice_gender_female() {
        let config = TtsConfig::new().with_gender(Gender::Female);
        assert_eq!(SayProvider::resolve_voice(&config), Some("Samantha"));
    }

    #[test]
    fn test_resolve_voice_gender_any() {
        let config = TtsConfig::new();
        assert_eq!(SayProvider::resolve_voice(&config), None);
    }

    #[test]
    fn test_resolve_rate_normal() {
        assert_eq!(SayProvider::resolve_rate(SpeedLevel::Normal), None);
    }

    #[test]
    fn test_resolve_rate_fast() {
        // Fast = 1.25x, so 175 * 1.25 = 219 (rounded)
        let rate = SayProvider::resolve_rate(SpeedLevel::Fast).unwrap();
        assert_eq!(rate, 219);
    }

    #[test]
    fn test_resolve_rate_slow() {
        // Slow = 0.75x, so 175 * 0.75 = 131 (rounded)
        let rate = SayProvider::resolve_rate(SpeedLevel::Slow).unwrap();
        assert_eq!(rate, 131);
    }

    #[test]
    fn test_resolve_rate_explicit() {
        // 2x speed = 175 * 2.0 = 350
        let rate = SayProvider::resolve_rate(SpeedLevel::Explicit(2.0)).unwrap();
        assert_eq!(rate, 350);
    }

    #[test]
    fn test_info() {
        let provider = SayProvider;
        assert!(provider.info().contains("macOS"));
        assert!(provider.info().contains("say"));
    }

    // ========================================================================
    // Voice line parsing tests
    // ========================================================================

    #[test]
    fn test_parse_simple_voice() {
        let line = "Albert              en_US    # Hello! My name is Albert.";
        let voice = SayProvider::parse_voice_line(line).unwrap();

        assert_eq!(voice.name, "Albert");
        assert_eq!(voice.gender, Gender::Male);
        assert_eq!(voice.quality, VoiceQuality::Moderate);
        assert_eq!(voice.languages, vec![Language::English]);
    }

    #[test]
    fn test_parse_enhanced_voice() {
        let line = "Samantha (Enhanced) en_US    # Hello! My name is Samantha.";
        let voice = SayProvider::parse_voice_line(line).unwrap();

        assert_eq!(voice.name, "Samantha (Enhanced)");
        assert_eq!(voice.gender, Gender::Female);
        assert_eq!(voice.quality, VoiceQuality::Good);
        assert_eq!(voice.languages, vec![Language::English]);
    }

    #[test]
    fn test_parse_premium_voice() {
        let line = "Karen (Premium)     en_AU    # Hello! My name is Karen.";
        let voice = SayProvider::parse_voice_line(line).unwrap();

        assert_eq!(voice.name, "Karen (Premium)");
        // Karen is classified as Male by gender_guesser (Danish male name)
        assert_eq!(voice.gender, Gender::Male);
        assert_eq!(voice.quality, VoiceQuality::Good);
        assert_eq!(voice.languages, vec![Language::English]);
    }

    #[test]
    fn test_parse_voice_with_language_qualifier() {
        let line = "Eddy (English (US)) en_US    # Hello! My name is Eddy.";
        let voice = SayProvider::parse_voice_line(line).unwrap();

        assert_eq!(voice.name, "Eddy (English (US))");
        // Eddy is an ambiguous name, so gender_guesser returns Any
        assert_eq!(voice.quality, VoiceQuality::Moderate);
        assert_eq!(voice.languages, vec![Language::English]);
    }

    #[test]
    fn test_parse_non_english_voice() {
        let line = "Alice               it_IT    # Ciao! Mi chiamo Alice.";
        let voice = SayProvider::parse_voice_line(line).unwrap();

        assert_eq!(voice.name, "Alice");
        assert_eq!(voice.gender, Gender::Female);
        assert_eq!(voice.quality, VoiceQuality::Moderate);
        assert_eq!(voice.languages, vec![Language::Custom("it-IT".into())]);
    }

    #[test]
    fn test_parse_french_voice() {
        let line = "Amélie              fr_CA    # Bonjour! Je m'appelle Amélie.";
        let voice = SayProvider::parse_voice_line(line).unwrap();

        assert_eq!(voice.name, "Amélie");
        assert_eq!(voice.quality, VoiceQuality::Moderate);
        assert_eq!(voice.languages, vec![Language::Custom("fr-CA".into())]);
    }

    #[test]
    fn test_parse_german_voice() {
        let line = "Anna                de_DE    # Hallo! Ich heiße Anna.";
        let voice = SayProvider::parse_voice_line(line).unwrap();

        assert_eq!(voice.name, "Anna");
        // Anna is classified as BothMaleFemale by gender_guesser (varies by culture)
        assert_eq!(voice.gender, Gender::Any);
        assert_eq!(voice.quality, VoiceQuality::Moderate);
        assert_eq!(voice.languages, vec![Language::Custom("de-DE".into())]);
    }

    #[test]
    fn test_parse_voice_en_gb() {
        let line = "Daniel              en_GB    # Hello! My name is Daniel.";
        let voice = SayProvider::parse_voice_line(line).unwrap();

        assert_eq!(voice.name, "Daniel");
        assert_eq!(voice.gender, Gender::Male);
        assert_eq!(voice.languages, vec![Language::English]);
    }

    #[test]
    fn test_parse_voice_en_in() {
        let line = "Aman (English (India)) en_IN    # Hello! My name is Aman.";
        let voice = SayProvider::parse_voice_line(line).unwrap();

        assert_eq!(voice.name, "Aman (English (India))");
        assert_eq!(voice.languages, vec![Language::English]);
    }

    #[test]
    fn test_filter_eloquence_voice() {
        let line = "Eloquence Clara     en_US    # Hello! My name is Clara.";
        let voice = SayProvider::parse_voice_line(line);

        assert!(voice.is_none(), "Eloquence voices should be filtered out");
    }

    #[test]
    fn test_parse_empty_line() {
        assert!(SayProvider::parse_voice_line("").is_none());
        assert!(SayProvider::parse_voice_line("   ").is_none());
    }

    #[test]
    fn test_parse_malformed_line() {
        // No locale - just a name
        assert!(SayProvider::parse_voice_line("BadVoice").is_none());
    }

    #[test]
    fn test_parse_novelty_voices() {
        // macOS includes some novelty/effect voices
        let line = "Bells               en_US    # Hello! My name is Bells.";
        let voice = SayProvider::parse_voice_line(line).unwrap();

        assert_eq!(voice.name, "Bells");
        // "Bells" is not a human name, so gender should be Any
        assert_eq!(voice.gender, Gender::Any);
        assert_eq!(voice.quality, VoiceQuality::Moderate);
    }

    #[test]
    fn test_parse_bad_news_voice() {
        let line = "Bad News            en_US    # Hello! My name is Bad News.";
        let voice = SayProvider::parse_voice_line(line).unwrap();

        assert_eq!(voice.name, "Bad News");
        assert_eq!(voice.quality, VoiceQuality::Moderate);
    }

    // ========================================================================
    // Base name extraction tests
    // ========================================================================

    #[test]
    fn test_extract_base_name_simple() {
        assert_eq!(SayProvider::extract_base_name("Albert"), "Albert");
    }

    #[test]
    fn test_extract_base_name_enhanced() {
        assert_eq!(
            SayProvider::extract_base_name("Samantha (Enhanced)"),
            "Samantha"
        );
    }

    #[test]
    fn test_extract_base_name_premium() {
        assert_eq!(SayProvider::extract_base_name("Karen (Premium)"), "Karen");
    }

    #[test]
    fn test_extract_base_name_nested_parens() {
        assert_eq!(
            SayProvider::extract_base_name("Eddy (English (US))"),
            "Eddy"
        );
    }

    #[test]
    fn test_extract_base_name_with_spaces() {
        assert_eq!(SayProvider::extract_base_name("Bad News"), "Bad News");
    }

    // ========================================================================
    // Locale parsing tests
    // ========================================================================

    #[test]
    fn test_parse_locale_english_us() {
        assert_eq!(
            SayProvider::parse_locale_to_language("en_US"),
            Language::English
        );
    }

    #[test]
    fn test_parse_locale_english_gb() {
        assert_eq!(
            SayProvider::parse_locale_to_language("en_GB"),
            Language::English
        );
    }

    #[test]
    fn test_parse_locale_english_au() {
        assert_eq!(
            SayProvider::parse_locale_to_language("en_AU"),
            Language::English
        );
    }

    #[test]
    fn test_parse_locale_french() {
        assert_eq!(
            SayProvider::parse_locale_to_language("fr_FR"),
            Language::Custom("fr-FR".into())
        );
    }

    #[test]
    fn test_parse_locale_german() {
        assert_eq!(
            SayProvider::parse_locale_to_language("de_DE"),
            Language::Custom("de-DE".into())
        );
    }

    #[test]
    fn test_parse_locale_japanese() {
        assert_eq!(
            SayProvider::parse_locale_to_language("ja_JP"),
            Language::Custom("ja-JP".into())
        );
    }

    #[test]
    fn test_parse_locale_chinese() {
        assert_eq!(
            SayProvider::parse_locale_to_language("zh_CN"),
            Language::Custom("zh-CN".into())
        );
    }

    // ========================================================================
    // select_best_voice tests
    // ========================================================================

    #[test]
    fn test_default_voice_selection_prefers_highest_quality() {
        let voices = vec![
            Voice::new("Albert")
                .with_gender(Gender::Male)
                .with_quality(VoiceQuality::Moderate)
                .with_language(Language::English),
            Voice::new("Alex (Premium)")
                .with_gender(Gender::Male)
                .with_quality(VoiceQuality::Good)
                .with_language(Language::English),
            Voice::new("Daniel")
                .with_gender(Gender::Male)
                .with_quality(VoiceQuality::Moderate)
                .with_language(Language::English),
        ];

        let config = TtsConfig::new().with_gender(Gender::Male);
        let best = SayProvider::select_best_voice(&voices, &config).unwrap();
        assert_eq!(best.name, "Alex (Premium)");
    }

    #[test]
    fn test_default_voice_selection_alphabetical_tiebreak() {
        let voices = vec![
            Voice::new("Zara")
                .with_gender(Gender::Female)
                .with_quality(VoiceQuality::Good)
                .with_language(Language::English),
            Voice::new("Amy")
                .with_gender(Gender::Female)
                .with_quality(VoiceQuality::Good)
                .with_language(Language::English),
            Voice::new("Kate")
                .with_gender(Gender::Female)
                .with_quality(VoiceQuality::Good)
                .with_language(Language::English),
        ];

        let config = TtsConfig::new().with_gender(Gender::Female);
        let best = SayProvider::select_best_voice(&voices, &config).unwrap();
        assert_eq!(best.name, "Amy");
    }

    #[test]
    fn test_default_voice_selection_gender_fallback() {
        let voices = vec![
            Voice::new("Samantha")
                .with_gender(Gender::Female)
                .with_quality(VoiceQuality::Good)
                .with_language(Language::English),
        ];

        let config = TtsConfig::new().with_gender(Gender::Male);
        let best = SayProvider::select_best_voice(&voices, &config).unwrap();
        assert_eq!(best.name, "Samantha");
    }

    // ========================================================================
    // Resolution and the shared cache, with pinned system defaults
    // ========================================================================

    use system_defaults::FakeSpeechDefaults;

    fn resolution(voice: &str, rate_wpm: u32) -> Resolution {
        Resolution {
            voice: Some(voice.into()),
            rate_wpm: Some(rate_wpm),
        }
    }

    #[tokio::test]
    async fn resolution_keeps_requested_and_gender_voices_and_reads_only_missing_defaults() {
        let cases = [
            (TtsConfig::new().with_voice("Victoria").with_speed(SpeedLevel::Fast), Some("Victoria"), Some(219), 0),
            (TtsConfig::new().with_gender(Gender::Male), Some("Alex"), Some(175), 1),
            (TtsConfig::new().with_gender(Gender::Female), Some("Samantha"), Some(175), 1),
            (TtsConfig::new(), Some("Ava (Premium)"), Some(175), 2),
            (TtsConfig::new().with_speed(SpeedLevel::Slow), Some("Ava (Premium)"), Some(131), 1),
        ];
        for (config, voice, rate_wpm, lookups) in cases {
            let defaults = FakeSpeechDefaults::new(Some("Ava (Premium)"), Some(175));
            let resolved = SayProvider::resolve(&config, &defaults).await;
            assert_eq!(resolved.voice.as_deref(), voice, "{config:?}");
            assert_eq!(resolved.rate_wpm, rate_wpm, "{config:?}");
            assert_eq!(defaults.lookups(), lookups, "{config:?}");
        }
    }

    #[tokio::test]
    async fn an_unresolved_system_setting_leaves_the_request_unresolved() {
        let no_rate = FakeSpeechDefaults::new(Some("Ava (Premium)"), None);
        let resolved = SayProvider::resolve(&TtsConfig::new(), &no_rate).await;
        assert_eq!(resolved, Resolution { voice: None, rate_wpm: None });
        assert_eq!(no_rate.lookups(), 1, "the voice is not looked up without a rate");
        let male = SayProvider::resolve(&TtsConfig::new().with_gender(Gender::Male), &no_rate).await;
        assert_eq!(male.concrete(), None);

        let no_voice = FakeSpeechDefaults::new(None, Some(175));
        let resolved = SayProvider::resolve(&TtsConfig::new().with_speed(SpeedLevel::Fast), &no_voice).await;
        assert_eq!(resolved, Resolution { voice: None, rate_wpm: Some(219) });
        assert_eq!(resolved.concrete(), None);
    }

    #[test]
    fn the_cache_key_keeps_its_existing_spelling_and_ignores_volume() {
        let text = "This is a test message.";
        assert_eq!(
            SayProvider::cache_key(text, "Alex", 219).cache_path(),
            CacheKey::new("say", "Alex;rate=219", text, "wav").cache_path()
        );
        assert_ne!(
            SayProvider::cache_key(text, "Alex", 219).cache_path(),
            SayProvider::cache_key(text, "Alex", 175).cache_path()
        );
        assert_ne!(
            SayProvider::cache_key(text, "Alex", 175).cache_path(),
            SayProvider::cache_key(text, "Samantha", 175).cache_path()
        );
    }

    /// A `say` on a private `PATH` that writes fixed bytes to its `-o` file
    /// and logs each argument list, with the OS temporary directory (and so
    /// the shared audio cache) inside the same fixture.
    #[cfg(unix)]
    struct FakeSay {
        root: tempfile::TempDir,
        _env: [test_toolkit::EnvGuard; 4],
    }

    #[cfg(unix)]
    impl FakeSay {
        fn install() -> Self {
            Self::writing("fake say audio")
        }

        /// A fake that writes `audio` (possibly nothing) as its output.
        fn writing(audio: &str) -> Self {
            use std::os::unix::fs::PermissionsExt;
            let root = tempfile::tempdir().unwrap();
            let bin = root.path().join("bin");
            let tmp = root.path().join("tmp");
            std::fs::create_dir(&bin).unwrap();
            std::fs::create_dir(&tmp).unwrap();
            let log = root.path().join("say.log");
            let script = format!(
                "#!/bin/sh\necho \"$*\" >> '{}'\n/bin/cat > /dev/null\n\
                 while [ \"$#\" -gt 0 ]; do if [ \"$1\" = -o ]; then printf '{audio}' > \"$2\"; fi; shift; done\n",
                log.display()
            );
            std::fs::write(bin.join("say"), script).unwrap();
            std::fs::set_permissions(bin.join("say"), std::fs::Permissions::from_mode(0o755)).unwrap();
            let env = [
                test_toolkit::EnvGuard::set_safe("PATH", &bin),
                test_toolkit::EnvGuard::set_safe("TMPDIR", &tmp),
                test_toolkit::EnvGuard::remove_safe("PLAYA_DRY_RUN"),
                test_toolkit::EnvGuard::remove_safe("TTS_PROVIDER"),
            ];
            Self { root, _env: env }
        }

        fn syntheses(&self) -> Vec<String> {
            std::fs::read_to_string(self.root.path().join("say.log"))
                .unwrap_or_default()
                .lines()
                .map(str::to_owned)
                .collect()
        }
    }

    /// With the system voice and rate both resolved, normal speed prepares
    /// once, retains the resolved pair, and a fresh lookup finds the file. A
    /// changed default makes the fresh lookup miss while the retained pair
    /// still names the prepared file.
    #[cfg(unix)]
    #[tokio::test]
    #[serial_test::serial]
    async fn resolved_system_defaults_prepare_once_and_a_changed_default_misses() {
        let say = FakeSay::install();
        let defaults = FakeSpeechDefaults::new(Some("Ava (Premium)"), Some(175));
        let config = TtsConfig::new();
        let text = "This is a test message: pinned defaults";

        let mut prepared = None;
        for _ in 0..2 {
            prepared = SayProvider::prepare_with(text, &config, &defaults).await.unwrap();
        }
        let expected = ResolvedSynthesis::Say { voice: "Ava (Premium)".into(), rate_wpm: 175 };
        assert_eq!(prepared, Some(expected));
        let syntheses = say.syntheses();
        assert_eq!(syntheses.len(), 1, "{syntheses:?}");
        assert!(syntheses[0].contains("-v Ava (Premium) -r 175"), "{syntheses:?}");

        let fresh = SayProvider::resolve(&config, &defaults).await;
        let path = SayProvider::cached_path(text, &fresh).expect("a fresh object finds the file");
        assert_eq!(std::fs::read(&path).unwrap(), b"fake say audio");

        defaults.set(Some("Samantha"), Some(175));
        let changed = SayProvider::resolve(&config, &defaults).await;
        assert_eq!(SayProvider::cached_path(text, &changed), None);
        assert_eq!(SayProvider::cached_path(text, &resolution("Ava (Premium)", 175)), Some(path));
        defaults.set(Some("Ava (Premium)"), Some(200));
        let faster = SayProvider::resolve(&config, &defaults).await;
        assert_eq!(SayProvider::cached_path(text, &faster), None, "the rate is part of the key");
    }

    #[cfg(unix)]
    #[tokio::test]
    #[serial_test::serial]
    async fn unresolved_system_defaults_prepare_nothing() {
        let say = FakeSay::install();
        for defaults in [
            FakeSpeechDefaults::new(Some("Ava (Premium)"), None),
            FakeSpeechDefaults::new(None, Some(175)),
            FakeSpeechDefaults::new(None, None),
        ] {
            let prepared = SayProvider::prepare_with("This is a test message.", &TtsConfig::new(), &defaults).await;
            assert_eq!(prepared.unwrap(), None);
        }
        assert!(say.syntheses().is_empty());
        assert_eq!(std::fs::read_dir(std::env::temp_dir()).unwrap().count(), 0);
    }

    #[cfg(unix)]
    #[tokio::test]
    #[serial_test::serial]
    async fn empty_output_is_an_error_and_is_never_published() {
        let say = FakeSay::writing("");
        let config = TtsConfig::new().with_voice("Samantha").with_speed(SpeedLevel::Fast);
        let defaults = FakeSpeechDefaults::default();
        let result = SayProvider::prepare_with("This is a test message.", &config, &defaults).await;
        assert!(
            matches!(&result, Err(TtsError::ProcessFailed { stderr, .. }) if stderr == "say produced no audio"),
            "{result:?}"
        );
        assert_eq!(say.syntheses().len(), 1);
        assert_eq!(std::fs::read_dir(std::env::temp_dir()).unwrap().count(), 0);
    }

    #[cfg(unix)]
    #[tokio::test]
    #[serial_test::serial]
    async fn dry_run_prepares_nothing_before_any_lookup() {
        let say = FakeSay::install();
        let _dry_run = test_toolkit::EnvGuard::set_safe("PLAYA_DRY_RUN", "true");
        let defaults = FakeSpeechDefaults::new(Some("Ava (Premium)"), Some(175));
        let prepared = SayProvider::prepare_with("This is a test message.", &TtsConfig::new(), &defaults).await;
        assert_eq!(prepared.unwrap(), None);
        assert_eq!(defaults.lookups(), 0);
        assert!(say.syntheses().is_empty());
    }

    // ========================================================================
    // Integration tests - macOS only
    // ========================================================================

    #[cfg(target_os = "macos")]
    #[tokio::test]
    async fn real_say_provider_speaks_muted() {
        let config = TtsConfig::default().with_volume(crate::types::VolumeLevel::Explicit(0.0));
        let resolution = SayProvider::resolve(&config, &OsSpeechDefaults).await;
        let audio = SayProvider::synthesize("This is a test message.", &resolution).await.unwrap();
        let report = SayProvider::play_report(audio.path(), &config).await.unwrap();
        #[cfg(feature = "playa")]
        {
            assert_eq!(report.route, crate::types::SpeakPlaybackRoute::Native);
            assert_eq!(report.verdict, crate::types::SpeakPlaybackVerdict::Complete);
        }
        #[cfg(not(feature = "playa"))]
        assert_eq!(report.route, crate::types::SpeakPlaybackRoute::Host("afplay".into()));
    }

    #[cfg(target_os = "macos")]
    #[tokio::test]
    async fn test_is_ready_on_macos() {
        let provider = SayProvider;
        // On macOS, say should always be ready
        assert!(provider.is_ready().await);
    }

    #[cfg(target_os = "macos")]
    #[tokio::test]
    async fn test_list_voices_returns_voices() {
        let provider = SayProvider;
        let voices = provider.list_voices().await.unwrap();

        // Should have at least some voices
        assert!(!voices.is_empty(), "Expected at least one voice");

        // Check that we have expected common voices
        let voice_names: Vec<&str> = voices.iter().map(|v| v.name.as_str()).collect();

        // These voices are typically available on macOS
        let expected_voices = ["Samantha", "Alex", "Victoria"];
        for expected in expected_voices {
            let found = voice_names.iter().any(|name| name.contains(expected));
            // Note: Not all voices may be installed, so we just check a few
            if found {
                println!("Found expected voice: {}", expected);
            }
        }

        // Verify voice properties are populated
        for voice in &voices {
            assert!(!voice.name.is_empty(), "Voice name should not be empty");
            assert!(
                !voice.languages.is_empty(),
                "Voice should have at least one language"
            );
        }
    }

    #[cfg(target_os = "macos")]
    #[tokio::test]
    async fn test_list_voices_no_eloquence() {
        let provider = SayProvider;
        let voices = provider.list_voices().await.unwrap();

        // Verify no Eloquence voices are included
        for voice in &voices {
            assert!(
                !voice.name.contains("Eloquence"),
                "Eloquence voice should be filtered: {}",
                voice.name
            );
        }
    }

    #[cfg(target_os = "macos")]
    #[tokio::test]
    async fn test_enhanced_voices_have_good_quality() {
        let provider = SayProvider;
        let voices = provider.list_voices().await.unwrap();

        for voice in &voices {
            if voice.name.contains("(Enhanced)") || voice.name.contains("(Premium)") {
                assert_eq!(
                    voice.quality,
                    VoiceQuality::Good,
                    "Enhanced/Premium voice {} should have Good quality",
                    voice.name
                );
            }
        }
    }

    #[cfg(target_os = "macos")]
    #[tokio::test]
    async fn test_default_voice_integration_male() {
        let provider = SayProvider;
        let voice = provider.default_voice(Gender::Male).await.unwrap();
        assert_eq!(voice.gender, Gender::Male);
        assert!(!voice.name.is_empty());
    }

    #[cfg(target_os = "macos")]
    #[tokio::test]
    async fn test_default_voice_integration_female() {
        let provider = SayProvider;
        let voice = provider.default_voice(Gender::Female).await.unwrap();
        assert_eq!(voice.gender, Gender::Female);
        assert!(!voice.name.is_empty());
    }

    #[cfg(target_os = "macos")]
    #[tokio::test]
    async fn test_default_voice_integration_any() {
        let provider = SayProvider;
        let voice = provider.default_voice(Gender::Any).await.unwrap();
        assert!(!voice.name.is_empty());
    }
}
