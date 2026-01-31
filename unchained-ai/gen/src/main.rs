use std::collections::HashMap;
use std::path::PathBuf;

use clap::Parser;
use strum::IntoEnumIterator;
use tracing::{Level, info, warn};

use unchained_ai::api::openai_api::{get_api_keys, get_provider_models_from_api};
use unchained_ai::rigging::providers::Provider;

mod errors;
mod generator;
mod metadata_generator;
mod parsera;

use errors::GeneratorError;
use generator::ModelEnumGenerator;
use metadata_generator::MetadataGenerator;
use parsera::{ParseraModel, fetch_parsera_specs_with_retry, find_parsera_metadata};

#[derive(Parser)]
#[command(name = "gen-models")]
#[command(about = "Generate provider model enum files from API")]
#[command(version)]
struct Cli {
    /// Output directory for generated files
    #[arg(
        short,
        long,
        default_value = "unchained-ai/lib/src/rigging/providers/models"
    )]
    output: PathBuf,

    /// Specific providers to generate (comma-separated)
    #[arg(short, long)]
    providers: Option<String>,

    /// Skip specific providers (comma-separated)
    #[arg(short, long)]
    skip: Option<String>,

    /// Verbosity level (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Dry run - don't write files, just show what would be generated
    #[arg(long)]
    dry_run: bool,
}

/// Summary of the generation process.
#[derive(Default)]
struct GenerationSummary {
    succeeded: Vec<(Provider, usize)>,
    skipped: Vec<(Provider, String)>,
}

impl GenerationSummary {
    fn print(&self) {
        println!("\n=== Generation Summary ===\n");

        if !self.succeeded.is_empty() {
            println!("Succeeded:");
            for (provider, count) in &self.succeeded {
                println!("  {:?}: {} models", provider, count);
            }
        }

        if !self.skipped.is_empty() {
            println!("\nSkipped:");
            for (provider, reason) in &self.skipped {
                println!("  {:?}: {}", provider, reason);
            }
        }

        println!(
            "\nTotal: {} succeeded, {} skipped",
            self.succeeded.len(),
            self.skipped.len()
        );
    }
}

/// Write file atomically (write to temp, then rename).
fn write_atomic(path: &std::path::Path, content: &str) -> Result<(), GeneratorError> {
    let tmp_path = path.with_extension("rs.tmp");
    std::fs::write(&tmp_path, content)?;
    std::fs::rename(&tmp_path, path)?;
    Ok(())
}

/// Convert provider name to snake_case filename.
fn provider_to_filename(provider: Provider) -> String {
    format!("{:?}", provider).to_lowercase()
}

/// Result of processing a single provider.
struct ProviderResult {
    /// Number of models generated.
    model_count: usize,
    /// Model IDs for metadata collection.
    model_ids: Vec<String>,
}

/// Process a single provider.
async fn process_single_provider(
    provider: Provider,
    api_key: &str,
    output_dir: &std::path::Path,
    dry_run: bool,
) -> Result<ProviderResult, GeneratorError> {
    // Fetch models from API
    let models = get_provider_models_from_api(provider, api_key)
        .await
        .map_err(|e| GeneratorError::FetchFailed {
            provider: format!("{:?}", provider),
            reason: e.to_string(),
        })?;

    if models.is_empty() {
        return Err(GeneratorError::FetchFailed {
            provider: format!("{:?}", provider),
            reason: "No models returned from API".to_string(),
        });
    }

    // Collect model IDs for metadata
    let model_ids: Vec<String> = models.clone();

    // Generate code
    let provider_name = format!("{:?}", provider);
    let generator = ModelEnumGenerator::new(provider_name, models);
    let code = generator.generate();
    let model_count = generator.model_count();

    if dry_run {
        println!("\n--- {:?} ({} models) ---", provider, model_count);
        println!("{}", code);
    } else {
        // Write to file
        let filename = format!("{}.rs", provider_to_filename(provider));
        let output_path = output_dir.join(&filename);

        write_atomic(&output_path, &code).map_err(|e| GeneratorError::WriteFailed {
            path: output_path.display().to_string(),
            reason: e.to_string(),
        })?;

        info!("Wrote {} models to {}", model_count, output_path.display());
    }

    Ok(ProviderResult {
        model_count,
        model_ids,
    })
}

