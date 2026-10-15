use super::super::*;
use schematic_define::{DefinitionContext, DefinitionError};
use schematic_define::{Endpoint, RestMethod};

/// Client management endpoints (list, get, disconnect, subscribe, unsubscribe).
pub fn client_endpoints(ctx: &mut DefinitionContext) -> Result<Vec<Endpoint>, DefinitionError> {
    Ok(vec![
        Endpoint::new("ListClients", RestMethod::Get, "/clients")
            .describes("List connected MQTT clients with pagination")
            .returns_json::<ListClientsResponse>(ctx)
            .pagination(schematic_define::pagination::Collection::at("data").item_type("ClientInfo").unpaginated())
            .build()?,
        Endpoint::new("GetClient", RestMethod::Get, "/clients/{clientid}")
            .describes("Get detailed information about a specific client")
            .returns_json::<ClientInfo>(ctx)
            .build()?,
        Endpoint::new("DisconnectClient", RestMethod::Delete, "/clients/{clientid}")
            .describes("Forcefully disconnect a client from the broker")
            .returns_empty()
            .build()?,
        Endpoint::new("SubscribeClient", RestMethod::Post, "/clients/{clientid}/subscribe")
            .describes("Create a subscription for a connected client")
            .request_json::<SubscribeBody>(ctx)
            .returns_empty()
            .build()?,
        Endpoint::new("UnsubscribeClient", RestMethod::Post, "/clients/{clientid}/unsubscribe")
            .describes("Remove a subscription from a connected client")
            .request_json::<SubscribeBody>(ctx)
            .returns_empty()
            .build()?,
    ])

}
