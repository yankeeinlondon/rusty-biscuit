use std::collections::{BTreeMap, BTreeSet};

use clap::Args;
use color_eyre::eyre::Result;

use biscuit_terminal::components::compose::Compose;
use biscuit_terminal::components::filesystem::FileSystem;
use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::{Layout, Length, Edges, WordWrap};
use claudine::badges;
use claudine::linking::{
    ExceptionType, LinkableResource, ProviderSkillPaths, SkillDirectoryDiagnostic, SkillException,
    SkillFilter, SkillFixSummary, SkillInfo, SkillScope, fix_missing_skills, list_skills,
};
use sniff::filesystem::git::detect_git;

use super::link_display::{
    LinkableResourceDisplay, build_provider_header, render_canonical_providers, render_footer,
    render_normal, render_verbose, repo_canonical_needs_init,
};
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
    if is_git_repo && args.apply && repo_canonical_needs_init(&paths, LinkableResource::Skill) {
        log::message("");
        log::message("Repo canonical provider is not configured.");
        log::message("Run `claudine config` and set the repo provider in the Preferences tab.");
        log::message("");
        return Ok(());
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
            log::data(&format!("No skills matching: {}", args.filter.join(", ")));
        }
        return Ok(());
    }

    let term = crate::log::terminal();

    // Header
    let header = Prose::new("<blue><b>Skills</b></blue>").render(&term);
    log::data("");
    log::data(&header);
    log::data(&Prose::new("<blue>==================</blue>").render(&term));
    log::data("");

    // Canonical providers line
    render_canonical_providers(&term, &paths, is_git_repo, LinkableResource::Skill);

    let skill_count = report.skills.len();

    // Three-mode rendering
    if skill_count == 1 {
        render_detail(&term, &report.skills[0]);
    } else if skill_count < 6 || verbose {
        render_verbose(&term, &report.skills, scope_badge);
    } else {
        render_normal(&term, &report.skills, scope_badge);
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
        "skills",
        "<dim><i>using the <green>--verbose</green> switch will provide not only topic names but also descriptions</i></dim>",
    );

    Ok(())
}

/// Detail mode: shown when exactly 1 skill matches. Shows name/badge, description, + filesystem tree.
fn render_detail(term: &Terminal, skill: &SkillInfo) {
    let badge = scope_badge(skill.scope);
    let desc = skill.description.as_deref().unwrap_or("no description");

    let name_line = Prose::new(format!(
        r#"<a href="{}"><b>{}</b></a> {badge}"#,
        biscuit_file::to_portable_string(&skill.skill_md_path),
        skill.name,
    ));
    log::data(&name_line.render(term));

    let desc_line = Prose::new(format!("<dim><i>{desc}</i></dim>"))
        .with_word_wrap(WordWrap::BespokeProse(None, vec![' ', '-', '_'], None));
    log::data(&desc_line.render(term));
    log::data("");

    if let Some(dir) = skill.skill_md_path.parent()
        && let Some(dir_str) = dir.to_str()
        && let Ok(mut fs) = FileSystem::new_with_formatting(dir_str)
    {
        let layout = Layout {
            margin: Edges::x(Length::ch(2)),
            ..Layout::default()
        };
        fs = fs.show_tokens().with_file_links().layout(layout);
        fs.ensure_tree_built();
        log::data(&fs.render(term));
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
        let provider_header = build_provider_header(provider_name, LinkableResource::Skill);
        outer_list.add(Prose::new(provider_header));

        let mut inner_list = UnorderedList::empty();

        if let Some(type_map) = by_provider.get(provider_name) {
            for (exc_type, entries) in type_map {
                let count = entries.len();
                let category_label = Prose::new(format!("<b>{exc_type}</b> ({count})"));
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
                        let topics: Vec<String> = entries.iter().map(|e| e.topic.clone()).collect();
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
                                    biscuit_file::to_portable_string(&e.skill_md_path),
                                    e.topic
                                )
                            } else {
                                format!(
                                    r#"<b><a href="{}">{}</a></b> (<i>missing the properties {}</i>)"#,
                                    biscuit_file::to_portable_string(&e.skill_md_path),
                                    e.topic,
                                    props_markup
                                )
                            };
                            detail_list.add(Prose::new(label));
                        }
                    }
                    ExceptionType::YamlTabs => {
                        for e in entries {
                            let label = format!(
                                r#"<b><a href="{}">{}</a></b> (<i>contains tab characters in YAML indentation</i>)"#,
                                biscuit_file::to_portable_string(&e.skill_md_path),
                                e.topic
                            );
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
                                biscuit_file::to_portable_string(&first.skill_md_path),
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
                                && let Ok(mut fs) = FileSystem::new_with_formatting(dir_str)
                            {
                                let layout = Layout {
                                    margin: Edges::x(Length::ch(4)),
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
                                    biscuit_file::to_portable_string(&e.skill_md_path),
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

    log::data(&outer_list.render(term));
}

/// Render the summary of fix operations.
fn render_fix_summary(term: &Terminal, summary: &SkillFixSummary) {
    log::data("");
    let header = Prose::new("<b>Fix Summary</b>").render(term);
    log::data(&header);

    let parts = [
        format!("directories_created={}", summary.directories_created),
        format!("links_created={}", summary.links_created),
        format!("already_linked={}", summary.already_linked),
        format!("skipped={}", summary.skipped),
        format!("names_inserted={}", summary.names_inserted),
        format!("yaml_tabs_fixed={}", summary.yaml_tabs_fixed),
        format!("not_shareable={}", summary.not_shareable),
    ];
    let detail = Prose::new(format!("<dim>{}</dim>", parts.join(", ")));
    log::data(&format!(" {}", detail.render(term)));
}

/// Return the rendered badge string for a scope.
fn scope_badge(scope: SkillScope) -> &'static str {
    match scope {
        SkillScope::User => &badges::USER_SCOPED,
        SkillScope::RepoMasked => &badges::MASKED_REPO_SCOPED,
        SkillScope::Repo => &badges::REPO_SCOPED,
    }
}

impl LinkableResourceDisplay for SkillInfo {
    type Scope = SkillScope;

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
        &self.skill_md_path
    }
}
