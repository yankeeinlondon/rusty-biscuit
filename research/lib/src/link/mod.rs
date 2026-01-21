//! Link command implementation for the research CLI.
//!
//! This module provides functionality to create symbolic links from research topic
//! skill directories to Claude Code and OpenCode user-scoped skill locations.
//!
//! # Usage
//!
//! ```rust,no_run
//! use research_lib::link::link;
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
pub mod format;
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
/// use research_lib::link::link;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Link all library topics
/// let result = link(vec![], vec!["library".to_string()], false).await?;
///
/// // Link topics matching "clap*"
/// let result = link(vec!["clap*".to_string()], vec![], false).await?;
/// # Ok(())
/// # }
/// ```
#[instrument(skip(filters, types), fields(filter_count = filters.len(), type_count = types.len(), json = json))]
pub async fn link(
    filters: Vec<String>,
    types: Vec<String>,
    json: bool,
) -> Result<LinkResult, LinkError> {
    use std::path::PathBuf;
    use tracing::{debug, error, info, warn};

    info!(
        "Starting link command with {} filters and {} type filters",
        filters.len(),
        types.len()
    );

    // Get RESEARCH_DIR from env (default to HOME)
    let research_dir = std::env::var("RESEARCH_DIR").unwrap_or_else(|_| {
        std::env::var("HOME").expect("Neither RESEARCH_DIR nor HOME environment variable is set")
    });

    // Construct library path: $RESEARCH_DIR/.research/library/
    let library_path = PathBuf::from(research_dir)
        .join(".research")
        .join("library");

    debug!("Searching for topics in: {:?}", library_path);

    // 1. Get all target directories for skills and docs
    let claude_skills_dir =
        detection::get_claude_skills_dir().map_err(|_| LinkError::HomeDirectory)?;
    let opencode_skills_dir =
        detection::get_opencode_skills_dir().map_err(|_| LinkError::HomeDirectory)?;
    let claude_docs_dir = detection::get_claude_docs_dir().map_err(|_| LinkError::HomeDirectory)?;
    let opencode_docs_dir =
        detection::get_opencode_docs_dir().map_err(|_| LinkError::HomeDirectory)?;

    info!("Claude Code skills dir: {}", claude_skills_dir.display());
    info!("OpenCode skills dir: {}", opencode_skills_dir.display());
    info!("Claude Code docs dir: {}", claude_docs_dir.display());
    info!("OpenCode docs dir: {}", opencode_docs_dir.display());

    // 2. Scan and remove stale symlinks from all target directories
    let mut stale_removed = Vec::new();
    let mut stale_failed = Vec::new();

    for (dir, dir_name) in [
        (&claude_skills_dir, "Claude Code skills"),
        (&opencode_skills_dir, "OpenCode skills"),
        (&claude_docs_dir, "Claude Code docs"),
        (&opencode_docs_dir, "OpenCode docs"),
    ] {
        let scan_result = detection::scan_and_remove_stale_symlinks(dir);
        for removed in scan_result.removed {
            let display_path = removed.display().to_string();
            warn!("Removed stale symlink in {}: {}", dir_name, display_path);
            eprintln!(
                "warning: removed stale symlink in {}: {}",
                dir_name, display_path
            );
            stale_removed.push(display_path);
        }
        for (path, err) in scan_result.failed {
            let display_path = path.display().to_string();
            error!(
                "Failed to remove stale symlink in {}: {}: {}",
                dir_name, display_path, err
            );
            eprintln!(
                "error: failed to remove stale symlink in {}: {}: {}",
                dir_name, display_path, err
            );
            stale_failed.push((display_path, err));
        }
    }

    // 3. Discover topics
    let all_topics =
        crate::list::discovery::discover_topics(library_path).map_err(LinkError::Discovery)?;

    info!("Discovered {} topics", all_topics.len());

    // 4. Filter topics
    let filtered_topics = crate::list::filter::apply_filters(all_topics, &filters, &types)
        .map_err(LinkError::Filter)?;

    info!("Filtered to {} topics", filtered_topics.len());

    // 5. Process each topic
    let mut links = Vec::new();
    let mut errors = Vec::new();

    for topic in filtered_topics {
        let source_path = topic.location.join("skill");
        let deep_dive_path = topic.location.join("deep_dive.md");

        // Validate skill source (early filtering)
        let skill_source_valid = detection::validate_skill_source(&source_path);
        if !skill_source_valid {
            tracing::debug!(
                "Invalid skill source for {}: {}",
                topic.name,
                source_path.display()
            );
        }

        // Determine skill actions for both services
        let (final_claude_action, final_opencode_action) = if skill_source_valid {
            let claude_target = claude_skills_dir.join(&topic.name);
            let claude_action = detection::determine_action(&claude_target, &source_path);

            let opencode_target = opencode_skills_dir.join(&topic.name);
            let opencode_action = detection::determine_action(&opencode_target, &source_path);

            // Attempt creation for both services (asymmetric failure handling)
            let final_claude_action = match claude_action {
                SkillAction::CreatedLink => {
                    match creation::create_skill_symlink(&source_path, &claude_target) {
                        Ok(()) => {
                            info!("Created skill symlink for {} at Claude Code", topic.name);
                            SkillAction::CreatedLink
                        }
                        Err(creation::CreationError::InvalidSource(_)) => {
                            SkillAction::NoneSkillDirectoryInvalid
                        }
                        Err(creation::CreationError::SymlinkCreation(e))
                            if e.kind() == std::io::ErrorKind::PermissionDenied =>
                        {
                            error!(
                                "Permission denied creating skill symlink for {}: {}",
                                topic.name, e
                            );
                            errors.push((topic.name.clone(), format!("Claude Code skill: {}", e)));
                            SkillAction::FailedPermissionDenied(e.to_string())
                        }
                        Err(e) => {
                            error!("Failed to create skill symlink for {}: {}", topic.name, e);
                            errors.push((topic.name.clone(), format!("Claude Code skill: {}", e)));
                            SkillAction::FailedOther(e.to_string())
                        }
                    }
                }
                other => other,
            };

            let final_opencode_action = match opencode_action {
                SkillAction::CreatedLink => {
                    match creation::create_skill_symlink(&source_path, &opencode_target) {
                        Ok(()) => {
                            info!("Created skill symlink for {} at OpenCode", topic.name);
                            SkillAction::CreatedLink
                        }
                        Err(creation::CreationError::InvalidSource(_)) => {
                            SkillAction::NoneSkillDirectoryInvalid
                        }
                        Err(creation::CreationError::SymlinkCreation(e))
                            if e.kind() == std::io::ErrorKind::PermissionDenied =>
                        {
                            error!(
                                "Permission denied creating skill symlink for {}: {}",
                                topic.name, e
                            );
                            errors.push((topic.name.clone(), format!("OpenCode skill: {}", e)));
                            SkillAction::FailedPermissionDenied(e.to_string())
                        }
                        Err(e) => {
                            error!("Failed to create skill symlink for {}: {}", topic.name, e);
                            errors.push((topic.name.clone(), format!("OpenCode skill: {}", e)));
                            SkillAction::FailedOther(e.to_string())
                        }
                    }
                }
                other => other,
            };

            (final_claude_action, final_opencode_action)
        } else {
            (
                SkillAction::NoneSkillDirectoryInvalid,
                SkillAction::NoneSkillDirectoryInvalid,
            )
        };

        // Process deep dive linking (use topic name as file name: {topic}.md)
        let (claude_doc_action, opencode_doc_action) = if deep_dive_path.exists() {
            let claude_doc_target = claude_docs_dir.join(format!("{}.md", topic.name));
            let opencode_doc_target = opencode_docs_dir.join(format!("{}.md", topic.name));

            // Claude Code deep dive
            let claude_doc_action = if detection::check_is_symlink(&claude_doc_target) {
                SkillAction::NoneAlreadyLinked
            } else if detection::check_local_definition_exists(&claude_doc_target) {
                SkillAction::NoneLocalDefinition
            } else {
                match creation::create_deep_dive_symlink(&deep_dive_path, &claude_doc_target) {
                    Ok(()) => {
                        info!(
                            "Created deep dive symlink for {} at Claude Code",
                            topic.name
                        );
                        SkillAction::CreatedLink
                    }
                    Err(creation::CreationError::SymlinkCreation(e))
                        if e.kind() == std::io::ErrorKind::PermissionDenied =>
                    {
                        error!(
                            "Permission denied creating deep dive symlink for {}: {}",
                            topic.name, e
                        );
                        errors.push((topic.name.clone(), format!("Claude Code doc: {}", e)));
                        SkillAction::FailedPermissionDenied(e.to_string())
                    }
                    Err(e) => {
                        error!(
                            "Failed to create deep dive symlink for {}: {}",
                            topic.name, e
                        );
                        errors.push((topic.name.clone(), format!("Claude Code doc: {}", e)));
                        SkillAction::FailedOther(e.to_string())
                    }
                }
            };

            // OpenCode deep dive
            let opencode_doc_action = if detection::check_is_symlink(&opencode_doc_target) {
                SkillAction::NoneAlreadyLinked
            } else if detection::check_local_definition_exists(&opencode_doc_target) {
                SkillAction::NoneLocalDefinition
            } else {
                match creation::create_deep_dive_symlink(&deep_dive_path, &opencode_doc_target) {
                    Ok(()) => {
                        info!("Created deep dive symlink for {} at OpenCode", topic.name);
                        SkillAction::CreatedLink
                    }
                    Err(creation::CreationError::SymlinkCreation(e))
                        if e.kind() == std::io::ErrorKind::PermissionDenied =>
                    {
                        error!(
                            "Permission denied creating deep dive symlink for {}: {}",
                            topic.name, e
                        );
                        errors.push((topic.name.clone(), format!("OpenCode doc: {}", e)));
                        SkillAction::FailedPermissionDenied(e.to_string())
                    }
                    Err(e) => {
                        error!(
                            "Failed to create deep dive symlink for {}: {}",
                            topic.name, e
                        );
                        errors.push((topic.name.clone(), format!("OpenCode doc: {}", e)));
                        SkillAction::FailedOther(e.to_string())
                    }
                }
            };

            (Some(claude_doc_action), Some(opencode_doc_action))
        } else {
            debug!(
                "No deep_dive.md found for {}: {}",
                topic.name,
                deep_dive_path.display()
            );
            (None, None)
        };

        links.push(SkillLink::new_with_docs(
            topic.name,
            final_claude_action,
            final_opencode_action,
            claude_doc_action,
            opencode_doc_action,
        ));
    }

    let result = LinkResult {
        links,
        errors,
        stale_removed,
        stale_failed,
    };

    info!(
        "Link command completed: {} processed, {} created, {} failed, {} stale removed",
        result.total_processed(),
        result.total_created(),
        result.total_failed(),
        result.stale_removed.len()
    );

    // Format output
    if json {
        let output =
            format::format_json(&result).map_err(|e| LinkError::Io(std::io::Error::other(e)))?;
        println!("{}", output);
    } else {
        let output = format::format_terminal(&result);
        println!("{}", output);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_link_basic_functionality() {
        // Set RESEARCH_DIR to a directory that exists (current directory has .research/library)
        // This test just verifies the function runs without error
        let result = link(vec![], vec![], false).await;

        // Should succeed or fail gracefully
        assert!(result.is_ok() || matches!(result, Err(LinkError::Discovery(_))));
    }

    #[tokio::test]
    async fn test_link_with_filters() {
        let filters = vec!["nonexistent*".to_string()];
        let types = vec!["library".to_string()];
        let result = link(filters, types, false).await;

        // Should succeed or fail gracefully
        assert!(result.is_ok() || matches!(result, Err(LinkError::Discovery(_))));
    }

    #[tokio::test]
    async fn test_link_json_mode() {
        let result = link(vec![], vec![], true).await;

        // Should succeed or fail gracefully
        assert!(result.is_ok() || matches!(result, Err(LinkError::Discovery(_))));
    }
}
