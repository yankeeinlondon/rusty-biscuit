//! Typed Agent Client Protocol (ACP) metadata.
//!
//! Phase 7 of the centralized providers refactor introduces first-class ACP
//! metadata. Prior phases referenced ACP only in prose comments (e.g. Goose
//! `request_permission`, Kimi `ApprovalRequest`). [`AcpSupport`] makes the
//! provider's ACP server posture, client capability, and the set of events
//! captured via ACP queryable from [`ProviderInfo`](super::ProviderInfo).
//!
//! The [`EventSupportLevel::Acp`](crate::provider::EventSupportLevel) variant
//! tags individual event mapping rows whose capture mechanism is ACP.
//! `server_mode` is a provider-capability fact (research-fed, acp topic),
//! deliberately decoupled from Claudine's own event wiring: a provider can
//! speak ACP without Claudine capturing any events through it.

use serde::Serialize;

pub use claudine_catalog_types::AcpServerMode;

/// Canonical ACP events Claudine cares about.
///
/// `Custom` is reserved for provider-specific extensions that do not yet
/// have a canonical mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AcpEvent {
    /// `session/request_permission` — agent requests a tool-call approval.
    RequestPermission,
    /// Kimi `ApprovalRequest` JSON-RPC envelope.
    ApprovalRequest,
    /// Tool-call lifecycle event.
    ToolCall,
    /// Tool-call result event.
    ToolResult,
    /// Generic session update notification.
    SessionUpdate,
    /// Provider-specific extension not yet canonicalized.
    Custom(&'static str),
}

/// Provider ACP capability descriptor.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct AcpSupport {
    /// ACP server posture the provider itself offers (research-fed).
    pub server_mode: AcpServerMode,
    /// Whether the provider can act as an ACP client.
    pub client_supported: bool,
    /// ACP-borne events the provider observes or emits.
    pub events_via_acp: &'static [AcpEvent],
}

impl AcpSupport {
    /// Returns whether the provider speaks ACP (native or adapter,
    /// including partial support).
    ///
    /// Semantic shift with the 2026-07-05 `server_mode` graduation: this
    /// reports the PROVIDER's ACP capability, no longer whether Claudine
    /// has an ACP/wire capture path (see `events_via_acp` for that).
    pub fn is_supported(&self) -> bool {
        !matches!(
            self.server_mode,
            AcpServerMode::None | AcpServerMode::Unknown
        )
    }
}
