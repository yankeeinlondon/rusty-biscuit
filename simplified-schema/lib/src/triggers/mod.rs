//! Content-triggered schema layering — foundational schema-trigger core.
//!
//! A schema trigger is a standalone YAML file claiming the
//! `kind: schema-trigger` envelope. Triggers live in the five
//! [schema roots](crate::roots), activate by matching a document's parsed
//! frontmatter + path, and layer a payload schema between the caller
//! baseline and the document `$schema`.
//!
//! ## Phase scope
//!
//! - **Phase 1** — the match grammar ([`grammar`]), the schema-trigger
//!   envelope parser ([`envelope`]), the vacuous-trigger lint ([`lint`]), and
//!   the matcher ([`matcher`]). No I/O beyond judging one known path.
//! - **Phase 2** — trigger discovery in the schema roots and the
//!   per-document trigger registry ([`discovery`]). Adds I/O for filesystem
//!   scanning.
//!
//! Bare-name resolution (Phase 3) and effective-schema assembly (Phase 4)
//! build on top of these. A host may add an [`EmbeddedSchemaBundle`] as a
//! last-resort trigger location after the five roots.
//!
//! The language owns the trigger grammar, discovery, and matching; a host
//! supplies its reserved shipped namespace ([`ShippedSources`]) to every
//! entry point that resolves a payload, and layers the matched payloads into
//! its own effective schema (Darkmatter's `DarkmatterSchemas`). Discovery
//! reads the filesystem only through a [`SchemaRootSource`]; the
//! filesystem-backed source and the scans that use it need the `file`
//! feature.
//!
//! [`ShippedSources`]: crate::embedded::ShippedSources
//!
//! The user-facing contract is the "Repository Trigger Schemas" section of
//! `darkmatter/docs/topics/schemas/definition.md`.

pub mod assemble;
pub mod discovery;
pub mod envelope;
pub mod finding;
pub mod grammar;
pub mod identify;
pub mod lint;
pub mod matcher;
pub mod prepared;
#[cfg(all(test, feature = "file"))]
mod payload_memo_tests;

pub use assemble::{
    PayloadMemoCounts, PayloadMemoTally, TriggerConditionTrace, TriggerEvaluation, TriggerTrace, TriggerTraceEntry, evaluate_registry,
    matched_triggers, trace_registry,
};
pub use crate::embedded::EmbeddedSchemaBundle;
pub use discovery::{
    LoadedTrigger, SchemaRootSource, ShadowedFile, TriggerOrigin, TriggerRegistry, TriggerSource, scan_with_source,
};
#[cfg(feature = "file")]
pub use discovery::{enumerate_root_files, scan, scan_with_embedded};
pub use finding::InvalidTriggerFinding;
pub use envelope::{
    NAME_PATTERN, TriggerEnvelope, claims_trigger_envelope, is_valid_name, parse_trigger_envelope,
    parse_trigger_envelope_from_str, trigger_error_span,
};
pub use grammar::{
    COMBINATOR_KEYS, PATH_KEY, MatchConditions, MatchExpr, PathGlobs, is_match_safe_constraint,
    parse_match_conditions,
};
pub use identify::{IDENTIFY_TOO_BROAD_CODE, IdentifyTooBroad, IdentifyingTrigger, admit_identifying_trigger, identifies};
pub use matcher::{PathSubject, first_defeat, matches};
/// Code of the warning for a schema trigger automatic discovery skipped
/// because its envelope or payload failed to load.
pub const SCHEMA_INVALID_TRIGGER_CODE: &str = "dm.schema.invalid_trigger";

pub use prepared::{
    AdvisoryRecord, PayloadRecord, PreparedBundleDescriptor, PreparedTriggerBundle, TriggerBundleRecord,
    TriggerDocumentRecord, TriggerOutcomeRecord, prepare_trigger_bundle,
};
