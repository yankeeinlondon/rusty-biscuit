//! Module name resolution from RestApi definitions.

use schematic_define::RestApi;

/// Resolves the canonical module name for an API.
///
/// Uses `module_path` if set, otherwise lowercases the API name.
///
/// ## Examples
///
/// ```
/// use schematic_define::{RestApi, RestMethod, AuthStrategy, Endpoint, ApiResponse};
/// use schematic_gen::export::resolve_module_name;
///
/// let api = RestApi {
///     name: "OpenAI".to_string(),
///     description: "OpenAI API".to_string(),
///     base_url: "https://api.openai.com/v1".to_string(),
///     docs_url: None,
///     auth: AuthStrategy::None,
///     auth_policy: None,
///     env_auth: vec![],
///     env_username: None,
///     headers: vec![],
///     endpoints: vec![],
///     module_path: None,
///     request_suffix: None,
///     version: None,
///     env_mapping: None,
/// };
/// assert_eq!(resolve_module_name(&api), "openai");
/// ```
pub fn resolve_module_name(api: &RestApi) -> String {
    api.module_path
        .as_deref()
        .unwrap_or(&api.name.to_lowercase())
        .to_string()
}
