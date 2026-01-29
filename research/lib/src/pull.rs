//! Pull command implementation for copying skills from user to repo scope.
//!
//! This module provides the `pull` functionality that copies research skills
//! from the user's global research library (`~/.research/library/<topic>/skill/`)
//! to a git repository's local `.claude/skills/<topic>/` directory.
//!
//! ## Features
//!
//! - **Git repository detection**: Fails if not in a git repository
//! - **Skill directory copying**: Copies the entire skill directory
//! - **Relative symlinks**: Creates symlinks for Roo and OpenCode frameworks
//! - **Local flag**: Optionally copies underlying research documents
//!
//! ## Examples
//!
//! ```no_run
//! use research_lib::pull::{PullOptions, pull_topic};
//! use std::path::PathBuf;
//!
//! let options = PullOptions {
//!     topic: "clap".to_string(),
//!     local: false,
//! };
//!
//! pull_topic(&options).unwrap();
//! ```

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use thiserror::Error;
use tracing::{debug, info, instrument};

/// Errors that can occur during pull operations.
#[derive(Debug, Error)]
pub enum PullError {
    /// Not in a git repository.
    #[error("Not in a git repository. Run this command from within a git repository.")]
    NotInGitRepo,

    /// The specified topic was not found.
    #[error("Topic not found: {0}")]
    TopicNotFound(String),

    /// The topic doesn't have a skill directory.
    #[error("Topic '{0}' has no skill directory")]
    NoSkillDirectory(String),

    /// Failed to copy files.
    #[error("Failed to copy files: {0}")]
    CopyError(#[from] std::io::Error),

    /// Failed to create symlink.
    #[error("Failed to create symlink: {0}")]
    SymlinkError(std::io::Error),

    /// Failed to load inventory.
    #[error("Failed to load research inventory: {0}")]
    InventoryError(#[from] crate::metadata::inventory::InventoryError),

    /// Git command failed.
    #[error("Git command failed: {0}")]
    GitError(String),
}

/// Result type for pull operations.
pub type Result<T> = std::result::Result<T, PullError>;

/// Options for the pull command.
#[derive(Debug, Clone)]
pub struct PullOptions {
    /// The topic to pull.
    pub topic: String,
    /// If true, also copy underlying research documents.
    pub local: bool,
}

/// Result of a pull operation.
#[derive(Debug)]
pub struct PullResult {
    /// The topic that was pulled.
    pub topic: String,
    /// Path to the copied skill directory.
    pub skill_path: PathBuf,
    /// Path to the copied research directory (if --local was used).
    pub research_path: Option<PathBuf>,
    /// Symlinks created for other frameworks.
    pub symlinks_created: Vec<PathBuf>,
    /// Warnings encountered during the operation.
    pub warnings: Vec<String>,
}

/// Get the git repository root directory.
///
/// Uses `git rev-parse --show-toplevel` to find the repository root.
#[instrument]
pub fn get_git_root() -> Result<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|e| PullError::GitError(format!("Failed to run git: {}", e)))?;

    if !output.status.success() {
        return Err(PullError::NotInGitRepo);
    }

    let path_str = String::from_utf8_lossy(&output.stdout);
    let path = PathBuf::from(path_str.trim());
    debug!("Git root: {:?}", path);
    Ok(path)
}

/// Get the user's research library path.
///
/// Uses `$RESEARCH_DIR/.research/library` if `RESEARCH_DIR` is set,
/// otherwise falls back to `$HOME/.research/library`.
pub fn get_research_library_path() -> Result<PathBuf> {
    let base = std::env::var("RESEARCH_DIR").unwrap_or_else(|_| {
        std::env::var("HOME").expect("Neither RESEARCH_DIR nor HOME environment variable is set")
    });

    Ok(PathBuf::from(base).join(".research").join("library"))
}

