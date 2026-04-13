//! Shared test utilities for environment variable manipulation.
//!
//! Due to `std::env::set_var` and `std::env::remove_var` being unsafe in
//! Rust 2024 (they are not thread-safe), and Cargo running tests in parallel
//! by default, all environment-modifying tests must use a shared mutex to
//! prevent race conditions across modules.

use std::ffi::{OsStr, OsString};
use std::sync::Mutex;

pub(crate) static ENV_MUTEX: Mutex<()> = Mutex::new(());

pub(crate) struct ScopedEnv {
    vars: Vec<(String, Option<OsString>)>,
}

impl ScopedEnv {
    pub(crate) fn new() -> Self {
        Self { vars: Vec::new() }
    }

    pub(crate) fn set(&mut self, key: &str, value: &str) -> &mut Self {
        let original = std::env::var_os(key);
        self.vars.push((key.to_string(), original));
        unsafe { std::env::set_var(key, value) };
        self
    }

    pub(crate) fn set_os(&mut self, key: &str, value: &OsStr) -> &mut Self {
        let original = std::env::var_os(key);
        self.vars.push((key.to_string(), original));
        unsafe { std::env::set_var(key, value) };
        self
    }

    pub(crate) fn remove(&mut self, key: &str) -> &mut Self {
        let original = std::env::var_os(key);
        self.vars.push((key.to_string(), original));
        unsafe { std::env::remove_var(key) };
        self
    }
}

impl Drop for ScopedEnv {
    fn drop(&mut self) {
        for (key, original) in self.vars.iter().rev() {
            match original {
                Some(value) => unsafe { std::env::set_var(key, value) },
                None => unsafe { std::env::remove_var(key) },
            }
        }
    }
}
