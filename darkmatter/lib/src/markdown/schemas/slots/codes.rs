//! Diagnostic codes of the schema-slot family.
//!
//! A slot's own failures (its syntax, a forbidden binding or function, a
//! wrong result, an approval) use these. A failure of the expression a slot
//! runs keeps its own code: `dm.data.*` for a `load_data` failure and
//! `dm.iteration.*` for an exhausted work limit.

/// The slot's text is not an expression.
pub const SLOT_SYNTAX_CODE: &str = "dm.schema.slot_syntax";
/// The slot reads a binding a schema may not depend on (`fm`, a document
/// property, a host global, `ctx.cwd`, document identity, a per-run nonce).
pub const SLOT_FORBIDDEN_BINDING_CODE: &str = "dm.schema.slot_forbidden_binding";
/// The slot calls a function a slot may never run.
pub const SLOT_FORBIDDEN_FUNCTION_CODE: &str = "dm.schema.slot_forbidden_function";
/// The slot's value has the wrong shape for its position.
pub const SLOT_WRONG_SHAPE_CODE: &str = "dm.schema.slot_wrong_shape";
/// A list's slots produced no members.
pub const SLOT_EMPTY_CODE: &str = "dm.schema.slot_empty";
/// A slot-supplied pattern is not one the linear engine accepts.
pub const SLOT_PATTERN_REJECTED_CODE: &str = "dm.schema.slot_pattern_rejected";
/// The slot reads something no grant covers.
pub const SLOT_APPROVAL_REQUIRED_CODE: &str = "dm.schema.slot_approval_required";
/// The slot reads something a deny decision covers.
pub const SLOT_DENIED_CODE: &str = "dm.schema.slot_denied";
/// Resolving the slot needs the template it belongs to.
pub const SLOT_CYCLE_CODE: &str = "dm.schema.slot_cycle";
/// The schema holds slots that were not resolved for this run.
pub const SLOT_PENDING_CODE: &str = "dm.schema.slot_pending";
/// The slot's expression failed with no more specific code.
pub const SLOT_FAILED_CODE: &str = "dm.schema.slot_failed";
/// A newer request superseded the resolution.
pub const SLOT_CANCELED_CODE: &str = "dm.schema.slot_canceled";
/// A kind definition holds a slot.
pub const SLOT_NOT_ALLOWED_IN_KIND_CODE: &str = "dm.schema.slot_not_allowed_in_kind";
/// The baseline or an editor extension layer holds a slot.
pub const SLOT_NOT_ALLOWED_IN_BASELINE_CODE: &str = "dm.schema.slot_not_allowed_in_baseline";
/// The user's slot-read grant store could not be read.
pub const SLOT_GRANT_STORE_INVALID_CODE: &str = "dm.schema.slot_grant_store_invalid";

/// Every code of the family, for registries and conformance tests.
pub const SLOT_DIAGNOSTIC_CODES: &[&str] = &[
    SLOT_SYNTAX_CODE,
    SLOT_FORBIDDEN_BINDING_CODE,
    SLOT_FORBIDDEN_FUNCTION_CODE,
    SLOT_WRONG_SHAPE_CODE,
    SLOT_EMPTY_CODE,
    SLOT_PATTERN_REJECTED_CODE,
    SLOT_APPROVAL_REQUIRED_CODE,
    SLOT_DENIED_CODE,
    SLOT_CYCLE_CODE,
    SLOT_PENDING_CODE,
    SLOT_FAILED_CODE,
    SLOT_CANCELED_CODE,
    SLOT_NOT_ALLOWED_IN_KIND_CODE,
    SLOT_NOT_ALLOWED_IN_BASELINE_CODE,
    SLOT_GRANT_STORE_INVALID_CODE,
];

/// The code of a slot whose value the library could not use.
#[must_use]
pub(crate) fn slot_result_code(kind: &simplified_schema::SlotResultKind) -> &'static str {
    use simplified_schema::SlotResultKind;
    match kind {
        SlotResultKind::Empty => SLOT_EMPTY_CODE,
        SlotResultKind::PatternRejected(_) => SLOT_PATTERN_REJECTED_CODE,
        _ => SLOT_WRONG_SHAPE_CODE,
    }
}

