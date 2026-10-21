//! Level 1 integration tests for the diagram components (`terminal` feature).
//!
//! `Cargo.toml` sets `autotests = false`, so a file in this directory that is
//! not declared below never compiles.

// biscuit-terminal's layout-matrix scenarios and row builders, compiled here
// unchanged (`source-inputs` in Cargo.toml).
#[path = "../../../../biscuit-terminal/lib/tests/layout_matrix_support/mod.rs"]
mod layout_matrix_support;

mod graph_expression_parity;
mod layout_matrix;
mod mermaid_parity;
mod render_tree_diagram_hook;
