//! Project-local bin directory search.
//!
//! Test runners (and a few other ecosystem tools) are commonly installed under
//! the project tree rather than on the global `PATH`. `node_modules/.bin`,
//! PHP `vendor/bin`, Ruby `bin`, and Python virtualenv `bin` directories hold
//! executables that are runnable from the project but invisible to a bare
//! `which` lookup.
//!
//! [`LocalBinIndex`] resolves a binary name against the ecosystem-specific bin
//! roots relative to a starting directory. It is intentionally cwd-sensitive:
//! the caller supplies the directory to search from (typically
//! `std::env::current_dir()`), and the index walks up the tree for ecosystems
//! that support hoisted installs (notably Node).

use std::path::{Path, PathBuf};

use tracing::instrument;

use crate::programs::test_runner_spec::TestRunnerEcosystem;

/// Suffix appended to binary names on Windows.
#[cfg(windows)]
const WINDOWS_EXE: &str = ".exe";

/// Suffix appended to binary names on Windows when wrapping with `cmd`.
#[cfg(windows)]
const WINDOWS_CMD: &str = ".cmd";

/// Suffix appended to binary names on Windows when wrapping with PowerShell.
#[cfg(windows)]
const WINDOWS_PS1: &str = ".ps1";

/// Per-ecosystem project-local bin search roots.
///
/// Roots are repo-relative path templates. Each entry is a directory that
/// exists *under* a project root (e.g. `node_modules/.bin`). The caller walks
/// up from the start dir and probes the template at each ancestor.
#[derive(Debug, Clone, Copy)]
pub struct LocalBinRoot {
    /// Ecosystem that owns this root.
    pub ecosystem: TestRunnerEcosystem,
    /// Repo-relative bin directory (e.g. `"node_modules/.bin"`).
    pub relative: &'static str,
    /// Whether the search should walk up the directory tree from the start dir
    /// to the filesystem root. Node uses walk-up to catch hoisted installs;
    /// other ecosystems only probe at the start dir itself.
    pub walk_up: bool,
}

impl LocalBinRoot {
    /// Returns the canonical project-local bin roots for the host platform.
    ///
    /// Windows uses a `Scripts` directory for Python virtualenvs and adds
    /// `.exe`/`.cmd`/`.ps1` suffixed aliases alongside bare names; Unix uses
    /// `bin` for everything.
    #[must_use]
    pub fn canonical() -> &'static [LocalBinRoot] {
        static UNIX: &[LocalBinRoot] = &[
            LocalBinRoot {
                ecosystem: TestRunnerEcosystem::Node,
                relative: "node_modules/.bin",
                walk_up: true,
            },
            LocalBinRoot {
                ecosystem: TestRunnerEcosystem::Php,
                relative: "vendor/bin",
                walk_up: false,
            },
            LocalBinRoot {
                ecosystem: TestRunnerEcosystem::Python,
                relative: ".venv/bin",
                walk_up: false,
            },
            LocalBinRoot {
                ecosystem: TestRunnerEcosystem::Python,
                relative: "venv/bin",
                walk_up: false,
            },
            LocalBinRoot {
                ecosystem: TestRunnerEcosystem::Python,
                relative: "env/bin",
                walk_up: false,
            },
            LocalBinRoot {
                ecosystem: TestRunnerEcosystem::Ruby,
                relative: "bin",
                walk_up: false,
            },
        ];
        static WINDOWS: &[LocalBinRoot] = &[
            LocalBinRoot {
                ecosystem: TestRunnerEcosystem::Node,
                relative: "node_modules/.bin",
                walk_up: true,
            },
            LocalBinRoot {
                ecosystem: TestRunnerEcosystem::Php,
                relative: "vendor/bin",
                walk_up: false,
            },
            LocalBinRoot {
                ecosystem: TestRunnerEcosystem::Python,
                relative: ".venv/Scripts",
                walk_up: false,
            },
            LocalBinRoot {
                ecosystem: TestRunnerEcosystem::Python,
                relative: "venv/Scripts",
                walk_up: false,
            },
            LocalBinRoot {
                ecosystem: TestRunnerEcosystem::Python,
                relative: "env/Scripts",
                walk_up: false,
            },
            LocalBinRoot {
                ecosystem: TestRunnerEcosystem::Ruby,
                relative: "bin",
                walk_up: false,
            },
        ];

        if cfg!(windows) { WINDOWS } else { UNIX }
    }
}

/// Per-ecosystem project-local bin resolver.
///
/// Built once with a starting directory (typically the current working
/// directory) and then queried for many binary names. Results carry the
/// resolved path and the bin root it was found under so callers can report
/// `availability: local` with both pieces.
#[derive(Debug, Clone)]
pub struct LocalBinIndex {
    start_dir: PathBuf,
    roots: &'static [LocalBinRoot],
    virtual_env: Option<PathBuf>,
}

