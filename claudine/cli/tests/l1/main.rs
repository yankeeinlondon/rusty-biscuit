//! Level 1 integration tests for `claudine-cli`, one test binary per
//! execution contract (`2026-09-21-consolidated-test-binaries`).
//!
//! Each module was its own test target before the consolidation and keeps
//! that target's name, which is the first segment of every test path here.
//! `Cargo.toml` sets `autotests = false`, so a file in this directory that is
//! not declared below never compiles; `test_placement.rs` rejects one.

#[path = "../common/mod.rs"]
mod common;

mod agent_cwd;
mod argv_normalization;
mod characterization_error_routes;
mod cli_process_fixture;
mod command_routing;
mod completion_cli;
mod completion_compose;
mod completion_contract;
mod completion_inline_compose;
mod completion_perf;
mod completion_resolution_round_trip;
mod completion_sequence;
mod completion_setter;
mod compose_caller_file_provenance;
#[cfg(unix)]
mod compose_cli;
#[cfg(unix)]
mod compose_frontmatter_model;
#[cfg(unix)]
mod compose_header_first;
#[cfg(unix)]
mod compose_initialize_acceptance;
#[cfg(unix)]
mod compose_initialize_staged_boot;
#[cfg(unix)]
mod compose_interactive_timeout_cli;
#[cfg(unix)]
mod compose_removed_validation_keys;
mod compose_repository_context;
mod compose_schema_cli;
#[cfg(unix)]
mod compose_system_prompt_lifetime;
#[cfg(unix)]
mod compose_ttff_perf;
#[cfg(unix)]
mod composition_outputs;
mod composition_seams;
#[cfg(unix)]
mod contamination_probes;
mod context_command;
mod contextual_errors;
mod ctx_launch_anchor;
mod detached_audio;
mod diagnostic_discovery;
mod dispatch_inventory;
mod effective_diagnostic_render;
mod error_guards;
mod errors_command;
#[cfg(unix)]
mod handle_blocking_output;
mod handle_deadline;
mod handle_repo_config;
mod hooks_cli;
#[cfg(unix)]
mod inline_completion_lifecycle;
#[cfg(unix)]
mod inline_compose_cli;
mod inline_compose_hash;
mod inline_compose_sequence_mismatch;
#[cfg(unix)]
mod level1_compose_autocomplete_failure_pty;
#[cfg(unix)]
mod level1_dry_run_pty;
#[cfg(unix)]
mod level1_inline_compose_mismatch_pty;
#[cfg(unix)]
mod level1_provided_partial_file_pty;
#[cfg(unix)]
mod level1_provider_overlay_home;
#[cfg(unix)]
mod level1_pty_wrapper_summary;
#[cfg(unix)]
mod level1_review_router_partial_pty;
#[cfg(unix)]
mod level1_schema_prompt_pty;
#[cfg(unix)]
mod level1_structured_error_message;
#[cfg(unix)]
mod loop_cli;
mod loop_initialize_state;
mod mcp_cli;
#[cfg(unix)]
mod prompt_reporting;
mod propagated_context_fixtures;
mod protect_cli;
mod provider_error_finalize;
mod run_harness_loop_call_sites;
// Spawns the `claudine-fake-goose` fixture binary, which only
// `test-fixtures` builds.
#[cfg(feature = "test-fixtures")]
mod sequence_budget;
mod sequence_cli;
#[cfg(windows)]
mod sequence_ctrl_c_windows;
mod sequence_errors_cli;
#[cfg(unix)]
mod sequence_groups;
mod sequence_initialize_include_preflight;
#[cfg(unix)]
mod sequence_jit;
#[cfg(unix)]
mod sequence_magic_reference;
#[cfg(unix)]
mod sequence_overlay_pty;
#[cfg(unix)]
mod sequence_perf;
#[cfg(unix)]
mod sequence_prompt_property;
#[cfg(unix)]
mod sequence_schema;
mod sequence_sources_cli;
mod shipped_prompt_contract;
mod shipped_prompt_route_drift;
mod shipped_prompts;
mod skills_integration;
mod spawn_site_guard;
mod system_prompt_perf_bench;
mod test_placement;
#[cfg(unix)]
mod wrap_antigravity_exit_signal;
mod wrap_basics;
#[cfg(unix)]
mod wrap_compose_agent;
#[cfg(unix)]
mod wrap_compose_exec;
#[cfg(unix)]
mod wrap_compose_preflight;
// Installs the `claudine-fake-goose` fixture binary, which only
// `test-fixtures` builds.
#[cfg(feature = "test-fixtures")]
mod wrap_compose_validation;
mod wrap_ctrl_c_windows;
#[cfg(unix)]
mod wrap_direct_argv;
#[cfg(unix)]
mod wrap_incomplete_subagents;
mod wrap_inline_compose;
#[cfg(unix)]
mod wrap_inline_compose_interactive;
#[cfg(unix)]
mod wrap_opencode;
#[cfg(unix)]
mod wrap_opencode_models;
#[cfg(unix)]
mod wrap_perf;
#[cfg(unix)]
mod wrap_provider_flags;
#[cfg(unix)]
mod wrap_sequence_composition;
#[cfg(unix)]
mod wrap_sigint;
#[cfg(unix)]
mod wrap_structured_stream;
#[cfg(unix)]
mod wrap_watchdog_startup_stall;
#[cfg(unix)]
mod wrap_watchdog_timeout;
