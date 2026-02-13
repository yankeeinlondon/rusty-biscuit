use super::repo::detect_repo;
use crate::{Result, SniffError};
use biscuit_hash::xx_hash;
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Metadata for a markdown document in a git repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkdownMeta {
    /// Fully qualified file path to the document.
    pub filepath: PathBuf,
    /// Relative file path from the repo root directory.
    pub relative: String,
    /// The package in the monorepo this file resides in.
    /// None if not a monorepo or file is in the root folder.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package: Option<String>,
    /// Document title extracted from frontmatter "title" field,
    /// first H1, first H2, first H3, or empty string (in that priority).
    pub title: String,
    /// The frontmatter "model" property if defined.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// The frontmatter "prompt" property if defined.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    /// Last updated timestamp resolved from frontmatter or file mtime.
    pub last_updated: DateTime<Utc>,
    /// xxHash (XXH64) of the document body (content after frontmatter).
    pub content_hash: String,
}

/// Discovers and analyzes markdown documents in a git repository.
pub struct RepoDocuments {
    /// The git repository root directory.
    repo_root: PathBuf,
    /// Monorepo package directories (name, relative path from repo root).
    packages: Vec<(String, PathBuf)>,
}

impl RepoDocuments {
    /// Create a new `RepoDocuments` by discovering the git repo root from
    /// the given directory path.
    ///
    /// ## Errors
    ///
    /// Returns `SniffError::NotARepository` if the path is not inside a git repo.
    pub fn new(path: &Path) -> Result<Self> {
        let repo = git2::Repository::discover(path)
            .map_err(|_| SniffError::NotARepository(path.to_path_buf()))?;

        let repo_root = repo
            .workdir()
            .ok_or_else(|| SniffError::NotARepository(path.to_path_buf()))?
            .to_path_buf();

        // Leverage the existing repo detection to get monorepo package locations.
        // Package.path is absolute, so convert to relative for matching.
        let packages = detect_repo(&repo_root)
            .ok()
            .flatten()
            .and_then(|info| info.packages)
            .map(|pkgs| {
                pkgs.into_iter()
                    .map(|p| {
                        let rel_path =
                            p.path.strip_prefix(&repo_root).unwrap_or(&p.path).to_path_buf();
                        (p.name, rel_path)
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        Ok(Self {
            repo_root,
            packages,
        })
    }

    /// Returns metadata for all markdown documents in the repository.
    pub fn documents(&self) -> Vec<MarkdownMeta> {
        collect_markdown_files(&self.repo_root, &self.packages)
    }

    /// Returns metadata for markdown documents that have a "prompt" property set.
    pub fn prompt_docs(&self) -> Vec<MarkdownMeta> {
        self.documents().into_iter().filter(|doc| doc.prompt.is_some()).collect()
    }
}

/// Detect markdown documents in the given directory (standalone function).
///
/// This matches sniff's detection function pattern. Returns `None` if the
/// directory is not inside a git repository.
pub fn detect_docs(root: &Path) -> Option<Vec<MarkdownMeta>> {
    let repo_docs = RepoDocuments::new(root).ok()?;
    let docs = repo_docs.documents();
    if docs.is_empty() {
        None
    } else {
        Some(docs)
    }
}

/// Collect all markdown files from repo root using .gitignore-aware walking.
fn collect_markdown_files(repo_root: &Path, packages: &[(String, PathBuf)]) -> Vec<MarkdownMeta> {
    let walker = WalkBuilder::new(repo_root)
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .build();

    let mut docs: Vec<MarkdownMeta> = walker
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry.file_type().is_some_and(|ft| ft.is_file())
                && entry.path().extension().is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
        })
        .filter_map(|entry| parse_markdown_meta(entry.path(), repo_root, packages))
        .collect();

    docs.sort_by(|a, b| a.relative.cmp(&b.relative));
    docs
}

/// Parse a single markdown file into its metadata.
fn parse_markdown_meta(
    path: &Path,
    repo_root: &Path,
    packages: &[(String, PathBuf)],
) -> Option<MarkdownMeta> {
    let content = fs::read_to_string(path).ok()?;
    let (frontmatter, body) = extract_frontmatter(&content);

    let relative = path.strip_prefix(repo_root).ok()?.to_string_lossy().to_string();

    let relative_path = Path::new(&relative);
    let package = determine_package(relative_path, packages);
    let title = extract_title(&frontmatter, body);
    let model = get_string_field(&frontmatter, "model");
    let prompt = get_string_field(&frontmatter, "prompt");
    let last_updated = resolve_last_updated(&frontmatter, path);
    let content_hash = format!("{:x}", xx_hash(body));

    Some(MarkdownMeta {
        filepath: path.to_path_buf(),
        relative,
        package,
        title,
        model,
        prompt,
        last_updated,
        content_hash,
    })
}

/// Extract YAML frontmatter from markdown content.
///
/// Returns the parsed frontmatter map and the body (content after frontmatter).
/// If no valid frontmatter is found, returns an empty map and the full content.
fn extract_frontmatter(content: &str) -> (HashMap<String, serde_yaml::Value>, &str) {
    let empty = (HashMap::new(), content);

    if !content.starts_with("---") {
        return empty;
    }

    // Find the closing delimiter (skip the opening "---" line)
    let after_opening = match content.find('\n') {
        Some(idx) => idx + 1,
        None => return empty,
    };

    let rest = &content[after_opening..];

    // Find the closing "---" delimiter.
    // It can be at the start of `rest` (empty frontmatter) or after a newline.
    let (yaml_str, body_start) = if rest.starts_with("---") {
        ("", after_opening + 3)
    } else if let Some(offset) = rest.find("\n---") {
        (&rest[..offset], after_opening + offset + 4)
    } else {
        return empty;
    };

    // Skip the newline after closing delimiter if present
    let body = if body_start < content.len() {
        let remaining = &content[body_start..];
        remaining.strip_prefix('\n').unwrap_or(remaining)
    } else {
        ""
    };

    if yaml_str.trim().is_empty() {
        return (HashMap::new(), body);
    }

    match serde_yaml::from_str::<HashMap<String, serde_yaml::Value>>(yaml_str) {
        Ok(map) => (map, body),
        Err(_) => (HashMap::new(), content),
    }
}

/// Extract a string field from the frontmatter map.
fn get_string_field(frontmatter: &HashMap<String, serde_yaml::Value>, key: &str) -> Option<String> {
    frontmatter.get(key).and_then(|v| match v {
        serde_yaml::Value::String(s) => Some(s.clone()),
        serde_yaml::Value::Bool(b) => Some(b.to_string()),
        serde_yaml::Value::Number(n) => Some(format!("{n}")),
        _ => None,
    })
}

/// Extract the document title using priority:
/// 1. Frontmatter "title" field
/// 2. First H1 heading (`# ...`)
/// 3. First H2 heading (`## ...`)
/// 4. First H3 heading (`### ...`)
/// 5. Empty string
fn extract_title(frontmatter: &HashMap<String, serde_yaml::Value>, body: &str) -> String {
    // 1. Frontmatter title
    if let Some(serde_yaml::Value::String(title)) = frontmatter.get("title")
        && !title.is_empty()
    {
        return title.clone();
    }

    // 2-4. Search for headings in priority order
    let mut first_h2: Option<&str> = None;
    let mut first_h3: Option<&str> = None;

    for line in body.lines() {
        let trimmed = line.trim();
        if let Some(h1) = trimmed.strip_prefix("# ")
            && !h1.starts_with('#')
        {
            return h1.trim().to_string();
        }
        if first_h2.is_none()
            && let Some(h2) = trimmed.strip_prefix("## ")
            && !h2.starts_with('#')
        {
            first_h2 = Some(h2.trim());
        }
        if first_h3.is_none()
            && let Some(h3) = trimmed.strip_prefix("### ")
            && !h3.starts_with('#')
        {
            first_h3 = Some(h3.trim());
        }
    }

    if let Some(h2) = first_h2 {
        return h2.to_string();
    }
    if let Some(h3) = first_h3 {
        return h3.to_string();
    }

    String::new()
}

/// Resolve last updated timestamp from frontmatter or file metadata.
///
/// Priority:
/// 1. Frontmatter `last_updated` (date or datetime)
/// 2. Frontmatter `updated_at` (date or datetime)
/// 3. File system last modified time
fn resolve_last_updated(
    frontmatter: &HashMap<String, serde_yaml::Value>,
    path: &Path,
) -> DateTime<Utc> {
    // Try frontmatter fields in priority order
    for key in &[
        "last_updated",
        "updated_at",
    ] {
        if let Some(dt) = frontmatter.get(*key).and_then(parse_datetime_value) {
            return dt;
        }
    }

    // Fall back to file modification time
    file_mtime(path)
}

/// Try to parse a YAML value as a DateTime<Utc>.
fn parse_datetime_value(value: &serde_yaml::Value) -> Option<DateTime<Utc>> {
    let s = match value {
        serde_yaml::Value::String(s) => s.as_str(),
        _ => return None,
    };

    // Try RFC 3339 / ISO 8601 datetime
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Some(dt.with_timezone(&Utc));
    }

    // Try common datetime formats
    let datetime_formats = [
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%d %H:%M",
    ];
    for fmt in &datetime_formats {
        if let Ok(ndt) = NaiveDateTime::parse_from_str(s, fmt) {
            return Some(ndt.and_utc());
        }
    }

    // Try date-only formats (midnight UTC)
    let date_formats = [
        "%Y-%m-%d", "%Y/%m/%d",
    ];
    for fmt in &date_formats {
        if let Ok(nd) = NaiveDate::parse_from_str(s, fmt) {
            return nd.and_hms_opt(0, 0, 0).map(|ndt| ndt.and_utc());
        }
    }

    None
}

/// Get file modification time as DateTime<Utc>.
fn file_mtime(path: &Path) -> DateTime<Utc> {
    fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .map(DateTime::<Utc>::from)
        .unwrap_or_else(Utc::now)
}

/// Determine which monorepo package a file belongs to.
///
/// Matches the file's relative path against known package directories.
/// Returns `None` if the file is in the repo root or not in any package.
fn determine_package(relative_path: &Path, packages: &[(String, PathBuf)]) -> Option<String> {
    for (name, pkg_path) in packages {
        if relative_path.starts_with(pkg_path) {
            return Some(name.clone());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    mod frontmatter_parsing {
        use super::*;

        #[test]
        fn parses_valid_yaml_frontmatter() {
            let content = "---\ntitle: Hello\nmodel: gpt-4\n---\n# Body";
            let (fm, body) = extract_frontmatter(content);
            assert_eq!(fm.get("title").and_then(|v| v.as_str()), Some("Hello"));
            assert_eq!(fm.get("model").and_then(|v| v.as_str()), Some("gpt-4"));
            assert_eq!(body, "# Body");
        }

        #[test]
        fn returns_empty_map_for_no_frontmatter() {
            let content = "# Just a heading\n\nSome body text.";
            let (fm, body) = extract_frontmatter(content);
            assert!(fm.is_empty());
            assert_eq!(body, content);
        }

        #[test]
        fn returns_empty_map_for_unclosed_frontmatter() {
            let content = "---\ntitle: Test\n# No closing delimiter";
            let (fm, body) = extract_frontmatter(content);
            assert!(fm.is_empty());
            assert_eq!(body, content);
        }

        #[test]
        fn handles_empty_frontmatter() {
            let content = "---\n---\n# Content";
            let (fm, body) = extract_frontmatter(content);
            assert!(fm.is_empty());
            assert_eq!(body, "# Content");
        }

        #[test]
        fn handles_frontmatter_with_prompt() {
            let content = "---\ntitle: Test\nprompt: Generate a summary\n---\nBody text";
            let (fm, body) = extract_frontmatter(content);
            assert_eq!(fm.get("prompt").and_then(|v| v.as_str()), Some("Generate a summary"));
            assert_eq!(body, "Body text");
        }
    }

    mod title_extraction {
        use super::*;

        #[test]
        fn prefers_frontmatter_title() {
            let mut fm = HashMap::new();
            fm.insert("title".to_string(), serde_yaml::Value::String("FM Title".to_string()));
            assert_eq!(extract_title(&fm, "# Heading Title"), "FM Title");
        }

        #[test]
        fn falls_back_to_h1() {
            let fm = HashMap::new();
            assert_eq!(extract_title(&fm, "Some text\n# Main Title\n## Sub"), "Main Title");
        }

        #[test]
        fn falls_back_to_h2_when_no_h1() {
            let fm = HashMap::new();
            assert_eq!(extract_title(&fm, "Some text\n## Section Title\n### Sub"), "Section Title");
        }

        #[test]
        fn falls_back_to_h3_when_no_h1_or_h2() {
            let fm = HashMap::new();
            assert_eq!(extract_title(&fm, "Some text\n### Subsection"), "Subsection");
        }

        #[test]
        fn returns_empty_string_when_no_title() {
            let fm = HashMap::new();
            assert_eq!(extract_title(&fm, "Just some plain text."), "");
        }

        #[test]
        fn h2_not_confused_with_h1() {
            let fm = HashMap::new();
            // "## Title" should not match as H1
            assert_eq!(extract_title(&fm, "## Only H2\n### And H3"), "Only H2");
        }
    }

    mod date_resolution {
        use super::*;

        #[test]
        fn parses_iso_date() {
            let val = serde_yaml::Value::String("2025-06-15".to_string());
            let dt = parse_datetime_value(&val).unwrap();
            assert_eq!(dt.date_naive(), NaiveDate::from_ymd_opt(2025, 6, 15).unwrap());
        }

        #[test]
        fn parses_iso_datetime() {
            let val = serde_yaml::Value::String("2025-06-15T10:30:00Z".to_string());
            let dt = parse_datetime_value(&val).unwrap();
            assert_eq!(dt.date_naive(), NaiveDate::from_ymd_opt(2025, 6, 15).unwrap());
        }

        #[test]
        fn parses_datetime_without_timezone() {
            let val = serde_yaml::Value::String("2025-06-15 10:30:00".to_string());
            let dt = parse_datetime_value(&val).unwrap();
            assert_eq!(dt.date_naive(), NaiveDate::from_ymd_opt(2025, 6, 15).unwrap());
        }

        #[test]
        fn returns_none_for_invalid_date() {
            let val = serde_yaml::Value::String("not-a-date".to_string());
            assert!(parse_datetime_value(&val).is_none());
        }

        #[test]
        fn returns_none_for_non_string() {
            let val = serde_yaml::Value::Number(serde_yaml::Number::from(42));
            assert!(parse_datetime_value(&val).is_none());
        }
    }

    mod package_detection {
        use super::*;

        #[test]
        fn matches_file_to_package() {
            // Package paths are the actual member paths from detect_repo
            let packages = vec![
                ("research-lib".to_string(), PathBuf::from("research/lib")),
                ("research-cli".to_string(), PathBuf::from("research/cli")),
                ("sniff".to_string(), PathBuf::from("sniff/lib")),
            ];
            let path = Path::new("research/lib/docs/architecture.md");
            assert_eq!(determine_package(path, &packages), Some("research-lib".to_string()));
        }

        #[test]
        fn returns_none_for_root_file() {
            let packages = vec![("research-lib".to_string(), PathBuf::from("research/lib"))];
            let path = Path::new("README.md");
            assert_eq!(determine_package(path, &packages), None);
        }

        #[test]
        fn matches_correct_nested_package() {
            let packages = vec![
                ("sniff".to_string(), PathBuf::from("sniff/lib")),
                ("sniff-cli".to_string(), PathBuf::from("sniff/cli")),
            ];
            let path = Path::new("sniff/lib/docs/design.md");
            assert_eq!(determine_package(path, &packages), Some("sniff".to_string()));
        }

        #[test]
        fn file_between_packages_returns_none() {
            // A file at sniff/README.md doesn't belong to sniff/lib or sniff/cli
            let packages = vec![
                ("sniff".to_string(), PathBuf::from("sniff/lib")),
                ("sniff-cli".to_string(), PathBuf::from("sniff/cli")),
            ];
            let path = Path::new("sniff/README.md");
            assert_eq!(determine_package(path, &packages), None);
        }

        #[test]
        fn hidden_dirs_not_matched() {
            let packages = vec![("research-lib".to_string(), PathBuf::from("research/lib"))];
            let path = Path::new(".ai/plans/some-plan.md");
            assert_eq!(determine_package(path, &packages), None);
        }
    }

    mod content_hashing {
        use super::*;

        #[test]
        fn same_content_produces_same_hash() {
            let hash1 = xx_hash("hello world");
            let hash2 = xx_hash("hello world");
            assert_eq!(hash1, hash2);
        }

        #[test]
        fn different_content_produces_different_hash() {
            let hash1 = xx_hash("hello world");
            let hash2 = xx_hash("goodbye world");
            assert_ne!(hash1, hash2);
        }
    }

    mod integration {
        use super::*;

        #[test]
        fn packages_detected_in_workspace() {
            let repo = RepoDocuments::new(Path::new(".")).unwrap();
            assert!(
                !repo.packages.is_empty(),
                "Should detect workspace packages. repo_root: {:?}",
                repo.repo_root,
            );
        }

        #[test]
        fn documents_have_correct_packages() {
            let repo = RepoDocuments::new(Path::new(".")).unwrap();
            let docs = repo.documents();

            // Files inside a specific package member should be assigned
            let sniff_lib_docs: Vec<_> =
                docs.iter().filter(|d| d.relative.starts_with("sniff/lib/")).collect();
            for doc in &sniff_lib_docs {
                assert!(
                    doc.package.is_some(),
                    "Doc {} should have a package assigned",
                    doc.relative,
                );
            }

            // Verify some docs are assigned to packages (not all root)
            let with_package = docs.iter().filter(|d| d.package.is_some()).count();
            assert!(with_package > 0, "At least some documents should have package assignments");
        }

        #[test]
        fn repo_documents_from_current_dir() {
            // This test runs within the rusty-biscuit repo
            let repo = RepoDocuments::new(Path::new("."));
            assert!(repo.is_ok(), "Should detect git repo from current dir");
            let repo = repo.unwrap();
            let docs = repo.documents();
            assert!(!docs.is_empty(), "Should find markdown documents in repo");
        }

        #[test]
        fn all_docs_have_relative_paths() {
            let repo = RepoDocuments::new(Path::new(".")).unwrap();
            for doc in repo.documents() {
                assert!(
                    !doc.relative.starts_with('/'),
                    "Relative path should not start with /: {}",
                    doc.relative
                );
            }
        }

        #[test]
        fn all_docs_have_content_hash() {
            let repo = RepoDocuments::new(Path::new(".")).unwrap();
            for doc in repo.documents() {
                assert!(
                    !doc.content_hash.is_empty(),
                    "Content hash should not be empty for: {}",
                    doc.relative
                );
            }
        }

        #[test]
        fn detect_docs_returns_some_in_repo() {
            let docs = detect_docs(Path::new("."));
            assert!(docs.is_some(), "detect_docs should return Some in a git repo");
        }
    }
}
