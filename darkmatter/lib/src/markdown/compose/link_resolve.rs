//! Link Resolve operation for the compose pipeline.
//!
//! Converts all local links in a document to absolute paths during the
//! Inline-Pre stage. This ensures that as documents are moved or transcluded,
//! their references remain valid until they are eventually normalized
//! back to portable forms in the Finalization stage.

use crate::markdown::Markdown;
use crate::markdown::compose::util::{document_resolution_context, find_git_root_from};
use crate::markdown::compose::{ComposeOptions, ComposeReport, ComposeSource};
use crate::markdown::reference::{
    ReferenceKind, ReferenceTarget,
    html::{
        extract_html_audio, extract_html_iframes, extract_html_images, extract_html_link_tags,
        extract_html_links, extract_html_script_blocks, extract_html_sources, extract_html_videos,
    },
    local::{extract_markdown_images, extract_markdown_links},
};
use crate::markdown::types::MarkdownResult;
use std::path::Path;
use tracing::trace;

/// Resolves all local link targets (Markdown hyperlinks/images and
/// supported HTML embeds) to absolute paths.
pub fn link_resolve(
    markdown: &mut Markdown,
    options: &ComposeOptions,
    report: &mut ComposeReport,
) -> MarkdownResult<()> {
    let source = options.source.clone();
    let content = markdown.content();

    // 2.3 Extract local path references
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
    records.extend(extract_html_script_blocks(content, &source));

    // 2.4 Filter to link-like references only
    // ReferenceKind::Hyperlink, ReferenceKind::Image cover most of them.
    // extract_html_link_tags returns CssImport/FontImport/Hyperlink.
    let mut to_resolve = Vec::new();
    trace!("Total records extracted: {}", records.len());
    for record in records {
        trace!(
            "Record kind: {:?}, target: {:?}",
            record.kind, record.target
        );
        match record.kind {
            ReferenceKind::Hyperlink
            | ReferenceKind::Image
            | ReferenceKind::HtmlVideo
            | ReferenceKind::HtmlAudio
            | ReferenceKind::HtmlSource
            | ReferenceKind::HtmlIframe
            | ReferenceKind::CssImport
            | ReferenceKind::ScriptImport
            | ReferenceKind::FontImport => {
                if let ReferenceTarget::LocalPath { .. } = &record.target {
                    to_resolve.push(record);
                }
            }
            _ => {}
        }
    }
    trace!("Records to resolve: {}", to_resolve.len());

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

            if let Some((start, end)) = super::find_target_range(&new_content, &record, &raw_target)
            {
                new_content.replace_range(start..end, &abs_path_str);
                applied_count += 1;
            }
        }
    }

    report.link_resolves_applied += applied_count;
    if applied_count > 0 {
        *markdown.content_mut() = new_content;
    }

    Ok(())
}

