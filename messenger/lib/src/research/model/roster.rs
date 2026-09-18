//! The research roster (`messenger/docs/platforms.yaml`).
//!
//! The roster owns platform, interface, and adapter identity, coverage,
//! curated sources, and refresh intervals; it never holds platform facts.

use serde::{Deserialize, Serialize};

use super::common::{AdapterId, Date, PlatformId, string_enum};
use super::document::{InterfaceRole, Relationship};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Roster {
    pub roster_version: u32,
    pub refresh_interval_days: u32,
    /// Configurable per-platform cap; the schema ceiling is 10.
    pub curated_source_cap: u32,
    pub platforms: Vec<RosterPlatform>,
    pub excluded: Vec<RosterExclusion>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hash: Option<serde_json::Value>,
}

impl Roster {
    pub fn platform(&self, platform_id: PlatformId) -> Option<&RosterPlatform> {
        self.platforms.iter().find(|platform| platform.platform_id == platform_id)
    }

    /// Active platforms in roster order; paused platforms never enter the fleet.
    pub fn active_platforms(&self) -> impl Iterator<Item = &RosterPlatform> {
        self.platforms
            .iter()
            .filter(|platform| platform.status == PlatformStatus::Active)
    }
}

string_enum! {
    pub enum PlatformStatus {
        Active => "active",
        Paused => "paused",
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RosterInterface {
    pub interface_id: String,
    pub role: InterfaceRole,
    pub api_identity: String,
    pub adapters: Vec<AdapterId>,
    pub identification_url: String,
    #[serde(default)]
    pub relationships: Vec<Relationship>,
}

string_enum! {
    pub enum CuratedReview {
        Proposed => "proposed",
        Approved => "approved",
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CuratedSource {
    pub url: String,
    pub contribution: String,
    pub interfaces: Vec<String>,
    pub review: CuratedReview,
    pub approved_by: Option<String>,
    pub approved_on: Option<Date>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RosterPlatform {
    pub platform_id: PlatformId,
    pub name: String,
    pub status: PlatformStatus,
    /// The document filename, `{platform_id}.md`.
    pub file: String,
    pub website: String,
    pub api_url: Option<String>,
    pub refresh_interval_days: Option<u32>,
    pub interfaces: Vec<RosterInterface>,
    pub curated_sources: Vec<CuratedSource>,
    pub notes: Option<String>,
}

impl RosterPlatform {
    pub fn interface(&self, interface_id: &str) -> Option<&RosterInterface> {
        self.interfaces
            .iter()
            .find(|interface| interface.interface_id == interface_id)
    }
}

string_enum! {
    pub enum ExclusionKind {
        Platform => "platform",
        Provider => "provider",
        Interface => "interface",
    }
}

string_enum! {
    pub enum ExclusionStatus {
        Excluded => "excluded",
        Paused => "paused",
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RosterExclusion {
    pub subject: String,
    pub kind: ExclusionKind,
    pub status: ExclusionStatus,
    pub reason: String,
    pub existing_research: Option<String>,
}
