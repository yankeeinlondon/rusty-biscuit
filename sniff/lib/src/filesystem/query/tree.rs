use super::budget::Budget;
use super::identity::{self, FileIdentity};
use super::report::{
    Coverage, CoverageScope, CoverageStatus, LimitationExample, LimitationKind, Limitations,
    Mechanism, TargetKind,
};
use super::root::Root;
use super::{IndexProvenance, PathUsageError};
use crate::performance::{self, counters};
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

/// The identities inside a directory tree and every path naming each one,
/// as [`query_path_usage`](super::query_path_usage) matches them.
///
/// A query walks its target to build one of these, unless the caller already
/// walked the tree and supplies the index it built through
/// [`PathUsageOptions::with_tree_index`](super::PathUsageOptions::with_tree_index).
/// Both are the same type and are matched the same way.
///
/// Paths are stored relative to the root (the root itself is the empty
/// path), so a supplied index is reusable across queries and only matched
/// paths are joined onto the verified root. Every hard-link alias is kept,
/// and a link is indexed as itself, never as its target.
///
/// An index describes the tree when it was captured: an entry created later
/// is not in it, and a query does not re-read a supplied index's
/// descendants. Build it with [`TreeIndex::builder`].
///
/// ## Examples
///
/// ```
/// use sniff::filesystem::query::{FileIdentity, TreeEntryKind, TreeIndex};
///
/// let id = |inode| FileIdentity::Unix { device: 1, inode };
/// let mut builder = TreeIndex::builder(id(10));
/// builder
///     .entry("src", TreeEntryKind::Directory, id(11))
///     .entry("src/main.rs", TreeEntryKind::File, id(12))
///     .entry("main-link.rs", TreeEntryKind::File, id(12));
/// let index = builder.build()?;
/// assert_eq!(index.len(), 3);
///
/// let mut bad = TreeIndex::builder(id(10));
/// bad.entry("../outside", TreeEntryKind::File, id(13));
/// assert!(bad.build().is_err());
/// # Ok::<(), sniff::filesystem::query::TreeIndexError>(())
/// ```
pub struct TreeIndex {
    root: FileIdentity,
    /// Descendant entries, excluding the root.
    descendants: usize,
    /// The caller's reason when it declared the index incomplete.
    partial: Option<String>,
    paths: HashMap<FileIdentity, Vec<PathBuf>>,
    #[cfg(any(windows, test))]
    probes: Vec<Probe>,
}

impl std::fmt::Debug for TreeIndex {
    // An index can hold hundreds of thousands of paths.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TreeIndex")
            .field("root", &self.root)
            .field("descendants", &self.descendants)
            .field("partial", &self.partial)
            .finish_non_exhaustive()
    }
}

/// What a supplied index entry is, as the caller read it without following
/// links.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TreeEntryKind {
    File,
    Directory,
    /// A symlink, junction, or other name-surrogate reparse point. It is
    /// matched by its own identity and never opened as its target, and the
    /// index may hold nothing below it.
    Link,
    /// A socket, FIFO, device, or other special file: matched, never opened.
    Other,
}

/// A supplied index entry that cannot describe a tree, naming the entry.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TreeIndexError {
    #[error("tree index entry {}: an absolute path; entries are relative to the root", path.display())]
    AbsolutePath { path: PathBuf },

    #[error("tree index entry {}: `..` leaves the root", path.display())]
    ParentTraversal { path: PathBuf },

    /// An empty path (the root is implicit) or a `.` component.
    #[error("tree index entry {:?}: not a normalized path below the root", path)]
    NotNormalized { path: PathBuf },

    #[error("tree index entry {}: listed more than once", path.display())]
    Duplicate { path: PathBuf },

    #[error(
        "tree index entry {}: below the link {}, which an index never expands",
        path.display(),
        link.display()
    )]
    BelowLink { path: PathBuf, link: PathBuf },
}

/// Collects a caller's entries for one [`TreeIndex`]; see
/// [`TreeIndex::builder`].
#[derive(Debug)]
pub struct TreeIndexBuilder {
    root: FileIdentity,
    entries: Vec<(PathBuf, TreeEntryKind, FileIdentity)>,
    partial: Option<String>,
}

