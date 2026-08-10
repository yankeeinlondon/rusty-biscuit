//! Backend factory for platform-specific ducking implementation selection.
//!
//! This module provides [`create_backend()`] which returns the appropriate
//! ducking backend for the current platform at runtime.

use crate::ducking::{DuckingBackend, NoopBackend};

#[cfg(all(target_os = "macos", feature = "audio-ducking-macos"))]
use crate::ducking::{MacOsBackend, MediaKeysBackend};

#[cfg(all(target_os = "windows", feature = "audio-ducking-windows"))]
use crate::ducking::WindowsBackend;

#[cfg(all(target_os = "linux", feature = "audio-ducking-linux"))]
use crate::ducking::LinuxBackend;

/// Creates the appropriate ducking backend for the current platform.
///
/// ## Platform Selection
///
/// - **macOS**: Returns `MacOsBackend` (CoreAudio virtual master volume)
/// - **Windows**: Returns `WindowsBackend` (WASAPI per-session volume)
/// - **Linux**: Returns `LinuxBackend` (PulseAudio/PipeWire) or `NoopBackend`
/// - **Other**: Returns `NoopBackend` (no-op, graceful degradation)
///
/// ## Returns
///
/// A boxed [`DuckingBackend`] trait object suitable for the current platform.
/// The returned backend is guaranteed to be Send + Sync for use in async contexts.
///
/// ## Example
///
/// ```
/// use playa::ducking::{create_backend, DuckingBackend};
///
/// let backend = create_backend();
/// println!("Using ducking backend: {}", backend.name());
/// ```
pub fn create_backend() -> Box<dyn DuckingBackend> {
    #[cfg(all(target_os = "macos", feature = "audio-ducking-macos"))]
    {
        // Try CoreAudio volume control first
        let macos_backend = MacOsBackend::new();
        if macos_backend.is_available() {
            return Box::new(macos_backend);
        }

        // Fall back to media key pause/resume if volume control unavailable
        // (e.g., professional DACs, USB audio interfaces)
        let media_keys_backend = MediaKeysBackend::new();
        if media_keys_backend.is_available() {
            return Box::new(media_keys_backend);
        }

        // Last resort: no-op
        return Box::new(NoopBackend::new());
    }

    #[cfg(all(target_os = "macos", not(feature = "audio-ducking-macos")))]
    {
        return Box::new(NoopBackend::new());
    }

    #[cfg(all(target_os = "windows", feature = "audio-ducking-windows"))]
    {
        let backend = WindowsBackend::new();
        if backend.is_available() {
            return Box::new(backend);
        }
        Box::new(NoopBackend::new())
    }

    #[cfg(all(target_os = "windows", not(feature = "audio-ducking-windows")))]
    {
        use std::sync::Once;
        static WARN_ONCE: Once = Once::new();
        WARN_ONCE.call_once(|| {
            eprintln!(
                "playa: audio ducking on Windows requires the `audio-ducking-windows` feature; \
                 falling back to noop"
            );
        });
        return Box::new(NoopBackend::new());
    }

    #[cfg(all(target_os = "linux", feature = "audio-ducking-linux"))]
    {
        // Try PulseAudio/PipeWire only
        let linux_backend = LinuxBackend::new();
        if linux_backend.is_available() {
            return Box::new(linux_backend);
        }

        // PulseAudio unavailable: skip ALSA fallback (it would also duck
        // Playa's own output) and degrade to noop.
        use std::sync::Once;
        static WARN_ONCE: Once = Once::new();
        WARN_ONCE.call_once(|| {
            eprintln!(
                "playa: PulseAudio/PipeWire unavailable; skipping Linux ducking because \
                 ALSA fallback would also duck Playa's own output"
            );
        });

        Box::new(NoopBackend::new())
    }

    #[cfg(all(target_os = "linux", not(feature = "audio-ducking-linux")))]
    {
        return Box::new(NoopBackend::new());
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        Box::new(NoopBackend::new())
    }
}

