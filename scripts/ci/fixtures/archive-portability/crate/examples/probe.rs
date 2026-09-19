//! An example target the package's tests spawn.
//!
//! `cargo nextest archive` builds lib, bin, and test targets and never
//! examples, so this only reaches a consumer because the producer recognizes
//! the declared `examples/…` include and builds it first.

fn main() {
    println!("archive-portability probe");
}
