//! Markdown document rendering for `sniff docs`.

use std::fmt::Write;
use std::rc::Rc;

use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::{TerminalRenderable, RenderableTerminalContent};
use biscuit_terminal::terminal::Terminal;
use sniff::filesystem::docs::MarkdownMeta;
use sniff::filesystem::{TitleSource, UpdatedSource};

use super::TextOutput;
use super::path_format::format_styled_filepath;

/// Render markdown documents with split stdout/stderr output.
///
/// - **stderr**: header line + footer (when not verbose)
/// - **stdout**: document list
pub fn render_docs_output(docs: &[MarkdownMeta], verbose: u8) -> TextOutput {
    let terminal = Terminal::default();
    let prompt_count = docs.iter().filter(|d| d.prompt.is_some()).count();

    // --- stderr: header ---
    let mut stderr = String::new();
    let header = if prompt_count > 0 {
        format!(
            "<b>Docs</b> <dim>({} documents, {} with prompts)</dim>",
            docs.len(),
            prompt_count
        )
    } else {
        format!("<b>Docs</b> <dim>({} documents)</dim>", docs.len())
    };
    writeln!(stderr, "\n{}\n", Prose::new(&header).render(&terminal)).unwrap();

    // --- stdout: document list ---
    let mut stdout = String::new();

    let items: Vec<RenderableTerminalContent> = docs
        .iter()
        .flat_map(|doc| {
            let file_link =
                format_styled_filepath(&doc.relative, &doc.filepath.display().to_string());
            let main = Prose::new(&file_link).render(&terminal);
            let mut result = vec![RenderableTerminalContent::String(main)];

            if verbose > 0 {
                let mut details: Vec<String> = Vec::new();

                // title
                let title_source_label = match doc.title_source {
                    TitleSource::FrontmatterTitle => "title property",
                    TitleSource::H1Heading => "H1 heading",
                    TitleSource::H2Heading => "H2 heading",
                    TitleSource::H3Heading => "H3 heading",
                    TitleSource::None => "none",
                };
                if !doc.title.is_empty() {
                    let title_line = format!(
                        "<b>title:</b> {} <dim><i>(from {})</i></dim>",
                        doc.title, title_source_label
                    );
                    details.push(Prose::new(&title_line).render(&terminal));
                } else {
                    let title_line = format!(
                        "<b>title:</b> <yellow>none</yellow> <dim><i>(from {})</i></dim>",
                        title_source_label
                    );
                    details.push(Prose::new(&title_line).render(&terminal));
                }

                // updated
                let date_str = doc.last_updated.format("%Y-%m-%d").to_string();
                let updated_source_label = match doc.updated_source {
                    UpdatedSource::UpdatedProperty => "updated property",
                    UpdatedSource::FileMetadata => "file metadata",
                };
                let updated_line = format!(
                    "<b>updated:</b> {} <dim><i>(from {})</i></dim>",
                    date_str, updated_source_label
                );
                details.push(Prose::new(&updated_line).render(&terminal));

                // frontmatter properties
                if !doc.frontmatter_keys.is_empty() {
                    let props = doc.frontmatter_keys.join(", ");
                    let props_line = format!("<b>frontmatter properties:</b> <i>{props}</i>");
                    details.push(Prose::new(&props_line).render(&terminal));
                }

                let detail_list = UnorderedList::new(details).with_bullet("  ");
                result.push(RenderableTerminalContent::Component(Rc::new(detail_list)));
            }

            result
        })
        .collect();

    let list = UnorderedList::from(items);
    writeln!(stdout, "{}", list.render(&terminal)).unwrap();

    // --- stderr: footer ---
    if verbose == 0 {
        writeln!(
            stderr,
            "{}",
            Prose::new(
                "<dim>Use <blue>--verbose</blue> / <blue>-v</blue> to include metadata for documents</dim>"
            )
            .render(&terminal)
        )
        .unwrap();
    }

    TextOutput { stdout, stderr }
}
