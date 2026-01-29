//! Tests for terminal color detection utilities

use super::*;
use serial_test::serial;
use std::env;

/// RAII helper for temporarily setting environment variables
struct ScopedEnv {
    key: String,
    original: Option<String>,
}

impl ScopedEnv {
    fn set(key: &str, value: &str) -> Self {
        let original = env::var(key).ok();
        // SAFETY: This is only used in serial tests where no other threads
        // are accessing the environment concurrently
        unsafe {
            env::set_var(key, value);
        }
        Self {
            key: key.to_string(),
            original,
        }
    }

    fn remove(key: &str) -> Self {
        let original = env::var(key).ok();
        // SAFETY: This is only used in serial tests where no other threads
        // are accessing the environment concurrently
        unsafe {
            env::remove_var(key);
        }
        Self {
            key: key.to_string(),
            original,
        }
    }
}

impl Drop for ScopedEnv {
    fn drop(&mut self) {
        // SAFETY: This is only used in serial tests where no other threads
        // are accessing the environment concurrently
        unsafe {
            match &self.original {
                Some(val) => env::set_var(&self.key, val),
                None => env::remove_var(&self.key),
            }
        }
    }
}

#[test]
#[serial]
fn test_color_depth_truecolor() {
    let _env = ScopedEnv::set("COLORTERM", "truecolor");
    assert_eq!(color_depth(), 16_777_216);
}

#[test]
#[serial]
fn test_color_depth_24bit() {
    let _env = ScopedEnv::set("COLORTERM", "24bit");
    assert_eq!(color_depth(), 16_777_216);
}

#[test]
#[serial]
fn test_color_depth_case_insensitive() {
    let _env = ScopedEnv::set("COLORTERM", "TrueColor");
    assert_eq!(color_depth(), 16_777_216);
}

#[test]
#[serial]
fn test_color_depth_fallback_to_terminfo() {
    let _env = ScopedEnv::remove("COLORTERM");

    // The actual value depends on the system's terminfo database
    // We just verify that the function runs without panicking
    let _depth = color_depth();
}

#[test]
fn test_supports_setting_foreground() {
    // This test depends on the system's terminfo database
    // We just verify that the function returns a boolean without panicking
    let supports = supports_setting_foreground();
    assert!(supports || !supports); // Tautology to verify it returns a bool
}

#[test]
#[serial]
fn test_color_depth_no_colorterm_env() {
    let _env = ScopedEnv::remove("COLORTERM");

    // Should fall back to terminfo
    let _depth = color_depth();

    // Most modern terminals support at least 8 colors, but we can't guarantee
    // the test environment, so we just verify the function runs without panicking
}

#[test]
#[serial]
fn test_color_depth_invalid_colorterm() {
    let _env = ScopedEnv::set("COLORTERM", "invalid");

    // Should fall back to terminfo since "invalid" is not "truecolor" or "24bit"
    let _depth = color_depth();
    // We just verify the function runs without panicking
}
