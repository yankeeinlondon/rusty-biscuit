use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::error::{ClaudineError, Result};

use super::paths::ResourceScope;

/// Result of a single symlink creation attempt.
#[derive(Debug)]
pub enum LinkResult {
    /// Symlink was created successfully.
    Linked {
        /// Absolute path to the source skill directory.
        source: PathBuf,
        /// Absolute path to the created symlink.
        dest: PathBuf,
        /// The value of the symlink (absolute or relative path).
        link_target: PathBuf,
    },
    /// Symlink already exists pointing to the correct target.
    AlreadyLinked,
    /// Linking was skipped for the stated reason.
    Skipped {
        /// Why the link was skipped.
        reason: String,
    },
}

/// Return the symlink target when a resource root is itself a symlink.
pub fn category_link_target(path: &Path) -> Result<Option<PathBuf>> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Ok(Some(fs::read_link(path)?)),
        Ok(_) => Ok(None),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

/// Create a symlink from a resource path into a destination resource root.
///
/// The destination preserves the resource's relative path beneath `source_root`.
///
/// ## Errors
///
/// Returns an error if:
/// - The source is not contained within `source_root`
/// - The relative path is empty
/// - The parent directory cannot be created
/// - The symlink cannot be created
pub fn create_resource_link(
    source: &Path,
    source_root: &Path,
    dest_root: &Path,
    scope: ResourceScope,
) -> Result<LinkResult> {
    let relative = source.strip_prefix(source_root).map_err(|_| {
        ClaudineError::LinkingError(format!(
            "source path {} is not contained within source root {}",
            biscuit_file::to_portable_string(source),
            biscuit_file::to_portable_string(source_root)
        ))
    })?;

    if relative.as_os_str().is_empty() {
        return Err(ClaudineError::LinkingError(format!(
            "source path {} resolves to an empty relative path beneath {}",
            biscuit_file::to_portable_string(source),
            biscuit_file::to_portable_string(source_root)
        )));
    }

    let dest = dest_root.join(relative);

    let parent = dest
        .parent()
        .ok_or_else(|| ClaudineError::LinkingError("dest has no parent".to_string()))?;
    fs::create_dir_all(parent)?;

    let link_target = match scope {
        ResourceScope::User => source.to_path_buf(),
        ResourceScope::Repo => relative_path(parent, source),
    };

    #[cfg(unix)]
    let result = std::os::unix::fs::symlink(&link_target, &dest);

    // Windows fixes the link type at creation time, unlike Unix's single
    // symlink syscall, so select the API from the native source type.
    #[cfg(windows)]
    let result = if source.is_dir() {
        std::os::windows::fs::symlink_dir(&link_target, &dest)
    } else {
        std::os::windows::fs::symlink_file(&link_target, &dest)
    };

    #[cfg(not(any(unix, windows)))]
    let result = Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "symlink creation is not supported on this platform",
    ));

    match result {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            if dest
                .symlink_metadata()
                .map(|m| m.file_type().is_symlink())
                .unwrap_or(false)
            {
                let existing_target = fs::read_link(&dest)?;
                if existing_target == link_target {
                    return Ok(LinkResult::AlreadyLinked);
                }

                return Ok(LinkResult::Skipped {
                    reason: format!(
                        "symlink exists but points to {} (expected {})",
                        biscuit_file::to_portable_string(&existing_target),
                        biscuit_file::to_portable_string(&link_target)
                    ),
                });
            }

            let path_label = if dest.is_dir() { "directory" } else { "file" };
            return Ok(LinkResult::Skipped {
                reason: format!(
                    "real {path_label} already exists at {}",
                    biscuit_file::to_portable_string(&dest)
                ),
            });
        }
        Err(error) => return Err(error.into()),
    }

    Ok(LinkResult::Linked {
        source: source.to_path_buf(),
        dest,
        link_target,
    })
}

