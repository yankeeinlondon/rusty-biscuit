//! Validation utilities for research topics
//!
//! This module provides validation functionality for research topics, including:
//! - Frontmatter parsing and validation for SKILL.md files
//! - Comprehensive health checking for research topic completeness

pub mod frontmatter;
pub mod health;

// Re-export commonly used types
pub use frontmatter::{parse_and_validate_frontmatter, FrontmatterError, SkillFrontmatter};
pub use health::{research_health, ResearchHealth, ResearchType, ValidationError};
