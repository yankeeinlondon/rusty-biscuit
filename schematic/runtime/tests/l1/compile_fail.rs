//! Compile-fail cases, each checked against the compiler's diagnostic in its
//! `.stderr` file (R12, R2 groundwork).
//!
//! `slow_`: trybuild runs Cargo, which a test archive does not have, so the
//! L1 filter leaves this out and `just test` runs it through
//! `just test-compile-fail`.

use test_toolkit::EnvGuard;

#[test]
fn slow_compile_fail_cases_report_their_intended_diagnostic() {
    // A caching compiler wrapper rewrites source paths in diagnostics, which
    // would make the expected output depend on the host.
    let _wrapper = EnvGuard::set_safe("RUSTC_WRAPPER", "");
    let cases = biscuit_test_harness::manifest_dir!().join("tests/fixtures/compile_fail/*.rs");
    trybuild::TestCases::new().compile_fail(cases);
}
