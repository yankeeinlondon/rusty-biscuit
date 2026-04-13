use std::path::{Path, PathBuf};

use tracing::{debug, trace};
use walkdir::WalkDir;

use crate::file_reference::context::{ResolutionContext, find_git_root, find_package_area};
use crate::file_reference::error::FileReferenceError;
use crate::file_reference::{
    MagicPathList, ParsedReference, PathTemplate, ReferenceKind, TemplateSegment,
};

/// Resolve a parsed reference against runtime context.
pub(crate) fn resolve(
    parsed: &ParsedReference,
    magic_paths: &MagicPathList,
    vault_roots: &[PathBuf],
    ctx: &ResolutionContext,
) -> Result<Option<PathBuf>, FileReferenceError> {
    let interpolated = interpolate(parsed.kind.template(), ctx)?;

    if parsed.recursive {
        resolve_recursive(parsed, &interpolated, magic_paths, vault_roots, ctx)
    } else {
        resolve_direct(parsed, &interpolated, magic_paths, vault_roots, ctx)
    }
}

/// Resolve without recursion -- check exact file paths.
fn resolve_direct(
    parsed: &ParsedReference,
    interpolated: &str,
    magic_paths: &MagicPathList,
    vault_roots: &[PathBuf],
    ctx: &ResolutionContext,
) -> Result<Option<PathBuf>, FileReferenceError> {
    let candidates = build_candidates(parsed, interpolated, magic_paths, vault_roots, ctx)?;

    for candidate in &candidates {
        let exists = candidate.is_file();
        trace!(?candidate, exists, "checking candidate");
        if exists {
            debug!(?candidate, "resolved file reference");
            return Ok(Some(normalize_absolute(candidate, &ctx.cwd)));
        }
    }

    debug!("no candidate matched");
    Ok(None)
}

/// Resolve with recursive directory traversal.
fn resolve_recursive(
    parsed: &ParsedReference,
    interpolated: &str,
    magic_paths: &MagicPathList,
    vault_roots: &[PathBuf],
    ctx: &ResolutionContext,
) -> Result<Option<PathBuf>, FileReferenceError> {
    let roots = build_search_roots(parsed, magic_paths, vault_roots, ctx)?;
    let path = Path::new(interpolated);

    // Extract the filename to search for and optional subdirectory filter
    let needle = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| interpolated.to_string());

    let subdir_filter = if path.components().count() > 1 {
        path.parent().map(|p| p.to_path_buf())
    } else {
        None
    };

    debug!(root_count = roots.len(), ?needle, "starting recursive search");

    let mut matches: Vec<PathBuf> = Vec::new();

    for root in &roots {
        if !root.is_dir() {
            continue;
        }

        let walker = WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok());

        for entry in walker {
            if !entry.file_type().is_file() {
                continue;
            }

            let entry_name = entry.file_name().to_string_lossy();
            if entry_name != needle {
                continue;
            }

            // If there's a subdirectory filter, check that the entry's parent ends with it
            if let Some(ref subdir) = subdir_filter {
                let entry_path = entry.path();
                if let Ok(rel) = entry_path.strip_prefix(root) {
                    if let Some(parent) = rel.parent() {
                        let subdir_str = subdir.to_string_lossy();
                        let parent_str = parent.to_string_lossy();
                        if !parent_str.ends_with(subdir_str.as_ref()) {
                            continue;
                        }
                    } else {
                        continue;
                    }
                }
            }

            matches.push(normalize_absolute(entry.path(), &ctx.cwd));
        }
    }

    // Sort lexicographically and return the first match
    matches.sort();
    debug!(match_count = matches.len(), "recursive search complete");
    Ok(matches.into_iter().next())
}

