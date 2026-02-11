//! Search command - searches HuggingFace for GGUF models.

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::components::table::table::{Table, TableCellContent, TableColumn};
use biscuit_terminal::components::table::types::ColumnType;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::Alignment;
use color_eyre::eyre::Result;
use model_citizen::SortOrder;
use model_citizen::huggingface::HuggingFaceClient;

const HF_BASE: &str = "https://huggingface.co";

pub async fn run(
    query: Option<&str>,
    limit: usize,
    sort: SortOrder,
    json_output: bool,
    verbose: bool,
) -> Result<()> {
    let client = HuggingFaceClient::new();

    match query {
        Some(_) => println!(),
        None => println!("Browsing top models by {}...", sort.display_label()),
    }

    let results = client.search_models(query, limit, sort).await?;

    if results.is_empty() {
        match query {
            Some(q) => println!("No models found matching '{q}'"),
            None => println!("No models found"),
        }
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
                    "last_modified": r.last_modified,
                    "tags": r.tags,
                    "pipeline_tag": r.pipeline_tag
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
        let show_modified = verbose || sort == SortOrder::Modified;

        let mut columns = vec![
            TableColumn::new(Prose::new("Repository").fallback_render(&term)),
            TableColumn::new(
                Prose::new(&sort_header("Downloads", sort == SortOrder::Downloads))
                    .fallback_render(&term),
            )
            .with_type(ColumnType::Integer),
            TableColumn::new(
                Prose::new(&sort_header("Likes", sort == SortOrder::Likes)).fallback_render(&term),
            )
            .with_type(ColumnType::Integer),
            TableColumn::new(Prose::new("G").fallback_render(&term))
                .with_alignment(Alignment::Center),
            TableColumn::new(Prose::new("ST").fallback_render(&term))
                .with_alignment(Alignment::Center),
            TableColumn::new(Prose::new("Tags").fallback_render(&term)),
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
        if show_modified {
            columns.push(
                TableColumn::new(
                    Prose::new(&sort_header("Modified", sort == SortOrder::Modified))
                        .fallback_render(&term),
                )
                .with_alignment(Alignment::Center),
            );
        }

        let mut table = Table::new().with_columns(columns).prefer_cursor_alignment();

        for r in &results {
            let repo_link = format_repo_link(&r.repo_id);
            let check = TableCellContent::Text("\u{2713}".to_string());
            let blank = TableCellContent::Text(String::new());
            let tags_markup = format_tags(r);
            let neither = !r.has_gguf() && !r.has_safetensors();
            let dot = Prose::new("<red-500>\u{23fa}</red-500>").fallback_render(&term);

            let mut row = vec![
                Prose::new(&repo_link).fallback_render(&term).into(),
                TableCellContent::Integer(r.downloads as i64),
                TableCellContent::Integer(r.likes as i64),
                if r.has_gguf() {
                    check.clone()
                } else if neither {
                    dot.clone().into()
                } else {
                    blank.clone()
                },
                if r.has_safetensors() {
                    check
                } else if neither {
                    dot.into()
                } else {
                    blank
                },
                Prose::new(&tags_markup).fallback_render(&term).into(),
            ];
            if show_created {
                row.push(format_date(&r.created_at));
            }
            if show_modified {
                row.push(format_date(&r.last_modified));
            }

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

/// Tag definitions: (api_tag, display_label, bg_color, fg_color).
///
/// Uses Tailwind block tags for both background (`<bg-*>`) and foreground (`<*>`).
const TAG_RULES: &[(&str, &str, &str, &str)] = &[
    ("image-text-to-text", "image input", "bg-blue-700", "white"),
    ("image-to-text", "image input", "bg-blue-700", "white"),
    ("text-to-image", "video output", "bg-violet-700", "white"),
    ("text-to-video", "video output", "bg-violet-700", "white"),
    ("mlx", "mlx", "bg-emerald-700", "white"),
    ("text-to-speech", "tts", "bg-teal-700", "white"),
    ("function-calling", "fn", "bg-amber-600", "white"),
    ("tool-use", "tool", "bg-rose-700", "white"),
];

/// Builds a Prose markup string of colored tag badges for a search result.
fn format_tags(result: &model_citizen::huggingface::SearchResult) -> String {
    let mut seen = Vec::new();
    let mut parts = Vec::new();

    // Check both tags and pipeline_tag for matches.
    let all_tags: Vec<&str> = result
        .tags
        .iter()
        .map(String::as_str)
        .chain(result.pipeline_tag.as_deref())
        .collect();

    for &(api_tag, label, bg, fg) in TAG_RULES {
        if seen.contains(&label) {
            continue;
        }
        if all_tags.iter().any(|t| t.eq_ignore_ascii_case(api_tag)) {
            parts.push(format!("<{bg}><{fg}> {label} </{fg}></{bg}>"));
            seen.push(label);
        }
    }

    parts.join(" ")
}

/// Extracts the date portion (YYYY-MM-DD) from an ISO 8601 timestamp.
fn format_date(iso: &Option<String>) -> TableCellContent {
    match iso {
        Some(s) => TableCellContent::Text(s.get(..10).unwrap_or(s).to_string()),
        None => TableCellContent::Text("—".to_string()),
    }
}
