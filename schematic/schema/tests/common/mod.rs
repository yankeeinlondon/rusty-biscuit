//! Helpers shared by the generated-client integration tests.

#![allow(dead_code)]

/// Saves the named environment variables, clears them, and restores the saved
/// values on drop, so a test sees only the credentials it sets itself.
///
/// The process environment is global: every test that creates a guard must
/// also be `#[serial_test::serial]`.
pub struct EnvGuard {
    saved: Vec<(&'static str, Option<std::ffi::OsString>)>,
}

impl EnvGuard {
    /// Saves and clears `names`.
    pub fn clear(names: &[&'static str]) -> Self {
        let saved = names
            .iter()
            .map(|name| (*name, std::env::var_os(name)))
            .collect();
        for name in names {
            // SAFETY: callers are `#[serial]`, so no other test thread reads or
            // writes the environment concurrently.
            unsafe { std::env::remove_var(name) };
        }
        Self { saved }
    }

    /// Sets `name` for the rest of the test; `name` must be one of the
    /// guarded variables so that it is restored.
    pub fn set(&self, name: &'static str, value: &str) {
        assert!(
            self.saved.iter().any(|(saved, _)| *saved == name),
            "{name} is not guarded"
        );
        // SAFETY: see `clear`.
        unsafe { std::env::set_var(name, value) };
    }

    /// Removes `name` for the rest of the test.
    pub fn remove(&self, name: &'static str) {
        // SAFETY: see `clear`.
        unsafe { std::env::remove_var(name) };
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (name, value) in &self.saved {
            // SAFETY: see `clear`.
            unsafe {
                match value {
                    Some(value) => std::env::set_var(name, value),
                    None => std::env::remove_var(name),
                }
            }
        }
    }
}
