//! `biscuit-terminal-cli` integration tests that need the `terminal-tests` feature,
//! one test binary per execution contract (`2026-09-21-consolidated-test-binaries`).
//!
//! The binary groups tests by feature contract, not by tier: tier selection
//! follows each test's path, so the unmarked tests in `diagrams` and
//! `prose_cells` run in Level 1 and the `level2_` modules in Level 2. Each
//! module keeps its former target's name except those two, which drop the
//! `level2_` prefix of `level2_diagrams` and `level2_prose_cells` so their
//! tests keep their tier. `Cargo.toml` sets `autotests = false`, so a file in
//! this directory that is not declared below never compiles; `test_layout.rs`
//! rejects one.

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
