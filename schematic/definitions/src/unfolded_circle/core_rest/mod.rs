//! Unfolded Circle Core REST API definition.

mod types;

pub use types::*;

use schematic_define::{
    ApiRequest, ApiResponse, AuthStrategy, Endpoint, EnvList, EnvMapping, FormField, RestApi,
    RestMethod,
};

/// Build the Unfolded Circle Core REST API definition.
#[must_use]
pub fn define_unfolded_circle_core_rest_api() -> RestApi {
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
        endpoints: vec![
            Endpoint {
                id: "Login".to_string(),
                method: RestMethod::Post,
                path: "/pub/login".to_string(),
                description: "Create cookie session with username/password".to_string(),
                request: Some(ApiRequest::json_type("LoginRequest")),
                response: ApiResponse::json_type("ApiResponseMessage"),
                headers: vec![],
                params: None,
            },
            Endpoint {
                id: "Logout".to_string(),
                method: RestMethod::Post,
                path: "/pub/logout".to_string(),
                description: "Clear current cookie session".to_string(),
                request: None,
                response: ApiResponse::Empty,
                headers: vec![],
                params: None,
            },
            Endpoint {
                id: "GetSystemInfo".to_string(),
                method: RestMethod::Get,
                path: "/system".to_string(),
                description: "Return device system information".to_string(),
                request: None,
                response: ApiResponse::json_type("SystemInfo"),
                headers: vec![],
                params: None,
            },
            Endpoint {
                id: "ExportBackup".to_string(),
                method: RestMethod::Get,
                path: "/system/backup/export".to_string(),
                description: "Export binary backup archive".to_string(),
                request: None,
                response: ApiResponse::Binary,
                headers: vec![],
                params: None,
            },
            Endpoint {
                id: "RestoreBackup".to_string(),
                method: RestMethod::Put,
                path: "/system/backup/restore".to_string(),
                description: "Upload and restore backup archive".to_string(),
                request: Some(ApiRequest::form_data(vec![FormField::file("file")])),
                response: ApiResponse::json_type("BackupRestoreReportItems"),
                headers: vec![],
                params: None,
            },
            Endpoint {
                id: "UploadResource".to_string(),
                method: RestMethod::Post,
                path: "/resources/{resource_type}".to_string(),
                description: "Upload resource file for selected type".to_string(),
                request: Some(ApiRequest::form_data(vec![FormField::file("file")])),
                response: ApiResponse::json_type("ResourceItems"),
                headers: vec![],
                params: None,
            },
            Endpoint {
                id: "GetResource".to_string(),
                method: RestMethod::Get,
                path: "/resources/{resource_type}/{resource_id}".to_string(),
                description: "Download single resource file".to_string(),
                request: None,
                response: ApiResponse::Binary,
                headers: vec![],
                params: None,
            },
            Endpoint {
                id: "InstallCustomIntegration".to_string(),
                method: RestMethod::Post,
                path: "/intg/install".to_string(),
                description: "Install custom integration from tar archive".to_string(),
                request: Some(ApiRequest::form_data(vec![FormField::file("file")])),
                response: ApiResponse::json_type("IntegrationDriverInfo"),
                headers: vec![],
                params: None,
            },
            Endpoint {
                id: "UploadCustomIrCodeSet".to_string(),
                method: RestMethod::Post,
                path: "/ir/codes/custom/{code_set_id}".to_string(),
                description: "Upload CSV file for custom IR code set".to_string(),
                request: Some(ApiRequest::form_data(vec![FormField::file("file")])),
                response: ApiResponse::json_type("CodeSetUploadResult"),
                headers: vec![],
                params: None,
            },
            Endpoint {
                id: "InstallCustomComponent".to_string(),
                method: RestMethod::Post,
                path: "/system/install/{custom_component}".to_string(),
                description: "Upload and install custom UI/web component".to_string(),
                request: Some(ApiRequest::form_data(vec![FormField::file("file")])),
                response: ApiResponse::json_type("CustomInstall"),
                headers: vec![],
                params: None,
            },
            Endpoint {
                id: "QueryLogsText".to_string(),
                method: RestMethod::Get,
                path: "/system/logs".to_string(),
                description: "Retrieve logs as plain-text export".to_string(),
                request: None,
                response: ApiResponse::Text,
                headers: vec![],
                params: None,
            },
        ],
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

    #[test]
    fn core_rest_has_expected_metadata() {
        let api = define_unfolded_circle_core_rest_api();
        assert_eq!(api.name, "UnfoldedCircleCoreRest");
        assert_eq!(
            api.module_path.as_deref(),
            Some("unfolded_circle_core_rest")
        );
        assert_eq!(api.request_suffix.as_deref(), Some("CoreRestRequest"));
    }

    #[test]
    fn core_rest_has_multipart_endpoints() {
        let api = define_unfolded_circle_core_rest_api();
        let multipart_count = api
            .endpoints
            .iter()
            .filter(|ep| matches!(ep.request, Some(ApiRequest::FormData { .. })))
            .count();
        assert!(multipart_count >= 4);
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
