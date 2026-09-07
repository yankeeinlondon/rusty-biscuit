//! The completion verdict both composition modes reach.
//!
//! One passive check decides whether a run is routed to `success` or
//! `failure` (spec `2026-09-05-inline-flow-and-validations` §D5/§D6):
//!
//! - `inline-compose` reconciles the agent's on-disk artifact first, then
//!   validates the live effective frontmatter with the agent's semantic
//!   frontmatter delta and the closure's owned values layered on top.
//! - `compose` validates the live effective frontmatter unchanged and performs
//!   no source-file read or write.
//!
//! [`evaluate_completion`] is pure: it composes nothing, executes no
//! expression or shell, resolves no schema reference, and touches no file. The
//! schema it judges is the one retained from stabilized launch, so a run cannot
//! weaken its own contract by editing `$schema` mid-run.

use std::path::Path;

use darkmatter::markdown::Markdown;
use darkmatter::markdown::hash::{FrontmatterDelta, FrontmatterDeltaEntry};
use darkmatter::markdown::schemas::{SchemaPhase, ValidationProblem, ValidationProblemKind};

use super::closure::{
    BodyRejection, CLOSURE_OWNED_PROPERTIES, InlineArtifact, InlineReconciliation,
    reconcile_inline_artifact_with_evidence,
};
use super::error::CompositionError;
use super::schema::{
    SchemaStatusReport, schema_error_to_composition_error, status_report_from_validation,
};
use super::types::{CompositionMode, InlineClosurePlan, LaunchSchema};

/// What the completion verdict knows about the run's body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyEvidence {
    /// Direct compose produces no document body, so the check does not apply.
    NotApplicable,
    /// The inline artifact's body changed meaningfully.
    Changed,
    /// The inline artifact's body was refused before any stamp or write.
    Rejected(BodyRejection),
}

impl BodyEvidence {
    /// Whether the body half of the verdict is satisfied.
    pub const fn is_satisfied(self) -> bool {
        matches!(self, Self::NotApplicable | Self::Changed)
    }

    /// The rejection reason, when the body was refused.
    pub const fn rejection(self) -> Option<BodyRejection> {
        match self {
            Self::Rejected(reason) => Some(reason),
            _ => None,
        }
    }
}

/// One property-level completion failure, in schema declaration order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionProblem {
    /// Dotted property path (`products[2].uk_price`, `researched_by`).
    pub property: String,
    /// Human-facing reason, in the same wording the launch report uses.
    pub message: String,
    /// Whether the property was absent or present-but-wrong.
    pub kind: CompletionProblemKind,
}

/// Coarse classification of a [`CompletionProblem`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionProblemKind {
    /// A property required at completion is absent.
    Missing,
    /// A present value violates its declared type or constraint.
    Invalid,
}

impl CompletionProblemKind {
    /// The stable snake_case slug projected to `err.detail.properties[].kind`.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::Invalid => "invalid",
        }
    }
}

/// The passive verdict that routes a run to `success` or `failure`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionVerdict {
    /// The active document the verdict was taken against.
    pub source_path: std::path::PathBuf,
    /// What the verdict knows about the body.
    pub body: BodyEvidence,
    /// Per-property completion failures, in schema declaration order.
    pub problems: Vec<CompletionProblem>,
    /// The whole schema's per-property status, so an author sees satisfied
    /// properties alongside the failures. `None` when the document declares no
    /// `$schema` or the schema exposes no property table.
    pub status: Option<SchemaStatusReport>,
}

impl CompletionVerdict {
    /// Whether both halves of the verdict passed.
    pub fn is_satisfied(&self) -> bool {
        self.body.is_satisfied() && self.problems.is_empty()
    }

    /// The typed error a failed verdict carries into `failure`.
    ///
    /// Body evidence outranks schema problems: a document the agent never
    /// updated has no meaningful schema story to tell.
    pub fn into_error(self) -> Option<CompositionError> {
        if let Some(reason) = self.body.rejection() {
            return Some(CompositionError::CompletionBodyUnchanged {
                source_path: self.source_path,
                reason,
            });
        }
        if self.problems.is_empty() {
            return None;
        }
        let status = self.status.unwrap_or_else(|| SchemaStatusReport {
            source_path: self.source_path.clone(),
            required: Vec::new(),
            optional: Vec::new(),
            has_invalid_optional: false,
            raw_json_schema: true,
        });
        Some(CompositionError::CompletionSchemaFailed {
            source_path: self.source_path,
            problems: self.problems,
            status,
        })
    }
}

