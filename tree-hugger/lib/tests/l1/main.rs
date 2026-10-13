//! Level 1 integration tests for `tree-hugger`, one test binary per
//! execution contract (`2026-09-21-consolidated-test-binaries`).
//!
//! Each module was its own test target before the consolidation and keeps
//! that target's name, which is the first segment of every test path here.
//! `Cargo.toml` sets `autotests = false`, so a file in this directory that is
//! not declared below never compiles; `test_layout.rs` rejects one.

mod adapter_tests;
mod cache_tests;
mod corpus_tests;
mod file_identity;
mod lint_diagnostics;
mod lint_severity_registry;
mod phase1_diagnostics;
mod phase6_neovim_query_reuse;
mod query_compile;
mod resolver_tests;
mod rule_registry_consistency;
mod symbols_adapter;
mod symbols_async;
mod symbols_cache;
mod symbols_cache_imports;
mod symbols_cache_lexical;
mod symbols_containment;
mod symbols_coverage;
mod symbols_decl_text;
mod symbols_declarator_wrappers;
mod symbols_deferred_metadata;
mod symbols_export;
mod symbols_field_lists;
mod symbols_flags;
mod symbols_fixtures;
mod symbols_function_locals;
mod symbols_generics;
mod symbols_modifiers;
mod symbols_namespace_bodies;
mod symbols_name_identity;
mod symbols_occurrences;
mod symbols_header;
mod symbols_kind_coverage;
mod symbols_kinds;
mod symbols_member_wrappers;
mod symbols_metrics;
mod symbols_owner_identity;
mod symbols_qualified_names;
mod symbols_reference_summaries;
mod symbols_relation_work;
mod symbols_relations;
mod symbols_query;
mod reexports_consumers;
mod reexports_equivalence;
mod symbols_reexports;
mod reexports_support;
mod symbols_reexports_cpp;
mod symbols_reexports_ecma;
mod symbols_reexports_go_lua;
mod symbols_reexports_python_perl;
mod symbols_reexports_rust;
mod symbols_reexports_swift_scala_csharp;
mod symbols_query_filters;
mod symbols_scope;
mod symbols_selection_filters;
mod symbols_shared_context;
mod symbols_signatures;
mod symbols_static;
mod symbols_visibility;
mod test_layout;
mod tree_file;
mod tree_package;

#[path = "../support/language_cases.rs"]
mod language_cases;

use tree_hugger::ProgrammingLanguage;
