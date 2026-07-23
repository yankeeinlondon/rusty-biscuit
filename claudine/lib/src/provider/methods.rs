//! Methods on the canonical [`Provider`] enum.
//!
//! This module hosts the `impl Provider` blocks and the [`Display`] impl.
//! Phase 2 of the centralized providers refactor (Review 1) eliminated all
//! lib-side `match Provider { ... }` dispatch from this module: every
//! identity helper now reads its data straight off
//! [`provider_info`](super::registry::provider_info), so the only authoritative
//! `match Provider` site in the lib crate is the central registry.

use serde_json::Value;
use sniff::programs::AiCli;
use std::fmt;

use super::event_mapping::EventSupportLevel;
use super::identity::{PROVIDERS_DISPLAY_ORDER, Provider};
use super::registry::provider_info;
use crate::events::AgenticEvent;

impl Provider {
    /// Returns common CLI aliases accepted for this provider.
    pub fn cli_aliases(&self) -> &'static [&'static str] {
        provider_info(*self).cli_aliases
    }

    /// Parse a provider from a CLI-facing name or alias.
    pub fn parse_cli_name(input: &str) -> Option<Self> {
        let normalized = normalize_provider_input(input);
        if normalized.is_empty() {
            return None;
        }

        PROVIDERS_DISPLAY_ORDER.into_iter().find(|provider| {
            provider.as_slug() == normalized
                || normalize_provider_input(&provider.to_string()) == normalized
                || provider.cli_aliases().contains(&normalized.as_str())
        })
    }

    /// Fuzzy match a provider from user input (exact/prefix/contains).
    pub fn fuzzy_match_cli_name(input: &str) -> Option<Self> {
        let normalized = normalize_provider_input(input);
        if normalized.is_empty() {
            return None;
        }

        if let Some(provider) = Self::parse_cli_name(&normalized) {
            return Some(provider);
        }

        for provider in PROVIDERS_DISPLAY_ORDER {
            let display = normalize_provider_input(&provider.to_string());
            if display.starts_with(&normalized) || provider.as_slug().starts_with(&normalized) {
                return Some(provider);
            }
            if provider
                .cli_aliases()
                .iter()
                .any(|alias| alias.starts_with(&normalized))
            {
                return Some(provider);
            }
        }

        for provider in PROVIDERS_DISPLAY_ORDER {
            let display = normalize_provider_input(&provider.to_string());
            if display.contains(&normalized) || provider.as_slug().contains(&normalized) {
                return Some(provider);
            }
            if provider
                .cli_aliases()
                .iter()
                .any(|alias| alias.contains(&normalized))
            {
                return Some(provider);
            }
        }

        None
    }

    /// Fuzzy match returning all matching providers (exact/prefix/contains).
    ///
    /// Unlike `fuzzy_match_cli_name` which returns the first match, this
    /// collects all providers that match the input at the best matching tier.
    /// Exact matches are returned alone; otherwise all prefix matches; otherwise
    /// all contains matches.
    pub fn fuzzy_match_all(input: &str) -> Vec<Self> {
        let normalized = normalize_provider_input(input);
        if normalized.is_empty() {
            return Vec::new();
        }

        if let Some(provider) = Self::parse_cli_name(&normalized) {
            return vec![provider];
        }

        let mut prefix_matches = Vec::new();
        for provider in PROVIDERS_DISPLAY_ORDER {
            let display = normalize_provider_input(&provider.to_string());
            if display.starts_with(&normalized)
                || provider.as_slug().starts_with(&normalized)
                || provider
                    .cli_aliases()
                    .iter()
                    .any(|alias| alias.starts_with(&normalized))
            {
                prefix_matches.push(provider);
            }
        }
        if !prefix_matches.is_empty() {
            return prefix_matches;
        }

        let mut contains_matches = Vec::new();
        for provider in PROVIDERS_DISPLAY_ORDER {
            let display = normalize_provider_input(&provider.to_string());
            if display.contains(&normalized)
                || provider.as_slug().contains(&normalized)
                || provider
                    .cli_aliases()
                    .iter()
                    .any(|alias| alias.contains(&normalized))
            {
                contains_matches.push(provider);
            }
        }
        contains_matches
    }

    /// Returns the corresponding sniff `AiCli` variant for install detection.
    pub fn sniff_ai_cli(&self) -> AiCli {
        provider_info(*self).sniff_binding
    }

    /// Detect a provider from raw payload shape.
    ///
    /// Walks providers in canonical display order and asks each provider's
    /// central behavior to recognize the payload. The first hit wins; this
    /// keeps detection table-driven rather than re-implementing payload
    /// shape rules in a per-variant `match` here. Provider-specific shape
    /// rules live behind each provider's `ProviderBehavior::detect_from_payload`
    /// override (see `provider/<name>.rs`); the source guard
    /// `detect_from_payload_has_no_provider_specific_branches` enforces that
    /// this function stays a pure registry walk.
    pub fn detect_from_payload(raw: &Value) -> Option<Self> {
        PROVIDERS_DISPLAY_ORDER
            .into_iter()
            .find(|provider| provider_info(*provider).behavior.detect_from_payload(raw))
    }

    /// Returns a snake_case identifier suitable for file paths and JSON keys.
    ///
    /// Use this for file system paths, config keys, and anywhere that needs a
    /// stable, machine-readable identifier. Use `Display` for user-facing output.
    pub fn as_slug(&self) -> &'static str {
        provider_info(*self).slug
    }

    /// Returns whether this provider supports skill discovery.
    pub fn supports_skills(&self) -> bool {
        provider_info(*self).supports_skills
    }

    /// Returns the documentation URL for this provider.
    pub fn docs_url(&self) -> &'static str {
        provider_info(*self).docs_url
    }

    /// Returns the usage/billing dashboard URL for this provider, if one exists.
    pub fn usage_dashboard_url(&self) -> Option<&'static str> {
        provider_info(*self).usage_dashboard_url
    }

    /// Returns the native name used by configurators for hook registration.
    pub(crate) fn registration_native_event_name(
        &self,
        event: &AgenticEvent,
    ) -> Option<&'static str> {
        provider_info(*self)
            .event_mapping
            .registration_native_name(*event)
    }

    /// Maps a provider-native event name back to a canonical `AgenticEvent`.
    pub(crate) fn event_from_shared_native_name(&self, native_name: &str) -> Option<AgenticEvent> {
        provider_info(*self)
            .event_mapping
            .event_from_native_name(native_name)
    }

    /// Returns the level of support for the given event.
    pub fn event_support_level(&self, event: &AgenticEvent) -> EventSupportLevel {
        provider_info(*self).event_mapping.support_level(*event)
    }

    /// Returns whether this provider supports the given event (via any method).
    pub fn supports_event(&self, event: &AgenticEvent) -> bool {
        self.event_support_level(event).is_supported()
    }

    /// Returns whether this provider supports the given event via native hooks.
    pub fn supports_event_via_hook(&self, event: &AgenticEvent) -> bool {
        self.event_support_level(event).is_hook()
    }

    /// Returns the native event name used by this provider for the given event.
    pub fn native_event_name(&self, event: &AgenticEvent) -> Option<&'static str> {
        provider_info(*self).event_mapping.native_name(*event)
    }

    /// Returns the agent offset directory name for this provider.
    pub fn agent_offset(&self) -> &'static str {
        provider_info(*self).agent_offset
    }
}

fn normalize_provider_input(input: &str) -> String {
    input.trim().to_ascii_lowercase().replace([' ', '-'], "_")
}

impl fmt::Display for Provider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(provider_info(*self).display_name)
    }
}

#[cfg(test)]
mod tests;
