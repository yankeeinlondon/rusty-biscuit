//! Level 2 (`terminal-tests`) integration tests for `claudine-cli`, one test binary per
//! execution contract (`2026-09-21-consolidated-test-binaries`).
//!
//! Each module was its own test target before the consolidation and keeps
//! that target's name, which is the first segment of every test path here.
//! `Cargo.toml` sets `autotests = false`, so a file in this directory that is
//! not declared below never compiles; `test_placement.rs` rejects one.

#[path = "../common/mod.rs"]
mod common;

#[cfg(unix)]
mod level2_auto_complete_chooser;
#[cfg(unix)]
mod level2_auto_complete_operation_file;
#[cfg(unix)]
mod level2_context_capture;
#[cfg(unix)]
mod level2_dry_run_approval_capture;
#[cfg(unix)]
mod level2_dry_run_metadata_capture;
#[cfg(unix)]
mod level2_explicit_operation_file_miss;
#[cfg(unix)]
mod level2_file_resolution_capture;
#[cfg(unix)]
mod level2_incomplete_subagents_capture;
#[cfg(unix)]
mod level2_initialize_generated_transclusion;
#[cfg(unix)]
mod level2_inline_compose_mismatch_capture;
#[cfg(unix)]
mod level2_interrupt_feedback_capture;
#[cfg(unix)]
mod level2_invalid_file_reference_capture;
#[cfg(unix)]
mod level2_lifecycle_action_forms;
#[cfg(unix)]
mod level2_lifecycle_control;
#[cfg(unix)]
mod level2_lifecycle_dispatch;
#[cfg(unix)]
mod level2_lifecycle_loop;
#[cfg(unix)]
mod level2_malformed_frontmatter_capture;
#[cfg(unix)]
mod level2_perf_capture;
#[cfg(unix)]
mod level2_prompt_reporting_capture;
#[cfg(unix)]
mod level2_provided_partial_file_capture;
mod level2_provider_overlay_capture;
#[cfg(unix)]
mod level2_removed_validation_key_capture;
#[cfg(unix)]
mod level2_schema_parse_capture;
#[cfg(unix)]
mod level2_sequence_task_stream_capture;
#[cfg(unix)]
mod level2_stalled_generation_capture;
#[cfg(unix)]
mod level2_typed_error_render_capture;
#[cfg(windows)]
mod level2_windows_provided_partial_file_capture;
#[cfg(unix)]
mod level2_wrap_ctrl_c_loop_wedge_tmux;
#[cfg(unix)]
mod level2_wrap_ctrl_c_tmux;
