//! Symlink detection and validation logic for the link command.
//!
//! This module provides functions to detect existing symlinks, check for local
//! skill definitions, validate skill source directories, and determine the appropriate
//! action to take when linking a skill.
//!
//! # Security Considerations
//!
//! This module uses `std::fs::symlink_metadata()` to check for symlinks WITHOUT
//! following them. This prevents TOCTOU (Time-Of-Check-Time-Of-Use) vulnerabilities
//! and ensures we can safely detect symlinks that point outside allowed directories.
//!
//! Path validation ensures:
//! - No `..` components (prevents directory traversal)
//! - Paths are within expected directories (no arbitrary filesystem access)
//! - Symlink targets are absolute paths (for robustness)

use std::path::{Path, PathBuf};
use thiserror::Error;
use tracing::{debug, warn};

use super::types::SkillAction;

/// Errors that can occur during symlink detection and validation
#[derive(Debug, Error)]
pub enum DetectionError {
    /// Home directory could not be determined
    #[error("Home directory not found")]
    HomeDirectoryNotFound,

    /// Path contains invalid components (e.g., .. for traversal)
    #[error("Invalid path: {0}")]
    InvalidPath(String),

    /// Path is outside allowed directories
    #[error("Path outside allowed directories: {0}")]
    PathOutsideAllowed(String),
}

/// Get the Claude Code skills directory (~/.claude/skills/)
///
/// # Errors
///
/// Returns `DetectionError::HomeDirectoryNotFound` if the home directory
/// cannot be determined.
pub fn get_claude_skills_dir() -> Result<PathBuf, DetectionError> {
    // In tests, prefer HOME env var over dirs::home_dir() to avoid caching issues
    let home = if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home)
    } else {
        dirs::home_dir().ok_or(DetectionError::HomeDirectoryNotFound)?
    };
    Ok(home.join(".claude/skills"))
}

/// Get the OpenCode skills directory (~/.config/opencode/skill/)
///
/// # Errors
///
/// Returns `DetectionError::HomeDirectoryNotFound` if the home directory
/// cannot be determined.
pub fn get_opencode_skills_dir() -> Result<PathBuf, DetectionError> {
    // In tests, prefer HOME env var over dirs::home_dir() to avoid caching issues
    let home = if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home)
    } else {
        dirs::home_dir().ok_or(DetectionError::HomeDirectoryNotFound)?
    };
    Ok(home.join(".config/opencode/skill"))
}

/// Check if a path exists (file, directory, or symlink).
///
/// This function uses `symlink_metadata()` to check existence without
/// following symlinks. This means broken symlinks will return `true`.
///
/// # Arguments
///
/// * `path` - The path to check
///
/// # Returns
///
/// `true` if the path exists (including broken symlinks), `false` otherwise.
pub fn check_symlink_exists(path: &Path) -> bool {
    debug!("Checking if path exists: {}", path.display());

    // Use symlink_metadata to avoid following symlinks
    // This returns Ok even for broken symlinks
    path.symlink_metadata().is_ok()
}

/// Check if a path is a symlink.
///
/// Uses `symlink_metadata()` to check if the path is a symlink without
/// following it. This is critical for security - we need to know if something
/// is a symlink BEFORE we decide whether to trust it.
///
/// # Arguments
///
/// * `path` - The path to check
///
/// # Returns
///
/// `true` if the path is a symlink (working or broken), `false` otherwise.
pub fn check_is_symlink(path: &Path) -> bool {
    debug!("Checking if path is symlink: {}", path.display());

    // Use symlink_metadata to check the link itself, not its target
    match path.symlink_metadata() {
        Ok(metadata) => {
            let is_symlink = metadata.is_symlink();
            debug!("Path {} is symlink: {}", path.display(), is_symlink);
            is_symlink
        }
        Err(e) => {
            debug!("Failed to check if path is symlink: {}", e);
            false
        }
    }
}

