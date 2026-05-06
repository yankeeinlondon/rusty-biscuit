//! Link Resolve operation for the compose pipeline.
//!
//! Converts all local links in a document to absolute paths during the
//! Inline-Pre stage. This ensures that as documents are moved or transcluded,
//! their references remain valid until they are eventually normalized
//! back to portable forms in the Finalization stage.

use crate::markdown::Markdown;
use crate::markdown::compose::{ComposeOptions, ComposeReport, ComposeSource};
use crate::markdown::reference::{
    ReferenceKind, ReferenceRecord, ReferenceTarget,
    local::{extract_markdown_links, extract_markdown_images},
    html::{
        extract_html_links, extract_html_images, extract_html_videos,
        extract_html_audio, extract_html_sources, extract_html_iframes,
        extract_html_link_tags,
    },
};
use crate::markdown::types::MarkdownResult;
use std::path::Path;

/// Resolves all local link targets (Markdown hyperlinks/images and
/// supported HTML embeds) to absolute paths.
pub fn link_resolve(
    markdown: &mut Markdown,
    options: &ComposeOptions,
    report: &mut ComposeReport,
) -> MarkdownResult<()> {
    let source = options.source.clone();
    let content = markdown.content();

    // 2.3 Extract all references
    let mut records = Vec::new();
    records.extend(extract_markdown_links(content, &source));
    records.extend(extract_markdown_images(content, &source));
    records.extend(extract_html_links(content, &source));
    records.extend(extract_html_images(content, &source));
    records.extend(extract_html_videos(content, &source));
    records.extend(extract_html_audio(content, &source));
    records.extend(extract_html_sources(content, &source));
    records.extend(extract_html_iframes(content, &source));
    records.extend(extract_html_link_tags(content, &source));

    // 2.4 Filter to link-like references only
    // ReferenceKind::Hyperlink, ReferenceKind::Image cover most of them.
    // extract_html_link_tags returns CssImport/FontImport/Hyperlink.
    let mut to_resolve = Vec::new();
    for record in records {
        match record.kind {
            ReferenceKind::Hyperlink
            | ReferenceKind::Image
            | ReferenceKind::CssImport
            | ReferenceKind::FontImport => {
                if let ReferenceTarget::LocalPath { .. } = &record.target {
                    to_resolve.push(record);
                }
            }
            _ => {}
        }
    }

    if to_resolve.is_empty() {
        return Ok(());
    }

    // Sort by span start descending for safe in-place replacement
    to_resolve.sort_by_key(|r| std::cmp::Reverse(r.origin.span.start));

    let mut new_content = content.to_string();
    let mut applied_count = 0;

    let base_dir = match &source {
        ComposeSource::File(path) => path.parent(),
        _ => None,
    };

    for record in to_resolve {
        let raw_target = match &record.target {
            ReferenceTarget::LocalPath { raw } => raw.to_string_lossy().to_string(),
            _ => continue,
        };

        // 2.5 Resolve to absolute path
        if let Some(abs_path) = resolve_absolute(&raw_target, base_dir, options) {
            let abs_path_str = abs_path.to_string_lossy().to_string();

            // 2.6 Replace original link text with absolute path
            // We need to find the raw target string within the span.
            // For Markdown links [text](target), the target is at the end.
            // For HTML tags <tag attr="target">, it's inside the attribute.
            // The ReferenceRecord span is for the WHOLE tag/link.

            if let Some((start, end)) = find_target_range(&new_content, &record, &raw_target) {
                new_content.replace_range(start..end, &abs_path_str);
                applied_count += 1;
            }
        }
    }

    if applied_count > 0 {
        *markdown.content_mut() = new_content;
        report.link_resolves_applied += applied_count;
    }

    Ok(())
}

