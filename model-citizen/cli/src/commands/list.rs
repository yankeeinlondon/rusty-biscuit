//! List command - shows all models across runners.

use crate::output::print_models;
use color_eyre::eyre::Result;
use model_citizen::{
    Config, ModelRegistry,
    scanner::{LlamaCppScanner, LmStudioScanner, OllamaScanner},
};

pub async fn run(
    name_filter: Option<String>,
    runner_filter: Option<String>,
    json_output: bool,
    verbose: bool,
    sort_by_app: bool,
    sort_by_size: bool,
) -> Result<()> {
    let config = Config::load()?;
    let mut registry = ModelRegistry::new();

    // Add scanners based on filter
    let filter = runner_filter.as_deref().map(|s| s.to_lowercase());

    if (filter.is_none() || filter.as_deref() == Some("ollama")) && config.scanners.ollama.enabled {
        registry.add_scanner(OllamaScanner::new(&config));
    }

    if (filter.is_none() || filter.as_deref() == Some("lmstudio"))
        && config.scanners.lmstudio.enabled
    {
        registry.add_scanner(LmStudioScanner::new(&config));
    }

    if (filter.is_none() || filter.as_deref() == Some("llamacpp"))
        && config.scanners.llamacpp.enabled
    {
        registry.add_scanner(LlamaCppScanner::new(&config));
    }

    let mut models = registry.scan_all().await;

    if let Some(ref name_filter) = name_filter {
        let filter_lower = name_filter.to_lowercase();
        models.retain(|m| m.name.to_lowercase().contains(&filter_lower));
    }

    if sort_by_size {
        models.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));
    } else if sort_by_app {
        models.sort_by(|a, b| {
            a.source
                .as_str()
                .cmp(b.source.as_str())
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });
    } else {
        models.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    }

    print_models(&models, json_output, verbose, runner_filter.as_deref())
}
