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

mod types;

pub use types::*;

use schematic_define::{ApiRequest, ApiResponse, AuthStrategy, Endpoint, RestApi, RestMethod};

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
/// ## Endpoints
///
/// | ID | Method | Path | Description |
/// |----|--------|------|-------------|
/// | ListNodes | GET | /nodes | List all nodes in the cluster |
/// | GetNode | GET | /nodes/{node} | Get specific node details |
/// | GetCluster | GET | /cluster | Get cluster status |
/// | ListClients | GET | /clients | List connected clients |
/// | GetClient | GET | /clients/{clientid} | Get client details |
/// | DisconnectClient | DELETE | /clients/{clientid} | Disconnect a client |
/// | SubscribeClient | POST | /clients/{clientid}/subscribe | Subscribe client to topic |
/// | UnsubscribeClient | POST | /clients/{clientid}/unsubscribe | Unsubscribe client from topic |
/// | ListSubscriptions | GET | /subscriptions | List all subscriptions |
/// | Publish | POST | /publish | Publish a message |
/// | PublishBulk | POST | /publish/bulk | Publish multiple messages |
/// | ListRules | GET | /rules | List all rules |
/// | CreateRule | POST | /rules | Create a new rule |
/// | GetRule | GET | /rules/{id} | Get rule details |
/// | UpdateRule | PUT | /rules/{id} | Update a rule |
/// | DeleteRule | DELETE | /rules/{id} | Delete a rule |
/// | TestRule | POST | /rules/{id}/test | Test a rule |
/// | ListAuthenticators | GET | /authentication | List authenticators |
/// | GetAuthenticator | GET | /authentication/{id} | Get authenticator details |
/// | ListAuthUsers | GET | /authentication/{id}/users | List auth users |
/// | CreateAuthUser | POST | /authentication/{id}/users | Create auth user |
/// | DeleteAuthUser | DELETE | /authentication/{id}/users/{user_id} | Delete auth user |
/// | ListAuthzSources | GET | /authorization/sources | List authorization sources |
/// | ListListeners | GET | /listeners | List all listeners |
/// | GetListener | GET | /listeners/{id} | Get listener details |
/// | ListMetrics | GET | /metrics | Get broker metrics |
/// | ListStats | GET | /stats | Get broker statistics |
/// | ListTopics | GET | /topics | List active topics |
/// | ListRetained | GET | /retainer/messages | List retained messages |
/// | GetRetained | GET | /retainer/messages/{topic} | Get specific retained message |
/// | DeleteRetained | DELETE | /retainer/messages/{topic} | Delete retained message |
/// | ListAlarms | GET | /alarms | List active alarms |
/// | ListBanned | GET | /banned | List banned clients |
/// | CreateBan | POST | /banned | Ban a client |
/// | DeleteBan | DELETE | /banned/{as}/{who} | Remove a ban |
/// | GetPrometheus | GET | /prometheus/stats | Get Prometheus metrics |
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
        env_auth: vec!["EMQX_API_SECRET".to_string()],
        env_username: Some("EMQX_API_KEY".to_string()),
        headers: vec![],
        endpoints: build_common_endpoints(),
        module_path: Some("emqx".to_string()),
        request_suffix: Some("BasicRequest".to_string()),
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
/// ```bash
/// # Get token
/// curl -X POST http://localhost:18083/api/v5/login \
///   -H "Content-Type: application/json" \
///   -d '{"username":"admin","password":"public"}'
///
/// # Use token
/// curl -H "Authorization: Bearer {token}" http://localhost:18083/api/v5/nodes
/// ```
///
/// ## Endpoints
///
/// Includes all endpoints from [`define_emqx_basic_api`] plus:
///
/// | ID | Method | Path | Description |
/// |----|--------|------|-------------|
/// | Login | POST | /login | Authenticate and get JWT token |
/// | Logout | POST | /logout | Invalidate the current token |
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
        },
    ];
    endpoints.extend(build_common_endpoints());

    RestApi {
        name: "EmqxBearer".to_string(),
        description: "EMQX Broker REST API with Bearer Token authentication (JWT)".to_string(),
        base_url: "http://localhost:18083/api/v5".to_string(),
        docs_url: Some("https://docs.emqx.com/en/emqx/latest/admin/api.html".to_string()),
        auth: AuthStrategy::BearerToken { header: None },
        env_auth: vec!["EMQX_TOKEN".to_string()],
        env_username: None,
        headers: vec![],
        endpoints,
        module_path: Some("emqx".to_string()),
        request_suffix: Some("BearerRequest".to_string()),
    }
}

