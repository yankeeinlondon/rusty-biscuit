//! Validation module for SKILL.md files
//!
//! This module provides validation for SKILL.md files, including:
//! - Frontmatter parsing and validation
//! - Required field checking
//! - YAML structure validation

pub mod frontmatter;

pub use frontmatter::{
    FrontmatterError, SkillFrontmatter, extract_frontmatter, parse_and_validate_frontmatter,
};
