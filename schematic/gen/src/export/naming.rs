//! Module name resolution from RestApi definitions.

use schematic_define::RestApi;

/// Resolves the canonical module name for an API.
///
/// Uses `module_path` if set, otherwise lowercases the API name.
///
/// ## Examples
///
/// ```
/// use schematic_define::{DefinitionContext, RestApi};
/// use schematic_gen::export::resolve_module_name;
///
/// let ctx = DefinitionContext::types_in("crate::types");
/// let api = RestApi::builder(&ctx, "OpenAI", "https://api.openai.com/v1")
///     .describes("OpenAI API")
///     .no_auth()
///     .build().unwrap();
/// assert_eq!(resolve_module_name(&api), "openai");
/// ```
pub fn resolve_module_name(api: &RestApi) -> String {
    api.module_path
        .as_deref()
        .unwrap_or(&api.name.to_lowercase())
        .to_string()
}
