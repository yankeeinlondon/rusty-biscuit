use proc_macro2::TokenStream;
use quote::quote;
use schematic_define::RestApi;

/// Generates explicit authentication helper methods based on the API's auth policy.
pub(crate) fn generate_explicit_auth_helpers(api: &RestApi) -> TokenStream {
    let policy = api.effective_auth_policy();

    let bearer_header = policy.explicit.iter().find_map(|method| match method {
        schematic_define::AuthMethod::BearerToken { header } => Some(header.clone()),
        _ => None,
    });
    let api_key_header = policy.explicit.iter().find_map(|method| match method {
        schematic_define::AuthMethod::ApiKey { header, .. } => Some(header.clone()),
        _ => None,
    });
    let has_basic = policy
        .explicit
        .iter()
        .any(|method| matches!(method, schematic_define::AuthMethod::Basic));
    let has_oauth = policy.oauth2().is_some();

    let bearer_helper = bearer_header.map(|header| match header {
        Some(header_name) => quote! {
            /// Returns a clone of this client configured with an explicit bearer token.
            #[must_use]
            pub fn bearer_token(&self, token: impl Into<String>) -> Self {
                self.variant()
                    .headers_builder(self.headers.clone().use_bearer_token_with_header(token, #header_name))
                    .build()
            }
        },
        None => quote! {
            /// Returns a clone of this client configured with an explicit bearer token.
            #[must_use]
            pub fn bearer_token(&self, token: impl Into<String>) -> Self {
                self.variant()
                    .headers_builder(self.headers.clone().use_bearer_token(token))
                    .build()
            }
        },
    }).unwrap_or_default();

    let api_key_helper = api_key_header
        .map(|header_name| {
            quote! {
                /// Returns a clone of this client configured with an explicit API key.
                #[must_use]
                pub fn api_key(&self, key: impl Into<String>) -> Self {
                    self.variant()
                        .headers_builder(self.headers.clone().use_api_key(key, #header_name))
                        .build()
                }
            }
        })
        .unwrap_or_default();

    let basic_helper = if has_basic {
        quote! {
            /// Returns a clone of this client configured with explicit basic auth.
            #[must_use]
            pub fn basic_auth(
                &self,
                username: impl Into<String>,
                password: impl Into<String>,
            ) -> Self {
                self.variant()
                    .headers_builder(self.headers.clone().use_basic_auth(username, password))
                    .build()
            }
        }
    } else {
        quote! {}
    };

    let oauth_helper = if has_oauth {
        quote! {
            /// Returns a clone of this client configured with an explicit OAuth access token.
            #[must_use]
            pub fn oauth_token(&self, token: impl Into<String>) -> Self {
                self.variant()
                    .headers_builder(self.headers.clone().use_bearer_token(token))
                    .build()
            }
        }
    } else {
        quote! {}
    };

    quote! {
        #api_key_helper
        #bearer_helper
        #basic_helper
        #oauth_helper
    }
}

/// Generates the initialization code for Headers from EnvMapping.
///
/// Creates a Headers builder configured with environment variable mapping
/// and any static headers from the API definition.
pub(crate) fn generate_headers_init_from_mapping(api: &RestApi) -> TokenStream {
    // Build the env_mapping at compile time using api.default_env_mapping()
    // and inline the values into the generated code
    let mapping = api.default_env_mapping();

    // Generate env_mapping initialization from the resolved mapping
    let env_mapping_init = {
        let bearer_token_init = if let Some(ref token_list) = mapping.bearer_token {
            let names = token_list.names();
            quote! { Some(schematic_define::EnvList::new(vec![#(#names.to_string()),*])) }
        } else {
            quote! { None }
        };

        let basic_user_init = if let Some(ref user_list) = mapping.basic_user {
            let names = user_list.names();
            quote! { Some(schematic_define::EnvList::new(vec![#(#names.to_string()),*])) }
        } else {
            quote! { None }
        };

        let basic_pass_init = if let Some(ref pass_list) = mapping.basic_pass {
            let names = pass_list.names();
            quote! { Some(schematic_define::EnvList::new(vec![#(#names.to_string()),*])) }
        } else {
            quote! { None }
        };

        let api_key_init = if let Some(ref api_key) = mapping.api_key {
            let names = api_key.names.names();
            let header = &api_key.header;
            quote! {
                Some(schematic_define::ApiKeyEnv {
                    names: schematic_define::EnvList::new(vec![#(#names.to_string()),*]),
                    header: #header.to_string(),
                })
            }
        } else {
            quote! { None }
        };

        let oauth_client_id_init = if let Some(ref id_list) = mapping.oauth_client_id {
            let names = id_list.names();
            quote! { Some(schematic_define::EnvList::new(vec![#(#names.to_string()),*])) }
        } else {
            quote! { None }
        };

        let oauth_client_secret_init = if let Some(ref secret_list) = mapping.oauth_client_secret {
            let names = secret_list.names();
            quote! { Some(schematic_define::EnvList::new(vec![#(#names.to_string()),*])) }
        } else {
            quote! { None }
        };

        let oauth_redirect_uri_init = if let Some(ref uri_list) = mapping.oauth_redirect_uri {
            let names = uri_list.names();
            quote! { Some(schematic_define::EnvList::new(vec![#(#names.to_string()),*])) }
        } else {
            quote! { None }
        };

        quote! {
            schematic_define::EnvMapping {
                bearer_token: #bearer_token_init,
                basic_user: #basic_user_init,
                basic_pass: #basic_pass_init,
                api_key: #api_key_init,
                oauth_client_id: #oauth_client_id_init,
                oauth_client_secret: #oauth_client_secret_init,
                oauth_redirect_uri: #oauth_redirect_uri_init,
            }
        }
    };

    // Add static headers if any
    if api.headers.is_empty() {
        quote! {
            schematic_define::Headers::default()
                .with_env_mapping(#env_mapping_init)
        }
    } else {
        let header_methods = api.headers.iter().map(|(k, v)| {
            quote! { .header(#k, #v) }
        });
        quote! {
            schematic_define::Headers::default()
                .with_env_mapping(#env_mapping_init)
                #(#header_methods)*
        }
    }
}

/// Generates the initialization code for the headers Vec (legacy, kept for reference).
#[allow(dead_code)]
pub(crate) fn generate_headers_init(headers: &[(String, String)]) -> TokenStream {
    if headers.is_empty() {
        quote! { vec![] }
    } else {
        let header_pairs = headers.iter().map(|(k, v)| {
            quote! { (#k.to_string(), #v.to_string()) }
        });
        quote! { vec![#(#header_pairs),*] }
    }
}
