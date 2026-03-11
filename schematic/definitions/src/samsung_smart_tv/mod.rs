//! Samsung Smart TV API definitions (S95C-focused modern Tizen subset).
//!
//! This module defines the Samsung Smart TV LAN control APIs:
//!
//! - **REST API** (`SamsungSmartTv`): Smart View HTTP API on port 8001 for device
//!   info and application launch.
//! - **WebSocket API** (`SamsungSmartTvRemote`): Remote control channel on port 8002
//!   for key transport and event monitoring.
//!
//! ## Endpoint Groups
//!
//! | Group | Protocol | Port | Path Root |
//! |-------|----------|------|-----------|
//! | Device Info | REST | 8001 | `/api/v2/` |
//! | Server Logs | REST | 8001 | `/logs/` |
//! | App Launch | REST | 8001 | `/api/v2/applications/`, `/ws/apps/` |
//! | Remote Control | WebSocket | 8002 | `/api/v2/channels/samsung.remote.control` |

mod types;

pub mod remote_ws;

pub use remote_ws::define_samsung_smart_tv_remote_ws_api;
pub use types::*;

use crate::registry::SchemaRegistry;
use schematic_define::{ApiResponse, AuthStrategy, Endpoint, EnvMapping, RestApi, RestMethod};

/// Creates a schema registry containing Samsung Smart TV REST response types.
///
/// This registry is used for OpenAPI export of the `SamsungSmartTv` REST API.
/// The `SamsungDeviceInfo` nested type is registered explicitly so `$ref`
/// resolution remains complete in generated component schemas.
///
/// ## Examples
///
/// ```
/// use schematic_definitions::samsung_smart_tv::{define_samsung_smart_tv_api, openapi_registry};
///
/// let registry = openapi_registry();
/// let api = define_samsung_smart_tv_api();
///
/// assert!(registry.get("SamsungDeviceInfoResponse").is_some());
/// assert!(registry.get("SamsungDeviceInfo").is_some());
/// assert!(registry.validate_completeness(&api).is_ok());
/// ```
#[must_use]
pub fn openapi_registry() -> SchemaRegistry {
    SchemaRegistry::new()
        .register::<SamsungDeviceInfoResponse>("SamsungDeviceInfoResponse")
        .register::<SamsungDeviceInfo>("SamsungDeviceInfo")
        .register::<SamsungAppStatusResponse>("SamsungAppStatusResponse")
}

/// Creates the Samsung Smart TV REST API definition.
///
/// Models the Smart View HTTP API on port 8001 with four endpoints covering
/// device identity, debug logs, and application launch. No authentication
/// is required for these HTTP endpoints; approval/token semantics are
/// handled on the companion WebSocket channel.
///
/// ## Examples
///
/// ```rust
/// use schematic_definitions::samsung_smart_tv::define_samsung_smart_tv_api;
///
/// let api = define_samsung_smart_tv_api();
/// assert_eq!(api.name, "SamsungSmartTv");
/// assert_eq!(api.endpoints.len(), 7);
/// ```
#[must_use]
pub fn define_samsung_smart_tv_api() -> RestApi {
    RestApi {
        name: "SamsungSmartTv".to_string(),
        description: "Samsung Smart TV LAN API (S95C-focused modern Tizen subset)".to_string(),
        base_url: "http://192.168.1.1:8001".to_string(),
        docs_url: Some(
            "https://developer.samsung.com/smarttv/develop/extension-libraries/smart-view-sdk/receiver-apps/debugging.html".to_string(),
        ),
        auth: AuthStrategy::None,
        auth_policy: None,
        env_auth: vec![],
        env_username: None,
        headers: vec![],
        endpoints: build_samsung_smart_tv_endpoints(),
        module_path: Some("samsung_smart_tv".to_string()),
        request_suffix: None,
        version: None,
        env_mapping: Some(EnvMapping::default()),
    }
}

