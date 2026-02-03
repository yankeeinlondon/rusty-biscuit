//! Linux PulseAudio/PipeWire backend for audio ducking.
//!
//! This backend uses PulseAudio (or PipeWire's PulseAudio compatibility layer)
//! to duck individual sink inputs (applications playing audio) while excluding
//! Playa's own audio stream.
//!
//! Falls back to ALSA master mixer control on systems without PulseAudio.

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use pulsectl::controllers::types::ApplicationInfo;
use pulsectl::controllers::{AppControl, SinkController};

use crate::ducking::{
    DuckConfig, DuckResult, DuckingBackend, DuckingError, SessionId, SessionVolume, VolumeSnapshot,
    compute_fade_steps,
};

/// Linux PulseAudio/PipeWire backend for audio ducking.
///
/// Ducks individual sink inputs (applications) rather than system-wide volume,
/// allowing Playa's own audio to play at full volume while other apps are ducked.
///
/// ## Self-Exclusion
///
/// Excludes Playa's own audio by matching on:
/// - `application.process.id` (PID) matching current process
/// - `application.name` containing "playa" (case-insensitive)
///
/// ## PipeWire Compatibility
///
/// This backend works with PipeWire's PulseAudio compatibility layer, which is
/// the default on many modern Linux distributions (Fedora, Ubuntu 22.10+).
#[derive(Debug)]
pub struct LinuxBackend {
    /// Whether PulseAudio is available.
    pulse_available: AtomicBool,
    /// Current process PID for self-exclusion.
    our_pid: u32,
    /// Cached original volumes for restoration (index -> volume percentage).
    original_volumes: Mutex<HashMap<u32, f64>>,
}

impl LinuxBackend {
    /// Creates a new Linux backend.
    ///
    /// Checks if PulseAudio is available by attempting to create a controller.
    pub fn new() -> Self {
        let pulse_available = check_pulse_available();
        let our_pid = std::process::id();

        Self {
            pulse_available: AtomicBool::new(pulse_available),
            our_pid,
            original_volumes: Mutex::new(HashMap::new()),
        }
    }

    /// Returns true if the given application should be excluded from ducking.
    ///
    /// Excludes based on:
    /// - PID matching current process
    /// - Application name containing "playa"
    fn should_exclude(&self, app: &ApplicationInfo) -> bool {
        // Check PID from proplist
        if let Some(pid_str) = app.proplist.get_str("application.process.id") {
            if let Ok(pid) = pid_str.parse::<u32>() {
                if pid == self.our_pid {
                    return true;
                }
            }
        }

        // Check application name
        if let Some(ref name) = app.name {
            if name.to_lowercase().contains("playa") {
                return true;
            }
        }

        // Also check proplist application.name
        if let Some(app_name) = app.proplist.get_str("application.name") {
            if app_name.to_lowercase().contains("playa") {
                return true;
            }
        }

        false
    }

    /// Converts a volume scalar (0.0-1.0) to a percentage (0-100) for pulsectl.
    fn scalar_to_percent(scalar: f32) -> f64 {
        (scalar * 100.0) as f64
    }

    /// Converts a pulsectl volume percentage (0-100+) to a scalar (0.0-1.0+).
    fn percent_to_scalar(percent: f64) -> f32 {
        (percent / 100.0) as f32
    }
}

