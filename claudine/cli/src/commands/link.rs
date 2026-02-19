use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use clap::{Args, ValueEnum};
use color_eyre::eyre::Result;

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::components::table::table::{Table, TableCellContent, TableColumn};
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::{Alignment, Margin};
use claudine::dispatch::loader::load_config;
use claudine::events::Provider;
use claudine::linking::model::{ReferenceStatus, Resource, ResourceReference};
use claudine::linking::{
    self, all_capabilities, canonical_provider, capabilities_for, ApplySummary, LinkableResource,
    ResourceFormat, ResourceScope as PathScope, SupportLevel, ALL_PROVIDERS,
};
use sniff::programs::InstalledAiClients;

use crate::log;

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum LinkScopeArg {
    User,
    Repo,
}

impl LinkScopeArg {
    fn path_scope(self) -> PathScope {
        match self {
            Self::User => PathScope::User,
            Self::Repo => PathScope::Repo,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Repo => "repo",
        }
    }
}

/// Arguments for the link subcommand.
#[derive(Args)]
pub struct LinkArgs {
    /// Optional provider name for detailed view (fuzzy matching supported)
    #[arg(value_name = "PROVIDER")]
    pub provider_arg: Option<String>,

    /// Show provider resource support matrix
    #[arg(long)]
    pub support: bool,

    /// Filter to a specific resource name.
    #[arg(long)]
    pub filter: Option<String>,

    /// Resource scope to analyze/link.
    #[arg(long, value_enum, default_value_t = LinkScopeArg::User)]
    pub scope: LinkScopeArg,

    /// Apply fixable states (default is dry-run analysis only).
    #[arg(long)]
    pub apply: bool,

    /// Show detailed output.
    #[arg(long)]
    pub detailed: bool,
}

fn bool_indicator(value: bool) -> TableCellContent {
    if value {
        "\u{2705}".into()
    } else {
        "\u{274C}".into()
    }
}

