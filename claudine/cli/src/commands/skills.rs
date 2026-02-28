use std::collections::{BTreeMap, BTreeSet};

use clap::Args;
use color_eyre::eyre::Result;

use biscuit_terminal::components::compose::Compose;
use biscuit_terminal::components::filesystem::FileSystem;
use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::{Layout, Margin, WordWrap};
use claudine::badges;
use claudine::dispatch::loader::load_config;
use claudine::linking::{
    ExceptionType, LinkableResource, ProviderSkillPaths, ResourceScope, SkillDirectoryDiagnostic,
    SkillException, SkillFilter, SkillFixSummary, SkillInfo, SkillScope, canonical_provider,
    capabilities_for, fix_missing_skills, list_skills,
};
use sniff::filesystem::git::detect_git;

use crate::log;

/// Arguments for the skills subcommand.
#[derive(Args)]
pub struct SkillsArgs {
    /// Filter skills by name. Supports negation (`-rust` or `!rust`) and exact match (`rust!`).
    #[arg(value_name = "FILTER")]
    pub filter: Vec<String>,

    /// Fix missing skill links for non-Claude providers.
    #[arg(long, visible_alias = "fix")]
    pub apply: bool,
}

/// List available skills and their scopes.
pub async fn run(args: SkillsArgs, verbose: bool) -> Result<()> {
    let paths = ProviderSkillPaths::new();

    // Git repo detection (needed early for init check)
    let cwd = std::env::current_dir().unwrap_or_default();
    let is_git_repo = detect_git(&cwd, false, 1).ok().flatten().is_some();

    // When --fix is used in a git repo, check if repo canonical provider needs initialization
    if is_git_repo && args.apply && repo_canonical_needs_init(&paths) {
        log::message("");
        log::message(
            "Repo canonical provider is not configured. Running claudine init --repo ...",
        );
        log::message("");
        super::init::run(super::init::InitArgs {
            quick: false,
            repo: true,
        })
        .await?;
        log::message("");
    }

    // Apply fixes before listing if requested
    let fix_summary = if args.apply {
        Some(fix_missing_skills(&paths)?)
    } else {
        None
    };

    let mut report = list_skills(&paths, &args.filter)?;

    // Filter exceptions to match the same filters applied to skills
    if !args.filter.is_empty() {
        let filters = SkillFilter::parse_all(&args.filter);
        if !filters.is_empty() {
            report
                .exceptions
                .retain(|exc| SkillFilter::retain(&filters, &exc.topic));
        }
        report.diagnostics.clear();
    }

    if report.skills.is_empty() {
        if args.filter.is_empty() {
            log::data("No skills found.");
        } else {
            log::data(&format!(
                "No skills matching: {}",
                args.filter.join(", ")
            ));
        }
        return Ok(());
    }

    let term = Terminal::new();

    // Header
    let header = Prose::new("<blue><b>Skills</b></blue>").fallback_render(&term);
    log::data("");
    log::data(&header);
    log::data(
        &Prose::new("<blue>==================</blue>").fallback_render(&term),
    );
    log::data("");

    // Canonical providers line
    render_canonical_providers(&term, &paths, is_git_repo);

    let skill_count = report.skills.len();

    // Three-mode rendering
    if skill_count == 1 {
        render_detail(&term, &report.skills[0]);
    } else if skill_count < 6 || verbose {
        render_verbose(&term, &report.skills);
    } else {
        render_normal(&term, &report.skills);
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
        skill_count,
        verbose,
        &args.filter,
    );

    Ok(())
}

/// Check whether repo-scoped canonical provider needs initialization.
///
/// Returns `true` when a config exists (user or merged) but has no
/// repo-scoped canonical skill provider configured.  Returns `false`
/// when no config exists at all (user needs `claudine init` first).
fn repo_canonical_needs_init(paths: &ProviderSkillPaths) -> bool {
    let repo_root = Some(paths.repo_root());
    match load_config(None, repo_root) {
        Ok(config) => match &config.settings.linking {
            Some(linking) => canonical_provider(
                &linking.canonical_provider,
                ResourceScope::Repo,
                LinkableResource::Skill,
            )
            .is_none(),
            None => true,
        },
        Err(_) => false,
    }
}

