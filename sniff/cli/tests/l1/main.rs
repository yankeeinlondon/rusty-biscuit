//! Level 1 integration tests for `sniff-cli`, one test binary per
//! execution contract (`2026-09-21-consolidated-test-binaries`).
//!
//! Each module was its own test target before the consolidation and keeps
//! that target's name, which is the first segment of every test path here.
//! `Cargo.toml` sets `autotests = false`, so a file in this directory that is
//! not declared below never compiles; `test_layout` rejects one.

#[path = "../common/mod.rs"]
mod common;

mod cli;
mod cli_process_fixture;
mod install_interview_cli;
mod install_plan;
mod snapshots;
mod spawn_site_guard;
mod test_layout;
mod tty;
