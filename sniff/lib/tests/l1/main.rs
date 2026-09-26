//! Level 1 integration tests for `sniff`, one test binary per
//! execution contract (`2026-09-21-consolidated-test-binaries`).
//!
//! Each module was its own test target before the consolidation and keeps
//! that target's name, which is the first segment of every test path here.
//! `Cargo.toml` sets `autotests = false`, so a file in this directory that is
//! not declared below never compiles; `test_layout.rs` rejects one.

// Shared by `bench_fixtures`, `benchmark_workloads`, and `git_parity`; one
// binary may load a file as a module only once.
#[path = "../../benches/support/builder.rs"]
mod builder;

mod bench_fixtures;
mod bench_ids_sync;
mod bench_plans;
mod benchmark_workloads;
#[cfg(feature = "remote")]
mod focused_provider;
mod git_parity;
mod host_capability_cache;
mod integration;
mod merge_conflict_prediction;
mod network_primitives;
#[cfg(feature = "remote")]
mod open_pull_requests;
#[cfg(feature = "remote")]
mod pr_for_branch;
mod program_installable;
mod program_serialization;
mod recent_commits;
#[cfg(feature = "network")]
mod remote_observation;
#[cfg(feature = "remote")]
mod remote_providers;
mod remote_resolution;
mod test_layout;
mod uv_with_install_plan;
#[cfg(target_os = "windows")]
mod windows_app_paths_orphan;
#[cfg(target_os = "windows")]
mod windows_find_program_priority;
