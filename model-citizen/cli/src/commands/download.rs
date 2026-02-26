//! Download command - search and download GGUF models from HuggingFace.

use color_eyre::eyre::{Result, eyre};
use indicatif::{ProgressBar, ProgressStyle};
use inquire::{MultiSelect, Select};
use model_citizen::SortOrder;
use model_citizen::huggingface::{GgufVariant, HuggingFaceClient, SearchResult};
use std::path::Path;

pub async fn run(
    query: Option<&str>,
    limit: usize,
    sort: SortOrder,
    verbose: bool,
    output_dir: Option<&Path>,
    remove_partial: bool,
) -> Result<()> {
    let client = HuggingFaceClient::new();

    // If query contains `/`, treat as a direct repo ID
    let repo_id = match query {
        Some(q) if q.contains('/') => q.to_string(),
        _ => search_and_select(&client, query, limit, sort, verbose).await?,
    };

    // List available GGUF variants
    println!("Fetching variants for {}...", repo_id);
    let variants = client.list_variants(&repo_id).await?;

    if variants.is_empty() {
        return Err(eyre!("No GGUF files found in repository: {}", repo_id));
    }

    let selected = select_variants(variants)?;

    if selected.is_empty() {
        println!("No variants selected.");
        return Ok(());
    }

    // Determine output directory: --output flag > MODELS_DIR env > default shared dir > cwd
    let dest_dir = output_dir
        .map(|p| p.to_path_buf())
        .or_else(|| {
            std::env::var("MODELS_DIR")
                .ok()
                .map(std::path::PathBuf::from)
        })
        .or_else(model_citizen::sharing::default_shared_dir)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    std::fs::create_dir_all(&dest_dir)?;

    for variant in selected {
        download_variant(&client, &repo_id, &variant, &dest_dir, remove_partial).await?;
    }

    Ok(())
}

/// Searches HuggingFace and lets the user pick a repository interactively.
async fn search_and_select(
    client: &HuggingFaceClient,
    query: Option<&str>,
    limit: usize,
    sort: SortOrder,
    _verbose: bool,
) -> Result<String> {
    match query {
        Some(_) => println!(),
        None => println!("Browsing top models by {}...", sort.display_label()),
    }

    let results: Vec<SearchResult> = client.search_gguf_models(query, limit, sort).await?;

    if results.is_empty() {
        match query {
            Some(q) => return Err(eyre!("No GGUF models found matching '{q}'")),
            None => return Err(eyre!("No GGUF models found")),
        }
    }

    let options: Vec<String> = results.iter().map(format_result_option).collect();

    let selection = Select::new("Select a model to download:", options.clone())
        .with_help_message("Use arrow keys to navigate, enter to select")
        .prompt()?;

    let idx = options
        .iter()
        .position(|o| *o == selection)
        .ok_or_else(|| eyre!("Failed to resolve selected model option"))?;

    Ok(results[idx].repo_id.clone())
}

/// Formats a search result for display in the selection prompt.
fn format_result_option(r: &SearchResult) -> String {
    format!(
        "{} ({} downloads, {} likes)",
        r.repo_id,
        format_count(r.downloads),
        format_count(r.likes)
    )
}

/// Formats a count with comma separators (e.g., 1234567 -> "1,234,567").
fn format_count(n: u64) -> String {
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
        .filter(|v| selected.iter().any(|s| s.contains(&v.filename)))
        .collect();

    Ok(selected_variants)
}

async fn download_variant(
    client: &HuggingFaceClient,
    repo: &str,
    variant: &GgufVariant,
    dest_dir: &Path,
    remove_partial: bool,
) -> Result<()> {
    if !handle_partial_download(dest_dir, &variant.filename, remove_partial)? {
        println!("Skipped {}", variant.filename);
        return Ok(());
    }

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
            Err(e.into())
        }
    }
}

fn handle_partial_download(dest_dir: &Path, filename: &str, remove_partial: bool) -> Result<bool> {
    let Some(partial_size) = HuggingFaceClient::partial_download_size(dest_dir, filename)? else {
        return Ok(true);
    };

    if partial_size == 0 {
        return Ok(true);
    }

    let tmp_path = HuggingFaceClient::temp_download_path(dest_dir, filename);

    if remove_partial {
        std::fs::remove_file(&tmp_path)?;
        println!(
            "Removed partial download for {} ({}). Starting fresh.",
            filename,
            format_bytes(partial_size)
        );
        return Ok(true);
    }

    let options = vec![
        "Resume partial download (Recommended)".to_string(),
        "Remove partial download and restart".to_string(),
        "Skip this file".to_string(),
    ];

    let selection = Select::new(
        &format!(
            "Found partial download for {} ({}).",
            filename,
            format_bytes(partial_size)
        ),
        options.clone(),
    )
    .with_help_message("Choose how to continue")
    .prompt()?;

    let idx = options
        .iter()
        .position(|option| option == &selection)
        .ok_or_else(|| eyre!("Failed to resolve selected partial-download option"))?;

    match idx {
        0 => Ok(true),
        1 => {
            std::fs::remove_file(&tmp_path)?;
            Ok(true)
        }
        _ => Ok(false),
    }
}

fn format_bytes(bytes: u64) -> String {
    const GB: u64 = 1024 * 1024 * 1024;
    const MB: u64 = 1024 * 1024;
    const KB: u64 = 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_bytes_displays_expected_units() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1_536), "1.5 KB");
        assert_eq!(format_bytes(5_242_880), "5.0 MB");
    }
}
