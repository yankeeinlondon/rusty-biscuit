//! UI tests for api-macros using trybuild.
//!
//! These tests verify that the proc macros:
//! 1. Accept valid input and generate compilable code (pass tests)
//! 2. Reject invalid input with helpful error messages (fail tests)
//!
//! NOTE: These tests are currently disabled because they require the `api`
//! crate to be available, which creates circular dependencies in trybuild's
//! isolated test environment. Full integration testing should be done in
//! the `api` crate's tests which can properly configure dependencies.

#[test]
#[ignore = "requires api crate - run integration tests in api crate instead"]
fn ui_tests() {
    let t = trybuild::TestCases::new();

    // Tests that should compile successfully
    t.pass("tests/ui/pass/*.rs");

    // Tests that should fail with specific error messages
    t.compile_fail("tests/ui/fail/*.rs");
}
