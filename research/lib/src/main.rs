//! Development binary for testing the research library

use research_lib::research;
use tts::Tts;

#[tokio::main]
async fn main() {
    let topic = "rig-core";

    match research(topic, None, &[]).await {
        Ok(result) => {
            println!("\n{}", "=".repeat(60));
            println!(
                "Complete: {} succeeded, {} failed in {:.1}s",
                result.succeeded, result.failed, result.total_time_secs
            );
            println!(
                "Total tokens: {} in, {} out, {} total",
                result.total_input_tokens, result.total_output_tokens, result.total_tokens
            );
            println!("{}", "=".repeat(60));

            // Announce completion via TTS
            announce_completion(&result.topic);
        }
        Err(e) => {
            eprintln!("Research failed: {}", e);
            std::process::exit(1);
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
