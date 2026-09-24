//! Level 1 integration tests for `claudine`, one test binary per
//! execution contract (`2026-09-21-consolidated-test-binaries`).
//!
//! Each module was its own test target before the consolidation and keeps
//! that target's name, which is the first segment of every test path here.
//! `Cargo.toml` sets `autotests = false`, so a file in this directory that is
//! not declared below never compiles; `test_layout.rs` rejects one.

mod agent_errors_fleet;
mod boundary_lint;
mod canonical_dispatch;
mod deprecated_compatibility;
mod diagnostic_detail_conformance;
mod kimi_wire;
mod lifecycle_control_flow_spike;
mod model_catalog_integration;
mod opencode_stderr_lifecycle;
mod protocol_fixture_replay;
mod semantic_fidelity;
mod strict_mode_provenance_spike;
mod test_layout;
mod tts_phase1_contract;
mod tts_phase5_contract;
mod typed_stream_protocols;
