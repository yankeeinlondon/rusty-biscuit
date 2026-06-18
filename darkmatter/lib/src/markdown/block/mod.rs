//! Block-level markdown extensions beyond standard CommonMark.
//!
//! Currently this covers horizontal rules with attributes: `hr_parser` is the
//! single source of truth for parsing `--- { kind: waves }` directives (and the
//! strict-style preflight), and `hr_builder` maps the typed `style.hr` schema
//! enums back to the canonical strings the render-tree HR hints carry.

mod hr_builder;
mod hr_parser;

pub(crate) use hr_builder::{hr_alignment_to_string, hr_kind_to_string, hr_weight_to_string};
pub use hr_parser::{scan_inline_hr_warnings, try_parse_hr_attrs};
pub(crate) use hr_parser::{matches_horizontal_rule_pattern, parse_hr_attribute_block};
