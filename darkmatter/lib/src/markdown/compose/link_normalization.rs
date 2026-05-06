//! Link Normalization operation for the compose pipeline.
//!
//! Converts absolute paths back to portable forms in the Finalization stage.
//! Runs only on the root document.

use std::path::{Path, PathBuf};

use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::components::status::{Status, StatusState};
use biscuit_terminal::terminal::Terminal;

use crate::markdown::Markdown;
use crate::markdown::compose::{ComposeOptions, ComposeReport, ComposeSource};
use crate::markdown::reference::{
    ReferenceKind, ReferenceRecord, ReferenceTarget,
    html::{
        extract_html_audio, extract_html_iframes, extract_html_images, extract_html_link_tags,
        extract_html_links, extract_html_sources, extract_html_videos,
    },
    local::{extract_markdown_images, extract_markdown_links},
};
use crate::markdown::types::MarkdownResult;

/// Normalizes absolute path links back into portable forms.
///
/// This operation is the inverse of [`link_resolve`](super::link_resolve::link_resolve).
/// It runs during the Finalization phase on the root document only.
///
/// Rules applied in order:
/// 1. **Same-repo**: If path is inside the same git repo as the document, make it relative.
/// 2. **Home-dir**: If path is under HOME, use `~/` prefix.
/// 3. **ENV-var**: If path is under a whitelisted environment variable, use `${VAR}/` prefix.
pub fn normalize_links(
    markdown: &mut Markdown,
    options: &ComposeOptions,
    report: &mut ComposeReport,
) -> MarkdownResult<()> {
    let source = options.source.clone();
    let content = markdown.content();

    // 3.5 Extract absolute path references
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

    let mut to_normalize = Vec::new();
    for record in records {
        let mut abs_path = None;
        if let ReferenceTarget::LocalPath { raw } = &record.target
            && raw.is_absolute()
        {
            abs_path = Some(raw.clone());
        }

        if let Some(abs_path) = abs_path {
            match record.kind {
                ReferenceKind::Hyperlink
                | ReferenceKind::Image
                | ReferenceKind::CssImport
                | ReferenceKind::FontImport => {
                    to_normalize.push((record, abs_path));
                }
                _ => {}
            }
        }
    }

    if to_normalize.is_empty() {
        return Ok(());
    }

    // Sort by span start descending for safe in-place replacement
    to_normalize.sort_by_key(|(r, _)| std::cmp::Reverse(r.origin.span.start));

    let mut new_content = content.to_string();
    let mut applied_count = 0;

    let base_file = match &source {
        ComposeSource::File(path) => Some(path.clone()),
        _ => None,
    };

    let base_dir = base_file
        .as_ref()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()));
    let git_root = base_dir.as_ref().and_then(|d| find_git_repo_root(d));
    let home = dirs::home_dir();

    for (record, abs_path) in to_normalize {
        let mut replacement = None;

        // 3.6 Same-repo rule
        if let Some(ref repo) = git_root
            && abs_path.starts_with(repo)
            && let Some(ref doc_path) = base_file
        {
            let rel = compute_relative_path(doc_path, &abs_path);
            replacement = Some(rel.to_string_lossy().to_string());
        }

        // 3.7 Home-dir rule
        if replacement.is_none()
            && let Some(ref h) = home
            && abs_path.starts_with(h)
            && let Ok(rel) = abs_path.strip_prefix(h)
        {
            replacement = Some(format!("~/{}", rel.display()));
        }

        // 3.8 ENV-var rule
        if replacement.is_none() {
            let whitelist = options.effective_env_path_whitelist();
            let mut best_var = None;
            let mut longest_len = 0;

            for var_name in whitelist {
                if let Ok(val) = std::env::var(&var_name) {
                    let var_path = PathBuf::from(val);
                    if abs_path.starts_with(&var_path) {
                        let path_len = var_path.as_os_str().len();
                        if path_len > longest_len {
                            longest_len = path_len;
                            best_var = Some((var_name, var_path));
                        }
                    }
                }
            }

            if let Some((var_name, var_path)) = best_var
                && let Ok(rel) = abs_path.strip_prefix(&var_path)
            {
                let env_replacement = format!("${{{}}}/{}", var_name, rel.display());

                // 3.9 Emit warning
                let msg = format!(
                    "the path {} was found to be an offset of the {} environment variable and will use this abstraction.",
                    abs_path.display(),
                    var_name
                );
                let status = Status::new(msg).state(StatusState::Warning);
                eprintln!("{}", status.display(&Terminal::default()));

                replacement = Some(env_replacement);
            }
        }

        if let Some(new_target) = replacement
            && let Some((start, end)) =
                find_target_range(&new_content, &record, &abs_path.to_string_lossy())
        {
            new_content.replace_range(start..end, &new_target);
            applied_count += 1;
        }
    }

    if applied_count > 0 {
        *markdown.content_mut() = new_content;
        report.link_normalizations_applied += applied_count;
    }

    Ok(())
}

/// Helper to find target range within content (same as link_resolve.rs)
fn find_target_range(
    content: &str,
    record: &ReferenceRecord,
    raw_target: &str,
) -> Option<(usize, usize)> {
    let span = &record.origin.span;
    if span.end > content.len() {
        return None;
    }
    let outer_text = &content[span.clone()];

    let search_patterns = [
        format!("\"{}\"", raw_target),
        format!("'{}'", raw_target),
        format!("({})", raw_target),
    ];

    for pattern in &search_patterns {
        if let Some(idx) = outer_text.find(pattern) {
            let start = span.start + idx + 1;
            let end = start + raw_target.len();
            return Some((start, end));
        }
    }

    if let Some(idx) = outer_text.find(raw_target) {
        let start = span.start + idx;
        let end = start + raw_target.len();
        return Some((start, end));
    }

    None
}

/// 3.2 Implement find_git_repo_root helper
fn find_git_repo_root(from: &Path) -> Option<PathBuf> {
    let mut current = from;
    loop {
        if current.join(".git").exists() {
            return Some(current.to_path_buf());
        }
        match current.parent() {
            Some(parent) => current = parent,
            None => break,
        }
    }
    None
}

/// 3.3 Implement compute_relative_path helper
fn compute_relative_path(from: &Path, to: &Path) -> PathBuf {
    let from_dir = if from.is_file() {
        from.parent().unwrap_or(from)
    } else {
        from
    };

    match diff_paths(to, from_dir) {
        Some(p) => p,
        None => to.to_path_buf(),
    }
}

/// Resolve `.` and `..` components without touching the filesystem.
fn normalize_components(path: &Path) -> PathBuf {
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                components.pop();
            }
            std::path::Component::CurDir => {}
            other => components.push(other),
        }
    }
    components.iter().collect()
}

/// Compute a relative path from `base` to `target`.
fn diff_paths(target: &Path, base: &Path) -> Option<PathBuf> {
    let target = normalize_components(target);
    let base = normalize_components(base);

    // Both must be absolute
    if !target.is_absolute() || !base.is_absolute() {
        return None;
    }

    let mut target_components = target.components().peekable();
    let mut base_components = base.components().peekable();

    // Skip common prefix
    while let (Some(t), Some(b)) = (target_components.peek(), base_components.peek()) {
        if t == b {
            target_components.next();
            base_components.next();
        } else {
            break;
        }
    }

    // Add `..` for each remaining base component
    let mut result = PathBuf::new();
    for _ in base_components {
        result.push("..");
    }

    // Add remaining target components
    for component in target_components {
        result.push(component);
    }

    if result.as_os_str().is_empty() {
        Some(PathBuf::from("."))
    } else {
        Some(result)
    }
}
