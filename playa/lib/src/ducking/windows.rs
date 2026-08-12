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
//!
//! ## Multi-session fade scheduling
//!
//! When multiple audio sessions are active, all sessions share a single
//! `ramp_ms` window. The fade steps are precomputed per session, then
//! applied in lockstep: step 0 is applied to every session, then we sleep
//! once, then step 1 is applied to every session, and so on. This keeps
//! the total ramp time bounded by `ramp_ms` regardless of session count.

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
// `Interface::cast` is used to query sibling COM interfaces off the base
// `IAudioSessionControl`; the trait must be in scope for the method to resolve.
use windows::core::Interface;

use crate::ducking::{
    DuckConfig, DuckResult, DuckingBackend, DuckingError, FadeStep, SessionId, SessionVolume,
    VolumeSnapshot, compute_fade_steps,
};
use crate::windows_com::{ComGuard, pwstr_to_string_and_free};

/// A live session discovered during enumeration.
struct LiveSession {
    pid: u32,
    key: String,
    volume: f32,
    mute: bool,
}

/// Injectable write interface for volume and mute operations.
///
/// This trait enables unit testing of fade scheduling and failure
/// propagation without requiring a live WASAPI device.
pub(crate) trait VolumeWriter {
    /// Sets the master volume level (0.0 to 1.0).
    ///
    /// ## Returns
    ///
    /// `Ok(())` on success, `Err` with a descriptive message on failure.
    fn set_volume(&self, volume: f32) -> Result<(), String>;

    /// Gets the current master volume level.
    ///
    /// ## Returns
    ///
    /// The volume level on success, `Err` with a descriptive message on failure.
    fn get_volume(&self) -> Result<f32, String>;

    /// Sets the mute state.
    ///
    /// ## Returns
    ///
    /// `Ok(())` on success, `Err` with a descriptive message on failure.
    fn set_mute(&self, mute: bool) -> Result<(), String>;
}

struct WasapiVolumeWriter {
    simple_vol: ISimpleAudioVolume,
}

impl VolumeWriter for WasapiVolumeWriter {
    fn set_volume(&self, volume: f32) -> Result<(), String> {
        unsafe {
            self.simple_vol
                .SetMasterVolume(volume.clamp(0.0, 1.0), std::ptr::null())
                .map_err(|e| format!("SetMasterVolume failed: {e}"))
        }
    }

    fn get_volume(&self) -> Result<f32, String> {
        unsafe {
            self.simple_vol
                .GetMasterVolume()
                .map_err(|e| format!("GetMasterVolume failed: {e}"))
        }
    }

    fn set_mute(&self, mute: bool) -> Result<(), String> {
        unsafe {
            self.simple_vol
                .SetMute(mute, std::ptr::null())
                .map_err(|e| format!("SetMute failed: {e}"))
        }
    }
}

/// Precomputed fade plan for a single session within a multi-session ramp.
///
/// Each entry stores the resolved key (for volume map lookup), the full
/// list of fade steps, the original mute state, and the target volume
/// (used only for floor fade direction).
#[derive(Debug, Clone)]
pub(crate) struct SessionFadePlan {
    key: String,
    steps: Vec<FadeStep>,
    mute: bool,
}

/// Summary of write failures encountered during a fade or restore pass.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct FadeFailures {
    pub(crate) volume_write_failures: usize,
    pub(crate) mute_write_failures: usize,
}

impl FadeFailures {
    pub(crate) fn is_empty(&self) -> bool {
        self.volume_write_failures == 0 && self.mute_write_failures == 0
    }
}

