//! Reference validation engine.
//!
//! Validates references found in markdown documents:
//! - Local path existence
//! - URL syntax
//! - Remote URL reachability (opt-in)
//! - Fragment target resolution (opt-in)

use std::time::Duration;

use super::errors::ReferenceError;
use super::types::{
    ReferenceGraphOptions, ReferenceKind, ReferenceOrigin, ReferenceRecord, ReferenceTarget,
};
use crate::markdown::Markdown;
use crate::markdown::compose::ComposeSource;

/// Options for reference validation.
#[derive(Clone)]
pub struct ReferenceValidationOptions {
    /// Graph options for reference extraction.
    pub graph: ReferenceGraphOptions,
    /// Whether to validate remote URLs via HTTP.
    pub validate_remote: bool,
    /// Timeout for remote URL validation.
    pub remote_timeout: Duration,
    /// Whether to validate fragment targets (`#section`).
    pub validate_fragments: bool,
    /// Stop on first error-severity issue.
    pub fail_fast: bool,
}

impl Default for ReferenceValidationOptions {
    fn default() -> Self {
        Self {
            graph: ReferenceGraphOptions::default(),
            validate_remote: false,
            remote_timeout: Duration::from_secs(10),
            validate_fragments: false,
            fail_fast: false,
        }
    }
}

/// Report from reference validation.
#[derive(Debug, Clone, Default)]
pub struct ReferenceValidationReport {
    /// Total references scanned.
    pub references_scanned: usize,
    /// References that passed validation.
    pub references_valid: usize,
    /// Issues found.
    pub issues: Vec<ReferenceIssue>,
    /// Non-blocking warnings.
    pub warnings: Vec<String>,
}

impl ReferenceValidationReport {
    /// Returns `true` if no error-severity issues were found.
    pub fn is_valid(&self) -> bool {
        !self
            .issues
            .iter()
            .any(|i| i.severity == ReferenceSeverity::Error)
    }

    /// Returns the count of error-severity issues.
    pub fn error_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| i.severity == ReferenceSeverity::Error)
            .count()
    }
}

/// A single validation issue.
#[derive(Debug, Clone)]
pub struct ReferenceIssue {
    /// Issue classification code.
    pub code: ReferenceIssueCode,
    /// Human-readable message.
    pub message: String,
    /// Severity level.
    pub severity: ReferenceSeverity,
    /// Semantic kind of the reference (hyperlink, image, transclusion, etc.).
    pub kind: ReferenceKind,
    /// The raw reference target string for display (e.g., `"./foo.md"`, `"https://example.com"`).
    pub reference_display: String,
    /// ID of the reference that triggered this issue.
    pub reference_id: String,
    /// Where the reference was found.
    pub origin: ReferenceOrigin,
}

/// Classification codes for validation issues.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceIssueCode {
    /// Local file target does not exist.
    MissingLocalTarget,
    /// URL is syntactically invalid.
    InvalidUrl,
    /// Remote URL returned an error or timed out.
    RemoteUnreachable,
    /// Remote validation was not performed (disabled).
    ///
    /// Emitted when `validate_remote` is false and a remote reference
    /// cannot be verified.
    RemoteDisallowed,
    /// Source context needed but not available.
    MissingSourceContext,
    /// Non-HTTP/HTTPS scheme.
    UnsupportedScheme,
    /// Fragment target not found in document headings.
    MissingFragmentTarget,
    /// HTML tag is malformed (e.g., missing required attributes).
    ///
    /// Reserved for future use with structured HTML validation.
    MalformedHtmlTag,
    /// CSS `@import` is malformed.
    ///
    /// Reserved for future use when CSS parsing is upgraded to `cssparser`.
    MalformedCssImport,
    /// Meta tag is malformed.
    ///
    /// Reserved for future use with structured meta tag validation.
    MalformedMetaTag,
}

