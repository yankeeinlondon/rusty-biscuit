//! The five schema roots: where trigger schemas are discovered and where a
//! bare-name `$schema` (`$schema: claudine.yaml`) is looked up.
//!
//! The roots are searched most local first, and the first root that holds a
//! file name shadows the same name in every later root:
//!
//! 1. `{package root}/schemas`
//! 2. `{package-area root}/schemas`
//! 3. `{base_dir()}/schemas`, the file tree root (the repository root in a
//!    repository)
//! 4. the folder `SCHEMAS_DIR` names, searched itself (no `schemas/` is
//!    appended)
//! 5. `{home}/schemas`
//!
//! Every input comes from the document's [`FileResolutionContext`]: the
//! package, package area, and tree root of the document being checked, and
//! `SCHEMAS_DIR` and the home directory from the request snapshot. Nothing is
//! read from the process. The user-facing contract is the "Schema roots"
//! section of `darkmatter/docs/topics/schemas/definition.md`.

use std::path::{Path, PathBuf};

use biscuit_file::{FileResolutionContext, PathIdentity, canonicalize_simplified};

/// The environment variable naming the fourth schema root.
pub const SCHEMAS_DIR_VARIABLE: &str = "SCHEMAS_DIR";

/// The folder name a package, package-area, tree, or home root searches.
const SCHEMAS_DIR_NAME: &str = "schemas";

/// Which of the five schema roots an entry is, in search order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaRootKind {
    /// `{package root}/schemas`.
    Package,
    /// `{package-area root}/schemas`.
    PackageArea,
    /// `{base_dir()}/schemas`.
    Tree,
    /// The folder `SCHEMAS_DIR` names.
    SchemasDir,
    /// `{home}/schemas`.
    Home,
}

impl SchemaRootKind {
    /// A short human label, used by `md schema triggers`.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Package => "package root",
            Self::PackageArea => "package-area root",
            Self::Tree => "file tree root",
            Self::SchemasDir => "SCHEMAS_DIR",
            Self::Home => "home",
        }
    }

    /// Whether a bare or `./` trigger pattern from this root is read from the
    /// checked document's tree root rather than from the folder holding the
    /// root's `schemas/` directory. A user-level root holds no repository
    /// documents, so it has no folder of its own to read from.
    fn reads_patterns_from_tree_root(self) -> bool {
        matches!(self, Self::SchemasDir | Self::Home)
    }
}

/// Why a set `SCHEMAS_DIR` adds no root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidSchemasDir {
    /// Empty or whitespace only.
    Empty,
    /// Not an absolute path; a relative root would depend on the launch
    /// directory.
    Relative,
}

/// What one of the five roots contributes for this document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaRootState {
    /// The folder exists and is searched.
    Searched(PathBuf),
    /// The root applies but its folder does not exist, so it is skipped.
    Absent(PathBuf),
    /// The folder is the same as an earlier root's, which searches it.
    Duplicate {
        /// This root's spelling of the folder.
        path: PathBuf,
        /// The earlier root that searches it.
        of: SchemaRootKind,
    },
    /// The root does not apply: the document has no package or package area,
    /// `SCHEMAS_DIR` is unset, or the snapshot has no home directory.
    NotApplicable,
    /// `SCHEMAS_DIR` is set to a value that cannot be a root.
    Invalid {
        /// The value as set.
        value: String,
        /// Why it was refused.
        reason: InvalidSchemasDir,
    },
}

/// One of the five schema roots and what it contributes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaRoot {
    /// Which root this is.
    pub kind: SchemaRootKind,
    /// What it contributes.
    pub state: SchemaRootState,
}

/// A searched root: its folder and where its triggers' bare and `./` `$path`
/// patterns are read from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchedRoot {
    /// Which root this is.
    pub kind: SchemaRootKind,
    /// The folder whose files are read.
    pub path: PathBuf,
    /// The `cwd` of a bare or `./` `$path` pattern in a trigger from this
    /// root: the folder holding `schemas/` for the package, package-area, and
    /// tree roots, and the document's tree root for `SCHEMAS_DIR` and home.
    pub pattern_cwd: PathBuf,
}

/// The five schema roots of one document, in search order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaRoots {
    entries: Vec<SchemaRoot>,
    searched: Vec<SearchedRoot>,
    search_paths: Vec<PathBuf>,
}

