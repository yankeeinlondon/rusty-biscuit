use std::collections::{BTreeMap, BTreeSet};

use clap::Args;
use color_eyre::eyre::Result;

use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::WordWrap;
use claudine::badges;
use claudine::linking::{
    CommandDirectoryDiagnostic, CommandException, CommandExceptionType, CommandFixSummary,
    CommandInfo, CommandScope, LinkableResource, ProviderSkillPaths, ResourceFilter,
    fix_missing_commands, list_commands,
};
use sniff::filesystem::git::detect_git;

use super::link_display::{
    LinkableResourceDisplay, build_provider_header, render_canonical_providers, render_footer,
    render_normal, render_verbose, repo_canonical_needs_init,
};
use crate::log;

/// Arguments for the slash-commands subcommand.
#[derive(Args)]
pub struct SlashCommandsArgs {
    /// Filter commands by name. Supports negation (`-test` or `!test`) and exact match (`test!`).
    #[arg(value_name = "FILTER")]
    pub filter: Vec<String>,

    /// Fix missing command links for non-Claude providers.
    #[arg(long, visible_alias = "fix")]
    pub apply: bool,
}

/// List available slash commands and their scopes.
pub async fn run(args: SlashCommandsArgs, verbose: bool) -> Result<()> {
    let paths = ProviderSkillPaths::new();

    let cwd = std::env::current_dir().unwrap_or_default();
    let is_git_repo = detect_git(&cwd, false, 1).ok().flatten().is_some();

    if is_git_repo && args.apply && repo_canonical_needs_init(&paths, LinkableResource::Command) {
        log::message("");
        log::message("Repo canonical provider is not configured.");
        log::message("Run `claudine config` and set the repo provider in the Preferences tab.");
        log::message("");
        return Ok(());
    }

    let fix_summary = if args.apply {
        Some(fix_missing_commands(&paths)?)
    } else {
        None
    };

    let mut report = list_commands(&paths, &args.filter)?;

    if !args.filter.is_empty() {
        let filters = ResourceFilter::parse_all(&args.filter);
        if !filters.is_empty() {
            report
                .exceptions
                .retain(|exc| ResourceFilter::retain(&filters, &exc.name));
        }
        report.diagnostics.clear();
    }

    if report.commands.is_empty() {
        if args.filter.is_empty() {
            log::data("No slash commands found.");
        } else {
            log::data(&format!(
                "No slash commands matching: {}",
                args.filter.join(", ")
            ));
        }
        return Ok(());
    }

    let term = crate::log::terminal();

    let header = Prose::new("<blue><b>Slash Commands</b></blue>").render(&term);
    log::data("");
    log::data(&header);
    log::data(&Prose::new("<blue>==================</blue>").render(&term));
    log::data("");

    render_canonical_providers(&term, &paths, is_git_repo, LinkableResource::Command);

    let cmd_count = report.commands.len();

    if cmd_count == 1 {
        render_detail(&term, &report.commands[0]);
    } else if cmd_count < 6 || verbose {
        render_verbose(&term, &report.commands, scope_badge);
    } else {
        render_normal(&term, &report.commands, scope_badge);
    }

    if let Some(summary) = fix_summary {
        render_fix_summary(&term, &summary);
    }

    let has_exceptions = !report.exceptions.is_empty() || !report.diagnostics.is_empty();
    if has_exceptions {
        render_exceptions(&term, &report.exceptions, &report.diagnostics, verbose);
    }

    render_footer(
        &term,
        has_exceptions,
        args.apply,
        is_git_repo,
        cmd_count,
        verbose,
        &args.filter,
        "commands",
        "<dim><i>using the <green>--verbose</green> switch will provide not only names but also descriptions</i></dim>",
    );

    Ok(())
}

