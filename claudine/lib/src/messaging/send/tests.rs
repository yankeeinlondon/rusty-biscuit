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

#[test]
fn failure_hint_recognizes_auth_failures() {
    assert_eq!(
        failure_hint("Send failed: 401 Unauthorized"),
        Some("Check the route's credentials — the provider rejected the token.")
    );
    assert_eq!(
        failure_hint("Slack API error: invalid_auth"),
        Some("Check the route's credentials — the provider rejected the token.")
    );
}

#[test]
fn failure_hint_recognizes_missing_env_var() {
    assert_eq!(
        failure_hint("Discord: env var DISCORD_BOT_TOKEN is not set"),
        Some("Set the referenced environment variable or supply the secret inline.")
    );
}

#[test]
fn failure_hint_recognizes_channel_not_found() {
    assert_eq!(
        failure_hint("Slack API error: channel_not_found"),
        Some("Verify the channel, recipient, or group id (or recreate the webhook).")
    );
}

#[test]
fn failure_hint_returns_none_for_unknown_errors() {
    assert!(failure_hint("something weird happened").is_none());
}

#[test]
fn failure_hint_recognizes_webhook_url_errors() {
    assert_eq!(
        failure_hint("Discord webhook: invalid webhook URL"),
        Some("Re-check the configured webhook URL format.")
    );
    assert_eq!(
        failure_hint("Slack webhook: invalid message — webhook URL host mismatch"),
        Some("Re-check the configured webhook URL format.")
    );
}

#[test]
fn failure_hint_recognizes_slack_no_service() {
    assert_eq!(
        failure_hint("Slack webhook: no_service"),
        Some("The Slack webhook was disabled or deleted — recreate it in Slack.")
    );
}

#[test]
fn failure_hint_discord_rate_limit() {
    let hint = failure_hint("Discord returned 429: rate limited, retry_after: 5000");
    assert!(hint.is_some());
    assert!(hint.unwrap().contains("rate limit"));

    assert!(
        failure_hint("429 Too Many Requests")
            .unwrap()
            .contains("rate limit")
    );
    assert!(
        failure_hint("Hit retry_after")
            .unwrap()
            .contains("rate limit")
    );
}

#[test]
fn prose_escape_neutralizes_angle_brackets() {
    assert_eq!(
        prose_escape("<script>alert('x')</script>"),
        "\\<script\\>alert('x')\\</script\\>"
    );
}

#[test]
fn prose_escape_neutralizes_template_and_bold_tokens() {
    assert_eq!(prose_escape("{{variable}}"), "\\{{variable\\}}");
    assert_eq!(prose_escape("**bold**"), "\\*\\*bold\\*\\*");
    assert_eq!(
        prose_escape("Error: {{url}} and **token**"),
        "Error: \\{{url\\}} and \\*\\*token\\*\\*"
    );
}

// =====================================================================
// Webhook payload build tests (Phase 2)
// =====================================================================

#[test]
fn build_payload_maps_discord_webhook_to_correct_target_and_kind() {
    let route = route(
        "alerts",
        MessagingRouteConfig::DiscordWebhook {
            webhook_url: None,
            webhook_url_env: "DISCORD_WEBHOOK_URL".to_string(),
        },
    );

    let payload = build_payload(
        &route,
        "deploy finished".to_string(),
        None,
        Some("/workspace"),
        None,
    )
    .expect("discord webhook text payload should be kept");

    assert_eq!(payload.provider_kind, ProviderKind::DiscordWebhook);
    assert!(matches!(payload.target, Target::DiscordWebhook(_)));
}

#[test]
fn build_payload_maps_slack_webhook_to_correct_target_and_kind() {
    let route = route(
        "deploys",
        MessagingRouteConfig::SlackWebhook {
            webhook_url: None,
            webhook_url_env: "SLACK_WEBHOOK_URL".to_string(),
        },
    );

    let payload = build_payload(
        &route,
        "deploy finished".to_string(),
        None,
        Some("/workspace"),
        None,
    )
    .expect("slack webhook text payload should be kept");

    assert_eq!(payload.provider_kind, ProviderKind::SlackWebhook);
    assert!(matches!(payload.target, Target::SlackWebhook(_)));
}

#[test]
fn build_payload_keeps_image_only_for_discord_webhook() {
    let route = route(
        "alerts",
        MessagingRouteConfig::DiscordWebhook {
            webhook_url: None,
            webhook_url_env: "DISCORD_WEBHOOK_URL".to_string(),
        },
    );

    let payload = build_payload(
        &route,
        "".to_string(),
        Some("images/chart.png".to_string()),
        Some("/workspace"),
        None,
    )
    .expect("discord webhook image-only payload should be kept");

    assert!(payload.message.body.is_none());
    assert_eq!(payload.message.attachments.len(), 1);
}

