//! Test: Basic API definition with derive macro compiles successfully.
//!
//! NOTE: This test only verifies that the macro expands without errors.
//! Full functionality testing requires the `api` crate which creates
//! circular dependencies in trybuild tests.

use api_macros::RestApi;

#[derive(RestApi)]
#[api(base_url = "https://api.example.com")]
pub struct BasicApi;

fn main() {
    // Test passes if compilation succeeds
    // Actual functionality tested in integration tests
}
