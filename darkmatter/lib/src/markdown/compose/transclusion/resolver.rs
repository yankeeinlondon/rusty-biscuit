//! Path and URL resolution for transclusion references.

use super::types::{DirectiveKind, ResolvedTarget, TransclusionError};
use crate::markdown::compose::util::document_resolution_context;
use crate::markdown::compose::{ComposeSource, TransclusionOptions};
use crate::markdown::compose::context::options::SourceDerivation;
use biscuit_file::{FileReference, FileReferenceError, FileReferenceKind};
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
    let file_ref = FileReference::new(raw_target)?;
    resolve_parsed_target(kind, &file_ref, options, source, line, ctx)
}

/// Resolves an already-parsed directive target into a canonical local path or URL.
#[instrument(skip_all, fields(target = %file_ref.raw(), kind = ?kind))]
pub(crate) fn resolve_parsed_target(
    kind: DirectiveKind,
    file_ref: &FileReference,
    options: &TransclusionOptions,
    source: &ComposeSource,
    line: usize,
    ctx: SourceContext,
) -> Result<ResolvedTarget, TransclusionError> {
    debug!("transclusion: resolving target");
    match (kind, file_ref.class().kind) {
        (DirectiveKind::Url, _) | (_, FileReferenceKind::Url) => {
            resolve_url_target(file_ref.raw(), options)
        }
        (DirectiveKind::File | DirectiveKind::Code, _) => {
            let path = resolve_file_reference(file_ref, kind, options, source, line, ctx)?;
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
/// source directory first then the repository root, `~`/`~/…` against the user's
/// home, and `@` (magic), `!` (package), `vault:`, `%` (recursive), absolute,
/// and `{{ENV}}` references by their existing `FileReference` semantics. There
/// is no ambient-CWD read (D2).
///
/// Source provenance controls context derivation. Ordinary sources retain
/// repository containment; only a child that already resolved outside that
/// boundary uses trusted-external derivation. The originating request boundary
/// remains validated in both cases.
pub(crate) fn resolve_path(
    raw_target: &str,
    kind: DirectiveKind,
    options: &TransclusionOptions,
    source: &ComposeSource,
    line: usize,
    ctx: SourceContext,
) -> Result<PathBuf, TransclusionError> {
    trace!(raw_target = %raw_target, "transclusion: resolving path");
    let file_ref = FileReference::new(raw_target)?;
    resolve_file_reference(&file_ref, kind, options, source, line, ctx)
}

fn resolve_file_reference(
    file_ref: &FileReference,
    kind: DirectiveKind,
    options: &TransclusionOptions,
    source: &ComposeSource,
    line: usize,
    ctx: SourceContext,
) -> Result<PathBuf, TransclusionError> {
    let raw_target = file_ref.raw();
    if file_ref.class().kind == FileReferenceKind::Url {
        return Err(TransclusionError::UnsupportedReferenceType {
            reference: raw_target.to_string(),
        });
    }

    // `@`-magic requires repo-root resolution to be enabled.
    if file_ref.class().kind == FileReferenceKind::Magic && !options.resolve_repo_root {
        return Err(TransclusionError::InvalidReference {
            ctx: Box::new(ctx),
            reference: raw_target.to_string(),
            line,
            directive_kind: kind,
        });
    }

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
        Some(snapshot) => match (source_file_path(source), options.source_derivation) {
            (Some(path), SourceDerivation::TrustedExternal) => {
                snapshot.for_trusted_external_source(path)
            }
            (Some(path), SourceDerivation::Ordinary) => snapshot.for_source(path),
            (None, _) => snapshot.for_base(&base_dir),
        },
        None => document_resolution_context(
            &base_dir,
            source_file_path(source).as_deref(),
            &options.magic_paths,
            None,
        ),
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

/// Classification of a frontmatter prologue or epilogue value.
#[derive(Debug)]
pub(crate) enum FrontmatterReference {
    /// Markdown content that should be emitted without file resolution.
    Inline,
    /// A syntactically valid file reference.
    Parsed(FileReference),
    /// A reference-shaped value rejected by the shared parser.
    ParseError(FileReferenceError),
}

/// Classifies a frontmatter prologue or epilogue value without suppressing
/// errors from the shared file-reference parser.
pub(crate) fn classify_frontmatter_reference(reference: &str) -> FrontmatterReference {
    if reference.is_empty() || reference.contains('\n') || reference.contains("](") {
        return FrontmatterReference::Inline;
    }

    let file_ref = match FileReference::new(reference) {
        Ok(file_ref) => file_ref,
        Err(error) => return FrontmatterReference::ParseError(error),
    };
    if file_ref.class().kind != FileReferenceKind::ImplicitRelative {
        return FrontmatterReference::Parsed(file_ref);
    }

    if reference.contains('/') || reference.contains('\\') || reference.contains("{{") {
        return FrontmatterReference::Parsed(file_ref);
    }

    // Bare filenames like "intro.md" should be treated as file-like for
    // frontmatter transclusion classification.
    if std::path::Path::new(reference)
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| !ext.is_empty() && !reference.chars().any(char::is_whitespace))
        .unwrap_or(false)
    {
        FrontmatterReference::Parsed(file_ref)
    } else {
        FrontmatterReference::Inline
    }
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

        let mut options = default_options();
        options.file_resolution_context = Some(
            crate::markdown::compose::capture_file_resolution_context(
                source_path.parent().expect("source parent"),
            ),
        );
        let resolved = resolve_path(
            "^shared.md",
            DirectiveKind::File,
            &options,
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

        let mut options = default_options();
        options.file_resolution_context = Some(
            crate::markdown::compose::capture_file_resolution_context(&nested),
        );

        let resolved = resolve_path(
            "@/shared.md",
            DirectiveKind::File,
            &options,
            &ComposeSource::File(source_path),
            1,
            dummy_ctx("# root"),
        );

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
    fn classifies_frontmatter_references() {
        for reference in [
            "./intro.md",
            "../shared/header.md",
            "@/docs/intro.md",
            "~/notes/intro.md",
            "/absolute/path.md",
            "intro.md",
            "^README.md",
            "%@docs/spec.md",
            "vault:notes/today.md",
            "{{CONFIG_DIR}}/app.toml",
            "HTTPS://example.com/intro.md",
        ] {
            assert!(matches!(
                classify_frontmatter_reference(reference),
                FrontmatterReference::Parsed(_)
            ));
        }
        assert!(matches!(
            classify_frontmatter_reference("!README.md"),
            FrontmatterReference::ParseError(_)
        ));
        for reference in ["Just some text content", "**Bold** markdown", ""] {
            assert!(matches!(
                classify_frontmatter_reference(reference),
                FrontmatterReference::Inline
            ));
        }

        // Inline content containing markdown links or newlines must not be
        // misidentified as file paths.
        for reference in [
            "---\n\n- No [animals](./animals.md) were hurt",
            "See [other](./other.md) for details",
            "Line one\nLine two",
        ] {
            assert!(matches!(
                classify_frontmatter_reference(reference),
                FrontmatterReference::Inline
            ));
        }

        for reference in ["@//escape.md", "%@//escape.md", "~alice/secret.md"] {
            assert!(matches!(
                classify_frontmatter_reference(reference),
                FrontmatterReference::ParseError(_)
            ));
        }
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

    #[test]
    fn file_and_code_directives_route_uppercase_https_through_remote_resolution() {
        let mut options = default_options();
        options.allow_remote = true;

        for kind in [DirectiveKind::File, DirectiveKind::Code] {
            let resolved = resolve_target(
                kind,
                "HTTPS://example.com/asset.md",
                &options,
                &ComposeSource::Unknown,
                1,
                dummy_ctx(""),
            )
            .unwrap();

            let ResolvedTarget::Url { url, .. } = resolved else {
                panic!("{kind:?} uppercase HTTPS target was not routed remotely");
            };
            assert_eq!(url.as_str(), "https://example.com/asset.md");
        }
    }

    #[test]
    fn mixed_case_https_still_honors_remote_execution_policy() {
        let err = resolve_target(
            DirectiveKind::File,
            "hTtPs://example.com/blocked.md",
            &default_options(),
            &ComposeSource::Unknown,
            1,
            dummy_ctx(""),
        )
        .unwrap_err();

        assert!(matches!(err, TransclusionError::UrlExecutionDisabled { .. }));
    }
}
