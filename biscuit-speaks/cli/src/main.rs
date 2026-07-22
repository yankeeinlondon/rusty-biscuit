use std::fmt;
use std::io::{self, IsTerminal, Read};

use biscuit_speaks::{
    CloudTtsProvider, ESpeakProvider, EchogardenProvider, ElevenLabsProvider, Gender, GttsProvider,
    HostTtsProvider, KokoroTtsProvider, Language, SapiProvider, SayProvider, SpeakResult,
    SpeedLevel, TtsConfig, TtsError, TtsFailoverStrategy, TtsProvider, TtsVoiceInventory, Voice,
    VoiceQuality, VolumeLevel, bust_host_capability_cache, get_available_providers,
    parse_provider_name, populate_cache_for_all_providers, read_from_cache, speak,
    speak_with_result,
};
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::CompleteEnv;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::output::terminal::TerminalOptions;
use inquire::{Confirm, Select};
use owo_colors::OwoColorize;
use sniff::programs::{
    HostCapabilities, InstallInterviewInput, InstallInterviewOptions, InstallInterviewOutcome,
    InstallOptions, InstalledTtsClients, ProgramDetector, TtsClient, build_install_plan,
    run_install_interview,
};
use strum::IntoEnumIterator;

mod install_ui;
use install_ui::SoYouSayInstallUi;

const CLI_BINARY_NAME: &str = "so-you-say";

const AFTER_HELP: &str = "\
\x1b[1m\x1b[4mShell Completions\x1b[0m
  Enable tab completions by adding one of the following to your shell config:

  # Bash (~/.bashrc)
  source <(COMPLETE=bash so-you-say)

  # Zsh (~/.zshrc)
  source <(COMPLETE=zsh so-you-say)

  # Fish (~/.config/fish/config.fish)
  COMPLETE=fish so-you-say | source";

/// Gender preference for voice selection
#[derive(Debug, Clone, Copy, ValueEnum)]
enum GenderArg {
    /// Use a male voice
    Male,
    /// Use a female voice
    Female,
}

impl From<GenderArg> for Gender {
    fn from(arg: GenderArg) -> Self {
        match arg {
            GenderArg::Male => Gender::Male,
            GenderArg::Female => Gender::Female,
        }
    }
}

#[derive(Parser)]
#[command(name = CLI_BINARY_NAME)]
#[command(about = "Convert text to speech using system TTS", long_about = None)]
#[command(version)]
#[command(after_help = AFTER_HELP)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Use a specific TTS provider
    #[arg(long)]
    provider: Option<String>,

    /// Use a specific voice by name
    #[arg(long)]
    voice: Option<String>,

    /// Voice gender preference (male or female)
    #[arg(short, long, value_enum)]
    gender: Option<GenderArg>,

    #[arg(
        short,
        long,
        help = "Language code for voice selection (e.g., \"en\", \"fr\", \"de\")",
        long_help = "Language code for voice selection (e.g., \"en\", \"fr\", \"de\"). When used with list-voices, filters to only show voices for this language. When speaking, sets the preferred language for voice selection."
    )]
    lang: Option<String>,

    /// Display metadata about the voice used after speaking
    #[arg(long)]
    meta: bool,

    /// Increase volume to maximum (loud)
    #[arg(long, conflicts_with = "soft")]
    loud: bool,

    /// Decrease volume to softer level
    #[arg(long, conflicts_with = "loud")]
    soft: bool,

    /// Increase speech rate (faster)
    #[arg(long, conflicts_with = "slow")]
    fast: bool,

    /// Decrease speech rate (slower)
    #[arg(long, conflicts_with = "fast")]
    slow: bool,

    #[arg(
        long,
        help = "Refresh the TTS provider and voice cache",
        long_help = "Refresh the TTS provider and voice cache. Clears the cached voice data (~/.biscuit-speaks-cache.json) and repopulates it with fresh data from all available providers. Use this after installing new voices or if voice listings appear stale."
    )]
    refresh_cache: bool,

    /// Speak in the background, returning control to the terminal immediately
    #[arg(long)]
    background: bool,

    /// Text to speak (reads from stdin if not provided)
    text: Vec<String>,
}

#[derive(Subcommand)]
enum Command {
    /// List available TTS providers
    ListProviders,

    /// List available voices for a provider
    ListVoices {
        /// Show voices for a specific provider
        #[arg(long)]
        provider: Option<String>,

        /// Filter voices by language code (e.g., "en", "fr", "de")
        #[arg(short, long)]
        lang: Option<String>,
    },

    /// Check TTS provider setup and recommend improvements
    Init,

    /// Install a TTS provider
    Install {
        /// Provider to install (interactive selection if omitted)
        pkg: Option<String>,
        /// Show the install command without executing it
        #[arg(long)]
        dry_run: bool,
    },
}

/// Joins multiple arguments into a single string with spaces
fn join_args(args: Vec<String>) -> String {
    args.join(" ")
}

/// Reads text from stdin with a 10,000 character limit
fn read_from_stdin() -> io::Result<String> {
    let mut buffer = String::new();
    let mut handle = io::stdin().take(10_000);
    handle.read_to_string(&mut buffer)?;
    let text = buffer.trim().to_string();

    if text.is_empty() {
        eprintln!("Error: No input provided");
        eprintln!(
            "Usage: {} <text> or echo \"text\" | {}",
            CLI_BINARY_NAME, CLI_BINARY_NAME
        );
        std::process::exit(1);
    }

    Ok(text)
}

fn provider_display_name(provider: &TtsProvider) -> &'static str {
    match provider {
        TtsProvider::Host(h) => match h {
            HostTtsProvider::Say => "say (macOS)",
            HostTtsProvider::ESpeak => "espeak (eSpeak-NG)",
            HostTtsProvider::Piper => "piper (Piper TTS)",
            HostTtsProvider::EchoGarden => "echogarden",
            HostTtsProvider::Sherpa => "sherpa (Sherpa-ONNX)",
            HostTtsProvider::Mimic3 => "mimic3 (Mycroft)",
            HostTtsProvider::Festival => "festival",
            HostTtsProvider::Gtts => "gtts (Google TTS CLI)",
            HostTtsProvider::Sapi => "sapi (Windows)",
            HostTtsProvider::KokoroTts => "kokoro (Kokoro TTS)",
            HostTtsProvider::Pico2Wave => "pico2wave",
            HostTtsProvider::SpdSay => "spd-say (Speech Dispatcher)",
            _ => "unknown host provider",
        },
        TtsProvider::Cloud(c) => match c {
            CloudTtsProvider::ElevenLabs => "elevenlabs (ElevenLabs API)",
            _ => "unknown cloud provider",
        },
        _ => "unknown provider",
    }
}

fn print_speak_result(result: &SpeakResult, volume: VolumeLevel, speed: SpeedLevel) {
    println!();
    println!(
        "  {}: {}",
        "Provider".dimmed(),
        provider_display_name(&result.provider)
    );
    println!("  {}: {}", "Voice".dimmed(), result.voice.name);
    // Show Voice ID if available (useful for ElevenLabs)
    if let Some(ref id) = result.voice.identifier {
        println!("  {}: {}", "Voice ID".dimmed(), id);
    }
    println!(
        "  {}: {}",
        "Gender".dimmed(),
        voice_gender_label(result.voice.gender)
    );
    println!(
        "  {}: {}",
        "Quality".dimmed(),
        voice_quality_label(result.voice.quality)
    );
    println!("  {}: {}", "Volume".dimmed(), volume_label(volume));
    println!("  {}: {}", "Speed".dimmed(), speed_label(speed));
    // Show Model Used if available (useful for ElevenLabs)
    if let Some(ref model) = result.model_used {
        println!("  {}: {}", "Model".dimmed(), model);
    }
    // Show Audio File path if available
    if let Some(ref path) = result.audio_file_path {
        println!("  {}: {}", "Audio File".dimmed(), path.display());
    }
    // Show Audio Codec if available
    if let Some(ref codec) = result.audio_codec {
        println!("  {}: {}", "Codec".dimmed(), codec);
    }
    // Show Cache status
    println!(
        "  {}: {}",
        "Cache".dimmed(),
        if result.cache_hit { "hit" } else { "miss" }
    );
}

fn provider_supports_voice_listing(provider: &TtsProvider) -> bool {
    matches!(
        provider,
        TtsProvider::Host(
            HostTtsProvider::Say
                | HostTtsProvider::ESpeak
                | HostTtsProvider::Gtts
                | HostTtsProvider::EchoGarden
                | HostTtsProvider::KokoroTts
                | HostTtsProvider::Sapi
        ) | TtsProvider::Cloud(CloudTtsProvider::ElevenLabs)
    )
}

