//! URL discovery and remote-read configuration for transclusion targets.
//!
//! This module provides the types and discovery functions needed to find
//! remote URL dependencies in a markdown document before any network access
//! occurs. It performs **no I/O** — only cataloging and validation.

use std::collections::HashSet;
use std::path::PathBuf;

use thiserror::Error;

use crate::markdown::compose::ComposeSource;
use crate::markdown::compose::transclusion::BlockDirective;

/// What kind of directive or expression is consuming a remote URL.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteUrlConsumer {
    /// From `::file` or `::url` directive.
    TransclusionFile,
    /// From `::code` directive.
    TransclusionCode,
    /// From expression functions like `frontmatter(url)`, `file_exists(url)`.
    ExpressionFunction,
    /// From `::toc-linking` directive.
    TocLinking,
}

/// A single discovered remote URL dependency.
#[derive(Debug, Clone)]
pub struct DiscoveredRemoteUrl {
    /// The parsed, normalized URL.
    pub url: url::Url,
    /// The normalized string form (from `Url::to_string()`).
    pub normalized_key: String,
    /// What kind of consumer found this URL.
    pub consumer: RemoteUrlConsumer,
    /// File where the URL reference was found.
    pub source_file: Option<PathBuf>,
    /// 1-based line number where the URL reference appeared.
    pub line: usize,
}

/// How to handle cache staleness for remote artifacts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RemoteFreshnessMode {
    /// Serve cache without revalidation when within TTL.
    Optimistic,
    /// Always revalidate with conditional GET.
    #[default]
    Strict,
    /// Serve stale on network failure.
    Fallback,
}

/// Configuration for remote URL reads.
///
/// Controls which hosts may be fetched, concurrency limits, and
/// cache-freshness behavior. The default policy denies all remote reads.
///
/// ## Examples
///
/// ```rust
/// use darkmatter::markdown::compose::remote::RemoteReadConfig;
///
/// let config = RemoteReadConfig::default();
/// assert!(!config.is_host_allowed("example.com"));
/// ```
#[derive(Debug, Clone)]
pub struct RemoteReadConfig {
    /// List of allowed host patterns (empty = deny all).
    pub allowed_hosts: Vec<String>,
    /// Maximum concurrent remote fetches.
    pub remote_concurrency: usize,
    /// TTL override for remote artifacts (`None` = use server-provided).
    pub remote_ttl: Option<std::time::Duration>,
    /// Force revalidation even when cached content is fresh.
    pub refresh: bool,
    /// How to handle cache staleness.
    pub freshness_mode: RemoteFreshnessMode,
}

impl Default for RemoteReadConfig {
    fn default() -> Self {
        Self {
            allowed_hosts: Vec::new(),
            remote_concurrency: 4,
            remote_ttl: None,
            refresh: false,
            freshness_mode: RemoteFreshnessMode::Strict,
        }
    }
}

impl RemoteReadConfig {
    /// Checks whether `host` is in the allowlist (case-insensitive exact match).
    pub fn is_host_allowed(&self, host: &str) -> bool {
        let lower = host.to_ascii_lowercase();
        self.allowed_hosts
            .iter()
            .any(|h| h.to_ascii_lowercase() == lower)
    }
}

/// Errors from remote URL validation or policy checks.
#[derive(Debug, Clone, Error)]
pub enum RemoteReadError {
    /// Remote reads disabled or host not in the allowlist.
    #[error("remote read denied for host '{host}'")]
    DeniedByPolicy {
        /// The rejected host.
        host: String,
    },
    /// URL validation failure.
    #[error("invalid URL '{url}': {reason}")]
    InvalidUrl {
        /// The raw URL string that failed validation.
        url: String,
        /// Why the URL is invalid.
        reason: String,
    },
}

/// Collects and deduplicates discovered remote URLs.
#[derive(Debug, Clone, Default)]
pub struct RemoteUrlCatalog {
    entries: Vec<DiscoveredRemoteUrl>,
}

impl RemoteUrlCatalog {
    /// Creates an empty catalog.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a discovery (does NOT deduplicate yet).
    pub fn add(&mut self, entry: DiscoveredRemoteUrl) {
        self.entries.push(entry);
    }

