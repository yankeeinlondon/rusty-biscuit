//! UI tests for api-macros using trybuild.
//!
//! These tests verify that the proc macros:
//! 1. Accept valid input and generate compilable code (pass tests)
//! 2. Reject invalid input with helpful error messages (fail tests)

#[test]
fn ui_tests() {
    let t = trybuild::TestCases::new();

    // Tests that should compile successfully
    t.pass("tests/ui/pass/*.rs");

    // Tests that should fail with specific error messages
    t.compile_fail("tests/ui/fail/*.rs");
}
