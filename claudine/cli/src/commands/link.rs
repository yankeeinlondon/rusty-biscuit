use clap::Args;
use color_eyre::eyre::Result;

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::components::table::table::{Table, TableCellContent, TableColumn};
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::{Alignment, Margin};
use claudine::events::Provider;
use claudine::linking::{
    self, ALL_PROVIDERS, LinkScope, LinkableResource, ResourceFormat, SupportLevel,
    all_capabilities, capabilities_for,
};
use sniff::programs::{InstalledAiClients, enums::AiCli};

use crate::log;

/// Arguments for the link subcommand.
#[derive(Args)]
pub struct LinkArgs {
    /// Optional provider name for detailed view (fuzzy matching supported)
    #[arg(value_name = "PROVIDER")]
    pub provider_arg: Option<String>,

    /// Show provider resource support matrix
    #[arg(long)]
    pub support: bool,

    /// Filter to a specific skill name.
    #[arg(long)]
    pub filter: Option<String>,

    /// Show detailed output.
    #[arg(long)]
    pub detailed: bool,
}

/// Map a claudine `Provider` to the corresponding sniff `AiCli` variant.
fn provider_to_ai_cli(provider: Provider) -> AiCli {
    match provider {
        Provider::Claude => AiCli::Claude,
        Provider::Codex => AiCli::Codex,
        Provider::Gemini => AiCli::GeminiCli,
        Provider::Goose => AiCli::Goose,
        Provider::KimiCode => AiCli::KimiCli,
        Provider::OpenCode => AiCli::Opencode,
        Provider::QwenCode => AiCli::QwenCli,
    }
}

fn bool_indicator(value: bool) -> TableCellContent {
    if value {
        "\u{2705}".into()
    } else {
        "\u{274C}".into()
    }
}

/// Fuzzy match a user input string to a provider.
fn fuzzy_match_provider(input: &str) -> Option<Provider> {
    let input_lower = input.to_lowercase();

    // Try exact match first
    for provider in ALL_PROVIDERS {
        let display = provider.to_string().to_lowercase();
        let slug = provider.as_slug().to_lowercase();
        if display == input_lower || slug == input_lower {
            return Some(provider);
        }
    }

    // Try prefix match
    for provider in ALL_PROVIDERS {
        let display = provider.to_string().to_lowercase();
        let slug = provider.as_slug().to_lowercase();
        if display.starts_with(&input_lower) || slug.starts_with(&input_lower) {
            return Some(provider);
        }
    }

    // Try contains match
    for provider in ALL_PROVIDERS {
        let display = provider.to_string().to_lowercase();
        let slug = provider.as_slug().to_lowercase();
        if display.contains(&input_lower) || slug.contains(&input_lower) {
            return Some(provider);
        }
    }

    None
}

/// Link skills and commands across providers.
pub fn run(args: LinkArgs) -> Result<()> {
    // Handle --support flag
    if args.support {
        // If a provider is specified with --support, show detailed view
        if let Some(ref provider_input) = args.provider_arg {
            match fuzzy_match_provider(provider_input) {
                Some(provider) => return run_provider_detail(provider),
                None => {
                    let available: Vec<String> =
                        ALL_PROVIDERS.iter().map(|p| p.to_string()).collect();
                    log::error(&format!(
                        "Unknown provider '{}'. Available: {}",
                        provider_input,
                        available.join(", ")
                    ));
                    return Ok(());
                }
            }
        }
        return run_support();
    }

    // If just a provider name is given (no --support), show detailed view
    if let Some(ref provider_input) = args.provider_arg {
        match fuzzy_match_provider(provider_input) {
            Some(provider) => return run_provider_detail(provider),
            None => {
                let available: Vec<String> = ALL_PROVIDERS.iter().map(|p| p.to_string()).collect();
                log::error(&format!(
                    "Unknown provider '{}'. Available: {}",
                    provider_input,
                    available.join(", ")
                ));
                return Ok(());
            }
        }
    }

    // Report current link state (read-only)
    let scope = LinkScope::User;
    let filter = args.filter.as_deref();

    let report = linking::link_skills(scope, filter, true)?;

    // LinkReport implements Display via format_report()
    log::data(&format!("{report}"));

    // Show hints about --support
    log::data("");
    let hint = Prose::new(
        "{{dim}}- Use {{blue}}{{bold}}--support{{reset}}{{dim}} to see which resources each provider supports{{reset}}",
    );
    log::data(&format!(" {}", hint.render(Some(100))));

    Ok(())
}

