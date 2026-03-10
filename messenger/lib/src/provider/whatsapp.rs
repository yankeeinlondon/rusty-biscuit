use std::collections::BTreeMap;

use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};

use crate::capabilities::CapabilitySet;
use crate::dispatch::Dispatch;
use crate::error::MessengerError;
use crate::message::{Message, MessageBody};
use crate::receipt::{MessageRef, ProviderKind, SendReceipt};
use crate::target::Target;

/// Configuration for the WhatsApp provider.
pub struct WhatsAppConfig {
    pub access_token: SecretString,
    pub phone_number_id: String,
    /// Cloud API version (default: `v23.0`).
    pub api_version: Option<String>,
    /// Override the base URL (useful for testing with wiremock).
    pub api_base_url: Option<String>,
}

/// WhatsApp provider adapter using the Cloud API via reqwest.
pub struct WhatsAppProvider {
    access_token: SecretString,
    #[allow(dead_code)]
    phone_number_id: String,
    client: reqwest::Client,
    api_base_url: String,
}

impl WhatsAppProvider {
    pub fn new(config: WhatsAppConfig) -> Self {
        let version = config.api_version.unwrap_or_else(|| "v23.0".to_string());
        let api_base_url = config.api_base_url.unwrap_or_else(|| {
            format!(
                "https://graph.facebook.com/{}/{}",
                version, config.phone_number_id
            )
        });
        Self {
            access_token: config.access_token,
            phone_number_id: config.phone_number_id,
            client: reqwest::Client::new(),
            api_base_url,
        }
    }
}

#[derive(Serialize)]
struct WhatsAppMessageRequest<'a> {
    messaging_product: &'static str,
    to: &'a str,
    #[serde(rename = "type")]
    message_type: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<TextPayload>,
    #[serde(skip_serializing_if = "Option::is_none")]
    location: Option<LocationPayload>,
    #[serde(skip_serializing_if = "Option::is_none")]
    context: Option<ContextPayload<'a>>,
}

#[derive(Serialize)]
struct TextPayload {
    body: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    preview_url: Option<bool>,
}

#[derive(Serialize)]
struct LocationPayload {
    latitude: f64,
    longitude: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    address: Option<String>,
}

#[derive(Serialize)]
struct ContextPayload<'a> {
    message_id: &'a str,
}

#[derive(Deserialize)]
struct WhatsAppResponse {
    #[serde(default)]
    messages: Option<Vec<WhatsAppMessageId>>,
    #[serde(default)]
    error: Option<WhatsAppError>,
}

#[derive(Deserialize)]
struct WhatsAppMessageId {
    id: String,
}

#[derive(Deserialize)]
struct WhatsAppError {
    #[serde(default)]
    code: Option<i64>,
    #[serde(default)]
    message: Option<String>,
}

#[async_trait::async_trait]
impl super::Provider for WhatsAppProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::WhatsApp
    }

    fn capabilities(&self) -> CapabilitySet {
        CapabilitySet {
            supports_markdown_rendering: false,
            supports_reply: true,
            supports_attachments: true,
            supports_location: true,
            supports_silent_delivery: false,
            supports_link_preview_control: false,
        }
    }

    async fn send(
        &self,
        dispatch: &Dispatch,
        message: &Message,
    ) -> Result<SendReceipt, MessengerError> {
        let recipient = match &dispatch.target {
            Target::WhatsApp(t) => &t.recipient,
            _ => {
                return Err(MessengerError::InvalidMessage(
                    "expected WhatsApp target".into(),
                ))
            }
        };

        let context = match &dispatch.reply_to {
            Some(MessageRef::WhatsApp { message_id }) => Some(ContextPayload {
                message_id: message_id.as_str(),
            }),
            _ => None,
        };

        // Choose which API method to use based on content
        let body = if let Some(location) = &message.location {
            WhatsAppMessageRequest {
                messaging_product: "whatsapp",
                to: recipient,
                message_type: "location",
                text: None,
                location: Some(LocationPayload {
                    latitude: location.latitude,
                    longitude: location.longitude,
                    name: location.name.clone(),
                    address: location.address.clone(),
                }),
                context,
            }
        } else {
            // Render body as plain text (WhatsApp doesn't support rich formatting)
            let text = match &message.body {
                Some(MessageBody::Plain(text)) => text.clone(),
                Some(MessageBody::Markdown(md)) => {
                    crate::markdown::render_for_provider(md, ProviderKind::WhatsApp)
                }
                None => String::new(),
            };

            WhatsAppMessageRequest {
                messaging_product: "whatsapp",
                to: recipient,
                message_type: "text",
                text: Some(TextPayload {
                    body: text,
                    preview_url: None,
                }),
                location: None,
                context,
            }
        };

        let url = format!("{}/messages", self.api_base_url);

        let response = self
            .client
            .post(&url)
            .header(
                "Authorization",
                format!("Bearer {}", self.access_token.expose_secret()),
            )
            .json(&body)
            .send()
            .await
            .map_err(|e| MessengerError::Transport {
                provider: ProviderKind::WhatsApp,
                message: e.to_string(),
            })?;

        let status = response.status();

        if status.as_u16() == 429 {
            return Err(MessengerError::RateLimited {
                provider: ProviderKind::WhatsApp,
                retry_after_ms: None,
            });
        }

        if status.is_server_error() {
            return Err(MessengerError::Transport {
                provider: ProviderKind::WhatsApp,
                message: format!("server error: {status}"),
            });
        }

        let resp: WhatsAppResponse =
            response
                .json()
                .await
                .map_err(|e| MessengerError::Transport {
                    provider: ProviderKind::WhatsApp,
                    message: e.to_string(),
                })?;

        if let Some(error) = resp.error {
            let code = error.code.unwrap_or(0);
            let msg = error
                .message
                .unwrap_or_else(|| "unknown error".to_string());

            if code == 190 {
                return Err(MessengerError::Authentication {
                    provider: ProviderKind::WhatsApp,
                    message: msg,
                });
            }

            return Err(MessengerError::Provider {
                provider: ProviderKind::WhatsApp,
                code: Some(code.to_string()),
                message: msg,
            });
        }

        let message_id = resp
            .messages
            .and_then(|mut m| m.pop().map(|msg| msg.id))
            .unwrap_or_default();

        Ok(SendReceipt {
            provider: ProviderKind::WhatsApp,
            message_ref: MessageRef::WhatsApp {
                message_id: message_id.clone(),
            },
            raw_id: message_id,
            metadata: BTreeMap::new(),
        })
    }
}
