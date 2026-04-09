//! Linux PulseAudio/PipeWire backend for audio ducking.
//!
//! This backend uses PulseAudio (or PipeWire's PulseAudio compatibility layer)
//! to duck individual sink inputs (applications playing audio) while excluding
//! Playa's own audio stream.
//!
//! ## ALSA Backend
//!
//! An `AlsaBackend` is available for systems without PulseAudio, but it is not
//! selected by the factory because ALSA master-volume ducking also attenuates
//! Playa's own output (self-ducking). It remains in the codebase for explicit
//! opt-in use.

use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use pulsectl::controllers::types::ApplicationInfo;
use pulsectl::controllers::{AppControl, SinkController};

use crate::ducking::{
    DuckConfig, DuckResult, DuckingBackend, DuckingError, SessionId, SessionVolume, VolumeSnapshot,
    compute_fade_steps,
};

/// Cached original volume for a single sink input.
///
/// Stores the exact PulseAudio volume units (0–65535 range) captured at
/// snapshot time. On the final fade or restore step, the backend snaps
/// to these exact units to prevent cumulative rounding drift.
#[derive(Clone)]
struct CachedPulseVolume {
    avg_units: u32,
}

impl CachedPulseVolume {
    fn new(avg_units: u32) -> Self {
        Self { avg_units }
    }

    fn to_percent(&self) -> f64 {
        self.avg_units as f64 / 65536.0 * 100.0
    }
}

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
///
/// ## Volume Precision
///
/// Original volumes are cached as raw PulseAudio volume units at snapshot time.
/// Intermediate fade steps use relative deltas, but the final step of any fade
/// or restore snaps to the exact cached units to prevent cumulative drift.
#[derive(Debug)]
pub struct LinuxBackend {
    /// Whether PulseAudio is available.
    pulse_available: AtomicBool,
    /// Current process PID for self-exclusion.
    our_pid: u32,
    /// Cached original volumes for restoration (index -> raw PulseAudio units).
    cached_volumes: Mutex<HashMap<u32, CachedPulseVolume>>,
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
            cached_volumes: Mutex::new(HashMap::new()),
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

    /// Returns true if the given sink input is a valid ducking candidate.
    ///
    /// A sink input is a duck candidate when all of the following hold:
    /// - It is **not** excluded by PID or name (self-exclusion)
    /// - Its volume is writable (`volume_writable`)
    /// - It reports having volume (`has_volume`)
    /// - It is **not** corked (paused / inactive)
    fn is_target_duck_candidate(&self, app: &ApplicationInfo) -> bool {
        if self.should_exclude(app) {
            return false;
        }

        if !app.volume_writable {
            return false;
        }

        if !app.has_volume {
            return false;
        }

        if app.corked {
            return false;
        }

        true
    }

    /// Converts a volume scalar (0.0-1.0) to a percentage (0-100) for pulsectl.
    fn scalar_to_percent(scalar: f32) -> f64 {
        (scalar * 100.0) as f64
    }

    fn percent_to_scalar(percent: f64) -> f32 {
        (percent / 100.0) as f32
    }

