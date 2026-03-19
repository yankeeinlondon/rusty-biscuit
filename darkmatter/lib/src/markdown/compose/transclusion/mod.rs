//! Stage 2 transclusion support.
//!
//! This module provides parsing, resolution, condition evaluation, and content
//! formatting helpers used by the compose pipeline's Stage 2 transclusion
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
pub use parser::{parse_directives, parse_frontmatter_refs};
pub use resolver::{
    is_file_like_reference, is_url_like, normalize_reference_token, resolve_path, resolve_target,
};
pub use types::{
    BlockDirective, BlockOptions, DependencyNode, DirectiveKind, FrontmatterRefs, ReplaceOption,
    ResolvedTarget, SourceContext, TransclusionError, TransclusionRuntime,
};
pub use wrappers::{wrap_disclosure, wrap_quotation};