fn resolve_absolute(
    raw: &str,
    base_dir: Option<&Path>,
    options: &ComposeOptions,
) -> Option<std::path::PathBuf> {
    if raw.starts_with("http://") || raw.starts_with("https://") {
        trace!("resolve_absolute: skipping HTTP(S) URL '{}'", raw);
        return None;
    }

    trace!("resolve_absolute called with raw: '{}'", raw);
    // A value that does not parse as a `FileReference` is not a resolvable
    // local path; leave the link untouched rather than fabricating a path.
    let file_ref = biscuit_file::FileReference::new(raw).ok()?;

    // Resolve through the shared document-backed context so relative and `@`
    // references anchor on the document directory (implicit paths
    // repository-first then source), never the ambient process CWD. Magic roots
    // live on the context, not on the reference. We intentionally do NOT use
    // resolve_relative here — link resolve's job is to produce absolute paths,
    // not make them relative again.
    let resolved = if let Some(dir) = base_dir {
        let repo_root = find_git_root_from(dir);
        let resolution_ctx =
            document_resolution_context(dir, None, &options.magic_paths, repo_root.as_deref());
        // An existing target resolves to its matched path; a clean miss (a link
        // to a not-yet-created file) is absolutized to the FIRST shared
        // candidate — repository-first for an implicit bare path — via the same
        // `FileReference` grammar execution uses, never a source-first
        // `dir.join(raw)` that would bypass shared classification. A hard
        // resolver failure (invalid context, missing anchor) leaves the link
        // untouched.
        match file_ref.resolve_in_context(&resolution_ctx) {
            Ok(Some(path)) => Some(path),
            Ok(None) => file_ref
                .candidate_plan(&resolution_ctx)
                .ok()
                .and_then(|plan| plan.into_iter().next())
                .map(|candidate| candidate.path().to_path_buf()),
            Err(_) => None,
        }
    } else {
        // Bare-API path with no document base: only an existing target can be
        // absolutized; a miss has no candidate anchor without a base.
        file_ref.resolve().ok().flatten()
    };

    let resolved = resolved?;
    // `canonicalize` can fail on a resolved-but-since-removed (or not-yet-
    // created) path; the resolved absolute path is a correct fallback then.
    let result = std::fs::canonicalize(&resolved).ok().or(Some(resolved));
    trace!("resolve_absolute success: {:?}", result);
    result
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

        let resolved_path = fs::canonicalize(&file_b)
            .unwrap()
            .to_string_lossy()
            .to_string();
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

        let content =
            r#"<a href="./b.md">link</a> and <img src="b.md"> and <iframe src="./b.md"></iframe>"#;
        let mut md = Markdown::new(content);
        let options = ComposeOptions::new().with_source_file(&file_a);
        let mut report = ComposeReport::new();

        link_resolve(&mut md, &options, &mut report).unwrap();

        let resolved_path = fs::canonicalize(&file_b)
            .unwrap()
            .to_string_lossy()
            .to_string();
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

        let resolved_path = fs::canonicalize(&file_b)
            .unwrap()
            .to_string_lossy()
            .to_string();
        assert!(md.content().contains(&format!("\"{}\"", resolved_path)));
        assert_eq!(report.link_resolves_applied, 3);
    }

    #[test]
    fn test_link_resolve_css_font_script() {
        let dir = tempdir().unwrap();
        let file_a = dir.path().join("a.md");
        let target_css = dir.path().join("styles.css");
        let target_font = dir.path().join("font.woff2");
        let target_script = dir.path().join("app.js");
        fs::write(&target_css, "body {}").unwrap();
        fs::write(&target_font, "binary...").unwrap();
        fs::write(&target_script, "console.log();").unwrap();

        let content = "<link rel=\"stylesheet\" href=\"./styles.css\">\n<link rel=\"preload\" as=\"font\" href=\"font.woff2\">\n<script src=\"./app.js\"></script>";

        let mut md = Markdown::new(content);
        let options = ComposeOptions::new().with_source_file(&file_a);
        let mut report = ComposeReport::new();

        link_resolve(&mut md, &options, &mut report).unwrap();

        let resolved_css = fs::canonicalize(&target_css)
            .unwrap()
            .to_string_lossy()
            .to_string();
        let resolved_font = fs::canonicalize(&target_font)
            .unwrap()
            .to_string_lossy()
            .to_string();
        let resolved_script = fs::canonicalize(&target_script)
            .unwrap()
            .to_string_lossy()
            .to_string();

        assert!(
            md.content().contains(&format!("\"{}\"", resolved_css)),
            "CSS failed. Content: {}",
            md.content()
        );
        assert!(
            md.content().contains(&format!("\"{}\"", resolved_font)),
            "Font failed. Content: {}",
            md.content()
        );
        assert!(
            md.content().contains(&format!("\"{}\"", resolved_script)),
            "Script failed. Content: {}",
            md.content()
        );
        assert_eq!(report.link_resolves_applied, 3);
    }

    #[test]
    fn test_link_resolve_edge_cases() {
        let dir = tempdir().unwrap();
        let file_a = dir.path().join("a.md");
        let target_parens = dir.path().join("with (parens).md");
        let target_quotes = dir.path().join("single_quotes.md");
        let target_mixed = dir.path().join("mixed.md");
        let target_multi = dir.path().join("multi.md");

        fs::write(&target_parens, "").unwrap();
        fs::write(&target_quotes, "").unwrap();
        fs::write(&target_mixed, "").unwrap();
        fs::write(&target_multi, "").unwrap();

        // Targets appearing multiple times in the same span? Wait, the span is per element usually.
        // E.g., <a href='single_quotes.md' data-alt='single_quotes.md'>link</a>
        let content = "[link with parens](<./with (parens).md>)\n<img src='single_quotes.md'>\n<a href=\"mixed.md\" data-target='mixed.md'>mixed</a>\n<a href=\"multi.md\">multi.md is the target</a>";

        let mut md = Markdown::new(content);
        let options = ComposeOptions::new().with_source_file(&file_a);
        let mut report = ComposeReport::new();

        link_resolve(&mut md, &options, &mut report).unwrap();

        let resolved_parens = fs::canonicalize(&target_parens)
            .unwrap()
            .to_string_lossy()
            .to_string();
        let resolved_quotes = fs::canonicalize(&target_quotes)
            .unwrap()
            .to_string_lossy()
            .to_string();
        let resolved_mixed = fs::canonicalize(&target_mixed)
            .unwrap()
            .to_string_lossy()
            .to_string();
        let resolved_multi = fs::canonicalize(&target_multi)
            .unwrap()
            .to_string_lossy()
            .to_string();

        assert!(
            md.content().contains(&format!("(<{}>)", resolved_parens)),
            "Parens failed. Content: {}",
            md.content()
        );
        assert!(
            md.content().contains(&format!("'{}'", resolved_quotes)),
            "Quotes failed. Content: {}",
            md.content()
        );
        assert!(
            md.content().contains(&format!("\"{}\"", resolved_mixed)),
            "Mixed failed. Content: {}",
            md.content()
        );
        assert!(
            md.content().contains(&format!("\"{}\"", resolved_multi)),
            "Multi failed. Content: {}",
            md.content()
        );

        // Ensure "multi.md is the target" wasn't mangled, but the href was.
        assert!(
            md.content()
                .contains(&format!(">{} is the target</a>", "multi.md"))
        );

        // 4 elements
        assert_eq!(report.link_resolves_applied, 4);
    }

    #[test]
    fn test_link_resolve_non_existent() {
        let dir = tempdir().unwrap();
        let file_a = dir.path().join("a.md");
        fs::write(&file_a, "source").unwrap();

        // b.md does not exist
        let content = "[link](./b.md)";
        let mut md = Markdown::new(content);
        let options = ComposeOptions::new().with_source_file(&file_a);
        let mut report = ComposeReport::new();

        link_resolve(&mut md, &options, &mut report).unwrap();

        // `./b.md` is an EXPLICIT-relative reference: even missing, it is pinned
        // to the source directory (its sole shared candidate), so the absolute
        // shape is `<source_dir>/./b.md`. This flows through `FileReference`'s
        // candidate plan, not a private `dir.join(raw)` fallback.
        let joined = dir.path().join("./b.md");
        let resolved_path = joined.to_string_lossy().to_string();

        assert!(
            md.content().contains(&format!("({})", resolved_path)),
            "Non-existent failed. Content: {}",
            md.content()
        );
        assert_eq!(report.link_resolves_applied, 1);
    }

    /// A missing IMPLICIT bare reference is absolutized repository-first — the
    /// same anchoring an existing implicit reference resolves with — rather than
    /// source-joined. This is the D2/D3 precedence: `link_resolve` no longer
    /// falls back to `source_dir.join(raw)` after a miss.
    #[test]
    fn test_link_resolve_non_existent_implicit_is_repository_first() {
        let dir = tempdir().unwrap();
        // Plant a `.git` marker so the tempdir is a repository root distinct
        // from the nested document directory.
        std::fs::create_dir_all(dir.path().join(".git")).unwrap();
        let nested = dir.path().join("prompts");
        std::fs::create_dir_all(&nested).unwrap();
        let source = nested.join("a.md");
        std::fs::write(&source, "source").unwrap();

        // `missing.md` exists nowhere; as an implicit bare reference its shape
        // anchors on the repository root, not the source directory.
        let content = "[link](missing.md)";
        let mut md = Markdown::new(content);
        let options = ComposeOptions::new().with_source_file(&source);
        let mut report = ComposeReport::new();

        link_resolve(&mut md, &options, &mut report).unwrap();

        let repo_first = dir.path().join("missing.md").to_string_lossy().to_string();
        let source_first = nested.join("missing.md").to_string_lossy().to_string();
        assert!(
            md.content().contains(&format!("({repo_first})")),
            "expected repository-first shape {repo_first}. Content: {}",
            md.content()
        );
        assert!(
            !md.content().contains(&format!("({source_first})")),
            "must not source-join a missing implicit reference. Content: {}",
            md.content()
        );
        assert_eq!(report.link_resolves_applied, 1);
    }

    #[test]
    fn test_link_resolve_wrong_attribute_not_replaced() {
        let dir = tempdir().unwrap();
        let file_a = dir.path().join("a.md");
        let logo = dir.path().join("logo.png");
        fs::write(&logo, "png").unwrap();

        // alt contains same path as src - should only replace src
        let content = r#"<img alt="logo.png" src="logo.png">"#;
        let mut md = Markdown::new(content);
        let options = ComposeOptions::new().with_source_file(&file_a);
        let mut report = ComposeReport::new();

        link_resolve(&mut md, &options, &mut report).unwrap();

        let resolved_path = fs::canonicalize(&logo)
            .unwrap()
            .to_string_lossy()
            .to_string();

        // src should be resolved to absolute path
        assert!(
            md.content().contains(&format!("src=\"{}\"", resolved_path)),
            "src was not resolved. Content: {}",
            md.content()
        );
        // alt should remain unchanged
        assert!(
            md.content().contains(r#"alt="logo.png""#),
            "alt was incorrectly modified. Content: {}",
            md.content()
        );
        assert_eq!(report.link_resolves_applied, 1);
    }

    #[test]
    fn test_link_resolve_html_entity_in_attribute() {
        let dir = tempdir().unwrap();
        let file_a = dir.path().join("a.md");
        let target = dir.path().join("foo&bar.md");
        fs::write(&target, "content").unwrap();

        // HTML entity in href
        let content = r#"<a href="foo&amp;bar.md">link</a>"#;
        let mut md = Markdown::new(content);
        let options = ComposeOptions::new().with_source_file(&file_a);
        let mut report = ComposeReport::new();

        link_resolve(&mut md, &options, &mut report).unwrap();

        let resolved_path = fs::canonicalize(&target)
            .unwrap()
            .to_string_lossy()
            .to_string();

        assert!(
            md.content().contains(&format!("\"{}\"", resolved_path)),
            "HTML entity link was not resolved. Content: {}",
            md.content()
        );
        assert_eq!(report.link_resolves_applied, 1);
    }

    #[test]
    fn test_link_resolve_nested_source_in_video() {
        let dir = tempdir().unwrap();
        let file_a = dir.path().join("a.md");
        let movie = dir.path().join("movie.mp4");
        fs::write(&movie, "video content").unwrap();

        // Nested source inside video tag
        let content = r#"<video><source src="./movie.mp4"></video>"#;
        let mut md = Markdown::new(content);
        let options = ComposeOptions::new().with_source_file(&file_a);
        let mut report = ComposeReport::new();

        link_resolve(&mut md, &options, &mut report).unwrap();

        let resolved_path = fs::canonicalize(&movie)
            .unwrap()
            .to_string_lossy()
            .to_string();

        assert!(
            md.content().contains(&format!("\"{}\"", resolved_path)),
            "Nested source was not resolved. Content: {}",
            md.content()
        );
        assert_eq!(report.link_resolves_applied, 1);
    }

    #[test]
    fn test_link_resolve_html_spaced_attributes() {
        let dir = tempdir().unwrap();
        let file_a = dir.path().join("a.md");
        let file_b = dir.path().join("b.md");
        let file_movie = dir.path().join("movie.mp4");
        let file_css = dir.path().join("styles.css");
        fs::write(&file_b, "target content").unwrap();
        fs::write(&file_movie, "video content").unwrap();
        fs::write(&file_css, "body {}").unwrap();

        let content = r#"<a href = "./b.md">link</a> and <img src = "b.md"> and <video src = "./movie.mp4"></video> and <link href = "styles.css">"#;
        let mut md = Markdown::new(content);
        let options = ComposeOptions::new().with_source_file(&file_a);
        let mut report = ComposeReport::new();

        link_resolve(&mut md, &options, &mut report).unwrap();

        let resolved_b = fs::canonicalize(&file_b)
            .unwrap()
            .to_string_lossy()
            .to_string();
        let resolved_movie = fs::canonicalize(&file_movie)
            .unwrap()
            .to_string_lossy()
            .to_string();
        let resolved_css = fs::canonicalize(&file_css)
            .unwrap()
            .to_string_lossy()
            .to_string();

        assert!(
            md.content().contains(&format!("\"{}\"", resolved_b)),
            "Spaced href failed. Content: {}",
            md.content()
        );
        assert!(
            md.content().contains(&format!("\"{}\"", resolved_movie)),
            "Spaced video src failed. Content: {}",
            md.content()
        );
        assert!(
            md.content().contains(&format!("\"{}\"", resolved_css)),
            "Spaced link href failed. Content: {}",
            md.content()
        );
        assert_eq!(report.link_resolves_applied, 4);
    }

    #[test]
    fn test_link_resolve_preserves_http_urls() {
        let content = "[link](https://example.com/page) and ![img](http://cdn.example.com/img.png)";
        let mut md = Markdown::new(content);
        let options = ComposeOptions::new();
        let mut report = ComposeReport::new();

        link_resolve(&mut md, &options, &mut report).unwrap();

        assert!(
            md.content().contains("https://example.com/page"),
            "HTTPS URL was modified. Content: {}",
            md.content()
        );
        assert!(
            md.content().contains("http://cdn.example.com/img.png"),
            "HTTP URL was modified. Content: {}",
            md.content()
        );
        assert_eq!(report.link_resolves_applied, 0);
    }
}
