use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;

use clap::Parser;
use strum::IntoEnumIterator;
use tracing::{Level, info, warn};

use unchained_ai::api::openai_api::{
    ProviderModelEntry, get_api_keys, get_provider_models_from_api,
};
use unchained_ai::rigging::providers::Provider;

mod errors;
mod generator;
mod metadata_generator;
mod models_dev;
mod provider_metadata;

use errors::GeneratorError;
use generator::ModelEnumGenerator;
use metadata_generator::MetadataGenerator;
use models_dev::{
    fetch_models_dev_with_retry, find_models_dev_metadata, models_dev_provider_key,
    models_dev_to_metadata, validate_models_dev_index, ModelsDevError, ModelsDevIndex,
};

#[derive(Parser)]
#[command(name = "gen-models")]
#[command(about = "Generate provider model enum files from API")]
#[command(version)]
struct Cli {
    /// Output directory for generated files.
    ///
    /// Defaults to `<unchained-ai-gen crate>/../lib/src/rigging/providers/models`
    /// resolved against `CARGO_MANIFEST_DIR`, so the generator targets the real
    /// `unchained-ai/lib` regardless of the caller's working directory.
    #[arg(short, long)]
    output: Option<PathBuf>,

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

/// Resolve the default output directory relative to the gen crate's manifest,
/// not the caller's current working directory.
fn default_output_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("lib")
        .join("src")
        .join("rigging")
        .join("providers")
        .join("models")
}

/// Summary of the generation process.
#[derive(Default)]
struct GenerationSummary {
    succeeded: Vec<(Provider, usize)>,
    skipped: Vec<(Provider, String)>,
    metadata_coverage: Vec<(Provider, usize, usize)>,
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