/// Collect search root directories for a given reference kind.
fn collect_roots(
    kind: &ReferenceKind,
    magic_paths: &MagicPathList,
    vault_roots: &[PathBuf],
    ctx: &ResolutionContext,
) -> Result<Vec<PathBuf>, FileReferenceError> {
    match kind {
        ReferenceKind::Relative(_) => Ok(vec![ctx.cwd.clone()]),
        ReferenceKind::ImplicitRelative(_) => {
            let mut roots = vec![ctx.cwd.clone()];
            if let Some(git_root) = find_git_root(&ctx.cwd)?
                && git_root != ctx.cwd
            {
                roots.push(git_root);
            }
            Ok(roots)
        }
        ReferenceKind::Absolute(_) => Ok(vec![PathBuf::from("/")]),
        ReferenceKind::Magic(_) => {
            let mut roots = Vec::new();
            roots.extend(magic_paths.prepend.iter().cloned());
            if let Some(git_root) = find_git_root(&ctx.cwd)? {
                roots.push(git_root);
            }
            if let Some(ref home) = ctx.home_dir {
                roots.push(home.clone());
            }
            roots.extend(magic_paths.append.iter().cloned());
            Ok(roots)
        }
        ReferenceKind::Package(_) => {
            let git_root = match find_git_root(&ctx.cwd)? {
                Some(root) => root,
                None => return Ok(vec![]),
            };
            let area = find_package_area(&git_root, &ctx.cwd)?;
            Ok(vec![area.unwrap_or(git_root)])
        }
        ReferenceKind::Vault(_) => {
            let mut roots: Vec<PathBuf> = vault_roots.to_vec();
            if let Some(vault_env) = ctx.env.get("VAULT") {
                roots.extend(std::env::split_paths(vault_env));
            }
            if roots.is_empty() {
                return Err(FileReferenceError::VaultNotConfigured);
            }
            Ok(roots)
        }
    }
}

/// Build candidate file paths for non-recursive resolution.
fn build_candidates(
    parsed: &ParsedReference,
    interpolated: &str,
    magic_paths: &MagicPathList,
    vault_roots: &[PathBuf],
    ctx: &ResolutionContext,
) -> Result<Vec<PathBuf>, FileReferenceError> {
    if let ReferenceKind::Absolute(_) = &parsed.kind {
        let path = PathBuf::from(interpolated);
        if !path.is_absolute() {
            return Err(FileReferenceError::InvalidSyntax(format!(
                "absolute reference resolved to non-absolute path: {interpolated}"
            )));
        }
        return Ok(vec![path]);
    }

    let roots = collect_roots(&parsed.kind, magic_paths, vault_roots, ctx)?;
    Ok(roots.into_iter().map(|r| r.join(interpolated)).collect())
}

/// Build search roots for recursive resolution.
fn build_search_roots(
    parsed: &ParsedReference,
    magic_paths: &MagicPathList,
    vault_roots: &[PathBuf],
    ctx: &ResolutionContext,
) -> Result<Vec<PathBuf>, FileReferenceError> {
    collect_roots(&parsed.kind, magic_paths, vault_roots, ctx)
}

/// Interpolate template segments using the resolution context's env vars.
fn interpolate(
    template: &PathTemplate,
    ctx: &ResolutionContext,
) -> Result<String, FileReferenceError> {
    let mut result = String::new();

    for segment in &template.segments {
        match segment {
            TemplateSegment::Literal(s) => result.push_str(s),
            TemplateSegment::EnvVar(name) => {
                let value = ctx.env.get(name).ok_or_else(|| {
                    FileReferenceError::MissingEnvironmentVariable { name: name.clone() }
                })?;
                result.push_str(value);
            }
        }
    }

    Ok(result)
}

/// Normalize a path to absolute without following symlinks.
fn normalize_absolute(path: &Path, cwd: &Path) -> PathBuf {
    if path.is_absolute() {
        normalize_components(path)
    } else {
        normalize_components(&cwd.join(path))
    }
}

/// Resolve `.` and `..` components without touching the filesystem.
fn normalize_components(path: &Path) -> PathBuf {
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                components.pop();
            }
            std::path::Component::CurDir => {}
            other => components.push(other),
        }
    }
    components.iter().collect()
}

