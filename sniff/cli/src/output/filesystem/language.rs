use std::fmt::Write;
use std::path::Path;

use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::{RenderableTerminalContent, TerminalRenderable};
use biscuit_terminal::terminal::Terminal;
use sniff::filesystem::FrameworkStats;
use sniff::filesystem::repo::{Package, RepoInfo};

use super::format_number;
use super::resolve_dir;

/// Render the repository's primary programming language as plain text.
///
/// Returns the language name (e.g. `"Rust"`) followed by a newline,
/// suitable for piping. When no primary language can be determined,
/// returns an empty string; the caller in `output/mod.rs` treats an
/// empty result as "not detected" and exits with status `1` — the same
/// contract used by `repo root` / `repo package-root` / `repo package-area-root`.
///
/// ## Notes
///
/// The JSON path uses [`primary_language_name`] directly and emits
/// `{ "language": null }` with `exit_code = 1` for the same case.
pub fn render_repo_language(result: &sniff::SniffResult, _base_dir: Option<&Path>) -> String {
    primary_language_name(result)
        .map(|name| format!("{name}\n"))
        .unwrap_or_default()
}

/// Return the repository's primary programming language as a string, when known.
///
/// Used by both the text renderer and the JSON builder so they stay in sync.
pub(crate) fn primary_language_name(result: &sniff::SniffResult) -> Option<String> {
    result
        .filesystem
        .as_ref()
        .and_then(|fs| fs.languages.as_ref())
        .and_then(|langs| langs.primary)
        .map(|lang| lang.to_string())
}
/// Categorize a language name for display styling.
fn render_language_name(
    name: &impl std::fmt::Display,
    is_primary: bool,
    term: &Terminal,
) -> String {
    let markup = if is_primary {
        format!("<b>{}</b>", name)
    } else {
        name.to_string()
    };
    Prose::new(&markup).render(term)
}

fn render_language_usage(
    lang: &sniff::filesystem::languages::LanguageStats,
    term: &Terminal,
) -> String {
    Prose::new(format!(
        "{} direct, {} framework ({:.1}%)",
        format_number(lang.direct_file_count),
        format_number(lang.framework_file_count),
        lang.percentage
    ))
    .render(term)
}

pub(crate) fn render_framework_summary(frameworks: &[FrameworkStats]) -> String {
    frameworks
        .iter()
        .map(|framework| format!("{} ({})", framework.framework, framework.file_count))
        .collect::<Vec<_>>()
        .join(", ")
}
enum LanguageContext<'a> {
    MonorepoRoot(&'a RepoInfo),
    PackageArea(&'a RepoInfo, &'a str),
    Package(&'a RepoInfo, &'a Package),
    NonMonorepo,
}

fn resolve_language_context<'a>(
    result: &'a sniff::SniffResult,
    base_dir: Option<&Path>,
) -> LanguageContext<'a> {
    let dir = resolve_dir(base_dir);
    let repo = result.filesystem.as_ref().and_then(|fs| fs.repo.as_ref());

    let Some(repo) = repo else {
        return LanguageContext::NonMonorepo;
    };

    if !repo.is_monorepo {
        return LanguageContext::NonMonorepo;
    }

    if let Some(pkg) = repo.package_for_dir(&dir) {
        return LanguageContext::Package(repo, pkg);
    }

    if let Some(area) = repo.package_area_for_dir(&dir) {
        return LanguageContext::PackageArea(repo, area);
    }

    LanguageContext::MonorepoRoot(repo)
}

