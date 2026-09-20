//! Carrying a launch's provider-state changes back to the user's source root.
//!
//! A launch root is disposable, but a provider may rewrite stable state inside
//! it — most sharply an OAuth token it just rotated, after which the user's
//! original copy no longer authenticates. [`WriteBack`] records, at
//! materialization, the top-level source *files* the overlay mirrored, and when
//! the launch's [`OverlayLease`](super::OverlayLease) is released it copies a
//! changed overlay file back over its source.
//!
//! The rule is deliberately narrow:
//!
//! - **Eligible** are only top-level regular files mirrored from the source (and
//!   Claude's home-root state file). Excluded and repo-masked resources,
//!   materialized replacements, live state, directories, and entries the
//!   provider created are never recorded, so they are never written back.
//! - **Changed** means the overlay entry is now a regular file — no longer the
//!   Unix mirror's link, or a copy whose bytes differ — and its bytes differ
//!   from the source's.
//! - **Refused** when the source's length or modification time moved since
//!   materialization (someone else wrote it), when the overlay entry was
//!   removed, or when Claudine itself wrote the overlay file
//!   ([`record_claudine_write`], used by runtime MCP injection).
//!
//! Directories are not walked. The provider credential files this protects
//! (`auth.json`, `.credentials.json`, `oauth_creds.json`) sit at the top of
//! their roots, while a changed file
//! deeper in a *copied* directory (native Windows) is session data whose
//! write-back would need new-file and conflict rules this does not attempt.
//!
//! A changed file that cannot be written back — a permission error, a sharing
//! violation, a full disk — is reported in [`WriteBackOutcome::failed`], and the
//! lease keeps the root rather than delete the only copy of that state. Failing
//! to *inspect* an entry counts the same: only a `NotFound` overlay entry is
//! "removed", and an unreadable source is never taken as a conflicting change.
//!
//! Only a lease released in-process writes back. A root reclaimed by
//! [`sweep_abandoned_overlays`](super::sweep_abandoned_overlays) after a crash
//! is removed without it: after an arbitrary delay the source-unchanged check
//! can no longer be trusted.

use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

use crate::config::atomic::atomic_write;
use crate::provider::Provider;

/// Overlay files Claudine wrote itself during this process, which a write-back
/// must never carry into the user's source (injected MCP configuration).
fn claudine_writes() -> &'static Mutex<BTreeSet<PathBuf>> {
    static WRITES: OnceLock<Mutex<BTreeSet<PathBuf>>> = OnceLock::new();
    WRITES.get_or_init(|| Mutex::new(BTreeSet::new()))
}

/// Mark `path`, a file Claudine wrote inside an overlay, as never eligible for
/// write-back.
///
/// Process-wide rather than per plan because the runtime MCP injectors receive
/// only a config root, from several launch routes, and must be excluded on all
/// of them. Launch roots are unique, so a path never names another launch's file.
pub fn record_claudine_write(path: &Path) {
    claudine_writes()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .insert(path.to_path_buf());
}

fn is_claudine_write(path: &Path) -> bool {
    claudine_writes()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .contains(path)
}

/// The filesystem operations that decide whether a launch's changed state
/// survives its release.
///
/// Every call write-back and lease release make on the overlay, the source, and
/// the recovery marker goes through this seam, so a test can fail any one of
/// them deterministically on every OS instead of relying on permission modes.
pub(super) trait OverlayIo: std::fmt::Debug + Send + Sync {
    fn symlink_metadata(&self, path: &Path) -> io::Result<fs::Metadata> {
        fs::symlink_metadata(path)
    }
    fn metadata(&self, path: &Path) -> io::Result<fs::Metadata> {
        fs::metadata(path)
    }
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        fs::read(path)
    }
    fn atomic_write(&self, path: &Path, contents: &[u8]) -> io::Result<()> {
        atomic_write(path, contents)
    }
    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        fs::rename(from, to)
    }
    fn remove_file(&self, path: &Path) -> io::Result<()> {
        fs::remove_file(path)
    }
    fn write(&self, path: &Path, contents: &[u8]) -> io::Result<()> {
        fs::write(path, contents)
    }
}

/// The real filesystem.
#[derive(Debug)]
pub(super) struct HostIo;

impl OverlayIo for HostIo {}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Fingerprint {
    len: u64,
    modified: Option<SystemTime>,
}

impl Fingerprint {
    fn of(io: &dyn OverlayIo, path: &Path) -> io::Result<Self> {
        let metadata = io.metadata(path)?;
        Ok(Self {
            len: metadata.len(),
            modified: metadata.modified().ok(),
        })
    }
}

