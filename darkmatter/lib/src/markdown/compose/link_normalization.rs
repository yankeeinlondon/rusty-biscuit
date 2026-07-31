//! Link Normalization operation for the compose pipeline.
//!
//! Converts absolute paths back to portable forms in the Finalization stage.
//! Runs only on the root document.

use std::path::{Path, PathBuf};

use crate::markdown::Markdown;
use crate::markdown::compose::{ComposeOptions, ComposeReport, ComposeSource};
use crate::markdown::reference::{
    ReferenceKind, ReferenceTarget,
    html::{
        extract_html_audio, extract_html_iframes, extract_html_images, extract_html_link_tags,
        extract_html_links, extract_html_sources, extract_html_videos,
    },
    local::{extract_markdown_images, extract_markdown_links},
};
use crate::markdown::types::MarkdownResult;

fn comparison_path(path: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        PathBuf::from(super::path_to_markdown(path))
    }
    #[cfg(not(windows))]
    {
        path.to_path_buf()
    }
}

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
    records.extend(crate::markdown::reference::html::extract_html_script_blocks(content, &source));

    let mut to_normalize = Vec::new();

    for record in records {
        let mut abs_path = None;
        let mut raw_abs = None;
        if let ReferenceTarget::RemoteUrl { .. } = &record.target {
            continue;
        }
        if let ReferenceTarget::LocalPath { raw } = &record.target {
            if raw.is_absolute() {
                raw_abs = Some(raw.clone());
            } else if let ComposeSource::File(path) = &source
                && let Some(parent) = path.parent()
            {
                let joined = parent.join(raw);
                raw_abs = std::fs::canonicalize(&joined).ok().or(Some(joined));
            }
        }
        if let Some(raw) = raw_abs
            && raw.is_absolute()
        {
            abs_path = Some(raw.clone());
        }

        if let Some(abs_path) = abs_path {
            match record.kind {
                ReferenceKind::Hyperlink
                | ReferenceKind::Image
                | ReferenceKind::HtmlVideo
                | ReferenceKind::HtmlAudio
                | ReferenceKind::HtmlSource
                | ReferenceKind::HtmlIframe
                | ReferenceKind::ScriptImport
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
        ComposeSource::File(path) => match std::fs::canonicalize(path) {
            Ok(p) => Some(comparison_path(&p)),
            Err(e) => {
                report.add_warning(crate::markdown::compose::ComposeWarning::new(
                    "link_normalization",
                    format!(
                        "Failed to canonicalize source path '{}': {}",
                        path.display(),
                        e
                    ),
                ));
                return Ok(());
            }
        },
        _ => None,
    };

    let base_dir = base_file
        .as_ref()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()));
    let git_root = match options.file_resolution_context.as_ref() {
        Some(context) => context.repository_root().map(Path::to_path_buf),
        None => base_dir.as_ref().and_then(|d| super::find_git_root_from(d)),
    }
    .map(|r| std::fs::canonicalize(&r).unwrap_or(r))
    .map(|r| comparison_path(&r));
    let home = match options.file_resolution_context.as_ref() {
        Some(context) => context.home_dir().map(Path::to_path_buf),
        None => dirs::home_dir(),
    }
    .map(|path| std::fs::canonicalize(&path).unwrap_or(path))
    .map(|path| comparison_path(&path));

    for (record, abs_path) in to_normalize {
        let comparable_abs = comparison_path(
            &std::fs::canonicalize(&abs_path).unwrap_or_else(|_| abs_path.clone()),
        );
        let mut replacement = None;

        // 3.6 Same-repo rule
        if let Some(ref repo) = git_root
            && comparable_abs.starts_with(repo)
            && let Some(ref doc_path) = base_file
        {
            let rel = compute_relative_path(doc_path, &comparable_abs);

            replacement = Some(super::path_to_markdown(&rel));
        }

        if replacement.is_none()
            && let Some(ref h) = home
            && comparable_abs.starts_with(h)
            && let Ok(rel) = comparable_abs.strip_prefix(h)
        {
            replacement = Some(format!("~/{}", super::path_to_markdown(rel)));
        }

        // 3.8 ENV-var rule
        if replacement.is_none() {
            let whitelist = options.effective_env_path_whitelist();
            let mut best_var = None;
            let mut longest_len = 0;

            for var_name in whitelist {
                let val = match options.file_resolution_context.as_ref() {
                    Some(context) => context.env().get(&var_name).cloned(),
                    None => std::env::var(&var_name).ok(),
                };
                if let Some(val) = val {
                    let var_path = PathBuf::from(val);
                    let var_path = std::fs::canonicalize(&var_path).unwrap_or(var_path);
                    let var_path = comparison_path(&var_path);
                    if comparable_abs.starts_with(&var_path) {
                        let path_len = var_path.as_os_str().len();
                        if path_len > longest_len {
                            longest_len = path_len;
                            best_var = Some((var_name, var_path));
                        }
                    }
                }
            }

            if let Some((var_name, var_path)) = best_var
                && let Ok(rel) = comparable_abs.strip_prefix(&var_path)
            {
                let env_replacement =
                    format!("${{{}}}/{}", var_name, super::path_to_markdown(rel));

                // 3.9 Emit warning
                let msg = format!(
                    "the path <blue>{}</blue> was found to be an offset of the <b>{}</b> environment variable and will use this abstraction.",
                    abs_path.display(),
                    var_name
                );
                report.add_warning(crate::markdown::compose::ComposeWarning::new(
                    "link_normalization",
                    msg,
                ));

                replacement = Some(env_replacement);
            }
        }

        if let Some(new_target) = replacement
            && let Some((start, end)) =
                super::find_target_range(&new_content, &record, &abs_path.to_string_lossy())
        {
            new_content.replace_range(start..end, &new_target);
            applied_count += 1;
        }
    }

    report.link_normalizations_applied += applied_count;
    if applied_count > 0 {
        *markdown.content_mut() = new_content;
    }

    Ok(())
}