/// Severity levels for validation issues.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceSeverity {
    /// Must be fixed.
    Error,
    /// Should be addressed.
    Warning,
    /// Informational.
    Info,
}

/// Run validation on a markdown document's references.
pub(crate) fn validate(
    md: &Markdown,
    options: &ReferenceValidationOptions,
) -> Result<ReferenceValidationReport, ReferenceError> {
    let ref_set = {
        let graph = super::graph::build_reference_graph(md, &options.graph)
            .map_err(|e| ReferenceError::Validation(e.to_string()))?;
        super::graph::flatten_graph(&graph)
    };

    let mut report = ReferenceValidationReport {
        references_scanned: ref_set.len(),
        ..Default::default()
    };

    // Collect headings for fragment validation from the composed document.
    // Uses the graph's prepared content which includes transcluded headings.
    let headings = if options.validate_fragments {
        Some(collect_composed_heading_slugs(md, &options.graph))
    } else {
        None
    };

    for record in &ref_set.records {
        // Use the reference's own origin source for validation (rec #5),
        // not the root document source. This ensures child references
        // are validated relative to their own file location.
        let ref_source = &record.origin.source;

        match &record.target {
            ReferenceTarget::LocalPath { raw } => {
                // Check for fragment in local path (e.g., "./other.md#section")
                let (path_part, fragment) = split_path_fragment(raw);
                validate_local_path(
                    &path_part,
                    ref_source,
                    record,
                    &mut report,
                    &options.graph.compose.magic_paths,
                );

                // Validate fragment if enabled and path exists
                if options.validate_fragments
                    && let Some(ref frag) = fragment
                {
                    validate_cross_doc_fragment(
                        &path_part,
                        frag,
                        ref_source,
                        record,
                        &mut report,
                        &options.graph,
                    );
                }

                if options.fail_fast && report.error_count() > 0 {
                    return Ok(report);
                }
            }
            ReferenceTarget::RemoteUrl { raw } => {
                let normalized = normalize_remote_url(raw, &record.origin.source);
                if let Err(e) = url::Url::parse(&normalized) {
                    report.issues.push(ReferenceIssue {
                        code: ReferenceIssueCode::InvalidUrl,
                        message: format!("Invalid URL: {e}"),
                        severity: ReferenceSeverity::Error,
                        kind: record.kind,
                        reference_display: raw.clone(),
                        reference_id: record.id.clone(),
                        origin: record.origin.clone(),
                    });
                } else if options.validate_remote {
                    // Remote validation will be done in a batch below
                    report.references_valid += 1;
                } else {
                    report.references_valid += 1;
                    report
                        .warnings
                        .push(format!("Remote URL not verified: {raw}"));
                }
                if options.fail_fast && report.error_count() > 0 {
                    return Ok(report);
                }
            }
            ReferenceTarget::Fragment { raw } => {
                if options.validate_fragments {
                    if let Some(ref heading_set) = headings {
                        let slug = &raw[1..]; // Strip leading '#'
                        if !heading_set.contains(&slug.to_lowercase()) {
                            report.issues.push(ReferenceIssue {
                                code: ReferenceIssueCode::MissingFragmentTarget,
                                message: format!("Fragment target not found: {raw}"),
                                severity: ReferenceSeverity::Error,
                                kind: record.kind,
                                reference_display: raw.clone(),
                                reference_id: record.id.clone(),
                                origin: record.origin.clone(),
                            });
                        } else {
                            report.references_valid += 1;
                        }
                    } else {
                        report.references_valid += 1;
                    }
                } else {
                    report.references_valid += 1;
                }
                if options.fail_fast && report.error_count() > 0 {
                    return Ok(report);
                }
            }
            ReferenceTarget::DataUri { .. } => {
                report.references_valid += 1;
            }
            ReferenceTarget::OtherScheme { scheme, raw } => {
                report.issues.push(ReferenceIssue {
                    code: ReferenceIssueCode::UnsupportedScheme,
                    message: format!("Unsupported scheme: {scheme}"),
                    severity: ReferenceSeverity::Info,
                    kind: record.kind,
                    reference_display: raw.clone(),
                    reference_id: record.id.clone(),
                    origin: record.origin.clone(),
                });
                report.references_valid += 1;
            }
            ReferenceTarget::Inline => {
                report.references_valid += 1;
            }
        }
    }

    // Remote URL validation (batch, async)
    if options.validate_remote {
        let remote_refs: Vec<_> = ref_set
            .records
            .iter()
            .filter(|r| matches!(&r.target, ReferenceTarget::RemoteUrl { .. }))
            .collect();

        if !remote_refs.is_empty() {
            let results = validate_remote_urls(&remote_refs, options.remote_timeout);
            for (record, result) in remote_refs.iter().zip(results) {
                match result {
                    RemoteResult::Ok => {}
                    RemoteResult::Error(msg) => {
                        let raw = match &record.target {
                            ReferenceTarget::RemoteUrl { raw } => raw.clone(),
                            _ => String::new(),
                        };
                        // Undo the valid count added above
                        report.references_valid = report.references_valid.saturating_sub(1);
                        report.issues.push(ReferenceIssue {
                            code: ReferenceIssueCode::RemoteUnreachable,
                            message: msg,
                            severity: ReferenceSeverity::Error,
                            kind: record.kind,
                            reference_display: raw,
                            reference_id: record.id.clone(),
                            origin: record.origin.clone(),
                        });
                    }
                }
            }
        }
    }

    Ok(report)
}

