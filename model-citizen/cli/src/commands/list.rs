//! List command - shows all models across runners.

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::components::table::table::{
    Conditional,
    Table,
    TableCellContent,
    TableColumn,
};
use biscuit_terminal::components::table::types::ColumnType;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::Alignment;
use color_eyre::eyre::Result;
use model_citizen::{
    scanner::{LlamaCppScanner, LmStudioScanner, OllamaScanner},
    Config, ModelRegistry,
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

    let mut models = registry.scan_all().await;

    if let Some(ref filter) = name_filter {
        let filter_lower = filter.to_lowercase();
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
            // Insert Params after Name and Format before Source
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

        let mut table = Table::new()
            .with_columns(columns)
            .prefer_cursor_alignment();

        for m in &models {
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
                        m.metadata
                            .parameters
                            .as_deref()
                            .unwrap_or("-")
                            .to_string(),
                    ),
                );
                row.insert(5, TableCellContent::Text(m.format.as_str().to_string()));
            }

            table.add_row(row);
        }

        print!("{}", table.display(&term));
        println!("\nTotal: {} models", models.len());
    }

    Ok(())
}
