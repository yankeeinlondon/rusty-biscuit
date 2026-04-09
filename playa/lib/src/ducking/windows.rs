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
//!
//! All fade operations run on a blocking thread via `spawn_blocking` so that
//! the COM apartment stays pinned to a single thread for the duration of
//! the fade loop. The snapshot operation runs synchronously (no `.await`
//! points during COM usage) and is safe to call directly from an async
//! context.

use std::collections::HashMap;
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
use crate::windows_com::{ComGuard, pwstr_to_string_and_free};

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
        let our_pid = self.our_pid;
        let available = self.available.load(Ordering::SeqCst);

        Box::pin(async move {
            if !available {
                return Err(DuckingError::BackendUnavailable(
                    "WASAPI session manager not available".to_string(),
                ));
            }

            let sessions = enumerate_sessions(our_pid)?;

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
        let available = self.available.load(Ordering::SeqCst);

        Box::pin(async move {
            if !available || snapshot.is_empty() {
                return Ok(());
            }

            tokio::task::spawn_blocking(move || fade_to_floor_blocking(&snapshot, &config))
                .await
                .map_err(|e| DuckingError::FadeFailed(format!("blocking task failed: {e}")))?
        })
    }

    fn fade_restore(&self, snapshot: &VolumeSnapshot, config: &DuckConfig) -> DuckResult<'_, ()> {
        let snapshot = snapshot.clone();
        let config = *config;
        let available = self.available.load(Ordering::SeqCst);

        Box::pin(async move {
            if !available || snapshot.is_empty() {
                return Ok(());
            }

            tokio::task::spawn_blocking(move || fade_restore_blocking(&snapshot, &config))
                .await
                .map_err(|e| DuckingError::RestoreFailed(format!("blocking task failed: {e}")))?
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
///
/// Each session's COM-allocated instance identifier string is converted
/// to a Rust `String` and freed immediately via [`pwstr_to_string_and_free`].
fn enumerate_sessions(our_pid: u32) -> Result<Vec<LiveSession>, DuckingError> {
    let _com =
        ComGuard::new().map_err(|e| DuckingError::Platform(format!("COM init failed: {e}")))?;

    let session_mgr = get_session_manager()?;

    unsafe {
        let enumerator: IAudioSessionEnumerator = session_mgr
            .GetSessionEnumerator()
            .map_err(|e| {
                DuckingError::SnapshotFailed(format!("failed to enumerate sessions: {e}"))
            })?;

        let count = enumerator
            .GetCount()
            .map_err(|e| {
                DuckingError::SnapshotFailed(format!("failed to get session count: {e}"))
            })?;

        let mut sessions = Vec::new();

        for i in 0..count {
            let Ok(control) = enumerator.GetSession(i) else {
                continue;
            };

            let Ok(control2): Result<IAudioSessionControl2, _> = control.cast() else {
                continue;
            };

            let Ok(state) = control2.GetState() else {
                continue;
            };
            if state != AudioSessionStateActive {
                continue;
            }

            let Ok(pid) = control2.GetProcessId() else {
                continue;
            };
            if pid == our_pid {
                continue;
            }

            let Ok(instance_id_pwstr) = control2.GetSessionInstanceIdentifier() else {
                continue;
            };
            let Some(key) = (unsafe { pwstr_to_string_and_free(instance_id_pwstr) }) else {
                continue;
            };
            if key.is_empty() {
                continue;
            }

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

/// Builds a map from session instance identifier to `ISimpleAudioVolume`.
///
/// Enumerates all sessions once and returns a lookup table so that callers
/// avoid O(n²) re-enumeration per snapshot entry.
fn build_volume_map(
    session_mgr: &IAudioSessionManager2,
) -> Result<HashMap<String, ISimpleAudioVolume>, DuckingError> {
    unsafe {
        let enumerator = session_mgr.GetSessionEnumerator().map_err(|e| {
            DuckingError::SnapshotFailed(format!("failed to enumerate sessions: {e}"))
        })?;

        let count = enumerator.GetCount().map_err(|e| {
            DuckingError::SnapshotFailed(format!("failed to get session count: {e}"))
        })?;

        let mut map = HashMap::new();

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

            let Some(key) = (unsafe { pwstr_to_string_and_free(instance_id_pwstr) }) else {
                continue;
            };
            if key.is_empty() {
                continue;
            }

            let Ok(simple_vol): Result<ISimpleAudioVolume, _> = control.cast() else {
                continue;
            };

            map.insert(key, simple_vol);
        }

        Ok(map)
    }
}

/// Fades all snapshotted sessions to the configured floor level.
///
/// Runs entirely on a single blocking thread so that the COM apartment
/// stays pinned for the full duration of the fade loop.
fn fade_to_floor_blocking(
    snapshot: &VolumeSnapshot,
    config: &DuckConfig,
) -> Result<(), DuckingError> {
    let _com = ComGuard::new()
        .map_err(|e| DuckingError::Platform(format!("COM init failed: {e}")))?;

    let session_mgr = get_session_manager()?;
    let volume_map = build_volume_map(&session_mgr)?;

    for entry in &snapshot.entries {
        let SessionId::WasapiSession { key, .. } = &entry.id else {
            continue;
        };

        let Some(simple_vol) = volume_map.get(key) else {
            continue;
        };

        let original_volume = entry.channels.first().copied().unwrap_or(1.0);
        let target_volume = (original_volume * config.floor_scalar()).clamp(0.0, 1.0);
        let steps = compute_fade_steps(original_volume, target_volume, config);

        for step in steps {
            let vol = step.volume.clamp(0.0, 1.0);
            let _ = unsafe { simple_vol.SetMasterVolume(vol, std::ptr::null()) };
            std::thread::sleep(std::time::Duration::from_millis(step.delay_ms as u64));
        }
    }

    Ok(())
}

/// Restores all snapshotted sessions to their original volume and mute state.
///
/// Runs entirely on a single blocking thread so that the COM apartment
/// stays pinned for the full duration of the fade loop. Sessions that
/// disappeared since the snapshot are silently skipped. Mute state is
/// restored only after the final volume step.
fn fade_restore_blocking(
    snapshot: &VolumeSnapshot,
    config: &DuckConfig,
) -> Result<(), DuckingError> {
    let _com = ComGuard::new()
        .map_err(|e| DuckingError::Platform(format!("COM init failed: {e}")))?;

    let session_mgr = get_session_manager()?;
    let volume_map = build_volume_map(&session_mgr)?;

    for entry in &snapshot.entries {
        let SessionId::WasapiSession { key, .. } = &entry.id else {
            continue;
        };

        let Some(simple_vol) = volume_map.get(key) else {
            continue;
        };

        let original_volume = entry.channels.first().copied().unwrap_or(1.0);
        let current_volume = unsafe { simple_vol.GetMasterVolume() }.unwrap_or(0.0);
        let steps = compute_fade_steps(current_volume, original_volume, config);

        for step in &steps {
            let vol = step.volume.clamp(0.0, 1.0);
            let _ = unsafe { simple_vol.SetMasterVolume(vol, std::ptr::null()) };
            std::thread::sleep(std::time::Duration::from_millis(step.delay_ms as u64));
        }

        let _ = unsafe { simple_vol.SetMute(entry.mute.into(), std::ptr::null()) };
    }

    Ok(())
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

    #[test]
    #[ignore = "requires Windows audio device"]
    fn snapshot_enumerates_sessions_on_live_device() {
        let backend = WindowsBackend::new();
        if !backend.is_available() {
            eprintln!("Skipping: WASAPI not available");
            return;
        }
        let rt = tokio::runtime::Runtime::new().unwrap();
        let snapshot = rt.block_on(backend.snapshot()).unwrap();
        assert!(snapshot.len() <= 20, "unexpectedly many sessions");
    }

    #[test]
    #[ignore = "requires Windows audio device"]
    fn com_guard_succeeds_when_com_already_initialized() {
        let _com1 = ComGuard::new().expect("first COM init");
        let _com2 = ComGuard::new().expect("second COM init (S_FALSE)");
    }
}
