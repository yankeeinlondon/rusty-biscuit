use std::collections::BTreeMap;

use secrecy::{ExposeSecret, SecretString};
use twilight_http::Client;
use twilight_model::id::Id;
use twilight_model::id::marker::{ChannelMarker, MessageMarker};

use crate::capabilities::CapabilitySet;
use crate::dispatch::Dispatch;
use crate::error::MessengerError;
use crate::message::MessageBody;
use crate::prepared::PreparedMessage;
use crate::receipt::{MessageRef, ProviderKind, SendReceipt};
use crate::target::Target;

/// Configuration for the Discord provider.
pub struct DiscordConfig {
    pub bot_token: SecretString,
}

/// Discord provider adapter using the Discord REST API via twilight-http.
pub struct DiscordProvider {
    client: Client,
}

impl DiscordProvider {
    pub fn new(config: DiscordConfig) -> Self {
        let client = Client::new(config.bot_token.expose_secret().to_string());
        Self { client }
    }

    fn parse_channel_id(s: &str) -> Result<Id<ChannelMarker>, MessengerError> {
        s.parse::<u64>()
            .map(Id::new)
            .map_err(|_| MessengerError::InvalidMessage(format!("invalid Discord channel ID: {s}")))
    }

    fn parse_message_id(s: &str) -> Result<Id<MessageMarker>, MessengerError> {
        s.parse::<u64>()
            .map(Id::new)
            .map_err(|_| MessengerError::InvalidMessage(format!("invalid Discord message ID: {s}")))
    }
}

#[cfg(test)]
mod tests {
    use super::DiscordProvider;

    #[test]
    fn rejects_invalid_channel_id() {
        let err = DiscordProvider::parse_channel_id("not-a-number").unwrap_err();
        assert!(matches!(
            err,
            crate::MessengerError::InvalidMessage(message)
            if message.contains("invalid Discord channel ID")
        ));
    }

    #[test]
    fn rejects_invalid_message_id() {
        let err = DiscordProvider::parse_message_id("not-a-number").unwrap_err();
        assert!(matches!(
            err,
            crate::MessengerError::InvalidMessage(message)
            if message.contains("invalid Discord message ID")
        ));
    }
}

#[async_trait::async_trait]
impl super::Provider for DiscordProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Discord
    }

    fn capabilities(&self) -> CapabilitySet {
        CapabilitySet {
            supports_markdown_rendering: true,
            supports_reply: true,
            supports_attachments: false,
            supports_location: false,
            supports_silent_delivery: false,
            supports_link_preview_control: false,
        }
    }

    async fn send_prepared(
        &self,
        dispatch: &Dispatch,
        message: &PreparedMessage,
    ) -> Result<SendReceipt, MessengerError> {
        let channel_id = match &dispatch.target {
            Target::Discord(t) => Self::parse_channel_id(&t.channel_id)?,
            _ => {
                return Err(MessengerError::InvalidMessage(
                    "expected Discord target".into(),
                ))
            }
        };

        // Render the message body
        let content = match message.body() {
            Some(MessageBody::Plain(_)) | Some(MessageBody::Markdown(_)) => {
                message.render_body_for_provider(ProviderKind::Discord)
            }
            None => String::new(),
        };

        // Build the message request
        let mut req = self.client.create_message(channel_id);

        if !content.is_empty() {
            req = req.content(&content);
        }

        // Handle reply_to
        if let Some(MessageRef::Discord { message_id, .. }) = &dispatch.reply_to {
            let msg_id = Self::parse_message_id(message_id)?;
            req = req.reply(msg_id);
        }
        // Execute the request
        let response = req.await.map_err(|e| MessengerError::Transport {
            provider: ProviderKind::Discord,
            message: e.to_string(),
        })?;

        let msg = response
            .model()
            .await
            .map_err(|e| MessengerError::Transport {
                provider: ProviderKind::Discord,
                message: e.to_string(),
            })?;

        Ok(SendReceipt {
            provider: ProviderKind::Discord,
            message_ref: MessageRef::Discord {
                channel_id: channel_id.to_string(),
                message_id: msg.id.to_string(),
            },
            raw_id: msg.id.to_string(),
            metadata: BTreeMap::new(),
        })
    }
}
