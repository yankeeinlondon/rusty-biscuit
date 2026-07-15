//! Send helper for executing outbound messages via the messenger library.
//!
//! This module bridges Claudine's messaging configuration with the messenger
//! library, handling template interpolation, provider construction, and
//! fire-and-forget async dispatch.

use std::collections::BTreeMap;
use std::path::Path;

use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::status::{Status, StatusState};
use biscuit_terminal::terminal::Terminal;
use messenger::provider::{
    Messenger,
    discord::{DiscordConfig, DiscordProvider},
    discord_webhook::{DiscordWebhookConfig, DiscordWebhookProvider},
    signal::{SignalConfig, SignalProvider},
    slack::{SlackConfig, SlackProvider},
    slack_webhook::{SlackWebhookConfig, SlackWebhookProvider},
    whatsapp::{WhatsAppConfig, WhatsAppProvider},
};
use messenger::target::SignalAddress;
use messenger::{
    DesktopConfig, DesktopNotificationProvider, Dispatch, Message, MessageBody, ProviderKind,
    Target,
};
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
            report_send_failure(&route, &e, "message");
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
            report_send_failure(&route, &e, "lifecycle message");
        }
    });
}

/// Register a webhook provider on a messenger from a webhook route config.
///
/// This helper is shared between the production send path and the config TUI
/// test-connection path so that both use identical provider construction and
/// redaction logic.
fn register_webhook_provider(
    messenger: &mut Messenger,
    config: &MessagingRouteConfig,
) -> Result<(Target, ProviderKind), String> {
    match config {
        MessagingRouteConfig::DiscordWebhook {
            webhook_url,
            webhook_url_env,
        } => {
            let url = resolve_secret(webhook_url.as_deref(), webhook_url_env)
                .map_err(|e| format!("Discord webhook URL: {}", e))?;
            let provider = DiscordWebhookProvider::try_new(DiscordWebhookConfig {
                webhook_url: SecretString::from(url),
            })
            .map_err(|e| redact_webhook_urls(&format!("Discord webhook: {}", e)))?;
            messenger.register(Box::new(provider));
            Ok((Target::discord_webhook(), ProviderKind::DiscordWebhook))
        }
        MessagingRouteConfig::SlackWebhook {
            webhook_url,
            webhook_url_env,
        } => {
            let url = resolve_secret(webhook_url.as_deref(), webhook_url_env)
                .map_err(|e| format!("Slack webhook URL: {}", e))?;
            let provider = SlackWebhookProvider::try_new(SlackWebhookConfig {
                webhook_url: SecretString::from(url),
            })
            .map_err(|e| redact_webhook_urls(&format!("Slack webhook: {}", e)))?;
            messenger.register(Box::new(provider));
            Ok((Target::slack_webhook(), ProviderKind::SlackWebhook))
        }
        _ => Err("Not a webhook route".to_string()),
    }
}

/// Test a webhook connection by sending a short test message.
///
/// This helper is designed for the config TUI test-connection workflow.
/// It accepts a [`MessagingRouteConfig`] webhook variant and attempts to
/// send a test message without requiring the route to be active or persisted.
///
/// Uses the same provider constructors and redaction rules as normal sends.
///
/// ## Returns
///
/// `Ok(())` if the test message was dispatched successfully, or `Err(message)`
/// with a redacted error string suitable for display in the TUI.
pub async fn test_webhook_connection(config: &MessagingRouteConfig) -> Result<(), String> {
    let mut messenger = Messenger::new();
    let (target, _kind) = register_webhook_provider(&mut messenger, config)?;

    let message = Message::text("Claudine test connection".to_string());
    let dispatch = Dispatch::to(target);
    messenger
        .send(dispatch, &message)
        .await
        .map_err(|e| redact_webhook_urls(&format!("Send failed: {}", e)))?;

    Ok(())
}

/// Fire a local desktop notification with the supplied title and optional body.
///
/// This helper is intentionally zero-config: it does not read Claudine
/// messaging config, does not require an active route, and never returns an
/// error to lifecycle code. Blank or whitespace-only titles are no-ops, and
/// send failures are logged as operational warnings rather than propagated.
/// Pass `None` for `body` to render a title-only notification.
///
/// Driver selection, capability detection, and OS integration are owned by
/// `messenger::DesktopNotificationProvider::new(DesktopConfig::default())`.
pub fn execute_notification(title: &str, body: Option<&str>) {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        return;
    }

    let handle = match tokio::runtime::Handle::try_current() {
        Ok(h) => h,
        Err(_) => {
            tracing::warn!("Cannot execute desktop notification: no Tokio runtime active");
            return;
        }
    };

    let owned_title = trimmed.to_string();
    let owned_body = body.map(str::to_string);
    handle.spawn(async move {
        if let Err(e) = send_desktop_notification(&owned_title, owned_body.as_deref()).await {
            report_notification_failure(&e);
        }
    });
}