impl LocalBinIndex {
    /// Build a resolver rooted at `start_dir`.
    ///
    /// `start_dir` should be the directory the binary will be invoked from
    /// (typically `std::env::current_dir()`). `$VIRTUAL_ENV` is honored on
    /// all platforms when set.
    #[must_use]
    #[instrument(skip_all)]
    pub fn new(start_dir: PathBuf) -> Self {
        let virtual_env = std::env::var_os("VIRTUAL_ENV").map(PathBuf::from);
        Self {
            start_dir,
            roots: LocalBinRoot::canonical(),
            virtual_env,
        }
    }

    /// Build a resolver with explicit roots (for testing).
    #[must_use]
    pub fn new_with_roots(start_dir: PathBuf, roots: &'static [LocalBinRoot]) -> Self {
        Self {
            start_dir,
            roots,
            virtual_env: None,
        }
    }

    /// Returns the start directory the resolver searches from.
    #[must_use]
    pub fn start_dir(&self) -> &Path {
        &self.start_dir
    }

    /// Resolve `binary` for `ecosystem`.
    ///
    /// Returns the resolved path and the bin root directory it was found
    /// under. Search order:
    ///
    /// 1. `$VIRTUAL_ENV/{bin|Scripts}/<binary>` (Python only, when env var set)
    /// 2. Per-root search: walk-up roots iterate ancestors of `start_dir`;
    ///    non-walk-up roots probe `start_dir` only.
    /// 3. On Windows, probe `<binary>.exe`, `<binary>.cmd`, and `<binary>.ps1`
    ///    aliases in addition to the bare name.
    #[must_use]
    pub fn find(&self, ecosystem: TestRunnerEcosystem, binary: &str) -> Option<(PathBuf, PathBuf)> {
        // Layer 0: $VIRTUAL_ENV (Python-only, cross-platform).
        if ecosystem == TestRunnerEcosystem::Python
            && let Some(venv) = self.virtual_env.as_ref()
        {
            let bin_dir = venv_join_bin(venv);
            if let Some(path) = probe_bin(&bin_dir, binary) {
                return Some((path, venv.clone()));
            }
        }

        // Layer 1: per-root search.
        for root in self.roots.iter().filter(|r| r.ecosystem == ecosystem) {
            if root.walk_up {
                for ancestor in self.start_dir.ancestors() {
                    let bin_dir = ancestor.join(root.relative);
                    if let Some(path) = probe_bin(&bin_dir, binary) {
                        return Some((path, ancestor.to_path_buf()));
                    }
                }
            } else {
                let bin_dir = self.start_dir.join(root.relative);
                if let Some(path) = probe_bin(&bin_dir, binary) {
                    return Some((path, self.start_dir.clone()));
                }
            }
        }

        None
    }
}

/// Join a virtualenv root with the platform bin directory.
fn venv_join_bin(venv: &Path) -> PathBuf {
    if cfg!(windows) {
        venv.join("Scripts")
    } else {
        venv.join("bin")
    }
}

/// Probe `bin_dir/<binary>` (and Windows aliases) for an existing executable.
fn probe_bin(bin_dir: &Path, binary: &str) -> Option<PathBuf> {
    let direct = bin_dir.join(binary);
    if is_executable(&direct) {
        return Some(direct);
    }

    #[cfg(windows)]
    {
        for ext in [WINDOWS_EXE, WINDOWS_CMD, WINDOWS_PS1] {
            let candidate = binenv_path(bin_dir, binary, ext);
            if is_executable(&candidate) {
                return Some(candidate);
            }
        }
    }

    None
}

#[cfg(windows)]
fn binenv_path(bin_dir: &Path, binary: &str, ext: &str) -> PathBuf {
    let mut name = String::with_capacity(binary.len() + ext.len());
    name.push_str(binary);
    name.push_str(ext);
    bin_dir.join(name)
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    // Cheap existence + executable-bit check. We do not follow symlinks
    // twice; `symlink_metadata` is enough to know the link itself exists,
    // and we let the OS resolve the target's mode via `metadata`.
    use std::os::unix::fs::PermissionsExt;
    match std::fs::metadata(path) {
        Ok(meta) => {
            if !meta.is_file() {
                return false;
            }
            meta.permissions().mode() & 0o111 != 0
        }
        Err(_) => false,
    }
}

#[cfg(windows)]
fn is_executable(path: &Path) -> bool {
    // On Windows, existence of a file with a known extension is enough.
    // `cmd` and `ps1` wrappers are not "executables" in the POSIX sense but
    // are runnable via the shell, so we treat them as hits.
    path.is_file()
}

