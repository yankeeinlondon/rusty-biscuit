//! Shared RAII helper for Windows COM initialization.
//!
//! Both the SFX player and the ducking backend need COM initialized on their
//! thread. This module centralizes the lifecycle so we correctly handle
//! `S_OK`, `S_FALSE`, and `RPC_E_CHANGED_MODE` without leaking init counts.

use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx, CoUninitialize};
use windows::core::HRESULT;

/// HRESULT for "COM already initialized with a different threading model".
const RPC_E_CHANGED_MODE: i32 = -2147417850_i32; // 0x80010106

/// RAII guard for COM initialization on the current thread.
///
/// Calls `CoInitializeEx(COINIT_MULTITHREADED)` on creation and
/// `CoUninitialize` on drop only when this call actually incremented
/// the init count (`S_OK` or `S_FALSE`).
///
/// If COM was already initialized with a different threading model
/// (`RPC_E_CHANGED_MODE`), the guard succeeds but skips uninit on drop.
#[derive(Debug)]
pub struct ComGuard {
    /// Whether we should call `CoUninitialize` on drop.
    should_uninit: bool,
}

impl ComGuard {
    /// Initializes COM for the current thread.
    ///
    /// ## Errors
    ///
    /// Returns the failing `HRESULT` as a string if COM initialization
    /// fails for a reason other than `RPC_E_CHANGED_MODE`.
    pub fn new() -> Result<Self, String> {
        let hr = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };

        match hr {
            Ok(()) => {
                // S_OK: first init on this thread — uninit on drop
                Ok(Self { should_uninit: true })
            }
            Err(e) if e.code() == HRESULT(1) => {
                // S_FALSE: already initialized with same model — uninit on drop
                Ok(Self { should_uninit: true })
            }
            Err(e) if e.code() == HRESULT(RPC_E_CHANGED_MODE) => {
                // Different threading model, but COM is usable — don't uninit
                Ok(Self {
                    should_uninit: false,
                })
            }
            Err(e) => Err(format!("COM initialization failed: {}", e)),
        }
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
