//! `PortablePath`: strategies, environment anchors, reference inputs,
//! verification, and diagnostics, observed through `PortableReference` and
//! `PortablePathError` only.
//!
//! Every fixture lives in one canonical temporary directory with a repository
//! (`repo/`), a home directory (`home/`), and room for trees outside both.
//! Contexts are built with `from_snapshot`, so no test reads the live home
//! directory or environment.

mod configuration;
mod environment;
mod inputs;
mod platform;
mod properties;
mod strategies;

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use biscuit_file::{
    Attempt, AttemptOutcome, FileReference, FileResolutionContext, NotApplicable, PathIdentity,
    PortablePath, PortablePathError, PortableReference,
};
use tempfile::TempDir;

pub(crate) struct Fixture {
    _tmp: TempDir,
    pub root: PathBuf,
    pub repo: PathBuf,
    pub home: PathBuf,
}

impl Fixture {
    pub fn new() -> Self {
        let tmp = TempDir::new().unwrap();
        // Canonical, so the boundary's real-landing check sees the same
        // spelling as the lexical one (macOS `/var` is `/private/var`).
        let root = dunce::canonicalize(tmp.path()).unwrap();
        let repo = root.join("repo");
        let home = root.join("home");
        fs::create_dir_all(&repo).unwrap();
        fs::create_dir_all(&home).unwrap();
        Self {
            _tmp: tmp,
            root,
            repo,
            home,
        }
    }

    /// The native path of `rel` (`/`-separated) below the fixture root.
    ///
    /// Joined name by name: a `/` is not a separator under a Windows `\\?\`
    /// prefix, and `mklink /J` rejects it.
    pub fn path(&self, rel: &str) -> PathBuf {
        rel.split('/').fold(self.root.clone(), |path, name| path.join(name))
    }

    /// Create `rel` (below the fixture root) as a file and return its path.
    pub fn file(&self, rel: &str) -> PathBuf {
        let path = self.path(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, rel).unwrap();
        path
    }

    /// Create `rel` (below the fixture root) as a directory and return it.
    pub fn dir(&self, rel: &str) -> PathBuf {
        let path = self.path(rel);
        fs::create_dir_all(&path).unwrap();
        path
    }

    /// A repository context at `cwd` with the fixture home and `env`.
    pub fn ctx(&self, cwd: &Path, env: &[(&str, &str)]) -> FileResolutionContext {
        self.bare_ctx(cwd, env).with_repository_root(&self.repo)
    }

    /// A context with no repository.
    pub fn bare_ctx(&self, cwd: &Path, env: &[(&str, &str)]) -> FileResolutionContext {
        fs::create_dir_all(cwd).unwrap();
        FileResolutionContext::from_snapshot(cwd, Some(self.home.clone()), env_map(env))
    }
}

pub(crate) fn env_map(env: &[(&str, &str)]) -> HashMap<String, String> {
    env.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
}

pub(crate) fn reference(raw: &str) -> FileReference {
    FileReference::new(raw).unwrap_or_else(|error| panic!("`{raw}`: {error}"))
}

/// Evaluate, panicking with the full diagnostic on error.
pub(crate) fn evaluate(portable: PortablePath) -> PortableReference {
    portable.file_reference().unwrap_or_else(|error| panic!("{error}"))
}

pub(crate) fn evaluate_err(portable: PortablePath) -> PortablePathError {
    match portable.file_reference() {
        Ok(found) => panic!(
            "expected an error, got `{}` from {}",
            found.reference().raw(),
            found.strategy()
        ),
        Err(error) => error,
    }
}

/// The attempt made for `strategy`, which must exist.
pub(crate) fn attempt_for<'a>(
    attempts: &'a [Attempt],
    strategy: &biscuit_file::PortabilityPreference,
) -> &'a Attempt {
    attempts
        .iter()
        .find(|attempt| &attempt.strategy == strategy)
        .unwrap_or_else(|| panic!("no attempt for {strategy}"))
}

pub(crate) fn not_applicable(attempt: &Attempt) -> &NotApplicable {
    match &attempt.outcome {
        AttemptOutcome::NotApplicable(reason) => reason,
        other => panic!("{}: expected not applicable, got {other}", attempt.strategy),
    }
}

/// The absolute reference text the library writes for `path`.
pub(crate) fn absolute_text(path: &Path) -> String {
    biscuit_file::to_portable_string(path)
}

pub(crate) fn same_file(left: &Path, right: &Path) -> bool {
    PathIdentity::new(left) == PathIdentity::new(right)
}
