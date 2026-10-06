//! File-association breakdown rendering and path-list rendering.
//!
//! A filtered verbose report (`sniff files --association <cat> -v`) follows
//! its summary table, any incomplete-scan notice, and this module's file list
//! (see [`super::file_list`]) with the captured matching paths; language and
//! framework details come after the list. The list itself renders only when
//! the caller supplies the link root the captured paths are relative to.

use std::fmt::Write;
use std::path::{Path, PathBuf};

use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::{InlineProse, Prose};
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use sniff::filesystem::{FileAssociationBreakdown, FileAssociationStats};

use super::file_list::render_file_list;
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

/// Renders the file-association summary table and, for a filtered verbose
/// report, the captured matching paths.
///
/// `link_root` is the root the captured `files` paths are relative to (the
/// owning package root, else the effective base); the command layer resolves
/// it before rendering and reports a failure there, so this renderer never
/// guesses one. `None` renders no list — verbosity changes presentation only.
pub fn render_files_section(
    files: &FileAssociationBreakdown,
    verbose: u8,
    filter: &FilesFilter,
    link_root: Option<&Path>,
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

    if filtered.truncated {
        writeln!(
            out,
            "{}",
            Prose::new("<b>Incomplete scan:</b> the file classification limit was reached. Counts and percentages describe a partial sample and may vary between runs.")
                .render(&term)
        )
        .unwrap();
    }

    // The file list comes after the table and any incomplete-scan notice and
    // before the language/framework details. A missing root is not guessed
    // here; the caller resolves it or reports why it cannot.
    if verbose > 0 && filter.association.is_some() && let Some(root) = link_root {
        let paths: Vec<PathBuf> = filtered
            .by_association
            .iter()
            .flat_map(|stats| stats.files.iter().cloned())
            .collect();
        out.push_str(&render_file_list(&paths, root, &term));
    }

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
        InlineProse::new(&markup).render(&terminal)
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Strips renderer styling so assertions are independent of the test
    /// host's detected terminal capabilities (a non-tty harness renders the
    /// `[label](url)` link fallback instead of OSC8).
    fn visible(rendered: &str) -> String {
        biscuit_terminal::prelude::strip_escape_codes(rendered)
    }

    fn stats_with_files(association: sniff::filesystem::FileAssociation, files: &[&str]) -> FileAssociationStats {
        FileAssociationStats {
            association,
            file_count: files.len(),
            percentage: 100.0,
            files: files.iter().map(PathBuf::from).collect(),
        }
    }

    #[test]
    fn truncated_files_report_discloses_partial_sample_even_without_matches() {
        let files = FileAssociationBreakdown {
            truncated: true,
            limit: Some(10_000),
            ..FileAssociationBreakdown::default()
        };
        let filter = FilesFilter {
            association: Some(sniff::filesystem::FileAssociation::Image),
        };
        for verbose in [0, 1] {
            let rendered = render_files_section(&files, verbose, &filter, Some(Path::new("/work")));
            assert!(rendered.contains("Incomplete scan"), "{rendered}");
            assert!(rendered.contains("partial sample"), "{rendered}");
        }
    }

    #[test]
    fn verbose_filtered_report_lists_captured_paths_after_table() {
        let files = FileAssociationBreakdown {
            by_association: vec![stats_with_files(sniff::filesystem::FileAssociation::Image, &[
                "assets/banner.png",
                "assets/logo.svg",
            ])],
            ..FileAssociationBreakdown::default()
        };
        let filter = FilesFilter {
            association: Some(sniff::filesystem::FileAssociation::Image),
        };
        let rendered = render_files_section(&files, 1, &filter, Some(Path::new("/work")));
        let text = visible(&rendered);
        let table_end = text.find("Image").expect("table row");
        let heading = text.find("Files:").expect("list heading");
        assert!(table_end < heading, "{text}");
        assert!(text.contains("assets/banner.png"), "{text}");
        assert!(text.contains("assets/logo.svg"), "{text}");
    }

    #[test]
    fn file_list_precedes_framework_and_language_details() {
        let files = FileAssociationBreakdown {
            by_association: vec![stats_with_files(
                sniff::filesystem::FileAssociation::ProgrammingLanguage,
                &["src/main.rs"],
            )],
            by_language: vec![sniff::filesystem::ProgrammingLanguageStats {
                language: sniff::filesystem::ProgrammingLanguage::Rust,
                language_type: sniff::filesystem::ProgrammingLanguageType::CompiledBinary,
                direct_file_count: 1,
                framework_file_count: 0,
                total_file_count: 1,
                signal: 1.0,
                percentage: 100.0,
                direct_files: vec![PathBuf::from("src/main.rs")],
                framework_files: vec![],
            }],
            ..FileAssociationBreakdown::default()
        };
        let filter = FilesFilter {
            association: Some(sniff::filesystem::FileAssociation::ProgrammingLanguage),
        };
        let rendered = render_files_section(&files, 1, &filter, Some(Path::new("/work")));
        let text = visible(&rendered);
        let heading = text.find("Files:").expect("list heading");
        let languages = text.find("Languages:").expect("language details");
        assert!(heading < languages, "{text}");
        assert!(text.contains("src/main.rs"), "{text}");
    }

    #[test]
    fn no_list_without_link_root() {
        let files = FileAssociationBreakdown {
            by_association: vec![stats_with_files(sniff::filesystem::FileAssociation::Image, &[
                "assets/logo.svg",
            ])],
            ..FileAssociationBreakdown::default()
        };
        let filter = FilesFilter {
            association: Some(sniff::filesystem::FileAssociation::Image),
        };
        let rendered = render_files_section(&files, 1, &filter, None);
        assert!(!rendered.contains("Files:"), "{rendered}");
    }

    #[test]
    fn no_list_when_not_verbose_unfiltered_or_empty() {
        let image = stats_with_files(sniff::filesystem::FileAssociation::Image, &["a.png"]);
        let files = FileAssociationBreakdown {
            by_association: vec![image.clone()],
            ..FileAssociationBreakdown::default()
        };
        let root = Some(Path::new("/work"));

        // Non-verbose filtered: no list.
        let filter = FilesFilter {
            association: Some(sniff::filesystem::FileAssociation::Image),
        };
        let rendered = render_files_section(&files, 0, &filter, root);
        assert!(!rendered.contains("Files:"), "{rendered}");

        // Verbose but unfiltered: no list.
        let unfiltered = FilesFilter { association: None };
        let rendered = render_files_section(&files, 1, &unfiltered, root);
        assert!(!rendered.contains("Files:"), "{rendered}");

        // Verbose filtered but the category has no captured files: no list.
        let empty = FileAssociationBreakdown {
            by_association: vec![stats_with_files(sniff::filesystem::FileAssociation::Image, &[])],
            ..FileAssociationBreakdown::default()
        };
        let rendered = render_files_section(&empty, 1, &filter, root);
        assert!(!rendered.contains("Files:"), "{rendered}");

        // The captured paths are untouched by the verbosity change.
        assert_eq!(image.files, vec![PathBuf::from("a.png")]);
    }

    #[test]
    fn unknown_association_lists_paths_and_notice_precedes_list() {
        let files = FileAssociationBreakdown {
            by_association: vec![stats_with_files(sniff::filesystem::FileAssociation::Unknown, &[
                "mystery.bin",
            ])],
            truncated: true,
            limit: Some(100),
            ..FileAssociationBreakdown::default()
        };
        let filter = FilesFilter {
            association: Some(sniff::filesystem::FileAssociation::Unknown),
        };
        let rendered = render_files_section(&files, 1, &filter, Some(Path::new("/work")));
        let text = visible(&rendered);
        let notice = text.find("Incomplete scan").expect("notice");
        let heading = text.find("Files:").expect("list heading");
        assert!(notice < heading, "{text}");
        assert!(text.contains("mystery.bin"), "{text}");
    }

    #[test]
    fn truncated_breakdown_projects_one_observation_to_text_and_json() {
        // One constructed, captured breakdown shared by both projections:
        // the text discloses the partial sample before listing it, and the
        // JSON projection carries the same captured files and truncation.
        let files = FileAssociationBreakdown {
            by_association: vec![stats_with_files(sniff::filesystem::FileAssociation::Image, &[
                "a.png", "b.png",
            ])],
            truncated: true,
            limit: Some(10),
            ..FileAssociationBreakdown::default()
        };
        let filter = FilesFilter {
            association: Some(sniff::filesystem::FileAssociation::Image),
        };
        let rendered = render_files_section(&files, 1, &filter, Some(Path::new("/work")));
        let text = visible(&rendered);
        let notice = text.find("Incomplete scan").expect("notice");
        let heading = text.find("Files:").expect("list heading");
        assert!(notice < heading, "{text}");
        assert!(text.contains("a.png") && text.contains("b.png"), "{text}");

        let projected = filter_file_breakdown(&files, &filter);
        assert!(projected.truncated);
        assert_eq!(projected.limit, Some(10));
        let json = serde_json::to_value(&projected).expect("serializable");
        assert_eq!(json["truncated"], serde_json::json!(true));
        assert_eq!(
            json["by_association"][0]["files"],
            serde_json::json!(["a.png", "b.png"]),
            "the captured sample is the list's source: {json}"
        );
        assert_eq!(json["by_association"][0]["file_count"], 2);
    }

    #[test]
    fn removed_file_remains_listed_and_linked_after_discovery() {
        // A file deleted after discovery stays listed with its link; no
        // existence probe decides either. The root must be absolute on every
        // OS, so anchor on the system temp directory.
        let root = std::env::temp_dir();
        let files = FileAssociationBreakdown {
            by_association: vec![stats_with_files(sniff::filesystem::FileAssociation::Image, &[
                "vanished/gone.png",
            ])],
            ..FileAssociationBreakdown::default()
        };
        let filter = FilesFilter {
            association: Some(sniff::filesystem::FileAssociation::Image),
        };
        let rendered = render_files_section(&files, 1, &filter, Some(&root));
        let text = visible(&rendered);
        assert!(text.contains("vanished/gone.png"), "{text}");
        assert!(
            rendered.contains("file://"),
            "the removed file keeps its hyperlink:\n{rendered}"
        );
    }
}
