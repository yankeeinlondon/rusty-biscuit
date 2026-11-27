//! Inline composition closure: read the agent's on-disk artifact, judge it
//! against the pre-run guard, repair and restore its frontmatter, store
//! agent-written instruction text as literal tokens, and persist the result
//! with exactly one atomic write.
//!
//! The agent is the writer (spec `2026-09-05-inline-flow-and-validations` §D4),
//! so nothing here parses provider output. The guard snapshot decides which
//! frontmatter the agent changed, restores the three owned properties, and
//! anchors the body-change verdict.

use chrono::{DateTime, Utc};
use darkmatter::markdown::hash::{
    Change, FrontmatterDelta, FrontmatterDeltaEntry, MdHashKind, MdHashOptions,
    restore_properties_text,
};
use darkmatter::markdown::{FrontmatterMap, Markdown, MarkdownError, extract_frontmatter_block};
use std::path::Path;

use crate::composition::error::CompositionError;
use crate::composition::types::InlineClosurePlan;

mod persist;

pub use persist::{
    AgentFrontmatterRejection, EncodeError, encode_agent_values, repair_agent_frontmatter,
};
pub(crate) use persist::{opens_stored_token, persisted_data, stored_text};

/// Frontmatter properties the closure owns; the agent is told not to touch them.
///
/// `prompt` is the author's immutable interface, while `hash` and
/// `last_updated` are stamped by the closure itself on every write.
pub const CLOSURE_OWNED_PROPERTIES: &[&str] = &["prompt", "hash", "last_updated"];

/// Why the agent's candidate body was refused before any stamp or write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyRejection {
    /// The candidate body is whitespace-only.
    Empty,
    /// The candidate body is semantically identical to the pre-run baseline.
    Unchanged,
}

impl BodyRejection {
    /// The stable snake_case slug projected to `err.detail.reason`.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Unchanged => "unchanged",
        }
    }
}

impl std::fmt::Display for BodyRejection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The reconciled inline artifact, exactly as it was written to disk.
#[derive(Debug, Clone)]
pub struct InlineArtifact {
    /// Final document text, including the fresh `hash` and `last_updated`.
    pub text: String,
    /// Owned properties the agent changed and the closure restored. One CLI
    /// warning is emitted per entry; this is never an error.
    pub restored_properties: Vec<String>,
    /// The agent's semantic frontmatter changes, excluding every owned
    /// property. Values are what composition reads, so a value stored as a
    /// literal token appears as its decoded text. Layered over live effective
    /// frontmatter to build the completion instance.
    pub frontmatter_delta: FrontmatterDelta,
    /// Whether Darkmatter's cleanup pass rewrote the agent's body.
    pub body_cleaned: bool,
}

/// Outcome of reconciling the agent's on-disk document with the guard.
#[derive(Debug, Clone)]
pub enum InlineReconciliation {
    /// The candidate was accepted, stamped, and written once.
    Written(Box<InlineArtifact>),
    /// The candidate body was refused; nothing was stamped or written.
    Rejected(BodyRejection),
}

/// Read, judge, reconcile, and persist the agent's inline artifact.
///
/// The document is written exactly once, and only when the candidate body is
/// non-empty and semantically different from the guard's baseline. A rejection
/// leaves the file untouched so the caller owns the rollback decision. A YAML
/// source stays YAML: its deliverable is a changed mapping, and only the
/// mapping is written back.
///
/// ## Errors
///
/// Returns [`CompositionError::InlineArtifactUnreadable`] when the document
/// cannot be read back, [`CompositionError::InlineAgentFrontmatterRejected`]
/// when the agent's frontmatter is invalid YAML after the narrow repair or a
/// value that must be stored as a literal token cannot be located,
/// [`CompositionError::InlineArtifactEditFailed`] when either frontmatter block
/// is otherwise unusable or carries duplicate owned keys,
/// [`CompositionError::InlineHashMalformed`] when the stored `hash` cannot be
/// parsed, and [`CompositionError::AtomicWriteFailed`] when the write fails.
pub fn reconcile_inline_artifact(
    plan: &InlineClosurePlan,
    now: DateTime<Utc>,
) -> Result<InlineReconciliation, CompositionError> {
    reconcile_inline_artifact_with_evidence(plan, now, false)
}

