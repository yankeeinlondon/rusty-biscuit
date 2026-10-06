use super::budget::Budget;
use super::identity::{self, FileIdentity};
use super::report::{
    Coverage, CoverageScope, CoverageStatus, LimitationExample, LimitationKind, Limitations,
    Mechanism, TargetKind,
};
use super::root::Root;
use crate::performance::{self, counters};
use std::collections::HashMap;
use std::path::PathBuf;

/// In-scope identities and every in-scope path naming each one.
#[derive(Debug, Default)]
pub(crate) struct TreeIndex {
    paths: HashMap<FileIdentity, Vec<PathBuf>>,
}

impl TreeIndex {
    fn insert(&mut self, identity: FileIdentity, path: PathBuf) {
        self.paths.entry(identity).or_default().push(path);
    }

    /// In-scope paths for `identity`; several when hard links share it.
    pub(crate) fn paths(&self, identity: &FileIdentity) -> Option<&[PathBuf]> {
        self.paths.get(identity).map(Vec::as_slice)
    }

    #[cfg(any(target_os = "linux", all(test, unix)))]
    pub(crate) fn identities(&self) -> impl Iterator<Item = &FileIdentity> {
        self.paths.keys()
    }
}

pub(crate) struct TreeCollection {
    pub(crate) index: TreeIndex,
    /// Present only for a recursive directory query, the one case that walks.
    pub(crate) coverage: Option<Coverage>,
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
    let mut index = TreeIndex::default();
    index.insert(root.identity, root.resolved.clone());
    if root.kind != TargetKind::Directory || !recursive {
        return TreeCollection {
            index,
            coverage: None,
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
                index.insert(identity, entry.path().to_path_buf());
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
        index,
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
