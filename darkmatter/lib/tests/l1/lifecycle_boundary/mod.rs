//! Stage 1 boundary work (`2026-10-27-darkmatter-lifecycle-ownership`).
//!
//! `replay` is the gate, now run against production: every recorded Claudine
//! trace reproduced by a scripted host driving
//! `darkmatter::lifecycle::run::Run`. The run protocol's own tests live in
//! `lifecycle_run_protocol`.
//!
//! The rest is still the test-only prototype, kept because the event registry
//! is a later phase and production has nothing to test it against yet:
//! `engine` is the prototype's registry (registration, renames, positions and
//! roles, outcome pairs, per-event `err` views, action types) and the minimal
//! run that exercises it; `profiles` holds the Claudine and `md`
//! registrations; `publisher` is a deterministic non-agent host with its own
//! events; and `protocol_tests` covers registration, renames, the core-only
//! `md` profile, `err` views, and registered action types. The prototype's
//! cancellation, retry, resume, iteration, and host-refusal paths were retired
//! once production covered them.

pub mod engine;
pub mod profiles;

mod protocol_tests;
mod publisher;
mod replay;
