use proc_macro2::TokenStream;
use quote::quote;
use schematic_define::{AuthPolicy, AuthStrategy};

/// Generates the initialization code for an AuthStrategy value.
pub(crate) fn generate_auth_strategy_init(auth: &AuthStrategy) -> TokenStream {
    match auth {
        AuthStrategy::None => quote! { schematic_define::AuthStrategy::None },
        AuthStrategy::BearerToken { header } => match header {
            Some(h) => {
                quote! { schematic_define::AuthStrategy::BearerToken { header: Some(#h.to_string()) } }
            }
            None => quote! { schematic_define::AuthStrategy::BearerToken { header: None } },
        },
        AuthStrategy::ApiKey { header } => {
            quote! { schematic_define::AuthStrategy::ApiKey { header: #header.to_string() } }
        }
        AuthStrategy::Basic => quote! { schematic_define::AuthStrategy::Basic },
        AuthStrategy::OAuth2(config) => {
            let grant_type = match &config.grant_type {
                schematic_define::OAuth2GrantType::AuthorizationCodePkce => {
                    quote! { schematic_define::OAuth2GrantType::AuthorizationCodePkce }
                }
                schematic_define::OAuth2GrantType::ClientCredentials => {
                    quote! { schematic_define::OAuth2GrantType::ClientCredentials }
                }
                schematic_define::OAuth2GrantType::DeviceCode => {
                    quote! { schematic_define::OAuth2GrantType::DeviceCode }
                }
                _ => quote! { schematic_define::OAuth2GrantType::AuthorizationCodePkce },
            };
            let token_url = &config.token_url;
            let authorization_url = match &config.authorization_url {
                Some(url) => quote! { Some(#url.to_string()) },
                None => quote! { None },
            };
            let revocation_url = match &config.revocation_url {
                Some(url) => quote! { Some(#url.to_string()) },
                None => quote! { None },
            };
            let device_authorization_url = match &config.device_authorization_url {
                Some(url) => quote! { Some(#url.to_string()) },
                None => quote! { None },
            };
            let default_scopes = &config.default_scopes;
            let pkce = match &config.pkce {
                schematic_define::PkceRequirement::Required => {
                    quote! { schematic_define::PkceRequirement::Required }
                }
                schematic_define::PkceRequirement::Supported => {
                    quote! { schematic_define::PkceRequirement::Supported }
                }
                schematic_define::PkceRequirement::NotUsed => {
                    quote! { schematic_define::PkceRequirement::NotUsed }
                }
                _ => quote! { schematic_define::PkceRequirement::Required },
            };
            let client_auth = match &config.client_auth {
                schematic_define::OAuth2ClientAuthMethod::ClientSecretBasic => {
                    quote! { schematic_define::OAuth2ClientAuthMethod::ClientSecretBasic }
                }
                schematic_define::OAuth2ClientAuthMethod::ClientSecretPost => {
                    quote! { schematic_define::OAuth2ClientAuthMethod::ClientSecretPost }
                }
                schematic_define::OAuth2ClientAuthMethod::None => {
                    quote! { schematic_define::OAuth2ClientAuthMethod::None }
                }
                _ => quote! { schematic_define::OAuth2ClientAuthMethod::ClientSecretBasic },
            };
            quote! {
                schematic_define::AuthStrategy::OAuth2(schematic_define::OAuth2Config {
                    grant_type: #grant_type,
                    authorization_url: #authorization_url,
                    token_url: #token_url.to_string(),
                    revocation_url: #revocation_url,
                    device_authorization_url: #device_authorization_url,
                    default_scopes: vec![#(#default_scopes.to_string()),*],
                    pkce: #pkce,
                    client_auth: #client_auth,
                })
            }
        }
        // Handle future variants (non_exhaustive)
        _ => quote! { schematic_define::AuthStrategy::None },
    }
}

/// Generates the initialization code for an [`AuthPolicy`].
pub(crate) fn generate_auth_policy_init(policy: &AuthPolicy) -> TokenStream {
    let explicit = policy.explicit.iter().map(generate_auth_method_init);
    let env_fallback = match &policy.env_fallback {
        Some(strategy) => {
            let strategy = generate_env_auth_strategy_init(strategy);
            quote! { Some(#strategy) }
        }
        None => quote! { None },
    };

    quote! {
        schematic_define::AuthPolicy {
            explicit: vec![#(#explicit),*],
            env_fallback: #env_fallback,
        }
    }
}

pub(crate) fn generate_auth_method_init(method: &schematic_define::AuthMethod) -> TokenStream {
    match method {
        schematic_define::AuthMethod::BearerToken { header } => match header {
            Some(header) => {
                quote! { schematic_define::AuthMethod::BearerToken { header: Some(#header.to_string()) } }
            }
            None => quote! { schematic_define::AuthMethod::BearerToken { header: None } },
        },
        schematic_define::AuthMethod::ApiKey { header } => {
            quote! { schematic_define::AuthMethod::ApiKey { header: #header.to_string() } }
        }
        schematic_define::AuthMethod::Basic => quote! { schematic_define::AuthMethod::Basic },
        schematic_define::AuthMethod::OAuth2(config) => {
            let config_tokens = generate_auth_strategy_init(&AuthStrategy::OAuth2(config.clone()));
            quote! {
                match #config_tokens {
                    schematic_define::AuthStrategy::OAuth2(config) => schematic_define::AuthMethod::OAuth2(config),
                    _ => unreachable!(),
                }
            }
        }
        _ => quote! { schematic_define::AuthMethod::Basic },
    }
}

pub(crate) fn generate_env_auth_strategy_init(
    strategy: &schematic_define::EnvAuthStrategy,
) -> TokenStream {
    match strategy {
        schematic_define::EnvAuthStrategy::BearerToken { header } => match header {
            Some(header) => {
                quote! { schematic_define::EnvAuthStrategy::BearerToken { header: Some(#header.to_string()) } }
            }
            None => quote! { schematic_define::EnvAuthStrategy::BearerToken { header: None } },
        },
        schematic_define::EnvAuthStrategy::ApiKey { header } => {
            quote! { schematic_define::EnvAuthStrategy::ApiKey { header: #header.to_string() } }
        }
        schematic_define::EnvAuthStrategy::Basic => {
            quote! { schematic_define::EnvAuthStrategy::Basic }
        }
        _ => quote! { schematic_define::EnvAuthStrategy::Basic },
    }
}