fn render_detail(term: &Terminal, cmd: &CommandInfo) {
    let badge = scope_badge(cmd.scope);
    let desc = cmd.description.as_deref().unwrap_or("no description");

    let name_line = Prose::new(format!(
        r#"<a href="{}"><b>{}</b></a> {badge}"#,
        biscuit_file::to_portable_string(&cmd.command_file_path),
        cmd.name,
    ));
    log::data(&name_line.render(term));

    let desc_line = Prose::new(format!("<dim><i>{desc}</i></dim>"))
        .with_word_wrap(WordWrap::BespokeProse(None, vec![' '], None));
    log::data(&desc_line.render(term));

    // Frontmatter properties
    if !cmd.frontmatter.is_empty() {
        log::data("");
        let mut props_list = UnorderedList::empty();
        for (key, value) in &cmd.frontmatter {
            let display_value = if key == "model" {
                format!("{value} <orange>(not shareable)</orange>")
            } else {
                value.clone()
            };
            props_list.add(Prose::new(format!(
                "<b>{key}</b>: <dim>{display_value}</dim>"
            )));
        }
        log::data(&props_list.render(term));
    }

    // File size/format info
    if let Ok(metadata) = std::fs::metadata(&cmd.command_file_path) {
        let size = metadata.len();
        let size_display = if size < 1024 {
            format!("{size} bytes")
        } else {
            format!("{:.1} KB", size as f64 / 1024.0)
        };
        log::data(
            &Prose::new(format!("<dim>format: Markdown, size: {size_display}</dim>")).render(term),
        );
    }

    if cmd.has_model_property {
        log::data("");
        let note = Prose::new(
            "<dim><i><orange>note:</orange> this command specifies a <b>model</b> property which limits cross-provider shareability</i></dim>",
        );
        log::data(&note.render(term));
    }
}

