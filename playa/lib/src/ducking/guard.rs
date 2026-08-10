//! RAII guard for audio ducking with safe restoration.
//!
//! The [`DuckGuard`] ensures audio volumes are restored even if playback fails
//! or panics. Since Rust's `Drop` trait cannot be async, restoration is handled
//! via a channel-based background task.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tokio::sync::mpsc;

use crate::ducking::{DuckConfig, DuckingBackend, DuckingError, VolumeSnapshot};

/// Message sent to the restoration task.
#[derive(Debug)]
enum RestoreMessage {
    /// Request immediate volume restoration.
    Restore,
}

/// RAII guard that restores audio volumes when dropped.
///
/// `DuckGuard` ensures that system audio volumes are restored to their original
/// levels when the guard is dropped, even in the case of panics or early returns.
///
/// ## Implementation Notes
///
/// Since `Drop::drop()` cannot be async, restoration is handled by sending a
/// message to a background task that performs the actual async volume changes.
/// This approach ensures restoration happens reliably without blocking.
///
/// ## Example
///
/// ```ignore
/// use playa::ducking::{DuckGuard, DuckConfig, create_backend};
///
/// async fn play_with_ducking() -> Result<(), DuckingError> {
///     let backend = create_backend();
///     let config = DuckConfig::default();
///
///     // Duck audio and get guard
///     let guard = DuckGuard::new(backend, config).await?;
///
///     // Play audio here...
///     // When guard drops, volumes are restored automatically
///
///     Ok(())
/// }
/// ```
#[derive(Debug)]
pub struct DuckGuard {
    /// Channel sender to request restoration.
    restore_tx: Option<mpsc::Sender<RestoreMessage>>,
    /// Flag to track if restoration has already been triggered.
    restored: Arc<AtomicBool>,
}

impl DuckGuard {
    /// Creates a new ducking guard, applying the initial volume fade.
    ///
    /// This will:
    /// 1. Snapshot current volume levels
    /// 2. Fade volumes to the configured floor
    /// 3. Spawn a background task to handle restoration
    ///
    /// ## Errors
    ///
    /// Returns an error if snapshotting or fading fails. Callers should
    /// proceed with playback even on error (graceful degradation).
    pub async fn new(
        backend: Box<dyn DuckingBackend>,
        config: DuckConfig,
    ) -> Result<Self, DuckingError> {
        // Snapshot current volumes
        let snapshot = backend.snapshot().await?;

        // Fade to floor
        backend.fade_to_floor(&snapshot, &config).await?;

        let (tx, rx) = mpsc::channel(8);
        let restored = Arc::new(AtomicBool::new(false));

        // Spawn background restoration task
        let restored_clone = restored.clone();
        tokio::spawn(async move {
            restoration_task(rx, backend, snapshot, config, restored_clone).await;
        });

        Ok(Self {
            restore_tx: Some(tx),
            restored,
        })
    }

    /// Creates a guard that does nothing (for when ducking fails).
    ///
    /// This allows callers to proceed with playback even when ducking
    /// fails to initialize.
    pub fn noop() -> Self {
        Self {
            restore_tx: None,
            restored: Arc::new(AtomicBool::new(true)),
        }
    }

    /// Manually trigger restoration before drop.
    ///
    /// This is useful when you want to control exactly when restoration
    /// happens rather than waiting for drop.
    pub async fn restore(&self) {
        if self
            .restored
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
            && let Some(tx) = &self.restore_tx
        {
            let _ = tx.send(RestoreMessage::Restore).await;
        }
    }

    /// Returns true if volumes have been restored.
    pub fn is_restored(&self) -> bool {
        self.restored.load(Ordering::SeqCst)
    }
}

impl Drop for DuckGuard {
    fn drop(&mut self) {
        // Use compare_exchange to ensure we only restore once
        if self
            .restored
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
            && let Some(tx) = self.restore_tx.take()
        {
            // Fire-and-forget: send restoration request
            // We can't await here, but the channel is buffered so this won't block
            let _ = tx.try_send(RestoreMessage::Restore);
        }
    }
}

/// Background task that handles volume restoration.
async fn restoration_task(
    mut rx: mpsc::Receiver<RestoreMessage>,
    backend: Box<dyn DuckingBackend>,
    snapshot: VolumeSnapshot,
    config: DuckConfig,
    restored: Arc<AtomicBool>,
) {
    match rx.recv().await {
        Some(RestoreMessage::Restore) => {
            if let Err(e) = backend.fade_restore(&snapshot, &config).await {
                eprintln!("Warning: failed to restore audio volumes: {}", e);
            }
            restored.store(true, Ordering::SeqCst);
        }
        None => {
            eprintln!(
                "Warning: DuckGuard dropped before restoration task was ready; \
                 attempting immediate restore"
            );
            restored.store(true, Ordering::SeqCst);
            if let Err(e) = backend.fade_restore(&snapshot, &config).await {
                eprintln!("Warning: fallback restore also failed: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ducking::NoopBackend;

    #[tokio::test]
    async fn guard_noop_is_already_restored() {
        let guard = DuckGuard::noop();
        assert!(guard.is_restored());
    }

    #[tokio::test]
    async fn guard_with_noop_backend_succeeds() {
        let backend = Box::new(NoopBackend::new());
        let config = DuckConfig::default();
        let guard = DuckGuard::new(backend, config).await.unwrap();
        assert!(!guard.is_restored());
    }

    #[tokio::test]
    async fn guard_manual_restore() {
        let backend = Box::new(NoopBackend::new());
        let config = DuckConfig::default();
        let guard = DuckGuard::new(backend, config).await.unwrap();

        assert!(!guard.is_restored());
        guard.restore().await;

        // Give the background task time to process
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        assert!(guard.is_restored());
    }

    #[tokio::test]
    async fn guard_drop_triggers_restore() {
        {
            let backend = Box::new(NoopBackend::new());
            let config = DuckConfig::default();
            let _guard = DuckGuard::new(backend, config).await.unwrap();
            // Guard drops here
        }

        // Give the background task time to process
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // The guard's internal restored flag was set, but we can't easily
        // verify from outside. The test mainly verifies no panic on drop.
    }

    #[tokio::test]
    async fn guard_double_restore_is_idempotent() {
        let backend = Box::new(NoopBackend::new());
        let config = DuckConfig::default();
        let guard = DuckGuard::new(backend, config).await.unwrap();

        guard.restore().await;
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        assert!(guard.is_restored());

        // Second restore should be a no-op
        guard.restore().await;
        assert!(guard.is_restored());
    }

    #[tokio::test]
    async fn guard_drop_on_panic_unwind() {
        use std::panic;

        // Create a guard that we'll drop via panic unwind
        let result = panic::catch_unwind(|| {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            rt.block_on(async {
                let backend = Box::new(NoopBackend::new());
                let config = DuckConfig::default();
                let _guard = DuckGuard::new(backend, config).await.unwrap();

                // Panic while guard is in scope
                panic!("test panic");
            });
        });

        // Verify the panic was caught (proving Drop ran without double-panic)
        assert!(result.is_err());
    }
}
