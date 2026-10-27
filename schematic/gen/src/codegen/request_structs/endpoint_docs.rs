//! Documentation of an endpoint's deprecation, authentication override,
//! rate-limit headers, and retry guidance.
//!
//! Everything here is documentation: the generated client neither reads the
//! rate-limit headers nor retries, and deprecation emits no `#[deprecated]`
//! attribute (it would fail `-D warnings` builds of the generated crate and
//! its dependents). Lines are returned as plain text without the leading
//! space of a `///` line; an absent value contributes no line, so an
//! endpoint that adopts none of this keeps its documentation unchanged.

use schematic_define::metadata::{Backoff, RateLimit, ResetUnit, RetryGuidance, StatusSelector};
use schematic_define::{
    ApiKeyLocation, AuthMethod, EnvAuthStrategy, EnvList, EnvMapping, Endpoint,
    ResolvedEndpointAuth, RestApi,
};

/// The line a deprecated endpoint or parameter adds to its docs.
pub(crate) const DEPRECATED_LINE: &str = "Deprecated.";

/// One paragraph describing `endpoint`'s own authentication, or `None` when
/// it inherits the API's.
pub(crate) fn auth_paragraph(api: &RestApi, endpoint: &Endpoint) -> Option<String> {
    match api.endpoint_auth(endpoint) {
        ResolvedEndpointAuth::Public => Some("This endpoint is public: it sends no credential.".to_string()),
        ResolvedEndpointAuth::Replaced { policy, env_mapping } => {
            let mut text = "This endpoint replaces the API's authentication.".to_string();
            if !policy.explicit.is_empty() {
                let methods: Vec<String> = policy.explicit.iter().map(method_text).collect();
                text.push_str(&format!(" It accepts {}.", methods.join(", ")));
            }
            text.push(' ');
            text.push_str(&env_fallback_text(api, policy.env_fallback.as_ref(), &env_mapping));
            Some(text)
        }
        _ => None,
    }
}

fn method_text(method: &AuthMethod) -> String {
    match method {
        AuthMethod::BearerToken { header } => {
            format!("a bearer token in the `{}` header", header.as_deref().unwrap_or("Authorization"))
        }
        AuthMethod::ApiKey { header, .. } => format!("an API key in the `{header}` header"),
        AuthMethod::ApiKeyParam { name, location } => format!("an API key in {}", param_location_text(name, *location)),
        AuthMethod::Basic => "HTTP Basic credentials".to_string(),
        AuthMethod::OAuth2(_) => "an OAuth2 access token".to_string(),
        _ => "a custom credential".to_string(),
    }
}

fn param_location_text(name: &str, location: ApiKeyLocation) -> String {
    match location {
        ApiKeyLocation::Cookie => format!("the `{name}` cookie"),
        _ => format!("the `{name}` query parameter"),
    }
}

/// Which environment variables the policy's fallback reads, in the order
/// tried. A parameter API key has no mapping slot and reads `env_auth`.
fn env_fallback_text(api: &RestApi, fallback: Option<&EnvAuthStrategy>, mapping: &EnvMapping) -> String {
    let names = |list: &Option<EnvList>| list.as_ref().map(|list| list.names().to_vec()).unwrap_or_default();
    let read = |what: &str, names: &[String]| {
        if names.is_empty() {
            format!("Without a supplied credential, it reads no environment variable for {what}.")
        } else {
            format!("Without a supplied credential, it reads {what} from {}.", or_list(names))
        }
    };
    match fallback {
        None => "It reads no environment variable.".to_string(),
        Some(EnvAuthStrategy::BearerToken { .. }) => read("a bearer token", &names(&mapping.bearer_token)),
        Some(EnvAuthStrategy::ApiKey { header, .. }) => read(
            &format!("an API key for the `{header}` header"),
            &mapping.api_key.as_ref().map(|key| key.names.names().to_vec()).unwrap_or_default(),
        ),
        Some(EnvAuthStrategy::ApiKeyParam { name, location }) => read(
            &format!("an API key for {}", param_location_text(name, *location)),
            &api.env_auth,
        ),
        Some(EnvAuthStrategy::Basic) => format!(
            "{} {}",
            read("the Basic username", &names(&mapping.basic_user)),
            read("the Basic password", &names(&mapping.basic_pass))
        ),
        Some(_) => "It reads no environment variable.".to_string(),
    }
}

