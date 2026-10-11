//! Opt-in tests of the repository's internal `prompts/`, built only with the
//! `prompt-tests` feature and run by `just test-prompts`, never by CI.
//!
//! A prompt may be a draft or deliberately broken, and editing one must never
//! fail the normal suite.

// Shared with the `l1` binary; this one calls only part of it.
#[allow(dead_code)]
#[path = "../request_support/mod.rs"]
mod request_support;

mod dasherized_identifier_corpus;
