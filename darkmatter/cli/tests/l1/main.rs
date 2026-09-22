//! Level 1 integration tests for `darkmatter-cli`, one test binary per
//! execution contract (`2026-09-21-consolidated-test-binaries`).
//!
//! Each module was its own test target before the consolidation and keeps
//! that target's name, which is the first segment of every test path here.
//! `Cargo.toml` sets `autotests = false`, so a file in this directory that is
//! not declared below never compiles; `test_layout.rs` rejects one.

#[path = "../common/mod.rs"]
mod common;

mod clean;
mod clean_frontmatter;
mod clean_json;
mod clean_schema;
mod code_block;
mod compose_array_rendering;
mod compose_base_schema;
mod compose_basic;
mod compose_interpolation;
mod compose_layout;
mod compose_page_blocks;
mod compose_perf;
mod compose_refs_and_missing;
mod compose_remote_caching;
mod compose_schema;
mod compose_schema_file_rewrite;
mod compose_shell;
mod compose_state_set;
mod compose_terminal_detection;
mod compose_transclusion;
mod delta;
mod get_set_rm;
mod graph;
mod hash;
mod hash_directory;
mod hash_kind_save_diff;
mod help;
mod layout_alignment;
mod layout_fill;
mod layout_flags;
mod layout_style_frontmatter;
mod md_process_fixture;
mod render_basic;
mod rm;
mod schema_about;
mod schema_detect;
mod schema_triggers;
mod schema_validate;
mod schema_validate_baseline;
mod spawn_site_guard;
mod test_layout;
mod toc;
mod validate_refs;
