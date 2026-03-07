use std::collections::{BTreeMap, BTreeSet};

use clap::Args;
use color_eyre::eyre::Result;

use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::WordWrap;
use claudine::badges;
use claudine::dispatch::loader::load_config;
use claudine::linking::{
    AgentDirectoryDiagnostic, AgentException, AgentExceptionType, AgentFixSummary, AgentInfo,
    AgentScope, LinkableResource, ProviderSkillPaths, ResourceFilter, ResourceScope,
    canonical_provider, capabilities_for, fix_missing_agents, list_agents,
};
use sniff::filesystem::git::detect_git;

use crate::log;

/// Arguments for the agents subcommand.
#[derive(Args)]
pub struct AgentsArgs {
    /// Filter agents by name. Supports negation (`-test` or `!test`) and exact match (`test!`).
    #[arg(value_name = "FILTER")]
    pub filter: Vec<String>,

    /// Fix missing agent links for non-Claude providers.
    #[arg(long, visible_alias = "fix")]
    pub apply: bool,
}

/// List available agent definitions and their scopes.
pub async fn run(args: AgentsArgs, verbose: bool) -> Result<()> {
    let paths = ProviderSkillPaths::new();

    let cwd = std::env::current_dir().unwrap_or_default();
    let is_git_repo = detect_git(&cwd, false, 1).ok().flatten().is_some();

    if is_git_repo && args.apply && repo_canonical_needs_init(&paths) {
        log::message("");
        log::message("Repo canonical provider is not configured. Running claudine init --repo ...");
        log::message("");
        super::init::run(super::init::InitArgs {
            quick: false,
            repo: true,
        })
        .await?;
        log::message("");
    }

    let fix_summary = if args.apply {
        Some(fix_missing_agents(&paths)?)
    } else {
        None
    };

    let mut report = list_agents(&paths, &args.filter)?;

    if !args.filter.is_empty() {
        let filters = ResourceFilter::parse_all(&args.filter);
        if !filters.is_empty() {
            report
                .exceptions
                .retain(|exc| ResourceFilter::retain(&filters, &exc.name));
        }
        report.diagnostics.clear();
    }

    if report.agents.is_empty() {
        if args.filter.is_empty() {
            log::data("No agents found.");
        } else {
            log::data(&format!("No agents matching: {}", args.filter.join(", ")));
        }
        return Ok(());
    }

    let term = Terminal::new();

    let header = Prose::new("<blue><b>Agents</b></blue>").render(&term);
    log::data("");
    log::data(&header);
    log::data(&Prose::new("<blue>==================</blue>").render(&term));
    log::data("");

    render_canonical_providers(&term, &paths, is_git_repo);

    let agent_count = report.agents.len();

    if agent_count == 1 {
        render_detail(&term, &report.agents[0]);
    } else if agent_count < 6 || verbose {
        render_verbose(&term, &report.agents);
    } else {
        render_normal(&term, &report.agents);
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
        agent_count,
        verbose,
        &args.filter,
    );

    Ok(())
}

fn repo_canonical_needs_init(paths: &ProviderSkillPaths) -> bool {
    let repo_root = Some(paths.repo_root());
    match load_config(None, repo_root) {
        Ok(config) => match &config.settings.linking {
            Some(linking) => canonical_provider(
                &linking.canonical_provider,
                ResourceScope::Repo,
                LinkableResource::Agent,
            )
            .is_none(),
            None => true,
        },
        Err(_) => false,
    }
}

fn render_canonical_providers(term: &Terminal, paths: &ProviderSkillPaths, is_git_repo: bool) {
    let repo_root = if is_git_repo {
        Some(paths.repo_root())
    } else {
        None
    };

    let config = match load_config(None, repo_root) {
        Ok(c) => c,
        Err(_) => return,
    };

    let Some(linking_settings) = config.settings.linking else {
        return;
    };

    let user_canonical = canonical_provider(
        &linking_settings.canonical_provider,
        ResourceScope::User,
        LinkableResource::Agent,
    );

    let not_configured = "<i><red>not configured</red></i>";

    let user_part = match user_canonical {
        Some(user) => format!("user: <b>{user}</b>"),
        None => format!("user: {not_configured}"),
    };

    let line = if is_git_repo {
        let repo_canonical = canonical_provider(
            &linking_settings.canonical_provider,
            ResourceScope::Repo,
            LinkableResource::Agent,
        );
        let repo_part = match repo_canonical {
            Some(repo) => format!("repo: <b>{repo}</b>"),
            None => format!("repo: {not_configured}"),
        };
        format!("<blue><b>Canonical Providers:</b></blue> {user_part}, {repo_part}")
    } else {
        format!("<blue><b>Canonical Providers:</b></blue> {user_part}")
    };

    log::data(&Prose::new(line).render(term));
    log::data("");
}

