//! Shared provider-neutral contract for one text-inference operation.
//!
//! This crate defines the request/response shape, error categories, and the
//! `InferenceAdapter` trait that Reaper, Darkmatter, Claudine, and Unchained
//! AI agree on. Provider implementations in Claudine and Unchained AI, and
//! consumer wiring in Reaper and Darkmatter, are follow-up work and do not
//! live here.

pub mod inference;
