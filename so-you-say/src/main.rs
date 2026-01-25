use biscuit_speaks::{
    get_available_providers, parse_provider_name, speak, Gender, HostTtsProvider, TtsConfig,
    TtsFailoverStrategy, TtsProvider,
};
use clap::{Parser, ValueEnum};
use std::io::{self, Read};

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

/// Print available TTS providers
fn print_providers() {
    let providers = get_available_providers();

    if providers.is_empty() {
        println!("No TTS providers available on this system.");
        return;
    }

    println!("Available TTS providers:");
    println!();

    for provider in providers {
        match provider {
            TtsProvider::Host(h) => {
                let name = match h {
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
                };
                println!("  - {}", name);
            }
            TtsProvider::Cloud(c) => {
                let name = match c {
                    biscuit_speaks::CloudTtsProvider::ElevenLabs => {
                        "elevenlabs (ElevenLabs API)"
                    }
                    _ => "unknown cloud provider",
                };
                println!("  - {} [cloud]", name);
            }
            _ => {
                println!("  - unknown provider");
            }
        }
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
