//! Shared test lifecycle helpers for the Rusty Biscuit workspace.
//!
//! ## Test level vocabulary
//!
//! The monorepo uses a unified Level 1 / 2 / 3 taxonomy for tests:
//!
//! - **Level 1 (`L1`)** — In-process or PTY-based tests. No real terminal,
//!   browser, device, or external service required.
//! - **Level 2 (`L2`)** — Run-in-real-terminal tests, headless-browser tests,
//!   and similar harness-backed tests that exercise a real rendering or
//!   tooling surface.
//! - **Level 3 (`L3`)** — OS-level keyboard or mouse injection tests
//!   (`cliclick`, `xdotool`, etc.).
//!
//! Use [`require_level!`] inside a test body to skip cleanly when the
//! requested level's harness is unavailable, or to hard-fail when the env
//! contract demands enforcement.
//!
//! ## Standard env contract
//!
//! - `BISCUIT_TEST_LEVEL=1|2|3` — runtime gate. Tests above this level skip
//!   cleanly. Defaults to `3` (run every level for which a harness is
//!   available).
//! - `BISCUIT_TEST_LEVEL_REQUIRED=2|3` — CI use. When set and the harness
//!   for the matching level is unavailable, [`require_level!`] panics
//!   instead of skipping. All-or-nothing: it cannot demand `tmux` while
//!   letting a GUI backend skip.
//! - `BISCUIT_TEST_REQUIRED_BACKENDS=tmux[,wezterm…]` — per-backend
//!   granularity for the same enforcement. See [`backend`].
//! - `RUN_LEVEL3=1` — explicit opt-in for Level 3 tests, on top of the
//!   normal availability probe. Level 3 tests skip when this is unset.
//!
//! ## Proving execution
//!
//! A provisioned backend that no test exercised is not evidence of anything.
//! When `BISCUIT_TEST_REQUIRED_BACKENDS` is set, every gate decision is
//! appended to a JSON Lines file (see [`evidence`]) and the `backend-proof`
//! binary asserts after the run that each required backend produced at least
//! one executed test.

pub mod backend;
pub mod evidence;

pub use backend::{
    BISCUIT_TEST_REQUIRED_BACKENDS, Backend, BackendParseError, HarnessSpec, RequiredBackendsError,
    parse_required_backends, required_backends,
};
pub use evidence::{
    BACKEND_EXECUTIONS_FILE, BISCUIT_JUNIT_STAGE_DIR, ExecutionDecision, ExecutionRecord,
    append_backend_execution, backend_executions_path, decision_counts, read_backend_executions,
    normalize_test_name, record_backend_execution, stage_dir, unproven_backends, workspace_root,
};

use std::env;
use std::ffi::{OsStr, OsString};
use std::sync::{Mutex, Once};

static INIT_TRACING: Once = Once::new();

/// Environment variable that sets the maximum test level to run.
///
/// Values: `1`, `2`, or `3`. Tests above this level skip cleanly. When
/// unset, defaults to `3` (run every level for which a harness is
/// available).
pub const BISCUIT_TEST_LEVEL: &str = "BISCUIT_TEST_LEVEL";

/// Environment variable that converts harness-unavailable skips into
/// hard failures for the matching level.
///
/// Values: `1`, `2`, or `3`. When set and the harness for that level is
/// unavailable, [`require_level!`] panics instead of skipping. Intended
/// for CI jobs that provision the relevant tooling.
pub const BISCUIT_TEST_LEVEL_REQUIRED: &str = "BISCUIT_TEST_LEVEL_REQUIRED";

/// Environment variable that opts in to Level 3 OS-keyboard-injection
/// tests. Set to `1` to enable.
pub const RUN_LEVEL3: &str = "RUN_LEVEL3";

/// Test-level taxonomy used across the rusty-biscuit workspace.
///
/// See the module-level documentation for the definition of each level
/// and the standard env contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    /// In-process or PTY-based tests.
    L1 = 1,
    /// Real-terminal, headless-browser, or other harness-backed tests.
    L2 = 2,
    /// OS-level keyboard/mouse injection tests.
    L3 = 3,
}