impl Default for LinuxBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl DuckingBackend for LinuxBackend {
    fn snapshot(&self) -> DuckResult<'_, VolumeSnapshot> {
        Box::pin(async move {
            if !self.pulse_available.load(Ordering::SeqCst) {
                return Err(DuckingError::BackendUnavailable(
                    "PulseAudio not available".to_string(),
                ));
            }

            let mut controller = SinkController::create().map_err(|e| {
                DuckingError::Platform(format!("failed to connect to PulseAudio: {}", e))
            })?;

            let apps = controller.list_applications().map_err(|e| {
                DuckingError::SnapshotFailed(format!("failed to list applications: {}", e))
            })?;

            let mut snapshot = VolumeSnapshot::new();
            let mut original_volumes = self.original_volumes.lock().unwrap();
            original_volumes.clear();

            for app in apps {
                // Skip our own audio
                if self.should_exclude(&app) {
                    continue;
                }

                // Skip apps without writable volume
                if !app.volume_writable {
                    continue;
                }

                // Get the average volume across channels as a percentage
                let avg_volume = app.volume.avg().0 as f64 / 65536.0 * 100.0;

                // Store original volume for restoration
                original_volumes.insert(app.index, avg_volume);

                // Create snapshot entry
                let app_name = app
                    .name
                    .clone()
                    .unwrap_or_else(|| format!("app-{}", app.index));
                snapshot.push(SessionVolume::new(
                    SessionId::PulseSinkInput {
                        index: app.index,
                        name: app_name,
                    },
                    vec![Self::percent_to_scalar(avg_volume)],
                    app.mute,
                ));
            }

            Ok(snapshot)
        })
    }

    fn fade_to_floor(&self, snapshot: &VolumeSnapshot, config: &DuckConfig) -> DuckResult<'_, ()> {
        let snapshot = snapshot.clone();
        let config = *config;

        Box::pin(async move {
            if snapshot.is_empty() {
                return Ok(());
            }

            let mut controller = SinkController::create().map_err(|e| {
                DuckingError::Platform(format!("failed to connect to PulseAudio: {}", e))
            })?;

            // For each app, fade to floor
            for entry in &snapshot.entries {
                let SessionId::PulseSinkInput { index, .. } = &entry.id else {
                    continue;
                };

                let original_volume = entry.channels.first().copied().unwrap_or(1.0);
                let target_volume = original_volume * config.floor_scalar();

                // Compute fade steps
                let steps = compute_fade_steps(original_volume, target_volume, &config);

                // Apply each step
                for step in steps {
                    let percent = Self::scalar_to_percent(step.volume);

                    // Calculate delta from current volume
                    // pulsectl uses increase/decrease by percent, so we need to get current and adjust
                    if let Ok(Some(app)) = controller.get_app_by_index(*index) {
                        let current_percent = app.volume.avg().0 as f64 / 65536.0 * 100.0;
                        let delta = percent - current_percent;

                        if delta < 0.0 {
                            let _ = controller.decrease_app_volume_by_percent(*index, -delta);
                        } else if delta > 0.0 {
                            let _ = controller.increase_app_volume_by_percent(*index, delta);
                        }
                    }

                    tokio::time::sleep(std::time::Duration::from_millis(step.delay_ms as u64))
                        .await;
                }
            }

            Ok(())
        })
    }

    fn fade_restore(&self, snapshot: &VolumeSnapshot, config: &DuckConfig) -> DuckResult<'_, ()> {
        let snapshot = snapshot.clone();
        let config = *config;
        let original_volumes = self.original_volumes.lock().unwrap().clone();

        Box::pin(async move {
            if snapshot.is_empty() {
                return Ok(());
            }

            let mut controller = SinkController::create().map_err(|e| {
                DuckingError::Platform(format!("failed to connect to PulseAudio: {}", e))
            })?;

            // For each app, fade back to original
            for entry in &snapshot.entries {
                let SessionId::PulseSinkInput { index, .. } = &entry.id else {
                    continue;
                };

                // Get original volume from our cache
                let Some(&original_percent) = original_volumes.get(index) else {
                    continue;
                };

                // Get current volume
                let Ok(Some(app)) = controller.get_app_by_index(*index) else {
                    continue;
                };

                let current_percent = app.volume.avg().0 as f64 / 65536.0 * 100.0;
                let current_scalar = Self::percent_to_scalar(current_percent);
                let target_scalar = Self::percent_to_scalar(original_percent);

                // Compute fade steps
                let steps = compute_fade_steps(current_scalar, target_scalar, &config);

                // Apply each step
                for step in steps {
                    let percent = Self::scalar_to_percent(step.volume);

                    if let Ok(Some(app)) = controller.get_app_by_index(*index) {
                        let current = app.volume.avg().0 as f64 / 65536.0 * 100.0;
                        let delta = percent - current;

                        if delta < 0.0 {
                            let _ = controller.decrease_app_volume_by_percent(*index, -delta);
                        } else if delta > 0.0 {
                            let _ = controller.increase_app_volume_by_percent(*index, delta);
                        }
                    }

                    tokio::time::sleep(std::time::Duration::from_millis(step.delay_ms as u64))
                        .await;
                }
            }

            Ok(())
        })
    }

    fn name(&self) -> &'static str {
        "linux-pulse"
    }

    fn is_available(&self) -> bool {
        self.pulse_available.load(Ordering::SeqCst)
    }
}

