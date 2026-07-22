//! Transclusion-phase support.
//!
//! This module provides parsing, resolution, condition evaluation, and content
//! formatting helpers used by the compose pipeline's transclusion-phase
//! engine.

mod code;
mod conditions;
mod engine;
mod parser;
mod resolver;
mod types;
mod wrappers;

pub use code::{ensure_vertical_spacing, generate_safe_fence, infer_language, wrap_in_code_block};
pub use conditions::evaluate_condition;
pub use engine::{find_preceding_heading_level, relevel_with_overflow};
pub(crate) use engine::{ApplyTarget, SectionSlot, TransclusionEngine, build_resolution_cache};
pub use parser::{parse_directives, parse_frontmatter_refs};
pub(crate) use resolver::{
    FrontmatterReference, classify_frontmatter_reference, resolve_parsed_target, resolve_path,
    resolve_target,
};
pub use types::{
    BlockDirective, BlockOptions, DeferredSetError, DependencyNode, DirectiveKind, FrontmatterRefs,
    ReplaceOption, ResolvedTarget, TransclusionError, TransclusionRuntime, TransclusionSource,
};
pub use wrappers::wrap_quotation;
