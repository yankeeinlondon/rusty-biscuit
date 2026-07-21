//! Reference validation engine.
//!
//! Validates references found in markdown documents:
//! - Local path existence
//! - URL syntax
//! - Remote URL reachability (opt-in)
//! - Fragment target resolution (opt-in)

use std::time::Duration;

use serde::Serialize;
use tracing::{debug, info, instrument, trace};

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

impl ReferenceValidationOptions {
    /// Creates validation options using the given graph options,
    /// avoiding the cost of constructing a default `ComposeOptions`.
    pub fn with_graph(graph: ReferenceGraphOptions) -> Self {
        Self {
            graph,
            validate_remote: false,
            remote_timeout: Duration::from_secs(10),
            validate_fragments: false,
            fail_fast: false,
        }
    }
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

impl Serialize for ReferenceValidationReport {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("ReferenceValidationReport", 5)?;
        state.serialize_field("valid", &self.is_valid())?;
        state.serialize_field("references_scanned", &self.references_scanned)?;
        state.serialize_field("references_valid", &self.references_valid)?;
        state.serialize_field("issues", &self.issues)?;
        state.serialize_field("warnings", &self.warnings)?;
        state.end()
    }
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

/// A terminal-rendered view of a [`ReferenceValidationReport`].
///
/// Groups error-severity issues by reference kind and renders them as a
/// styled list with a summary line. Empty reports render to an empty string.
#[derive(Debug, Clone)]
pub struct ValidationReportView {
    report: ReferenceValidationReport,
    layout: biscuit_terminal::utils::layout::Layout,
}

impl ValidationReportView {
    /// Creates a view for the given validation report.
    pub fn new(report: ReferenceValidationReport) -> Self {
        Self {
            report,
            layout: biscuit_terminal::utils::layout::Layout::default(),
        }
    }
}

impl biscuit_terminal::components::renderable::TerminalRenderable for ValidationReportView {
    fn layout(&self) -> &biscuit_terminal::utils::layout::Layout {
        &self.layout
    }

    fn layout_mut(&mut self) -> &mut biscuit_terminal::utils::layout::Layout {
        &mut self.layout
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn render(&self, term: &biscuit_terminal::terminal::Terminal) -> String {
        use std::collections::BTreeMap;

        use biscuit_terminal::components::list::UnorderedList;
        use biscuit_terminal::components::prose::Prose;

        let errors: Vec<_> = self
            .report
            .issues
            .iter()
            .filter(|i| i.severity == ReferenceSeverity::Error)
            .collect();

        if errors.is_empty() {
            return String::new();
        }

        let mut out = String::new();
        let mut groups: BTreeMap<u8, (ReferenceKind, Vec<&ReferenceIssue>)> = BTreeMap::new();
        for issue in &errors {
            groups
                .entry(issue.kind.validation_order())
                .or_insert_with(|| (issue.kind, Vec::new()))
                .1
                .push(*issue);
        }

        out.push('\n');

        for (kind, issues) in groups.values() {
            let label = kind.validation_category_label();
            out.push_str(&Prose::new(format!("<red-500><b>{label}</b></red-500>")).render(term));
            out.push('\n');

            let mut list = UnorderedList::empty();
            for issue in issues {
                let source_name = match &issue.origin.source {
                    ComposeSource::File(p) => p
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "unknown".to_string()),
                    ComposeSource::Url(u) => u.to_string(),
                    ComposeSource::Unknown => "unknown".to_string(),
                };

                let source_href = match &issue.origin.source {
                    ComposeSource::File(p) => format!("file://{}", p.display()),
                    _ => String::new(),
                };

                let item_text = if source_href.is_empty() {
                    format!(
                        "the <blue-500>{source_name}</blue-500> reference to <red-500>{ref_display}</red-500> is not valid",
                        ref_display = issue.reference_display,
                    )
                } else {
                    format!(
                        "the <a href=\"{source_href}\"><blue-500>{source_name}</blue-500></a> reference to <red-500>{ref_display}</red-500> is not valid",
                        ref_display = issue.reference_display,
                    )
                };

                list.add(Prose::new(item_text));
            }

            out.push_str(&list.render(term));
            out.push('\n');
        }

        let summary = format!(
            "{} references scanned, {} valid, <red-500><b>{} issues</b></red-500>",
            self.report.references_scanned,
            self.report.references_valid,
            errors.len()
        );
        out.push_str(&Prose::new(summary).render(term));
        out.push('\n');

        out
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

impl Serialize for ReferenceIssue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("ReferenceIssue", 7)?;
        state.serialize_field("code", &self.code)?;
        state.serialize_field("message", &self.message)?;
        state.serialize_field("severity", &self.severity)?;
        state.serialize_field("kind", &self.kind)?;
        state.serialize_field("reference", &self.reference_display)?;
        state.serialize_field("line", &self.origin.line)?;
        state.serialize_field("source", &self.origin.source)?;
        state.end()
    }
}

/// Classification codes for validation issues.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "PascalCase")]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReferenceSeverity {
    /// Must be fixed.
    Error,
    /// Should be addressed.
    Warning,
    /// Informational.
    Info,
}

