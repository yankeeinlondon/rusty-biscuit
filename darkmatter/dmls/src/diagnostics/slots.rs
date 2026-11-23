//! Diagnostics of a template schema, projected from the slot worker's last
//! resolution (see [`crate::slot_jobs`]).
//!
//! Each slot's record is the library's own
//! ([`report_diagnostics::template_slot_records`]), so its code, message, and
//! typed cause match `md compose`, and so is its position: a slot written in
//! this document is ranged at the span the library proved in this text; a
//! slot in another schema file (or one whose resolution read an earlier
//! revision of this buffer) is ranged at this document's `$schema` value (or
//! the frontmatter block, for a trigger payload) and points at the slot with
//! related information.
//!
//! | Code | Severity | When |
//! | --- | --- | --- |
//! | `dm.schema.slot_pending` | `INFORMATION` | No resolution serves the template yet, or what is served is being refreshed. |
//! | `dm.schema.slot_approval_required` | `WARNING` | A read no grant covers; quick fixes grant it. |
//! | any other slot or `dm.data.*` code | `ERROR` | The slot failed and its schema is unavailable. |
//!
//! A template that did not resolve is never validated against: the
//! frontmatter gets these records instead of a verdict, so an unconstrained
//! schema is never reported valid.

use std::path::Path;

use darkmatter::markdown::compose::report_diagnostics;
use darkmatter::markdown::schemas::slots::codes::{SLOT_APPROVAL_REQUIRED_CODE, SLOT_PENDING_CODE};
use lsp_types::{Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity, Location, Position, Range};
use serde::{Deserialize, Serialize};
use simplified_schema::SchemaError;

use crate::diagnostics::codes::source;
use crate::diagnostics::frontmatter::diagnostic;
use crate::overlay::{FrontmatterAst, SlotStatus};
use crate::providers::DocumentContext;
use crate::workspace::file_path_to_uri;

/// The command a slot-read quick fix runs: grants (or denies) one read.
pub const GRANT_SLOT_READ_COMMAND: &str = "dmls.grantSlotRead";

/// The command that re-resolves every open document's schema slots.
pub const REFRESH_SCHEMA_SLOTS_COMMAND: &str = "dmls.refreshSchemaSlots";

/// One approval a quick fix offers, carried in an approval-required
/// diagnostic's `data.choices` and passed back as the
/// [`GRANT_SLOT_READ_COMMAND`] argument.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotGrantChoice {
    /// `allow` or `deny`.
    pub decision: String,
    /// `file` or `url`.
    pub kind: String,
    /// The file's path or the exact URL.
    pub target: String,
    /// `once` (this server process) or `persisted` ("always allow" a file:
    /// saved to the user's `permissions.file-read`).
    pub scope: String,
}

impl SlotGrantChoice {
    /// The quick fix's title.
    pub fn title(&self) -> String {
        let what = match self.kind.as_str() {
            "file" => "file",
            "dir" => "folder",
            "url" | "url-any-query" => "URL",
            _ => "site",
        };
        let any_query = if self.kind == "url-any-query" { " (any query)" } else { "" };
        match (self.decision.as_str(), self.scope.as_str()) {
            ("allow", "persisted") => format!("Always allow schema slots to read {what} {}{any_query}", self.target),
            ("allow", _) => format!("Allow schema slots to read {what} {}{any_query} for this session", self.target),
            _ => format!("Deny schema slots reading {what} {}{any_query} for this session", self.target),
        }
    }
}

/// The `data` of an approval-required diagnostic.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SlotDiagnosticData {
    /// Every grant a quick fix offers, in the order a prompt lists them.
    pub choices: Vec<SlotGrantChoice>,
}

/// Publishes the records of a template whose schema did not assemble:
/// `error` is a slot failure (see [`crate::slot_jobs::is_slot_failure`]).
pub(crate) fn failure_diagnostics(
    ctx: &DocumentContext,
    ast: Option<&FrontmatterAst>,
    error: &SchemaError,
    status: Option<&SlotStatus>,
    out: &mut Vec<Diagnostic>,
) {
    let Some(slot_error) = crate::slot_jobs::slot_error(error) else {
        return;
    };
    let fallback = fallback_range(ctx, ast);
    let records = report_diagnostics::template_slot_records(slot_error).unwrap_or_default();
    let pending_only = matches!(slot_error, SchemaError::UnresolvedSlots(_));
    for (slot, record) in records {
        let local = slot.as_ref().filter(|slot| slot_is_local(ctx, slot.origin.as_deref()));
        let range = local
            .and_then(|slot| local_span(ctx, slot_error, slot, &record))
            .and_then(|span| ctx.source_map.byte_range_to_lsp(span))
            .unwrap_or(fallback);
        let severity = match record.code.as_str() {
            SLOT_PENDING_CODE => DiagnosticSeverity::INFORMATION,
            SLOT_APPROVAL_REQUIRED_CODE => DiagnosticSeverity::WARNING,
            _ => DiagnosticSeverity::ERROR,
        };
        let message = if pending_only {
            "the schema's slots have not been resolved yet; the frontmatter is checked once they are".to_string()
        } else {
            record.message.clone()
        };
        let mut published = diagnostic(range, severity, source::SCHEMA, &record.code, message);
        if local.is_none()
            && let Some(origin) = slot.as_ref().and_then(|slot| slot.origin.as_deref())
            && let Some(uri) = file_path_to_uri(origin)
        {
            let at = slot
                .as_ref()
                .and_then(|slot| related_position(ctx, slot_error, slot, &record, &uri))
                .unwrap_or_default();
            published.related_information = Some(vec![DiagnosticRelatedInformation {
                location: Location::new(uri, Range::new(at, at)),
                message: "the slot".to_string(),
            }]);
        }
        if record.code == SLOT_APPROVAL_REQUIRED_CODE {
            let choices = grant_choices(record.grants_needed.as_deref().unwrap_or_default());
            if !choices.is_empty() {
                published.data = serde_json::to_value(SlotDiagnosticData { choices }).ok();
            }
        }
        if pending_only && out.iter().any(|known| known.range == published.range && known.code == published.code) {
            continue;
        }
        out.push(published);
    }
    if !pending_only && status.is_some_and(|status| status.refreshing) {
        out.push(refreshing_diagnostic(fallback));
    }
}

