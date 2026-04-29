//! Per-provider event mapping table.
//!
//! Each provider owns a single [`EventMappingTable`] containing one
//! [`EventMapping`] entry per [`AgenticEvent`] that it cares about. The table
//! is the canonical source for:
//!
//! - Event support level ([`EventSupportLevel`]).
//! - The provider-native event name (used by adapters and reporting).
//! - Whether the entry participates in standard hook registration via the
//!   per-provider configurator.
//! - Parse aliases used to resolve a provider-native name back to an
//!   [`AgenticEvent`].
//!
//! Phase 3 of the centralized providers refactor moves event support and
//! native-name data out of `crate::events::provider` and into provider-local
//! tables exposed through [`ProviderInfo::event_mapping`](super::ProviderInfo).
//! The legacy `Provider::event_support_level`, `Provider::native_event_name`,
//! `Provider::registration_native_event_name`, and
//! `Provider::event_from_shared_native_name` accessors now forward to the
//! per-provider table.
//!
//! Empty `native_name` strings carry the legacy "supported but no specific
//! native name" meaning so existing snapshots are preserved bit-for-bit.

use serde::Serialize;

use crate::events::AgenticEvent;
use crate::provider::acp::AcpEvent;
use crate::stream::StreamProtocol;

/// Wire-proxy mode for providers that capture events through a protocol
/// interception layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WireProxyMode {
    /// JSON-RPC 2.0 wire proxy (Kimi Code `--wire`).
    JsonRpc,
}

/// Level of event support for a provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EventSupportLevel {
    /// Event is not supported by this provider.
    NotSupported,

    /// Event is supported via native hooks (config-file based).
    ///
    /// These events can be registered by modifying the provider's config file.
    /// Examples: Claude hooks, Gemini hooks, OpenCode plugins.
    Hook {
        /// Provider-native event name.
        native_name: &'static str,
    },

    /// Event is captured by parsing the provider's structured output stream.
    ///
    /// Examples: Codex JSONL, Qwen stream-json.
    StreamParse {
        /// Stream protocol used by this provider.
        protocol: StreamProtocol,
        /// Provider-native event name.
        native_name: &'static str,
    },

    /// Event is captured through a wire-mode protocol proxy.
    ///
    /// Example: Kimi Code `--wire` JSON-RPC envelopes.
    WireProxy {
        /// Wire proxy mode.
        mode: WireProxyMode,
        /// Provider-native event name.
        native_name: &'static str,
    },

    /// Event is captured via the Agent Client Protocol (ACP).
    ///
    /// Examples: Goose `request_permission`, Kimi `ApprovalRequest` (when
    /// reached via Claudine's wire-mode JSON-RPC proxy). Providers with at
    /// least one `Acp` row are required to report a non-`NotSupported`
    /// [`AcpSupport::server_mode`](super::AcpSupport).
    Acp {
        /// Canonical ACP event type.
        event: AcpEvent,
        /// Provider-native event name.
        native_name: &'static str,
    },

    /// Event is captured via a wrapper script or environment interception.
    ///
    /// Example: Goose `GOOSE_STATUS_HOOK`.
    Wrapper {
        /// Provider-native event name.
        native_name: &'static str,
    },
}

impl EventSupportLevel {
    /// Returns whether this level indicates any form of support.
    pub fn is_supported(&self) -> bool {
        !matches!(self, EventSupportLevel::NotSupported)
    }

    /// Returns whether this level indicates hook-based support.
    pub fn is_hook(&self) -> bool {
        matches!(self, EventSupportLevel::Hook { .. })
    }

    /// Returns whether this level indicates ACP-based capture.
    pub fn is_acp(&self) -> bool {
        matches!(self, EventSupportLevel::Acp { .. })
    }

    /// Returns the native name carried by this support level, if any.
    pub fn native_name(&self) -> Option<&'static str> {
        match self {
            EventSupportLevel::NotSupported => None,
            EventSupportLevel::Hook { native_name } => Some(native_name),
            EventSupportLevel::StreamParse { native_name, .. } => Some(native_name),
            EventSupportLevel::WireProxy { native_name, .. } => Some(native_name),
            EventSupportLevel::Acp { native_name, .. } => Some(native_name),
            EventSupportLevel::Wrapper { native_name } => Some(native_name),
        }
    }
}

