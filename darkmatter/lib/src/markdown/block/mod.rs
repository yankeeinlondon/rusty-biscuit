//! Block-level markdown extensions via iterator adapters.
//!
//! This module provides support for block-level markdown syntax that extends
//! beyond standard CommonMark, such as horizontal rules with attributes.

mod hr_builder;
mod rule_processor;

pub(crate) use hr_builder::{hr_alignment_to_string, hr_kind_to_string, hr_weight_to_string};
pub use rule_processor::{scan_inline_hr_warnings, try_parse_hr_attrs};
pub(crate) use rule_processor::{matches_horizontal_rule_pattern, parse_hr_attribute_block};