/// Build the common endpoints shared by both Basic and Bearer API variants.
fn build_common_endpoints() -> Vec<Endpoint> {
    vec![
        // =====================================================================
        // Node & Cluster Endpoints
        // =====================================================================
        Endpoint {
            id: "ListNodes".to_string(),
            method: RestMethod::Get,
            path: "/nodes".to_string(),
            description: "List all nodes in the EMQX cluster".to_string(),
            request: None,
            response: ApiResponse::json_type("ListNodesResponse"),
            headers: vec![],
        },
        Endpoint {
            id: "GetNode".to_string(),
            method: RestMethod::Get,
            path: "/nodes/{node}".to_string(),
            description: "Get detailed information about a specific node".to_string(),
            request: None,
            response: ApiResponse::json_type("NodeInfo"),
            headers: vec![],
        },
        Endpoint {
            id: "GetCluster".to_string(),
            method: RestMethod::Get,
            path: "/cluster".to_string(),
            description: "Get cluster status and node membership".to_string(),
            request: None,
            response: ApiResponse::json_type("ClusterStatus"),
            headers: vec![],
        },
        // =====================================================================
        // Client Endpoints
        // =====================================================================
        Endpoint {
            id: "ListClients".to_string(),
            method: RestMethod::Get,
            path: "/clients".to_string(),
            description: "List connected MQTT clients with pagination".to_string(),
            request: None,
            response: ApiResponse::json_type("ListClientsResponse"),
            headers: vec![],
        },
        Endpoint {
            id: "GetClient".to_string(),
            method: RestMethod::Get,
            path: "/clients/{clientid}".to_string(),
            description: "Get detailed information about a specific client".to_string(),
            request: None,
            response: ApiResponse::json_type("ClientInfo"),
            headers: vec![],
        },
        Endpoint {
            id: "DisconnectClient".to_string(),
            method: RestMethod::Delete,
            path: "/clients/{clientid}".to_string(),
            description: "Forcefully disconnect a client from the broker".to_string(),
            request: None,
            response: ApiResponse::Empty,
            headers: vec![],
        },
        Endpoint {
            id: "SubscribeClient".to_string(),
            method: RestMethod::Post,
            path: "/clients/{clientid}/subscribe".to_string(),
            description: "Create a subscription for a connected client".to_string(),
            request: Some(ApiRequest::json_type("SubscribeBody")),
            response: ApiResponse::Empty,
            headers: vec![],
        },
        Endpoint {
            id: "UnsubscribeClient".to_string(),
            method: RestMethod::Post,
            path: "/clients/{clientid}/unsubscribe".to_string(),
            description: "Remove a subscription from a connected client".to_string(),
            request: Some(ApiRequest::json_type("SubscribeBody")),
            response: ApiResponse::Empty,
            headers: vec![],
        },
        // =====================================================================
        // Subscription Endpoints
        // =====================================================================
        Endpoint {
            id: "ListSubscriptions".to_string(),
            method: RestMethod::Get,
            path: "/subscriptions".to_string(),
            description: "List all subscriptions across the cluster".to_string(),
            request: None,
            response: ApiResponse::json_type("ListSubscriptionsResponse"),
            headers: vec![],
        },
        // =====================================================================
        // Publishing Endpoints
        // =====================================================================
        Endpoint {
            id: "Publish".to_string(),
            method: RestMethod::Post,
            path: "/publish".to_string(),
            description: "Publish an MQTT message to a topic".to_string(),
            request: Some(ApiRequest::json_type("PublishBody")),
            response: ApiResponse::Empty,
            headers: vec![],
        },
        Endpoint {
            id: "PublishBulk".to_string(),
            method: RestMethod::Post,
            path: "/publish/bulk".to_string(),
            description: "Publish multiple MQTT messages in a single request".to_string(),
            request: Some(ApiRequest::json_type("PublishBatchBody")),
            response: ApiResponse::Empty,
            headers: vec![],
        },
        // =====================================================================
        // Rules Engine Endpoints
        // =====================================================================
        Endpoint {
            id: "ListRules".to_string(),
            method: RestMethod::Get,
            path: "/rules".to_string(),
            description: "List all rules in the rules engine".to_string(),
            request: None,
            response: ApiResponse::json_type("ListRulesResponse"),
            headers: vec![],
        },
        Endpoint {
            id: "CreateRule".to_string(),
            method: RestMethod::Post,
            path: "/rules".to_string(),
            description: "Create a new rule in the rules engine".to_string(),
            request: Some(ApiRequest::json_type("CreateRuleBody")),
            response: ApiResponse::json_type("RuleInfo"),
            headers: vec![],
        },
        Endpoint {
            id: "GetRule".to_string(),
            method: RestMethod::Get,
            path: "/rules/{id}".to_string(),
            description: "Get details of a specific rule".to_string(),
            request: None,
            response: ApiResponse::json_type("RuleInfo"),
            headers: vec![],
        },
        Endpoint {
            id: "UpdateRule".to_string(),
            method: RestMethod::Put,
            path: "/rules/{id}".to_string(),
            description: "Update an existing rule".to_string(),
            request: Some(ApiRequest::json_type("CreateRuleBody")),
            response: ApiResponse::json_type("RuleInfo"),
            headers: vec![],
        },
        Endpoint {
            id: "DeleteRule".to_string(),
            method: RestMethod::Delete,
            path: "/rules/{id}".to_string(),
            description: "Delete a rule from the rules engine".to_string(),
            request: None,
            response: ApiResponse::Empty,
            headers: vec![],
        },
        Endpoint {
            id: "TestRule".to_string(),
            method: RestMethod::Post,
            path: "/rules/{id}/test".to_string(),
            description: "Test a rule with sample data".to_string(),
            request: Some(ApiRequest::json_type("TestRuleBody")),
            response: ApiResponse::json_type("TestRuleResponse"),
            headers: vec![],
        },
        // =====================================================================
        // Authentication Endpoints
        // =====================================================================
        Endpoint {
            id: "ListAuthenticators".to_string(),
            method: RestMethod::Get,
            path: "/authentication".to_string(),
            description: "List all configured authentication providers".to_string(),
            request: None,
            response: ApiResponse::json_type("ListAuthenticatorsResponse"),
            headers: vec![],
        },
        Endpoint {
            id: "GetAuthenticator".to_string(),
            method: RestMethod::Get,
            path: "/authentication/{id}".to_string(),
            description: "Get details of a specific authenticator".to_string(),
            request: None,
            response: ApiResponse::json_type("AuthenticatorInfo"),
            headers: vec![],
        },
        Endpoint {
            id: "ListAuthUsers".to_string(),
            method: RestMethod::Get,
            path: "/authentication/{id}/users".to_string(),
            description: "List users in a built-in database authenticator".to_string(),
            request: None,
            response: ApiResponse::json_type("Vec<AuthUser>"),
            headers: vec![],
        },
        Endpoint {
            id: "CreateAuthUser".to_string(),
            method: RestMethod::Post,
            path: "/authentication/{id}/users".to_string(),
            description: "Create a new user in a built-in database authenticator".to_string(),
            request: Some(ApiRequest::json_type("CreateAuthUserBody")),
            response: ApiResponse::json_type("AuthUser"),
            headers: vec![],
        },
        Endpoint {
            id: "DeleteAuthUser".to_string(),
            method: RestMethod::Delete,
            path: "/authentication/{id}/users/{user_id}".to_string(),
            description: "Delete a user from a built-in database authenticator".to_string(),
            request: None,
            response: ApiResponse::Empty,
            headers: vec![],
        },
        // =====================================================================
        // Authorization Endpoints
        // =====================================================================
        Endpoint {
            id: "ListAuthzSources".to_string(),
            method: RestMethod::Get,
            path: "/authorization/sources".to_string(),
            description: "List all authorization sources".to_string(),
            request: None,
            response: ApiResponse::json_type("ListAuthzSourcesResponse"),
            headers: vec![],
        },
        // =====================================================================
        // Listener Endpoints
        // =====================================================================
        Endpoint {
            id: "ListListeners".to_string(),
            method: RestMethod::Get,
            path: "/listeners".to_string(),
            description: "List all configured listeners".to_string(),
            request: None,
            response: ApiResponse::json_type("ListListenersResponse"),
            headers: vec![],
        },
        Endpoint {
            id: "GetListener".to_string(),
            method: RestMethod::Get,
            path: "/listeners/{id}".to_string(),
            description: "Get details of a specific listener".to_string(),
            request: None,
            response: ApiResponse::json_type("ListenerInfo"),
            headers: vec![],
        },
        // =====================================================================
        // Metrics & Stats Endpoints
        // =====================================================================
        Endpoint {
            id: "ListMetrics".to_string(),
            method: RestMethod::Get,
            path: "/metrics".to_string(),
            description: "Get broker metrics for all nodes".to_string(),
            request: None,
            response: ApiResponse::json_type("ListMetricsResponse"),
            headers: vec![],
        },
        Endpoint {
            id: "ListStats".to_string(),
            method: RestMethod::Get,
            path: "/stats".to_string(),
            description: "Get broker statistics for all nodes".to_string(),
            request: None,
            response: ApiResponse::json_type("ListStatsResponse"),
            headers: vec![],
        },
        Endpoint {
            id: "GetPrometheus".to_string(),
            method: RestMethod::Get,
            path: "/prometheus/stats".to_string(),
            description: "Get metrics in Prometheus format".to_string(),
            request: None,
            response: ApiResponse::Text,
            headers: vec![],
        },
        // =====================================================================
        // Topics Endpoints
        // =====================================================================
        Endpoint {
            id: "ListTopics".to_string(),
            method: RestMethod::Get,
            path: "/topics".to_string(),
            description: "List active topics in the broker".to_string(),
            request: None,
            response: ApiResponse::json_type("ListTopicsResponse"),
            headers: vec![],
        },
        // =====================================================================
        // Retained Messages Endpoints
        // =====================================================================
        Endpoint {
            id: "ListRetained".to_string(),
            method: RestMethod::Get,
            path: "/retainer/messages".to_string(),
            description: "List retained messages with pagination".to_string(),
            request: None,
            response: ApiResponse::json_type("ListRetainedResponse"),
            headers: vec![],
        },
        Endpoint {
            id: "GetRetained".to_string(),
            method: RestMethod::Get,
            path: "/retainer/messages/{topic}".to_string(),
            description: "Get a specific retained message by topic".to_string(),
            request: None,
            response: ApiResponse::json_type("RetainedMessage"),
            headers: vec![],
        },
        Endpoint {
            id: "DeleteRetained".to_string(),
            method: RestMethod::Delete,
            path: "/retainer/messages/{topic}".to_string(),
            description: "Delete a retained message".to_string(),
            request: None,
            response: ApiResponse::Empty,
            headers: vec![],
        },
        // =====================================================================
        // Alarms Endpoints
        // =====================================================================
        Endpoint {
            id: "ListAlarms".to_string(),
            method: RestMethod::Get,
            path: "/alarms".to_string(),
            description: "List active alarms".to_string(),
            request: None,
            response: ApiResponse::json_type("ListAlarmsResponse"),
            headers: vec![],
        },
        // =====================================================================
        // Banned Clients Endpoints
        // =====================================================================
        Endpoint {
            id: "ListBanned".to_string(),
            method: RestMethod::Get,
            path: "/banned".to_string(),
            description: "List all banned clients, usernames, and hosts".to_string(),
            request: None,
            response: ApiResponse::json_type("ListBannedResponse"),
            headers: vec![],
        },
        Endpoint {
            id: "CreateBan".to_string(),
            method: RestMethod::Post,
            path: "/banned".to_string(),
            description: "Ban a client, username, or host".to_string(),
            request: Some(ApiRequest::json_type("CreateBanBody")),
            response: ApiResponse::json_type("BanInfo"),
            headers: vec![],
        },
        Endpoint {
            id: "DeleteBan".to_string(),
            method: RestMethod::Delete,
            path: "/banned/{ban_type}/{who}".to_string(),
            description: "Remove a ban by type (clientid, username, peerhost) and value".to_string(),
            request: None,
            response: ApiResponse::Empty,
            headers: vec![],
        },
    ]
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
        let endpoint = api.endpoints.iter().find(|e| e.id == "ListClients").unwrap();

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
        let endpoint = api.endpoints.iter().find(|e| e.id == "PublishBulk").unwrap();

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
        let endpoint = api.endpoints.iter().find(|e| e.id == "ListMetrics").unwrap();

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
        let endpoint = api.endpoints.iter().find(|e| e.id == "ListRetained").unwrap();

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
            assert!(found, "Endpoint {} missing in bearer API", basic_endpoint.id);
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
