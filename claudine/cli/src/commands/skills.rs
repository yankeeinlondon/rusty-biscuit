use std::collections::{BTreeMap, BTreeSet};

use clap::Args;
use color_eyre::eyre::Result;

use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::WordWrap;
use claudine::badges;
use claudine::linking::{
    ExceptionType, ProviderSkillPaths, SkillDirectoryDiagnostic, SkillException, SkillInfo,
    SkillScope, capabilities_for, list_skills, LinkableResource,
};

use crate::log;

/// Arguments for the skills subcommand.
#[derive(Args)]
pub struct SkillsArgs {
    /// Only show skills matching these terms (fuzzy, case-insensitive).
    #[arg(value_name = "FILTER")]
    pub filter: Vec<String>,
}

/// List available skills and their scopes.
pub fn run(args: SkillsArgs, verbose: bool) -> Result<()> {
    let paths = ProviderSkillPaths::new();
    let report = list_skills(&paths, &args.filter)?;

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
    let header = Prose::new("<b>Skills</b>").fallback_render(&term);
    log::data("");
    log::data(&header);
    log::data("==================");
    log::data("");

    let verbose = verbose || report.skills.len() < 10;

    if verbose {
        render_verbose(&term, &report.skills);
    } else {
        render_normal(&term, &report.skills);
    }

    if !report.exceptions.is_empty() || !report.diagnostics.is_empty() {
        render_exceptions(&term, &report.exceptions, &report.diagnostics);

        log::data("");
        let fix_hint = Prose::new(
            "<dim><i>use <red>--fix</red> to attempt to fix the reported issues</i></dim>",
        );
        log::data(&format!(" {}", fix_hint.fallback_render(&term)));
    }

    Ok(())
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
    log::data(&*badges::EXCEPTIONS);
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
                if is_missing {
                    if let Some(provider_diags) = diag_by_provider.get(provider_name) {
                        for diag in provider_diags {
                            detail_list.add(Prose::new(diag.message.clone()));
                        }
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

/// Return the rendered badge string for a scope.
fn scope_badge(scope: SkillScope) -> &'static str {
    match scope {
        SkillScope::User => &badges::USER_SCOPED,
        SkillScope::RepoMasked => &badges::MASKED_REPO_SCOPED,
        SkillScope::Repo => &badges::REPO_SCOPED,
    }
}
