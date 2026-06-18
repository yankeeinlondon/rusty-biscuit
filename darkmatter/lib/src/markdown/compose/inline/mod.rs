//! Inline compose stages, lifted off `impl Markdown` as free functions.
//!
//! Each stage operates over `&mut Markdown` plus the shared compose state
//! (`EffectiveState` / `ComposeOptions` / runtime / report) and is dispatched by
//! [`ComposeOperation`](super::ComposeOperation) from the pipeline driver in
//! [`super::pipeline`]. Keeping the stages as free functions decouples them from
//! the document type and lets each be exercised in isolation.
//!
//! Only the stage *runners* live here; the heavier per-operation
//! implementations stay in their existing homes (`replacement`,
//! `interpolation/`, `page_blocks/`, `shell_expansion/`, the top-level
//! `normalize`) and are called through these thin stage functions.

pub(crate) mod interpolation;
pub(crate) mod normalize;
pub(crate) mod page_blocks;
pub(crate) mod replacement;
pub(crate) mod shell_expansion;