/// Render the canonical providers line from config (if available).
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

    let user_canonical =
        canonical_provider(&linking_settings.canonical_provider, ResourceScope::User, LinkableResource::Skill);

    let not_configured = "<i><red>not configured</red></i>";

    let user_part = match user_canonical {
        Some(user) => format!("user: <b>{user}</b>"),
        None => format!("user: {not_configured}"),
    };

    let line = if is_git_repo {
        let repo_canonical = canonical_provider(
            &linking_settings.canonical_provider,
            ResourceScope::Repo,
            LinkableResource::Skill,
        );
        let repo_part = match repo_canonical {
            Some(repo) => format!("repo: <b>{repo}</b>"),
            None => format!("repo: {not_configured}"),
        };
        format!("<blue><b>Canonical Providers:</b></blue> {user_part}, {repo_part}")
    } else {
        format!("<blue><b>Canonical Providers:</b></blue> {user_part}")
    };

    log::data(&Prose::new(line).fallback_render(term));
    log::data("");
}

/// Detail mode: shown when exactly 1 skill matches. Shows name/badge, description, + filesystem tree.
fn render_detail(term: &Terminal, skill: &SkillInfo) {
    let badge = scope_badge(skill.scope);
    let desc = skill.description.as_deref().unwrap_or("no description");

    let name_line = Prose::new(format!(
        r#"<a href="{}"><b>{}</b></a> {badge}"#,
        skill.skill_md_path.display(),
        skill.name,
    ));
    log::data(&name_line.fallback_render(term));

    let desc_line = Prose::new(format!("<dim><i>{desc}</i></dim>"))
        .with_word_wrap(WordWrap::BespokeProse(None, vec![' '], None));
    log::data(&desc_line.fallback_render(term));
    log::data("");

    if let Some(dir) = skill.skill_md_path.parent()
        && let Some(dir_str) = dir.to_str()
        && let Ok(mut fs) = FileSystem::new_with_formatting(dir_str)
    {
        let layout = Layout {
            left_margin: Margin::Chars(2),
            ..Layout::default()
        };
        fs = fs.show_tokens().with_file_links().layout(layout);
        fs.ensure_tree_built();
        log::data(&fs.fallback_render(term));
    }
}

/// Verbose mode: single list with badge, name (as link), and description.
fn render_verbose(term: &Terminal, skills: &[SkillInfo]) {
    let mut list = UnorderedList::empty();

    for skill in skills {
        let badge = scope_badge(skill.scope);
        let desc = skill
            .description
            .as_deref()
            .unwrap_or("no description");
        let item = Prose::new(format!(
            r#"<a href="{}"><b>{}</b></a> {badge} <dim><i>{desc}</i></dim>"#,
            skill.skill_md_path.display(),
            skill.name,
        ));
        list.add(item);
    }

    log::data(&list.fallback_render(term));
}

/// Normal mode: group skills by scope, show badge header + tab-delimited names.
fn render_normal(term: &Terminal, skills: &[SkillInfo]) {
    let mut by_scope: BTreeMap<SkillScope, Vec<&SkillInfo>> = BTreeMap::new();
    for skill in skills {
        by_scope.entry(skill.scope).or_default().push(skill);
    }

    for (scope, group) in &by_scope {
        let count = group.len();
        let badge_line = format!("{} <dim>(<i>{count}</i>)</dim>", scope_badge(*scope));
        log::data(&Prose::new(badge_line).fallback_render(term));
        log::data("");

        let names: Vec<String> = group
            .iter()
            .map(|s| {
                format!(
                    r#"<a href="{}"><b>{}</b></a>"#,
                    s.skill_md_path.display(),
                    s.name
                )
            })
            .collect();

        let joined = names.join("  ");
        let rendered = Prose::new(joined)
            .with_word_wrap(WordWrap::BespokeProse(Some(50), vec![' '], None))
            .fallback_render(term);
        log::data(&rendered);
        log::data("");
    }
}

