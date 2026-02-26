//! Info command - shows detailed model information.

use color_eyre::eyre::{Result, eyre};
use inquire::Select;
use model_citizen::{
    Config, ModelRegistry, UnifiedModel,
    scanner::{LlamaCppScanner, LmStudioScanner, OllamaScanner},
};
use crate::output::print_model_info;

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

    // Find matching models, sorted alphabetically by name
    let query = model_name.to_lowercase();
    let mut matches: Vec<&UnifiedModel> = models
        .iter()
        .filter(|m| m.name.to_lowercase().contains(&query))
        .collect();
    matches.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    let mut model = match matches.len() {
        0 => return Err(eyre!("Model not found: {}", model_name)),
        1 => matches[0].clone(),
        _ => {
            if json_output {
                return Err(eyre!(
                    "Ambiguous model name '{}' matches {} models. Be more specific or use the full model ID.",
                    model_name,
                    matches.len()
                ));
            }
            select_model(&matches, model_name)?.clone()
        }
    };

    // Enrich with provider-specific metadata (e.g., Ollama /api/show)
    registry.enrich(&mut model).await;

    print_model_info(&model, json_output)
}

/// Prompts the user to select from multiple matching models.
fn select_model<'a>(matches: &[&'a UnifiedModel], query: &str) -> Result<&'a UnifiedModel> {
    let options: Vec<String> = matches
        .iter()
        .map(|m| {
            format!(
                "{} ({}, {}, {})",
                m.name,
                m.size_display(),
                m.quantization.as_str(),
                m.source.as_str(),
            )
        })
        .collect();

    let selection = Select::new(
        &format!("'{}' matches {} models:", query, matches.len()),
        options.clone(),
    )
    .prompt()?;

    let idx = options
        .iter()
        .position(|o| *o == selection)
        .expect("selection came from options");

    Ok(matches[idx])
}
