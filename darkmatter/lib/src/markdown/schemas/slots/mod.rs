//! Darkmatter's side of schema slots (`{{ … }}` inside a schema): passive
//! admission, evaluation on the expression engine, and the diagnostic codes.
//!
//! The language side (grammar, the pending template, substitution, the
//! [`SlotHost`](simplified_schema::slots::SlotHost) hook) is in the
//! `simplified-schema` crate; nothing there names a Darkmatter type.
//!
//! - [`admission`] decides passively which bindings and functions a slot may
//!   reach.
//! - [`evaluator`] runs one admitted slot in its defining file's context and
//!   reports what it depended on.
//! - [`grants`] holds the read grants, the user's grant store, and matching.
//! - [`host`] joins them as [`DarkmatterSlotHost`].
//! - [`session`] resolves a command's templates, pinning one snapshot per
//!   template and sharing results across commands through `cache`.
//! - [`codes`] holds the `dm.schema.slot_*` codes.

pub(crate) mod admission;
pub(crate) mod cache;
pub mod codes;
pub(crate) mod evaluator;
pub mod grants;
pub mod host;
pub mod session;

pub use codes::SLOT_DIAGNOSTIC_CODES;
pub use host::DarkmatterSlotHost;
pub use session::{SessionMode, SlotSession};

#[cfg(test)]
mod approval_tests;
#[cfg(test)]
mod tests;
