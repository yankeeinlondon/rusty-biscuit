//! Send helper for executing outbound messages via the messenger library.
//!
//! This module bridges Claudine's messaging configuration with the messenger
//! library, handling template interpolation, provider construction, and
//! fire-and-forget async dispatch.

use std::collections::BTreeMap;
use std::path::Path;

use messenger::provider::{
    Messenger,
    discord::{DiscordConfig, DiscordProvider},
    signal::{SignalConfig, SignalProvider},
    slack::{SlackConfig, SlackProvider},
    whatsapp::{WhatsAppConfig, WhatsAppProvider},
};
use messenger::target::SignalAddress;
use messenger::{Dispatch, Message, ProviderKind, Target};
use secrecy::SecretString;
use tracing::{debug, warn};

use super::config::MessagingRouteConfig;
use super::resolve::{
    ResolvedMessagingRoute, RuntimeMessagingSettings, SignalRecipient, parse_signal_recipient,
    resolve_effective_route, resolve_image_path, resolve_secret,
};
use crate::dispatch::template::interpolate;
use crate::events::EventMeta;

/// Executes a message delivery by interpolating templates, resolving the route,
/// and spawning an async task to send via the messenger library.
///
/// This function returns immediately after spawning the async task. Errors are
/// logged as warnings rather than propagated, following the fire-and-forget
/// pattern used in `runner.rs::execute_speak()`.
///
/// ## Examples
///
/// ```rust,ignore
/// use claudine::messaging::{execute_message, RuntimeMessagingSettings};
/// use claudine::events::EventMeta;
///
/// execute_message(
///     "Tool used: {{tool_name}}",
///     Some("~/screenshots/result.png"),
///     &event_meta,
///     &messaging_settings,
/// );
/// ```
pub fn execute_message(
    message_template: &str,
    image_template: Option<&str>,
    meta: &EventMeta,
    messaging: &RuntimeMessagingSettings,
) {
    // Interpolate message text
    let text = interpolate(message_template, meta);

    // Interpolate image path if provided, filter out empty/whitespace
    let image = image_template
        .map(|tmpl| interpolate(tmpl, meta))
        .filter(|s| !s.trim().is_empty());

    // Resolve the effective route
    let Some(route) = resolve_effective_route(messaging) else {
        return;
    };

    // If both text and image are empty, nothing to send
    if text.trim().is_empty() && image.is_none() {
        debug!("Empty message after interpolation; skipping send");
        return;
    }

    // Extract cwd and repo_root for image path resolution
    let cwd = meta.cwd.as_deref();
    let repo_root = meta.env.repo.as_ref().and_then(|r| r.root.to_str());

    // Build the payload
    let Some(payload) = build_payload(&route, text, image, cwd, repo_root) else {
        return;
    };

    // Spawn async task for fire-and-forget sending
    tokio::spawn(async move {
        if let Err(e) = send_payload(&route, payload).await {
            warn!(
                route = route.name,
                provider = provider_kind_label_from_config(&route.config),
                error = %e,
                "Failed to send message"
            );
        }
    });
}

/// Send a pre-rendered message without template interpolation.
///
/// Unlike [`execute_message`], this function accepts already-resolved text
/// and does not require an [`EventMeta`]. Designed for lifecycle notifications
/// where the message text is a fixed string from frontmatter.
///
/// Follows the same fire-and-forget pattern: spawns an async task and returns
/// immediately. Missing routes are a no-op.
pub fn execute_resolved_message(
    text: &str,
    image: Option<&str>,
    cwd: Option<&Path>,
    repo_root: Option<&Path>,
    messaging: &RuntimeMessagingSettings,
) {
    if text.trim().is_empty() && image.is_none() {
        return;
    }

    let Some(route) = resolve_effective_route(messaging) else {
        return;
    };

    let cwd_str = cwd.and_then(|p| p.to_str());
    let repo_str = repo_root.and_then(|p| p.to_str());

    let Some(payload) = build_payload(
        &route,
        text.to_string(),
        image.map(|s| s.to_string()),
        cwd_str,
        repo_str,
    ) else {
        return;
    };

    tokio::spawn(async move {
        if let Err(e) = send_payload(&route, payload).await {
            warn!(
                route = route.name,
                provider = provider_kind_label_from_config(&route.config),
                error = %e,
                "Failed to send lifecycle message"
            );
        }
    });
}

/// Internal payload structure for the async send task.
struct MessagePayload {
    message: Message,
    target: Target,
    provider_kind: ProviderKind,
    route_config: MessagingRouteConfig,
}

