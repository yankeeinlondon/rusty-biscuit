use std::collections::BTreeMap;

use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};

use crate::capabilities::CapabilitySet;
use crate::dispatch::Dispatch;
use crate::error::MessengerError;
use crate::prepared::PreparedMessage;
use crate::receipt::{MessageRef, ProviderKind, SendReceipt};
use crate::target::Target;

/// Expected host for Slack Incoming Webhook URLs.
const SLACK_WEBHOOK_HOST: &str = "hooks.slack.com";
/// Expected path prefix for Slack Incoming Webhook URLs.
const SLACK_WEBHOOK_PATH_PREFIX: &str = "/services/";

/// Configuration for the Slack Incoming Webhook provider.
///
/// The `webhook_url` must be a complete Slack Incoming Webhook URL in the form
/// `https://hooks.slack.com/services/{team}/{app}/{token}`. The URL binds both
/// the workspace and the target channel.
pub struct SlackWebhookConfig {
    pub webhook_url: SecretString,
}

/// Slack Incoming Webhook provider adapter.
///
/// Uses `reqwest` directly against the validated webhook URL, matching the
/// existing Slack bot provider's transport style. Production URLs are strictly
/// validated against `hooks.slack.com` at construction time; integration tests
/// can bypass validation through the crate-private
/// [`SlackWebhookProvider::new_unchecked`] constructor to target a local mock
/// server.
pub struct SlackWebhookProvider {
    client: reqwest::Client,
    webhook_url: SecretString,
}

impl SlackWebhookProvider {
    /// Construct a provider from a strictly validated Slack webhook URL.
    ///
    /// ## Errors
    ///
    /// Returns [`MessengerError::InvalidMessage`] if the URL does not meet the
    /// Slack Incoming Webhook format (see [`validate_webhook_url`] for the
    /// exact rules).
    pub fn try_new(config: SlackWebhookConfig) -> Result<Self, MessengerError> {
        validate_webhook_url(config.webhook_url.expose_secret())?;
        Ok(Self {
            client: reqwest::Client::new(),
            webhook_url: config.webhook_url,
        })
    }

    /// Construct a provider without validating the webhook URL.
    ///
    /// Used by integration tests to target a wiremock server. Production code
    /// must use [`SlackWebhookProvider::try_new`] so the strict
    /// `hooks.slack.com` validation is enforced.
    #[cfg(test)]
    #[allow(dead_code)]
    pub(crate) fn new_unchecked(webhook_url: SecretString) -> Self {
        Self {
            client: reqwest::Client::new(),
            webhook_url,
        }
    }
}

/// Validate a Slack Incoming Webhook URL against the strict runtime rules.
///
/// ## Errors
///
/// Returns [`MessengerError::InvalidMessage`] for any of:
/// - Empty or unparseable URL
/// - Scheme other than `https`
/// - Host not equal to `hooks.slack.com` (case-insensitive)
/// - Path not beginning with `/services/`
/// - Fewer or more than three non-empty path segments after `/services/`
/// - Any segment that decodes to empty or whitespace-only
/// - Trailing slash after the third segment
/// - Query string
/// - Fragment
fn validate_webhook_url(raw: &str) -> Result<(), MessengerError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(MessengerError::InvalidMessage(
            "Slack webhook URL is empty".into(),
        ));
    }

    let parsed = reqwest::Url::parse(trimmed).map_err(|error| {
        MessengerError::InvalidMessage(format!("Slack webhook URL is invalid: {error}"))
    })?;

    if parsed.scheme() != "https" {
        return Err(MessengerError::InvalidMessage(format!(
            "Slack webhook URL must use https, got {}",
            parsed.scheme()
        )));
    }

    let host = parsed.host_str().ok_or_else(|| {
        MessengerError::InvalidMessage("Slack webhook URL is missing a host".into())
    })?;
    if !host.eq_ignore_ascii_case(SLACK_WEBHOOK_HOST) {
        return Err(MessengerError::InvalidMessage(format!(
            "Slack webhook URL host must be {SLACK_WEBHOOK_HOST}, got {host}"
        )));
    }

    if parsed.query().is_some() {
        return Err(MessengerError::InvalidMessage(
            "Slack webhook URL must not contain a query string".into(),
        ));
    }
    if parsed.fragment().is_some() {
        return Err(MessengerError::InvalidMessage(
            "Slack webhook URL must not contain a fragment".into(),
        ));
    }

    let path = parsed.path();
    if !path.starts_with(SLACK_WEBHOOK_PATH_PREFIX) {
        return Err(MessengerError::InvalidMessage(format!(
            "Slack webhook URL path must begin with {SLACK_WEBHOOK_PATH_PREFIX}"
        )));
    }

    if path.len() > SLACK_WEBHOOK_PATH_PREFIX.len() && path.ends_with('/') {
        return Err(MessengerError::InvalidMessage(
            "Slack webhook URL must not have a trailing slash".into(),
        ));
    }

    let tail = &path[SLACK_WEBHOOK_PATH_PREFIX.len()..];
    let segments: Vec<&str> = tail.split('/').collect();
    if segments.len() != 3 {
        return Err(MessengerError::InvalidMessage(format!(
            "Slack webhook URL must have exactly three segments after {SLACK_WEBHOOK_PATH_PREFIX}, got {}",
            segments.len()
        )));
    }
    for segment in &segments {
        let decoded = percent_decode(segment);
        if decoded.trim().is_empty() {
            return Err(MessengerError::InvalidMessage(
                "Slack webhook URL contains an empty or whitespace-only segment".into(),
            ));
        }
    }

    Ok(())
}

