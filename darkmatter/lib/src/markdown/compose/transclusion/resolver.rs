//! Path and URL resolution for transclusion references.

use super::types::{DirectiveKind, ResolvedTarget, TransclusionError};
use crate::markdown::compose::{TransclusionOptions, ComposeSource};
use biscuit_file::FileReference;
use std::path::{Path, PathBuf};

/// Resolves a directive target into a canonical local path or URL.
pub fn resolve_target(
    kind: DirectiveKind,
    raw_target: &str,
    options: &TransclusionOptions,
    source: &ComposeSource,
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
///
/// Delegates to [`FileReference`] for `@` (magic/repo-root), `!` (package),
/// `vault:`, `%` (recursive), and `{{ENV}}` interpolation references.
/// Relative paths are resolved from the source file's directory (not CWD)
/// since transclusion context is file-relative.
pub fn resolve_path(
    raw_target: &str,
    options: &TransclusionOptions,
    source: &ComposeSource,
    line: usize,
) -> Result<PathBuf, TransclusionError> {
    if raw_target.starts_with("http://") || raw_target.starts_with("https://") {
        return Err(TransclusionError::UnsupportedReferenceType {
            reference: raw_target.to_string(),
        });
    }

    // Handle ~ (home directory) by converting to absolute path.
    // FileReference uses @ for magic refs (git root + HOME fallback),
    // but ~ should resolve directly to HOME without searching git root.
    if raw_target.starts_with('~') {
        let home = std::env::var("HOME").map_err(|_| TransclusionError::MissingSourceContext {
            reference: raw_target.to_string(),
            line,
        })?;
        let suffix = raw_target.trim_start_matches('~').trim_start_matches('/');
        return std::fs::canonicalize(Path::new(&home).join(suffix)).map_err(Into::into);
    }

    // Use FileReference for @, !, vault:, %, {{ENV}}, and absolute paths.
    if is_file_reference_target(raw_target) {
        if raw_target.starts_with('@') && !options.resolve_repo_root {
            return Err(TransclusionError::InvalidReference {
                reference: raw_target.to_string(),
                line,
            });
        }

        // Normalize @/ to @ — FileReference strips only the @ prefix,
        // so @/foo would leave /foo (absolute) which breaks the join.
        let normalized;
        let ref_input = if let Some(rest) = raw_target.strip_prefix("@/") {
            normalized = format!("@{rest}");
            &normalized
        } else {
            raw_target
        };

        let file_ref = FileReference::new(ref_input)?;
        let resolved = file_ref.resolve()?;

        return resolved.ok_or_else(|| {
            TransclusionError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("File not found: {raw_target}"),
            ))
        });
    }

    // Relative paths — resolve from the source file's directory.
    let raw = PathBuf::from(raw_target);

    if raw.is_absolute() {
        return std::fs::canonicalize(raw).map_err(Into::into);
    }

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
    std::fs::canonicalize(candidate).map_err(Into::into)
}

/// Returns `true` if the target should be routed through [`FileReference`]
/// rather than handled as a simple relative path.
fn is_file_reference_target(target: &str) -> bool {
    target.starts_with('@')
        || target.starts_with('!')
        || target.starts_with("vault:")
        || target.starts_with('%')
        || target.contains("{{")
}

fn source_file_path(source: &ComposeSource) -> Option<PathBuf> {
    match source {
        ComposeSource::File(path) => Some(path.clone()),
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

/// Attempts to infer whether a reference string is URL-like.
pub fn is_url_like(reference: &str) -> bool {
    reference.starts_with("http://") || reference.starts_with("https://")
}

/// Returns `true` if the reference looks like a filesystem path rather than
/// inline string content.  A reference is path-like when it contains a path
/// separator (`/` or `\`), starts with a file-reference prefix (`@`, `~`,
/// `!`, `%`), uses `vault:` syntax, or contains `{{` env-var interpolation.
pub fn is_file_like_reference(reference: &str) -> bool {
    if reference.contains('/')
        || reference.contains('\\')
        || reference.starts_with('@')
        || reference.starts_with('~')
        || reference.starts_with('!')
        || reference.starts_with('%')
        || reference.starts_with("vault:")
        || reference.contains("{{")
    {
        return true;
    }

    // Bare filenames like "intro.md" should be treated as file-like for
    // frontmatter transclusion classification.
    std::path::Path::new(reference)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| !ext.is_empty() && !reference.chars().any(char::is_whitespace))
        .unwrap_or(false)
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
            &ComposeSource::File(source_path),
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
            &ComposeSource::Unknown,
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
        // Canonicalize the tempdir root to resolve macOS /var -> /private/var symlink
        let root = std::fs::canonicalize(dir.path()).unwrap();

        // Initialize a real git repo so git2::Repository::discover() works
        git2::Repository::init(&root).unwrap();

        let nested = root.join("docs");
        std::fs::create_dir_all(&nested).unwrap();
        let source_path = nested.join("root.md");
        std::fs::write(&source_path, "# root").unwrap();

        let target_path = root.join("shared.md");
        std::fs::write(&target_path, "# shared").unwrap();

        // FileReference resolves @/ from CWD's git root, so we need to
        // temporarily change to the temp dir for this test.
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&nested).unwrap();

        let resolved = resolve_path(
            "@/shared.md",
            &default_options(),
            &ComposeSource::File(source_path),
            1,
        );

        std::env::set_current_dir(&original_dir).unwrap();

        let resolved = resolved.unwrap();
        assert_eq!(resolved, root.join("shared.md"));
    }

    #[test]
    fn repo_root_disabled_is_rejected() {
        let mut opts = default_options();
        opts.resolve_repo_root = false;

        let err = resolve_path("@/shared.md", &opts, &ComposeSource::Unknown, 1).unwrap_err();

        assert!(matches!(err, TransclusionError::InvalidReference { .. }));
    }

    #[test]
    fn classifies_file_like_references() {
        assert!(is_file_like_reference("./intro.md"));
        assert!(is_file_like_reference("../shared/header.md"));
        assert!(is_file_like_reference("@/docs/intro.md"));
        assert!(is_file_like_reference("~/notes/intro.md"));
        assert!(is_file_like_reference("/absolute/path.md"));
        assert!(is_file_like_reference("intro.md"));
        assert!(is_file_like_reference("!README.md"));
        assert!(is_file_like_reference("%@docs/spec.md"));
        assert!(is_file_like_reference("vault:notes/today.md"));
        assert!(is_file_like_reference("{{CONFIG_DIR}}/app.toml"));
        assert!(!is_file_like_reference("Just some text content"));
        assert!(!is_file_like_reference("**Bold** markdown"));
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
            &ComposeSource::File(source),
            1,
        )
        .unwrap_err();

        assert!(matches!(err, TransclusionError::UnsupportedFileType { .. }));
    }
}
