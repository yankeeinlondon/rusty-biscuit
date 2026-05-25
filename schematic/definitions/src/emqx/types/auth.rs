use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Request body for the `/login` endpoint.
///
/// ## Example
///
/// ```json
/// {
///   "username": "admin",
///   "password": "public"
/// }
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct LoginBody {
    /// Dashboard username.
    pub username: String,

    /// Dashboard password.
    pub password: String,
}

/// Response from the `/login` endpoint.
///
/// ## Example
///
/// ```json
/// {
///   "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
///   "license": {"edition": "enterprise", ...}
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct LoginResponse {
    /// JWT token for subsequent requests.
    pub token: String,

    /// License information (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<serde_json::Value>,
}

/// Authentication provider configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct AuthenticatorInfo {
    /// Authenticator ID.
    pub id: String,

    /// Authenticator type (built_in_database, http, jwt, etc.).
    #[serde(rename = "type")]
    pub auth_type: String,

    /// Whether the authenticator is enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable: Option<bool>,

    /// Backend configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend: Option<String>,

    /// Additional configuration (type-specific).
    #[serde(flatten)]
    pub config: Option<serde_json::Value>,
}

/// Response for authenticators list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ListAuthenticatorsResponse {
    /// List of configured authenticators.
    pub data: Vec<AuthenticatorInfo>,
}

/// User in built-in database authentication.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct AuthUser {
    /// User ID (username).
    pub user_id: String,

    /// Whether the user is a superuser.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_superuser: Option<bool>,
}

/// Request body for creating an auth user.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CreateAuthUserBody {
    /// User ID (username).
    pub user_id: String,

    /// Password.
    pub password: String,

    /// Whether the user is a superuser.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_superuser: Option<bool>,
}

/// Authorization source configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct AuthzSourceInfo {
    /// Source type (built_in_database, file, http, etc.).
    #[serde(rename = "type")]
    pub source_type: String,

    /// Whether the source is enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable: Option<bool>,

    /// Additional configuration.
    #[serde(flatten)]
    pub config: Option<serde_json::Value>,
}

/// Response for authorization sources list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ListAuthzSourcesResponse {
    /// List of authorization sources.
    pub sources: Vec<AuthzSourceInfo>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_body_serialization() {
        let body = LoginBody {
            username: "admin".to_string(),
            password: "public".to_string(),
        };
        let json = serde_json::to_string(&body).unwrap();
        assert!(json.contains("\"username\":\"admin\""));
        assert!(json.contains("\"password\":\"public\""));
    }
}
