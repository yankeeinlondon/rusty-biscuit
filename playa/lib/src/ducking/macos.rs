//! macOS CoreAudio backend for audio ducking.
//!
//! This backend uses the virtual master volume on the default output device
//! to duck system audio. It affects all audio including Playa's own output,
//! which is acceptable for simplicity.

use std::mem;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use coreaudio_sys::{
    kAudioDevicePropertyVolumeScalar, kAudioHardwarePropertyDefaultOutputDevice,
    kAudioObjectPropertyElementMain, kAudioObjectPropertyScopeGlobal,
    kAudioObjectPropertyScopeOutput, kAudioObjectSystemObject, AudioObjectGetPropertyData,
    AudioObjectID, AudioObjectPropertyAddress, AudioObjectSetPropertyData,
};

use crate::ducking::{
    compute_fade_steps, DuckConfig, DuckResult, DuckingBackend, DuckingError, SessionId,
    SessionVolume, VolumeSnapshot,
};

/// macOS CoreAudio backend for audio ducking.
///
/// Uses the device volume control on the default output device to attenuate
/// all system audio during playback.
///
/// ## Limitations
///
/// Some output devices (HDMI, USB audio interfaces, external DACs) don't support
/// software volume control. For these devices, `is_available()` returns false
/// and ducking will be skipped gracefully.
#[derive(Debug)]
pub struct MacOsBackend {
    /// Cached default output device ID.
    device_id: AtomicU32,
    /// Whether volume control is available on this device.
    volume_available: AtomicBool,
}

impl MacOsBackend {
    /// Creates a new macOS backend.
    ///
    /// Attempts to cache the default output device ID and check volume availability.
    pub fn new() -> Self {
        let device_id = get_default_output_device().unwrap_or(0);
        let volume_available = if device_id != 0 {
            get_device_volume(device_id).is_ok()
        } else {
            false
        };

        Self {
            device_id: AtomicU32::new(device_id),
            volume_available: AtomicBool::new(volume_available),
        }
    }

    /// Refreshes the cached device ID and volume availability.
    fn refresh_device(&self) -> Result<AudioObjectID, DuckingError> {
        let device_id = get_default_output_device()?;
        self.device_id.store(device_id, Ordering::SeqCst);

        let available = get_device_volume(device_id).is_ok();
        self.volume_available.store(available, Ordering::SeqCst);

        Ok(device_id)
    }

    /// Gets the current device ID, refreshing if needed.
    fn get_device(&self) -> Result<AudioObjectID, DuckingError> {
        let cached = self.device_id.load(Ordering::SeqCst);
        if cached == 0 {
            self.refresh_device()
        } else {
            Ok(cached)
        }
    }
}

impl Default for MacOsBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl DuckingBackend for MacOsBackend {
    fn snapshot(&self) -> DuckResult<'_, VolumeSnapshot> {
        Box::pin(async move {
            if !self.volume_available.load(Ordering::SeqCst) {
                return Err(DuckingError::BackendUnavailable(
                    "output device does not support software volume control".to_string(),
                ));
            }

            let device_id = self.get_device()?;
            let volume = get_device_volume(device_id)?;

            let mut snapshot = VolumeSnapshot::new();
            snapshot.push(SessionVolume::new(
                SessionId::MacEndpoint,
                vec![volume],
                false,
            ));

            Ok(snapshot)
        })
    }

    fn fade_to_floor(
        &self,
        snapshot: &VolumeSnapshot,
        config: &DuckConfig,
    ) -> DuckResult<'_, ()> {
        let snapshot = snapshot.clone();
        let config = *config;

        Box::pin(async move {
            let device_id = self.get_device()?;

            // Get the original volume from snapshot
            let original_volume = snapshot
                .entries
                .first()
                .and_then(|e| e.channels.first().copied())
                .unwrap_or(1.0);

            // Compute fade steps
            let target = original_volume * config.floor_scalar();
            let steps = compute_fade_steps(original_volume, target, &config);

            // Apply each step with delay
            for step in steps {
                set_device_volume(device_id, step.volume)?;
                tokio::time::sleep(std::time::Duration::from_millis(step.delay_ms as u64)).await;
            }

            Ok(())
        })
    }

    fn fade_restore(
        &self,
        snapshot: &VolumeSnapshot,
        config: &DuckConfig,
    ) -> DuckResult<'_, ()> {
        let snapshot = snapshot.clone();
        let config = *config;

        Box::pin(async move {
            let device_id = self.get_device()?;

            // Get the original volume from snapshot
            let original_volume = snapshot
                .entries
                .first()
                .and_then(|e| e.channels.first().copied())
                .unwrap_or(1.0);

            // Get current (ducked) volume
            let current_volume = get_device_volume(device_id)?;

            // Compute fade steps back to original
            let steps = compute_fade_steps(current_volume, original_volume, &config);

            // Apply each step with delay
            for step in steps {
                set_device_volume(device_id, step.volume)?;
                tokio::time::sleep(std::time::Duration::from_millis(step.delay_ms as u64)).await;
            }

            Ok(())
        })
    }

    fn name(&self) -> &'static str {
        "macos-coreaudio"
    }

    fn is_available(&self) -> bool {
        // Check if we have a device and volume control is available
        let device_ok = self.device_id.load(Ordering::SeqCst) != 0 || self.refresh_device().is_ok();
        device_ok && self.volume_available.load(Ordering::SeqCst)
    }
}