/// Validate a local file path reference.
///
/// Uses `biscuit_file::FileReference` for path resolution (rec #6),
/// supporting repo-root `@` paths and consistent semantics with compose.
fn validate_local_path(
    raw: &str,
    source: &ComposeSource,
    record: &ReferenceRecord,
    report: &mut ReferenceValidationReport,
    magic_paths: &[(std::path::PathBuf, biscuit_file::PathPosition)],
) {
    match source {
        ComposeSource::File(base_path) => {
            let base_dir = base_path.parent();

            // Try biscuit_file::FileReference first for @repo-root support
            if let Ok(file_ref) = biscuit_file::FileReference::new(raw) {
                let mut file_ref = file_ref;
                for (path, position) in magic_paths {
                    file_ref = file_ref.add_magic_path(path, *position);
                }
                if let Ok(Some(resolved)) = file_ref.resolve_relative(base_dir) {
                    if resolved.exists() {
                        report.references_valid += 1;
                    } else {
                        report.issues.push(ReferenceIssue {
                            code: ReferenceIssueCode::MissingLocalTarget,
                            message: format!("Missing local target: {raw}"),
                            severity: ReferenceSeverity::Error,
                            kind: record.kind,
                            reference_display: raw.to_string(),
                            reference_id: record.id.clone(),
                            origin: record.origin.clone(),
                        });
                    }
                    return;
                }
            }

            // Fallback to simple path join
            if let Some(base_dir) = base_dir {
                let resolved = base_dir.join(raw);
                if resolved.exists() {
                    report.references_valid += 1;
                } else {
                    report.issues.push(ReferenceIssue {
                        code: ReferenceIssueCode::MissingLocalTarget,
                        message: format!("Missing local target: {raw}"),
                        severity: ReferenceSeverity::Error,
                        kind: record.kind,
                        reference_display: raw.to_string(),
                        reference_id: record.id.clone(),
                        origin: record.origin.clone(),
                    });
                }
            } else {
                report.references_valid += 1;
            }
        }
        ComposeSource::Unknown => {
            report.issues.push(ReferenceIssue {
                code: ReferenceIssueCode::MissingSourceContext,
                message: format!("Cannot validate local path without source context: {raw}"),
                severity: ReferenceSeverity::Warning,
                kind: record.kind,
                reference_display: raw.to_string(),
                reference_id: record.id.clone(),
                origin: record.origin.clone(),
            });
        }
        ComposeSource::Url(_) => {
            report
                .warnings
                .push(format!("Local path in URL-sourced document: {raw}"));
            report.references_valid += 1;
        }
    }
}

