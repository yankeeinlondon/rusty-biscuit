//! Level 2 (`terminal-tests`) integration tests for `biscuit-terminal-cli`, one test binary per
//! execution contract (`2026-09-21-consolidated-test-binaries`).
//!
//! Each module was its own test target before the consolidation and keeps
//! that target's name, which is the first segment of every test path here.
//! `Cargo.toml` sets `autotests = false`, so a file in this directory that is
//! not declared below never compiles; `test_layout.rs` rejects one.

#[path = "../common/mod.rs"]
mod common;

mod diagrams;
mod level2_apple_terminal_prose;
mod level2_container_fenced_code;
mod level2_cursor_and_hygiene;
mod level2_image;
mod level2_layout;
mod level2_prose_styling;
mod level2_render_tree_style;
mod level2_status_block;
mod level2_style_everywhere_matrix;
mod prose_cells;