/// Render exceptions grouped by (provider, exception_type) with directory diagnostics.
fn render_exceptions(
    term: &Terminal,
    exceptions: &[SkillException],
    diagnostics: &[SkillDirectoryDiagnostic],
    verbose: bool,
) {
    log::data("");
    log::data(&badges::EXCEPTIONS);
    log::data("");

    // Group exceptions by provider
    let mut by_provider: BTreeMap<String, BTreeMap<ExceptionType, Vec<&SkillException>>> =
        BTreeMap::new();
    for exc in exceptions {
        by_provider
            .entry(exc.provider.to_string())
            .or_default()
            .entry(exc.exception_type)
            .or_default()
            .push(exc);
    }

    // Group diagnostics by provider
    let mut diag_by_provider: BTreeMap<String, Vec<&SkillDirectoryDiagnostic>> = BTreeMap::new();
    for diag in diagnostics {
        diag_by_provider
            .entry(diag.provider.to_string())
            .or_default()
            .push(diag);
    }

    // Collect all provider names that have exceptions OR diagnostics
    let mut all_providers: BTreeSet<String> = by_provider.keys().cloned().collect();
    for key in diag_by_provider.keys() {
        all_providers.insert(key.clone());
    }

    let mut outer_list = UnorderedList::empty();

    for provider_name in &all_providers {
        // Build provider header with skill paths
        let provider_header = build_provider_header(provider_name);
        outer_list.add(Prose::new(provider_header));

        let mut inner_list = UnorderedList::empty();

        if let Some(type_map) = by_provider.get(provider_name) {
            for (exc_type, entries) in type_map {
                let count = entries.len();
                let category_label =
                    Prose::new(format!("<b>{exc_type}</b> ({count})"));
                inner_list.add(category_label);

                let mut detail_list = UnorderedList::empty();

                match exc_type {
                    ExceptionType::Missing => {
                        // Show directory-level diagnostics before individual missing skills
                        if let Some(provider_diags) = diag_by_provider.get(provider_name) {
                            for diag in provider_diags {
                                detail_list.add(Prose::new(diag.message.clone()));
                            }
                        }
                        let topics: Vec<String> =
                            entries.iter().map(|e| e.topic.clone()).collect();
                        let topic_line = Prose::new(topics.join(", ")).with_word_wrap(
                            WordWrap::BespokeProse(Some(500), vec![' ', ','], Some(2)),
                        );
                        detail_list.add(topic_line);
                    }
                    ExceptionType::Invalid => {
                        // Each topic as its own bullet with missing property details
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
                                    e.skill_md_path.display(),
                                    e.topic
                                )
                            } else {
                                format!(
                                    r#"<b><a href="{}">{}</a></b> (<i>missing the properties {}</i>)"#,
                                    e.skill_md_path.display(),
                                    e.topic,
                                    props_markup
                                )
                            };
                            detail_list.add(Prose::new(label));
                        }
                    }
                    ExceptionType::BrokenLink => {
                        // Group broken links by topic, then list each broken link underneath
                        let mut by_topic: BTreeMap<&str, Vec<&SkillException>> = BTreeMap::new();
                        for e in entries {
                            by_topic.entry(&e.topic).or_default().push(e);
                        }
                        for (topic, topic_entries) in &by_topic {
                            let first = topic_entries[0];
                            let topic_label = format!(
                                r#"<b><a href="{}">{}</a></b>"#,
                                first.skill_md_path.display(),
                                topic
                            );
                            detail_list.add(Prose::new(topic_label));

                            let mut link_list = UnorderedList::empty();
                            for e in topic_entries {
                                let link_text = e.link_text.as_deref().unwrap_or("?");
                                let link_target = e.link_target.as_deref().unwrap_or("?");
                                let source_file = e
                                    .skill_md_path
                                    .file_name()
                                    .and_then(|f| f.to_str())
                                    .unwrap_or("SKILL.md");
                                let msg = format!(
                                    "<dim><i>in the file <blue>{source_file}</blue> the link \
                                     <orange>[{link_text}]</orange><red>({link_target})</red> \
                                     uses an invalid file reference!</i></dim>"
                                );
                                link_list.add(Prose::new(msg));
                            }
                            // In verbose mode, compose broken link messages with file tree
                            if verbose
                                && let Some(dir) = first.skill_md_path.parent()
                                && let Some(dir_str) = dir.to_str()
                                && let Ok(mut fs) =
                                    FileSystem::new_with_formatting(dir_str)
                            {
                                let layout = Layout {
                                    left_margin: Margin::Chars(4),
                                    ..Layout::default()
                                };
                                fs = fs.show_tokens().with_file_links().layout(layout);
                                fs.ensure_tree_built();
                                let mut composed = Compose::default();
                                composed
                                    .add_unordered_list(link_list)
                                    .add_text("\n")
                                    .add_file_system(fs)
                                    .add_text("\n");
                                detail_list.add(composed);
                            } else {
                                detail_list.add(link_list);
                            }
                        }
                    }
                    ExceptionType::NoLinks => {
                        // Comma-separated linked topic names
                        let topics: Vec<String> = entries
                            .iter()
                            .map(|e| {
                                format!(
                                    r#"<a href="{}">{}</a>"#,
                                    e.skill_md_path.display(),
                                    e.topic
                                )
                            })
                            .collect();
                        let topic_line = Prose::new(topics.join(", ")).with_word_wrap(
                            WordWrap::BespokeProse(Some(500), vec![' ', ','], Some(2)),
                        );
                        detail_list.add(topic_line);
                    }
                }

                inner_list.add(detail_list);
            }
        } else if let Some(provider_diags) = diag_by_provider.get(provider_name) {
            // Provider has only diagnostics, no individual exceptions
            let category_label = Prose::new("<b>missing</b> (0)".to_string());
            inner_list.add(category_label);

            let mut detail_list = UnorderedList::empty();
            for diag in provider_diags {
                detail_list.add(Prose::new(diag.message.clone()));
            }
            inner_list.add(detail_list);
        }

        outer_list.add(inner_list);
    }

    log::data(&outer_list.fallback_render(term));
}