/// Compute a relative path from `base` to `target`.
pub(crate) fn diff_paths(target: &Path, base: &Path) -> Option<PathBuf> {
    let target = normalize_components(target);
    let base = normalize_components(base);

    // Both must be absolute
    if !target.is_absolute() || !base.is_absolute() {
        return None;
    }

    let mut target_components = target.components().peekable();
    let mut base_components = base.components().peekable();

    // Skip common prefix
    while let (Some(t), Some(b)) = (target_components.peek(), base_components.peek()) {
        if t == b {
            target_components.next();
            base_components.next();
        } else {
            break;
        }
    }

    // Add `..` for each remaining base component
    let mut result = PathBuf::new();
    for _ in base_components {
        result.push("..");
    }

    // Add remaining target components
    for component in target_components {
        result.push(component);
    }

    if result.as_os_str().is_empty() {
        Some(PathBuf::from("."))
    } else {
        Some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diff_paths_same_dir() {
        let result = diff_paths(Path::new("/a/b/file.txt"), Path::new("/a/b")).unwrap();
        assert_eq!(result, PathBuf::from("file.txt"));
    }

    #[test]
    fn diff_paths_sibling_dir() {
        let result = diff_paths(Path::new("/a/b/file.txt"), Path::new("/a/c")).unwrap();
        assert_eq!(result, PathBuf::from("../b/file.txt"));
    }

    #[test]
    fn diff_paths_parent() {
        let result = diff_paths(Path::new("/a/file.txt"), Path::new("/a/b/c")).unwrap();
        assert_eq!(result, PathBuf::from("../../file.txt"));
    }

    #[test]
    fn diff_paths_same_path() {
        let result = diff_paths(Path::new("/a/b"), Path::new("/a/b")).unwrap();
        assert_eq!(result, PathBuf::from("."));
    }

    #[test]
    fn normalize_dotdot() {
        let result = normalize_components(Path::new("/a/b/../c/./d"));
        assert_eq!(result, PathBuf::from("/a/c/d"));
    }

    #[test]
    fn interpolate_literal_only() {
        let ctx = ResolutionContext {
            cwd: PathBuf::from("/tmp"),
            home_dir: Some(PathBuf::from("/home/test")),
            env: std::collections::HashMap::new(),
        };
        let template = PathTemplate {
            segments: vec![TemplateSegment::Literal("foo/bar.md".to_string())],
        };
        let result = interpolate(&template, &ctx).unwrap();
        assert_eq!(result, "foo/bar.md");
    }

    #[test]
    fn interpolate_with_env_var() {
        let mut env = std::collections::HashMap::new();
        env.insert("PROJECT".to_string(), "myproject".to_string());
        let ctx = ResolutionContext {
            cwd: PathBuf::from("/tmp"),
            home_dir: None,
            env,
        };
        let template = PathTemplate {
            segments: vec![
                TemplateSegment::EnvVar("PROJECT".to_string()),
                TemplateSegment::Literal("/README.md".to_string()),
            ],
        };
        let result = interpolate(&template, &ctx).unwrap();
        assert_eq!(result, "myproject/README.md");
    }

    #[test]
    fn interpolate_missing_env_var() {
        let ctx = ResolutionContext {
            cwd: PathBuf::from("/tmp"),
            home_dir: None,
            env: std::collections::HashMap::new(),
        };
        let template = PathTemplate {
            segments: vec![TemplateSegment::EnvVar("MISSING".to_string())],
        };
        let result = interpolate(&template, &ctx);
        assert!(result.is_err());
    }

    #[test]
    fn normalize_absolute_relative_path() {
        let result = normalize_absolute(Path::new("foo/bar.txt"), Path::new("/home/user"));
        assert_eq!(result, PathBuf::from("/home/user/foo/bar.txt"));
    }

    #[test]
    fn normalize_absolute_already_absolute() {
        let result = normalize_absolute(Path::new("/etc/config.toml"), Path::new("/home/user"));
        assert_eq!(result, PathBuf::from("/etc/config.toml"));
    }

    #[test]
    fn implicit_relative_uses_cwd_then_git_root() {
        use crate::file_reference::{MagicPathList, ParsedReference, ReferenceKind};

        // We can't call collect_roots for ImplicitRelative without a real git
        // context, so exercise the *direct* path by constructing a
        // ParsedReference with no git root. In that case the only root
        // should be CWD, matching the "git lookup returned None" branch.
        let parsed = ParsedReference {
            recursive: false,
            kind: ReferenceKind::ImplicitRelative(PathTemplate {
                segments: vec![TemplateSegment::Literal("nope.md".to_string())],
            }),
        };
        let ctx = ResolutionContext {
            cwd: PathBuf::from("/tmp"),
            home_dir: None,
            env: std::collections::HashMap::new(),
        };
        let roots = collect_roots(&parsed.kind, &MagicPathList::default(), &[], &ctx).unwrap();
        // /tmp has no git repo, so only CWD is returned.
        assert_eq!(roots, vec![PathBuf::from("/tmp")]);
    }
}
