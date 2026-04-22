//! Block-level markdown extensions via iterator adapters.
//!
//! This module provides support for block-level markdown syntax that extends
//! beyond standard CommonMark, such as horizontal rules with attributes.

mod rule_processor;

pub use rule_processor::RuleProcessor;