impl Level {
    /// Returns the numeric level (`1`, `2`, or `3`).
    #[must_use]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }
}

/// Decision returned by [`evaluate_level`] for a given test level and
/// harness availability check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LevelDecision {
    /// The test should run.
    Run,
    /// The test should skip cleanly. Carries a human-readable reason.
    Skip(String),
    /// The test should panic. Carries the panic message.
    Panic(String),
}

/// Decide whether a test at `level` should run, skip, or panic, given a
/// harness-availability result and a bare diagnostic label.
///
/// Equivalent to [`evaluate_harness`] with a label-only [`HarnessSpec`], which
/// carries no machine identity and so can never satisfy
/// [`BISCUIT_TEST_REQUIRED_BACKENDS`]. Prefer passing a [`Backend`].
#[must_use]
pub fn evaluate_level(level: Level, available: bool, harness_label: &str) -> LevelDecision {
    evaluate_harness(level, available, harness_label)
}

/// Decide whether a test at `level` should run, skip, or panic.
///
/// `available` is the result of the test's local probe (e.g.
/// `WezTermHarness::available()`). `harness` is either a [`Backend`] (stable
/// identity plus label) or a `&str` label for a requirement that has no
/// backend identity, such as a PTY device.
///
/// Decision rules, in order:
///
/// 1. If `BISCUIT_TEST_REQUIRED_BACKENDS` is set but malformed, return
///    `Panic(...)`. A typo is rejected even when the harness is present,
///    because a misspelled requirement can otherwise never fail again.
/// 2. If `BISCUIT_TEST_LEVEL=N` is set and `level > N`, return
///    `Skip("BISCUIT_TEST_LEVEL=N")`.
/// 3. If `level == L3` and `RUN_LEVEL3` is not `1`, return
///    `Skip("set RUN_LEVEL3=1")`.
/// 4. If `available == false`:
///     - If this harness's backend is in `BISCUIT_TEST_REQUIRED_BACKENDS`,
///       return `Panic(...)`.
///     - If `BISCUIT_TEST_LEVEL_REQUIRED == level`, return `Panic(...)`.
///     - Otherwise return `Skip("harness unavailable")`.
/// 5. Otherwise return `Run`.
///
/// ## Notes
///
/// Rules 2 and 3 precede the enforcement checks, so an explicit
/// `BISCUIT_TEST_LEVEL` ceiling still wins — configuring a required backend
/// out of the selected level range yields no `run` evidence and is caught by
/// `backend-proof verify` rather than by a per-test panic.
#[must_use]
pub fn evaluate_harness<'a>(
    level: Level,
    available: bool,
    harness: impl Into<HarnessSpec<'a>>,
) -> LevelDecision {
    let harness = harness.into();
    let harness_label = harness.label();

    let required_backends = match required_backends() {
        Ok(set) => set,
        Err(err) => return LevelDecision::Panic(err.to_string()),
    };

    if let Some(max) = read_level_env(BISCUIT_TEST_LEVEL)
        && level.as_u8() > max
    {
        return LevelDecision::Skip(format!(
            "skipping: {BISCUIT_TEST_LEVEL}={max} excludes level {}",
            level.as_u8()
        ));
    }

    if level == Level::L3 && !is_truthy(env::var(RUN_LEVEL3).ok().as_deref()) {
        return LevelDecision::Skip(format!(
            "skipping: set {RUN_LEVEL3}=1 to enable Level 3 ({harness_label})"
        ));
    }

    if !available {
        if let Some(backend) = harness.backend()
            && required_backends.contains(&backend)
        {
            return LevelDecision::Panic(format!(
                "{BISCUIT_TEST_REQUIRED_BACKENDS} lists `{}` but {harness_label} is unavailable. \
                 Provision the backend in this environment or drop it from the list.",
                backend.as_str()
            ));
        }

        if let Some(required) = read_level_env(BISCUIT_TEST_LEVEL_REQUIRED)
            && required == level.as_u8()
        {
            return LevelDecision::Panic(format!(
                "{BISCUIT_TEST_LEVEL_REQUIRED}={required} but {harness_label} is unavailable. \
                 Provision the required harness in this environment or unset the variable."
            ));
        }
        return LevelDecision::Skip(format!("skipping: requires {harness_label}"));
    }

    LevelDecision::Run
}