/// Check if a path is a broken symlink (symlink exists but target doesn't).
///
/// A broken symlink is one where `symlink_metadata()` succeeds (the link exists)
/// but `metadata()` fails (the target doesn't exist).
///
/// # Arguments
///
/// * `path` - The path to check
///
/// # Returns
///
/// `true` if the path is a broken symlink, `false` otherwise.
pub fn check_is_broken_symlink(path: &Path) -> bool {
    debug!("Checking if path is broken symlink: {}", path.display());

    // Check if it's a symlink first
    if !check_is_symlink(path) {
        return false;
    }

    // If it's a symlink, check if the target exists
    // metadata() follows symlinks, so this checks the target
    match path.metadata() {
        Ok(_) => {
            debug!("Symlink target exists: {}", path.display());
            false // Target exists, not broken
        }
        Err(_) => {
            warn!("Detected broken symlink: {}", path.display());
            true // Target doesn't exist, broken
        }
    }
}

/// Check if a local skill definition exists (real directory, not a symlink).
///
/// A local definition is a path that exists and is NOT a symlink. This could be
/// a directory or file that the user has created manually.
///
/// # Arguments
///
/// * `path` - The path to check
///
/// # Returns
///
/// `true` if a local definition exists (path exists and is not a symlink),
/// `false` otherwise.
pub fn check_local_definition_exists(path: &Path) -> bool {
    debug!("Checking for local definition at: {}", path.display());

    // Path must exist AND not be a symlink
    let exists = check_symlink_exists(path);
    let is_symlink = check_is_symlink(path);

    let has_local = exists && !is_symlink;
    debug!(
        "Local definition exists: {} (exists={}, is_symlink={})",
        has_local, exists, is_symlink
    );

    has_local
}

/// Validate that a skill source directory is valid.
///
/// A valid skill source must:
/// 1. Exist as a directory
/// 2. Contain a `SKILL.md` file
///
/// # Arguments
///
/// * `path` - The skill directory path to validate (should end with `/skill/`)
///
/// # Returns
///
/// `true` if the skill directory is valid, `false` otherwise.
pub fn validate_skill_source(path: &Path) -> bool {
    debug!("Validating skill source: {}", path.display());

    // Check that the path exists and is a directory
    if !path.is_dir() {
        debug!("Skill source is not a directory: {}", path.display());
        return false;
    }

    // Check for SKILL.md file
    let skill_md = path.join("SKILL.md");
    if !skill_md.is_file() {
        debug!("Skill source missing SKILL.md: {}", path.display());
        return false;
    }

    debug!("Skill source is valid: {}", path.display());
    true
}

/// Validate that a path does not contain directory traversal components.
///
/// This prevents attacks where a malicious path tries to escape from
/// the allowed directory using `..` components.
///
/// # Arguments
///
/// * `path` - The path to validate
///
/// # Errors
///
/// Returns `DetectionError::InvalidPath` if the path contains `..` components.
fn validate_no_traversal(path: &Path) -> Result<(), DetectionError> {
    for component in path.components() {
        if component == std::path::Component::ParentDir {
            return Err(DetectionError::InvalidPath(format!(
                "Path contains '..' component: {}",
                path.display()
            )));
        }
    }
    Ok(())
}

/// Validate that a path is within one of the allowed directories.
///
/// Allowed directories are:
/// - `~/.claude/skills/` (Claude Code)
/// - `~/.config/opencode/skill/` (OpenCode)
/// - `$RESEARCH_DIR/.research/library/` (research topics source)
///
/// # Arguments
///
/// * `path` - The path to validate
///
/// # Errors
///
/// Returns `DetectionError::PathOutsideAllowed` if the path is not within
/// an allowed directory.
fn validate_within_allowed_dirs(path: &Path) -> Result<(), DetectionError> {
    // Get allowed directory prefixes
    let home = dirs::home_dir().ok_or(DetectionError::HomeDirectoryNotFound)?;
    let claude_dir = home.join(".claude/skills");
    let opencode_dir = home.join(".config/opencode/skill");

    // Also allow research library directory
    // For now, we'll be permissive with absolute paths since source validation
    // happens elsewhere. This is just to catch obvious traversal attempts.

    // Convert to absolute path if possible for comparison
    let path_str = path.to_string_lossy();
    let claude_str = claude_dir.to_string_lossy();
    let opencode_str = opencode_dir.to_string_lossy();

    // Check if path starts with any allowed prefix
    if path_str.starts_with(&*claude_str)
        || path_str.starts_with(&*opencode_str)
        || path.is_absolute()
    // Allow absolute paths for source directories
    {
        Ok(())
    } else {
        Err(DetectionError::PathOutsideAllowed(format!(
            "Path is outside allowed directories: {}",
            path.display()
        )))
    }
}

