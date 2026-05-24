use openapiv3;

use super::super::options::ExportOptions;
use crate::types::RestApi;

/// Maps API metadata to OpenAPI Info.
///
/// Version precedence: `ExportOptions::version` > `RestApi::version` > `"1.0.0"`.
pub(super) fn map_info(api: &RestApi, options: &ExportOptions) -> openapiv3::Info {
    let version = options
        .version()
        .or(api.version.as_deref())
        .unwrap_or("1.0.0")
        .to_string();

    openapiv3::Info {
        title: api.name.clone(),
        description: Some(api.description.clone()),
        version,
        ..Default::default()
    }
}

/// Maps the base URL to OpenAPI servers.
pub(super) fn map_servers(api: &RestApi) -> Vec<openapiv3::Server> {
    vec![openapiv3::Server {
        url: api.base_url.clone(),
        description: Some(format!("{} API Server", api.name)),
        ..Default::default()
    }]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::AuthStrategy;
    use crate::headers::{EnvList, EnvMapping};

    fn create_test_api() -> RestApi {
        RestApi {
            name: "TestAPI".to_string(),
            description: "Test API description".to_string(),
            base_url: "https://api.test.com/v1".to_string(),
            docs_url: Some("https://docs.test.com".to_string()),
            auth: AuthStrategy::BearerToken { header: None },
            auth_policy: None,
            env_auth: vec!["TEST_API_KEY".to_string()],
            env_username: None,
            headers: vec![],
            endpoints: vec![],
            module_path: None,
            request_suffix: None,
            version: None,
            env_mapping: Some(EnvMapping {
                bearer_token: Some(EnvList::single("TEST_API_KEY")),
                ..Default::default()
            }),
        }
    }

    #[test]
    fn map_info_sets_title_from_api_name() {
        let api = create_test_api();
        let options = ExportOptions::new();
        let info = map_info(&api, &options);

        assert_eq!(info.title, "TestAPI");
    }

    #[test]
    fn map_info_sets_description_from_api() {
        let api = create_test_api();
        let options = ExportOptions::new();
        let info = map_info(&api, &options);

        assert_eq!(info.description, Some("Test API description".to_string()));
    }

    #[test]
    fn map_info_uses_version_from_options() {
        let api = create_test_api();
        let options = ExportOptions::new().with_version("2.5.0");
        let info = map_info(&api, &options);

        assert_eq!(info.version, "2.5.0");
    }

    #[test]
    fn map_info_uses_default_version_when_none() {
        let api = create_test_api();
        let options = ExportOptions::new();
        let info = map_info(&api, &options);

        assert_eq!(info.version, "1.0.0");
    }

    #[test]
    fn map_info_defaults_to_1_0_0_when_no_version_set() {
        let api = create_test_api();
        let options = ExportOptions::new();
        let info = map_info(&api, &options);
        assert_eq!(info.version, "1.0.0");
    }

    #[test]
    fn map_info_uses_api_version_when_options_version_unset() {
        let mut api = create_test_api();
        api.version = Some("2.3.0".to_string());
        let options = ExportOptions::new();
        let info = map_info(&api, &options);
        assert_eq!(info.version, "2.3.0");
    }

    #[test]
    fn map_info_options_version_overrides_api_version() {
        let mut api = create_test_api();
        api.version = Some("2.3.0".to_string());
        let options = ExportOptions::new().with_version("9.0.0");
        let info = map_info(&api, &options);
        assert_eq!(info.version, "9.0.0");
    }

    #[test]
    fn map_servers_creates_single_server() {
        let api = create_test_api();
        let servers = map_servers(&api);

        assert_eq!(servers.len(), 1);
    }

    #[test]
    fn map_servers_uses_base_url() {
        let api = create_test_api();
        let servers = map_servers(&api);

        assert_eq!(servers[0].url, "https://api.test.com/v1");
    }

    #[test]
    fn map_servers_sets_description() {
        let api = create_test_api();
        let servers = map_servers(&api);

        assert!(servers[0].description.is_some());
        assert!(servers[0].description.as_ref().unwrap().contains("TestAPI"));
    }
}
