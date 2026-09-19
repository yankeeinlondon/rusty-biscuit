//! The fixture's non-test executable.
//!
//! A test spawns it and asserts on this line, which can only happen if the
//! archive carried a `bin` target as well as the test binaries.

fn main() {
    println!("archive-portability-tool {}", archive_portability::dylib_marker());
}