/// Minimal percent-decoder that replaces `%XX` with the decoded byte,
/// interpreting the result as UTF-8 lossily. Non-UTF-8 bytes fall back to the
/// raw input — good enough for the whitespace-only check.
fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hi = (bytes[i + 1] as char).to_digit(16);
            let lo = (bytes[i + 2] as char).to_digit(16);
            if let (Some(hi), Some(lo)) = (hi, lo) {
                out.push((hi * 16 + lo) as u8);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Request body posted to Slack Incoming Webhooks.
#[derive(Serialize)]
struct WebhookRequest<'a> {
    text: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    thread_ts: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    unfurl_links: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    unfurl_media: Option<bool>,
}

/// Slack Incoming Webhook JSON response body (errors only — success is `{"ok": true}`).
#[derive(Deserialize)]
struct WebhookResponse {
    ok: bool,
    #[serde(default)]
    error: Option<String>,
}

#[async_trait::async_trait]
impl super::Provider for SlackWebhookProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::SlackWebhook
    }

    fn capabilities(&self) -> CapabilitySet {
        const SLACK_WEBHOOK_CAPABILITIES: CapabilitySet = CapabilitySet {
            supports_markdown_rendering: true,
            supports_reply: true,
            supports_attachments: false,
            supports_location: true,
            supports_silent_delivery: false,
            supports_link_preview_control: true,
        };
        SLACK_WEBHOOK_CAPABILITIES
    }

    #[tracing::instrument(skip_all, fields(provider = "slack-webhook"))]
    async fn send_prepared(
        &self,
        dispatch: &Dispatch,
        message: &PreparedMessage,
    ) -> Result<SendReceipt, MessengerError> {
        match &dispatch.target {
            Target::SlackWebhook(_) => {}
            _ => {
                return Err(MessengerError::InvalidMessage(
                    "expected SlackWebhook target".into(),
                ));
            }
        }

        let text = message.render_body_with_location(ProviderKind::SlackWebhook);

        let thread_ts = match &dispatch.reply_to {
            Some(MessageRef::SlackWebhook {
                thread_ts: Some(ts),
            }) => Some(ts.as_str()),
            _ => None,
        };

        let (unfurl_links, unfurl_media) = if dispatch.options.disable_link_preview {
            (Some(false), Some(false))
        } else {
            (None, None)
        };

        tracing::debug!(
            has_reply = thread_ts.is_some(),
            disable_link_preview = dispatch.options.disable_link_preview,
            text_len = text.len(),
            "sending Slack webhook message"
        );

        let body = WebhookRequest {
            text: &text,
            thread_ts,
            unfurl_links,
            unfurl_media,
        };

        let response = self
            .client
            .post(self.webhook_url.expose_secret())
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderKind::SlackWebhook.transport_error(e))?;

        let status = response.status();
        tracing::debug!(status = %status, "received Slack webhook response");

        if status.as_u16() == 429 {
            let retry_after = response
                .headers()
                .get("Retry-After")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
                .map(|s| s * 1000);
            tracing::warn!(
                retry_after_ms = ?retry_after,
                "Slack webhook rate limited"
            );
            return Err(MessengerError::RateLimited {
                provider: ProviderKind::SlackWebhook,
                retry_after_ms: retry_after,
            });
        }

        if status.is_server_error() {
            tracing::warn!(status = %status, "Slack webhook server error");
            return Err(
                ProviderKind::SlackWebhook.transport_error(format!("server error: {status}"))
            );
        }

        let resp: WebhookResponse = response
            .json()
            .await
            .map_err(|e| ProviderKind::SlackWebhook.transport_error(e))?;

        if !resp.ok {
            let error = resp.error.unwrap_or_else(|| "unknown error".into());
            return Err(map_webhook_error(&error));
        }

        let mut metadata = BTreeMap::new();
        metadata.insert("delivery_confirmed".to_string(), "true".to_string());

        Ok(SendReceipt {
            provider: ProviderKind::SlackWebhook,
            message_ref: MessageRef::SlackWebhook { thread_ts: None },
            raw_id: String::new(),
            metadata,
        })
    }
}