/// Builds a `MessagePayload` from the resolved route and interpolated content.
///
/// Returns `None` if the route configuration is incomplete or invalid.
fn build_payload(
    route: &ResolvedMessagingRoute,
    text: String,
    image: Option<String>,
    cwd: Option<&str>,
    repo_root: Option<&str>,
) -> Option<MessagePayload> {
    let (target, provider_kind) = match &route.config {
        MessagingRouteConfig::Discord { channel_id, .. } => (
            Target::discord_channel(channel_id.clone()),
            ProviderKind::Discord,
        ),
        MessagingRouteConfig::Slack { channel_id, .. } => (
            Target::slack_channel(channel_id.clone()),
            ProviderKind::Slack,
        ),
        MessagingRouteConfig::Signal { recipient, .. } => {
            let parsed = parse_signal_recipient(recipient);
            let target = match parsed {
                SignalRecipient::Phone(phone) => Target::signal_user(SignalAddress::Phone(phone)),
                SignalRecipient::Group(group_id) => Target::signal_group(group_id),
            };
            (target, ProviderKind::Signal)
        }
        MessagingRouteConfig::WhatsApp { recipient, .. } => (
            Target::whatsapp_recipient(recipient.clone()),
            ProviderKind::WhatsApp,
        ),
    };

    let has_text = !text.trim().is_empty();

    // Build the message
    let mut message = if has_text {
        Message::markdown(text)
    } else {
        empty_message()
    };

    // Attach image for Discord only; warn for other providers
    if let Some(img_path_str) = image {
        let resolved = resolve_image_path(&img_path_str, cwd, repo_root);

        if provider_kind == ProviderKind::Discord {
            message = message.image(resolved);
        } else {
            if !has_text {
                warn!(
                    provider = provider_kind_label(&provider_kind),
                    path = %resolved.display(),
                    "Image attachments not supported for this provider and message text is empty; skipping send"
                );
                return None;
            }

            warn!(
                provider = provider_kind_label(&provider_kind),
                path = %resolved.display(),
                "Image attachments not supported for this provider; ignoring"
            );
        }
    }

    Some(MessagePayload {
        message,
        target,
        provider_kind,
        route_config: route.config.clone(),
    })
}

fn empty_message() -> Message {
    Message {
        body: None,
        attachments: Vec::new(),
        location: None,
        metadata: BTreeMap::new(),
    }
}

/// Sends the payload by building the provider, creating a messenger, and
/// executing the dispatch plan.
async fn send_payload(
    route: &ResolvedMessagingRoute,
    payload: MessagePayload,
) -> Result<(), String> {
    let mut messenger = Messenger::new();

    // Build and register the provider based on route config
    match &payload.route_config {
        MessagingRouteConfig::Discord {
            bot_token,
            bot_token_env,
            ..
        } => {
            let token = resolve_secret(bot_token.as_deref(), bot_token_env)
                .map_err(|e| format!("Discord: {}", e))?;
            let provider = DiscordProvider::new(DiscordConfig {
                bot_token: SecretString::from(token),
            });
            messenger.register(Box::new(provider));
        }
        MessagingRouteConfig::Slack {
            bot_token,
            bot_token_env,
            ..
        } => {
            let token = resolve_secret(bot_token.as_deref(), bot_token_env)
                .map_err(|e| format!("Slack: {}", e))?;
            let provider = SlackProvider::new(SlackConfig {
                bot_token: SecretString::from(token),
                api_base_url: None,
            });
            messenger.register(Box::new(provider));
        }
        MessagingRouteConfig::Signal {
            rpc_url,
            rpc_url_env,
            account,
            account_env,
            ..
        } => {
            let resolved_rpc_url = resolve_secret(rpc_url.as_deref(), rpc_url_env)
                .map_err(|e| format!("Signal RPC URL: {}", e))?;
            let resolved_account = resolve_secret(account.as_deref(), account_env)
                .map_err(|e| format!("Signal account: {}", e))?;
            let provider = SignalProvider::new(SignalConfig {
                rpc_url: resolved_rpc_url,
                account: resolved_account,
            });
            messenger.register(Box::new(provider));
        }
        MessagingRouteConfig::WhatsApp {
            access_token,
            access_token_env,
            phone_number_id,
            phone_number_id_env,
            ..
        } => {
            let token = resolve_secret(access_token.as_deref(), access_token_env)
                .map_err(|e| format!("WhatsApp access token: {}", e))?;
            let phone_id = resolve_secret(phone_number_id.as_deref(), phone_number_id_env)
                .map_err(|e| format!("WhatsApp phone number ID: {}", e))?;
            let provider = WhatsAppProvider::new(WhatsAppConfig {
                access_token: SecretString::from(token),
                phone_number_id: phone_id,
                api_version: None,
                api_base_url: None,
            });
            messenger.register(Box::new(provider));
        }
    }

    // Create dispatch and plan the send
    let dispatch = Dispatch::to(payload.target);
    let plan = messenger
        .plan_send(dispatch, &payload.message)
        .map_err(|e| format!("Failed to plan send: {}", e))?;

    // Log compatibility warnings
    if !plan.warnings.is_empty() {
        warn!(
            route = route.name,
            provider = provider_kind_label(&payload.provider_kind),
            warnings = ?plan.warnings,
            "Message has compatibility warnings"
        );
    }

    // Execute the send
    messenger
        .send_planned(plan)
        .await
        .map_err(|e| format!("Send failed: {}", e))?;

    debug!(
        route = route.name,
        provider = provider_kind_label(&payload.provider_kind),
        "Message sent successfully"
    );

    Ok(())
}