fn render_detail(term: &Terminal, agent: &AgentInfo) {
    let badge = scope_badge(agent.scope);
    let desc = agent.description.as_deref().unwrap_or("no description");

    let name_line = Prose::new(format!(
        r#"<a href="{}"><b>{}</b></a> {badge}"#,
        agent.agent_file_path.display(),
        agent.name,
    ));
    log::data(&name_line.render(term));

    let desc_line = Prose::new(format!("<dim><i>{desc}</i></dim>"))
        .with_word_wrap(WordWrap::BespokeProse(None, vec![' '], None));
    log::data(&desc_line.render(term));

    if agent.has_model_property {
        log::data("");
        let note = Prose::new(
            "<dim><i><orange>note:</orange> this agent specifies a <b>model</b> property which limits cross-provider shareability</i></dim>",
        );
        log::data(&note.render(term));
    }

    // Content preview (up to 20 lines)
    if let Ok(content) = std::fs::read_to_string(&agent.agent_file_path) {
        let body = extract_body(&content);
        if !body.is_empty() {
            log::data("");
            let preview_lines: Vec<&str> = body.lines().take(20).collect();
            let preview = preview_lines.join("\n");
            let preview_prose = Prose::new(format!("<dim>{preview}</dim>"))
                .with_word_wrap(WordWrap::BespokeProse(None, vec![' '], None));
            log::data(&preview_prose.render(term));
            let total_lines = body.lines().count();
            if total_lines > 20 {
                log::data(
                    &Prose::new(format!(
                        "<dim><i>... ({} more lines)</i></dim>",
                        total_lines - 20
                    ))
                    .render(term),
                );
            }
        }
    }
}

fn render_verbose(term: &Terminal, agents: &[AgentInfo]) {
    let mut list = UnorderedList::empty();

    for agent in agents {
        let badge = scope_badge(agent.scope);
        let desc = agent.description.as_deref().unwrap_or("no description");
        let format_badge = "<dim>[md]</dim>";
        let item = Prose::new(format!(
            r#"<a href="{}"><b>{}</b></a> {badge} {format_badge} <dim><i>{desc}</i></dim>"#,
            agent.agent_file_path.display(),
            agent.name,
        ));
        list.add(item);
    }

    log::data(&list.render(term));
}

fn render_normal(term: &Terminal, agents: &[AgentInfo]) {
    let mut by_scope: BTreeMap<AgentScope, Vec<&AgentInfo>> = BTreeMap::new();
    for agent in agents {
        by_scope.entry(agent.scope).or_default().push(agent);
    }

    for (scope, group) in &by_scope {
        let count = group.len();
        let badge_line = format!("{} <dim>(<i>{count}</i>)</dim>", scope_badge(*scope));
        log::data(&Prose::new(badge_line).render(term));
        log::data("");

        let names: Vec<String> = group
            .iter()
            .map(|a| {
                format!(
                    r#"<a href="{}"><b>{}</b></a>"#,
                    a.agent_file_path.display(),
                    a.name
                )
            })
            .collect();

        let joined = names.join("  ");
        let rendered = Prose::new(joined)
            .with_word_wrap(WordWrap::BespokeProse(Some(50), vec![' '], None))
            .render(term);
        log::data(&rendered);
        log::data("");
    }
}

fn render_exceptions(
    term: &Terminal,
    exceptions: &[AgentException],
    diagnostics: &[AgentDirectoryDiagnostic],
    _verbose: bool,
) {
    log::data("");
    log::data(&badges::EXCEPTIONS);
    log::data("");

    let mut by_provider: BTreeMap<String, BTreeMap<AgentExceptionType, Vec<&AgentException>>> =
        BTreeMap::new();
    for exc in exceptions {
        by_provider
            .entry(exc.provider.to_string())
            .or_default()
            .entry(exc.exception_type)
            .or_default()
            .push(exc);
    }

    let mut diag_by_provider: BTreeMap<String, Vec<&AgentDirectoryDiagnostic>> = BTreeMap::new();
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
                .all(|t| *t == AgentExceptionType::FormatIncompatible)
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
            let provider_header = build_provider_header(provider_name);
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
                    if *exc_type == AgentExceptionType::FormatIncompatible {
                        continue;
                    }

                    let count = entries.len();
                    let category_label = Prose::new(format!("<b>{exc_type}</b> ({count})"));
                    inner_list.add(category_label);

                    let mut detail_list = UnorderedList::empty();

                    match exc_type {
                        AgentExceptionType::Missing => {
                            let topics: Vec<String> =
                                entries.iter().map(|e| e.name.clone()).collect();
                            let topic_line = Prose::new(topics.join(", ")).with_word_wrap(
                                WordWrap::BespokeProse(Some(500), vec![' ', ','], Some(2)),
                            );
                            detail_list.add(topic_line);
                        }
                        AgentExceptionType::Invalid => {
                            for e in entries {
                                let props_markup = e
                                    .missing_properties
                                    .iter()
                                    .map(|p| format!("<red>{p}</red>"))
                                    .collect::<Vec<_>>()
                                    .join(", ");
                                let label = if e.missing_properties.is_empty() {
                                    format!(
                                        r#"<b><a href="{}">{}</a></b>"#,
                                        e.agent_file_path.display(),
                                        e.name
                                    )
                                } else {
                                    format!(
                                        r#"<b><a href="{}">{}</a></b> (<i>missing the properties {}</i>)"#,
                                        e.agent_file_path.display(),
                                        e.name,
                                        props_markup
                                    )
                                };
                                detail_list.add(Prose::new(label));
                            }
                        }
                        AgentExceptionType::ModelPropertyNotShareable => {
                            for e in entries {
                                let label = format!(
                                    r#"<a href="{}"><b>{}</b></a> (<i>specifies <orange>model</orange></i>)"#,
                                    e.agent_file_path.display(),
                                    e.name
                                );
                                detail_list.add(Prose::new(label));
                            }
                        }
                        AgentExceptionType::NoLinks => {
                            let topics: Vec<String> =
                                entries.iter().map(|e| e.name.clone()).collect();
                            let topic_line = Prose::new(topics.join(", ")).with_word_wrap(
                                WordWrap::BespokeProse(Some(500), vec![' ', ','], Some(2)),
                            );
                            detail_list.add(topic_line);
                        }
                        AgentExceptionType::FormatIncompatible => unreachable!(),
                    }

                    inner_list.add(detail_list);
                }
            }

            outer_list.add(inner_list);
        }

        log::data(&outer_list.render(term));
    }
}

