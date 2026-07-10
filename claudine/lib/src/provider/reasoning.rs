//! Typed reasoning / extended-thinking metadata.
//!
//! Phase 5 of the centralized providers refactor replaces the
//! `(ReasoningStyle, Vec<&'static str>)` pair on the retired legacy
//! `ReasoningCapabilities` tree with
//! a structured [`ReasoningSupport`] enum that distinguishes named-level,
//! numeric-budget, binary-toggle, and provider-specific reasoning surfaces.

use serde::Serialize;

/// How a provider exposes reasoning / extended-thinking controls.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningSupport {
    /// The provider has no reasoning surface.
    NotSupported,
    /// Reasoning exists but its surface is undocumented.
    NotDocumented,
    /// A flag selects from a fixed set of named effort levels
    /// (e.g. Codex `low|medium|high|xhigh`).
    NamedLevels {
        /// CLI flag or config key controlling reasoning level.
        flag: &'static str,
        /// Accepted level names in canonical order.
        levels: &'static [&'static str],
    },
    /// A flag accepts a numeric token budget for reasoning.
    NumericBudget {
        /// CLI flag or config key controlling the budget.
        flag: &'static str,
        /// Inclusive minimum value the provider accepts.
        min: u32,
        /// Inclusive maximum value the provider accepts.
        max: u32,
        /// Provider's documented default value, if any.
        default: Option<u32>,
    },
    /// Reasoning is on/off, controlled by mutually exclusive flag values.
    BinaryToggle {
        /// Base flag name (e.g. `--thinking`).
        flag: &'static str,
        /// Value or flag form that turns reasoning on.
        on: &'static str,
        /// Value or flag form that turns reasoning off.
        off: &'static str,
    },
    /// Reasoning is exposed in a provider-specific way captured by the
    /// associated [`ReasoningCustomTag`].
    ProviderSpecific(ReasoningCustomTag),
}

/// Provider-specific reasoning behaviors that don't fit the standard
/// shapes. Extended as new providers are onboarded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningCustomTag {
    /// Goose: reasoning effort is delegated through provider-specific
    /// environment variables (e.g. `GEMINI3_THINKING_LEVEL`).
    GooseDelegated,
    /// Catch-all for not-yet-categorized reasoning surfaces.
    Other,
}