/// Build a desktop notification message with a title and optional body.
fn build_notification_message(title: &str, body: Option<&str>) -> Message {
    Message {
        title: Some(title.to_string()),
        body: body.map(|body| MessageBody::Plain(body.to_string())),
        attachments: Vec::new(),
        location: None,
        metadata: BTreeMap::new(),
    }
}

/// Build a transient messenger, register the desktop provider, and dispatch
/// a notification to the current host OS.
async fn send_desktop_notification(title: &str, body: Option<&str>) -> Result<(), String> {
    let mut messenger = Messenger::new();
    let provider = DesktopNotificationProvider::new(DesktopConfig::default());
    messenger.register(Box::new(provider));

    let message = build_notification_message(title, body);
    let dispatch = Dispatch::to(Target::desktop());

    messenger
        .send(dispatch, &message)
        .await
        .map_err(|e| format!("Desktop notification failed: {}", e))?;

    debug!(title, "Desktop notification sent");
    Ok(())
}

/// Render a user-facing Status Warning when a desktop notification fails.
///
/// Desktop notifications are a best-effort local side effect, so we surface
/// the failure without propagating an error to callers. We deliberately mirror
/// the Status rendering used by remote-messaging failures to keep warning
/// styling consistent across all operational messaging surfaces.
fn report_notification_failure(error: &str) {
    let body = format!(
        "Failed to send desktop notification: {}",
        prose_escape(error)
    );

    let rendered = Status::from_prose(body)
        .state(StatusState::Warning)
        .render(&Terminal::default());
    eprintln!("{rendered}");

    debug!(error, "desktop notification send failed");
}

/// Render a user-facing Status Warning describing a messaging send failure
/// and also log the raw error at debug level for anyone tailing traces.
///
/// The async send path has no `Terminal` handle in scope, so we build a
/// default terminal here — good enough for rendering an ANSI-colored
/// `Status::Warning` line. We deliberately avoid the tracing WARN layer
/// because this is operational feedback the user needs to see, not noise
/// for log aggregators.
fn report_send_failure(route: &ResolvedMessagingRoute, error: &str, kind: &str) {
    let provider = provider_kind_label_from_config(&route.config);
    // Defense in depth: even when callers already redact, run the regex here
    // so webhook secrets cannot reach stderr or test snapshots.
    let safe_error = redact_webhook_urls(error);
    let hint = failure_hint(&safe_error);
    let route_name = prose_escape(&route.name);
    let error_text = prose_escape(&safe_error);

    let mut body = format!(
        "Failed to send {kind} via <b>{provider}</b> route \
         <blue-500>{route_name}</blue-500>: {error_text}"
    );
    if let Some(hint) = hint {
        body.push_str(&format!("\n  <dim>\u{2192} {hint}</dim>"));
    }

    let rendered = Status::from_prose(body)
        .state(StatusState::Warning)
        .render(&Terminal::default());
    eprintln!("{rendered}");

    // Keep a machine-readable breadcrumb for log collectors without the
    // user-facing WARN noise. Use the redacted form so log aggregators never
    // receive webhook secrets either.
    debug!(
        route = route.name,
        provider,
        kind,
        error = safe_error.as_str(),
        "messaging send failed"
    );
}

/// Produce a short actionable hint based on common error signatures so the
/// warning suggests a concrete next step rather than just surfacing the
/// underlying send error.
fn failure_hint(error: &str) -> Option<&'static str> {
    let lower = error.to_ascii_lowercase();
    if lower.contains("env var") || lower.contains("environment variable") {
        Some("Set the referenced environment variable or supply the secret inline.")
    } else if lower.contains("invalid webhook")
        || lower.contains("webhook url")
        || lower.contains("invalid message")
    {
        Some("Re-check the configured webhook URL format.")
    } else if lower.contains("no_service") {
        Some("The Slack webhook was disabled or deleted — recreate it in Slack.")
    } else if lower.contains("429") || lower.contains("rate limit") || lower.contains("retry_after")
    {
        Some(
            "Discord rate limit hit. Wait a few seconds before retrying, \
             or reduce message frequency.",
        )
    } else if lower.contains("unauthorized")
        || lower.contains("401")
        || lower.contains("invalid_auth")
    {
        Some("Check the route's credentials — the provider rejected the token.")
    } else if lower.contains("forbidden") || lower.contains("403") {
        Some("Confirm the bot has permission to post to the configured channel/recipient.")
    } else if lower.contains("not found")
        || lower.contains("channel_not_found")
        || lower.contains("404")
    {
        Some("Verify the channel, recipient, or group id (or recreate the webhook).")
    } else if lower.contains("dns") || lower.contains("connect") || lower.contains("timed out") {
        Some("Check network connectivity to the messaging provider.")
    } else {
        None
    }
}