/// Record a gate decision when the requirement carries a [`Backend`].
///
/// Called by [`require_level!`]; exposed so bespoke gates can contribute the
/// same evidence.
pub fn record_decision(harness: HarnessSpec<'_>, test: &str, decision: ExecutionDecision) {
    if let Some(backend) = harness.backend() {
        record_backend_execution(backend, test, decision);
    }
}

fn read_level_env(key: &str) -> Option<u8> {
    let raw = env::var(key).ok()?;
    raw.trim().parse::<u8>().ok().filter(|n| (1..=3).contains(n))
}

fn is_truthy(value: Option<&str>) -> bool {
    matches!(value.map(str::trim), Some("1" | "true" | "TRUE" | "True"))
}

/// Gate a test body on the requested [`Level`].
///
/// `$level` is a [`Level`] value (`Level::L1` / `Level::L2` / `Level::L3`).
/// `$available` is an expression returning `bool` — typically the test's
/// harness availability probe (e.g. `WezTermHarness::available()`).
/// `$harness` is anything convertible into a [`HarnessSpec`]:
///
/// - a [`Backend`] — preferred. Carries the stable identity that
///   `BISCUIT_TEST_REQUIRED_BACKENDS` matches on and that execution evidence
///   is keyed by.
/// - a `&str` label — for requirements with no backend identity (a PTY
///   device, a permission). These can never be demanded per-backend.
///
/// The macro evaluates [`evaluate_harness`], records the decision when the
/// requirement names a backend, and either:
///
/// - emits a `skipping: ...` line to stderr and `return`s from the
///   enclosing function, or
/// - `panic!`s with the enforcement message, or
/// - falls through so the test body runs.
///
/// ## Examples
///
/// ```ignore
/// use test_toolkit::{require_level, Backend, Level};
///
/// #[test]
/// fn level2_renders_in_real_terminal() {
///     require_level!(Level::L2, WezTermHarness::available(), Backend::WezTerm);
///     // ... test body runs only when WezTerm is available.
/// }
/// ```
#[macro_export]
macro_rules! require_level {
    ($level:expr, $available:expr, $harness:expr $(,)?) => {{
        match $crate::decide_harness!($level, $available, $harness) {
            $crate::LevelDecision::Run => {}
            $crate::LevelDecision::Skip(msg) => {
                eprintln!("{}", msg);
                return;
            }
            $crate::LevelDecision::Panic(msg) => {
                panic!("{}", msg);
            }
        }
    }};
}

/// Evaluate a gate and record its execution evidence, yielding the
/// [`LevelDecision`] instead of acting on it.
///
/// [`require_level!`] is the normal entry point. Reach for this only where the
/// gate lives in a helper whose signature cannot be `return`ed from — a
/// `-> Option<T>` fixture builder, for example. Calling [`evaluate_harness`]
/// directly in such a helper skips the recording and leaves the backend
/// unproven even though its tests ran.
#[macro_export]
macro_rules! decide_harness {
    ($level:expr, $available:expr, $harness:expr $(,)?) => {{
        let __test_toolkit_harness: $crate::HarnessSpec<'_> =
            ::core::convert::Into::into($harness);
        let __test_toolkit_decision =
            $crate::evaluate_harness($level, $available, __test_toolkit_harness);
        $crate::record_decision(
            __test_toolkit_harness,
            $crate::current_test_name!(),
            match __test_toolkit_decision {
                $crate::LevelDecision::Run => $crate::ExecutionDecision::Run,
                $crate::LevelDecision::Skip(_) => $crate::ExecutionDecision::Skip,
                $crate::LevelDecision::Panic(_) => $crate::ExecutionDecision::Panic,
            },
        );
        __test_toolkit_decision
    }};
}