/// Publishes the stale marker of a template served from a snapshot that is
/// being refreshed: the frontmatter's verdict is the last resolution's, not
/// a current one.
pub(crate) fn served_diagnostics(ctx: &DocumentContext, ast: Option<&FrontmatterAst>, status: &SlotStatus, out: &mut Vec<Diagnostic>) {
    if status.refreshing {
        out.push(refreshing_diagnostic(fallback_range(ctx, ast)));
    }
}

fn refreshing_diagnostic(range: Range) -> Diagnostic {
    diagnostic(
        range,
        DiagnosticSeverity::INFORMATION,
        source::SCHEMA,
        SLOT_PENDING_CODE,
        "the schema's slots are being refreshed; these results come from their last resolution".to_string(),
    )
}

/// The range of `record`, the record of `slot` written in this document,
/// as the library positioned it: only when the resolution it came from read
/// this very text, so a record left from an earlier revision of the buffer
/// is never placed at bytes it no longer names.
fn local_span(
    ctx: &DocumentContext,
    error: &SchemaError,
    slot: &simplified_schema::simplified::Slot,
    record: &darkmatter::markdown::compose::expression::diagnostics::Diagnostic,
) -> Option<std::ops::Range<usize>> {
    let (_, text) = report_diagnostics::template_slot_text(error, slot)?;
    (*text == *ctx.text).then(|| record.span.clone()).flatten()
}

/// Where `record` is in the other file `uri` its slot was written in: its
/// span in the text the resolution kept, counted in the client's encoding
/// (the record's column is not), or the start of its line when no kept
/// text proves the span.
fn related_position(
    ctx: &DocumentContext,
    error: &SchemaError,
    slot: &simplified_schema::simplified::Slot,
    record: &darkmatter::markdown::compose::expression::diagnostics::Diagnostic,
    uri: &lsp_types::Uri,
) -> Option<Position> {
    let kept = report_diagnostics::template_slot_text(error, slot)
        .map(|(_, text)| text)
        .zip(record.span.as_ref())
        .and_then(|(text, span)| {
            let map = crate::source_map::SourceMap::new(uri.clone(), 0, ctx.source_map.encoding(), text);
            map.byte_to_lsp(span.start)
        });
    kept.or_else(|| {
        let line = record.source.as_ref()?.line?;
        Some(Position::new(u32::try_from(line.saturating_sub(1)).unwrap_or(u32::MAX), 0))
    })
}

/// The `$schema` value, else the frontmatter block, else the document start.
fn fallback_range(ctx: &DocumentContext, ast: Option<&FrontmatterAst>) -> Range {
    ast.and_then(|ast| {
        crate::overlay::named_types::frontmatter_value_span(ctx.text, ast).or_else(|| Some(ast.block_span()))
    })
    .and_then(|span| ctx.source_map.byte_range_to_lsp(span))
    .unwrap_or_default()
}

/// Whether a slot written at `origin` is in this document: an inline
/// `$schema` (no origin) or this very file.
fn slot_is_local(ctx: &DocumentContext, origin: Option<&Path>) -> bool {
    match origin {
        None => true,
        Some(origin) => {
            origin == ctx.path
                || biscuit_file::canonicalize_simplified(origin).ok() == biscuit_file::canonicalize_simplified(ctx.path).ok()
        }
    }
}

/// The quick fixes for an unapproved read: every allow the library offers
/// (the file or URL for this session and, for a file with valid
/// permissions, "always allow"), then a deny of it for this session. There
/// is no persistent deny.
fn grant_choices(needed: &[darkmatter::markdown::compose::expression::diagnostics::GrantNeeded]) -> Vec<SlotGrantChoice> {
    let mut choices: Vec<SlotGrantChoice> = needed
        .iter()
        .map(|grant| SlotGrantChoice {
            decision: "allow".to_string(),
            kind: grant.kind.to_string(),
            target: grant.target.clone(),
            scope: grant.scope.to_string(),
        })
        .collect();
    if let Some(first) = needed.first() {
        choices.push(SlotGrantChoice {
            decision: "deny".to_string(),
            kind: first.kind.to_string(),
            target: first.target.clone(),
            scope: "once".to_string(),
        });
    }
    choices
}