/// Gets the default output device ID.
fn get_default_output_device() -> Result<AudioObjectID, DuckingError> {
    unsafe {
        let mut device_id: AudioObjectID = 0;
        let mut data_size = mem::size_of::<AudioObjectID>() as u32;

        let address = AudioObjectPropertyAddress {
            mSelector: kAudioHardwarePropertyDefaultOutputDevice,
            mScope: kAudioObjectPropertyScopeGlobal,
            mElement: kAudioObjectPropertyElementMain,
        };

        let status = AudioObjectGetPropertyData(
            kAudioObjectSystemObject,
            &address,
            0,
            std::ptr::null(),
            &mut data_size,
            &mut device_id as *mut _ as *mut _,
        );

        if status != 0 {
            return Err(DuckingError::Platform(format!(
                "failed to get default output device: OSStatus {}",
                status
            )));
        }

        if device_id == 0 {
            return Err(DuckingError::BackendUnavailable(
                "no default output device".to_string(),
            ));
        }

        Ok(device_id)
    }
}

/// Gets the master volume for a device (0.0 to 1.0).
///
/// Tries channel 0 (master) first, then channel 1 if that fails.
fn get_device_volume(device_id: AudioObjectID) -> Result<f32, DuckingError> {
    // Try master channel (0) first, then channel 1
    for channel in [0u32, 1u32] {
        match get_channel_volume(device_id, channel) {
            Ok(vol) => return Ok(vol),
            Err(_) => continue,
        }
    }

    Err(DuckingError::Platform(
        "no accessible volume channel found".to_string(),
    ))
}

/// Gets the volume for a specific channel.
fn get_channel_volume(device_id: AudioObjectID, channel: u32) -> Result<f32, DuckingError> {
    unsafe {
        let mut volume: f32 = 0.0;
        let mut data_size = mem::size_of::<f32>() as u32;

        let address = AudioObjectPropertyAddress {
            mSelector: kAudioDevicePropertyVolumeScalar,
            mScope: kAudioObjectPropertyScopeOutput,
            mElement: channel,
        };

        let status = AudioObjectGetPropertyData(
            device_id,
            &address,
            0,
            std::ptr::null(),
            &mut data_size,
            &mut volume as *mut _ as *mut _,
        );

        if status != 0 {
            return Err(DuckingError::Platform(format!(
                "failed to get volume for channel {}: OSStatus {}",
                channel, status
            )));
        }

        Ok(volume)
    }
}

/// Sets the master volume for a device (0.0 to 1.0).
///
/// Sets both channel 0 and channel 1 for stereo devices.
fn set_device_volume(device_id: AudioObjectID, volume: f32) -> Result<(), DuckingError> {
    let volume = volume.clamp(0.0, 1.0);

    // Try to set both channels for stereo, ignore errors on individual channels
    let mut any_success = false;
    for channel in [0u32, 1u32] {
        if set_channel_volume(device_id, channel, volume).is_ok() {
            any_success = true;
        }
    }

    if any_success {
        Ok(())
    } else {
        Err(DuckingError::Platform(
            "failed to set volume on any channel".to_string(),
        ))
    }
}

/// Sets the volume for a specific channel.
fn set_channel_volume(device_id: AudioObjectID, channel: u32, volume: f32) -> Result<(), DuckingError> {
    unsafe {
        let data_size = mem::size_of::<f32>() as u32;

        let address = AudioObjectPropertyAddress {
            mSelector: kAudioDevicePropertyVolumeScalar,
            mScope: kAudioObjectPropertyScopeOutput,
            mElement: channel,
        };

        let status = AudioObjectSetPropertyData(
            device_id,
            &address,
            0,
            std::ptr::null(),
            data_size,
            &volume as *const _ as *const _,
        );

        if status != 0 {
            return Err(DuckingError::Platform(format!(
                "failed to set volume for channel {}: OSStatus {}",
                channel, status
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn macos_backend_name() {
        let backend = MacOsBackend::new();
        assert_eq!(backend.name(), "macos-coreaudio");
    }

    #[test]
    fn macos_backend_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<MacOsBackend>();
    }

    // Integration tests that actually manipulate volume are marked #[ignore]
    // Run with: cargo test -p playa --features audio-ducking-macos -- --ignored

    #[tokio::test]
    #[ignore = "requires real audio device"]
    async fn macos_backend_snapshot_captures_volume() {
        let backend = MacOsBackend::new();
        if !backend.is_available() {
            eprintln!("Skipping: no audio device available");
            return;
        }

        let snapshot = backend.snapshot().await.unwrap();
        assert_eq!(snapshot.len(), 1);

        let entry = &snapshot.entries[0];
        assert!(matches!(entry.id, SessionId::MacEndpoint));
        assert!(!entry.channels.is_empty());
        assert!(entry.channels[0] >= 0.0 && entry.channels[0] <= 1.0);
    }

    #[tokio::test]
    #[ignore = "requires real audio device - audibly changes volume"]
    async fn macos_backend_fade_cycle() {
        let backend = MacOsBackend::new();
        if !backend.is_available() {
            eprintln!("Skipping: no audio device available");
            return;
        }

        // Use a short ramp for testing
        let config = DuckConfig::new(200, 0.5).unwrap();

        let snapshot = backend.snapshot().await.unwrap();
        let original = snapshot.entries[0].channels[0];

        // Fade down
        backend.fade_to_floor(&snapshot, &config).await.unwrap();

        // Verify volume changed
        let current = get_device_volume(backend.device_id.load(Ordering::SeqCst)).unwrap();
        assert!(current < original, "volume should have decreased");

        // Fade back up
        backend.fade_restore(&snapshot, &config).await.unwrap();

        // Verify restored
        let restored = get_device_volume(backend.device_id.load(Ordering::SeqCst)).unwrap();
        assert!(
            (restored - original).abs() < 0.05,
            "volume should be restored"
        );
    }
}
