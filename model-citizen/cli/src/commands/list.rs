//! List command - shows all models across runners.

use color_eyre::eyre::Result;
use model_citizen::{
    scanner::{LlamaCppScanner, LmStudioScanner, OllamaScanner},
    Config, ModelRegistry, UnifiedModel,
};
use tabled::{settings::Style, Table, Tabled};

#[derive(Tabled)]
struct ModelRow {
    #[tabled(rename = "Name")]
    name: String,
    #[tabled(rename = "Size")]
    size: String,
    #[tabled(rename = "Quant")]
    quantization: String,
    #[tabled(rename = "Arch")]
    architecture: String,
    #[tabled(rename = "Source")]
    source: String,
}

impl From<&UnifiedModel> for ModelRow {
    fn from(model: &UnifiedModel) -> Self {
        Self {
            name: model.name.clone(),
            size: model.size_display(),
            quantization: model.quantization.as_str().to_string(),
            architecture: model.architecture.as_str().to_string(),
            source: model.source.as_str().to_string(),
        }
    }
}

pub async fn run(runner_filter: Option<String>, json_output: bool) -> Result<()> {
    let config = Config::load()?;
    let mut registry = ModelRegistry::new();

    // Add scanners based on filter
    let filter = runner_filter.as_deref().map(|s| s.to_lowercase());

    if filter.is_none() || filter.as_deref() == Some("ollama") {
        if config.scanners.ollama.enabled {
            registry.add_scanner(OllamaScanner::new(&config));
        }
    }

    if filter.is_none() || filter.as_deref() == Some("lmstudio") {
        if config.scanners.lmstudio.enabled {
            registry.add_scanner(LmStudioScanner::new(&config));
        }
    }

    if filter.is_none() || filter.as_deref() == Some("llamacpp") {
        if config.scanners.llamacpp.enabled {
            registry.add_scanner(LlamaCppScanner::new(&config));
        }
    }

    let models = registry.scan_all().await;

    if json_output {
        let json = serde_json::to_string_pretty(&models)?;
        println!("{json}");
    } else if models.is_empty() {
        println!("No models found.");
        if let Some(filter) = runner_filter {
            println!("Filtered by runner: {filter}");
        }
    } else {
        let rows: Vec<ModelRow> = models.iter().map(ModelRow::from).collect();
        let mut table = Table::new(rows);
        table.with(Style::rounded());
        println!("{table}");
        println!("\nTotal: {} models", models.len());
    }

    Ok(())
}
