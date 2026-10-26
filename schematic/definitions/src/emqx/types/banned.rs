use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::common::PaginationMeta;

/// Ban record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct BanInfo {
    /// Ban type (clientid, username, peerhost).
    #[serde(rename = "as")]
    pub ban_as: String,

    /// Banned value.
    pub who: String,

    /// Reason for ban.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,

    /// Ban start time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at: Option<String>,

    /// Ban expiration time.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub until: Option<String>,
}
impl schematic_define::Registration for BanInfo {
    const RUST_IDENTIFIER: Option<&'static str> = Some("BanInfo");
    fn register(ctx: &mut schematic_define::DefinitionContext) -> Result<schematic_define::TypeDesc, schematic_define::DefinitionError> {
        ctx.register_nominal::<Self>()
    }
}


/// Request body for creating a ban.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CreateBanBody {
    /// Ban type (clientid, username, peerhost).
    #[serde(rename = "as")]
    pub ban_as: String,

    /// Value to ban.
    pub who: String,

    /// Reason for ban.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,

    /// Ban duration in seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<u64>,
}
impl schematic_define::Registration for CreateBanBody {
    const RUST_IDENTIFIER: Option<&'static str> = Some("CreateBanBody");
    fn register(ctx: &mut schematic_define::DefinitionContext) -> Result<schematic_define::TypeDesc, schematic_define::DefinitionError> {
        ctx.register_nominal::<Self>()
    }
}


/// Response for banned list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ListBannedResponse {
    /// List of ban records.
    pub data: Vec<BanInfo>,

    /// Pagination metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<PaginationMeta>,
}
impl schematic_define::Registration for ListBannedResponse {
    const RUST_IDENTIFIER: Option<&'static str> = Some("ListBannedResponse");
    fn register(ctx: &mut schematic_define::DefinitionContext) -> Result<schematic_define::TypeDesc, schematic_define::DefinitionError> {
        ctx.register_nominal::<Self>()
    }
}