/// Run validation on a markdown document's references.
#[instrument(skip_all)]
pub(crate) fn validate(
    md: &Markdown,
    options: &ReferenceValidationOptions,
) -> Result<ReferenceValidationReport, ReferenceError> {
    info!("validate: starting reference validation");
    let ref_set = {
        let graph = super::graph::build_reference_graph(md, &options.graph).map_err(|error| {
            match error {
                crate::markdown::MarkdownError::Reference(inner) => *inner,
                other => ReferenceError::Compose(Box::new(other)),
            }
        })?;
        super::graph::flatten_graph(&graph)
    };

    let mut report = ReferenceValidationReport {
        references_scanned: ref_set.len(),
        ..Default::default()
    };

    debug!(
        ref_count = report.references_scanned,
        "validate: references collected"
    );

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
                // ::file-links targets are glob patterns or dir+depth specs, not
                // actual file paths — skip existence validation and defer to
                // compose-time discovery.
                if record.origin.syntax == super::types::ReferenceSyntax::DirectiveFileLinks {
                    report.references_valid += 1;
                    continue;
                }

                // Check for fragment in local path (e.g., "./other.md#section")
                let raw_str = raw.to_string_lossy();
                let (path_part, fragment) = split_path_fragment(&raw_str);
                let resolved_path = validate_local_path(
                    &path_part,
                    ref_source,
                    record,
                    &mut report,
                    &options.graph,
                )?;

                // Validate fragment if enabled and path exists
                if options.validate_fragments
                    && let Some(ref frag) = fragment
                {
                    validate_cross_doc_fragment(
                        resolved_path.as_deref(),
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
    graph_options: &ReferenceGraphOptions,
) -> Result<Option<std::path::PathBuf>, ReferenceError> {
    trace!(raw = %raw, "validate: checking local path");
    match source {
        ComposeSource::File(_) => {
            if let Some(resolved) = super::resolve_transclusion_target(
                raw,
                source,
                &graph_options.compose,
            )? {
                report.references_valid += 1;
                return Ok(Some(resolved));
            }
            report.issues.push(ReferenceIssue {
                code: ReferenceIssueCode::MissingLocalTarget,
                message: format!("Missing local target: {raw}"),
                severity: ReferenceSeverity::Error,
                kind: record.kind,
                reference_display: raw.to_string(),
                reference_id: record.id.clone(),
                origin: record.origin.clone(),
            });
            Ok(None)
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
            Ok(None)
        }
        ComposeSource::Url(_) => {
            report
                .warnings
                .push(format!("Local path in URL-sourced document: {raw}"));
            report.references_valid += 1;
            Ok(None)
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
    target_path: Option<&std::path::Path>,
    path: &str,
    fragment: &str,
    source: &ComposeSource,
    record: &ReferenceRecord,
    report: &mut ReferenceValidationReport,
    graph_options: &super::types::ReferenceGraphOptions,
) {
    let ComposeSource::File(_) = source else {
        return;
    };
    let Some(target_path) = target_path else {
        return; // Missing file is already reported by validate_local_path
    };

    // Only validate fragments in markdown files
    let ext = target_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    if !["md", "markdown", "mdx"].contains(&ext) {
        return;
    }

    if let Ok(target_md) = Markdown::try_from(target_path) {
        let target_source = ComposeSource::File(target_path.to_path_buf());
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
    debug!(
        url_count = records.len(),
        "validate: starting remote URL checks"
    );
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
    use crate::markdown::reference::ReferenceSyntax;
    use serial_test::serial;

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

    /// Regression test: magic-path (`@`) references must validate correctly
    /// regardless of CWD. The request-scoped context must anchor resolution to
    /// the document instead of ambient process state.
    #[test]
    #[serial]
    fn validate_magic_path_independent_of_cwd() {
        use biscuit_file::PathPosition;

        // Build a temp directory tree that mimics a repo with a magic path:
        //   <tmp>/
        //     darkmatter/docs/inline/text-replacement.md   (target)
        //     example-docs/composition/source.md           (source with @-link)
        let dir = tempfile::TempDir::new().unwrap();
        let root = dir.path();

        let target_dir = root.join("darkmatter/docs/inline");
        std::fs::create_dir_all(&target_dir).unwrap();
        std::fs::write(target_dir.join("text-replacement.md"), "# Text Replacement").unwrap();

        let source_dir = root.join("example-docs/composition");
        std::fs::create_dir_all(&source_dir).unwrap();
        let source_path = source_dir.join("source.md");
        std::fs::write(&source_path, "").unwrap();

        // The markdown references the target via a magic path
        let md = Markdown::new("[Text Replacement](@darkmatter/docs/inline/text-replacement.md)")
            .with_source(ComposeSource::File(source_path));

        // Configure the magic path: @darkmatter → <tmp>/darkmatter
        let mut options = ReferenceValidationOptions::default();
        options
            .graph
            .compose
            .magic_paths
            .push((root.to_path_buf(), PathPosition::Start));

        // CWD is whatever the test runner uses — NOT source_dir.
        // This is the exact scenario that was broken before the fix.
        let report = validate(&md, &options).unwrap();
        assert!(
            report.is_valid(),
            "Magic-path reference should be valid regardless of CWD, got issues: {:?}",
            report.issues
        );
        assert_eq!(report.references_valid, 1);
    }

    #[test]
    fn validation_report_serializes_graph_shape() {
        let issue = ReferenceIssue {
            code: ReferenceIssueCode::MissingLocalTarget,
            message: "Missing local target: ./missing.md".to_string(),
            severity: ReferenceSeverity::Error,
            kind: ReferenceKind::Hyperlink,
            reference_display: "./missing.md".to_string(),
            reference_id: "abc:3:10".to_string(),
            origin: ReferenceOrigin {
                source: ComposeSource::File("/tmp/source.md".into()),
                line: 3,
                span: 0..10,
                syntax: ReferenceSyntax::MarkdownLink,
            },
        };
        let report = ReferenceValidationReport {
            references_scanned: 1,
            references_valid: 0,
            issues: vec![issue],
            warnings: Vec::new(),
        };

        let value = serde_json::to_value(&report).unwrap();
        let obj = value.as_object().unwrap();
        assert_eq!(obj.get("valid").unwrap(), false);
        assert_eq!(obj.get("references_scanned").unwrap(), 1);
        assert_eq!(obj.get("references_valid").unwrap(), 0);
        assert_eq!(obj.get("warnings").unwrap(), &serde_json::json!([]));

        let issues = obj.get("issues").unwrap().as_array().unwrap();
        assert_eq!(issues.len(), 1);
        let issue_obj = issues[0].as_object().unwrap();
        assert_eq!(issue_obj.get("code").unwrap(), "MissingLocalTarget");
        assert_eq!(issue_obj.get("severity").unwrap(), "error");
        assert_eq!(issue_obj.get("kind").unwrap(), "hyperlink");
        assert_eq!(issue_obj.get("reference").unwrap(), "./missing.md");
        assert_eq!(issue_obj.get("line").unwrap(), 3);
        assert_eq!(
            issue_obj.get("source").unwrap(),
            "/tmp/source.md"
        );
    }

    #[test]
    fn validation_report_view_renders_error_groups() {
        let issue = ReferenceIssue {
            code: ReferenceIssueCode::MissingLocalTarget,
            message: "Missing local target: ./missing.md".to_string(),
            severity: ReferenceSeverity::Error,
            kind: ReferenceKind::Hyperlink,
            reference_display: "./missing.md".to_string(),
            reference_id: "abc:3:10".to_string(),
            origin: ReferenceOrigin {
                source: ComposeSource::File("/tmp/source.md".into()),
                line: 3,
                span: 0..10,
                syntax: ReferenceSyntax::MarkdownLink,
            },
        };
        let report = ReferenceValidationReport {
            references_scanned: 1,
            references_valid: 0,
            issues: vec![issue],
            warnings: Vec::new(),
        };

        use biscuit_terminal::components::renderable::TerminalRenderable as _;
        let rendered = ValidationReportView::new(report)
            .render(&biscuit_terminal::terminal::Terminal::new_forced());

        assert!(rendered.contains("Invalid Hyperlink(s)"));
        assert!(rendered.contains("missing.md"));
        assert!(rendered.contains("references scanned, 0 valid"));
    }

    #[test]
    fn validation_report_view_empty_report_renders_nothing() {
        let report = ReferenceValidationReport {
            references_scanned: 1,
            references_valid: 1,
            issues: Vec::new(),
            warnings: Vec::new(),
        };

        use biscuit_terminal::components::renderable::TerminalRenderable as _;
        let rendered = ValidationReportView::new(report)
            .render(&biscuit_terminal::terminal::Terminal::new_forced());

        assert!(rendered.is_empty());
    }
}
