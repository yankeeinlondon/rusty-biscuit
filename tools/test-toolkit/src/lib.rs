//! Shared test lifecycle helpers for the Rusty Biscuit workspace.

use std::env;
use std::ffi::{OsStr, OsString};
use std::sync::{Mutex, Once};

static INIT_TRACING: Once = Once::new();

/// Global lock that serializes process-environment mutation inside
/// [`EnvGuard`]'s safe constructors.
///
/// This does **not** make `std::env::var` safe to call concurrently with a
/// live guard, but it prevents two `EnvGuard` instances from racing each
/// other during creation or drop, which is the most common source of test
/// flakiness.
static ENV_GUARD_LOCK: Mutex<()> = Mutex::new(());

/// Initialize a tracing subscriber for tests.
///
/// Configures a subscriber at `INFO` level so that [`trace_phase!`] spans are
/// visible. The `RUST_LOG` environment variable can override the default.
///
/// Multiple calls in the same test binary are idempotent.
pub fn init_test_tracing() {
    INIT_TRACING.call_once(|| {
        let filter = tracing_subscriber::EnvFilter::builder()
            .with_default_directive(tracing::Level::INFO.into())
            .from_env_lossy();

        let _ = tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_test_writer()
            .try_init();
    });
}

/// Enter an INFO-level tracing span for a setup/body/teardown phase.
///
/// The macro evaluates to the wrapped block's result, so it can be used around
/// statement blocks and expression-producing fixture setup.
///
/// ## Level Warning
///
/// Spans are created at `INFO` level. If you do not see output, either:
///
/// - Call [`init_test_tracing()`] at the start of your test binary (sets an
///   `INFO` default and respects `RUST_LOG`).
/// - Set the `RUST_LOG` environment variable to at least `INFO`, e.g.
///   `RUST_LOG=test_toolkit=info` or `RUST_LOG=info`.
///
/// Because the default tracing subscriber is typically `ERROR` level, spans
/// from this macro are invisible unless one of the above steps is taken.
#[macro_export]
macro_rules! trace_phase {
    ($phase:literal, $block:block) => {{
        let __test_toolkit_span = ::tracing::span!(::tracing::Level::INFO, $phase);
        let __test_toolkit_guard = __test_toolkit_span.enter();
        let __test_toolkit_result = $block;
        drop(__test_toolkit_guard);
        __test_toolkit_result
    }};
}

/// Restores a process environment variable when dropped.
///
/// Rust 2024 marks environment mutation as unsafe because the process
/// environment is global state. Use this guard only in tests that serialize env
/// access, such as tests annotated with `#[serial_test::serial]`.
#[derive(Debug)]
pub struct EnvGuard {
    key: OsString,
    previous: PreviousEnvValue,
}

#[derive(Debug)]
enum PreviousEnvValue {
    Set(OsString),
    Unset,
}

impl EnvGuard {
    /// Set an environment variable for the guard lifetime.
    ///
    /// When the guard is dropped, the variable is restored to its previous value
    /// or removed if it was previously unset.
    ///
    /// This safe variant acquires an internal mutex during creation and drop,
    /// so it can be used without `#[serial_test::serial]` when the test suite
    /// does not otherwise touch the process environment. For heavy concurrent
    /// test suites, `#[serial_test::serial]` is still recommended to avoid
    /// contention on the internal lock.
    pub fn set_safe<K, V>(key: K, value: V) -> Self
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        let _lock = ENV_GUARD_LOCK.lock().unwrap();
        let key = key.as_ref().to_os_string();
        let previous = previous_value(&key);
        // SAFETY: We hold the global ENV_GUARD_LOCK, serializing all
        // test-toolkit env mutations.
        unsafe {
            env::set_var(&key, value);
        }
        drop(_lock);