/// Link skills and commands across providers.
pub fn run(args: LinkArgs) -> Result<()> {
    // Handle --support flag
    if args.support {
        // If a provider is specified with --support, show detailed view
        if let Some(ref provider_input) = args.provider_arg {
            match Provider::fuzzy_match_cli_name(provider_input) {
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
        match Provider::fuzzy_match_cli_name(provider_input) {
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

    run_link_strategy(args)
}

struct ResourceSection {
    resource: LinkableResource,
    canonical: Option<Provider>,
    rows: Vec<Resource>,
    apply_summary: ApplySummary,
}

fn run_link_strategy(args: LinkArgs) -> Result<()> {
    let scope = args.scope.path_scope();
    let paths = linking::ProviderSkillPaths::new();
    let repo_root = matches!(scope, PathScope::Repo).then_some(paths.repo_root());

    let config = match load_config(None, repo_root) {
        Ok(config) => config,
        Err(error) => {
            log::error(&format!("Failed to load Claudine config: {error}"));
            let init_hint = if matches!(scope, PathScope::Repo) {
                "Run `claudine init --repo` in this repository (or `claudine init` for user scope)."
            } else {
                "Run `claudine init` to create ~/.claudine/config.json."
            };
            log::data(&format!(" {init_hint}"));
            return Ok(());
        }
    };

    let Some(linking_settings) = config.settings.linking else {
        log::error("Config is missing settings.linking.");
        log::data(" Run `claudine init` (or `claudine init --repo`) to persist linking settings.");
        return Ok(());
    };

    let mut sections = Vec::new();
    let mut attention = BTreeSet::new();
    let mut total_apply = ApplySummary::default();

    for diagnostic in category_level_diagnostics(&paths, scope)? {
        attention.insert(diagnostic);
    }

    for resource in LinkableResource::ALL {
        let canonical = canonical_provider(&linking_settings.canonical_provider, scope, resource);
        let Some(canonical_provider) = canonical else {
            attention.insert(format!(
                "{}: no canonical provider configured for {} scope.",
                resource.name(),
                args.scope.label()
            ));
            sections.push(ResourceSection {
                resource,
                canonical: None,
                rows: Vec::new(),
                apply_summary: ApplySummary::default(),
            });
            continue;
        };

        let mut rows =
            linking::analyze_resource_links(&paths, resource, scope, canonical_provider)?;
        if let Some(filter) = args.filter.as_deref() {
            rows.retain(|resource| resource.name == filter);
        }

        let mut apply_summary = ApplySummary::default();
        if args.apply && !rows.is_empty() {
            apply_summary = linking::apply_fixable_resources(&paths, resource, scope, &rows)?;
            total_apply.links_created += apply_summary.links_created;
            total_apply.derived_written += apply_summary.derived_written;
            total_apply.skipped += apply_summary.skipped;

            rows = linking::analyze_resource_links(&paths, resource, scope, canonical_provider)?;
            if let Some(filter) = args.filter.as_deref() {
                rows.retain(|resource| resource.name == filter);
            }
        }

        if !rows.is_empty()
            && !rows.iter().any(|row| {
                matches!(
                    row.definition,
                    ResourceReference::Source(_) | ResourceReference::PartialSource(_, _)
                )
            })
        {
            attention.insert(format!(
                "{}: no canonical candidate found from configured provider {}.",
                resource.name(),
                canonical_provider
            ));
        }

        for row in &rows {
            match &row.definition {
                ResourceReference::PartialSource(definition, missing) => {
                    attention.insert(format!(
                        "{} '{}' ({}) is PartialSource; missing required properties [{}] in {}.",
                        resource.name(),
                        row.name,
                        row.provider,
                        missing.join(", "),
                        definition.filepath.display()
                    ));
                }
                ResourceReference::IncompleteLink(provider, _) => {
                    attention.insert(format!(
                        "{} '{}' for {} is IncompleteLink; manual fix required (requirements or existing target mismatch).",
                        resource.name(),
                        row.name,
                        provider
                    ));
                }
                _ => {}
            }
        }

        sections.push(ResourceSection {
            resource,
            canonical: Some(canonical_provider),
            rows,
            apply_summary,
        });
    }

    render_link_strategy(&args, &sections, &attention, total_apply);
    Ok(())
}

fn render_link_strategy(
    args: &LinkArgs,
    sections: &[ResourceSection],
    attention: &BTreeSet<String>,
    total_apply: ApplySummary,
) {
    let term = Terminal::new();
    let mode = if args.apply { "apply" } else { "dry-run" };
    let header = Prose::new(format!(
        "{{{{bold}}}}Link Strategy Analysis{{{{reset}}}} {{{{dim}}}}(scope: {}, mode: {}){{{{reset}}}}",
        args.scope.label(),
        mode
    ));
    log::data(&format!("\n {}", header.fallback_render(&term)));

    for section in sections {
        log::data("");
        let canonical = section
            .canonical
            .map(|provider| provider.to_string())
            .unwrap_or_else(|| "not configured".to_string());
        let section_header = Prose::new(format!(
            "{{{{bold}}}}{}{{{{reset}}}} {{{{dim}}}}(canonical: {}){{{{reset}}}}",
            section.resource.name(),
            canonical
        ));
        log::data(&format!(" {}", section_header.fallback_render(&term)));

        if section.rows.is_empty() {
            let empty = if args.filter.is_some() {
                "{{dim}}No matching resources found for current filter.{{reset}}"
            } else {
                "{{dim}}No resources discovered.{{reset}}"
            };
            log::data(&format!(" {}", Prose::new(empty).fallback_render(&term)));
            continue;
        }

        let mut table = Table::new()
            .with_columns(vec![
                TableColumn::new("Name"),
                TableColumn::new("Provider"),
                TableColumn::new("Variant"),
                TableColumn::new("Status"),
                TableColumn::new("Details"),
            ])
            .prefer_cursor_alignment();
        table.layout_mut().left_margin = Margin::Chars(1);

        let mut ok_count = 0usize;
        let mut fixable_count = 0usize;
        let mut attention_count = 0usize;

        for row in &section.rows {
            let status = row.definition.status();
            match status {
                ReferenceStatus::Ok => ok_count += 1,
                ReferenceStatus::IsFixable => fixable_count += 1,
                ReferenceStatus::NeedsUserAttention => attention_count += 1,
            }

            table.add_row(vec![
                row.name.clone().into(),
                row.provider.to_string().into(),
                reference_variant_name(&row.definition).to_string().into(),
                reference_status_label(status).into(),
                reference_details(&row.definition, args.detailed).into(),
            ]);
        }

        log::data(&table.fallback_render(&term));
        let summary = Prose::new(format!(
            "{{{{dim}}}}summary: ok={}, fixable={}, needs-attention={}{{{{reset}}}}",
            ok_count, fixable_count, attention_count
        ));
        log::data(&format!(" {}", summary.fallback_render(&term)));

        if args.apply {
            let apply = Prose::new(format!(
                "{{{{dim}}}}applied: links_created={}, derived_written={}, skipped={}{{{{reset}}}}",
                section.apply_summary.links_created,
                section.apply_summary.derived_written,
                section.apply_summary.skipped
            ));
            log::data(&format!(" {}", apply.fallback_render(&term)));
        }
    }

    if args.apply {
        log::data("");
        let apply_total = Prose::new(format!(
            "{{{{bold}}}}Apply Totals{{{{reset}}}} {{{{dim}}}}links_created={}, derived_written={}, skipped={}{{{{reset}}}}",
            total_apply.links_created, total_apply.derived_written, total_apply.skipped
        ));
        log::data(&format!(" {}", apply_total.fallback_render(&term)));
    } else {
        log::data("");
        let hint = Prose::new(
            "{{dim}}Run {{blue}}{{bold}}claudine link --apply{{reset}}{{dim}} to fix LinkMissing/DerivedMissing/DerivedStale states.{{reset}}",
        );
        log::data(&format!(" {}", hint.fallback_render(&term)));
    }

    if !attention.is_empty() {
        log::data("");
        let attention_header = Prose::new("{{bold}}Needs Attention{{reset}}");
        log::data(&format!(" {}", attention_header.fallback_render(&term)));
        for item in attention {
            let line = Prose::new(format!("{{{{yellow}}}}- {{{{reset}}}}{item}"));
            log::data(&format!(" {}", line.fallback_render(&term)));
        }
    }
}

fn category_level_diagnostics(
    paths: &linking::ProviderSkillPaths,
    scope: PathScope,
) -> Result<Vec<String>> {
    let mut diagnostics = Vec::new();

    for resource in LinkableResource::ALL {
        let mut roots = Vec::new();
        for provider in ALL_PROVIDERS {
            if let Some(root) = paths.resource_dir(provider, resource, scope) {
                roots.push((provider, root));
            }
        }

        for (provider, root) in &roots {
            let Some(target) = linking::category_link_target(root)? else {
                continue;
            };

            diagnostics.push(category_link_diagnostic(
                *provider, resource, root, &target, &roots,
            ));
        }
    }

    Ok(diagnostics)
}

fn category_link_diagnostic(
    provider: Provider,
    resource: LinkableResource,
    root: &Path,
    target: &Path,
    roots: &[(Provider, PathBuf)],
) -> String {
    let resolved = resolve_link_target(root, target);
    let mapped = roots
        .iter()
        .find(|(_, candidate)| *candidate == resolved)
        .map(|(provider, _)| *provider);

    if let Some(mapped_provider) = mapped {
        format!(
            "{} {} root is category-level symlink at {} -> {} (mapped to {}). Keep as-is or replace with granular links.",
            provider,
            resource.name(),
            root.display(),
            target.display(),
            mapped_provider
        )
    } else {
        format!(
            "{} {} root has unknown/mismatched category-level symlink at {} -> {}. Replace with granular links manually.",
            provider,
            resource.name(),
            root.display(),
            target.display()
        )
    }
}

fn resolve_link_target(root: &Path, target: &Path) -> PathBuf {
    if target.is_absolute() {
        normalize_path(target)
    } else {
        let joined = root
            .parent()
            .map(|parent| parent.join(target))
            .unwrap_or_else(|| target.to_path_buf());
        normalize_path(&joined)
    }
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            _ => normalized.push(component.as_os_str()),
        }
    }
    normalized
}

fn reference_variant_name(reference: &ResourceReference) -> &'static str {
    match reference {
        ResourceReference::Source(_) => "Source",
        ResourceReference::PartialSource(_, _) => "PartialSource",
        ResourceReference::Isolated(_) => "Isolated",
        ResourceReference::Link(_, _) => "Link",
        ResourceReference::LinkMissing(_, _) => "LinkMissing",
        ResourceReference::IncompleteLink(_, _) => "IncompleteLink",
        ResourceReference::DerivedLink(_, _) => "DerivedLink",
        ResourceReference::DerivedStale(_, _) => "DerivedStale",
        ResourceReference::DerivedMissing(_, _) => "DerivedMissing",
    }
}

fn reference_status_label(status: ReferenceStatus) -> &'static str {
    match status {
        ReferenceStatus::Ok => "Ok",
        ReferenceStatus::IsFixable => "IsFixable",
        ReferenceStatus::NeedsUserAttention => "NeedsUserAttention",
    }
}