// ── Fragment validation ─────────────────────────────────────────────

/// Collects heading slugs from prepared document content.
///
/// Runs InlinePre preparation (interpolation, page blocks, etc.) before
/// extracting headings, so fragment validation matches the actual headings
/// a reader would see after composition.
fn collect_prepared_heading_slugs(
    md: &Markdown,
    source: &ComposeSource,
    graph_options: &super::types::ReferenceGraphOptions,
) -> Vec<String> {
    // Prepare content with InlinePre to resolve interpolation, page blocks, etc.
    let content = match super::graph::prepare_content_for_validation(md, source, graph_options) {
        Ok(c) => c,
        Err(_) => md.content().to_string(),
    };

    // Parse headings from the prepared content
    let prepared_md = Markdown::new(&content);
    let toc = prepared_md.toc();
    toc.all_headings()
        .into_iter()
        .map(|h| h.slug.to_lowercase())
        .collect()
}

/// Collects heading slugs from the composed document (all graph nodes).
///
/// This provides the effective heading set after transclusion, so fragment
/// validation checks against the actual composed heading list. Each node's
/// headings are extracted from prepared content (after InlinePre).
fn collect_composed_heading_slugs(
    md: &Markdown,
    graph_options: &super::types::ReferenceGraphOptions,
) -> Vec<String> {
    let source = md.source().clone().unwrap_or(ComposeSource::Unknown);

    // Build the reference graph to discover all nodes
    let graph = match super::graph::build_reference_graph(md, graph_options) {
        Ok(g) => g,
        Err(_) => return collect_prepared_heading_slugs(md, &source, graph_options),
    };

    let mut all_slugs = Vec::new();

    // Collect headings from the root document (prepared)
    all_slugs.extend(collect_prepared_heading_slugs(md, &source, graph_options));

    // Collect headings from all child nodes (prepared)
    for node in &graph.nodes {
        if let ComposeSource::File(path) = &node.source
            && let Ok(child_md) = Markdown::try_from(path.as_path())
        {
            all_slugs.extend(collect_prepared_heading_slugs(
                &child_md,
                &node.source,
                graph_options,
            ));
        }
    }

    all_slugs
}

/// Validates a fragment reference against a cross-document target.
///
/// Resolves the target path using `biscuit_file::FileReference` (rec #6)
/// relative to the reference's own origin source (rec #5). Validates
/// against prepared headings (after InlinePre) rather than raw headings.
fn validate_cross_doc_fragment(
    path: &str,
    fragment: &str,
    source: &ComposeSource,
    record: &ReferenceRecord,
    report: &mut ReferenceValidationReport,
    graph_options: &super::types::ReferenceGraphOptions,
) {
    let ComposeSource::File(base_path) = source else {
        return;
    };
    let base_dir = base_path.parent();

    // Resolve via FileReference for @repo-root support, fallback to simple join
    let target_path = if let Ok(file_ref) = biscuit_file::FileReference::new(path) {
        let mut file_ref = file_ref;
        for (mp, position) in &graph_options.compose.magic_paths {
            file_ref = file_ref.add_magic_path(mp, *position);
        }
        file_ref.resolve_relative(base_dir).ok().flatten()
    } else {
        None
    }
    .unwrap_or_else(|| {
        base_dir
            .map(|d| d.join(path))
            .unwrap_or_else(|| std::path::PathBuf::from(path))
    });

    if !target_path.exists() {
        return; // Missing file is already reported by validate_local_path
    }

    // Only validate fragments in markdown files
    let ext = target_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    if !["md", "markdown", "mdx"].contains(&ext) {
        return;
    }

    if let Ok(target_md) = Markdown::try_from(target_path.as_path()) {
        let target_source = ComposeSource::File(target_path);
        let headings = collect_prepared_heading_slugs(&target_md, &target_source, graph_options);
        let frag_lower = fragment.to_lowercase();
        if !headings.contains(&frag_lower) {
            report.issues.push(ReferenceIssue {
                code: ReferenceIssueCode::MissingFragmentTarget,
                message: format!("Fragment '#{fragment}' not found in {path}"),
                severity: ReferenceSeverity::Error,
                kind: record.kind,
                reference_display: format!("{path}#{fragment}"),
                reference_id: record.id.clone(),
                origin: record.origin.clone(),
            });
        }
    }
}

