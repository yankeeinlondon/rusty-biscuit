use std::collections::BTreeMap;

use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};

use crate::capabilities::CapabilitySet;
use crate::dispatch::Dispatch;
use crate::error::MessengerError;
use crate::message::MessageBody;
use crate::prepared::PreparedMessage;
use crate::receipt::{MessageRef, ProviderKind, SendReceipt, TelegramChatRef};
use crate::target::{Target, TelegramChatId};

/// Configuration for the Telegram provider.
pub struct TelegramConfig {
    pub bot_token: SecretString,
    /// Override the base URL (useful for testing with wiremock).
    pub api_base_url: Option<String>,
}

/// Telegram provider adapter using the Bot API directly via reqwest.
pub struct TelegramProvider {
    client: reqwest::Client,
    api_base_url: String,
}

impl TelegramProvider {
    pub fn new(config: TelegramConfig) -> Self {
        let api_base_url = config.api_base_url.unwrap_or_else(|| {
            format!(
                "https://api.telegram.org/bot{}",
                config.bot_token.expose_secret()
            )
        });
        Self {
            client: reqwest::Client::new(),
            api_base_url,
        }
    }

    fn chat_id_to_string(chat_id: &TelegramChatId) -> String {
        match chat_id {
            TelegramChatId::Id(id) => id.to_string(),
            TelegramChatId::Username(u) => {
                if u.starts_with('@') {
                    u.clone()
                } else {
                    format!("@{u}")
                }
            }
        }
    }

    fn chat_id_to_ref(chat_id: &TelegramChatId) -> TelegramChatRef {
        match chat_id {
            TelegramChatId::Id(id) => TelegramChatRef::Id(*id),
            TelegramChatId::Username(u) => TelegramChatRef::Username(u.clone()),
        }
    }
}

#[derive(Serialize)]
struct SendMessageRequest<'a> {
    chat_id: &'a str,
    text: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    parse_mode: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reply_parameters: Option<ReplyParameters>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message_thread_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    disable_notification: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    disable_web_page_preview: Option<bool>,
}

#[derive(Serialize)]
struct ReplyParameters {
    message_id: i64,
}

#[derive(Serialize)]
struct SendLocationRequest<'a> {
    chat_id: &'a str,
    latitude: f64,
    longitude: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    reply_parameters: Option<ReplyParameters>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message_thread_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    disable_notification: Option<bool>,
}

#[derive(Deserialize)]
struct TelegramResponse {
    ok: bool,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    result: Option<TelegramMessage>,
    #[serde(default)]
    parameters: Option<TelegramErrorParameters>,
}

#[derive(Deserialize)]
struct TelegramMessage {
    message_id: i64,
}

#[derive(Deserialize)]
struct TelegramErrorParameters {
    #[serde(default)]
    retry_after: Option<u64>,
}

fn handle_error(resp: &TelegramResponse) -> MessengerError {
    let description = resp
        .description
        .clone()
        .unwrap_or_else(|| "unknown error".into());

    if let Some(params) = &resp.parameters
        && let Some(retry_after) = params.retry_after
    {
        return MessengerError::RateLimited {
            provider: ProviderKind::Telegram,
            retry_after_ms: Some(retry_after * 1000),
        };
    }

    if description.contains("Unauthorized") || description.contains("bot token") {
        return MessengerError::Authentication {
            provider: ProviderKind::Telegram,
            message: description,
        };
    }

    MessengerError::Provider {
        provider: ProviderKind::Telegram,
        code: None,
        message: description,
    }
}

