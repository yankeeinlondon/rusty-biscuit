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

/// Playback state detected from macOS Now Playing info.
#[derive(Debug, Clone, Copy, PartialEq)]
enum PlaybackState {
    /// Media is currently playing.
    Playing,
    /// Media is paused or stopped.
    Paused,
    /// Could not determine playback state.
    Unknown,
}

/// macOS media key backend for audio ducking fallback.
///
/// Uses AppleScript via `osascript` to simulate media key presses, pausing
/// other audio during playback and resuming it afterward.
///
/// ## Behavior
///
/// 1. On duck: Detects if media is currently playing, then sends pause
/// 2. On restore: Only sends play if media was playing before ducking
///
/// This ensures we don't accidentally start playing something that was
/// already paused.
///
/// ## Limitations
///
/// - Only affects apps that respond to media keys (most media players do)
/// - Won't pause system sounds, notification sounds, or non-media apps
#[derive(Debug)]
pub struct MediaKeysBackend {
    /// Whether media was playing when we started ducking.
    was_playing: AtomicBool,
    /// Whether we successfully sent a pause command.
    did_pause: AtomicBool,
}

impl MediaKeysBackend {
    /// Creates a new media keys backend.
    pub fn new() -> Self {
        Self {
            was_playing: AtomicBool::new(false),
            did_pause: AtomicBool::new(false),
        }
    }

