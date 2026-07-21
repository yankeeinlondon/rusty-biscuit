//! Path and URL resolution for transclusion references.

use super::types::{DirectiveKind, ResolvedTarget, TransclusionError};
use crate::markdown::compose::util::{
    document_resolution_context, find_git_root_from, find_package_area_from,
};
use crate::markdown::compose::{ComposeSource, TransclusionOptions};
use biscuit_file::{FileReference, FileReferenceKind};
use biscuit_terminal::errors::SourceContext;
use std::path::{Path, PathBuf};
use tracing::{debug, instrument, trace};

/// Resolves a directive target into a canonical local path or URL.
#[instrument(skip_all, fields(target = %raw_target, kind = ?kind))]
pub(crate) fn resolve_target(
    kind: DirectiveKind,
    raw_target: &str,
    options: &TransclusionOptions,
    source: &ComposeSource,
    line: usize,
    ctx: SourceContext,
) -> Result<ResolvedTarget, TransclusionError> {
    debug!("transclusion: resolving target");
    match kind {
        DirectiveKind::Url => resolve_url_target(raw_target, options),
        // `::file`/`::code` accept HTTP(S) targets too; route those to the URL
        // resolver so remote transclusion works through the same directives as
        // local paths.
        DirectiveKind::File | DirectiveKind::Code if is_url_like(raw_target) => {
            resolve_url_target(raw_target, options)
        }
        DirectiveKind::File | DirectiveKind::Code => {
            let path = resolve_path(raw_target, kind, options, source, line, ctx)?;
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
    trace!(url = %raw_target, "transclusion: resolving URL target");
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
/// Every non-URL target is parsed by [`FileReference`] and resolved through the
/// shared document-backed context ([`document_resolution_context`]): explicit
/// `./`/`../` from the source document's directory only, implicit bare paths
/// repository-root first then the source directory, `~`/`~/…` against the user's
/// home, and `@` (magic), `!` (package), `vault:`, `%` (recursive), absolute,
/// and `{{ENV}}` references by their existing `FileReference` semantics. There
/// is no ambient-CWD read (D2).
///
/// A file-backed `source` has already been accepted as the top-level document
/// or by a parent transclusion, so deriving its context uses the explicit
/// trusted-external boundary while retaining validation of the request root.
pub(crate) fn resolve_path(
    raw_target: &str,
    kind: DirectiveKind,
    options: &TransclusionOptions,
    source: &ComposeSource,
    line: usize,
    ctx: SourceContext,
) -> Result<PathBuf, TransclusionError> {
    trace!(raw_target = %raw_target, "transclusion: resolving path");
    if raw_target.starts_with("http://") || raw_target.starts_with("https://") {
        return Err(TransclusionError::UnsupportedReferenceType {
            reference: raw_target.to_string(),
        });
    }

    // `@`-magic requires repo-root resolution to be enabled.
    if raw_target.starts_with('@') && !options.resolve_repo_root {
        return Err(TransclusionError::InvalidReference {
            ctx: Box::new(ctx),
            reference: raw_target.to_string(),
            line,
            directive_kind: kind,
        });
    }

    // Normalize @/ to @ — FileReference strips only the leading @, so @/foo
    // would leave /foo (absolute) which breaks the magic-root join.
    let normalized;
    let ref_input = if let Some(rest) = raw_target.strip_prefix("@/") {
        normalized = format!("@{rest}");
        normalized.as_str()
    } else {
        raw_target
    };

    let file_ref = FileReference::new(ref_input)?;

    // Absolute, home, and URL references do not need a document base; every
    // other kind (explicit/implicit relative, magic, package, vault) resolves
    // against the source document's directory, so a file-backed source is
    // required for them.
    let needs_base = !matches!(
        file_ref.class().kind,
        FileReferenceKind::Absolute | FileReferenceKind::Home | FileReferenceKind::Url
    );
    let base_dir = match source_file_dir(source) {
        Some(dir) => dir,
        None if needs_base => {
            return Err(TransclusionError::MissingSourceContext {
                reference: raw_target.to_string(),
                line,
            });
        }
        // Absolute/home references ignore the base; a neutral one anchors the
        // context without reading the ambient CWD for candidate construction.
        None => PathBuf::from("."),
    };
    let resolution_ctx = match options.file_resolution_context.as_ref() {
        Some(snapshot) => match source_file_path(source) {
            Some(path) => snapshot.for_trusted_external_source(path),
            None => snapshot.for_base(&base_dir),
        },
        None => {
            let repo_root = find_git_root_from(&base_dir);
            let package_area = find_package_area_from(&base_dir, repo_root.as_deref());
            document_resolution_context(
                &base_dir,
                source_file_path(source).as_deref(),
                &options.magic_paths,
                repo_root.as_deref(),
                package_area.as_deref(),
            )
        }
    };

    let path = file_ref.resolve_in_context(&resolution_ctx)?.ok_or_else(|| {
        TransclusionError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("File not found: {raw_target}"),
        ))
    })?;
    // Canonicalize so downstream transclusion identity (and macOS
    // `/var`→`/private/var` symlinks) match historical behavior.
    let canonical = std::fs::canonicalize(&path).map_err(|e| {
        TransclusionError::Io(std::io::Error::new(
            e.kind(),
            format!("'{}' (resolved to '{}'): {e}", raw_target, path.display()),
        ))
    })?;
    debug!(resolved = %canonical.display(), "transclusion: path resolved");
    Ok(canonical)
}

/// The directory a file-backed source's references resolve against: the source
/// file's parent, or the source path itself when it is already a directory.
fn source_file_dir(source: &ComposeSource) -> Option<PathBuf> {
    let source_file = source_file_path(source)?;
    if source_file.is_dir() {
        Some(source_file)
    } else {
        source_file.parent().map(Path::to_path_buf)
    }
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
    // Multi-line content is inline markdown, never a file path.
    if reference.contains('\n') {
        return false;
    }

    // Markdown link syntax (`](`) indicates inline content, not a path.
    if reference.contains("](") {
        return false;
    }

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
    use serial_test::serial;
    use tempfile::tempdir;

    fn default_options() -> TransclusionOptions {
        TransclusionOptions::default()
    }

    fn dummy_ctx(content: &str) -> SourceContext {
        SourceContext::new(PathBuf::from("/test.md"), PathBuf::from("test.md"), content)
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
            DirectiveKind::File,
            &default_options(),
            &ComposeSource::File(source_path.clone()),
            1,
            dummy_ctx("# root"),
        )
        .unwrap();

        assert_eq!(resolved, std::fs::canonicalize(&child_path).unwrap());
    }

    #[test]
    #[serial]
    fn transclusion_reuses_snapshot_environment_for_nested_source() {
        let request = tempdir().unwrap();
        let nested = request.path().join("nested");
        std::fs::create_dir_all(&nested).unwrap();
        let target = request.path().join("captured.md");
        std::fs::write(&target, "# captured").unwrap();
        let ambient = tempdir().unwrap();
        let prior = std::env::var_os("DARKMATTER_TRANSCLUSION_ROOT");

        let mut env = std::collections::HashMap::new();
        env.insert(
            "DARKMATTER_TRANSCLUSION_ROOT".to_string(),
            request.path().display().to_string(),
        );
        let mut options = default_options();
        options.file_resolution_context = Some(
            biscuit_file::FileResolutionContext::new(request.path()).with_env(env),
        );
        // SAFETY: this test is serialized while mutating process-global state.
        unsafe { std::env::set_var("DARKMATTER_TRANSCLUSION_ROOT", ambient.path()) };
        let resolved = resolve_path(
            "{{DARKMATTER_TRANSCLUSION_ROOT}}/captured.md",
            DirectiveKind::File,
            &options,
            &ComposeSource::File(nested.join("child.md")),
            1,
            dummy_ctx("# child"),
        );
        match prior {
            Some(value) => unsafe { std::env::set_var("DARKMATTER_TRANSCLUSION_ROOT", value) },
            None => unsafe { std::env::remove_var("DARKMATTER_TRANSCLUSION_ROOT") },
        }

        assert_eq!(resolved.unwrap(), std::fs::canonicalize(target).unwrap());
    }

    #[test]
    fn package_reference_transclusion_prefers_package_area_over_repository() {
        let dir = tempdir().unwrap();
        let root = std::fs::canonicalize(dir.path()).unwrap();
        gix::init(&root).unwrap();
        std::fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"darkmatter/lib\"]\nresolver = \"2\"\n",
        )
        .unwrap();
        let package_area = root.join("darkmatter");
        let member = package_area.join("lib");
        std::fs::create_dir_all(member.join("docs")).unwrap();
        std::fs::write(
            member.join("Cargo.toml"),
            "[package]\nname = \"fixture-darkmatter\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        )
        .unwrap();
        std::fs::create_dir_all(member.join("src")).unwrap();
        std::fs::write(member.join("src/lib.rs"), "").unwrap();
        let source_path = member.join("docs/root.md");
        std::fs::write(&source_path, "# root").unwrap();
        std::fs::write(root.join("shared.md"), "# repository decoy").unwrap();
        let package_target = package_area.join("shared.md");
        std::fs::write(&package_target, "# package").unwrap();

        let resolved = resolve_path(
            "!shared.md",
            DirectiveKind::File,
            &default_options(),
            &ComposeSource::File(source_path),
            1,
            dummy_ctx("# root"),
        )
        .unwrap();

        assert_eq!(resolved, std::fs::canonicalize(package_target).unwrap());
    }

    #[test]
    fn relative_requires_source_context() {
        let err = resolve_path(
            "./child.md",
            DirectiveKind::File,
            &default_options(),
            &ComposeSource::Unknown,
            2,
            dummy_ctx(""),
        )
        .unwrap_err();
        assert!(matches!(
            err,
            TransclusionError::MissingSourceContext { .. }
        ));
    }

    #[test]
    #[serial]
    fn resolves_repo_root_reference() {
        let dir = tempdir().unwrap();
        // Canonicalize the tempdir root to resolve macOS /var -> /private/var symlink
        let root = std::fs::canonicalize(dir.path()).unwrap();

        // Initialize a real git repo so repo-root discovery works
        gix::init(&root).unwrap();

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
            DirectiveKind::File,
            &default_options(),
            &ComposeSource::File(source_path),
            1,
            dummy_ctx("# root"),
        );

        std::env::set_current_dir(&original_dir).unwrap();

        let resolved = resolved.unwrap();
        assert_eq!(resolved, root.join("shared.md"));
    }

    #[test]
    fn repo_root_disabled_is_rejected() {
        let mut opts = default_options();
        opts.resolve_repo_root = false;

        let err = resolve_path(
            "@/shared.md",
            DirectiveKind::File,
            &opts,
            &ComposeSource::Unknown,
            1,
            dummy_ctx(""),
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
        assert!(is_file_like_reference("intro.md"));
        assert!(is_file_like_reference("!README.md"));
        assert!(is_file_like_reference("%@docs/spec.md"));
        assert!(is_file_like_reference("vault:notes/today.md"));
        assert!(is_file_like_reference("{{CONFIG_DIR}}/app.toml"));
        assert!(!is_file_like_reference("Just some text content"));
        assert!(!is_file_like_reference("**Bold** markdown"));
        assert!(!is_file_like_reference(""));

        // Inline content containing markdown links or newlines must not be
        // misidentified as file paths.
        assert!(!is_file_like_reference(
            "---\n\n- No [animals](./animals.md) were hurt"
        ));
        assert!(!is_file_like_reference(
            "See [other](./other.md) for details"
        ));
        assert!(!is_file_like_reference("Line one\nLine two"));
    }

    #[test]
    #[serial]
    fn resolves_magic_path_prepended() {
        let dir = tempdir().unwrap();
        let root = std::fs::canonicalize(dir.path()).unwrap();

        // Create a custom magic-path directory with a target file
        let magic_dir = root.join("custom-root");
        std::fs::create_dir_all(&magic_dir).unwrap();
        let target_path = magic_dir.join("special.md");
        std::fs::write(&target_path, "# Special").unwrap();

        // Create source file
        let source_path = root.join("root.md");
        std::fs::write(&source_path, "# root").unwrap();

        // Initialize a git repo so FileReference works
        gix::init(&root).unwrap();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&root).unwrap();

        let mut opts = default_options();
        opts.magic_paths
            .push((magic_dir.clone(), biscuit_file::PathPosition::Start));

        let resolved = resolve_path(
            "@/special.md",
            DirectiveKind::File,
            &opts,
            &ComposeSource::File(source_path),
            1,
            dummy_ctx("# root"),
        );

        std::env::set_current_dir(&original_dir).unwrap();

        let resolved = resolved.unwrap();
        assert_eq!(resolved, std::fs::canonicalize(&target_path).unwrap());
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
            dummy_ctx("# root"),
        )
        .unwrap_err();

        assert!(matches!(err, TransclusionError::UnsupportedFileType { .. }));
    }
}
