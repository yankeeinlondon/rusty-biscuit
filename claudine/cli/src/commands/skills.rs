use std::collections::BTreeMap;

use clap::Args;
use color_eyre::eyre::Result;

use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::terminal::Terminal;
use claudine::badges;
use claudine::linking::{
    ExceptionType, ProviderSkillPaths, SkillException, SkillInfo, SkillScope, list_skills,
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
    let verbose = verbose || report.skills.len() < 10;

    if verbose {
        render_verbose(&term, &report.skills);
    } else {
        render_normal(&term, &report.skills);
    }

    if !report.exceptions.is_empty() {
        render_exceptions(&term, &report.exceptions);
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
            r#"{badge} <a href="{}"><b>{}</b></a> <dim><i>{desc}</i></dim>"#,
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
        log::data(&format!("\n{}", scope_badge(*scope)));

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
        let rendered = Prose::new(joined).fallback_render(term);
        log::data(&rendered);
        log::data("");
    }
}

/// Render exceptions grouped by (provider, exception_type).
fn render_exceptions(term: &Terminal, exceptions: &[SkillException]) {
    log::data("");
    let header = Prose::new("{{bold}}Exceptions{{reset}}");
    log::data(&format!(" {}", header.fallback_render(term)));

    // Group by provider
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

    let mut outer_list = UnorderedList::empty();

    for (provider_name, type_map) in &by_provider {
        let provider_label = Prose::new(format!("<b>{provider_name}</b>"));
        outer_list.add(provider_label);

        let mut inner_list = UnorderedList::empty();
        for (exc_type, entries) in type_map {
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

            let line = Prose::new(format!("<b>{exc_type}:</b> {}", topics.join(", ")));
            inner_list.add(line);
        }

        outer_list.add(inner_list);
    }

    log::data(&outer_list.fallback_render(term));
}

/// Return the rendered badge string for a scope.
fn scope_badge(scope: SkillScope) -> &'static str {
    match scope {
        SkillScope::User => &badges::USER_SCOPED,
        SkillScope::RepoMasked => &badges::MASKED_REPO_SCOPED,
        SkillScope::Repo => &badges::REPO_SCOPED,
    }
}