/// [`reconcile_inline_artifact`], with the operation's accumulated body-change
/// evidence.
///
/// `prior_body_change` is `true` when an earlier attempt of *this* active
/// document already wrote a meaningfully changed body (spec §D4). A recovery
/// attempt that only repairs frontmatter then leaves the body identical to the
/// operation baseline, and refusing it as `Unchanged` would reject the very
/// repair `retry`/`resume` was asked to make. An empty body is still refused:
/// no accumulated evidence makes a blank document a deliverable.
///
/// ## Errors
///
/// Identical to [`reconcile_inline_artifact`].
pub fn reconcile_inline_artifact_with_evidence(
    plan: &InlineClosurePlan,
    now: DateTime<Utc>,
    prior_body_change: bool,
) -> Result<InlineReconciliation, CompositionError> {
    let path = plan.document_path.as_path();
    let current = std::fs::read_to_string(path).map_err(|source| {
        CompositionError::InlineArtifactUnreadable {
            path: path.to_path_buf(),
            source,
        }
    })?;
    let format = SourceFormat::of(path);

    let (candidate, original, body_cleaned) = match format {
        SourceFormat::Markdown => {
            let raw_body = raw_body_of(&current)?.to_string();
            let cleaned_body = darkmatter::markdown::cleanup::cleanup_content(&raw_body);
            if cleaned_body.trim().is_empty() {
                return Ok(InlineReconciliation::Rejected(BodyRejection::Empty));
            }
            let candidate = replace_body(&current, &cleaned_body)?;
            // Both sides run through the same cleanup pass before hashing so a
            // baseline that was never cleanup-stable cannot masquerade as an
            // agent edit.
            let baseline_cleaned = cleaned_body_of(&plan.original_document_text)?;
            let baseline = replace_body(&plan.original_document_text, &baseline_cleaned)?;
            if !prior_body_change
                && non_strict_body_hash(&candidate) == non_strict_body_hash(&baseline)
            {
                return Ok(InlineReconciliation::Rejected(BodyRejection::Unchanged));
            }
            (candidate, plan.original_document_text.clone(), cleaned_body != raw_body)
        }
        // The whole file is the mapping; Markdown cleanup would rewrite YAML
        // lines (it spaces out lists), so the agent's bytes are taken as written.
        SourceFormat::Yaml => {
            if current.trim().is_empty() {
                return Ok(InlineReconciliation::Rejected(BodyRejection::Empty));
            }
            let candidate =
                yaml_envelope(&current).map_err(CompositionError::InlineArtifactEditFailed)?;
            let original = yaml_envelope(&plan.original_document_text)
                .map_err(CompositionError::InlineArtifactEditFailed)?;
            (candidate, original, false)
        }
    };

    // Order (ruling N7): repair before restoring, since restoring needs
    // parseable YAML; encode after, so the hash and the write see the tokens.
    let rejected = |rejection| CompositionError::InlineAgentFrontmatterRejected {
        path: path.to_path_buf(),
        rejection: Box::new(rejection),
    };
    let candidate = repair_agent_frontmatter(&candidate, &original).map_err(rejected)?;
    let restored = restore_properties_text(&candidate, &original, CLOSURE_OWNED_PROPERTIES)
        .map_err(CompositionError::InlineArtifactEditFailed)?;
    // A YAML source has no body, so its deliverable is a change to the
    // mapping outside the owned properties, judged after they are restored.
    if format == SourceFormat::Yaml && !prior_body_change && restored.frontmatter_delta.is_empty() {
        return Ok(InlineReconciliation::Rejected(BodyRejection::Unchanged));
    }
    let encoded = encode_agent_values(&restored.text, &restored.frontmatter_delta).map_err(
        |error| match error {
            EncodeError::Rejected(rejection) => rejected(rejection),
            EncodeError::Frontmatter(error) => CompositionError::InlineArtifactEditFailed(error),
        },
    )?;
    let frontmatter_delta = decoded_delta(restored.frontmatter_delta, &encoded);

    let opts = inline_hash_options();
    let md: Markdown = encoded.clone().into();
    // Every accepted inline closure is a known body mutation, so it dates the
    // baseline even when the document had no stored hash yet.
    let text = md
        .stamp_baseline(&encoded, &opts, now, Change::Known)
        .map_err(CompositionError::InlineHashMalformed)?
        .text
        .unwrap_or(encoded);
    let text = match format {
        SourceFormat::Markdown => text,
        SourceFormat::Yaml => {
            yaml_of_envelope(&text).map_err(CompositionError::InlineArtifactEditFailed)?
        }
    };

    crate::config::atomic::atomic_write(path, text.as_bytes()).map_err(|source| {
        CompositionError::AtomicWriteFailed {
            path: path.to_path_buf(),
            source,
        }
    })?;

    Ok(InlineReconciliation::Written(Box::new(InlineArtifact {
        text,
        restored_properties: restored.restored_properties,
        frontmatter_delta,
        body_cleaned,
    })))
}

