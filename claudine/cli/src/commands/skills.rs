use std::collections::{BTreeMap, BTreeSet};

use clap::Args;
use color_eyre::eyre::Result;

use biscuit_terminal::components::filesystem::FileSystem;
use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::WordWrap;
use claudine::badges;
use claudine::dispatch::loader::load_config;
use claudine::linking::{
    ExceptionType, LinkableResource, ProviderSkillPaths, ResourceScope, SkillDirectoryDiagnostic,
    SkillException, SkillFixSummary, SkillInfo, SkillScope, canonical_provider, capabilities_for,
    fix_missing_skills, list_skills,
};
use sniff::filesystem::git::detect_git;

use crate::log;

/// Arguments for the skills subcommand.
#[derive(Args)]
pub struct SkillsArgs {
    /// Only show skills matching these terms (fuzzy, case-insensitive).
    #[arg(value_name = "FILTER")]
    pub filter: Vec<String>,

    /// Fix missing skill links for non-Claude providers.
    #[arg(long, visible_alias = "fix")]
    pub apply: bool,
}

/// List available skills and their scopes.
pub fn run(args: SkillsArgs, verbose: bool) -> Result<()> {
    let paths = ProviderSkillPaths::new();

    // Apply fixes before listing if requested
    let fix_summary = if args.apply {
        Some(fix_missing_skills(&paths)?)
    } else {
        None
    };

    let mut report = list_skills(&paths, &args.filter)?;

    // Filter exceptions to match the same fuzzy filters applied to skills
    if !args.filter.is_empty() {
        let lowered: Vec<String> = args.filter.iter().map(|f| f.to_lowercase()).collect();
        report.exceptions.retain(|exc| {
            let topic_lower = exc.topic.to_lowercase();
            lowered.iter().any(|filter| topic_lower.contains(filter))
        });
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

    // Git repo detection
    let cwd = std::env::current_dir().unwrap_or_default();
    let is_git_repo = detect_git(&cwd, false, 1).ok().flatten().is_some();

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
        render_exceptions(&term, &report.exceptions, &report.diagnostics);
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
    let repo_canonical = if is_git_repo {
        canonical_provider(&linking_settings.canonical_provider, ResourceScope::Repo, LinkableResource::Skill)
    } else {
        None
    };

    let line = match (user_canonical, repo_canonical) {
        (Some(user), Some(repo)) => format!(
            "<blue><b>Canonical Providers:</b></blue> user: <b>{user}</b>, repo: <b>{repo}</b>"
        ),
        (Some(user), None) => format!(
            "<blue><b>Canonical Providers:</b></blue> user: <b>{user}</b>"
        ),
        (None, Some(repo)) => format!(
            "<blue><b>Canonical Providers:</b></blue> repo: <b>{repo}</b>"
        ),
        (None, None) => return,
    };

    log::data(&Prose::new(line).fallback_render(term));
    log::data("");
}

/// Detail mode: shown when exactly 1 skill matches. Shows badge/name/description + filesystem tree.
fn render_detail(term: &Terminal, skill: &SkillInfo) {
    let badge = scope_badge(skill.scope);
    let desc = skill.description.as_deref().unwrap_or("no description");
    let header = Prose::new(format!(
        r#"{badge} <a href="{}"><b>{}</b></a> <dim><i>{desc}</i></dim>"#,
        skill.skill_md_path.display(),
        skill.name,
    ));
    log::data(&header.fallback_render(term));
    log::data("");

    if let Some(dir) = skill.skill_md_path.parent()
        && let Some(dir_str) = dir.to_str()
        && let Ok(mut fs) = FileSystem::new_with_formatting(dir_str)
    {
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
        log::data(scope_badge(*scope));
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
                let is_missing = *exc_type == ExceptionType::Missing;
                let count = entries.len();
                let category_label =
                    Prose::new(format!("<b>{exc_type}</b> ({count})"));
                inner_list.add(category_label);

                // Show directory-level diagnostics before individual missing skills
                let mut detail_list = UnorderedList::empty();
                if is_missing
                    && let Some(provider_diags) = diag_by_provider.get(provider_name)
                {
                    for diag in provider_diags {
                        detail_list.add(Prose::new(diag.message.clone()));
                    }
                }

                let topics: Vec<String> = entries
                    .iter()
                    .map(|e| {
                        if is_missing {
                            e.topic.clone()
                        } else {
                            format!(
                                r#"<a href="{}">{}</a>"#,
                                e.skill_md_path.display(),
                                e.topic
                            )
                        }
                    })
                    .collect();

                let topic_line = Prose::new(topics.join(", "))
                    .with_word_wrap(WordWrap::BespokeProse(Some(500), vec![' ', ','], Some(2)));
                detail_list.add(topic_line);
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
        "<b>{provider_name}</b> [ <b>user:</b> {user_display}, <b>repo:</b> {repo_display} ]"
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