fn print_providers() {
    let providers = get_available_providers();

    if providers.is_empty() {
        println!("No TTS providers available on this system.");
        return;
    }

    println!("Available TTS providers:");
    println!();

    for provider in providers {
        let name = provider_display_name(provider);
        if matches!(provider, TtsProvider::Cloud(_)) {
            println!("  - {} [cloud]", name);
        } else {
            println!("  - {}", name);
        }
    }
}

#[derive(Clone)]
struct ProviderOption {
    provider: TtsProvider,
    label: String,
}

impl fmt::Display for ProviderOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.label)
    }
}

fn listable_providers() -> Vec<TtsProvider> {
    get_available_providers()
        .iter()
        .copied()
        .filter(provider_supports_voice_listing)
        .collect()
}

fn prompt_for_provider(providers: &[TtsProvider]) -> Result<TtsProvider, inquire::InquireError> {
    let options: Vec<ProviderOption> = providers
        .iter()
        .map(|provider| ProviderOption {
            provider: *provider,
            label: provider_display_name(provider).to_string(),
        })
        .collect();
    let selection = Select::new("Select a TTS provider", options).prompt()?;
    Ok(selection.provider)
}

/// List voices for a provider, using the cache if available.
///
/// This function first checks the cache for voice data. If found, it returns
/// the cached voices. Otherwise, it falls back to querying the provider directly.
/// Use `--refresh-cache` to update the cache with fresh data from providers.
async fn list_voices_for_provider(provider: TtsProvider) -> Result<Vec<Voice>, TtsError> {
    // Try to read from cache first
    if let Ok(cache) = read_from_cache()
        && let Some(capability) = cache.get_provider(&provider)
        && !capability.voices.is_empty()
    {
        return Ok(capability.voices.clone());
    }

    // Cache miss - query the provider directly
    match provider {
        TtsProvider::Host(HostTtsProvider::Say) => {
            let provider = SayProvider;
            provider.list_voices().await
        }
        TtsProvider::Host(HostTtsProvider::ESpeak) => {
            let provider = ESpeakProvider::new();
            provider.list_voices().await
        }
        TtsProvider::Host(HostTtsProvider::Gtts) => {
            let provider = GttsProvider::new();
            provider.list_voices().await
        }
        TtsProvider::Host(HostTtsProvider::EchoGarden) => {
            let provider = EchogardenProvider::new();
            provider.list_voices().await
        }
        TtsProvider::Host(HostTtsProvider::KokoroTts) => {
            let provider = KokoroTtsProvider::new();
            provider.list_voices().await
        }
        TtsProvider::Host(HostTtsProvider::Sapi) => {
            let provider = SapiProvider::new();
            provider.list_voices().await
        }
        TtsProvider::Cloud(CloudTtsProvider::ElevenLabs) => {
            let provider = ElevenLabsProvider::new()?;
            provider.list_voices().await
        }
        _ => Err(TtsError::VoiceEnumerationFailed {
            provider: provider_display_name(&provider).to_string(),
            message: "Voice listing is not supported for this provider".to_string(),
        }),
    }
}

fn voice_quality_rank(quality: VoiceQuality) -> u8 {
    match quality {
        VoiceQuality::Excellent => 0,
        VoiceQuality::Good => 1,
        VoiceQuality::Moderate => 2,
        VoiceQuality::Low => 3,
        VoiceQuality::Unknown => 4,
    }
}

fn voice_gender_label(gender: Gender) -> &'static str {
    match gender {
        Gender::Male => "Male",
        Gender::Female => "Female",
        Gender::Any => "-",
        _ => "?",
    }
}

fn voice_quality_label(quality: VoiceQuality) -> &'static str {
    match quality {
        VoiceQuality::Excellent => "Excellent",
        VoiceQuality::Good => "Good",
        VoiceQuality::Moderate => "Moderate",
        VoiceQuality::Low => "Low",
        VoiceQuality::Unknown => "Unknown",
    }
}

fn volume_label(volume: VolumeLevel) -> &'static str {
    match volume {
        VolumeLevel::Loud => "loud",
        VolumeLevel::Soft => "soft",
        VolumeLevel::Normal => "normal",
        VolumeLevel::Explicit(_) => "custom",
    }
}

fn speed_label(speed: SpeedLevel) -> &'static str {
    match speed {
        SpeedLevel::Fast => "fast",
        SpeedLevel::Slow => "slow",
        SpeedLevel::Normal => "normal",
        SpeedLevel::Explicit(_) => "custom",
    }
}

/// Quality tier extracted from voice name suffixes.
///
/// Higher tier = better quality. Used for deduplication.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum QualityTier {
    /// No quality suffix or unknown
    Base = 0,
    /// Compact/low quality variant
    Compact = 1,
    /// Enhanced quality variant
    Enhanced = 2,
    /// Premium quality variant (highest)
    Premium = 3,
}

/// Extract quality tier from a voice name suffix.
///
/// macOS voices use suffixes like "(Premium)", "(Enhanced)", "(Compact)".
fn extract_quality_tier(name: &str) -> QualityTier {
    let name_lower = name.to_lowercase();
    if name_lower.contains("(premium)") {
        QualityTier::Premium
    } else if name_lower.contains("(enhanced)") {
        QualityTier::Enhanced
    } else if name_lower.contains("(compact)") {
        QualityTier::Compact
    } else {
        QualityTier::Base
    }
}

/// Map quality tier to VoiceQuality.
fn quality_tier_to_voice_quality(tier: QualityTier) -> VoiceQuality {
    match tier {
        QualityTier::Premium => VoiceQuality::Excellent,
        QualityTier::Enhanced => VoiceQuality::Good,
        QualityTier::Compact => VoiceQuality::Low,
        QualityTier::Base => VoiceQuality::Moderate,
    }
}

/// Extract a clean display name from a voice name.
///
/// Handles multiple patterns:
/// - VITS-style: `de_DE-thorsten-high` → `Thorsten`
/// - macOS quality: `Ava (Premium)` → `Ava`
/// - macOS language: `Eddy (English (UK))` → `Eddy`
/// - Simple names: `Heart` → `Heart`
fn extract_display_name(name: &str) -> String {
    // First, strip macOS quality suffixes like "(Premium)", "(Enhanced)", "(Compact)"
    let name_without_quality_suffix = name
        .trim_end_matches(" (Premium)")
        .trim_end_matches(" (Enhanced)")
        .trim_end_matches(" (Compact)")
        .trim_end_matches(" (premium)")
        .trim_end_matches(" (enhanced)")
        .trim_end_matches(" (compact)");

    // Strip language/region suffixes like "(English (UK))", "(Chinese (China mainland))"
    // These are parenthetical suffixes that aren't quality indicators
    let name_without_lang_suffix = strip_parenthetical_suffix(name_without_quality_suffix);

    // Check for VITS-style pattern: locale-name-quality (e.g., de_DE-thorsten-high)
    // The pattern is: {locale}_{REGION}-{name}-{quality}
    // We want to extract the name part and title-case it

    // Strip common VITS quality suffixes
    let name_without_vits_quality = name_without_lang_suffix
        .strip_suffix("-high")
        .or_else(|| name_without_lang_suffix.strip_suffix("-medium"))
        .or_else(|| name_without_lang_suffix.strip_suffix("-low"))
        .or_else(|| name_without_lang_suffix.strip_suffix("-x_low"))
        .unwrap_or(&name_without_lang_suffix);

    // Check if it matches the locale pattern (e.g., de_DE-thorsten, en_US-lessac)
    if let Some(dash_pos) = name_without_vits_quality.find('-') {
        let potential_locale = &name_without_vits_quality[..dash_pos];
        // Check if it looks like a locale (e.g., de_DE, en_US, ar_JO)
        if potential_locale.len() >= 2
            && potential_locale.contains('_')
            && potential_locale
                .chars()
                .all(|c| c.is_ascii_alphabetic() || c == '_')
        {
            // Extract the name part after the locale
            let name_part = &name_without_vits_quality[dash_pos + 1..];
            // Title-case the name
            return title_case(name_part);
        }
    }

    // For simple names (possibly with suffix stripped), return as-is
    name_without_vits_quality.to_string()
}