    /// Returns deduplicated entries by `normalized_key`, keeping the first
    /// occurrence.
    pub fn deduplicated(&self) -> Vec<&DiscoveredRemoteUrl> {
        let mut seen = HashSet::new();
        self.entries
            .iter()
            .filter(|e| seen.insert(e.normalized_key.clone()))
            .collect()
    }

    /// Returns `true` if no entries have been added.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Total entries before dedup.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Unique entry count after dedup.
    pub fn unique_count(&self) -> usize {
        self.deduplicated().len()
    }

    /// Returns deduplicated URLs.
    pub fn urls(&self) -> Vec<url::Url> {
        self.deduplicated()
            .iter()
            .map(|e| e.url.clone())
            .collect()
    }
}

/// Scans parsed block directives for URL targets.
///
/// Looks for directives whose `raw_target` starts with `http://` or
/// `https://` and maps them to the appropriate [`RemoteUrlConsumer`] based
/// on directive kind.
pub fn discover_remote_urls_from_directives(
    directives: &[BlockDirective],
    source: &ComposeSource,
) -> Vec<DiscoveredRemoteUrl> {
    let source_file = match source {
        ComposeSource::File(p) => Some(p.clone()),
        _ => None,
    };

    directives
        .iter()
        .filter(|d| is_http_url(&d.raw_target))
        .filter_map(|d| {
            validate_url_for_remote_read(&d.raw_target).ok().map(|url| {
                let consumer = match d.kind {
                    crate::markdown::compose::transclusion::DirectiveKind::File
                    | crate::markdown::compose::transclusion::DirectiveKind::Url => {
                        RemoteUrlConsumer::TransclusionFile
                    }
                    crate::markdown::compose::transclusion::DirectiveKind::Code => {
                        RemoteUrlConsumer::TransclusionCode
                    }
                };
                DiscoveredRemoteUrl {
                    normalized_key: url.to_string(),
                    url,
                    consumer,
                    source_file: source_file.clone(),
                    line: d.line,
                }
            })
        })
        .collect()
}

/// Scans interpolation expression content for URL arguments to known
/// filesystem functions.
///
/// This is a best-effort scan that looks for function calls like
/// `frontmatter(http://…)`, `file_exists(https://…)`, etc. It does not
/// perform full expression parsing.
pub fn discover_remote_urls_from_expressions(
    content: &str,
    source: &ComposeSource,
) -> Vec<DiscoveredRemoteUrl> {
    let source_file = match source {
        ComposeSource::File(p) => Some(p.clone()),
        _ => None,
    };

    let mut results = Vec::new();
    let functions = [
        "frontmatter",
        "file_exists",
        "markdown_title",
        "markdown_body_empty",
        "validate_schema",
        "absolute",
        "relative",
    ];

    for line in content.lines() {
        let line_num = find_line_number(content, line);
        for func in &functions {
            scan_function_url(line, func, &source_file, line_num, &mut results);
        }
    }

    results
}

/// Convenience function that calls both directive and expression discovery
/// and returns a [`RemoteUrlCatalog`].
pub fn discover_all_remote_urls(
    directives: &[BlockDirective],
    content: &str,
    source: &ComposeSource,
) -> RemoteUrlCatalog {
    let mut catalog = RemoteUrlCatalog::new();

    for entry in discover_remote_urls_from_directives(directives, source) {
        catalog.add(entry);
    }

    for entry in discover_remote_urls_from_expressions(content, source) {
        catalog.add(entry);
    }

    catalog
}

/// Validates a raw URL string for use in remote reads.
///
/// Ensures the string parses as a URL, has `http` or `https` scheme, and
/// has a host component.
pub fn validate_url_for_remote_read(raw: &str) -> Result<url::Url, RemoteReadError> {
    let url = url::Url::parse(raw).map_err(|e| RemoteReadError::InvalidUrl {
        url: raw.to_string(),
        reason: e.to_string(),
    })?;

    match url.scheme() {
        "http" | "https" => {}
        other => {
            return Err(RemoteReadError::InvalidUrl {
                url: raw.to_string(),
                reason: format!("unsupported scheme '{other}' (expected http or https)"),
            })
        }
    }

    if url.host_str().is_none_or(|h| h.is_empty()) {
        return Err(RemoteReadError::InvalidUrl {
            url: raw.to_string(),
            reason: "missing host".to_string(),
        });
    }

    Ok(url)
}