fn reference_details(reference: &ResourceReference, detailed: bool) -> String {
    match reference {
        ResourceReference::Source(definition) => {
            if detailed {
                format!("canonical source at {}", definition.filepath.display())
            } else {
                "-".to_string()
            }
        }
        ResourceReference::PartialSource(definition, missing) => format!(
            "missing [{}] in {}",
            missing.join(", "),
            definition.filepath.display()
        ),
        ResourceReference::Isolated(definition) => {
            if detailed {
                format!("isolated resource at {}", definition.filepath.display())
            } else {
                "not linked to canonical".to_string()
            }
        }
        ResourceReference::Link(_, _) => {
            if detailed {
                "symlink in sync".to_string()
            } else {
                "-".to_string()
            }
        }
        ResourceReference::LinkMissing(_, _) => {
            "missing symlink (fixable with --apply)".to_string()
        }
        ResourceReference::IncompleteLink(_, _) => {
            "manual fix required (requirements/target mismatch)".to_string()
        }
        ResourceReference::DerivedLink(_, _) => {
            if detailed {
                "derived artifact in sync".to_string()
            } else {
                "-".to_string()
            }
        }
        ResourceReference::DerivedStale(_, _) => {
            "derived artifact stale (fixable with --apply)".to_string()
        }
        ResourceReference::DerivedMissing(_, _) => {
            "derived artifact missing (fixable with --apply)".to_string()
        }
    }
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
        let installed = clients.is_installed(provider.sniff_ai_cli());

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
    let installed = clients.is_installed(provider.sniff_ai_cli());
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use claudine::linking::model::{ResourceDefinition, ResourceScope as ModelScope};

    use super::*;

    fn sample_definition() -> ResourceDefinition {
        ResourceDefinition {
            name: "lint".to_string(),
            provider: Provider::Claude,
            scope: ModelScope::User,
            filepath: PathBuf::from("/tmp/lint.md"),
            frontmatter: BTreeMap::new(),
            body: "Run lint".to_string(),
            fm_hash: 1,
            body_hash: 2,
        }
    }

    #[test]
    fn variant_name_reports_all_reference_variants() {
        let definition = sample_definition();
        let variants = vec![
            ResourceReference::Source(definition.clone()),
            ResourceReference::PartialSource(definition.clone(), vec!["name".to_string()]),
            ResourceReference::Isolated(definition),
            ResourceReference::Link(Provider::Codex, ModelScope::User),
            ResourceReference::LinkMissing(Provider::Codex, ModelScope::User),
            ResourceReference::IncompleteLink(Provider::Codex, ModelScope::User),
            ResourceReference::DerivedLink(Provider::Codex, ModelScope::User),
            ResourceReference::DerivedStale(Provider::Codex, ModelScope::User),
            ResourceReference::DerivedMissing(Provider::Codex, ModelScope::User),
        ];

        let names: Vec<&str> = variants.iter().map(reference_variant_name).collect();
        assert_eq!(
            names,
            vec![
                "Source",
                "PartialSource",
                "Isolated",
                "Link",
                "LinkMissing",
                "IncompleteLink",
                "DerivedLink",
                "DerivedStale",
                "DerivedMissing",
            ]
        );
    }

    #[test]
    fn resolve_link_target_expands_relative_target() {
        let root = Path::new("/tmp/repo/.codex/agents");
        let target = Path::new("../.claude/agents");
        let resolved = resolve_link_target(root, target);
        assert_eq!(resolved, PathBuf::from("/tmp/repo/.claude/agents"));
    }

    #[test]
    fn category_link_diagnostic_reports_known_mapping() {
        let roots = vec![
            (Provider::Claude, PathBuf::from("/tmp/repo/.claude/skills")),
            (Provider::Codex, PathBuf::from("/tmp/repo/.codex/skills")),
        ];
        let message = category_link_diagnostic(
            Provider::Codex,
            LinkableResource::Skill,
            Path::new("/tmp/repo/.codex/skills"),
            Path::new("../.claude/skills"),
            &roots,
        );

        assert!(message.contains("mapped to Claude"));
        assert!(message.contains("category-level symlink"));
    }

    #[test]
    fn category_link_diagnostic_reports_unknown_mapping() {
        let roots = vec![
            (Provider::Claude, PathBuf::from("/tmp/repo/.claude/skills")),
            (Provider::Codex, PathBuf::from("/tmp/repo/.codex/skills")),
        ];
        let message = category_link_diagnostic(
            Provider::Codex,
            LinkableResource::Skill,
            Path::new("/tmp/repo/.codex/skills"),
            Path::new("/tmp/repo/.custom/skills"),
            &roots,
        );

        assert!(message.contains("unknown/mismatched"));
    }
}
