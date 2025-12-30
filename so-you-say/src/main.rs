use clap::Parser;
use std::io::{self, Read};

/// Simple text-to-speech CLI
///
/// # Examples
///
/// ```no_run
/// // Speak text from command-line arguments
/// // speak Hello world
///
/// // Speak text from stdin
/// // echo "Hello world" | speak
/// ```
#[derive(Parser)]
#[command(name = "speak")]
#[command(about = "Convert text to speech using system TTS", long_about = None)]
#[command(version)]
struct Cli {
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
        eprintln!("Usage: speak <text> or echo \"text\" | speak");
        std::process::exit(1);
    }

    Ok(text)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let message = if cli.text.is_empty() {
        // No arguments provided, read from stdin
        read_from_stdin()?
    } else {
        // Join all arguments with spaces
        join_args(cli.text)
    };

    // Call the shared TTS function
    shared::tts::speak(&message);

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
        let args = vec!["".to_string(), "Hello".to_string(), "".to_string(), "world".to_string()];
        assert_eq!(join_args(args), " Hello  world");
    }

    #[test]
    fn test_join_args_special_chars() {
        let args = vec!["Hello,".to_string(), "world!".to_string()];
        assert_eq!(join_args(args), "Hello, world!");
    }
}