        if !self.metadata_coverage.is_empty() {
            println!("\nmodels.dev match coverage:");
            for (provider, matched, total) in &self.metadata_coverage {
                println!("  {:?}: {}/{} matched", provider, matched, total);
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

/// Read the current generated model IDs for a provider from its existing enum file.
fn existing_model_ids(provider: Provider, output_dir: &std::path::Path) -> Vec<String> {
    let path = output_dir.join(format!("{}.rs", provider_to_filename(provider)));
    let Ok(contents) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };

    contents
        .lines()
        .filter_map(|line| {
            line.trim()
                .strip_prefix("/// Model: `")
                .and_then(|rest| rest.strip_suffix('`'))
                .map(ToString::to_string)
        })
        .collect()
}

/// Result of processing a single provider.
struct ProviderResult {
    /// Number of models generated.
    model_count: usize,
    /// Model IDs for metadata collection.
    model_ids: Vec<String>,
    /// Raw provider-native metadata keyed by unprefixed model ID.
    raw_metadata: HashMap<String, serde_json::Value>,
}

/// Process a single provider.
async fn process_single_provider(
    provider: Provider,
    api_key: &str,
    output_dir: &std::path::Path,
    dry_run: bool,
) -> Result<ProviderResult, GeneratorError> {
    let entries: Vec<ProviderModelEntry> = get_provider_models_from_api(provider, api_key)
        .await
        .map_err(|e| GeneratorError::FetchFailed {
            provider: format!("{:?}", provider),
            reason: e.to_string(),
        })?;

    if entries.is_empty() {
        return Err(GeneratorError::FetchFailed {
            provider: format!("{:?}", provider),
            reason: "No models returned from API".to_string(),
        });
    }

    // Collect model IDs and raw metadata
    let model_ids: Vec<String> = entries.iter().map(|e| e.id.clone()).collect();
    let raw_metadata: HashMap<String, serde_json::Value> = entries
        .into_iter()
        .filter_map(|e| e.raw_metadata.map(|v| (e.id, v)))
        .collect();

    // Generate code
    let provider_name = format!("{:?}", provider);
    let generator = ModelEnumGenerator::new(provider_name, model_ids.clone());
    let code = generator.generate();
    let model_count = generator.model_count();

    if dry_run {
        println!("\n--- {:?} ({} models) ---", provider, model_count);
        println!("{}", code);
    } else {
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
        raw_metadata,
    })
}

/// Result of processing all providers.
struct ProcessingResult {
    summary: GenerationSummary,
    provider_model_ids: Vec<(Provider, Vec<String>)>,
    /// Raw provider-native metadata from APIs that return rich data,
    /// keyed by model ID. Currently only OpenRouter provides rich metadata.
    provider_native_raw: HashMap<String, serde_json::Value>,
}

/// Process all providers.
async fn process_providers(
    providers: Vec<Provider>,
    api_keys: &std::collections::HashMap<Provider, String>,
    output_dir: &std::path::Path,
    dry_run: bool,
) -> ProcessingResult {
    let mut summary = GenerationSummary::default();
    let mut provider_model_ids = Vec::new();
    let mut provider_native_raw = HashMap::new();

    for provider in providers {
        let Some(api_key) = api_keys.get(&provider) else {
            provider_model_ids.push((provider, existing_model_ids(provider, output_dir)));
            summary
                .skipped
                .push((provider, "No API key configured".to_string()));
            continue;
        };

        if provider.config().is_local {
            provider_model_ids.push((provider, existing_model_ids(provider, output_dir)));
            summary
                .skipped
                .push((provider, "Local provider".to_string()));
            continue;
        }

        match process_single_provider(provider, api_key, output_dir, dry_run).await {
            Ok(result) => {
                info!("Generated {} models for {:?}", result.model_count, provider);
                summary.succeeded.push((provider, result.model_count));
                provider_model_ids.push((provider, result.model_ids));
                if provider == Provider::OpenRouter {
                    provider_native_raw.extend(result.raw_metadata);
                }
            }
            Err(e) => {
                warn!("Skipping {:?}: {}", provider, e);
                provider_model_ids.push((provider, existing_model_ids(provider, output_dir)));
                summary.skipped.push((provider, e.to_string()));
            }
        }
    }

    ProcessingResult {
        summary,
        provider_model_ids,
        provider_native_raw,
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

async fn load_models_dev_index_for_cli<F, Fut>(
    cli: &Cli,
    fetch_models_dev: F,
) -> Result<ModelsDevIndex, GeneratorError>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<ModelsDevIndex, ModelsDevError>>,
{
    if cli.dry_run {
        info!("Dry run enabled; generated files will not be written");
    }

    info!("Fetching model specs from models.dev...");
    let index = fetch_models_dev()
        .await
        .map_err(|e| GeneratorError::FetchFailed {
            provider: "models.dev".to_string(),
            reason: e.to_string(),
        })?;
    validate_models_dev_index(&index).map_err(|e| GeneratorError::FetchFailed {
        provider: "models.dev".to_string(),
        reason: e.to_string(),
    })?;
    info!("Loaded {} provider buckets from models.dev", index.len());
    Ok(index)
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

    let models_dev_index =
        load_models_dev_index_for_cli(&cli, fetch_models_dev_with_retry).await?;

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

    let output_dir = cli.output.clone().unwrap_or_else(default_output_dir);
    info!("Output directory: {}", output_dir.display());

    // Ensure output directory exists
    if !cli.dry_run {
        std::fs::create_dir_all(&output_dir)?;
    }

    // Process providers
    let result = process_providers(providers, &api_keys, &output_dir, cli.dry_run).await;

    // Generate metadata lookup table with merge logic
    let mut metadata_gen = MetadataGenerator::new();
    let mut matched_count = 0;
    let mut total_model_count = 0;
    let mut summary = result.summary;

    for (provider, model_ids) in &result.provider_model_ids {
        let models_dev_bucket =
            models_dev_provider_key(*provider).and_then(|key| models_dev_index.get(key));
        let mut provider_matched = 0;

        for model_id in model_ids {
            let models_dev_metadata = models_dev_bucket
                .and_then(|bucket| find_models_dev_metadata(model_id, *provider, bucket))
                .map(models_dev_to_metadata)
                .transpose()
                .map_err(|e| GeneratorError::FetchFailed {
                    provider: "models.dev".to_string(),
                    reason: format!("{provider:?}/{model_id}: {e}"),
                })?;

            let provider_native = if *provider == Provider::OpenRouter {
                result
                    .provider_native_raw
                    .get(model_id)
                    .and_then(|v| provider_metadata::parse_provider_metadata(Provider::OpenRouter, v))
            } else {
                None
            };

            let merged = MetadataGenerator::merge_metadata(provider_native, models_dev_metadata);

            if merged.is_some() {
                matched_count += 1;
                provider_matched += 1;
            }

            metadata_gen.register(model_id.clone(), merged);
        }

        total_model_count += model_ids.len();
        summary
            .metadata_coverage
            .push((*provider, provider_matched, model_ids.len()));
    }

    info!(
        "Matched {}/{} models with metadata",
        matched_count,
        total_model_count
    );

    // Write compact metadata file
    let metadata_code = metadata_gen.generate();
    if !cli.dry_run {
        let metadata_path = output_dir.join("metadata_generated.rs");
        write_atomic(&metadata_path, &metadata_code)?;
        info!("Wrote metadata to {}", metadata_path.display());
    } else {
        println!("\n--- Metadata ({} entries) ---", matched_count);
        println!("{}", metadata_code);
    }

    summary.print();

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn test_existing_model_ids_reads_generated_model_comments() {
        let dir = std::env::temp_dir().join(format!(
            "unchained-ai-gen-test-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let path = dir.join("xai.rs");
        std::fs::write(
            &path,
            r#"
pub enum ProviderModelXai {
    /// Model: `grok-4.3`
    Grok__4_3,
    /// Custom model ID not in the predefined list.
    Bespoke(String),
}
"#,
        )
        .expect("write generated model file");

        let ids = existing_model_ids(Provider::Xai, &dir);

        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_dir(dir);

        assert_eq!(ids, vec!["grok-4.3".to_string()]);
    }

    #[tokio::test]
    async fn test_dry_run_generation_validates_models_dev_source() {
        let cli = Cli {
            output: None,
            providers: None,
            skip: None,
            verbose: 0,
            dry_run: true,
        };
        let fetched = Cell::new(false);

        let err = load_models_dev_index_for_cli(&cli, || {
            fetched.set(true);
            async { Ok(ModelsDevIndex::new()) }
        })
        .await
        .expect_err("dry-run should still reject degraded models.dev data");

        assert!(fetched.get(), "dry-run should fetch models.dev data");
        let message = err.to_string();
        assert!(
            message.contains("models.dev response is implausibly thin"),
            "unexpected error: {message}"
        );
    }
}
