//! CLI for hashing content using various algorithms.
//!
//! ## Usage
//!
//! ```bash
//! # Hash content with xxHash (default)
//! bh "some content"
//!
//! # Hash file contents
//! bh --file path/to/file.txt
//!
//! # Use BLAKE3 cryptographic hash
//! bh --crypto "some content"
//!
//! # Hash a password with Argon2id
//! bh --password "mysecret"
//! echo "mysecret" | bh --password -
//!
//! # Generate shell completions
//! source <(COMPLETE=bash bh)
//! ```

use std::fs;
use std::io::{self, BufRead, IsTerminal};
use std::path::PathBuf;

use clap::{CommandFactory, Parser, ValueHint};
use clap_complete::Shell;

use biscuit_hash::{blake3_hash, hash_password, xx_hash};

/// Hash content using various algorithms (xxHash, BLAKE3, Argon2id)
#[derive(Parser)]
#[command(name = "bh", version, about, long_about = None)]
#[command(after_help = AFTER_HELP)]
struct Cli {
    /// Content to hash (use "-" with --password to read from stdin)
    #[arg(value_name = "CONTENT")]
    content: Option<String>,

    /// Hash contents of a file instead of direct content
    #[arg(short, long, value_name = "PATH", value_hint = ValueHint::FilePath, conflicts_with = "content")]
    file: Option<PathBuf>,

    /// Use BLAKE3 cryptographic hash instead of xxHash
    #[arg(short, long, conflicts_with = "password")]
    crypto: bool,

    /// Password hashing mode (Argon2id). Use "-" as content to read from stdin
    #[arg(short, long, conflicts_with = "crypto")]
    password: bool,
}

const AFTER_HELP: &str = "\
SHELL COMPLETIONS:
  Enable tab completions by adding one line to your shell config:

  Bash (~/.bashrc):
    source <(COMPLETE=bash bh)

  Zsh (~/.zshrc):
    source <(COMPLETE=zsh bh)

  Fish (~/.config/fish/config.fish):
    COMPLETE=fish bh | source

  PowerShell ($PROFILE):
    Invoke-Expression (& bh _complete powershell)

EXAMPLES:
  bh \"hello world\"              # xxHash of string
  bh --file Cargo.toml           # xxHash of file contents
  bh --crypto \"hello world\"     # BLAKE3 cryptographic hash
  bh --password \"secret\"        # Argon2id password hash
  echo \"secret\" | bh --password -  # Password from stdin
";

fn main() {
    // Check for shell completion generation before parsing args
    if let Ok(shell_name) = std::env::var("COMPLETE") {
        generate_completions(&shell_name);
        return;
    }

    let cli = Cli::parse();

    // Show help if no input provided and stdin is a terminal
    if cli.content.is_none() && cli.file.is_none() && io::stdin().is_terminal() {
        Cli::command().print_help().unwrap();
        println!();
        return;
    }

    // Get the content to hash
    let content = match get_content(&cli) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    };

    // Perform hashing based on mode
    let result = if cli.password {
        match hash_password(&content) {
            Ok(hash) => hash,
            Err(e) => {
                eprintln!("Error hashing password: {e}");
                std::process::exit(1);
            }
        }
    } else if cli.crypto {
        blake3_hash(&content)
    } else {
        xx_hash(&content).to_string()
    };

    println!("{result}");
}

/// Get content to hash from positional arg, file, or stdin.
fn get_content(cli: &Cli) -> Result<String, String> {
    // File mode
    if let Some(ref path) = cli.file {
        return fs::read_to_string(path)
            .map_err(|e| format!("Failed to read file '{}': {e}", path.display()));
    }

    // Content from positional arg or stdin
    match cli.content.as_deref() {
        Some("-") => {
            // Read from stdin
            read_from_stdin()
        }
        Some(content) => Ok(content.to_string()),
        None => {
            // No content provided - check if stdin has data
            if !io::stdin().is_terminal() {
                read_from_stdin()
            } else {
                Err("No content provided. Use positional argument, --file, or pipe to stdin.".into())
            }
        }
    }
}

/// Read content from stdin.
fn read_from_stdin() -> Result<String, String> {
    let stdin = io::stdin();
    let mut content = String::new();

    for line in stdin.lock().lines() {
        let line = line.map_err(|e| format!("Failed to read from stdin: {e}"))?;
        if !content.is_empty() {
            content.push('\n');
        }
        content.push_str(&line);
    }

    if content.is_empty() {
        return Err("Empty input from stdin".into());
    }

    Ok(content)
}

/// Generate shell completions and exit.
fn generate_completions(shell_name: &str) {
    let shell = match shell_name.to_lowercase().as_str() {
        "bash" => Shell::Bash,
        "zsh" => Shell::Zsh,
        "fish" => Shell::Fish,
        "powershell" => Shell::PowerShell,
        "elvish" => Shell::Elvish,
        _ => {
            eprintln!(
                "Unknown shell: {shell_name}. Supported: bash, zsh, fish, powershell, elvish"
            );
            std::process::exit(1);
        }
    };

    clap_complete::generate(shell, &mut Cli::command(), "bh", &mut io::stdout());
}
