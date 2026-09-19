//! A dynamically linked crate the fixture's test binaries resolve at run time.

/// The value a relocated test asserts on; reaching it at all proves the
/// dynamic library travelled inside the archive.
pub fn dylib_marker() -> &'static str {
    "archive-portability-dylib"
}