/// Checks that a URL's host is allowed by the remote-read policy.
pub fn check_remote_read_allowed(
    config: &RemoteReadConfig,
    url: &url::Url,
) -> Result<(), RemoteReadError> {
    let host = url
        .host_str()
        .ok_or_else(|| RemoteReadError::InvalidUrl {
            url: url.to_string(),
            reason: "missing host".to_string(),
        })?;

    if config.is_host_allowed(host) {
        Ok(())
    } else {
        Err(RemoteReadError::DeniedByPolicy {
            host: host.to_string(),
        })
    }
}

fn is_http_url(s: &str) -> bool {
    s.starts_with("http://") || s.starts_with("https://")
}

fn scan_function_url(
    line: &str,
    func_name: &str,
    source_file: &Option<PathBuf>,
    line_num: usize,
    results: &mut Vec<DiscoveredRemoteUrl>,
) {
    let mut search_from = 0;
    let pattern = format!("{func_name}(");

    while let Some(offset) = line[search_from..].find(&pattern) {
        let abs_offset = search_from + offset + pattern.len();
        // The interpolation expression parser only accepts a quoted string
        // literal for the URL argument, so the canonical authoring form is
        // `frontmatter("https://…")`. Skip an optional opening quote so the
        // quoted form is registered; the bare form is still recognized for
        // best-effort discovery. `extract_url_arg` stops at the closing quote.
        let rest = line[abs_offset..].trim_start_matches(['"', '\'']);

        let url_str = if rest.starts_with("https://") {
            extract_url_arg(rest, "https://")
        } else if rest.starts_with("http://") {
            extract_url_arg(rest, "http://")
        } else {
            search_from = abs_offset;
            continue;
        };

        if let Some(url_str) = url_str
            && let Ok(url) = validate_url_for_remote_read(&url_str)
        {
            results.push(DiscoveredRemoteUrl {
                normalized_key: url.to_string(),
                url,
                consumer: RemoteUrlConsumer::ExpressionFunction,
                source_file: source_file.clone(),
                line: line_num,
            });
        }

        search_from = abs_offset;
    }
}

fn extract_url_arg(rest: &str, prefix: &str) -> Option<String> {
    let url_start = prefix.len();
    let mut end = url_start;
    for ch in rest[url_start..].chars() {
        if ch == ')' || ch == ',' || ch.is_whitespace() || ch == '"' || ch == '\'' {
            break;
        }
        end += ch.len_utf8();
    }
    let candidate = &rest[..end];
    if candidate.len() > prefix.len() {
        Some(candidate.to_string())
    } else {
        None
    }
}

