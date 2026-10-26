use super::super::*;
use schematic_define::Endpoint;

/// Returns all branch endpoints.
pub fn all(ctx: &mut schematic_define::DefinitionContext) -> Result<Vec<Endpoint>, schematic_define::DefinitionError> {
    // GitLab rejects a raw slash in a branch; generated path parameters encode it.
    Ok(vec![Endpoint::get("GetBranch", "/projects/{id}/repository/branches/{branch}")
                .describes("Get a single branch (commit.id is the branch head commit)")
                .returns_json::<Branch>(ctx)
                .build()?])
}
