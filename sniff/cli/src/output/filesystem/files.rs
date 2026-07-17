//! File-association breakdown rendering and path-list rendering.

use std::fmt::Write;
use std::path::{Path, PathBuf};

use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use sniff::filesystem::{FileAssociationBreakdown, FileAssociationStats};

use super::language::render_framework_summary;
use super::path_format::{format_basename_filepath, format_styled_filepath};
use crate::args::FilesFilter;

pub(crate) fn filter_file_breakdown(
    breakdown: &FileAssociationBreakdown,
    filter: &FilesFilter,
) -> FileAssociationBreakdown {
    let Some(association) = filter.association else {
        return breakdown.clone();
    };

    let by_association: Vec<FileAssociationStats> = breakdown
        .by_association
        .iter()
        .filter(|stats| stats.association == association)
        .cloned()
        .collect();
    let total_files = by_association.iter().map(|stats| stats.file_count).sum();

    FileAssociationBreakdown {
        total_files,
        by_association,
        by_language: if association == sniff::filesystem::FileAssociation::ProgrammingLanguage {
            breakdown.by_language.clone()
        } else {
            Vec::new()
        },
        by_framework: if association == sniff::filesystem::FileAssociation::FrameworkFile {
            breakdown.by_framework.clone()
        } else {
            Vec::new()
        },
        // An association filter narrows what is shown; it cannot make a
        // truncated observation complete.
        truncated: breakdown.truncated,
        limit: breakdown.limit,
    }
}

pub fn render_files_section(
    files: &FileAssociationBreakdown,
    verbose: u8,
    filter: &FilesFilter,
) -> String {
    use biscuit_terminal::components::table::{Table, TableCellContent, TableColumn};
    use biscuit_terminal::utils::layout::{Alignment, Length, TargetValue};

    let mut out = String::new();
    let filtered = filter_file_breakdown(files, filter);
    let term = Terminal::default();

    let mut table = Table::new()
        .with_columns(vec![
            TableColumn::new("Association").with_min_width(18),
            TableColumn::new("Count").with_alignment(Alignment::Right),
        ])
        .prefer_cursor_alignment();
    table.layout_mut().margin.left = TargetValue::universal(Length::ch(1));
    table.layout_mut().margin.top = TargetValue::universal(Length::ch(1));
    table.layout_mut().margin.bottom = TargetValue::universal(Length::ch(1));

    for stats in &filtered.by_association {
        table.add_row(vec![
            TableCellContent::Text(stats.association.to_string()),
            TableCellContent::Text(format!("{} ({:.1}%)", stats.file_count, stats.percentage)),
        ]);
    }

    writeln!(out).unwrap();
    write!(out, "{}", table.display(&term)).unwrap();
    writeln!(out).unwrap();

    if verbose > 0 && !filtered.by_framework.is_empty() {
        writeln!(
            out,
            "Frameworks: {}",
            render_framework_summary(&filtered.by_framework)
        )
        .unwrap();
    }
    if verbose > 0 && !filtered.by_language.is_empty() {
        writeln!(
            out,
            "Languages: {}",
            filtered
                .by_language
                .iter()
                .map(|language| format!("{} ({})", language.language, language.total_file_count))
                .collect::<Vec<_>>()
                .join(", ")
        )
        .unwrap();
    }

    out
}

// ---------------------------------------------------------------------------
// Shared path-list renderer
// ---------------------------------------------------------------------------

/// Output format for path lists.
pub enum PathListFormat {
    /// One path per line (default).
    Lines,
    /// Bullet list with `- ` prefix.
    BulletList,
    /// Comma-separated on a single line.
    Csv,
}

/// Render a list of repo-relative paths in the chosen format.
///
/// Paths are displayed with OSC8 hyperlinks (absolute target), dim directory
/// segments, and bold basenames. With `no_path`, only the basename is shown.
pub fn render_path_list(
    repo_root: &Path,
    paths: &[PathBuf],
    format: PathListFormat,
    no_path: bool,
) -> String {
    let terminal = Terminal::default();

    let format_one = |p: &PathBuf| -> String {
        let relative = p.display().to_string();
        let absolute = repo_root.join(p).display().to_string();
        let markup = if no_path {
            format_basename_filepath(&relative, &absolute)
        } else {
            format_styled_filepath(&relative, &absolute)
        };
        Prose::new(&markup).render(&terminal)
    };

    match format {
        PathListFormat::Lines => {
            let mut out = String::new();
            for p in paths {
                writeln!(out, "{}", format_one(p)).unwrap();
            }
            out
        }
        PathListFormat::BulletList => {
            let items: Vec<String> = paths.iter().map(format_one).collect();
            let list = UnorderedList::new(items);
            let mut out = String::new();
            writeln!(out, "{}", list.render(&terminal)).unwrap();
            out
        }
        PathListFormat::Csv => {
            let items: Vec<String> = paths.iter().map(format_one).collect();
            let mut out = items.join(", ");
            out.push('\n');
            out
        }
    }
}
