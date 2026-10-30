//! Level 1 integration tests for `simplified-schema`: one test binary.
//!
//! `Cargo.toml` sets `autotests = false`, so a file in this directory that is
//! not declared below never compiles; `test_layout.rs` rejects one.

mod expression_syntax;
mod foundations;
mod test_layout;
