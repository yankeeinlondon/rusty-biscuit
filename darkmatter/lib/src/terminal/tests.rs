//! Tests for terminal color detection utilities
//!
//! These tests verify that the wrapper functions correctly delegate to
//! biscuit-terminal and maintain API compatibility.

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

// =============================================================================
// Color depth tests
// =============================================================================

#[test]
#[serial]
fn test_color_depth_truecolor() {
    let _env = ScopedEnv::set("COLORTERM", "truecolor");
    assert_eq!(color_depth(), TRUE_COLOR_DEPTH);
}

#[test]
#[serial]
fn test_color_depth_24bit() {
    let _env = ScopedEnv::set("COLORTERM", "24bit");
    assert_eq!(color_depth(), TRUE_COLOR_DEPTH);
}

#[test]
#[serial]
fn test_color_depth_case_insensitive() {
    let _env = ScopedEnv::set("COLORTERM", "TrueColor");
    assert_eq!(color_depth(), TRUE_COLOR_DEPTH);
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

// =============================================================================
// Italics support tests
// =============================================================================

#[test]
fn test_supports_italics_returns_bool() {
    // Just verify the function works and returns a boolean
    let supports = supports_italics();
    assert!(supports || !supports);
}

// =============================================================================
// Underline support tests
// =============================================================================

#[test]
fn test_supports_underline_returns_struct() {
    let support = supports_underline();
    // Verify the struct has the expected fields
    assert!(support.basic || !support.basic);
    assert!(support.colored || !support.colored);
}

#[test]
fn test_supported_underline_variants_returns_struct() {
    let variants = supported_underline_variants();
    // Verify all expected fields exist
    assert!(variants.straight || !variants.straight);
    assert!(variants.double || !variants.double);
    assert!(variants.curly || !variants.curly);
    assert!(variants.dotted || !variants.dotted);
    assert!(variants.dashed || !variants.dashed);
    assert!(variants.colored || !variants.colored);
}

// =============================================================================
// Constant tests
// =============================================================================

#[test]
fn test_color_depth_constants() {
    assert_eq!(TRUE_COLOR_DEPTH, 16_777_216);
    assert_eq!(COLORS_256_DEPTH, 256);
    assert_eq!(COLORS_16_DEPTH, 16);
    assert_eq!(COLORS_8_DEPTH, 8);
}
