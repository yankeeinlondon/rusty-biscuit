//! Q3: wrong expected types must not compile. `.stderr` files are the
//! compiler messages (judged for readability in findings.md).

#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