/// Create a symlink from source skill dir into dest_dir.
///
/// User scope creates absolute symlinks. Repo scope creates relative symlinks.
/// Never overwrites real (non-symlink) directories.
///
/// ## Errors
///
/// Returns an error if:
/// - The source has no file name component
/// - The parent directory cannot be created
/// - The symlink cannot be created
pub fn create_skill_link(
    source: &Path,
    dest_dir: &Path,
    scope: ResourceScope,
) -> Result<LinkResult> {
    let source_root = source.parent().ok_or_else(|| {
        ClaudineError::LinkingError(format!(
            "source path has no parent: {}",
            biscuit_file::to_portable_string(source)
        ))
    })?;
    create_resource_link(source, source_root, dest_dir, scope)
}

/// Compute the relative path from `from_dir` to `target`.
///
/// Both paths must be absolute. The result is suitable for
/// use as a symlink target when created inside `from_dir`.
///
/// ## Examples
///
/// ```ignore
/// let from = Path::new("/repo/.opencode/skills");
/// let target = Path::new("/repo/.claude/skills/my-skill");
/// assert_eq!(
///     relative_path(from, target),
///     PathBuf::from("../../.claude/skills/my-skill")
/// );
/// ```
pub fn relative_path(from_dir: &Path, target: &Path) -> PathBuf {
    #[cfg(unix)]
    {
        debug_assert!(
            from_dir.is_absolute(),
            "from_dir must be absolute: {}",
            biscuit_file::to_portable_string(from_dir)
        );
        debug_assert!(
            target.is_absolute(),
            "target must be absolute: {}",
            biscuit_file::to_portable_string(target)
        );
    }

    let from_components: Vec<Component<'_>> = from_dir.components().collect();
    let target_components: Vec<Component<'_>> = target.components().collect();

    // Find common prefix length
    let common_len = from_components
        .iter()
        .zip(target_components.iter())
        .take_while(|(a, b)| a == b)
        .count();

    // "../" for each remaining component in from_dir
    let ups = from_components.len() - common_len;
    let mut result = PathBuf::new();
    for _ in 0..ups {
        result.push("..");
    }

    // Append remaining components from target
    for component in &target_components[common_len..] {
        result.push(component);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn relative_path_computes_correct_result() {
        let from = Path::new("/repo/.opencode/skills");
        let target = Path::new("/repo/.claude/skills/my-skill");
        let result = relative_path(from, target);
        assert_eq!(result, PathBuf::from("../../.claude/skills/my-skill"));
    }

    #[test]
    fn relative_path_sibling_directories() {
        let from = Path::new("/home/user/.opencode/skills");
        let target = Path::new("/home/user/.claude/skills/clap");
        let result = relative_path(from, target);
        assert_eq!(result, PathBuf::from("../../.claude/skills/clap"));
    }

    #[test]
    fn relative_path_same_parent() {
        let from = Path::new("/a/b");
        let target = Path::new("/a/c");
        let result = relative_path(from, target);
        assert_eq!(result, PathBuf::from("../c"));
    }

    #[test]
    fn relative_path_no_common_prefix() {
        let from = Path::new("/a/b");
        let target = Path::new("/c/d");
        let result = relative_path(from, target);
        assert_eq!(result, PathBuf::from("../../c/d"));
    }

    #[cfg(unix)]
    #[test]
    #[should_panic]
    fn relative_path_requires_absolute_from_dir() {
        relative_path(Path::new("a/b"), Path::new("/c/d"));
    }

    #[cfg(unix)]
    #[test]
    #[should_panic]
    fn relative_path_requires_absolute_target() {
        relative_path(Path::new("/a/b"), Path::new("c/d"));
    }

    #[test]
    fn relative_path_deeply_nested() {
        let from = Path::new("/repo/sub/.opencode/skills");
        let target = Path::new("/repo/sub/.claude/skills/deep-skill");
        let result = relative_path(from, target);
        assert_eq!(result, PathBuf::from("../../.claude/skills/deep-skill"));
    }

    #[cfg(unix)]
    #[test]
    fn category_link_target_detects_root_symlink() {
        let tmp = TempDir::new().unwrap();
        let source_root = tmp.path().join("source");
        let target_root = tmp.path().join("target");
        fs::create_dir_all(&source_root).unwrap();
        std::os::unix::fs::symlink(&source_root, &target_root).unwrap();

        let target = category_link_target(&target_root).unwrap();
        assert_eq!(target, Some(source_root));
    }

    #[cfg(unix)]
    #[test]
    fn user_scope_creates_absolute_symlink() {
        let tmp = TempDir::new().unwrap();
        let source = tmp.path().join("source/my-skill");
        let dest_dir = tmp.path().join("dest");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("SKILL.md"), "# Skill").unwrap();
        fs::create_dir_all(&dest_dir).unwrap();

        let result = create_skill_link(&source, &dest_dir, ResourceScope::User).unwrap();

        match result {
            LinkResult::Linked {
                source: src,
                dest,
                link_target,
            } => {
                assert_eq!(src, source);
                assert!(dest.ends_with("my-skill"));
                // User scope: link target should be absolute
                assert!(link_target.is_absolute());
                // Verify the symlink was actually created
                assert!(dest.symlink_metadata().unwrap().file_type().is_symlink());
            }
            other => panic!("expected Linked, got {other:?}"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn repo_scope_creates_relative_symlink() {
        let tmp = TempDir::new().unwrap();
        let source = tmp.path().join("repo/.claude/skills/my-skill");
        let dest_dir = tmp.path().join("repo/.opencode/skills");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("SKILL.md"), "# Skill").unwrap();
        fs::create_dir_all(&dest_dir).unwrap();

        let result = create_skill_link(&source, &dest_dir, ResourceScope::Repo).unwrap();

        match result {
            LinkResult::Linked { link_target, .. } => {
                // Repo scope: link target should be relative
                assert!(link_target.is_relative());
                assert_eq!(link_target, PathBuf::from("../../.claude/skills/my-skill"));
            }
            other => panic!("expected Linked, got {other:?}"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn never_overwrites_real_directory() {
        let tmp = TempDir::new().unwrap();
        let source = tmp.path().join("source/my-skill");
        let dest_dir = tmp.path().join("dest");
        let existing = dest_dir.join("my-skill");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("SKILL.md"), "# Source").unwrap();
        fs::create_dir_all(&existing).unwrap();
        fs::write(existing.join("SKILL.md"), "# Existing").unwrap();

        let result = create_skill_link(&source, &dest_dir, ResourceScope::User).unwrap();

        match result {
            LinkResult::Skipped { reason } => {
                assert!(reason.contains("real directory already exists"));
            }
            other => panic!("expected Skipped, got {other:?}"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn returns_already_linked_for_existing_correct_symlink() {
        let tmp = TempDir::new().unwrap();
        let source = tmp.path().join("source/my-skill");
        let dest_dir = tmp.path().join("dest");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("SKILL.md"), "# Skill").unwrap();
        fs::create_dir_all(&dest_dir).unwrap();

        // Create the symlink first
        std::os::unix::fs::symlink(&source, dest_dir.join("my-skill")).unwrap();

        let result = create_skill_link(&source, &dest_dir, ResourceScope::User).unwrap();

        assert!(matches!(result, LinkResult::AlreadyLinked));
    }

    #[cfg(unix)]
    #[test]
    fn skips_symlink_pointing_elsewhere() {
        let tmp = TempDir::new().unwrap();
        let source = tmp.path().join("source/my-skill");
        let other = tmp.path().join("other/my-skill");
        let dest_dir = tmp.path().join("dest");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&other).unwrap();
        fs::create_dir_all(&dest_dir).unwrap();

        // Create a symlink pointing to 'other' instead of 'source'
        std::os::unix::fs::symlink(&other, dest_dir.join("my-skill")).unwrap();

        let result = create_skill_link(&source, &dest_dir, ResourceScope::User).unwrap();

        match result {
            LinkResult::Skipped { reason } => {
                assert!(reason.contains("symlink exists but points to"));
            }
            other => panic!("expected Skipped, got {other:?}"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn resource_link_preserves_nested_relative_path() {
        let tmp = TempDir::new().unwrap();
        let source_root = tmp.path().join("source");
        let source = source_root.join("prompts/create-deep-dive-document.md");
        let dest_root = tmp.path().join("dest");
        fs::create_dir_all(source.parent().unwrap()).unwrap();
        fs::write(&source, "# Prompt").unwrap();
        fs::create_dir_all(&dest_root).unwrap();

        let result =
            create_resource_link(&source, &source_root, &dest_root, ResourceScope::Repo).unwrap();

        match result {
            LinkResult::Linked {
                source: src,
                dest,
                link_target,
            } => {
                assert_eq!(src, source);
                assert_eq!(dest, dest_root.join("prompts/create-deep-dive-document.md"));
                assert_eq!(
                    link_target,
                    PathBuf::from("../../source/prompts/create-deep-dive-document.md")
                );
            }
            other => panic!("expected Linked, got {other:?}"),
        }
    }

    #[cfg(windows)]
    #[test]
    fn windows_resource_link_creates_file_symlink() {
        let fixture = TempDir::new().unwrap();
        let source_root = fixture.path().join("source");
        let source = source_root.join("prompts/plan.md");
        let dest_root = fixture.path().join("dest");
        fs::create_dir_all(source.parent().unwrap()).unwrap();
        fs::write(&source, "# Plan").unwrap();

        let result = create_resource_link(
            &source,
            &source_root,
            &dest_root,
            ResourceScope::User,
        )
        .unwrap();
        let LinkResult::Linked { dest, .. } = result else {
            panic!("expected a linked file, got {result:?}");
        };
        assert!(dest.symlink_metadata().unwrap().file_type().is_symlink());
        assert_eq!(fs::read_to_string(dest).unwrap(), "# Plan");
    }

    #[cfg(windows)]
    #[test]
    fn windows_resource_link_skips_real_file_collision() {
        let fixture = TempDir::new().unwrap();
        let source_root = fixture.path().join("source");
        let source = source_root.join("prompts/plan.md");
        let dest_root = fixture.path().join("dest");
        let dest = dest_root.join("prompts/plan.md");
        fs::create_dir_all(source.parent().unwrap()).unwrap();
        fs::create_dir_all(dest.parent().unwrap()).unwrap();
        fs::write(&source, "# Source").unwrap();
        fs::write(&dest, "# Existing").unwrap();

        let result = create_resource_link(
            &source,
            &source_root,
            &dest_root,
            ResourceScope::User,
        )
        .unwrap();
        let LinkResult::Skipped { reason } = result else {
            panic!("expected a collision skip, got {result:?}");
        };
        assert!(reason.contains("real file already exists"));
        assert_eq!(fs::read_to_string(dest).unwrap(), "# Existing");
    }

    #[cfg(windows)]
    #[test]
    fn windows_resource_link_reports_already_linked_file() {
        let fixture = TempDir::new().unwrap();
        let source_root = fixture.path().join("source");
        let source = source_root.join("prompts/plan.md");
        let dest_root = fixture.path().join("dest");
        fs::create_dir_all(source.parent().unwrap()).unwrap();
        fs::write(&source, "# Plan").unwrap();

        create_resource_link(
            &source,
            &source_root,
            &dest_root,
            ResourceScope::User,
        )
        .unwrap();
        let result = create_resource_link(
            &source,
            &source_root,
            &dest_root,
            ResourceScope::User,
        )
        .unwrap();

        assert!(matches!(result, LinkResult::AlreadyLinked));
    }
}