fn render_language_table_for_package(pkg: &Package, verbose: u8, term: &Terminal) -> String {
    use biscuit_terminal::components::table::{Table, TableCellContent, TableColumn};
    use biscuit_terminal::utils::layout::{Alignment, Margin};

    let mut out = String::new();

    if pkg.languages.is_empty() {
        let msg = Prose::new("<dim><i>No language data available</i></dim>").render(term);
        writeln!(out, "{}", msg).unwrap();
        return out;
    }

    if let Some(primary) = pkg.primary_language {
        let line = Prose::new(format!(
            "<b>{}</b> is the primary language in this package",
            primary
        ))
        .render(term);
        writeln!(out, "{}", line).unwrap();
    }
    if !pkg.secondary_languages.is_empty() {
        let line = Prose::new(format!(
            "Secondary languages: {}",
            pkg.secondary_languages
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ))
        .render(term);
        writeln!(out, "{}", line).unwrap();
    }

    let mut table = Table::new()
        .with_columns(vec![
            TableColumn::new("Language").with_min_width(16),
            TableColumn::new("Usage").with_alignment(Alignment::Right),
            TableColumn::new("Signal").with_alignment(Alignment::Right),
        ])
        .prefer_cursor_alignment();

    table.layout_mut().left_margin = Margin::Chars(1);
    table.layout_mut().top_margin = Margin::Chars(1);
    table.layout_mut().bottom_margin = Margin::Chars(1);

    for lang in &pkg.languages {
        table.add_row(vec![
            TableCellContent::Text(render_language_name(
                &lang.language,
                pkg.primary_language == Some(lang.language),
                term,
            )),
            TableCellContent::Text(render_language_usage(lang, term)),
            TableCellContent::Text(Prose::new(format!("{:.2}", lang.signal)).render(term)),
        ]);
    }

    writeln!(out).unwrap();
    write!(out, "{}", table.display(term)).unwrap();
    writeln!(out).unwrap();

    if verbose > 0 && !pkg.frameworks.is_empty() {
        writeln!(
            out,
            "Frameworks: {}",
            render_framework_summary(&pkg.frameworks)
        )
        .unwrap();
    }

    out
}

fn repo_display_name(result: &sniff::SniffResult) -> String {
    result
        .filesystem
        .as_ref()
        .and_then(|fs| fs.git.as_ref())
        .and_then(|git| git.repo.clone())
        .or_else(|| {
            result
                .filesystem
                .as_ref()
                .and_then(|fs| fs.git.as_ref())
                .map(|git| {
                    git.repo_root
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default()
                })
        })
        .unwrap_or_default()
}

