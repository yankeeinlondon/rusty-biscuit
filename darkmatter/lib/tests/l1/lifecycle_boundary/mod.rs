//! Stage 1 boundary work (`2026-10-27-darkmatter-lifecycle-ownership`).
//!
//! `replay` is the gate, now run against production: every recorded Claudine
//! trace reproduced by a scripted host driving
//! `darkmatter::lifecycle::run::Run`. The run protocol's own tests live in
//! `lifecycle_run_protocol`.
//!
//! The event registry is production (`darkmatter::lifecycle::registry`);
//! the rest is test-only: `engine` is the prototype's minimal run, driven by
//! that registry, and exercises a host's before-composition notification and
//! outcome pair; `profiles` holds test registrations for Claudine's shape and
//! `md`; `publisher` is a deterministic non-agent host with its own events;
//! and `protocol_tests` covers registration, renames, the core-only `md`
//! profile, `err` views, and registered action types against the production
//! registry. The prototype's cancellation, retry, resume, iteration, and
//! host-refusal paths were retired once production covered them.

pub mod engine;
pub mod profiles;

mod protocol_tests;
mod publisher;
mod replay;