/// Checks if PulseAudio is available by attempting to create a controller.
fn check_pulse_available() -> bool {
    SinkController::create().is_ok()
}

/// ALSA fallback backend for systems without PulseAudio.
///
/// This provides basic master volume ducking using ALSA's mixer interface.
/// It's less precise than PulseAudio (affects all audio, including Playa's)
/// but works on minimal Linux systems.
#[derive(Debug)]
pub struct AlsaBackend {
    /// Whether ALSA mixer is available.
    available: AtomicBool,
}

impl AlsaBackend {
    /// Creates a new ALSA backend.
    pub fn new() -> Self {
        let available = check_alsa_available();
        Self {
            available: AtomicBool::new(available),
        }
    }
}

impl Default for AlsaBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl DuckingBackend for AlsaBackend {
    fn snapshot(&self) -> DuckResult<'_, VolumeSnapshot> {
        Box::pin(async move {
            if !self.available.load(Ordering::SeqCst) {
                return Err(DuckingError::BackendUnavailable(
                    "ALSA mixer not available".to_string(),
                ));
            }

            let volume = get_alsa_master_volume()?;

            let mut snapshot = VolumeSnapshot::new();
            snapshot.push(SessionVolume::new(
                SessionId::AlsaMaster,
                vec![volume],
                false,
            ));

            Ok(snapshot)
        })
    }

    fn fade_to_floor(&self, snapshot: &VolumeSnapshot, config: &DuckConfig) -> DuckResult<'_, ()> {
        let snapshot = snapshot.clone();
        let config = *config;

        Box::pin(async move {
            let original_volume = snapshot
                .entries
                .first()
                .and_then(|e| e.channels.first().copied())
                .unwrap_or(1.0);

            let target = original_volume * config.floor_scalar();
            let steps = compute_fade_steps(original_volume, target, &config);

            for step in steps {
                set_alsa_master_volume(step.volume)?;
                tokio::time::sleep(std::time::Duration::from_millis(step.delay_ms as u64)).await;
            }

            Ok(())
        })
    }

    fn fade_restore(&self, snapshot: &VolumeSnapshot, config: &DuckConfig) -> DuckResult<'_, ()> {
        let snapshot = snapshot.clone();
        let config = *config;

        Box::pin(async move {
            let original_volume = snapshot
                .entries
                .first()
                .and_then(|e| e.channels.first().copied())
                .unwrap_or(1.0);

            let current_volume = get_alsa_master_volume()?;
            let steps = compute_fade_steps(current_volume, original_volume, &config);

            for step in steps {
                set_alsa_master_volume(step.volume)?;
                tokio::time::sleep(std::time::Duration::from_millis(step.delay_ms as u64)).await;
            }

            Ok(())
        })
    }

    fn name(&self) -> &'static str {
        "linux-alsa"
    }

    fn is_available(&self) -> bool {
        self.available.load(Ordering::SeqCst)
    }
}

/// Checks if ALSA master mixer is available.
fn check_alsa_available() -> bool {
    get_alsa_master_volume().is_ok()
}

/// Gets the ALSA master volume as a scalar (0.0-1.0).
fn get_alsa_master_volume() -> Result<f32, DuckingError> {
    use alsa::mixer::{Mixer, SelemChannelId, SelemId};

    let mixer = Mixer::new("default", false)
        .map_err(|e| DuckingError::Platform(format!("failed to open ALSA mixer: {}", e)))?;

    // Try common control names in order
    let control_names = ["Master", "PCM", "Speaker", "Headphone"];

    for name in control_names {
        let id = SelemId::new(name, 0);
        if let Some(elem) = mixer.find_selem(&id) {
            if elem.has_playback_volume() {
                let (min, max) = elem.get_playback_volume_range();
                if let Ok(vol) = elem.get_playback_volume(SelemChannelId::FrontLeft) {
                    let range = (max - min) as f32;
                    if range > 0.0 {
                        return Ok((vol - min) as f32 / range);
                    }
                }
            }
        }
    }

    Err(DuckingError::Platform(
        "no accessible ALSA volume control found".to_string(),
    ))
}

