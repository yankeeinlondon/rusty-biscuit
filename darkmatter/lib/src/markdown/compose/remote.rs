//! URL discovery and remote-read configuration for transclusion targets.
//!
//! This module provides the types and discovery functions needed to find
//! remote URL dependencies in a markdown document before any network access
//! occurs. It performs **no I/O** — only cataloging and validation.

use std::collections::HashSet;
use std::path::PathBuf;

use biscuit_file::{FileReference, FileReferenceKind};
use thiserror::Error;

use crate::markdown::compose::ComposeSource;
use crate::markdown::compose::expression::{Expr, ExpressionFinder, parse};
use crate::markdown::compose::transclusion::BlockDirective;

/// Expression-function identifiers whose first argument is a file/URL path the
/// evaluator will actually **fetch over the network** when it is an HTTP(S) URL.
///
/// This is deliberately narrower than the context-requiring read-side set
/// (the context-aware expression registrations): `absolute` and
/// `relative` also require a resolution context but only ever rewrite a path
/// string — they never touch the network — so registering them as remote
/// egress would make the pre-fetch discovery scanner contact a host for a URL
/// that is never actually read. Membership is an **exact** match — a longer
/// identifier such as `not_frontmatter` is deliberately excluded so it never
/// registers network egress.
const REMOTE_READ_FUNCTIONS: &[&str] = &[
    "frontmatter",
    "file_exists",
    "markdown_title",
    "markdown_body_empty",
    "validate_schema",
];

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

/// Environment variable overriding the default remote-fetch concurrency cap.
pub const REMOTE_CONCURRENCY_ENV: &str = "DARKMATTER_REMOTE_CONCURRENCY";

/// Default remote-fetch concurrency cap. Matches the value documented in the
/// spec's eager-prefetch section.
pub const DEFAULT_REMOTE_CONCURRENCY: usize = 16;

/// Resolves the remote-fetch concurrency cap with precedence:
/// explicit `override_value` (e.g. a CLI flag) > the
/// [`REMOTE_CONCURRENCY_ENV`] environment variable > [`DEFAULT_REMOTE_CONCURRENCY`].
///
/// A programmatic [`RemoteReadConfig`] value sits above all of these: callers
/// that build the config directly (the `ComposeOptions` path) never go through
/// this resolver. Values of `0` from any source are promoted to `1` so the
/// fetch runtime always makes forward progress.
pub fn resolve_remote_concurrency(override_value: Option<usize>) -> usize {
    resolve_remote_concurrency_from(override_value, std::env::var(REMOTE_CONCURRENCY_ENV).ok())
}

/// Pure core of [`resolve_remote_concurrency`], split out so precedence can be
/// tested without mutating the process environment.
fn resolve_remote_concurrency_from(
    override_value: Option<usize>,
    env_value: Option<String>,
) -> usize {
    if let Some(n) = override_value {
        return n.max(1);
    }
    if let Some(n) = env_value
        .as_deref()
        .map(str::trim)
        .and_then(|s| s.parse::<usize>().ok())
        .filter(|n| *n > 0)
    {
        return n;
    }
    DEFAULT_REMOTE_CONCURRENCY
}

