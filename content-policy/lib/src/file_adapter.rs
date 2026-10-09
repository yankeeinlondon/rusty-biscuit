//! The bundled `FileChanged` provider: reads watched files from the local
//! file system through Biscuit File's `FileReference`.
//!
//! The boundary is discovered for every request from its base directory: the
//! repository root containing it, else the document tree root (the directory
//! the process started in). `&` and `^` resolve against that boundary.

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};

use biscuit_file::{
    DetailedOutcome, FileReference, FileReferenceError, FileReferenceKind, FileResolutionContext,
    PackageAreaFallback, ProbeDisposition, RepositoryScopeCatalog, ResolutionFailure,
    canonicalize_simplified, find_git_root,
};

use crate::provider::{FileObservation, FileProvider, FileRequest};

/// Files that mark a package root for `^` references.
const PACKAGE_MANIFESTS: [&str; 2] = ["Cargo.toml", "package.json"];

/// Reads watched files from disk. Enabled by the `file-adapter` feature.
///
/// A request's base directory may be relative; it is resolved from the tree
/// root. Reports never show the paths the adapter resolves: they keep each
/// path as authored.
#[derive(Debug, Clone, Default)]
pub struct FileAdapter {
    tree_root: Option<PathBuf>,
}

impl FileAdapter {
    /// An adapter whose document tree root is the process's current
    /// directory, read at each request.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Replaces the document tree root: the directory that stands in for the
    /// repository root outside a repository, and that a relative base
    /// directory resolves from. A caller that knows the directory it started
    /// from, or a test that must not depend on the process's current
    /// directory, sets it here.
    #[must_use]
    pub fn with_tree_root(mut self, directory: impl Into<PathBuf>) -> Self {
        self.tree_root = Some(directory.into());
        self
    }

    fn tree_root(&self) -> io::Result<PathBuf> {
        match &self.tree_root {
            Some(root) => Ok(root.clone()),
            None => std::env::current_dir(),
        }
    }
}

impl FileProvider for FileAdapter {
    fn observe(&self, request: &FileRequest<'_>) -> FileObservation {
        self.resolve(request).unwrap_or_else(|observation| observation)
    }
}

/// The directories a request resolves against, in one canonical spelling so
/// that lexical containment compares like with like (macOS reports
/// `/private/var/…` for a temporary directory spelled `/var/…`).
struct Boundary {
    base: PathBuf,
    root: PathBuf,
    in_repository: bool,
}

impl FileAdapter {
    fn resolve(&self, request: &FileRequest<'_>) -> Result<FileObservation, FileObservation> {
        let reference = reference_for(request.path)?;
        let boundary = self.boundary(request.base_dir)?;
        let context = FileResolutionContext::from_snapshot(&boundary.base, None, HashMap::new())
            .with_repository_scope_catalog(scope_catalog(&boundary)?)
            .with_base_dir(&boundary.root);

        if reference.class().kind == FileReferenceKind::ExplicitRelative {
            let plan = reference
                .candidate_plan(&context)
                .map_err(|error| resolution_failure(&boundary, &error))?;
            for candidate in &plan {
                reference
                    .validate_contained_candidate(candidate.path(), &boundary.root)
                    .map_err(|error| match error {
                        FileReferenceError::BoundaryEscape {
                            escaped_candidate, ..
                        } => escape(&boundary, &escaped_candidate),
                        other => unreadable("the path cannot be checked", &other),
                    })?;
            }
        }

        let detailed = reference.resolve_detailed(&context);
        match detailed.outcome() {
            DetailedOutcome::Matched(path) => Ok(read(path)),
            DetailedOutcome::Failed(ResolutionFailure::NoMatch) => {
                let not_a_file = detailed
                    .candidates()
                    .iter()
                    .any(|probed| probed.disposition() == ProbeDisposition::NonFile);
                Ok(if not_a_file {
                    FileObservation::NotAFile
                } else {
                    FileObservation::Missing
                })
            }
            DetailedOutcome::Failed(_) => match detailed.error() {
                Some(error) => Err(resolution_failure(&boundary, error)),
                None => Err(FileObservation::Unreadable(
                    "the path cannot be resolved".to_string(),
                )),
            },
        }
    }