fn or_list(names: &[String]) -> String {
    format!("`{}`", names.join("` or `"))
}

/// The `## Rate limit` section for `rate_limit`.
pub(crate) fn rate_limit_lines(rate_limit: &RateLimit) -> Vec<String> {
    let reset_unit = match rate_limit.reset_unit {
        ResetUnit::Seconds => "seconds until the window resets",
        ResetUnit::EpochSeconds => "Unix time in seconds",
        ResetUnit::HttpDate => "an HTTP date",
    };
    let mut lines = vec![
        "## Rate limit".to_string(),
        String::new(),
        "Response headers reporting the rate limit (documentation only; the client does not act on them):"
            .to_string(),
        String::new(),
        format!("- limit: `{}`", rate_limit.limit_header),
        format!("- remaining: `{}`", rate_limit.remaining_header),
        format!("- reset: `{}`, as {reset_unit}", rate_limit.reset_header),
    ];
    if let Some(header) = &rate_limit.retry_after_header {
        lines.push(format!("- retry after: `{header}`"));
    }
    lines
}

/// The `## Retry guidance` section for `guidance`.
pub(crate) fn retry_guidance_lines(guidance: &RetryGuidance) -> Vec<String> {
    let statuses = if guidance.retry_on.is_empty() {
        "no status".to_string()
    } else {
        guidance
            .retry_on
            .iter()
            .map(|selector| match selector {
                StatusSelector::Code(code) => format!("`{code}`"),
                StatusSelector::Range { start, end } => format!("`{start}`-`{end}`"),
            })
            .collect::<Vec<_>>()
            .join(", ")
    };
    let backoff = match guidance.backoff {
        Backoff::None => "none, retry immediately".to_string(),
        Backoff::Fixed { delay_ms } => format!("fixed, {delay_ms} ms"),
        Backoff::Exponential { base_ms, cap_ms } => {
            format!("exponential from {base_ms} ms, capped at {cap_ms} ms")
        }
    };
    vec![
        "## Retry guidance".to_string(),
        String::new(),
        "The provider's retry guidance (documentation only; the client does not retry):".to_string(),
        String::new(),
        format!("- retry on: {statuses}"),
        format!(
            "- honor Retry-After: {}",
            if guidance.honor_retry_after { "yes" } else { "no" }
        ),
        format!("- max attempts: {}, including the first", guidance.max_attempts),
        format!("- backoff: {backoff}"),
    ]
}

/// The sections a request struct's docs add for `endpoint`, each preceded
/// by a blank line: deprecation, its own authentication, and its effective
/// rate limit and retry guidance (the endpoint's value, else the API's).
pub(crate) fn request_doc_sections(api: &RestApi, endpoint: &Endpoint) -> Vec<String> {
    let mut lines = Vec::new();
    if endpoint.deprecated {
        lines.extend([String::new(), DEPRECATED_LINE.to_string()]);
    }
    if let Some(auth) = auth_paragraph(api, endpoint) {
        lines.extend([String::new(), "## Authentication".to_string(), String::new(), auth]);
    }
    if let Some(rate_limit) = api.effective_rate_limit(endpoint) {
        lines.push(String::new());
        lines.extend(rate_limit_lines(rate_limit));
    }
    if let Some(guidance) = api.effective_retry_guidance(endpoint) {
        lines.push(String::new());
        lines.extend(retry_guidance_lines(guidance));
    }
    lines
}

/// `text` as the content of a `///` line: a leading space unless empty.
pub(crate) fn doc_line(text: &str) -> String {
    if text.is_empty() { String::new() } else { format!(" {text}") }
}