/// Splits a path into (path, optional_fragment).
fn split_path_fragment(raw: &str) -> (String, Option<String>) {
    if let Some(idx) = raw.find('#') {
        let path = raw[..idx].to_string();
        let fragment = raw[idx + 1..].to_string();
        if path.is_empty() {
            (raw.to_string(), None) // Pure fragment, not a cross-doc ref
        } else {
            (path, Some(fragment))
        }
    } else {
        (raw.to_string(), None)
    }
}

/// Simple heading slug generation (matches common markdown anchor behavior).
#[cfg(test)]
fn slugify(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else if c == ' ' {
                '-'
            } else {
                ' ' // will be stripped
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("")
}

// ── Remote validation ───────────────────────────────────────────────

enum RemoteResult {
    Ok,
    Error(String),
}

/// Validates remote URLs via HTTP HEAD (falling back to GET on 405).
fn validate_remote_urls(records: &[&ReferenceRecord], timeout: Duration) -> Vec<RemoteResult> {
    use tokio::runtime::Handle;

    // Try to use the existing tokio runtime, or create a temporary one
    if let Ok(handle) = Handle::try_current() {
        // We're inside a tokio runtime, use block_in_place
        tokio::task::block_in_place(|| {
            handle.block_on(validate_remote_urls_async(records, timeout))
        })
    } else {
        // No runtime, create a temporary one
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(validate_remote_urls_async(records, timeout))
    }
}

async fn validate_remote_urls_async(
    records: &[&ReferenceRecord],
    timeout: Duration,
) -> Vec<RemoteResult> {
    let client = reqwest::Client::builder()
        .timeout(timeout)
        .user_agent("darkmatter-reference-validator/0.1")
        .build()
        .unwrap_or_default();

    let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(10));

    let mut handles = Vec::new();
    for record in records {
        let url = match &record.target {
            ReferenceTarget::RemoteUrl { raw } => normalize_remote_url(raw, &record.origin.source),
            _ => continue,
        };

        let client = client.clone();
        let sem = semaphore.clone();

        handles.push(tokio::spawn(async move {
            let _permit = sem.acquire().await.unwrap();
            validate_single_url(&client, &url).await
        }));
    }

    let mut results = Vec::new();
    for handle in handles {
        match handle.await {
            Ok(result) => results.push(result),
            Err(e) => results.push(RemoteResult::Error(format!("Task error: {e}"))),
        }
    }

    results
}

async fn validate_single_url(client: &reqwest::Client, url: &str) -> RemoteResult {
    // Try HEAD first
    match client.head(url).send().await {
        Ok(resp) => {
            let status = resp.status();
            if status.is_success() || status.is_redirection() {
                return RemoteResult::Ok;
            }
            if status.as_u16() == 405 {
                // Method Not Allowed, fall back to GET
                return validate_with_get(client, url).await;
            }
            RemoteResult::Error(format!("HTTP {status}: {url}"))
        }
        Err(e) => {
            if e.is_timeout() {
                RemoteResult::Error(format!("Timeout: {url}"))
            } else {
                // Fall back to GET on any HEAD error
                validate_with_get(client, url).await
            }
        }
    }
}

