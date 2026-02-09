//! Info command - shows detailed model information.

use color_eyre::eyre::{eyre, Result};
use inquire::Select;
use model_citizen::{
    scanner::{LlamaCppScanner, LmStudioScanner, OllamaScanner},
    sharing, Config, ModelRegistry, UnifiedModel,
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

    if json_output {
        let json = serde_json::to_string_pretty(&model)?;
        println!("{json}");
    } else {
        let meta = &model.metadata;

        println!("Model Information");
        println!("─────────────────");
        println!("  Name:          {}", model.name);
        if let Some(params) = &meta.parameters {
            println!("  Parameters:    {params}");
        }
        if let Some(ctx) = meta.context_length {
            println!("  Context:       {}", format_number(ctx));
        }
        println!("  Architecture:  {}", model.architecture.as_str());
        println!("  Quantization:  {}", model.quantization.as_str());
        println!("  Size:          {}", model.size_display());
        println!("  Source:        {}", model.source.as_str());
        println!("  Format:        {}", model.format.as_str());

        // Extended metadata section
        if let Some(families) = &meta.families {
            println!("  Families:      {}", families.join(", "));
        }
        if let Some(emb) = meta.embedding_length {
            println!("  Embedding:     {}", format_number(emb));
        }
        if let Some(heads) = meta.head_count {
            println!("  Heads:         {heads}");
        }
        if let Some(layers) = meta.layer_count {
            println!("  Layers:        {layers}");
        }
        if let Some(vision) = meta.vision {
            println!("  Vision:        {}", if vision { "Yes" } else { "No" });
        }
        if let Some(fc) = meta.function_calling {
            println!("  Func. Call:    {}", if fc { "Yes" } else { "No" });
        }
        if let Some(license) = &meta.license {
            println!("  License:       {license}");
        }
        if let Some(parent) = &meta.parent_model {
            println!("  Parent:        {parent}");
        }
        if let Some(publisher) = &meta.publisher {
            println!("  Publisher:     {publisher}");
        }
        if let Some(modified) = &meta.modified_at {
            println!("  Modified:      {modified}");
        }

        // Inference defaults
        if let Some(temp) = meta.temperature {
            println!("  Temperature:   {temp}");
        }
        if let Some(top_k) = meta.top_k {
            println!("  Top-K:         {top_k}");
        }
        if let Some(top_p) = meta.top_p {
            println!("  Top-P:         {top_p}");
        }
        if let Some(rp) = meta.repeat_penalty {
            println!("  Repeat Pen.:   {rp}");
        }
        if let Some(caps) = &meta.capabilities {
            println!("  Capabilities:  {}", caps.join(", "));
        }
        if let Some(stops) = &meta.stop {
            if !stops.is_empty() {
                println!("  Stop tokens:   {}", stops.join(", "));
            }
        }

        if let Some(repo) = &meta.huggingface_repo {
            println!("  HuggingFace:   https://huggingface.co/{repo}");
        } else {
            let base_name = model.name.split(':').next().unwrap_or(&model.name);
            let search_term = if let Some(publisher) = &meta.publisher {
                format!("{publisher} {base_name}")
            } else {
                base_name.to_string()
            };
            println!(
                "  HuggingFace:   https://huggingface.co/models?search={}",
                search_term
            );
        }

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

/// Formats a number with comma separators (e.g., 8192 -> "8,192").
fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            result.push(',');
        }
        result.push(c);
    }
    result
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
