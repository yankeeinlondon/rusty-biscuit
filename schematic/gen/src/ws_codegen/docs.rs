//! Module documentation generation for WS modules.

use proc_macro2::TokenStream;

use super::plan::WsRuntimePlan;

/// Generate module-level documentation for a WS API module.
pub fn generate_module_docs(plan: &WsRuntimePlan) -> TokenStream {
    let mut lines = Vec::new();
    lines.push(format!(
        "//! Generated WebSocket runtime client for {}.",
        plan.api_name
    ));
    lines.push("//!".to_string());
    lines.push(format!("//! {}", plan.description));
    lines.push("//!".to_string());
    lines.push(format!("//! Base URL: `{}`", plan.base_url));
    lines.push("//!".to_string());
    lines.push("//! ## Endpoints".to_string());
    lines.push("//!".to_string());

    for ep in &plan.endpoints {
        lines.push(format!(
            "//! - `{}` (`{}`): {}",
            ep.id, ep.path, ep.description
        ));
    }

    let auth_lines = generate_auth_doc_lines(plan);
    if !auth_lines.is_empty() {
        lines.push("//!".to_string());
        lines.push("//! ## Authentication".to_string());
        lines.push("//!".to_string());
        lines.extend(auth_lines);
    }

    if let Some(ref url) = plan.docs_url {
        lines.push("//!".to_string());
        lines.push(format!("//! API documentation: <{}>", url));
    }

    lines.join("\n").parse().unwrap_or_default()
}

fn generate_auth_doc_lines(plan: &WsRuntimePlan) -> Vec<String> {
    match &plan.auth {
        super::plan::WsAuthStrategy::None => vec!["//! - No automatic handshake authentication is configured.".to_string()],
        super::plan::WsAuthStrategy::HeaderBased { strategy, env_auth } => {
            let mut lines = vec![format!(
                "//! - Handshake auth strategy: {}.",
                auth_strategy_label(strategy)
            )];
            if !env_auth.is_empty() {
                lines.push(format!(
                    "//! - Credential environment variables: `{}`.",
                    env_auth.join("`, `")
                ));
            }
            lines
        }
        super::plan::WsAuthStrategy::MessageBased {
            header_strategy,
            env_auth,
            ..
        } => {
            let mut lines = vec![format!(
                "//! - Message-based authentication flow with handshake strategy: {}.",
                auth_strategy_label(header_strategy)
            )];
            if !env_auth.is_empty() {
                lines.push(format!(
                    "//! - Credential environment variables: `{}`.",
                    env_auth.join("`, `")
                ));
            }
            if matches!(header_strategy, schematic_define::AuthStrategy::None) {
                lines.push(
                    "//! - Credentials are not auto-applied to headers; send auth messages explicitly (typed lifecycle helper or `send()`)."
                        .to_string(),
                );
            }
            lines
        }
    }
}

fn auth_strategy_label(strategy: &schematic_define::AuthStrategy) -> &'static str {
    match strategy {
        schematic_define::AuthStrategy::None => "none",
        schematic_define::AuthStrategy::BearerToken { .. } => "bearer token header",
        schematic_define::AuthStrategy::ApiKey { .. } => "API key header",
        schematic_define::AuthStrategy::Basic => "basic auth header",
        schematic_define::AuthStrategy::ApiKeyParam { .. } => "API key parameter",
    }
}
