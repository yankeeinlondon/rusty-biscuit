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
}