    fn boundary(&self, base_dir: &Path) -> Result<Boundary, FileObservation> {
        let tree_root = self
            .tree_root()
            .map_err(|error| unreadable("the current directory cannot be read", &error))?;
        let base = canonicalize_simplified(&tree_root.join(base_dir)).map_err(|error| {
            unreadable(&format!("the base directory `{}` cannot be read", base_dir.display()), &error)
        })?;
        let repository = find_git_root(&base)
            .map_err(|error| unreadable("the repository cannot be discovered", &error))?;
        let (root, in_repository) = match repository {
            Some(root) => (root, true),
            None => (tree_root, false),
        };
        let root = canonicalize_simplified(&root)
            .map_err(|error| unreadable("the boundary directory cannot be read", &error))?;
        if !base.starts_with(&root) {
            return Err(FileObservation::OutsideBoundary(format!(
                "the base directory `{}` is outside the document tree root `{}`",
                base.display(),
                root.display()
            )));
        }
        Ok(Boundary {
            base,
            root,
            in_repository,
        })
    }
}

/// Parses the authored path, rewriting an implicit relative path to `./`
/// so it resolves from the base directory only: Biscuit File would otherwise
/// retry a bare path at the repository root and watch a different file
/// without saying so.
fn reference_for(path: &str) -> Result<FileReference, FileObservation> {
    let rejected = |detail: String| {
        FileObservation::OutsideBoundary(format!("`{path}` is not a path the boundary admits ({detail})"))
    };
    let reference = FileReference::new(path).map_err(|error| rejected(error.to_string()))?;
    let class = reference.class();
    if class.recursive {
        return Err(rejected("a recursive search".to_string()));
    }
    match class.kind {
        FileReferenceKind::ImplicitRelative => {
            FileReference::new(&format!("./{path}")).map_err(|error| rejected(error.to_string()))
        }
        FileReferenceKind::ExplicitRelative
        | FileReferenceKind::RepositoryRoot
        | FileReferenceKind::RepositoryScoped => Ok(reference),
        other => Err(rejected(format!("{other:?}"))),
    }
}

/// The repository, package-area, and package roots `&` and `^` search.
///
/// In a repository, the package root is the nearest directory between the
/// base and the repository root that holds a package manifest, and the
/// package area is the first directory below the repository root. Outside a
/// repository the tree root stands in for the repository root, with no
/// package levels, so `^` falls back to it.
fn scope_catalog(boundary: &Boundary) -> Result<RepositoryScopeCatalog, FileObservation> {
    let (areas, packages) = if boundary.in_repository {
        let area = boundary
            .base
            .strip_prefix(&boundary.root)
            .ok()
            .and_then(|relative| relative.components().next())
            .map(|first| boundary.root.join(first.as_os_str()));
        let package = boundary
            .base
            .ancestors()
            .take_while(|directory| *directory != boundary.root)
            .find(|directory| {
                PACKAGE_MANIFESTS
                    .iter()
                    .any(|manifest| directory.join(manifest).is_file())
            })
            .map(Path::to_path_buf);
        (area.into_iter().collect(), package.into_iter().collect())
    } else {
        (Vec::new(), Vec::new())
    };
    RepositoryScopeCatalog::new(&boundary.root, areas, packages, PackageAreaFallback::None)
        .map_err(|error| {
            FileObservation::Unreadable(format!("the repository scope cannot be built: {error}"))
        })
}

fn read(path: &Path) -> FileObservation {
    match std::fs::read(path) {
        Ok(bytes) => FileObservation::Present(bytes),
        Err(error) if error.kind() == io::ErrorKind::NotFound => FileObservation::Missing,
        Err(error) => FileObservation::Unreadable(error.to_string()),
    }
}

fn escape(boundary: &Boundary, candidate: &Path) -> FileObservation {
    let what = if boundary.in_repository {
        "the repository root"
    } else {
        "the document tree root"
    };
    FileObservation::OutsideBoundary(format!(
        "`{}` is outside {what} `{}`",
        candidate.display(),
        boundary.root.display()
    ))
}

fn resolution_failure(boundary: &Boundary, error: &FileReferenceError) -> FileObservation {
    match error {
        FileReferenceError::RepositoryEscape {
            escaped_candidate, ..
        }
        | FileReferenceError::RelativeTreeEscape {
            candidate: escaped_candidate,
            ..
        } => escape(boundary, escaped_candidate),
        // A dangling symlink: its target does not exist.
        FileReferenceError::Io { source, .. } if source.kind() == io::ErrorKind::NotFound => {
            FileObservation::Missing
        }
        other => unreadable("the path cannot be resolved", other),
    }
}

fn unreadable(what: &str, error: &dyn std::fmt::Display) -> FileObservation {
    FileObservation::Unreadable(format!("{what}: {error}"))
}
