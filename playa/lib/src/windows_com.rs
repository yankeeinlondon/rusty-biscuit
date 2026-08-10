//! Shared RAII helper for Windows COM initialization.
//!
//! Both the SFX player and the ducking backend need COM initialized on their
//! thread. This module centralizes the lifecycle so we correctly handle
//! `S_OK`, `S_FALSE`, and `RPC_E_CHANGED_MODE` without leaking init counts.
//!
//! This module also provides a safe helper for converting COM-allocated
//! `PWSTR` strings to Rust `String` values with automatic memory cleanup.

use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize};

/// HRESULT for "COM already initialized with a different threading model".
const RPC_E_CHANGED_MODE: i32 = -2147417850_i32; // 0x80010106

/// Error from COM initialization.
///
/// Preserves the original HRESULT for diagnostics and logging.
#[derive(Debug)]
pub struct ComInitError {
    hr: i32,
}

impl std::fmt::Display for ComInitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "COM initialization failed: HRESULT {:#010X}", self.hr)
    }
}

impl std::error::Error for ComInitError {}

/// Classification of COM initialization outcomes.
///
/// Encodes the three success states of `CoInitializeEx` and makes the
/// uninit policy testable without calling the actual COM API.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum ComInitKind {
    /// First initialization on this thread (S_OK). Uninitialize on drop.
    FirstInit,
    /// Already initialized with same model (S_FALSE). Uninitialize on drop.
    AlreadySameModel,
    /// Already initialized with different model (RPC_E_CHANGED_MODE).
    /// Do not uninitialize on drop — the original owner is responsible.
    DifferentModel,
}

impl ComInitKind {
    /// Classifies a raw HRESULT from `CoInitializeEx`.
    ///
    /// ## Returns
    ///
    /// - `Ok(FirstInit)` for `S_OK` (0x00000000)
    /// - `Ok(AlreadySameModel)` for `S_FALSE` (0x00000001)
    /// - `Ok(DifferentModel)` for `RPC_E_CHANGED_MODE` (0x80010106)
    /// - `Err(ComInitError)` for any other HRESULT
    pub(crate) fn from_hresult(hr: i32) -> Result<Self, ComInitError> {
        match hr {
            0 => Ok(Self::FirstInit),
            1 => Ok(Self::AlreadySameModel),
            RPC_E_CHANGED_MODE => Ok(Self::DifferentModel),
            other => Err(ComInitError { hr: other }),
        }
    }

    /// Whether `CoUninitialize` should be called on cleanup.
    ///
    /// Only `FirstInit` and `AlreadySameModel` should uninitialize,
    /// because those are the cases where this call incremented the
    /// COM reference count.
    pub(crate) fn should_uninit(&self) -> bool {
        matches!(self, Self::FirstInit | Self::AlreadySameModel)
    }
}

/// RAII guard for COM initialization on the current thread.
///
/// Calls `CoInitializeEx(COINIT_MULTITHREADED)` on creation and
/// `CoUninitialize` on drop only when this call actually incremented
/// the init count (`S_OK` or `S_FALSE`).
///
/// If COM was already initialized with a different threading model
/// (`RPC_E_CHANGED_MODE`), the guard succeeds but skips uninit on drop.
///
/// ## Thread Safety
///
/// `ComGuard` and any COM interfaces acquired during its lifetime must
/// not be held across `.await` points in async code. Use `spawn_blocking`
/// to keep the entire COM lifecycle on one OS thread.
#[derive(Debug)]
pub struct ComGuard {
    should_uninit: bool,
}

impl ComGuard {
    /// Initializes COM for the current thread.
    ///
    /// ## Errors
    ///
    /// Returns [`ComInitError`] if COM initialization fails for a reason
    /// other than `RPC_E_CHANGED_MODE`.
    pub fn new() -> Result<Self, ComInitError> {
        // `CoInitializeEx` returns the raw `HRESULT` directly (not a `Result`)
        // because `S_FALSE` and `RPC_E_CHANGED_MODE` are meaningful non-error
        // outcomes we must classify ourselves rather than treat as failures.
        let hr = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        let kind = ComInitKind::from_hresult(hr.0)?;
        Ok(Self {
            should_uninit: kind.should_uninit(),
        })
    }
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        if self.should_uninit {
            unsafe {
                CoUninitialize();
            }
        }
    }
}

/// Converts a null-terminated UTF-16 `PWSTR` to a Rust `String`.
///
/// Returns `None` if the pointer is null or the UTF-16 data cannot be
/// converted to valid UTF-8.
///
/// ## Safety
///
/// The caller must ensure `pwstr` points to a valid null-terminated UTF-16
/// string.
#[cfg(any(test, feature = "audio-ducking-windows"))]
unsafe fn pwstr_to_string(pwstr: windows::core::PWSTR) -> Option<String> {
    if pwstr.is_null() {
        return None;
    }
    // SAFETY: The caller guarantees a valid null-terminated UTF-16 string.
    unsafe { pwstr.to_string() }.ok()
}

