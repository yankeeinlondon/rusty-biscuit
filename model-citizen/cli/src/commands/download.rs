//! Download command - downloads GGUF models from HuggingFace.

use color_eyre::eyre::{eyre, Result};
use indicatif::{ProgressBar, ProgressStyle};
use inquire::MultiSelect;
use model_citizen::huggingface::{GgufVariant, HuggingFaceClient};
use std::path::Path;

pub async fn run(
    repo: &str,
    variant: Option<&str>,
    output_dir: Option<&Path>,
) -> Result<()> {
    let client = HuggingFaceClient::new();

    // List available variants
    println!("Fetching variants for {}...", repo);
    let variants = client.list_variants(repo).await?;

    if variants.is_empty() {
        return Err(eyre!("No GGUF files found in repository: {}", repo));
    }

    // Select variants to download
    let selected = if let Some(variant_name) = variant {
        // Find specific variant
        let v = variants
            .into_iter()
            .find(|v| v.filename.contains(variant_name))
            .ok_or_else(|| eyre!("Variant '{}' not found", variant_name))?;
        vec![v]
    } else {
        // Interactive selection
        select_variants(variants)?
    };

    if selected.is_empty() {
        println!("No variants selected.");
        return Ok(());
    }

    // Determine output directory
    let dest_dir = output_dir
        .map(|p| p.to_path_buf())
        .or_else(model_citizen::sharing::default_shared_dir)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    std::fs::create_dir_all(&dest_dir)?;

    // Download each selected variant
    for variant in selected {
        download_variant(&client, repo, &variant, &dest_dir).await?;
    }

    Ok(())
}

fn select_variants(variants: Vec<GgufVariant>) -> Result<Vec<GgufVariant>> {
    let options: Vec<String> = variants
        .iter()
        .map(|v| {
            format!(
                "{} ({}, ~{} RAM)",
                v.filename,
                v.size_display(),
                v.estimated_ram()
            )
        })
        .collect();

    let selected = MultiSelect::new("Select variants to download:", options)
        .with_help_message("Use space to select, enter to confirm")
        .prompt()?;

    let selected_variants: Vec<GgufVariant> = variants
        .into_iter()
        .filter(|v| {
            selected.iter().any(|s| s.contains(&v.filename))
        })
        .collect();

    Ok(selected_variants)
}

async fn download_variant(
    client: &HuggingFaceClient,
    repo: &str,
    variant: &GgufVariant,
    dest_dir: &Path,
) -> Result<()> {
    let pb = ProgressBar::new(variant.size_bytes);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg}\n{spinner:.green} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")?
            .progress_chars("#>-"),
    );
    pb.set_message(format!("Downloading {}", variant.filename));

    let progress = |downloaded: u64, _total: u64| {
        pb.set_position(downloaded);
    };

    let result = client
        .download(repo, &variant.filename, dest_dir, progress)
        .await;

    match result {
        Ok(path) => {
            pb.finish_with_message(format!("Downloaded: {}", path.display()));
            Ok(())
        }
        Err(e) => {
            pb.abandon_with_message(format!("Failed: {}", e));
            // Cleanup temp files
            let _ = HuggingFaceClient::cleanup_temp_files(dest_dir);
            Err(e.into())
        }
    }
}
