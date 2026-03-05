//! Unfolded Circle Core REST API definition.
//!
//! The API supports Bearer tokens, Basic auth, and cookie-session auth flows.
//! This definition keeps Bearer token as the default strategy while exposing
//! Basic credentials via `env_mapping` and header overrides.
//!
//! ## Authentication Notes
//!
//! When a deployment requires Basic auth, override headers programmatically
//! with `Headers::use_basic_auth()` and keep `AuthStrategy::BearerToken` as
//! the default constructor path.
//!
//! ```text
//! use schematic_define::Headers;
//! use schematic_schema::unfolded_circle_core_rest::UnfoldedCircleCoreRest;
//!
//! let client = UnfoldedCircleCoreRest::new()?
//!     .variant()
//!     .headers_builder(
//!         Headers::default()
//!             .use_basic_auth("admin", "secret-pin"),
//!     )
//!     .build();
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod types;

pub use types::*;

use schematic_define::{
    ApiRequest, ApiResponse, AuthStrategy, Endpoint, EnvList, EnvMapping, FormField, RestApi,
    RestMethod,
};

/// Build the Unfolded Circle Core REST API definition.
#[must_use]
#[allow(clippy::vec_init_then_push)]
pub fn define_unfolded_circle_core_rest_api() -> RestApi {
    let mut endpoints = Vec::with_capacity(11);
    endpoints.push(Endpoint {
        id: "Login".to_string(),
        method: RestMethod::Post,
        path: "/pub/login".to_string(),
        description: "Create a cookie session using username/password credentials for clients that do not use bearer tokens. Docs: /pub/login in the official Core REST reference.".to_string(),
        request: Some(ApiRequest::json_type("LoginRequest")),
        response: ApiResponse::json_type("ApiResponseMessage"),
        headers: vec![],
        params: None,
    });
    endpoints.push(Endpoint {
        id: "Logout".to_string(),
        method: RestMethod::Post,
        path: "/pub/logout".to_string(),
        description: "Invalidate the active cookie session created by Login. Typical usage is explicit sign-out in long-running admin tools.".to_string(),
        request: None,
        response: ApiResponse::Empty,
        headers: vec![],
        params: None,
    });
    endpoints.push(Endpoint {
        id: "GetSystemInfo".to_string(),
        method: RestMethod::Get,
        path: "/system".to_string(),
        description: "Return model/serial/hardware metadata for the remote. If this endpoint is polled frequently, callers should avoid storing repeated owned `String` snapshots to reduce memory churn.".to_string(),
        request: None,
        response: ApiResponse::json_type("SystemInfo"),
        headers: vec![],
        params: None,
    });
    endpoints.push(Endpoint {
        id: "ExportBackup".to_string(),
        method: RestMethod::Get,
        path: "/system/backup/export".to_string(),
        description: "Download the current device backup archive as binary bytes. This endpoint maps to ApiResponse::Binary because the server returns an archive stream.".to_string(),
        request: None,
        response: ApiResponse::Binary,
        headers: vec![],
        params: None,
    });
    endpoints.push(Endpoint {
        id: "RestoreBackup".to_string(),
        method: RestMethod::Put,
        path: "/system/backup/restore".to_string(),
        description: "Upload and restore a backup archive using multipart form-data (`file`). The response contains per-domain restore counters.".to_string(),
        request: Some(ApiRequest::form_data(vec![FormField::file("file")])),
        response: ApiResponse::json_type("BackupRestoreReportItems"),
        headers: vec![],
        params: None,
    });
    endpoints.push(Endpoint {
        id: "UploadResource".to_string(),
        method: RestMethod::Post,
        path: "/resources/{resource_type}".to_string(),
        description: "Upload a custom resource for the selected resource type via multipart form-data (`file`). Valid `resource_type` values: `icon`, `background`, `sound`.".to_string(),
        request: Some(ApiRequest::form_data(vec![FormField::file("file")])),
        response: ApiResponse::json_type("ResourceItems"),
        headers: vec![],
        params: None,
    });
    endpoints.push(Endpoint {
        id: "GetResource".to_string(),
        method: RestMethod::Get,
        path: "/resources/{resource_type}/{resource_id}".to_string(),
        description: "Download a single stored resource by type and identifier. Valid `resource_type` values: `icon`, `background`, `sound`. Binary response supports image/audio/file resources.".to_string(),
        request: None,
        response: ApiResponse::Binary,
        headers: vec![],
        params: None,
    });
    endpoints.push(Endpoint {
        id: "InstallCustomIntegration".to_string(),
        method: RestMethod::Post,
        path: "/intg/install".to_string(),
        description: "Install a custom integration driver from a tar archive upload (`file`). Returned metadata includes required driver identity fields.".to_string(),
        request: Some(ApiRequest::form_data(vec![FormField::file("file")])),
        response: ApiResponse::json_type("IntegrationDriverInfo"),
        headers: vec![],
        params: None,
    });
    endpoints.push(Endpoint {
        id: "UploadCustomIrCodeSet".to_string(),
        method: RestMethod::Post,
        path: "/ir/codes/custom/{code_set_id}".to_string(),
        description: "Upload an IR code CSV for a custom code-set via multipart form-data (`file`). Response reports processed/added/updated counts.".to_string(),
        request: Some(ApiRequest::form_data(vec![FormField::file("file")])),
        response: ApiResponse::json_type("CodeSetUploadResult"),
        headers: vec![],
        params: None,
    });
    endpoints.push(Endpoint {
        id: "InstallCustomComponent".to_string(),
        method: RestMethod::Post,
        path: "/system/install/{custom_component}".to_string(),
        description: "Upload and install a custom UI or web-configurator component package (`file`). Valid `custom_component` values: `ui`, `web_configurator`. Returns install/activation status.".to_string(),
        request: Some(ApiRequest::form_data(vec![FormField::file("file")])),
        response: ApiResponse::json_type("CustomInstall"),
        headers: vec![],
        params: None,
    });
    endpoints.push(Endpoint {
        id: "QueryLogsText".to_string(),
        method: RestMethod::Get,
        path: "/system/logs".to_string(),
        description: "Retrieve system logs as plain text export for diagnostics pipelines and offline inspection.".to_string(),
        request: None,
        response: ApiResponse::Text,
        headers: vec![],
        params: None,
    });

    RestApi {
        name: "UnfoldedCircleCoreRest".to_string(),
        description: "Unfolded Circle Core REST API".to_string(),
        base_url: "http://remote.local/api".to_string(),
        docs_url: Some("https://unfoldedcircle.github.io/core-api/rest/".to_string()),
        auth: AuthStrategy::BearerToken { header: None },
        env_auth: vec![
            "UCR_CORE_API_KEY".to_string(),
            "UNFOLDED_CIRCLE_API_KEY".to_string(),
        ],
        env_username: Some("UCR_CORE_USER".to_string()),
        headers: vec![],
        endpoints,
        module_path: Some("unfolded_circle_core_rest".to_string()),
        request_suffix: Some("CoreRestRequest".to_string()),
        env_mapping: Some(EnvMapping {
            bearer_token: Some(EnvList::from_strs(&[
                "UCR_CORE_API_KEY",
                "UNFOLDED_CIRCLE_API_KEY",
            ])),
            basic_user: Some(EnvList::single("UCR_CORE_USER")),
            basic_pass: Some(EnvList::single("UCR_CORE_PASSWORD")),
            api_key: None,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn endpoint<'a>(api: &'a RestApi, id: &str) -> &'a Endpoint {
        api.endpoints
            .iter()
            .find(|ep| ep.id == id)
            .expect("endpoint must exist")
    }

    #[test]
    fn core_rest_has_expected_metadata_and_auth() {
        let api = define_unfolded_circle_core_rest_api();

        assert_eq!(api.name, "UnfoldedCircleCoreRest");
        assert_eq!(api.base_url, "http://remote.local/api");
        assert_eq!(
            api.docs_url.as_deref(),
            Some("https://unfoldedcircle.github.io/core-api/rest/")
        );
        assert_eq!(
            api.module_path.as_deref(),
            Some("unfolded_circle_core_rest")
        );
        assert_eq!(api.request_suffix.as_deref(), Some("CoreRestRequest"));
        assert_eq!(
            api.env_auth,
            vec![
                "UCR_CORE_API_KEY".to_string(),
                "UNFOLDED_CIRCLE_API_KEY".to_string()
            ]
        );
        assert!(matches!(
            api.auth,
            AuthStrategy::BearerToken { header: None }
        ));
    }

    #[test]
    fn core_rest_env_mapping_supports_bearer_and_basic() {
        let api = define_unfolded_circle_core_rest_api();
        let mapping = api.env_mapping.expect("env_mapping must exist");
        assert_eq!(
            mapping.bearer_token.expect("bearer token mapping").names(),
            &[
                "UCR_CORE_API_KEY".to_string(),
                "UNFOLDED_CIRCLE_API_KEY".to_string()
            ]
        );
        assert_eq!(
            mapping.basic_user.expect("basic user mapping").names(),
            &["UCR_CORE_USER".to_string()]
        );
        assert_eq!(
            mapping.basic_pass.expect("basic pass mapping").names(),
            &["UCR_CORE_PASSWORD".to_string()]
        );
        assert!(mapping.api_key.is_none());
    }

    #[test]
    fn core_rest_has_expected_endpoint_count_and_methods() {
        let api = define_unfolded_circle_core_rest_api();
        assert_eq!(api.endpoints.len(), 11);

        assert_eq!(endpoint(&api, "Login").method, RestMethod::Post);
        assert_eq!(endpoint(&api, "Logout").method, RestMethod::Post);
        assert_eq!(endpoint(&api, "GetSystemInfo").method, RestMethod::Get);
        assert_eq!(endpoint(&api, "ExportBackup").method, RestMethod::Get);
        assert_eq!(endpoint(&api, "RestoreBackup").method, RestMethod::Put);
        assert_eq!(endpoint(&api, "UploadResource").method, RestMethod::Post);
        assert_eq!(endpoint(&api, "GetResource").method, RestMethod::Get);
        assert_eq!(
            endpoint(&api, "InstallCustomIntegration").method,
            RestMethod::Post
        );
        assert_eq!(
            endpoint(&api, "UploadCustomIrCodeSet").method,
            RestMethod::Post
        );
        assert_eq!(
            endpoint(&api, "InstallCustomComponent").method,
            RestMethod::Post
        );
        assert_eq!(endpoint(&api, "QueryLogsText").method, RestMethod::Get);
    }

    #[test]
    fn core_rest_has_expected_paths_and_path_params() {
        let api = define_unfolded_circle_core_rest_api();
        assert_eq!(endpoint(&api, "GetSystemInfo").path, "/system");
        assert_eq!(
            endpoint(&api, "UploadResource").path,
            "/resources/{resource_type}"
        );
        assert_eq!(
            endpoint(&api, "GetResource").path,
            "/resources/{resource_type}/{resource_id}"
        );
        assert_eq!(
            endpoint(&api, "UploadCustomIrCodeSet").path,
            "/ir/codes/custom/{code_set_id}"
        );
        assert_eq!(
            endpoint(&api, "InstallCustomComponent").path,
            "/system/install/{custom_component}"
        );
        assert!(endpoint(&api, "UploadResource").params.is_none());
        assert!(endpoint(&api, "GetResource").params.is_none());
    }

    #[test]
    fn core_rest_has_multipart_endpoints() {
        let api = define_unfolded_circle_core_rest_api();
        let multipart_count = api
            .endpoints
            .iter()
            .filter(|ep| matches!(ep.request, Some(ApiRequest::FormData { .. })))
            .count();
        assert_eq!(multipart_count, 5);
    }

    #[test]
    fn core_rest_has_binary_and_text_endpoints() {
        let api = define_unfolded_circle_core_rest_api();
        assert!(
            api.endpoints
                .iter()
                .any(|ep| matches!(ep.response, ApiResponse::Binary))
        );
        assert!(
            api.endpoints
                .iter()
                .any(|ep| matches!(ep.response, ApiResponse::Text))
        );
    }
}
