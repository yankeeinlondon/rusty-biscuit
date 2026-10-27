//! Unfolded Circle Core REST API definition.
//!
//! The API supports Bearer tokens, Basic auth, and cookie-session auth flows.
//! This definition keeps Bearer token as the default strategy while exposing
//! Basic credentials via `env_mapping` and header overrides.
//!
//! ## Authentication Notes
//!
//! The generated client accepts only a bearer token, so it has no
//! `basic_auth(..)`, and a `Credential::basic` set on it is refused as an
//! incompatible credential. A deployment that requires Basic auth needs the
//! definition to accept `AuthMethod::Basic`.
//!
//! ```text
//! use schematic_schema::unfolded_circle_core_rest::UnfoldedCircleCoreRest;
//!
//! let client = UnfoldedCircleCoreRest::new()
//!     .base_url("http://remote.local/api")
//!     .api_key(token); // sent as `Authorization: Bearer <token>`
//! ```

mod types;

pub use types::*;

#[cfg(test)]
use schematic_define::ApiRequest;
#[cfg(test)]
use schematic_define::ResponseShape;
use crate::registry::assemble_module_group;
use schematic_define::{DefinitionContext, DefinitionError};
use schematic_define::{
    AuthStrategy, CrossOrigin, Endpoint, EnvList, EnvMapping, FormField,
    RestApi, RestMethod,
};

/// Assemble the APIs and ordered schemas in one destination context.
pub fn module_group() -> Result<Vec<RestApi>, DefinitionError> {
    assemble_module_group("::schematic_definitions::unfolded_circle::core_rest", |ctx| {
        ctx.register::<LoginRequest>()?;
        ctx.register::<ApiResponseMessage>()?;
        ctx.register::<SystemInfo>()?;
        ctx.register::<BackupRestoreReportItems>()?;
        ctx.register::<ResourceItems>()?;
        ctx.register::<IntegrationDriverInfo>()?;
        ctx.register::<CodeSetUploadResult>()?;
        ctx.register::<CustomInstall>()?;
        Ok(vec![build_unfolded_circle_core_rest_api(ctx)?])
    })
}

/// Build the Unfolded Circle Core REST API definition.
#[must_use]
pub fn define_unfolded_circle_core_rest_api() -> RestApi {
    module_group().expect("valid shipped API").remove(0)
}

