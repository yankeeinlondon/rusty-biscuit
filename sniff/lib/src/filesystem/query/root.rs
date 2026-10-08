use super::budget::Budget;
use super::identity::{self, FileIdentity};
use super::report::TargetKind;
use super::PathUsageError;
use crate::performance::{self, counters};
use std::fs::FileType;
use std::io;
use std::path::{Path, PathBuf};

/// The verified query root, captured once.
#[derive(Debug, Clone)]
pub(crate) struct Root {
    pub(crate) requested: PathBuf,
    pub(crate) resolved: PathBuf,
    pub(crate) kind: TargetKind,
    pub(crate) identity: FileIdentity,
}

impl Root {
    /// Spellings an in-scope observed path may start with: the resolved path,
    /// and the requested one when it is an absolute alias.
    pub(crate) fn spellings(&self) -> Vec<&Path> {
        let mut spellings = vec![self.resolved.as_path()];
        if self.requested.is_absolute() && self.requested != self.resolved {
            spellings.push(self.requested.as_path());
        }
        spellings
    }
}

/// Classifies a file type that is neither a regular file nor a directory.
fn unsupported_kind(file_type: &FileType) -> Option<&'static str> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileTypeExt;
        if file_type.is_socket() {
            return Some("socket");
        }
        if file_type.is_fifo() {
            return Some("fifo");
        }
        if file_type.is_block_device() {
            return Some("block device");
        }
        if file_type.is_char_device() {
            return Some("character device");
        }
    }
    if file_type.is_file() || file_type.is_dir() || file_type.is_symlink() {
        None
    } else {
        Some("other")
    }
}

fn timeout(path: &Path, budget: &Budget) -> PathUsageError {
    PathUsageError::RootValidationTimeout {
        path: path.to_path_buf(),
        budget: budget.limit(),
    }
}

fn io_error(path: &Path, source: io::Error) -> PathUsageError {
    if source.kind() == io::ErrorKind::NotFound {
        PathUsageError::MissingTarget {
            path: path.to_path_buf(),
        }
    } else {
        PathUsageError::RootIdentity {
            path: path.to_path_buf(),
            source,
        }
    }
}

fn symlink_metadata(path: &Path) -> io::Result<std::fs::Metadata> {
    performance::increment_counter(counters::FS_METADATA_PROBES, 1);
    std::fs::symlink_metadata(path)
}

/// Resolves the root alias once and captures its identity.
///
/// Every file-type decision comes from `symlink_metadata`, which never opens
/// the object, so a FIFO or device is rejected before anything could wait on
/// it.
pub(crate) fn resolve(path: &Path, budget: &Budget) -> Result<Root, PathUsageError> {
    if budget.expired() {
        return Err(timeout(path, budget));
    }
    let link_metadata = symlink_metadata(path).map_err(|e| io_error(path, e))?;
    if let Some(file_type) = unsupported_kind(&link_metadata.file_type()) {
        return Err(PathUsageError::UnsupportedTargetKind {
            path: path.to_path_buf(),
            file_type,
        });
    }

    if budget.expired() {
        return Err(timeout(path, budget));
    }
    performance::increment_counter(counters::FS_CANONICALIZATIONS, 1);
    let resolved = std::fs::canonicalize(path).map_err(|e| io_error(path, e))?;
    let metadata = if link_metadata.file_type().is_symlink() {
        if budget.expired() {
            return Err(timeout(path, budget));
        }
        let metadata = symlink_metadata(&resolved).map_err(|e| io_error(path, e))?;
        if let Some(file_type) = unsupported_kind(&metadata.file_type()) {
            return Err(PathUsageError::UnsupportedTargetKind {
                path: path.to_path_buf(),
                file_type,
            });
        }
        metadata
    } else {
        link_metadata
    };
    let kind = if metadata.is_dir() {
        TargetKind::Directory
    } else {
        TargetKind::File
    };

    if budget.expired() {
        return Err(timeout(path, budget));
    }
    #[cfg(unix)]
    let identity = identity::of_metadata(&metadata);
    #[cfg(windows)]
    let identity = {
        performance::increment_counter(counters::FS_METADATA_PROBES, 1);
        identity::of_path(&resolved, false).map_err(|e| io_error(path, e))?
    };

    Ok(Root {
        requested: path.to_path_buf(),
        resolved,
        kind,
        identity,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Recheck {
    Verified,
    /// The resolved path is gone or names a different object.
    Changed(String),
    /// The budget was exhausted or the recheck itself failed.
    Unverified(String),
}

/// Rechecks the captured identity, only while the budget allows.
pub(crate) fn recheck(root: &Root, budget: &Budget) -> Recheck {
    if budget.expired() {
        return Recheck::Unverified("the budget was exhausted before the recheck".to_string());
    }
    performance::increment_counter(counters::FS_METADATA_PROBES, 1);
    match identity::of_path(&root.resolved, false) {
        Ok(identity) if identity == root.identity => Recheck::Verified,
        Ok(_) => Recheck::Changed("the target path now names a different object".to_string()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            Recheck::Changed("the target no longer exists".to_string())
        }
        Err(e) => Recheck::Unverified(format!("the recheck failed: {e}")),
    }
}
