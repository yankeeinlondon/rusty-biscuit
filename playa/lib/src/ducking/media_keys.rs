//! macOS media key backend for audio ducking fallback.
//!
//! When CoreAudio volume control is unavailable (e.g., professional DACs, USB
//! audio interfaces), this backend uses media key simulation to pause other
//! audio during playback, then resume it afterward.
//!
//! This approach works with any app that responds to system media keys:
//! Spotify, Apple Music, YouTube, podcasts, etc.

use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::ducking::{
    DuckConfig, DuckResult, DuckingBackend, DuckingError, SessionId, SessionVolume, VolumeSnapshot,
};

/// macOS media key backend for audio ducking fallback.
///
/// Uses AppleScript via `osascript` to simulate media key presses, pausing
/// other audio during playback and resuming it afterward.
///
/// ## Limitations
///
/// - Only affects apps that respond to media keys (most media players do)
/// - Won't pause system sounds, notification sounds, or non-media apps
/// - Resume sends another play/pause toggle, which may not work perfectly
///   if the user had manually paused something
#[derive(Debug)]
pub struct MediaKeysBackend {
    /// Whether we successfully paused (to know if we should resume).
    paused: AtomicBool,
}

impl MediaKeysBackend {
    /// Creates a new media keys backend.
    pub fn new() -> Self {
        Self {
            paused: AtomicBool::new(false),
        }
    }

    /// Sends the play/pause media key via AppleScript.
    ///
    /// Uses `key code 49 using {command down}` which triggers the system
    /// media play/pause action on macOS.
    fn send_play_pause(&self) -> Result<(), DuckingError> {
        // AppleScript to simulate media key press
        // Key code 100 is the play/pause media key
        // Alternative: use System Events to send the key
        let script = r#"
            tell application "System Events"
                -- Media play/pause key code
                key code 100
            end tell
        "#;

        let output = Command::new("osascript")
            .arg("-e")
            .arg(script)
            .output()
            .map_err(|e| DuckingError::Platform(format!("failed to run osascript: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Don't fail if it's just an accessibility permission issue
            // The key press might still work
            if !stderr.contains("not allowed") {
                return Err(DuckingError::Platform(format!(
                    "osascript failed: {}",
                    stderr.trim()
                )));
            }
        }

        Ok(())
    }
}

impl Default for MediaKeysBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl DuckingBackend for MediaKeysBackend {
    fn snapshot(&self) -> DuckResult<'_, VolumeSnapshot> {
        Box::pin(async move {
            // We don't actually snapshot anything - just create a marker
            let mut snapshot = VolumeSnapshot::new();
            snapshot.push(SessionVolume::new(
                SessionId::MediaKeys,
                vec![1.0], // Placeholder
                false,
            ));
            Ok(snapshot)
        })
    }

    fn fade_to_floor(
        &self,
        _snapshot: &VolumeSnapshot,
        _config: &DuckConfig,
    ) -> DuckResult<'_, ()> {
        Box::pin(async move {
            // Send pause media key
            self.send_play_pause()?;
            self.paused.store(true, Ordering::SeqCst);

            // Small delay to let the pause take effect
            tokio::time::sleep(std::time::Duration::from_millis(150)).await;

            Ok(())
        })
    }

    fn fade_restore(
        &self,
        _snapshot: &VolumeSnapshot,
        _config: &DuckConfig,
    ) -> DuckResult<'_, ()> {
        Box::pin(async move {
            // Only resume if we paused
            if self.paused.swap(false, Ordering::SeqCst) {
                // Small delay before resuming to let our audio finish cleanly
                tokio::time::sleep(std::time::Duration::from_millis(150)).await;

                // Send play media key (same key toggles)
                self.send_play_pause()?;
            }

            Ok(())
        })
    }

    fn name(&self) -> &'static str {
        "macos-media-keys"
    }

    fn is_available(&self) -> bool {
        // Check if osascript is available (it should be on all macOS systems)
        Command::new("osascript")
            .arg("-e")
            .arg("return 1")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn media_keys_backend_name() {
        let backend = MediaKeysBackend::new();
        assert_eq!(backend.name(), "macos-media-keys");
    }

    #[test]
    fn media_keys_backend_is_available() {
        let backend = MediaKeysBackend::new();
        // Should be available on macOS
        #[cfg(target_os = "macos")]
        assert!(backend.is_available());
    }

    #[test]
    fn media_keys_backend_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<MediaKeysBackend>();
    }

    #[tokio::test]
    async fn media_keys_backend_snapshot_returns_marker() {
        let backend = MediaKeysBackend::new();
        let snapshot = backend.snapshot().await.unwrap();
        assert_eq!(snapshot.len(), 1);
        assert!(matches!(snapshot.entries[0].id, SessionId::MediaKeys));
    }

    // Integration tests that actually send media keys are marked #[ignore]
    // Run with: cargo test -p playa --features audio-ducking-macos -- --ignored

    #[tokio::test]
    #[ignore = "actually sends media keys - will pause your music!"]
    async fn media_keys_backend_pause_resume_cycle() {
        let backend = MediaKeysBackend::new();
        let config = DuckConfig::default();
        let snapshot = backend.snapshot().await.unwrap();

        // This will actually pause whatever is playing
        backend.fade_to_floor(&snapshot, &config).await.unwrap();
        assert!(backend.paused.load(Ordering::SeqCst));

        // Wait a moment
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        // This will resume
        backend.fade_restore(&snapshot, &config).await.unwrap();
        assert!(!backend.paused.load(Ordering::SeqCst));
    }
}
