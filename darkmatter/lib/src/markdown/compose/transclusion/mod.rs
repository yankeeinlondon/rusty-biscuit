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
pub(crate) use engine::{
    ApplyTarget, PreparedTransclusion, ResolvedTransclusion, SectionSlot, TransclusionEngine, apply_directive_set_overlay,
    build_resolution_cache, changed_before_read, created_before_read, ignores_invalid, inherited_child_options, markdown_child_options, remote_child_options, skips_unresolved,
};
pub use parser::{parse_directives, parse_frontmatter_refs};
pub use resolver::frontmatter_value_is_inline;
pub(crate) use parser::parse_directives_deferring;
pub(crate) use resolver::{
    FrontmatterReference, admit_local_target, admit_url_target, classify_frontmatter_reference, resolve_parsed_target, resolve_path,
    resolve_target, source_opening,
};
pub(crate) use types::{at_line, frontmatter_value_range, read_failure_at, target_failure_at};
pub use types::{
    BlockDirective, BlockOptions, DeferredSetError, DependencyNode, DirectiveKind, FrontmatterRefs,
    InclusionSite, ReplaceOption, ResolvedTarget, TransclusionError, TransclusionRuntime, TransclusionSource,
};
pub use wrappers::wrap_quotation;
