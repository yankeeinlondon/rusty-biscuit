//! Typed gaps in provider capability documentation / coverage.
//!
//! Phase 5 of the centralized providers refactor replaces the
//! `confidence.gaps: Vec<&'static str>` free-form list with a structured
//! [`KnownGap`] entry tagged by [`KnownGapArea`].

use serde::Serialize;

/// Coarse area in which a known gap lives. Mirrors the capability areas
/// of the retired legacy `AgentCapabilities` tree plus a small catch-all
/// set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KnownGapArea {
    /// Skill discovery / activation surface.
    Skills,
    /// Slash command surface.
    SlashCommands,
    /// Subagent surface.
    Subagents,
    /// Script / hook automation surface.
    Scripts,
    /// Hook surface.
    Hooks,
    /// Stream / structured output surface.
    Stream,
    /// MCP integration surface.
    Mcp,
    /// ACP integration surface.
    Acp,
    /// System-prompt / memory file surface.
    SystemPrompt,
    /// Permissions / sandboxing surface.
    Permissions,
    /// Reasoning / extended-thinking surface.
    Reasoning,
    /// Logging / debug surface.
    Logging,
    /// Billing / pricing model surface.
    Billing,
    /// Catch-all for gaps that don't fit elsewhere.
    Other,
}

/// One known gap in a provider's capability data or behavior coverage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct KnownGap {
    /// Coarse area the gap belongs to.
    pub area: KnownGapArea,
    /// Free-form note describing the gap.
    pub note: &'static str,
    /// Optional tracker reference (issue URL, doc path, etc.).
    pub tracker: Option<&'static str>,
}