/// Rehearse, on the captured baseline alone, the frontmatter edits an
/// accepted closure makes: restoring [`CLOSURE_OWNED_PROPERTIES`] from the
/// baseline and stamping it with [`inline_hash_options`] and
/// [`Change::Known`]. Every result is discarded; nothing is reconciled or
/// written.
///
/// `Ok` means only that the baseline as captured is editable. It does not
/// guarantee the closure will succeed: the agent's edits or an external change
/// can still make the document uneditable, and the closure's own validation
/// and rollback handle that.
///
/// ## Errors
///
/// Returns Darkmatter's refusal unchanged, typically
/// [`MarkdownError::FrontmatterTextEdit`] (for example an indented root
/// mapping) or [`MarkdownError::MalformedStoredHash`].
pub(crate) fn rehearse_inline_baseline(
    path: &Path,
    original: &str,
    now: DateTime<Utc>,
) -> Result<(), MarkdownError> {
    let original = match SourceFormat::of(path) {
        SourceFormat::Markdown => original.to_string(),
        SourceFormat::Yaml => yaml_envelope(original)?,
    };
    let original = original.as_str();
    let restored = restore_properties_text(original, original, CLOSURE_OWNED_PROPERTIES)?;
    let md: Markdown = restored.text.as_str().into();
    md.stamp_baseline(&restored.text, &inline_hash_options(), now, Change::Known)?;
    Ok(())
}

/// Atomically restore the active document to its pre-run baseline.
///
/// The rollback half of the inline guard (spec §D4). Nothing is stamped: the
/// baseline is written back exactly as it was captured, so a restored document
/// carries the `hash` and `last_updated` it had before the run.
///
/// ## Errors
///
/// Returns [`CompositionError::InlineRollbackFailed`] when the restoring write
/// fails. The caller keeps its initiating diagnostic and attaches this as the
/// rollback cause; it must not claim the document was restored.
pub fn restore_inline_baseline(plan: &InlineClosurePlan) -> Result<(), CompositionError> {
    let path = plan.document_path.as_path();
    crate::config::atomic::atomic_write(path, plan.original_document_text.as_bytes()).map_err(
        |source| CompositionError::InlineRollbackFailed {
            path: path.to_path_buf(),
            source,
        },
    )
}

/// Hash options used for every inline-compose hash computation.
///
/// Forces [`MdHashKind::Simple`] so any pre-existing `structured` or `detailed`
/// stored hash is normalized to the `Simple` shape on the next run. Excludes
/// the managed `hash` and `last_updated` keys from the frontmatter segment so
/// the stamp itself cannot influence the hash.
pub fn inline_hash_options() -> MdHashOptions {
    MdHashOptions {
        forced_kind: Some(MdHashKind::Simple),
        ..MdHashOptions::default()
    }
}

/// The frontmatter of `text`, a document the closure wrote to
/// `plan.document_path`, read in that file's own format.
pub(crate) fn written_frontmatter(plan: &InlineClosurePlan, text: &str) -> FrontmatterMap {
    let markdown: Markdown = match SourceFormat::of(&plan.document_path) {
        SourceFormat::Markdown => text.to_string().into(),
        SourceFormat::Yaml => match yaml_envelope(text) {
            Ok(envelope) => envelope.into(),
            Err(_) => return FrontmatterMap::default(),
        },
    };
    markdown.frontmatter().as_map().clone()
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// The on-disk format of an inline document, which decides how the closure
/// reads the agent's edit and writes the result back.
///
/// A YAML source (a `kind: sequence` file with a document-level `prompt`) is
/// loaded by composition as frontmatter with an empty body
/// ([`crate::composition::load_yaml_document`]). The closure edits it through
/// a transient frontmatter envelope and writes back only the mapping, so the
/// file stays one YAML document that the next read can load.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SourceFormat {
    Markdown,
    Yaml,
}

