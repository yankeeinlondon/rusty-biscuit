//! Validation utilities for research topics
//!
//! This module provides validation functionality for research topics, including:
//! - Frontmatter parsing and validation for SKILL.md files
//! - Comprehensive health checking for research topic completeness

pub mod frontmatter;
pub mod health;

// Re-export commonly used types
pub use frontmatter::{FrontmatterError, SkillFrontmatter, parse_and_validate_frontmatter};
pub use health::{ResearchHealth, ResearchType, ValidationError, research_health};
