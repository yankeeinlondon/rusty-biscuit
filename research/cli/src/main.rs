//! Research CLI - Automated research tool for software libraries

use clap::{Parser, Subcommand};
use research_lib::research;
use std::io::{self, BufRead};
use std::path::PathBuf;
use tts::Tts;

#[derive(Parser)]
#[command(name = "research")]
#[command(about = "Automated research tool for software libraries", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Research a software library
    Library {
        /// The library/topic to research (use "-" to read from stdin)
        #[arg(value_name = "TOPIC")]
        topic: String,

        /// Additional questions to research in parallel
        #[arg(value_name = "QUESTIONS")]
        questions: Vec<String>,

        /// Output directory for research files [default: research/<TOPIC>]
        #[arg(short, long, value_name = "DIR")]
        output: Option<PathBuf>,
    },
}

fn read_topic_from_stdin() -> io::Result<String> {
    let stdin = io::stdin();
    let mut line = String::new();
    stdin.lock().read_line(&mut line)?;
    Ok(line.trim().to_string())
}

#[tokio::main]
async fn main() {
    // Load environment variables from .env file
    dotenvy::dotenv().ok();

    let cli = Cli::parse();

    match cli.command {
        Commands::Library {
            topic,
            questions,
            output,
        } => {
            // Read topic from stdin if "-" is provided
            let topic = if topic == "-" {
                match read_topic_from_stdin() {
                    Ok(t) if !t.is_empty() => t,
                    Ok(_) => {
                        eprintln!("Error: No topic provided on stdin");
                        std::process::exit(1);
                    }
                    Err(e) => {
                        eprintln!("Error reading from stdin: {}", e);
                        std::process::exit(1);
                    }
                }
            } else {
                topic
            };

            match research(&topic, output, &questions).await {
                Ok(result) => {
                    println!("\n{}", "=".repeat(60));
                    if result.cancelled {
                        println!(
                            "Cancelled: {} succeeded, {} failed in {:.1}s",
                            result.succeeded, result.failed, result.total_time_secs
                        );
                    } else {
                        println!(
                            "Complete: {} succeeded, {} failed in {:.1}s",
                            result.succeeded, result.failed, result.total_time_secs
                        );
                    }
                    println!(
                        "Total tokens: {} in, {} out, {} total",
                        result.total_input_tokens, result.total_output_tokens, result.total_tokens
                    );
                    println!("Output: {:?}", result.output_dir);
                    println!("{}", "=".repeat(60));

                    // Only announce if not cancelled
                    if !result.cancelled {
                        announce_completion(&result.topic);
                    }
                }
                Err(e) => {
                    eprintln!("Research failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
}

fn announce_completion(topic: &str) {
    if let Ok(mut tts) = Tts::default() {
        if let Ok(voices) = tts.voices() {
            if let Some(voice) = voices.iter().find(|v| {
                !v.id().contains("compact")
                    && !v.id().contains("eloquence")
                    && v.language().starts_with("en")
            }) {
                let _ = tts.set_voice(voice);
            }
        }

        let message = format!("Research for the {} library has completed", topic);
        if tts.speak(&message, false).is_ok() {
            std::thread::sleep(std::time::Duration::from_millis(100));
            while tts.is_speaking().unwrap_or(false) {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
    }
}