impl SourceFormat {
    fn of(path: &Path) -> Self {
        if crate::composition::is_yaml_source(path) { Self::Yaml } else { Self::Markdown }
    }
}

/// `yaml` as a body-less Markdown document whose frontmatter is exactly
/// `yaml`'s bytes.
///
/// ## Errors
///
/// Returns [`MarkdownError::FrontmatterTextEdit`] when the bytes do not
/// survive the round trip, for example a YAML file holding its own `---`
/// document marker.
fn yaml_envelope(yaml: &str) -> Result<String, MarkdownError> {
    let mut envelope = String::with_capacity(yaml.len() + 8);
    envelope.push_str("---\n");
    envelope.push_str(yaml);
    if !yaml.is_empty() && !yaml.ends_with('\n') {
        envelope.push('\n');
    }
    envelope.push_str("---\n");
    match yaml_of_envelope(&envelope) {
        Ok(inner) if inner.trim_end_matches(['\r', '\n']) == yaml.trim_end_matches(['\r', '\n']) => {
            Ok(envelope)
        }
        _ => Err(MarkdownError::FrontmatterTextEdit {
            reason: "the YAML document cannot be edited as a mapping because it holds a \
                     `---` or `...` document marker line"
                .to_string(),
        }),
    }
}

/// The frontmatter bytes of an envelope built by [`yaml_envelope`].
fn yaml_of_envelope(envelope: &str) -> Result<String, MarkdownError> {
    match extract_frontmatter_block(envelope)? {
        Some(extraction) if extraction.body_span.is_empty() => {
            Ok(extraction.yaml.to_string())
        }
        _ => Err(MarkdownError::FrontmatterTextEdit {
            reason: "the edited YAML document no longer reads as one mapping".to_string(),
        }),
    }
}

/// `delta` with each added or replaced value as composition reads it from
/// `document`: literal tokens decoded, every other byte as written. A malformed
/// token stays raw; composition reports it.
fn decoded_delta(mut delta: FrontmatterDelta, document: &str) -> FrontmatterDelta {
    let md: Markdown = document.to_string().into();
    let written = md.frontmatter().as_map();
    for entry in &mut delta.entries {
        let (FrontmatterDeltaEntry::Addition { property, value }
        | FrontmatterDeltaEntry::Replacement {
            property, value, ..
        }) = entry
        else {
            continue;
        };
        let Some(stored) = written.get(property.as_str()) else {
            continue;
        };
        *value = stored_text(stored);
    }
    delta
}

fn raw_body_of(document: &str) -> Result<&str, CompositionError> {
    match extract_frontmatter_block(document).map_err(CompositionError::InlineArtifactEditFailed)? {
        Some(extraction) => Ok(&document[extraction.body_span]),
        None => Ok(document),
    }
}

fn cleaned_body_of(document: &str) -> Result<String, CompositionError> {
    Ok(darkmatter::markdown::cleanup::cleanup_content(raw_body_of(
        document,
    )?))
}

/// Replace `document`'s body while preserving every frontmatter byte.
fn replace_body(document: &str, body: &str) -> Result<String, CompositionError> {
    match extract_frontmatter_block(document).map_err(CompositionError::InlineArtifactEditFailed)? {
        Some(extraction) => {
            let head = &document[..extraction.block_span.end];
            let mut text = String::with_capacity(head.len() + body.len());
            text.push_str(head);
            text.push_str(body);
            Ok(text)
        }
        None => Ok(body.to_string()),
    }
}

/// The non-strict `Simple` body hash: leading/trailing whitespace and blank
/// lines are ignored, while internal whitespace stays significant.
fn non_strict_body_hash(document: &str) -> u64 {
    let md: Markdown = document.to_string().into();
    md.hash_body(false)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;
