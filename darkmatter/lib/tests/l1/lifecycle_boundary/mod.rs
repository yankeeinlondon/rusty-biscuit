//! Stage 1 boundary prototype (`2026-10-27-darkmatter-lifecycle-ownership`):
//! can Darkmatter host a neutral, synchronous, request/reply lifecycle engine
//! with registered host events, reproducing Claudine's behavior without
//! Claudine types and without callbacks that mutate engine state?
//!
//! Test-only by design: the production engine is extracted later, and two
//! independently evolving engines are not allowed. `engine` is the prototype
//! API, `profiles` the Claudine and `md` registrations, `publisher` a
//! deterministic non-agent host, `protocol_tests` the registration,
//! reply-protocol, cancellation, shell, and section 8 tests, and `replay` the
//! gate: every recorded Claudine trace reproduced by a scripted host.

pub mod engine;
pub mod profiles;

mod protocol_tests;
mod publisher;
mod replay;
