//! The one place a raw schema path captures its request context.
//!
//! A factory given a raw path (no bound context) calls [`request_context`]
//! once, at entry, before it resolves the path, reads the file, follows an
//! import, or validates anything. Everything after that works from the
//! returned snapshot, so a later change to the process's directory, home, or
//! environment cannot retarget the request. This mirrors how Darkmatter's
//! binaries capture a request: the process directory, home, and environment,
//! plus the repository root and package topology `sniff` observes there.

use std::path::Path;

use biscuit_file::{FileResolutionContext, PackageAreaFallback, RepositoryScopeCatalog, RepositoryScopeCatalogError};
use sniff::filesystem::repo::RepoInfo;

use super::SimplifiedSchemaError;

#[cfg(test)]
thread_local! {
    /// Captures made on this thread, for the context-boundary tests.
    static CAPTURES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Captures made on this thread so far.
#[cfg(test)]
pub(super) fn captures() -> usize {
    CAPTURES.with(std::cell::Cell::get)
}

/// A new request's context, captured from the running process.
///
/// ## Errors
///
/// [`SimplifiedSchemaError::Context`] when the current directory cannot be
/// read or the built context is invalid, and
/// [`SimplifiedSchemaError::Discovery`] when repository discovery fails (no
/// repository is not a failure).
pub(super) fn request_context() -> Result<FileResolutionContext, SimplifiedSchemaError> {
    #[cfg(test)]
    CAPTURES.with(|count| count.set(count.get() + 1));
    let cwd = std::env::current_dir()
        .map_err(|source| SimplifiedSchemaError::Context { source: biscuit_file::FileReferenceError::CurrentDirectory(source) })?;
    let mut context = FileResolutionContext::from_snapshot(&cwd, biscuit_file::home_dir(), biscuit_file::capture_env());
    let discovery = |error: sniff::SniffError| SimplifiedSchemaError::Discovery { dir: cwd.clone(), detail: error.to_string() };
    if let Some(root) = sniff::filesystem::git::repo_root(&cwd).map_err(discovery)? {
        let catalog = match sniff::filesystem::repo::detect_repo_structure(&root).map_err(discovery)? {
            Some(repo) => repository_scope_catalog(&repo, &root).ok(),
            None => RepositoryScopeCatalog::new(&root, Vec::new(), Vec::new(), PackageAreaFallback::None).ok(),
        };
        if let Some(catalog) = catalog {
            context = context.with_repository_scope_catalog(catalog);
        }
    }
    context.validate().map_err(|source| SimplifiedSchemaError::Context { source })?;
    Ok(context)
}

/// Projects a `sniff` repository observation into the file-reference scope
/// catalog: package and package-area roots under `repository_root`, with the
/// first-path-component area fallback for a monorepo.
///
/// `repository_root` is the request's spelling of the observed root; the
/// roots are rebuilt from repository-relative identities, so equivalent
/// canonical or symlinked spellings are never mixed. Darkmatter projects its
/// own observations through this same function.
///
/// ## Errors
///
/// The catalog's own validation failure.
pub fn repository_scope_catalog(
    repo: &RepoInfo,
    repository_root: &Path,
) -> Result<RepositoryScopeCatalog, RepositoryScopeCatalogError> {
    let packages = repo.packages.as_deref().unwrap_or_default();
    let package_roots = packages.iter().map(|package| repository_root.join(&package.relative)).collect();
    let package_area_roots = packages
        .iter()
        .filter(|package| !package.package_area.is_empty())
        .map(|package| repository_root.join(&package.package_area))
        .collect();
    let fallback = if repo.is_monorepo {
        PackageAreaFallback::FirstComponent
    } else {
        PackageAreaFallback::None
    };
    RepositoryScopeCatalog::new(repository_root, package_area_roots, package_roots, fallback)
}
