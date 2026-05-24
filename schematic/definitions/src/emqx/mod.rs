//! EMQX Broker REST API definitions.
//!
//! This module provides definitions for the EMQX Broker REST API with two authentication
//! variants: Basic Auth (API Key + Secret) and Bearer Token.
//!
//! The EMQX REST API is available at `http://<host>:18083/api/v5/<endpoint>` and provides
//! full management capabilities for the MQTT broker.
//!
//! ## API Variants
//!
//! ### Basic Auth API (`EmqxBasic`)
//!
//! Uses HTTP Basic Authentication with an API key and secret. Create API keys via the
//! EMQX Dashboard: **System > API Key**.
//!
//! ```bash
//! curl -u {api_key}:{secret_key} http://localhost:18083/api/v5/nodes
//! ```
//!
//! ### Bearer Token API (`EmqxBearer`)
//!
//! Uses JWT bearer tokens obtained from the `/login` endpoint. Suitable for interactive
//! sessions and dashboard integration.
//!
//! ```bash
//! # Get token
//! curl -X POST http://localhost:18083/api/v5/login \
//!   -H "Content-Type: application/json" \
//!   -d '{"username":"admin","password":"public"}'
//!
//! # Use token
//! curl -H "Authorization: Bearer {token}" http://localhost:18083/api/v5/nodes
//! ```
//!
//! ## Endpoint Categories
//!
//! | Category | Key Endpoints |
//! |----------|--------------|
//! | **Nodes** | `/nodes`, `/nodes/{node}` |
//! | **Cluster** | `/cluster` |
//! | **Clients** | `/clients`, `/clients/{clientid}` |
//! | **Subscriptions** | `/subscriptions` |
//! | **Publishing** | `/publish`, `/publish/bulk` |
//! | **Rules** | `/rules`, `/rules/{id}` |
//! | **Authentication** | `/authentication`, `/authentication/{id}/users` |
//! | **Authorization** | `/authorization/sources` |
//! | **Listeners** | `/listeners` |
//! | **Metrics** | `/metrics`, `/stats` |
//! | **Alarms** | `/alarms` |
//! | **Banned** | `/banned` |
//!
//! ## Resources
//!
//! - [Official EMQX Docs](https://docs.emqx.com/en/emqx/latest/)
//! - [REST API Reference](https://docs.emqx.com/en/emqx/latest/admin/api.html)
//! - [Swagger UI](http://localhost:18083/api-docs) (when running locally)

mod endpoints;
mod types;

pub use types::*;

use crate::registry::SchemaRegistry;
use schematic_define::{
    ApiRequest, ApiResponse, AuthStrategy, Endpoint, EnvList, EnvMapping, RestApi, RestMethod,
};