/// Sets the ALSA master volume from a scalar (0.0-1.0).
fn set_alsa_master_volume(volume: f32) -> Result<(), DuckingError> {
    use alsa::mixer::{Mixer, SelemChannelId, SelemId};

    let mixer = Mixer::new("default", false)
        .map_err(|e| DuckingError::Platform(format!("failed to open ALSA mixer: {}", e)))?;

    let volume = volume.clamp(0.0, 1.0);

    // Try common control names in order
    let control_names = ["Master", "PCM", "Speaker", "Headphone"];

    for name in control_names {
        let id = SelemId::new(name, 0);
        if let Some(elem) = mixer.find_selem(&id) {
            if elem.has_playback_volume() {
                let (min, max) = elem.get_playback_volume_range();
                let range = max - min;
                let target = min + (volume * range as f32) as i64;

                // Set all channels
                for channel in [
                    SelemChannelId::FrontLeft,
                    SelemChannelId::FrontRight,
                    SelemChannelId::FrontCenter,
                ] {
                    let _ = elem.set_playback_volume(channel, target);
                }

                return Ok(());
            }
        }
    }

    Err(DuckingError::Platform(
        "no accessible ALSA volume control found".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linux_backend_name() {
        let backend = LinuxBackend::new();
        assert_eq!(backend.name(), "linux-pulse");
    }

    #[test]
    fn linux_backend_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<LinuxBackend>();
    }

    #[test]
    fn alsa_backend_name() {
        let backend = AlsaBackend::new();
        assert_eq!(backend.name(), "linux-alsa");
    }

    #[test]
    fn alsa_backend_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<AlsaBackend>();
    }

    #[test]
    fn scalar_to_percent_conversion() {
        assert!((LinuxBackend::scalar_to_percent(0.5) - 50.0).abs() < f64::EPSILON);
        assert!((LinuxBackend::scalar_to_percent(1.0) - 100.0).abs() < f64::EPSILON);
        assert!((LinuxBackend::scalar_to_percent(0.0) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn percent_to_scalar_conversion() {
        assert!((LinuxBackend::percent_to_scalar(50.0) - 0.5).abs() < f32::EPSILON);
        assert!((LinuxBackend::percent_to_scalar(100.0) - 1.0).abs() < f32::EPSILON);
        assert!((LinuxBackend::percent_to_scalar(0.0) - 0.0).abs() < f32::EPSILON);
    }

    // Integration tests that actually manipulate volume are marked #[ignore]
    // Run with: cargo test -p playa --features audio-ducking-linux -- --ignored

    #[tokio::test]
    #[ignore = "requires PulseAudio daemon"]
    async fn linux_backend_snapshot_captures_apps() {
        let backend = LinuxBackend::new();
        if !backend.is_available() {
            eprintln!("Skipping: PulseAudio not available");
            return;
        }

        let snapshot = backend.snapshot().await.unwrap();
        println!("Found {} applications", snapshot.len());
        for entry in &snapshot.entries {
            if let SessionId::PulseSinkInput { index, name } = &entry.id {
                println!(
                    "  [{}] {} - volume: {:.0}%",
                    index,
                    name,
                    entry.channels[0] * 100.0
                );
            }
        }
    }

    #[tokio::test]
    #[ignore = "requires PulseAudio daemon - audibly changes volume"]
    async fn linux_backend_fade_cycle() {
        let backend = LinuxBackend::new();
        if !backend.is_available() {
            eprintln!("Skipping: PulseAudio not available");
            return;
        }

        let config = DuckConfig::new(500, 0.5).unwrap();
        let snapshot = backend.snapshot().await.unwrap();

        if snapshot.is_empty() {
            eprintln!("Skipping: no applications playing audio");
            return;
        }

        println!("Fading down...");
        backend.fade_to_floor(&snapshot, &config).await.unwrap();

        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        println!("Fading back up...");
        backend.fade_restore(&snapshot, &config).await.unwrap();
    }

    #[tokio::test]
    #[ignore = "requires ALSA"]
    async fn alsa_backend_snapshot_captures_master() {
        let backend = AlsaBackend::new();
        if !backend.is_available() {
            eprintln!("Skipping: ALSA not available");
            return;
        }

        let snapshot = backend.snapshot().await.unwrap();
        assert_eq!(snapshot.len(), 1);
        println!(
            "Master volume: {:.0}%",
            snapshot.entries[0].channels[0] * 100.0
        );
    }
}
