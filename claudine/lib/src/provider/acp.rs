//! Typed Agent Client Protocol (ACP) metadata.
//!
//! Phase 7 of the centralized providers refactor introduces first-class ACP
//! metadata. Prior phases referenced ACP only in prose comments (e.g. Goose
//! `request_permission`, Kimi `ApprovalRequest`). [`AcpSupport`] makes the
//! provider's ACP server posture, client capability, and the set of events
//! captured via ACP queryable from [`ProviderInfo`](super::ProviderInfo).
//!
//! The [`EventSupportLevel::Acp`](crate::provider::EventSupportLevel) variant
//! tags individual event mapping rows whose capture mechanism is ACP. The
//! library guarantees (via tests) that any provider with at least one
//! `EventSupportLevel::Acp` row reports a non-`NotSupported` ACP server
//! mode.

use serde::Serialize;

/// ACP server posture for a provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AcpServerMode {
    /// The provider does not expose an ACP server surface.
    NotSupported,
    /// The provider exposes ACP as a first-class server.
    Native,
    /// ACP is reachable via Claudine's wire-mode JSON-RPC proxy
    /// (currently Kimi `--wire`).
    AvailableViaWireProxy,
}

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
    /// ACP server mode for this provider.
    pub server_mode: AcpServerMode,
    /// Whether the provider can act as an ACP client.
    pub client_supported: bool,
    /// ACP-borne events the provider observes or emits.
    pub events_via_acp: &'static [AcpEvent],
}

impl AcpSupport {
    /// `AcpSupport` value for providers without ACP integration.
    pub const NOT_SUPPORTED: AcpSupport = AcpSupport {
        server_mode: AcpServerMode::NotSupported,
        client_supported: false,
        events_via_acp: &[],
    };

    /// Returns whether this provider exposes any ACP server surface.
    pub fn is_supported(&self) -> bool {
        !matches!(self.server_mode, AcpServerMode::NotSupported)
    }
}