/// Strip a trailing parenthetical suffix from a name.
///
/// Handles patterns like:
/// - "Eddy (English (UK))" → "Eddy"
/// - "Eddy (Chinese (China mainland))" → "Eddy"
/// - "Aman (English (India))" → "Aman"
///
/// Correctly handles nested parentheses by finding the outermost balanced group.
fn strip_parenthetical_suffix(name: &str) -> String {
    // Must end with ')' to have a parenthetical suffix
    if !name.ends_with(')') {
        return name.to_string();
    }

    // Find all " (" positions and check which one gives a balanced suffix
    let mut pos = 0;
    while let Some(relative_pos) = name[pos..].find(" (") {
        let space_paren_pos = pos + relative_pos;
        let suffix = &name[space_paren_pos + 1..]; // Include the space

        // Count parens in the suffix
        let open_count = suffix.chars().filter(|&c| c == '(').count();
        let close_count = suffix.chars().filter(|&c| c == ')').count();

        if open_count == close_count && open_count > 0 {
            // Found a balanced suffix, strip it
            return name[..space_paren_pos].to_string();
        }

        // Move past this position and continue searching
        pos = space_paren_pos + 2;
    }

    name.to_string()
}

/// Title-case a string (first letter uppercase, rest lowercase).
fn title_case(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first
            .to_uppercase()
            .chain(chars.map(|c| c.to_ascii_lowercase()))
            .collect(),
    }
}

