//! Path and URL resolution for transclusion references.

use super::types::{DirectiveKind, ResolvedTarget, TransclusionError};
use crate::markdown::transform::{TransclusionOptions, TransformSource};
use std::path::{Path, PathBuf};

/// Resolves a directive target into a canonical local path or URL.
pub fn resolve_target(
    kind: DirectiveKind,
    raw_target: &str,
    options: &TransclusionOptions,
    source: &TransformSource,
    line: usize,
) -> Result<ResolvedTarget, TransclusionError> {
    match kind {
        DirectiveKind::Url => resolve_url_target(raw_target, options),
        DirectiveKind::File | DirectiveKind::Code => {
            let path = resolve_path(raw_target, options, source, line)?;
            validate_local_target(kind, &path, options)?;
            Ok(ResolvedTarget::File {
                id: path.to_string_lossy().to_string(),
                path,
            })
        }
    }
}

fn resolve_url_target(
    raw_target: &str,
    options: &TransclusionOptions,
) -> Result<ResolvedTarget, TransclusionError> {
    let url = url::Url::parse(raw_target)?;
    if !options.allow_remote {
        return Err(TransclusionError::UrlExecutionDisabled {
            url: url.to_string(),
        });
    }

    Ok(ResolvedTarget::Url {
        id: url.to_string(),
        url,
    })
}

/// Resolves a local filesystem path.
pub fn resolve_path(
    raw_target: &str,
    options: &TransclusionOptions,
    source: &TransformSource,
    line: usize,
) -> Result<PathBuf, TransclusionError> {
    if raw_target.starts_with("http://") || raw_target.starts_with("https://") {
        return Err(TransclusionError::UnsupportedReferenceType {
            reference: raw_target.to_string(),
        });
    }

    let raw = PathBuf::from(raw_target);

    let absolute = if raw_target.starts_with("@") {
        if !options.resolve_repo_root {
            return Err(TransclusionError::InvalidReference {
                reference: raw_target.to_string(),
                line,
            });
        }

        let source_file =
            source_file_path(source).ok_or_else(|| TransclusionError::MissingSourceContext {
                reference: raw_target.to_string(),
                line,
            })?;

        let repo_root =
            find_repo_root(&source_file).ok_or_else(|| TransclusionError::InvalidReference {
                reference: raw_target.to_string(),
                line,
            })?;

        let rel = raw_target
            .strip_prefix("@/")
            .or_else(|| raw_target.strip_prefix('@'))
            .unwrap_or(raw_target);
        let joined = repo_root.join(rel);

        let canonical_repo = std::fs::canonicalize(&repo_root)?;
        let canonical_path = std::fs::canonicalize(&joined)?;

        if !canonical_path.starts_with(&canonical_repo) {
            return Err(TransclusionError::InvalidReference {
                reference: raw_target.to_string(),
                line,
            });
        }

        canonical_path
    } else if raw_target.starts_with("~") {
        let home = std::env::var("HOME").map_err(|_| TransclusionError::MissingSourceContext {
            reference: raw_target.to_string(),
            line,
        })?;
        let suffix = raw_target.trim_start_matches('~').trim_start_matches('/');
        std::fs::canonicalize(Path::new(&home).join(suffix))?
    } else if raw.is_absolute() {
        std::fs::canonicalize(raw)?
    } else {
        let source_file =
            source_file_path(source).ok_or_else(|| TransclusionError::MissingSourceContext {
                reference: raw_target.to_string(),
                line,
            })?;
        let base_dir = if source_file.is_dir() {
            source_file
        } else {
            source_file.parent().map(Path::to_path_buf).ok_or_else(|| {
                TransclusionError::MissingSourceContext {
                    reference: raw_target.to_string(),
                    line,
                }
            })?
        };

        let candidate = if raw_target.starts_with("./") || raw_target.starts_with("../") {
            base_dir.join(&raw)
        } else {
            base_dir.join(Path::new(raw_target))
        };
        std::fs::canonicalize(candidate)?
    };

    Ok(absolute)
}

fn source_file_path(source: &TransformSource) -> Option<PathBuf> {
    match source {
        TransformSource::File(path) => Some(path.clone()),
        _ => None,
    }
}

fn validate_local_target(
    kind: DirectiveKind,
    path: &Path,
    options: &TransclusionOptions,
) -> Result<(), TransclusionError> {
    match kind {
        DirectiveKind::File => {
            if !options.allow_local_markdown {
                return Err(TransclusionError::UnsupportedReferenceType {
                    reference: path.to_string_lossy().to_string(),
                });
            }

            if !is_markdown_path(path) {
                return Err(TransclusionError::UnsupportedFileType {
                    path: path.to_path_buf(),
                });
            }
        }
        DirectiveKind::Code => {
            if !options.allow_local_code_text {
                return Err(TransclusionError::UnsupportedReferenceType {
                    reference: path.to_string_lossy().to_string(),
                });
            }
        }
        DirectiveKind::Url => {}
    }

    Ok(())
}

