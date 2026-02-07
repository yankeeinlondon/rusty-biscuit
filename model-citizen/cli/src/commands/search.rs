//! Search command - searches HuggingFace for GGUF models.

use color_eyre::eyre::Result;
use model_citizen::huggingface::HuggingFaceClient;
use tabled::{settings::Style, Table, Tabled};

#[derive(Tabled)]
struct SearchRow {
    #[tabled(rename = "Repository")]
    repo: String,
    #[tabled(rename = "Author")]
    author: String,
    #[tabled(rename = "Downloads")]
    downloads: String,
    #[tabled(rename = "Likes")]
    likes: String,
}

pub async fn run(query: &str, limit: usize, json_output: bool) -> Result<()> {
    let client = HuggingFaceClient::new();

    println!("Searching for '{}'...", query);

    let results = client.search_models(query, limit).await?;

    if results.is_empty() {
        println!("No models found matching '{}'", query);
        return Ok(());
    }

    if json_output {
        // Create a serializable version
        let json_results: Vec<_> = results
            .iter()
            .map(|r| {
                serde_json::json!({
                    "repo_id": r.repo_id,
                    "author": r.author,
                    "downloads": r.downloads,
                    "likes": r.likes
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&json_results)?);
    } else {
        let rows: Vec<SearchRow> = results
            .iter()
            .map(|r| SearchRow {
                repo: r.repo_id.clone(),
                author: r.author.clone().unwrap_or_default(),
                downloads: format_count(r.downloads),
                likes: format_count(r.likes),
            })
            .collect();

        let mut table = Table::new(rows);
        table.with(Style::rounded());
        println!("{table}");
        println!("\nFound {} results. Use 'model download <repo>' to download.", results.len());
    }

    Ok(())
}

fn format_count(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}
