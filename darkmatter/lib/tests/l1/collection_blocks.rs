//! Library acceptance corpus for `::map` / `::filter`.
//!
//! One case table drives both kinds wherever their behavior is shared; every
//! map-versus-filter difference is spelled out as its own row. The focused
//! submodules own one concern each and reuse the execution helpers of
//! `collection_execution` and the logging probes of `collection_effects`.
//! Rows that spawn nothing heavy are generated one `#[test]` per row by
//! `macro_rules!`, and each macro-generated list has a guard test against the
//! table it indexes.

mod support;

mod budget;
mod docs_corpus;
mod grammar;
mod pipeline;
mod provenance;
mod scope_schedule;
mod types_values;