/// Creates a schema registry containing all EMQX response types.
///
/// This registry can be used to generate OpenAPI schemas for the EMQX API.
/// All response types used by both the Basic and Bearer API endpoints are registered.
///
/// ## Examples
///
/// ```
/// use schematic_definitions::emqx::{openapi_registry, define_emqx_basic_api, define_emqx_bearer_api};
///
/// let registry = openapi_registry();
/// let basic_api = define_emqx_basic_api();
/// let bearer_api = define_emqx_bearer_api();
///
/// // Registry contains all response types
/// assert!(registry.get("NodeInfo").is_some());
/// assert!(registry.get("ListClientsResponse").is_some());
///
/// // Registry is complete for both APIs
/// assert!(registry.validate_completeness(&basic_api).is_ok());
/// assert!(registry.validate_completeness(&bearer_api).is_ok());
/// ```
#[must_use]
pub fn openapi_registry() -> SchemaRegistry {
    SchemaRegistry::new()
        // Authentication types (Bearer API only)
        .register::<LoginBody>("LoginBody")
        .register::<LoginResponse>("LoginResponse")
        // Publishing types
        .register::<SubscribeBody>("SubscribeBody")
        .register::<PublishBody>("PublishBody")
        .register::<PublishBatchBody>("PublishBatchBody")
        // Rules Engine types
        .register::<CreateRuleBody>("CreateRuleBody")
        .register::<TestRuleBody>("TestRuleBody")
        // Authentication types
        .register::<CreateAuthUserBody>("CreateAuthUserBody")
        // Banned types
        .register::<CreateBanBody>("CreateBanBody")
        // Node & Cluster types
        .register::<NodeInfo>("NodeInfo")
        .register::<ListNodesResponse>("ListNodesResponse")
        .register::<ClusterStatus>("ClusterStatus")
        // Client types
        .register::<ClientInfo>("ClientInfo")
        .register::<ListClientsResponse>("ListClientsResponse")
        // Subscription types
        .register::<SubscriptionInfo>("SubscriptionInfo")
        .register::<ListSubscriptionsResponse>("ListSubscriptionsResponse")
        // Rules types
        .register::<RuleInfo>("RuleInfo")
        .register::<RuleAction>("RuleAction")
        .register::<ListRulesResponse>("ListRulesResponse")
        .register::<TestRuleResponse>("TestRuleResponse")
        // Authentication types
        .register::<AuthUser>("AuthUser")
        .register::<Vec<AuthUser>>("Vec<AuthUser>")
        .register::<AuthenticatorInfo>("AuthenticatorInfo")
        .register::<ListAuthenticatorsResponse>("ListAuthenticatorsResponse")
        // Authorization types
        .register::<AuthzSourceInfo>("AuthzSourceInfo")
        .register::<ListAuthzSourcesResponse>("ListAuthzSourcesResponse")
        // Listener types
        .register::<ListenerInfo>("ListenerInfo")
        .register::<ListListenersResponse>("ListListenersResponse")
        // Metrics types
        .register::<MetricsInfo>("MetricsInfo")
        .register::<ListMetricsResponse>("ListMetricsResponse")
        // Stats types
        .register::<StatsInfo>("StatsInfo")
        .register::<ListStatsResponse>("ListStatsResponse")
        // Topic types
        .register::<TopicInfo>("TopicInfo")
        .register::<ListTopicsResponse>("ListTopicsResponse")
        // Retained message types
        .register::<RetainedMessage>("RetainedMessage")
        .register::<ListRetainedResponse>("ListRetainedResponse")
        // Alarm types
        .register::<AlarmInfo>("AlarmInfo")
        .register::<ListAlarmsResponse>("ListAlarmsResponse")
        // Banned types
        .register::<BanInfo>("BanInfo")
        .register::<ListBannedResponse>("ListBannedResponse")
        // Common types
        .register::<PaginationMeta>("PaginationMeta")
}

/// Creates the EMQX REST API definition with Basic Authentication.
///
/// Uses HTTP Basic Auth with an API key (username) and secret (password).
/// API keys are created via the EMQX Dashboard under **System > API Key**.
///
/// ## Authentication
///
/// The API key is used as the username and the secret as the password:
///
/// ```bash
/// curl -u {EMQX_API_KEY}:{EMQX_API_SECRET} http://localhost:18083/api/v5/nodes
/// ```
///
/// ## Environment Variables
///
/// - `EMQX_API_KEY` - The API key (used as Basic Auth username)
/// - `EMQX_API_SECRET` - The API secret (used as Basic Auth password)
///
/// ## Examples
///
/// ```rust
/// use schematic_definitions::emqx::define_emqx_basic_api;
///
/// let api = define_emqx_basic_api();
/// assert_eq!(api.name, "EmqxBasic");
/// assert!(api.endpoints.len() >= 30);
/// ```
pub fn define_emqx_basic_api() -> RestApi {
    RestApi {
        name: "EmqxBasic".to_string(),
        description: "EMQX Broker REST API with Basic Authentication (API Key + Secret)"
            .to_string(),
        base_url: "http://localhost:18083/api/v5".to_string(),
        docs_url: Some("https://docs.emqx.com/en/emqx/latest/admin/api.html".to_string()),
        auth: AuthStrategy::Basic,
        auth_policy: None,
        env_auth: vec!["EMQX_API_SECRET".to_string()],
        env_username: Some("EMQX_API_KEY".to_string()),
        headers: vec![],
        endpoints: build_common_endpoints(),
        module_path: Some("emqx".to_string()),
        request_suffix: Some("BasicRequest".to_string()),
        version: None,
        env_mapping: Some(EnvMapping {
            basic_user: Some(EnvList::single("EMQX_API_KEY")),
            basic_pass: Some(EnvList::single("EMQX_API_SECRET")),
            ..Default::default()
        }),
    }
}