impl SchemaRoots {
    /// The roots for the document whose context is `ctx`.
    ///
    /// `ctx` must be the checked document's own context: its package, package
    /// area, and tree root are the document's. A root that names the same
    /// folder as an earlier one (compared by canonical path and
    /// [`PathIdentity`]) is reported as a duplicate and searched once, at the
    /// earlier position. The package, package-area, and tree roots must be
    /// real directories (a symlinked `schemas/` in a repository is not
    /// followed); `SCHEMAS_DIR` and `~/schemas` are user configuration and
    /// may be symlinks to a directory.
    #[must_use]
    pub fn for_document(ctx: &FileResolutionContext) -> Self {
        let tree_root = ctx.base_dir().to_path_buf();
        let candidates = [
            (
                SchemaRootKind::Package,
                Candidate::from_folder(ctx.package_root()),
            ),
            (
                SchemaRootKind::PackageArea,
                Candidate::from_folder(ctx.package_area()),
            ),
            (SchemaRootKind::Tree, Candidate::from_folder(Some(&tree_root))),
            (
                SchemaRootKind::SchemasDir,
                schemas_dir_candidate(ctx.env().get(SCHEMAS_DIR_VARIABLE)),
            ),
            (SchemaRootKind::Home, Candidate::from_folder(ctx.home_dir())),
        ];

        let mut entries = Vec::with_capacity(candidates.len());
        let mut searched = Vec::new();
        let mut seen: Vec<(PathIdentity, SchemaRootKind)> = Vec::new();
        for (kind, candidate) in candidates {
            let state = match candidate {
                Candidate::NotApplicable => SchemaRootState::NotApplicable,
                Candidate::Invalid { value, reason } => SchemaRootState::Invalid { value, reason },
                Candidate::Folder(path) => {
                    let identity = identity_of(&path);
                    if let Some((_, of)) = seen.iter().find(|(earlier, _)| *earlier == identity) {
                        SchemaRootState::Duplicate { path, of: *of }
                    } else {
                        seen.push((identity, kind));
                        let user_level = kind.reads_patterns_from_tree_root();
                        if is_searchable_directory(&path, user_level) {
                            let pattern_cwd = if user_level {
                                tree_root.clone()
                            } else {
                                path.parent().map_or_else(|| tree_root.clone(), Path::to_path_buf)
                            };
                            searched.push(SearchedRoot {
                                kind,
                                path: path.clone(),
                                pattern_cwd,
                            });
                            SchemaRootState::Searched(path)
                        } else {
                            SchemaRootState::Absent(path)
                        }
                    }
                }
            };
            entries.push(SchemaRoot { kind, state });
        }
        let search_paths = searched.iter().map(|root| root.path.clone()).collect();
        Self {
            entries,
            searched,
            search_paths,
        }
    }

    /// No roots at all, for a registry built without discovery.
    #[must_use]
    pub fn none() -> Self {
        Self {
            entries: Vec::new(),
            searched: Vec::new(),
            search_paths: Vec::new(),
        }
    }

    /// All five roots in search order, including the ones that contribute
    /// nothing, for `md schema triggers`.
    #[must_use]
    pub fn entries(&self) -> &[SchemaRoot] {
        &self.entries
    }

    /// The roots that are searched, in search order.
    #[must_use]
    pub fn searched(&self) -> &[SearchedRoot] {
        &self.searched
    }

    /// The searched folders, in search order: the list bare-name `$schema`
    /// lookup walks.
    #[must_use]
    pub fn search_paths(&self) -> &[PathBuf] {
        &self.search_paths
    }
}

enum Candidate {
    NotApplicable,
    Invalid {
        value: String,
        reason: InvalidSchemasDir,
    },
    Folder(PathBuf),
}

impl Candidate {
    /// `{folder}/schemas`, when the folder exists in the context.
    fn from_folder(folder: Option<&Path>) -> Self {
        folder.map_or(Self::NotApplicable, |folder| {
            Self::Folder(native_spelling(&folder.join(SCHEMAS_DIR_NAME)))
        })
    }
}

/// `SCHEMAS_DIR` names the schemas folder itself. An unset variable adds no
/// root; an empty or relative one is refused rather than ignored, so
/// `md schema triggers` can say why it adds nothing.
fn schemas_dir_candidate(value: Option<&String>) -> Candidate {
    let Some(value) = value else {
        return Candidate::NotApplicable;
    };
    if value.trim().is_empty() {
        return Candidate::Invalid {
            value: value.clone(),
            reason: InvalidSchemasDir::Empty,
        };
    }
    let path = PathBuf::from(value);
    if !path.is_absolute() {
        return Candidate::Invalid {
            value: value.clone(),
            reason: InvalidSchemasDir::Relative,
        };
    }
    Candidate::Folder(native_spelling(&path))
}

/// `path` re-spelled with the platform's separators. On Windows a package
/// root from the repository catalog can arrive as `C:\repo\area/pkg`;
/// re-collecting the components prints it as `C:\repo\area\pkg`.
fn native_spelling(path: &Path) -> PathBuf {
    path.components().collect()
}

/// The identity two spellings of one folder share: canonical when the folder
/// exists (`/var` and `/private/var` on macOS), else lexical.
fn identity_of(path: &Path) -> PathIdentity {
    let canonical = canonicalize_simplified(path).unwrap_or_else(|_| path.to_path_buf());
    PathIdentity::new(&canonical)
}

fn is_searchable_directory(path: &Path, follow_symlink: bool) -> bool {
    if follow_symlink {
        return path.is_dir();
    }
    std::fs::symlink_metadata(path).is_ok_and(|metadata| {
        let file_type = metadata.file_type();
        file_type.is_dir() && !file_type.is_symlink()
    })
}
