//! A [`biscuit_contract::inference::InferenceAdapter`] backed by Claudine.
//!
//! [`ClaudineInferenceAdapter`] runs an agentic CLI (Claude Code, Codex, …) as
//! a single **non-interactive, tool-free, filesystem-isolated** session and
//! returns its final assistant text as the inference result. Deterministic
//! consumers — Reaper first, Darkmatter next — inject it as
//! `Arc<dyn InferenceAdapter>` and never depend on a provider crate directly.
//!
//! ## Security posture
//!
//! Consumers feed untrusted scraped text, and an agentic CLI is tool-capable
//! in its normal mode. Every session therefore runs with:
//!
//! - an ephemeral working directory (never the consumer's repo or CWD),
//! - an isolated shadow `HOME` holding only the provider's credential file(s)
//!   and a deny-all policy — never the user's real provider home,
//! - an explicit environment allowlist (no ambient-environment leakage),
//! - real provider tool-denial controls (Claude deny-all `settings.json` +
//!   strict MCP config; Codex read-only sandbox — blocks writes, with network
//!   denial treated as defense-in-depth), with the guard prompt and
//!   post-hoc stream rejection as defense-in-depth,
//! - adapter-owned guard instructions delivered out-of-band from the prompt
//!   where the provider exposes a system-prompt channel.
//!
//! A provider that cannot be run this way is reported as
//! [`InferenceErrorKind::Unsupported`] rather than run unsafely. See the crate
//! README for the per-provider support matrix.
//!
//! [`InferenceErrorKind::Unsupported`]: biscuit_contract::inference::InferenceErrorKind::Unsupported
//!
//! ## Example
//!
//! ```no_run
//! use std::sync::Arc;
//! use biscuit_contract::inference::InferenceAdapter;
//! use claudine::provider_id::Provider;
//! use claudine_contract::ClaudineInferenceAdapter;
//!
//! let adapter: Arc<dyn InferenceAdapter> =
//!     ClaudineInferenceAdapter::new(Provider::Claude).build();
//! ```

mod adapter;
mod error;
mod home;
mod profile;
mod session;
mod structured;
mod support;

#[cfg(test)]
mod tests;

pub use adapter::ClaudineInferenceAdapter;
pub use support::{ProviderSupport, provider_support, support_matrix, SupportRow};
