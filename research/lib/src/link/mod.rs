//! Link command implementation for the research CLI.
//!
//! This module provides functionality to create symbolic links from research topic
//! skill directories to Claude Code, OpenCode, and Roo Code user-scoped skill locations.
//!
//! # Usage
//!
//! ```rust,no_run
//! use research::link::link;
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

use std::path::Path;

use tracing::instrument;

/// Attempt to create a skill symlink for a single service.
fn create_service_skill_link(
    source_path: &Path,
    target: &Path,
    service_name: &str,
    topic_name: &str,
    errors: &mut Vec<(String, String)>,
) -> SkillAction {
    match creation::create_skill_symlink(source_path, target) {
        Ok(()) => {
            tracing::info!(
                "Created skill symlink for {} at {}",
                topic_name,
                service_name
            );
            SkillAction::CreatedLink
        }
        Err(creation::CreationError::InvalidSource(_)) => SkillAction::NoneSkillDirectoryInvalid,
        Err(creation::CreationError::SymlinkCreation(e))
            if e.kind() == std::io::ErrorKind::PermissionDenied =>
        {
            tracing::error!(
                "Permission denied creating skill symlink for {}: {}",
                topic_name,
                e
            );
            errors.push((
                topic_name.to_string(),
                format!("{} skill: {}", service_name, e),
            ));
            SkillAction::FailedPermissionDenied(e.to_string())
        }
        Err(e) => {
            tracing::error!("Failed to create skill symlink for {}: {}", topic_name, e);
            errors.push((
                topic_name.to_string(),
                format!("{} skill: {}", service_name, e),
            ));
            SkillAction::FailedOther(e.to_string())
        }
    }
}

