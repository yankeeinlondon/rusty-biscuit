use biscuit_speaks::{
    Gender, SystemVoiceInfo, VoiceConfig, VoiceSelector, available_system_voices,
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
/// // List available voices
/// // so-you-say --list-voices
///
/// // Speak text from stdin
/// // echo "Hello world" | so-you-say
/// ```
#[derive(Parser)]
#[command(name = "so-you-say")]
#[command(about = "Convert text to speech using system TTS", long_about = None)]
#[command(version)]
struct Cli {
    /// List available system voices and exit
    #[arg(long)]
    list_voices: bool,

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
///
/// # Examples
///
/// ```
/// # use so_you_say::join_args;
/// let args = vec!["Hello".to_string(), "world".to_string()];
/// assert_eq!(join_args(args), "Hello world");
/// ```
fn join_args(args: Vec<String>) -> String {
    args.join(" ")
}

/// Reads text from stdin with a 10,000 character limit
///
/// # Errors
///
/// Returns an error if stdin cannot be read or if input is empty
///
/// # Examples
///
/// ```no_run
/// # use so_you_say::read_from_stdin;
/// # fn main() -> std::io::Result<()> {
/// let text = read_from_stdin()?;
/// println!("Read: {}", text);
/// # Ok(())
/// # }
/// ```
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

/// Check if a voice ID should be excluded (compact/eloquence voices).
fn is_excluded_voice(id: &str) -> bool {
    let lower = id.to_lowercase();
    lower.contains("compact") || lower.contains("eloquence")
}

/// Check if a voice is Premium quality (highest on macOS).
fn is_premium_voice(name: &str) -> bool {
    name.contains("(Premium)")
}

/// Check if a voice is Enhanced quality (high on macOS).
fn is_enhanced_voice(name: &str) -> bool {
    name.contains("(Enhanced)")
}

/// Find the best voice matching criteria, preferring Premium > Enhanced > regular.
fn find_best_voice<'a>(
    voices: &'a [SystemVoiceInfo],
    lang_prefix: &str,
    target_gender: Option<Gender>,
) -> Option<&'a SystemVoiceInfo> {
    let matches_criteria = |v: &&SystemVoiceInfo| {
        !is_excluded_voice(&v.id)
            && v.language.starts_with(lang_prefix)
            && (target_gender.is_none() || v.gender == target_gender)
    };

    // Try Premium first
    if let Some(voice) = voices
        .iter()
        .filter(matches_criteria)
        .find(|v| is_premium_voice(&v.name))
    {
        return Some(voice);
    }

    // Try Enhanced next
    if let Some(voice) = voices
        .iter()
        .filter(matches_criteria)
        .find(|v| is_enhanced_voice(&v.name))
    {
        return Some(voice);
    }

    // Fall back to any matching voice
    voices.iter().find(matches_criteria)
}

/// Find the default voice ID that would be selected by the voice selection algorithm.
///
/// Algorithm matches shared::tts::select_voice:
/// 1. If gender specified: best quality voice matching language + gender (Premium > Enhanced > regular)
/// 2. Fallback: best quality voice matching language (any gender)
/// 3. Final fallback: any English voice
fn find_default_voice_id(voices: &[SystemVoiceInfo], gender: Option<GenderArg>) -> Option<String> {
    let lang_prefix = "en";

    // Step 1: Try language + gender filtering with quality preference
    if let Some(g) = gender {
        let target_gender = match g {
            GenderArg::Male => Some(Gender::Male),
            GenderArg::Female => Some(Gender::Female),
        };
        if let Some(voice) = find_best_voice(voices, lang_prefix, target_gender) {
            return Some(voice.id.clone());
        }
    }

    // Step 2: Fall back to language filtering only (any gender) with quality preference
    if let Some(voice) = find_best_voice(voices, lang_prefix, None) {
        return Some(voice.id.clone());
    }

    // Step 3: Final fallback - any English voice
    voices
        .iter()
        .find(|v| v.language.starts_with("en"))
        .map(|v| v.id.clone())
}

/// ANSI escape codes for terminal formatting.
const BOLD: &str = "\x1b[1m";
const RESET: &str = "\x1b[0m";

/// Print available voices in a formatted table with the default voice highlighted.
fn print_voices(voices: &[SystemVoiceInfo], filter_gender: Option<GenderArg>) {
    // Find the default voice ID before filtering
    let default_id = find_default_voice_id(voices, filter_gender);

    // Filter by gender if specified
    let filtered: Vec<_> = voices
        .iter()
        .filter(|v| match filter_gender {
            None => true,
            Some(GenderArg::Male) => v.gender == Some(Gender::Male),
            Some(GenderArg::Female) => v.gender == Some(Gender::Female),
        })
        .collect();

    if filtered.is_empty() {
        println!("No voices found.");
        return;
    }

    // Calculate column widths
    let name_width = filtered
        .iter()
        .map(|v| v.name.len())
        .max()
        .unwrap_or(4)
        .max(4);
    let lang_width = filtered
        .iter()
        .map(|v| v.language.len())
        .max()
        .unwrap_or(8)
        .max(8);

    // Print header
    println!(
        "{:<name_width$}  {:<lang_width$}  {:<7}  ID",
        "NAME",
        "LANGUAGE",
        "GENDER",
        name_width = name_width,
        lang_width = lang_width
    );
    println!(
        "{:-<name_width$}  {:-<lang_width$}  {:-<7}  {:-<40}",
        "",
        "",
        "",
        "",
        name_width = name_width,
        lang_width = lang_width
    );

    // Print voices (bold for default)
    for voice in filtered {
        let is_default = default_id.as_ref() == Some(&voice.id);
        let (start, end) = if is_default { (BOLD, RESET) } else { ("", "") };

        println!(
            "{start}{:<name_width$}  {:<lang_width$}  {:<7}  {}{end}",
            voice.name,
            voice.language,
            voice.gender_str(),
            voice.id,
            name_width = name_width,
            lang_width = lang_width,
            start = start,
            end = end
        );
    }

    // Print legend if a default was found
    if default_id.is_some() {
        println!();
        println!("{}Bold{} = default voice", BOLD, RESET);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Handle --list-voices flag
    if cli.list_voices {
        match available_system_voices() {
            Ok(voices) => {
                print_voices(&voices, cli.gender);
                return Ok(());
            }
            Err(e) => {
                eprintln!("Error: Failed to query system voices: {}", e);
                std::process::exit(1);
            }
        }
    }

    let message = if cli.text.is_empty() {
        // No arguments provided, read from stdin
        read_from_stdin()?
    } else {
        // Join all arguments with spaces
        join_args(cli.text)
    };

    // Build voice config
    let mut config = VoiceConfig::new();

    // Apply --voice if specified
    if let Some(voice_name) = &cli.voice {
        config = config.with_voice(VoiceSelector::ByName(voice_name.clone()));
    }

    // Apply --gender if specified
    if let Some(gender) = cli.gender {
        config = config.of_gender(gender.into());
    }

    // Call the TTS function
    biscuit_speaks::speak_when_able(&message, &config);

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
