//! Inline composition closure: read the agent's on-disk artifact, judge it
//! against the pre-run guard, restore the closure-owned nodes, and persist the
//! result with exactly one atomic write.
//!
//! The agent is the writer (spec `2026-09-05-inline-flow-and-validations` §D4),
//! so nothing here parses provider output. The guard snapshot is consulted only
//! for the three owned properties and for the body-change verdict.

use darkmatter::markdown::hash::{
    FrontmatterDelta, MdHashKind, MdHashOptions, StoredHash, apply_hash_save_text,
    restore_properties_text,
};
use darkmatter::markdown::{Markdown, MarkdownResult, extract_frontmatter_block};

use crate::composition::error::CompositionError;
use crate::composition::types::InlineClosurePlan;

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
    /// property. Layered over live effective frontmatter to build the
    /// completion instance.
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
/// leaves the file untouched so the caller owns the rollback decision.
///
/// ## Errors
///
/// Returns [`CompositionError::InlineArtifactUnreadable`] when the document
/// cannot be read back, [`CompositionError::InlineArtifactEditFailed`] when
/// either frontmatter block is malformed or carries duplicate owned keys,
/// [`CompositionError::InlineHashMalformed`] when the stored `hash` cannot be
/// parsed, and [`CompositionError::AtomicWriteFailed`] when the write fails.
pub fn reconcile_inline_artifact(
    plan: &InlineClosurePlan,
    today: &str,
) -> Result<InlineReconciliation, CompositionError> {
    reconcile_inline_artifact_with_evidence(plan, today, false)
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
    today: &str,
    prior_body_change: bool,
) -> Result<InlineReconciliation, CompositionError> {
    let path = plan.document_path.as_path();
    let current = std::fs::read_to_string(path).map_err(|source| {
        CompositionError::InlineArtifactUnreadable {
            path: path.to_path_buf(),
            source,
        }
    })?;

    let raw_body = raw_body_of(&current)?.to_string();
    let cleaned_body = darkmatter::markdown::cleanup::cleanup_content(&raw_body);
    if cleaned_body.trim().is_empty() {
        return Ok(InlineReconciliation::Rejected(BodyRejection::Empty));
    }
    let body_cleaned = cleaned_body != raw_body;

    let candidate = replace_body(&current, &cleaned_body)?;
    // Both sides run through the same cleanup pass before hashing so a baseline
    // that was never cleanup-stable cannot masquerade as an agent edit.
    let baseline_cleaned = cleaned_body_of(&plan.original_document_text)?;
    let baseline = replace_body(&plan.original_document_text, &baseline_cleaned)?;
    if !prior_body_change && non_strict_body_hash(&candidate) == non_strict_body_hash(&baseline) {
        return Ok(InlineReconciliation::Rejected(BodyRejection::Unchanged));
    }

    let restored = restore_properties_text(
        &candidate,
        &plan.original_document_text,
        CLOSURE_OWNED_PROPERTIES,
    )
    .map_err(CompositionError::InlineArtifactEditFailed)?;

    let opts = inline_hash_options();
    let md: Markdown = restored.text.clone().into();
    let stored =
        parse_inline_stored_hash(&md, &opts).map_err(CompositionError::InlineHashMalformed)?;
    let mut decision = md
        .plan_hash_save(stored.as_ref(), &opts)
        .map_err(CompositionError::InlineHashMalformed)?;
    // Hash-save treats a missing stored hash as baseline creation, but every
    // accepted inline closure is a known body mutation and must date it.
    decision.bump_last_updated = true;
    let text = apply_hash_save_text(&restored.text, &decision, &opts, today)
        .map_err(CompositionError::InlineHashMalformed)?
        .unwrap_or(restored.text);

    crate::config::atomic::atomic_write(path, text.as_bytes()).map_err(|source| {
        CompositionError::AtomicWriteFailed {
            path: path.to_path_buf(),
            source,
        }
    })?;

    Ok(InlineReconciliation::Written(Box::new(InlineArtifact {
        text,
        restored_properties: restored.restored_properties,
        frontmatter_delta: restored.frontmatter_delta,
        body_cleaned,
    })))
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

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

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

/// Parses the document's stored `hash` property, or returns `None` when it is
/// absent or null.
///
/// Mirrors the CLI pattern in `darkmatter/cli/src/commands/hash.rs` so
/// inline-compose shares the same stored-hash contract as `md hash --save`.
fn parse_inline_stored_hash(
    md: &Markdown,
    opts: &MdHashOptions,
) -> MarkdownResult<Option<StoredHash>> {
    match md.frontmatter().as_map().get(opts.property.as_str()) {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(value) => StoredHash::parse(value, &opts.property).map(Some),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;
