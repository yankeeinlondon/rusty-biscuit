use super::super::*;
use schematic_define::{DefinitionContext, DefinitionError};
use schematic_define::{Endpoint, RestMethod};

pub fn all(ctx: &mut DefinitionContext) -> Result<Vec<Endpoint>, DefinitionError> {
    Ok(vec![
        Endpoint::new("GetUsageStats", RestMethod::Get, "/v1/usage/character-stats")
            .describes("Gets usage statistics")
            .returns_json::<UsageStatsResponse>(ctx)
            .build()?,
        Endpoint::new("GetUser", RestMethod::Get, "/v1/user")
            .describes("Gets current user information")
            .returns_json::<UserResponse>(ctx)
            .build()?,
        Endpoint::new("GetUserSubscription", RestMethod::Get, "/v1/user/subscription")
            .describes("Gets user subscription information")
            .returns_json::<SubscriptionModel>(ctx)
            .build()?,
        Endpoint::new("GetResource", RestMethod::Get, "/v1/workspace/resources/{resource_id}")
            .describes("Gets resource information")
            .returns_json::<ResourceResponse>(ctx)
            .build()?,
        Endpoint::new("ShareResource", RestMethod::Post, "/v1/workspace/resources/{resource_id}/share")
            .describes("Shares a resource")
            .request_json::<ShareResourceBody>(ctx)
            .returns_json::<StatusResponse>(ctx)
            .build()?,
        Endpoint::new("UnshareResource", RestMethod::Post, "/v1/workspace/resources/{resource_id}/unshare")
            .describes("Removes sharing for a resource")
            .request_json::<UnshareResourceBody>(ctx)
            .returns_json::<StatusResponse>(ctx)
            .build()?,
        Endpoint::new("CopyResourceToWorkspace", RestMethod::Post, "/v1/workspace/resources/{resource_id}/copy-to-workspace")
            .describes("Copies a resource to another workspace")
            .request_json::<CopyResourceBody>(ctx)
            .returns_json::<StatusResponse>(ctx)
            .build()?,
        Endpoint::new("ListServiceAccounts", RestMethod::Get, "/v1/service-accounts")
            .describes("Lists service accounts")
            .returns_json::<ListServiceAccountsResponse>(ctx)
            .build()?,
        Endpoint::new("ListServiceAccountApiKeys", RestMethod::Get, "/v1/service-accounts/{service_account_user_id}/api-keys")
            .describes("Lists API keys for a service account")
            .returns_json::<ListApiKeysResponse>(ctx)
            .build()?,
        Endpoint::new("CreateApiKey", RestMethod::Post, "/v1/service-accounts/{service_account_user_id}/api-keys")
            .describes("Creates an API key for a service account")
            .request_json::<CreateApiKeyBody>(ctx)
            .returns_json::<CreateApiKeyResponse>(ctx)
            .build()?,
        Endpoint::new("UpdateApiKey", RestMethod::Patch, "/v1/service-accounts/{service_account_user_id}/api-keys/{api_key_id}"
                )
            .describes("Updates an API key")
            .request_json::<UpdateApiKeyBody>(ctx)
            .returns_json::<StatusResponse>(ctx)
            .build()?,
        Endpoint::new("DeleteApiKey", RestMethod::Delete, "/v1/service-accounts/{service_account_user_id}/api-keys/{api_key_id}"
                )
            .describes("Deletes an API key")
            .returns_json::<StatusResponse>(ctx)
            .build()?,
        Endpoint::new("ListWebhooks", RestMethod::Get, "/v1/workspace/webhooks")
            .describes("Lists webhooks")
            .returns_json::<ListWebhooksResponse>(ctx)
            .build()?,
        Endpoint::new("CreateWebhook", RestMethod::Post, "/v1/workspace/webhooks")
            .describes("Creates a webhook")
            .request_json::<CreateWebhookBody>(ctx)
            .returns_json::<CreateWebhookResponse>(ctx)
            .build()?,
        Endpoint::new("UpdateWebhook", RestMethod::Patch, "/v1/workspace/webhooks/{webhook_id}")
            .describes("Updates a webhook")
            .request_json::<UpdateWebhookBody>(ctx)
            .returns_json::<StatusResponse>(ctx)
            .build()?,
        Endpoint::new("DeleteWebhook", RestMethod::Delete, "/v1/workspace/webhooks/{webhook_id}")
            .describes("Deletes a webhook")
            .returns_json::<StatusResponse>(ctx)
            .build()?,
    ])

}
