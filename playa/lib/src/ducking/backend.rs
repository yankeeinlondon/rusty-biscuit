//! Backend trait for platform-specific audio ducking implementations.
//!
//! Each platform implements [`DuckingBackend`] to handle the actual volume
//! manipulation. The trait is designed for async execution with Send-safe futures.

use std::future::Future;
use std::pin::Pin;

use crate::ducking::{DuckConfig, DuckingError, VolumeSnapshot};

/// Type alias for boxed async results (Send-safe).
pub type DuckResult<'a, T> = Pin<Box<dyn Future<Output = Result<T, DuckingError>> + Send + 'a>>;

/// Backend trait for platform-specific audio ducking.
///
/// Implementors provide the actual volume manipulation for their platform:
/// - macOS: CoreAudio
/// - Windows: WASAPI
/// - Linux: PulseAudio/PipeWire/ALSA
///
/// All async methods return Send-safe futures to support use in async runtimes
/// like Tokio.
pub trait DuckingBackend: Send + Sync {
    /// Captures the current volume state of all audio sessions.
    ///
    /// This snapshot is used to restore volumes after ducking completes.
    /// Implementations should capture all relevant sessions except Playa's own.
    fn snapshot(&self) -> DuckResult<'_, VolumeSnapshot>;

    /// Fades all captured sessions to the configured floor level.
    ///
    /// This performs a gradual volume reduction over `config.ramp_ms()` milliseconds
    /// to avoid jarring audio transitions.
    fn fade_to_floor(&self, snapshot: &VolumeSnapshot, config: &DuckConfig) -> DuckResult<'_, ()>;

    /// Restores all sessions to their original volumes from the snapshot.
    ///
    /// This performs a gradual volume increase over `config.ramp_ms()` milliseconds.
    /// Called both on normal completion and in Drop for cleanup.
    fn fade_restore(&self, snapshot: &VolumeSnapshot, config: &DuckConfig) -> DuckResult<'_, ()>;

    /// Returns the backend name for diagnostics.
    fn name(&self) -> &'static str;

    /// Returns true if this backend is functional on the current system.
    ///
    /// Some backends may be compiled but not usable (e.g., PulseAudio on a
    /// system without a running pulse daemon).
    fn is_available(&self) -> bool {
        true
    }
}

/// No-op backend for unsupported platforms or when ducking is disabled.
///
/// This implementation performs no actual volume changes but satisfies the
/// trait requirements, allowing graceful degradation on unsupported systems.
#[derive(Debug, Default)]
pub struct NoopBackend;

impl NoopBackend {
    /// Creates a new no-op backend.
    pub fn new() -> Self {
        Self
    }
}

impl DuckingBackend for NoopBackend {
    fn snapshot(&self) -> DuckResult<'_, VolumeSnapshot> {
        Box::pin(async { Ok(VolumeSnapshot::new()) })
    }

    fn fade_to_floor(
        &self,
        _snapshot: &VolumeSnapshot,
        _config: &DuckConfig,
    ) -> DuckResult<'_, ()> {
        Box::pin(async { Ok(()) })
    }

    fn fade_restore(&self, _snapshot: &VolumeSnapshot, _config: &DuckConfig) -> DuckResult<'_, ()> {
        Box::pin(async { Ok(()) })
    }

    fn name(&self) -> &'static str {
        "noop"
    }

    fn is_available(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn noop_backend_snapshot_returns_empty() {
        let backend = NoopBackend::new();
        let snapshot = backend.snapshot().await.unwrap();
        assert!(snapshot.is_empty());
    }

    #[tokio::test]
    async fn noop_backend_fade_to_floor_succeeds() {
        let backend = NoopBackend::new();
        let snapshot = VolumeSnapshot::new();
        let config = DuckConfig::default();
        assert!(backend.fade_to_floor(&snapshot, &config).await.is_ok());
    }

    #[tokio::test]
    async fn noop_backend_fade_restore_succeeds() {
        let backend = NoopBackend::new();
        let snapshot = VolumeSnapshot::new();
        let config = DuckConfig::default();
        assert!(backend.fade_restore(&snapshot, &config).await.is_ok());
    }

    #[test]
    fn noop_backend_name() {
        let backend = NoopBackend::new();
        assert_eq!(backend.name(), "noop");
    }

    #[test]
    fn noop_backend_is_available() {
        let backend = NoopBackend::new();
        assert!(backend.is_available());
    }

    #[test]
    fn noop_backend_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<NoopBackend>();
    }

    #[test]
    fn trait_object_is_send() {
        fn assert_send<T: Send>() {}
        assert_send::<Box<dyn DuckingBackend>>();
    }
}