/// Attempt to create a deep-dive doc symlink for a single service.
fn create_service_doc_link(
    source_path: &Path,
    target: &Path,
    service_name: &str,
    topic_name: &str,
    errors: &mut Vec<(String, String)>,
) -> SkillAction {
    if detection::check_is_symlink(target) {
        return SkillAction::NoneAlreadyLinked;
    }
    if detection::check_local_definition_exists(target) {
        return SkillAction::NoneLocalDefinition;
    }

    match creation::create_deep_dive_symlink(source_path, target) {
        Ok(()) => {
            tracing::info!(
                "Created deep dive symlink for {} at {}",
                topic_name,
                service_name
            );
            SkillAction::CreatedLink
        }
        Err(creation::CreationError::SymlinkCreation(e))
            if e.kind() == std::io::ErrorKind::PermissionDenied =>
        {
            tracing::error!(
                "Permission denied creating deep dive symlink for {}: {}",
                topic_name,
                e
            );
            errors.push((
                topic_name.to_string(),
                format!("{} doc: {}", service_name, e),
            ));
            SkillAction::FailedPermissionDenied(e.to_string())
        }
        Err(e) => {
            tracing::error!(
                "Failed to create deep dive symlink for {}: {}",
                topic_name,
                e
            );
            errors.push((
                topic_name.to_string(),
                format!("{} doc: {}", service_name, e),
            ));
            SkillAction::FailedOther(e.to_string())
        }
    }
}

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
/// use research::link::link;
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
    let claude_skills_dir = detection::get_claude_skills_dir()
        .map_err(|e| LinkError::HomeDirectory(format!("Claude skills: {}", e)))?;
    let opencode_skills_dir = detection::get_opencode_skills_dir()
        .map_err(|e| LinkError::HomeDirectory(format!("OpenCode skills: {}", e)))?;
    let roo_skills_dir = detection::get_roo_skills_dir()
        .map_err(|e| LinkError::HomeDirectory(format!("Roo skills: {}", e)))?;
    let claude_docs_dir = detection::get_claude_docs_dir()
        .map_err(|e| LinkError::HomeDirectory(format!("Claude docs: {}", e)))?;
    let opencode_docs_dir = detection::get_opencode_docs_dir()
        .map_err(|e| LinkError::HomeDirectory(format!("OpenCode docs: {}", e)))?;
    let roo_docs_dir = detection::get_roo_docs_dir()
        .map_err(|e| LinkError::HomeDirectory(format!("Roo docs: {}", e)))?;

    info!("Claude Code skills dir: {}", claude_skills_dir.display());
    info!("OpenCode skills dir: {}", opencode_skills_dir.display());
    info!("Roo Code skills dir: {}", roo_skills_dir.display());
    info!("Claude Code docs dir: {}", claude_docs_dir.display());
    info!("OpenCode docs dir: {}", opencode_docs_dir.display());
    info!("Roo Code docs dir: {}", roo_docs_dir.display());

    // 2. Scan and remove stale symlinks from all target directories
    let mut stale_removed = Vec::new();
    let mut stale_failed = Vec::new();

    for (dir, dir_name) in [
        (&claude_skills_dir, "Claude Code skills"),
        (&opencode_skills_dir, "OpenCode skills"),
        (&roo_skills_dir, "Roo Code skills"),
        (&claude_docs_dir, "Claude Code docs"),
        (&opencode_docs_dir, "OpenCode docs"),
        (&roo_docs_dir, "Roo Code docs"),
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
        let deep_dive_path = topic.location.join(format!("deep-dive/{}.md", topic.name));

        // Validate skill source (early filtering)
        let skill_source_valid = detection::validate_skill_source(&source_path);
        if !skill_source_valid {
            tracing::debug!(
                "Invalid skill source for {}: {}",
                topic.name,
                source_path.display()
            );
        }

        // Determine skill actions for all three services
        let (final_claude_action, final_opencode_action, final_roo_action) = if skill_source_valid {
            let services = [
                (&claude_skills_dir, "Claude Code"),
                (&opencode_skills_dir, "OpenCode"),
                (&roo_skills_dir, "Roo Code"),
            ];

            let actions: Vec<SkillAction> = services
                .iter()
                .map(|(dir, name)| {
                    let target = dir.join(&topic.name);
                    let action = detection::determine_action(&target, &source_path);
                    match action {
                        SkillAction::CreatedLink => create_service_skill_link(
                            &source_path,
                            &target,
                            name,
                            &topic.name,
                            &mut errors,
                        ),
                        other => other,
                    }
                })
                .collect();

            (actions[0].clone(), actions[1].clone(), actions[2].clone())
        } else {
            (
                SkillAction::NoneSkillDirectoryInvalid,
                SkillAction::NoneSkillDirectoryInvalid,
                SkillAction::NoneSkillDirectoryInvalid,
            )
        };

        // Process deep dive linking (use topic name as file name: {topic}.md)
        let (claude_doc_action, opencode_doc_action, roo_doc_action) = if deep_dive_path.exists() {
            let doc_dirs = [
                (&claude_docs_dir, "Claude Code"),
                (&opencode_docs_dir, "OpenCode"),
                (&roo_docs_dir, "Roo Code"),
            ];

            let doc_actions: Vec<SkillAction> = doc_dirs
                .iter()
                .map(|(dir, name)| {
                    let target = dir.join(format!("{}.md", topic.name));
                    create_service_doc_link(
                        &deep_dive_path,
                        &target,
                        name,
                        &topic.name,
                        &mut errors,
                    )
                })
                .collect();

            (
                Some(doc_actions[0].clone()),
                Some(doc_actions[1].clone()),
                Some(doc_actions[2].clone()),
            )
        } else {
            debug!(
                "No deep-dive/{}.md found for {}: {}",
                topic.name,
                topic.name,
                deep_dive_path.display()
            );
            (None, None, None)
        };

        links.push(SkillLink::new_with_docs(
            topic.name,
            final_claude_action,
            final_opencode_action,
            final_roo_action,
            claude_doc_action,
            opencode_doc_action,
            roo_doc_action,
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

    #[test]
    fn test_create_service_skill_link_success() {
        let temp = tempfile::TempDir::new().unwrap();
        let source = temp.path().join("source_skill");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(
            source.join("SKILL.md"),
            "---\nname: t\ndescription: t\n---\nBody",
        )
        .unwrap();

        let target = temp.path().join("target_dir").join("test-skill");
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();

        let mut errors = Vec::new();
        let action =
            create_service_skill_link(&source, &target, "TestService", "test-skill", &mut errors);

        assert_eq!(action, SkillAction::CreatedLink);
        assert!(target.exists() || target.is_symlink());
        assert!(errors.is_empty());
    }

    #[test]
    fn test_create_service_doc_link_already_linked() {
        let temp = tempfile::TempDir::new().unwrap();
        let source = temp.path().join("source.md");
        std::fs::write(&source, "content").unwrap();

        let target = temp.path().join("target.md");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&source, &target).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_file(&source, &target).unwrap();

        let mut errors = Vec::new();
        let action =
            create_service_doc_link(&source, &target, "TestService", "test-topic", &mut errors);

        assert_eq!(action, SkillAction::NoneAlreadyLinked);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_create_service_doc_link_local_definition() {
        let temp = tempfile::TempDir::new().unwrap();
        let source = temp.path().join("source.md");
        std::fs::write(&source, "content").unwrap();

        let target = temp.path().join("target.md");
        std::fs::write(&target, "local content").unwrap();

        let mut errors = Vec::new();
        let action =
            create_service_doc_link(&source, &target, "TestService", "test-topic", &mut errors);

        assert_eq!(action, SkillAction::NoneLocalDefinition);
    }
}
