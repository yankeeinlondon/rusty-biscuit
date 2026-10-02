//! Content-triggered schema layering — foundational trigger-schema core.
//!
//! A trigger schema is a standalone YAML file claiming the
//! `kind: trigger-schema` envelope. Triggers live in the five
//! [schema roots](super::roots), activate by matching a document's parsed
//! frontmatter + path, and layer a payload schema between the caller
//! baseline and the document `$schema`.
//!
//! ## Phase scope
//!
//! - **Phase 1** — the match grammar ([`grammar`]), the trigger-schema
//!   envelope parser ([`envelope`]), the vacuous-trigger lint ([`lint`]), and
//!   the matcher ([`matcher`]). No I/O beyond judging one known path.
//! - **Phase 2** — trigger discovery in the schema roots and the
//!   per-document trigger registry ([`discovery`]). Adds I/O for filesystem
//!   scanning.
//!
//! Bare-name resolution (Phase 3) and effective-schema assembly (Phase 4)
//! build on top of these.
//!
//! See `darkmatter/features/2026-07-10-schema-triggers/spec.md`.

pub mod assemble;
pub mod discovery;
pub mod envelope;
pub mod grammar;
pub mod lint;
pub mod matcher;

pub use assemble::{
    TriggerArmTrace, TriggerEvaluation, TriggerTrace, TriggerTraceEntry, evaluate_registry,
    matched_triggers, trace_registry,
};
pub use discovery::{LoadedTrigger, ShadowedFile, TriggerRegistry, scan};
pub use envelope::{TriggerEnvelope, parse_trigger_envelope, parse_trigger_envelope_from_str};
pub use grammar::{
    COMBINATOR_KEYS, PATH_KEY, MatchArms, MatchExpr, PathGlobs, is_match_safe_constraint,
    parse_match_arms,
};
pub use matcher::{PathSubject, first_defeat, matches};
