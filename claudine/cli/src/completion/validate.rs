//! Mode-specific file validation for completion candidates.
//!
//! Phase 3 of the `2026-04-17-file-completion` feature implements:
//!
//! - a shared markdown-extension gate (`.md` / `.markdown`) and
//!   size/UTF-8 guard shared across `compose`, `inline-compose`, and
//!   `sequence`;
//! - `compose` validation as an extension-only filter (no frontmatter
//!   parsing);
//! - `inline-compose` validation requiring a non-empty string `prompt:`
//!   frontmatter value via `darkmatter::markdown::Markdown`;
//! - `sequence` validation via a temporary `ResolvedCompositionSource` +
//!   `resolve_sequence_plan(...)` check that matches the real parser.
//!
//! Phase 1 only reserves the module name. Validators land in Phase 3
//! once the classifier and walker from Phase 2 are in place.