/// Returns a lowercase label for the provider kind for logging.
fn provider_kind_label(kind: &ProviderKind) -> &'static str {
    match kind {
        ProviderKind::Discord => "discord",
        ProviderKind::Slack => "slack",
        ProviderKind::Signal => "signal",
        ProviderKind::WhatsApp => "whatsapp",
        ProviderKind::Telegram => "telegram",
    }
}

/// Helper to map MessagingRouteConfig to a ProviderKind label string.
fn provider_kind_label_from_config(config: &MessagingRouteConfig) -> &'static str {
    match config {
        MessagingRouteConfig::Discord { .. } => "discord",
        MessagingRouteConfig::Slack { .. } => "slack",
        MessagingRouteConfig::Signal { .. } => "signal",
        MessagingRouteConfig::WhatsApp { .. } => "whatsapp",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messaging::{MessagingRouteConfig, MessagingScope};

    fn route(name: &str, config: MessagingRouteConfig) -> ResolvedMessagingRoute {
        ResolvedMessagingRoute {
            scope: MessagingScope::User,
            name: name.to_string(),
            config,
        }
    }

    #[test]
    fn build_payload_skips_image_only_message_for_slack() {
        let route = route(
            "ops",
            MessagingRouteConfig::Slack {
                channel_id: "C123".to_string(),
                bot_token: None,
                bot_token_env: "SLACK_BOT_TOKEN".to_string(),
            },
        );

        let payload = build_payload(
            &route,
            "   ".to_string(),
            Some("screenshots/result.png".to_string()),
            Some("/tmp"),
            None,
        );

        assert!(payload.is_none());
    }

    #[test]
    fn build_payload_keeps_image_only_message_for_discord() {
        let route = route(
            "alerts",
            MessagingRouteConfig::Discord {
                channel_id: "123".to_string(),
                bot_token: None,
                bot_token_env: "DISCORD_BOT_TOKEN".to_string(),
            },
        );

        let payload = build_payload(
            &route,
            "".to_string(),
            Some("images/chart.png".to_string()),
            Some("/workspace"),
            None,
        )
        .expect("discord image-only payload should be kept");

        assert!(payload.message.body.is_none());
        assert_eq!(payload.message.attachments.len(), 1);
    }

    #[test]
    fn build_payload_ignores_unsupported_image_when_text_exists() {
        let route = route(
            "ops",
            MessagingRouteConfig::Slack {
                channel_id: "C123".to_string(),
                bot_token: None,
                bot_token_env: "SLACK_BOT_TOKEN".to_string(),
            },
        );

        let payload = build_payload(
            &route,
            "**Deploy finished**".to_string(),
            Some("images/chart.png".to_string()),
            Some("/workspace"),
            None,
        )
        .expect("text payload should still be sent");

        assert_eq!(
            payload.message.body,
            Some(messenger::MessageBody::Markdown(
                "**Deploy finished**".to_string()
            ))
        );
        assert!(payload.message.attachments.is_empty());
    }

    #[test]
    fn execute_resolved_message_with_no_route_is_noop() {
        let messaging = RuntimeMessagingSettings {
            user: None,
            repo: None,
        };
        execute_resolved_message("Hello world", None, None, None, &messaging);
    }

    #[test]
    fn execute_resolved_message_empty_text_is_noop() {
        let messaging = RuntimeMessagingSettings {
            user: None,
            repo: None,
        };
        execute_resolved_message("  ", None, None, None, &messaging);
    }
}
