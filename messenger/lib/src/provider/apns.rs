use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

use jsonwebtoken::{Algorithm, EncodingKey, Header};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};

use crate::capabilities::CapabilitySet;
use crate::dispatch::Dispatch;
use crate::error::MessengerError;
use crate::message::MessageBody;
use crate::prepared::PreparedMessage;
use crate::receipt::{MessageRef, ProviderKind, SendReceipt};
use crate::target::Target;

/// Configuration for the Apple Push Notification service provider.
pub struct ApnsConfig {
    /// Apple Developer Team ID (10 characters).
    pub team_id: String,
    /// APNS Auth Key ID (10 characters).
    pub key_id: String,
    /// The raw PKCS#8 private key contents from the `.p8` file.
    pub private_key: SecretString,
    /// The app bundle identifier (e.g. `com.example.app`).
    pub bundle_id: String,
    /// Use the sandbox environment instead of production.
    pub use_sandbox: bool,
    /// Override the base URL (useful for testing with wiremock).
    pub api_base_url: Option<String>,
}

/// Apple Push Notification service provider adapter.
///
/// Sends push notifications to iOS devices via Apple's HTTP/2 APNS API.
/// Authentication uses JWT (ES256) signed with a `.p8` private key.
pub struct ApnsProvider {
    client: reqwest::Client,
    config: ApnsConfig,
}

impl ApnsProvider {
    pub fn new(config: ApnsConfig) -> Self {
        Self {
            client: reqwest::Client::new(),
            config,
        }
    }

    fn base_url(&self) -> String {
        self.config
            .api_base_url
            .clone()
            .unwrap_or_else(|| {
                if self.config.use_sandbox {
                    "https://api.sandbox.push.apple.com".into()
                } else {
                    "https://api.push.apple.com".into()
                }
            })
    }

    fn generate_jwt(&self) -> Result<String, MessengerError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| ProviderKind::Apns.transport_error(format!("system time error: {e}")))?
            .as_secs();

        let claims = ApnsJwtClaims {
            iss: self.config.team_id.clone(),
            iat: now,
            exp: now + 3000, // 50 minutes; Apple recommends max 1 hour
        };

        let mut header = Header::new(Algorithm::ES256);
        header.kid = Some(self.config.key_id.clone());

        let key = EncodingKey::from_ec_pem(self.config.private_key.expose_secret().as_bytes())
            .map_err(|e| {
                MessengerError::Authentication {
                    provider: ProviderKind::Apns,
                    message: format!("invalid APNS private key: {e}"),
                }
            })?;

        jsonwebtoken::encode(&header, &claims, &key).map_err(|e| {
            MessengerError::Authentication {
                provider: ProviderKind::Apns,
                message: format!("failed to sign APNS JWT: {e}"),
            }
        })
    }
}

#[derive(Serialize)]
struct ApnsJwtClaims {
    iss: String,
    iat: u64,
    exp: u64,
}

#[derive(Serialize)]
struct ApnsPayload<'a> {
    aps: ApnsAps<'a>,
}

#[derive(Serialize)]
struct ApnsAps<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    alert: Option<ApnsAlert<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    badge: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    sound: Option<&'a str>,
}

#[derive(Serialize)]
struct ApnsAlert<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<&'a str>,
}

#[derive(Deserialize)]
struct ApnsErrorResponse {
    #[serde(default)]
    reason: String,
    #[serde(default)]
    timestamp: Option<u64>,
}

#[async_trait::async_trait]
impl super::Provider for ApnsProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Apns
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

    #[tracing::instrument(skip_all, fields(provider = "apns", device = tracing::field::Empty))]
    async fn send_prepared(
        &self,
        dispatch: &Dispatch,
        message: &PreparedMessage,
    ) -> Result<SendReceipt, MessengerError> {
        let device_token = match &dispatch.target {
            Target::Apns(t) => &t.device_token,
            _ => {
                return Err(MessengerError::InvalidMessage(
                    "expected APNS target".into(),
                ));
            }
        };
        tracing::Span::current().record("device", tracing::field::display(device_token));

        let jwt = self.generate_jwt()?;

        let title = message.title();
        let body = match message.body() {
            Some(MessageBody::Plain(text)) => Some(text.as_str()),
            Some(MessageBody::Markdown(text)) => Some(text.as_str()),
            None => None,
        };

        let alert = if title.is_some() || body.is_some() {
            Some(ApnsAlert {
                title,
                body,
            })
        } else {
            None
        };

        let payload = ApnsPayload {
            aps: ApnsAps {
                alert,
                badge: None,
                sound: if dispatch.options.silent { None } else { Some("default") },
            },
        };

        let url = format!("{}/3/device/{}", self.base_url(), device_token);
        tracing::debug!(url = %url, "sending APNS request");

        let mut request = self.client.post(&url).json(&payload);
        request = request.header("authorization", format!("bearer {jwt}"));
        request = request.header("apns-topic", &self.config.bundle_id);
        request = request.header("content-type", "application/json");

        if dispatch.options.silent {
            request = request.header("apns-priority", "5");
        } else {
            request = request.header("apns-priority", "10");
        }

        let response = request.send().await.map_err(|e| {
            tracing::warn!(error = %e, "APNS transport request failed");
            ProviderKind::Apns.transport_error(e)
        })?;

        let status = response.status();
        tracing::debug!(status = %status, "received APNS response");

        if status.is_success() {
            let apns_id = response
                .headers()
                .get("apns-id")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("")
                .to_string();

            tracing::info!(apns_id = %apns_id, "APNS send complete");
            return Ok(SendReceipt {
                provider: ProviderKind::Apns,
                message_ref: MessageRef::Apns {
                    apns_id: apns_id.clone(),
                },
                raw_id: apns_id,
                metadata: BTreeMap::new(),
            });
        }

        let error_body: ApnsErrorResponse = response.json().await.unwrap_or(ApnsErrorResponse {
            reason: format!("HTTP {status}"),
            timestamp: None,
        });

        tracing::warn!(reason = %error_body.reason, "APNS returned error");

        if error_body.reason == "BadDeviceToken"
            || error_body.reason == "Unregistered"
            || error_body.reason == "DeviceTokenNotForTopic"
        {
            return Err(MessengerError::InvalidMessage(format!(
                "APNS device token error: {}",
                error_body.reason
            )));
        }

        if error_body.reason == "ExpiredProviderToken"
            || error_body.reason == "InvalidProviderToken"
            || error_body.reason == "MissingProviderToken"
        {
            return Err(MessengerError::Authentication {
                provider: ProviderKind::Apns,
                message: error_body.reason,
            });
        }

        Err(MessengerError::Provider {
            provider: ProviderKind::Apns,
            code: Some(error_body.reason.clone()),
            message: error_body.reason,
        })
    }
}