/// Builds fade plans for all snapshotted sessions that match the live volume map.
///
/// Each session gets its own step list computed from its original volume to
/// the target. Sessions not found in the volume map are silently skipped
/// (they may have disappeared between snapshot and fade).
pub(crate) fn build_floor_plans(
    snapshot: &VolumeSnapshot,
    volume_map: &HashMap<String, Box<dyn VolumeWriter>>,
    config: &DuckConfig,
) -> Vec<SessionFadePlan> {
    let mut plans = Vec::new();

    for entry in &snapshot.entries {
        let SessionId::WasapiSession { key, .. } = &entry.id else {
            continue;
        };

        if !volume_map.contains_key(key) {
            continue;
        }

        let original_volume = entry.channels.first().copied().unwrap_or(1.0);
        let target_volume = (original_volume * config.floor_scalar()).clamp(0.0, 1.0);
        let steps = compute_fade_steps(original_volume, target_volume, config);

        plans.push(SessionFadePlan {
            key: key.clone(),
            steps,
            mute: entry.mute,
        });
    }

    plans
}

/// Builds restore fade plans for all snapshotted sessions.
///
/// Each session fades from its current volume (read from the writer) back
/// to its snapshotted original volume.
pub(crate) fn build_restore_plans(
    snapshot: &VolumeSnapshot,
    volume_map: &HashMap<String, Box<dyn VolumeWriter>>,
    config: &DuckConfig,
) -> Vec<SessionFadePlan> {
    let mut plans = Vec::new();

    for entry in &snapshot.entries {
        let SessionId::WasapiSession { key, .. } = &entry.id else {
            continue;
        };

        if !volume_map.contains_key(key) {
            continue;
        }

        let original_volume = entry.channels.first().copied().unwrap_or(1.0);
        let current_volume = volume_map
            .get(key)
            .and_then(|w| w.get_volume().ok())
            .unwrap_or(0.0);
        let steps = compute_fade_steps(current_volume, original_volume, config);

        plans.push(SessionFadePlan {
            key: key.clone(),
            steps,
            mute: entry.mute,
        });
    }

    plans
}

