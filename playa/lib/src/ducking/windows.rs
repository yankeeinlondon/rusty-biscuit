//! Windows WASAPI backend for audio ducking.
//!
//! This backend ducks individual WASAPI audio sessions on the default
//! multimedia render endpoint, excluding Playa's own process. Sessions are
//! identified by `GetSessionInstanceIdentifier()` so that restore matches
//! the exact session that was snapshotted.
//!
//! The backend is intentionally stateless: no COM interface pointers are
//! stored. Each operation creates a fresh `ComGuard`, resolves the endpoint,
//! and enumerates sessions from scratch. This avoids Send/Sync issues with
//! COM pointers and keeps the lifetime simple.

use std::sync::atomic::{AtomicBool, Ordering};

use windows::Win32::{
    Media::Audio::{
        AudioSessionStateActive, IAudioSessionControl2, IAudioSessionEnumerator,
        IAudioSessionManager2, IMMDeviceEnumerator, ISimpleAudioVolume, MMDeviceEnumerator,
        eMultimedia, eRender,
    },
    System::Com::{CLSCTX_ALL, CoCreateInstance},
};

use crate::ducking::{
    DuckConfig, DuckResult, DuckingBackend, DuckingError, SessionId, SessionVolume, VolumeSnapshot,
    compute_fade_steps,
};
use crate::windows_com::ComGuard;

/// A live session discovered during enumeration.
struct LiveSession {
    pid: u32,
    key: String,
    volume: f32,
    mute: bool,
}

/// Windows WASAPI per-session ducking backend.
///
/// Enumerates active audio sessions on the default render endpoint and
/// adjusts their `ISimpleAudioVolume` levels. Only sessions that are
/// `AudioSessionStateActive` and not owned by the current process are
/// included.
#[derive(Debug)]
pub struct WindowsBackend {
    our_pid: u32,
    available: AtomicBool,
}

impl WindowsBackend {
    /// Creates a new Windows backend.
    ///
    /// Probes whether the default multimedia render endpoint is accessible
    /// and `IAudioSessionManager2` can be activated on it.
    pub fn new() -> Self {
        let our_pid = std::process::id();
        let available = probe_wasapi_available();

        Self {
            our_pid,
            available: AtomicBool::new(available),
        }
    }
}

impl Default for WindowsBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl DuckingBackend for WindowsBackend {
    fn snapshot(&self) -> DuckResult<'_, VolumeSnapshot> {
        Box::pin(async move {
            if !self.available.load(Ordering::SeqCst) {
                return Err(DuckingError::BackendUnavailable(
                    "WASAPI session manager not available".to_string(),
                ));
            }

            let sessions = enumerate_sessions(self.our_pid)?;

            let mut snapshot = VolumeSnapshot::new();
            for s in sessions {
                snapshot.push(SessionVolume::new(
                    SessionId::WasapiSession {
                        pid: s.pid,
                        key: s.key,
                    },
                    vec![s.volume],
                    s.mute,
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

            let _com = ComGuard::new()
                .map_err(|e| DuckingError::Platform(format!("COM init failed: {e}")))?;

            let session_mgr = get_session_manager()?;

            for entry in &snapshot.entries {
                let SessionId::WasapiSession { key, .. } = &entry.id else {
                    continue;
                };

                let original_volume = entry.channels.first().copied().unwrap_or(1.0);
                let target_volume = (original_volume * config.floor_scalar()).clamp(0.0, 1.0);
                let steps = compute_fade_steps(original_volume, target_volume, &config);

                // Find the live session by key
                if let Some(simple_vol) = find_simple_volume_by_key(&session_mgr, key) {
                    for step in steps {
                        let vol = step.volume.clamp(0.0, 1.0);
                        let _ = unsafe { simple_vol.SetMasterVolume(vol, std::ptr::null()) };
                        tokio::time::sleep(std::time::Duration::from_millis(step.delay_ms as u64))
                            .await;
                    }
                }
            }

            Ok(())
        })
    }

    fn fade_restore(&self, snapshot: &VolumeSnapshot, config: &DuckConfig) -> DuckResult<'_, ()> {
        let snapshot = snapshot.clone();
        let config = *config;

        Box::pin(async move {
            if snapshot.is_empty() {
                return Ok(());
            }

            let _com = ComGuard::new()
                .map_err(|e| DuckingError::Platform(format!("COM init failed: {e}")))?;

            let session_mgr = get_session_manager()?;

            for entry in &snapshot.entries {
                let SessionId::WasapiSession { key, .. } = &entry.id else {
                    continue;
                };

                let original_volume = entry.channels.first().copied().unwrap_or(1.0);

                // Find the live session by key
                if let Some(simple_vol) = find_simple_volume_by_key(&session_mgr, key) {
                    let current_volume = unsafe { simple_vol.GetMasterVolume() }.unwrap_or(0.0);
                    let steps = compute_fade_steps(current_volume, original_volume, &config);

                    for step in &steps {
                        let vol = step.volume.clamp(0.0, 1.0);
                        let _ = unsafe { simple_vol.SetMasterVolume(vol, std::ptr::null()) };
                        tokio::time::sleep(std::time::Duration::from_millis(step.delay_ms as u64))
                            .await;
                    }

                    // Restore mute state after the final volume step
                    let _ = unsafe {
                        simple_vol.SetMute(entry.mute.into(), std::ptr::null())
                    };
                }
            }

            Ok(())
        })
    }

    fn name(&self) -> &'static str {
        "windows-wasapi"
    }

    fn is_available(&self) -> bool {
        self.available.load(Ordering::SeqCst)
    }
}

/// Probes whether the default multimedia render endpoint supports
/// `IAudioSessionManager2`.
fn probe_wasapi_available() -> bool {
    let Ok(_com) = ComGuard::new() else {
        return false;
    };
    get_session_manager().is_ok()
}

