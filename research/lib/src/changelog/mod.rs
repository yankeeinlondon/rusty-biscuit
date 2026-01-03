//! Changelog generation infrastructure for research library.
//!
//! This module provides types and utilities for fetching, parsing, and aggregating
//! version history information from multiple sources (GitHub Releases, package registries,
//! changelog files, and LLM knowledge).
//!
//! ## Architecture
//!
//! The changelog system uses a **tiered information gathering** approach:
//!
//! - **Tier 1: Structured Sources** (Highest Confidence)
//!   - GitHub Releases API
//!   - Package registry version history (crates.io, npm, PyPI)
//!   - CHANGELOG.md/HISTORY.md files
//!
//! - **Tier 2: LLM Synthesis**
//!   - Enrich structured data with context
//!   - Generate from knowledge when no structured sources available
//!   - Produce minimal timeline as fallback
//!
//! ## Module Structure
//!
//! - [`types`]: Core data structures (VersionInfo, VersionHistory, errors)
//! - [`discovery`]: Changelog file discovery and parsing
//! - [`github`]: GitHub Releases API client
//! - [`registry`]: Package registry version fetchers (crates.io, npm, PyPI)
//! - [`aggregator`]: Source aggregation and deduplication
//!
//! ## Examples
//!
//! ```rust,no_run
//! use research_lib::changelog::types::{VersionHistory, ConfidenceLevel};
//!
//! let history = VersionHistory::default();
//! assert_eq!(history.confidence, ConfidenceLevel::Low);
//! ```

pub mod aggregator;
pub mod discovery;
pub mod github;
pub mod registry;
pub mod types;