/// Executes a multi-session fade in lockstep.
///
/// All sessions advance through the same step index together. The sleep
/// occurs once per step (not once per session), so the total ramp time
/// is bounded by `ramp_ms` regardless of session count.
///
/// Returns a [`FadeFailures`] summary counting any volume or mute write
/// failures on resolved sessions.
pub(crate) fn execute_multi_session_fade(
    plans: &[SessionFadePlan],
    volume_map: &HashMap<String, Box<dyn VolumeWriter>>,
    restore_mute: bool,
) -> FadeFailures {
    let max_steps = plans.iter().map(|p| p.steps.len()).max().unwrap_or(0);
    let mut failures = FadeFailures::default();

    for step_idx in 0..max_steps {
        for plan in plans {
            if step_idx >= plan.steps.len() {
                continue;
            }

            let step = &plan.steps[step_idx];
            let vol = step.volume.clamp(0.0, 1.0);

            if let Some(writer) = volume_map.get(&plan.key)
                && writer.set_volume(vol).is_err()
            {
                failures.volume_write_failures += 1;
            }
        }

        if step_idx < max_steps - 1 {
            let delay = plans
                .iter()
                .filter_map(|p| p.steps.get(step_idx))
                .map(|s| s.delay_ms)
                .min()
                .unwrap_or(15);
            std::thread::sleep(std::time::Duration::from_millis(delay as u64));
        }
    }

    if restore_mute {
        for plan in plans {
            if let Some(writer) = volume_map.get(&plan.key)
                && writer.set_mute(plan.mute).is_err()
            {
                failures.mute_write_failures += 1;
            }
        }
    }

    failures
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

        let session_mgr: IAudioSessionManager2 =
            device.Activate(CLSCTX_ALL, None).map_err(|e| {
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
        let enumerator: IAudioSessionEnumerator =
            session_mgr.GetSessionEnumerator().map_err(|e| {
                DuckingError::SnapshotFailed(format!("failed to enumerate sessions: {e}"))
            })?;

        let count = enumerator.GetCount().map_err(|e| {
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
            let Some(key) = pwstr_to_string_and_free(instance_id_pwstr) else {
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

/// Builds a map from session instance identifier to a boxed [`VolumeWriter`].
///
/// Enumerates all sessions once and returns a lookup table so that callers
/// avoid O(n²) re-enumeration per snapshot entry.
fn build_volume_map(
    session_mgr: &IAudioSessionManager2,
) -> Result<HashMap<String, Box<dyn VolumeWriter>>, DuckingError> {
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

            let Some(key) = pwstr_to_string_and_free(instance_id_pwstr) else {
                continue;
            };
            if key.is_empty() {
                continue;
            }

            let Ok(simple_vol): Result<ISimpleAudioVolume, _> = control.cast() else {
                continue;
            };

            map.insert(
                key,
                Box::new(WasapiVolumeWriter { simple_vol }) as Box<dyn VolumeWriter>,
            );
        }

        Ok(map)
    }
}

/// Fades all snapshotted sessions to the configured floor level.
///
/// Runs entirely on a single blocking thread so that the COM apartment
/// stays pinned for the full duration of the fade loop. All sessions
/// share a single `ramp_ms` window: steps are applied in lockstep across
/// sessions so total ramp time does not scale with session count.
///
/// ## Errors
///
/// Returns [`DuckingError::FadeFailed`] if any resolved session could not
/// be updated.
fn fade_to_floor_blocking(
    snapshot: &VolumeSnapshot,
    config: &DuckConfig,
) -> Result<(), DuckingError> {
    let _com =
        ComGuard::new().map_err(|e| DuckingError::Platform(format!("COM init failed: {e}")))?;

    let session_mgr = get_session_manager()?;
    let volume_map = build_volume_map(&session_mgr)?;
    let plans = build_floor_plans(snapshot, &volume_map, config);

    if plans.is_empty() {
        return Ok(());
    }

    let failures = execute_multi_session_fade(&plans, &volume_map, false);

    if !failures.is_empty() {
        return Err(DuckingError::FadeFailed(format!(
            "fade encountered {} volume write failures across {} sessions",
            failures.volume_write_failures,
            plans.len(),
        )));
    }

    Ok(())
}

/// Restores all snapshotted sessions to their original volume and mute state.
///
/// Runs entirely on a single blocking thread so that the COM apartment
/// stays pinned for the full duration of the fade loop. Sessions that
/// disappeared since the snapshot are silently skipped. Mute state is
/// restored only after the final volume step.
///
/// All sessions share a single `ramp_ms` window: steps are applied in
/// lockstep across sessions so total ramp time does not scale with
/// session count.
///
/// ## Errors
///
/// Returns [`DuckingError::RestoreFailed`] if any resolved session could
/// not be updated.
fn fade_restore_blocking(
    snapshot: &VolumeSnapshot,
    config: &DuckConfig,
) -> Result<(), DuckingError> {
    let _com =
        ComGuard::new().map_err(|e| DuckingError::Platform(format!("COM init failed: {e}")))?;

    let session_mgr = get_session_manager()?;
    let volume_map = build_volume_map(&session_mgr)?;
    let plans = build_restore_plans(snapshot, &volume_map, config);

    if plans.is_empty() {
        return Ok(());
    }

    let failures = execute_multi_session_fade(&plans, &volume_map, true);

    if !failures.is_empty() {
        return Err(DuckingError::RestoreFailed(format!(
            "restore encountered {} volume and {} mute write failures across {} sessions",
            failures.volume_write_failures,
            failures.mute_write_failures,
            plans.len(),
        )));
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
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let snapshot = rt.block_on(backend.snapshot()).unwrap();
        assert!(snapshot.len() <= 20, "unexpectedly many sessions");
    }

    #[test]
    #[ignore = "requires Windows audio device"]
    fn com_guard_succeeds_when_com_already_initialized() {
        let _com1 = ComGuard::new().expect("first COM init");
        let _com2 = ComGuard::new().expect("second COM init (S_FALSE)");
    }

    #[tokio::test]
    #[ignore = "requires Windows audio device"]
    async fn windows_backend_fade_restore_roundtrip() {
        let backend = WindowsBackend::new();
        if !backend.is_available() {
            eprintln!("Skipping: WASAPI not available");
            return;
        }

        let snapshot = backend.snapshot().await.unwrap();
        if snapshot.is_empty() {
            eprintln!("Skipping: no active sessions to duck");
            return;
        }

        let config = DuckConfig::new(50, 0.5).unwrap();

        backend
            .fade_to_floor(&snapshot, &config)
            .await
            .expect("fade to floor should succeed");

        backend
            .fade_restore(&snapshot, &config)
            .await
            .expect("fade restore should succeed");

        let after = backend
            .snapshot()
            .await
            .expect("post-restore snapshot should succeed");
        assert!(
            after.len() >= snapshot.len().saturating_sub(1),
            "sessions should still exist after round-trip (before={}, after={})",
            snapshot.len(),
            after.len()
        );
    }

    struct MockVolumeWriter {
        volume: std::sync::Mutex<f32>,
        mute: std::sync::Mutex<bool>,
        set_volume_fails: bool,
        set_mute_fails: bool,
        volume_writes: std::sync::Mutex<Vec<f32>>,
    }

    impl MockVolumeWriter {
        fn new(volume: f32, mute: bool) -> Self {
            Self {
                volume: std::sync::Mutex::new(volume),
                mute: std::sync::Mutex::new(mute),
                set_volume_fails: false,
                set_mute_fails: false,
                volume_writes: std::sync::Mutex::new(Vec::new()),
            }
        }

        fn with_volume_failure(mut self) -> Self {
            self.set_volume_fails = true;
            self
        }

        fn with_mute_failure(mut self) -> Self {
            self.set_mute_fails = true;
            self
        }

        fn written_volumes(&self) -> Vec<f32> {
            self.volume_writes.lock().unwrap().clone()
        }
    }

    impl VolumeWriter for MockVolumeWriter {
        fn set_volume(&self, volume: f32) -> Result<(), String> {
            if self.set_volume_fails {
                return Err("SetMasterVolume failed: mock error".to_string());
            }
            *self.volume.lock().unwrap() = volume;
            self.volume_writes.lock().unwrap().push(volume);
            Ok(())
        }

        fn get_volume(&self) -> Result<f32, String> {
            Ok(*self.volume.lock().unwrap())
        }

        fn set_mute(&self, mute: bool) -> Result<(), String> {
            if self.set_mute_fails {
                return Err("SetMute failed: mock error".to_string());
            }
            *self.mute.lock().unwrap() = mute;
            Ok(())
        }
    }

    fn make_snapshot(entries: Vec<(&str, f32, bool)>) -> VolumeSnapshot {
        VolumeSnapshot::with_entries(
            entries
                .into_iter()
                .enumerate()
                .map(|(i, (key, vol, mute))| {
                    SessionVolume::new(
                        SessionId::WasapiSession {
                            pid: 100 + i as u32,
                            key: key.to_string(),
                        },
                        vec![vol],
                        mute,
                    )
                })
                .collect(),
        )
    }

    fn make_volume_map(mocks: Vec<(&str, f32, bool)>) -> HashMap<String, Box<dyn VolumeWriter>> {
        mocks
            .into_iter()
            .map(|(key, vol, mute)| {
                (
                    key.to_string(),
                    Box::new(MockVolumeWriter::new(vol, mute)) as Box<dyn VolumeWriter>,
                )
            })
            .collect()
    }

    #[test]
    fn build_floor_plans_skips_missing_sessions() {
        let snapshot = make_snapshot(vec![("alive", 0.8, false), ("gone", 0.6, false)]);
        let volume_map = make_volume_map(vec![("alive", 0.8, false)]);
        let config = DuckConfig::new(100, 0.2).unwrap();

        let plans = build_floor_plans(&snapshot, &volume_map, &config);
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].key, "alive");
    }

    #[test]
    fn build_floor_plans_targets_floor_scalar() {
        let snapshot = make_snapshot(vec![("app", 1.0, false)]);
        let volume_map = make_volume_map(vec![("app", 1.0, false)]);
        let config = DuckConfig::new(100, 0.2).unwrap();

        let plans = build_floor_plans(&snapshot, &volume_map, &config);
        assert_eq!(plans.len(), 1);
        let last = plans[0].steps.last().unwrap();
        assert!((last.volume - 0.2).abs() < f32::EPSILON);
    }

    #[test]
    fn build_restore_plans_fades_from_current_to_original() {
        let snapshot = make_snapshot(vec![("app", 0.9, false)]);
        let volume_map = make_volume_map(vec![("app", 0.2, false)]);
        let config = DuckConfig::new(100, 0.2).unwrap();

        let plans = build_restore_plans(&snapshot, &volume_map, &config);
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].key, "app");
        let last = plans[0].steps.last().unwrap();
        assert!(
            (last.volume - 0.9).abs() < f32::EPSILON,
            "should restore to original volume, got {}",
            last.volume
        );
    }

    #[test]
    fn multi_session_fade_applies_all_steps_to_each_session() {
        let snapshot = make_snapshot(vec![("a", 0.8, false), ("b", 0.6, false)]);
        let volume_map = make_volume_map(vec![("a", 0.8, false), ("b", 0.6, false)]);
        let config = DuckConfig::new(100, 0.2).unwrap();

        let plans = build_floor_plans(&snapshot, &volume_map, &config);
        let failures = execute_multi_session_fade(&plans, &volume_map, false);

        assert!(failures.is_empty());
    }

    #[test]
    fn multi_session_fade_shares_single_ramp_window() {
        let snapshot = make_snapshot(vec![
            ("a", 1.0, false),
            ("b", 0.5, false),
            ("c", 0.8, false),
        ]);
        let volume_map = make_volume_map(vec![
            ("a", 1.0, false),
            ("b", 0.5, false),
            ("c", 0.8, false),
        ]);
        let config = DuckConfig::new(100, 0.2).unwrap();

        let plans = build_floor_plans(&snapshot, &volume_map, &config);
        let max_steps = plans.iter().map(|p| p.steps.len()).max().unwrap_or(0);

        assert!(max_steps > 0, "should have at least one step per session");

        for plan in &plans {
            assert_eq!(
                plan.steps.len(),
                max_steps,
                "all sessions should have the same number of steps since they share the same config"
            );
        }
    }

    #[test]
    fn multi_session_fade_counts_volume_write_failures() {
        let snapshot = make_snapshot(vec![("ok", 0.8, false), ("bad", 0.6, false)]);
        let mut volume_map: HashMap<String, Box<dyn VolumeWriter>> = HashMap::new();
        volume_map.insert(
            "ok".to_string(),
            Box::new(MockVolumeWriter::new(0.8, false)),
        );
        volume_map.insert(
            "bad".to_string(),
            Box::new(MockVolumeWriter::new(0.6, false).with_volume_failure()),
        );

        let config = DuckConfig::new(100, 0.2).unwrap();
        let plans = build_floor_plans(&snapshot, &volume_map, &config);
        let failures = execute_multi_session_fade(&plans, &volume_map, false);

        assert!(failures.volume_write_failures > 0);
        assert_eq!(failures.mute_write_failures, 0);
    }

    #[test]
    fn multi_session_restore_counts_mute_failures() {
        let snapshot = make_snapshot(vec![("ok", 0.8, true), ("bad-mute", 0.6, false)]);
        let mut volume_map: HashMap<String, Box<dyn VolumeWriter>> = HashMap::new();
        volume_map.insert("ok".to_string(), Box::new(MockVolumeWriter::new(0.2, true)));
        volume_map.insert(
            "bad-mute".to_string(),
            Box::new(MockVolumeWriter::new(0.2, false).with_mute_failure()),
        );

        let config = DuckConfig::new(100, 0.2).unwrap();
        let plans = build_restore_plans(&snapshot, &volume_map, &config);
        let failures = execute_multi_session_fade(&plans, &volume_map, true);

        assert_eq!(failures.mute_write_failures, 1);
    }

    #[test]
    fn fade_failures_default_is_empty() {
        let f = FadeFailures::default();
        assert!(f.is_empty());
        assert_eq!(f.volume_write_failures + f.mute_write_failures, 0);
    }

    #[test]
    fn fade_failures_non_empty_with_volume_writes() {
        let f = FadeFailures {
            volume_write_failures: 3,
            mute_write_failures: 0,
        };
        assert!(!f.is_empty());
        assert_eq!(f.volume_write_failures + f.mute_write_failures, 3);
    }

    #[test]
    fn empty_snapshot_produces_no_plans() {
        let snapshot = VolumeSnapshot::new();
        let volume_map = make_volume_map(vec![("a", 0.8, false)]);
        let config = DuckConfig::new(100, 0.2).unwrap();

        let plans = build_floor_plans(&snapshot, &volume_map, &config);
        assert!(plans.is_empty());
    }

    #[test]
    fn empty_plans_produces_no_failures() {
        let plans: Vec<SessionFadePlan> = Vec::new();
        let volume_map: HashMap<String, Box<dyn VolumeWriter>> = HashMap::new();
        let failures = execute_multi_session_fade(&plans, &volume_map, false);
        assert!(failures.is_empty());
    }

    #[test]
    fn mock_writer_records_all_volume_writes() {
        let writer = MockVolumeWriter::new(0.8, false);
        writer.set_volume(0.6).unwrap();
        writer.set_volume(0.4).unwrap();
        assert_eq!(writer.written_volumes(), vec![0.6, 0.4]);
    }

    #[test]
    fn mock_writer_volume_failure_propagates() {
        let writer = MockVolumeWriter::new(0.8, false).with_volume_failure();
        let result = writer.set_volume(0.4);
        assert!(result.is_err());
    }

    #[test]
    fn mock_writer_mute_failure_propagates() {
        let writer = MockVolumeWriter::new(0.8, false).with_mute_failure();
        let result = writer.set_mute(true);
        assert!(result.is_err());
    }

    #[test]
    fn step_count_is_consistent_for_same_config() {
        let snapshot = make_snapshot(vec![
            ("a", 1.0, false),
            ("b", 0.3, false),
            ("c", 0.7, false),
        ]);
        let volume_map = make_volume_map(vec![
            ("a", 1.0, false),
            ("b", 0.3, false),
            ("c", 0.7, false),
        ]);
        let config = DuckConfig::new(100, 0.2).unwrap();

        let plans = build_floor_plans(&snapshot, &volume_map, &config);

        let step_counts: Vec<usize> = plans.iter().map(|p| p.steps.len()).collect();
        assert!(
            step_counts.iter().all(|c| *c == step_counts[0]),
            "all sessions should have equal step counts for the same config: {:?}",
            step_counts
        );
    }

    #[test]
    fn restore_preserves_original_mute_state() {
        let snapshot = make_snapshot(vec![("was-muted", 0.5, true), ("was-unmuted", 0.7, false)]);
        let volume_map =
            make_volume_map(vec![("was-muted", 0.1, false), ("was-unmuted", 0.1, false)]);
        let config = DuckConfig::new(100, 0.2).unwrap();

        let plans = build_restore_plans(&snapshot, &volume_map, &config);
        assert_eq!(plans.len(), 2);

        for plan in &plans {
            match plan.key.as_str() {
                "was-muted" => assert!(plan.mute, "should restore mute=true"),
                "was-unmuted" => assert!(!plan.mute, "should restore mute=false"),
                _ => panic!("unexpected key"),
            }
        }
    }
}
