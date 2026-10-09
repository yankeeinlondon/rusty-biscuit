//! Deciding whether an observed object is in scope.
//!
//! Identity decides. A backend that supplies an identity is matched on it
//! alone, never on its path text. A backend that supplies only a path gets an
//! identity lookup of that path, after a component-aware containment check;
//! a raw string prefix never decides containment (`/work/app-copy` is outside
//! `/work/app`). Looking the path up lets the OS resolve aliases the text
//! cannot: Windows drive, UNC, and `\\?\` spellings, 8.3 short names, and
//! per-directory case sensitivity.

use super::budget::Budget;
use super::identity::{self, FileIdentity};
use super::report::MatchBasis;
use super::root::Root;
use super::tree::TreeIndex;
use crate::performance::{self, counters};
use std::path::{Path, PathBuf};

/// What a backend observed about one object a process holds.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ObservedObject {
    pub(crate) identity: Option<FileIdentity>,
    /// The backend's spelling. Descriptive only when `deleted` is set.
    pub(crate) path: Option<PathBuf>,
    /// The backend knows the object was unlinked; its former path may now
    /// name something else, so it is never looked up.
    pub(crate) deleted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ObjectMatch {
    pub(crate) matched_paths: Vec<PathBuf>,
    pub(crate) basis: MatchBasis,
    pub(crate) identity: FileIdentity,
}

/// Matches `object` against the in-scope identities.
pub(crate) fn match_object(
    object: &ObservedObject,
    root: &Root,
    index: &TreeIndex,
    budget: &Budget,
) -> Option<ObjectMatch> {
    if let Some(identity) = object.identity {
        return index.paths(&identity).map(|paths| ObjectMatch {
            matched_paths: sorted(paths),
            basis: MatchBasis::Identity,
            identity,
        });
    }
    if object.deleted {
        return None;
    }
    let path = object.path.as_deref()?;
    if !root.spellings().iter().any(|spelling| may_contain(spelling, path)) || budget.expired() {
        return None;
    }
    performance::increment_counter(counters::QUERY_PATH_LOOKUPS, 1);
    let identity = identity::of_path(path, true).ok()?;
    index.paths(&identity).map(|paths| ObjectMatch {
        matched_paths: sorted(paths),
        basis: MatchBasis::PathLookup,
        identity,
    })
}

fn sorted(paths: &[PathBuf]) -> Vec<PathBuf> {
    let mut paths = paths.to_vec();
    paths.sort();
    paths
}

/// Whether `path` could name `root` or a descendant. Over-inclusive on
/// Windows by design: the identity lookup that follows is the decision.
pub(crate) fn may_contain(root: &Path, path: &Path) -> bool {
    if !path.is_absolute() {
        return false;
    }
    #[cfg(not(windows))]
    {
        path.starts_with(root)
    }
    #[cfg(windows)]
    {
        windows_spelling::may_contain(root, path)
    }
}

#[cfg(windows)]
pub(crate) mod windows_spelling {
    use std::path::{Component, Path, Prefix};

    /// One comparable component. Prefixes fold `\\?\C:` into `C:` and
    /// `\\?\UNC\server\share` into `\\server\share`.
    #[derive(Debug, PartialEq, Eq)]
    enum Part {
        Prefix(String),
        Name(String),
    }

    fn fold(text: &str) -> String {
        text.to_uppercase()
    }

    fn parts(path: &Path) -> Vec<Part> {
        path.components()
            .filter_map(|component| match component {
                Component::Prefix(prefix) => Some(Part::Prefix(match prefix.kind() {
                    Prefix::Disk(d) | Prefix::VerbatimDisk(d) => {
                        format!("{}:", char::from(d).to_ascii_uppercase())
                    }
                    Prefix::UNC(server, share) | Prefix::VerbatimUNC(server, share) => format!(
                        r"\\{}\{}",
                        fold(&server.to_string_lossy()),
                        fold(&share.to_string_lossy())
                    ),
                    _ => fold(&prefix.as_os_str().to_string_lossy()),
                })),
                Component::RootDir | Component::CurDir => None,
                Component::ParentDir => Some(Part::Name("..".to_string())),
                Component::Normal(name) => Some(Part::Name(fold(&name.to_string_lossy()))),
            })
            .collect()
    }

    /// A component that may be an 8.3 alias of the long name it is compared
    /// with, so text comparison cannot reject it.
    fn may_be_short_name(part: &Part) -> bool {
        matches!(part, Part::Name(name) if name.contains('~'))
    }

    /// Case-insensitive, prefix-normalized, component-wise containment.
    ///
    /// Case folding here only widens a pre-filter; whether a case-sensitive
    /// directory makes two spellings different objects is left to the
    /// identity lookup.
    pub(crate) fn may_contain(root: &Path, path: &Path) -> bool {
        let root = parts(root);
        let path = parts(path);
        if path.len() < root.len() {
            return path.iter().any(may_be_short_name);
        }
        root.iter()
            .zip(&path)
            .all(|(r, p)| r == p || may_be_short_name(p))
    }
}