fn resolve_absolute(raw: &str, base_dir: Option<&Path>, options: &ComposeOptions) -> Option<std::path::PathBuf> {
    let mut file_ref = biscuit_file::FileReference::new(raw).ok()?;

    // Add magic paths from options
    for (path, position) in &options.magic_paths {
        file_ref = file_ref.add_magic_path(path, *position);
    }

    if let Ok(Some(resolved)) = file_ref.resolve_relative(base_dir) {
        // canonicalize to get absolute path
        return std::fs::canonicalize(&resolved).ok().or(Some(resolved));
    }

    // Fallback to simple join if FileReference fails or returns nothing
    if let Some(dir) = base_dir {
        let joined = dir.join(raw);
        return std::fs::canonicalize(&joined).ok().or(Some(joined));
    }

    None
}

fn find_target_range(content: &str, record: &ReferenceRecord, raw_target: &str) -> Option<(usize, usize)> {
    let span = &record.origin.span;
    let outer_text = &content[span.clone()];

    // Search for the raw_target within the span.
    // For safety, we look for it quoted or in parentheses.
    let search_patterns = [
        format!("\"{}\"", raw_target),
        format!("'{}'", raw_target),
        format!("({})", raw_target),
    ];

    for pattern in &search_patterns {
        if let Some(idx) = outer_text.find(pattern) {
            // idx is relative to span.start.
            // We want the range of raw_target itself, excluding quotes/parents.
            let start = span.start + idx + 1;
            let end = start + raw_target.len();
            return Some((start, end));
        }
    }

    // Fallback to just finding the raw string if pattern matching fails
    if let Some(idx) = outer_text.find(raw_target) {
        let start = span.start + idx;
        let end = start + raw_target.len();
        return Some((start, end));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::compose::ComposeReport;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_link_resolve_markdown() {
        let dir = tempdir().unwrap();
        let file_a = dir.path().join("a.md");
        let file_b = dir.path().join("b.md");
        fs::write(&file_b, "target content").unwrap();

        let content = "[link](./b.md) and ![img](b.md)";
        let mut md = Markdown::new(content);
        let options = ComposeOptions::new().with_source_file(&file_a);
        let mut report = ComposeReport::new();

        link_resolve(&mut md, &options, &mut report).unwrap();

        let resolved_path = fs::canonicalize(&file_b).unwrap().to_string_lossy().to_string();
        assert!(md.content().contains(&format!("({})", resolved_path)));
        assert!(md.content().contains(&format!("({})", resolved_path)));
        assert_eq!(report.link_resolves_applied, 2);
    }

    #[test]
    fn test_link_resolve_html() {
        let dir = tempdir().unwrap();
        let file_a = dir.path().join("a.md");
        let file_b = dir.path().join("b.md");
        fs::write(&file_b, "target content").unwrap();

        let content = r#"<a href="./b.md">link</a> and <img src="b.md"> and <iframe src="./b.md"></iframe>"#;
        let mut md = Markdown::new(content);
        let options = ComposeOptions::new().with_source_file(&file_a);
        let mut report = ComposeReport::new();

        link_resolve(&mut md, &options, &mut report).unwrap();

        let resolved_path = fs::canonicalize(&file_b).unwrap().to_string_lossy().to_string();
        assert!(md.content().contains(&format!("\"{}\"", resolved_path)));
        assert!(md.content().contains(&format!("\"{}\"", resolved_path)));
        assert!(md.content().contains(&format!("\"{}\"", resolved_path)));
        assert_eq!(report.link_resolves_applied, 3);
    }

    #[test]
    fn test_link_resolve_media() {
        let dir = tempdir().unwrap();
        let file_a = dir.path().join("a.md");
        let file_b = dir.path().join("movie.mp4");
        fs::write(&file_b, "video content").unwrap();

        let content = r#"<video src="./movie.mp4"></video><audio src="movie.mp4"></audio><source src="./movie.mp4">"#;
        let mut md = Markdown::new(content);
        let options = ComposeOptions::new().with_source_file(&file_a);
        let mut report = ComposeReport::new();

        link_resolve(&mut md, &options, &mut report).unwrap();

        let resolved_path = fs::canonicalize(&file_b).unwrap().to_string_lossy().to_string();
        assert!(md.content().contains(&format!("\"{}\"", resolved_path)));
        assert_eq!(report.link_resolves_applied, 3);
    }
}
