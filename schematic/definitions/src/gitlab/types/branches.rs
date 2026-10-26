use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::merge_requests::Commit;

/// A branch from `GET /projects/{id}/repository/branches/{branch}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Branch {
    pub name: String,
    /// Head commit of the branch.
    pub commit: Commit,
    #[serde(default)]
    pub merged: bool,
    #[serde(default)]
    pub protected: bool,
    #[serde(default)]
    pub default: bool,
    #[serde(default)]
    pub web_url: Option<String>,
}
impl schematic_define::Registration for Branch {
    const RUST_IDENTIFIER: Option<&'static str> = Some(stringify!(Branch));
    fn register(ctx: &mut schematic_define::DefinitionContext) -> Result<schematic_define::TypeDesc, schematic_define::DefinitionError> {
        ctx.register_nominal::<Self>()
    }
}