fn render_fix_summary(term: &Terminal, summary: &AgentFixSummary) {
    log::data("");
    let header = Prose::new("<b>Fix Summary</b>").render(term);
    log::data(&header);

    let parts = [
        format!("directories_created={}", summary.directories_created),
        format!("links_created={}", summary.links_created),
        format!("already_linked={}", summary.already_linked),
        format!("skipped={}", summary.skipped),
        format!("format_incompatible={}", summary.format_incompatible),
    ];
    let detail = Prose::new(format!("<dim>{}</dim>", parts.join(", ")));
    log::data(&format!(" {}", detail.render(term)));
}

fn build_provider_header(provider_name: &str) -> String {
    use claudine::events::Provider;

    let Some(provider) = Provider::fuzzy_match_cli_name(provider_name) else {
        return format!("<b>{provider_name}</b>");
    };

    let caps = capabilities_for(provider);
    let agent_support = caps.support_for(LinkableResource::Agent);

    let user_display = agent_support
        .user_path
        .as_ref()
        .map(|p| format!("~/{}", p.display()))
        .unwrap_or_else(|| "-".to_string());
    let repo_display = agent_support
        .repo_path
        .as_ref()
        .map(|p| format!("<magenta>{}</magenta>", p.display()))
        .unwrap_or_else(|| "-".to_string());

    format!("<b>{provider_name} [ user:</b> {user_display}<b>, repo:</b> {repo_display} ]")
}

fn render_footer(
    term: &Terminal,
    has_exceptions: bool,
    applied_fix: bool,
    is_git_repo: bool,
    agent_count: usize,
    verbose: bool,
    filters: &[String],
) {
    let mut messages = Vec::new();

    if has_exceptions && !applied_fix {
        messages.push(
            "<dim><i>use <red>--fix</red> to attempt to fix the reported issues</i></dim>"
                .to_string(),
        );
    }

    if !is_git_repo {
        messages.push(
            "<dim><i>the current working directory is <b>not</b> a <b>git</b> repo so we are only showing user-based scope</i></dim>"
                .to_string(),
        );
    }

    if agent_count > 10 && !verbose {
        messages.push(
            "<dim><i>using the <green>--verbose</green> switch will provide not only names but also descriptions</i></dim>"
                .to_string(),
        );
    }

    if filters.is_empty() {
        messages.push(
            "<dim><i>using parameters in the CLI call will act as <b>filters</b> to help reduce the agents to only those you are interested in</i></dim>"
                .to_string(),
        );
    }

    if messages.is_empty() {
        return;
    }

    log::data("");

    if messages.len() == 1 {
        let rendered = Prose::new(messages.into_iter().next().unwrap()).render(term);
        log::data(&format!(" {rendered}"));
    } else {
        let mut list = UnorderedList::empty();
        for msg in messages {
            list.add(Prose::new(msg));
        }
        log::data(&list.render(term));
    }
}

fn scope_badge(scope: AgentScope) -> &'static str {
    match scope {
        AgentScope::User => &badges::USER_SCOPED,
        AgentScope::RepoMasked => &badges::MASKED_REPO_SCOPED,
        AgentScope::Repo => &badges::REPO_SCOPED,
    }
}

/// Extract body content after frontmatter.
fn extract_body(content: &str) -> &str {
    if let Some(rest) = content.strip_prefix("---")
        && let Some(end) = rest.find("\n---")
    {
        let after = &rest[end + 4..];
        return after.trim_start_matches('\n').trim_start_matches('\r');
    }
    content
}