/// Show provider resource support matrix.
fn run_support() -> Result<()> {
    let term = Terminal::new();
    let clients = InstalledAiClients::new();

    // Build columns: Provider, Installed, then one per resource type
    let mut columns = vec![
        TableColumn::new("Provider"),
        TableColumn::new("∃"), // existence symbol for installed
    ];

    for resource in LinkableResource::ALL {
        columns.push(TableColumn::new(resource.abbrev()).with_alignment(Alignment::Center));
    }

    let mut table = Table::new().with_columns(columns).prefer_cursor_alignment();
    table.layout_mut().left_margin = Margin::Chars(1);

    for caps in all_capabilities() {
        let provider = caps.provider;
        let installed = clients.is_installed(provider_to_ai_cli(provider));

        // Create OSC8 hyperlink for provider name
        let provider_link = format!(r#"<a href="{}">{}</a>"#, provider.docs_url(), provider);
        let provider_cell: TableCellContent = Prose::new(provider_link).render(None).into();

        let mut row: Vec<TableCellContent> = vec![provider_cell, bool_indicator(installed)];

        // Add support indicator for each resource type
        for resource in LinkableResource::ALL {
            let support = caps.support_for(resource);
            let cell = format_support_cell(support.level, support.format);
            row.push(cell);
        }

        table.add_row(row);
    }

    let rendered = table.fallback_render(&term);
    log::data(&format!("\n{}", rendered));

    // Show legend
    log::data("");
    let legend = Prose::new(
        "{{dim}}Legend: {{reset}}\u{2705}{{dim}} = full support, {{reset}}\u{2699}\u{fe0f}{{dim}} = custom format, {{reset}}\u{25cb}{{dim}} = limited/built-in only, {{reset}}\u{274c}{{dim}} = not supported{{reset}}",
    );
    log::data(&format!(" {}\n", legend.fallback_render(&term)));

    // Show hints
    let hints = [
        "{{dim}}- Use {{blue}}{{bold}}claudine link --support <provider>{{reset}}{{dim}} to see detailed capabilities{{reset}}",
        "{{dim}}- Providers with {{green}}custom format{{reset}}{{dim}} may need format conversion for linking{{reset}}",
    ];
    for hint in hints {
        log::data(&format!(" {}", Prose::new(hint).fallback_render(&term)));
    }

    Ok(())
}

/// Format a support level cell with optional format info.
fn format_support_cell(level: SupportLevel, format: Option<ResourceFormat>) -> TableCellContent {
    match level {
        SupportLevel::Full => level.indicator().into(),
        SupportLevel::CustomFormat => {
            if let Some(fmt) = format {
                format!("{} {}", level.indicator(), fmt.abbrev()).into()
            } else {
                level.indicator().into()
            }
        }
        SupportLevel::Limited | SupportLevel::None => level.indicator().into(),
    }
}

/// Show detailed capabilities for a specific provider.
fn run_provider_detail(provider: Provider) -> Result<()> {
    let term = Terminal::new();
    let clients = InstalledAiClients::new();
    let installed = clients.is_installed(provider_to_ai_cli(provider));
    let caps = capabilities_for(provider);

    // Header
    let status_icon = if installed { "\u{2705}" } else { "\u{274c}" };
    let header = Prose::new(format!(
        "{{{{bold}}}}{}{{{{reset}}}} {} {{{{dim}}}}({}installed){{{{reset}}}}",
        provider,
        status_icon,
        if installed { "" } else { "not " }
    ));
    log::data(&format!("\n {}", header.fallback_render(&term)));

    // Show docs URL
    let docs = Prose::new(format!("{{{{dim}}}}{}{{{{reset}}}}", provider.docs_url()));
    log::data(&format!(" {}", docs.fallback_render(&term)));
    log::data("");

    // Resource support table
    let columns = vec![
        TableColumn::new("Resource"),
        TableColumn::new("Support"),
        TableColumn::new("Format"),
        TableColumn::new("Repo Path"),
        TableColumn::new("Notes"),
    ];
    let mut table = Table::new().with_columns(columns).prefer_cursor_alignment();
    table.layout_mut().left_margin = Margin::Chars(1);

    for resource in LinkableResource::ALL {
        let support = caps.support_for(resource);

        let support_cell = format_support_level_cell(&term, support.level);

        let format_cell: TableCellContent = support
            .format
            .map(|f| f.name().to_string())
            .unwrap_or_else(|| "-".to_string())
            .into();

        let repo_path_cell: TableCellContent = support
            .repo_path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "-".to_string())
            .into();

        let notes_cell: TableCellContent = support
            .notes
            .map(|n| Prose::new(format!("{{{{dim}}}}{}{{{{reset}}}}", n)).fallback_render(&term))
            .unwrap_or_else(|| "-".to_string())
            .into();

        table.add_row(vec![
            resource.name().into(),
            support_cell,
            format_cell,
            repo_path_cell,
            notes_cell,
        ]);
    }

    let rendered = table.fallback_render(&term);
    log::data(&rendered);

    // Show "also reads from" info if applicable
    let has_also_reads = LinkableResource::ALL
        .iter()
        .any(|r| !caps.support_for(*r).also_reads_from.is_empty());

    if has_also_reads {
        log::data("");
        let also_header = Prose::new("{{bold}}Cross-Provider Discovery{{reset}}");
        log::data(&format!(" {}", also_header.fallback_render(&term)));

        for resource in LinkableResource::ALL {
            let support = caps.support_for(resource);
            if !support.also_reads_from.is_empty() {
                let paths: Vec<String> = support
                    .also_reads_from
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect();
                let also_reads = Prose::new(format!(
                    "{{{{dim}}}}{}: also reads from {{{{green}}}}{}{{{{reset}}}}",
                    resource.name(),
                    paths.join(", ")
                ));
                log::data(&format!("  {}", also_reads.fallback_render(&term)));
            }
        }
    }

    // Show skill frontmatter support
    log::data("");
    let fm_header = Prose::new("{{bold}}Skill Frontmatter Fields{{reset}}");
    log::data(&format!(" {}", fm_header.fallback_render(&term)));

    let fm = &caps.skill_frontmatter;
    let fm_columns = vec![
        TableColumn::new("Field"),
        TableColumn::new("Supported").with_alignment(Alignment::Center),
        TableColumn::new("Usage"),
    ];
    let mut fm_table = Table::new()
        .with_columns(fm_columns)
        // .prefer_cursor_alignment()
        .alternate_background_color();
    fm_table.layout_mut().left_margin = Margin::Chars(1);

    let fm_fields: [(&str, bool, &str); 8] = [
        (
            "name",
            fm.name,
            "Skill identifier used for activation and display",
        ),
        (
            "description",
            fm.description,
            "Triggers automatic activation based on context",
        ),
        (
            "license",
            fm.license,
            "SPDX license identifier (e.g., MIT, Apache-2.0)",
        ),
        (
            "compatibility",
            fm.compatibility,
            "Environment requirements (OS, runtime, tools)",
        ),
        (
            "metadata",
            fm.metadata,
            "Custom key-value pairs for categorization",
        ),
        (
            "allowed-tools",
            fm.allowed_tools,
            "Restricts which tools the skill can invoke",
        ),
        (
            "user-invocable",
            fm.user_invocable,
            "Enables manual activation via slash command",
        ),
        (
            "disable-model-invocation",
            fm.disable_model_invocation,
            "Prevents automatic activation by the model",
        ),
    ];

    for (field, supported, usage) in fm_fields {
        let usage_cell: TableCellContent = if supported {
            Prose::new(format!("{{{{dim}}}}{}{{{{reset}}}}", usage))
                .fallback_render(&term)
                .into()
        } else {
            "".into()
        };
        fm_table.add_row(vec![field.into(), bool_indicator(supported), usage_cell]);
    }

    let fm_rendered = fm_table.fallback_render(&term);
    log::data(&fm_rendered);

    Ok(())
}

/// Format a support level cell with color.
fn format_support_level_cell(term: &Terminal, level: SupportLevel) -> TableCellContent {
    let text = match level {
        SupportLevel::Full => "{{green}}Full{{reset}}",
        SupportLevel::CustomFormat => "{{yellow}}Custom format{{reset}}",
        SupportLevel::Limited => "{{dim}}Limited{{reset}}",
        SupportLevel::None => "{{dim}}-{{reset}}",
    };
    Prose::new(text).fallback_render(term).into()
}