/// Render the summary of fix operations.
fn render_fix_summary(term: &Terminal, summary: &SkillFixSummary) {
    log::data("");
    let header = Prose::new("<b>Fix Summary</b>").fallback_render(term);
    log::data(&header);

    let parts = [
        format!("directories_created={}", summary.directories_created),
        format!("links_created={}", summary.links_created),
        format!("already_linked={}", summary.already_linked),
        format!("skipped={}", summary.skipped),
        format!("names_inserted={}", summary.names_inserted),
    ];
    let detail = Prose::new(format!("<dim>{}</dim>", parts.join(", ")));
    log::data(&format!(" {}", detail.fallback_render(term)));
}

/// Build the provider header line with user/repo skill paths.
fn build_provider_header(provider_name: &str) -> String {
    use claudine::events::Provider;

    let Some(provider) = Provider::fuzzy_match_cli_name(provider_name) else {
        return format!("<b>{provider_name}</b>");
    };

    let caps = capabilities_for(provider);
    let skill_support = caps.support_for(LinkableResource::Skill);

    let user_display = skill_support
        .user_path
        .as_ref()
        .map(|p| format!("~/{}", p.display()))
        .unwrap_or_else(|| "-".to_string());
    let repo_display = skill_support
        .repo_path
        .as_ref()
        .map(|p| format!("<magenta>{}</magenta>", p.display()))
        .unwrap_or_else(|| "-".to_string());

    format!(
        "<b>{provider_name} [ user:</b> {user_display}<b>, repo:</b> {repo_display} ]",
    )
}

/// Render footer messages based on current state.
fn render_footer(
    term: &Terminal,
    has_exceptions: bool,
    applied_fix: bool,
    is_git_repo: bool,
    skill_count: usize,
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

    if skill_count > 10 && !verbose {
        messages.push(
            "<dim><i>using the <green>--verbose</green> switch will provide not only topic names but also descriptions</i></dim>"
                .to_string(),
        );
    }

    if filters.is_empty() {
        messages.push(
            "<dim><i>using parameters in the CLI call will act as <b>filters</b> to help reduce the skills to only those you are interested in</i></dim>"
                .to_string(),
        );
    }

    if messages.is_empty() {
        return;
    }

    log::data("");

    if messages.len() == 1 {
        let rendered = Prose::new(messages.into_iter().next().unwrap()).fallback_render(term);
        log::data(&format!(" {rendered}"));
    } else {
        let mut list = UnorderedList::empty();
        for msg in messages {
            list.add(Prose::new(msg));
        }
        log::data(&list.fallback_render(term));
    }
}

/// Return the rendered badge string for a scope.
fn scope_badge(scope: SkillScope) -> &'static str {
    match scope {
        SkillScope::User => &badges::USER_SCOPED,
        SkillScope::RepoMasked => &badges::MASKED_REPO_SCOPED,
        SkillScope::Repo => &badges::REPO_SCOPED,
    }
}
