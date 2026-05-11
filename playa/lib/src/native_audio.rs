use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::mpsc;
#[cfg(test)]
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

use crate::channels::DeviceLookupError;

/// Shared deadline for native device lookup and device open operations.
pub(crate) const NATIVE_DEVICE_TIMEOUT: Duration = Duration::from_secs(5);

static NATIVE_AUDIO_ENABLED: AtomicBool = AtomicBool::new(true);
static NATIVE_AUDIO_FAILURE_REASON: AtomicU8 = AtomicU8::new(0);
static NATIVE_AUDIO_DISABLED_LOGGED: AtomicBool = AtomicBool::new(false);

#[cfg(test)]
static TEST_STATE_LOCK: Mutex<()> = Mutex::new(());

/// Process-local reasons why native audio was disabled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum NativeAudioFailureKind {
    /// A native device open operation exceeded the shared timeout.
    DeviceOpenTimeout = 1,
}

impl NativeAudioFailureKind {
    fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::DeviceOpenTimeout),
            _ => None,
        }
    }

    fn summary(self) -> &'static str {
        match self {
            Self::DeviceOpenTimeout => "device-open timeout",
        }
    }
}

/// Returns `true` while native audio is still considered safe to attempt.
pub(crate) fn native_audio_available() -> bool {
    NATIVE_AUDIO_ENABLED.load(Ordering::Acquire)
}

/// Returns the recorded native audio failure reason, if any.
pub(crate) fn native_audio_failure_reason() -> Option<NativeAudioFailureKind> {
    NativeAudioFailureKind::from_u8(NATIVE_AUDIO_FAILURE_REASON.load(Ordering::Acquire))
}

/// Disables native audio for the rest of the process.
pub(crate) fn trip_native_audio_breaker(reason: NativeAudioFailureKind) {
    let was_enabled = NATIVE_AUDIO_ENABLED.swap(false, Ordering::AcqRel);
    let _ = NATIVE_AUDIO_FAILURE_REASON.compare_exchange(
        0,
        reason as u8,
        Ordering::AcqRel,
        Ordering::Acquire,
    );

    if was_enabled {
        eprintln!(
            "playa: disabling native audio playback for this process after {}",
            reason.summary()
        );
    }
}

/// Logs a single process-wide diagnostic when native audio is skipped.
pub(crate) fn log_native_audio_disabled_once() {
    if native_audio_available() || NATIVE_AUDIO_DISABLED_LOGGED.swap(true, Ordering::AcqRel) {
        return;
    }

    let reason = native_audio_failure_reason()
        .map(NativeAudioFailureKind::summary)
        .unwrap_or("an earlier native audio failure");
    eprintln!(
        "playa: skipping native audio playback because it was disabled earlier in this process ({reason})"
    );
}

/// Returns the remaining time until `deadline`, saturating at zero.
pub(crate) fn remaining_until(deadline: Instant) -> Duration {
    deadline.saturating_duration_since(Instant::now())
}

/// Runs a potentially blocking native-device operation behind a timeout.
pub(crate) fn run_with_timeout<T, E, F, TimeoutFn>(
    timeout: Duration,
    operation: F,
    on_timeout: TimeoutFn,
) -> Result<T, E>
where
    T: Send + 'static,
    E: Send + 'static,
    F: FnOnce() -> Result<T, E> + Send + 'static,
    TimeoutFn: FnOnce(Duration) -> E,
{
    let (tx, rx) = mpsc::channel();

    std::thread::spawn(move || {
        let _ = tx.send(operation());
    });

    match rx.recv_timeout(timeout) {
        Ok(result) => result,
        Err(mpsc::RecvTimeoutError::Timeout | mpsc::RecvTimeoutError::Disconnected) => {
            Err(on_timeout(timeout))
        }
    }
}

/// Resolve an optional requested channel, then open the requested or default
/// device within a single shared deadline.
pub(crate) fn open_with_channel_fallback<D, T, E, Lookup, OpenRequested, OpenDefault, Exhausted>(
    requested_channel: Option<&str>,
    deadline: Instant,
    mut lookup_device: Lookup,
    mut open_requested: OpenRequested,
    mut open_default: OpenDefault,
    on_deadline_exhausted: Exhausted,
) -> Result<T, E>
where
    Lookup: FnMut(&str, Duration) -> Result<Option<D>, DeviceLookupError>,
    OpenRequested: FnMut(D, Duration) -> Result<T, E>,
    OpenDefault: FnMut(Duration) -> Result<T, E>,
    Exhausted: Fn() -> E,
{
    if let Some(channel_name) = requested_channel {
        let remaining = remaining_until(deadline);
        if remaining.is_zero() {
            return Err(on_deadline_exhausted());
        }

        match lookup_device(channel_name, remaining) {
            Ok(Some(device)) => {
                let remaining = remaining_until(deadline);
                if remaining.is_zero() {
                    return Err(on_deadline_exhausted());
                }
                return open_requested(device, remaining);
            }
            Ok(None) => {
                eprintln!(
                    "playa: output channel '{channel_name}' was not found; falling back to the default output device"
                );
            }
            Err(DeviceLookupError::TimedOut) => {
                eprintln!(
                    "playa: audio device lookup for channel '{channel_name}' timed out after {}s; falling back to the default output device",
                    NATIVE_DEVICE_TIMEOUT.as_secs()
                );
            }
        }
    }

    let remaining = remaining_until(deadline);
    if remaining.is_zero() {
        return Err(on_deadline_exhausted());
    }

    open_default(remaining)
}

#[cfg(test)]
pub(crate) struct NativeAudioTestGuard {
    _guard: MutexGuard<'static, ()>,
}

#[cfg(test)]
impl Drop for NativeAudioTestGuard {
    fn drop(&mut self) {
        reset_native_audio_breaker_for_tests();
    }
}

#[cfg(test)]
pub(crate) fn lock_native_audio_test_state() -> NativeAudioTestGuard {
    let guard = TEST_STATE_LOCK
        .lock()
        .expect("native audio test lock poisoned");
    reset_native_audio_breaker_for_tests();
    NativeAudioTestGuard { _guard: guard }
}

#[cfg(test)]
pub(crate) fn reset_native_audio_breaker_for_tests() {
    NATIVE_AUDIO_ENABLED.store(true, Ordering::Release);
    NATIVE_AUDIO_FAILURE_REASON.store(0, Ordering::Release);
    NATIVE_AUDIO_DISABLED_LOGGED.store(false, Ordering::Release);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn breaker_starts_enabled() {
        let _guard = lock_native_audio_test_state();
        assert!(native_audio_available());
        assert_eq!(native_audio_failure_reason(), None);
    }

    #[test]
    fn breaker_records_timeout_reason() {
        let _guard = lock_native_audio_test_state();
        trip_native_audio_breaker(NativeAudioFailureKind::DeviceOpenTimeout);
        assert!(!native_audio_available());
        assert_eq!(
            native_audio_failure_reason(),
            Some(NativeAudioFailureKind::DeviceOpenTimeout)
        );
    }

    #[test]
    fn channel_lookup_timeout_falls_back_to_default_path() {
        let _guard = lock_native_audio_test_state();
        let deadline = Instant::now() + Duration::from_secs(1);
        let result = open_with_channel_fallback(
            Some("desk-speakers"),
            deadline,
            |_channel, _timeout| Err(DeviceLookupError::TimedOut),
            |_device: &'static str, _timeout| Ok::<_, &'static str>("requested"),
            |_timeout| Ok::<_, &'static str>("default"),
            || "deadline exhausted",
        )
        .expect("should fall back to default path");

        assert_eq!(result, "default");
    }
}
