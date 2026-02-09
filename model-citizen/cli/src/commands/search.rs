//! Search command - searches HuggingFace for GGUF models.

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::components::table::table::{Table, TableCellContent, TableColumn};
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::Alignment;
use color_eyre::eyre::Result;
use model_citizen::huggingface::HuggingFaceClient;
use model_citizen::SortOrder;

const HF_BASE: &str = "https://huggingface.co";

pub async fn run(
    query: &str,
    limit: usize,
    sort: SortOrder,
    json_output: bool,
    verbose: bool,
) -> Result<()> {
    let client = HuggingFaceClient::new();

    println!("Searching for '{}'...", query);

    let results = client.search_models(query, limit, sort).await?;

    if results.is_empty() {
        println!("No models found matching '{}'", query);
        return Ok(());
    }

    if json_output {
        let json_results: Vec<_> = results
            .iter()
            .map(|r| {
                serde_json::json!({
                    "repo_id": r.repo_id,
                    "author": r.author,
                    "downloads": r.downloads,
                    "likes": r.likes,
                    "created_at": r.created_at,
                    "last_modified": r.last_modified
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&json_results)?);
    } else {
        let term = Terminal::default();

        let arrow = " <blue-500>↓</blue-500>";
        let sort_header = |label: &str, active: bool| -> String {
            if active {
                format!("{label}{arrow}")
            } else {
                label.to_string()
            }
        };

        let show_created = verbose || sort == SortOrder::Created;

        let mut columns = vec![
            TableColumn::new(Prose::new("Repository").fallback_render(&term)),
            TableColumn::new(
                Prose::new(&sort_header("Downloads", sort == SortOrder::Downloads))
                    .fallback_render(&term),
            )
            .with_alignment(Alignment::Right),
            TableColumn::new(
                Prose::new(&sort_header("Likes", sort == SortOrder::Likes))
                    .fallback_render(&term),
            )
            .with_alignment(Alignment::Right),
            TableColumn::new(Prose::new("G").fallback_render(&term))
                .with_alignment(Alignment::Center),
            TableColumn::new(Prose::new("ST").fallback_render(&term))
                .with_alignment(Alignment::Center),
        ];
        if show_created {
            columns.push(
                TableColumn::new(
                    Prose::new(&sort_header("Created", sort == SortOrder::Created))
                        .fallback_render(&term),
                )
                .with_alignment(Alignment::Center),
            );
        }
        columns.push(
            TableColumn::new(
                Prose::new(&sort_header("Modified", sort == SortOrder::Modified))
                    .fallback_render(&term),
            )
            .with_alignment(Alignment::Center),
        );

        let mut table = Table::new()
            .with_columns(columns)
            .prefer_cursor_alignment();

        for r in &results {
            let repo_link = format_repo_link(&r.repo_id);
            let check = TableCellContent::Text("\u{2713}".to_string());
            let blank = TableCellContent::Text(String::new());

            let mut row = vec![
                Prose::new(&repo_link).fallback_render(&term).into(),
                format_count(r.downloads),
                format_count(r.likes),
                if r.has_gguf() { check.clone() } else { blank.clone() },
                if r.has_safetensors() { check } else { blank },
            ];
            if show_created {
                row.push(format_date(&r.created_at));
            }
            row.push(format_date(&r.last_modified));

            table.add_row(row);
        }

        print!("{}", table.display(&term));
        println!(
            "\nShowing top {} results by {}. Use 'model download <repo>' to download.",
            results.len(),
            sort.display_label()
        );
    }

    Ok(())
}

/// Formats a repo ID as an OSC8 hyperlink with colored org/name segments.
fn format_repo_link(repo_id: &str) -> String {
    let url = format!("{HF_BASE}/{repo_id}");
    match repo_id.split_once('/') {
        Some((org, name)) => {
            format!("<a href=\"{url}\"><blue-600>{org}/</blue-600><blue-500>{name}</blue-500></a>")
        }
        None => format!("<a href=\"{url}\"><blue-500>{repo_id}</blue-500></a>"),
    }
}

/// Extracts the date portion (YYYY-MM-DD) from an ISO 8601 timestamp.
fn format_date(iso: &Option<String>) -> TableCellContent {
    match iso {
        Some(s) => TableCellContent::Text(s.get(..10).unwrap_or(s).to_string()),
        None => TableCellContent::Text("—".to_string()),
    }
}

fn format_count(n: u64) -> TableCellContent {
    let s = if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else {
        format_with_commas(n)
    };
    TableCellContent::Text(s)
}

/// Formats a number with comma separators (e.g., 46633 -> "46,633").
fn format_with_commas(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result
}
