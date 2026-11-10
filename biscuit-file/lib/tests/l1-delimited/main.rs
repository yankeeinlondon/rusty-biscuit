//! Level 1 (`delimited`) integration tests for `biscuit-file`: the strict
//! CSV/TSV reader, which only exists with the opt-in `delimited` feature.
//!
//! `Cargo.toml` sets `autotests = false`, so a file in this directory that is
//! not declared below never compiles; the `l1` binary's `test_layout.rs`
//! rejects one.

mod delimited_locations;
mod delimited_reader;