    /// Detects the current playback state using Now Playing info.
    ///
    /// Uses AppleScript to query the system's now playing state via
    /// the Media Remote framework (exposed through System Events).
    fn detect_playback_state(&self) -> PlaybackState {
        // AppleScript to get the current playback state
        // This queries the system's Now Playing info
        let script = r#"
            use framework "Foundation"
            use scripting additions

            -- Try to get now playing info via Media Remote
            -- Returns "playing", "paused", or "unknown"
            try
                set nowPlayingInfo to do shell script "nowplaying-cli get playbackRate 2>/dev/null || echo unknown"
                if nowPlayingInfo is "1" or nowPlayingInfo is "1.0" then
                    return "playing"
                else if nowPlayingInfo is "0" or nowPlayingInfo is "0.0" then
                    return "paused"
                else
                    return "unknown"
                end if
            on error
                return "unknown"
            end try
        "#;

        let output = Command::new("osascript")
            .arg("-e")
            .arg(script)
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let state = String::from_utf8_lossy(&out.stdout).trim().to_lowercase();
                match state.as_str() {
                    "playing" => PlaybackState::Playing,
                    "paused" => PlaybackState::Paused,
                    _ => {
                        // Fallback: try alternative detection method
                        self.detect_playback_state_fallback()
                    }
                }
            }
            _ => self.detect_playback_state_fallback(),
        }
    }

    /// Fallback detection using common media apps directly.
    ///
    /// Note: This only detects apps with AppleScript support. For better detection
    /// of browser audio, podcasts, etc., install `nowplaying-cli`:
    /// `brew install nowplaying-cli`
    fn detect_playback_state_fallback(&self) -> PlaybackState {
        // Try checking common media apps directly
        let script = r#"
            -- Check if any common media app is playing
            set isPlaying to false

            -- Check Spotify
            if application "Spotify" is running then
                tell application "Spotify"
                    if player state is playing then
                        set isPlaying to true
                    end if
                end tell
            end if

            -- Check Music (Apple Music)
            if not isPlaying and application "Music" is running then
                tell application "Music"
                    if player state is playing then
                        set isPlaying to true
                    end if
                end tell
            end if

            -- Check iTunes (older macOS)
            if not isPlaying and application "iTunes" is running then
                tell application "iTunes"
                    if player state is playing then
                        set isPlaying to true
                    end if
                end tell
            end if

            -- Check VLC
            if not isPlaying and application "VLC" is running then
                tell application "VLC"
                    if playing then
                        set isPlaying to true
                    end if
                end tell
            end if

            -- Check IINA
            if not isPlaying and application "IINA" is running then
                try
                    tell application "IINA"
                        if (count of windows) > 0 then
                            -- IINA doesn't expose playback state directly
                            -- Assume playing if window is open
                            set isPlaying to true
                        end if
                    end tell
                end try
            end if

            -- Check Podcasts
            if not isPlaying and application "Podcasts" is running then
                try
                    tell application "Podcasts"
                        if player state is playing then
                            set isPlaying to true
                        end if
                    end tell
                end try
            end if

            -- Check QuickTime Player
            if not isPlaying and application "QuickTime Player" is running then
                try
                    tell application "QuickTime Player"
                        if (count of documents) > 0 then
                            if playing of document 1 then
                                set isPlaying to true
                            end if
                        end if
                    end tell
                end try
            end if

            -- Check TIDAL
            if not isPlaying and application "TIDAL" is running then
                try
                    tell application "TIDAL"
                        if playerState is 1 then
                            set isPlaying to true
                        end if
                    end tell
                end try
            end if

            -- Check Deezer
            if not isPlaying and application "Deezer" is running then
                try
                    tell application "Deezer"
                        if player state is playing then
                            set isPlaying to true
                        end if
                    end tell
                end try
            end if

            if isPlaying then
                return "playing"
            else
                return "paused"
            end if
        "#;

        let output = Command::new("osascript")
            .arg("-e")
            .arg(script)
            .output();

        match output {
            Ok(out) if out.status.success() => {
                let state = String::from_utf8_lossy(&out.stdout).trim().to_lowercase();
                match state.as_str() {
                    "playing" => PlaybackState::Playing,
                    _ => PlaybackState::Paused,
                }
            }
            _ => PlaybackState::Unknown,
        }
    }

    /// Sends the pause media key via AppleScript.
    fn send_pause(&self) -> Result<(), DuckingError> {
        // Key code 100 is the play/pause media key
        let script = r#"
            tell application "System Events"
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
            if !stderr.contains("not allowed") && !stderr.is_empty() {
                return Err(DuckingError::Platform(format!(
                    "osascript failed: {}",
                    stderr.trim()
                )));
            }
        }

        Ok(())
    }

    /// Sends the play media key via AppleScript.
    fn send_play(&self) -> Result<(), DuckingError> {
        // Same key code - it's a toggle, but we only call this when we know
        // we previously paused something that was playing
        self.send_pause()
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
            // Detect current playback state
            let state = self.detect_playback_state();
            let is_playing = state == PlaybackState::Playing;

            // Store as 1.0 for playing, 0.0 for paused/unknown
            let mut snapshot = VolumeSnapshot::new();
            snapshot.push(SessionVolume::new(
                SessionId::MediaKeys,
                vec![if is_playing { 1.0 } else { 0.0 }],
                false,
            ));

            Ok(snapshot)
        })
    }

    fn fade_to_floor(
        &self,
        snapshot: &VolumeSnapshot,
        _config: &DuckConfig,
    ) -> DuckResult<'_, ()> {
        let snapshot = snapshot.clone();

        Box::pin(async move {
            // Check if media was playing from snapshot
            let was_playing = snapshot
                .entries
                .first()
                .and_then(|e| e.channels.first())
                .map(|v| *v > 0.5)
                .unwrap_or(false);

            self.was_playing.store(was_playing, Ordering::SeqCst);

            // Only pause if something is playing
            if was_playing {
                self.send_pause()?;
                self.did_pause.store(true, Ordering::SeqCst);

                // Small delay to let the pause take effect
                tokio::time::sleep(std::time::Duration::from_millis(150)).await;
            }

            Ok(())
        })
    }

    fn fade_restore(
        &self,
        _snapshot: &VolumeSnapshot,
        _config: &DuckConfig,
    ) -> DuckResult<'_, ()> {
        Box::pin(async move {
            // Only resume if we actually paused something that was playing
            let was_playing = self.was_playing.load(Ordering::SeqCst);
            let did_pause = self.did_pause.swap(false, Ordering::SeqCst);

            if was_playing && did_pause {
                // Small delay before resuming to let our audio finish cleanly
                tokio::time::sleep(std::time::Duration::from_millis(150)).await;

                // Send play to resume
                self.send_play()?;
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
        // Snapshot should contain playback state (0.0 or 1.0)
        let volume = snapshot.entries[0].channels[0];
        assert!(volume == 0.0 || volume == 1.0);
    }

    #[test]
    fn playback_state_equality() {
        assert_eq!(PlaybackState::Playing, PlaybackState::Playing);
        assert_eq!(PlaybackState::Paused, PlaybackState::Paused);
        assert_ne!(PlaybackState::Playing, PlaybackState::Paused);
    }

    // Integration tests that actually send media keys are marked #[ignore]
    // Run with: cargo test -p playa --features audio-ducking-macos -- --ignored

    #[tokio::test]
    #[ignore = "actually sends media keys - will pause your music!"]
    async fn media_keys_backend_pause_resume_cycle() {
        let backend = MediaKeysBackend::new();
        let config = DuckConfig::default();

        // Take snapshot (detects if media is playing)
        let snapshot = backend.snapshot().await.unwrap();
        let was_playing = snapshot.entries[0].channels[0] > 0.5;
        println!("Was playing: {}", was_playing);

        // This will pause if something was playing
        backend.fade_to_floor(&snapshot, &config).await.unwrap();

        if was_playing {
            assert!(backend.did_pause.load(Ordering::SeqCst));
        }

        // Wait a moment
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        // This will resume only if we paused something that was playing
        backend.fade_restore(&snapshot, &config).await.unwrap();
        assert!(!backend.did_pause.load(Ordering::SeqCst));
    }

    #[tokio::test]
    #[ignore = "queries system state"]
    async fn media_keys_backend_detects_playback_state() {
        let backend = MediaKeysBackend::new();
        let state = backend.detect_playback_state();
        println!("Current playback state: {:?}", state);
        // Just verify it returns a valid state
        assert!(matches!(
            state,
            PlaybackState::Playing | PlaybackState::Paused | PlaybackState::Unknown
        ));
    }
}
