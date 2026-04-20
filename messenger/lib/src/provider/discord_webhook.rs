use std::collections::BTreeMap;
use std::fs;

use reqwest::multipart::{Form, Part};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};

use crate::attachment::{Attachment, AttachmentSource};
use crate::capabilities::CapabilitySet;
use crate::dispatch::Dispatch;
use crate::error::MessengerError;
use crate::prepared::PreparedMessage;
use crate::receipt::{MessageRef, ProviderKind, SendReceipt};
use crate::target::Target;

/// Configuration for the Discord webhook provider.
///
/// The `webhook_url` must be a complete Discord webhook URL in the form
/// `https://discord.com/api/v{version}/webhooks/{id}/{token}`. Both the
/// channel and authentication are bound by the URL itself.
pub struct DiscordWebhookConfig {
    pub webhook_url: SecretString,
}

/// Discord webhook provider adapter.
///
/// Uses `reqwest` directly against the parsed webhook URL rather than
/// `twilight-http`'s `execute_webhook` helper. The direct-HTTP approach keeps
/// the wiremock integration tests simple (we just substitute the mock server
/// URI) and sidesteps twilight-http's bot-token-oriented client construction.
pub struct DiscordWebhookProvider {
    client: reqwest::Client,
    webhook_id: String,
    /// The base URL up to and including the webhook token, without any query
    /// string. Used as the POST target — the token itself is embedded in
    /// the path, so no separate credential field is needed.
    base_url: String,
}

/// Fields extracted from a Discord webhook URL.
struct ParsedWebhook {
    webhook_id: String,
    /// Captured for diagnostics and future callers that may need it in
    /// isolation; the live provider currently embeds the token inside
    /// `base_url` for `reqwest` POSTs.
    #[allow(dead_code)]
    token: String,
    base_url: String,
}

impl DiscordWebhookProvider {
    /// Construct a provider from a validated webhook URL.
    ///
    /// ## Errors
    ///
    /// Returns `MessengerError::InvalidMessage` if the URL does not contain
    /// `/webhooks/{id}/{token}` segments.
    pub fn try_new(config: DiscordWebhookConfig) -> Result<Self, MessengerError> {
        let parsed = parse_webhook_url(config.webhook_url.expose_secret())?;
        Ok(Self {
            client: reqwest::Client::new(),
            webhook_id: parsed.webhook_id,
            base_url: parsed.base_url,
        })
    }

    /// Construct a provider from a webhook URL, panicking on malformed input.
    ///
    /// Mirrors `DiscordProvider::new` for ergonomic parity. Use `try_new` when
    /// you need fallible construction (for example, a CLI loading config at
    /// runtime).
    ///
    /// ## Panics
    ///
    /// Panics if the URL does not contain `/webhooks/{id}/{token}` segments.
    pub fn new(config: DiscordWebhookConfig) -> Self {
        Self::try_new(config).expect("DiscordWebhookProvider::new given a malformed webhook URL")
    }

    fn build_part(attachment: &Attachment) -> Result<(String, Part), MessengerError> {
        let (filename, bytes) = match &attachment.source {
            AttachmentSource::Path(path) => {
                let filename = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .ok_or_else(|| {
                        MessengerError::InvalidMessage(format!(
                            "Discord webhook attachment path has no valid filename: {}",
                            path.display()
                        ))
                    })?
                    .to_owned();
                let bytes = fs::read(path).map_err(|error| {
                    MessengerError::InvalidMessage(format!(
                        "Discord webhook attachment path is not readable ({}): {error}",
                        path.display()
                    ))
                })?;
                (filename, bytes)
            }
            AttachmentSource::Bytes { filename, data, .. } => (filename.clone(), data.to_vec()),
            AttachmentSource::Url(_) => {
                return Err(MessengerError::InvalidMessage(
                    "Discord webhook attachments must come from a local path or bytes payload"
                        .into(),
                ));
            }
            AttachmentSource::ProviderFileId(_) => {
                return Err(MessengerError::InvalidMessage(
                    "Discord webhook does not support provider file ID attachments".into(),
                ));
            }
        };

        let part = Part::bytes(bytes).file_name(filename.clone());
        Ok((filename, part))
    }
}

