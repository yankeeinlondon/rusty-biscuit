use std::collections::BTreeMap;

use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};

use crate::capabilities::CapabilitySet;
use crate::dispatch::Dispatch;
use crate::error::MessengerError;
use crate::prepared::PreparedMessage;
use crate::receipt::{MessageRef, ProviderKind, SendReceipt};
use crate::target::Target;

/// Configuration for the Slack provider.
pub struct SlackConfig {
    pub bot_token: SecretString,
    /// Override the base URL (useful for testing with wiremock).
    pub api_base_url: Option<String>,
}

/// Slack provider adapter using the Slack Web API directly via reqwest.
pub struct SlackProvider {
    bot_token: SecretString,
    client: reqwest::Client,
    api_base_url: String,
}

impl SlackProvider {
    pub fn new(config: SlackConfig) -> Self {
        let api_base_url = config
            .api_base_url
            .unwrap_or_else(|| "https://slack.com/api".to_string());
        Self {
            bot_token: config.bot_token,
            client: reqwest::Client::new(),
            api_base_url,
        }
    }
}

#[derive(Serialize)]
struct ChatPostMessageRequest<'a> {
    channel: &'a str,
    text: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    thread_ts: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    unfurl_links: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    unfurl_media: Option<bool>,
}

#[derive(Deserialize)]
struct ChatPostMessageResponse {
    ok: bool,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    channel: Option<String>,
    #[serde(default)]
    ts: Option<String>,
}

#[async_trait::async_trait]
impl super::Provider for SlackProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Slack
    }

    fn capabilities(&self) -> CapabilitySet {
        const SLACK_CAPABILITIES: CapabilitySet = CapabilitySet {
            supports_markdown_rendering: true,
            supports_reply: true,
            supports_attachments: false,
            supports_location: true,
            supports_silent_delivery: false,
            supports_link_preview_control: true,
        };
        SLACK_CAPABILITIES
    }

    #[tracing::instrument(skip_all, fields(provider = "slack", channel = tracing::field::Empty))]
    async fn send_prepared(
        &self,
        dispatch: &Dispatch,
        message: &PreparedMessage,
    ) -> Result<SendReceipt, MessengerError> {
        let channel_id = match &dispatch.target {
            Target::Slack(t) => &t.channel_id,
            _ => {
                return Err(MessengerError::InvalidMessage(
                    "expected Slack target".into(),
                ));
            }
        };
        tracing::Span::current().record("channel", tracing::field::display(channel_id));

        // Render the message body to Slack mrkdwn (with location text fallback)
        let text = message.render_body_with_location(ProviderKind::Slack);

        // Build the thread_ts from reply_to
        let thread_ts = match &dispatch.reply_to {
            Some(MessageRef::Slack { thread_ts, .. }) => Some(thread_ts.as_str()),
            _ => None,
        };

        // Link preview control
        let (unfurl_links, unfurl_media) = if dispatch.options.disable_link_preview {
            (Some(false), Some(false))
        } else {
            (None, None)
        };
        tracing::debug!(
            has_reply = thread_ts.is_some(),
            disable_link_preview = dispatch.options.disable_link_preview,
            text_len = text.len(),
            "sending Slack message"
        );

        let body = ChatPostMessageRequest {
            channel: channel_id,
            text: &text,
            thread_ts,
            unfurl_links,
            unfurl_media,
        };

        let url = format!("{}/chat.postMessage", self.api_base_url);

        let response = self
            .client
            .post(&url)
            .header(
                "Authorization",
                format!("Bearer {}", self.bot_token.expose_secret()),
            )
            .json(&body)
            .send()
            .await
            .map_err(|e| ProviderKind::Slack.transport_error(e))?;

        tracing::debug!(status = %response.status(), "received Slack response");

        let resp: ChatPostMessageResponse =
            super::http_helpers::handle_http_response(response, ProviderKind::Slack).await?;

        if !resp.ok {
            let error = resp.error.unwrap_or_else(|| "unknown error".into());
            if error == "invalid_auth" || error == "not_authed" || error == "token_revoked" {
                tracing::warn!(code = %error, "Slack authentication failed");
                return Err(MessengerError::Authentication {
                    provider: ProviderKind::Slack,
                    message: error,
                });
            }
            tracing::warn!(code = %error, "Slack provider returned error");
            return Err(MessengerError::Provider {
                provider: ProviderKind::Slack,
                code: Some(error.clone()),
                message: error,
            });
        }

        let channel = resp.channel.unwrap_or_default();
        let ts = resp.ts.unwrap_or_default();
        tracing::debug!(raw_id = %ts, "Slack message sent");

        Ok(SendReceipt {
            provider: ProviderKind::Slack,
            message_ref: MessageRef::Slack {
                channel_id: channel.clone(),
                thread_ts: ts.clone(),
            },
            raw_id: ts,
            metadata: BTreeMap::new(),
        })
    }
}
