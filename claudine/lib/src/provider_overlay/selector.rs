//! The provider-owned selector and the storage it points at.
//!
//! A selector is a documented provider environment variable whose effect is
//! limited to that provider. Claudine sets it to an overlay it owns instead of
//! replacing the child's `HOME`.
//!
//! The one thing that must never be guessed is the selector's **shape**: some
//! variables name the directory the provider reads its config file from, others
//! name a parent under which the provider creates that directory. Getting it
//! backwards writes the overlay one level away from where the provider reads,
//! which looks like isolation and is not
//! (`fixes/2026-09-12-shadow-home/audit.md` → F1). Both directions of that
//! translation live in this module and nowhere else.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::provider::{OverlaySelectorShape, OverlaySelectorSpec};

/// The directory a provider reads its configuration from, given the directory
/// its selector's value names.
///
/// This is the only expression of the shape distinction in the codebase;
/// [`OverlayStorage::new`] and [`OverlaySelector::provider_visible_root`] both
/// apply it through here.
///
/// ## Examples
///
/// ```
/// use std::path::Path;
/// use claudine::provider::OverlaySelectorShape;
/// use claudine::provider_overlay::provider_visible_root;
///
/// let storage = Path::new("/home/me/.claudine/overlays/gemini/launch");
/// assert_eq!(
///     provider_visible_root(
///         OverlaySelectorShape::ParentOfProviderDir { child: ".gemini" },
///         storage,
///     ),
///     Path::new("/home/me/.claudine/overlays/gemini/launch/.gemini"),
/// );
/// ```
pub fn provider_visible_root(shape: OverlaySelectorShape, storage_root: &Path) -> PathBuf {
    match shape {
        OverlaySelectorShape::ProviderDir | OverlaySelectorShape::Inline => {
            storage_root.to_path_buf()
        }
        OverlaySelectorShape::ParentOfProviderDir { child } => storage_root.join(child),
    }
}

/// The directory under Claudine's data directory that holds every launch's
/// overlay root, one subdirectory per provider slug.
///
/// Compatible legacy storage (`~/.claudine/<agent_offset>`, and for a parent
/// shaped selector `~/.claudine` itself) is outside this directory, so a launch
/// never reads, mutates, or removes it.
pub const OVERLAY_LAUNCHES_DIR: &str = "overlays";

/// Names an absolute directory that replaces `~/.claudine/overlays` as the
/// parent of every launch's `<provider>/<launch-id>` root.
///
/// Read from the invocation's launch baseline, like a provider selector. It
/// moves only overlay storage, never a provider's source root or a home
/// variable. It exists because native Windows resolves the home through the
/// known-folder profile and ignores `USERPROFILE`, so a disposable home (a test
/// fixture) cannot otherwise keep overlays out of the real profile.
pub const OVERLAY_DIR_ENV: &str = "CLAUDINE_OVERLAY_DIR";

/// Where Claudine keeps one launch's overlay, and what the provider sees.
///
/// `storage_root` is the value the selector carries; `provider_visible_root` is
/// where the provider's config file actually lands. They differ only for a
/// [`OverlaySelectorShape::ParentOfProviderDir`] selector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlayStorage {
    storage_root: PathBuf,
    provider_visible_root: PathBuf,
}

impl OverlayStorage {
    /// Place a provider's overlay at `launch_root`, a directory owned by this
    /// launch alone, so no other launch can observe or change its view.
    ///
    /// Returns `None` for [`OverlaySelectorShape::Inline`], which carries
    /// configuration content rather than naming a directory and therefore has
    /// no storage at all.
    pub fn new(launch_root: &Path, shape: OverlaySelectorShape) -> Option<Self> {
        if shape == OverlaySelectorShape::Inline {
            return None;
        }
        let storage_root = launch_root.to_path_buf();
        let provider_visible_root = provider_visible_root(shape, &storage_root);
        Some(Self {
            storage_root,
            provider_visible_root,
        })
    }

    /// The directory the selector's value names.
    pub fn storage_root(&self) -> &Path {
        &self.storage_root
    }

    /// The directory the provider reads its configuration from.
    pub fn provider_visible_root(&self) -> &Path {
        &self.provider_visible_root
    }
}

/// A provider-owned environment variable pointed at an overlay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlaySelector {
    env_var: &'static str,
    shape: OverlaySelectorShape,
    value: PathBuf,
}

impl OverlaySelector {
    /// Point `spec`'s variable at `value`, which must be the storage root —
    /// the directory the selector names, not the one the provider reads from.
    pub fn new(spec: &OverlaySelectorSpec, value: impl Into<PathBuf>) -> Self {
        Self {
            env_var: spec.env_var,
            shape: spec.shape,
            value: value.into(),
        }
    }

    /// The environment variable name, e.g. `CODEX_HOME`.
    pub fn env_var(&self) -> &'static str {
        self.env_var
    }

    /// What the value names, relative to the provider's config directory.
    pub fn shape(&self) -> OverlaySelectorShape {
        self.shape
    }

    /// The value the child's environment receives.
    pub fn value(&self) -> &Path {
        &self.value
    }

    /// The directory the provider will read its configuration from.
    pub fn provider_visible_root(&self) -> PathBuf {
        provider_visible_root(self.shape, &self.value)
    }

    /// The entry to write into the child environment.
    pub fn env_entry(&self) -> (OsString, OsString) {
        (
            OsString::from(self.env_var),
            OsString::from(self.value.clone()),
        )
    }
}
