//! Info command - shows detailed model information.

use color_eyre::eyre::{eyre, Result};
use model_citizen::{
    scanner::{LlamaCppScanner, LmStudioScanner, OllamaScanner},
    sharing, Config, ModelRegistry,
};

pub async fn run(model_name: &str, json_output: bool) -> Result<()> {
    let config = Config::load()?;
    let mut registry = ModelRegistry::new();

    if config.scanners.ollama.enabled {
        registry.add_scanner(OllamaScanner::new(&config));
    }
    if config.scanners.lmstudio.enabled {
        registry.add_scanner(LmStudioScanner::new(&config));
    }
    if config.scanners.llamacpp.enabled {
        registry.add_scanner(LlamaCppScanner::new(&config));
    }

    let models = registry.scan_all().await;

    // Find matching model
    let model = models
        .iter()
        .find(|m| {
            m.name.to_lowercase().contains(&model_name.to_lowercase())
                || m.id.to_lowercase().contains(&model_name.to_lowercase())
        })
        .ok_or_else(|| eyre!("Model not found: {}", model_name))?;

    if json_output {
        let json = serde_json::to_string_pretty(model)?;
        println!("{json}");
    } else {
        println!("Model Information");
        println!("─────────────────");
        println!("  ID:            {}", model.id);
        println!("  Name:          {}", model.name);
        println!("  Size:          {}", model.size_display());
        println!("  Quantization:  {}", model.quantization.as_str());
        println!("  Architecture:  {}", model.architecture.as_str());
        println!("  Source:        {}", model.source.as_str());
        println!("  Path:          {}", model.path.display());

        // Check if it's a symlink
        if sharing::is_symlink(&model.path) {
            if let Ok(original) = sharing::resolve_original(&model.path) {
                println!("  Original:      {} (symlink)", original.display());
            } else {
                println!("  Status:        Broken symlink");
            }
        }

        // Show other locations if shared
        if let Some(registry_path) = sharing::default_registry_path() {
            if let Ok(share_registry) = sharing::ShareRegistry::load(&registry_path) {
                let shares = share_registry.get_shares(&model.path);
                if !shares.is_empty() {
                    println!("\n  Shared to:");
                    for share in shares {
                        println!("    - {}", share.display());
                    }
                }
            }
        }
    }

    Ok(())
}