/// The slot code of a schema error that refuses slots where none may be
/// written: [`SLOT_NOT_ALLOWED_IN_BASELINE_CODE`] for a baseline (or editor
/// extension layer) that holds slots, and the code a `kind` selection
/// carries for a kind definition that does
/// ([`SLOT_NOT_ALLOWED_IN_KIND_CODE`]). `None` for every other error.
#[must_use]
pub fn slot_prohibition_code(error: &simplified_schema::SchemaError) -> Option<&'static str> {
    use simplified_schema::{KindSelectionCause, SchemaError};
    match error {
        SchemaError::Baseline { source: Some(source), .. } if matches!(**source, SchemaError::UnresolvedSlots(_)) => {
            Some(SLOT_NOT_ALLOWED_IN_BASELINE_CODE)
        }
        SchemaError::KindSelection {
            cause: KindSelectionCause::Reserved { code, .. } | KindSelectionCause::Unavailable { code, .. },
            ..
        } => *code,
        _ => None,
    }
}

/// The message for slots found where `place` (a kind definition, the
/// baseline, an editor extension layer) may not hold any.
pub(crate) fn slots_not_allowed_message(place: &str, unresolved: &simplified_schema::UnresolvedSlots) -> String {
    let count = unresolved.slots.len();
    let mut message = format!(
        "{place} cannot hold `{{{{ … }}}}` slots; it holds {count} slot{}",
        if count == 1 { "" } else { "s" }
    );
    if let Some(first) = unresolved.slots.first() {
        message.push_str(&format!(" (first: `{{{{ {} }}}}`)", first.expression));
    }
    message
}

/// `error` as a baseline failure when it is a template: the baseline and
/// every editor extension layer apply to documents before any slot could be
/// resolved, so they never hold slots. `place` names the layer for the
/// message. Any other error is returned unchanged.
pub(crate) fn refuse_slots_in_baseline(place: &str, error: simplified_schema::SchemaError) -> simplified_schema::SchemaError {
    use simplified_schema::SchemaError;
    match error {
        SchemaError::UnresolvedSlots(unresolved) => SchemaError::Baseline {
            message: slots_not_allowed_message(place, &unresolved),
            source: Some(Box::new(SchemaError::UnresolvedSlots(unresolved))),
        },
        SchemaError::Baseline { source: Some(source), .. } if matches!(*source, SchemaError::UnresolvedSlots(_)) => {
            refuse_slots_in_baseline(place, *source)
        }
        other => other,
    }
}

/// `error` as a baseline failure when the editor extension layer at `path`
/// is a template: an extension is merged into the baseline, so it is refused
/// exactly as a baseline holding slots is ([`slot_prohibition_code`] gives
/// [`SLOT_NOT_ALLOWED_IN_BASELINE_CODE`]). Any other error is unchanged.
#[must_use]
pub fn refuse_slots_in_extension(path: &std::path::Path, error: simplified_schema::SchemaError) -> simplified_schema::SchemaError {
    refuse_slots_in_baseline(&format!("the schema extension `{}`", path.display()), error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_code_is_a_unique_three_segment_schema_code() {
        let mut seen = std::collections::HashSet::new();
        for code in SLOT_DIAGNOSTIC_CODES {
            let segments: Vec<_> = code.split('.').collect();
            assert_eq!(segments.len(), 3, "{code}");
            assert_eq!(&segments[..2], ["dm", "schema"], "{code}");
            assert!(segments[2].starts_with("slot_"), "{code}");
            assert!(seen.insert(code), "{code} repeats");
        }
        assert_eq!(SLOT_DIAGNOSTIC_CODES.len(), 15);
    }

    /// Only a prohibition carries a slot code: a template met where slots
    /// may be resolved (a `$schema` document) and every other error give none.
    #[test]
    fn only_a_prohibition_has_a_slot_code() {
        use simplified_schema::{KindSelectionCause, SchemaError, UnresolvedSlots};
        let template = || SchemaError::UnresolvedSlots(Box::new(UnresolvedSlots::new(Vec::new())));
        assert_eq!(slot_prohibition_code(&template()), None);
        assert_eq!(
            slot_prohibition_code(&refuse_slots_in_baseline("the baseline", template())),
            Some(SLOT_NOT_ALLOWED_IN_BASELINE_CODE)
        );
        let other = SchemaError::Baseline { message: "x".into(), source: None };
        assert_eq!(slot_prohibition_code(&other), None);
        assert!(matches!(refuse_slots_in_baseline("the baseline", other), SchemaError::Baseline { source: None, .. }));
        let selection = |code| SchemaError::KindSelection {
            kind: "k".into(),
            cause: KindSelectionCause::Reserved { file: "d.yaml".into(), defect: "x".into(), code },
        };
        assert_eq!(slot_prohibition_code(&selection(None)), None);
        assert_eq!(slot_prohibition_code(&selection(Some(SLOT_NOT_ALLOWED_IN_KIND_CODE))), Some(SLOT_NOT_ALLOWED_IN_KIND_CODE));
    }
}
