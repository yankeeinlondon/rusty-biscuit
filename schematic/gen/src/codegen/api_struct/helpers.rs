//! The generated `definition()` body: the API data the runtime needs.

use proc_macro2::TokenStream;
use quote::quote;
use schematic_define::{CrossOrigin, RestApi};

use super::auth::{auth_config, auth_config_tokens};

/// The `schematic_runtime::ApiDefinition` expression for `api`.
///
/// Settings equal to the runtime's defaults (no authentication, the
/// same-origin policy, no extra secret names, no request-id header) are left
/// out, so the expression lists only what the definition chose.
pub(super) fn generate_definition(api: &RestApi) -> TokenStream {
    let auth = auth_config_tokens(&auth_config(api)).map(|auth| quote! { .auth(#auth) });
    let cross_origin = origin_policy_tokens(&api.cross_origin).map(|policy| quote! { .cross_origin(#policy) });
    let secret_names = (!api.secret_names.is_empty()).then(|| {
        let names = &api.secret_names;
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