fn build_samsung_smart_tv_endpoints() -> Vec<Endpoint> {
    vec![
        Endpoint {
            id: "GetDeviceInfo".to_string(),
            method: RestMethod::Get,
            path: "/api/v2/".to_string(),
            description: "Canonical Smart View info endpoint for modern Samsung TVs. \
                Returns device identity, model, firmware, and network details. \
                Payload fields vary by model and firmware revision; unknown fields \
                are preserved via flattened extras. For high-frequency polling, \
                prefer conditional requests or payload diffing to reduce repeated \
                owned-string allocations when firmware does not support ETag-style \
                cache validation."
                .to_string(),
            request: None,
            response: ApiResponse::json_type("SamsungDeviceInfoResponse"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        Endpoint {
            id: "GetServerLogs".to_string(),
            method: RestMethod::Get,
            path: "/logs/".to_string(),
            description: "Debug/developer-facing log endpoint. May require developer mode \
                enabled on the TV and can return non-success status when disabled."
                .to_string(),
            request: None,
            response: ApiResponse::Text,
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        Endpoint {
            id: "LaunchApplicationById".to_string(),
            method: RestMethod::Post,
            path: "/api/v2/applications/{app_id}".to_string(),
            description: "Launch a Samsung application by its package ID. Firmware-dependent \
                support; use when the Samsung app ID is known. Recommended runtime \
                strategy: attempt this endpoint first, fall back to LaunchApplicationByName."
                .to_string(),
            request: None,
            response: ApiResponse::Empty,
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        Endpoint {
            id: "LaunchApplicationByName".to_string(),
            method: RestMethod::Post,
            path: "/ws/apps/{app_name}".to_string(),
            description: "Launch a Samsung application by name. Compatibility fallback for \
                firmware that does not honor /api/v2/applications/{id}."
                .to_string(),
            request: None,
            response: ApiResponse::Empty,
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        Endpoint {
            id: "GetAppStatus".to_string(),
            method: RestMethod::Get,
            path: "/api/v2/applications/{app_id}".to_string(),
            description: "Get the running status and metadata of a specific application. \
                Returns visibility, running state, version, and name."
                .to_string(),
            request: None,
            response: ApiResponse::json_type("SamsungAppStatusResponse"),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        Endpoint {
            id: "CloseApp".to_string(),
            method: RestMethod::Delete,
            path: "/api/v2/applications/{app_id}".to_string(),
            description: "Close a running application by its ID. Sends a DELETE to the \
                application endpoint, which requests the TV terminate the app."
                .to_string(),
            request: None,
            response: ApiResponse::Empty,
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
        Endpoint {
            id: "InstallApp".to_string(),
            method: RestMethod::Put,
            path: "/api/v2/applications/{app_id}".to_string(),
            description: "Install (or reinstall) an application by its ID. The TV downloads \
                and installs the app from the Samsung app store."
                .to_string(),
            request: None,
            response: ApiResponse::Empty,
            headers: vec![],
            params: None,
            oauth_scopes: None,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use schematic_define::AuthStrategy;

    #[test]
    fn openapi_registry_contains_samsung_rest_types() {
        let registry = openapi_registry();
        assert!(registry.get("SamsungDeviceInfoResponse").is_some());
        assert!(registry.get("SamsungDeviceInfo").is_some());
        assert!(registry.get("SamsungAppStatusResponse").is_some());
        assert_eq!(registry.len(), 3);
    }

    #[test]
    fn openapi_registry_validates_against_api() {
        let registry = openapi_registry();
        let api = define_samsung_smart_tv_api();
        let result = registry.validate_completeness(&api);
        assert!(result.is_ok(), "Registry should be complete: {:?}", result);
    }

    #[test]
    fn openapi_registry_converts_to_openapi_schemas() {
        let registry = openapi_registry();
        let schemas = registry.to_openapi_schemas();
        assert!(schemas.contains_key("SamsungDeviceInfoResponse"));
        assert!(schemas.contains_key("SamsungDeviceInfo"));
    }

    #[test]
    fn api_metadata_is_correct() {
        let api = define_samsung_smart_tv_api();
        assert_eq!(api.name, "SamsungSmartTv");
        assert_eq!(api.base_url, "http://192.168.1.1:8001");
        assert!(api.docs_url.is_some());
        assert!(matches!(api.auth, AuthStrategy::None));
        assert_eq!(api.module_path.as_deref(), Some("samsung_smart_tv"));
    }

    #[test]
    fn api_has_seven_endpoints() {
        let api = define_samsung_smart_tv_api();
        assert_eq!(api.endpoints.len(), 7);
    }

    #[test]
    fn get_device_info_endpoint() {
        let api = define_samsung_smart_tv_api();
        let ep = api
            .endpoints
            .iter()
            .find(|e| e.id == "GetDeviceInfo")
            .expect("GetDeviceInfo endpoint must exist");
        assert_eq!(ep.method, RestMethod::Get);
        assert_eq!(ep.path, "/api/v2/");
        assert!(matches!(ep.response, ApiResponse::Json { .. }));
    }

    #[test]
    fn get_server_logs_endpoint() {
        let api = define_samsung_smart_tv_api();
        let ep = api
            .endpoints
            .iter()
            .find(|e| e.id == "GetServerLogs")
            .expect("GetServerLogs endpoint must exist");
        assert_eq!(ep.method, RestMethod::Get);
        assert_eq!(ep.path, "/logs/");
        assert!(matches!(ep.response, ApiResponse::Text));
    }

    #[test]
    fn launch_application_by_id_endpoint() {
        let api = define_samsung_smart_tv_api();
        let ep = api
            .endpoints
            .iter()
            .find(|e| e.id == "LaunchApplicationById")
            .expect("LaunchApplicationById endpoint must exist");
        assert_eq!(ep.method, RestMethod::Post);
        assert_eq!(ep.path, "/api/v2/applications/{app_id}");
        assert!(matches!(ep.response, ApiResponse::Empty));
    }

    #[test]
    fn launch_application_by_name_endpoint() {
        let api = define_samsung_smart_tv_api();
        let ep = api
            .endpoints
            .iter()
            .find(|e| e.id == "LaunchApplicationByName")
            .expect("LaunchApplicationByName endpoint must exist");
        assert_eq!(ep.method, RestMethod::Post);
        assert_eq!(ep.path, "/ws/apps/{app_name}");
        assert!(matches!(ep.response, ApiResponse::Empty));
    }

    #[test]
    fn get_app_status_endpoint() {
        let api = define_samsung_smart_tv_api();
        let ep = api
            .endpoints
            .iter()
            .find(|e| e.id == "GetAppStatus")
            .expect("GetAppStatus endpoint must exist");
        assert_eq!(ep.method, RestMethod::Get);
        assert_eq!(ep.path, "/api/v2/applications/{app_id}");
        assert!(matches!(ep.response, ApiResponse::Json { .. }));
    }

    #[test]
    fn close_app_endpoint() {
        let api = define_samsung_smart_tv_api();
        let ep = api
            .endpoints
            .iter()
            .find(|e| e.id == "CloseApp")
            .expect("CloseApp endpoint must exist");
        assert_eq!(ep.method, RestMethod::Delete);
        assert_eq!(ep.path, "/api/v2/applications/{app_id}");
        assert!(matches!(ep.response, ApiResponse::Empty));
    }

    #[test]
    fn install_app_endpoint() {
        let api = define_samsung_smart_tv_api();
        let ep = api
            .endpoints
            .iter()
            .find(|e| e.id == "InstallApp")
            .expect("InstallApp endpoint must exist");
        assert_eq!(ep.method, RestMethod::Put);
        assert_eq!(ep.path, "/api/v2/applications/{app_id}");
        assert!(matches!(ep.response, ApiResponse::Empty));
    }

    #[test]
    fn export_samsung_api_to_openapi() {
        use schematic_define::openapi::{ExportOptions, export};

        let api = define_samsung_smart_tv_api();
        let registry = openapi_registry();
        let doc = export(&api, &registry, &ExportOptions::new().with_version("1.0.0"))
            .expect("Samsung OpenAPI export should succeed");

        assert_eq!(doc.openapi, "3.0.3");
        assert!(doc.paths.paths.contains_key("/api/v2/"));
        assert!(doc.paths.paths.contains_key("/logs/"));

        let components = doc.components.expect("components should be present");
        assert!(components.schemas.contains_key("SamsungDeviceInfoResponse"));
        assert!(components.schemas.contains_key("SamsungDeviceInfo"));
    }
}