#[cfg(not(any(unix, windows)))]
fn is_executable(path: &Path) -> bool {
    path.is_file()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{ScopedEnv, make_executable_fixture};
    use std::fs;

    fn temp_root() -> tempfile::TempDir {
        tempfile::tempdir().expect("temp dir")
    }

    #[test]
    fn canonical_roots_include_node_php_python_ruby() {
        let roots = LocalBinRoot::canonical();
        let ecosystems: std::collections::HashSet<_> = roots.iter().map(|r| r.ecosystem).collect();
        assert!(ecosystems.contains(&TestRunnerEcosystem::Node));
        assert!(ecosystems.contains(&TestRunnerEcosystem::Php));
        assert!(ecosystems.contains(&TestRunnerEcosystem::Python));
        assert!(ecosystems.contains(&TestRunnerEcosystem::Ruby));
    }

    #[test]
    fn node_root_walks_up_to_find_hoisted_install() {
        let tmp = temp_root();
        let project = tmp.path().join("project");
        let subdir = project.join("packages").join("a");
        fs::create_dir_all(&subdir).unwrap();

        let bin_dir = project.join("node_modules").join(".bin");
        let expected = make_executable_fixture(&bin_dir.join("vitest"));

        let index = LocalBinIndex::new(subdir);
        let (path, root) = index
            .find(TestRunnerEcosystem::Node, "vitest")
            .expect("hoisted vitest should resolve");
        assert_eq!(path, expected);
        assert_eq!(root, project);
    }

    #[test]
    fn php_root_does_not_walk_up() {
        let tmp = temp_root();
        let project = tmp.path().join("project");
        let subdir = project.join("src");
        fs::create_dir_all(&subdir).unwrap();

        // vendor/bin at project root, but cwd is project/src.
        let top_bin = project.join("vendor").join("bin");
        let expected = make_executable_fixture(&top_bin.join("phpunit"));

        // The PHP root only probes the start dir.
        let index = LocalBinIndex::new(subdir);
        assert!(index.find(TestRunnerEcosystem::Php, "phpunit").is_none());

        // From project root itself it resolves.
        let index = LocalBinIndex::new(project.clone());
        let (path, root) = index
            .find(TestRunnerEcosystem::Php, "phpunit")
            .expect("phpunit should resolve from project root");
        assert_eq!(path, expected);
        assert_eq!(root, project);
    }

    #[test]
    fn python_virtualenv_is_honored_when_set() {
        let venv_tmp = temp_root();
        let mut env = ScopedEnv::new();
        env.set("VIRTUAL_ENV", venv_tmp.path().to_str().unwrap());

        let bin_dir = if cfg!(windows) {
            venv_tmp.path().join("Scripts")
        } else {
            venv_tmp.path().join("bin")
        };
        let expected = make_executable_fixture(&bin_dir.join("pytest"));

        // Use a different start dir to prove VIRTUAL_ENV wins regardless of cwd.
        let elsewhere = temp_root();
        let index = LocalBinIndex::new(elsewhere.path().to_path_buf());
        let (path, root) = index
            .find(TestRunnerEcosystem::Python, "pytest")
            .expect("VIRTUAL_ENV should provide pytest");
        assert_eq!(path, expected);
        assert_eq!(root, venv_tmp.path());
    }

    #[test]
    fn missing_binary_returns_none() {
        let tmp = temp_root();
        let index = LocalBinIndex::new(tmp.path().to_path_buf());
        assert!(index.find(TestRunnerEcosystem::Node, "nope").is_none());
    }

    #[test]
    fn non_executable_file_at_bin_path_is_skipped_on_unix() {
        if cfg!(windows) {
            return;
        }
        let tmp = temp_root();
        let bin_dir = tmp.path().join("node_modules").join(".bin");
        fs::create_dir_all(&bin_dir).unwrap();
        // Plain file, no +x bit.
        fs::write(bin_dir.join("vitest"), b"not executable").unwrap();

        let index = LocalBinIndex::new(tmp.path().to_path_buf());
        assert!(index.find(TestRunnerEcosystem::Node, "vitest").is_none());
    }

    #[test]
    fn ruby_bin_root_probes_start_dir() {
        let tmp = temp_root();
        let project = tmp.path().join("ruby-proj");
        fs::create_dir_all(&project).unwrap();
        let bin_dir = project.join("bin");
        let expected = make_executable_fixture(&bin_dir.join("rspec"));

        let index = LocalBinIndex::new(project.clone());
        let (path, root) = index
            .find(TestRunnerEcosystem::Ruby, "rspec")
            .expect("binstub rspec should resolve");
        assert_eq!(path, expected);
        assert_eq!(root, project);
    }
}
