use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::common::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct UsageStatsResponse {
    pub time: Vec<i64>,
    pub usage: std::collections::HashMap<String, Vec<f64>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct InvoiceModel {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount_due: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<Currency>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_date_unix: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SubscriptionModel {
    pub tier: String,
    pub character_count: i64,
    pub character_limit: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_character_limit_extension: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_slots_used: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_limit: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub professional_voice_slots_used: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub professional_voice_limit: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<SubscriptionStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_period: Option<BillingPeriod>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<Currency>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_invoice: Option<InvoiceModel>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct UserResponse {
    pub user_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscription: Option<SubscriptionModel>,
    #[serde(default)]
    pub is_new_user: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub xi_api_key: Option<String>,
    #[serde(default)]
    pub is_onboarding_completed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ShareOptionModel {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_id: Option<String>,
    pub role: AccessLevel,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ResourceResponse {
    pub resource_id: String,
    pub resource_type: ResourceType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creator_user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anonymous_access_level_override: Option<AccessLevel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role_to_group_ids: Option<std::collections::HashMap<String, Vec<String>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub share_options: Option<Vec<ShareOptionModel>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ShareResourceBody {
    pub role: AccessLevel,
    pub resource_type: ResourceType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_api_key_id: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct UnshareResourceBody {
    pub resource_type: ResourceType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_api_key_id: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CopyResourceBody {
    pub resource_type: ResourceType,
    pub target_user_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ApiKeyModel {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    pub key_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_account_user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at_unix: Option<i64>,
    #[serde(default)]
    pub is_disabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<Vec<ApiPermission>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub character_limit: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub character_count: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ServiceAccountModel {
    pub service_account_user_id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at_unix: Option<i64>,
    #[serde(rename = "api-keys", skip_serializing_if = "Option::is_none")]
    pub api_keys: Option<Vec<ApiKeyModel>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ListServiceAccountsResponse {
    #[serde(rename = "service-accounts")]
    pub service_accounts: Vec<ServiceAccountModel>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ListApiKeysResponse {
    #[serde(rename = "api-keys")]
    pub api_keys: Vec<ApiKeyModel>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum PermissionSpec {
    All(String),
    List(Vec<ApiPermission>),
}

impl Default for PermissionSpec {
    fn default() -> Self {
        Self::All("all".to_string())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CreateApiKeyBody {
    pub name: String,
    pub permissions: PermissionSpec,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub character_limit: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct CreateApiKeyResponse {
    pub xi_api_key: String,
    pub key_id: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct UpdateApiKeyBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<PermissionSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub character_limit: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProductModel {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct WebhookModel {
    pub name: String,
    pub webhook_id: String,
    pub webhook_url: String,
    #[serde(default)]
    pub is_disabled: bool,
    #[serde(default)]
    pub is_auto_disabled: bool,
    pub created_at_unix: i64,
    pub auth_type: WebhookAuthType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Vec<ProductModel>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub most_recent_failure_error_code: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub most_recent_failure_timestamp: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ListWebhooksResponse {
    pub webhooks: Vec<WebhookModel>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WebhookSettings {
    pub auth_type: WebhookAuthType,
    #[serde(default)]
    pub name: String,
    pub webhook_url: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CreateWebhookBody {
    pub settings: WebhookSettings,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CreateWebhookResponse {
    pub webhook_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub webhook_secret: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct UpdateWebhookBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_disabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}