pub fn render_language_section(
    result: &sniff::SniffResult,
    verbose: u8,
    base_dir: Option<&Path>,
) -> String {
    use biscuit_terminal::components::table::{Table, TableCellContent, TableColumn};
    use biscuit_terminal::utils::layout::{Alignment, Margin};

    let mut out = String::new();
    let term = Terminal::default();
    let ctx = resolve_language_context(result, base_dir);

    match ctx {
        LanguageContext::MonorepoRoot(repo) => {
            let packages = match repo.packages.as_ref() {
                Some(p) if !p.is_empty() => p,
                _ => return out,
            };
            let repo_name = repo_display_name(result);
            let header = Prose::new(format!(
                "In the <yellow>{}</yellow> monorepo, {} packages defined:",
                repo_name,
                format_number(packages.len())
            ))
            .render(&term);
            writeln!(out, "{}", header).unwrap();

            let mut items: Vec<RenderableTerminalContent> = Vec::new();
            for pkg in packages {
                let title = Prose::new(format!("<b>{}</b>", pkg.name)).render(&term);
                let body = render_language_table_for_package(pkg, verbose, &term);
                items.push(RenderableTerminalContent::String(title));
                items.push(RenderableTerminalContent::String(body));
            }

            let list = UnorderedList::from(items).with_indent_children(Some(4));
            write!(out, "{}", list.render(&term)).unwrap();
        }
        LanguageContext::PackageArea(repo, area) => {
            let packages = match repo.packages.as_ref() {
                Some(p) if !p.is_empty() => p,
                _ => return out,
            };
            let area_packages: Vec<&Package> =
                packages.iter().filter(|p| p.package_area == area).collect();
            let repo_name = repo_display_name(result);
            let header = Prose::new(format!(
                "In the <yellow>{}</yellow> monorepo's \"{}\" package area, {} packages defined:",
                repo_name,
                area,
                format_number(area_packages.len())
            ))
            .render(&term);
            writeln!(out, "{}", header).unwrap();

            let mut items: Vec<RenderableTerminalContent> = Vec::new();
            for pkg in &area_packages {
                let title = Prose::new(format!("<b>{}</b>", pkg.name)).render(&term);
                let body = render_language_table_for_package(pkg, verbose, &term);
                items.push(RenderableTerminalContent::String(title));
                items.push(RenderableTerminalContent::String(body));
            }

            let list = UnorderedList::from(items).with_indent_children(Some(4));
            write!(out, "{}", list.render(&term)).unwrap();
        }
        LanguageContext::Package(repo, pkg) => {
            let total = repo.packages.as_ref().map(|p| p.len()).unwrap_or(1);
            let repo_name = repo_display_name(result);
            let header = Prose::new(format!(
                "In the <yellow>{}</yellow> monorepo's <b>{}</b> package (<dim><i>1 of {} packages</i></dim>):",
                repo_name,
                pkg.name,
                format_number(total)
            ))
            .render(&term);
            writeln!(out, "{}", header).unwrap();

            let body = render_language_table_for_package(pkg, verbose, &term);
            let items = vec![RenderableTerminalContent::String(body)];
            let list = UnorderedList::from(items).with_indent_children(Some(4));
            write!(out, "{}", list.render(&term)).unwrap();
        }
        LanguageContext::NonMonorepo => {
            let langs = match result
                .filesystem
                .as_ref()
                .and_then(|fs| fs.languages.as_ref())
            {
                Some(l) => l,
                None => return out,
            };

            let repo_name = repo_display_name(result);
            let header = if repo_name.is_empty() {
                Prose::new(
                    "Evaluating programming languages:".to_string(),
                )
            } else {
                Prose::new(format!(
                    "Evaluating the <yellow>{}</yellow> (<dim><i>a non-monorepo repo</i></dim>) programming languages:",
                    repo_name
                ))
            }
            .render(&term);
            writeln!(out, "{}", header).unwrap();

            let primary = langs.primary;
            if let Some(primary) = primary {
                let line = Prose::new(format!(
                    "<b>{}</b> is the primary language in this repo",
                    primary
                ))
                .render(&term);
                let items = vec![RenderableTerminalContent::String(line)];
                let list = UnorderedList::from(items).with_indent_children(Some(4));
                write!(out, "{}", list.render(&term)).unwrap();
            }

            let mut table = Table::new()
                .with_columns(vec![
                    TableColumn::new("Language").with_min_width(16),
                    TableColumn::new("Usage").with_alignment(Alignment::Right),
                    TableColumn::new("Signal").with_alignment(Alignment::Right),
                ])
                .prefer_cursor_alignment();

            table.layout_mut().left_margin = Margin::Chars(1);
            table.layout_mut().top_margin = Margin::Chars(1);
            table.layout_mut().bottom_margin = Margin::Chars(1);

            for lang in &langs.languages {
                table.add_row(vec![
                    TableCellContent::Text(render_language_name(
                        &lang.language,
                        Some(lang.language) == primary,
                        &term,
                    )),
                    TableCellContent::Text(render_language_usage(lang, &term)),
                    TableCellContent::Text(Prose::new(format!("{:.2}", lang.signal)).render(&term)),
                ]);
            }

            writeln!(out).unwrap();
            write!(out, "{}", table.display(&term)).unwrap();
            writeln!(out).unwrap();

            if verbose > 0 && !langs.frameworks.is_empty() {
                writeln!(
                    out,
                    "Frameworks: {}",
                    render_framework_summary(&langs.frameworks)
                )
                .unwrap();
            }

            let footer = Prose::new(format!(
                "<dim><i>- analyzed </i></dim><yellow>{}</yellow><dim><i> files, </i></dim><yellow>{}</yellow><dim><i> contributed to language selection</i></dim>",
                format_number(langs.total_files_scanned),
                format_number(langs.total_language_files)
            ))
            .render(&term);
            writeln!(out, "{}", footer).unwrap();
        }
    }

    out
}
