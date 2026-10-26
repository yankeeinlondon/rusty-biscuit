//! Translation of a definition's authentication policies into the runtime's
//! `AuthConfig`: the API's effective policy, and each endpoint policy that
//! replaces it. Generated WebSocket clients translate their handshake policy
//! with the same functions ([`policy_config`], [`policy_tokens`]).
//!
//! The runtime does not depend on `schematic-define`, so the generated
//! `definition()` spells a policy out in runtime terms: every accepted
//! explicit method becomes a `CredentialPlacement`, and the environment
//! fallback becomes an `EnvFallback` over the variables the policy's
//! environment mapping names.

use proc_macro2::TokenStream;
use quote::quote;
use schematic_define::{
    ApiKeyLocation, AuthMethod, AuthPolicy, EnvAuthStrategy, EnvMapping, ResolvedEndpointAuth, RestApi,
};

/// A runtime credential placement, as generation-time data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Placement {
    Bearer { header: Option<String> },
    ApiKeyHeader { header: String, prefix: Option<String> },
    ApiKeyQuery { name: String },
    ApiKeyCookie { name: String },
    Basic,
}

/// The runtime form of an API's environment fallback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EnvFallback {
    pub placement: Placement,
    /// The token, key, or Basic password variables, read in order.
    pub vars: Vec<String>,
    /// The Basic username variables; empty for every other placement.
    pub username_vars: Vec<String>,
}

/// The runtime form of an API's effective authentication policy.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct AuthConfig {
    pub accepted: Vec<Placement>,
    pub env: Option<EnvFallback>,
}

/// Translates `api`'s effective policy.
pub(super) fn auth_config(api: &RestApi) -> AuthConfig {
    policy_config(&api.env_auth, &api.effective_auth_policy(), &api.default_env_mapping())
}

/// Translates `policy`, reading its fallback's variables from `mapping`
/// (parameter keys from the definition's `env_auth`, which the mapping has
/// no slot for).
///
/// An OAuth2 method is accepted as a bearer access token, which the caller
/// obtains out of band. A fallback with no variable to read is dropped, as is
/// a method or location this generator does not know: under the API's
/// policy the runtime then sends that request anonymously, exactly as when no
/// variable is set, and under an endpoint replacement it refuses the send.
pub(crate) fn policy_config(env_auth: &[String], policy: &AuthPolicy, mapping: &EnvMapping) -> AuthConfig {
    let mut accepted: Vec<Placement> = Vec::new();
    for method in &policy.explicit {
        if let Some(placement) = explicit_placement(method)
            && !accepted.contains(&placement)
        {
            accepted.push(placement);
        }
    }
    let env = policy.env_fallback.as_ref().and_then(|strategy| env_fallback(env_auth, mapping, strategy));
    AuthConfig { accepted, env }
}

/// One endpoint's authentication where it replaces the API's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum EndpointConfig {
    Public,
    Replaced(AuthConfig),
}

/// The endpoints of `api` that declare their own authentication, in
/// definition order, with what each uses instead of the API's policy.
pub(super) fn endpoint_configs(api: &RestApi) -> Vec<(&str, EndpointConfig)> {
    api.endpoints
        .iter()
        .filter(|endpoint| endpoint.auth.is_some())
        .filter_map(|endpoint| {
            let config = match api.endpoint_auth(endpoint) {
                ResolvedEndpointAuth::Public => EndpointConfig::Public,
                ResolvedEndpointAuth::Replaced { policy, env_mapping } => {
                    EndpointConfig::Replaced(policy_config(&api.env_auth, &policy, &env_mapping))
                }
                _ => return None,
            };
            Some((endpoint.id.as_str(), config))
        })
        .collect()
}

/// Whether the API's policy or any endpoint replacement accepts a
/// caller-supplied Basic credential.
pub(super) fn accepts_basic(api: &RestApi) -> bool {
    std::iter::once(auth_config(api))
        .chain(endpoint_configs(api).into_iter().filter_map(|(_, config)| match config {
            EndpointConfig::Replaced(config) => Some(config),
            EndpointConfig::Public => None,
        }))
        .any(|config| config.accepted.contains(&Placement::Basic))
}

