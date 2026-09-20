//! The fixture's build sidecar.
//!
//! Stands in for the monorepo's real compile-time tools (the darkmatter `md`
//! fixture, the messenger desktop stubs, the harness broker): another package's
//! binary that a consumer must be handed, because it cannot build one.

fn main() {
    println!("archive-portability-sidecar");
}
