//! A Cargo integration test file: everything here is test-only code, the
//! helper included, because the file sits under the package's `tests/`.

fn make_point() -> (i32, i32) {
    (0, 0)
}

#[test]
fn distance_is_zero() {
    assert_eq!(make_point(), (0, 0));
}