fn find_line_number(content: &str, target_line: &str) -> usize {
    content
        .lines()
        .enumerate()
        .find(|(_, l)| std::ptr::eq(l.as_ptr(), target_line.as_ptr()))
        .map(|(i, _)| i + 1)
        .unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::compose::transclusion::{BlockOptions, DirectiveKind};
    use std::ops::Range;

    fn file_source() -> ComposeSource {
        ComposeSource::File(PathBuf::from("/test/doc.md"))
    }

    fn unknown_source() -> ComposeSource {
        ComposeSource::Unknown
    }

    fn make_directive(
        kind: DirectiveKind,
        raw_target: &str,
        line: usize,
    ) -> BlockDirective {
        BlockDirective {
            kind,
            raw_target: raw_target.to_string(),
            options: BlockOptions::default(),
            span: Range::default(),
            line,
        }
    }

    #[test]
    fn discover_from_file_directive_with_http_target() {
        let directives = vec![
            make_directive(DirectiveKind::File, "https://example.com/doc.md", 5),
        ];
        let results = discover_remote_urls_from_directives(&directives, &file_source());
        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].url.as_str(),
            "https://example.com/doc.md"
        );
        assert_eq!(results[0].consumer, RemoteUrlConsumer::TransclusionFile);
        assert_eq!(results[0].line, 5);
        assert_eq!(
            results[0].source_file.as_ref().map(|p| p.to_string_lossy().to_string()),
            Some("/test/doc.md".to_string())
        );
    }

    #[test]
    fn discover_from_code_directive_with_http_target() {
        let directives = vec![
            make_directive(DirectiveKind::Code, "https://example.com/snippet.rs", 10),
        ];
        let results = discover_remote_urls_from_directives(&directives, &file_source());
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].consumer, RemoteUrlConsumer::TransclusionCode);
    }

    #[test]
    fn discover_from_url_directive() {
        let directives = vec![
            make_directive(DirectiveKind::Url, "https://example.com/doc.md", 3),
        ];
        let results = discover_remote_urls_from_directives(&directives, &file_source());
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].consumer, RemoteUrlConsumer::TransclusionFile);
    }

    #[test]
    fn skip_local_file_directives() {
        let directives = vec![
            make_directive(DirectiveKind::File, "./local.md", 1),
            make_directive(DirectiveKind::Code, "/abs/path.rs", 2),
        ];
        let results = discover_remote_urls_from_directives(&directives, &file_source());
        assert!(results.is_empty());
    }

    #[test]
    fn discover_from_expression_frontmatter() {
        let content = "Some text\n{{ frontmatter(https://example.com/api.md) }}\nMore text";
        let results = discover_remote_urls_from_expressions(content, &file_source());
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].consumer, RemoteUrlConsumer::ExpressionFunction);
        assert_eq!(results[0].url.as_str(), "https://example.com/api.md");
    }

    #[test]
    fn discover_from_expression_quoted_url() {
        // The interpolation expression parser requires the quoted form, so
        // discovery must register it too (the bare form stays best-effort).
        let content = "{{ frontmatter(\"https://example.com/api.md\", \"status\") }}";
        let results = discover_remote_urls_from_expressions(content, &file_source());
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].consumer, RemoteUrlConsumer::ExpressionFunction);
        assert_eq!(results[0].url.as_str(), "https://example.com/api.md");
    }

    #[test]
    fn discover_from_expression_file_exists() {
        let content = "Check: {{ file_exists(https://example.com/remote.md) }}";
        let results = discover_remote_urls_from_expressions(content, &unknown_source());
        assert_eq!(results.len(), 1);
        assert!(results[0].source_file.is_none());
    }

    #[test]
    fn discover_from_expression_markdown_title() {
        let content = "Title: {{ markdown_title(https://example.com/page.md) }}";
        let results = discover_remote_urls_from_expressions(content, &file_source());
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn discover_multiple_expression_functions() {
        let content = "\
{{ frontmatter(https://a.com/f.md) }}
{{ file_exists(https://b.com/f.md) }}
{{ markdown_title(https://c.com/f.md) }}";
        let results = discover_remote_urls_from_expressions(content, &file_source());
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn skip_expression_local_paths() {
        let content = "{{ frontmatter(./local.md) }}";
        let results = discover_remote_urls_from_expressions(content, &file_source());
        assert!(results.is_empty());
    }

    #[test]
    fn catalog_deduplication() {
        let mut catalog = RemoteUrlCatalog::new();
        let url = url::Url::parse("https://example.com/doc.md").unwrap();

        catalog.add(DiscoveredRemoteUrl {
            url: url.clone(),
            normalized_key: url.to_string(),
            consumer: RemoteUrlConsumer::TransclusionFile,
            source_file: Some(PathBuf::from("/a.md")),
            line: 1,
        });
        catalog.add(DiscoveredRemoteUrl {
            url: url.clone(),
            normalized_key: url.to_string(),
            consumer: RemoteUrlConsumer::ExpressionFunction,
            source_file: Some(PathBuf::from("/a.md")),
            line: 5,
        });

        assert_eq!(catalog.len(), 2);
        assert_eq!(catalog.unique_count(), 1);
        assert_eq!(catalog.urls().len(), 1);
    }

    #[test]
    fn catalog_empty() {
        let catalog = RemoteUrlCatalog::new();
        assert!(catalog.is_empty());
        assert_eq!(catalog.len(), 0);
        assert_eq!(catalog.unique_count(), 0);
    }

    #[test]
    fn catalog_non_empty() {
        let mut catalog = RemoteUrlCatalog::new();
        let url = url::Url::parse("https://example.com/doc.md").unwrap();
        catalog.add(DiscoveredRemoteUrl {
            url,
            normalized_key: "https://example.com/doc.md".to_string(),
            consumer: RemoteUrlConsumer::TransclusionFile,
            source_file: None,
            line: 1,
        });
        assert!(!catalog.is_empty());
        assert_eq!(catalog.len(), 1);
        assert_eq!(catalog.unique_count(), 1);
    }

    #[test]
    fn default_config_denies_all() {
        let config = RemoteReadConfig::default();
        assert!(!config.is_host_allowed("example.com"));
        assert!(!config.is_host_allowed("localhost"));
        assert!(!config.is_host_allowed(""));
        assert_eq!(config.remote_concurrency, 4);
        assert!(config.remote_ttl.is_none());
        assert!(!config.refresh);
        assert_eq!(config.freshness_mode, RemoteFreshnessMode::Strict);
    }

    #[test]
    fn host_allowlist_case_insensitive() {
        let config = RemoteReadConfig {
            allowed_hosts: vec!["Example.COM".to_string()],
            ..Default::default()
        };
        assert!(config.is_host_allowed("example.com"));
        assert!(config.is_host_allowed("EXAMPLE.COM"));
        assert!(config.is_host_allowed("Example.Com"));
        assert!(!config.is_host_allowed("other.com"));
    }

    #[test]
    fn validate_url_valid_https() {
        let url = validate_url_for_remote_read("https://example.com/path").unwrap();
        assert_eq!(url.scheme(), "https");
        assert_eq!(url.host_str(), Some("example.com"));
    }

    #[test]
    fn validate_url_valid_http() {
        let url = validate_url_for_remote_read("http://example.com/path").unwrap();
        assert_eq!(url.scheme(), "http");
    }

    #[test]
    fn validate_url_rejects_ftp() {
        let err = validate_url_for_remote_read("ftp://example.com/file").unwrap_err();
        assert!(matches!(err, RemoteReadError::InvalidUrl { .. }));
        assert!(err.to_string().contains("unsupported scheme"));
    }

    #[test]
    fn validate_url_rejects_garbage() {
        let err = validate_url_for_remote_read("not-a-url").unwrap_err();
        assert!(matches!(err, RemoteReadError::InvalidUrl { .. }));
    }

    #[test]
    fn validate_url_rejects_no_host() {
        let err = validate_url_for_remote_read("http://").unwrap_err();
        assert!(matches!(err, RemoteReadError::InvalidUrl { .. }));
    }

    #[test]
    fn check_allowed_passes() {
        let config = RemoteReadConfig {
            allowed_hosts: vec!["example.com".to_string()],
            ..Default::default()
        };
        let url = url::Url::parse("https://example.com/doc.md").unwrap();
        assert!(check_remote_read_allowed(&config, &url).is_ok());
    }

    #[test]
    fn check_denied_by_policy() {
        let config = RemoteReadConfig::default();
        let url = url::Url::parse("https://example.com/doc.md").unwrap();
        let err = check_remote_read_allowed(&config, &url).unwrap_err();
        assert!(matches!(err, RemoteReadError::DeniedByPolicy { .. }));
        assert!(err.to_string().contains("example.com"));
    }

    #[test]
    fn discover_all_combines_directives_and_expressions() {
        let directives = vec![
            make_directive(DirectiveKind::File, "https://example.com/a.md", 1),
        ];
        let content = "{{ frontmatter(https://example.com/b.md) }}";
        let catalog = discover_all_remote_urls(&directives, content, &file_source());

        assert_eq!(catalog.len(), 2);
        assert_eq!(catalog.unique_count(), 2);
    }

    #[test]
    fn discover_all_deduplicates_across_sources() {
        let directives = vec![
            make_directive(DirectiveKind::File, "https://example.com/shared.md", 1),
        ];
        let content = "{{ frontmatter(https://example.com/shared.md) }}";
        let catalog = discover_all_remote_urls(&directives, content, &file_source());

        assert_eq!(catalog.len(), 2);
        assert_eq!(catalog.unique_count(), 1);
    }
}