fn explicit_placement(method: &AuthMethod) -> Option<Placement> {
    Some(match method {
        AuthMethod::BearerToken { header } => Placement::Bearer { header: header.clone() },
        AuthMethod::ApiKey { header, value_prefix } => Placement::ApiKeyHeader {
            header: header.clone(),
            prefix: value_prefix.clone(),
        },
        AuthMethod::ApiKeyParam { name, location } => param_placement(name, *location)?,
        AuthMethod::Basic => Placement::Basic,
        AuthMethod::OAuth2(_) => Placement::Bearer { header: None },
        _ => return None,
    })
}

fn param_placement(name: &str, location: ApiKeyLocation) -> Option<Placement> {
    match location {
        ApiKeyLocation::Query => Some(Placement::ApiKeyQuery { name: name.to_string() }),
        ApiKeyLocation::Cookie => Some(Placement::ApiKeyCookie { name: name.to_string() }),
        _ => None,
    }
}

fn env_fallback(env_auth: &[String], mapping: &EnvMapping, strategy: &EnvAuthStrategy) -> Option<EnvFallback> {
    let names = |list: Option<&schematic_define::EnvList>| {
        list.map(|list| list.names().to_vec()).unwrap_or_default()
    };
    let (placement, vars, username_vars) = match strategy {
        EnvAuthStrategy::BearerToken { header } => (
            Placement::Bearer { header: header.clone() },
            names(mapping.bearer_token.as_ref()),
            Vec::new(),
        ),
        EnvAuthStrategy::ApiKey { header, value_prefix } => (
            Placement::ApiKeyHeader {
                header: header.clone(),
                prefix: value_prefix.clone(),
            },
            names(mapping.api_key.as_ref().map(|api_key| &api_key.names)),
            Vec::new(),
        ),
        // A parameter key is not a header credential, so the mapping has no
        // entry for it; the definition's `env_auth` names its variables.
        EnvAuthStrategy::ApiKeyParam { name, location } => {
            (param_placement(name, *location)?, env_auth.to_vec(), Vec::new())
        }
        EnvAuthStrategy::Basic => (
            Placement::Basic,
            names(mapping.basic_pass.as_ref()),
            names(mapping.basic_user.as_ref()),
        ),
        _ => return None,
    };
    (!vars.is_empty()).then_some(EnvFallback {
        placement,
        vars,
        username_vars,
    })
}

/// The `schematic_runtime::AuthConfig` expression for `config`, or `None`
/// when the API accepts no credential.
pub(super) fn auth_config_tokens(config: &AuthConfig) -> Option<TokenStream> {
    if config.accepted.is_empty() && config.env.is_none() {
        return None;
    }
    Some(policy_tokens(config))
}

/// The `schematic_runtime::AuthConfig` expression for `config`, even when it
/// accepts no credential.
pub(crate) fn policy_tokens(config: &AuthConfig) -> TokenStream {
    let accepted = config.accepted.iter().map(|placement| {
        let placement = placement_tokens(placement);
        quote! { .accept(#placement) }
    });
    let env = config.env.as_ref().map(|env| {
        let placement = placement_tokens(&env.placement);
        let vars = &env.vars;
        let username = if env.username_vars.is_empty() {
            quote! {}
        } else {
            let username_vars = &env.username_vars;
            quote! { .username_vars([#(#username_vars),*]) }
        };
        quote! {
            .env_fallback(
                schematic_runtime::EnvFallback::new(#placement, [#(#vars),*])
                    #username
            )
        }
    });
    quote! {
        schematic_runtime::AuthConfig::none()
            #(#accepted)*
            #env
    }
}

fn placement_tokens(placement: &Placement) -> TokenStream {
    let optional = |value: &Option<String>| match value {
        Some(value) => quote! { Some(#value.to_string()) },
        None => quote! { None },
    };
    match placement {
        Placement::Bearer { header } => {
            let header = optional(header);
            quote! { schematic_runtime::CredentialPlacement::Bearer { header: #header } }
        }
        Placement::ApiKeyHeader { header, prefix } => {
            let prefix = optional(prefix);
            quote! {
                schematic_runtime::CredentialPlacement::ApiKeyHeader {
                    header: #header.to_string(),
                    prefix: #prefix,
                }
            }
        }
        Placement::ApiKeyQuery { name } => quote! {
            schematic_runtime::CredentialPlacement::ApiKeyQuery { name: #name.to_string() }
        },
        Placement::ApiKeyCookie { name } => quote! {
            schematic_runtime::CredentialPlacement::ApiKeyCookie { name: #name.to_string() }
        },
        Placement::Basic => quote! { schematic_runtime::CredentialPlacement::Basic },
    }
}