/// Pull a topic from the user's research library to the current repository.
#[instrument(skip(options))]
pub fn pull_topic(options: &PullOptions) -> Result<PullResult> {
    let git_root = get_git_root()?;
    let library_path = get_research_library_path()?;

    // Check if topic exists in research library
    let topic_dir = library_path.join(&options.topic);
    let skill_source = topic_dir.join("skill");

    if !skill_source.exists() || !skill_source.is_dir() {
        // Check if topic exists at all
        if !topic_dir.exists() {
            return Err(PullError::TopicNotFound(options.topic.clone()));
        }
        return Err(PullError::NoSkillDirectory(options.topic.clone()));
    }

    info!("Pulling topic '{}' to repository", options.topic);

    // Create destination paths
    let skill_dest = git_root.join(".claude").join("skills").join(&options.topic);
    let mut warnings = Vec::new();

    // Copy skill directory
    copy_directory(&skill_source, &skill_dest)?;
    info!("Copied skill to {:?}", skill_dest);

    // Handle --local flag
    let research_path = if options.local {
        let research_dest = git_root
            .join(".claude")
            .join("research")
            .join(&options.topic);

        // Copy all research documents (excluding skill directory)
        copy_research_documents(&topic_dir, &research_dest)?;
        info!("Copied research documents to {:?}", research_dest);
        Some(research_dest)
    } else {
        None
    };

    // Create relative symlinks for other frameworks
    let symlinks_created = create_framework_symlinks(&git_root, &options.topic, &mut warnings)?;

    Ok(PullResult {
        topic: options.topic.clone(),
        skill_path: skill_dest,
        research_path,
        symlinks_created,
        warnings,
    })
}

/// Copy a directory recursively.
fn copy_directory(source: &Path, dest: &Path) -> Result<()> {
    debug!("Copying directory {:?} to {:?}", source, dest);

    // Create destination directory
    fs::create_dir_all(dest)?;

    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let dest_path = dest.join(entry.file_name());

        if source_path.is_dir() {
            copy_directory(&source_path, &dest_path)?;
        } else {
            fs::copy(&source_path, &dest_path)?;
        }
    }

    Ok(())
}

/// Copy research documents (excluding the skill directory).
fn copy_research_documents(topic_dir: &Path, dest: &Path) -> Result<()> {
    debug!(
        "Copying research documents from {:?} to {:?}",
        topic_dir, dest
    );

    fs::create_dir_all(dest)?;

    for entry in fs::read_dir(topic_dir)? {
        let entry = entry?;
        let source_path = entry.path();
        let file_name = entry.file_name();
        let file_name_str = file_name.to_string_lossy();

        // Skip the skill directory and metadata.json
        if file_name_str == "skill" || file_name_str == "metadata.json" {
            continue;
        }

        let dest_path = dest.join(&file_name);

        if source_path.is_dir() {
            copy_directory(&source_path, &dest_path)?;
        } else {
            fs::copy(&source_path, &dest_path)?;
        }
    }

    Ok(())
}

/// Create relative symlinks for other frameworks (Roo, OpenCode).
fn create_framework_symlinks(
    git_root: &Path,
    topic: &str,
    warnings: &mut Vec<String>,
) -> Result<Vec<PathBuf>> {
    let mut created = Vec::new();

    // Check for .roo directory (Roo framework)
    let roo_dir = git_root.join(".roo");
    if roo_dir.exists() && roo_dir.is_dir() {
        let roo_skills_dir = roo_dir.join("skills");
        let roo_symlink = roo_skills_dir.join(topic);

        if let Some(path) =
            create_relative_symlink(&roo_symlink, "../../.claude/skills", topic, warnings)?
        {
            created.push(path);
        }
    }

    // Check for AGENTS.md (OpenCode framework)
    let agents_md = git_root.join("AGENTS.md");
    if agents_md.exists() {
        let opencode_dir = git_root.join(".opencode").join("skill");
        let opencode_symlink = opencode_dir.join(topic);

        if let Some(path) =
            create_relative_symlink(&opencode_symlink, "../../.claude/skills", topic, warnings)?
        {
            created.push(path);
        }
    }

    Ok(created)
}