/// Converts a COM-allocated `PWSTR` to a `String` and frees the allocation.
///
/// Always frees the COM memory (when non-null), even if conversion fails.
/// Returns `None` if the pointer is null or the UTF-16 data cannot be
/// converted to valid UTF-8.
///
/// ## Safety
///
/// The caller must ensure `pwstr` points to a valid null-terminated UTF-16
/// string allocated with `CoTaskMemAlloc` (or returned by a COM method that
/// uses `CoTaskMemAlloc`).
#[cfg(any(test, feature = "audio-ducking-windows"))]
pub(crate) unsafe fn pwstr_to_string_and_free(pwstr: windows::core::PWSTR) -> Option<String> {
    // SAFETY: The caller provides the valid COM-allocated string required by
    // this function's contract.
    let result = unsafe { pwstr_to_string(pwstr) };
    if !pwstr.is_null() {
        // SAFETY: The caller guarantees the non-null pointer came from COM's
        // task allocator, and this function frees it exactly once.
        unsafe {
            windows::Win32::System::Com::CoTaskMemFree(Some(pwstr.0 as *const _));
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_s_ok() {
        let kind = ComInitKind::from_hresult(0).unwrap();
        assert_eq!(kind, ComInitKind::FirstInit);
        assert!(kind.should_uninit());
    }

    #[test]
    fn classify_s_false() {
        let kind = ComInitKind::from_hresult(1).unwrap();
        assert_eq!(kind, ComInitKind::AlreadySameModel);
        assert!(kind.should_uninit());
    }

    #[test]
    fn classify_rpc_e_changed_mode() {
        let kind = ComInitKind::from_hresult(RPC_E_CHANGED_MODE).unwrap();
        assert_eq!(kind, ComInitKind::DifferentModel);
        assert!(!kind.should_uninit());
    }

    #[test]
    fn classify_unknown_failure() {
        let result = ComInitKind::from_hresult(-2147467259); // E_FAIL 0x80004005
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.hr, -2147467259);
    }

    #[test]
    fn classify_preserves_hresult_in_error() {
        let hr = -2147418113i32; // CO_E_NOTINITIALIZED
        let err = ComInitKind::from_hresult(hr).unwrap_err();
        assert_eq!(err.hr, hr);
    }

    #[test]
    fn error_display_contains_hresult() {
        let err = ComInitError { hr: -2147467259 };
        let msg = err.to_string();
        assert!(msg.contains("HRESULT"), "message: {msg}");
        assert!(msg.contains("80004005"), "message: {msg}");
    }

    #[test]
    fn pwstr_to_string_returns_none_for_null() {
        let result = unsafe { pwstr_to_string_and_free(windows::core::PWSTR::null()) };
        assert!(result.is_none());
    }

    #[test]
    fn pwstr_to_string_converts_valid_utf16() {
        let text = "Hello, WASAPI!";
        let mut buffer: Vec<u16> = text.encode_utf16().collect();
        buffer.push(0);
        let pwstr = windows::core::PWSTR(buffer.as_mut_ptr());
        let result = unsafe { pwstr_to_string(pwstr) };
        assert_eq!(result.as_deref(), Some(text));
    }

    #[test]
    fn pwstr_to_string_handles_empty_string() {
        let mut buffer = [0u16];
        let pwstr = windows::core::PWSTR(buffer.as_mut_ptr());
        let result = unsafe { pwstr_to_string(pwstr) };
        assert_eq!(result.as_deref(), Some(""));
    }

    #[test]
    fn pwstr_to_string_and_free_with_allocated_memory() {
        use windows::Win32::System::Com::CoTaskMemAlloc;

        let text = "session-identifier-test";
        let utf16: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        let byte_len = utf16.len() * 2;

        unsafe {
            let ptr = CoTaskMemAlloc(byte_len) as *mut u16;
            assert!(!ptr.is_null(), "CoTaskMemAlloc failed");
            std::ptr::copy_nonoverlapping(utf16.as_ptr(), ptr, utf16.len());

            let pwstr = windows::core::PWSTR(ptr);
            let result = pwstr_to_string_and_free(pwstr);
            assert_eq!(result.as_deref(), Some(text));
        }
    }

    #[test]
    fn pwstr_to_string_and_free_with_empty_allocated() {
        use windows::Win32::System::Com::CoTaskMemAlloc;

        let utf16: Vec<u16> = vec![0];
        let byte_len = 2;

        unsafe {
            let ptr = CoTaskMemAlloc(byte_len) as *mut u16;
            assert!(!ptr.is_null(), "CoTaskMemAlloc failed");
            std::ptr::copy_nonoverlapping(utf16.as_ptr(), ptr, utf16.len());

            let pwstr = windows::core::PWSTR(ptr);
            let result = pwstr_to_string_and_free(pwstr);
            assert_eq!(result.as_deref(), Some(""));
        }
    }

    #[test]
    fn pwstr_to_string_and_free_with_unicode() {
        use windows::Win32::System::Com::CoTaskMemAlloc;

        let text = "日本語セッション\u{1F3B5}";
        let utf16: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        let byte_len = utf16.len() * 2;

        unsafe {
            let ptr = CoTaskMemAlloc(byte_len) as *mut u16;
            assert!(!ptr.is_null(), "CoTaskMemAlloc failed");
            std::ptr::copy_nonoverlapping(utf16.as_ptr(), ptr, utf16.len());

            let pwstr = windows::core::PWSTR(ptr);
            let result = pwstr_to_string_and_free(pwstr);
            assert_eq!(result.as_deref(), Some(text));
        }
    }

    #[test]
    #[ignore = "requires Windows audio device"]
    fn com_guard_first_init() {
        let guard = ComGuard::new().expect("first COM init should succeed");
        assert!(guard.should_uninit);
    }

    #[test]
    #[ignore = "requires Windows audio device"]
    fn com_guard_second_init_same_model() {
        let _guard1 = ComGuard::new().expect("first COM init");
        let guard2 = ComGuard::new().expect("second COM init (S_FALSE)");
        assert!(guard2.should_uninit);
    }

    #[test]
    #[ignore = "requires Windows audio device"]
    fn com_guard_drop_balancing() {
        {
            let _g1 = ComGuard::new().unwrap();
            let _g2 = ComGuard::new().unwrap();
        }
        let guard = ComGuard::new().expect("COM init after balanced drops");
        assert!(guard.should_uninit);
    }
}
