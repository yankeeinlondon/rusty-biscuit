//! Output formatting and presentation logic for CLI commands.

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::components::table::table::{
    Conditional, Table, TableCellContent, TableColumn,
};
use biscuit_terminal::components::table::types::ColumnType;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::Alignment;
use color_eyre::eyre::Result;
use model_citizen::{SortOrder, UnifiedModel, huggingface::SearchResult, sharing};

const HF_BASE: &str = "https://huggingface.co";

/// Prints a list of unified models, either as a terminal table or JSON.
pub fn print_models(models: &[UnifiedModel], json_output: bool, verbose: bool, runner_filter: Option<&str>) -> Result<()> {
    if json_output {
        let json = serde_json::to_string_pretty(&models)?;
        println!("{json}");
    } else if models.is_empty() {
        println!("No models found.");
        if let Some(filter) = runner_filter {
            println!("Filtered by runner: {filter}");
        }
    } else {
        let term = Terminal::default();

        let mut columns = vec![
            TableColumn::new(Prose::new("Name").fallback_render(&term)),
            TableColumn::new(Prose::new("Quant").fallback_render(&term))
                .with_alignment(Alignment::Center),
            TableColumn::new(Prose::new("Size").fallback_render(&term))
                .with_type(ColumnType::String)
                .with_alignment(Alignment::Right),
            TableColumn::new(Prose::new("Arch").fallback_render(&term))
                .with_alignment(Alignment::Center)
                .with_when(Conditional::WidthGreaterThan(89)),
            TableColumn::new(Prose::new("Source").fallback_render(&term))
                .with_alignment(Alignment::Center)
                .with_when(Conditional::WidthGreaterThan(69)),
        ];

        if verbose {
            columns.insert(
                1,
                TableColumn::new(Prose::new("Params").fallback_render(&term))
                    .with_alignment(Alignment::Center),
            );
            columns.insert(
                5,
                TableColumn::new(Prose::new("Format").fallback_render(&term))
                    .with_alignment(Alignment::Center),
            );
        }

        let mut table = Table::new().with_columns(columns).prefer_cursor_alignment();

        for m in models {
            let name_cell = if let Some(repo) = m.metadata.huggingface_repo.as_deref() {
                let name_link = format!("<a href=\"https://huggingface.co/{repo}\">{}</a>", m.name);
                TableCellContent::Text(Prose::new(name_link).fallback_render(&term))
            } else {
                TableCellContent::Text(m.name.clone())
            };
            let mut row: Vec<TableCellContent> = vec![
                name_cell,
                TableCellContent::Text(m.quantization.as_str().to_string()),
                TableCellContent::Text(m.size_display()),
                TableCellContent::Text(m.architecture.as_str().to_string()),
                TableCellContent::Text(m.source.display_name().to_string()),
            ];

            if verbose {
                row.insert(
                    1,
                    TableCellContent::Text(
                        m.metadata.parameters.as_deref().unwrap_or("-").to_string(),
                    ),
                );
                row.insert(5, TableCellContent::Text(m.format.as_str().to_string()));
            }

            table.add_row(row);
        }

        print!("{}", table.display(&term));
        println!("
Total: {} models", models.len());
    }

    Ok(())
}

/// Prints a single model's detailed information.
pub fn print_model_info(model: &UnifiedModel, json_output: bool) -> Result<()> {
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
        if let Some(stops) = &meta.stop
            && !stops.is_empty()
        {
            println!("  Stop tokens:   {}", stops.join(", "));
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
        if let Some(registry_path) = sharing::default_registry_path()
            && let Ok(share_registry) = sharing::ShareRegistry::load(&registry_path)
        {
            let shares = share_registry.get_shares(&model.path);
            if !shares.is_empty() {
                println!("
  Shared to:");
                for share in shares {
                    println!("    - {}", share.display());
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

/// Prints HuggingFace search results.
pub fn print_search_results(
    results: &[SearchResult],
    query: Option<&str>,
    sort: SortOrder,
    json_output: bool,
    verbose: bool,
) -> Result<()> {
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
                Prose::new(sort_header("Downloads", sort == SortOrder::Downloads))
                    .fallback_render(&term),
            )
            .with_type(ColumnType::Integer),
            TableColumn::new(
                Prose::new(sort_header("Likes", sort == SortOrder::Likes)).fallback_render(&term),
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
                    Prose::new(sort_header("Created", sort == SortOrder::Created))
                        .fallback_render(&term),
                )
                .with_alignment(Alignment::Center),
            );
        }
        if show_modified {
            columns.push(
                TableColumn::new(
                    Prose::new(sort_header("Modified", sort == SortOrder::Modified))
                        .fallback_render(&term),
                )
                .with_alignment(Alignment::Center),
            );
        }

        let mut table = Table::new().with_columns(columns).prefer_cursor_alignment();

        for r in results {
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
            "
Showing top {} results by {}. Use 'model download <repo>' to download.",
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

fn format_tags(result: &SearchResult) -> String {
    let mut seen = Vec::new();
    let mut parts = Vec::new();

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

fn format_date(iso: &Option<String>) -> TableCellContent {
    match iso {
        Some(s) => TableCellContent::Text(s.get(..10).unwrap_or(s).to_string()),
        None => TableCellContent::Text("—".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use model_citizen::{ModelFormat, ModelSource};

    #[test]
    fn test_model_json_snapshot() {
        let models = vec![UnifiedModel {
            id: "test-model-1".to_string(),
            name: "Llama-2-7b-test".to_string(),
            size_bytes: 4_000_000_000,
            architecture: model_citizen::ModelArchitecture::Llama,
            quantization: model_citizen::QuantizationType::Q4Km,
            source: ModelSource::LlamaCpp,
            format: ModelFormat::Gguf,
            metadata: Default::default(),
            path: "/path/to/llama.gguf".into(),
        }];

        let json = serde_json::to_string_pretty(&models).unwrap();
        
        // Remove the path from json string because it might differ across OS 
        // if path representation changes, though here it's static.
        insta::assert_snapshot!(json, @r###"
        [
          {
            "id": "test-model-1",
            "name": "Llama-2-7b-test",
            "size_bytes": 4000000000,
            "quantization": "Q4_KM",
            "architecture": "llama",
            "source": "llamacpp",
            "format": "gguf",
            "path": "/path/to/llama.gguf",
            "metadata": {}
          }
        ]
        "###);
    }
}