fn voice_language_label(languages: &[Language]) -> String {
    if languages.is_empty() {
        return "-".to_string();
    }

    languages
        .iter()
        .map(|language| match language {
            Language::English => "en".to_string(),
            Language::Custom(code) => code.clone(),
            _ => "?".to_string(),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Check if a voice matches a given language code.
///
/// Matching rules:
/// - "en" matches Language::English and any "en-*" custom codes
/// - Other codes match if the voice language starts with that code
fn voice_matches_language(voice: &Voice, lang_code: &str) -> bool {
    let lang_lower = lang_code.to_lowercase();

    voice.languages.iter().any(|lang| match lang {
        Language::English => lang_lower == "en" || lang_lower.starts_with("en-"),
        Language::Custom(code) => {
            let code_lower = code.to_lowercase();
            // Exact match or prefix match (e.g., "fr" matches "fr-CA")
            code_lower == lang_lower || code_lower.starts_with(&format!("{}-", lang_lower))
        }
        _ => false,
    })
}

/// Filter voices by language code.
fn filter_voices_by_language(voices: Vec<Voice>, lang_code: &str) -> Vec<Voice> {
    voices
        .into_iter()
        .filter(|v| voice_matches_language(v, lang_code))
        .collect()
}

/// Deduplicate voices by display name + language, keeping only the highest quality.
///
/// When multiple voices exist for the same name/language (e.g., "Ava (Enhanced)" and
/// "Ava (Premium)"), this keeps only the highest quality version.
fn deduplicate_voices(voices: Vec<Voice>) -> Vec<Voice> {
    use std::collections::HashMap;

    // Group by (display_name_lowercase, language_label)
    let mut best_voices: HashMap<(String, String), (Voice, QualityTier)> = HashMap::new();

    for voice in voices {
        let display_name = extract_display_name(&voice.name).to_lowercase();
        let language = voice_language_label(&voice.languages);
        let key = (display_name, language);

        // Determine quality tier from the name suffix
        let tier = extract_quality_tier(&voice.name);

        // Keep the voice with the highest quality tier
        match best_voices.get(&key) {
            Some((_, existing_tier)) if tier <= *existing_tier => {
                // Existing voice is same or better quality, keep it
            }
            _ => {
                // New voice is better or no existing voice
                best_voices.insert(key, (voice, tier));
            }
        }
    }

    best_voices.into_values().map(|(voice, _)| voice).collect()
}

/// Escape pipe characters in markdown table cells.
fn escape_markdown_cell(value: &str) -> String {
    value.replace('|', "\\|")
}

/// Result of voice resolution.
#[derive(Debug)]
enum VoiceResolution {
    /// Found exactly one matching voice.
    Found(Voice),
    /// No matching voice found.
    NotFound,
    /// Multiple voices match - need language to disambiguate.
    Ambiguous(Vec<Voice>),
}

/// Resolve a user-provided voice name to the actual voice.
///
/// Performs case-insensitive matching on:
/// 1. The display name (e.g., "Heart", "Thorsten")
/// 2. The original voice name (e.g., "de_DE-thorsten-high")
///
/// If multiple voices match with the same display name and language (e.g., "Ava (Premium)"
/// and "Ava (Enhanced)"), returns the highest quality version.
///
/// If multiple voices match with different languages, uses the language filter to disambiguate.
fn resolve_voice_name(
    voices: &[Voice],
    user_voice: &str,
    lang_filter: Option<&str>,
) -> VoiceResolution {
    let user_lower = user_voice.to_lowercase();

    // Find all voices matching by display name OR original name (case-insensitive)
    let matches: Vec<Voice> = voices
        .iter()
        .filter(|v| {
            extract_display_name(&v.name).to_lowercase() == user_lower
                || v.name.to_lowercase() == user_lower
        })
        .cloned()
        .collect();

    if matches.is_empty() {
        return VoiceResolution::NotFound;
    }

    // Deduplicate by display name + language, keeping highest quality
    // This handles cases like "Ava (Premium)" and "Ava (Enhanced)" -> keep Premium
    let matches = deduplicate_voices(matches);

    if matches.len() == 1 {
        return VoiceResolution::Found(matches.into_iter().next().unwrap());
    }

    // Multiple matches with different languages - try to filter by language
    if let Some(lang) = lang_filter {
        let lang_matches = filter_voices_by_language(matches.clone(), lang);
        if lang_matches.len() == 1 {
            return VoiceResolution::Found(lang_matches.into_iter().next().unwrap());
        }
        if !lang_matches.is_empty() {
            // Still ambiguous even with language filter
            return VoiceResolution::Ambiguous(lang_matches);
        }
    }

    VoiceResolution::Ambiguous(matches)
}

/// Get the actual voice name to pass to the TTS engine.
///
/// For most providers, this is the voice name. For some (like echogarden),
/// the identifier without the engine prefix is what the engine expects.
fn get_engine_voice_name(voice: &Voice) -> String {
    // If there's an identifier like "kokoro:Heart", extract "Heart"
    // If there's an identifier like "vits:de_DE-thorsten-high", extract "de_DE-thorsten-high"
    if let Some(ref identifier) = voice.identifier {
        if let Some(colon_pos) = identifier.find(':') {
            return identifier[colon_pos + 1..].to_string();
        }
        return identifier.clone();
    }
    voice.name.clone()
}

/// Get the effective quality for a voice, preferring inferred quality from name suffix.
///
/// If the voice name has a quality suffix like "(Premium)" or "(Enhanced)", use that.
/// Otherwise, fall back to the provider-reported quality.
fn effective_voice_quality(voice: &Voice) -> VoiceQuality {
    let tier = extract_quality_tier(&voice.name);
    if tier != QualityTier::Base {
        // Name has a quality suffix, use inferred quality
        quality_tier_to_voice_quality(tier)
    } else {
        // No suffix, use provider-reported quality
        voice.quality
    }
}

fn print_voices(provider: TtsProvider, voices: &[Voice], lang_filter: Option<&str>) {
    // Apply language filter if specified
    let voices: Vec<Voice> = if let Some(lang) = lang_filter {
        filter_voices_by_language(voices.to_vec(), lang)
    } else {
        voices.to_vec()
    };

    // Deduplicate voices by display name + language, keeping highest quality
    let voices = deduplicate_voices(voices);

    if voices.is_empty() {
        if let Some(lang) = lang_filter {
            println!(
                "No voices found for {} with language '{}'.",
                provider_display_name(&provider),
                lang
            );
        } else {
            println!("No voices found for {}.", provider_display_name(&provider));
        }
        return;
    }

    if let Some(lang) = lang_filter {
        println!(
            "Found {} voices for {} (language: {}):",
            voices.len(),
            provider_display_name(&provider),
            lang
        );
    } else {
        println!(
            "Found {} voices for {}:",
            voices.len(),
            provider_display_name(&provider)
        );
    }
    println!();

    let mut voices = voices;
    voices.sort_by(|a, b| {
        voice_quality_rank(effective_voice_quality(a))
            .cmp(&voice_quality_rank(effective_voice_quality(b)))
            .then_with(|| {
                extract_display_name(&a.name)
                    .to_lowercase()
                    .cmp(&extract_display_name(&b.name).to_lowercase())
            })
    });

    // Build markdown table
    let mut lines = Vec::new();
    lines.push("| Voice | Description | Language | Quality | Gender |".to_string());
    lines.push("| --- | --- | --- | --- | --- |".to_string());

    for voice in voices {
        let display_name = escape_markdown_cell(&extract_display_name(&voice.name));
        let description = voice
            .description
            .as_deref()
            .map(escape_markdown_cell)
            .unwrap_or_default();
        let language = escape_markdown_cell(&voice_language_label(&voice.languages));
        let quality = voice_quality_label(effective_voice_quality(&voice));
        let gender = voice_gender_label(voice.gender);

        lines.push(format!(
            "| {} | {} | {} | {} | {} |",
            display_name, description, language, quality, gender
        ));
    }

    let markdown = Markdown::from(lines.join("\n"));
    match markdown.as_terminal(TerminalOptions::default()) {
        Ok(rendered) => print!("{}", rendered),
        Err(_) => println!("{}", markdown.content()),
    }

    println!(
        "{}",
        "- Use --voice <name> to select a voice (case-insensitive)."
            .dimmed()
            .italic()
    );
}

// ============================================================================
// HostTtsProvider <-> TtsClient mapping
// ============================================================================

/// Map a `HostTtsProvider` to its corresponding `TtsClient` for sniff operations.
#[cfg(test)]
fn host_provider_to_tts_client(provider: HostTtsProvider) -> Option<TtsClient> {
    match provider {
        HostTtsProvider::Say => Some(TtsClient::Say),
        HostTtsProvider::ESpeak => Some(TtsClient::EspeakNg),
        HostTtsProvider::Piper => Some(TtsClient::Piper),
        HostTtsProvider::EchoGarden => Some(TtsClient::Echogarden),
        HostTtsProvider::Sherpa => Some(TtsClient::SherpaOnnx),
        HostTtsProvider::Mimic3 => Some(TtsClient::Mimic3),
        HostTtsProvider::Festival => Some(TtsClient::Festival),
        HostTtsProvider::Gtts => Some(TtsClient::GttsCli),
        HostTtsProvider::Sapi => Some(TtsClient::WindowsSapi),
        HostTtsProvider::KokoroTts => Some(TtsClient::KokoroTts),
        HostTtsProvider::Pico2Wave => Some(TtsClient::Pico2Wave),
        HostTtsProvider::SpdSay => None, // Not tracked by sniff
        _ => None,
    }
}

/// Map a `TtsClient` back to its `HostTtsProvider`.
#[cfg(test)]
fn tts_client_to_host_provider(client: TtsClient) -> Option<HostTtsProvider> {
    match client {
        TtsClient::Say => Some(HostTtsProvider::Say),
        TtsClient::Espeak | TtsClient::EspeakNg => Some(HostTtsProvider::ESpeak),
        TtsClient::Piper => Some(HostTtsProvider::Piper),
        TtsClient::Echogarden => Some(HostTtsProvider::EchoGarden),
        TtsClient::SherpaOnnx => Some(HostTtsProvider::Sherpa),
        TtsClient::Mimic3 => Some(HostTtsProvider::Mimic3),
        TtsClient::Festival => Some(HostTtsProvider::Festival),
        TtsClient::GttsCli => Some(HostTtsProvider::Gtts),
        TtsClient::WindowsSapi => Some(HostTtsProvider::Sapi),
        TtsClient::KokoroTts => Some(HostTtsProvider::KokoroTts),
        TtsClient::Pico2Wave => Some(HostTtsProvider::Pico2Wave),
        _ => None,
    }
}

/// Collapse TTS clients that share an underlying binary.
///
/// On Linux the `espeak-ng` package ships both `espeak` and `espeak-ng`
/// binaries that run the same code, so sniff reports two detections for one
/// installation. This helper prefers `EspeakNg` and drops `Espeak` when both
/// are present.
fn dedupe_tts_client_aliases(mut clients: Vec<TtsClient>) -> Vec<TtsClient> {
    let has_espeak_ng = clients.contains(&TtsClient::EspeakNg);
    if has_espeak_ng {
        clients.retain(|c| *c != TtsClient::Espeak);
    }
    clients
}

/// Display name for a TtsClient (for install UI).
fn tts_client_display_name(client: TtsClient) -> &'static str {
    match client {
        TtsClient::Say => "say (macOS)",
        TtsClient::Espeak | TtsClient::EspeakNg => "espeak-ng",
        TtsClient::Piper => "piper (Piper TTS)",
        TtsClient::Echogarden => "echogarden",
        TtsClient::SherpaOnnx => "sherpa (Sherpa-ONNX)",
        TtsClient::Mimic => "mimic",
        TtsClient::Mimic3 => "mimic3 (Mycroft)",
        TtsClient::Festival => "festival",
        TtsClient::GttsCli => "gtts (Google TTS CLI)",
        TtsClient::WindowsSapi => "sapi (Windows)",
        TtsClient::KokoroTts => "kokoro (Kokoro TTS)",
        TtsClient::Pico2Wave => "pico2wave",
        TtsClient::Balcon => "balcon",
        TtsClient::CoquiTts => "coqui-tts",
    }
}

// ============================================================================
// init subcommand
// ============================================================================

fn run_init() {
    let installed = InstalledTtsClients::new();
    let available = get_available_providers();

    println!("{}", "TTS Provider Status".bold());
    println!();

    if available.is_empty() {
        println!(
            "  {} No TTS providers detected on this system.",
            "!".yellow().bold()
        );
        println!();
        println!("  Run {} to install one.", "so-you-say install".bold());
        return;
    }

    // Show installed providers
    for provider in available {
        let name = provider_display_name(provider);
        println!("  {} {}", "✓".green().bold(), name);
    }
    println!();

    // Check if user only has low-quality providers (espeak-only)
    let only_espeak =
        available.len() == 1 && matches!(available[0], TtsProvider::Host(HostTtsProvider::ESpeak));
    let has_high_quality = available.iter().any(|p| {
        matches!(
            p,
            TtsProvider::Host(
                HostTtsProvider::KokoroTts
                    | HostTtsProvider::EchoGarden
                    | HostTtsProvider::Sherpa
                    | HostTtsProvider::Say
                    | HostTtsProvider::Piper
            ) | TtsProvider::Cloud(CloudTtsProvider::ElevenLabs)
        )
    });

    if only_espeak || !has_high_quality {
        println!(
            "  {} Your current setup only has basic-quality voices.",
            "→".yellow().bold()
        );
        println!(
            "    For much better quality, consider installing {}.",
            "kokoro".bold()
        );
        println!();

        // Check if kokoro is installable
        if installed.installable(TtsClient::KokoroTts) {
            match Confirm::new("Would you like to install kokoro now?")
                .with_default(true)
                .prompt()
            {
                Ok(true) => {
                    println!();
                    install_client_via_interview(TtsClient::KokoroTts, false);
                }
                Ok(false) => {
                    println!(
                        "  You can install it later with: {}",
                        "so-you-say install kokoro".bold()
                    );
                }
                Err(_) => {}
            }
        } else {
            println!(
                "    Install it manually: {}",
                "https://github.com/nazdridoy/kokoro-tts".dimmed()
            );
        }
    } else {
        println!("  {} Your TTS setup looks good!", "✓".green().bold());
    }
}

// ============================================================================
// install subcommand
// ============================================================================

fn run_install(pkg: Option<&str>, dry_run: bool) {
    let installed = InstalledTtsClients::new();

    if let Some(pkg_name) = pkg {
        let client = match parse_tts_client_name(pkg_name) {
            Some(c) => c,
            None => {
                eprintln!("Error: Unknown TTS provider '{}'", pkg_name);
                eprintln!(
                    "Use {} to see installable providers.",
                    "so-you-say install".bold()
                );
                std::process::exit(1);
            }
        };

        if installed.is_installed(client) && !dry_run {
            println!(
                "  {} {} is already installed.",
                "✓".green().bold(),
                tts_client_display_name(client)
            );
            return;
        }

        install_client_via_interview(client, dry_run);
    } else {
        // Interactive mode: show installed, prompt to install from remaining
        let installed_clients = dedupe_tts_client_aliases(installed.installed());

        if !installed_clients.is_empty() {
            println!("{}", "Installed TTS providers:".bold());
            println!();
            for client in &installed_clients {
                println!(
                    "  {} {}",
                    "✓".green().bold(),
                    tts_client_display_name(*client)
                );
            }
            println!();
        }

        let installable: Vec<TtsClient> = dedupe_tts_client_aliases(
            TtsClient::iter()
                .filter(|c| !installed.is_installed(*c) && installed.installable(*c))
                .collect(),
        );

        if installable.is_empty() {
            if installed_clients.is_empty() {
                println!("No TTS providers can be automatically installed on this OS.");
            } else {
                println!("All installable TTS providers are already installed.");
            }
            return;
        }

        #[derive(Clone)]
        struct ClientOption {
            client: TtsClient,
            label: String,
        }

        impl fmt::Display for ClientOption {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.label)
            }
        }

        let options: Vec<ClientOption> = installable
            .iter()
            .map(|c| ClientOption {
                client: *c,
                label: tts_client_display_name(*c).to_string(),
            })
            .collect();

        if let Ok(selection) = Select::new("Select a TTS provider to install", options).prompt() {
            println!();
            install_client_via_interview(selection.client, dry_run);
        }
    }
}

