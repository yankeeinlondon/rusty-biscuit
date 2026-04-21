//! Snapshot-style coverage tests for every [`BlockError`] variant.
//!
//! This integration-test binary acts as the error-snapshot suite called for in
//! `darkmatter/features/2026-04-20-better-errors/plan.md` (Phase 4). Every
//! error enum is exercised by a dedicated module that renders each variant at
//! 80 columns using [`BlockError::report_block_error_optimistic`], strips
//! ANSI, and asserts the resulting prose contains the error-type name, the
//! human summary, and the variant-specific tokens that make the block
//! actionable (paths, commands, accepted values, CTAs, …).
//!
//! The renderer output is deterministic in optimistic mode so these tests act
//! as "self-documenting snapshots" — if a variant changes its prose, the
//! failing test points directly at the tokens that drifted.
//!
//! [`BlockError`]: biscuit_terminal::errors::BlockError
//! [`BlockError::report_block_error_optimistic`]: biscuit_terminal::errors::BlockError::report_block_error_optimistic

mod helpers;

mod condition;
mod ctx_merge;
mod deferred_set;
mod editor;
mod file_tree;
mod image_ref;
mod link;
mod markdown_error;
mod mermaid_theme;
mod normalization;
mod page_block;
mod reference;
mod shell_expansion;
mod stylesheet;
mod toc_linking;
mod transclusion;