/// Fully qualified name of the enclosing function, for execution evidence.
///
/// ## Notes
///
/// Derived from `type_name` of a locally defined probe function, which is the
/// only stable way to learn the enclosing item's path — `module_path!` stops
/// at the module and there is no `function!` in `core`. Inside a closure the
/// path gains a `::{{closure}}` suffix.
#[macro_export]
macro_rules! current_test_name {
    () => {{
        fn __test_toolkit_probe() {}
        fn __test_toolkit_name_of<T>(_: T) -> &'static str {
            ::core::any::type_name::<T>()
        }
        let __test_toolkit_full = __test_toolkit_name_of(__test_toolkit_probe);
        $crate::normalize_test_name(
            __test_toolkit_full
                .strip_suffix("::__test_toolkit_probe")
                .unwrap_or(__test_toolkit_full),
        )
    }};
}

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
    use super::{
        BISCUIT_TEST_LEVEL, BISCUIT_TEST_LEVEL_REQUIRED, BISCUIT_TEST_REQUIRED_BACKENDS, EnvGuard,
        Level, LevelDecision, RUN_LEVEL3, evaluate_level, init_test_tracing,
    };
    use std::env;
    use std::sync::{Arc, Mutex};
    use tracing::Subscriber;
    use tracing_subscriber::Registry;
    use tracing_subscriber::layer::{Context, Layer};
    use tracing_subscriber::prelude::*;

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
    #[serial_test::serial]
    fn evaluate_level_runs_when_available_and_no_env() {
        let _g1 = EnvGuard::remove_safe(BISCUIT_TEST_LEVEL);
        let _g2 = EnvGuard::remove_safe(BISCUIT_TEST_LEVEL_REQUIRED);
        let _g3 = EnvGuard::remove_safe(RUN_LEVEL3);
        let _g4 = EnvGuard::remove_safe(BISCUIT_TEST_REQUIRED_BACKENDS);

        assert_eq!(
            evaluate_level(Level::L2, true, "WezTerm"),
            LevelDecision::Run,
        );
    }

    #[test]
    #[serial_test::serial]
    fn evaluate_level_skips_when_above_test_level_max() {
        let _g1 = EnvGuard::set_safe(BISCUIT_TEST_LEVEL, "1");
        let _g2 = EnvGuard::remove_safe(BISCUIT_TEST_LEVEL_REQUIRED);
        let _g3 = EnvGuard::remove_safe(RUN_LEVEL3);
        let _g4 = EnvGuard::remove_safe(BISCUIT_TEST_REQUIRED_BACKENDS);

        match evaluate_level(Level::L2, true, "WezTerm") {
            LevelDecision::Skip(msg) => assert!(msg.contains("BISCUIT_TEST_LEVEL=1")),
            other => panic!("expected Skip, got {other:?}"),
        }
    }

    #[test]
    #[serial_test::serial]
    fn evaluate_level_skips_when_harness_unavailable_and_not_required() {
        let _g1 = EnvGuard::remove_safe(BISCUIT_TEST_LEVEL);
        let _g2 = EnvGuard::remove_safe(BISCUIT_TEST_LEVEL_REQUIRED);
        let _g3 = EnvGuard::remove_safe(RUN_LEVEL3);
        let _g4 = EnvGuard::remove_safe(BISCUIT_TEST_REQUIRED_BACKENDS);

        match evaluate_level(Level::L2, false, "WezTerm") {
            LevelDecision::Skip(msg) => assert!(msg.contains("WezTerm")),
            other => panic!("expected Skip, got {other:?}"),
        }
    }

    #[test]
    #[serial_test::serial]
    fn evaluate_level_panics_when_required_level_matches_and_unavailable() {
        let _g1 = EnvGuard::remove_safe(BISCUIT_TEST_LEVEL);
        let _g2 = EnvGuard::set_safe(BISCUIT_TEST_LEVEL_REQUIRED, "2");
        let _g3 = EnvGuard::remove_safe(RUN_LEVEL3);
        let _g4 = EnvGuard::remove_safe(BISCUIT_TEST_REQUIRED_BACKENDS);

        match evaluate_level(Level::L2, false, "WezTerm") {
            LevelDecision::Panic(msg) => {
                assert!(msg.contains("BISCUIT_TEST_LEVEL_REQUIRED=2"));
                assert!(msg.contains("WezTerm"));
            }
            other => panic!("expected Panic, got {other:?}"),
        }
    }

    #[test]
    #[serial_test::serial]
    fn evaluate_level_does_not_panic_when_required_level_differs() {
        let _g1 = EnvGuard::remove_safe(BISCUIT_TEST_LEVEL);
        let _g2 = EnvGuard::set_safe(BISCUIT_TEST_LEVEL_REQUIRED, "3");
        let _g3 = EnvGuard::set_safe(RUN_LEVEL3, "1");
        let _g4 = EnvGuard::remove_safe(BISCUIT_TEST_REQUIRED_BACKENDS);

        // Level 2 unavailable, but REQUIRED=3 → should skip, not panic.
        match evaluate_level(Level::L2, false, "WezTerm") {
            LevelDecision::Skip(_) => {}
            other => panic!("expected Skip, got {other:?}"),
        }
    }

    #[test]
    #[serial_test::serial]
    fn evaluate_level_l3_skips_without_run_level3() {
        let _g1 = EnvGuard::remove_safe(BISCUIT_TEST_LEVEL);
        let _g2 = EnvGuard::remove_safe(BISCUIT_TEST_LEVEL_REQUIRED);
        let _g3 = EnvGuard::remove_safe(RUN_LEVEL3);
        let _g4 = EnvGuard::remove_safe(BISCUIT_TEST_REQUIRED_BACKENDS);

        match evaluate_level(Level::L3, true, "cliclick") {
            LevelDecision::Skip(msg) => assert!(msg.contains("RUN_LEVEL3")),
            other => panic!("expected Skip, got {other:?}"),
        }
    }

    #[test]
    #[serial_test::serial]
    fn evaluate_level_l3_runs_with_run_level3_and_available() {
        let _g1 = EnvGuard::remove_safe(BISCUIT_TEST_LEVEL);
        let _g2 = EnvGuard::remove_safe(BISCUIT_TEST_LEVEL_REQUIRED);
        let _g3 = EnvGuard::set_safe(RUN_LEVEL3, "1");
        let _g4 = EnvGuard::remove_safe(BISCUIT_TEST_REQUIRED_BACKENDS);

        assert_eq!(
            evaluate_level(Level::L3, true, "cliclick"),
            LevelDecision::Run,
        );
    }

    #[test]
    #[serial_test::serial]
    fn evaluate_level_test_level_max_takes_precedence_over_required() {
        // BISCUIT_TEST_LEVEL=1 forces a skip even when REQUIRED=2 would panic.
        let _g1 = EnvGuard::set_safe(BISCUIT_TEST_LEVEL, "1");
        let _g2 = EnvGuard::set_safe(BISCUIT_TEST_LEVEL_REQUIRED, "2");
        let _g3 = EnvGuard::remove_safe(RUN_LEVEL3);
        let _g4 = EnvGuard::remove_safe(BISCUIT_TEST_REQUIRED_BACKENDS);

        match evaluate_level(Level::L2, false, "WezTerm") {
            LevelDecision::Skip(_) => {}
            other => panic!("expected Skip (max wins), got {other:?}"),
        }
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
                self.0
                    .lock()
                    .unwrap()
                    .push(attrs.metadata().name().to_string());
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