fn is_markdown_path(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        return false;
    };

    matches!(ext.to_ascii_lowercase().as_str(), "md" | "markdown")
}

fn find_repo_root(start: &Path) -> Option<PathBuf> {
    let mut current = if start.is_dir() {
        start.to_path_buf()
    } else {
        start.parent()?.to_path_buf()
    };

    loop {
        if current.join(".git").exists() {
            return Some(current);
        }

        if !current.pop() {
            return None;
        }
    }
}

/// Attempts to infer whether a reference string is URL-like.
pub fn is_url_like(reference: &str) -> bool {
    reference.starts_with("http://") || reference.starts_with("https://")
}

/// Returns `true` if the reference looks like a filesystem path rather than
/// inline string content.  A reference is path-like when it contains a path
/// separator (`/` or `\`) or starts with a special path prefix (`@` or `~`).
pub fn is_file_like_reference(reference: &str) -> bool {
    reference.contains('/')
        || reference.contains('\\')
        || reference.starts_with('@')
        || reference.starts_with('~')
}

/// Attempts to normalize relative reference tokens before parsing.
pub fn normalize_reference_token(raw: &str) -> String {
    if raw.starts_with('@') {
        // Collapse @./foo and @foo into @/foo for stable semantics.
        let trimmed = raw.trim_start_matches('@').trim_start_matches('/');
        return format!("@/{trimmed}");
    }

    raw.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    fn default_options() -> TransclusionOptions {
        TransclusionOptions::default()
    }

    #[test]
    fn resolves_relative_from_source_file() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let source_path = root.join("root.md");
        let child_path = root.join("child.md");

        std::fs::write(&source_path, "# root").unwrap();
        std::fs::write(&child_path, "# child").unwrap();

        let resolved = resolve_path(
            "./child.md",
            &default_options(),
            &TransformSource::File(source_path),
            1,
        )
        .unwrap();

        assert_eq!(resolved, std::fs::canonicalize(&child_path).unwrap());
    }

    #[test]
    fn relative_requires_source_context() {
        let err = resolve_path(
            "./child.md",
            &default_options(),
            &TransformSource::Unknown,
            2,
        )
        .unwrap_err();
        assert!(matches!(
            err,
            TransclusionError::MissingSourceContext { .. }
        ));
    }

    #[test]
    fn resolves_repo_root_reference() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir(root.join(".git")).unwrap();

        let nested = root.join("docs");
        std::fs::create_dir_all(&nested).unwrap();
        let source_path = nested.join("root.md");
        std::fs::write(&source_path, "# root").unwrap();

        let target_path = root.join("shared.md");
        std::fs::write(&target_path, "# shared").unwrap();

        let resolved = resolve_path(
            "@/shared.md",
            &default_options(),
            &TransformSource::File(source_path),
            1,
        )
        .unwrap();

        assert_eq!(resolved, std::fs::canonicalize(target_path).unwrap());
    }

    #[test]
    fn repo_root_escape_is_rejected() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir(root.join(".git")).unwrap();

        let source_path = root.join("root.md");
        std::fs::write(&source_path, "# root").unwrap();

        let outside = root.parent().unwrap().join("outside.md");
        let mut file = std::fs::File::create(&outside).unwrap();
        writeln!(file, "# outside").unwrap();

        let err = resolve_path(
            "@/../outside.md",
            &default_options(),
            &TransformSource::File(source_path),
            1,
        )
        .unwrap_err();

        assert!(matches!(err, TransclusionError::InvalidReference { .. }));
    }

    #[test]
    fn classifies_file_like_references() {
        assert!(is_file_like_reference("./intro.md"));
        assert!(is_file_like_reference("../shared/header.md"));
        assert!(is_file_like_reference("@/docs/intro.md"));
        assert!(is_file_like_reference("~/notes/intro.md"));
        assert!(is_file_like_reference("/absolute/path.md"));
        assert!(!is_file_like_reference("Just some text content"));
        assert!(!is_file_like_reference("**Bold** markdown"));
        assert!(!is_file_like_reference("intro.md")); // no path separator → inline
        assert!(!is_file_like_reference(""));
    }

    #[test]
    fn file_directive_requires_markdown_extension() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let source = root.join("root.md");
        let code = root.join("main.rs");
        std::fs::write(&source, "# root").unwrap();
        std::fs::write(&code, "fn main() {}").unwrap();

        let err = resolve_target(
            DirectiveKind::File,
            "./main.rs",
            &default_options(),
            &TransformSource::File(source),
            1,
        )
        .unwrap_err();

        assert!(matches!(err, TransclusionError::UnsupportedFileType { .. }));
    }
}
