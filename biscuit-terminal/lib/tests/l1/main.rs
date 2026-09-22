//! Level 1 integration tests for `biscuit-terminal`, one test binary per
//! execution contract (`2026-09-21-consolidated-test-binaries`).
//!
//! Each module was its own test target before the consolidation and keeps
//! that target's name, which is the first segment of every test path here.
//! `Cargo.toml` sets `autotests = false`, so a file in this directory that is
//! not declared below never compiles; `test_layout.rs` rejects one.

#[path = "../common/mod.rs"]
mod common;
#[path = "../layout_matrix_support/mod.rs"]
mod layout_matrix_support;

mod compose_parity;
mod filesystem_parity;
#[cfg(feature = "image")]
mod graph_expression_parity;
#[cfg(feature = "image")]
mod horizontal_rule_parity;
mod html_page_example;
mod inline_content_matrix;
mod integration;
mod layout_matrix;
mod level1_apple_terminal_prose;
mod level1_clipboard;
#[cfg(unix)]
mod level1_cursor;
mod level1_mode_2027;
#[cfg(unix)]
mod level1_osc_queries;
mod level1_terminal_init;
mod level1_terminal_osc_cache;
mod list_parity;
#[cfg(feature = "image")]
mod mermaid_parity;
mod metrics_tree_parity;
mod ordered_list_parity;
// Also compiled privately by each `*_parity` module (`#[path]`, with
// `clippy::duplicate_mod` allowed there): each former target ran its own copy
// of these helper tests, and those test paths are kept.
mod parity_helpers;
mod perf_gate;
mod prelude_exports;
mod progress_parity;
mod prose_cells_parity;
mod render_comparison;
mod render_tree_code_context;
mod render_tree_component_parity;
mod section_parity;
mod status_block_parity;
mod status_parity;
mod table_parity;
mod terminal_image_parity;
mod test_layout;
mod text_block_parity;
mod todo_parity;
mod tree_layout;
mod two_column_parity;
mod unordered_list_parity;
