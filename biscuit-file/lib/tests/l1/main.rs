//! Level 1 integration tests for `biscuit-file`, one test binary per
//! execution contract (`2026-09-21-consolidated-test-binaries`).
//!
//! Each module was its own test target before the consolidation and keeps
//! that target's name, which is the first segment of every test path here.
//! `Cargo.toml` sets `autotests = false`, so a file in this directory that is
//! not declared below never compiles; `test_layout.rs` rejects one.
//!
//! Modules that resolve references or convert other formats are gated on
//! their features, so the binary also builds with only
//! `file-reference-syntax,yaml`; `reference_syntax_only` runs there.

#[cfg(feature = "file-reference")]
mod boundary_containment;
#[cfg(feature = "file-reference")]
mod completion_round_trip;
#[cfg(feature = "file-reference")]
mod detailed_resolution;
#[cfg(feature = "file-reference")]
mod file_tree;
#[cfg(feature = "file-reference")]
mod finalized_reference_resolution;
mod data_locations;
mod format_detection;
#[cfg(feature = "file-reference")]
mod glob_reference;
#[cfg(feature = "file-reference")]
mod implicit_relative;
mod json_lines;
#[cfg(feature = "file-reference")]
mod magic_local_roots;
mod path_lookup_guard;
mod parse_count;
#[cfg(feature = "file-reference")]
mod portable_path;
#[cfg(feature = "file-reference")]
mod precedence_flip;
mod reference_grammar;
mod reference_syntax_only;
mod repository_scope_catalog;
#[cfg(feature = "file-reference")]
mod resolution_context;
#[cfg(all(feature = "toml", feature = "json5"))]
mod round_trip;
mod source_position;
mod span_compat;
mod structural_validation;
mod test_layout;
mod yaml_corpus;
mod yaml_mutation;
mod yaml_safety;