impl TreeIndexBuilder {
    /// Adds the entry at `path`, relative to the root and spelled with native
    /// names. Validation happens in [`build`](Self::build).
    pub fn entry(
        &mut self,
        path: impl Into<PathBuf>,
        kind: TreeEntryKind,
        identity: FileIdentity,
    ) -> &mut Self {
        self.entries.push((path.into(), kind, identity));
        self
    }

    /// Declares the index incomplete, for example because the caller's walk
    /// skipped unreadable entries. A query reports the tree's coverage as
    /// partial with `reason`.
    pub fn partial(&mut self, reason: impl Into<String>) -> &mut Self {
        self.partial = Some(reason.into());
        self
    }

    /// Validates every entry and builds the index.
    ///
    /// ## Errors
    ///
    /// The first invalid entry in path order: an absolute path, a `..` or `.`
    /// component, an empty path, a path listed twice, or a path below an
    /// entry of kind [`TreeEntryKind::Link`].
    pub fn build(self) -> Result<TreeIndex, TreeIndexError> {
        let Self { root, mut entries, partial } = self;
        for (path, _, _) in &mut entries {
            validate(path)?;
            // Drops repeated and trailing separators from the stored spelling.
            *path = path.components().collect();
        }
        // Component order puts each path's descendants directly after it, so
        // duplicates are adjacent and a link's descendants follow the link.
        let mut order: Vec<usize> = (0..entries.len()).collect();
        order.sort_unstable_by(|&a, &b| entries[a].0.cmp(&entries[b].0));
        let mut link: Option<&Path> = None;
        let mut previous: Option<&Path> = None;
        for &position in &order {
            let (path, kind, _) = &entries[position];
            if previous == Some(path.as_path()) {
                return Err(TreeIndexError::Duplicate { path: path.clone() });
            }
            if let Some(link) = link.filter(|link| path.starts_with(link)) {
                return Err(TreeIndexError::BelowLink {
                    path: path.clone(),
                    link: link.to_path_buf(),
                });
            }
            if *kind == TreeEntryKind::Link {
                link = Some(path);
            }
            previous = Some(path);
        }

        let mut index = TreeIndex::rooted(root, true);
        index.partial = partial;
        index.descendants = entries.len();
        for (path, kind, identity) in entries {
            #[cfg(any(windows, test))]
            if matches!(kind, TreeEntryKind::File | TreeEntryKind::Directory) {
                index.probes.push(Probe {
                    relative: path.clone(),
                    identity,
                    directory: kind == TreeEntryKind::Directory,
                });
            }
            #[cfg(not(any(windows, test)))]
            let _ = kind;
            index.insert(identity, path);
        }
        Ok(index)
    }
}

/// Accepts only normal components. `Path::components` already folds
/// repeated and trailing separators and an interior `.`, so two spellings of
/// one path compare equal in the duplicate check; a leading `.` is the one
/// spelling left to refuse.
fn validate(path: &Path) -> Result<(), TreeIndexError> {
    let mut components = 0;
    for component in path.components() {
        components += 1;
        match component {
            Component::Normal(_) => {}
            Component::Prefix(_) | Component::RootDir => {
                return Err(TreeIndexError::AbsolutePath { path: path.to_path_buf() });
            }
            Component::ParentDir => {
                return Err(TreeIndexError::ParentTraversal { path: path.to_path_buf() });
            }
            Component::CurDir => {
                return Err(TreeIndexError::NotNormalized { path: path.to_path_buf() });
            }
        }
    }
    if components == 0 {
        return Err(TreeIndexError::NotNormalized { path: path.to_path_buf() });
    }
    Ok(())
}

/// A regular file or directory in an index, safe to open for probing.
#[cfg(any(windows, test))]
#[derive(Debug, Clone)]
struct Probe {
    relative: PathBuf,
    identity: FileIdentity,
    directory: bool,
}

/// A [`Probe`] joined onto the verified root, ready to open.
#[cfg(any(windows, test))]
#[derive(Debug, Clone)]
pub(crate) struct ProbeEntry {
    pub(crate) path: PathBuf,
    pub(crate) identity: FileIdentity,
    pub(crate) directory: bool,
}

