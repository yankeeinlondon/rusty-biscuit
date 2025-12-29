//! Symlink creation functions for the link command.
//!
//! This module provides safe symlink creation with comprehensive validation
//! and error handling. It creates absolute symlinks for robustness.

use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tracing::{debug, instrument};

/// Errors that can occur during symlink creation.
#[derive(Debug, Error)]
pub enum CreationError {
    /// The source skill directory is missing or invalid.
    #[error("Invalid source: skill directory missing or invalid: {0}")]
    InvalidSource(PathBuf),

    /// Failed to create parent directory for the symlink.
    #[error("Failed to create parent directory: {0}")]
    ParentDirectory(#[source] std::io::Error),

    /// Failed to create the symlink itself.
    #[error("Failed to create symlink: {0}")]
    SymlinkCreation(#[source] std::io::Error),
}

/// Validates that a skill directory exists and contains a SKILL.md file.
///
/// # Arguments
///
/// * `path` - Path to the skill directory to validate
///
/// # Returns
///
/// Returns `Ok(())` if the directory exists and contains `SKILL.md`, otherwise
/// returns a `CreationError::InvalidSource`.
///
/// # Example
///
/// ```rust,no_run
/// use std::path::Path;
/// use research_lib::link::creation::validate_skill_directory;
///
/// let skill_dir = Path::new("/path/to/skill");
/// validate_skill_directory(skill_dir)?;
/// # Ok::<(), research_lib::link::creation::CreationError>(())
/// ```
#[instrument]
pub fn validate_skill_directory(path: &Path) -> Result<(), CreationError> {
    debug!("Validating skill directory: {:?}", path);

    // Check if path exists and is a directory
    if !path.exists() {
        debug!("Path does not exist: {:?}", path);
        return Err(CreationError::InvalidSource(path.to_path_buf()));
    }

    if !path.is_dir() {
        debug!("Path is not a directory: {:?}", path);
        return Err(CreationError::InvalidSource(path.to_path_buf()));
    }

    // Check for SKILL.md file
    let skill_md = path.join("SKILL.md");
    if !skill_md.exists() || !skill_md.is_file() {
        debug!("SKILL.md not found in directory: {:?}", path);
        return Err(CreationError::InvalidSource(path.to_path_buf()));
    }

    debug!("Skill directory validated successfully: {:?}", path);
    Ok(())
}

/// Ensures the parent directory of a path exists, creating it if necessary.
///
/// This function is idempotent - it can be called multiple times safely.
///
/// # Arguments
///
/// * `path` - Path whose parent directory should be ensured
///
/// # Returns
///
/// Returns `Ok(())` if the parent directory exists or was created successfully,
/// otherwise returns an I/O error.
///
/// # Example
///
/// ```rust,no_run
/// use std::path::Path;
/// use research_lib::link::creation::ensure_parent_directory;
///
/// let symlink_path = Path::new("/home/user/.claude/skills/clap");
/// ensure_parent_directory(symlink_path)?;
/// # Ok::<(), std::io::Error>(())
/// ```
#[instrument]
pub fn ensure_parent_directory(path: &Path) -> Result<(), std::io::Error> {
    debug!("Ensuring parent directory exists for: {:?}", path);

    if let Some(parent) = path.parent() {
        if !parent.exists() {
            debug!("Creating parent directory: {:?}", parent);
            fs::create_dir_all(parent)?;
            debug!("Parent directory created successfully");
        } else {
            debug!("Parent directory already exists: {:?}", parent);
        }
    }

    Ok(())
}

/// Creates an absolute symbolic link from a skill directory to a symlink location.
///
/// This function performs comprehensive validation and error handling:
/// 1. Validates the source skill directory contains SKILL.md
/// 2. Creates parent directories if needed
/// 3. Creates an absolute symlink for robustness
///
/// # Arguments
///
/// * `skill_dir` - The source skill directory (e.g., `~/.research/library/clap/skill/`)
/// * `symlink_location` - Where to create the symlink (e.g., `~/.claude/skills/clap`)
///
/// # Returns
///
/// Returns `Ok(())` if the symlink was created successfully, otherwise returns
/// a `CreationError` describing what went wrong.
///
/// # Errors
///
/// - `CreationError::InvalidSource` - Source directory missing or no SKILL.md
/// - `CreationError::ParentDirectory` - Failed to create parent directory
/// - `CreationError::SymlinkCreation` - Failed to create the symlink
///
/// # Example
///
/// ```rust,no_run
/// use std::path::Path;
/// use research_lib::link::creation::create_skill_symlink;
///
/// let skill_dir = Path::new("/home/user/.research/library/clap/skill");
/// let symlink_location = Path::new("/home/user/.claude/skills/clap");
/// create_skill_symlink(skill_dir, symlink_location)?;
/// # Ok::<(), research_lib::link::creation::CreationError>(())
/// ```
#[instrument]
pub fn create_skill_symlink(skill_dir: &Path, symlink_location: &Path) -> Result<(), CreationError> {
    debug!(
        "Creating symlink: {:?} -> {:?}",
        symlink_location, skill_dir
    );

    // 1. Validate source exists and contains SKILL.md
    validate_skill_directory(skill_dir)?;

    // 2. Create parent directory if needed
    ensure_parent_directory(symlink_location).map_err(CreationError::ParentDirectory)?;

    // 3. Create absolute symlink (canonicalize source for absolute path)
    let absolute_source = skill_dir
        .canonicalize()
        .map_err(|_| CreationError::InvalidSource(skill_dir.to_path_buf()))?;

    debug!("Creating absolute symlink to: {:?}", absolute_source);

    #[cfg(unix)]
    std::os::unix::fs::symlink(&absolute_source, symlink_location)
        .map_err(CreationError::SymlinkCreation)?;

    #[cfg(not(unix))]
    {
        // This should never happen as the project targets Unix-like systems
        return Err(CreationError::SymlinkCreation(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "Symlink creation is only supported on Unix-like systems",
        )));
    }

    debug!("Symlink created successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    // Helper function to create a valid skill directory
    fn create_valid_skill_dir(base: &Path, name: &str) -> PathBuf {
        let skill_dir = base.join(name);
        fs::create_dir_all(&skill_dir).unwrap();
        let skill_md = skill_dir.join("SKILL.md");
        let mut file = File::create(&skill_md).unwrap();
        writeln!(file, "# Test Skill").unwrap();
        skill_dir
    }

    // validate_skill_directory tests
    #[test]
    fn validate_skill_directory_accepts_valid_directory() {
        let temp = TempDir::new().unwrap();
        let skill_dir = create_valid_skill_dir(temp.path(), "valid_skill");

        let result = validate_skill_directory(&skill_dir);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_skill_directory_rejects_nonexistent_path() {
        let temp = TempDir::new().unwrap();
        let nonexistent = temp.path().join("does_not_exist");

        let result = validate_skill_directory(&nonexistent);
        assert!(matches!(result, Err(CreationError::InvalidSource(_))));
    }

    #[test]
    fn validate_skill_directory_rejects_file_instead_of_directory() {
        let temp = TempDir::new().unwrap();
        let file_path = temp.path().join("file.txt");
        File::create(&file_path).unwrap();

        let result = validate_skill_directory(&file_path);
        assert!(matches!(result, Err(CreationError::InvalidSource(_))));
    }

    #[test]
    fn validate_skill_directory_rejects_directory_without_skill_md() {
        let temp = TempDir::new().unwrap();
        let empty_dir = temp.path().join("empty");
        fs::create_dir(&empty_dir).unwrap();

        let result = validate_skill_directory(&empty_dir);
        assert!(matches!(result, Err(CreationError::InvalidSource(_))));
    }

    #[test]
    fn validate_skill_directory_rejects_directory_with_skill_md_as_directory() {
        let temp = TempDir::new().unwrap();
        let skill_dir = temp.path().join("skill");
        fs::create_dir_all(&skill_dir).unwrap();
        // Create SKILL.md as a directory instead of a file
        let skill_md_dir = skill_dir.join("SKILL.md");
        fs::create_dir(&skill_md_dir).unwrap();

        let result = validate_skill_directory(&skill_dir);
        assert!(matches!(result, Err(CreationError::InvalidSource(_))));
    }

    // ensure_parent_directory tests
    #[test]
    fn ensure_parent_directory_creates_missing_parent() {
        let temp = TempDir::new().unwrap();
        let nested_path = temp.path().join("a").join("b").join("c").join("file");

        let result = ensure_parent_directory(&nested_path);
        assert!(result.is_ok());
        assert!(nested_path.parent().unwrap().exists());
    }

    #[test]
    fn ensure_parent_directory_succeeds_when_parent_exists() {
        let temp = TempDir::new().unwrap();
        let existing_dir = temp.path().join("existing");
        fs::create_dir(&existing_dir).unwrap();
        let file_in_existing = existing_dir.join("file");

        let result = ensure_parent_directory(&file_in_existing);
        assert!(result.is_ok());
    }

    #[test]
    fn ensure_parent_directory_is_idempotent() {
        let temp = TempDir::new().unwrap();
        let nested_path = temp.path().join("x").join("y").join("file");

        // First call
        ensure_parent_directory(&nested_path).unwrap();
        assert!(nested_path.parent().unwrap().exists());

        // Second call - should succeed without error
        let result = ensure_parent_directory(&nested_path);
        assert!(result.is_ok());
        assert!(nested_path.parent().unwrap().exists());
    }

    #[test]
    fn ensure_parent_directory_handles_root_path() {
        let result = ensure_parent_directory(Path::new("/"));
        assert!(result.is_ok());
    }

    #[test]
    fn ensure_parent_directory_handles_path_without_parent() {
        let result = ensure_parent_directory(Path::new("file"));
        assert!(result.is_ok());
    }

    // create_skill_symlink tests
    #[test]
    fn create_skill_symlink_creates_valid_symlink() {
        let temp = TempDir::new().unwrap();
        let skill_dir = create_valid_skill_dir(temp.path(), "skill");
        let symlink_location = temp.path().join("links").join("my_skill");

        let result = create_skill_symlink(&skill_dir, &symlink_location);
        assert!(result.is_ok());
        assert!(symlink_location.exists());
        assert!(symlink_location.is_symlink());
    }

    #[test]
    fn create_skill_symlink_creates_absolute_symlink() {
        let temp = TempDir::new().unwrap();
        let skill_dir = create_valid_skill_dir(temp.path(), "skill");
        let symlink_location = temp.path().join("links").join("my_skill");

        create_skill_symlink(&skill_dir, &symlink_location).unwrap();

        // Read the symlink target
        let target = fs::read_link(&symlink_location).unwrap();
        // Absolute paths start with /
        assert!(target.is_absolute());
    }

    #[test]
    fn create_skill_symlink_creates_parent_directories() {
        let temp = TempDir::new().unwrap();
        let skill_dir = create_valid_skill_dir(temp.path(), "skill");
        let symlink_location = temp.path().join("a").join("b").join("c").join("my_skill");

        let result = create_skill_symlink(&skill_dir, &symlink_location);
        assert!(result.is_ok());
        assert!(symlink_location.parent().unwrap().exists());
        assert!(symlink_location.exists());
    }

    #[test]
    fn create_skill_symlink_rejects_invalid_source() {
        let temp = TempDir::new().unwrap();
        let invalid_source = temp.path().join("nonexistent");
        let symlink_location = temp.path().join("link");

        let result = create_skill_symlink(&invalid_source, &symlink_location);
        assert!(matches!(result, Err(CreationError::InvalidSource(_))));
    }

    #[test]
    fn create_skill_symlink_rejects_source_without_skill_md() {
        let temp = TempDir::new().unwrap();
        let empty_dir = temp.path().join("empty");
        fs::create_dir(&empty_dir).unwrap();
        let symlink_location = temp.path().join("link");

        let result = create_skill_symlink(&empty_dir, &symlink_location);
        assert!(matches!(result, Err(CreationError::InvalidSource(_))));
    }

    #[test]
    fn create_skill_symlink_verifies_symlink_points_to_correct_target() {
        let temp = TempDir::new().unwrap();
        let skill_dir = create_valid_skill_dir(temp.path(), "skill");
        let symlink_location = temp.path().join("link");

        create_skill_symlink(&skill_dir, &symlink_location).unwrap();

        let target = fs::read_link(&symlink_location).unwrap();
        let expected = skill_dir.canonicalize().unwrap();
        assert_eq!(target, expected);
    }

    #[test]
    fn create_skill_symlink_fails_when_symlink_already_exists() {
        let temp = TempDir::new().unwrap();
        let skill_dir = create_valid_skill_dir(temp.path(), "skill");
        let symlink_location = temp.path().join("link");

        // Create symlink first time
        create_skill_symlink(&skill_dir, &symlink_location).unwrap();

        // Try to create again - should fail
        let result = create_skill_symlink(&skill_dir, &symlink_location);
        assert!(matches!(result, Err(CreationError::SymlinkCreation(_))));
    }

    #[test]
    fn create_skill_symlink_error_contains_path_info() {
        let temp = TempDir::new().unwrap();
        let invalid_source = temp.path().join("invalid");
        let symlink_location = temp.path().join("link");

        let result = create_skill_symlink(&invalid_source, &symlink_location);
        match result {
            Err(CreationError::InvalidSource(path)) => {
                assert_eq!(path, invalid_source);
            }
            _ => panic!("Expected InvalidSource error"),
        }
    }

    #[test]
    fn create_skill_symlink_validates_before_creating_parent() {
        let temp = TempDir::new().unwrap();
        let invalid_source = temp.path().join("invalid");
        let symlink_location = temp.path().join("nested").join("deep").join("link");

        // Should fail validation before creating parent directories
        let result = create_skill_symlink(&invalid_source, &symlink_location);
        assert!(matches!(result, Err(CreationError::InvalidSource(_))));

        // Parent directories should not have been created
        assert!(!symlink_location.parent().unwrap().exists());
    }

    #[test]
    fn create_skill_symlink_with_complex_directory_structure() {
        let temp = TempDir::new().unwrap();

        // Create skill directory with subdirectories
        let skill_dir = temp.path().join("library").join("clap").join("skill");
        fs::create_dir_all(&skill_dir).unwrap();
        let mut skill_md = File::create(skill_dir.join("SKILL.md")).unwrap();
        writeln!(skill_md, "# Clap Skill").unwrap();

        // Add some additional files
        fs::create_dir(skill_dir.join("docs")).unwrap();
        File::create(skill_dir.join("docs").join("readme.md")).unwrap();

        let symlink_location = temp.path().join("skills").join("clap");

        let result = create_skill_symlink(&skill_dir, &symlink_location);
        assert!(result.is_ok());
        assert!(symlink_location.is_symlink());

        // Verify we can access files through the symlink
        assert!(symlink_location.join("SKILL.md").exists());
        assert!(symlink_location.join("docs").join("readme.md").exists());
    }
}
