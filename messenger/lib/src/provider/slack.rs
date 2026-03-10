use std::collections::BTreeMap;

use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};

use crate::capabilities::CapabilitySet;
use crate::dispatch::Dispatch;
use crate::error::MessengerError;
use crate::message::MessageBody;
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
        CapabilitySet {
            supports_markdown_rendering: true,
            supports_reply: true,
            supports_attachments: false, // Deferred: Slack file upload is complex
            supports_location: false,
            supports_silent_delivery: false,
            supports_link_preview_control: true,
        }
    }

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
                ))
            }
        };

        // Render the message body to Slack mrkdwn
        let text = match message.body() {
            Some(MessageBody::Plain(_)) | Some(MessageBody::Markdown(_)) => {
                message.render_body_for_provider(ProviderKind::Slack)
            }
            None => String::new(),
        };

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
            .map_err(|e| MessengerError::Transport {
                provider: ProviderKind::Slack,
                message: e.to_string(),
            })?;

        let status = response.status();

        if status.as_u16() == 429 {
            let retry_after = response
                .headers()
                .get("Retry-After")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
                .map(|s| s * 1000);
            return Err(MessengerError::RateLimited {
                provider: ProviderKind::Slack,
                retry_after_ms: retry_after,
            });
        }

        if status.is_server_error() {
            return Err(MessengerError::Transport {
                provider: ProviderKind::Slack,
                message: format!("server error: {status}"),
            });
        }

        let resp: ChatPostMessageResponse =
            response
                .json()
                .await
                .map_err(|e| MessengerError::Transport {
                    provider: ProviderKind::Slack,
                    message: e.to_string(),
                })?;

        if !resp.ok {
            let error = resp.error.unwrap_or_else(|| "unknown error".into());
            if error == "invalid_auth" || error == "not_authed" || error == "token_revoked" {
                return Err(MessengerError::Authentication {
                    provider: ProviderKind::Slack,
                    message: error,
                });
            }
            return Err(MessengerError::Provider {
                provider: ProviderKind::Slack,
                code: Some(error.clone()),
                message: error,
            });
        }

        let channel = resp.channel.unwrap_or_default();
        let ts = resp.ts.unwrap_or_default();

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