#[allow(missing_docs)]
impl TreeIndex {
    /// Starts an index of the directory whose identity is `root`. The root is
    /// implicit: add only its descendants.
    pub fn builder(root: FileIdentity) -> TreeIndexBuilder {
        TreeIndexBuilder {
            root,
            entries: Vec::new(),
            partial: None,
        }
    }

    pub fn root(&self) -> FileIdentity {
        self.root
    }

    /// Descendant entries, excluding the root.
    pub fn len(&self) -> usize {
        self.descendants
    }

    pub fn is_empty(&self) -> bool {
        self.descendants == 0
    }

    /// The caller's reason, when it declared the index incomplete.
    pub fn partial_reason(&self) -> Option<&str> {
        self.partial.as_deref()
    }

    /// An index holding only the root, which is probed as a directory when
    /// `directory` is set.
    fn rooted(root: FileIdentity, directory: bool) -> Self {
        let mut index = Self {
            root,
            descendants: 0,
            partial: None,
            paths: HashMap::new(),
            #[cfg(any(windows, test))]
            probes: Vec::new(),
        };
        index.insert(root, PathBuf::new());
        #[cfg(any(windows, test))]
        index.probes.push(Probe {
            relative: PathBuf::new(),
            identity: root,
            directory,
        });
        #[cfg(not(any(windows, test)))]
        let _ = directory;
        index
    }

    fn insert(&mut self, identity: FileIdentity, path: PathBuf) {
        self.paths.entry(identity).or_default().push(path);
    }

    /// In-scope paths for `identity` under `root`, sorted; several when hard
    /// links share it.
    pub(crate) fn paths(&self, identity: &FileIdentity, root: &Path) -> Option<Vec<PathBuf>> {
        self.paths.get(identity).map(|relative| {
            let mut paths: Vec<_> = relative.iter().map(|path| join(root, path)).collect();
            paths.sort();
            paths
        })
    }

    #[cfg(any(target_os = "linux", all(test, unix)))]
    pub(crate) fn identities(&self) -> impl Iterator<Item = &FileIdentity> {
        self.paths.keys()
    }

    /// Regular files and directories (never links) to probe, joined onto
    /// `root`. The root comes first.
    #[cfg(any(windows, test))]
    pub(crate) fn probe_entries(&self, root: &Path) -> Vec<ProbeEntry> {
        self.probes
            .iter()
            .map(|probe| ProbeEntry {
                path: join(root, &probe.relative),
                identity: probe.identity,
                directory: probe.directory,
            })
            .collect()
    }
}

/// `root.join("")` would append a separator, so the root stays as spelled.
fn join(root: &Path, relative: &Path) -> PathBuf {
    if relative.as_os_str().is_empty() {
        root.to_path_buf()
    } else {
        root.join(relative)
    }
}

pub(crate) struct TreeCollection {
    pub(crate) index: Arc<TreeIndex>,
    /// Present only for a recursive directory query, the one case that walks
    /// or accepts a supplied index.
    pub(crate) coverage: Option<Coverage>,
    pub(crate) provenance: IndexProvenance,
}

/// Uses a caller's index in place of the walk, after checking it describes
/// the root this query verified.
pub(crate) fn supplied(
    root: &Root,
    index: &Arc<TreeIndex>,
) -> Result<TreeCollection, PathUsageError> {
    if root.kind != TargetKind::Directory {
        return Err(PathUsageError::InvalidOption {
            message: format!(
                "a tree index was supplied, but the target is a file: {}",
                root.resolved.display()
            ),
        });
    }
    if index.root != root.identity {
        return Err(PathUsageError::InvalidOption {
            message: format!(
                "the supplied tree index describes a different root than {}",
                root.resolved.display()
            ),
        });
    }
    let mut limitations = Limitations::default();
    if let Some(reason) = &index.partial {
        limitations.record(LimitationKind::IncompleteTreeIndex, reason.clone(), 1, None);
    }
    Ok(TreeCollection {
        index: Arc::clone(index),
        coverage: Some(Coverage {
            mechanism: Mechanism::TreeIdentity,
            scope: CoverageScope::TargetTree,
            status: if index.partial.is_some() {
                CoverageStatus::Partial
            } else {
                CoverageStatus::Complete
            },
            // The caller's count, not Sniff's: no descendant was read here.
            attempted: None,
            succeeded: None,
            reason: Some(
                "the caller supplied the tree index; this query did not read its descendants"
                    .to_string(),
            ),
            truncation: None,
            limitations: limitations.into_vec(),
        }),
        provenance: IndexProvenance::Supplied,
    })
}

