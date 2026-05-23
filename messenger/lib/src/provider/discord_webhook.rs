use std::collections::BTreeMap;

use reqwest::multipart::{Form, Part};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};

use crate::attachment::Attachment;
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
    /// The webhook endpoint prefix up to `/webhooks/{id}`. The token is stored
    /// separately so routine field logging cannot accidentally expose it.
    webhook_url_prefix: String,
    webhook_token: SecretString,
}

/// Fields extracted from a Discord webhook URL.
struct ParsedWebhook {
    webhook_id: String,
    webhook_token: SecretString,
    webhook_url_prefix: String,
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
            webhook_url_prefix: parsed.webhook_url_prefix,
            webhook_token: parsed.webhook_token,
        })
    }

    fn parse_thread_id(thread_id: &str) -> Result<u64, MessengerError> {
        thread_id.parse::<u64>().map_err(|_| {
            MessengerError::InvalidMessage(format!(
                "invalid Discord webhook thread ID: {thread_id}"
            ))
        })
    }

    fn build_request_url(&self, thread_id: Option<&str>) -> Result<reqwest::Url, MessengerError> {
        let mut url = reqwest::Url::parse(&self.webhook_url_prefix).map_err(|error| {
            MessengerError::InvalidMessage(format!(
                "Discord webhook URL became invalid after parsing: {error}"
            ))
        })?;

        {
            let mut path_segments = url.path_segments_mut().map_err(|_| {
                MessengerError::InvalidMessage(
                    "Discord webhook URL cannot be used as a base URL".into(),
                )
            })?;
            path_segments.pop_if_empty();
            path_segments.push(self.webhook_token.expose_secret());
        }

        {
            let mut query_pairs = url.query_pairs_mut();
            query_pairs.append_pair("wait", "true");
            if let Some(thread_id) = thread_id {
                let thread_id = Self::parse_thread_id(thread_id)?.to_string();
                query_pairs.append_pair("thread_id", &thread_id);
            }
        }

        Ok(url)
    }

    fn build_part(attachment: &Attachment) -> Result<(String, Part), MessengerError> {
        let (filename, bytes) = super::attachment_helpers::read_local_or_bytes_attachment(
            &attachment.source,
            "Discord webhook",
        )?;

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

    let mut parsed_url = reqwest::Url::parse(trimmed).map_err(|error| {
        MessengerError::InvalidMessage(format!("Discord webhook URL is invalid: {error}"))
    })?;
    parsed_url.set_query(None);
    parsed_url.set_fragment(None);

    let normalized_path = parsed_url.path().trim_end_matches('/').to_owned();

    let idx = normalized_path.find("/webhooks/").ok_or_else(|| {
        MessengerError::InvalidMessage(
            "Discord webhook URL must contain a /webhooks/{id}/{token} segment".into(),
        )
    })?;
    let tail = &normalized_path[idx + "/webhooks/".len()..];
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

    let mut webhook_url_prefix = parsed_url;
    {
        let mut path_segments = webhook_url_prefix.path_segments_mut().map_err(|_| {
            MessengerError::InvalidMessage(
                "Discord webhook URL cannot be used as a base URL".into(),
            )
        })?;
        path_segments.pop_if_empty();
        path_segments.pop();
    }

    Ok(ParsedWebhook {
        webhook_id: webhook_id.to_owned(),
        webhook_token: SecretString::from(token.to_owned()),
        webhook_url_prefix: webhook_url_prefix.as_str().trim_end_matches('/').to_owned(),
    })
}

/// Request body sent to Discord when there are no attachments (JSON path).
#[derive(Serialize)]
struct WebhookJsonBody<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    attachments: Option<Vec<AttachmentMeta>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    embeds: Option<Vec<EmbedBody>>,
}

#[derive(Serialize, Clone)]
struct EmbedBody {
    description: String,
}

/// Attachment metadata included in the JSON or `payload_json` field.
#[derive(Serialize)]
struct AttachmentMeta {
    id: u64,
    filename: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
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
        CapabilitySet {
            supports_markdown_rendering: true,
            supports_reply: false,
            supported_attachment_kinds: std::collections::BTreeSet::from([
                crate::AttachmentKind::Image,
                crate::AttachmentKind::Audio,
                crate::AttachmentKind::Video,
                crate::AttachmentKind::Document,
                crate::AttachmentKind::Binary,
            ]),
            supports_location: true,
            supports_silent_delivery: false,
            supports_link_preview_control: false,
        }
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

        let summary = message.render_summary();
        let rich = message.render_rich(ProviderKind::DiscordWebhook);
        let location_line = message.location().map(|l| l.format_text_line());