/// Everything [`complete_active_document`] needs, and nothing it may re-derive.
#[derive(Debug, Clone, Copy)]
pub struct CompletionContext<'a> {
    /// Which composition mode's completion policy applies.
    pub mode: CompositionMode,
    /// The active document, which may be a proxy target rather than the
    /// originally invoked file.
    pub active_path: &'a Path,
    /// The schema resolved once at stabilized launch and retained since.
    pub launch_schema: Option<&'a LaunchSchema>,
    /// The attempt's live effective frontmatter.
    pub live_frontmatter: &'a serde_json::Value,
    /// The inline guard captured when the document was invoked or adopted.
    /// Required for [`CompositionMode::InlineFrontmatterPrompt`].
    pub inline_guard: Option<&'a InlineClosurePlan>,
    /// Whether an earlier attempt of this active document already wrote a
    /// meaningfully changed body.
    ///
    /// Operation-level, not attempt-level: a `retry`/`resume` that repairs only
    /// frontmatter leaves the body identical to the guard's baseline, and the
    /// body requirement was already satisfied by the write it is recovering
    /// from (spec §D4).
    pub prior_body_change: bool,
    /// Today's date, in `%Y-%m-%d`, for the `last_updated` stamp.
    pub today: &'a str,
}

/// The completion verdict plus the inline artifact it was taken against.
#[derive(Debug, Clone)]
pub struct CompletionOutcome {
    /// The verdict that routes the run.
    pub verdict: CompletionVerdict,
    /// The reconciled artifact, for `inline-compose` runs that wrote one.
    /// `None` for direct compose and for a refused inline candidate.
    pub artifact: Option<Box<InlineArtifact>>,
}

/// The one orchestration entry point both composition modes reach.
///
/// For `inline-compose` this reads, judges, reconciles, and persists the
/// agent's artifact before validating. For `compose` it validates the live
/// effective frontmatter and performs no source-file read or write.
///
/// ## Errors
///
/// Returns [`CompositionError::InlineGuardMissing`] when an inline context
/// carries no guard, and propagates the typed read/edit/hash/write failures
/// from [`reconcile_inline_artifact`]. A failed *verdict* is not an error: it
/// is data the caller routes through `failure`.
pub fn complete_active_document(
    context: CompletionContext<'_>,
) -> Result<CompletionOutcome, CompositionError> {
    match context.mode {
        CompositionMode::ChainedDocument => {
            let verdict = evaluate_completion(
                context.launch_schema,
                context.live_frontmatter,
                BodyEvidence::NotApplicable,
                context.active_path,
            )?;
            Ok(CompletionOutcome {
                verdict,
                artifact: None,
            })
        }
        CompositionMode::InlineFrontmatterPrompt => {
            let guard = context.inline_guard.ok_or_else(|| {
                CompositionError::InlineGuardMissing {
                    source_path: context.active_path.to_path_buf(),
                }
            })?;
            match reconcile_inline_artifact_with_evidence(
                guard,
                context.today,
                context.prior_body_change,
            )? {
                InlineReconciliation::Rejected(reason) => Ok(CompletionOutcome {
                    verdict: evaluate_completion(
                        context.launch_schema,
                        context.live_frontmatter,
                        BodyEvidence::Rejected(reason),
                        context.active_path,
                    )?,
                    artifact: None,
                }),
                InlineReconciliation::Written(artifact) => {
                    let instance =
                        inline_completion_instance(context.live_frontmatter, artifact.as_ref());
                    let verdict = evaluate_completion(
                        context.launch_schema,
                        &instance,
                        BodyEvidence::Changed,
                        context.active_path,
                    )?;
                    Ok(CompletionOutcome {
                        verdict,
                        artifact: Some(artifact),
                    })
                }
            }
        }
    }
}

/// Judge `instance` against the retained schema at [`SchemaPhase::Completion`].
///
/// Pure and passive: no composition, coercion, expression or shell execution,
/// schema re-resolution, or filesystem work happens here. A document with no
/// retained schema is satisfied as long as its body evidence is.
///
/// ## Errors
///
/// Returns a typed composition error if the retained phase schema can no
/// longer be projected or compiled.
pub fn evaluate_completion(
    schema: Option<&LaunchSchema>,
    instance: &serde_json::Value,
    body: BodyEvidence,
    source_path: &Path,
) -> Result<CompletionVerdict, CompositionError> {
    let Some(effective) = schema.map(|launch| &launch.effective) else {
        return Ok(CompletionVerdict {
            source_path: source_path.to_path_buf(),
            body,
            problems: Vec::new(),
            status: None,
        });
    };

    let report = effective
        .validate_for_phase(instance, SchemaPhase::Completion)
        .map_err(|error| {
            schema_error_to_composition_error(
                source_path,
                error.to_string(),
                Some(&error),
            )
        })?;
    let status = status_report_from_validation(
        effective,
        Some(SchemaPhase::Completion),
        source_path,
        instance.as_object().cloned().unwrap_or_default(),
        &report,
    );
    let problems = declaration_ordered_problems(&report.problems, status.as_ref());

    Ok(CompletionVerdict {
        source_path: source_path.to_path_buf(),
        body,
        problems,
        status,
    })
}

