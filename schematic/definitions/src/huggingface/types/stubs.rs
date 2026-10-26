use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::common::*;

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct StatusResponse {
    pub ok: Option<bool>,
    pub message: Option<String>,
}
impl schematic_define::Registration for StatusResponse {
    const RUST_IDENTIFIER: Option<&'static str> = Some("StatusResponse");
    fn register(ctx: &mut schematic_define::DefinitionContext) -> Result<schematic_define::TypeDesc, schematic_define::DefinitionError> {
        ctx.register_nominal::<Self>()
    }
}


pub type Commit = CommitInfo;

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct FileMetadata {
    pub path: String,
    pub size: Option<u64>,
    pub oid: Option<String>,
    #[serde(rename = "type")]
    pub file_type: Option<String>,
}
impl schematic_define::Registration for FileMetadata {
    const RUST_IDENTIFIER: Option<&'static str> = Some("FileMetadata");
    fn register(ctx: &mut schematic_define::DefinitionContext) -> Result<schematic_define::TypeDesc, schematic_define::DefinitionError> {
        ctx.register_nominal::<Self>()
    }
}


#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct RepoInfo {
    pub id: String,
    #[serde(rename = "type")]
    pub repo_type: Option<RepoType>,
    pub private: Option<bool>,
}
impl schematic_define::Registration for RepoInfo {
    const RUST_IDENTIFIER: Option<&'static str> = Some("RepoInfo");
    fn register(ctx: &mut schematic_define::DefinitionContext) -> Result<schematic_define::TypeDesc, schematic_define::DefinitionError> {
        ctx.register_nominal::<Self>()
    }
}


#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct RepoUrl {
    pub url: String,
}
impl schematic_define::Registration for RepoUrl {
    const RUST_IDENTIFIER: Option<&'static str> = Some("RepoUrl");
    fn register(ctx: &mut schematic_define::DefinitionContext) -> Result<schematic_define::TypeDesc, schematic_define::DefinitionError> {
        ctx.register_nominal::<Self>()
    }
}
