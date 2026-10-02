//! `GlobReference`: the pattern grammar, native order and ownership, the
//! relative boundary and file symlinks, literal versus class brackets, `%`
//! rebuilt on `take_first`, and the absence of filters. Every assertion goes
//! through the public API (`list_files`, `take_first`, `matches`, `roots`,
//! `FileReference`).
//!
//! The fixture is a Git repository `repo/` holding the package area
//! `repo/area` and the package `repo/area/pkg`, a home directory `home/`, and
//! `outside/` beside them, all in one canonical temporary directory.
//! Contexts are built with `from_snapshot`, so no test reads the live home
//! directory, environment, or current directory.

mod boundary;
mod grammar;
mod literal;
mod order;
mod unfiltered;

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use biscuit_file::{FileResolutionContext, GlobReference};
use tempfile::TempDir;

pub(crate) struct Fixture {
    _tmp: TempDir,
    pub root: PathBuf,
    pub repo: PathBuf,
    pub area: PathBuf,
    pub pkg: PathBuf,
    pub home: PathBuf,
}

impl Fixture {
    pub fn new() -> Self {
        let tmp = TempDir::new().unwrap();
        let root = dunce::canonicalize(tmp.path()).unwrap();
        let repo = root.join("repo");
        let area = repo.join("area");
        let pkg = area.join("pkg");
        let home = root.join("home");
        for dir in [&pkg, &home, &root.join("outside")] {
            fs::create_dir_all(dir).unwrap();
        }
        gix::init(&repo).expect("git init");
        Self {
            _tmp: tmp,
            root,
            repo,
            area,
            pkg,
            home,
        }
    }

    /// Write a file at `relative` below the fixture root and return its path.
    pub fn file(&self, relative: &str) -> PathBuf {
        let path = self.root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, relative).unwrap();
        path
    }

    /// A request context at `cwd` inside the repository, with the package
    /// and package area of the fixture and the fixture home.
    pub fn ctx(&self, cwd: &Path) -> FileResolutionContext {
        self.ctx_with_env(cwd, &[])
    }

    pub fn ctx_with_env(&self, cwd: &Path, env: &[(&str, &str)]) -> FileResolutionContext {
        snapshot(cwd, Some(&self.home), env)
            .with_repository_root(&self.repo)
            .with_package_area(&self.area)
            .with_package_root(&self.pkg)
    }
}

/// A context that reads no ambient HOME or environment.
pub(crate) fn snapshot(
    cwd: &Path,
    home: Option<&Path>,
    env: &[(&str, &str)],
) -> FileResolutionContext {
    let env: HashMap<String, String> =
        env.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
    FileResolutionContext::from_snapshot(cwd, home.map(Path::to_path_buf), env)
}

pub(crate) fn glob(patterns: &[&str]) -> GlobReference {
    GlobReference::new(patterns).unwrap()
}

/// The matches of `patterns` in `ctx`, in native order.
pub(crate) fn listed(patterns: &[&str], ctx: &FileResolutionContext) -> Vec<PathBuf> {
    glob(patterns).list_files(ctx).unwrap().matches
}

/// A directory link that needs no privilege: a symlink on Unix, a junction on
/// Windows.
pub(crate) fn link_dir(target: &Path, link: &Path) {
    #[cfg(unix)]
    std::os::unix::fs::symlink(target, link).unwrap();
    #[cfg(windows)]
    {
        // `cmd` reads a `/` inside an argument as a switch (`docs/shared`
        // becomes `/shared`), so both paths are re-spelled natively.
        let native = |path: &Path| path.components().collect::<PathBuf>();
        let status = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(native(link))
            .arg(native(target))
            .stdout(std::process::Stdio::null())
            .status()
            .unwrap();
        assert!(status.success(), "mklink /J failed");
    }
}

/// A file symlink. On Windows this needs Developer Mode or
/// `SeCreateSymbolicLinkPrivilege`, which the repository's Windows hosts have.
pub(crate) fn link_file(target: &Path, link: &Path) {
    #[cfg(unix)]
    std::os::unix::fs::symlink(target, link).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(target, link)
        .expect("creating a file symlink needs Developer Mode or SeCreateSymbolicLinkPrivilege");
}