/// Activates `IAudioSessionManager2` on the default multimedia render endpoint.
fn get_session_manager() -> Result<IAudioSessionManager2, DuckingError> {
    unsafe {
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
                .map_err(|e| DuckingError::Platform(format!("failed to create enumerator: {e}")))?;

        let device = enumerator
            .GetDefaultAudioEndpoint(eRender, eMultimedia)
            .map_err(|e| {
                DuckingError::Platform(format!("failed to get default audio endpoint: {e}"))
            })?;

        let session_mgr: IAudioSessionManager2 = device.Activate(CLSCTX_ALL, None).map_err(|e| {
            DuckingError::Platform(format!("failed to activate session manager: {e}"))
        })?;

        Ok(session_mgr)
    }
}

/// Enumerates active audio sessions excluding the given PID.
fn enumerate_sessions(our_pid: u32) -> Result<Vec<LiveSession>, DuckingError> {
    let _com =
        ComGuard::new().map_err(|e| DuckingError::Platform(format!("COM init failed: {e}")))?;

    let session_mgr = get_session_manager()?;

    unsafe {
        let enumerator: IAudioSessionEnumerator = session_mgr
            .GetSessionEnumerator()
            .map_err(|e| DuckingError::SnapshotFailed(format!("failed to enumerate sessions: {e}")))?;

        let count = enumerator
            .GetCount()
            .map_err(|e| DuckingError::SnapshotFailed(format!("failed to get session count: {e}")))?;

        let mut sessions = Vec::new();

        for i in 0..count {
            // Skip per-session failures
            let Ok(control) = enumerator.GetSession(i) else {
                continue;
            };

            // Get IAudioSessionControl2 for PID and state
            let Ok(control2): Result<IAudioSessionControl2, _> = control.cast() else {
                continue;
            };

            // Only keep active sessions
            let Ok(state) = control2.GetState() else {
                continue;
            };
            if state != AudioSessionStateActive {
                continue;
            }

            // Exclude our own process
            let Ok(pid) = control2.GetProcessId() else {
                continue;
            };
            if pid == our_pid {
                continue;
            }

            // Get session instance identifier (the unique key)
            let Ok(instance_id_pwstr) = control2.GetSessionInstanceIdentifier() else {
                continue;
            };
            let key = instance_id_pwstr.to_string();
            if key.is_empty() {
                // Free the allocated string even on skip
                windows::Win32::System::Com::CoTaskMemFree(Some(
                    instance_id_pwstr.0 as *const _ as *const _,
                ));
                continue;
            }
            // Free the COM-allocated string
            windows::Win32::System::Com::CoTaskMemFree(Some(
                instance_id_pwstr.0 as *const _ as *const _,
            ));

            // Get ISimpleAudioVolume
            let Ok(simple_vol): Result<ISimpleAudioVolume, _> = control.cast() else {
                continue;
            };

            let Ok(volume) = simple_vol.GetMasterVolume() else {
                continue;
            };

            let mute = simple_vol.GetMute().map(|b| b.as_bool()).unwrap_or(false);

            sessions.push(LiveSession {
                pid,
                key,
                volume,
                mute,
            });
        }

        Ok(sessions)
    }
}

/// Finds the `ISimpleAudioVolume` for a session matching the given instance key.
fn find_simple_volume_by_key(
    session_mgr: &IAudioSessionManager2,
    target_key: &str,
) -> Option<ISimpleAudioVolume> {
    unsafe {
        let enumerator = session_mgr.GetSessionEnumerator().ok()?;
        let count = enumerator.GetCount().ok()?;

        for i in 0..count {
            let Ok(control) = enumerator.GetSession(i) else {
                continue;
            };
            let Ok(control2): Result<IAudioSessionControl2, _> = control.cast() else {
                continue;
            };
            let Ok(instance_id_pwstr) = control2.GetSessionInstanceIdentifier() else {
                continue;
            };

            let key = instance_id_pwstr.to_string();
            windows::Win32::System::Com::CoTaskMemFree(Some(
                instance_id_pwstr.0 as *const _ as *const _,
            ));

            if key == target_key {
                return control.cast().ok();
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_backend_name() {
        let backend = WindowsBackend {
            our_pid: 12345,
            available: AtomicBool::new(false),
        };
        assert_eq!(backend.name(), "windows-wasapi");
    }

    #[test]
    fn windows_backend_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<WindowsBackend>();
    }

    #[test]
    #[ignore = "requires Windows audio device"]
    fn windows_backend_availability_probe() {
        let backend = WindowsBackend::new();
        // On a machine with a default render device, this should succeed
        assert!(
            backend.is_available(),
            "expected WASAPI to be available on this machine"
        );
    }

    #[tokio::test]
    #[ignore = "requires Windows audio device"]
    async fn windows_backend_snapshot_returns_sessions() {
        let backend = WindowsBackend::new();
        if !backend.is_available() {
            eprintln!("Skipping: WASAPI not available");
            return;
        }

        let snapshot = backend.snapshot().await.unwrap();
        println!("Found {} active sessions", snapshot.len());
        for entry in &snapshot.entries {
            if let SessionId::WasapiSession { pid, key } = &entry.id {
                println!(
                    "  PID {} [{}] - volume: {:.0}%, mute: {}",
                    pid,
                    key,
                    entry.channels[0] * 100.0,
                    entry.mute,
                );
            }
        }
    }

    #[test]
    #[ignore = "requires Windows audio device"]
    fn backend_name_is_windows_wasapi_when_feature_enabled() {
        assert_eq!(
            WindowsBackend {
                our_pid: 0,
                available: AtomicBool::new(true),
            }
            .name(),
            "windows-wasapi"
        );
    }
}
