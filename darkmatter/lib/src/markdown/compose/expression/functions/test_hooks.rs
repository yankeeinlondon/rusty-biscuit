//! Test hooks for expression-function dispatch: thread-local test functions
//! that dispatch consults before the built-in registry, and counters of how
//! often each handler was entered.
//!
//! Compiled under `cfg(test)` or the `effects-instrumentation` feature. A test
//! function binds and has its result checked exactly like a built-in, so a
//! test can prove that a call which does not bind never enters a handler, and
//! that a lazy operand is never evaluated, by counting entries.
//!
//! ## Examples
//!
//! ```
//! use std::sync::Arc;
//! use darkmatter::markdown::compose::expression::call_binding::FunctionRegistration;
//! use darkmatter::markdown::compose::expression::{evaluate, function_test_hooks, parse};
//! # struct Empty;
//! # impl darkmatter::markdown::compose::expression::EvaluationLookup for Empty {
//! #     fn get(&self, _: &str) -> Option<serde_json::Value> { None }
//! # }
//!
//! let registration =
//!     FunctionRegistration::builder("twice", "function(parameters(x: number); returns(number))").build().unwrap();
//! let _guard = function_test_hooks::register(
//!     registration,
//!     Arc::new(|call| Ok(serde_json::json!(call.args().number(0).unwrap() * 2.0))),
//! );
//! function_test_hooks::reset_handler_entries();
//! assert_eq!(evaluate(&parse("twice('4')").unwrap(), &Empty).unwrap(), serde_json::json!(8.0));
//! assert!(evaluate(&parse("twice('pear')").unwrap(), &Empty).is_err());
//! assert_eq!(function_test_hooks::handler_entries("twice"), 1);
//! ```

use std::cell::RefCell;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::Arc;

use serde_json::Value;

use super::call_error;
use crate::markdown::compose::expression::ExpressionError;
use crate::markdown::compose::expression::call_binding::{BoundCall, FunctionRegistration};

/// A test function's handler: computes from the bound call; an `Err` is a
/// domain failure.
pub type TestHandler = Arc<dyn Fn(&BoundCall<'_>) -> Result<Value, String> + Send + Sync>;

pub(super) struct TestFunction {
    registration: FunctionRegistration,
    handler: TestHandler,
}

thread_local! {
    static FUNCTIONS: RefCell<HashMap<String, Rc<TestFunction>>> = RefCell::new(HashMap::new());
    static ENTRIES: RefCell<HashMap<String, usize>> = RefCell::new(HashMap::new());
}

/// Removes its test function when dropped.
#[must_use = "the test function is removed when the guard drops"]
pub struct TestFunctionGuard {
    name: String,
    /// Registration is thread-local, so the guard stays on its thread.
    _thread: PhantomData<*const ()>,
}

impl Drop for TestFunctionGuard {
    fn drop(&mut self) {
        FUNCTIONS.with(|functions| functions.borrow_mut().remove(&self.name));
    }
}

/// Registers a test-only function for the current thread under its
/// registration's name (matched case-insensitively). Dispatch consults it
/// before the built-in registry, so it may shadow a built-in eager function;
/// it is always called eagerly.
pub fn register(registration: FunctionRegistration, handler: TestHandler) -> TestFunctionGuard {
    let name = registration.name().to_ascii_lowercase();
    FUNCTIONS.with(|functions| {
        functions
            .borrow_mut()
            .insert(name.clone(), Rc::new(TestFunction { registration, handler }))
    });
    TestFunctionGuard {
        name,
        _thread: PhantomData,
    }
}

/// How many times the handler of `name` (canonical, built-in or test) was
/// entered on the current thread since the last reset.
pub fn handler_entries(name: &str) -> usize {
    ENTRIES.with(|entries| entries.borrow().get(&name.to_ascii_lowercase()).copied().unwrap_or(0))
}

/// Resets every handler-entry counter on the current thread.
pub fn reset_handler_entries() {
    ENTRIES.with(|entries| entries.borrow_mut().clear());
}

pub(super) fn record_entry(name: &str) {
    ENTRIES.with(|entries| *entries.borrow_mut().entry(name.to_ascii_lowercase()).or_default() += 1);
}

impl TestFunction {
    pub(super) fn registration(&self) -> &FunctionRegistration {
        &self.registration
    }
}

/// The test function registered under the lowercased `name` on this thread.
pub(super) fn lookup(name: &str) -> Option<Rc<TestFunction>> {
    FUNCTIONS.with(|functions| functions.borrow().get(name).cloned())
}

/// Calls a test function through the same steps as a built-in pure handler:
/// bind, count the entry, compute, check the return.
pub(super) fn call(function: &TestFunction, name: &str, args: &[Value]) -> Result<Value, ExpressionError> {
    let registration = &function.registration;
    let call = registration.bind_call(args).map_err(call_error)?;
    record_entry(registration.name());
    let value = (function.handler)(&call).map_err(|message| ExpressionError::Other {
        function: name.to_string(),
        message,
    })?;
    registration.check_return(&call, &value).map_err(call_error)?;
    Ok(value)
}
