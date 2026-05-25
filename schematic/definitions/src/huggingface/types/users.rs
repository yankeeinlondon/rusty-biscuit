use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct UserInfo {
    #[serde(rename = "name", alias = "user")]
    pub name: String,
    #[serde(rename = "fullname", skip_serializing_if = "Option::is_none")]
    pub fullname: Option<String>,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub user_type: Option<String>,
    #[serde(rename = "avatarUrl", skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    #[serde(rename = "isPro", default)]
    pub is_pro: bool,
    #[serde(rename = "numModels", skip_serializing_if = "Option::is_none")]
    pub num_models: Option<u64>,
    #[serde(rename = "numDatasets", skip_serializing_if = "Option::is_none")]
    pub num_datasets: Option<u64>,
    #[serde(rename = "numSpaces", skip_serializing_if = "Option::is_none")]
    pub num_spaces: Option<u64>,
    #[serde(rename = "numLikes", skip_serializing_if = "Option::is_none")]
    pub num_likes: Option<u64>,
    #[serde(rename = "numFollowers", skip_serializing_if = "Option::is_none")]
    pub num_followers: Option<u64>,
    #[serde(rename = "numFollowing", skip_serializing_if = "Option::is_none")]
    pub num_following: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WhoAmIResponse {
    #[serde(rename = "type")]
    pub account_type: String,
    pub name: String,
    #[serde(rename = "fullname", skip_serializing_if = "Option::is_none")]
    pub fullname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(rename = "emailVerified", default)]
    pub email_verified: bool,
    #[serde(rename = "canPay", default)]
    pub can_pay: bool,
    #[serde(rename = "isPro", default)]
    pub is_pro: bool,
    #[serde(
        rename = "periodicalAccountData",
        skip_serializing_if = "Option::is_none"
    )]
    pub periodical_account_data: Option<PeriodicalAccountData>,
    #[serde(rename = "avatarUrl", skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub orgs: Vec<OrganizationRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<AuthInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PeriodicalAccountData {
    #[serde(rename = "numModels", skip_serializing_if = "Option::is_none")]
    pub num_models: Option<u64>,
    #[serde(rename = "numDatasets", skip_serializing_if = "Option::is_none")]
    pub num_datasets: Option<u64>,
    #[serde(rename = "numSpaces", skip_serializing_if = "Option::is_none")]
    pub num_spaces: Option<u64>,
    #[serde(rename = "numDiscussions", skip_serializing_if = "Option::is_none")]
    pub num_discussions: Option<u64>,
    #[serde(rename = "numPapers", skip_serializing_if = "Option::is_none")]
    pub num_papers: Option<u64>,
    #[serde(rename = "numUpvotes", skip_serializing_if = "Option::is_none")]
    pub num_upvotes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct OrganizationRef {
    pub name: String,
    #[serde(rename = "fullname", skip_serializing_if = "Option::is_none")]
    pub fullname: Option<String>,
    #[serde(rename = "avatarUrl", skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    #[serde(rename = "isPro", default)]
    pub is_pro: bool,
    #[serde(rename = "roleInOrg", skip_serializing_if = "Option::is_none")]
    pub role_in_org: Option<String>,
    #[serde(rename = "isEnterprise", default)]
    pub is_enterprise: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Organization {
    pub name: String,
    #[serde(rename = "fullname", skip_serializing_if = "Option::is_none")]
    pub fullname: Option<String>,
    #[serde(rename = "avatarUrl", skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    #[serde(rename = "isPro", default)]
    pub is_pro: bool,
    #[serde(rename = "isEnterprise", default)]
    pub is_enterprise: bool,
    #[serde(rename = "numMembers", skip_serializing_if = "Option::is_none")]
    pub num_members: Option<u64>,
    #[serde(rename = "numModels", skip_serializing_if = "Option::is_none")]
    pub num_models: Option<u64>,
    #[serde(rename = "numDatasets", skip_serializing_if = "Option::is_none")]
    pub num_datasets: Option<u64>,
    #[serde(rename = "numSpaces", skip_serializing_if = "Option::is_none")]
    pub num_spaces: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct AuthInfo {
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub auth_type: Option<String>,
    #[serde(rename = "accessToken", skip_serializing_if = "Option::is_none")]
    pub access_token: Option<TokenInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TokenInfo {
    #[serde(rename = "displayName", skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
}
