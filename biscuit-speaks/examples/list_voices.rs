//! List Voices Example
//!
//! This example demonstrates how to use the biscuit-speaks cache system
//! to list all available TTS voices on the system.
//!
//! ## Usage
//!
//! ```bash
//! cargo run --example list_voices
//! cargo run --example list_voices -- --refresh  # Force refresh the cache
//! ```

use biscuit_speaks::{
    bust_host_capability_cache, populate_cache_for_all_providers, read_from_cache,
    HostTtsCapability, TtsProvider,
};

/// Display voices grouped by provider.
fn display_voices(capabilities: &[HostTtsCapability]) {
    if capabilities.is_empty() {
        println!("No TTS providers found in cache.");
        return;
    }

    let total_voices: usize = capabilities.iter().map(|c| c.voices.len()).sum();
    println!(
        "Found {} TTS providers with {} total voices:\n",
        capabilities.len(),
        total_voices
    );

    for cap in capabilities {
        let provider_name = match &cap.provider {
            TtsProvider::Host(h) => format!("{:?}", h),
            TtsProvider::Cloud(c) => format!("{:?}", c),
            _ => "Unknown".to_string(),
        };

        println!(
            "## {} ({} voices)",
            provider_name,
            cap.voices.len()
        );

        if cap.voices.is_empty() {
            println!("   (no voices available)");
        } else {
            // Group voices by quality for display
            let mut voices = cap.voices.clone();
            voices.sort_by(|a, b| {
                // Sort by quality (excellent first), then by name
                let quality_order = |q: &biscuit_speaks::VoiceQuality| match q {
                    biscuit_speaks::VoiceQuality::Excellent => 0,
                    biscuit_speaks::VoiceQuality::Good => 1,
                    biscuit_speaks::VoiceQuality::Moderate => 2,
                    biscuit_speaks::VoiceQuality::Low => 3,
                    biscuit_speaks::VoiceQuality::Unknown => 4,
                };
                quality_order(&a.quality)
                    .cmp(&quality_order(&b.quality))
                    .then_with(|| a.name.cmp(&b.name))
            });

            for voice in voices.iter().take(20) {
                // Limit display to 20 voices per provider
                let gender_str = match voice.gender {
                    biscuit_speaks::Gender::Male => "M",
                    biscuit_speaks::Gender::Female => "F",
                    biscuit_speaks::Gender::Any => "-",
                    _ => "?",
                };
                let quality_str = match voice.quality {
                    biscuit_speaks::VoiceQuality::Excellent => "excellent",
                    biscuit_speaks::VoiceQuality::Good => "good",
                    biscuit_speaks::VoiceQuality::Moderate => "moderate",
                    biscuit_speaks::VoiceQuality::Low => "low",
                    biscuit_speaks::VoiceQuality::Unknown => "unknown",
                };
                let lang_str = voice
                    .languages
                    .first()
                    .map(|l| match l {
                        biscuit_speaks::Language::English => "en".to_string(),
                        biscuit_speaks::Language::Custom(c) => c.clone(),
                        _ => "?".to_string(),
                    })
                    .unwrap_or_else(|| "-".to_string());

                println!(
                    "   - {} ({}/{}/{})",
                    voice.name, gender_str, quality_str, lang_str
                );
            }

            if voices.len() > 20 {
                println!("   ... and {} more voices", voices.len() - 20);
            }
        }

        // Show available (not installed) voices if any
        if !cap.available_voices.is_empty() {
            println!(
                "   ({} additional voices available for download)",
                cap.available_voices.len()
            );
        }

        println!();
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Check for --refresh flag
    let args: Vec<String> = std::env::args().collect();
    let force_refresh = args.iter().any(|a| a == "--refresh" || a == "-r");

    // Try to read from cache first (unless refresh requested)
    let capabilities = if force_refresh {
        println!("Refreshing voice cache...\n");
        bust_host_capability_cache()?;
        None
    } else {
        match read_from_cache() {
            Ok(caps) => {
                println!("Loaded voices from cache.\n");
                Some(caps)
            }
            Err(e) => {
                println!("Cache not found or invalid ({}), populating...\n", e);
                None
            }
        }
    };

    // If cache wasn't available, populate it
    let capabilities = match capabilities {
        Some(caps) => caps,
        None => {
            println!("Detecting available TTS providers and enumerating voices...");
            println!("(This may take a moment for cloud providers)\n");

            match populate_cache_for_all_providers().await {
                Ok(()) => {
                    println!("Cache populated successfully.\n");
                    read_from_cache()?
                }
                Err(e) => {
                    eprintln!("Failed to populate cache: {}", e);
                    eprintln!("\nNote: Make sure at least one TTS provider is installed.");
                    eprintln!("For cloud providers (ElevenLabs), set the appropriate API key.");
                    return Err(e.into());
                }
            }
        }
    };

    // Display the voices
    display_voices(&capabilities.providers);

    // Display cache metadata
    if capabilities.last_updated > 0 {
        let timestamp = capabilities.last_updated;
        println!("---");
        println!(
            "Cache last updated: {} (Unix timestamp: {})",
            chrono_lite_format(timestamp),
            timestamp
        );
    }

    Ok(())
}

/// Simple timestamp formatter (avoids chrono dependency).
fn chrono_lite_format(unix_timestamp: u64) -> String {
    // Just return a human-readable relative time
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let diff = now.saturating_sub(unix_timestamp);

    if diff < 60 {
        "just now".to_string()
    } else if diff < 3600 {
        format!("{} minutes ago", diff / 60)
    } else if diff < 86400 {
        format!("{} hours ago", diff / 3600)
    } else {
        format!("{} days ago", diff / 86400)
    }
}