/// Drive sniff's install-plan + interview flow for the given `TtsClient`.
///
/// Exits with status 1 on failure. Honors `dry_run` by asking sniff to render
/// the command without executing it.
fn install_client_via_interview(client: TtsClient, dry_run: bool) {
    let host = HostCapabilities::detect();
    let plan = build_install_plan(&client, &host);

    if !plan.successful {
        eprintln!(
            "  {} {} cannot be automatically installed on this host.",
            "✗".red().bold(),
            tts_client_display_name(client)
        );
        for option in plan.failed_with_reason() {
            eprintln!(
                "    - {}: {}",
                option.kind.manager_name().dimmed(),
                option.reason
            );
        }
        if !plan.website.is_empty() {
            eprintln!("  See: {}", plan.website.dimmed());
        }
        std::process::exit(1);
    }

    let input = InstallInterviewInput {
        program: tts_client_display_name(client).to_string(),
        website: plan.website,
        plan,
    };
    let options = InstallInterviewOptions {
        install: if dry_run {
            InstallOptions::dry_run()
        } else {
            InstallOptions::default()
        },
        prompt_on_failure: true,
    };

    let mut ui = SoYouSayInstallUi::new();
    match run_install_interview(&input, &options, &mut ui) {
        Ok(InstallInterviewOutcome::Installed { .. } | InstallInterviewOutcome::DryRun { .. }) => {}
        Ok(InstallInterviewOutcome::AbortedByUser) => {
            eprintln!("  {} Installation aborted.", "!".yellow().bold());
            std::process::exit(1);
        }
        Ok(InstallInterviewOutcome::Failed { .. })
        | Ok(InstallInterviewOutcome::NotInstallable) => {
            std::process::exit(1);
        }
        // The interview already emitted the detached-descendant warning as a
        // `TimeoutWarning` event; this arm only supplies the terminal verdict.
        Ok(InstallInterviewOutcome::TimedOut { .. }) => {
            eprintln!(
                "  {} Installation timed out.",
                "✗".red().bold()
            );
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("  {} Install interview error: {}", "✗".red().bold(), e);
            std::process::exit(1);
        }
    }
}