/// Returns the name of the backend that would be created for this platform.
///
/// Useful for diagnostics without actually creating the backend.
///
/// Note: On macOS with `audio-ducking-macos` feature, this returns the
/// dynamically selected backend name based on device capabilities.
pub fn backend_name() -> &'static str {
    #[cfg(all(target_os = "macos", feature = "audio-ducking-macos"))]
    {
        // Check which backend would actually be used
        let macos_backend = MacOsBackend::new();
        if macos_backend.is_available() {
            return "macos-coreaudio";
        }

        let media_keys_backend = MediaKeysBackend::new();
        if media_keys_backend.is_available() {
            return "macos-media-keys";
        }

        return "noop";
    }

    #[cfg(all(target_os = "macos", not(feature = "audio-ducking-macos")))]
    {
        return "noop";
    }

    #[cfg(all(target_os = "windows", feature = "audio-ducking-windows"))]
    {
        let backend = WindowsBackend::new();
        if backend.is_available() {
            return "windows-wasapi";
        }
        "noop"
    }

    #[cfg(all(target_os = "windows", not(feature = "audio-ducking-windows")))]
    {
        return "noop";
    }

    #[cfg(all(target_os = "linux", feature = "audio-ducking-linux"))]
    {
        let linux_backend = LinuxBackend::new();
        if linux_backend.is_available() {
            return "linux-pulse";
        }

        "noop"
    }

    #[cfg(all(target_os = "linux", not(feature = "audio-ducking-linux")))]
    {
        return "noop";
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        "noop"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_backend_returns_valid_backend() {
        let backend = create_backend();
        // Backend should be available (even if it's noop)
        assert!(backend.is_available());
    }

    #[test]
    fn create_backend_is_send_sync() {
        let backend = create_backend();
        fn assert_send_sync<T: Send + Sync + ?Sized>(_: &T) {}
        assert_send_sync(&*backend);
    }

    #[test]
    fn backend_name_returns_string() {
        let name = backend_name();
        assert!(!name.is_empty());
    }

    #[cfg(all(target_os = "macos", feature = "audio-ducking-macos"))]
    #[test]
    fn macos_factory_creates_real_backend() {
        let backend = create_backend();
        // Backend will be either coreaudio (if volume control available) or
        // media-keys (fallback for DACs/interfaces without software volume)
        let name = backend.name();
        assert!(
            name == "macos-coreaudio" || name == "macos-media-keys",
            "expected macos-coreaudio or macos-media-keys, got {}",
            name
        );
    }

    #[cfg(all(target_os = "macos", not(feature = "audio-ducking-macos")))]
    #[test]
    fn macos_factory_creates_noop_without_feature() {
        let backend = create_backend();
        assert_eq!(backend.name(), "noop");
    }

    #[cfg(all(target_os = "linux", feature = "audio-ducking-linux"))]
    #[test]
    fn linux_factory_creates_real_backend() {
        let backend = create_backend();
        let name = backend.name();
        assert!(
            name == "linux-pulse" || name == "noop",
            "expected linux-pulse or noop, got {}",
            name
        );
    }

    #[cfg(all(target_os = "linux", not(feature = "audio-ducking-linux")))]
    #[test]
    fn linux_factory_creates_noop_without_feature() {
        let backend = create_backend();
        assert_eq!(backend.name(), "noop");
    }

    #[cfg(all(target_os = "windows", feature = "audio-ducking-windows"))]
    #[test]
    fn windows_factory_creates_real_backend() {
        let backend = create_backend();
        let name = backend.name();
        assert!(
            name == "windows-wasapi" || name == "noop",
            "expected windows-wasapi or noop, got {}",
            name
        );
    }

    #[cfg(all(target_os = "windows", not(feature = "audio-ducking-windows")))]
    #[test]
    fn windows_factory_creates_noop_without_feature() {
        let backend = create_backend();
        assert_eq!(backend.name(), "noop");
    }
}