        Self { key, previous }
    }

    /// Remove an environment variable for the guard lifetime.
    ///
    /// When the guard is dropped, the variable is restored to its previous value
    /// or left unset if it was previously unset.
    ///
    /// This safe variant acquires an internal mutex during creation and drop,
    /// so it can be used without `#[serial_test::serial]` when the test suite
    /// does not otherwise touch the process environment.
    pub fn remove_safe<K>(key: K) -> Self
    where
        K: AsRef<OsStr>,
    {
        let _lock = ENV_GUARD_LOCK.lock().unwrap();
        let key = key.as_ref().to_os_string();
        let previous = previous_value(&key);
        // SAFETY: We hold the global ENV_GUARD_LOCK, serializing all
        // test-toolkit env mutations.
        unsafe {
            env::remove_var(&key);
        }
        drop(_lock);

        Self { key, previous }
    }

    /// Set an environment variable for the guard lifetime (unsafe).
    ///
    /// When the guard is dropped, the variable is restored to its previous value
    /// or removed if it was previously unset.
    ///
    /// # Safety
    ///
    /// The caller must ensure no other thread reads or writes the process
    /// environment while the guard is created, alive, or being dropped. In test
    /// code, use `#[serial_test::serial]` or an equivalent process-wide
    /// serialization strategy.
    pub unsafe fn set<K, V>(key: K, value: V) -> Self
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        let key = key.as_ref().to_os_string();
        let previous = previous_value(&key);
        // SAFETY: The caller upholds the process-environment synchronization
        // requirement documented on this function.
        unsafe {
            env::set_var(&key, value);
        }

        Self { key, previous }
    }

    /// Remove an environment variable for the guard lifetime (unsafe).
    ///
    /// When the guard is dropped, the variable is restored to its previous value
    /// or left unset if it was previously unset.
    ///
    /// # Safety
    ///
    /// The caller must ensure no other thread reads or writes the process
    /// environment while the guard is created, alive, or being dropped. In test
    /// code, use `#[serial_test::serial]` or an equivalent process-wide
    /// serialization strategy.
    pub unsafe fn remove<K>(key: K) -> Self
    where
        K: AsRef<OsStr>,
    {
        let key = key.as_ref().to_os_string();
        let previous = previous_value(&key);
        // SAFETY: The caller upholds the process-environment synchronization
        // requirement documented on this function.
        unsafe {
            env::remove_var(&key);
        }

        Self { key, previous }
    }

    /// Return the guarded environment variable name.
    #[must_use]
    pub fn key(&self) -> &OsStr {
        &self.key
    }

    /// Return whether the variable existed before this guard changed it.
    #[must_use]
    pub fn had_previous_value(&self) -> bool {
        matches!(self.previous, PreviousEnvValue::Set(_))
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        // Acquire the global lock during restoration so that safe constructors
        // remain race-free even if multiple tests drop guards concurrently.
        let _lock = ENV_GUARD_LOCK.lock().unwrap();
        match &self.previous {
            PreviousEnvValue::Set(value) => {
                // SAFETY: We hold the global ENV_GUARD_LOCK, serializing all
                // test-toolkit env mutations.
                unsafe {
                    env::set_var(&self.key, value);
                }
            }
            PreviousEnvValue::Unset => {
                // SAFETY: We hold the global ENV_GUARD_LOCK, serializing all
                // test-toolkit env mutations.
                unsafe {
                    env::remove_var(&self.key);
                }
            }
        }
    }
}

fn previous_value(key: &OsStr) -> PreviousEnvValue {
    match env::var_os(key) {
        Some(value) => PreviousEnvValue::Set(value),
        None => PreviousEnvValue::Unset,
    }
}

#[cfg(test)]
mod tests {
    use super::{init_test_tracing, EnvGuard};
    use std::env;
    use std::sync::{Arc, Mutex};
    use tracing::Subscriber;
    use tracing_subscriber::layer::{Context, Layer};
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::Registry;

    const RESTORE_KEY: &str = "TEST_TOOLKIT_ENV_GUARD_RESTORE";
    const REMOVE_KEY: &str = "TEST_TOOLKIT_ENV_GUARD_REMOVE";
    const NESTED_KEY: &str = "TEST_TOOLKIT_ENV_GUARD_NESTED";
    const SAFE_SET_RESTORE_KEY: &str = "TEST_TOOLKIT_SAFE_SET_RESTORE";
    const SAFE_REMOVE_KEY: &str = "TEST_TOOLKIT_SAFE_REMOVE";
    const SAFE_SET_UNSET_KEY: &str = "TEST_TOOLKIT_SAFE_SET_UNSET";

    #[test]
    fn trace_phase_returns_wrapped_expression_result() {
        let value = trace_phase!("expression_return", { 40 + 2 });

        assert_eq!(value, 42);
    }

    #[test]
    #[serial_test::serial]
    fn env_guard_restores_previous_value() {
        // SAFETY: This test is serialized and confines env access to this key.
        unsafe {
            env::set_var(RESTORE_KEY, "before");
        }

        {
            // SAFETY: This test is serialized and confines env access to this key.
            let guard = unsafe { EnvGuard::set(RESTORE_KEY, "during") };
            assert_eq!(guard.key(), RESTORE_KEY);
            assert!(guard.had_previous_value());
            assert_eq!(env::var(RESTORE_KEY).as_deref(), Ok("during"));
        }

        assert_eq!(env::var(RESTORE_KEY).as_deref(), Ok("before"));
        // SAFETY: This test is serialized and confines env access to this key.
        unsafe {
            env::remove_var(RESTORE_KEY);
        }
    }

    #[test]
    #[serial_test::serial]
    fn env_guard_removes_variable_that_was_previously_unset() {
        // SAFETY: This test is serialized and confines env access to this key.
        unsafe {
            env::remove_var(REMOVE_KEY);
        }

        {
            // SAFETY: This test is serialized and confines env access to this key.
            let guard = unsafe { EnvGuard::set(REMOVE_KEY, "during") };
            assert_eq!(guard.key(), REMOVE_KEY);
            assert!(!guard.had_previous_value());
            assert_eq!(env::var(REMOVE_KEY).as_deref(), Ok("during"));
        }

        assert!(env::var_os(REMOVE_KEY).is_none());
    }