#[test]
fn build_payload_skips_image_only_for_slack_webhook() {
    let route = route(
        "deploys",
        MessagingRouteConfig::SlackWebhook {
            webhook_url: None,
            webhook_url_env: "SLACK_WEBHOOK_URL".to_string(),
        },
    );

    let payload = build_payload(
        &route,
        "   ".to_string(),
        Some("images/chart.png".to_string()),
        Some("/workspace"),
        None,
    );

    assert!(payload.is_none());
}

#[test]
fn build_payload_slack_webhook_text_drops_image() {
    let route = route(
        "deploys",
        MessagingRouteConfig::SlackWebhook {
            webhook_url: None,
            webhook_url_env: "SLACK_WEBHOOK_URL".to_string(),
        },
    );

    let payload = build_payload(
        &route,
        "**Deploy finished**".to_string(),
        Some("images/chart.png".to_string()),
        Some("/workspace"),
        None,
    )
    .expect("slack webhook text payload should be kept");

    assert!(payload.message.attachments.is_empty());
}

// =====================================================================
// HookAction::Message through webhook route (Finding #6)
// =====================================================================

#[test]
fn build_payload_discord_webhook_message_body() {
    let route = route(
        "alerts",
        MessagingRouteConfig::DiscordWebhook {
            webhook_url: Some("https://discord.com/api/webhooks/123/abc".to_string()),
            webhook_url_env: "DISCORD_WEBHOOK_URL".to_string(),
        },
    );

    let payload = build_payload(
        &route,
        "Hello webhook".to_string(),
        None,
        Some("/workspace"),
        None,
    )
    .expect("discord webhook payload should build");

    assert!(matches!(payload.target, Target::DiscordWebhook(_)));
    assert_eq!(payload.provider_kind, ProviderKind::DiscordWebhook);
    assert_eq!(
        payload.message.body,
        Some(messenger::MessageBody::Markdown(
            "Hello webhook".to_string()
        ))
    );
}

#[test]
fn build_payload_slack_webhook_message_body() {
    let route = route(
        "deploys",
        MessagingRouteConfig::SlackWebhook {
            webhook_url: Some("https://hooks.slack.com/services/T1/B1/X".to_string()),
            webhook_url_env: "SLACK_WEBHOOK_URL".to_string(),
        },
    );

    let payload = build_payload(
        &route,
        "Hello slack webhook".to_string(),
        None,
        Some("/workspace"),
        None,
    )
    .expect("slack webhook payload should build");

    assert!(matches!(payload.target, Target::SlackWebhook(_)));
    assert_eq!(payload.provider_kind, ProviderKind::SlackWebhook);
    assert_eq!(
        payload.message.body,
        Some(messenger::MessageBody::Markdown(
            "Hello slack webhook".to_string()
        ))
    );
}

// =====================================================================
// Redaction tests (Phase 2)
// =====================================================================

#[test]
fn redact_webhook_urls_masks_discord_webhook() {
    let input =
        "failed to reach https://discord.com/api/webhooks/123456789/abcDEF-secret_token xyz";
    let output = redact_webhook_urls(input);
    assert!(!output.contains("abcDEF-secret_token"));
    assert!(!output.contains("123456789"));
    assert!(output.contains("<redacted-webhook-url>"));
}

#[test]
fn redact_webhook_urls_masks_discordapp_alias() {
    let input = "https://discordapp.com/api/webhooks/9999/supersecret";
    let output = redact_webhook_urls(input);
    assert!(!output.contains("supersecret"));
    assert!(output.contains("<redacted-webhook-url>"));
}

#[test]
fn redact_webhook_urls_masks_slack_webhook() {
    let input = "POST https://hooks.slack.com/services/T00ABC/B00XYZ/super_secret_token failed";
    let output = redact_webhook_urls(input);
    assert!(!output.contains("super_secret_token"));
    assert!(!output.contains("T00ABC"));
    assert!(output.contains("<redacted-webhook-url>"));
}

#[test]
fn redact_webhook_urls_leaves_safe_text_alone() {
    let input = "Send failed: 401 Unauthorized";
    assert_eq!(redact_webhook_urls(input), input);
}

