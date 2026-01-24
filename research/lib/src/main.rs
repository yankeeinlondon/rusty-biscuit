//! Development binary for testing the research library

use research_lib::research;

#[tokio::main]
async fn main() {
    let topic = "rig-core";

    match research(topic, None, &[], false, false).await {
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
            let message = format!("Research for the {} library has completed", result.topic);
            biscuit_speaks::speak_when_able(&message, &biscuit_speaks::TtsConfig::default()).await;
        }
        Err(e) => {
            eprintln!("Research failed: {}", e);
            std::process::exit(1);
        }
    }
}
