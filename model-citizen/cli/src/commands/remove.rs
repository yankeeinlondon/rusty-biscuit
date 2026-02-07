//! Remove command - removes a model from one or all runners.

use color_eyre::eyre::{eyre, Result};
use inquire::Confirm;
use model_citizen::{
    scanner::{LlamaCppScanner, LmStudioScanner, OllamaScanner},
    sharing, Config, ModelRegistry, ModelSource, UnifiedModel,
};

pub async fn run(model_name: &str, runner_filter: Option<&str>, force: bool) -> Result<()> {
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

    // Find matching models
    let matching: Vec<&UnifiedModel> = models
        .iter()
        .filter(|m| {
            let name_match = m.name.to_lowercase().contains(&model_name.to_lowercase())
                || m.id.to_lowercase().contains(&model_name.to_lowercase());

            let runner_match = runner_filter
                .map(|r| m.source.as_str().eq_ignore_ascii_case(r))
                .unwrap_or(true);

            name_match && runner_match
        })
        .collect();

    if matching.is_empty() {
        return Err(eyre!("No models found matching: {}", model_name));
    }

    // Show what will be deleted
    println!("The following models will be removed:\n");
    for model in &matching {
        let symlink_info = if sharing::is_symlink(&model.path) {
            " (symlink)"
        } else {
            ""
        };
        println!(
            "  - {} [{}] {}{}",
            model.name,
            model.source.as_str(),
            model.path.display(),
            symlink_info
        );
    }
    println!();

    // Warn about dependent symlinks for originals
    for model in &matching {
        if !sharing::is_symlink(&model.path) {
            if let Some(registry_path) = sharing::default_registry_path() {
                if let Ok(share_registry) = sharing::ShareRegistry::load(&registry_path) {
                    let shares = share_registry.get_shares(&model.path);
                    if !shares.is_empty() {
                        println!(
                            "  Warning: {} has {} dependent symlinks that will break:",
                            model.name,
                            shares.len()
                        );
                        for share in shares {
                            println!("    - {}", share.display());
                        }
                        println!();
                    }
                }
            }
        }
    }

    // Confirm deletion
    if !force {
        let confirm = Confirm::new("Proceed with removal?")
            .with_default(false)
            .prompt()?;

        if !confirm {
            println!("Cancelled.");
            return Ok(());
        }
    }

    // Remove each model
    let mut removed = 0;
    let mut failed = 0;

    for model in matching {
        match remove_model(model).await {
            Ok(()) => {
                println!("Removed: {} [{}]", model.name, model.source.as_str());
                removed += 1;
            }
            Err(e) => {
                eprintln!(
                    "Failed to remove {} [{}]: {}",
                    model.name,
                    model.source.as_str(),
                    e
                );
                failed += 1;
            }
        }
    }

    println!("\nRemoved: {}, Failed: {}", removed, failed);

    Ok(())
}

async fn remove_model(model: &UnifiedModel) -> Result<()> {
    // Handle based on source and symlink status
    let is_link = sharing::is_symlink(&model.path);

    match model.source {
        ModelSource::Ollama => {
            // For Ollama, we need to delete the manifest and potentially blobs
            // For now, just remove the file/symlink
            if model.path.exists() || model.path.is_symlink() {
                std::fs::remove_file(&model.path)?;
            }
        }
        ModelSource::LmStudio | ModelSource::LlamaCpp => {
            // For LM Studio and Llama.cpp, just remove the file
            if model.path.exists() || model.path.is_symlink() {
                std::fs::remove_file(&model.path)?;
            }
        }
    }

    // Update share registry
    if is_link {
        if let Some(registry_path) = sharing::default_registry_path() {
            if let Ok(mut share_registry) = sharing::ShareRegistry::load(&registry_path) {
                share_registry.remove_share(&model.path);
                let _ = share_registry.save(&registry_path);
            }
        }
    }

    Ok(())
}