#[allow(clippy::vec_init_then_push)]
fn build_unfolded_circle_core_rest_api(ctx: &mut DefinitionContext) -> Result<RestApi, DefinitionError> {
    let mut endpoints = Vec::with_capacity(11);
    endpoints.push(Endpoint::new("Login", RestMethod::Post, "/pub/login")
            .describes("Create a cookie session using username/password credentials for clients that do not use bearer tokens. Docs: /pub/login in the official Core REST reference.")
            .request_json::<LoginRequest>(ctx)
            .returns_json::<ApiResponseMessage>(ctx)
            .build()?);
    endpoints.push(Endpoint::new("Logout", RestMethod::Post, "/pub/logout")
            .describes("Invalidate the active cookie session created by Login. Typical usage is explicit sign-out in long-running admin tools.")
            .returns_empty()
            .build()?);
    endpoints.push(Endpoint::new("GetSystemInfo", RestMethod::Get, "/system")
            .describes("Return model/serial/hardware metadata for the remote. If this endpoint is polled frequently, callers should avoid storing repeated owned `String` snapshots to reduce memory churn.")
            .returns_json::<SystemInfo>(ctx)
            .build()?);
    endpoints.push(Endpoint::new("ExportBackup", RestMethod::Get, "/system/backup/export")
            .describes("Download the current device backup archive as binary bytes. This endpoint maps to ApiResponse::Binary because the server returns an archive stream.")
            .returns_binary()
            .build()?);
    endpoints.push(Endpoint::new("RestoreBackup", RestMethod::Put, "/system/backup/restore")
            .describes("Upload and restore a backup archive using multipart form-data (`file`). The response contains per-domain restore counters.")
            .request_form(vec![FormField::file("file")])
            .returns_json::<BackupRestoreReportItems>(ctx)
            .build()?);
    endpoints.push(Endpoint::new("UploadResource", RestMethod::Post, "/resources/{resource_type}")
            .describes("Upload a custom resource for the selected resource type via multipart form-data (`file`). Valid `resource_type` values: `icon`, `background`, `sound`.")
            .request_form(vec![FormField::file("file")])
            .returns_json::<ResourceItems>(ctx)
            .build()?);
    endpoints.push(Endpoint::new("GetResource", RestMethod::Get, "/resources/{resource_type}/{resource_id}")
            .describes("Download a single stored resource by type and identifier. Valid `resource_type` values: `icon`, `background`, `sound`. Binary response supports image/audio/file resources.")
            .returns_binary()
            .build()?);
    endpoints.push(Endpoint::new("InstallCustomIntegration", RestMethod::Post, "/intg/install")
            .describes("Install a custom integration driver from a tar archive upload (`file`). Returned metadata includes required driver identity fields.")
            .request_form(vec![FormField::file("file")])
            .returns_json::<IntegrationDriverInfo>(ctx)
            .build()?);
    endpoints.push(Endpoint::new("UploadCustomIrCodeSet", RestMethod::Post, "/ir/codes/custom/{code_set_id}")
            .describes("Upload an IR code CSV for a custom code-set via multipart form-data (`file`). Response reports processed/added/updated counts.")
            .request_form(vec![FormField::file("file")])
            .returns_json::<CodeSetUploadResult>(ctx)
            .build()?);
    endpoints.push(Endpoint::new("InstallCustomComponent", RestMethod::Post, "/system/install/{custom_component}")
            .describes("Upload and install a custom UI or web-configurator component package (`file`). Valid `custom_component` values: `ui`, `web_configurator`. Returns install/activation status.")
            .request_form(vec![FormField::file("file")])
            .returns_json::<CustomInstall>(ctx)
            .build()?);
    endpoints.push(Endpoint::new("QueryLogsText", RestMethod::Get, "/system/logs")
            .describes("Retrieve system logs as plain text export for diagnostics pipelines and offline inspection.")
            .returns_text()
            .build()?);

    {
        RestApi::builder(ctx, "UnfoldedCircleCoreRest", "http://remote.local/api")
            .describes("Unfolded Circle Core REST API")
            .authentication(AuthStrategy::BearerToken { header: None }, vec![
            "UCR_CORE_API_KEY".to_string(),
            "UNFOLDED_CIRCLE_API_KEY".to_string(),
        ])
            .env_username("UCR_CORE_USER".to_string())
            .env_mapping(EnvMapping {
            bearer_token: Some(EnvList::from_strs(&[
                "UCR_CORE_API_KEY",
                "UNFOLDED_CIRCLE_API_KEY",
            ])),
            basic_user: Some(EnvList::single("UCR_CORE_USER")),
            basic_pass: Some(EnvList::single("UCR_CORE_PASSWORD")),
            api_key: None,
            ..Default::default()
        })
            .docs_url("https://unfoldedcircle.github.io/core-api/rest/".to_string())
            .module_path("unfolded_circle_core_rest".to_string())
            .request_suffix("CoreRestRequest".to_string())
            .cross_origin(CrossOrigin::same_origin())
            .endpoints(endpoints)
            .build()
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
                .any(|ep| matches!(ep.response[0].shape, ResponseShape::Binary))
        );
        assert!(
            api.endpoints
                .iter()
                .any(|ep| matches!(ep.response[0].shape, ResponseShape::Text))
        );
    }
    #[test]
    fn typed_module_group_preserves_endpoint_order_and_inline_components() {
        let group = module_group().unwrap();
        let ids = |api: &RestApi| api.endpoints.iter().map(|ep| ep.id.clone()).collect::<Vec<_>>();
        assert_eq!(ids(&group[0]), ["Login", "Logout", "GetSystemInfo", "ExportBackup", "RestoreBackup", "UploadResource", "GetResource", "InstallCustomIntegration", "UploadCustomIrCodeSet", "InstallCustomComponent", "QueryLogsText"]);
        for api in &group {
            api.validate_references().unwrap();
            assert_eq!(api.schemas, group[0].schemas);
        }
        for name in ["BackupRestoreReportItems", "ResourceItems"] {
            assert!(group[0].component(name).is_none(), "container component {name} must be inline");
        }
    }

}