    #[test]
    #[serial_test::serial]
    fn env_guard_remove_restores_previous_value() {
        // SAFETY: This test is serialized and confines env access to this key.
        unsafe {
            env::set_var(REMOVE_KEY, "before");
        }

        {
            // SAFETY: This test is serialized and confines env access to this key.
            let guard = unsafe { EnvGuard::remove(REMOVE_KEY) };
            assert_eq!(guard.key(), REMOVE_KEY);
            assert!(guard.had_previous_value());
            assert!(env::var_os(REMOVE_KEY).is_none());
        }

        assert_eq!(env::var(REMOVE_KEY).as_deref(), Ok("before"));
        // SAFETY: This test is serialized and confines env access to this key.
        unsafe {
            env::remove_var(REMOVE_KEY);
        }
    }

    #[test]
    #[serial_test::serial]
    fn nested_env_guards_restore_in_stack_order() {
        // SAFETY: This test is serialized and confines env access to this key.
        unsafe {
            env::set_var(NESTED_KEY, "outer-before");
        }

        {
            // SAFETY: This test is serialized and confines env access to this key.
            let _outer = unsafe { EnvGuard::set(NESTED_KEY, "outer") };
            assert_eq!(env::var(NESTED_KEY).as_deref(), Ok("outer"));

            {
                // SAFETY: This test is serialized and confines env access to this key.
                let _inner = unsafe { EnvGuard::set(NESTED_KEY, "inner") };
                assert_eq!(env::var(NESTED_KEY).as_deref(), Ok("inner"));
            }

            assert_eq!(env::var(NESTED_KEY).as_deref(), Ok("outer"));
        }

        assert_eq!(env::var(NESTED_KEY).as_deref(), Ok("outer-before"));
        // SAFETY: This test is serialized and confines env access to this key.
        unsafe {
            env::remove_var(NESTED_KEY);
        }
    }

    #[test]
    fn env_guard_safe_set_restores_previous_value() {
        // set_safe does not require #[serial_test::serial] because it
        // acquires the internal ENV_GUARD_LOCK.
        unsafe {
            env::set_var(SAFE_SET_RESTORE_KEY, "before");
        }

        {
            let guard = EnvGuard::set_safe(SAFE_SET_RESTORE_KEY, "during");
            assert_eq!(guard.key(), SAFE_SET_RESTORE_KEY);
            assert!(guard.had_previous_value());
            assert_eq!(env::var(SAFE_SET_RESTORE_KEY).as_deref(), Ok("during"));
        }

        assert_eq!(env::var(SAFE_SET_RESTORE_KEY).as_deref(), Ok("before"));
        unsafe {
            env::remove_var(SAFE_SET_RESTORE_KEY);
        }
    }

    #[test]
    fn env_guard_safe_remove_restores_previous_value() {
        unsafe {
            env::set_var(SAFE_REMOVE_KEY, "before");
        }

        {
            let guard = EnvGuard::remove_safe(SAFE_REMOVE_KEY);
            assert_eq!(guard.key(), SAFE_REMOVE_KEY);
            assert!(guard.had_previous_value());
            assert!(env::var_os(SAFE_REMOVE_KEY).is_none());
        }

        assert_eq!(env::var(SAFE_REMOVE_KEY).as_deref(), Ok("before"));
        unsafe {
            env::remove_var(SAFE_REMOVE_KEY);
        }
    }

    #[test]
    fn env_guard_safe_set_restores_unset_variable() {
        unsafe {
            env::remove_var(SAFE_SET_UNSET_KEY);
        }

        {
            let guard = EnvGuard::set_safe(SAFE_SET_UNSET_KEY, "during");
            assert!(!guard.had_previous_value());
            assert_eq!(env::var(SAFE_SET_UNSET_KEY).as_deref(), Ok("during"));
        }

        assert!(env::var_os(SAFE_SET_UNSET_KEY).is_none());
    }

    #[test]
    fn init_test_tracing_is_idempotent() {
        init_test_tracing();
        init_test_tracing(); // should not panic
    }

    #[test]
    fn trace_phase_span_is_emitted_after_init() {
        init_test_tracing();

        let captured_spans = Arc::new(Mutex::new(Vec::new()));

        struct CaptureLayer(Arc<Mutex<Vec<String>>>);

        impl<S: Subscriber> Layer<S> for CaptureLayer {
            fn on_new_span(
                &self,
                attrs: &tracing::span::Attributes<'_>,
                _id: &tracing::span::Id,
                _ctx: Context<'_, S>,
            ) {
                self.0.lock().unwrap().push(attrs.metadata().name().to_string());
            }
        }

        let subscriber = Registry::default().with(CaptureLayer(captured_spans.clone()));

        tracing::subscriber::with_default(subscriber, || {
            let result = trace_phase!("verify", { 1 + 1 });
            assert_eq!(result, 2);
        });

        let spans = captured_spans.lock().unwrap();
        assert!(spans.contains(&"verify".to_string()));
    }
}
