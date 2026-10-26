use super::super::*;
use schematic_define::{DefinitionContext, DefinitionError};
use schematic_define::{Endpoint, RestMethod};

/// Authentication and authorization endpoints (authenticators, users, authz sources).
pub fn auth_endpoints(ctx: &mut DefinitionContext) -> Result<Vec<Endpoint>, DefinitionError> {
    Ok(vec![
        Endpoint::new("ListAuthenticators", RestMethod::Get, "/authentication")
            .describes("List all configured authentication providers")
            .returns_json::<ListAuthenticatorsResponse>(ctx)
            .build()?,
        Endpoint::new("GetAuthenticator", RestMethod::Get, "/authentication/{id}")
            .describes("Get details of a specific authenticator")
            .returns_json::<AuthenticatorInfo>(ctx)
            .build()?,
        Endpoint::new("ListAuthUsers", RestMethod::Get, "/authentication/{id}/users")
            .describes("List users in a built-in database authenticator")
            .returns_json::<Vec<AuthUser>>(ctx)
            .build()?,
        Endpoint::new("CreateAuthUser", RestMethod::Post, "/authentication/{id}/users")
            .describes("Create a new user in a built-in database authenticator")
            .request_json::<CreateAuthUserBody>(ctx)
            .returns_json::<AuthUser>(ctx)
            .build()?,
        Endpoint::new("DeleteAuthUser", RestMethod::Delete, "/authentication/{id}/users/{user_id}")
            .describes("Delete a user from a built-in database authenticator")
            .returns_empty()
            .build()?,
        // Authorization
        Endpoint::new("ListAuthzSources", RestMethod::Get, "/authorization/sources")
            .describes("List all authorization sources")
            .returns_json::<ListAuthzSourcesResponse>(ctx)
            .build()?,
    ])

}