#[async_trait::async_trait]
impl super::Provider for TelegramProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Telegram
    }

    fn capabilities(&self) -> CapabilitySet {
        CapabilitySet {
            supports_markdown_rendering: true,
            supports_reply: true,
            supported_attachment_kinds: std::collections::BTreeSet::new(),
            supports_location: true,
            supports_silent_delivery: true,
            supports_link_preview_control: true,
        }
    }

    #[tracing::instrument(skip_all, fields(provider = "telegram", chat = tracing::field::Empty))]
    async fn send_prepared(
        &self,
        dispatch: &Dispatch,
        message: &PreparedMessage,
    ) -> Result<SendReceipt, MessengerError> {
        let (chat_id_target, thread_id) = match &dispatch.target {
            Target::Telegram(t) => (&t.chat_id, t.thread_id),
            _ => {
                return Err(MessengerError::InvalidMessage(
                    "expected Telegram target".into(),
                ));
            }
        };

        let chat_id_str = Self::chat_id_to_string(chat_id_target);
        let chat_ref = Self::chat_id_to_ref(chat_id_target);
        tracing::Span::current().record("chat", tracing::field::display(&chat_id_str));

        // Build reply parameters
        let reply_params = match &dispatch.reply_to {
            Some(MessageRef::Telegram { message_id, .. }) => Some(ReplyParameters {
                message_id: *message_id,
            }),
            _ => None,
        };

        let disable_notification = if dispatch.options.silent {
            Some(true)
        } else {
            None
        };

        // Send location if present
        if let Some(location) = message.location() {
            let body = SendLocationRequest {
                chat_id: &chat_id_str,
                latitude: location.latitude,
                longitude: location.longitude,
                reply_parameters: reply_params.as_ref().map(|r| ReplyParameters {
                    message_id: r.message_id,
                }),
                message_thread_id: thread_id,
                disable_notification,
            };

            tracing::debug!(
                has_reply = reply_params.is_some(),
                thread_id = ?thread_id,
                silent = dispatch.options.silent,
                "sending Telegram location"
            );
            let url = format!("{}/sendLocation", self.api_base_url);
            let resp = self.post_request("sendLocation", &url, &body).await?;
            tracing::debug!(raw_id = %resp.message_id, "Telegram location sent");

            return Ok(SendReceipt {
                provider: ProviderKind::Telegram,
                message_ref: MessageRef::Telegram {
                    chat_id: chat_ref,
                    message_id: resp.message_id,
                    thread_id,
                },
                raw_id: resp.message_id.to_string(),
                metadata: BTreeMap::new(),
            });
        }

        // Render message body
        let (text, parse_mode) = match message.body() {
            Some(MessageBody::Markdown(_)) => (
                message.render_body_for_provider(ProviderKind::Telegram),
                Some("HTML"),
            ),
            Some(MessageBody::Plain(_)) => (
                message.render_body_for_provider(ProviderKind::Telegram),
                None,
            ),
            None => (String::new(), None),
        };
        tracing::debug!(
            has_reply = reply_params.is_some(),
            thread_id = ?thread_id,
            silent = dispatch.options.silent,
            disable_link_preview = dispatch.options.disable_link_preview,
            text_len = text.len(),
            "sending Telegram message"
        );

        let body = SendMessageRequest {
            chat_id: &chat_id_str,
            text: &text,
            parse_mode,
            reply_parameters: reply_params,
            message_thread_id: thread_id,
            disable_notification,
            disable_web_page_preview: if dispatch.options.disable_link_preview {
                Some(true)
            } else {
                None
            },
        };

        let url = format!("{}/sendMessage", self.api_base_url);
        let result = self.post_request("sendMessage", &url, &body).await?;
        tracing::debug!(raw_id = %result.message_id, "Telegram message sent");

        Ok(SendReceipt {
            provider: ProviderKind::Telegram,
            message_ref: MessageRef::Telegram {
                chat_id: chat_ref,
                message_id: result.message_id,
                thread_id,
            },
            raw_id: result.message_id.to_string(),
            metadata: BTreeMap::new(),
        })
    }
}

impl TelegramProvider {
    async fn post_request<T: Serialize>(
        &self,
        request_kind: &'static str,
        url: &str,
        body: &T,
    ) -> Result<TelegramMessage, MessengerError> {
        tracing::debug!(request_kind, "sending Telegram API request");
        let response = self.client.post(url).json(body).send().await.map_err(|e| {
            tracing::warn!(request_kind, error = %e, "Telegram transport request failed");
            ProviderKind::Telegram.transport_error(e)
        })?;
        tracing::debug!(request_kind, status = %response.status(), "received Telegram response");

        let resp: TelegramResponse =
            super::http_helpers::handle_http_response(response, ProviderKind::Telegram).await?;

        if !resp.ok {
            let error = handle_error(&resp);
            match &error {
                MessengerError::RateLimited { retry_after_ms, .. } => {
                    tracing::warn!(
                        request_kind,
                        retry_after_ms = ?retry_after_ms,
                        "Telegram rate limited request"
                    );
                }
                MessengerError::Authentication { message, .. } => {
                    tracing::warn!(request_kind, error = %message, "Telegram authentication failed");
                }
                MessengerError::Provider { code, message, .. } => {
                    tracing::warn!(
                        request_kind,
                        code = ?code,
                        error = %message,
                        "Telegram provider returned error"
                    );
                }
                _ => {}
            }
            return Err(error);
        }

        resp.result.ok_or_else(|| {
            ProviderKind::Telegram.transport_error("missing result in Telegram response")
        })
    }
}