/// Layer the agent's on-disk delta and the closure's owned values over the
/// live effective frontmatter (ruling 11).
///
/// Transient caller, sequence, and `proxy.with` inputs survive because the base
/// is the effective map, not the file; the agent's additions, replacements, and
/// deletions are still observable because the delta is applied on top.
fn inline_completion_instance(
    live_frontmatter: &serde_json::Value,
    artifact: &InlineArtifact,
) -> serde_json::Value {
    let mut map = live_frontmatter
        .as_object()
        .cloned()
        .unwrap_or_default();
    apply_delta(&mut map, &artifact.frontmatter_delta);

    // The restored owned values and the fresh stamps are what actually landed
    // on disk, so they — not the pre-run effective values — are what the schema
    // sees for `prompt`, `hash`, and `last_updated`.
    //
    // An owned property the file does not carry is *not* an absence: an inline
    // run's `prompt` may be a transient CLI or interactively collected value
    // that is deliberately never persisted (spec §D3/AC13). Removing it here
    // would fail completion for a run whose launch gate the same value
    // satisfied, so the live effective value stands.
    let written: Markdown = artifact.text.clone().into();
    let written_fm = written.frontmatter().as_map();
    for property in CLOSURE_OWNED_PROPERTIES {
        if let Some(value) = written_fm.get(*property) {
            map.insert((*property).to_string(), value.clone());
        }
    }

    serde_json::Value::Object(map)
}

fn apply_delta(
    map: &mut serde_json::Map<String, serde_json::Value>,
    delta: &FrontmatterDelta,
) {
    for entry in &delta.entries {
        match entry {
            FrontmatterDeltaEntry::Addition { property, value }
            | FrontmatterDeltaEntry::Replacement {
                property, value, ..
            } => {
                map.insert(property.clone(), value.clone());
            }
            FrontmatterDeltaEntry::Deletion { property, .. } => {
                map.remove(property);
            }
        }
    }
}

/// Project validation problems into declaration-ordered completion problems.
///
/// The status report already walks the schema's property table in declaration
/// order, so it supplies the ordering; nested and array pointers that have no
/// top-level entry are appended in validator order so nothing is lost.
fn declaration_ordered_problems(
    problems: &[ValidationProblem],
    status: Option<&SchemaStatusReport>,
) -> Vec<CompletionProblem> {
    let mut collected: Vec<CompletionProblem> = problems
        .iter()
        .map(|problem| CompletionProblem {
            property: problem_property(problem),
            message: problem.message.clone(),
            kind: match problem.kind {
                ValidationProblemKind::Missing => CompletionProblemKind::Missing,
                ValidationProblemKind::Type | ValidationProblemKind::Invalid => {
                    CompletionProblemKind::Invalid
                }
            },
        })
        .collect();
    collected.dedup_by(|a, b| a.property == b.property && a.message == b.message);

    let Some(order) = declaration_order(status) else {
        return collected;
    };
    collected.sort_by_key(|problem| {
        let root = problem
            .property
            .split(['.', '['])
            .next()
            .unwrap_or(problem.property.as_str());
        order
            .iter()
            .position(|name| name == root)
            .unwrap_or(usize::MAX)
    });
    collected
}

fn declaration_order(status: Option<&SchemaStatusReport>) -> Option<Vec<String>> {
    let status = status?;
    if status.raw_json_schema {
        return None;
    }
    Some(
        status
            .required
            .iter()
            .chain(status.optional.iter())
            .map(|entry| entry.name.clone())
            .collect(),
    )
}

/// The dotted property path a problem names.
///
/// A missing property's pointer addresses its *parent*, so the property name
/// is appended; every other pointer already addresses the failing value.
fn problem_property(problem: &ValidationProblem) -> String {
    let base = dotted_pointer(&problem.path);
    match (&problem.property, problem.kind) {
        (Some(name), ValidationProblemKind::Missing) if base.is_empty() => name.clone(),
        (Some(name), ValidationProblemKind::Missing) => format!("{base}.{name}"),
        _ if base.is_empty() => "<root>".to_string(),
        _ => base,
    }
}

/// Render a JSON pointer as the dotted/indexed form the status block shows
/// (`/products/2/uk_price` → `products[2].uk_price`).
fn dotted_pointer(pointer: &str) -> String {
    let mut out = String::new();
    for segment in pointer.trim_start_matches('/').split('/') {
        if segment.is_empty() {
            continue;
        }
        let decoded = segment.replace("~1", "/").replace("~0", "~");
        if decoded.chars().all(|c| c.is_ascii_digit()) && !out.is_empty() {
            out.push('[');
            out.push_str(&decoded);
            out.push(']');
        } else {
            if !out.is_empty() {
                out.push('.');
            }
            out.push_str(&decoded);
        }
    }
    out
}

#[cfg(test)]
mod tests;