fn parse_webhook_url(url: &str) -> Result<ParsedWebhook, MessengerError> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err(MessengerError::InvalidMessage(
            "Discord webhook URL is empty".into(),
        ));
    }

    // Strip any query string / fragment before dissecting the path.
    let without_query = trimmed.split(['?', '#']).next().unwrap_or(trimmed);
    let base = without_query.trim_end_matches('/');

    let idx = base.find("/webhooks/").ok_or_else(|| {
        MessengerError::InvalidMessage(
            "Discord webhook URL must contain a /webhooks/{id}/{token} segment".into(),
        )
    })?;
    let tail = &base[idx + "/webhooks/".len()..];
    let mut segments = tail.splitn(3, '/');
    let webhook_id = segments.next().unwrap_or("");
    let token = segments.next().unwrap_or("");
    let extra = segments.next();

    if webhook_id.is_empty() || token.is_empty() || extra.is_some() {
        return Err(MessengerError::InvalidMessage(
            "Discord webhook URL must be of the form \
             https://discord.com/api/v{version}/webhooks/{id}/{token}"
                .into(),
        ));
    }

    if !webhook_id.bytes().all(|b| b.is_ascii_digit()) {
        return Err(MessengerError::InvalidMessage(format!(
            "Discord webhook URL has a non-numeric webhook id: {webhook_id}"
        )));
    }

    Ok(ParsedWebhook {
        webhook_id: webhook_id.to_owned(),
        token: token.to_owned(),
        base_url: base.to_owned(),
    })
}

/// Request body sent to Discord when there are no attachments (JSON path).
#[derive(Serialize)]
struct WebhookJsonBody<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    attachments: Option<Vec<AttachmentMeta<'a>>>,
}

/// Attachment metadata included in the JSON or `payload_json` field.
#[derive(Serialize)]
struct AttachmentMeta<'a> {
    id: u64,
    filename: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<&'a str>,
}

/// Subset of Discord's webhook-execute response we care about.
#[derive(Deserialize)]
struct WebhookMessageResponse {
    id: String,
    channel_id: String,
    #[serde(default)]
    webhook_id: Option<String>,
}

