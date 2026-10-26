use super::super::*;
use schematic_define::{DefinitionContext, DefinitionError};
use schematic_define::{Endpoint, RestMethod};

/// Banned clients endpoints (list, create, delete).
pub fn banned_endpoints(ctx: &mut DefinitionContext) -> Result<Vec<Endpoint>, DefinitionError> {
    Ok(vec![
        Endpoint::new("ListBanned", RestMethod::Get, "/banned")
            .describes("List all banned clients, usernames, and hosts")
            .returns_json::<ListBannedResponse>(ctx)
            .pagination(schematic_define::pagination::Collection::at("data").item_type("BanInfo").unpaginated())
            .build()?,
        Endpoint::new("CreateBan", RestMethod::Post, "/banned")
            .describes("Ban a client, username, or host")
            .request_json::<CreateBanBody>(ctx)
            .returns_json::<BanInfo>(ctx)
            .build()?,
        Endpoint::new("DeleteBan", RestMethod::Delete, "/banned/{ban_type}/{who}")
            .describes("Remove a ban by type (clientid, username, peerhost) and value"
                )
            .returns_empty()
            .build()?,
    ])

}