/// Result of processing all providers.
struct ProcessingResult {
    summary: GenerationSummary,
    all_model_ids: Vec<String>,
}

/// Process all providers.
async fn process_providers(
    providers: Vec<Provider>,
    api_keys: &std::collections::HashMap<Provider, String>,
    output_dir: &std::path::Path,
    dry_run: bool,
) -> ProcessingResult {
    let mut summary = GenerationSummary::default();
    let mut all_model_ids = Vec::new();

    for provider in providers {
        // Check if we have an API key
        let Some(api_key) = api_keys.get(&provider) else {
            summary
                .skipped
                .push((provider, "No API key configured".to_string()));
            continue;
        };

        // Skip local providers
        if provider.config().is_local {
            summary
                .skipped
                .push((provider, "Local provider".to_string()));
            continue;
        }

        match process_single_provider(provider, api_key, output_dir, dry_run).await {
            Ok(result) => {
                info!("Generated {} models for {:?}", result.model_count, provider);
                summary.succeeded.push((provider, result.model_count));
                all_model_ids.extend(result.model_ids);
            }
            Err(e) => {
                warn!("Skipping {:?}: {}", provider, e);
                summary.skipped.push((provider, e.to_string()));
            }
        }
    }

    ProcessingResult {
        summary,
        all_model_ids,
    }
}

/// Parse provider list from comma-separated string.
fn parse_provider_list(input: &str) -> Vec<Provider> {
    input
        .split(',')
        .filter_map(|s| {
            let name = s.trim().to_lowercase();
            Provider::iter().find(|p| format!("{:?}", p).to_lowercase() == name)
        })
        .collect()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    // Setup tracing
    let level = match cli.verbose {
        0 => Level::WARN,
        1 => Level::INFO,
        2 => Level::DEBUG,
        _ => Level::TRACE,
    };

    tracing_subscriber::fmt()
        .with_max_level(level)
        .with_target(false)
        .init();

    // Fetch Parsera specs once at startup (graceful degradation on failure)
    info!("Fetching model specs from Parsera API...");
    let parsera_index: HashMap<String, ParseraModel> = fetch_parsera_specs_with_retry().await;
    if parsera_index.is_empty() {
        warn!("Parsera API unavailable - metadata will be empty");
    } else {
        info!("Loaded {} model specs from Parsera", parsera_index.len());
    }

    // Get all available API keys
    let api_keys = get_api_keys();

    if api_keys.is_empty() && !cli.dry_run {
        eprintln!("No API keys configured. Set environment variables for providers.");
        eprintln!("Example: OPENAI_API_KEY, ANTHROPIC_API_KEY, GEMINI_API_KEY, etc.");
        std::process::exit(1);
    }

    // Determine which providers to process
    let mut providers: Vec<Provider> = if let Some(ref provider_list) = cli.providers {
        parse_provider_list(provider_list)
    } else {
        Provider::iter().collect()
    };

    // Apply skip filter
    if let Some(ref skip_list) = cli.skip {
        let skip_providers = parse_provider_list(skip_list);
        providers.retain(|p| !skip_providers.contains(p));
    }

    info!("Processing {} providers", providers.len());

    // Ensure output directory exists
    if !cli.dry_run {
        std::fs::create_dir_all(&cli.output)?;
    }

    // Process providers
    let result = process_providers(providers, &api_keys, &cli.output, cli.dry_run).await;

    // Generate metadata lookup table
    let mut metadata_gen = MetadataGenerator::new();
    let mut matched_count = 0;

    for model_id in &result.all_model_ids {
        let parsera_data = find_parsera_metadata(model_id, &parsera_index);
        if parsera_data.is_some() {
            matched_count += 1;
        }
        metadata_gen.register(model_id.clone(), parsera_data.cloned());
    }

    info!(
        "Matched {}/{} models with Parsera metadata",
        matched_count,
        result.all_model_ids.len()
    );

    // Write metadata file
    if !cli.dry_run {
        let metadata_code = metadata_gen.generate();
        let metadata_path = cli.output.join("metadata_generated.rs");
        write_atomic(&metadata_path, &metadata_code)?;
        info!("Wrote metadata to {}", metadata_path.display());
    } else {
        println!("\n--- Metadata ({} entries) ---", matched_count);
        println!("{}", metadata_gen.generate());
    }

    result.summary.print();

    Ok(())
}