/// Map a Slack Incoming Webhook error string to the appropriate
/// [`MessengerError`] variant.
fn map_webhook_error(code: &str) -> MessengerError {
    match code {
        "invalid_token" | "no_service" | "no_team" | "action_prohibited" => {
            tracing::warn!(code, "Slack webhook authentication failed");
            MessengerError::Authentication {
                provider: ProviderKind::SlackWebhook,
                message: code.to_string(),
            }
        }
        "invalid_payload" | "channel_is_archived" | "channel_not_found" => {
            tracing::warn!(code, "Slack webhook rejected payload");
            MessengerError::InvalidMessage(format!("Slack webhook error: {code}"))
        }
        _ => {
            tracing::warn!(code, "Slack webhook returned provider error");
            MessengerError::Provider {
                provider: ProviderKind::SlackWebhook,
                code: Some(code.to_string()),
                message: code.to_string(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn expect_err(url: &str) -> MessengerError {
        match validate_webhook_url(url) {
            Ok(()) => panic!("expected webhook URL validation to fail for {url:?}"),
            Err(err) => err,
        }
    }

    #[test]
    fn accepts_standard_webhook_url() {
        validate_webhook_url("https://hooks.slack.com/services/T000/B000/XXXXX").unwrap();
    }

    #[test]
    fn accepts_mixed_case_host() {
        validate_webhook_url("https://Hooks.Slack.Com/services/T000/B000/XXXXX").unwrap();
    }

    #[test]
    fn rejects_empty_url() {
        let err = expect_err("");
        assert!(matches!(
            err,
            MessengerError::InvalidMessage(message) if message.contains("empty")
        ));
    }

    #[test]
    fn rejects_http_scheme() {
        let err = expect_err("http://hooks.slack.com/services/T000/B000/XXXXX");
        assert!(matches!(
            err,
            MessengerError::InvalidMessage(message) if message.contains("https")
        ));
    }

    #[test]
    fn rejects_wrong_host() {
        let err = expect_err("https://hooks.slack.example/services/T000/B000/XXXXX");
        assert!(matches!(
            err,
            MessengerError::InvalidMessage(message) if message.contains("host")
        ));
    }

    #[test]
    fn rejects_subdomain_host() {
        let err = expect_err("https://a.hooks.slack.com/services/T000/B000/XXXXX");
        assert!(matches!(
            err,
            MessengerError::InvalidMessage(message) if message.contains("host")
        ));
    }

    #[test]
    fn rejects_wrong_path_prefix() {
        let err = expect_err("https://hooks.slack.com/services_v2/T000/B000/XXXXX");
        assert!(matches!(
            err,
            MessengerError::InvalidMessage(message) if message.contains("/services/")
        ));
    }

    #[test]
    fn rejects_too_few_segments() {
        let err = expect_err("https://hooks.slack.com/services/T000/B000");
        assert!(matches!(
            err,
            MessengerError::InvalidMessage(message) if message.contains("three segments")
        ));
    }

    #[test]
    fn rejects_too_many_segments() {
        let err = expect_err("https://hooks.slack.com/services/T000/B000/XXXXX/extra");
        assert!(matches!(
            err,
            MessengerError::InvalidMessage(message) if message.contains("three segments")
        ));
    }

    #[test]
    fn rejects_empty_segment() {
        let err = expect_err("https://hooks.slack.com/services/T000//XXXXX");
        assert!(matches!(
            err,
            MessengerError::InvalidMessage(message) if message.contains("empty")
        ));
    }

    #[test]
    fn rejects_whitespace_only_segment() {
        let err = expect_err("https://hooks.slack.com/services/T000/%20/XXXXX");
        assert!(matches!(
            err,
            MessengerError::InvalidMessage(message) if message.contains("empty")
        ));
    }

    #[test]
    fn rejects_trailing_slash() {
        let err = expect_err("https://hooks.slack.com/services/T000/B000/XXXXX/");
        assert!(matches!(
            err,
            MessengerError::InvalidMessage(message) if message.contains("trailing")
        ));
    }

    #[test]
    fn rejects_query_string() {
        let err = expect_err("https://hooks.slack.com/services/T000/B000/XXXXX?foo=bar");
        assert!(matches!(
            err,
            MessengerError::InvalidMessage(message) if message.contains("query")
        ));
    }

    #[test]
    fn rejects_fragment() {
        let err = expect_err("https://hooks.slack.com/services/T000/B000/XXXXX#frag");
        assert!(matches!(
            err,
            MessengerError::InvalidMessage(message) if message.contains("fragment")
        ));
    }

    #[test]
    fn try_new_rejects_malformed_url() {
        let result = SlackWebhookProvider::try_new(SlackWebhookConfig {
            webhook_url: SecretString::from("not a url"),
        });
        let err = result.err().expect("expected invalid URL to fail");
        assert!(matches!(err, MessengerError::InvalidMessage(_)));
    }

    #[test]
    fn try_new_accepts_valid_url() {
        SlackWebhookProvider::try_new(SlackWebhookConfig {
            webhook_url: SecretString::from(
                "https://hooks.slack.com/services/T000/B000/XXXXX".to_string(),
            ),
        })
        .unwrap();
    }

    #[tokio::test]
    async fn capabilities_match_spec() {
        let provider = SlackWebhookProvider::try_new(SlackWebhookConfig {
            webhook_url: SecretString::from(
                "https://hooks.slack.com/services/T000/B000/XXXXX".to_string(),
            ),
        })
        .unwrap();
        let caps = super::super::Provider::capabilities(&provider);
        assert!(caps.supports_markdown_rendering);
        assert!(caps.supports_reply);
        assert!(!caps.supports_attachments);
        assert!(caps.supports_location);
        assert!(!caps.supports_silent_delivery);
        assert!(caps.supports_link_preview_control);
    }

    #[test]
    fn kind_reports_slack_webhook() {
        let provider = SlackWebhookProvider::try_new(SlackWebhookConfig {
            webhook_url: SecretString::from(
                "https://hooks.slack.com/services/T000/B000/XXXXX".to_string(),
            ),
        })
        .unwrap();
        assert_eq!(
            super::super::Provider::kind(&provider),
            ProviderKind::SlackWebhook
        );
    }

    #[test]
    fn map_webhook_error_authentication_codes() {
        for code in ["invalid_token", "no_service", "no_team", "action_prohibited"] {
            let err = map_webhook_error(code);
            assert!(
                matches!(err, MessengerError::Authentication { .. }),
                "expected Authentication for {code}, got {err:?}"
            );
        }
    }

    #[test]
    fn map_webhook_error_invalid_message_codes() {
        for code in ["invalid_payload", "channel_is_archived", "channel_not_found"] {
            let err = map_webhook_error(code);
            assert!(
                matches!(err, MessengerError::InvalidMessage(_)),
                "expected InvalidMessage for {code}, got {err:?}"
            );
        }
    }

    #[test]
    fn map_webhook_error_unknown_codes_map_to_provider() {
        let err = map_webhook_error("mystery_error");
        assert!(
            matches!(err, MessengerError::Provider { .. }),
            "expected Provider, got {err:?}"
        );
    }
}