async fn validate_with_get(client: &reqwest::Client, url: &str) -> RemoteResult {
    match client.get(url).send().await {
        Ok(resp) => {
            let status = resp.status();
            if status.is_success() || status.is_redirection() {
                RemoteResult::Ok
            } else {
                RemoteResult::Error(format!("HTTP {status}: {url}"))
            }
        }
        Err(e) => {
            if e.is_timeout() {
                RemoteResult::Error(format!("Timeout: {url}"))
            } else {
                RemoteResult::Error(format!("Request failed: {e}"))
            }
        }
    }
}

fn normalize_remote_url(raw: &str, source: &ComposeSource) -> String {
    if raw.starts_with("//") {
        let scheme = match source {
            ComposeSource::Url(url) if url.scheme() == "http" || url.scheme() == "https" => {
                url.scheme()
            }
            _ => "https",
        };
        format!("{scheme}:{raw}")
    } else {
        raw.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::Markdown;

    #[test]
    fn validate_valid_url_syntax() {
        let md = Markdown::new("[link](https://example.com)");
        let options = ReferenceValidationOptions::default();
        let report = validate(&md, &options).unwrap();
        assert_eq!(report.references_scanned, 1);
        assert_eq!(report.references_valid, 1);
        assert!(report.is_valid());
    }

    #[test]
    fn validate_protocol_relative_url_syntax() {
        let md = Markdown::new("[cdn](//cdn.example.com/app.js)");
        let options = ReferenceValidationOptions::default();
        let report = validate(&md, &options).unwrap();
        assert_eq!(report.references_scanned, 1);
        assert_eq!(report.references_valid, 1);
        assert!(
            !report
                .issues
                .iter()
                .any(|i| i.code == ReferenceIssueCode::MissingLocalTarget)
        );
    }

    #[test]
    fn validate_fragment_not_validated_by_default() {
        let md = Markdown::new("[link](#section)");
        let options = ReferenceValidationOptions::default();
        let report = validate(&md, &options).unwrap();
        assert_eq!(report.references_valid, 1);
    }

    #[test]
    fn validate_fragment_existing_heading() {
        let md = Markdown::new("# My Heading\n\n[link](#my-heading)");
        let options = ReferenceValidationOptions {
            validate_fragments: true,
            ..Default::default()
        };
        let report = validate(&md, &options).unwrap();
        assert!(report.is_valid());
    }

    #[test]
    fn validate_fragment_missing_heading() {
        let md = Markdown::new("# My Heading\n\n[link](#nonexistent)");
        let options = ReferenceValidationOptions {
            validate_fragments: true,
            ..Default::default()
        };
        let report = validate(&md, &options).unwrap();
        assert!(
            report
                .issues
                .iter()
                .any(|i| i.code == ReferenceIssueCode::MissingFragmentTarget)
        );
    }

    #[test]
    fn validate_missing_source_context_for_local() {
        let md = Markdown::new("[link](./file.md)");
        let options = ReferenceValidationOptions::default();
        let report = validate(&md, &options).unwrap();
        assert!(
            report
                .issues
                .iter()
                .any(|i| i.code == ReferenceIssueCode::MissingSourceContext)
        );
    }

    #[test]
    fn validate_other_scheme_is_info() {
        let md = Markdown::new("[email](mailto:test@example.com)");
        let options = ReferenceValidationOptions::default();
        let report = validate(&md, &options).unwrap();
        let scheme_issues: Vec<_> = report
            .issues
            .iter()
            .filter(|i| i.code == ReferenceIssueCode::UnsupportedScheme)
            .collect();
        assert_eq!(scheme_issues.len(), 1);
        assert_eq!(scheme_issues[0].severity, ReferenceSeverity::Info);
        assert!(report.is_valid());
    }

    #[test]
    fn validate_fail_fast() {
        let md = Markdown::new("[a](./missing1.md)\n[b](./missing2.md)")
            .with_source(ComposeSource::File("/tmp/test.md".into()));

        let options = ReferenceValidationOptions {
            fail_fast: true,
            ..Default::default()
        };
        let report = validate(&md, &options).unwrap();
        assert_eq!(report.error_count(), 1);
    }

    #[test]
    fn report_counts_correct() {
        let md =
            Markdown::new("[a](https://example.com)\n[b](#section)\n[c](data:text/plain,hello)");
        let options = ReferenceValidationOptions::default();
        let report = validate(&md, &options).unwrap();
        assert_eq!(report.references_scanned, 3);
        assert_eq!(report.references_valid, 3);
    }

    #[test]
    fn validate_local_file_exists() {
        let dir = tempfile::TempDir::new().unwrap();
        let target = dir.path().join("exists.md");
        std::fs::write(&target, "# Exists").unwrap();

        let source_path = dir.path().join("source.md");
        std::fs::write(&source_path, "").unwrap();

        let md = Markdown::new("[link](./exists.md)").with_source(ComposeSource::File(source_path));

        let options = ReferenceValidationOptions::default();
        let report = validate(&md, &options).unwrap();
        assert!(report.is_valid());
        assert_eq!(report.references_valid, 1);
    }

    #[test]
    fn validate_local_file_missing() {
        let dir = tempfile::TempDir::new().unwrap();
        let source_path = dir.path().join("source.md");
        std::fs::write(&source_path, "").unwrap();

        let md =
            Markdown::new("[link](./missing.md)").with_source(ComposeSource::File(source_path));

        let options = ReferenceValidationOptions::default();
        let report = validate(&md, &options).unwrap();
        assert!(!report.is_valid());
        assert!(
            report
                .issues
                .iter()
                .any(|i| i.code == ReferenceIssueCode::MissingLocalTarget)
        );
    }

    #[test]
    fn validate_cross_doc_fragment() {
        let dir = tempfile::TempDir::new().unwrap();
        let target = dir.path().join("other.md");
        std::fs::write(&target, "# Getting Started\n\nContent.").unwrap();

        let source_path = dir.path().join("source.md");
        std::fs::write(&source_path, "").unwrap();

        let md = Markdown::new("[link](./other.md#getting-started)")
            .with_source(ComposeSource::File(source_path));

        let options = ReferenceValidationOptions {
            validate_fragments: true,
            ..Default::default()
        };
        let report = validate(&md, &options).unwrap();
        assert!(report.is_valid());
    }

    #[test]
    fn validate_cross_doc_fragment_missing() {
        let dir = tempfile::TempDir::new().unwrap();
        let target = dir.path().join("other.md");
        std::fs::write(&target, "# Getting Started\n\nContent.").unwrap();

        let source_path = dir.path().join("source.md");
        std::fs::write(&source_path, "").unwrap();

        let md = Markdown::new("[link](./other.md#nonexistent)")
            .with_source(ComposeSource::File(source_path));

        let options = ReferenceValidationOptions {
            validate_fragments: true,
            ..Default::default()
        };
        let report = validate(&md, &options).unwrap();
        assert!(
            report
                .issues
                .iter()
                .any(|i| i.code == ReferenceIssueCode::MissingFragmentTarget)
        );
    }

    #[test]
    fn split_path_fragment_works() {
        let (path, frag) = split_path_fragment("./other.md#section");
        assert_eq!(path, "./other.md");
        assert_eq!(frag, Some("section".to_string()));

        let (path, frag) = split_path_fragment("./file.md");
        assert_eq!(path, "./file.md");
        assert_eq!(frag, None);

        let (path, frag) = split_path_fragment("#only-fragment");
        assert_eq!(path, "#only-fragment");
        assert_eq!(frag, None); // Pure fragment, not cross-doc
    }

    #[test]
    fn slugify_works() {
        assert_eq!(slugify("Getting Started"), "getting-started");
        assert_eq!(slugify("My API (v2)"), "my-api-v2");
        assert_eq!(slugify("Hello World!"), "hello-world");
    }
}