/// How to handle cache staleness for remote artifacts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RemoteFreshnessMode {
    /// Serve any cached artifact without revalidation, even when stale (TTL is
    /// ignored).
    Optimistic,
    /// Always revalidate with conditional GET.
    Strict,
    /// Serve stale on network failure. The default: the spec's
    /// "TTL → conditional → stale-on-failure" behavior keeps CI/offline builds
    /// resilient when a remote dependency is briefly unreachable.
    #[default]
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
            remote_concurrency: DEFAULT_REMOTE_CONCURRENCY,
            remote_ttl: None,
            refresh: false,
            freshness_mode: RemoteFreshnessMode::Fallback,
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
/// Uses [`FileReference`] classification to find URL targets and maps them to
/// the appropriate [`RemoteUrlConsumer`] based on directive kind.
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
        .filter_map(|d| {
            let file_ref = FileReference::new(&d.raw_target).ok()?;
            if file_ref.class().kind != FileReferenceKind::Url {
                return None;
            }
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

/// Discovers URL arguments to read-side expression functions
/// (`frontmatter("https://…")`, `file_exists("https://…")`, …).
///
/// Discovery is driven by the same parser the evaluator uses: only genuine
/// `{{ … }}` expressions are considered (fenced/indented code blocks are
/// skipped), each is parsed to an AST, and a URL is registered only for an
/// exact-identifier [`REMOTE_READ_FUNCTIONS`] call whose first argument is a
/// string literal that validates as an `http(s)` URL. Prose, inline code, and
/// longer identifiers that merely *contain* a function name therefore never
/// trigger network egress — an allowlisted host is contacted only for inputs
/// the evaluator would actually read.
pub fn discover_remote_urls_from_expressions(
    content: &str,
    source: &ComposeSource,
) -> Vec<DiscoveredRemoteUrl> {
    // A remote-read URL argument is a string literal that validates as an
    // `http(s)` URL, so it necessarily contains the substring "http". When the
    // document has none, no expression can register a URL — skip the full
    // expression scan/parse the interpolation stage repeats anyway (F33).
    // Byte-identical: the scan would produce the same empty result.
    if !content.contains("http") {
        return Vec::new();
    }

    let locations = ExpressionFinder::new(content).find_all();
    if locations.is_empty() {
        return Vec::new();
    }

    let source_file = match source {
        ComposeSource::File(p) => Some(p.clone()),
        _ => None,
    };

    // Built once for the whole document rather than rescanning `content[..start]`
    // per expression, which was quadratic in document size (F33).
    let newline_offsets = newline_offset_table(content);
    let mut results = Vec::new();

    for loc in locations {
        let Ok(expr) = parse(&loc.expression) else {
            continue;
        };
        let line = line_at_offset(&newline_offsets, loc.start);
        collect_expression_urls(&expr, &source_file, line, &mut results);
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

/// Recursively walks a parsed expression, registering a remote URL for every
/// exact-identifier [`REMOTE_READ_FUNCTIONS`] call whose first argument is a
/// string-literal `http(s)` URL. Recurses into all sub-expressions so calls
/// nested inside ternaries, comparisons, fallbacks, or other function
/// arguments are still discovered.
fn collect_expression_urls(
    expr: &Expr,
    source_file: &Option<PathBuf>,
    line: usize,
    results: &mut Vec<DiscoveredRemoteUrl>,
) {
    if let Expr::FunctionCall { name, args } = expr
        && REMOTE_READ_FUNCTIONS.contains(&name.as_str())
        && let Some(Expr::StringLiteral(raw)) = args.first()
        && let Ok(url) = validate_url_for_remote_read(raw)
    {
        results.push(DiscoveredRemoteUrl {
            normalized_key: url.to_string(),
            url,
            consumer: RemoteUrlConsumer::ExpressionFunction,
            source_file: source_file.clone(),
            line,
        });
    }

    for child in expr_children(expr) {
        collect_expression_urls(child, source_file, line, results);
    }
}

/// Returns the immediate sub-expressions of `expr` for recursive traversal.
fn expr_children(expr: &Expr) -> Vec<&Expr> {
    match expr {
        Expr::Variable(_)
        | Expr::StringLiteral(_)
        | Expr::NumberLiteral(_)
        | Expr::BoolLiteral(_) => Vec::new(),
        Expr::UnaryNot(e) | Expr::UnaryMinus(e) | Expr::Paren(e) => vec![e.as_ref()],
        Expr::Binary { left, right, .. } => vec![left.as_ref(), right.as_ref()],
        Expr::Index { base, index } => vec![base.as_ref(), index.as_ref()],
        Expr::MemberAccess { base, .. } => vec![base.as_ref()],
        Expr::Fallback { primary, fallback } => vec![primary.as_ref(), fallback.as_ref()],
        Expr::Ternary {
            condition,
            then_branch,
            else_branch,
        } => vec![condition.as_ref(), then_branch.as_ref(), else_branch.as_ref()],
        Expr::Comparison { left, right, .. } => vec![left.as_ref(), right.as_ref()],
        Expr::FunctionCall { args, .. } => args.iter().collect(),
        Expr::ArrayLiteral(items) => items.iter().collect(),
        Expr::ObjectLiteral(entries) => entries.iter().map(|(_, value)| value).collect(),
    }
}

/// Byte offsets of every `\n` in `content`, in ascending order.
fn newline_offset_table(content: &str) -> Vec<usize> {
    content
        .bytes()
        .enumerate()
        .filter_map(|(i, byte)| (byte == b'\n').then_some(i))
        .collect()
}

/// Returns the 1-based line containing `offset`, using the precomputed
/// [`newline_offset_table`].
///
/// Byte-identical to counting the `\n` in `content[..offset]` and adding one,
/// including for an `offset` past the end of `content` (every newline precedes
/// it, so it lands on the final line — matching the clamping scan this
/// replaced). CRLF needs no special case: only `\n` is counted, so `\r\n` is
/// one break on every platform.
///
/// Note this is deliberately *not* the TOC module's same-named lookup, which
/// adds a trailing-newline adjustment to match `str::lines` counting semantics.
/// Remote discovery reports the line an expression's `{{` sits on, so a `{{` at
/// a line's first byte belongs to that line, not the previous one.
fn line_at_offset(newline_offsets: &[usize], offset: usize) -> usize {
    newline_offsets.partition_point(|&pos| pos < offset) + 1
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
    fn discovers_file_and_code_targets_with_case_insensitive_http_schemes() {
        let directives = vec![
            make_directive(DirectiveKind::File, "hTtP://example.com/doc.md", 5),
            make_directive(DirectiveKind::Code, "HTTPS://example.com/snippet.rs", 10),
        ];

        let results = discover_remote_urls_from_directives(&directives, &file_source());

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].url.as_str(), "http://example.com/doc.md");
        assert_eq!(results[0].consumer, RemoteUrlConsumer::TransclusionFile);
        assert_eq!(results[1].url.as_str(), "https://example.com/snippet.rs");
        assert_eq!(results[1].consumer, RemoteUrlConsumer::TransclusionCode);
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
        let content =
            "Some text\n{{ frontmatter(\"https://example.com/api.md\") }}\nMore text";
        let results = discover_remote_urls_from_expressions(content, &file_source());
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].consumer, RemoteUrlConsumer::ExpressionFunction);
        assert_eq!(results[0].url.as_str(), "https://example.com/api.md");
        // Line of the `{{` (1-based): the expression is on the second line.
        assert_eq!(results[0].line, 2);
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
    fn discover_from_expression_literal_children() {
        let content =
            "{{ length({ source: [frontmatter(\"https://example.com/api.md\")] }) }}";
        let results = discover_remote_urls_from_expressions(content, &file_source());
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].url.as_str(), "https://example.com/api.md");
    }

    #[test]
    fn discover_from_expression_file_exists() {
        let content = "Check: {{ file_exists(\"https://example.com/remote.md\") }}";
        let results = discover_remote_urls_from_expressions(content, &unknown_source());
        assert_eq!(results.len(), 1);
        assert!(results[0].source_file.is_none());
    }

    #[test]
    fn discover_from_expression_markdown_title() {
        let content = "Title: {{ markdown_title(\"https://example.com/page.md\") }}";
        let results = discover_remote_urls_from_expressions(content, &file_source());
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn discover_multiple_expression_functions() {
        let content = "\
{{ frontmatter(\"https://a.com/f.md\") }}
{{ file_exists(\"https://b.com/f.md\") }}
{{ markdown_title(\"https://c.com/f.md\") }}";
        let results = discover_remote_urls_from_expressions(content, &file_source());
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn skip_expression_local_paths() {
        let content = "{{ frontmatter(\"./local.md\") }}";
        let results = discover_remote_urls_from_expressions(content, &file_source());
        assert!(results.is_empty());
    }

    #[test]
    fn discover_from_nested_expression_function() {
        // A read-side call buried in a ternary branch is still discovered.
        let content =
            "{{ flag ? frontmatter(\"https://example.com/api.md\") : 'none' }}";
        let results = discover_remote_urls_from_expressions(content, &file_source());
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].url.as_str(), "https://example.com/api.md");
    }

    #[test]
    fn no_fetch_for_url_inside_interpolation_literal() {
        // `{{{ ... }}}` content is inert: a URL inside a literal must not be
        // discovered as remote egress.
        let content = "See {{{ frontmatter(\"https://example.com/doc.md\") }}} for details.";
        let results = discover_remote_urls_from_expressions(content, &file_source());
        assert!(results.is_empty(), "URL inside interpolation literal must not be discovered");
    }

    #[test]
    fn no_fetch_for_prose_function_text() {
        // Plain prose that merely contains a function-call-shaped substring is
        // not an expression and must never be fetched.
        let content = "See frontmatter(\"https://example.com/doc.md\") for details.";
        let results = discover_remote_urls_from_expressions(content, &file_source());
        assert!(results.is_empty());
    }

    #[test]
    fn no_fetch_for_fenced_code_block() {
        // Fenced code blocks are excluded from expression scanning entirely.
        let content = "\
Intro

```text
{{ frontmatter(\"https://example.com/doc.md\") }}
```
";
        let results = discover_remote_urls_from_expressions(content, &file_source());
        assert!(results.is_empty());
    }

    #[test]
    fn no_fetch_for_inline_code_non_expression() {
        // Inline code containing a function-call string but no `{{ }}` is prose,
        // not an expression — the evaluator never runs it, so it is not fetched.
        let content = "Try `frontmatter(\"https://example.com/doc.md\")` here.";
        let results = discover_remote_urls_from_expressions(content, &file_source());
        assert!(results.is_empty());
    }

    #[test]
    fn absolute_url_arg_is_not_remote_egress() {
        // `absolute`/`relative` require a resolution context but never fetch:
        // a URL argument must not be registered as network egress.
        let content = "{{ absolute(\"https://example.com/doc.md\") }}";
        let results = discover_remote_urls_from_expressions(content, &file_source());
        assert!(results.is_empty(), "absolute() must not register remote egress");
    }

    #[test]
    fn relative_url_arg_is_not_remote_egress() {
        let content = "{{ relative(\"https://example.com/doc.md\") }}";
        let results = discover_remote_urls_from_expressions(content, &file_source());
        assert!(results.is_empty(), "relative() must not register remote egress");
    }

    #[test]
    fn no_fetch_for_longer_identifier() {
        // A longer identifier that ends with a supported function name must not
        // match — identifier comparison is exact.
        let content = "{{ not_frontmatter(\"https://example.com/doc.md\") }}";
        let results = discover_remote_urls_from_expressions(content, &file_source());
        assert!(results.is_empty());
    }

    // --- F33: line positions from the newline-offset table --------------------
    //
    // The offset table replaced a per-expression `content[..start]` prefix
    // rescan. These pin the reported lines the scan produced, so a regression
    // in the table/binary-search path fails here rather than silently
    // mislabeling a remote-fetch diagnostic.

    /// The formula the offset-table path must reproduce exactly: count the
    /// `\n` before `offset` and add one.
    fn naive_line(content: &str, offset: usize) -> usize {
        content[..offset.min(content.len())]
            .bytes()
            .filter(|&b| b == b'\n')
            .count()
            + 1
    }

    fn url_fn(n: usize) -> String {
        format!("{{{{ frontmatter(\"https://example.com/{n}.md\") }}}}")
    }

    #[test]
    fn expression_lines_with_lf_newlines() {
        let content = format!("intro\n{}\nmiddle prose\n\n{}\n", url_fn(1), url_fn(2));
        let results = discover_remote_urls_from_expressions(&content, &file_source());
        assert_eq!(
            results.iter().map(|r| r.line).collect::<Vec<_>>(),
            vec![2, 5]
        );
    }

    #[test]
    fn expression_lines_with_crlf_newlines() {
        // Only `\n` delimits a line, so a `\r\n` document reports the same
        // lines its LF twin does.
        let content = format!(
            "intro\r\n{}\r\nmiddle prose\r\n\r\n{}\r\n",
            url_fn(1),
            url_fn(2)
        );
        let results = discover_remote_urls_from_expressions(&content, &file_source());
        assert_eq!(
            results.iter().map(|r| r.line).collect::<Vec<_>>(),
            vec![2, 5]
        );
    }

    #[test]
    fn expression_line_after_multibyte_unicode() {
        // A wide multi-byte prefix (630 bytes / 210 chars) puts the expression's
        // byte offset far beyond the *character* offset of every newline that
        // follows it. A table keyed on char indices would therefore count the
        // trailing lines as preceding the expression and over-report the line;
        // only a byte-offset table reports line 2.
        let wide = "日本語テキスト".repeat(30);
        let content = format!("{wide}\n{}\ntail one\ntail two\ntail three\n", url_fn(1));
        let results = discover_remote_urls_from_expressions(&content, &file_source());
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].line, 2);
    }

    #[test]
    fn expression_line_between_multibyte_lines() {
        // Multi-byte text on both sides, expression mid-document: byte offsets
        // and character offsets diverge before *and* after it.
        let content = format!(
            "café ☕\n🎉 naïve 🎉\n{}\n日本語\n{}\n",
            url_fn(1),
            url_fn(2)
        );
        let results = discover_remote_urls_from_expressions(&content, &file_source());
        assert_eq!(
            results.iter().map(|r| r.line).collect::<Vec<_>>(),
            vec![3, 5]
        );
    }

    #[test]
    fn expression_at_start_of_file_is_line_one() {
        // Offset 0: no newline precedes it, and the `partition_point` must not
        // underflow off the front of the table.
        let content = format!("{}\ntrailing prose\n", url_fn(1));
        let results = discover_remote_urls_from_expressions(&content, &file_source());
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].line, 1);
    }

    #[test]
    fn expression_at_end_of_file_with_and_without_trailing_newline() {
        // The final line is reported the same whether or not the document ends
        // in a newline — the terminator follows the expression either way.
        let without = format!("a\nb\n{}", url_fn(1));
        let with = format!("a\nb\n{}\n", url_fn(1));

        for content in [without, with] {
            let results = discover_remote_urls_from_expressions(&content, &file_source());
            assert_eq!(results.len(), 1);
            assert_eq!(results[0].line, 3, "content: {content:?}");
        }
    }

    #[test]
    fn multiple_expressions_on_one_line_share_a_line_number() {
        // Distinct byte offsets between the same pair of newlines must all map
        // to that one line.
        let content = format!("intro\n{} and {} and {}\n", url_fn(1), url_fn(2), url_fn(3));
        let results = discover_remote_urls_from_expressions(&content, &file_source());
        assert_eq!(results.len(), 3);
        assert!(results.iter().all(|r| r.line == 2), "{results:#?}");
        // Each URL is still distinct — the shared line does not collapse them.
        assert_eq!(
            results.iter().map(|r| r.url.as_str()).collect::<Vec<_>>(),
            vec![
                "https://example.com/1.md",
                "https://example.com/2.md",
                "https://example.com/3.md",
            ]
        );
    }

    #[test]
    fn consecutive_blank_lines_do_not_skew_line_numbers() {
        // Adjacent newlines put several table entries at consecutive offsets;
        // the binary search must still count every one of them.
        let content = format!("intro\n\n\n\n{}\n", url_fn(1));
        let results = discover_remote_urls_from_expressions(&content, &file_source());
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].line, 5);
    }

    #[test]
    fn line_at_offset_matches_naive_at_every_offset() {
        // Exhaustive equivalence over a mixed LF/CRLF/Unicode document: the
        // table must agree with the replaced prefix scan at every char
        // boundary, plus offsets past the end (which the old scan clamped).
        let content = "first\r\nsecond café\n\n\n日本語 line\nlast no newline";
        let table = newline_offset_table(content);

        for offset in 0..=content.len() {
            if !content.is_char_boundary(offset) {
                continue;
            }
            assert_eq!(
                line_at_offset(&table, offset),
                naive_line(content, offset),
                "offset {offset}"
            );
        }

        for past_end in [content.len() + 1, content.len() + 100] {
            assert_eq!(
                line_at_offset(&table, past_end),
                naive_line(content, past_end),
                "past-end offset {past_end}"
            );
        }
    }

    #[test]
    fn empty_newline_table_reports_line_one() {
        // A single-line document has no table entries at all.
        let content = "no newlines here";
        let table = newline_offset_table(content);
        assert!(table.is_empty());
        assert_eq!(line_at_offset(&table, 0), 1);
        assert_eq!(line_at_offset(&table, content.len()), 1);
    }

    #[test]
    fn no_http_guard_short_circuits_before_expression_scan() {
        // The cheap guard is retained: a document with expressions but no
        // `http` substring can never register a URL.
        let content = "{{ frontmatter(\"./local.md\") }}\n{{ file_exists(\"./b.md\") }}";
        assert!(!content.contains("http"));
        assert!(discover_remote_urls_from_expressions(content, &file_source()).is_empty());
    }

    #[test]
    fn http_prose_without_any_expression_discovers_nothing() {
        // Passes the `http` guard but has no `{{ }}` at all — the early return
        // before the table build must not change the result.
        let content = "See https://example.com/doc.md for details.\nNo expressions here.\n";
        assert!(discover_remote_urls_from_expressions(content, &file_source()).is_empty());
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
        assert_eq!(config.remote_concurrency, DEFAULT_REMOTE_CONCURRENCY);
        assert!(config.remote_ttl.is_none());
        assert!(!config.refresh);
        assert_eq!(config.freshness_mode, RemoteFreshnessMode::Fallback);
    }

    #[test]
    fn concurrency_default_when_no_override_or_env() {
        // Lowest precedence: no CLI override and no env var → the spec default.
        assert_eq!(resolve_remote_concurrency_from(None, None), DEFAULT_REMOTE_CONCURRENCY);
    }

    #[test]
    fn concurrency_env_overrides_default() {
        assert_eq!(resolve_remote_concurrency_from(None, Some("8".to_string())), 8);
    }

    #[test]
    fn concurrency_cli_override_beats_env() {
        // An explicit CLI flag wins over the env var.
        assert_eq!(resolve_remote_concurrency_from(Some(3), Some("8".to_string())), 3);
    }

    #[test]
    fn concurrency_invalid_or_zero_env_falls_back_to_default() {
        // Garbage, empty, and zero env values are ignored rather than silently
        // deadlocking or weakening the cap.
        assert_eq!(resolve_remote_concurrency_from(None, Some("nope".to_string())), DEFAULT_REMOTE_CONCURRENCY);
        assert_eq!(resolve_remote_concurrency_from(None, Some("".to_string())), DEFAULT_REMOTE_CONCURRENCY);
        assert_eq!(resolve_remote_concurrency_from(None, Some("0".to_string())), DEFAULT_REMOTE_CONCURRENCY);
    }

    #[test]
    fn concurrency_zero_cli_override_is_promoted_to_one() {
        // A `0` CLI flag would deadlock every fetch; clamp it to one.
        assert_eq!(resolve_remote_concurrency_from(Some(0), None), 1);
    }

    #[test]
    fn concurrency_compose_options_value_is_authoritative() {
        // Highest precedence: a programmatically constructed config carries its
        // own value, never consulting the resolver/env at all.
        let config = RemoteReadConfig {
            remote_concurrency: 32,
            ..Default::default()
        };
        assert_eq!(config.remote_concurrency, 32);
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
        let content = "{{ frontmatter(\"https://example.com/b.md\") }}";
        let catalog = discover_all_remote_urls(&directives, content, &file_source());

        assert_eq!(catalog.len(), 2);
        assert_eq!(catalog.unique_count(), 2);
    }

    #[test]
    fn discover_all_deduplicates_across_sources() {
        let directives = vec![
            make_directive(DirectiveKind::File, "https://example.com/shared.md", 1),
        ];
        let content = "{{ frontmatter(\"https://example.com/shared.md\") }}";
        let catalog = discover_all_remote_urls(&directives, content, &file_source());

        assert_eq!(catalog.len(), 2);
        assert_eq!(catalog.unique_count(), 1);
    }
}