#[derive(Debug)]
struct Entry {
    /// The file actually written: the source entry with symbolic links
    /// resolved, so a dotfiles link in the user's root is kept, not replaced.
    source: PathBuf,
    overlay: PathBuf,
    /// Relative name for diagnostics; never the file's content.
    name: String,
    fingerprint: Fingerprint,
    permissions: fs::Permissions,
}

/// What one released launch did with each eligible file.
#[derive(Debug, Default)]
pub struct WriteBackOutcome {
    /// Names written back to the source.
    pub written: Vec<String>,
    /// Names whose overlay change was refused because the source changed.
    pub refused: Vec<String>,
    /// Changed files that could not be written back. Their overlay copy is the
    /// only copy of that state.
    pub failed: Vec<WriteBackFailure>,
}

/// One changed overlay file that could not be carried back to its source.
#[derive(Debug)]
pub struct WriteBackFailure {
    /// Relative name for diagnostics.
    pub name: String,
    /// The source file that still holds the old state.
    pub source: PathBuf,
    /// The overlay file holding the changed state.
    pub overlay: PathBuf,
    pub error: io::Error,
}

/// The eligible mirrored files of one launch and their source fingerprints.
#[derive(Debug)]
pub struct WriteBack {
    provider: Provider,
    entries: Vec<Entry>,
}

impl WriteBack {
    /// An empty record for `provider`'s launch.
    pub fn new(provider: Provider) -> Self {
        Self {
            provider,
            entries: Vec::new(),
        }
    }

    /// Record that `overlay` mirrors the source file `source`.
    ///
    /// Anything that is not a regular file once links are followed is ignored:
    /// directories are not eligible.
    ///
    /// ## Errors
    ///
    /// Reading the source's metadata or resolving its links.
    pub fn record(&mut self, source: &Path, overlay: &Path) -> io::Result<()> {
        if !fs::metadata(source)?.is_file() {
            return Ok(());
        }
        let resolved = fs::canonicalize(source)?;
        let name = overlay
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        self.entries.push(Entry {
            fingerprint: Fingerprint::of(&HostIo, &resolved)?,
            permissions: fs::metadata(&resolved)?.permissions(),
            source: resolved,
            overlay: overlay.to_path_buf(),
            name,
        });
        Ok(())
    }

    /// The provider whose launch this record belongs to.
    pub fn provider(&self) -> Provider {
        self.provider
    }

    /// Copy every changed eligible overlay file back over its unchanged source.
    ///
    /// A file whose overlay entry, source, or fingerprint cannot be read, or
    /// that cannot be written, is recorded in [`WriteBackOutcome::failed`] and
    /// the rest are still applied: a launch that already ended cannot be
    /// failed, so the caller must keep the overlay copy.
    pub fn apply(&self) -> WriteBackOutcome {
        self.apply_with(&HostIo)
    }

    pub(super) fn apply_with(&self, io: &dyn OverlayIo) -> WriteBackOutcome {
        let mut outcome = WriteBackOutcome::default();
        for entry in &self.entries {
            match self.apply_entry(io, entry) {
                Ok(Some(true)) => outcome.written.push(entry.name.clone()),
                Ok(Some(false)) => outcome.refused.push(entry.name.clone()),
                Ok(None) => {}
                Err(error) => {
                    tracing::warn!(
                        provider = %self.provider,
                        entry = %entry.name,
                        %error,
                        "provider state changed in the overlay could not be written back"
                    );
                    outcome.failed.push(WriteBackFailure {
                        name: entry.name.clone(),
                        source: entry.source.clone(),
                        overlay: entry.overlay.clone(),
                        error,
                    });
                }
            }
        }
        outcome
    }

    /// `Some(true)` written, `Some(false)` refused, `None` nothing to do.
    fn apply_entry(&self, io: &dyn OverlayIo, entry: &Entry) -> io::Result<Option<bool>> {
        let overlay = match io.symlink_metadata(&entry.overlay) {
            Ok(overlay) => overlay,
            // A removal inside the overlay is not propagated.
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error),
        };
        if !overlay.is_file() || is_claudine_write(&entry.overlay) {
            return Ok(None);
        }
        let changed = io.read(&entry.overlay)?;
        if io.read(&entry.source)? == changed {
            return Ok(None);
        }
        if Fingerprint::of(io, &entry.source)? != entry.fingerprint {
            tracing::warn!(
                provider = %self.provider,
                entry = %entry.name,
                "provider state changed in the overlay was not written back: the source changed during the launch"
            );
            return Ok(Some(false));
        }
        // Last rename wins against a writer that slips in after the check.
        io.atomic_write(&entry.source, &changed)?;
        // The temporary file's mode is private; keep the source's own.
        let _ = fs::set_permissions(&entry.source, entry.permissions.clone());
        tracing::debug!(
            provider = %self.provider,
            entry = %entry.name,
            "wrote provider state changed in the overlay back to the source"
        );
        Ok(Some(true))
    }
}