#[test]
fn redactor_covers_every_valid_webhook_url() {
    let samples = [
        // Discord
        "https://discord.com/api/webhooks/123456789/abcDEF123.-_",
        "https://discordapp.com/api/webhooks/987654321/XYZ789",
        // Slack
        "https://hooks.slack.com/services/T00000000/B00000000/XXXXXXXXXXXXXXXXXXXXXXXX",
        "https://hooks.slack.com/services/T123/B456/abc123DEF",
    ];

    for url in &samples {
        let is_valid = super::super::config::validate_discord_webhook_url(url)
            || super::super::config::validate_slack_webhook_url(url);
        assert!(is_valid, "sample URL should be valid: {url}");

        let redacted = redact_webhook_urls(&format!("prefix {url} suffix"));
        assert!(
            !redacted.contains(url),
            "redaction failed on valid URL: {url}"
        );
    }
}

#[test]
fn redact_webhook_urls_masks_multiple_urls_in_one_string() {
    let input = "first https://discord.com/api/webhooks/1/aaa and second \
                 https://hooks.slack.com/services/T0/B0/ccc both leaked";
    let output = redact_webhook_urls(input);
    assert!(!output.contains("aaa"));
    assert!(!output.contains("ccc"));
    assert_eq!(output.matches("<redacted-webhook-url>").count(), 2);
}

// =====================================================================
// Secret resolution and webhook provider construction (Phase 2)
// =====================================================================

#[test]
fn resolve_secret_inline_wins_over_env() {
    let result =
        super::super::resolve::resolve_secret(Some("inline-wins"), "DEFINITELY_UNSET_VAR_X");
    assert_eq!(result, Ok("inline-wins".to_string()));
}

// =====================================================================
// execute_notification (Phase 2)
// =====================================================================

#[test]
fn execute_notification_blank_is_noop() {
    // Blank titles are no-ops and return immediately.
    execute_notification("   ", None);
    execute_notification("", None);
    execute_notification("\n\t", None);
    execute_notification("   ", Some("body should not matter"));
}

#[test]
fn execute_notification_no_panic_without_runtime() {
    // Must not panic when called outside a Tokio runtime.
    execute_notification("test title", None);
    execute_notification("test title", Some("test body"));
}

#[test]
fn build_notification_message_title_only() {
    let title = "Deployment Successful";
    let message = build_notification_message(title, None);
    assert_eq!(message.title.as_deref(), Some(title));
    assert!(message.body.is_none());
    assert!(message.attachments.is_empty());
}

#[test]
fn build_notification_message_with_body() {
    let title = "Deployment Successful";
    let body = "Released v1.2.3 to production";
    let message = build_notification_message(title, Some(body));
    assert_eq!(message.title.as_deref(), Some(title));
    assert_eq!(message.body, Some(MessageBody::Plain(body.to_string())));
    assert!(message.attachments.is_empty());
}

// =====================================================================
// test_webhook_connection (Phase 5)
// =====================================================================

#[tokio::test]
async fn test_webhook_connection_rejects_non_webhook_route() {
    let route = MessagingRouteConfig::Discord {
        channel_id: "123".to_string(),
        bot_token: None,
        bot_token_env: "DISCORD_BOT_TOKEN".to_string(),
    };
    let result = test_webhook_connection(&route).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Not a webhook route"));
}

#[tokio::test]
async fn test_webhook_connection_rejects_invalid_discord_url() {
    let route = MessagingRouteConfig::DiscordWebhook {
        webhook_url: Some("not-a-valid-url".to_string()),
        webhook_url_env: "DISCORD_WEBHOOK_URL".to_string(),
    };
    let result = test_webhook_connection(&route).await;
    assert!(result.is_err());
    // The error should be redacted (no raw URL in output)
    let err = result.unwrap_err();
    assert!(!err.contains("not-a-valid-url"));
}

#[tokio::test]
async fn test_webhook_connection_rejects_missing_secret() {
    let route = MessagingRouteConfig::SlackWebhook {
        webhook_url: None,
        webhook_url_env: "DEFINITELY_UNSET_ENV_VAR_FOR_TEST".to_string(),
    };
    let result = test_webhook_connection(&route).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("secret not found"));
}

#[test]
fn redact_webhook_urls_in_error_strings() {
    // Deterministic test: verify the redactor strips URLs from known
    // reqwest-style error strings without relying on network timing.
    let url = "https://discord.com/api/webhooks/123/abc_secret";
    let error =
        format!("reqwest error: error sending request for url ({url}): connection failed");
    let redacted = redact_webhook_urls(&error);
    assert!(
        !redacted.contains(url),
        "URL must be redacted in error: {redacted}"
    );
    assert!(
        redacted.contains("<redacted-webhook-url>"),
        "redaction marker should be present"
    );
}
