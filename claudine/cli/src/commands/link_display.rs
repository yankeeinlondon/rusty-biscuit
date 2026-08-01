use std::collections::BTreeMap;
use std::path::Path;

use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::layout::WordWrap;
use claudine::linking::{LinkableResource, ProviderSkillPaths, capabilities_for};

use crate::log;

pub(crate) trait LinkableResourceDisplay {
    type Scope: Ord + Copy;

    fn scope(&self) -> Self::Scope;
    fn name(&self) -> &str;
    fn description(&self) -> Option<&str>;
    fn path(&self) -> &Path;

    fn verbose_badge(&self) -> Option<&'static str> {
        None
    }
}

fn resource_name_markup(item: &impl LinkableResourceDisplay) -> String {
    crate::cli_utils::file_url(item.path()).map_or_else(
        || format!("<b>{}</b>", item.name()),
        |href| format!(r#"<a href="{href}"><b>{}</b></a>"#, item.name()),
    )
}

pub(crate) fn repo_canonical_needs_init(
    paths: &ProviderSkillPaths,
    _resource: LinkableResource,
) -> bool {
    let repo_path = paths.repo_root().join(".claudine").join("config.json");
    if !repo_path.exists() {
        return true;
    }
    match claudine::dispatch::loader::load_repo_override_config(&repo_path) {
        Ok(Some(config)) => config.canonical_provider.is_none(),
        Ok(None) | Err(_) => true,
    }
}

pub(crate) fn render_canonical_providers(
    term: &Terminal,
    paths: &ProviderSkillPaths,
    is_git_repo: bool,
    _resource: LinkableResource,
) {
    let user_config = claudine::dispatch::loader::load_claudine_config(None, None).ok();
    let user_canonical = user_config.as_ref().and_then(|c| c.canonical_provider);

    let not_configured = "<i><red>not configured</red></i>";

    let user_part = match user_canonical {
        Some(p) => format!("user: <b>{p}</b>"),
        None => format!("user: {not_configured}"),
    };

    let line = if is_git_repo {
        let repo_path = paths.repo_root().join(".claudine").join("config.json");
        let repo_config = claudine::dispatch::loader::load_repo_override_config(&repo_path)
            .ok()
            .flatten();
        let repo_canonical = repo_config.as_ref().and_then(|c| c.canonical_provider);
        let repo_part = match repo_canonical {
            Some(p) => format!("repo: <b>{p}</b>"),
            None => format!("repo: {not_configured}"),
        };
        format!("<blue><b>Canonical Provider:</b></blue> {user_part}, {repo_part}")
    } else {
        format!("<blue><b>Canonical Provider:</b></blue> {user_part}")
    };

    log::data(&Prose::new(line).render(term));
    log::data("");
}

pub(crate) fn render_verbose<T, F>(term: &Terminal, items: &[T], scope_badge: F)
where
    T: LinkableResourceDisplay,
    F: Fn(T::Scope) -> &'static str,
{
    let mut list = UnorderedList::empty();

    for item in items {
        let badge = scope_badge(item.scope());
        let desc = item.description().unwrap_or("no description");
        let format_badge = item.verbose_badge().unwrap_or("");
        let format_badge = if format_badge.is_empty() {
            String::new()
        } else {
            format!(" {format_badge}")
        };
        let name = resource_name_markup(item);
        let item = Prose::new(format!(
            "{name} {badge}{format_badge} <dim><i>{desc}</i></dim>"
        ));
        list.add(item);
    }

    log::data(&list.render(term));
}

pub(crate) fn render_normal<T, F>(term: &Terminal, items: &[T], scope_badge: F)
where
    T: LinkableResourceDisplay,
    F: Fn(T::Scope) -> &'static str,
{
    let mut by_scope: BTreeMap<T::Scope, Vec<&T>> = BTreeMap::new();
    for item in items {
        by_scope.entry(item.scope()).or_default().push(item);
    }

    for (scope, group) in &by_scope {
        let count = group.len();
        let badge_line = format!("{} <dim>(<i>{count}</i>)</dim>", scope_badge(*scope));
        log::data(&Prose::new(badge_line).render(term));
        log::data("");

        let names: Vec<String> = group
            .iter()
            .map(|item| resource_name_markup(*item))
            .collect();

        let rendered = Prose::new(names.join("  "))
            .with_word_wrap(WordWrap::BespokeProse(Some(50), vec![' '], None))
            .render(term);
        log::data(&rendered);
        log::data("");
    }
}

pub(crate) fn build_provider_header(provider_name: &str, resource: LinkableResource) -> String {
    let Some(provider) = claudine::provider::Provider::fuzzy_match_cli_name(provider_name) else {
        return format!("<b>{provider_name}</b>");
    };

    let capabilities = capabilities_for(provider);
    let support = capabilities.support_for(resource);
    let user_display = support
        .user_path
        .as_ref()
        .map(|p| format!("~/{}", biscuit_file::to_portable_string(p)))
        .unwrap_or_else(|| "-".to_string());
    let repo_display = support
        .repo_path
        .as_ref()
        .map(|p| {
            format!(
                "<magenta>{}</magenta>",
                biscuit_file::to_portable_string(p)
            )
        })
        .unwrap_or_else(|| "-".to_string());

    format!("<b>{provider_name} [ user:</b> {user_display}<b>, repo:</b> {repo_display} ]")
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn render_footer(
    term: &Terminal,
    has_exceptions: bool,
    applied_fix: bool,
    is_git_repo: bool,
    item_count: usize,
    verbose: bool,
    filters: &[String],
    resource_plural: &str,
    verbose_hint: &str,
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

    if item_count > 10 && !verbose {
        messages.push(verbose_hint.to_string());
    }

    if filters.is_empty() {
        messages.push(format!(
            "<dim><i>using parameters in the CLI call will act as <b>filters</b> to help reduce the {resource_plural} to only those you are interested in</i></dim>"
        ));
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
