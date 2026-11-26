//! Request-scoped observations of the first component below a launch base.

use std::{
    collections::HashMap,
    ffi::{OsStr, OsString},
    path::Path,
    sync::{Arc, Mutex},
};

use super::{PathIdentity, ProbeError};
use crate::file_reference::resolve::{Boundary, validate_containment};
use crate::FileReferenceError;

/// Filesystem observations used to rule out a nearer launch candidate.
pub trait ShadowProbe: Send + Sync {
    /// Authorize a path before the shadow check inspects its containment or
    /// metadata. A caller with a read policy can override this method.
    fn admit(&self, _path: &Path) -> Result<(), ProbeError> {
        Ok(())
    }

    /// Check whether the entry is present, including a dangling symlink.
    fn exists(&self, path: &Path) -> Result<bool, ProbeError>;

    /// Check a prospective target's parent. The default consults
    /// `exists` first so a probe's access policy also gates this lookup.
    fn is_dir(&self, path: &Path) -> Result<bool, ProbeError> {
        if !self.exists(path)? {
            return Ok(false);
        }
        FsShadowProbe.is_dir(path)
    }
}

/// The default probe for callers that do not need an access policy.
#[derive(Debug, Default)]
pub struct FsShadowProbe;

impl ShadowProbe for FsShadowProbe {
    fn exists(&self, path: &Path) -> Result<bool, ProbeError> {
        match std::fs::symlink_metadata(path) {
            Ok(_) => Ok(true),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(ProbeError::new(path, &error)),
        }
    }

    fn is_dir(&self, path: &Path) -> Result<bool, ProbeError> {
        match std::fs::metadata(path) {
            Ok(metadata) => Ok(metadata.is_dir()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(ProbeError::new(path, &error)),
        }
    }
}

type Observations = HashMap<(PathIdentity, OsString), Result<bool, ProbeError>>;

/// Share first-component observations across spellings in a single request.
#[derive(Clone)]
pub struct ShadowCache {
    probe: Arc<dyn ShadowProbe>,
    observations: Arc<Mutex<Observations>>,
}

impl std::fmt::Debug for ShadowCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShadowCache")
            .field("probe_count", &self.probe_count())
            .finish_non_exhaustive()
    }
}

impl Default for ShadowCache {
    fn default() -> Self {
        Self::new(Arc::new(FsShadowProbe))
    }
}

impl ShadowCache {
    pub fn new(probe: Arc<dyn ShadowProbe>) -> Self {
        Self { probe, observations: Arc::default() }
    }

    /// Number of distinct first-component lookups, including refused probes.
    pub fn probe_count(&self) -> usize {
        self.observations.lock().expect("shadow cache poisoned").len()
    }

    pub(crate) fn first_exists(
        &self,
        base: &Path,
        first: &OsStr,
        boundary: Option<Boundary<'_>>,
    ) -> Result<bool, ProbeError> {
        self.observations.lock().expect("shadow cache poisoned")
            .entry((PathIdentity::new(base), first.to_os_string()))
            .or_insert_with(|| {
                let path = base.join(first);
                self.probe.admit(&path)?;
                if let Some(boundary) = boundary {
                    validate_containment(boundary, &path.to_string_lossy(), base).map_err(|error| {
                        match error {
                            FileReferenceError::Io { path, source } => ProbeError::new(&path, &source),
                            other => ProbeError {
                                path: path.clone(),
                                kind: std::io::ErrorKind::PermissionDenied,
                                os_code: None,
                                message: other.to_string(),
                            },
                        }
                    })?;
                }
                self.probe.exists(&path)
            })
            .clone()
    }

    pub(crate) fn probe(&self) -> &dyn ShadowProbe {
        self.probe.as_ref()
    }
}