/// Parse a user-provided string into a TtsClient.
fn parse_tts_client_name(name: &str) -> Option<TtsClient> {
    match name.to_lowercase().as_str() {
        "say" | "macos" => Some(TtsClient::Say),
        "espeak" | "espeak-ng" | "espeakng" => Some(TtsClient::EspeakNg),
        "piper" => Some(TtsClient::Piper),
        "echogarden" => Some(TtsClient::Echogarden),
        "sherpa" | "sherpa-onnx" | "sherpaonnx" => Some(TtsClient::SherpaOnnx),
        "mimic3" | "mimic" => Some(TtsClient::Mimic3),
        "festival" => Some(TtsClient::Festival),
        "gtts" | "gtts-cli" | "google" => Some(TtsClient::GttsCli),
        "sapi" | "windows" => Some(TtsClient::WindowsSapi),
        "kokoro" | "kokoro-tts" | "kokorotts" => Some(TtsClient::KokoroTts),
        "pico" | "pico2wave" => Some(TtsClient::Pico2Wave),
        "balcon" => Some(TtsClient::Balcon),
        "coqui" | "coqui-tts" => Some(TtsClient::CoquiTts),
        _ => None,
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    CompleteEnv::with_factory(Cli::command).complete();

    let cli = Cli::parse();

    // Handle subcommands first
    if let Some(command) = &cli.command {
        match command {
            Command::ListProviders => {
                print_providers();
                return Ok(());
            }
            Command::ListVoices { provider, lang } => {
                let resolved_provider = if let Some(provider_name) = provider {
                    match parse_provider_name(provider_name) {
                        Some(provider) => provider,
                        None => {
                            eprintln!("Error: Unknown provider '{}'", provider_name);
                            eprintln!("Use `so-you-say list-providers` to see available providers");
                            std::process::exit(1);
                        }
                    }
                } else {
                    let providers = listable_providers();
                    if providers.is_empty() {
                        println!("No TTS providers available for voice listing on this system.");
                        return Ok(());
                    }

                    match prompt_for_provider(&providers) {
                        Ok(selected) => selected,
                        Err(err) => {
                            eprintln!("Error: {}", err);
                            eprintln!("Use --provider to select a provider directly");
                            std::process::exit(1);
                        }
                    }
                };

                if !provider_supports_voice_listing(&resolved_provider) {
                    eprintln!(
                        "Error: Voice listing is not supported for '{}'",
                        provider_display_name(&resolved_provider)
                    );
                    eprintln!("Use `so-you-say list-providers` to see available providers");
                    std::process::exit(1);
                }

                match list_voices_for_provider(resolved_provider).await {
                    Ok(voices) => print_voices(resolved_provider, &voices, lang.as_deref()),
                    Err(err) => {
                        eprintln!("Error: {}", err);
                        std::process::exit(1);
                    }
                }

                return Ok(());
            }
            Command::Init => {
                run_init();
                return Ok(());
            }
            Command::Install { pkg, dry_run } => {
                run_install(pkg.as_deref(), *dry_run);
                return Ok(());
            }
        }
    }

    // Handle --background: re-spawn ourselves without --background as a detached process
    if cli.background {
        let args: Vec<String> = std::env::args().filter(|a| a != "--background").collect();
        std::process::Command::new(&args[0])
            .args(&args[1..])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()?;
        return Ok(());
    }

    // Handle --refresh-cache flag (does not exit early - continues with other operations)
    if cli.refresh_cache {
        println!("Clearing TTS provider cache...");
        if let Err(e) = bust_host_capability_cache() {
            eprintln!("Warning: Failed to clear cache: {}", e);
        }

        println!("Repopulating cache from all available providers...");
        match populate_cache_for_all_providers().await {
            Ok(()) => {
                println!("{}", "Cache refreshed successfully.".green());
                println!();
            }
            Err(e) => {
                eprintln!("Error refreshing cache: {}", e);
                std::process::exit(1);
            }
        }
        // Continue with other operations (speaking, etc.)
    }

    // No subcommand: speaking mode
    // If no text args and stdin is a terminal (not piped), show help
    let message = if cli.text.is_empty() {
        if io::stdin().is_terminal() {
            // No piped input and no text args — show help
            Cli::command().print_help()?;
            return Ok(());
        }
        // Piped input — read from stdin
        read_from_stdin()?
    } else {
        // Join all arguments with spaces
        join_args(cli.text)
    };

    // Build TTS config
    let mut config = TtsConfig::new();

    // Resolve provider first (needed for voice resolution)
    let provider = if let Some(provider_name) = &cli.provider {
        match parse_provider_name(provider_name) {
            Some(p) => Some(p),
            None => {
                eprintln!("Error: Unknown provider '{}'", provider_name);
                eprintln!("Use `so-you-say list-providers` to see available providers");
                std::process::exit(1);
            }
        }
    } else {
        None
    };

    // Apply --voice if specified
    // If a provider is specified, resolve the voice name to the actual engine voice
    if let Some(voice_name) = &cli.voice {
        let (resolved_voice, recommended_model) = if let Some(prov) = provider {
            // Try to resolve the voice name for this provider
            if provider_supports_voice_listing(&prov) {
                match list_voices_for_provider(prov).await {
                    Ok(voices) => {
                        match resolve_voice_name(&voices, voice_name, cli.lang.as_deref()) {
                            VoiceResolution::Found(voice) => {
                                let engine_name = get_engine_voice_name(&voice);
                                let model = voice.recommended_model().map(|s| s.to_string());
                                if std::env::var("DEBUG").is_ok() {
                                    eprintln!(
                                        "[DEBUG] Resolved '{}' -> identifier={:?}, engine_name='{}', model={:?}",
                                        voice_name, voice.identifier, engine_name, model
                                    );
                                }
                                (engine_name, model)
                            }
                            VoiceResolution::NotFound => {
                                eprintln!(
                                    "Error: Voice '{}' not found for {}",
                                    voice_name,
                                    provider_display_name(&prov)
                                );
                                eprintln!(
                                    "Use `so-you-say list-voices --provider {}` to see available voices",
                                    cli.provider.as_ref().unwrap()
                                );
                                std::process::exit(1);
                            }
                            VoiceResolution::Ambiguous(matches) => {
                                eprintln!("Error: Multiple voices match '{}':", voice_name);
                                for v in &matches {
                                    let lang = voice_language_label(&v.languages);
                                    eprintln!("  - {} ({})", extract_display_name(&v.name), lang);
                                }
                                eprintln!(
                                    "Use --lang to disambiguate (e.g., --lang {} --voice {})",
                                    voice_language_label(&matches[0].languages),
                                    voice_name
                                );
                                std::process::exit(1);
                            }
                        }
                    }
                    Err(_) => {
                        // Couldn't list voices, fall back to using the name directly
                        (voice_name.clone(), None)
                    }
                }
            } else {
                // Provider doesn't support voice listing, use the name directly
                (voice_name.clone(), None)
            }
        } else {
            // No provider specified, use the name directly
            (voice_name.clone(), None)
        };
        config = config.with_voice(resolved_voice);
        if let Some(model) = recommended_model {
            config = config.with_model(model);
        }
    }

    // Apply --gender if specified
    if let Some(gender) = cli.gender {
        config = config.with_gender(gender.into());
    }

    // Apply --lang if specified
    if let Some(lang_code) = &cli.lang {
        let language = if lang_code.eq_ignore_ascii_case("en")
            || lang_code.to_lowercase().starts_with("en-")
        {
            Language::English
        } else {
            Language::Custom(lang_code.clone())
        };
        config = config.with_language(language);
    }

    // Apply --provider if specified
    if let Some(prov) = provider {
        config = config.with_failover(TtsFailoverStrategy::SpecificProvider(prov));
    }

    // Apply --loud or --soft if specified
    let volume = if cli.loud {
        VolumeLevel::Loud
    } else if cli.soft {
        VolumeLevel::Soft
    } else {
        VolumeLevel::Normal
    };
    config = config.with_volume(volume);

    // Apply --fast or --slow if specified
    let speed = if cli.fast {
        SpeedLevel::Fast
    } else if cli.slow {
        SpeedLevel::Slow
    } else {
        SpeedLevel::Normal
    };
    config = config.with_speed(speed);

    // Call the async TTS function
    if cli.meta {
        // Use speak_with_result to get metadata
        match speak_with_result(&message, &config).await {
            Ok(result) => {
                print_speak_result(&result, volume, speed);
            }
            Err(e) => {
                eprintln!("Error: {:?}", e);
                std::process::exit(1);
            }
        }
    } else {
        // Standard speak without metadata
        if let Err(e) = speak(&message, &config).await {
            eprintln!("Error: {:?}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_join_args_multi_word() {
        let args = vec!["Hello".to_string(), "world".to_string()];
        assert_eq!(join_args(args), "Hello world");
    }

    #[test]
    fn test_join_args_single_word() {
        let args = vec!["Hello".to_string()];
        assert_eq!(join_args(args), "Hello");
    }

    #[test]
    fn test_join_args_empty() {
        let args: Vec<String> = vec![];
        assert_eq!(join_args(args), "");
    }

    #[test]
    fn test_join_args_unicode() {
        let args = vec!["Hello".to_string(), "世界".to_string(), "🚀".to_string()];
        assert_eq!(join_args(args), "Hello 世界 🚀");
    }

    #[test]
    fn test_join_args_with_empty_strings() {
        let args = vec![
            "".to_string(),
            "Hello".to_string(),
            "".to_string(),
            "world".to_string(),
        ];
        assert_eq!(join_args(args), " Hello  world");
    }

    #[test]
    fn test_join_args_special_chars() {
        let args = vec!["Hello,".to_string(), "world!".to_string()];
        assert_eq!(join_args(args), "Hello, world!");
    }

    // ========================================================================
    // Engine voice name tests
    // ========================================================================
    //
    // Tests for get_engine_voice_name() which extracts the actual voice name
    // that the TTS engine expects from our voice representation.

    #[test]
    fn test_get_engine_voice_name_with_prefixed_identifier() {
        // Echogarden uses prefixed identifiers like "kokoro:Heart"
        let voice = Voice::new("Heart")
            .with_identifier("kokoro:Heart")
            .with_quality(VoiceQuality::Excellent);

        // Should extract "Heart" from "kokoro:Heart"
        assert_eq!(get_engine_voice_name(&voice), "Heart");
    }

    #[test]
    fn test_get_engine_voice_name_vits() {
        // VITS voices have identifiers like "vits:de_DE-thorsten-high"
        let voice = Voice::new("de_DE-thorsten-high")
            .with_identifier("vits:de_DE-thorsten-high")
            .with_quality(VoiceQuality::Good);

        // Should extract "de_DE-thorsten-high" from "vits:de_DE-thorsten-high"
        assert_eq!(get_engine_voice_name(&voice), "de_DE-thorsten-high");
    }

    #[test]
    fn test_get_engine_voice_name_no_identifier() {
        // macOS say voices don't have identifiers
        let voice = Voice::new("Samantha")
            .with_gender(Gender::Female)
            .with_quality(VoiceQuality::Good);

        // Should return the name directly
        assert_eq!(get_engine_voice_name(&voice), "Samantha");
    }

    #[test]
    fn test_get_engine_voice_name_identifier_without_prefix() {
        // Some providers might have identifiers without colons
        let voice = Voice::new("Welsh")
            .with_identifier("cy")
            .with_quality(VoiceQuality::Good);

        // Should return the identifier as-is
        assert_eq!(get_engine_voice_name(&voice), "cy");
    }

    // ========================================================================
    // Voice resolution tests
    // ========================================================================
    //
    // Tests for resolve_voice_name() which maps user-provided display names
    // to the actual voice objects (case-insensitive, with language disambiguation).

    #[test]
    fn test_resolve_voice_name_exact_match() {
        let voices = vec![
            Voice::new("Heart").with_language(Language::English),
            Voice::new("Michael").with_language(Language::English),
        ];

        match resolve_voice_name(&voices, "Heart", None) {
            VoiceResolution::Found(v) => assert_eq!(v.name, "Heart"),
            _ => panic!("Expected Found"),
        }
    }

    #[test]
    fn test_resolve_voice_name_case_insensitive() {
        let voices = vec![
            Voice::new("Heart").with_language(Language::English),
            Voice::new("Michael").with_language(Language::English),
        ];

        // Lowercase should match
        match resolve_voice_name(&voices, "heart", None) {
            VoiceResolution::Found(v) => assert_eq!(v.name, "Heart"),
            _ => panic!("Expected Found"),
        }

        // Uppercase should match
        match resolve_voice_name(&voices, "HEART", None) {
            VoiceResolution::Found(v) => assert_eq!(v.name, "Heart"),
            _ => panic!("Expected Found"),
        }

        // Mixed case should match
        match resolve_voice_name(&voices, "HeArT", None) {
            VoiceResolution::Found(v) => assert_eq!(v.name, "Heart"),
            _ => panic!("Expected Found"),
        }
    }

    #[test]
    fn test_resolve_voice_name_vits_display_name() {
        let voices = vec![
            Voice::new("de_DE-thorsten-high").with_language(Language::Custom("de-DE".into())),
            Voice::new("en_US-lessac-high").with_language(Language::English),
        ];

        // "Thorsten" (extracted display name) should match "de_DE-thorsten-high"
        match resolve_voice_name(&voices, "Thorsten", None) {
            VoiceResolution::Found(v) => assert_eq!(v.name, "de_DE-thorsten-high"),
            _ => panic!("Expected Found"),
        }

        // Case insensitive
        match resolve_voice_name(&voices, "thorsten", None) {
            VoiceResolution::Found(v) => assert_eq!(v.name, "de_DE-thorsten-high"),
            _ => panic!("Expected Found"),
        }
    }

    #[test]
    fn test_resolve_voice_name_by_technical_name() {
        // Users should also be able to use the original technical name
        let voices = vec![
            Voice::new("de_DE-thorsten-high").with_language(Language::Custom("de-DE".into())),
            Voice::new("en_US-lessac-high").with_language(Language::English),
        ];

        // Technical name should match directly
        match resolve_voice_name(&voices, "de_DE-thorsten-high", None) {
            VoiceResolution::Found(v) => assert_eq!(v.name, "de_DE-thorsten-high"),
            _ => panic!("Expected Found"),
        }

        // Case insensitive for technical names too
        match resolve_voice_name(&voices, "DE_DE-THORSTEN-HIGH", None) {
            VoiceResolution::Found(v) => assert_eq!(v.name, "de_DE-thorsten-high"),
            _ => panic!("Expected Found"),
        }
    }

    #[test]
    fn test_resolve_voice_name_not_found() {
        let voices = vec![Voice::new("Heart").with_language(Language::English)];

        match resolve_voice_name(&voices, "NonExistent", None) {
            VoiceResolution::NotFound => {}
            _ => panic!("Expected NotFound"),
        }
    }

    #[test]
    fn test_resolve_voice_name_ambiguous() {
        // Alex exists in multiple languages
        let voices = vec![
            Voice::new("Alex").with_language(Language::Custom("es-ES".into())),
            Voice::new("Alex").with_language(Language::Custom("pt-BR".into())),
        ];

        match resolve_voice_name(&voices, "Alex", None) {
            VoiceResolution::Ambiguous(matches) => {
                assert_eq!(matches.len(), 2);
            }
            _ => panic!("Expected Ambiguous"),
        }
    }

    #[test]
    fn test_resolve_voice_name_disambiguate_with_language() {
        // Alex exists in multiple languages
        let voices = vec![
            Voice::new("Alex").with_language(Language::Custom("es-ES".into())),
            Voice::new("Alex").with_language(Language::Custom("pt-BR".into())),
        ];

        // With language filter, should resolve to one
        match resolve_voice_name(&voices, "Alex", Some("es")) {
            VoiceResolution::Found(v) => {
                assert!(
                    v.languages
                        .iter()
                        .any(|l| matches!(l, Language::Custom(c) if c == "es-ES"))
                );
            }
            _ => panic!("Expected Found with language filter"),
        }

        match resolve_voice_name(&voices, "Alex", Some("pt")) {
            VoiceResolution::Found(v) => {
                assert!(
                    v.languages
                        .iter()
                        .any(|l| matches!(l, Language::Custom(c) if c == "pt-BR"))
                );
            }
            _ => panic!("Expected Found with language filter"),
        }
    }

    // ========================================================================
    // Display name extraction tests
    // ========================================================================
    //
    // Tests for extracting clean voice names from VITS-style identifiers.
    // VITS voices have pattern: {locale}-{name}-{quality} (e.g., de_DE-thorsten-high)
    // We extract and title-case the name part.

    #[test]
    fn test_extract_display_name_vits_high_quality() {
        // VITS voice: de_DE-thorsten-high -> Thorsten
        assert_eq!(extract_display_name("de_DE-thorsten-high"), "Thorsten");
    }

    #[test]
    fn test_extract_display_name_vits_medium_quality() {
        // VITS voice: en_US-lessac-medium -> Lessac
        assert_eq!(extract_display_name("en_US-lessac-medium"), "Lessac");
    }

    #[test]
    fn test_extract_display_name_vits_low_quality() {
        // VITS voice: ar_JO-kareem-low -> Kareem
        assert_eq!(extract_display_name("ar_JO-kareem-low"), "Kareem");
    }

    #[test]
    fn test_extract_display_name_vits_x_low_quality() {
        // VITS voice: nl_BE-nathalie-x_low -> Nathalie
        assert_eq!(extract_display_name("nl_BE-nathalie-x_low"), "Nathalie");
    }

    #[test]
    fn test_extract_display_name_simple_kokoro_voice() {
        // Kokoro voices are simple names like "Heart", "Michael"
        assert_eq!(extract_display_name("Heart"), "Heart");
        assert_eq!(extract_display_name("Michael"), "Michael");
    }

    #[test]
    fn test_extract_display_name_simple_say_voice() {
        // macOS say voices are simple names
        assert_eq!(extract_display_name("Samantha"), "Samantha");
        assert_eq!(extract_display_name("Alex"), "Alex");
    }

    #[test]
    fn test_title_case_lowercase() {
        assert_eq!(title_case("thorsten"), "Thorsten");
        assert_eq!(title_case("lessac"), "Lessac");
    }

    #[test]
    fn test_title_case_uppercase() {
        assert_eq!(title_case("THORSTEN"), "Thorsten");
    }

    #[test]
    fn test_title_case_mixed() {
        assert_eq!(title_case("tHoRsTeN"), "Thorsten");
    }

    #[test]
    fn test_title_case_empty() {
        assert_eq!(title_case(""), "");
    }

    #[test]
    fn test_title_case_single_char() {
        assert_eq!(title_case("a"), "A");
        assert_eq!(title_case("A"), "A");
    }

    // ========================================================================
    // Language filtering tests
    // ========================================================================

    #[test]
    fn test_voice_matches_language_english_exact() {
        let voice = Voice::new("English").with_language(Language::English);

        assert!(voice_matches_language(&voice, "en"));
        assert!(voice_matches_language(&voice, "EN")); // case insensitive
    }

    #[test]
    fn test_voice_matches_language_english_variant_filter() {
        // A voice with Language::English should match "en-us" filter
        let voice = Voice::new("English").with_language(Language::English);

        assert!(voice_matches_language(&voice, "en-us"));
        assert!(voice_matches_language(&voice, "en-gb"));
    }

    #[test]
    fn test_voice_matches_language_custom_exact() {
        let voice = Voice::new("French").with_language(Language::Custom("fr".into()));

        assert!(voice_matches_language(&voice, "fr"));
        assert!(voice_matches_language(&voice, "FR")); // case insensitive
    }

    #[test]
    fn test_voice_matches_language_custom_prefix() {
        // "fr-CA" voice should match "fr" filter
        let voice = Voice::new("French (Canada)").with_language(Language::Custom("fr-CA".into()));

        assert!(voice_matches_language(&voice, "fr"));
    }

    #[test]
    fn test_voice_matches_language_no_match() {
        let voice = Voice::new("German").with_language(Language::Custom("de".into()));

        assert!(!voice_matches_language(&voice, "fr"));
        assert!(!voice_matches_language(&voice, "en"));
    }

    #[test]
    fn test_filter_voices_by_language() {
        let voices = vec![
            Voice::new("English").with_language(Language::English),
            Voice::new("French").with_language(Language::Custom("fr".into())),
            Voice::new("French (Canada)").with_language(Language::Custom("fr-CA".into())),
            Voice::new("German").with_language(Language::Custom("de".into())),
        ];

        let french_voices = filter_voices_by_language(voices.clone(), "fr");
        assert_eq!(french_voices.len(), 2);
        assert!(french_voices.iter().any(|v| v.name == "French"));
        assert!(french_voices.iter().any(|v| v.name == "French (Canada)"));

        let english_voices = filter_voices_by_language(voices.clone(), "en");
        assert_eq!(english_voices.len(), 1);
        assert!(english_voices.iter().any(|v| v.name == "English"));

        let german_voices = filter_voices_by_language(voices, "de");
        assert_eq!(german_voices.len(), 1);
        assert!(german_voices.iter().any(|v| v.name == "German"));
    }

    #[test]
    fn test_filter_voices_by_language_empty_result() {
        let voices = vec![
            Voice::new("English").with_language(Language::English),
            Voice::new("French").with_language(Language::Custom("fr".into())),
        ];

        let spanish_voices = filter_voices_by_language(voices, "es");
        assert!(spanish_voices.is_empty());
    }

    // ========================================================================
    // macOS voice quality suffix tests
    // ========================================================================
    //
    // Tests for handling macOS voice names like "Ava (Premium)", "Ava (Enhanced)".

    #[test]
    fn test_extract_display_name_macos_premium() {
        assert_eq!(extract_display_name("Ava (Premium)"), "Ava");
        assert_eq!(extract_display_name("Samantha (Premium)"), "Samantha");
    }

    #[test]
    fn test_extract_display_name_macos_enhanced() {
        assert_eq!(extract_display_name("Ava (Enhanced)"), "Ava");
        assert_eq!(extract_display_name("Samantha (Enhanced)"), "Samantha");
    }

    #[test]
    fn test_extract_display_name_macos_compact() {
        assert_eq!(extract_display_name("Ava (Compact)"), "Ava");
    }

    #[test]
    fn test_extract_display_name_macos_language_suffix() {
        // Voices with language/region suffixes should have just the name
        assert_eq!(extract_display_name("Eddy (English (UK))"), "Eddy");
        assert_eq!(extract_display_name("Eddy (English (US))"), "Eddy");
        assert_eq!(
            extract_display_name("Eddy (Chinese (China mainland))"),
            "Eddy"
        );
        assert_eq!(extract_display_name("Eddy (French (Canada))"), "Eddy");
        assert_eq!(extract_display_name("Eddy (German (Germany))"), "Eddy");
    }

    #[test]
    fn test_extract_display_name_macos_language_suffix_simple() {
        // Some voices have simpler language suffixes
        assert_eq!(extract_display_name("Aman (English (India))"), "Aman");
        assert_eq!(extract_display_name("Amélie (French (Canada))"), "Amélie");
    }

    #[test]
    fn test_strip_parenthetical_suffix_nested() {
        assert_eq!(
            strip_parenthetical_suffix("Eddy (Chinese (China mainland))"),
            "Eddy"
        );
        assert_eq!(strip_parenthetical_suffix("Eddy (English (UK))"), "Eddy");
    }

    #[test]
    fn test_strip_parenthetical_suffix_simple() {
        assert_eq!(strip_parenthetical_suffix("Voice (Something)"), "Voice");
    }

    #[test]
    fn test_strip_parenthetical_suffix_no_suffix() {
        assert_eq!(strip_parenthetical_suffix("Voice"), "Voice");
        assert_eq!(strip_parenthetical_suffix("Heart"), "Heart");
    }

    #[test]
    fn test_strip_parenthetical_suffix_preserves_middle_parens() {
        // If parens are in the middle (not a suffix), don't strip
        assert_eq!(strip_parenthetical_suffix("Foo (bar) baz"), "Foo (bar) baz");
    }

    #[test]
    fn test_extract_quality_tier_premium() {
        assert_eq!(extract_quality_tier("Ava (Premium)"), QualityTier::Premium);
        assert_eq!(
            extract_quality_tier("Samantha (Premium)"),
            QualityTier::Premium
        );
    }

    #[test]
    fn test_extract_quality_tier_enhanced() {
        assert_eq!(
            extract_quality_tier("Ava (Enhanced)"),
            QualityTier::Enhanced
        );
    }

    #[test]
    fn test_extract_quality_tier_compact() {
        assert_eq!(extract_quality_tier("Ava (Compact)"), QualityTier::Compact);
    }

    #[test]
    fn test_extract_quality_tier_no_suffix() {
        assert_eq!(extract_quality_tier("Ava"), QualityTier::Base);
        assert_eq!(extract_quality_tier("Heart"), QualityTier::Base);
    }

    #[test]
    fn test_quality_tier_ordering() {
        // Premium > Enhanced > Compact > Base
        assert!(QualityTier::Premium > QualityTier::Enhanced);
        assert!(QualityTier::Enhanced > QualityTier::Compact);
        assert!(QualityTier::Compact > QualityTier::Base);
    }

    // ========================================================================
    // Voice deduplication tests
    // ========================================================================

    #[test]
    fn test_deduplicate_voices_keeps_premium() {
        // When Ava exists as both Premium and Enhanced, keep Premium
        let voices = vec![
            Voice::new("Ava (Enhanced)").with_language(Language::English),
            Voice::new("Ava (Premium)").with_language(Language::English),
        ];

        let deduped = deduplicate_voices(voices);
        assert_eq!(deduped.len(), 1);
        assert_eq!(deduped[0].name, "Ava (Premium)");
    }

    #[test]
    fn test_deduplicate_voices_keeps_enhanced_over_compact() {
        let voices = vec![
            Voice::new("Ava (Compact)").with_language(Language::English),
            Voice::new("Ava (Enhanced)").with_language(Language::English),
        ];

        let deduped = deduplicate_voices(voices);
        assert_eq!(deduped.len(), 1);
        assert_eq!(deduped[0].name, "Ava (Enhanced)");
    }

    #[test]
    fn test_deduplicate_voices_different_languages_kept() {
        // Ava in English and Ava in Spanish should both be kept
        let voices = vec![
            Voice::new("Ava (Premium)").with_language(Language::English),
            Voice::new("Ava (Premium)").with_language(Language::Custom("es".into())),
        ];

        let deduped = deduplicate_voices(voices);
        assert_eq!(deduped.len(), 2);
    }

    #[test]
    fn test_deduplicate_voices_mixed() {
        // Mix of duplicates and unique voices
        let voices = vec![
            Voice::new("Ava (Enhanced)").with_language(Language::English),
            Voice::new("Ava (Premium)").with_language(Language::English),
            Voice::new("Samantha (Enhanced)").with_language(Language::English),
            Voice::new("Heart").with_language(Language::English),
        ];

        let deduped = deduplicate_voices(voices);
        assert_eq!(deduped.len(), 3);

        // Check that Ava Premium was kept
        assert!(deduped.iter().any(|v| v.name == "Ava (Premium)"));
        assert!(!deduped.iter().any(|v| v.name == "Ava (Enhanced)"));

        // Other voices should still be there
        assert!(deduped.iter().any(|v| v.name == "Samantha (Enhanced)"));
        assert!(deduped.iter().any(|v| v.name == "Heart"));
    }

    #[test]
    fn test_resolve_voice_deduplicates_quality_variants() {
        // When resolving "ava", should return the Premium version
        let voices = vec![
            Voice::new("Ava (Enhanced)").with_language(Language::English),
            Voice::new("Ava (Premium)").with_language(Language::English),
        ];

        match resolve_voice_name(&voices, "ava", None) {
            VoiceResolution::Found(v) => assert_eq!(v.name, "Ava (Premium)"),
            _ => panic!("Expected Found"),
        }
    }

    #[test]
    fn test_effective_voice_quality_from_suffix() {
        let premium = Voice::new("Ava (Premium)").with_quality(VoiceQuality::Good);
        let enhanced = Voice::new("Ava (Enhanced)").with_quality(VoiceQuality::Good);
        let compact = Voice::new("Ava (Compact)").with_quality(VoiceQuality::Good);

        // Quality should be inferred from suffix, not provider-reported
        assert_eq!(effective_voice_quality(&premium), VoiceQuality::Excellent);
        assert_eq!(effective_voice_quality(&enhanced), VoiceQuality::Good);
        assert_eq!(effective_voice_quality(&compact), VoiceQuality::Low);
    }

    #[test]
    fn test_effective_voice_quality_falls_back_to_provider() {
        // Voice without suffix should use provider-reported quality
        let voice = Voice::new("Heart").with_quality(VoiceQuality::Excellent);
        assert_eq!(effective_voice_quality(&voice), VoiceQuality::Excellent);
    }

    // ========================================================================
    // Volume label tests
    // ========================================================================

    #[test]
    fn test_volume_label() {
        assert_eq!(volume_label(VolumeLevel::Loud), "loud");
        assert_eq!(volume_label(VolumeLevel::Soft), "soft");
        assert_eq!(volume_label(VolumeLevel::Normal), "normal");
        assert_eq!(volume_label(VolumeLevel::Explicit(0.8)), "custom");
    }

    #[test]
    fn test_speed_label() {
        assert_eq!(speed_label(SpeedLevel::Fast), "fast");
        assert_eq!(speed_label(SpeedLevel::Slow), "slow");
        assert_eq!(speed_label(SpeedLevel::Normal), "normal");
        assert_eq!(speed_label(SpeedLevel::Explicit(1.5)), "custom");
    }

    // ========================================================================
    // TtsClient mapping tests
    // ========================================================================

    #[test]
    fn test_parse_tts_client_name() {
        assert_eq!(parse_tts_client_name("kokoro"), Some(TtsClient::KokoroTts));
        assert_eq!(
            parse_tts_client_name("kokoro-tts"),
            Some(TtsClient::KokoroTts)
        );
        assert_eq!(parse_tts_client_name("espeak"), Some(TtsClient::EspeakNg));
        assert_eq!(parse_tts_client_name("piper"), Some(TtsClient::Piper));
        assert_eq!(parse_tts_client_name("unknown-thing"), None);
    }

    #[test]
    fn test_host_provider_to_tts_client_roundtrip() {
        // Verify roundtrip for key providers
        let providers = [
            HostTtsProvider::Say,
            HostTtsProvider::ESpeak,
            HostTtsProvider::Piper,
            HostTtsProvider::KokoroTts,
            HostTtsProvider::EchoGarden,
        ];
        for provider in providers {
            let client = host_provider_to_tts_client(provider).unwrap();
            let back = tts_client_to_host_provider(client).unwrap();
            assert_eq!(back, provider);
        }
    }
}