/// Collects the identity of the root and, for a recursive directory query,
/// every descendant, in one streaming walk.
///
/// Descendant symlinks, junctions, and other reparse points are indexed as
/// themselves and never expanded. Hidden and Git-ignored entries are in scope.
/// No file body is read and no per-entry handle outlives its identity read;
/// the walker's directory handles close before this returns, so they cannot
/// appear as the querying process's own usage.
pub(crate) fn collect(root: &Root, recursive: bool, budget: &Budget) -> TreeCollection {
    let mut index = TreeIndex::rooted(root.identity, root.kind == TargetKind::Directory);
    if root.kind != TargetKind::Directory || !recursive {
        return TreeCollection {
            index: Arc::new(index),
            coverage: None,
            provenance: IndexProvenance::Walked,
        };
    }

    performance::increment_counter(counters::QUERY_TREE_WALKS, 1);
    let mut limitations = Limitations::default();
    let mut attempted = 0u64;
    let mut succeeded = 0u64;
    let mut walk = walkdir::WalkDir::new(&root.resolved)
        .min_depth(1)
        .follow_links(false)
        .into_iter();
    loop {
        if budget.expired() {
            limitations.record(
                LimitationKind::BudgetExhausted,
                "the budget expired before the tree walk finished",
                1,
                None,
            );
            break;
        }
        let Some(next) = walk.next() else { break };
        attempted += 1;
        let entry = match next {
            Ok(entry) => entry,
            Err(error) => {
                let example = error
                    .path()
                    .map(|p| LimitationExample::path(p).with_detail(error.to_string()));
                limitations.record(
                    LimitationKind::UnreadableTreeEntry,
                    "some tree entries could not be read",
                    1,
                    example,
                );
                continue;
            }
        };
        performance::increment_counter(counters::QUERY_TREE_IDENTITY_READS, 1);
        match identity::of_entry(&entry) {
            Ok(identity) => {
                succeeded += 1;
                // The walker yields every path under the root it was given.
                let relative = entry
                    .path()
                    .strip_prefix(&root.resolved)
                    .expect("walked entry under the root")
                    .to_path_buf();
                #[cfg(any(windows, test))]
                if entry.file_type().is_file() || entry.file_type().is_dir() {
                    index.probes.push(Probe {
                        relative: relative.clone(), identity,
                        directory: entry.file_type().is_dir(),
                    });
                }
                index.insert(identity, relative);
                index.descendants += 1;
            }
            Err(error) => limitations.record(
                LimitationKind::UnreadableTreeEntry,
                "some tree entries could not be read",
                1,
                Some(LimitationExample::path(entry.path()).with_detail(error.to_string())),
            ),
        }
        #[cfg(windows)]
        if is_reparse_directory(&entry) {
            walk.skip_current_dir();
        }
    }

    let status = if limitations.is_empty() {
        CoverageStatus::Complete
    } else {
        CoverageStatus::Partial
    };
    TreeCollection {
        index: Arc::new(index),
        provenance: IndexProvenance::Walked,
        coverage: Some(Coverage {
            mechanism: Mechanism::TreeIdentity,
            scope: CoverageScope::TargetTree,
            status,
            attempted: Some(attempted),
            succeeded: Some(succeeded),
            reason: None,
            truncation: None,
            limitations: limitations.into_vec(),
        }),
    }
}

/// A directory that is a reparse point the walker would otherwise enter.
///
/// std reports name-surrogate reparse points (symlinks, junctions) as links,
/// which the walker already skips; other tags, such as cloud-file
/// placeholders, look like directories, and entering them can hydrate content.
#[cfg(windows)]
fn is_reparse_directory(entry: &walkdir::DirEntry) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    entry.file_type().is_dir()
        && entry
            .metadata()
            .is_ok_and(|m| m.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0)
}
