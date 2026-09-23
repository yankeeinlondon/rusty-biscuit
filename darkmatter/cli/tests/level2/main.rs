//! Level 2 (`terminal-tests`) integration tests for `darkmatter-cli`, one test binary per
//! execution contract (`2026-09-21-consolidated-test-binaries`).
//!
//! Each module was its own test target before the consolidation and keeps
//! that target's name, which is the first segment of every test path here.
//! `Cargo.toml` sets `autotests = false`, so a file in this directory that is
//! not declared below never compiles; `test_layout.rs` rejects one.

#[path = "../common/mod.rs"]
mod common;

mod harness_integrity;
mod level2_code_block_styling;
mod level2_disclosure_blocks;
mod level2_errors;
mod level2_frontmatter_images;
mod level2_frontmatter_tables;
mod level2_horizontal_rules;
mod level2_layout_dimensions;
mod level2_ordered_lists;
mod level2_schema_about;
mod level2_schema_validate;
