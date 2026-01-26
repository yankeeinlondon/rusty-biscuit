use std::fmt;
use std::io::{self, Read};

use biscuit_speaks::{
    get_available_providers, parse_provider_name, speak, CloudTtsProvider, EchogardenProvider,
    ESpeakProvider, ElevenLabsProvider, Gender, GttsProvider, HostTtsProvider, KokoroTtsProvider,
    Language, SayProvider, SapiProvider, TtsConfig, TtsError, TtsFailoverStrategy, TtsProvider,
    TtsVoiceInventory, Voice, VoiceQuality,
};
use clap::{Parser, ValueEnum};
use inquire::Select;

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

/// Simple text-to-speech CLI
///
/// # Examples
///
/// ```no_run
/// // Speak text from command-line arguments
/// // so-you-say Hello world
///
/// // Speak text with a specific voice
/// // so-you-say --voice Samantha Hello world
///
/// // Speak text with a female voice
/// // so-you-say --gender female Hello world
///
/// // List available TTS providers
/// // so-you-say --list-providers
///
/// // List available voices for a provider
/// // so-you-say --list-voices --provider say
///
/// // Select a provider interactively to list voices
/// // so-you-say --list-voices
///
/// // Use a specific TTS provider
/// // so-you-say --provider say Hello world
///
/// // Speak text from stdin
/// // echo "Hello world" | so-you-say
/// ```
#[derive(Parser)]
#[command(name = "so-you-say")]
#[command(about = "Convert text to speech using system TTS", long_about = None)]
#[command(version)]
struct Cli {
    /// List available TTS providers and exit
    #[arg(long)]
    list_providers: bool,

    /// List available voices for a provider and exit
    #[arg(long, conflicts_with = "list_providers")]
    list_voices: bool,

    /// Use a specific TTS provider
    #[arg(long)]
    provider: Option<String>,

    /// Use a specific voice by name
    #[arg(long)]
    voice: Option<String>,

    /// Voice gender preference (male or female)
    #[arg(short, long, value_enum)]
    gender: Option<GenderArg>,

    /// Text to speak (reads from stdin if not provided)
    text: Vec<String>,
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
        eprintln!("Usage: so-you-say <text> or echo \"text\" | so-you-say");
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
        let name = provider_display_name(&provider);
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

async fn list_voices_for_provider(provider: TtsProvider) -> Result<Vec<Voice>, TtsError> {
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
        Gender::Male => "M",
        Gender::Female => "F",
        Gender::Any => "-",
        _ => "?",
    }
}

fn voice_quality_label(quality: VoiceQuality) -> &'static str {
    match quality {
        VoiceQuality::Excellent => "excellent",
        VoiceQuality::Good => "good",
        VoiceQuality::Moderate => "moderate",
        VoiceQuality::Low => "low",
        VoiceQuality::Unknown => "unknown",
    }
}

fn voice_language_label(languages: &[Language]) -> String {
    languages
        .first()
        .map(|language| match language {
            Language::English => "en".to_string(),
            Language::Custom(code) => code.clone(),
            _ => "?".to_string(),
        })
        .unwrap_or_else(|| "-".to_string())
}

fn print_voices(provider: TtsProvider, voices: &[Voice]) {
    if voices.is_empty() {
        println!(
            "No voices found for {}.",
            provider_display_name(&provider)
        );
        return;
    }

    println!(
        "Found {} voices for {}:\n",
        voices.len(),
        provider_display_name(&provider)
    );

    let mut voices = voices.to_vec();
    voices.sort_by(|a, b| {
        voice_quality_rank(a.quality)
            .cmp(&voice_quality_rank(b.quality))
            .then_with(|| a.name.cmp(&b.name))
    });

    for voice in voices {
        let gender = voice_gender_label(voice.gender);
        let quality = voice_quality_label(voice.quality);
        let language = voice_language_label(&voice.languages);
        println!("  - {} ({}/{}/{})", voice.name, gender, quality, language);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Handle --list-providers flag
    if cli.list_providers {
        print_providers();
        return Ok(());
    }

    if cli.list_voices {
        let provider = if let Some(provider_name) = &cli.provider {
            match parse_provider_name(provider_name) {
                Some(provider) => provider,
                None => {
                    eprintln!("Error: Unknown provider '{}'", provider_name);
                    eprintln!("Use --list-providers to see available providers");
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

        if !provider_supports_voice_listing(&provider) {
            eprintln!(
                "Error: Voice listing is not supported for '{}'",
                provider_display_name(&provider)
            );
            eprintln!("Use --list-providers to see available providers");
            std::process::exit(1);
        }

        match list_voices_for_provider(provider).await {
            Ok(voices) => print_voices(provider, &voices),
            Err(err) => {
                eprintln!("Error: {}", err);
                std::process::exit(1);
            }
        }

        return Ok(());
    }

    let message = if cli.text.is_empty() {
        // No arguments provided, read from stdin
        read_from_stdin()?
    } else {
        // Join all arguments with spaces
        join_args(cli.text)
    };

    // Build TTS config
    let mut config = TtsConfig::new();

    // Apply --voice if specified
    if let Some(voice_name) = &cli.voice {
        config = config.with_voice(voice_name.clone());
    }

    // Apply --gender if specified
    if let Some(gender) = cli.gender {
        config = config.with_gender(gender.into());
    }

    // Apply --provider if specified
    if let Some(provider_name) = &cli.provider {
        if let Some(provider) = parse_provider_name(provider_name) {
            config = config.with_failover(TtsFailoverStrategy::SpecificProvider(provider));
        } else {
            eprintln!("Error: Unknown provider '{}'", provider_name);
            eprintln!("Use --list-providers to see available providers");
            std::process::exit(1);
        }
    }

    // Call the async TTS function
    if let Err(e) = speak(&message, &config).await {
        eprintln!("Error: {:?}", e);
        std::process::exit(1);
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
}
