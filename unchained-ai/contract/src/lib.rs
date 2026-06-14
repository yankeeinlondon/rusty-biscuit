//! A [`biscuit_contract::inference::InferenceAdapter`] backed by the
//! `unchained-ai` provider/model abstraction.
//!
//! [`UnchainedInferenceAdapter`] routes provider-neutral inference requests
//! through the `unchained-ai` execution surface and model resolver, returning
//! prose or schema-validated structured output. Consumers inject it as
//! `Arc<dyn InferenceAdapter>` and never depend on a provider crate directly.
//!
//! ## Example
//!
//! ```no_run
//! use std::sync::Arc;
//! use biscuit_contract::inference::InferenceAdapter;
//! use unchained_ai_contract::UnchainedInferenceAdapter;
//!
//! let adapter: Arc<dyn InferenceAdapter> = UnchainedInferenceAdapter::new().build();
//! ```

mod adapter;
mod error;
mod profile;
mod structured;

pub use adapter::UnchainedInferenceAdapter;
