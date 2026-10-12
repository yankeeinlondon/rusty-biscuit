//! (a) and (i): unauthored knobs do not compile; authored ones do. The
//! `.stderr` files beside each case are the real rustc messages.

#[test]
fn knob_surface() {
    let cases = trybuild::TestCases::new();
    cases.pass("tests/ui/pass_*.rs");
    cases.compile_fail("tests/ui/fail_*.rs");
}
