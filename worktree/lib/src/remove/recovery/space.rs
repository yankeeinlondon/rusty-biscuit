//! The free-space reading the store compares with a new attempt's journal
//! before it writes one. Tests inject readings through [`SpaceSource`].

use std::path::Path;

/// Where the store reads the space available to this user.
pub trait SpaceSource: Sync {
    /// Bytes this user can write on the filesystem holding `path`, which may
    /// not exist yet (its nearest existing ancestor is measured); `Err`
    /// explains why the space could not be read.
    fn available_bytes(&self, path: &Path) -> Result<u64, String>;
}

/// [`SpaceSource`] through Sniff's path-based query: native filesystem
/// calls only, with every alias in the path resolved.
#[derive(Debug, Clone, Copy, Default)]
pub struct NativeSpace;

impl SpaceSource for NativeSpace {
    fn available_bytes(&self, path: &Path) -> Result<u64, String> {
        sniff::filesystem::space::available_space(path).map(|space| space.available_bytes).map_err(|error| error.to_string())
    }
}
