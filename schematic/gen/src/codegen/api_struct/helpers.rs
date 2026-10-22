//! The generated `definition()` body: the API data the runtime needs.

use proc_macro2::TokenStream;
use quote::quote;
use schematic_define::{CrossOrigin, RestApi};

use super::auth::{EndpointConfig, auth_config, auth_config_tokens, endpoint_configs, policy_tokens};

/// The `schematic_runtime::ApiDefinition` expression for `api`.
///
/// Settings equal to the runtime's defaults (no authentication, the
/// same-origin policy, no extra secret names, no request-id header) are left
/// out. Endpoint replacements are always emitted, including same-origin,
/// because their API default may differ; an endpoint without its own
/// authentication emits nothing for it.
pub(super) fn generate_definition(api: &RestApi) -> TokenStream {
    let auth = auth_config_tokens(&auth_config(api)).map(|auth| quote! { .auth(#auth) });
    let endpoint_auth = endpoint_configs(api).into_iter().map(|(id, config)| match config {
        EndpointConfig::Public => quote! { .endpoint_public(#id) },
        EndpointConfig::Replaced(config) => {
            let config = policy_tokens(&config);
            quote! { .endpoint_auth(#id, #config) }
        }
    });
    let cross_origin = origin_policy_tokens(&api.cross_origin).map(|policy| quote! { .cross_origin(#policy) });
    let endpoint_policies = api.endpoints.iter().filter_map(|endpoint| {
        endpoint.cross_origin.as_ref().map(|policy| {
            let id = &endpoint.id;
            let policy = origin_policy_tokens(policy).unwrap_or_else(|| quote! { schematic_runtime::OriginPolicy::same_origin() });
            quote! { .endpoint_cross_origin(#id, #policy) }
        })
    });
    let mut names = api.secret_names.clone();
    for params in api.endpoints.iter().filter_map(|endpoint| endpoint.params.as_ref()) {
        for param in params.query.iter().chain(&params.header).chain(&params.cookie).filter(|param| param.secret) {
            if !names.contains(&param.name) {
                names.push(param.name.clone());
            }
        }
    }
    let secret_names = (!names.is_empty()).then(|| {
        quote! { .secret_names([#(#names),*]) }
    });
    let request_id_header = api
        .request_id_header
        .as_ref()
        .map(|header| quote! { .request_id_header(#header) });

    quote! {
        schematic_runtime::ApiDefinition::new(Self::BASE_URL)
            #auth
            #cross_origin
            #(#endpoint_policies)*
            #(#endpoint_auth)*
            #secret_names
            #request_id_header
    }
}

/// The runtime `OriginPolicy` for `policy`, or `None` for the same-origin
/// default. The match is exhaustive on purpose: `CrossOrigin` is not
/// `#[non_exhaustive]`, so a new policy fails to compile here rather than
/// being emitted as a looser one.
fn origin_policy_tokens(policy: &CrossOrigin) -> Option<TokenStream> {
    Some(match policy {
        CrossOrigin::SameOrigin => return None,
        CrossOrigin::AllowOrigins(origins) if origins.is_empty() => {
            quote! { schematic_runtime::OriginPolicy::allow_origins(Vec::<String>::new()) }
        }
        CrossOrigin::AllowOrigins(origins) => {
            quote! { schematic_runtime::OriginPolicy::allow_origins([#(#origins),*]) }
        }
        CrossOrigin::AllowWithoutCredentials => {
            quote! { schematic_runtime::OriginPolicy::allow_without_credentials() }
        }
        CrossOrigin::DangerAllowAny => quote! { schematic_runtime::OriginPolicy::danger_allow_any() },
        CrossOrigin::Deny => quote! { schematic_runtime::OriginPolicy::deny() },
    })
}