/// Create a relative symlink, handling existing symlinks.
fn create_relative_symlink(
    symlink_path: &Path,
    relative_prefix: &str,
    topic: &str,
    warnings: &mut Vec<String>,
) -> Result<Option<PathBuf>> {
    let target = format!("{}/{}", relative_prefix, topic);

    // Ensure parent directory exists
    if let Some(parent) = symlink_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Check if symlink already exists
    if symlink_path.exists() || symlink_path.is_symlink() {
        // Check if it points to the same target
        if let Ok(existing_target) = fs::read_link(symlink_path) {
            let existing_str = existing_target.to_string_lossy();
            if existing_str == target {
                debug!("Symlink already exists with correct target: {:?}", symlink_path);
                return Ok(None);
            } else {
                warnings.push(format!(
                    "Symlink {:?} exists with different target: {} (expected {})",
                    symlink_path, existing_str, target
                ));
                return Ok(None);
            }
        } else {
            warnings.push(format!(
                "Path {:?} exists but is not a symlink",
                symlink_path
            ));
            return Ok(None);
        }
    }

    // Create relative symlink
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&target, symlink_path)
            .map_err(|e| PullError::SymlinkError(e))?;
        info!("Created symlink: {:?} -> {}", symlink_path, target);
        Ok(Some(symlink_path.to_path_buf()))
    }

    #[cfg(not(unix))]
    {
        warnings.push("Symlinks are only supported on Unix-like systems".to_string());
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_test_environment() -> (TempDir, TempDir) {
        let git_repo = TempDir::new().unwrap();
        let research_dir = TempDir::new().unwrap();

        // Initialize git repo
        Command::new("git")
            .args(["init"])
            .current_dir(git_repo.path())
            .output()
            .expect("Failed to init git repo");

        (git_repo, research_dir)
    }

    #[test]
    fn test_get_git_root_in_repo() {
        let (git_repo, _) = setup_test_environment();

        // Change to the git repo directory and test
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(git_repo.path()).unwrap();

        let result = get_git_root();
        assert!(result.is_ok());

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_get_git_root_not_in_repo() {
        let temp = TempDir::new().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(temp.path()).unwrap();

        let result = get_git_root();
        assert!(matches!(result, Err(PullError::NotInGitRepo)));

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_copy_directory() {
        let temp = TempDir::new().unwrap();
        let source = temp.path().join("source");
        let dest = temp.path().join("dest");

        // Create source structure
        fs::create_dir_all(source.join("subdir")).unwrap();
        fs::write(source.join("file1.txt"), "content1").unwrap();
        fs::write(source.join("subdir").join("file2.txt"), "content2").unwrap();

        copy_directory(&source, &dest).unwrap();

        assert!(dest.join("file1.txt").exists());
        assert!(dest.join("subdir").join("file2.txt").exists());
        assert_eq!(
            fs::read_to_string(dest.join("file1.txt")).unwrap(),
            "content1"
        );
    }

    #[test]
    fn test_copy_research_documents_excludes_skill() {
        let temp = TempDir::new().unwrap();
        let topic_dir = temp.path().join("topic");
        let dest = temp.path().join("dest");

        // Create topic structure
        fs::create_dir_all(topic_dir.join("skill")).unwrap();
        fs::write(topic_dir.join("overview.md"), "overview").unwrap();
        fs::write(topic_dir.join("metadata.json"), "{}").unwrap();
        fs::write(topic_dir.join("skill").join("SKILL.md"), "skill").unwrap();

        copy_research_documents(&topic_dir, &dest).unwrap();

        assert!(dest.join("overview.md").exists());
        assert!(!dest.join("skill").exists());
        assert!(!dest.join("metadata.json").exists());
    }

    #[test]
    fn test_create_relative_symlink() {
        let temp = TempDir::new().unwrap();
        let symlink_path = temp.path().join("link");
        let mut warnings = Vec::new();

        let result = create_relative_symlink(&symlink_path, "../target", "topic", &mut warnings);

        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.is_some());

        // Verify symlink exists and has correct target
        assert!(symlink_path.is_symlink());
        let target = fs::read_link(&symlink_path).unwrap();
        assert_eq!(target.to_string_lossy(), "../target/topic");
    }

    #[test]
    fn test_create_relative_symlink_existing_same_target() {
        let temp = TempDir::new().unwrap();
        let symlink_path = temp.path().join("link");
        let mut warnings = Vec::new();

        // Create symlink first
        #[cfg(unix)]
        std::os::unix::fs::symlink("../target/topic", &symlink_path).unwrap();

        // Try to create again
        let result = create_relative_symlink(&symlink_path, "../target", "topic", &mut warnings);

        assert!(result.is_ok());
        assert!(result.unwrap().is_none()); // Should return None (no new symlink created)
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_create_relative_symlink_existing_different_target() {
        let temp = TempDir::new().unwrap();
        let symlink_path = temp.path().join("link");
        let mut warnings = Vec::new();

        // Create symlink with different target
        #[cfg(unix)]
        std::os::unix::fs::symlink("../other/path", &symlink_path).unwrap();

        let result = create_relative_symlink(&symlink_path, "../target", "topic", &mut warnings);

        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
        assert!(!warnings.is_empty());
        assert!(warnings[0].contains("different target"));
    }
}