fn render_exceptions(
    term: &Terminal,
    exceptions: &[CommandException],
    diagnostics: &[CommandDirectoryDiagnostic],
    _verbose: bool,
) {
    log::data("");
    log::data(&badges::EXCEPTIONS);
    log::data("");

    let mut by_provider: BTreeMap<String, BTreeMap<CommandExceptionType, Vec<&CommandException>>> =
        BTreeMap::new();
    for exc in exceptions {
        by_provider
            .entry(exc.provider.to_string())
            .or_default()
            .entry(exc.exception_type)
            .or_default()
            .push(exc);
    }

    let mut diag_by_provider: BTreeMap<String, Vec<&CommandDirectoryDiagnostic>> = BTreeMap::new();
    for diag in diagnostics {
        diag_by_provider
            .entry(diag.provider.to_string())
            .or_default()
            .push(diag);
    }

    let mut all_providers: BTreeSet<String> = by_provider.keys().cloned().collect();
    for key in diag_by_provider.keys() {
        all_providers.insert(key.clone());
    }

    // Separate format-incompatible-only providers from others
    let mut format_incompatible_providers: Vec<&String> = Vec::new();
    let mut regular_providers: Vec<&String> = Vec::new();

    for provider_name in &all_providers {
        let has_only_format_incompatible = by_provider.get(provider_name).is_some_and(|type_map| {
            type_map
                .keys()
                .all(|t| *t == CommandExceptionType::FormatIncompatible)
        }) && !diag_by_provider.contains_key(provider_name);

        if has_only_format_incompatible {
            format_incompatible_providers.push(provider_name);
        } else {
            regular_providers.push(provider_name);
        }
    }

    // Render format-incompatible providers as simple one-liners
    if !format_incompatible_providers.is_empty() {
        let mut fi_list = UnorderedList::empty();
        for provider_name in &format_incompatible_providers {
            fi_list.add(Prose::new(format!(
                "<b>{provider_name}</b> — ❌ uses a non-standard format which is incompatible with Claudine."
            )));
        }
        log::data(&fi_list.render(term));
    }

    // Render regular providers with full exception detail
    if !regular_providers.is_empty() {
        let mut outer_list = UnorderedList::empty();

        for provider_name in &regular_providers {
            let provider_header = build_provider_header(provider_name, LinkableResource::Command);
            outer_list.add(Prose::new(provider_header));

            let mut inner_list = UnorderedList::empty();

            // Render directory-level diagnostics directly at provider level (not nested under "missing")
            if let Some(provider_diags) = diag_by_provider.get(*provider_name) {
                for diag in provider_diags {
                    inner_list.add(Prose::new(diag.message.clone()));
                }
            }

            if let Some(type_map) = by_provider.get(*provider_name) {
                for (exc_type, entries) in type_map {
                    // Skip FormatIncompatible here — handled above
                    if *exc_type == CommandExceptionType::FormatIncompatible {
                        continue;
                    }

                    let count = entries.len();
                    let category_label = Prose::new(format!("<b>{exc_type}</b> ({count})"));
                    inner_list.add(category_label);

                    let mut detail_list = UnorderedList::empty();

                    match exc_type {
                        CommandExceptionType::Missing => {
                            let topics: Vec<String> =
                                entries.iter().map(|e| e.name.clone()).collect();
                            let topic_line = Prose::new(topics.join(", ")).with_word_wrap(
                                WordWrap::BespokeProse(Some(500), vec![' ', ','], Some(2)),
                            );
                            detail_list.add(topic_line);
                        }
                        CommandExceptionType::Invalid => {
                            for e in entries {
                                let props_markup = e
                                    .missing_properties
                                    .iter()
                                    .map(|p| format!("<red>{p}</red>"))
                                    .collect::<Vec<_>>()
                                    .join(", ");
                                let label = format!(
                                    r#"<b><a href="{}">{}</a></b> (<i>missing the properties {}</i>)"#,
                                    biscuit_file::to_portable_string(&e.command_file_path),
                                    e.name,
                                    props_markup
                                );
                                detail_list.add(Prose::new(label));
                            }
                        }
                        CommandExceptionType::YamlTabs => {
                            for e in entries {
                                let label = format!(
                                    r#"<b><a href="{}">{}</a></b> (<i>contains tab characters in YAML indentation</i>)"#,
                                    biscuit_file::to_portable_string(&e.command_file_path),
                                    e.name
                                );
                                detail_list.add(Prose::new(label));
                            }
                        }
                        CommandExceptionType::ModelPropertyNotShareable => {
                            for e in entries {
                                let model_info = e
                                    .model_value
                                    .as_deref()
                                    .map(|v| format!(" = {v}"))
                                    .unwrap_or_default();
                                let label = format!(
                                    r#"<a href="{}"><b>{}</b></a> (<i>specifies <orange>model{model_info}</orange></i>)"#,
                                    biscuit_file::to_portable_string(&e.command_file_path),
                                    e.name
                                );
                                detail_list.add(Prose::new(label));
                            }
                        }
                        CommandExceptionType::NoLinks => {
                            let topics: Vec<String> =
                                entries.iter().map(|e| e.name.clone()).collect();
                            let topic_line = Prose::new(topics.join(", ")).with_word_wrap(
                                WordWrap::BespokeProse(Some(500), vec![' ', ','], Some(2)),
                            );
                            detail_list.add(topic_line);
                        }
                        CommandExceptionType::FormatIncompatible => unreachable!(),
                    }

                    inner_list.add(detail_list);
                }
            }

            outer_list.add(inner_list);
        }

        log::data(&outer_list.render(term));
    }
}

fn render_fix_summary(term: &Terminal, summary: &CommandFixSummary) {
    log::data("");
    let header = Prose::new("<b>Fix Summary</b>").render(term);
    log::data(&header);

    let parts = [
        format!("directories_created={}", summary.directories_created),
        format!("links_created={}", summary.links_created),
        format!("already_linked={}", summary.already_linked),
        format!("skipped={}", summary.skipped),
        format!("format_incompatible={}", summary.format_incompatible),
        format!("yaml_tabs_fixed={}", summary.yaml_tabs_fixed),
        format!("not_shareable={}", summary.not_shareable),
    ];
    let detail = Prose::new(format!("<dim>{}</dim>", parts.join(", ")));
    log::data(&format!(" {}", detail.render(term)));
}

fn scope_badge(scope: CommandScope) -> &'static str {
    match scope {
        CommandScope::User => &badges::USER_SCOPED,
        CommandScope::RepoMasked => &badges::MASKED_REPO_SCOPED,
        CommandScope::Repo => &badges::REPO_SCOPED,
    }
}

impl LinkableResourceDisplay for CommandInfo {
    type Scope = CommandScope;

    fn scope(&self) -> Self::Scope {
        self.scope
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    fn path(&self) -> &std::path::Path {
        &self.command_file_path
    }
}