fn compute_relative_path(from: &Path, to: &Path) -> PathBuf {
    let mut from_can = from.to_path_buf();
    if from_can.to_string_lossy().starts_with("/private/") {
        from_can = PathBuf::from(&from_can.to_string_lossy()[8..]);
    }

    let mut to_can = to.to_path_buf();
    if to_can.to_string_lossy().starts_with("/private/") {
        to_can = PathBuf::from(&to_can.to_string_lossy()[8..]);
    }

    let from_dir = if from_can.extension().is_some() {
        from_can.parent().unwrap_or(std::path::Path::new(""))
    } else {
        &from_can
    };

    match diff_paths(&to_can, from_dir) {
        Some(p) => p,
        None => to_can,
    }
}

fn diff_paths(target: &Path, base: &Path) -> Option<PathBuf> {
    let target_components: Vec<_> = target.components().collect();
    let base_components: Vec<_> = base.components().collect();
    let mut common_idx = 0;
    while common_idx < target_components.len() && common_idx < base_components.len() {
        if target_components[common_idx] == base_components[common_idx] {
            common_idx += 1;
        } else {
            break;
        }
    }
    let mut result = PathBuf::new();
    for _ in common_idx..base_components.len() {
        result.push("..");
    }
    for component in target_components.iter().skip(common_idx) {
        result.push(component);
    }

    if result.as_os_str().is_empty() {
        Some(PathBuf::from("."))
    } else {
        Some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::compose::ComposeReport;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_normalize_links_same_repo() {
        let dir = tempdir().unwrap();
        let repo = dir.path().join("repo");
        fs::create_dir_all(repo.join(".git")).unwrap();
        let docs = repo.join("docs");
        let assets = repo.join("assets");

        fs::create_dir_all(&docs).unwrap();
        fs::create_dir_all(&assets).unwrap();
        let source_file = docs.join("source.md");
        fs::write(&source_file, "").unwrap();
        let target_file = assets.join("image.png");
        fs::write(&target_file, "png").unwrap();
        let abs_path = std::fs::canonicalize(&target_file).unwrap();
        let content = format!("![img]({})\n", super::super::path_to_markdown(&abs_path));
        let mut md = Markdown::new(&content);
        let options = ComposeOptions::new().with_source_file(&source_file);
        let mut report = ComposeReport::new();
        normalize_links(&mut md, &options, &mut report).unwrap();
        assert!(
            md.content().contains("../assets/image.png"),
            "Content was: {}",
            md.content()
        );
        assert_eq!(report.link_normalizations_applied, 1);
    }

    #[test]
    fn test_normalize_links_home_dir() {
        let home = dirs::home_dir().expect("Has home dir");
        let target = home.join("some_file.txt");
        let content = format!("[file]({})\n", super::super::path_to_markdown(&target));
        let mut md = Markdown::new(&content);
        let options = ComposeOptions::new();
        let mut report = ComposeReport::new();
        normalize_links(&mut md, &options, &mut report).unwrap();
        assert!(
            md.content().contains("~/some_file.txt"),
            "Content was: {}",
            md.content()
        );
        assert_eq!(report.link_normalizations_applied, 1);
    }

    #[test]
    fn test_normalize_links_env_var() {
        let dir = tempdir().unwrap();
        let project_root = dir.path().join("project");
        fs::create_dir_all(&project_root).unwrap();
        let target = project_root.join("config.json");
        fs::write(&target, "{}").unwrap();
        let abs_path = match std::fs::canonicalize(&target) {
            Ok(p) => {
                if p.to_string_lossy().starts_with("/private/") {
                    PathBuf::from(&p.to_string_lossy()[8..])
                } else {
                    p
                }
            }
            Err(_) => target.clone(),
        };
        let canonical_root = std::fs::canonicalize(&project_root).unwrap();
        let mut env = std::collections::HashMap::new();
        env.insert(
            "PROJECT_ROOT".to_string(),
            canonical_root.to_string_lossy().into_owned(),
        );
        let snapshot = biscuit_file::FileResolutionContext::new(&project_root)
            .without_home_dir()
            .with_env(env);
        let content = format!(
            "<a href=\"{}\">config</a>\n",
            super::super::path_to_markdown(&abs_path)
        );
        let mut md = Markdown::new(&content);
        let options = ComposeOptions::new()
            .with_env_path_whitelist(vec!["PROJECT_ROOT".to_string()])
            .with_file_resolution_context(snapshot);
        let mut report = ComposeReport::new();
        normalize_links(&mut md, &options, &mut report).unwrap();
        assert!(
            md.content().contains("${PROJECT_ROOT}/config.json"),
            "Content was: {}",
            md.content()
        );
        assert_eq!(report.link_normalizations_applied, 1);
    }

    #[test]
    fn link_normalization_reuses_snapshot_environment() {
        let dir = tempdir().unwrap();
        let root = std::fs::canonicalize(dir.path()).unwrap();
        let target = root.join("config.json");
        fs::write(&target, "{}").unwrap();
        let content = format!(
            "[config]({})\n",
            super::super::path_to_markdown(&target)
        );
        let mut md = Markdown::new(&content);
        let mut env = std::collections::HashMap::new();
        env.insert("CAPTURED_ROOT".to_string(), root.display().to_string());
        let snapshot = biscuit_file::FileResolutionContext::new(&root)
            .without_home_dir()
            .with_env(env);
        let options = ComposeOptions::new()
            .with_env_path_whitelist(vec!["CAPTURED_ROOT".to_string()])
            .with_file_resolution_context(snapshot);
        let mut report = ComposeReport::new();

        normalize_links(&mut md, &options, &mut report).unwrap();

        assert!(md.content().contains("${CAPTURED_ROOT}/config.json"));
        assert_eq!(report.link_normalizations_applied, 1);
    }

    #[test]
    fn test_normalize_links_css_font_script() {
        let dir = tempdir().unwrap();
        let repo = dir.path().join("repo");
        fs::create_dir_all(repo.join(".git")).unwrap();
        let docs = repo.join("docs");
        let assets = repo.join("assets");

        fs::create_dir_all(&docs).unwrap();
        fs::create_dir_all(&assets).unwrap();

        let source_file = docs.join("source.md");
        fs::write(&source_file, "").unwrap();

        let target_css = assets.join("styles.css");
        let target_font = assets.join("font.woff2");
        let target_script = assets.join("app.js");
        fs::write(&target_css, "").unwrap();
        fs::write(&target_font, "").unwrap();
        fs::write(&target_script, "").unwrap();

        let abs_css = std::fs::canonicalize(&target_css).unwrap();
        let abs_font = std::fs::canonicalize(&target_font).unwrap();
        let abs_script = std::fs::canonicalize(&target_script).unwrap();

        let content = format!(
            "<link rel=\"stylesheet\" href=\"{}\">\n<link rel=\"preload\" as=\"font\" href=\"{}\">\n<script src=\"{}\"></script>",
            super::super::path_to_markdown(&abs_css),
            super::super::path_to_markdown(&abs_font),
            super::super::path_to_markdown(&abs_script)
        );
        let mut md = Markdown::new(&content);
        let options = ComposeOptions::new().with_source_file(&source_file);
        let mut report = ComposeReport::new();

        normalize_links(&mut md, &options, &mut report).unwrap();

        assert!(
            md.content().contains("../assets/styles.css"),
            "CSS failed. Content: {}",
            md.content()
        );
        assert!(
            md.content().contains("../assets/font.woff2"),
            "Font failed. Content: {}",
            md.content()
        );
        assert!(
            md.content().contains("../assets/app.js"),
            "Script failed. Content: {}",
            md.content()
        );
        assert_eq!(report.link_normalizations_applied, 3);
    }

    #[test]
    fn test_normalize_links_deep_nesting() {
        let dir = tempdir().unwrap();
        let repo = dir.path().join("repo");
        fs::create_dir_all(repo.join(".git")).unwrap();

        let docs = repo.join("docs").join("deep").join("nested").join("dir");
        let assets = repo.join("assets").join("images");

        fs::create_dir_all(&docs).unwrap();
        fs::create_dir_all(&assets).unwrap();

        let source_file = docs.join("source.md");
        fs::write(&source_file, "").unwrap();

        let target_file = assets.join("image.png");
        fs::write(&target_file, "png").unwrap();

        let same_dir_file = docs.join("sibling.md");
        fs::write(&same_dir_file, "md").unwrap();

        let abs_img = std::fs::canonicalize(&target_file).unwrap();
        let abs_sibling = std::fs::canonicalize(&same_dir_file).unwrap();

        let content = format!(
            "[img]({})\n[sibling]({})",
            super::super::path_to_markdown(&abs_img),
            super::super::path_to_markdown(&abs_sibling)
        );
        let mut md = Markdown::new(&content);
        let options = ComposeOptions::new().with_source_file(&source_file);
        let mut report = ComposeReport::new();

        normalize_links(&mut md, &options, &mut report).unwrap();

        assert!(md.content().contains("../../../../assets/images/image.png"));
        assert!(md.content().contains("sibling.md") || md.content().contains("./sibling.md"));
        assert_eq!(report.link_normalizations_applied, 2);
    }

    #[test]
    fn test_normalize_links_env_var_specificity() {
        let dir = tempdir().unwrap();
        let parent = dir.path().join("parent");
        let child = parent.join("child");
        fs::create_dir_all(&child).unwrap();

        let target = child.join("config.json");
        fs::write(&target, "{}").unwrap();

        let abs_path = std::fs::canonicalize(&target).unwrap_or(target);
        let abs_parent = std::fs::canonicalize(&parent).unwrap_or(parent);
        let abs_child = std::fs::canonicalize(&child).unwrap_or_else(|_| child.clone());

        let mut env = std::collections::HashMap::new();
        env.insert(
            "USER".to_string(),
            abs_parent.to_string_lossy().into_owned(),
        );
        env.insert(
            "USER_NAME".to_string(),
            abs_child.to_string_lossy().into_owned(),
        );
        let snapshot = biscuit_file::FileResolutionContext::new(&child)
            .without_home_dir()
            .with_env(env);

        let content = format!(
            "[config]({})",
            super::super::path_to_markdown(&abs_path)
        );
        let mut md = Markdown::new(&content);
        let options = ComposeOptions::new()
            .with_env_path_whitelist(vec!["USER".to_string(), "USER_NAME".to_string()])
            .with_file_resolution_context(snapshot);
        let mut report = ComposeReport::new();

        normalize_links(&mut md, &options, &mut report).unwrap();

        // Should use the longer match USER_NAME
        assert!(
            md.content().contains("${USER_NAME}/config.json"),
            "Content was: {}",
            md.content()
        );
        assert_eq!(report.link_normalizations_applied, 1);
    }

    #[test]
    fn test_normalize_links_edge_cases() {
        let dir = tempdir().unwrap();
        let repo = dir.path().join("repo");
        fs::create_dir_all(repo.join(".git")).unwrap();

        let source_file = repo.join("source.md");
        fs::write(&source_file, "").unwrap();

        let parens_file = repo.join("with (parens).md");
        let quotes_file = repo.join("single_quotes.md");
        fs::write(&parens_file, "").unwrap();
        fs::write(&quotes_file, "").unwrap();

        let abs_parens = std::fs::canonicalize(&parens_file).unwrap();
        let abs_quotes = std::fs::canonicalize(&quotes_file).unwrap();

        let content = format!(
            "[link](<{}>)\n<img src='{}'>\n<a href=\"{}\" data-alt='{}'>link</a>",
            super::super::path_to_markdown(&abs_parens),
            super::super::path_to_markdown(&abs_quotes),
            super::super::path_to_markdown(&abs_quotes),
            super::super::path_to_markdown(&abs_quotes)
        );

        let mut md = Markdown::new(&content);
        let options = ComposeOptions::new().with_source_file(&source_file);
        let mut report = ComposeReport::new();

        normalize_links(&mut md, &options, &mut report).unwrap();

        assert!(
            md.content().contains("(<with (parens).md>)"),
            "Parens failed. Content: {}",
            md.content()
        );
        assert!(
            md.content().contains("'single_quotes.md'"),
            "Quotes failed. Content: {}",
            md.content()
        );
        assert!(
            md.content().contains("\"single_quotes.md\""),
            "Mixed failed. Content: {}",
            md.content()
        );
        assert_eq!(report.link_normalizations_applied, 3);
    }

    #[test]
    fn test_normalize_links_html_spaced_attributes() {
        let dir = tempdir().unwrap();
        let repo = dir.path().join("repo");
        fs::create_dir_all(repo.join(".git")).unwrap();
        let docs = repo.join("docs");
        let assets = repo.join("assets");

        fs::create_dir_all(&docs).unwrap();
        fs::create_dir_all(&assets).unwrap();

        let source_file = docs.join("source.md");
        fs::write(&source_file, "").unwrap();

        let target_img = assets.join("image.png");
        let target_video = assets.join("movie.mp4");
        let target_css = assets.join("styles.css");
        fs::write(&target_img, "png").unwrap();
        fs::write(&target_video, "video").unwrap();
        fs::write(&target_css, "body {}").unwrap();

        let abs_img = std::fs::canonicalize(&target_img).unwrap();
        let abs_video = std::fs::canonicalize(&target_video).unwrap();
        let abs_css = std::fs::canonicalize(&target_css).unwrap();

        let content = format!(
            "<a href = \"{}\">link</a>\n<img src = \"{}\">\n<video src = \"{}\"></video>\n<link href = \"{}\">",
            super::super::path_to_markdown(&abs_img),
            super::super::path_to_markdown(&abs_img),
            super::super::path_to_markdown(&abs_video),
            super::super::path_to_markdown(&abs_css)
        );
        let mut md = Markdown::new(&content);
        let options = ComposeOptions::new().with_source_file(&source_file);
        let mut report = ComposeReport::new();

        normalize_links(&mut md, &options, &mut report).unwrap();

        assert!(
            md.content().contains("../assets/image.png"),
            "Spaced anchor href failed. Content: {}",
            md.content()
        );
        assert!(
            md.content().contains("\"../assets/image.png\""),
            "Spaced img src failed. Content: {}",
            md.content()
        );
        assert!(
            md.content().contains("\"../assets/movie.mp4\""),
            "Spaced video src failed. Content: {}",
            md.content()
        );
        assert!(
            md.content().contains("\"../assets/styles.css\""),
            "Spaced link href failed. Content: {}",
            md.content()
        );
        assert_eq!(report.link_normalizations_applied, 4);
    }

    #[test]
    fn test_normalize_links_preserves_remote_urls() {
        let content = "[link](https://example.com/page) and ![img](http://cdn.example.com/img.png)";
        let mut md = Markdown::new(content);
        let options = ComposeOptions::new();
        let mut report = ComposeReport::new();

        normalize_links(&mut md, &options, &mut report).unwrap();

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
        assert_eq!(report.link_normalizations_applied, 0);
    }
}