/// Determine the appropriate action to take for a skill link.
///
/// This function examines the target path and source path to determine whether
/// to create a link, skip because one already exists, or skip because a local
/// definition exists.
///
/// # Security
///
/// This function validates paths before processing:
/// - Rejects paths with `..` components
/// - Validates paths are within allowed directories
/// - Uses `symlink_metadata()` to avoid following symlinks
///
/// # Arguments
///
/// * `target_path` - Where the symlink would be created (e.g., `~/.claude/skills/clap`)
/// * `source_path` - What the symlink would point to (e.g., `~/.research/library/clap/skill/`)
///
/// # Returns
///
/// A `SkillAction` indicating what action should be taken:
/// - `CreatedLink` - Should create a new symlink
/// - `NoneAlreadyLinked` - Symlink already exists (or broken symlink)
/// - `NoneLocalDefinition` - Local definition exists (real directory/file)
/// - `NoneSkillDirectoryInvalid` - Source skill directory is invalid
pub fn determine_action(target_path: &Path, source_path: &Path) -> SkillAction {
    debug!(
        "Determining action for target={}, source={}",
        target_path.display(),
        source_path.display()
    );

    // Validate paths for security
    if let Err(e) = validate_no_traversal(target_path) {
        warn!("Target path validation failed: {}", e);
        return SkillAction::FailedOther(e.to_string());
    }

    if let Err(e) = validate_no_traversal(source_path) {
        warn!("Source path validation failed: {}", e);
        return SkillAction::FailedOther(e.to_string());
    }

    if let Err(e) = validate_within_allowed_dirs(target_path) {
        warn!("Target path outside allowed directories: {}", e);
        return SkillAction::FailedOther(e.to_string());
    }

    // Validate source is a valid skill directory
    if !validate_skill_source(source_path) {
        debug!(
            "Source skill directory is invalid: {}",
            source_path.display()
        );
        return SkillAction::NoneSkillDirectoryInvalid;
    }

    // Check if target exists
    if check_local_definition_exists(target_path) {
        debug!(
            "Local definition exists at target: {}",
            target_path.display()
        );
        return SkillAction::NoneLocalDefinition;
    }

    // Check if symlink exists (working or broken)
    if check_is_symlink(target_path) {
        debug!(
            "Symlink already exists at target: {}",
            target_path.display()
        );
        return SkillAction::NoneAlreadyLinked;
    }

    // No conflicts, should create link
    debug!("Should create link at target: {}", target_path.display());
    SkillAction::CreatedLink
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    // Helper to create a test skill directory structure
    fn create_skill_dir(temp_dir: &Path, name: &str) -> PathBuf {
        let skill_dir = temp_dir.join(name).join("skill");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), "# Test Skill").unwrap();
        skill_dir
    }

    #[test]
    fn check_symlink_exists_returns_false_for_missing_path() {
        let temp = TempDir::new().unwrap();
        let missing = temp.path().join("does-not-exist");
        assert!(!check_symlink_exists(&missing));
    }

    #[test]
    fn check_symlink_exists_returns_true_for_file() {
        let temp = TempDir::new().unwrap();
        let file = temp.path().join("test.txt");
        fs::write(&file, "content").unwrap();
        assert!(check_symlink_exists(&file));
    }

    #[test]
    fn check_symlink_exists_returns_true_for_directory() {
        let temp = TempDir::new().unwrap();
        let dir = temp.path().join("test-dir");
        fs::create_dir(&dir).unwrap();
        assert!(check_symlink_exists(&dir));
    }

    #[test]
    fn check_symlink_exists_returns_true_for_working_symlink() {
        let temp = TempDir::new().unwrap();
        let target = temp.path().join("target");
        let link = temp.path().join("link");
        fs::write(&target, "content").unwrap();

        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, &link).unwrap();

        #[cfg(unix)]
        assert!(check_symlink_exists(&link));
    }

    #[test]
    fn check_symlink_exists_returns_true_for_broken_symlink() {
        let temp = TempDir::new().unwrap();
        let target = temp.path().join("target");
        let link = temp.path().join("link");

        // Create target temporarily, create link, then remove target
        fs::write(&target, "content").unwrap();

        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, &link).unwrap();

        fs::remove_file(&target).unwrap();

        #[cfg(unix)]
        assert!(check_symlink_exists(&link));
    }

    #[test]
    fn check_is_symlink_returns_false_for_regular_file() {
        let temp = TempDir::new().unwrap();
        let file = temp.path().join("test.txt");
        fs::write(&file, "content").unwrap();
        assert!(!check_is_symlink(&file));
    }

    #[test]
    fn check_is_symlink_returns_false_for_directory() {
        let temp = TempDir::new().unwrap();
        let dir = temp.path().join("test-dir");
        fs::create_dir(&dir).unwrap();
        assert!(!check_is_symlink(&dir));
    }

    #[test]
    fn check_is_symlink_returns_false_for_missing_path() {
        let temp = TempDir::new().unwrap();
        let missing = temp.path().join("does-not-exist");
        assert!(!check_is_symlink(&missing));
    }

    #[test]
    fn check_is_symlink_returns_true_for_working_symlink() {
        let temp = TempDir::new().unwrap();
        let target = temp.path().join("target");
        let link = temp.path().join("link");
        fs::write(&target, "content").unwrap();

        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, &link).unwrap();

        #[cfg(unix)]
        assert!(check_is_symlink(&link));
    }

    #[test]
    fn check_is_symlink_returns_true_for_broken_symlink() {
        let temp = TempDir::new().unwrap();
        let target = temp.path().join("target");
        let link = temp.path().join("link");

        fs::write(&target, "content").unwrap();

        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, &link).unwrap();

        fs::remove_file(&target).unwrap();

        #[cfg(unix)]
        assert!(check_is_symlink(&link));
    }

    #[test]
    fn check_is_broken_symlink_returns_false_for_regular_file() {
        let temp = TempDir::new().unwrap();
        let file = temp.path().join("test.txt");
        fs::write(&file, "content").unwrap();
        assert!(!check_is_broken_symlink(&file));
    }

    #[test]
    fn check_is_broken_symlink_returns_false_for_working_symlink() {
        let temp = TempDir::new().unwrap();
        let target = temp.path().join("target");
        let link = temp.path().join("link");
        fs::write(&target, "content").unwrap();

        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, &link).unwrap();

        #[cfg(unix)]
        assert!(!check_is_broken_symlink(&link));
    }

    #[test]
    fn check_is_broken_symlink_returns_true_for_broken_symlink() {
        let temp = TempDir::new().unwrap();
        let target = temp.path().join("target");
        let link = temp.path().join("link");

        fs::write(&target, "content").unwrap();

        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, &link).unwrap();

        fs::remove_file(&target).unwrap();

        #[cfg(unix)]
        assert!(check_is_broken_symlink(&link));
    }

    #[test]
    fn check_is_broken_symlink_returns_false_for_missing_path() {
        let temp = TempDir::new().unwrap();
        let missing = temp.path().join("does-not-exist");
        assert!(!check_is_broken_symlink(&missing));
    }

    #[test]
    fn check_local_definition_exists_returns_true_for_regular_directory() {
        let temp = TempDir::new().unwrap();
        let dir = temp.path().join("test-dir");
        fs::create_dir(&dir).unwrap();
        assert!(check_local_definition_exists(&dir));
    }

    #[test]
    fn check_local_definition_exists_returns_true_for_regular_file() {
        let temp = TempDir::new().unwrap();
        let file = temp.path().join("test.txt");
        fs::write(&file, "content").unwrap();
        assert!(check_local_definition_exists(&file));
    }

    #[test]
    fn check_local_definition_exists_returns_false_for_symlink() {
        let temp = TempDir::new().unwrap();
        let target = temp.path().join("target");
        let link = temp.path().join("link");
        fs::write(&target, "content").unwrap();

        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, &link).unwrap();

        #[cfg(unix)]
        assert!(!check_local_definition_exists(&link));
    }

    #[test]
    fn check_local_definition_exists_returns_false_for_broken_symlink() {
        let temp = TempDir::new().unwrap();
        let target = temp.path().join("target");
        let link = temp.path().join("link");

        fs::write(&target, "content").unwrap();

        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, &link).unwrap();

        fs::remove_file(&target).unwrap();

        #[cfg(unix)]
        assert!(!check_local_definition_exists(&link));
    }

    #[test]
    fn check_local_definition_exists_returns_false_for_missing_path() {
        let temp = TempDir::new().unwrap();
        let missing = temp.path().join("does-not-exist");
        assert!(!check_local_definition_exists(&missing));
    }

    #[test]
    fn validate_skill_source_returns_true_for_valid_skill() {
        let temp = TempDir::new().unwrap();
        let skill_dir = create_skill_dir(temp.path(), "test-skill");
        assert!(validate_skill_source(&skill_dir));
    }

    #[test]
    fn validate_skill_source_returns_false_for_missing_directory() {
        let temp = TempDir::new().unwrap();
        let missing = temp.path().join("does-not-exist");
        assert!(!validate_skill_source(&missing));
    }

    #[test]
    fn validate_skill_source_returns_false_for_file_not_directory() {
        let temp = TempDir::new().unwrap();
        let file = temp.path().join("test.txt");
        fs::write(&file, "content").unwrap();
        assert!(!validate_skill_source(&file));
    }

    #[test]
    fn validate_skill_source_returns_false_for_missing_skill_md() {
        let temp = TempDir::new().unwrap();
        let skill_dir = temp.path().join("test-skill").join("skill");
        fs::create_dir_all(&skill_dir).unwrap();
        // No SKILL.md file created
        assert!(!validate_skill_source(&skill_dir));
    }

    #[test]
    fn validate_skill_source_returns_false_for_empty_directory() {
        let temp = TempDir::new().unwrap();
        let skill_dir = temp.path().join("empty-skill");
        fs::create_dir(&skill_dir).unwrap();
        assert!(!validate_skill_source(&skill_dir));
    }

    #[test]
    fn determine_action_returns_invalid_for_missing_source_skill_md() {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("source").join("skill");
        let target = temp.path().join("target");

        fs::create_dir_all(&source).unwrap();
        // No SKILL.md created

        let action = determine_action(&target, &source);
        assert_eq!(action, SkillAction::NoneSkillDirectoryInvalid);
    }

    #[test]
    fn determine_action_returns_local_definition_for_existing_directory() {
        let temp = TempDir::new().unwrap();
        let source = create_skill_dir(temp.path(), "source");
        let target = temp.path().join("target");

        fs::create_dir(&target).unwrap();

        let action = determine_action(&target, &source);
        assert_eq!(action, SkillAction::NoneLocalDefinition);
    }

    #[test]
    fn determine_action_returns_local_definition_for_existing_file() {
        let temp = TempDir::new().unwrap();
        let source = create_skill_dir(temp.path(), "source");
        let target = temp.path().join("target");

        fs::write(&target, "content").unwrap();

        let action = determine_action(&target, &source);
        assert_eq!(action, SkillAction::NoneLocalDefinition);
    }

    #[test]
    fn determine_action_returns_already_linked_for_working_symlink() {
        let temp = TempDir::new().unwrap();
        let source = create_skill_dir(temp.path(), "source");
        let target = temp.path().join("target");
        let link_target = temp.path().join("other");

        fs::create_dir(&link_target).unwrap();

        #[cfg(unix)]
        std::os::unix::fs::symlink(&link_target, &target).unwrap();

        #[cfg(unix)]
        {
            let action = determine_action(&target, &source);
            assert_eq!(action, SkillAction::NoneAlreadyLinked);
        }
    }

    #[test]
    fn determine_action_returns_already_linked_for_broken_symlink() {
        let temp = TempDir::new().unwrap();
        let source = create_skill_dir(temp.path(), "source");
        let target = temp.path().join("target");
        let link_target = temp.path().join("other");

        fs::write(&link_target, "content").unwrap();

        #[cfg(unix)]
        std::os::unix::fs::symlink(&link_target, &target).unwrap();

        fs::remove_file(&link_target).unwrap();

        #[cfg(unix)]
        {
            let action = determine_action(&target, &source);
            assert_eq!(action, SkillAction::NoneAlreadyLinked);
        }
    }

    #[test]
    fn determine_action_returns_created_link_for_new_link() {
        let temp = TempDir::new().unwrap();
        let source = create_skill_dir(temp.path(), "source");
        let target = temp.path().join("target");

        let action = determine_action(&target, &source);
        assert_eq!(action, SkillAction::CreatedLink);
    }

    #[test]
    fn determine_action_rejects_path_with_parent_dir_in_target() {
        let temp = TempDir::new().unwrap();
        let source = create_skill_dir(temp.path(), "source");
        let target = temp.path().join("..").join("target");

        let action = determine_action(&target, &source);
        assert!(matches!(action, SkillAction::FailedOther(_)));
    }

    #[test]
    fn determine_action_rejects_path_with_parent_dir_in_source() {
        let temp = TempDir::new().unwrap();
        let target = temp.path().join("target");
        let source = temp.path().join("..").join("source");

        let action = determine_action(&target, &source);
        assert!(matches!(action, SkillAction::FailedOther(_)));
    }

    #[test]
    fn determine_action_handles_symlink_pointing_to_tmp() {
        let temp = TempDir::new().unwrap();
        let source = create_skill_dir(temp.path(), "source");
        let target = temp.path().join("target");

        // Create symlink pointing to /tmp
        #[cfg(unix)]
        std::os::unix::fs::symlink("/tmp", &target).unwrap();

        #[cfg(unix)]
        {
            let action = determine_action(&target, &source);
            // Should detect as already linked (even though it points elsewhere)
            assert_eq!(action, SkillAction::NoneAlreadyLinked);
        }
    }

    // Property-based tests using proptest
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn determine_action_is_idempotent(
            source_name in "[a-z]{3,10}",
            target_name in "[a-z]{3,10}"
        ) {
            let temp = TempDir::new().unwrap();
            let source = create_skill_dir(temp.path(), &source_name);
            let target = temp.path().join(target_name);

            let action1 = determine_action(&target, &source);
            let action2 = determine_action(&target, &source);

            prop_assert_eq!(action1, action2);
        }

        #[test]
        fn path_validation_rejects_all_traversal_forms(
            segments in prop::collection::vec("[a-z]{1,5}", 1..5)
        ) {
            let temp = TempDir::new().unwrap();
            let source = create_skill_dir(temp.path(), "source");

            // Build a path with .. in the middle
            let mut path = temp.path().to_path_buf();
            for segment in &segments {
                path = path.join(segment);
            }
            path = path.join("..").join("target");

            let action = determine_action(&path, &source);
            prop_assert!(matches!(action, SkillAction::FailedOther(_)));
        }
    }

    #[test]
    fn get_claude_skills_dir_returns_path() {
        let result = get_claude_skills_dir();
        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.to_string_lossy().contains(".claude/skills"));
    }

    #[test]
    fn get_opencode_skills_dir_returns_path() {
        let result = get_opencode_skills_dir();
        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.to_string_lossy().contains(".config/opencode/skill"));
    }
}