        let (content_owned, embeds_owned): (Option<String>, Option<Vec<EmbedBody>>) = match rich {
            Some(mut rich_md) => {
                if let Some(loc) = location_line {
                    if !rich_md.is_empty() {
                        rich_md.push('\n');
                    }
                    rich_md.push_str(&loc);
                }
                let content = if summary.is_empty() {
                    None
                } else {
                    Some(summary)
                };
                (
                    content,
                    Some(vec![EmbedBody {
                        description: rich_md,
                    }]),
                )
            }
            None => {
                let content = message.render_body_with_location(ProviderKind::DiscordWebhook);
                let content = if content.is_empty() {
                    None
                } else {
                    Some(content)
                };
                (content, None)
            }
        };
        let content_opt = content_owned.as_deref();

        let attachments = message.attachments();
        tracing::debug!(
            content_len = content_owned.as_ref().map(String::len).unwrap_or(0),
            has_embed = embeds_owned.is_some(),
            attachment_count = attachments.len(),
            has_thread = thread_id.is_some(),
            "sending Discord webhook message"
        );

        // `wait=true` ensures Discord returns the created message payload
        // instead of an empty 204.
        let url = self.build_request_url(thread_id.as_deref())?;

        let response = if attachments.is_empty() {
            let body = WebhookJsonBody {
                content: content_opt,
                attachments: None,
                embeds: embeds_owned.clone(),
            };
            self.client.post(url).json(&body).send().await
        } else {
            let mut form = Form::new();
            let mut metas: Vec<AttachmentMeta> = Vec::with_capacity(attachments.len());
            for (index, attachment) in attachments.iter().enumerate() {
                let (filename, part) = Self::build_part(attachment)?;
                let description = attachment
                    .alt_text
                    .clone()
                    .or_else(|| attachment.caption.clone());
                metas.push(AttachmentMeta {
                    id: index as u64,
                    filename,
                    description,
                });
                form = form.part(format!("files[{index}]"), part);
            }
            let payload = WebhookJsonBody {
                content: content_opt,
                attachments: Some(metas),
                embeds: embeds_owned.clone(),
            };
            let payload_json = serde_json::to_string(&payload)
                .map_err(|e| ProviderKind::DiscordWebhook.transport_error(e))?;
            form = form.text("payload_json", payload_json);

            self.client.post(url).multipart(form).send().await
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
        assert_eq!(parsed.webhook_token.expose_secret(), "abc-token");
        assert_eq!(
            parsed.webhook_url_prefix,
            "https://discord.com/api/v10/webhooks/1234567890"
        );
    }

    #[test]
    fn parses_webhook_url_with_trailing_slash() {
        let parsed = parse_webhook_url("https://discord.com/api/v10/webhooks/1/tok/").unwrap();
        assert_eq!(parsed.webhook_id, "1");
        assert_eq!(parsed.webhook_token.expose_secret(), "tok");
    }

    #[test]
    fn parses_webhook_url_with_query_string() {
        let parsed =
            parse_webhook_url("https://discord.com/api/v10/webhooks/1/tok?foo=bar").unwrap();
        assert_eq!(parsed.webhook_token.expose_secret(), "tok");
        assert_eq!(
            parsed.webhook_url_prefix,
            "https://discord.com/api/v10/webhooks/1"
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
        assert!(
            caps.supported_attachment_kinds
                .contains(&crate::AttachmentKind::Image)
        );
        assert!(
            caps.supported_attachment_kinds
                .contains(&crate::AttachmentKind::Document)
        );
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

    #[test]
    fn rejects_invalid_thread_id() {
        let err = DiscordWebhookProvider::parse_thread_id("abc123").unwrap_err();
        assert!(matches!(
            err,
            MessengerError::InvalidMessage(message)
            if message.contains("invalid Discord webhook thread ID")
        ));
    }

    #[test]
    fn build_request_url_adds_wait_and_thread_query_parameters() {
        let provider = DiscordWebhookProvider::try_new(DiscordWebhookConfig {
            webhook_url: SecretString::from(
                "https://discord.com/api/v10/webhooks/123/token".to_string(),
            ),
        })
        .unwrap();

        let url = provider.build_request_url(Some("456")).unwrap();

        assert_eq!(
            url.as_str(),
            "https://discord.com/api/v10/webhooks/123/token?wait=true&thread_id=456"
        );
    }

    #[test]
    fn build_request_url_rejects_invalid_thread_id_before_request() {
        let provider = DiscordWebhookProvider::try_new(DiscordWebhookConfig {
            webhook_url: SecretString::from(
                "https://discord.com/api/v10/webhooks/123/token".to_string(),
            ),
        })
        .unwrap();

        let err = provider
            .build_request_url(Some("123&wait=false"))
            .unwrap_err();

        assert!(matches!(
            err,
            MessengerError::InvalidMessage(message)
            if message.contains("invalid Discord webhook thread ID")
        ));
    }
}
