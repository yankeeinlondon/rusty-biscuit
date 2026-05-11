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

/// Configuration for the Firebase Cloud Messaging provider.
pub struct FcmConfig {
    /// The Firebase project ID (e.g. `my-project-12345`).
    pub project_id: String,
    /// An OAuth2 access token with `https://www.googleapis.com/auth/firebase.messaging` scope.
    pub access_token: SecretString,
    /// Override the base URL (useful for testing with wiremock).
    pub api_base_url: Option<String>,
}

/// Firebase Cloud Messaging provider adapter.
///
/// Sends push notifications to Android devices via the FCM HTTP v1 API.
pub struct FcmProvider {
    client: reqwest::Client,
    config: FcmConfig,
}

impl FcmProvider {
    pub fn new(config: FcmConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            config,
        }
    }

    fn base_url(&self) -> String {
        self.config
            .api_base_url
            .clone()
            .unwrap_or_else(|| "https://fcm.googleapis.com".into())
    }
}

#[derive(Serialize)]
struct FcmSendRequest<'a> {
    message: FcmMessage<'a>,
}

#[derive(Serialize)]
struct FcmMessage<'a> {
    token: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    notification: Option<FcmNotification<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    android: Option<FcmAndroidConfig>,
}

#[derive(Serialize)]
struct FcmNotification<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<&'a str>,
}

#[derive(Serialize)]
struct FcmAndroidConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    priority: Option<String>,
}

#[derive(Deserialize)]
struct FcmSendResponse {
    #[serde(default)]
    name: String,
}

#[derive(Deserialize)]
struct FcmErrorResponse {
    error: FcmErrorDetail,
}

#[derive(Deserialize)]
struct FcmErrorDetail {
    code: i64,
    message: String,
    status: String,
}

#[async_trait::async_trait]
impl super::Provider for FcmProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Fcm
    }

    fn capabilities(&self) -> CapabilitySet {
        CapabilitySet {
            supports_markdown_rendering: false,
            supports_reply: false,
            supported_attachment_kinds: std::collections::BTreeSet::new(),
            supports_location: false,
            supports_silent_delivery: true,
            supports_link_preview_control: false,
        }
    }

    #[tracing::instrument(skip_all, fields(provider = "fcm", device = tracing::field::Empty))]
    async fn send_prepared(
        &self,
        dispatch: &Dispatch,
        message: &PreparedMessage,
    ) -> Result<SendReceipt, MessengerError> {
        let device_token = match &dispatch.target {
            Target::Fcm(t) => &t.device_token,
            _ => {
                return Err(MessengerError::InvalidMessage("expected FCM target".into()));
            }
        };
        tracing::Span::current().record("device", tracing::field::display(device_token));

        let title = message.title();
        let body = match message.body() {
            Some(MessageBody::Plain(text)) => Some(text.as_str()),
            Some(MessageBody::Markdown(text)) => Some(text.as_str()),
            Some(MessageBody::Summarized { summary, .. }) => Some(summary.as_str()),
            None => None,
        };

        let notification = if title.is_some() || body.is_some() {
            Some(FcmNotification { title, body })
        } else {
            None
        };

        let android = if dispatch.options.silent {
            Some(FcmAndroidConfig {
                priority: Some("normal".into()),
            })
        } else {
            Some(FcmAndroidConfig {
                priority: Some("high".into()),
            })
        };

        let payload = FcmSendRequest {
            message: FcmMessage {
                token: device_token,
                notification,
                android,
            },
        };

        let url = format!(
            "{}/v1/projects/{}/messages:send",
            self.base_url(),
            self.config.project_id
        );
        tracing::debug!(url = %url, "sending FCM request");

        let response = self
            .client
            .post(&url)
            .json(&payload)
            .header(
                "authorization",
                format!("Bearer {}", self.config.access_token.expose_secret()),
            )
            .header("content-type", "application/json")
            .send()
            .await
            .map_err(|e| {
                tracing::warn!(error = %e, "FCM transport request failed");
                ProviderKind::Fcm.transport_error(e)
            })?;

        let status = response.status();
        tracing::debug!(status = %status, "received FCM response");

        if status.is_success() {
            let resp: FcmSendResponse = response.json().await.map_err(|e| {
                ProviderKind::Fcm.transport_error(format!("invalid JSON response: {e}"))
            })?;

            tracing::info!(message_id = %resp.name, "FCM send complete");
            return Ok(SendReceipt {
                provider: ProviderKind::Fcm,
                message_ref: MessageRef::Fcm {
                    message_id: resp.name.clone(),
                    project_id: self.config.project_id.clone(),
                },
                raw_id: resp.name,
                metadata: BTreeMap::new(),
            });
        }

        let error_body: FcmErrorResponse = response.json().await.unwrap_or(FcmErrorResponse {
            error: FcmErrorDetail {
                code: status.as_u16() as i64,
                message: format!("HTTP {status}"),
                status: status.to_string(),
            },
        });

        tracing::warn!(
            code = error_body.error.code,
            message = %error_body.error.message,
            "FCM returned error"
        );

        if error_body.error.message.contains("invalid registration")
            || error_body
                .error
                .message
                .contains("registration-token-not-registered")
        {
            return Err(MessengerError::InvalidMessage(format!(
                "FCM device token error: {}",
                error_body.error.message
            )));
        }

        if status.as_u16() == 401 || status.as_u16() == 403 {
            return Err(MessengerError::Authentication {
                provider: ProviderKind::Fcm,
                message: error_body.error.message,
            });
        }

        Err(MessengerError::Provider {
            provider: ProviderKind::Fcm,
            code: Some(error_body.error.status.clone()),
            message: error_body.error.message,
        })
    }
}