/// Redact webhook URLs from an arbitrary string.
///
/// Webhook URLs are secrets: the path segment after `/webhooks/{id}/` or
/// `/services/...` is a bearer credential. This helper swaps any Discord or
/// Slack webhook URL match with a stable placeholder before the string is
/// rendered in user-facing warnings, logs, or test snapshots.
fn redact_webhook_urls(input: &str) -> String {
    use std::sync::LazyLock;
    static DISCORD_WEBHOOK_URL_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(
            r"https://(?:discord\.com|discordapp\.com)/api/webhooks/[0-9]+/[A-Za-z0-9._-]+",
        )
        .expect("discord webhook redaction regex")
    });
    static SLACK_WEBHOOK_URL_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"https://hooks\.slack\.com/services/[A-Za-z0-9/_-]+")
            .expect("slack webhook redaction regex")
    });

    let redacted = DISCORD_WEBHOOK_URL_RE.replace_all(input, "<redacted-webhook-url>");
    let redacted = SLACK_WEBHOOK_URL_RE.replace_all(&redacted, "<redacted-webhook-url>");
    redacted.into_owned()
}

/// Escape Prose markup tokens so arbitrary error text can't be interpreted
/// as markup when embedded in a `Status::from_prose` body.
///
/// Covers `< >` (HTML-style tags), `{{ }}` (template syntax), and `**`
/// (bold) which are the most likely to appear in reqwest/serde error strings.
fn prose_escape(text: &str) -> String {
    text.replace('<', "\\<")
        .replace('>', "\\>")
        .replace("{{", "\\{{")
        .replace("}}", "\\}}")
        .replace("**", "\\*\\*")
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
        MessagingRouteConfig::DiscordWebhook { .. } => {
            (Target::discord_webhook(), ProviderKind::DiscordWebhook)
        }
        MessagingRouteConfig::SlackWebhook { .. } => {
            (Target::slack_webhook(), ProviderKind::SlackWebhook)
        }
    };

    let has_text = !text.trim().is_empty();

    // Build the message
    let mut message = if has_text {
        Message::markdown(text)
    } else {
        empty_message()
    };

    // Attach images for Discord-family providers only; warn or skip otherwise
    if let Some(img_path_str) = image {
        let resolved = resolve_image_path(&img_path_str, cwd, repo_root);

        if matches!(
            provider_kind,
            ProviderKind::Discord | ProviderKind::DiscordWebhook
        ) {
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
        title: None,
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
        MessagingRouteConfig::DiscordWebhook { .. } | MessagingRouteConfig::SlackWebhook { .. } => {
            register_webhook_provider(&mut messenger, &payload.route_config)?;
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

    // Execute the send. Webhook routes may embed a URL in transport error
    // strings (the messenger crate does not redact by default), so redact
    // any webhook URL before the error reaches user-facing logging.
    messenger
        .send_planned(plan)
        .await
        .map_err(|e| redact_webhook_urls(&format!("Send failed: {}", e)))?;

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
        ProviderKind::DiscordWebhook => "discord_webhook",
        ProviderKind::Slack => "slack",
        ProviderKind::Signal => "signal",
        ProviderKind::WhatsApp => "whatsapp",
        ProviderKind::Telegram => "telegram",
        ProviderKind::SlackWebhook => "slack_webhook",
        ProviderKind::Desktop => "desktop",
        ProviderKind::Apns => "apns",
        ProviderKind::Fcm => "fcm",
    }
}

/// Helper to map MessagingRouteConfig to a ProviderKind label string.
fn provider_kind_label_from_config(config: &MessagingRouteConfig) -> &'static str {
    match config {
        MessagingRouteConfig::Discord { .. } => "discord",
        MessagingRouteConfig::Slack { .. } => "slack",
        MessagingRouteConfig::Signal { .. } => "signal",
        MessagingRouteConfig::WhatsApp { .. } => "whatsapp",
        MessagingRouteConfig::DiscordWebhook { .. } => "discord_webhook",
        MessagingRouteConfig::SlackWebhook { .. } => "slack_webhook",
    }
}

#[cfg(test)]
mod tests;