/// Single event mapping entry: support level + parse aliases.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct EventMapping {
    /// Canonical agentic event for this row.
    pub event: AgenticEvent,
    /// Support level for this event on this provider. The variant carries
    /// the provider-native event name when relevant.
    pub support_level: EventSupportLevel,
    /// Parse aliases used to resolve a native event name back to this
    /// [`AgenticEvent`]. Includes the native name when relevant.
    pub parse_aliases: &'static [&'static str],
    /// When `true`, this row participates in the provider's standard hook
    /// registration workflow (e.g. settings.json hooks for Claude/Gemini,
    /// opencode.json plugin hooks for OpenCode). When `false`, the event
    /// either piggybacks on another hook entry (e.g. Claude
    /// `HumanInTheLoop` reuses `PreToolUse`) or is captured outside of
    /// configurator-driven hook registration entirely.
    pub registration_target: bool,
}

/// Per-provider event mapping table.
#[derive(Debug, Serialize)]
pub struct EventMappingTable {
    /// All mapping rows. Rows with [`EventSupportLevel::NotSupported`] may be
    /// included for completeness but are typically omitted.
    pub mappings: &'static [EventMapping],
}

impl EventMappingTable {
    /// Returns the mapping row for `event`, if one exists.
    pub fn lookup(&self, event: AgenticEvent) -> Option<&EventMapping> {
        self.mappings.iter().find(|entry| entry.event == event)
    }

    /// Returns the support level for `event`. Defaults to
    /// [`EventSupportLevel::NotSupported`] when no row is registered.
    pub fn support_level(&self, event: AgenticEvent) -> EventSupportLevel {
        self.lookup(event)
            .map(|entry| entry.support_level)
            .unwrap_or(EventSupportLevel::NotSupported)
    }

    /// Returns the provider-native name for `event`.
    ///
    /// Returns `None` when the event is not supported, matching the legacy
    /// `Provider::native_event_name` contract.
    pub fn native_name(&self, event: AgenticEvent) -> Option<&'static str> {
        let entry = self.lookup(event)?;
        entry.support_level.native_name()
    }

    /// Returns the registration native name used by configurators when
    /// writing a hook entry for `event`.
    ///
    /// Only rows with `registration_target == true` and a non-empty
    /// native name produce `Some`; everything else returns `None`. This
    /// preserves the legacy `Provider::registration_native_event_name`
    /// contract where only events present in the previous shared mapping
    /// constants resolved.
    pub fn registration_native_name(&self, event: AgenticEvent) -> Option<&'static str> {
        let entry = self.lookup(event)?;
        let name = entry.support_level.native_name()?;
        if entry.registration_target && !name.is_empty() {
            Some(name)
        } else {
            None
        }
    }

    /// Resolves a provider-native event name back to a canonical
    /// [`AgenticEvent`].
    ///
    /// Only rows with `registration_target == true` participate in this
    /// lookup; this preserves the legacy `event_from_shared_native_name`
    /// contract. Matching is case-insensitive and walks the row's
    /// `parse_aliases` slice.
    pub fn event_from_native_name(&self, native_name: &str) -> Option<AgenticEvent> {
        self.mappings.iter().find_map(|entry| {
            if !entry.registration_target {
                return None;
            }
            if entry
                .parse_aliases
                .iter()
                .any(|alias| alias.eq_ignore_ascii_case(native_name))
            {
                Some(entry.event)
            } else {
                None
            }
        })
    }

    /// Returns an iterator over rows with `registration_target == true`.
    ///
    /// Replaces the legacy `Provider::shared_native_mappings()` accessor used
    /// by adapter and configurator tests.
    pub fn registration_targets(&self) -> impl Iterator<Item = &EventMapping> {
        self.mappings
            .iter()
            .filter(|entry| entry.registration_target)
    }
}
