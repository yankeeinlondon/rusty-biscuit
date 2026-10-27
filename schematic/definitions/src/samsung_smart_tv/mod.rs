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

use crate::registry::assemble_module_group;
use schematic_define::{DefinitionContext, DefinitionError};
#[cfg(test)]
use schematic_define::{ResponseShape, RestMethod};
use schematic_define::{
    Endpoint, EnvMapping, RestApi,
};

/// Creates the Samsung Smart TV REST API definition.
///
/// Models the Smart View HTTP API on port 8001 with seven endpoints covering
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
    module_group().expect("valid samsung_smart_tv definitions").remove(0)
}

fn build_samsung_smart_tv_api(ctx: &mut DefinitionContext) -> Result<RestApi, DefinitionError> {
    let endpoints = build_samsung_smart_tv_endpoints(ctx)?;
    RestApi::builder(ctx, "SamsungSmartTv", "http://192.168.1.1:8001")
        .describes("Samsung Smart TV LAN API (S95C-focused modern Tizen subset)")
        .no_auth()
        .docs_url("https://developer.samsung.com/smarttv/develop/extension-libraries/smart-view-sdk/receiver-apps/debugging.html")
        .module_path("samsung_smart_tv")
        .env_mapping(EnvMapping::default())
        .endpoints(endpoints)
        .build()
}

fn build_samsung_smart_tv_endpoints(ctx: &mut DefinitionContext) -> Result<Vec<Endpoint>, DefinitionError> {
    Ok(vec![
        Endpoint::get("GetDeviceInfo", "/api/v2/")
                .describes("Canonical Smart View info endpoint for modern Samsung TVs. \
                Returns device identity, model, firmware, and network details. \
                Payload fields vary by model and firmware revision; unknown fields \
                are preserved via flattened extras. For high-frequency polling, \
                prefer conditional requests or payload diffing to reduce repeated \
                owned-string allocations when firmware does not support ETag-style \
                cache validation.")
                .returns_json::<SamsungDeviceInfoResponse>(ctx)
                .build()?,
        Endpoint::get("GetServerLogs", "/logs/")
                .describes("Debug/developer-facing log endpoint. May require developer mode \
                enabled on the TV and can return non-success status when disabled.")
                .returns_text()
                .build()?,
        Endpoint::post("LaunchApplicationById", "/api/v2/applications/{app_id}")
                .describes("Launch a Samsung application by its package ID. Firmware-dependent \
                support; use when the Samsung app ID is known. Recommended runtime \
                strategy: attempt this endpoint first, fall back to LaunchApplicationByName.")
                .returns_empty()
                .build()?,
        Endpoint::post("LaunchApplicationByName", "/ws/apps/{app_name}")
                .describes("Launch a Samsung application by name. Compatibility fallback for \
                firmware that does not honor /api/v2/applications/{id}.")
                .returns_empty()
                .build()?,
        Endpoint::get("GetAppStatus", "/api/v2/applications/{app_id}")
                .describes("Get the running status and metadata of a specific application. \
                Returns visibility, running state, version, and name.")
                .returns_json::<SamsungAppStatusResponse>(ctx)
                .build()?,
        Endpoint::delete("CloseApp", "/api/v2/applications/{app_id}")
                .describes("Close a running application by its ID. Sends a DELETE to the \
                application endpoint, which requests the TV terminate the app.")
                .returns_empty()
                .build()?,
        Endpoint::put("InstallApp", "/api/v2/applications/{app_id}")
                .describes("Install (or reinstall) an application by its ID. The TV downloads \
                and installs the app from the Samsung app store.")
                .returns_empty()
                .build()?,
    ])
}

/// Assemble sibling APIs and their ordered nominal schemas in one context.
pub fn module_group() -> Result<Vec<RestApi>, DefinitionError> {
    assemble_module_group("::schematic_definitions::samsung_smart_tv", |ctx| {
        ctx.register::<SamsungDeviceInfoResponse>()?;
        ctx.register::<SamsungDeviceInfo>()?;
        ctx.register::<SamsungAppStatusResponse>()?;
        Ok(vec![
            build_samsung_smart_tv_api(ctx)?,
        ])
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use schematic_define::AuthStrategy;

    #[test]
    fn schemas_contain_samsung_rest_types() {
        let api = define_samsung_smart_tv_api();
        assert!(api.component("SamsungDeviceInfoResponse").is_some());
        assert!(api.component("SamsungDeviceInfo").is_some());
        assert!(api.component("SamsungAppStatusResponse").is_some());
        assert_eq!(crate::registry::registered_names(&api).len(), 3);
    }

    #[test]
    fn references_resolve_in_api_schemas() {
        let api = define_samsung_smart_tv_api();
        api.validate_references().expect("every reference resolves in api.schemas");
    }

    #[test]
    fn openapi_export_includes_api_schemas() {
        let api = define_samsung_smart_tv_api();
        let schemas = schematic_define::openapi::export(&api, &schematic_define::openapi::ExportOptions::new())
            .expect("export succeeds").components.expect("components").schemas;
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
        assert!(matches!(ep.response[0].shape, ResponseShape::Json { .. }));
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
        assert!(matches!(ep.response[0].shape, ResponseShape::Text));
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
        assert!(matches!(ep.response[0].shape, ResponseShape::Empty));
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
        assert!(matches!(ep.response[0].shape, ResponseShape::Empty));
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
        assert!(matches!(ep.response[0].shape, ResponseShape::Json { .. }));
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
        assert!(matches!(ep.response[0].shape, ResponseShape::Empty));
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
        assert!(matches!(ep.response[0].shape, ResponseShape::Empty));
    }

    #[test]
    fn export_samsung_api_to_openapi() {
        use schematic_define::openapi::{ExportOptions, export};

        let api = define_samsung_smart_tv_api();
        let doc = export(&api, &ExportOptions::new().with_version("1.0.0"))
            .expect("Samsung OpenAPI export should succeed");

        assert_eq!(doc.openapi, "3.0.3");
        assert!(doc.paths.paths.contains_key("/api/v2/"));
        assert!(doc.paths.paths.contains_key("/logs/"));

        let components = doc.components.expect("components should be present");
        assert!(components.schemas.contains_key("SamsungDeviceInfoResponse"));
        assert!(components.schemas.contains_key("SamsungDeviceInfo"));
    }

    #[test]
    fn module_group_preserves_endpoint_and_registry_inventory() {
        let group = module_group().unwrap();
        assert_eq!(
            group.iter().flat_map(|api| api.endpoints.iter().map(|endpoint| endpoint.id.as_str())).collect::<Vec<_>>(),
            vec!["GetDeviceInfo", "GetServerLogs", "LaunchApplicationById", "LaunchApplicationByName", "GetAppStatus", "CloseApp", "InstallApp"]
        );
        assert_eq!(crate::registry::registered_names(&group[0]), vec!["SamsungDeviceInfoResponse", "SamsungDeviceInfo", "SamsungAppStatusResponse"]);
        for api in &group {
            api.validate_references().unwrap();
            assert_eq!(api.schemas, group[0].schemas);
        }
    }

}
