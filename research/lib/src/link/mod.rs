//! Link command implementation for the research CLI.
//!
//! This module provides functionality to create symbolic links from research topic
//! skill directories to Claude Code and OpenCode user-scoped skill locations.
//!
//! # Usage
//!
//! ```rust,no_run
//! use research_lib::link;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let filters = vec!["foo*".to_string()];
//!     let types = vec!["library".to_string()];
//!     let json = false;
//!
//!     let result = link(filters, types, json).await?;
//!     println!("Processed {} skills", result.total_processed());
//!     Ok(())
//! }
//! ```

pub mod creation;
pub mod detection;
pub mod types;

// Re-export main types for convenience
pub use types::{LinkError, LinkResult, SkillAction, SkillLink};

use tracing::instrument;

/// Create symbolic links from research topic skill directories to Claude Code
/// and OpenCode user-scoped skill locations.
///
/// This function discovers research topics, applies filters, and creates symlinks
/// for skills that don't already have them. It handles both Claude Code
/// (`~/.claude/skills/`) and OpenCode (`~/.config/opencode/skill/`) locations.
///
/// # Arguments
///
/// * `filters` - Glob patterns to filter topics (e.g., "foo", "foo*", "bar")
/// * `types` - Topic types to filter by (e.g., "library", "software")
/// * `json` - If true, output JSON format; otherwise use terminal format
///
/// # Returns
///
/// Returns a `LinkResult` containing all link operations and any errors encountered.
///
/// # Errors
///
/// Returns `LinkError` if:
/// - Topic discovery fails
/// - Filter application fails
/// - Home directory cannot be determined
/// - Critical I/O errors occur
///
/// Note: Individual symlink creation failures are captured in `LinkResult.errors`
/// and do not cause the entire operation to fail.
///
/// # Example
///
/// ```rust,no_run
/// # use research_lib::link;
/// # async fn example() -> Result<(), research_lib::link::LinkError> {
/// // Link all library topics
/// let result = link(vec![], vec!["library".to_string()], false).await?;
///
/// // Link topics matching "clap*"
/// let result = link(vec!["clap*".to_string()], vec![], false).await?;
/// # Ok(())
/// # }
/// ```
#[instrument(skip(filters, types), fields(filter_count = filters.len(), type_count = types.len()))]
pub async fn link(
    filters: Vec<String>,
    types: Vec<String>,
    json: bool,
) -> Result<LinkResult, LinkError> {
    tracing::info!(
        "Starting link command with {} filters and {} type filters",
        filters.len(),
        types.len()
    );

    // TODO: Phase 4 - Implement full orchestration
    // 1. Discover topics using list::discovery::discover_topics()
    // 2. Apply filters using list::filter::apply_filters()
    // 3. For each filtered topic:
    //    a. Validate source skill directory
    //    b. Check Claude Code and OpenCode locations
    //    c. Create symlinks where needed
    //    d. Collect results
    // 4. Format and output results (terminal or JSON)

    // Stub implementation - returns empty result
    let result = LinkResult::new();

    tracing::debug!(
        "Link command completed: {} processed, {} created",
        result.total_processed(),
        result.total_created()
    );

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_link_stub_returns_empty_result() {
        let result = link(vec![], vec![], false).await.unwrap();
        assert_eq!(result.total_processed(), 0);
        assert_eq!(result.total_created(), 0);
        assert!(!result.has_errors());
    }

    #[tokio::test]
    async fn test_link_stub_with_filters() {
        let filters = vec!["foo*".to_string()];
        let types = vec!["library".to_string()];
        let result = link(filters, types, false).await.unwrap();

        assert_eq!(result.total_processed(), 0);
    }

    #[tokio::test]
    async fn test_link_stub_json_mode() {
        let result = link(vec![], vec![], true).await.unwrap();
        assert_eq!(result.total_processed(), 0);
    }
}