#[async_trait::async_trait]
impl super::Provider for DiscordWebhookProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::DiscordWebhook
    }

    fn capabilities(&self) -> CapabilitySet {
        const DISCORD_WEBHOOK_CAPABILITIES: CapabilitySet = CapabilitySet {
            supports_markdown_rendering: true,
            supports_reply: false,
            supports_attachments: true,
            supports_location: true,
            supports_silent_delivery: false,
            supports_link_preview_control: false,
        };
        DISCORD_WEBHOOK_CAPABILITIES
    }

    #[tracing::instrument(skip_all, fields(
        provider = "discord-webhook",
        webhook_id = %self.webhook_id,
        thread_id = tracing::field::Empty,
    ))]
    async fn send_prepared(
        &self,
        dispatch: &Dispatch,
        message: &PreparedMessage,
    ) -> Result<SendReceipt, MessengerError> {
        let thread_id = match &dispatch.target {
            Target::DiscordWebhook(target) => target.thread_id.clone(),
            _ => {
                return Err(MessengerError::InvalidMessage(
                    "expected DiscordWebhook target".into(),
                ));
            }
        };
        if let Some(thread) = thread_id.as_deref() {
            tracing::Span::current().record("thread_id", tracing::field::display(thread));
        }

        let content = message.render_body_with_location(ProviderKind::DiscordWebhook);
        let content_opt = if content.is_empty() {
            None
        } else {
            Some(content.as_str())
        };

        let attachments = message.attachments();
        tracing::debug!(
            content_len = content.len(),
            attachment_count = attachments.len(),
            has_thread = thread_id.is_some(),
            "sending Discord webhook message"
        );

        // Build the request URL with `wait=true` so we always receive the
        // created message payload (id/channel_id/webhook_id).
        let mut url = format!("{}?wait=true", self.base_url);
        if let Some(thread) = thread_id.as_deref() {
            url.push_str("&thread_id=");
            url.push_str(thread);
        }

        let response = if attachments.is_empty() {
            let body = WebhookJsonBody {
                content: content_opt,
                attachments: None,
            };
            self.client.post(&url).json(&body).send().await
        } else {
            let mut form = Form::new();
            let mut metas: Vec<AttachmentMeta<'_>> = Vec::with_capacity(attachments.len());
            let mut owned: Vec<(String, Option<String>)> = Vec::with_capacity(attachments.len());
            for (index, attachment) in attachments.iter().enumerate() {
                let (filename, part) = Self::build_part(attachment)?;
                let description = attachment
                    .alt_text
                    .clone()
                    .or_else(|| attachment.caption.clone());
                owned.push((filename, description));
                form = form.part(format!("files[{index}]"), part);
            }
            for (index, (filename, description)) in owned.iter().enumerate() {
                metas.push(AttachmentMeta {
                    id: index as u64,
                    filename: filename.as_str(),
                    description: description.as_deref(),
                });
            }
            let payload = WebhookJsonBody {
                content: content_opt,
                attachments: Some(metas),
            };
            let payload_json = serde_json::to_string(&payload)
                .map_err(|e| ProviderKind::DiscordWebhook.transport_error(e))?;
            form = form.text("payload_json", payload_json);

            self.client.post(&url).multipart(form).send().await
        };

        let response = response.map_err(|e| {
            tracing::warn!(error = %e, "Discord webhook request failed");
            ProviderKind::DiscordWebhook.transport_error(e)
        })?;

        let status = response.status();
        tracing::debug!(status = %status, "received Discord webhook response");

        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            let body = response.text().await.unwrap_or_default();
            tracing::warn!(status = %status, body = %body, "Discord webhook authentication failed");
            return Err(MessengerError::Authentication {
                provider: ProviderKind::DiscordWebhook,
                message: if body.is_empty() {
                    status.to_string()
                } else {
                    body
                },
            });
        }

        // 429 rate limits and 5xx transport issues are handled uniformly by
        // the shared helper; any remaining 4xx lands in the `Err` branch
        // below as a provider-level error.
        if status.is_client_error() && status != reqwest::StatusCode::TOO_MANY_REQUESTS {
            let body = response.text().await.unwrap_or_default();
            tracing::warn!(status = %status, body = %body, "Discord webhook provider error");
            return Err(MessengerError::Provider {
                provider: ProviderKind::DiscordWebhook,
                code: Some(status.as_u16().to_string()),
                message: if body.is_empty() {
                    status.to_string()
                } else {
                    body
                },
            });
        }

        let msg: WebhookMessageResponse =
            super::http_helpers::handle_http_response(response, ProviderKind::DiscordWebhook)
                .await?;
        tracing::debug!(raw_id = %msg.id, "Discord webhook message sent");

        let webhook_id = msg.webhook_id.unwrap_or_else(|| self.webhook_id.clone());

        Ok(SendReceipt {
            provider: ProviderKind::DiscordWebhook,
            message_ref: MessageRef::DiscordWebhook {
                webhook_id,
                channel_id: msg.channel_id,
                message_id: msg.id.clone(),
                thread_id,
            },
            raw_id: msg.id,
            metadata: BTreeMap::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_standard_webhook_url() {
        let parsed =
            parse_webhook_url("https://discord.com/api/v10/webhooks/1234567890/abc-token").unwrap();
        assert_eq!(parsed.webhook_id, "1234567890");
        assert_eq!(parsed.token, "abc-token");
        assert_eq!(
            parsed.base_url,
            "https://discord.com/api/v10/webhooks/1234567890/abc-token"
        );
    }

    #[test]
    fn parses_webhook_url_with_trailing_slash() {
        let parsed = parse_webhook_url("https://discord.com/api/v10/webhooks/1/tok/").unwrap();
        assert_eq!(parsed.webhook_id, "1");
        assert_eq!(parsed.token, "tok");
    }

    #[test]
    fn parses_webhook_url_with_query_string() {
        let parsed =
            parse_webhook_url("https://discord.com/api/v10/webhooks/1/tok?foo=bar").unwrap();
        assert_eq!(parsed.token, "tok");
        assert_eq!(
            parsed.base_url,
            "https://discord.com/api/v10/webhooks/1/tok"
        );
    }

    fn expect_parse_err(url: &str) -> MessengerError {
        match parse_webhook_url(url) {
            Ok(_) => panic!("expected webhook URL parsing to fail for {url:?}"),
            Err(err) => err,
        }
    }

    #[test]
    fn rejects_empty_url() {
        let err = expect_parse_err("");
        assert!(matches!(
            err,
            MessengerError::InvalidMessage(message) if message.contains("empty")
        ));
    }

    #[test]
    fn rejects_url_without_webhooks_segment() {
        let err = expect_parse_err("https://discord.com/api/v10/channels/1/messages");
        assert!(matches!(
            err,
            MessengerError::InvalidMessage(message) if message.contains("/webhooks/")
        ));
    }

    #[test]
    fn rejects_url_with_missing_token() {
        let err = expect_parse_err("https://discord.com/api/v10/webhooks/1234");
        assert!(matches!(
            err,
            MessengerError::InvalidMessage(message) if message.contains("form")
        ));
    }

    #[test]
    fn rejects_url_with_extra_path_segment() {
        let err = expect_parse_err("https://discord.com/api/v10/webhooks/1/tok/messages");
        assert!(matches!(
            err,
            MessengerError::InvalidMessage(message) if message.contains("form")
        ));
    }

    #[test]
    fn rejects_url_with_non_numeric_webhook_id() {
        let err = expect_parse_err("https://discord.com/api/v10/webhooks/abc/tok");
        assert!(matches!(
            err,
            MessengerError::InvalidMessage(message) if message.contains("non-numeric")
        ));
    }

    #[test]
    fn try_new_returns_error_for_malformed_url() {
        let result = DiscordWebhookProvider::try_new(DiscordWebhookConfig {
            webhook_url: SecretString::from("not a url"),
        });
        let err = match result {
            Ok(_) => panic!("expected malformed URL to fail parsing"),
            Err(err) => err,
        };
        assert!(matches!(err, MessengerError::InvalidMessage(_)));
    }

    #[test]
    #[should_panic(expected = "malformed webhook URL")]
    fn new_panics_on_malformed_url() {
        let _ = DiscordWebhookProvider::new(DiscordWebhookConfig {
            webhook_url: SecretString::from("http://example.com/".to_string()),
        });
    }

    #[tokio::test]
    async fn capabilities_disable_replies() {
        let provider = DiscordWebhookProvider::try_new(DiscordWebhookConfig {
            webhook_url: SecretString::from(
                "https://discord.com/api/v10/webhooks/123/token".to_string(),
            ),
        })
        .unwrap();

        let caps = super::super::Provider::capabilities(&provider);
        assert!(!caps.supports_reply);
        assert!(caps.supports_markdown_rendering);
        assert!(caps.supports_attachments);
        assert!(caps.supports_location);
        assert!(!caps.supports_silent_delivery);
        assert!(!caps.supports_link_preview_control);
    }

    #[test]
    fn kind_reports_discord_webhook() {
        let provider = DiscordWebhookProvider::try_new(DiscordWebhookConfig {
            webhook_url: SecretString::from(
                "https://discord.com/api/v10/webhooks/123/token".to_string(),
            ),
        })
        .unwrap();
        assert_eq!(
            super::super::Provider::kind(&provider),
            ProviderKind::DiscordWebhook
        );
    }
}