/// Creates the EMQX REST API definition with Bearer Token authentication.
///
/// Uses JWT tokens obtained from the `/login` endpoint. This is suitable for
/// interactive sessions and dashboard integration.
///
/// ## Authentication Flow
///
/// 1. Call `/login` with username and password to get a token
/// 2. Use the token in subsequent requests via `Authorization: Bearer {token}`
///
/// ## Examples
///
/// ```rust
/// use schematic_definitions::emqx::define_emqx_bearer_api;
///
/// let api = define_emqx_bearer_api();
/// assert_eq!(api.name, "EmqxBearer");
/// // Bearer API has login/logout endpoints plus all common endpoints
/// assert!(api.endpoints.len() > 30);
/// ```
pub fn define_emqx_bearer_api() -> RestApi {
    let mut endpoints = vec![
        // Login endpoint (unique to Bearer API)
        Endpoint {
            id: "Login".to_string(),
            method: RestMethod::Post,
            path: "/login".to_string(),
            description: "Authenticate with username/password and receive a JWT token".to_string(),
            request: Some(ApiRequest::json_type("LoginBody")),
            response: ApiResponse::json_type("LoginResponse"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        // Logout endpoint
        Endpoint {
            id: "Logout".to_string(),
            method: RestMethod::Post,
            path: "/logout".to_string(),
            description: "Invalidate the current bearer token".to_string(),
            request: None,
            response: ApiResponse::Empty,
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
    ];
    endpoints.extend(build_common_endpoints());

    RestApi {
        name: "EmqxBearer".to_string(),
        description: "EMQX Broker REST API with Bearer Token authentication (JWT)".to_string(),
        base_url: "http://localhost:18083/api/v5".to_string(),
        docs_url: Some("https://docs.emqx.com/en/emqx/latest/admin/api.html".to_string()),
        auth: AuthStrategy::BearerToken { header: None },
        auth_policy: None,
        env_auth: vec!["EMQX_TOKEN".to_string()],
        env_username: None,
        headers: vec![],
        endpoints,
        module_path: Some("emqx".to_string()),
        request_suffix: Some("BearerRequest".to_string()),
        version: None,
        env_mapping: Some(EnvMapping {
            bearer_token: Some(EnvList::single("EMQX_TOKEN")),
            ..Default::default()
        }),
    }
}

/// Build the common endpoints shared by both Basic and Bearer API variants.
fn build_common_endpoints() -> Vec<Endpoint> {
    let mut endpoints = Vec::new();
    endpoints.extend(endpoints::monitoring_endpoints());
    endpoints.extend(endpoints::client_endpoints());
    endpoints.extend(endpoints::messaging_endpoints());
    endpoints.extend(endpoints::rules_endpoints());
    endpoints.extend(endpoints::auth_endpoints());
    endpoints.extend(endpoints::banned_endpoints());
    endpoints
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Basic Auth API Tests
    // =========================================================================

    #[test]
    fn basic_api_has_correct_metadata() {
        let api = define_emqx_basic_api();

        assert_eq!(api.name, "EmqxBasic");
        assert_eq!(api.base_url, "http://localhost:18083/api/v5");
        assert!(api.docs_url.is_some());
    }

    #[test]
    fn basic_api_uses_basic_auth() {
        let api = define_emqx_basic_api();

        assert!(matches!(api.auth, AuthStrategy::Basic));
        assert!(api.env_auth.contains(&"EMQX_API_SECRET".to_string()));
        assert_eq!(api.env_username, Some("EMQX_API_KEY".to_string()));
    }

    #[test]
    fn basic_api_has_expected_endpoint_count() {
        let api = define_emqx_basic_api();
        // 36 common endpoints
        assert_eq!(api.endpoints.len(), 36);
    }

    #[test]
    fn basic_api_does_not_have_login() {
        let api = define_emqx_basic_api();
        let login = api.endpoints.iter().find(|e| e.id == "Login");
        assert!(login.is_none());
    }

    // =========================================================================
    // Bearer Token API Tests
    // =========================================================================

    #[test]
    fn bearer_api_has_correct_metadata() {
        let api = define_emqx_bearer_api();

        assert_eq!(api.name, "EmqxBearer");
        assert_eq!(api.base_url, "http://localhost:18083/api/v5");
        assert!(api.docs_url.is_some());
    }

    #[test]
    fn bearer_api_uses_bearer_auth() {
        let api = define_emqx_bearer_api();

        assert!(matches!(api.auth, AuthStrategy::BearerToken { .. }));
        assert!(api.env_auth.contains(&"EMQX_TOKEN".to_string()));
        assert!(api.env_username.is_none());
    }

    #[test]
    fn bearer_api_has_login_endpoint() {
        let api = define_emqx_bearer_api();
        let login = api.endpoints.iter().find(|e| e.id == "Login");

        assert!(login.is_some());
        let login = login.unwrap();
        assert_eq!(login.method, RestMethod::Post);
        assert_eq!(login.path, "/login");
        assert!(login.request.is_some());
    }

    #[test]
    fn bearer_api_has_logout_endpoint() {
        let api = define_emqx_bearer_api();
        let logout = api.endpoints.iter().find(|e| e.id == "Logout");

        assert!(logout.is_some());
        let logout = logout.unwrap();
        assert_eq!(logout.method, RestMethod::Post);
        assert_eq!(logout.path, "/logout");
        assert!(matches!(logout.response, ApiResponse::Empty));
    }

    #[test]
    fn bearer_api_has_more_endpoints() {
        let basic = define_emqx_basic_api();
        let bearer = define_emqx_bearer_api();

        // Bearer has 2 additional endpoints (Login, Logout)
        assert_eq!(bearer.endpoints.len(), basic.endpoints.len() + 2);
    }

    // =========================================================================
    // Node & Cluster Endpoint Tests
    // =========================================================================

    #[test]
    fn list_nodes_endpoint() {
        let api = define_emqx_basic_api();
        let endpoint = api.endpoints.iter().find(|e| e.id == "ListNodes").unwrap();

        assert_eq!(endpoint.method, RestMethod::Get);
        assert_eq!(endpoint.path, "/nodes");
        assert!(endpoint.request.is_none());
    }

    #[test]
    fn get_node_endpoint() {
        let api = define_emqx_basic_api();
        let endpoint = api.endpoints.iter().find(|e| e.id == "GetNode").unwrap();

        assert_eq!(endpoint.method, RestMethod::Get);
        assert_eq!(endpoint.path, "/nodes/{node}");
    }

    #[test]
    fn get_cluster_endpoint() {
        let api = define_emqx_basic_api();
        let endpoint = api.endpoints.iter().find(|e| e.id == "GetCluster").unwrap();

        assert_eq!(endpoint.method, RestMethod::Get);
        assert_eq!(endpoint.path, "/cluster");
    }

    // =========================================================================
    // Client Endpoint Tests
    // =========================================================================

    #[test]
    fn list_clients_endpoint() {
        let api = define_emqx_basic_api();
        let endpoint = api
            .endpoints
            .iter()
            .find(|e| e.id == "ListClients")
            .unwrap();

        assert_eq!(endpoint.method, RestMethod::Get);
        assert_eq!(endpoint.path, "/clients");
    }

    #[test]
    fn get_client_endpoint() {
        let api = define_emqx_basic_api();
        let endpoint = api.endpoints.iter().find(|e| e.id == "GetClient").unwrap();

        assert_eq!(endpoint.method, RestMethod::Get);
        assert_eq!(endpoint.path, "/clients/{clientid}");
    }

    #[test]
    fn disconnect_client_endpoint() {
        let api = define_emqx_basic_api();
        let endpoint = api
            .endpoints
            .iter()
            .find(|e| e.id == "DisconnectClient")
            .unwrap();

        assert_eq!(endpoint.method, RestMethod::Delete);
        assert_eq!(endpoint.path, "/clients/{clientid}");
        assert!(matches!(endpoint.response, ApiResponse::Empty));
    }

    #[test]
    fn subscribe_client_endpoint() {
        let api = define_emqx_basic_api();
        let endpoint = api
            .endpoints
            .iter()
            .find(|e| e.id == "SubscribeClient")
            .unwrap();

        assert_eq!(endpoint.method, RestMethod::Post);
        assert_eq!(endpoint.path, "/clients/{clientid}/subscribe");
        assert!(endpoint.request.is_some());
    }

    // =========================================================================
    // Publishing Endpoint Tests
    // =========================================================================

    #[test]
    fn publish_endpoint() {
        let api = define_emqx_basic_api();
        let endpoint = api.endpoints.iter().find(|e| e.id == "Publish").unwrap();

        assert_eq!(endpoint.method, RestMethod::Post);
        assert_eq!(endpoint.path, "/publish");
        assert!(endpoint.request.is_some());
    }

    #[test]
    fn publish_bulk_endpoint() {
        let api = define_emqx_basic_api();
        let endpoint = api
            .endpoints
            .iter()
            .find(|e| e.id == "PublishBulk")
            .unwrap();

        assert_eq!(endpoint.method, RestMethod::Post);
        assert_eq!(endpoint.path, "/publish/bulk");
        assert!(endpoint.request.is_some());
    }

    // =========================================================================
    // Rules Engine Endpoint Tests
    // =========================================================================

    #[test]
    fn list_rules_endpoint() {
        let api = define_emqx_basic_api();
        let endpoint = api.endpoints.iter().find(|e| e.id == "ListRules").unwrap();

        assert_eq!(endpoint.method, RestMethod::Get);
        assert_eq!(endpoint.path, "/rules");
    }

    #[test]
    fn create_rule_endpoint() {
        let api = define_emqx_basic_api();
        let endpoint = api.endpoints.iter().find(|e| e.id == "CreateRule").unwrap();

        assert_eq!(endpoint.method, RestMethod::Post);
        assert_eq!(endpoint.path, "/rules");
        assert!(endpoint.request.is_some());
    }

    #[test]
    fn update_rule_endpoint() {
        let api = define_emqx_basic_api();
        let endpoint = api.endpoints.iter().find(|e| e.id == "UpdateRule").unwrap();

        assert_eq!(endpoint.method, RestMethod::Put);
        assert_eq!(endpoint.path, "/rules/{id}");
    }

    #[test]
    fn delete_rule_endpoint() {
        let api = define_emqx_basic_api();
        let endpoint = api.endpoints.iter().find(|e| e.id == "DeleteRule").unwrap();

        assert_eq!(endpoint.method, RestMethod::Delete);
        assert_eq!(endpoint.path, "/rules/{id}");
    }

    #[test]
    fn test_rule_endpoint() {
        let api = define_emqx_basic_api();
        let endpoint = api.endpoints.iter().find(|e| e.id == "TestRule").unwrap();

        assert_eq!(endpoint.method, RestMethod::Post);
        assert_eq!(endpoint.path, "/rules/{id}/test");
    }

    // =========================================================================
    // Authentication Endpoint Tests
    // =========================================================================

    #[test]
    fn list_authenticators_endpoint() {
        let api = define_emqx_basic_api();
        let endpoint = api
            .endpoints
            .iter()
            .find(|e| e.id == "ListAuthenticators")
            .unwrap();

        assert_eq!(endpoint.method, RestMethod::Get);
        assert_eq!(endpoint.path, "/authentication");
    }

    #[test]
    fn create_auth_user_endpoint() {
        let api = define_emqx_basic_api();
        let endpoint = api
            .endpoints
            .iter()
            .find(|e| e.id == "CreateAuthUser")
            .unwrap();

        assert_eq!(endpoint.method, RestMethod::Post);
        assert_eq!(endpoint.path, "/authentication/{id}/users");
        assert!(endpoint.request.is_some());
    }

    // =========================================================================
    // Listener Endpoint Tests
    // =========================================================================

    #[test]
    fn list_listeners_endpoint() {
        let api = define_emqx_basic_api();
        let endpoint = api
            .endpoints
            .iter()
            .find(|e| e.id == "ListListeners")
            .unwrap();

        assert_eq!(endpoint.method, RestMethod::Get);
        assert_eq!(endpoint.path, "/listeners");
    }

    // =========================================================================
    // Metrics & Stats Endpoint Tests
    // =========================================================================

    #[test]
    fn list_metrics_endpoint() {
        let api = define_emqx_basic_api();
        let endpoint = api
            .endpoints
            .iter()
            .find(|e| e.id == "ListMetrics")
            .unwrap();

        assert_eq!(endpoint.method, RestMethod::Get);
        assert_eq!(endpoint.path, "/metrics");
    }

    #[test]
    fn prometheus_endpoint() {
        let api = define_emqx_basic_api();
        let endpoint = api
            .endpoints
            .iter()
            .find(|e| e.id == "GetPrometheus")
            .unwrap();

        assert_eq!(endpoint.method, RestMethod::Get);
        assert_eq!(endpoint.path, "/prometheus/stats");
        assert!(matches!(endpoint.response, ApiResponse::Text));
    }

    // =========================================================================
    // Retained Messages Endpoint Tests
    // =========================================================================

    #[test]
    fn list_retained_endpoint() {
        let api = define_emqx_basic_api();
        let endpoint = api
            .endpoints
            .iter()
            .find(|e| e.id == "ListRetained")
            .unwrap();

        assert_eq!(endpoint.method, RestMethod::Get);
        assert_eq!(endpoint.path, "/retainer/messages");
    }

    #[test]
    fn delete_retained_endpoint() {
        let api = define_emqx_basic_api();
        let endpoint = api
            .endpoints
            .iter()
            .find(|e| e.id == "DeleteRetained")
            .unwrap();

        assert_eq!(endpoint.method, RestMethod::Delete);
        assert_eq!(endpoint.path, "/retainer/messages/{topic}");
    }

    // =========================================================================
    // Banned Endpoint Tests
    // =========================================================================

    #[test]
    fn create_ban_endpoint() {
        let api = define_emqx_basic_api();
        let endpoint = api.endpoints.iter().find(|e| e.id == "CreateBan").unwrap();

        assert_eq!(endpoint.method, RestMethod::Post);
        assert_eq!(endpoint.path, "/banned");
        assert!(endpoint.request.is_some());
    }

    #[test]
    fn delete_ban_endpoint() {
        let api = define_emqx_basic_api();
        let endpoint = api.endpoints.iter().find(|e| e.id == "DeleteBan").unwrap();

        assert_eq!(endpoint.method, RestMethod::Delete);
        assert_eq!(endpoint.path, "/banned/{ban_type}/{who}");
    }

    // =========================================================================
    // Cross-API Tests
    // =========================================================================

    #[test]
    fn both_apis_share_base_url() {
        let basic = define_emqx_basic_api();
        let bearer = define_emqx_bearer_api();

        assert_eq!(basic.base_url, bearer.base_url);
    }

    #[test]
    fn apis_have_distinct_names() {
        let basic = define_emqx_basic_api();
        let bearer = define_emqx_bearer_api();

        assert_ne!(basic.name, bearer.name);
    }

    #[test]
    fn both_apis_have_same_common_endpoints() {
        let basic = define_emqx_basic_api();
        let bearer = define_emqx_bearer_api();

        // Check that all basic endpoints exist in bearer (excluding auth-specific ones)
        for basic_endpoint in &basic.endpoints {
            let found = bearer.endpoints.iter().any(|e| e.id == basic_endpoint.id);
            assert!(
                found,
                "Endpoint {} missing in bearer API",
                basic_endpoint.id
            );
        }
    }

    #[test]
    fn basic_api_has_module_path() {
        let api = define_emqx_basic_api();
        assert_eq!(api.module_path, Some("emqx".to_string()));
    }

    #[test]
    fn bearer_api_has_module_path() {
        let api = define_emqx_bearer_api();
        assert_eq!(api.module_path, Some("emqx".to_string()));
    }

    #[test]
    fn apis_have_different_request_suffixes() {
        let basic = define_emqx_basic_api();
        let bearer = define_emqx_bearer_api();

        assert_eq!(basic.request_suffix, Some("BasicRequest".to_string()));
        assert_eq!(bearer.request_suffix, Some("BearerRequest".to_string()));
    }
}