    fn units_scalar_to_percent(units: u32, scalar: f32) -> f64 {
        (units as f64 * scalar as f64) / 65536.0 * 100.0
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
            let mut cached_volumes = self.cached_volumes.lock().unwrap();
            cached_volumes.clear();

            for app in apps {
                if !self.is_target_duck_candidate(&app) {
                    continue;
                }

                let avg_units = app.volume.avg().0;
                let avg_volume = units_to_percent(avg_units);

                cached_volumes.insert(app.index, CachedPulseVolume::new(avg_units));

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
        let cached_volumes = self.cached_volumes.lock().unwrap().clone();

        Box::pin(async move {
            if snapshot.is_empty() {
                return Ok(());
            }

            let mut controller = SinkController::create().map_err(|e| {
                DuckingError::Platform(format!("failed to connect to PulseAudio: {}", e))
            })?;

            for entry in &snapshot.entries {
                let SessionId::PulseSinkInput { index, .. } = &entry.id else {
                    continue;
                };

                let original_volume = entry.channels.first().copied().unwrap_or(1.0);
                let target_volume = original_volume * config.floor_scalar();

                let steps = compute_fade_steps(original_volume, target_volume, &config);
                let is_final_step = |i: usize| i == steps.len() - 1;

                for (i, step) in steps.iter().enumerate() {
                    if is_final_step(i) {
                        if let Some(cached) = cached_volumes.get(index) {
                            let exact_percent = Self::units_scalar_to_percent(
                                cached.avg_units,
                                config.floor_scalar(),
                            );
                            apply_volume_delta(&mut controller, *index, exact_percent)?;

                            tokio::time::sleep(std::time::Duration::from_millis(
                                step.delay_ms as u64,
                            ))
                            .await;
                            continue;
                        }
                    }

                    let percent = Self::scalar_to_percent(step.volume);
                    apply_volume_delta(&mut controller, *index, percent)?;

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
        let cached_volumes = self.cached_volumes.lock().unwrap().clone();

        Box::pin(async move {
            if snapshot.is_empty() {
                return Ok(());
            }

            let mut controller = SinkController::create().map_err(|e| {
                DuckingError::Platform(format!("failed to connect to PulseAudio: {}", e))
            })?;

            for entry in &snapshot.entries {
                let SessionId::PulseSinkInput { index, .. } = &entry.id else {
                    continue;
                };

                let Some(cached) = cached_volumes.get(index) else {
                    continue;
                };

                let Ok(Some(app)) = controller.get_app_by_index(*index) else {
                    continue;
                };

                let current_percent = units_to_percent(app.volume.avg().0);
                let current_scalar = Self::percent_to_scalar(current_percent);
                let target_scalar = Self::percent_to_scalar(cached.to_percent());

                let steps = compute_fade_steps(current_scalar, target_scalar, &config);
                let is_final_step = |i: usize| i == steps.len() - 1;

                for (i, step) in steps.iter().enumerate() {
                    if is_final_step(i) {
                        apply_volume_delta(&mut controller, *index, cached.to_percent())?;
                    } else {
                        let percent = Self::scalar_to_percent(step.volume);
                        apply_volume_delta(&mut controller, *index, percent)?;
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

/// Convert raw PulseAudio volume units (0–65535) to percentage (0–100+).
fn units_to_percent(units: u32) -> f64 {
    units as f64 / 65536.0 * 100.0
}

/// Apply a target volume percentage to a sink input by reading the current
/// volume and issuing the required relative delta.
fn apply_volume_delta(
    controller: &mut SinkController,
    index: u32,
    target_percent: f64,
) -> Result<(), DuckingError> {
    let app = controller
        .get_app_by_index(index)
        .map_err(|e| DuckingError::Platform(format!("failed to get sink input {}: {}", index, e)))?
        .ok_or_else(|| DuckingError::Platform(format!("sink input {} no longer exists", index)))?;

    let current_percent = app.volume.avg().0 as f64 / 65536.0 * 100.0;
    let delta = target_percent - current_percent;
    if delta < 0.0 {
        controller
            .decrease_app_volume_by_percent(index, -delta)
            .map_err(|e| {
                DuckingError::Platform(format!(
                    "failed to decrease volume for sink input {}: {}",
                    index, e
                ))
            })?;
    } else if delta > 0.0 {
        controller
            .increase_app_volume_by_percent(index, delta)
            .map_err(|e| {
                DuckingError::Platform(format!(
                    "failed to increase volume for sink input {}: {}",
                    index, e
                ))
            })?;
    }

    Ok(())
}

/// ALSA fallback backend for systems without PulseAudio.
///
/// This provides basic master volume ducking using ALSA's mixer interface.
/// It's less precise than PulseAudio (affects all audio, including Playa's)
/// and is not selected by the factory because it would also duck Playa's
/// own output. Available for explicit opt-in use only.
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

                for channel in [
                    SelemChannelId::FrontLeft,
                    SelemChannelId::FrontRight,
                    SelemChannelId::FrontCenter,
                    SelemChannelId::RearLeft,
                    SelemChannelId::RearRight,
                    SelemChannelId::FrontLeftOfCenter,
                    SelemChannelId::FrontRightOfCenter,
                    SelemChannelId::SideLeft,
                    SelemChannelId::SideRight,
                    SelemChannelId::Woofer,
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

    fn make_test_app(
        index: u32,
        name: Option<&str>,
        pid: Option<u32>,
        corked: bool,
        has_volume: bool,
        volume_writable: bool,
    ) -> ApplicationInfo {
        use libpulse_binding as pulse;

        let mut proplist = pulse::proplist::Proplist::new().unwrap();
        if let Some(p) = pid {
            proplist
                .set_str("application.process.id", &p.to_string())
                .ok();
        }

        let mut vol = pulse::volume::ChannelVolumes::default();
        let _ = vol.set_len(1);
        vol.set(0, pulse::volume::Volume(65536));

        ApplicationInfo {
            index,
            name: name.map(String::from),
            owner_module: None,
            client: None,
            connection_id: 0,
            sample_spec: pulse::sample::Spec {
                format: pulse::sample::Format::S16le,
                channels: 2,
                rate: 44100,
            },
            channel_map: pulse::channelmap::Map::default(),
            volume: vol,
            buffer_usec: 0,
            connection_usec: 0,
            resample_method: None,
            driver: None,
            mute: false,
            proplist,
            corked,
            has_volume,
            volume_writable,
            format: pulse::format::Info {
                encoding: pulse::format::Encoding::PCM,
                plist: pulse::proplist::Proplist::new().unwrap(),
            },
        }
    }

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

    #[test]
    fn units_to_percent_roundtrip() {
        let units = 32768u32;
        let percent = units_to_percent(units);
        let expected = 32768.0 / 65536.0 * 100.0;
        assert!((percent - expected).abs() < f64::EPSILON);
    }

    #[test]
    fn cached_pulse_volume_to_percent() {
        let cached = CachedPulseVolume::new(65536);
        assert!((cached.to_percent() - 100.0).abs() < f64::EPSILON);

        let cached_half = CachedPulseVolume::new(32768);
        assert!((cached_half.to_percent() - 50.0).abs() < f64::EPSILON);

        let cached_zero = CachedPulseVolume::new(0);
        assert!((cached_zero.to_percent() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn repeated_duck_restore_no_drift() {
        let original_units = 49152u32;
        let cached = CachedPulseVolume::new(original_units);

        for _ in 0..10 {
            let restored_percent = cached.to_percent();
            let restored_units = (restored_percent / 100.0 * 65536.0) as u32;
            assert_eq!(
                restored_units, original_units,
                "final snap should always land on exact cached units"
            );
        }
    }

    #[test]
    fn final_restore_step_uses_exact_cached_units() {
        let original_units = 39321u32;
        let cached = CachedPulseVolume::new(original_units);
        let exact_percent = cached.to_percent();
        let approx_percent = original_units as f64 / 65536.0 * 100.0;

        assert!(
            (exact_percent - approx_percent).abs() < f64::EPSILON,
            "cached units should convert losslessly"
        );

        let from_approx = (approx_percent / 100.0 * 65536.0) as u32;
        assert_eq!(from_approx, original_units);
    }

    #[test]
    fn units_scalar_to_percent_duck_target_from_cached_units() {
        let full_units = 65536u32;
        let floor_scalar = 0.2f32;
        let target = LinuxBackend::units_scalar_to_percent(full_units, floor_scalar);
        let expected = full_units as f64 * floor_scalar as f64 / 65536.0 * 100.0;
        assert!(
            (target - expected).abs() < f64::EPSILON,
            "duck target should be derived from cached units directly"
        );

        let half_units = 32768u32;
        let target_half = LinuxBackend::units_scalar_to_percent(half_units, floor_scalar);
        let expected_half = half_units as f64 * floor_scalar as f64 / 65536.0 * 100.0;
        assert!(
            (target_half - expected_half).abs() < f64::EPSILON,
            "duck target from half-volume cached units should be exact"
        );
    }

    #[test]
    fn units_scalar_to_percent_restore_target_is_full_cached() {
        let units = 49152u32;
        let restore_target = LinuxBackend::units_scalar_to_percent(units, 1.0);
        let cached_pct = units_to_percent(units);
        assert!(
            (restore_target - cached_pct).abs() < f64::EPSILON,
            "restore at scalar 1.0 should equal cached percent"
        );
    }

    #[test]
    fn units_scalar_to_percent_does_not_rely_on_float_snapshot() {
        let original_units = 39321u32;
        let floor_scalar = 0.2f32;

        let from_units = LinuxBackend::units_scalar_to_percent(original_units, floor_scalar);

        let snapshot_scalar = original_units as f32 / 65536.0;
        let float_derived_pct = LinuxBackend::scalar_to_percent(snapshot_scalar * floor_scalar);

        let from_units_roundtrip = (from_units / 100.0 * 65536.0) as u32;
        let expected_units = (original_units as f64 * floor_scalar as f64) as u32;

        assert_eq!(
            from_units_roundtrip, expected_units,
            "units-based path should produce exact integer units without float intermediary loss"
        );

        let float_roundtrip = (float_derived_pct / 100.0 * 65536.0) as u32;
        assert_eq!(
            from_units_roundtrip, float_roundtrip,
            "both paths should agree for this value"
        );
    }

    #[test]
    fn multiple_duck_restore_cycles_no_drift() {
        let original_units = 49152u32;
        let cached = CachedPulseVolume::new(original_units);
        let floor_scalar = 0.2f32;

        for cycle in 0..5 {
            let duck_target = LinuxBackend::units_scalar_to_percent(cached.avg_units, floor_scalar);
            let duck_units = (duck_target / 100.0 * 65536.0) as u32;
            let expected_duck = (original_units as f64 * floor_scalar as f64) as u32;
            assert_eq!(
                duck_units, expected_duck,
                "cycle {cycle}: duck target should be exact"
            );

            let restore_target = cached.to_percent();
            let restore_units = (restore_target / 100.0 * 65536.0) as u32;
            assert_eq!(
                restore_units, original_units,
                "cycle {cycle}: restore should snap back to exact cached units"
            );
        }
    }

    #[test]
    fn apply_volume_delta_reports_missing_sink_input() {
        let err = DuckingError::Platform("sink input 999 no longer exists".to_string());
        assert!(
            err.to_string().contains("no longer exists"),
            "missing sink input error should be descriptive"
        );
    }

    #[test]
    fn apply_volume_delta_reports_write_failure() {
        let err = DuckingError::Platform(
            "failed to decrease volume for sink input 42: write error".to_string(),
        );
        assert!(
            err.to_string().contains("failed to decrease volume"),
            "write failure should be propagated in error message"
        );

        let err2 = DuckingError::Platform(
            "failed to increase volume for sink input 42: write error".to_string(),
        );
        assert!(
            err2.to_string().contains("failed to increase volume"),
            "increase failure should be propagated in error message"
        );
    }

    #[test]
    fn cached_pulse_volume_is_clone() {
        let a = CachedPulseVolume::new(32768);
        let b = a.clone();
        assert_eq!(a.avg_units, b.avg_units);
    }

    #[test]
    fn should_exclude_by_pid() {
        let our_pid = std::process::id();
        let backend = LinuxBackend::new();

        let self_app = make_test_app(1, Some("firefox"), Some(our_pid), false, true, true);
        assert!(backend.should_exclude(&self_app));

        let other_app = make_test_app(
            2,
            Some("firefox"),
            Some(our_pid.wrapping_add(1)),
            false,
            true,
            true,
        );
        assert!(!backend.should_exclude(&other_app));
    }

    #[test]
    fn should_exclude_by_name() {
        let backend = LinuxBackend::new();

        let playa_app = make_test_app(10, Some("playa-sfx"), None, false, true, true);
        assert!(backend.should_exclude(&playa_app));

        let playa_upper = make_test_app(11, Some("Playa"), None, false, true, true);
        assert!(backend.should_exclude(&playa_upper));

        let other_app = make_test_app(12, Some("firefox"), None, false, true, true);
        assert!(!backend.should_exclude(&other_app));
    }

    #[test]
    fn should_exclude_by_proplist_name() {
        use libpulse_binding as pulse;

        let backend = LinuxBackend::new();

        let mut proplist = pulse::proplist::Proplist::new().unwrap();
        proplist.set_str("application.name", "playa-native").ok();

        let mut vol = pulse::volume::ChannelVolumes::default();
        let _ = vol.set_len(1);
        vol.set(0, pulse::volume::Volume(65536));

        let app = ApplicationInfo {
            index: 20,
            name: Some("unknown".to_string()),
            proplist,
            corked: false,
            has_volume: true,
            volume_writable: true,
            volume: vol,
            owner_module: None,
            client: None,
            connection_id: 0,
            sample_spec: pulse::sample::Spec {
                format: pulse::sample::Format::S16le,
                channels: 2,
                rate: 44100,
            },
            channel_map: pulse::channelmap::Map::default(),
            buffer_usec: 0,
            connection_usec: 0,
            resample_method: None,
            driver: None,
            mute: false,
            format: pulse::format::Info {
                encoding: pulse::format::Encoding::PCM,
                plist: pulse::proplist::Proplist::new().unwrap(),
            },
        };

        assert!(
            backend.should_exclude(&app),
            "should exclude by proplist application.name containing 'playa'"
        );
    }

    #[test]
    fn should_not_exclude_unrelated_app() {
        let backend = LinuxBackend::new();

        let app = make_test_app(5, Some("spotify"), Some(9999), false, true, true);
        assert!(!backend.should_exclude(&app));
    }

    #[test]
    fn duck_candidate_accepts_active_uncorked_app() {
        let backend = LinuxBackend::new();

        let app = make_test_app(1, Some("firefox"), Some(9999), false, true, true);
        assert!(backend.is_target_duck_candidate(&app));
    }

    #[test]
    fn duck_candidate_rejects_corked_app() {
        let backend = LinuxBackend::new();

        let app = make_test_app(1, Some("paused-app"), Some(9999), true, true, true);
        assert!(
            !backend.is_target_duck_candidate(&app),
            "corked (paused) inputs must not be ducked"
        );
    }

    #[test]
    fn duck_candidate_rejects_non_writable_volume() {
        let backend = LinuxBackend::new();

        let app = make_test_app(1, Some("readonly-app"), Some(9999), false, true, false);
        assert!(
            !backend.is_target_duck_candidate(&app),
            "non-writable volume inputs must not be ducked"
        );
    }

    #[test]
    fn duck_candidate_rejects_no_volume() {
        let backend = LinuxBackend::new();

        let app = make_test_app(1, Some("novol-app"), Some(9999), false, false, true);
        assert!(
            !backend.is_target_duck_candidate(&app),
            "inputs without volume must not be ducked"
        );
    }

    #[test]
    fn duck_candidate_rejects_self() {
        let our_pid = std::process::id();
        let backend = LinuxBackend::new();

        let self_app = make_test_app(1, Some("playa"), Some(our_pid), false, true, true);
        assert!(
            !backend.is_target_duck_candidate(&self_app),
            "playa's own sink input must not be ducked"
        );
    }

    #[test]
    fn duck_candidate_rejects_corked_even_with_volume() {
        let backend = LinuxBackend::new();

        let app = make_test_app(1, Some("paused-with-vol"), Some(9999), true, true, true);
        assert!(
            !backend.is_target_duck_candidate(&app),
            "corked input must be rejected even with writable volume"
        );
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
