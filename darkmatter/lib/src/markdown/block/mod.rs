//! Block-level markdown extensions via iterator adapters.
//!
//! This module provides support for block-level markdown syntax that extends
//! beyond standard CommonMark, such as horizontal rules with attributes.

mod hr_builder;
mod rule_processor;

pub(crate) use hr_builder::{build_rule_with_defaults, hr_defaults_from_frontmatter};
pub use rule_processor::RuleProcessor;
